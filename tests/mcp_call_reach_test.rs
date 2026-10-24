// ─── mcp-call must reach EVERY advertised tool ──────────────────
//
// `switchpost mcp-call <tool>` is the shell path the agent skill
// documents (116+ references). The MCP session boots with the `core`
// toolset only, and that filter is applied to the session's tool
// router.
//
// It used to be applied to `mcp-call` too, so every NON-core tool
// failed with:
//
//     Error: mcp-call 'reddit_get_karma' failed: Mcp error: -32602: tool not found
//
// An agent that read the skill, discovered the tool with
// `mcp-tools`, and called it got "tool not found" for ~90% of the
// surface.
//
// The filter exists to cut the ADVERTISED schema (the tools/list
// payload an agent pays for in context). It is not a capability
// switch: `mcp-call` names ONE tool explicitly and pays no list cost.
//
// These tests drive the REAL binary, so they pin the agent's actual
// path rather than a re-implementation of it.

use std::process::Command;

/// Run `switchpost mcp-call <tool>` and return its combined output.
fn mcp_call(tool: &str, args: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_switchpost"))
        .args([
            "mcp-call",
            tool,
            "--args",
            args,
        ])
        .output()
        .expect("the binary must run");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn a_core_tool_is_callable() {
    let out = mcp_call("posts_list", "{}");
    assert!(
        !out.contains("tool not found"),
        "a core tool must be callable: {out}"
    );
}

#[test]
fn a_non_core_tool_is_callable_too() {
    // `reddit_get_karma` is in the `reddit` set, which the default
    // session does NOT enable. The call must still REACH the tool —
    // it may fail on missing credentials, but it must not fail with
    // "tool not found".
    let out = mcp_call("reddit_get_karma", r#"{"username":"definitely_not_a_real_user"}"#);
    assert!(
        !out.contains("tool not found"),
        "a non-core tool must be callable from the shell: {out}"
    );
    assert!(
        !out.contains("unknown tool"),
        "the tool must be KNOWN to the router: {out}"
    );
}

#[test]
fn a_meta_tool_is_callable() {
    let out = mcp_call("tools_search", r#"{"query":"karma"}"#);
    assert!(
        !out.contains("tool not found"),
        "the meta tools must be callable: {out}"
    );
    assert!(
        out.contains("karma") || out.contains("matches"),
        "tools_search must actually search: {out}"
    );
}

#[test]
fn an_unknown_tool_still_reports_unknown() {
    // The fix must not turn the unknown-tool error into a success.
    let out = mcp_call("definitely_not_a_tool", "{}");
    assert!(
        out.contains("unknown tool"),
        "an unknown tool must be reported as unknown: {out}"
    );
}

// ── The --json/--pretty flags must reach mcp-call's output ──────
//
// mcp-call used to print the raw tool text with `print!`, bypassing
// the global format flags entirely — the same hardcode the platform
// shims' emit_result had. These drive the REAL binary on a DB-only
// tool (integrations_list: no provider calls, deterministic).

fn mcp_call_env(args: &[&str]) -> std::process::Output {
    // A scratch DB so the test never reads the operator's data. The
    // fresh DB has no user row until the binary initializes it — the
    // cheapest reliable way to seed one is to boot `serve` briefly
    // (its startup path calls ensure_local_user + migrations) and
    // kill it as soon as it prints the listening line.
    let db = std::env::temp_dir().join(format!("mcpfmt-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&db);
    let url = format!("sqlite://{}?mode=rwc", db.display());
    let mut serve = std::process::Command::new(env!("CARGO_BIN_EXE_switchpost"))
        .args(["serve", "--port", "47891"])
        .env("DATABASE_URL", &url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("serve must spawn");
    // Give it a moment to run migrations + seed the user, then kill.
    std::thread::sleep(std::time::Duration::from_secs(3));
    let _ = serve.kill();
    let _ = serve.wait();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_switchpost"))
        .args(args)
        .env("DATABASE_URL", &url)
        .output()
        .expect("the binary must run");
    let _ = std::fs::remove_file(&db);
    out
}

#[test]
fn json_flag_makes_mcp_call_emit_parseable_json() {
    let out = mcp_call_env(&["--json", "mcp-call", "integrations_list", "--args", "{}"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed = serde_json::from_str::<serde_json::Value>(stdout.trim())
        .unwrap_or_else(|e| panic!("--json output must parse as JSON ({e}): {stdout}"));
    assert!(
        parsed.get("integrations").is_some(),
        "the tool payload must survive the render: {stdout}"
    );
}

#[test]
fn mcp_call_defaults_to_toon_not_json() {
    let out = mcp_call_env(&["mcp-call", "integrations_list", "--args", "{}"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.trim_start().starts_with('{'),
        "default must stay TOON (an agent asking for JSON passes --json): {stdout}"
    );
    assert!(
        stdout.contains("integrations"),
        "the payload keys must still be present in TOON: {stdout}"
    );
}
