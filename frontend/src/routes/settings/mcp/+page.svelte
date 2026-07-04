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

<div class="page-enter space-y-6">
  <h2 class="text-xl font-semibold">MCP & CLI Configuration</h2>

  <!-- MCP Configuration -->
  <div class="bg-surface border border-line rounded-xl p-5 space-y-3">
    <h3 class="text-sm font-medium">MCP Configuration</h3>
    <p class="text-xs text-muted">Add this to your Claude Desktop or Cursor MCP config:</p>
    <div class="relative">
      <pre class="bg-[#0b0e14] border border-line rounded-lg p-4 text-xs text-indigo-300 font-mono overflow-x-auto">{mcpConfig}</pre>
      <button
        onclick={() => copy(mcpConfig, 'mcp')}
        class="absolute top-2 right-2 px-2 py-1 text-xs rounded bg-[#1e2435] text-muted hover:text-white transition-colors"
      >
        {copiedMcp ? '✓ Copied' : 'Copy'}
      </button>
    </div>
    <p class="text-xs text-muted">This exposes 130+ tools with full JSON Schema descriptions to your AI agent.</p>
  </div>

  <!-- CLI Quick Start -->
  <div class="bg-surface border border-line rounded-xl p-5 space-y-3">
    <h3 class="text-sm font-medium">CLI Quick Start</h3>
    <p class="text-xs text-muted">Common commands for AI agents and terminal usage:</p>
    <div class="relative">
      <pre class="bg-[#0b0e14] border border-line rounded-lg p-4 text-xs text-green-300 font-mono overflow-x-auto">{cliCommands}</pre>
      <button
        onclick={() => copy(cliCommands, 'cli')}
        class="absolute top-2 right-2 px-2 py-1 text-xs rounded bg-[#1e2435] text-muted hover:text-white transition-colors"
      >
        {copiedCli ? '✓ Copied' : 'Copy'}
      </button>
    </div>
    <p class="text-xs text-muted">All CLI output is JSON by default — designed for machine consumption.</p>
  </div>

  <!-- Configuration File -->
  <div class="bg-surface border border-line rounded-xl p-5 space-y-3">
    <h3 class="text-sm font-medium">Configuration File</h3>
    <p class="text-xs text-muted">Social Forge stores user config at <code class="text-indigo-400">~/.social-forge/.env</code></p>
    <pre class="bg-[#0b0e14] border border-line rounded-lg p-4 text-xs text-amber-300 font-mono">Run: social-forge init
Then edit: ~/.social-forge/.env</pre>
    <p class="text-xs text-muted">Set <code class="text-indigo-400">DATABASE_URL</code> and any platform credentials there. The CLI reads this file automatically from any directory.</p>
  </div>

  <!-- API Access -->
  <div class="bg-surface border border-line rounded-xl p-5 space-y-3">
    <h3 class="text-sm font-medium">API Access</h3>
    <p class="text-xs text-muted">REST API base URL:</p>
    <pre class="bg-[#0b0e14] border border-line rounded-lg p-4 text-xs text-indigo-300 font-mono">{typeof window !== 'undefined' ? window.location.origin : 'https://your-host'}/api</pre>
    <p class="text-xs text-muted">The WebUI is protected by a password gate (<code class="text-indigo-400">APP_PASSWORD</code> env var). After login, a signed <code class="text-indigo-400">sf_session</code> cookie is set and sent automatically by the browser. For CLI/script access, use the MCP stdio server:</p>
    <pre class="bg-[#0b0e14] border border-line rounded-lg p-4 text-xs text-green-300 font-mono overflow-x-auto">social-forge mcp</pre>
    <p class="text-xs text-muted">AI agents connect to this stdio server directly — no HTTP auth needed (local shell access implies trust).</p>
  </div>
</div>
