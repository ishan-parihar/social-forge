use axum::{
    body::Bytes,
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;

use super::AppState;

// ── Models ────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub stripe_subscription_id: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub plan: String,
    pub status: String,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    pub plan: String,
    pub interval: String,
    pub success_url: String,
    pub cancel_url: String,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    pub subscription: Subscription,
    pub plan_name: String,
    pub plan_features: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct InvoiceListResponse {
    pub invoices: Vec<Invoice>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Invoice {
    pub id: Uuid,
    pub user_id: Uuid,
    pub subscription_id: Option<Uuid>,
    pub stripe_invoice_id: Option<String>,
    pub amount: i32,
    pub currency: String,
    pub status: String,
    pub invoice_url: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct InvoiceQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ── Plan features ─────────────────────────────────────────────

fn plan_features(plan: &str) -> Vec<String> {
    match plan {
        "pro" => vec![
            "Up to 15 social channels".into(),
            "Advanced analytics".into(),
            "Team members (3)".into(),
            "AI suggestions".into(),
            "RSS autopost".into(),
        ],
        "business" => vec![
            "Unlimited social channels".into(),
            "Premium analytics".into(),
            "Unlimited team members".into(),
            "AI assistant".into(),
            "Priority support".into(),
            "Custom integrations".into(),
        ],
        _ => vec![
            "Up to 5 social channels".into(),
            "Basic analytics".into(),
            "Schedule posts".into(),
        ],
    }
}

fn plan_name(plan: &str) -> &'static str {
    match plan {
        "pro" => "Pro",
        "business" => "Business",
        _ => "Free",
    }
}

// ── Stripe API helpers ────────────────────────────────────────

fn stripe_client(config: &crate::config::Config) -> Result<reqwest::Client, AppError> {
    let secret = config
        .stripe_secret_key
        .as_deref()
        .ok_or_else(|| AppError::Internal("Stripe secret key not configured".into()))?;
    let mut headers = reqwest::header::HeaderMap::new();
    let auth = format!("Bearer {}", secret);
    let mut auth_header = reqwest::header::HeaderValue::try_from(&auth)
        .map_err(|e| AppError::Internal(format!("Invalid auth header: {e}")))?;
    auth_header.set_sensitive(true);
    headers.insert(reqwest::header::AUTHORIZATION, auth_header);
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {e}")))
}

async fn stripe_create_customer(
    config: &crate::config::Config,
    user_id: Uuid,
) -> Result<String, AppError> {
    let client = stripe_client(config)?;
    let params = [
        ("metadata[user_id]", user_id.to_string()),
        ("name", format!("User {}", user_id)),
    ];
    let res = client
        .post("https://api.stripe.com/v1/customers")
        .form(&params)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Stripe API error: {e}")))?;
    let status = res.status();
    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Stripe parse error: {e}")))?;
    if !status.is_success() {
        return Err(AppError::Internal(format!(
            "Stripe create customer failed: {}",
            body.get("error")
                .and_then(|e| e.get("message").and_then(|m| m.as_str()))
                .unwrap_or("unknown error")
        )));
    }
    body.get("id")
        .and_then(|id| id.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Internal("Missing customer id in Stripe response".into()))
}

async fn stripe_create_checkout_session(
    config: &crate::config::Config,
    customer_id: &str,
    price_id: &str,
    success_url: &str,
    cancel_url: &str,
    user_id: Uuid,
    plan: &str,
) -> Result<String, AppError> {
    let client = stripe_client(config)?;
    let user_id_str = user_id.to_string();
    let params: Vec<(&str, &str)> = vec![
        ("mode", "subscription"),
        ("customer", customer_id),
        ("line_items[0][price]", price_id),
        ("line_items[0][quantity]", "1"),
        ("success_url", success_url),
        ("cancel_url", cancel_url),
        ("metadata[user_id]", &user_id_str),
        ("metadata[plan]", plan),
        ("allow_promotion_codes", "true"),
    ];
    let res = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .form(&params)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Stripe API error: {e}")))?;
    let status = res.status();
    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Stripe parse error: {e}")))?;
    if !status.is_success() {
        return Err(AppError::Internal(format!(
            "Stripe checkout session failed: {}",
            body.get("error")
                .and_then(|e| e.get("message").and_then(|m| m.as_str()))
                .unwrap_or("unknown error")
        )));
    }
    body.get("url")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Internal("Missing url in Stripe checkout response".into()))
}

async fn stripe_create_portal_session(
    config: &crate::config::Config,
    customer_id: &str,
    return_url: &str,
) -> Result<String, AppError> {
    let client = stripe_client(config)?;
    let params = [
        ("customer", customer_id),
        ("return_url", return_url),
    ];
    let res = client
        .post("https://api.stripe.com/v1/billing_portal/sessions")
        .form(&params)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Stripe API error: {e}")))?;
    let status = res.status();
    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Stripe parse error: {e}")))?;
    if !status.is_success() {
        return Err(AppError::Internal(format!(
            "Stripe portal session failed: {}",
            body.get("error")
                .and_then(|e| e.get("message").and_then(|m| m.as_str()))
                .unwrap_or("unknown error")
        )));
    }
    body.get("url")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Internal("Missing url in Stripe portal response".into()))
}

// ── Webhook verification (manual HMAC-SHA256) ─────────────────

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut k = key.to_vec();
    if k.len() > BLOCK_SIZE {
        k = Sha256::digest(&k).to_vec();
    }
    k.resize(BLOCK_SIZE, 0);
    let mut ipad = vec![0x36u8; BLOCK_SIZE];
    let mut opad = vec![0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let mut inner_input = ipad;
    inner_input.extend_from_slice(data);
    let inner = Sha256::digest(&inner_input);
    let mut outer_input = opad;
    outer_input.extend_from_slice(&inner);
    let result = Sha256::digest(&outer_input);
    result.into()
}

fn verify_webhook_signature(
    payload: &[u8],
    sig_header: &str,
    secret: &str,
) -> Result<serde_json::Value, AppError> {
    let parts: Vec<&str> = sig_header.split(',').collect();
    let mut timestamp = String::new();
    let mut signature = String::new();
    for part in parts {
        if let Some(t) = part.strip_prefix("t=") {
            timestamp = t.to_string();
        } else if let Some(s) = part.strip_prefix("v1=") {
            signature = s.to_string();
        }
    }
    if timestamp.is_empty() || signature.is_empty() {
        return Err(AppError::BadRequest("Invalid Stripe signature format".into()));
    }
    // Verify timestamp is within tolerance (5 minutes)
    let ts_secs: i64 = timestamp.parse().map_err(|_| AppError::BadRequest("Invalid webhook timestamp".into()))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    if (now - ts_secs).abs() > 300 {
        return Err(AppError::BadRequest("Webhook timestamp outside tolerance window".into()));
    }
    let signed_payload = format!("{}.{}", timestamp, std::str::from_utf8(payload).unwrap_or(""));
    let expected = hex::encode(hmac_sha256(secret.as_bytes(), signed_payload.as_bytes()));
    if expected != signature {
        return Err(AppError::BadRequest("Invalid webhook signature".into()));
    }
    serde_json::from_slice(payload)
        .map_err(|e| AppError::BadRequest(format!("Invalid webhook payload: {e}")))
}

// ── Query helpers ─────────────────────────────────────────────

async fn get_subscription_for_user(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> Result<Option<Subscription>, AppError> {
    sqlx::query_as::<_, Subscription>(
        "SELECT id, user_id, stripe_subscription_id, stripe_customer_id, plan::text, status::text, current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at FROM subscriptions WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e))
}

async fn create_default_subscription(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> Result<Subscription, AppError> {
    sqlx::query_as::<_, Subscription>(
        "INSERT INTO subscriptions (user_id, plan, status) VALUES ($1, 'free', 'active') RETURNING id, user_id, stripe_subscription_id, stripe_customer_id, plan::text, status::text, current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e))
}

async fn upsert_subscription(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    stripe_subscription_id: &str,
    stripe_customer_id: &str,
    plan: &str,
    status: &str,
    period_start: Option<DateTime<Utc>>,
    period_end: Option<DateTime<Utc>>,
) -> Result<Subscription, AppError> {
    sqlx::query_as::<_, Subscription>(
        r#"INSERT INTO subscriptions (user_id, stripe_subscription_id, stripe_customer_id, plan, status, current_period_start, current_period_end)
        VALUES ($1, $2, $3, $4::subscription_plan, $5::subscription_status, $6, $7)
        ON CONFLICT (user_id)
        DO UPDATE SET
            stripe_subscription_id = EXCLUDED.stripe_subscription_id,
            stripe_customer_id = EXCLUDED.stripe_customer_id,
            plan = EXCLUDED.plan,
            status = EXCLUDED.status,
            current_period_start = EXCLUDED.current_period_start,
            current_period_end = EXCLUDED.current_period_end,
            updated_at = NOW()
        RETURNING id, user_id, stripe_subscription_id, stripe_customer_id, plan::text, status::text, current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at"#,
    )
    .bind(user_id)
    .bind(stripe_subscription_id)
    .bind(stripe_customer_id)
    .bind(plan)
    .bind(status)
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e))
}

async fn update_subscription_status(
    pool: &sqlx::PgPool,
    stripe_subscription_id: &str,
    status: &str,
    period_start: Option<DateTime<Utc>>,
    period_end: Option<DateTime<Utc>>,
    cancel_at_period_end: bool,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE subscriptions SET status = $1::subscription_status, current_period_start = $2, current_period_end = $3, cancel_at_period_end = $4, updated_at = NOW() WHERE stripe_subscription_id = $5",
    )
    .bind(status)
    .bind(period_start)
    .bind(period_end)
    .bind(cancel_at_period_end)
    .bind(stripe_subscription_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(e))?;
    Ok(())
}

async fn insert_invoice(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    subscription_id: Option<Uuid>,
    stripe_invoice_id: &str,
    amount: i32,
    currency: &str,
    status: &str,
    invoice_url: Option<&str>,
    paid_at: Option<DateTime<Utc>>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO invoices (user_id, subscription_id, stripe_invoice_id, amount, currency, status, invoice_url, paid_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(user_id)
    .bind(subscription_id)
    .bind(stripe_invoice_id)
    .bind(amount)
    .bind(currency)
    .bind(status)
    .bind(invoice_url)
    .bind(paid_at)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(e))?;
    Ok(())
}

async fn set_subscription_plan_free(
    pool: &sqlx::PgPool,
    stripe_subscription_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE subscriptions SET plan = 'free', status = 'canceled', updated_at = NOW() WHERE stripe_subscription_id = $1",
    )
    .bind(stripe_subscription_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(e))?;
    Ok(())
}

// ── Handlers ──────────────────────────────────────────────────
// Note: create_checkout_session, get_subscription, get_invoices, and
// create_portal_session are kept for potential future Stripe billing
// integration but are NOT mounted in the router (single-user mode).
// Only stripe_webhook is actively routed.

/// POST /api/billing/create-checkout
#[allow(dead_code)]
pub async fn create_checkout_session(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Json(body): Json<CreateCheckoutRequest>,
) -> Result<Json<CheckoutResponse>, AppError> {
    let sub = match get_subscription_for_user(&state.db, auth.user_id).await? {
        Some(s) => s,
        None => create_default_subscription(&state.db, auth.user_id).await?,
    };

    let customer_id = match sub.stripe_customer_id {
        Some(ref cid) => cid.clone(),
        None => {
            let cid = stripe_create_customer(&state.config, auth.user_id).await?;
            sqlx::query("UPDATE subscriptions SET stripe_customer_id = $1, updated_at = NOW() WHERE id = $2")
                .bind(&cid)
                .bind(sub.id)
                .execute(&state.db)
                .await
                .map_err(|e| AppError::Database(e))?;
            cid
        }
    };

    let price_id = match (body.plan.as_str(), body.interval.as_str()) {
        ("pro", "monthly") => state.config.stripe_price_pro_monthly.as_deref(),
        ("pro", "annual") => state.config.stripe_price_pro_annual.as_deref(),
        ("business", "monthly") => state.config.stripe_price_business_monthly.as_deref(),
        ("business", "annual") => state.config.stripe_price_business_annual.as_deref(),
        _ => None,
    }
    .ok_or_else(|| {
        AppError::BadRequest(format!(
            "Invalid plan/interval combination: {}/{}",
            body.plan, body.interval
        ))
    })?;

    let url = stripe_create_checkout_session(
        &state.config,
        &customer_id,
        price_id,
        &body.success_url,
        &body.cancel_url,
        auth.user_id,
        &body.plan,
    )
    .await?;

    Ok(Json(CheckoutResponse { url }))
}

/// POST /api/billing/webhook
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    let secret = state
        .config
        .stripe_webhook_secret
        .as_deref()
        .ok_or_else(|| AppError::Internal("Stripe webhook secret not configured".into()))?;

    let sig_header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing stripe-signature header".into()))?;

    let event = verify_webhook_signature(&body, sig_header, secret)?;

    let event_type = event
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("");

    tracing::info!("Stripe webhook received: {}", event_type);

    match event_type {
        "checkout.session.completed" => {
            let session = event
                .get("data")
                .and_then(|d| d.get("object"))
                .ok_or_else(|| AppError::BadRequest("Missing object in event".into()))?;

            let customer_id = session
                .get("customer")
                .and_then(|c| c.as_str())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AppError::BadRequest("Missing customer in checkout session".into()))?;
            let subscription_id = session
                .get("subscription")
                .and_then(|s| s.as_str())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AppError::BadRequest("Missing subscription in checkout session".into()))?;
            let user_id_str = session
                .get("metadata")
                .and_then(|m| m.get("user_id"))
                .and_then(|u| u.as_str())
                .ok_or_else(|| AppError::BadRequest("Missing user_id in metadata".into()))?;
            let user_id = Uuid::parse_str(user_id_str)
                .map_err(|e| AppError::BadRequest(format!("Invalid user_id: {e}")))?;

            let plan = session
                .get("metadata")
                .and_then(|m| m.get("plan"))
                .and_then(|p| p.as_str())
                .unwrap_or("free");

            upsert_subscription(
                &state.db,
                user_id,
                subscription_id,
                customer_id,
                plan,
                "active",
                None,
                None,
            )
            .await?;

            tracing::info!("Subscription activated for user {}", user_id);
        }
        "invoice.paid" => {
            let invoice = event
                .get("data")
                .and_then(|d| d.get("object"))
                .ok_or_else(|| AppError::BadRequest("Missing object in event".into()))?;

            let invoice_id = invoice
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("");
            let subscription_id = invoice
                .get("subscription")
                .and_then(|s| s.as_str());
            let customer_id = invoice
                .get("customer")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let amount = invoice
                .get("amount_paid")
                .and_then(|a| a.as_i64())
                .unwrap_or(0) as i32;
            let currency = invoice
                .get("currency")
                .and_then(|c| c.as_str())
                .unwrap_or("usd");
            let invoice_url = invoice
                .get("hosted_invoice_url")
                .and_then(|u| u.as_str());
            let status = invoice
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("paid");
            let paid_at = invoice
                .get("paid_at")
                .and_then(|p| p.as_i64())
                .map(|ts| {
                    DateTime::from_timestamp(ts, 0)
                        .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap())
                });

            let period_start = invoice
                .get("lines")
                .and_then(|l| l.get("data"))
                .and_then(|d| d.as_array())
                .and_then(|arr| arr.first())
                .and_then(|line| line.get("period"))
                .and_then(|p| p.get("start"))
                .and_then(|s| s.as_i64())
                .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap()));

            let period_end = invoice
                .get("lines")
                .and_then(|l| l.get("data"))
                .and_then(|d| d.as_array())
                .and_then(|arr| arr.first())
                .and_then(|line| line.get("period"))
                .and_then(|p| p.get("end"))
                .and_then(|e| e.as_i64())
                .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap()));

            let user_id = if let Some(sid) = subscription_id {
                let sub: Option<Subscription> = sqlx::query_as::<_, Subscription>(
                    "SELECT id, user_id, stripe_subscription_id, stripe_customer_id, plan::text, status::text, current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at FROM subscriptions WHERE stripe_subscription_id = $1",
                )
                .bind(sid)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| AppError::Database(e))?;
                sub.map(|s| (s.id, s.user_id))
            } else {
                let sub: Option<Subscription> = sqlx::query_as::<_, Subscription>(
                    "SELECT id, user_id, stripe_subscription_id, stripe_customer_id, plan::text, status::text, current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at FROM subscriptions WHERE stripe_customer_id = $1",
                )
                .bind(customer_id)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| AppError::Database(e))?;
                sub.map(|s| (s.id, s.user_id))
            };

            if let Some((sub_id, user_id)) = user_id {
                insert_invoice(
                    &state.db,
                    user_id,
                    Some(sub_id),
                    invoice_id,
                    amount,
                    currency,
                    status,
                    invoice_url,
                    paid_at,
                )
                .await?;

                if let Some(sid) = subscription_id {
                    update_subscription_status(
                        &state.db,
                        sid,
                        "active",
                        period_start,
                        period_end,
                        false,
                    )
                    .await?;
                }
            }
        }
        "customer.subscription.updated" => {
            let sub_obj = event
                .get("data")
                .and_then(|d| d.get("object"))
                .ok_or_else(|| AppError::BadRequest("Missing object in event".into()))?;

            let sub_id = sub_obj
                .get("id")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            let sub_status = sub_obj
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("active");
            let cancel_at_period_end = sub_obj
                .get("cancel_at_period_end")
                .and_then(|c| c.as_bool())
                .unwrap_or(false);

            let period_start = sub_obj
                .get("current_period_start")
                .and_then(|p| p.as_i64())
                .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap()));

            let period_end = sub_obj
                .get("current_period_end")
                .and_then(|p| p.as_i64())
                .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap()));

            update_subscription_status(
                &state.db,
                sub_id,
                sub_status,
                period_start,
                period_end,
                cancel_at_period_end,
            )
            .await?;
        }
        "customer.subscription.deleted" => {
            let sub_obj = event
                .get("data")
                .and_then(|d| d.get("object"))
                .ok_or_else(|| AppError::BadRequest("Missing object in event".into()))?;

            let sub_id = sub_obj
                .get("id")
                .and_then(|s| s.as_str())
                .unwrap_or("");

            set_subscription_plan_free(&state.db, sub_id).await?;
        }
        _ => {
            tracing::debug!("Unhandled webhook event type: {}", event_type);
        }
    }

    Ok(Json(serde_json::json!({"received": true})))
}

/// GET /api/billing/subscription
#[allow(dead_code)]
pub async fn get_subscription(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<SubscriptionResponse>, AppError> {
    let sub = match get_subscription_for_user(&state.db, auth.user_id).await? {
        Some(s) => s,
        None => create_default_subscription(&state.db, auth.user_id).await?,
    };

    let plan_name = plan_name(&sub.plan).to_string();
    let plan_features = plan_features(&sub.plan);

    Ok(Json(SubscriptionResponse {
        subscription: sub,
        plan_name,
        plan_features,
    }))
}

/// GET /api/billing/invoices
#[allow(dead_code)]
pub async fn get_invoices(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Query(query): Query<InvoiceQuery>,
) -> Result<Json<InvoiceListResponse>, AppError> {
    let limit = query.limit.unwrap_or(10).min(100);
    let offset = query.offset.unwrap_or(0);

    let invoices = sqlx::query_as::<_, Invoice>(
        "SELECT id, user_id, subscription_id, stripe_invoice_id, amount, currency, status, invoice_url, paid_at, created_at FROM invoices WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(auth.user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Database(e))?;

    Ok(Json(InvoiceListResponse { invoices }))
}

/// POST /api/billing/portal-session
#[allow(dead_code)]
pub async fn create_portal_session(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<CheckoutResponse>, AppError> {
    let sub = get_subscription_for_user(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("No subscription found".into()))?;

    let customer_id = sub
        .stripe_customer_id
        .ok_or_else(|| AppError::BadRequest("No Stripe customer ID found".into()))?;

    let return_url = format!("{}/settings/billing", state.config.frontend_url);

    let url = stripe_create_portal_session(&state.config, &customer_id, &return_url).await?;

    Ok(Json(CheckoutResponse { url }))
}
