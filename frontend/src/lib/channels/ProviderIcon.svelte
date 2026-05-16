<script lang="ts">
  let { provider = "", size = "md" }: { provider?: string; size?: "sm" | "md" | "lg" } = $props();

  const icons: Record<string, string> = {
    x: "🐦", facebook: "📘", instagram: "📷", threads: "🧵",
    linkedin: "💼", linkedin_page: "🏢", "linkedin-page": "🏢",
    youtube: "▶️", pinterest: "📌",
    reddit: "👽", bluesky: "🦋", discord: "💬", telegram: "✈️",
    twitch: "📺", vk: "💬", google_my_business: "🏪", whop: "🛒",
    whatsapp: "💚", skool: "🎓", lemmy: "🗳️",
    farcaster: "🏰", nostr: "🧡",
    mewe: "👥", moltbook: "📘", kick: "🦵",
  };

  const dims = { sm: "w-6 h-6 text-sm", md: "w-8 h-8 text-base", lg: "w-10 h-10 text-lg" };
  const icon = $derived(icons[provider] || "🔌");
</script>

<div class="rounded-full bg-[#1e2435] flex items-center justify-center {dims[size]}" aria-hidden="true">
  {icon}
</div>
