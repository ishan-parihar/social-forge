// ─── Automation Engine ──────────────────────────────────────────
// Handles auto-reply to comments and DMs based on configurable rules.

use chrono::{DateTime, Utc, Duration};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug)]
pub struct AutomationRule {
    pub id: Uuid,
    pub user_id: Uuid,
    pub integration_id: Uuid,
    pub name: String,
    pub trigger_type: String,
    pub trigger_filter: serde_json::Value,
    pub response_template: String,
    pub response_type: String,
    pub ai_model: Option<String>,
    pub is_active: Option<bool>,
    pub cooldown_minutes: Option<i32>,
    pub max_responses_per_hour: Option<i32>,
}

#[derive(Debug)]
pub struct TriggerContext {
    pub trigger_id: String,
    pub trigger_type: String,
    pub platform: String,
    pub author_name: Option<String>,
    pub content: String,
    pub post_id: Option<String>,
}

#[derive(Debug)]
pub struct AutomationAction {
    pub rule: AutomationRule,
    pub response: String,
    pub target_id: String,
}

pub struct AutomationEngine {
    db: PgPool,
}

impl AutomationEngine {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn check_triggers(
        &self,
        trigger_type: &str,
        integration_id: Uuid,
        context: &TriggerContext,
    ) -> Result<Vec<AutomationAction>, String> {
        let rules = sqlx::query_as!(
            AutomationRule,
            r#"SELECT id, user_id, integration_id, name, trigger_type,
                      trigger_filter, response_template, response_type,
                      ai_model, is_active, cooldown_minutes, max_responses_per_hour
               FROM automation_rules
               WHERE integration_id = $1
                 AND trigger_type = $2
                 AND is_active = true"#,
            integration_id,
            trigger_type,
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| format!("Failed to fetch rules: {e}"))?;

        let mut actions = Vec::new();

        for rule in rules {
            if !self.matches_filter(&rule, context).await? {
                continue;
            }

            if !self.check_cooldown(&rule, context).await? {
                continue;
            }

            if !self.check_rate_limit(&rule).await? {
                continue;
            }

            let response = self.generate_response(&rule, context).await?;

            actions.push(AutomationAction {
                rule,
                response,
                target_id: context.trigger_id.clone(),
            });
        }

        Ok(actions)
    }

    async fn matches_filter(
        &self,
        rule: &AutomationRule,
        context: &TriggerContext,
    ) -> Result<bool, String> {
        let filter = &rule.trigger_filter;

        if let Some(keywords) = filter.get("keywords").and_then(|v| v.as_array()) {
            let content_lower = context.content.to_lowercase();
            let has_keyword = keywords.iter().any(|k| {
                k.as_str()
                    .map(|kw| content_lower.contains(&kw.to_lowercase()))
                    .unwrap_or(false)
            });
            if !has_keyword {
                return Ok(false);
            }
        }

        if let Some(platforms) = filter.get("platforms").and_then(|v| v.as_array()) {
            let has_platform = platforms.iter().any(|p| {
                p.as_str()
                    .map(|pl| pl.eq_ignore_ascii_case(&context.platform))
                    .unwrap_or(false)
            });
            if !has_platform {
                return Ok(false);
            }
        }

        if let Some(exclude_own) = filter.get("exclude_own").and_then(|v| v.as_bool()) {
            if exclude_own {
                let integration = sqlx::query!(
                    r#"SELECT internal_id FROM integrations WHERE id = $1"#,
                    rule.integration_id,
                )
                .fetch_optional(&self.db)
                .await
                .map_err(|e| e.to_string())?;

                if let Some(int) = integration {
                    if context.author_name.as_deref() == Some(&int.internal_id) {
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }

    async fn check_cooldown(
        &self,
        rule: &AutomationRule,
        context: &TriggerContext,
    ) -> Result<bool, String> {
        let cooldown = rule.cooldown_minutes.unwrap_or(0);
        if cooldown <= 0 {
            return Ok(true);
        }

        let cutoff = Utc::now() - Duration::minutes(cooldown as i64);

        let recent = sqlx::query!(
            r#"SELECT COUNT(*) as count FROM automation_logs
               WHERE rule_id = $1
                 AND trigger_id = $2
                 AND created_at > $3
                 AND status = 'sent'"#,
            rule.id,
            context.trigger_id,
            cutoff,
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| e.to_string())?;

        Ok(recent.count.unwrap_or(0) == 0)
    }

    async fn check_rate_limit(&self, rule: &AutomationRule) -> Result<bool, String> {
        let max_per_hour = rule.max_responses_per_hour.unwrap_or(10);
        if max_per_hour <= 0 {
            return Ok(true);
        }

        let cutoff = Utc::now() - Duration::hours(1);

        let recent = sqlx::query!(
            r#"SELECT COUNT(*) as count FROM automation_logs
               WHERE rule_id = $1
                 AND created_at > $2
                 AND status = 'sent'"#,
            rule.id,
            cutoff,
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| e.to_string())?;

        Ok(recent.count.unwrap_or(0) < max_per_hour as i64)
    }

    async fn generate_response(
        &self,
        rule: &AutomationRule,
        context: &TriggerContext,
    ) -> Result<String, String> {
        match rule.response_type.as_str() {
            "fixed" => Ok(rule.response_template.clone()),
            "template" => {
                let mut response = rule.response_template.clone();
                response = response.replace("{author}", context.author_name.as_deref().unwrap_or("there"));
                response = response.replace("{content}", &context.content);
                response = response.replace("{platform}", &context.platform);
                Ok(response)
            }
            "ai_generated" => {
                self.generate_ai_response(rule, context).await
            }
            _ => Ok(rule.response_template.clone()),
        }
    }

    async fn generate_ai_response(
        &self,
        rule: &AutomationRule,
        context: &TriggerContext,
    ) -> Result<String, String> {
        let model = rule.ai_model.as_deref().unwrap_or("gpt-4o-mini");

        let prompt = format!(
            "You are a social media manager. Respond to this {} comment/message on {}.\n\nComment: {}\n\nGenerate a brief, friendly, professional response.",
            context.trigger_type,
            context.platform,
            context.content,
        );

        let client = reqwest::Client::new();
        let resp = client
            .post("http://localhost:4488/v1/chat/completions")
            .json(&serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": prompt}],
                "max_tokens": 200,
            }))
            .send()
            .await
            .map_err(|e| format!("AI request failed: {e}"))?;

        let json: serde_json::Value = resp.json().await
            .map_err(|e| format!("AI response parse failed: {e}"))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Empty AI response".into())
    }

    pub async fn log_execution(
        &self,
        rule_id: Uuid,
        trigger_id: &str,
        trigger_type: &str,
        response: Option<&str>,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<(), String> {
        sqlx::query!(
            r#"INSERT INTO automation_logs (rule_id, trigger_id, trigger_type, response, status, error_message)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
            rule_id,
            trigger_id,
            trigger_type,
            response,
            status,
            error_message,
        )
        .execute(&self.db)
        .await
        .map_err(|e| format!("Failed to log execution: {e}"))?;

        Ok(())
    }
}
