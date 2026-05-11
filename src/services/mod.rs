// ─── Shared Service Layer ─────────────────────────────────────
// Business logic shared between HTTP API handlers and MCP tools.
// Eliminates code duplication by providing a single entry point
// for post CRUD, integration management, and calendar operations.

pub mod posts;
pub mod integrations;
pub mod telegram_daemon;
pub mod whatsapp_daemon;

pub use posts::PostService;
pub use integrations::IntegrationService;
