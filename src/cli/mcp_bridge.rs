// ─── MCP Bridge ───────────────────────────────────────────────
// Thin in-process adapter that lets the CLI (`social-forge mcp-call
// <tool> --args '<json>'` and `social-forge mcp-tools`) share the
// exact same tool dispatch as the rmcp stdio server.
//
// Replaces the previous 2,542-LOC hand-written match dispatch with
// a delegation to `SocialForgeMcpServer` (whose `#[tool_router]`
// already encodes the same routing). New `#[tool]` methods are now
// automatically CLI-accessible without editing two parallel tables.

use rmcp::ServerHandler;
use rmcp::model::{CallToolRequestParam, CallToolResult, Content, RawContent};

use crate::api::AppState;
use crate::mcp::SocialForgeMcpServer;

/// Call an MCP tool by name with a JSON argument string.
/// Returns the tool's JSON output (same contract as the old bridge).
pub async fn call_tool(
    state: AppState,
    tool_name: &str,
    args_json: &str,
) -> Result<serde_json::Value, String> {
    // Single-user mode — no register/login/auth_me over MCP.
    // `auth_status` is allowed; everything else goes through the
    // rmcp server's own dispatch.
    if matches!(tool_name, "auth_register" | "auth_login" | "auth_me") {
        return Err(
            "Single-user mode — no register/login/auth_me over MCP. Use `auth_status` instead."
                .to_string(),
        );
    }

    let args_value: serde_json::Value = if args_json.is_empty() || args_json == "{}" {
        serde_json::json!({})
    } else {
        serde_json::from_str(args_json).map_err(|e| format!("Invalid JSON arguments: {e}"))?
    };

    // rmcp expects CallToolRequestParam { arguments: Option<Map<String, Value>> }.
    // Non-object JSON (arrays/scalars) is rejected — matches the old bridge,
    // which always deserialized into struct inputs (i.e. objects).
    let arguments = match args_value {
        serde_json::Value::Object(map) => Some(map),
        _ => return Err(format!("Tool arguments must be a JSON object, got: {args_json}")),
    };

    let server = SocialForgeMcpServer::new(state);
    let result: CallToolResult = server
        .call_tool(tool_name, CallToolRequestParam { arguments })
        .await
        .map_err(|e| format!("MCP call failed: {e}"))?;

    // rmcp wraps tool-internal `Err(String)` as `CallToolResult { is_error: true, ... }`
    // rather than returning `Err` from `call_tool`. Preserve old bridge semantics
    // by converting `is_error: true` back into an `Err`.
    if result.is_error {
        let msg = extract_text(&result.content);
        return Err(msg.unwrap_or_else(|| "Tool returned an error".into()));
    }

    // rmcp serializes the tool's `Json<T>` return value as a `Text` content item
    // containing the JSON string. Parse it back into a `serde_json::Value` to
    // preserve the old bridge's return type.
    let text = extract_text(&result.content)
        .ok_or_else(|| "Tool returned no text content".to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({ "text": text }));
    Ok(value)
}

/// List all available MCP tools with their descriptions.
pub async fn list_tools(state: AppState) -> Result<Vec<(String, String)>, String> {
    let server = SocialForgeMcpServer::new(state);
    let result = server
        .list_tools()
        .await
        .map_err(|e| format!("MCP list_tools failed: {e}"))?;
    Ok(result
        .tools
        .into_iter()
        .map(|t| (t.name, t.description.unwrap_or_default()))
        .collect())
}

fn extract_text(content: &[Content]) -> Option<String> {
    content.iter().find_map(|c| match c {
        RawContent::Text(t) => Some(t.text.clone()),
        _ => None,
    })
}
