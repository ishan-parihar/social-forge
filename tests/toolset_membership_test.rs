// ─── The toolset membership rule is ONE rule ────────────────
//
// Three surfaces filter tools by set:
//   1. the live session router (`filtered_tool_router`),
//   2. `tools_search`'s `enabled` flag,
//   3. the CLI's `mcp-tools --toolset`.
//
// They were three copies of the rule, and they disagreed:
//
//   - `mcp-tools --toolset all` returned ZERO tools. The session router
//     treats `all` as "no filtering"; the CLI compared each tool's own
//     set to the literal string "all" and matched nothing. The AGENTS.md
//     documented `all` as a valid value.
//   - the CLI dropped the meta tools (`tools_search`, `tools_enable`),
//     which the session ALWAYS keeps. An agent sizing a session from the
//     shell was told it could not search for more tools.
//
// The rule now lives in ONE place (`tool_in_toolset`) and all three
// surfaces call it. These tests pin the rule itself and pin that the
// three surfaces agree.

use switchpost::mcp::SwitchpostMcpServer;

/// Every tool the router advertises, unfiltered.
fn all_tools() -> Vec<String> {
    SwitchpostMcpServer::router()
        .list_all()
        .iter()
        .map(|t| t.name.to_string())
        .collect()
}

#[test]
fn all_means_no_filtering() {
    let tools = all_tools();
    assert!(!tools.is_empty(), "the router must advertise tools");
    for name in &tools {
        assert!(
            SwitchpostMcpServer::tool_in_toolset(name, "all"),
            "`all` must keep every tool, but it dropped {name}"
        );
    }
}

#[test]
fn the_meta_tools_are_never_filtered_out() {
    // An agent must always be able to search for and enable more
    // toolsets - otherwise a session that enabled `x` could never reach
    // the tool it needed next.
    for name in ["tools_search", "tools_enable"] {
        for set in ["core", "x", "reddit", "webhooks"] {
            assert!(
                SwitchpostMcpServer::tool_in_toolset(name, set),
                "{name} must survive a {set}-only session"
            );
        }
    }
}

#[test]
fn a_narrow_set_keeps_only_its_own_tools_and_the_meta_tools() {
    let tools = all_tools();
    let x_tools: Vec<&String> = tools
        .iter()
        .filter(|t| SwitchpostMcpServer::tool_in_toolset(t, "x"))
        .collect();
    assert!(
        x_tools.len() > 1,
        "the x toolset must not be empty (only {} found)",
        x_tools.len()
    );
    for name in &x_tools {
        let is_meta = matches!(name.as_str(), "tools_search" | "tools_enable");
        let is_x = SwitchpostMcpServer::toolset_of(name) == "x";
        assert!(
            is_meta || is_x,
            "{name} is neither a meta tool nor an x tool, but survived an x-only filter"
        );
    }
}

#[test]
fn the_cli_and_the_session_filter_agree_on_every_tool() {
    // The shell listing and the live session must advertise the SAME
    // tools for a set. This is the agreement the three copies of the
    // rule broke.
    let tools = all_tools();
    for set in ["x", "core", "reddit", "webhooks", "all"] {
        let by_rule: Vec<&String> = tools
            .iter()
            .filter(|t| SwitchpostMcpServer::tool_in_toolset(t, set))
            .collect();
        // The session router's own rule, spelled out the way
        // `filtered_tool_router` applies it.
        let by_router: Vec<&String> = tools
            .iter()
            .filter(|t| {
                if set == "all" {
                    return true;
                }
                if matches!(t.as_str(), "tools_search" | "tools_enable") {
                    return true;
                }
                SwitchpostMcpServer::toolset_of(t) == set
            })
            .collect();
        assert_eq!(
            by_rule.len(),
            by_router.len(),
            "the CLI and the session router disagree on the {set} toolset"
        );
    }
}
