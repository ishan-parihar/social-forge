/** Auth type mapping: determines which connect UI to show per provider. */
export type AuthType = "oauth" | "api_key" | "web3" | "extension" | "cookie" | "pat";

export const AUTH_TYPES: Record<string, AuthType> = {
  lemmy: "api_key",
  wordpress: "api_key",
  medium: "api_key",
  devto: "api_key",
  hashnode: "api_key",
  farcaster: "web3",
  nostr: "web3",
  skool: "extension",
};

/** Providers that support multiple connect methods */
export const MULTI_AUTH_PROVIDERS: Record<string, AuthType[]> = {
  x: ["oauth", "cookie"],
  reddit: ["oauth", "cookie"],
  github: ["pat"],
  "telegram-bot": ["pat"],  // custom bot token only
};

/**
 * Returns the auth type for a given provider.
 * Defaults to "oauth" for all unlisted providers.
 */
export function getAuthType(provider: string): AuthType {
  return AUTH_TYPES[provider] ?? "oauth";
}
