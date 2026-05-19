<script lang="ts">
  let copiedMcp = $state(false);
  let copiedCli = $state(false);

  const mcpConfig = JSON.stringify({
    mcpServers: {
      "social-forge": {
        command: "social-forge",
        args: ["mcp"]
      }
    }
  }, null, 2);

  const cliCommands = `social-forge init                    # Initialize config
social-forge providers               # List connected accounts
social-forge x post "Hello world"    # Post to X/Twitter
social-forge reddit browse rust      # Browse subreddit
social-forge linkedin profile        # View LinkedIn profile`;

  async function copy(text: string, which: 'mcp' | 'cli') {
    await navigator.clipboard.writeText(text);
    if (which === 'mcp') { copiedMcp = true; setTimeout(() => copiedMcp = false, 2000); }
    else { copiedCli = true; setTimeout(() => copiedCli = false, 2000); }
  }
</script>

<div class="space-y-6">
  <h2 class="text-xl font-semibold">MCP & CLI Configuration</h2>

  <!-- MCP Configuration -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-3">
    <h3 class="text-sm font-medium">MCP Configuration</h3>
    <p class="text-xs text-[#6b7280]">Add this to your Claude Desktop or Cursor MCP config:</p>
    <div class="relative">
      <pre class="bg-[#0b0e14] border border-[#1e2435] rounded-lg p-4 text-xs text-indigo-300 font-mono overflow-x-auto">{mcpConfig}</pre>
      <button
        onclick={() => copy(mcpConfig, 'mcp')}
        class="absolute top-2 right-2 px-2 py-1 text-xs rounded bg-[#1e2435] text-[#6b7280] hover:text-white transition-colors"
      >
        {copiedMcp ? '✓ Copied' : 'Copy'}
      </button>
    </div>
    <p class="text-xs text-[#6b7280]">This exposes 130+ tools with full JSON Schema descriptions to your AI agent.</p>
  </div>

  <!-- CLI Quick Start -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-3">
    <h3 class="text-sm font-medium">CLI Quick Start</h3>
    <p class="text-xs text-[#6b7280]">Common commands for AI agents and terminal usage:</p>
    <div class="relative">
      <pre class="bg-[#0b0e14] border border-[#1e2435] rounded-lg p-4 text-xs text-green-300 font-mono overflow-x-auto">{cliCommands}</pre>
      <button
        onclick={() => copy(cliCommands, 'cli')}
        class="absolute top-2 right-2 px-2 py-1 text-xs rounded bg-[#1e2435] text-[#6b7280] hover:text-white transition-colors"
      >
        {copiedCli ? '✓ Copied' : 'Copy'}
      </button>
    </div>
    <p class="text-xs text-[#6b7280]">All CLI output is JSON by default — designed for machine consumption.</p>
  </div>

  <!-- Configuration File -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-3">
    <h3 class="text-sm font-medium">Configuration File</h3>
    <p class="text-xs text-[#6b7280]">Social Forge stores user config at <code class="text-indigo-400">~/.social-forge/.env</code></p>
    <pre class="bg-[#0b0e14] border border-[#1e2435] rounded-lg p-4 text-xs text-amber-300 font-mono">Run: social-forge init
Then edit: ~/.social-forge/.env</pre>
    <p class="text-xs text-[#6b7280]">Set <code class="text-indigo-400">DATABASE_URL</code> and any platform credentials there. The CLI reads this file automatically from any directory.</p>
  </div>

  <!-- API Access -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-3">
    <h3 class="text-sm font-medium">API Access</h3>
    <p class="text-xs text-[#6b7280]">REST API base URL:</p>
    <pre class="bg-[#0b0e14] border border-[#1e2435] rounded-lg p-4 text-xs text-indigo-300 font-mono">{window.location.origin}/api</pre>
    <p class="text-xs text-[#6b7280]">Authenticate requests with a Bearer token:</p>
    <pre class="bg-[#0b0e14] border border-[#1e2435] rounded-lg p-4 text-xs text-green-300 font-mono overflow-x-auto">curl {window.location.origin}/api/posts \
  -H "Authorization: Bearer YOUR_API_KEY"</pre>
    <p class="text-xs text-[#6b7280]">Generate API keys in <a href="/settings/developer" class="text-indigo-400 hover:text-indigo-300">Developer Settings</a>.</p>
  </div>
</div>
