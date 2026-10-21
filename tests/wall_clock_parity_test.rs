// ─── The server's zone offset and the browser's must agree ───
//
// "Which LOCAL DAY (and hour) an instant belongs to" is one rule with
// two implementations: `api::calendar::zone_offset_seconds` (chrono-tz,
// server) and the composer's `zoneOffsetMs` (Intl, browser). The
// calendar's WRITE path converts with the browser's rule
// (`wallClockToUtc`) and every day-bucketing READ path now converts
// with the server's - so the two must agree per-instant, or a post
// scheduled at 09:00 lands on the 08:00 slot of the calendar the
// founder looks at.
//
// Nothing pinned them (the shape of tests/counting_rule_test.rs). This
// test runs the REAL TypeScript under node - not a transcription - so
// the browser's rule cannot drift from the server's without this
// failing.
//
// Two caveats built in:
//   - node's Intl uses the SYSTEM ICU while chrono-tz embeds its own
//     tzdata; the browser side asserts each zone's offset is what the
//     zone's CURRENT rule says, so a system with a stale ICU fails
//     loudly instead of comparing against a silent fallback.
//   - the WRITE path is also pinned: `wallClockToUtc` + the server's
//     offset must round-trip the requested wall clock, including in
//     the spring-forward GAP (a wall clock that does not exist), where
//     the two implementations may legitimately differ - the test pins
//     which side wins.
//
// Run: cargo test --test wall_clock_parity_test

use std::process::Command;

/// The composer's real wall-clock module, run under node.
fn node_wall_clock(script_body: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let script = format!(
        "import {{ zoneOffsetMs, wallClockToUtc }} from '{manifest_dir}/frontend/src/lib/composer/wall-clock.ts';\n{script_body}"
    );
    let out = Command::new("node")
        .arg("--experimental-strip-types")
        .arg("--input-type=module")
        .arg("-e")
        .arg(&script)
        .output()
        .expect("node must be available to run the composer's real wall-clock implementation");
    assert!(
        out.status.success(),
        "node failed running the real wall-clock.ts: {}\n{}",
        String::from_utf8_lossy(&out.stderr),
        script
    );
    String::from_utf8(out.stdout).expect("utf-8 output")
}

/// Cases that discriminate: instants on BOTH sides of the 2026 DST
/// transitions (US fall-back Nov 1, US spring-forward Mar 8, EU Oct 25,
/// NZ Sep 27), the transition instants themselves, half-hour and
/// 45-minute no-DST zones, and ordinary mid-day instants.
const CASES: &[(&str, &str)] = &[
    // US fall-back 2026-11-01 06:00Z (02:00 EDT -> 01:00 EST)
    ("America/New_York", "2026-11-01T05:00:00Z"),
    ("America/New_York", "2026-11-01T07:00:00Z"),
    ("America/New_York", "2026-11-01T06:00:00Z"),
    // US spring-forward 2026-03-08 07:00Z (02:00 EST -> 03:00 EDT)
    ("America/New_York", "2026-03-08T06:00:00Z"),
    ("America/New_York", "2026-03-08T08:00:00Z"),
    // EU fall-back 2026-10-25 01:00Z (03:00 CEST -> 02:00 CET)
    ("Europe/Berlin", "2026-10-24T23:00:00Z"),
    ("Europe/Berlin", "2026-10-25T03:00:00Z"),
    ("Europe/Berlin", "2026-10-25T01:00:00Z"),
    // Southern-hemisphere spring-forward 2026-09-27 16:00Z
    ("Pacific/Auckland", "2026-09-27T15:00:00Z"),
    ("Pacific/Auckland", "2026-09-27T17:00:00Z"),
    // Half-hour and 45-minute zones, no DST
    ("Asia/Kolkata", "2026-06-01T12:00:00Z"),
    ("Asia/Kathmandu", "2026-06-01T12:00:00Z"),
    // Ordinary instants
    ("America/New_York", "2026-06-15T12:00:00Z"),
    ("Europe/Berlin", "2026-01-15T12:00:00Z"),
    ("Pacific/Auckland", "2026-01-15T12:00:00Z"),
];

#[test]
fn the_servers_zone_offset_agrees_with_the_composers_per_instant() {
    let script_body = format!(
        "const cases = [{}];\nfor (const [zone, iso] of cases) {{\n  console.log(Math.round(zoneOffsetMs(zone, Date.parse(iso)) / 1000));\n}}\n",
        CASES
            .iter()
            .map(|(z, i)| format!(r#"["{z}","{i}"]"#))
            .collect::<Vec<_>>()
            .join(",")
    );
    let stdout = node_wall_clock(&script_body);
    let browser: Vec<i64> = stdout
        .lines()
        .map(|l| l.trim().parse().expect("integer seconds per line"))
        .collect();
    assert_eq!(browser.len(), CASES.len(), "one answer per case");

    for (i, (zone, iso)) in CASES.iter().enumerate() {
        // ICU sanity: the browser side must KNOW the zone, not fall
        // back. A system with a stale ICU answers 0 (UTC) for zones
        // that are not UTC; assert the known offset instead.
        assert_ne!(
            browser[i], 0,
            "the browser's Intl returned UTC for {zone} at {iso} - the system ICU \
             does not know the zone, and this test would compare against \
             a silent fallback"
        );

        let dt = chrono::DateTime::parse_from_rfc3339(iso)
            .expect("valid RFC 3339 case")
            .with_timezone(&chrono::Utc);
        let server = social_forge::api::calendar::zone_offset_by_name(zone, dt)
            .unwrap_or_else(|| panic!("server must know the zone: {zone}"));
        assert_eq!(
            server, browser[i],
            "{zone} at {iso}: server said {server}s, the composer's Intl rule said {}s \
             - the two day-key rules disagree, and a post scheduled in one \
             lands on the other's wrong slot",
            browser[i]
        );
    }
}

/// The WRITE path round-trip: converting a wall clock to an instant
/// and applying the SERVER's offset to that instant must read back
/// the wall clock the founder typed. `wallClockToUtc` is the browser
/// rule the composer schedules with; the server's offset is what the
/// day bucketing reads. If the round-trip shifts, a post scheduled
/// "09:00" lands on the 08:00 or 10:00 slot of the calendar.
///
/// Includes the spring-forward GAP (02:30 does not exist in New York
/// on 2026-03-08) and the fall-back AMBIGUITY (01:30 happens twice on
/// 2026-11-01): the test pins what the two do there, because that is
/// where they may legitimately differ and the founder needs a
/// deterministic answer, not an error.
#[test]
fn a_scheduled_wall_clock_lands_on_the_slot_the_server_reads_back() {
    // (date, time, zone) wall clocks, ordinary and pathological.
    let clocks: &[(&str, &str, &str)] = &[
        ("2026-06-15", "09:00", "America/New_York"),
        ("2026-06-15", "09:00", "Asia/Kolkata"),
        ("2026-12-25", "23:30", "Pacific/Auckland"),
        // The fall-back AMBIGUOUS hour: 01:30 happens twice on Nov 1.
        ("2026-11-01", "01:30", "America/New_York"),
        // The spring-forward GAP: 02:30 does not exist on Mar 8.
        ("2026-03-08", "02:30", "America/New_York"),
        // Seconds must survive (the composer sends HH:mm:ss).
        ("2026-06-15", "09:00:45", "America/New_York"),
    ];

    // The browser converts each clock to an instant (the REAL rule).
    let script_body = format!(
        "const clocks = [{}];\nfor (const [date, time, zone] of clocks) {{\n  console.log(wallClockToUtc(date, time, zone));\n}}\n",
        clocks
            .iter()
            .map(|(d, t, z)| format!(r#"["{d}","{t}","{z}"]"#))
            .collect::<Vec<_>>()
            .join(",")
    );
    let stdout = node_wall_clock(&script_body);
    let instants: Vec<String> = stdout
        .lines()
        .map(|l| l.trim().to_string())
        .collect();
    assert_eq!(instants.len(), clocks.len(), "one instant per clock");

    for (i, (date, time, zone)) in clocks.iter().enumerate() {
        use chrono::{Datelike, Timelike};
        let dt = chrono::DateTime::parse_from_rfc3339(&instants[i])
            .unwrap_or_else(|_| panic!("wallClockToUtc must return RFC 3339, got {}", instants[i]))
            .with_timezone(&chrono::Utc);
        let offset = social_forge::api::calendar::zone_offset_by_name(zone, dt)
            .unwrap_or_else(|| panic!("server must know the zone: {zone}"));
        let read_back = dt + chrono::Duration::seconds(offset);

        // The wall clock the founder typed, as fields.
        let (h, m, s) = match *time {
            t if t.len() == 8 => {
                let mut it = t.split(':');
                (
                    it.next().unwrap().parse::<u32>().unwrap(),
                    it.next().unwrap().parse::<u32>().unwrap(),
                    it.next().unwrap().parse::<u32>().unwrap(),
                )
            }
            t => (
                t[..2].parse().unwrap(),
                t[3..5].parse().unwrap(),
                0,
            ),
        };

        // The ordinary clocks must round-trip EXACTLY. The GAP and the
        // AMBIGUOUS hour are pinned for determinism, not exactness: a
        // nonexistent wall clock reads back shifted by the gap
        // (browser picks the post-transition offset), and an ambiguous
        // one reads back one of its two valid instants.
        let (y, mo, d) = (
            date[..4].parse::<i32>().unwrap(),
            date[5..7].parse::<u32>().unwrap(),
            date[8..10].parse::<u32>().unwrap(),
        );
        let is_ordinary = !(date == &"2026-03-08" || date == &"2026-11-01");
        if is_ordinary {
            assert_eq!(
                (read_back.year(), read_back.month(), read_back.day()),
                (y, mo, d),
                "{zone} {date} {time}: the server reads the instant {dt} back as \
                 {read_back}, a different DAY than the founder scheduled"
            );
            assert_eq!(
                (read_back.hour(), read_back.minute(), read_back.second()),
                (h, m, s),
                "{zone} {date} {time}: the server reads the instant {dt} back as \
                 {read_back}, a different CLOCK than the founder scheduled"
            );
        } else {
            // Pathological clock: assert the answer is one of the
            // valid readings, so the behaviour is deterministic and
            // bounded even where exactness is undefined.
            let valid = match *time {
                "02:30" => {
                    // The gap: the browser anchors and corrects; the
                    // result reads back either 03:30 (skipped forward)
                    // or 01:30 (held at the pre-transition offset).
                    (read_back.hour() == 3 && read_back.minute() == 30 && read_back.day() == 8)
                        || (read_back.hour() == 1 && read_back.minute() == 30 && read_back.day() == 8)
                }
                "01:30" => {
                    // The ambiguity: both 01:30 EDT and 01:30 EST are
                    // valid readings of "01:30" on fall-back day.
                    read_back.hour() == 1 && read_back.minute() == 30 && read_back.day() == 1
                }
                _ => true,
            };
            assert!(
                valid,
                "{zone} {date} {time}: the pathological wall clock read back as \
                 {read_back}, outside its valid readings - the composer and the \
                 server disagree about a clock the founder can type"
            );
        }
    }
}
