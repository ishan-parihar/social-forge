// ─── Red-team: a connected provider keeps its tile in the grid ───────
//
// The Channels grid used to hide a provider the moment ONE integration
// existed for it. The founder managing many channels connects a second
// Google account, a third LinkedIn identity and a second X handle as a
// matter of course, and the filter made every one of those impossible:
// one account connected left one tile missing, five accounts connected
// left five tiles missing, and the only way back was to disconnect the
// one you wanted to keep alongside.
//
// The fix is a count (`connectedCounts`) instead of a filter, and an
// "N connected" badge, so the tile reads "Add another X" rather than
// disappearing.
//
// This is a source-invariant test: the grid is a browser-side Svelte
// component with no API a unit test drives, so the invariant it must hold
// is the shape of the derivation that decides which tiles render. It pins
// the absence of the hiding rule rather than the presence of a fix, which
// is the only durable assertion here - a re-implementation can use a
// different mechanism and still be correct, as long as it does not
// FILTER a connected provider out of `connectableProviders`.
//
// Run: cargo test --test channels_grid_multi_account_test

use std::path::PathBuf;

const PAGE: &str = "frontend/src/routes/channels/+page.svelte";

fn page_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(PAGE);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {PAGE}: {e}"))
}

/// A provider already present in `integrations` must still render a tile,
/// because that tile is now the only way to add a SECOND account.
#[test]
fn a_connected_provider_is_not_filtered_out_of_the_grid() {
    let src = page_source();

    // The defect was a `.filter(...)` over the provider list against the
    // set of already-connected ones. Anything that reintroduces a filter
    // on `availableProviders` reintroduces the bug.
    //
    // Take the whole statement, not one line: the filter in the original
    // defect sat on its own line inside a multi-line $derived, and a
    // line-scoped check on the opening line sails straight past it. That
    // is how this test failed to notice a mutation it was written for.
    let start = src
        .find("let connectableProviders =")
        .expect("connectableProviders is not declared");
    let stmt = &src[start..];
    let end = stmt
        .find(";\n")
        .unwrap_or_else(|| panic!("connectableProviders statement never terminates"));
    let stmt = &stmt[..end];

    assert!(
        !stmt.contains(".filter("),
        "connectableProviders filters the provider list again: `{stmt}` - a \
         provider with one integration would lose its tile, so adding a \
         second account of that provider becomes impossible"
    );

    // And the derivation must exist at all, or the grid renders nothing.
    assert!(
        src.contains("connectableProviders"),
        "connectableProviders is gone; the grid's `{{#each}}` renders nothing"
    );
}

/// The "N connected" count is what makes a kept tile readable: without it
/// a connected provider's tile is indistinguishable from a fresh one and
/// the founder clicks it expecting a first connect.
#[test]
fn the_count_of_already_connected_channels_is_available_to_the_grid() {
    let src = page_source();

    assert!(
        src.contains("connectedCounts"),
        "connectedCounts is gone - the grid cannot tell 'connected once' from \
         'not connected', and the tile's label no longer says 'Add another'"
    );

    // The label must follow the count, which is the user-visible half of
    // keeping the tile. Without it the tile reads 'Connect' for a provider
    // that is already connected, and clicking it looks like a bug.
    assert!(
        src.contains("Add another"),
        "the tile no longer says 'Add another' when the provider is already \
         connected - the kept tile reads as a first connect"
    );
}

/// A plain `Set` of connected providers is not enough: the founder with
/// three LinkedIn profiles must see the three. A Map count works; a Set
/// does not. This pins that the counting is a count and not a boolean.
#[test]
fn the_connected_state_is_counted_not_merely_flagged() {
    let src = page_source();
    // Take the WHOLE declaration: from `let connectedCounts =` to the
    // `});` that closes the $derived.by callback. A fixed character window
    // truncates it, which is a test that passes for the wrong reason.
    let start = src
        .find("let connectedCounts =")
        .expect("connectedCounts is not declared");
    let decl = &src[start..];
    let end = decl
        .find("});")
        .unwrap_or_else(|| panic!("connectedCounts declaration never closes"));
    let decl = &decl[..end];

    assert!(
        decl.contains("new Map<"),
        "connectedCounts is not a Map, so it cannot say how many channels a \
         provider has - only whether it has any"
    );
    assert!(
        decl.contains("?? 0) + 1"),
        "connectedCounts does not accumulate, so N integrations for one \
         provider read as 1"
    );
}
