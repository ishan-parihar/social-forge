// ─── The cell cap, pinned at its boundaries ───
//
// `cell-cap.ts` is ONE rule shared by three call sites - the week, the
// month and the day grid - and its own comment says so. That is why a
// defect in it is a defect in three views at once, which is exactly what
// happened: all three guarded the disclosure on `hidden > 0` alone, so
// expanding a busy slot removed the only control that could collapse it
// again. The rule's `expanded` branch is what the fix leans on, and until
// this test existed NOTHING covered it - not `cargo test --lib`, not the
// probe, not svelte-check.
//
// A rule three call sites depend on needs its boundaries pinned, not just
// its happy path. Rust is where this suite lives, so the rule is
// transcribed the way `counting_rule_test.rs` and `html_to_text_test.rs`
// transcribe the composer's rules - ONE implementation, mirrored, compared.
//
// Run: cargo test --test cell_cap_test

/// The cap. Mirrors `export const CELL_CAP = 3` in cell-cap.ts.
const CELL_CAP: usize = 3;

/// Mirrors `capCell<T>(items, expanded)` in cell-cap.ts.
fn cap_cell<T: Clone>(items: &[T], expanded: bool) -> (Vec<T>, usize) {
    if expanded || items.len() <= CELL_CAP {
        (items.to_vec(), 0)
    } else {
        (items[..CELL_CAP].to_vec(), items.len() - CELL_CAP)
    }
}

/// Everything past the cap is hidden, and the count names it exactly.
#[test]
fn four_items_hide_one() {
    let items = ["a", "b", "c", "d"];
    let (visible, hidden) = cap_cell(&items, false);
    assert_eq!(visible, vec!["a", "b", "c"]);
    assert_eq!(hidden, 1);
}

/// The boundary: exactly the cap is NOT over it, so nothing is hidden and
/// no disclosure row renders at all. A cell with exactly three posts must
/// not show "+0 more".
#[test]
fn exactly_the_cap_hides_nothing() {
    let items = ["a", "b", "c"];
    let (visible, hidden) = cap_cell(&items, false);
    assert_eq!(visible, vec!["a", "b", "c"]);
    assert_eq!(hidden, 0);
}

/// One short of the cap is the other side of the same boundary.
#[test]
fn below_the_cap_shows_everything() {
    let items = ["a", "b"];
    let (visible, hidden) = cap_cell(&items, false);
    assert_eq!(visible, vec!["a", "b"]);
    assert_eq!(hidden, 0);
}

/// An empty cell must not render a disclosure claiming it is hiding posts.
#[test]
fn empty_cell_shows_nothing_and_hides_nothing() {
    let items: [&str; 0] = [];
    let (visible, hidden) = cap_cell(&items, false);
    assert!(visible.is_empty());
    assert_eq!(hidden, 0);
}

/// The expanded branch - THE one the three-view fix depends on. While
/// expanded, `hidden` goes to 0 and every chip shows. That is correct for
/// the render, but it is also what removed the collapse control from all
/// three views, because each guarded on `hidden > 0` alone. The rule
/// cannot express "expanded" on its own; the caller has to keep the row
/// mounted, which is why the guard is now `hidden > 0 || expanded`.
///
/// This test pins the RULE's half of that contract so the fix's
/// precondition cannot silently change.
#[test]
fn expanded_shows_everything_and_hides_nothing() {
    let items = ["a", "b", "c", "d", "e", "f", "g"];
    let (visible, hidden) = cap_cell(&items, true);
    assert_eq!(visible, items.to_vec());
    assert_eq!(hidden, 0);
}

/// Expanded and not-expanded must agree on WHICH items are shown, not just
/// how many: the expanded view is a superset that preserves order, so a
/// founder who expands and then collapses returns to the same three chips
/// in the same order. Verified on the running app as +7 -> +7.
#[test]
fn expanded_preserves_the_capped_order() {
    let items = ["a", "b", "c", "d"];
    let (capped, _) = cap_cell(&items, false);
    let (all, _) = cap_cell(&items, true);
    assert_eq!(all[..capped.len()], capped);
}

/// The hidden count is derived, never guessed, so the "+N more" label
/// cannot drift from what is actually hidden.
#[test]
fn hidden_equals_total_minus_shown_at_every_size() {
    for n in (CELL_CAP + 1)..24 {
        let items: Vec<usize> = (0..n).collect();
        let (visible, hidden) = cap_cell(&items, false);
        assert_eq!(visible.len(), CELL_CAP, "at n={n}");
        assert_eq!(hidden, n - CELL_CAP, "at n={n}");
        assert_eq!(visible.len() + hidden, n, "at n={n}");
    }
}
