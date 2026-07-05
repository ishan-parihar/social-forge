// AI API — calls the backend's /api/ai/* endpoints which proxy to the
// configured LLM_ENDPOINT + LLM_MODEL. This keeps the API key server-side
// and enables MCP/CLI parity (the same LLM logic runs for all interfaces).

interface AiResponse {
  content: string;
  error?: string;
}

async function callBackend(endpoint: string, body: Record<string, unknown>, signal?: AbortSignal): Promise<string> {
  try {
    const r = await fetch(`/api/ai/${endpoint}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "include",
      body: JSON.stringify(body),
      signal,
    });
    if (!r.ok) {
      const err = await r.json().catch(() => ({}));
      throw new Error(err.error || `Backend returned ${r.status}`);
    }
    const data: AiResponse = await r.json();
    return data.content || "";
  } catch (err) {
    if (err instanceof Error && err.name === "AbortError") throw err;
    throw new Error(`AI request failed: ${err instanceof Error ? err.message : "Unknown error"}`);
  }
}

export const ai = {
  /** Generate a post from topic/tone/length */
  async generatePost(topic: string, tone: string, length: string, signal?: AbortSignal): Promise<string> {
    return callBackend("generate-post", { topic, tone, length }, signal);
  },

  /** Improve existing content */
  async improveWriting(content: string, signal?: AbortSignal): Promise<string> {
    return callBackend("improve-writing", { content }, signal);
  },

  /** Suggest hashtags from content */
  async suggestHashtags(content: string, signal?: AbortSignal): Promise<string> {
    return callBackend("suggest-hashtags", { content }, signal);
  },

  /** Change tone of content */
  async changeTone(content: string, tone: string, signal?: AbortSignal): Promise<string> {
    return callBackend("change-tone", { content, tone }, signal);
  },

  /** Summarize for X/Twitter (280 chars) */
  async summarize(content: string, signal?: AbortSignal): Promise<string> {
    return callBackend("summarize", { content }, signal);
  },
};
