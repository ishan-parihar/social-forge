use std::path::PathBuf;
use std::sync::Arc;

use grammers_client::types::{LoginToken, PasswordToken};
use grammers_client::{Client, Config, InitParams, SignInError};
use grammers_session::Session;
use serde_json::Value;
use tokio::sync::Mutex;

struct InnerState {
    client: Option<Client>,
    pending_token: Option<LoginToken>,
    pending_password_token: Option<PasswordToken>,
}

pub struct TelegramClientManager {
    inner: Mutex<InnerState>,
    api_id: i32,
    api_hash: String,
    session_path: PathBuf,
}

impl TelegramClientManager {
    pub fn new(api_id: i32, api_hash: String, session_dir: PathBuf) -> Self {
        let session_path = session_dir.join("telegram-user.session");
        Self {
            inner: Mutex::new(InnerState {
                client: None,
                pending_token: None,
                pending_password_token: None,
            }),
            api_id,
            api_hash,
            session_path,
        }
    }

    pub async fn is_authenticated(&self) -> Result<bool, String> {
        let mut guard = self.inner.lock().await;
        Self::ensure_client(&mut guard, self.api_id, &self.api_hash, &self.session_path).await?;
        match guard.client.as_ref() {
            Some(client) => client
                .is_authorized()
                .await
                .map_err(|e| format!("Auth check failed: {e}")),
            None => Ok(false),
        }
    }

    pub async fn request_login_code(&self, phone: &str) -> Result<Value, String> {
        let mut guard = self.inner.lock().await;
        Self::ensure_client(&mut guard, self.api_id, &self.api_hash, &self.session_path).await?;

        let client = guard.client.as_ref().unwrap();
        match client.request_login_code(phone).await {
            Ok(token) => {
                guard.pending_token = Some(token);
                Ok(serde_json::json!({ "status": "code_sent" }))
            }
            Err(e) => {
                let err_str = format!("{e}");
                if err_str.contains("AUTH_RESTART") {
                    // Session corrupted — clear and retry with fresh session
                    tracing::warn!("AUTH_RESTART: clearing Telegram session and retrying");
                    guard.client = None;
                    guard.pending_token = None;
                    let _ = std::fs::remove_file(&self.session_path);
                    Self::ensure_client(&mut guard, self.api_id, &self.api_hash, &self.session_path).await?;
                    let client = guard.client.as_ref().unwrap();
                    let token = client
                        .request_login_code(phone)
                        .await
                        .map_err(|e| format!("Request login code failed after session reset: {e}"))?;
                    guard.pending_token = Some(token);
                    Ok(serde_json::json!({ "status": "code_sent", "note": "session was reset" }))
                } else {
                    Err(format!("Request login code failed: {e}"))
                }
            }
        }
    }

    pub async fn sign_in(&self, _phone: &str, code: &str) -> Result<Value, String> {
        let mut guard = self.inner.lock().await;
        Self::ensure_client(&mut guard, self.api_id, &self.api_hash, &self.session_path).await?;

        let token = guard.pending_token.take().ok_or_else(|| {
            "No pending login token. Call request_login_code first.".to_string()
        })?;
        let client = guard.client.as_ref().unwrap();

        match client.sign_in(&token, code).await {
            Ok(user) => {
                let _ = client.session().save_to_file(&self.session_path);
                Ok(serde_json::json!({
                    "id": user.id(),
                    "first_name": user.first_name(),
                    "username": user.username(),
                }))
            }
            Err(SignInError::PasswordRequired(pw_token)) => {
                guard.pending_password_token = Some(pw_token);
                Err("2FA required. Use check_password to provide account password.".to_string())
            }
            Err(e) => Err(format!("Sign in failed: {e}")),
        }
    }

    pub async fn check_password(&self, password: &str) -> Result<Value, String> {
        let mut guard = self.inner.lock().await;
        let pw_token = guard.pending_password_token.take().ok_or_else(|| {
            "No pending 2FA.".to_string()
        })?;
        let client = guard.client.as_ref().ok_or_else(|| {
            "Client not connected.".to_string()
        })?;

        let user = client
            .check_password(pw_token, password)
            .await
            .map_err(|e| format!("Check password failed: {e}"))?;

        let _ = client.session().save_to_file(&self.session_path);
        Ok(serde_json::json!({
            "id": user.id(),
            "first_name": user.first_name(),
            "username": user.username(),
        }))
    }

    pub async fn send_message(&self, peer: &str, text: &str) -> Result<Value, String> {
        let guard = self.inner.lock().await;
        let client = guard.client.as_ref().ok_or_else(|| {
            "Telegram client not connected.".to_string()
        })?;

        let chat = client
            .resolve_username(peer)
            .await
            .map_err(|e| format!("Resolve username failed: {e}"))?
            .ok_or_else(|| format!("Could not resolve username: {peer}"))?;

        let msg = client
            .send_message(chat, text)
            .await
            .map_err(|e| format!("Send message failed: {e}"))?;

        Ok(serde_json::json!({
            "id": msg.id(),
            "date": msg.date().to_string(),
        }))
    }

    pub async fn list_dialogs(&self) -> Result<Value, String> {
        let guard = self.inner.lock().await;
        let client = guard.client.as_ref().ok_or_else(|| {
            "Telegram client not connected.".to_string()
        })?;

        let mut dialogs: Vec<Value> = Vec::new();
        let mut iter = client.iter_dialogs();
        while let Some(dialog) = iter
            .next()
            .await
            .map_err(|e| format!("Dialog error: {e}"))?
        {
            let chat = dialog.chat();
            dialogs.push(serde_json::json!({
                "id": chat.id(),
                "name": chat.name(),
            }));
        }

        Ok(serde_json::json!({ "dialogs": dialogs }))
    }

    pub async fn list_dialogs_detailed(&self) -> Result<Value, String> {
        let guard = self.inner.lock().await;
        let client = guard.client.as_ref().ok_or_else(|| {
            "Telegram client not connected.".to_string()
        })?;

        let mut dialogs: Vec<Value> = Vec::new();
        let mut iter = client.iter_dialogs();
        while let Some(dialog) = iter
            .next()
            .await
            .map_err(|e| format!("Dialog error: {e}"))?
        {
            let chat = dialog.chat();
            let chat_id = chat.id();
            // Telegram ID conventions: positive = user/peer,
            // negative with -100 prefix = channel/supergroup,
            // negative without prefix = group
            let target_type = if chat_id < -1_000_000_000_000i64 {
                "channel"
            } else if chat_id < 0 {
                "group"
            } else {
                "peer"
            };
            dialogs.push(serde_json::json!({
                "id": chat_id,
                "name": chat.name(),
                "type": target_type,
            }));
        }

        Ok(serde_json::json!({ "dialogs": dialogs }))
    }

    pub async fn list_contacts(&self) -> Result<Value, String> {
        let guard = self.inner.lock().await;
        let client = guard.client.as_ref().ok_or_else(|| {
            "Telegram client not connected.".to_string()
        })?;

        let mut contacts: Vec<Value> = Vec::new();
        let mut iter = client.iter_dialogs();
        while let Some(dialog) = iter
            .next()
            .await
            .map_err(|e| format!("Dialog error: {e}"))?
        {
            let chat = dialog.chat();
            contacts.push(serde_json::json!({
                "id": chat.id(),
                "name": chat.name(),
            }));
        }

        Ok(serde_json::json!({ "contacts": contacts }))
    }

    pub async fn search(&self, query: &str) -> Result<Value, String> {
        let guard = self.inner.lock().await;
        let client = guard.client.as_ref().ok_or_else(|| {
            "Telegram client not connected.".to_string()
        })?;

        let mut results: Vec<Value> = Vec::new();
        let q = query.to_lowercase();
        let mut iter = client.iter_dialogs();
        while let Some(dialog) = iter
            .next()
            .await
            .map_err(|e| format!("Dialog error: {e}"))?
        {
            let chat = dialog.chat();
            if chat.name().to_lowercase().contains(&q) {
                results.push(serde_json::json!({
                    "id": chat.id(),
                    "name": chat.name(),
                    "type": "dialog",
                }));
            }
        }

        Ok(serde_json::json!({ "results": results }))
    }

    pub async fn user_info(&self) -> Result<Value, String> {
        let guard = self.inner.lock().await;
        let client = guard.client.as_ref().ok_or_else(|| {
            "Telegram client not connected.".to_string()
        })?;

        let me = client
            .get_me()
            .await
            .map_err(|e| format!("Get me failed: {e}"))?;

        Ok(serde_json::json!({
            "id": me.id(),
            "first_name": me.first_name(),
            "username": me.username(),
        }))
    }

    async fn ensure_client(
        guard: &mut InnerState,
        api_id: i32,
        api_hash: &str,
        session_path: &PathBuf,
    ) -> Result<(), String> {
        if guard.client.is_some() {
            return Ok(());
        }

        tokio::fs::create_dir_all(
            session_path.parent().expect("session_path must have parent"),
        )
        .await
        .map_err(|e| format!("Failed to create session dir: {e}"))?;

        let session = Session::load_file_or_create(session_path)
            .map_err(|e| format!("Failed to load or create session: {e}"))?;

        let config = Config {
            session,
            api_id,
            api_hash: api_hash.to_string(),
            params: InitParams::default(),
        };

        let client = Client::connect(config)
            .await
            .map_err(|e| format!("Failed to connect to Telegram: {e}"))?;

        guard.client = Some(client);
        Ok(())
    }
}

pub type OptionalTelegramClient = Option<Arc<TelegramClientManager>>;


