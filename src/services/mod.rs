// ─── Shared Service Layer ─────────────────────────────────────
// Business logic shared between HTTP API handlers and MCP tools.
// Eliminates code duplication by providing a single entry point
// for post CRUD, integration management, and calendar operations.

pub mod notifications;
pub mod posts;
pub mod integrations;
pub mod telegram_client;
pub mod webhook_dispatcher;
pub mod content_splitter;
pub mod staging;
pub mod short_link;
pub mod plugs;

pub use posts::PostService;
pub use integrations::IntegrationService;
