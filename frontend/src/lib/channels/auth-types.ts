/** Auth type mapping: determines which connect UI to show per provider. */
export type AuthType = "oauth" | "api_key" | "web3" | "extension" | "cookie" | "pat";

export const AUTH_TYPES: Record<string, AuthType> = {
  wordpress: "api_key",
  medium: "api_key",
  devto: "api_key",
  hashnode: "api_key",
  // Tier-3 archive provider (v25 §1): only reachable when the backend is
  // started with ENABLE_ARCHIVE_PROVIDERS=1.
  farcaster: "web3",
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

/**
 * v25 F4 — the connect methods a provider offers, in the order they should be
 * presented. Before F4 the method list was computed ad-hoc in three places (the
 * channels page branching on MULTI_AUTH_PROVIDERS, ConnectFlow branching on
 * getAuthType, and a bespoke `credDialog`), which is why the same provider
 * behaved differently depending on which one you asked. This is the one list.
 *
 * Returns several entries only where the user genuinely has a choice: X and
 * Reddit can be connected by OAuth or by pasting browser cookies, and the
 * cookie path unlocks capabilities the OAuth scopes do not. That choice is the
 * user's to make, not the app's.
 */
export function connectMethodsFor(provider: string): AuthType[] {
  const multi = MULTI_AUTH_PROVIDERS[provider];
  if (multi && multi.length > 0) return multi;
  return [getAuthType(provider)];
}

/** Human labels for the method picker, so the wording is consistent. */
export const METHOD_LABELS: Record<AuthType, string> = {
  oauth: "OAuth 2.0",
  cookie: "Browser cookies",
  api_key: "API key",
  pat: "Personal access token",
  web3: "Wallet address",
  extension: "Browser extension",
};
