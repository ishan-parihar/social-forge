<script lang="ts">
  let { provider = "", size = "md" }: { provider?: string; size?: "sm" | "md" | "lg" } = $props();

  const icons: Record<string, string> = {
    x: "🐦", facebook: "📘", instagram: "📷",
    "instagram-standalone": "📸", threads: "🧵",
    linkedin: "💼", "linkedin-page": "🏢",
    google: "🔵", youtube: "▶️", google_my_business: "🏪",
    reddit: "👽", bluesky: "🦋", discord: "💬", pinterest: "📌",
    tiktok: "🎵", twitch: "📺", vk: "🔵", mewe: "👥",
    moltbook: "📘", kick: "🦵", mastodon: "🐘",
    "telegram-bot": "🤖", "telegram-user": "✈️", whatsapp: "💚", slack: "💬",
    wordpress: "📝", medium: "📰", devto: "👩‍💻", hashnode: "📓",
    github: "🐙", lemmy: "🗳️", whop: "🛒",
    farcaster: "🏰", nostr: "🧡",
    skool: "🎓",
  };

  const dims = { sm: "w-6 h-6 text-sm", md: "w-8 h-8 text-base", lg: "w-10 h-10 text-lg" };
  const icon = $derived(icons[provider] || "🔌");
</script>

<div class="rounded-full bg-[#1e2435] flex items-center justify-center {dims[size]}" aria-hidden="true">
  {icon}
</div>
