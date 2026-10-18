// ─── Agent-surface meta audit (D1 + D2 from OUTLINE-2026-10-19) ─────────
// Pure source/doc parsing — no DB, no network. Guards the two drift
// classes that cost agents context:
//   D1: the documented tool count vs the registered #[tool(] count.
//   D2: schema shape — every tool carries a non-empty, unique,
//       size-bounded description and a provider-prefixed name.

use std::path::PathBuf;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// (name, description) for every `#[tool(description = "…")]` + the fn that
/// follows it. Fails loudly on an unparsed `#[tool(` line, and asserts the
/// parsed count equals the raw `#[tool(` count so the check can never
/// silently drop a tool.
fn parse_tools() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut raw_attrs = 0;
    for entry in std::fs::read_dir(repo_path("src/mcp")).expect("read src/mcp") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("read source");
        let lines: Vec<&str> = src.lines().collect();
        raw_attrs += lines.iter().filter(|l| l.trim().starts_with("#[tool(")).count();
        let mut i = 0;
        while i < lines.len() {
            let t = lines[i].trim();
            if !t.starts_with("#[tool(") {
                i += 1;
                continue;
            }
            // Two forms in-tree: single-line and `#[tool(` … `)]` spanning
            // lines. Collect the whole macro, then pull the description.
            let mut end = i;
            let mut macro_text = t.to_string();
            while !lines[end].trim().ends_with(")]") {
                end += 1;
                let l = lines
                    .get(end)
                    .unwrap_or_else(|| panic!("unterminated #[tool(] in {path:?}"));
                macro_text.push(' ');
                macro_text.push_str(l.trim());
            }
            let desc = extract_description(&macro_text)
                .unwrap_or_else(|| panic!("unparsed #[tool(] (extend parser) in {path:?}: {macro_text}"));
            // Methods are `pub async fn` or plain `async fn`. Stop at the
            // first fn line and never step over another `#[tool(`.
            let mut j = end + 1;
            let name = loop {
                let l = lines
                    .get(j)
                    .unwrap_or_else(|| panic!("no fn after #[tool] in {path:?}"))
                    .trim();
                assert!(!l.starts_with("#[tool("), "#[tool( with no fn before line {} in {path:?}", j + 1);
                if let Some(name) = fn_name(l) {
                    break name;
                }
                j += 1;
            };
            out.push((name, desc));
            i = j + 1;
        }
    }
    assert_eq!(out.len(), raw_attrs, "parser dropped #[tool( attributes");
    out
}

/// Name of a `[pub] [async] fn <name>` declaration line, if this is one.
fn fn_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("pub ").unwrap_or(line);
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    let rest = rest.strip_prefix("fn ")?;
    Some(rest.split(['(', '<']).next().unwrap_or("").trim().to_string())
}

/// `description = "…"` with the quotes as delimiters. Description text
/// containing an escaped quote is not currently in-tree; the None path
/// panics at the call site so the check can never silently pass.
fn extract_description(macro_text: &str) -> Option<String> {
    let start = macro_text.find("description = \"")? + "description = \"".len();
    let rest = &macro_text[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[test]
fn d1_documented_tool_counts_match_registry() {
    let tools = parse_tools();
    let count = tools.len();
    assert!(count > 0, "no tools parsed");
    let agents = std::fs::read_to_string(repo_path("AGENTS.md")).unwrap();
    assert!(
        agents.contains(&format!("{count} MCP tools")),
        "AGENTS.md must state the true MCP tool count ({count} MCP tools)"
    );
    let providers =
        std::fs::read_to_string(repo_path("skills/social-forge-agent/references/providers.md"))
            .unwrap();
    assert!(
        providers.contains(&format!("{count} tools")),
        "providers.md must state the true tool count ({count} tools)"
    );
    let readme = std::fs::read_to_string(repo_path("README.md")).unwrap();
    assert!(
        readme.contains(&format!("{count} MCP tools")),
        "README.md must state the true MCP tool count ({count} MCP tools)"
    );
    assert!(
        !readme.contains("327 tools") && !readme.contains("327 MCP"),
        "README.md still states the stale 327-tool count"
    );
}

#[test]
fn d1_documented_cli_command_count_matches_clap() {
    use clap::CommandFactory;
    let commands = social_forge::cli::Cli::command()
        .get_subcommands()
        .filter(|c| c.get_name() != "help")
        .count();
    assert!(commands > 0, "no CLI commands parsed");
    let readme = std::fs::read_to_string(repo_path("README.md")).unwrap();
    assert!(
        readme.contains(&format!("clap, {commands} commands")),
        "README.md must state the true CLI command count ({commands} commands)"
    );
}

#[test]
fn d2_tool_schema_shape() {
    let tools = parse_tools();
    let mut names: Vec<&str> = Vec::with_capacity(tools.len());
    for (name, desc) in &tools {
        assert!(!desc.is_empty(), "{name}: empty description");
        // Schema-size discipline (17.9k chars total today; median 521B):
        // one tight line per tool keeps tools/list affordable.
        assert!(
            desc.len() <= 240,
            "{name}: description {} chars > 240 — trim it",
            desc.len()
        );
        // Provider-prefixed names: the first `_` segment is the area
        // (posts_, x_, reddit_, tu_, …). tools_search and tools_enable
        // are the documented meta exceptions.
        assert!(name.contains('_'), "{name}: no area prefix");
        assert!(!name.starts_with('_'), "{name}: leading underscore");
        names.push(name);
    }
    let mut sorted = names.clone();
    sorted.sort_unstable();
    let before = sorted.len();
    sorted.dedup();
    assert_eq!(before, sorted.len(), "duplicate tool names registered");
}
