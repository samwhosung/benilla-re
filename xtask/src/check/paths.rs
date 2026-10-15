//! Every Markdown file's pointers to other files of the repo resolve: `[text](path)` links, and a
//! backticked path that starts in a folder of the repo (`docs/METHOD.md`, `tools/callers.py`) or
//! names a Markdown file. A path is tried from the repo root and from the file's own folder.
//! Generated and local folders (`dumps/`, `WoW/`, `target/`) are not in the tree and are skipped.

use super::Problems;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

/// The folders of the repo a backticked path may start in.
const FOLDERS: [&str; 7] = [
    "docs/", "symbols/", "systems/", "tables/", "tools/", "crates/", "xtask/",
];
/// Paths that are not in the tree: made on the reader's machine or by a script, or in a hidden
/// folder, which the walk does not enter.
const LOCAL: [&str; 7] = [
    "dumps/",
    "WoW/",
    "target/",
    ".github/",
    "tools/capture/build",
    "tools/capture/out",
    "xtask/fixtures/",
];
/// Files that are templates for another place: their paths are that place's.
const TEMPLATES: [&str; 1] = ["tools/export/README.md"];

static LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\]\(([^)\s]+)\)").expect("a valid pattern"));
static CODE_SPAN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`[^`]*`").expect("a valid pattern"));
static TICKED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`([A-Za-z0-9_./-]+)`").expect("a valid pattern"));

/// `path` resolved against `folder`, with `.` and `..` folded; `None` when it climbs out of the repo.
fn resolve(folder: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = folder.split('/').filter(|s| !s.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    Some(parts.join("/"))
}

pub fn check(files: &[String], texts: &[(&String, String)], p: &mut Problems) {
    let known: HashSet<&str> = files.iter().map(String::as_str).collect();
    let exists = |path: &str| {
        known.contains(path)
            || files.iter().any(|f| {
                f.strip_prefix(path)
                    .is_some_and(|rest| rest.starts_with('/'))
            })
    };
    for (path, text) in texts {
        if TEMPLATES.contains(&path.as_str()) {
            continue;
        }
        let folder = path.rsplit_once('/').map_or("", |(dir, _)| dir);
        let mut in_code = false;
        for (index, line) in text.split('\n').enumerate() {
            if line.starts_with("```") {
                in_code = !in_code;
            }
            if in_code {
                continue;
            }
            let place = format!("{path}:{}", index + 1);
            let prose = CODE_SPAN.replace_all(line, "");
            for link in LINK.captures_iter(&prose) {
                let target = link[1].split('#').next().unwrap_or_default();
                if target.is_empty() || target.contains("://") || target.starts_with("mailto:") {
                    continue;
                }
                match resolve(folder, target) {
                    Some(found) if exists(&found) => {}
                    Some(found) if LOCAL.iter().any(|l| found.starts_with(l)) => {}
                    _ => p.add(&place, format!("links to {target}, which is no file here")),
                }
            }
            for ticked in TICKED.captures_iter(line) {
                let token = ticked[1].trim_end_matches('/');
                let named = FOLDERS.iter().any(|f| token.starts_with(f))
                    || token.ends_with(".md") && !token.starts_with('.');
                if !named || LOCAL.iter().any(|l| token.starts_with(l)) {
                    continue;
                }
                // A doc or a table named without its extension (`systems/README`, `symbols/functions`).
                let there = |x: &str| {
                    exists(x) || exists(&format!("{x}.md")) || exists(&format!("{x}.tsv"))
                };
                let found = there(token) || resolve(folder, token).is_some_and(|f| there(&f));
                if !found {
                    p.add(&place, format!("`{token}` is no file here"));
                }
            }
        }
    }
}
