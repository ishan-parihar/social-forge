<script lang="ts">
  import { onMount } from "svelte";
  import { toast } from "$lib/stores/toast";
  import { modals } from '$lib/stores/modals.svelte';
  import { profileApi, type BrandProfile } from '$lib/api/profile';

  // v24-4: brand profile is now synced to the backend (PUT /api/profile).
  // Previously it was localStorage-only — not synced across devices and
  // not read by the AiAssistant. Now the AiAssistant can use it as
  // context for generate/improve/tone, and the cadence widget can read
  // posts_per_day_goal.
  let brandName = $state("");
  let brandDescription = $state("");
  let toneOfVoice = $state("professional");
  let targetAudience = $state("");
  let contentPillars = $state("");
  let postingFrequency = $state("daily");
  let hashtagSets = $state("");
  let brandKeywords = $state("");
  let avoidTopics = $state("");
  let postsPerDayGoal = $state<number | ''>('');
  let saving = $state(false);
  let loading = $state(true);

  const toneOptions = [
    { value: "professional", label: "Professional" },
    { value: "casual", label: "Casual & Friendly" },
    { value: "witty", label: "Witty & Humorous" },
    { value: "authoritative", label: "Authoritative" },
    { value: "inspirational", label: "Inspirational" },
    { value: "educational", label: "Educational" },
    { value: "conversational", label: "Conversational" },
  ];

  const frequencyOptions = [
    { value: "hourly", label: "Multiple times per day" },
    { value: "daily", label: "Once per day" },
    { value: "weekly", label: "A few times per week" },
    { value: "biweekly", label: "Every two weeks" },
  ];

  // Map posting_frequency to a numeric posts_per_day_goal.
  function frequencyToGoal(freq: string): number | null {
    switch (freq) {
      case 'hourly': return 3;    // 3x/day
      case 'daily': return 1;     // 1x/day
      case 'weekly': return 0.43; // ~3x/week = 3/7
      case 'biweekly': return 0.07; // ~1x/2weeks = 1/14
      default: return null;
    }
  }

  async function loadProfile() {
    loading = true;
    const r = await profileApi.get();
    if (r.data) {
      const p = r.data;
      brandName = p.brand_name ?? "";
      brandDescription = p.description ?? "";
      toneOfVoice = p.tone_of_voice ?? "professional";
      targetAudience = p.audience ?? "";
      // content_pillars is JSONB [{title, description}] — join to newline string.
      if (Array.isArray(p.content_pillars)) {
        contentPillars = p.content_pillars.map((cp: { title?: string; description?: string }) => cp.title).join('\n');
      }
      postingFrequency = p.posting_frequency ?? "daily";
      // hashtag_sets is JSONB [{name, tags}] — join to newline string.
      if (Array.isArray(p.hashtag_sets)) {
        hashtagSets = p.hashtag_sets.map((hs: { name?: string; tags?: string[] }) => (hs.tags || []).join(' ')).join('\n');
      }
      // keywords is JSONB ["k1", "k2"] — join to comma string.
      if (Array.isArray(p.keywords)) {
        brandKeywords = p.keywords.join(', ');
      }
      // avoid_topics is JSONB ["t1", "t2"] — join to comma string.
      if (Array.isArray(p.avoid_topics)) {
        avoidTopics = p.avoid_topics.join(', ');
      }
      postsPerDayGoal = p.posts_per_day_goal ?? '';
    } else {
      // Fallback: try localStorage (migration path for existing users).
      try {
        const stored = localStorage.getItem("sf_brand_profile");
        if (stored) {
          const p = JSON.parse(stored);
          brandName = p.brandName || "";
          brandDescription = p.brandDescription || "";
          toneOfVoice = p.toneOfVoice || "professional";
          targetAudience = p.targetAudience || "";
          contentPillars = p.contentPillars || "";
          postingFrequency = p.postingFrequency || "daily";
          hashtagSets = p.hashtagSets || "";
          brandKeywords = p.brandKeywords || "";
          avoidTopics = p.avoidTopics || "";
        }
      } catch { /* ignore */ }
    }
    loading = false;
  }

  async function saveProfile() {
    saving = true;
    // Convert the form fields to the backend's JSONB structure.
    const contentPillarsArray = contentPillars
      .split('\n').map(s => s.trim()).filter(Boolean)
      .map(title => ({ title, description: '' }));
    const hashtagSetsArray = hashtagSets
      .split('\n').map(s => s.trim()).filter(Boolean)
      .map(tags => ({ name: '', tags: tags.split(/\s+/) }));
    const keywordsArray = brandKeywords.split(',').map(s => s.trim()).filter(Boolean);
    const avoidTopicsArray = avoidTopics.split(',').map(s => s.trim()).filter(Boolean);

    const r = await profileApi.update({
      brand_name: brandName || undefined,
      description: brandDescription || undefined,
      tone_of_voice: toneOfVoice || undefined,
      audience: targetAudience || undefined,
      content_pillars: contentPillarsArray.length > 0 ? contentPillarsArray : undefined,
      keywords: keywordsArray.length > 0 ? keywordsArray : undefined,
      hashtag_sets: hashtagSetsArray.length > 0 ? hashtagSetsArray : undefined,
      avoid_topics: avoidTopicsArray.length > 0 ? avoidTopicsArray : undefined,
      posting_frequency: postingFrequency || undefined,
      posts_per_day_goal: typeof postsPerDayGoal === 'number' ? postsPerDayGoal : frequencyToGoal(postingFrequency),
    });
    saving = false;
    if (r.error) {
      toast(`Failed to save: ${r.error}`, 'error');
    } else {
      // Also save to localStorage as a fallback cache.
      try {
        localStorage.setItem("sf_brand_profile", JSON.stringify({
          brandName, brandDescription, toneOfVoice, targetAudience,
          contentPillars, postingFrequency, hashtagSets, brandKeywords, avoidTopics,
          savedAt: new Date().toISOString(),
        }));
      } catch { /* ignore */ }
      toast("Brand profile saved", "success");
    }
  }

  async function clearProfile() {
    if (!(await modals.areYouSure({
      title: 'Clear all brand profile data?',
      message: 'This will reset brand name, description, tone of voice, and topics to their defaults.',
      confirmLabel: 'Clear',
      cancelLabel: 'Cancel',
      danger: true,
    }))) return;
    brandName = "";
    brandDescription = "";
    toneOfVoice = "professional";
    targetAudience = "";
    contentPillars = "";
    postingFrequency = "daily";
    hashtagSets = "";
    brandKeywords = "";
    avoidTopics = "";
    postsPerDayGoal = '';
    // Save the cleared profile to the backend too.
    await profileApi.update({
      brand_name: null,
      description: null,
      tone_of_voice: 'professional',
      audience: null,
      content_pillars: null,
      keywords: null,
      hashtag_sets: null,
      avoid_topics: null,
      posting_frequency: 'daily',
      posts_per_day_goal: null,
    });
    localStorage.removeItem("sf_brand_profile");
    toast("Profile cleared", "success");
  }

  onMount(loadProfile);
</script>

<div class="page-enter space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-xl font-semibold">Brand Profile</h2>
      <p class="text-sm text-muted mt-1">Define your brand voice, audience, and content strategy for AI-assisted posting</p>
    </div>
    <button onclick={saveProfile} disabled={saving || loading} class="px-4 py-2 bg-brand-500 hover:bg-brand-600 rounded-lg text-sm font-medium transition-colors disabled:opacity-50">
      {saving ? "Saving..." : "Save Profile"}
    </button>
  </div>

  {#if loading}
    <div class="text-center py-12 text-sm text-muted">Loading...</div>
  {:else}
    <!-- Brand Identity -->
    <div class="bg-surface border border-line rounded-xl p-5 space-y-4">
      <h3 class="text-sm font-medium text-content">Brand Identity</h3>
      <div>
        <label class="text-xs text-muted mb-1 block">Brand Name</label>
        <input bind:value={brandName} placeholder="e.g. Acme Corp" class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm" />
      </div>
      <div>
        <label class="text-xs text-muted mb-1 block">Brand Description</label>
        <textarea bind:value={brandDescription} placeholder="What does your brand do? What's your mission?" rows="3" class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm resize-none"></textarea>
      </div>
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="text-xs text-muted mb-1 block">Tone of Voice</label>
          <select bind:value={toneOfVoice} class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm">
            {#each toneOptions as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>
        <div>
          <label class="text-xs text-muted mb-1 block">Posting Frequency Goal</label>
          <select bind:value={postingFrequency} class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm">
            {#each frequencyOptions as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>
      </div>
    </div>

    <!-- Target Audience -->
    <div class="bg-surface border border-line rounded-xl p-5 space-y-4">
      <h3 class="text-sm font-medium text-content">Target Audience</h3>
      <div>
        <label class="text-xs text-muted mb-1 block">Audience Description</label>
        <textarea bind:value={targetAudience} placeholder="Who are you trying to reach? e.g. 'Startup founders aged 25-40 interested in SaaS and productivity tools'" rows="3" class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm resize-none"></textarea>
      </div>
    </div>

    <!-- Content Strategy -->
    <div class="bg-surface border border-line rounded-xl p-5 space-y-4">
      <h3 class="text-sm font-medium text-content">Content Strategy</h3>
      <div>
        <label class="text-xs text-muted mb-1 block">Content Pillars (one per line)</label>
        <textarea bind:value={contentPillars} placeholder={"Product updates\nIndustry insights\nCustomer stories\nEducational tutorials"} rows="5" class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm resize-none font-mono"></textarea>
        <p class="text-[10px] text-muted-dark mt-1">Topics you regularly post about. AI will use these to suggest content.</p>
      </div>
      <div>
        <label class="text-xs text-muted mb-1 block">Brand Keywords</label>
        <input bind:value={brandKeywords} placeholder="e.g. productivity, automation, SaaS, workflow" class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm" />
        <p class="text-[10px] text-muted-dark mt-1">Comma-separated keywords that should appear in your content.</p>
      </div>
      <div>
        <label class="text-xs text-muted mb-1 block">Hashtag Sets (one set per line, space-separated)</label>
        <textarea bind:value={hashtagSets} placeholder={"#SaaS #Productivity #Startup\n#Automation #Workflow #Tech\n#Marketing #Growth #Business"} rows="4" class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm resize-none font-mono"></textarea>
        <p class="text-[10px] text-muted-dark mt-1">Pre-defined hashtag groups for different content types.</p>
      </div>
      <div>
        <label class="text-xs text-muted mb-1 block">Topics to Avoid</label>
        <input bind:value={avoidTopics} placeholder="e.g. politics, controversial topics, competitor names" class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm" />
        <p class="text-[10px] text-muted-dark mt-1">AI will avoid these topics when generating content.</p>
      </div>
    </div>

    <!-- Actions -->
    <div class="flex gap-3 justify-end">
      <button onclick={clearProfile} class="px-4 py-2 text-sm text-error hover:text-error/80 transition-colors">
        Clear All
      </button>
      <button onclick={saveProfile} disabled={saving} class="px-4 py-2 bg-brand-500 hover:bg-brand-600 rounded-lg text-sm font-medium transition-colors disabled:opacity-50">
        {saving ? "Saving..." : "Save Profile"}
      </button>
    </div>
  {/if}
</div>
