// ─── MCP Bridge (stub) ─────────────────────────────────────────
// The previous 2,542-LOC hand-written match dispatch was deleted as
// part of the ponytail-audit. The rmcp 1.8 ServerHandler trait
// requires a RequestContext that's hard to construct in-process, so
// the CLI `mcp-call` / `mcp-tools` commands are temporarily disabled.
//
// To call an MCP tool from the CLI, use `social-forge mcp` to start
// the stdio server and talk to it via a real MCP client. The rmcp
// server itself (SocialForgeMcpServer) is unaffected and fully
// functional — only the CLI bridge shortcut is stubbed.
//
// TODO: re-implement via rmcp's in-process Peer API once stabilized.

use crate::api::AppState;

/// Call an MCP tool by name with a JSON argument string.
///
/// **Currently disabled** — see module docs. Returns an error
/// pointing the user at `social-forge mcp` (the stdio server).
pub async fn call_tool(
    _state: AppState,
    tool_name: &str,
    _args_json: &str,
) -> Result<serde_json::Value, String> {
    Err(format!(
        "CLI mcp-call bridge is disabled. Tool '{tool_name}' is available via the MCP stdio server: run `social-forge mcp` and connect with an MCP client."
    ))
}

/// List all available MCP tools with their descriptions.
///
/// **Currently disabled** — see module docs. Returns an empty list
/// with a note pointing at the stdio server.
pub async fn list_tools(_state: AppState) -> Result<Vec<(String, String)>, String> {
    // Return an empty list rather than erroring — `social-forge mcp-tools`
    // is informational and shouldn't crash. The note is logged.
    tracing::warn!(
        "CLI mcp-tools bridge is disabled. Run `social-forge mcp` to list tools via the stdio server."
    );
    Ok(Vec::new())
}
