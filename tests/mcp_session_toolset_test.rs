// ─── A live MCP session can enable and call a non-core toolset ───
//
// The shell path (`switchpost mcp-call`) was fixed to bypass the
// session's toolset filter. The LIVE session path is the other half of
// the same contract, and it has never been proven at runtime:
//
//   - the session boots with the `core` toolset only,
//   - `tools/list` advertises only the enabled sets,
//   - `tools/call` REJECTS a disabled tool ("tool not found"),
//   - `tools_enable` adds a set,
//   - and the newly enabled tool must be callable AFTERWARDS.
//
// The rmcp `#[tool_handler]` macro evaluates `self.filtered_tool_router()`
// inside the generated `call_tool` / `list_tools` bodies, so the router
// IS re-read per request — but that is a source-level reading of macro
// expansion, not evidence. This test drives the REAL binary over stdio.
//
// If the router were captured once at construction, the post-enable
// `tools/call` would fail with "tool not found" — exactly the defect
// this pins.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// A stdio JSON-RPC client for the real `switchpost mcp` server.
struct Session {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl Session {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_switchpost"))
            .arg("mcp")
            .env("SWITCHPOST_TOOLSETS", "core")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the switchpost binary must start an MCP stdio server");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let mut s = Session {
            child,
            stdin,
            stdout,
            next_id: 0,
        };
        s.initialize();
        s
    }

    fn send(&mut self, method: &str, params: Option<serde_json::Value>) -> i64 {
        self.next_id += 1;
        let id = self.next_id;
        let mut req = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "id": id,
        });
        if let Some(p) = params {
            req["params"] = p;
        }
        let line = format!("{req}\n");
        self.stdin.write_all(line.as_bytes()).expect("write");
        self.stdin.flush().expect("flush");
        id
    }

    fn notify(&mut self, method: &str) {
        let line = format!(
            "{{\"jsonrpc\":\"2.0\",\"method\":\"{method}\"}}\n"
        );
        self.stdin.write_all(line.as_bytes()).expect("write");
        self.stdin.flush().expect("flush");
    }

    /// Read lines until the response for `id` arrives, skipping the
    /// server's notifications.
    fn read_response(&mut self, id: i64) -> serde_json::Value {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for MCP response {id}"
            );
            let mut line = String::new();
            let n = self.stdout.read_line(&mut line).expect("read");
            assert!(n > 0, "MCP server closed stdout while waiting for {id}");
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if msg.get("id").and_then(|v| v.as_i64()) == Some(id) {
                return msg;
            }
        }
    }

    fn request(&mut self, method: &str, params: Option<serde_json::Value>) -> serde_json::Value {
        let id = self.send(method, params);
        self.read_response(id)
    }

    fn initialize(&mut self) {
        let resp = self.request(
            "initialize",
            Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "toolset-contract-test", "version": "1.0.0"}
            })),
        );
        assert!(
            resp.get("error").is_none(),
            "MCP initialize failed: {resp}"
        );
        self.notify("notifications/initialized");
    }

    fn list_tools(&mut self) -> Vec<String> {
        let resp = self.request("tools/list", None);
        assert!(resp.get("error").is_none(), "tools/list failed: {resp}");
        resp["result"]["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .map(|t| t["name"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    fn call_tool(&mut self, name: &str, args: serde_json::Value) -> serde_json::Value {
        self.request(
            "tools/call",
            Some(serde_json::json!({"name": name, "arguments": args})),
        )
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The raw text of a tools/call response, so an assertion can look for
/// "tool not found" wherever the server put it.
fn call_text(resp: &serde_json::Value) -> String {
    let mut out = resp.to_string();
    if let Some(content) = resp["result"]["content"].as_array() {
        for c in content {
            if let Some(t) = c["text"].as_str() {
                out.push_str(t);
            }
        }
    }
    out
}

#[test]
fn a_live_session_can_enable_and_then_call_a_non_core_toolset() {
    let mut s = Session::start();

    // 1. The boot toolset is `core` — the reddit tools are NOT advertised.
    let before = s.list_tools();
    assert!(
        !before.iter().any(|n| n.starts_with("reddit_")),
        "the core boot toolset must not advertise reddit tools: {before:?}"
    );
    // The meta tools are always present, so an agent can find the rest.
    assert!(
        before.iter().any(|n| n == "tools_enable"),
        "tools_enable must always be advertised: {before:?}"
    );
    assert!(
        before.iter().any(|n| n == "tools_search"),
        "tools_search must always be advertised: {before:?}"
    );

    // 2. A disabled tool is REJECTED by name — this is the failure mode
    //    that made the surface look broken.
    let rejected = s.call_tool("reddit_get_karma", serde_json::json!({"username": "x"}));
    let rejected_text = call_text(&rejected);
    assert!(
        rejected_text.contains("tool not found"),
        "a tool outside the enabled toolsets must be rejected by name: {rejected_text}"
    );

    // 3. Enable the set.
    let enabled = s.call_tool(
        "tools_enable",
        serde_json::json!({"sets": ["reddit"]}),
    );
    assert!(
        enabled.get("error").is_none(),
        "tools_enable must succeed: {enabled}"
    );

    // 4. The newly enabled set is advertised on the NEXT tools/list.
    let after = s.list_tools();
    assert!(
        after.iter().any(|n| n.starts_with("reddit_")),
        "an enabled toolset must be advertised on the next tools/list: {after:?}"
    );

    // 5. And the tool is now CALLABLE — it may fail on credentials, but
    //    it must not fail with "tool not found".
    let called = s.call_tool("reddit_get_karma", serde_json::json!({"username": "x"}));
    let called_text = call_text(&called);
    assert!(
        !called_text.contains("tool not found"),
        "an enabled tool must be callable after tools_enable: {called_text}"
    );
}
