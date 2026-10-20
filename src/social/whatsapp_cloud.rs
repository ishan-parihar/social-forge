// ─── WhatsApp Cloud API provider (E6) ──────────────────────────
// The business-number path alongside the wa-rs Web session: template CRUD
// via the WhatsApp Business Management API and template-message sends via
// the Cloud API (the only compliant way to message outside the 24h window).
//
// Selection: when WHATSAPP_CLOUD_* creds are set, the WhatsAppProvider's
// publish path routes here for posts whose settings say auth_method=cloud;
// without creds the Web-session path is untouched. The HTTP surface here
// is plain graph.facebook.com JSON — no SDK, one shared client.

use super::*;
use crate::config::Config;

const GRAPH_BASE: &str = "https://graph.facebook.com/v21.0";

#[derive(Debug, Clone)]
pub struct CloudCreds {
    pub token: String,
    pub phone_number_id: String,
    pub waba_id: String,
}

/// Read the Cloud creds from Config; None when unset (Web-session mode).
pub fn creds_from_config(config: &Config) -> Option<CloudCreds> {
    let token = config.whatsapp_cloud_token.clone()?;
    if token.is_empty() {
        return None;
    }
    Some(CloudCreds {
        token,
        phone_number_id: config.whatsapp_cloud_phone_number_id.clone()?,
        waba_id: config.whatsapp_cloud_waba_id.clone()?,
    })
}

#[derive(Debug)]
pub struct WhatsAppCloudProvider {
    creds: CloudCreds,
    http: reqwest::Client,
}

impl WhatsAppCloudProvider {
    pub fn new(creds: CloudCreds) -> Self {
        Self {
            creds,
            http: shared_http(),
        }
    }

    /// The Cloud path participates in publish via the combined
    /// "whatsapp" WhatsAppProvider — this identifier is diagnostic only.
    pub fn identifier(&self) -> &'static str {
        "whatsapp-cloud"
    }

    pub fn from_config(config: &Config) -> Option<Self> {
        creds_from_config(config).map(Self::new)
    }

    async fn graph(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{GRAPH_BASE}/{path}");
        let mut req = self
            .http
            .request(method, &url)
            .bearer_auth(&self.creds.token);
        if !query.is_empty() {
            req = req.query(query);
        }
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else {
            let msg = json["error"]["message"]
                .as_str()
                .or_else(|| json["error"].as_str())
                .unwrap_or("WhatsApp Cloud API error")
                .to_string();
            Err(ProviderError::Api(format!(
                "WhatsApp Cloud error ({status}): {msg}"
            )))
        }
    }

    // ── Template CRUD (Management API) ──────────────────────────

    /// List the WABA's message templates ("read" of CRUD).
    pub async fn list_templates(
        &self,
        limit: u32,
        after: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let mut query = vec![("limit", limit.to_string())];
        if let Some(a) = after {
            query.push(("after", a.to_string()));
        }
        let query: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();
        self.graph(
            reqwest::Method::GET,
            &format!("{}/message_templates", self.creds.waba_id),
            &query,
            None,
        )
        .await
    }

    /// Create a template. `components` is the raw Meta component array
    /// (HEADER/BODY/FOOTER/BUTTONS); category is MARKETING|UTILITY|AUTHENTICATION.
    pub async fn create_template(
        &self,
        name: &str,
        language: &str,
        category: &str,
        components: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        self.graph(
            reqwest::Method::POST,
            &format!("{}/message_templates", self.creds.waba_id),
            &[],
            Some(serde_json::json!({
                "name": name,
                "language": language,
                "category": category,
                "components": components,
            })),
        )
        .await
    }

    /// Delete a template by name.
    pub async fn delete_template(&self, name: &str) -> Result<serde_json::Value, ProviderError> {
        self.graph(
            reqwest::Method::DELETE,
            &format!("{}/message_templates", self.creds.waba_id),
            &[("name", name)],
            None,
        )
        .await
    }

    // ── Sends (Cloud API) ────────────────────────────────────────

    /// Send a template message. `components` carries the BODY/HEADER params.
    pub async fn send_template(
        &self,
        to: &str,
        template_name: &str,
        language: &str,
        components: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        self.send_raw(serde_json::json!({
            "messaging_product": "whatsapp",
            "to": to,
            "type": "template",
            "template": {
                "name": template_name,
                "language": { "code": language },
                "components": components,
            },
        }))
        .await
    }

    /// Send a plain text message (only deliverable inside the 24h customer
    /// service window; template sends are the compliant outbound path).
    pub async fn send_text(
        &self,
        to: &str,
        text: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        self.send_raw(serde_json::json!({
            "messaging_product": "whatsapp",
            "to": to,
            "type": "text",
            "text": { "body": text },
        }))
        .await
    }

    async fn send_raw(&self, body: serde_json::Value) -> Result<serde_json::Value, ProviderError> {
        self.graph(
            reqwest::Method::POST,
            &format!("{}/messages", self.creds.phone_number_id),
            &[],
            Some(body),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creds_require_all_three_fields() {
        let mut config = crate::social::test_config();
        config.whatsapp_cloud_token = None;
        config.whatsapp_cloud_phone_number_id = None;
        config.whatsapp_cloud_waba_id = None;
        assert!(creds_from_config(&config).is_none());
        config.whatsapp_cloud_token = Some("tok".into());
        assert!(
            creds_from_config(&config).is_none(),
            "phone id still missing"
        );
        config.whatsapp_cloud_phone_number_id = Some("123".into());
        assert!(
            creds_from_config(&config).is_none(),
            "waba id still missing"
        );
        config.whatsapp_cloud_waba_id = Some("456".into());
        let creds = creds_from_config(&config).expect("all set");
        assert_eq!(creds.phone_number_id, "123");
        assert_eq!(creds.waba_id, "456");
    }

    #[test]
    fn webhook_signature_verification_lives_in_api_module() {
        // The actual HMAC round-trip test is in api::whatsapp_webhook's
        // test module (the helpers live there). This asserts the cross-
        // module path stays reachable so the tests can't drift apart.
        let sig = crate::api::whatsapp_webhook::signature_hex("k", b"b");
        assert_eq!(sig.len(), 64, "hex sha256");
        assert!(crate::api::whatsapp_webhook::verify_signature(
            "k",
            b"b",
            &format!("sha256={sig}")
        ));
    }
}
