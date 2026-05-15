import type { Integration } from "$lib/api/integrations";

export function groupIntegrations(integrations: Integration[]): Map<string, Integration[]> {
  const g = new Map<string, Integration[]>();
  for (const int of integrations) {
    const key = int.provider_name || int.provider_identifier;
    const existing = g.get(key) || [];
    existing.push(int);
    g.set(key, existing);
  }
  return g;
}
