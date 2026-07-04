<script lang="ts">
  import { onMount } from "svelte";
  import { toast } from "$lib/stores/toast";

  // Brand profile stored in localStorage — used by AI assistant for
  // content generation context. No backend changes needed.
  let brandName = $state("");
  let brandDescription = $state("");
  let toneOfVoice = $state("professional");
  let targetAudience = $state("");
  let contentPillars = $state("");
  let postingFrequency = $state("daily");
  let hashtagSets = $state("");
  let brandKeywords = $state("");
  let avoidTopics = $state("");
  let saving = $state(false);

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

  function loadProfile() {
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

  function saveProfile() {
    saving = true;
    const profile = {
      brandName,
      brandDescription,
      toneOfVoice,
      targetAudience,
      contentPillars,
      postingFrequency,
      hashtagSets,
      brandKeywords,
      avoidTopics,
      savedAt: new Date().toISOString(),
    };
    try {
      localStorage.setItem("sf_brand_profile", JSON.stringify(profile));
      toast("Brand profile saved", "success");
    } catch {
      toast("Failed to save profile", "error");
    }
    saving = false;
  }

  function clearProfile() {
    if (!confirm("Clear all brand profile data?")) return;
    localStorage.removeItem("sf_brand_profile");
    brandName = "";
    brandDescription = "";
    toneOfVoice = "professional";
    targetAudience = "";
    contentPillars = "";
    postingFrequency = "daily";
    hashtagSets = "";
    brandKeywords = "";
    avoidTopics = "";
    toast("Profile cleared", "success");
  }

  onMount(loadProfile);
</script>

<div class="page-enter space-y-5">
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-xl font-semibold">Brand Profile</h2>
      <p class="text-sm text-[#6b7280] mt-1">Define your brand voice, audience, and content strategy for AI-assisted posting</p>
    </div>
    <button onclick={saveProfile} disabled={saving} class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50">
      {saving ? "Saving..." : "Save Profile"}
    </button>
  </div>

  <!-- Brand Identity -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-4">
    <h3 class="text-sm font-medium text-[#e8edf5]">Brand Identity</h3>
    <div>
      <label class="text-xs text-[#6b7280] mb-1 block">Brand Name</label>
      <input bind:value={brandName} placeholder="e.g. Acme Corp" class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm" />
    </div>
    <div>
      <label class="text-xs text-[#6b7280] mb-1 block">Brand Description</label>
      <textarea bind:value={brandDescription} placeholder="What does your brand do? What's your mission?" rows="3" class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm resize-none"></textarea>
    </div>
    <div class="grid grid-cols-2 gap-4">
      <div>
        <label class="text-xs text-[#6b7280] mb-1 block">Tone of Voice</label>
        <select bind:value={toneOfVoice} class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm">
          {#each toneOptions as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </div>
      <div>
        <label class="text-xs text-[#6b7280] mb-1 block">Posting Frequency Goal</label>
        <select bind:value={postingFrequency} class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm">
          {#each frequencyOptions as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </div>
    </div>
  </div>

  <!-- Target Audience -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-4">
    <h3 class="text-sm font-medium text-[#e8edf5]">Target Audience</h3>
    <div>
      <label class="text-xs text-[#6b7280] mb-1 block">Audience Description</label>
      <textarea bind:value={targetAudience} placeholder="Who are you trying to reach? e.g. 'Startup founders aged 25-40 interested in SaaS and productivity tools'" rows="3" class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm resize-none"></textarea>
    </div>
  </div>

  <!-- Content Strategy -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-4">
    <h3 class="text-sm font-medium text-[#e8edf5]">Content Strategy</h3>
    <div>
      <label class="text-xs text-[#6b7280] mb-1 block">Content Pillars (one per line)</label>
      <textarea bind:value={contentPillars} placeholder={"Product updates\nIndustry insights\nCustomer stories\nEducational tutorials"} rows="5" class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm resize-none font-mono"></textarea>
      <p class="text-[10px] text-[#4b5563] mt-1">Topics you regularly post about. AI will use these to suggest content.</p>
    </div>
    <div>
      <label class="text-xs text-[#6b7280] mb-1 block">Brand Keywords</label>
      <input bind:value={brandKeywords} placeholder="e.g. productivity, automation, SaaS, workflow" class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm" />
      <p class="text-[10px] text-[#4b5563] mt-1">Comma-separated keywords that should appear in your content.</p>
    </div>
    <div>
      <label class="text-xs text-[#6b7280] mb-1 block">Hashtag Sets (one set per line, space-separated)</label>
      <textarea bind:value={hashtagSets} placeholder={"#SaaS #Productivity #Startup\n#Automation #Workflow #Tech\n#Marketing #Growth #Business"} rows="4" class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm resize-none font-mono"></textarea>
      <p class="text-[10px] text-[#4b5563] mt-1">Pre-defined hashtag groups for different content types.</p>
    </div>
    <div>
      <label class="text-xs text-[#6b7280] mb-1 block">Topics to Avoid</label>
      <input bind:value={avoidTopics} placeholder="e.g. politics, controversial topics, competitor names" class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm" />
      <p class="text-[10px] text-[#4b5563] mt-1">AI will avoid these topics when generating content.</p>
    </div>
  </div>

  <!-- Actions -->
  <div class="flex gap-3 justify-end">
    <button onclick={clearProfile} class="px-4 py-2 text-sm text-red-400 hover:text-red-300 transition-colors">
      Clear All
    </button>
    <button onclick={saveProfile} disabled={saving} class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50">
      {saving ? "Saving..." : "Save Profile"}
    </button>
  </div>
</div>
