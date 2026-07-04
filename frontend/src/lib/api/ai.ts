// AI proxy URL — must be set via VITE_PUBLIC_AI_PROXY_URL at build time.
// If unset, AI features are disabled and the UI shows a configuration hint
// instead of silently hitting a localhost default that doesn't exist in
// production.
const AI_PROXY_URL: string | null =
  (import.meta.env?.VITE_PUBLIC_AI_PROXY_URL as string | undefined) || null;

interface AiResponse {
  choices: Array<{
    message: { content: string };
  }>;
}

async function generate(prompt: string, temperature = 0.7, signal?: AbortSignal): Promise<string> {
  if (!AI_PROXY_URL) {
    throw new Error("AI features are disabled. Set VITE_PUBLIC_AI_PROXY_URL at build time to enable.");
  }
  try {
    const r = await fetch(`${AI_PROXY_URL}/v1/chat/completions`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: "gpt-4o-mini",
        messages: [{ role: "user", content: prompt }],
        temperature,
      }),
      signal,
    });
    if (!r.ok) throw new Error(`LLM-Proxy returned ${r.status}`);
    const data: AiResponse = await r.json();
    return data.choices?.[0]?.message?.content || "";
  } catch (err) {
    throw new Error(`AI request failed: ${err instanceof Error ? err.message : "Unknown error"}`);
  }
}

export const ai = {
  /** Generate a post from topic/tone/length */
  async generatePost(topic: string, tone: string, length: string, signal?: AbortSignal): Promise<string> {
    return generate(`Write a ${tone} social media post about "${topic}". Length: ${length}. Do not add hashtags.`, 0.7, signal);
  },

  /** Improve existing content */
  async improveWriting(content: string, signal?: AbortSignal): Promise<string> {
    return generate(`Improve the following social media post for clarity and engagement. Keep the same message and length:\n\n${content}`, 0.5, signal);
  },

  /** Suggest hashtags from content */
  async suggestHashtags(content: string, signal?: AbortSignal): Promise<string> {
    return generate(`Extract 5-10 relevant hashtags from this content. Return ONLY the hashtags separated by spaces, no explanations:\n\n${content}`, 0.3, signal);
  },

  /** Change tone of content */
  async changeTone(content: string, tone: string, signal?: AbortSignal): Promise<string> {
    return generate(`Rewrite the following post in a ${tone} tone. Keep the same information and approximate length:\n\n${content}`, 0.8, signal);
  },

  /** Summarize for X/Twitter (280 chars) */
  async summarize(content: string, signal?: AbortSignal): Promise<string> {
    return generate(`Summarize the following content for an X/Twitter post. Maximum 280 characters. Return ONLY the post, no explanations:\n\n${content}`, 0.3, signal);
  },
};
