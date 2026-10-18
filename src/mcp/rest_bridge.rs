// ─── MCP ↔ REST bridge ──────────────────────────────────────────
// MCP tools for REST-only resources call the axum handler in-process, so the
// SQL and validation live in one place. The tool layer only converts types.

use serde::Serialize;

use crate::error::AppError;

/// Turn a REST handler result into an MCP JSON result. Errors keep the REST
/// message, so the agent sees the same text a browser would.
pub(crate) fn rest_result<T: Serialize>(
    result: Result<axum::Json<T>, AppError>,
) -> Result<rmcp::Json<serde_json::Value>, String> {
    let axum::Json(body) = result.map_err(|e| e.to_string())?;
    serde_json::to_value(body)
        .map(rmcp::Json)
        .map_err(|e| format!("Failed to encode response: {e}"))
}

/// Parse a UUID tool argument, naming the field on failure.
pub(crate) fn parse_id(field: &str, raw: &str) -> Result<uuid::Uuid, String> {
    uuid::Uuid::parse_str(raw).map_err(|_| format!("{field} must be a UUID, got `{raw}`"))
}
