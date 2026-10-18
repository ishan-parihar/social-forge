# Agent integration

Moved out of the README to keep it short. Content is unchanged.

## Agent Integration (AXI §7)

Social Forge ships an installable AI agent skill that provides ambient context at session start — showing connected platforms, provider status, and contextual help hints.

### Install the Skill

```bash
# Via npx (recommended)
npx skills add ishan-parihar/social-forge --skill social-forge

# Or download manually (installed automatically by install.sh unless SKIP_SKILL=true)
curl -fsSL https://raw.githubusercontent.com/ishan-parihar/social-forge/master/SKILL.md \
  -o ~/.agents/skills/social-forge/SKILL.md
```

### Session Hook (Claude Code)

Add to `~/.claude/settings.json` or project `.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "social-forge"
          }
        ]
      }
    ]
  }
}
```

At session start, Social Forge prints a compact dashboard:

```
bin: /usr/local/bin/social-forge
description: Post to 26 social platforms from a single CLI

providers[3]{name,status,platforms}:
  x,connected,X/Twitter
  reddit,connected,Reddit
  bluesky,connected,Bluesky
  ...

platforms_total: 26

help[4]:
  Run `social-forge providers` to see all connected accounts
  Run `social-forge post "Hello" --platforms x,bluesky` to post
  Run `social-forge stage "Long content" --platforms x,linkedin` to stage
  Run `social-forge doctor` to check system health
```

### Session Hook (Codex)

Add to `~/.codex/hooks.json` or project `.codex/hooks.json`:

```json
{
  "SessionStart": "social-forge"
}
```

### Session Hook (OpenCode)

Create `~/.config/opencode/plugins/social-forge.ts`:

```typescript
export default {
  name: "social-forge",
  onSessionStart: async () => {
    const { execSync } = require("child_process");
    return execSync("social-forge").toString();
  },
};
```

---
