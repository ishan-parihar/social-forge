# Session Hooks — Ambient Context at Session Start

AXI §7: Register your tool into the agent's session lifecycle so every conversation starts with relevant state already visible.

## What these do

At session start, the hook runs your CLI with no arguments and injects the output as context. The agent sees live state (connected accounts, open items, tool count) before it takes any action.

## Installation

### Claude Code

Add to `~/.claude/settings.json` (global) or `.claude/settings.json` (project):

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "switchpost"
          }
        ]
      }
    ]
  }
}
```

### Codex

Add to `~/.codex/hooks.json`:

```json
{
  "SessionStart": {
    "command": "switchpost"
  }
}
```

Ensure `[features].hooks = true` in `~/.codex/config.toml`.

### OpenCode

Create `~/.config/opencode/plugins/switchpost.ts`:

```typescript
export default {
  name: "switchpost",
  description: "Social media management ambient context",
  sessionStart: async () => {
    const { execSync } = await import("child_process");
    return execSync("switchpost", { encoding: "utf-8" });
  },
};
```

## How it works

1. Agent session starts → hook fires
2. Hook runs `switchpost` (no args = content-first home view)
3. Agent sees connected accounts, open items, tool count
4. Agent can act immediately without a discovery call

## Rules

- **Portable**: hooks use the binary name (`switchpost`). If the binary isn't on PATH, use the full absolute path.
- **Idempotent**: repeated installs with the same path are silent no-ops.
- **Token-budget-aware**: the home view is already optimized for minimal token cost.
