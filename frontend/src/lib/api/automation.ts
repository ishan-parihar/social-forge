import { api } from './client';

// Mirrors src/api/automation.rs RuleResponse. The backend currently
// returns a slimmed-down view of the rule — fields like trigger_filter,
// response_template, ai_model, cooldown_minutes, max_responses_per_hour
// exist in the DB but are not returned by list_rules. If you need them
// on the frontend, extend RuleResponse in the backend first.
export interface AutomationRule {
  id: string;
  name: string;
  trigger_type: string;
  response_type: string;
  is_active: boolean;
  created_at: string;
}

// Fields the frontend automation page uses for display but the backend
// doesn't currently return. They'll be `undefined` at runtime — the
// page guards with optional chaining. Marked optional here so the
// type-checker doesn't complain.
export interface AutomationRuleDisplay extends AutomationRule {
  platform?: string;
  last_triggered?: string;
}

// Mirrors src/api/automation.rs LogEntryResponse.
export interface ExecutionLog {
  id: string;
  trigger_id: string;
  trigger_type: string;
  response: string | null;
  status: string;
  error_message: string | null;
  created_at: string;
}

// Display-only fields the automation page references but the backend
// doesn't return. Optional so the page compiles.
export interface ExecutionLogDisplay extends ExecutionLog {
  input_text?: string;
  output_text?: string;
}

// Create-rule payload — matches CreateRuleRequest in the backend.
export interface CreateRulePayload {
  integration_id: string;
  name: string;
  trigger_type: string;
  trigger_filter?: Record<string, unknown>;
  response_template: string;
  response_type: string;
  ai_model?: string | null;
  cooldown_minutes?: number | null;
  max_responses_per_hour?: number | null;
}

export const automationApi = {
  listRules: () =>
    api.get<{ rules: AutomationRule[] }>('/api/automation/rules'),
  createRule: (rule: CreateRulePayload) =>
    api.post<{ rule: AutomationRule }>('/api/automation/rules', rule),
  updateRule: (id: string, updates: Partial<AutomationRule>) =>
    api.put<{ rule: AutomationRule }>(`/api/automation/rules/${id}`, updates),
  deleteRule: (id: string) =>
    api.del<{ success: boolean }>(`/api/automation/rules/${id}`),
  getLogs: (ruleId: string) =>
    api.get<{ logs: ExecutionLog[] }>(`/api/automation/rules/${ruleId}/logs`),
};
