// ─── MCP Server ───────────────────────────────────────────────
// Model Context Protocol server exposing all Postiz operations as tools.
// Designed for AI agents to schedule, manage, and monitor posts.
//
// Uses rmcp crate with ServerHandler + #[tool_router] pattern.
// Same business logic as the REST API — shared via AppState.

use rmcp::{
    ServiceExt,
    handler::server::wrapper::Parameters,
    schemars::JsonSchema,
    tool, tool_router,
    transport::stdio,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::jwt;
use crate::db::queries;


mod tools_calendar;
mod tools_integrations;
mod tools_posts;

/// Shared state passed to both API and MCP layers
#[derive(Clone)]
pub struct McpState {
    pub state: AppState,
}

// ══════════════════════════════════════════════════════════════
// AUTH TOOLS
// ══════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LoginOutput {
    pub token: String,
    pub user_id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RegisterInput {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RegisterOutput {
    pub token: String,
    pub user_id: String,
    pub name: String,
}

// ══════════════════════════════════════════════════════════════
// SHARED TYPES
// ══════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SuccessOutput {
    pub success: bool,
    pub message: String,
}

// ══════════════════════════════════════════════════════════════
// TOOL ROUTER
// ══════════════════════════════════════════════════════════════



#[derive(Clone)]
pub struct PostizMcpServer {
    pub state: AppState,
}

// Helper: get DB pool from state


#[tool_router(server_handler)]
impl PostizMcpServer {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    // ── Auth Tools ──────────────────────────────────────────

    #[tool(description = "Register a new account. Returns JWT token for authentication.")]
    async fn auth_register(
        &self,
        params: Parameters<RegisterInput>,
    ) -> Result<Json<RegisterOutput>, String> {
        let input = params.0;
        if input.email.is_empty() || !input.email.contains('@') {
            return Err("Invalid email".into());
        }
        if input.password.len() < 6 {
            return Err("Password must be at least 6 characters".into());
        }

        if queries::get_user_by_email(&self.state.db, &input.email)
            .await
            .map_err(|e| e.to_string())?
            .is_some()
        {
            return Err("Email already registered".into());
        }

        let hash = jwt::hash_password(&input.password).map_err(|e| e.to_string())?;
        let user = queries::create_user(&self.state.db, &input.email, &hash, &input.name)
            .await
            .map_err(|e| e.to_string())?;

        let token = jwt::create_token(user.id, &self.state.config.jwt_secret)
            .map_err(|e| e.to_string())?;

        Ok(Json(RegisterOutput {
            token,
            user_id: user.id.to_string(),
            name: user.name,
        }))
    }

    #[tool(description = "Login with email and password. Returns JWT token.")]
    async fn auth_login(
        &self,
        params: Parameters<LoginInput>,
    ) -> Result<Json<LoginOutput>, String> {
        let input = params.0;
        let user = queries::get_user_by_email(&self.state.db, &input.email)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Invalid email or password".to_string())?;

        let valid = jwt::verify_password(&input.password, &user.password)
            .map_err(|e| e.to_string())?;
        if !valid {
            return Err("Invalid email or password".into());
        }

        let token = jwt::create_token(user.id, &self.state.config.jwt_secret)
            .map_err(|e| e.to_string())?;

        Ok(Json(LoginOutput {
            token,
            user_id: user.id.to_string(),
            name: user.name,
        }))
    }

    #[tool(description = "Get user info from a JWT token")]
    async fn auth_me(
        &self,
        params: Parameters<MeInput>,
    ) -> Result<Json<MeOutput>, String> {
        let claims = jwt::validate_token(&params.0.token, &self.state.config.jwt_secret)
            .map_err(|e| format!("Invalid token: {e}"))?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| "Invalid user ID in token".to_string())?;

        let user = queries::get_user_by_id(&self.state.db, user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "User not found".to_string())?;

        Ok(Json(MeOutput {
            user_id: user.id.to_string(),
            email: user.email,
            name: user.name,
        }))
    }

    // ── Calendar Tools ──────────────────────────────────────

    #[tool(description = "Get posts for a date range (for content calendar)")]
    async fn calendar_get(
        &self,
        params: Parameters<tools_calendar::CalendarInput>,
    ) -> Result<Json<tools_calendar::CalendarOutput>, String> {
        tools_calendar::get_calendar(&self.state, &params.0).await
    }

    // ── Integration Tools ────────────────────────────────────

    #[tool(description = "List all connected social media channels")]
    async fn integrations_list(
        &self,
        params: Parameters<tools_integrations::ListIntegrationsInput>,
    ) -> Result<Json<tools_integrations::ListIntegrationsOutput>, String> {
        tools_integrations::list_integrations(&self.state, &params.0).await
    }

    #[tool(description = "Get OAuth URL to connect a social media channel")]
    async fn integrations_connect(
        &self,
        params: Parameters<tools_integrations::ConnectInput>,
    ) -> Result<Json<tools_integrations::ConnectOutput>, String> {
        tools_integrations::connect_integration(&self.state, &params.0).await
    }

    #[tool(description = "Disconnect/remove a social media channel")]
    async fn integrations_disconnect(
        &self,
        params: Parameters<tools_integrations::DisconnectInput>,
    ) -> Result<Json<SuccessOutput>, String> {
        tools_integrations::disconnect_integration(&self.state, &params.0).await
    }

    // ── Post Tools ───────────────────────────────────────────

    #[tool(description = "Create a new post. Set scheduled_at to auto-schedule. Returns post ID and state.")]
    async fn posts_create(
        &self,
        params: Parameters<tools_posts::CreatePostInput>,
    ) -> Result<Json<tools_posts::CreatePostOutput>, String> {
        tools_posts::create_post(&self.state, &params.0).await
    }

    #[tool(description = "List posts with optional state filter (draft|queued|published|error)")]
    async fn posts_list(
        &self,
        params: Parameters<tools_posts::ListPostsInput>,
    ) -> Result<Json<tools_posts::ListPostsOutput>, String> {
        tools_posts::list_posts(&self.state, &params.0).await
    }

    #[tool(description = "Get a single post by ID")]
    async fn posts_get(
        &self,
        params: Parameters<tools_posts::GetPostInput>,
    ) -> Result<Json<tools_posts::GetPostOutput>, String> {
        tools_posts::get_post(&self.state, &params.0).await
    }

    #[tool(description = "Schedule a post for publishing at a specific time")]
    async fn posts_schedule(
        &self,
        params: Parameters<tools_posts::SchedulePostInput>,
    ) -> Result<Json<tools_posts::SchedulePostOutput>, String> {
        tools_posts::schedule_post(&self.state, &params.0).await
    }

    #[tool(description = "Delete a post by ID")]
    async fn posts_delete(
        &self,
        params: Parameters<tools_posts::DeletePostInput>,
    ) -> Result<Json<SuccessOutput>, String> {
        tools_posts::delete_post(&self.state, &params.0).await
    }

    #[tool(description = "Find the next available free time slot for scheduling")]
    async fn posts_find_slot(
        &self,
        params: Parameters<tools_posts::FindSlotInput>,
    ) -> Result<Json<tools_posts::FindSlotOutput>, String> {
        tools_posts::find_slot(&self.state, &params.0).await
    }

    #[tool(description = "Update a post's content, title, media, or settings by ID")]
    async fn posts_update(
        &self,
        params: Parameters<tools_posts::UpdatePostInput>,
    ) -> Result<Json<tools_posts::UpdatePostOutput>, String> {
        tools_posts::update_post(&self.state, &params.0).await
    }
}

// ══════════════════════════════════════════════════════════════
// RUNNER
// ══════════════════════════════════════════════════════════════

/// Start the MCP server on stdio (for AI clients that spawn the binary)
pub async fn run_mcp_stdio(state: AppState) -> anyhow::Result<()> {
    let server = PostizMcpServer::new(state);
    let service = server.serve(stdio()).await?;
    tracing::info!("MCP server started on stdio");
    service.waiting().await?;
    Ok(())
}

// ── Helper Input Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MeInput {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MeOutput {
    pub user_id: String,
    pub email: String,
    pub name: String,
}
