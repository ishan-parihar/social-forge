use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;

use super::AppState;

// ── Types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Team {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamMember {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamMemberWithUser {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    pub email: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamInvitation {
    pub id: Uuid,
    pub team_id: Uuid,
    pub email: String,
    pub role: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamWithMemberCount {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub member_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberResponse {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub name: String,
    pub role: String,
    pub joined_at: String,
}

impl From<TeamMemberWithUser> for TeamMemberResponse {
    fn from(m: TeamMemberWithUser) -> Self {
        Self {
            id: m.id,
            team_id: m.team_id,
            user_id: m.user_id,
            email: m.email,
            name: m.name,
            role: m.role,
            joined_at: m.joined_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TeamResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub owner_id: Uuid,
    pub created_at: String,
    pub updated_at: String,
    pub member_count: i64,
}

impl From<TeamWithMemberCount> for TeamResponse {
    fn from(t: TeamWithMemberCount) -> Self {
        Self {
            id: t.id,
            name: t.name,
            slug: t.slug,
            owner_id: t.owner_id,
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
            member_count: t.member_count,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub email: String,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptInviteRequest {
    pub token: String,
}

// ── Helpers ──────────────────────────────────────────────────

/// Check if the authenticated user has at least the given role in the team.
/// Returns true if user is owner/admin regardless of minimum_role.
async fn check_team_role(
    db: &sqlx::PgPool,
    team_id: Uuid,
    user_id: Uuid,
    minimum_role: &[&str],
) -> Result<String, AppError> {
    let row = sqlx::query_scalar::<_, String>(
        r#"SELECT role FROM team_members WHERE team_id = $1 AND user_id = $2"#,
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("You are not a member of this team".into()))?;

    let role_str = row.as_str();

    if minimum_role.is_empty() {
        return Ok(role_str.to_string());
    }

    let is_authorized = match role_str {
        "owner" => true,
        "admin" => minimum_role.contains(&"admin") || minimum_role.contains(&"owner"),
        "member" => minimum_role.contains(&"member"),
        "viewer" => minimum_role.contains(&"viewer"),
        _ => false,
    };

    if !is_authorized {
        return Err(AppError::Unauthorized(
            "You do not have the required role for this action".into(),
        ));
    }

    Ok(role_str.to_string())
}

// ── Handlers ─────────────────────────────────────────────────

/// GET /api/teams — list teams where current user is a member
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<TeamResponse>>, AppError> {
    let teams = sqlx::query_as::<_, TeamWithMemberCount>(
        r#"SELECT t.id, t.name, t.slug, t.owner_id, t.created_at, t.updated_at,
                  COALESCE(mc.member_count, 0) AS member_count
           FROM teams t
           INNER JOIN team_members tm ON tm.team_id = t.id
           LEFT JOIN (
               SELECT team_id, COUNT(*)::bigint AS member_count
               FROM team_members
               GROUP BY team_id
           ) mc ON mc.team_id = t.id
           WHERE tm.user_id = $1
           ORDER BY t.created_at DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(teams.into_iter().map(TeamResponse::from).collect()))
}

/// POST /api/teams — create a new team
pub async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(input): Json<CreateTeamRequest>,
) -> Result<Json<TeamResponse>, AppError> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Team name cannot be empty".into()));
    }
    let slug = input.slug.trim().to_string();
    if slug.is_empty() {
        return Err(AppError::BadRequest("Team slug cannot be empty".into()));
    }
    if !slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(AppError::BadRequest("Slug must be lowercase alphanumeric with hyphens only".into()));
    }

    let mut tx = state.db.begin().await?;

    let team = sqlx::query_as::<_, Team>(
        r#"INSERT INTO teams (name, slug, owner_id)
           VALUES ($1, $2, $3)
           RETURNING id, name, slug, owner_id, created_at, updated_at"#,
    )
    .bind(&name)
    .bind(&slug)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("teams_slug_key") {
                return AppError::Conflict("A team with this slug already exists".into());
            }
        }
        AppError::from(e)
    })?;

    sqlx::query(
        r#"INSERT INTO team_members (team_id, user_id, role)
           VALUES ($1, $2, 'owner')"#,
    )
    .bind(team.id)
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(TeamResponse {
        id: team.id,
        name: team.name,
        slug: team.slug,
        owner_id: team.owner_id,
        created_at: team.created_at.to_rfc3339(),
        updated_at: team.updated_at.to_rfc3339(),
        member_count: 1,
    }))
}

/// GET /api/teams/{id} — get team details
pub async fn get(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<TeamResponse>, AppError> {
    // Verify membership
    check_team_role(&state.db, id, auth.user_id, &[]).await?;

    let team = sqlx::query_as::<_, TeamWithMemberCount>(
        r#"SELECT t.id, t.name, t.slug, t.owner_id, t.created_at, t.updated_at,
                  COALESCE(mc.member_count, 0) AS member_count
           FROM teams t
           LEFT JOIN (
               SELECT team_id, COUNT(*)::bigint AS member_count
               FROM team_members
               GROUP BY team_id
           ) mc ON mc.team_id = t.id
           WHERE t.id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Team not found".into()))?;

    Ok(Json(TeamResponse::from(team)))
}

/// PUT /api/teams/{id} — update team (owner/admin only)
pub async fn update(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateTeamRequest>,
) -> Result<Json<TeamResponse>, AppError> {
    check_team_role(&state.db, id, auth.user_id, &["owner", "admin"]).await?;

    let name = input.name.as_deref();
    let slug = input.slug.as_deref();

    if let Some(slug) = slug {
        if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(AppError::BadRequest("Slug must be lowercase alphanumeric with hyphens only".into()));
        }
    }

    let team = sqlx::query_as::<_, Team>(
        r#"UPDATE teams SET
              name = COALESCE($3, name),
              slug = COALESCE($4, slug),
              updated_at = now()
           WHERE id = $1
           RETURNING id, name, slug, owner_id, created_at, updated_at"#,
    )
    .bind(id)
    .bind(name)
    .bind(slug)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("teams_slug_key") {
                return AppError::Conflict("A team with this slug already exists".into());
            }
        }
        AppError::from(e)
    })?;

    // Get member count
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint FROM team_members WHERE team_id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(TeamResponse {
        id: team.id,
        name: team.name,
        slug: team.slug,
        owner_id: team.owner_id,
        created_at: team.created_at.to_rfc3339(),
        updated_at: team.updated_at.to_rfc3339(),
        member_count: count.0,
    }))
}

/// DELETE /api/teams/{id} — delete team (owner only)
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    check_team_role(&state.db, id, auth.user_id, &["owner"]).await?;

    sqlx::query("DELETE FROM teams WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"deleted": true})))
}

/// POST /api/teams/{id}/invite — create invitation
pub async fn invite(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(input): Json<InviteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    check_team_role(&state.db, id, auth.user_id, &["owner", "admin"]).await?;

    let email = input.email.trim().to_string();
    if email.is_empty() {
        return Err(AppError::BadRequest("Email cannot be empty".into()));
    }

    let role = input.role.as_deref().unwrap_or("member");
    if !["admin", "member", "viewer"].contains(&role) {
        return Err(AppError::BadRequest(
            "Role must be one of: admin, member, viewer".into(),
        ));
    }

    // Check if the invited user is already a member
    let invited_user_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM users WHERE email = $1",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await?;

    if let Some(invited_user_id) = invited_user_id {
        let already_member: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
        )
        .bind(id)
        .bind(invited_user_id)
        .fetch_one(&state.db)
        .await?;

        if already_member {
            return Err(AppError::Conflict("This user is already a member of the team".into()));
        }
    }

    let token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::Duration::days(7);

    let invitation = sqlx::query_as::<_, TeamInvitation>(
        r#"INSERT INTO team_invitations (team_id, email, role, token, expires_at)
           VALUES ($1, $2, $3::team_role, $4, $5)
           RETURNING id, team_id, email, role::text, token, expires_at, created_at, accepted_at"#,
    )
    .bind(id)
    .bind(&email)
    .bind(role)
    .bind(&token)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "id": invitation.id,
        "team_id": invitation.team_id,
        "email": invitation.email,
        "role": invitation.role,
        "token": invitation.token,
        "expires_at": invitation.expires_at.to_rfc3339(),
        "created_at": invitation.created_at.to_rfc3339(),
    })))
}

/// POST /api/teams/accept — accept invitation by token
pub async fn accept_invite(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(input): Json<AcceptInviteRequest>,
) -> Result<Json<TeamMemberResponse>, AppError> {
    let token = input.token.trim().to_string();
    if token.is_empty() {
        return Err(AppError::BadRequest("Token cannot be empty".into()));
    }

    let invitation = sqlx::query_as::<_, TeamInvitation>(
        r#"SELECT id, team_id, email, role::text, token, expires_at, created_at, accepted_at
           FROM team_invitations WHERE token = $1"#,
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Invitation not found".into()))?;

    if invitation.accepted_at.is_some() {
        return Err(AppError::BadRequest("Invitation has already been accepted".into()));
    }

    if invitation.expires_at < Utc::now() {
        return Err(AppError::BadRequest("Invitation has expired".into()));
    }

    // Verify the authenticated user's email matches the invitation email
    let user_email: Option<(String,)> = sqlx::query_as(
        "SELECT email FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?;

    let user_email = user_email
        .map(|r| r.0)
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    if user_email.to_lowercase() != invitation.email.to_lowercase() {
        return Err(AppError::Unauthorized(
            "This invitation was sent to a different email address".into(),
        ));
    }

    let mut tx = state.db.begin().await?;

    let member = sqlx::query_as::<_, TeamMemberWithUser>(
        r#"INSERT INTO team_members (team_id, user_id, role)
           VALUES ($1, $2, $3::team_role)
           RETURNING
               id, team_id, user_id,
               role::text,
               joined_at,
               (SELECT email FROM users WHERE id = $2) AS email,
               (SELECT name FROM users WHERE id = $2) AS name"#,
    )
    .bind(invitation.team_id)
    .bind(auth.user_id)
    .bind(&invitation.role)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("team_members_team_id_user_id_key") {
                return AppError::Conflict("You are already a member of this team".into());
            }
        }
        AppError::from(e)
    })?;

    sqlx::query(
        "UPDATE team_invitations SET accepted_at = now() WHERE id = $1",
    )
    .bind(invitation.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(TeamMemberResponse::from(member)))
}

/// GET /api/teams/{id}/members — list team members
pub async fn members(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<TeamMemberResponse>>, AppError> {
    check_team_role(&state.db, id, auth.user_id, &[]).await?;

    let members = sqlx::query_as::<_, TeamMemberWithUser>(
        r#"SELECT tm.id, tm.team_id, tm.user_id, tm.role::text, tm.joined_at,
                  u.email, u.name
           FROM team_members tm
           INNER JOIN users u ON u.id = tm.user_id
           WHERE tm.team_id = $1
           ORDER BY
               CASE tm.role
                   WHEN 'owner' THEN 0
                   WHEN 'admin' THEN 1
                   WHEN 'member' THEN 2
                   WHEN 'viewer' THEN 3
               END,
               u.name ASC"#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(members.into_iter().map(TeamMemberResponse::from).collect()))
}

/// DELETE /api/teams/{id}/members/{user_id} — remove member
pub async fn remove_member(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((team_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let caller_role = check_team_role(&state.db, team_id, auth.user_id, &["owner", "admin"]).await?;

    if user_id == auth.user_id {
        return Err(AppError::BadRequest("You cannot remove yourself. Use team delete or transfer ownership.".into()));
    }

    let target_role = sqlx::query_scalar::<_, String>(
        r#"SELECT role::text FROM team_members WHERE team_id = $1 AND user_id = $2"#,
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Member not found".into()))?;

    if target_role == "owner" {
        return Err(AppError::BadRequest("Cannot remove the team owner".into()));
    }

    if caller_role == "admin" && target_role == "admin" {
        return Err(AppError::Unauthorized(
            "Admins cannot remove other admins".into(),
        ));
    }

    sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
        .bind(team_id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"deleted": true})))
}
