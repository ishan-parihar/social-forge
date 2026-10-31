// ─── Red-team: a multi-page batch must not stop at its first failure ──
//
// The page picker connects a selection of pages one at a time. Its loop
// used to end at the first failure:
//
//     if (r.error) {
//       errorMsg = `Failed to connect ${pageId}: ${r.error}`;
//       break;
//     }
//
// A `break` in a batch loop abandons everything after the failure. The
// founder selects five pages; the second is refused (a permission on
// that one page, a page deleted upstream, a rate limit mid-batch); the
// modal reports only that one, and the three pages after it are never
// attempted at all - they stay unconnected with no signal that anything
// happened to them. The founder's next move is to reopen the picker and
// re-select, and because `onSuccess` was also skipped, the connected one
// was still shown as selectable.
//
// This is a source-invariant test: the picker is a browser-side Svelte
// component with no API surface a unit test can drive, so the invariant
// it must hold is the shape of the loop body. It is deliberately narrow -
// it pins the `continue` AND the plural reporting, because a fix that
// only removed the `break` would drop the failures on the floor.
//
// Run: cargo test --test page_picker_batch_test

use std::path::PathBuf;

const PICKER: &str = "frontend/src/lib/channels/PagePicker.svelte";

fn picker_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(PICKER);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {PICKER}: {e}"))
}

/// The body of `connectSelected` - the function that runs the batch.
fn connect_selected_body(src: &str) -> String {
    let start = src
        .find("async function connectSelected()")
        .expect("connectSelected not found");
    let rest = &src[start..];
    // The fn ends at the first closing brace at column 0 (`\n  }\n` for a
    // two-space-indented Svelte script block).
    let end = rest
        .find("\n  }\n")
        .unwrap_or_else(|| panic!("no body end for connectSelected"));
    rest[..end].to_string()
}

/// A failure must not abandon the rest of the batch. `continue` is the
/// fix; `break` is the defect.
#[test]
fn a_failure_does_not_abandon_the_pages_after_it() {
    let body = connect_selected_body(&picker_source());

    assert!(
        !body.contains("break;"),
        "connectSelected still breaks on the first failure, abandoning every \
         page after it with no signal"
    );
    assert!(
        body.contains("continue;"),
        "connectSelected neither breaks nor continues - a failure is now \
         silently swallowed"
    );
}

/// The failures must be REPORTED, and reported together. A loop that
/// continues but forgets the error is worse than one that stops: the
/// founder gets a success and a list still missing pages.
#[test]
fn failures_are_collected_and_reported_together() {
    let body = connect_selected_body(&picker_source());

    assert!(
        body.contains("failures.push("),
        "failures are not collected, so a partial batch reports only 'N \
         connected' and never names what failed"
    );
    assert!(
        body.contains("failures.join("),
        "collected failures are not joined into the message"
    );
}

/// A partial success is still a success for the channels that connected:
/// the list must refresh either way, or a connected page stays offered
/// as connectable.
#[test]
fn a_partial_success_still_refreshes_the_list() {
    let body = connect_selected_body(&picker_source());

    // `onSuccess` must be called unconditionally at the end, not gated on
    // "no failures". The gated form is what makes a connected page look
    // unconnected in the list behind the modal.
    assert!(
        !body.contains("if (!errorMsg)"),
        "onSuccess is still gated on 'no failures', so a partial batch leaves \
         the connected pages looking unconnected in the list behind the modal"
    );
    assert!(
        body.contains("onSuccess?.()"),
        "onSuccess is not called at all - the list behind the modal is never \
         refreshed after a batch"
    );

    // And it must NOT sit inside the `if (failures.length > 0)` branch, which
    // is the other way to gate it: an all-success batch would then never
    // refresh the list behind the modal.
    //
    // Take the branch's BODY only - the text between the opening brace of
    // `if (failures.length > 0) {` and the `selected = new Set();` that
    // follows the branch close. Splitting on the marker and taking [1]
    // instead would give everything after the marker, which includes the
    // unconditional call at the end and makes the assertion always-false.
    let branch_start = body
        .find("if (failures.length > 0) {")
        .expect("the failures reporting branch is missing");
    let branch_body = &body[branch_start..];
    let branch_end = branch_body
        .find("selected = new Set();")
        .expect("the branch never closes before the end of the function");
    assert!(
        !branch_body[..branch_end].contains("onSuccess"),
        "onSuccess only runs when something failed - an all-success batch no \
         longer refreshes the list"
    );
}
