// ─── Skill-install guard ─────────────────────────────────────────────────
// `make install-skill` copies the skill into ~/.agents/skills/ so an agent
// can read it. It used to name each reference doc individually, and it named
// four of the five the skill ships — `quick-reference.md` was left behind,
// while SKILL.md and docs/PAGE-INDEX.md both link to it. The installed skill
// therefore pointed at a document the agent could not open, and nothing
// failed: the target still printed "Installed skill".
//
// A hardcoded file list in a Makefile cannot notice that the directory gained
// a file. This test reads the target and the directory and asserts they agree,
// so adding a reference without updating the target is a failing build rather
// than a silent gap.
//
// The target copies `references/*.md` now, which is the property under test:
// whatever the directory holds, the install takes.

use std::path::PathBuf;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// `name` is the basename SKILL.md leaves after stripping `references/`; the
/// file lives in the skill's `references/` directory.
fn check(base: &std::path::Path, name: &str, missing: &mut Vec<String>) {
    let p = base.join("references").join(name);
    if !p.is_file() {
        missing.push(name.to_string());
    }
}

/// The cp lines install-skill runs for reference docs.
fn installed_references() -> Vec<String> {
    let makefile = std::fs::read_to_string(repo_path("Makefile")).expect("read Makefile");
    let mut out = Vec::new();
    let mut in_target = false;
    for line in makefile.lines() {
        if line.starts_with("install-skill:") {
            in_target = true;
            continue;
        }
        if in_target {
            // A target ends at the next rule or a non-tab line.
            if !line.starts_with('\t') && !line.trim().is_empty() {
                break;
            }
            // Recipe lines are tab-indented and conventionally `@`-silenced:
            // `\t@cp $(SKILL_SRC)/SKILL.md ...`.
            let t = line.trim().trim_start_matches('@').trim();
            if let Some(rest) = t.strip_prefix("cp ") {
                out.push(rest.to_string());
            }
        }
    }
    out
}

/// What the skill actually ships in references/.
fn shipped_references() -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(repo_path("skills/switchpost-agent/references"))
        .expect("read references dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".md"))
        .collect();
    out.sort();
    out
}

#[test]
fn install_skill_copies_every_reference_the_skill_ships() {
    let shipped = shipped_references();
    assert!(
        !shipped.is_empty(),
        "skills/switchpost-agent/references/ holds no .md files — has the skill moved?"
    );

    // Either a wildcard copy, or an explicit list that covers every file.
    let lines = installed_references();
    let wildcard = lines
        .iter()
        .any(|l| l.contains("references/*.md") || l.contains("references/*"));

    if !wildcard {
        let named: std::collections::HashSet<String> = lines
            .iter()
            .filter_map(|l| {
                l.split_whitespace()
                    .next()
                    .map(std::path::Path::new)
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
            })
            .collect();
        let missing: Vec<&String> = shipped.iter().filter(|f| !named.contains(*f)).collect();
        assert!(
            missing.is_empty(),
            "make install-skill does not copy these shipped reference docs: {missing:?}. \
             Either add them to the target, or copy `references/*.md` so a new doc is never \
             silently left out of the installed skill."
        );
    }

    // Either way, SKILL.md itself must be installed.
    let skill_copied = lines
        .iter()
        .any(|l| l.contains("SKILL.md") && !l.contains("references"));
    assert!(
        skill_copied,
        "make install-skill does not copy SKILL.md; the installed skill would have no entrypoint"
    );
}

/// The docs that describe the skill must point at files that exist in it, so a
/// reader following a link is not sent to nothing.
#[test]
fn every_reference_doc_the_skill_links_to_exists() {
    let skill = std::fs::read_to_string(repo_path("skills/switchpost-agent/SKILL.md"))
        .expect("read SKILL.md");
    let base = repo_path("skills/switchpost-agent");
    let mut missing = Vec::new();
    for line in skill.lines() {
        // Markdown links too: `…](references/providers.md)`.
        for span in line.split('`').skip(1).step_by(2) {
            if let Some(rest) = span.strip_prefix("references/") {
                check(&base, rest.trim(), &mut missing);
            }
        }
        for (_before, after) in line.split("](").zip(line.split("](").skip(1)) {
            let target = after.split(')').next().unwrap_or("");
            if let Some(rest) = target.strip_prefix("references/") {
                check(&base, rest.trim(), &mut missing);
            }
        }
    }
    assert!(
        missing.is_empty(),
        "SKILL.md links to reference docs that do not ship: {missing:?}"
    );
}
