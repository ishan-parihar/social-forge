/**
 * Auth type mapping for providers.
 * - "oauth": Standard OAuth flow (window.open) — default
 * - "api_key": API Key input dialog
 * - "web3": Web3 address/public key dialog
 * - "extension": Browser extension info dialog
 */
export type AuthType = "oauth" | "api_key" | "web3" | "extension";

export const AUTH_TYPES: Record<string, AuthType> = {
  // API Key providers
  lemmy: "api_key",
  // Web3 providers
  farcaster: "web3",
  nostr: "web3",
  // Extension providers
  skool: "extension",
};

/**
 * Returns the auth type for a given provider.
 * Defaults to "oauth" for all unlisted providers.
 */
export function getAuthType(provider: string): AuthType {
  return AUTH_TYPES[provider] ?? "oauth";
}
