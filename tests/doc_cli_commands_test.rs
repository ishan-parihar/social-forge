// ─── Doc-to-CLI guard ───────────────────────────────────────────────────
// Every `social-forge …` command the agent skill, its references and the
// README show must parse under the real clap definitions. Agents copy these
// lines verbatim, so a documented flag the CLI rejects is a silent failure
// for every agent that reads the doc.
//
// Only fenced code blocks and inline backtick spans are read, so prose that
// mentions the binary is ignored. Placeholders such as `<UUID>` become a
// fixed valid UUID, and lines that need a shell (`$VAR`, pipes, `&&`,
// command substitution) are skipped rather than guessed at.

use clap::Parser;
use social_forge::cli::Cli;
use std::path::PathBuf;

const PLACEHOLDER: &str = "00000000-0000-0000-0000-000000000001";

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Documents whose commands are checked: the skill, its references, README.
fn docs() -> Vec<String> {
    let mut out = vec![
        "README.md".to_string(),
        "skills/social-forge-agent/SKILL.md".to_string(),
    ];
    for entry in std::fs::read_dir(repo_path("skills/social-forge-agent/references"))
        .expect("read references")
    {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let rel = path
                .strip_prefix(repo_path(""))
                .expect("under repo")
                .to_string_lossy()
                .into_owned();
            out.push(rel);
        }
    }
    out.sort();
    out
}

/// (1-based line, command text after `social-forge `) for each documented command.
fn doc_commands(rel: &str) -> Vec<(usize, String)> {
    let text = std::fs::read_to_string(repo_path(rel)).expect("read doc");
    let mut out = Vec::new();
    let mut in_fence = false;
    for (idx, line) in text.lines().enumerate() {
        let t = line.trim();
        if t.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            if let Some(rest) = t.strip_prefix("social-forge ") {
                out.push((idx + 1, rest.to_string()));
            }
        } else {
            // Inline code: odd-indexed segments between backticks.
            for span in t.split('`').skip(1).step_by(2) {
                if let Some(rest) = span.strip_prefix("social-forge ") {
                    out.push((idx + 1, rest.to_string()));
                }
            }
        }
    }
    out
}

/// Cut a documented line at shell syntax we do not model. Returns None for
/// lines that only make sense inside a shell.
fn trim_shell(cmd: &str) -> Option<String> {
    if cmd.contains('$') || cmd.contains("...") {
        return None;
    }
    let mut quote: Option<char> = None;
    let bytes: Vec<char> = cmd.chars().collect();
    let mut cut = bytes.len();
    for (i, &c) in bytes.iter().enumerate() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '|' || c == ';' || c == '>' => {
                cut = i;
                break;
            }
            None if c == '&' && bytes.get(i + 1) == Some(&'&') => {
                cut = i;
                break;
            }
            None if c == '#' && (i == 0 || bytes[i - 1].is_whitespace()) => {
                cut = i;
                break;
            }
            None => {}
        }
    }
    let kept: String = bytes[..cut].iter().collect();
    Some(kept.trim().to_string())
}

/// Split a command line into argv, honouring double and single quotes.
fn tokenize(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut has_token = false;
    let mut quote: Option<char> = None;
    for c in cmd.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => cur.push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                has_token = true;
            }
            None if c.is_whitespace() => {
                if has_token {
                    args.push(std::mem::take(&mut cur));
                    has_token = false;
                }
            }
            None => {
                cur.push(c);
                has_token = true;
            }
        }
    }
    if has_token {
        args.push(cur);
    }
    args
}

/// Replace `<PLACEHOLDER>` tokens with a value that parses for the argument
/// type: `<..NUMBER..>` becomes an integer, everything else a valid UUID.
fn fill_placeholders(cmd: &str) -> String {
    let mut out = String::new();
    let mut rest = cmd;
    while let Some(start) = rest.find('<') {
        match rest[start..].find('>') {
            Some(len)
                if rest[start + 1..start + len]
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c == '_' || c == ' ') =>
            {
                out.push_str(&rest[..start]);
                let name = &rest[start + 1..start + len];
                out.push_str(if name.contains("NUMBER") {
                    "1"
                } else {
                    PLACEHOLDER
                });
                rest = &rest[start + len + 1..];
            }
            _ => {
                out.push_str(&rest[..=start]);
                rest = &rest[start + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[test]
fn every_documented_social_forge_command_parses() {
    let mut checked = 0usize;
    let mut failures = Vec::new();
    for doc in docs() {
        for (line, raw) in doc_commands(&doc) {
            // Fill placeholders before trimming: `<UUID>` contains `>`, which
            // trim_shell would otherwise read as a redirect.
            let filled = fill_placeholders(&raw);
            // A `<lowercase>` template (e.g. `<command>`) is not a runnable line.
            if filled.contains('<') {
                continue;
            }
            let Some(trimmed) = trim_shell(&filled) else {
                continue;
            };
            if trimmed.is_empty() {
                continue;
            }
            let argv = std::iter::once("social-forge".to_string()).chain(tokenize(&trimmed));
            checked += 1;
            if let Err(e) = Cli::try_parse_from(argv) {
                use clap::error::ErrorKind;
                if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                    continue;
                }
                let why = e.to_string().lines().next().unwrap_or_default().to_string();
                failures.push(format!("{doc}:{line}: `social-forge {raw}` -> {why}"));
            }
        }
    }
    assert!(
        checked >= 40,
        "only {checked} documented commands were checked; the extractor is broken"
    );
    assert!(
        failures.is_empty(),
        "{} documented command(s) the CLI rejects:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn trim_shell_keeps_only_the_command_part() {
    assert_eq!(
        trim_shell("x post \"hi\" | jq .").as_deref(),
        Some("x post \"hi\"")
    );
    assert_eq!(
        trim_shell("posts list --limit 5 # newest").as_deref(),
        Some("posts list --limit 5")
    );
    assert_eq!(
        trim_shell("providers --pretty && echo ok").as_deref(),
        Some("providers --pretty")
    );
    assert!(trim_shell("posts create \"$TEXT\"").is_none());
}

#[test]
fn tokenize_keeps_quoted_arguments_whole() {
    assert_eq!(
        tokenize("carousel \"5 Tips for Devs\" --media a.jpg,b.jpg"),
        vec!["carousel", "5 Tips for Devs", "--media", "a.jpg,b.jpg"]
    );
}
