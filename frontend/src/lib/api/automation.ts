import { api } from './client';

export interface AutomationRule {
  id: string;
  integration_id: string;
  name: string;
  trigger_type: string;
  trigger_filter: Record<string, unknown>;
  response_template: string;
  response_type: string;
  ai_model: string | null;
  is_active: boolean;
  cooldown_minutes: number | null;
  max_responses_per_hour: number | null;
}

export interface ExecutionLog {
  id: string;
  rule_id: string;
  trigger_id: string;
  response: string;
  status: string;
  error: string | null;
  created_at: string;
}

export const automationApi = {
  listRules: () =>
    api.get<{ rules: AutomationRule[] }>('/api/automation/rules'),
  createRule: (rule: Omit<AutomationRule, 'id' | 'is_active'>) =>
    api.post<{ rule: AutomationRule }>('/api/automation/rules', rule),
  updateRule: (id: string, updates: Partial<AutomationRule>) =>
    api.put<{ rule: AutomationRule }>(`/api/automation/rules/${id}`, updates),
  deleteRule: (id: string) =>
    api.del<{ success: boolean }>(`/api/automation/rules/${id}`),
  getLogs: (ruleId: string) =>
    api.get<{ logs: ExecutionLog[] }>(`/api/automation/rules/${ruleId}/logs`),
};
