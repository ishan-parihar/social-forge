// ─── Shared Service Layer ─────────────────────────────────────
// Business logic shared between HTTP API handlers and MCP tools.
// Eliminates code duplication by providing a single entry point
// for post CRUD, integration management, and calendar operations.

pub mod content_splitter;
pub mod integrations;
pub mod media_transform;
pub mod notifications;
pub mod posts;
pub mod short_link;
pub mod staging;
pub mod telegram_client;
pub mod telegram_rate;
pub mod webhook_dispatcher;

pub use integrations::IntegrationService;
pub use posts::PostService;
