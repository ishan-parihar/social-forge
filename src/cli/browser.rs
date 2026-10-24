// ─── BrowserOS Neo MCP client (for `connect --browser`) ──────────
//
// A minimal JSON-RPC client for the browseros-neo MCP server.
//
// Why not `rmcp`'s streamable-HTTP client transport: it would pull in a
// SECOND reqwest (rmcp 1.6 pins reqwest 0.13; this repo is on 0.12), and
// this server speaks a tiny subset of the protocol — initialize, one
// notification, tools/call. The wire protocol is ~80 lines; a second HTTP
// stack is not.
//
// The handshake (verified against the live server):
//   POST /mcp  {"jsonrpc":"2.0","id":1,"method":"initialize",...}
//     -> the session id is in the RESPONSE HEADER `mcp-session-id`
//   POST /mcp  {"jsonrpc":"2.0","method":"notifications/initialized"}
//   POST /mcp  {"jsonrpc":"2.0","id":2,"method":"tools/call",...}
//     -> the body is SSE: `data:` lines, last one carrying result/error
//
// Tool results are text blocks; `evaluate` wraps its value in
// [UNTRUSTED_PAGE_CONTENT ...] markers, so the caller strips them.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};

/// One browseros-neo MCP session.
pub struct Browser {
    client: reqwest::Client,
    endpoint: String,
    session_id: Option<String>,
    next_id: u64,
}

impl Browser {
    /// Connect to the browseros-neo MCP server and complete the handshake.
    pub async fn connect(endpoint: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .context("Failed to build HTTP client for browseros-neo")?;

        let mut browser = Browser {
            client,
            endpoint: endpoint.to_string(),
            session_id: None,
            next_id: 0,
        };

        let result = browser
            .request(
                "initialize",
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "switchpost-cli", "version": env!("CARGO_PKG_VERSION")},
                }),
            )
            .await?;
        if let Some(error) = result.get("error") {
            return Err(anyhow!("browseros-neo initialize failed: {error}"));
        }
        browser.notify("notifications/initialized", json!({})).await?;
        Ok(browser)
    }

    /// Call one browseros-neo tool and return its RAW result object
    /// (the `result` member of the JSON-RPC response).
    pub async fn call(&mut self, tool: &str, args: Value) -> Result<Value> {
        let result = self
            .request("tools/call", json!({"name": tool, "arguments": args}))
            .await?;
        if let Some(error) = result.get("error") {
            return Err(anyhow!("browseros-neo tool '{tool}' failed: {error}"));
        }
        // The JSON-RPC result is `{content: [...], structuredContent: {...}}`.
        // The MCP spec puts tool errors in `isError`, not in `error`.
        let inner = result
            .get("result")
            .cloned()
            .unwrap_or(result);
        if inner.get("isError").and_then(Value::as_bool) == Some(true) {
            return Err(anyhow!(
                "browseros-neo tool '{tool}' reported an error: {}",
                tool_text(&inner)
            ));
        }
        Ok(inner)
    }

    /// The text of a tool result — the concatenation of its content blocks.
    pub fn text(result: &Value) -> String {
        tool_text(result)
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let payload = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let (headers, body) = self.post(payload).await?;
        if let Some(session) = headers.get("mcp-session-id") {
            self.session_id = Some(session.to_str().unwrap_or_default().to_string());
        }
        parse_sse_response(&body)
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let payload = json!({"jsonrpc": "2.0", "method": method, "params": params});
        self.post(payload).await?;
        Ok(())
    }

    async fn post(
        &self,
        payload: Value,
    ) -> Result<(reqwest::header::HeaderMap, String)> {
        let mut req = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&payload);
        if let Some(session) = &self.session_id {
            req = req.header("mcp-session-id", session);
        }
        let response = req
            .send()
            .await
            .with_context(|| format!("Failed to reach browseros-neo at {}", self.endpoint))?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(anyhow!(
                "browseros-neo returned HTTP {status}: {}",
                body.chars().take(300).collect::<String>()
            ));
        }
        Ok((headers, body))
    }
}

/// Concatenate the text of every content block in a tool result.
fn tool_text(result: &Value) -> String {
    result
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|b| b.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

/// Pull the JSON-RPC payload out of an SSE response body.
///
/// The server answers with `text/event-stream`; each event's payload is a
/// `data:` line. We take the LAST line that carries a `result` or an
/// `error`, because a session can emit progress notifications first.
fn parse_sse_response(body: &str) -> Result<Value> {
    let mut last: Option<Value> = None;
    for line in body.lines() {
        let Some(payload) = line.strip_prefix("data:") else {
            continue;
        };
        let payload = payload.trim();
        if payload.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(payload) {
            if value.get("result").is_some() || value.get("error").is_some() {
                last = Some(value);
            }
        }
    }
    last.ok_or_else(|| anyhow!("browseros-neo returned no JSON-RPC result: {body}"))
}

/// Strip the `[UNTRUSTED_PAGE_CONTENT ...]` wrapper browseros-neo puts
/// around an evaluated value.
pub fn strip_untrusted(text: &str) -> &str {
    const START: &str = "[UNTRUSTED_PAGE_CONTENT";
    const END: &str = "[END_UNTRUSTED_PAGE_CONTENT]";
    let Some(start) = text.find(START) else {
        return text.trim();
    };
    let after = &text[start..];
    let body_start = after.find(']').map(|i| i + 1).unwrap_or(START.len());
    let body = &after[body_start..];
    match body.find(END) {
        Some(i) => body[..i].trim(),
        None => body.trim(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_response_takes_the_last_result() {
        let body = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"a\":1}}\n\nevent: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"a\":2}}\n\n";
        let parsed = parse_sse_response(body).unwrap();
        assert_eq!(parsed["result"]["a"], 2);
    }

    #[test]
    fn sse_response_reports_an_error_payload() {
        let body = "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"code\":-32601,\"message\":\"nope\"}}\n";
        let parsed = parse_sse_response(body).unwrap();
        assert!(parsed.get("error").is_some());
    }

    #[test]
    fn untrusted_wrapper_is_stripped() {
        let wrapped = "[UNTRUSTED_PAGE_CONTENT]\n{\"href\":\"https://x.test\"}\n[END_UNTRUSTED_PAGE_CONTENT]";
        assert_eq!(strip_untrusted(wrapped), "{\"href\":\"https://x.test\"}");
    }

    #[test]
    fn plain_text_is_returned_unchanged() {
        assert_eq!(strip_untrusted("  opened page 7  "), "opened page 7");
    }
}
