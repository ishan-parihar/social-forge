-- Automation rules for auto-reply to comments/DMs
CREATE TABLE IF NOT EXISTS automation_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    integration_id UUID NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    trigger_type TEXT NOT NULL CHECK (trigger_type IN ('comment', 'dm', 'mention', 'follow')),
    trigger_filter JSONB DEFAULT '{}',
    response_template TEXT NOT NULL,
    response_type TEXT NOT NULL CHECK (response_type IN ('ai_generated', 'template', 'fixed')),
    ai_model TEXT,
    is_active BOOLEAN DEFAULT true,
    cooldown_minutes INT DEFAULT 0,
    max_responses_per_hour INT DEFAULT 10,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_automation_rules_user ON automation_rules(user_id);
CREATE INDEX IF NOT EXISTS idx_automation_rules_integration ON automation_rules(integration_id);
CREATE INDEX IF NOT EXISTS idx_automation_rules_active ON automation_rules(is_active) WHERE is_active = true;

-- Automation execution log
CREATE TABLE IF NOT EXISTS automation_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id UUID NOT NULL REFERENCES automation_rules(id) ON DELETE CASCADE,
    trigger_id TEXT NOT NULL,
    trigger_type TEXT NOT NULL,
    response TEXT,
    status TEXT NOT NULL CHECK (status IN ('sent', 'failed', 'skipped_cooldown', 'skipped_limit')),
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_automation_logs_rule ON automation_logs(rule_id);
CREATE INDEX IF NOT EXISTS idx_automation_logs_status ON automation_logs(status);
CREATE INDEX IF NOT EXISTS idx_automation_logs_created ON automation_logs(created_at);
