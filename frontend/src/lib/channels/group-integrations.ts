import type { Integration } from "$lib/api/integrations";

export function groupIntegrations(integrations: Integration[]): Map<string, Integration[]> {
  return integrations.reduce((g, int) => {
    const key = int.provider_name || int.provider_identifier;
    const existing = g.get(key) || [];
    existing.push(int);
    return g.set(key, existing);
  }, new Map<string, Integration[]>());
}
