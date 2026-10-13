//! `cargo xtask cited <benilla>`: the client addresses a benilla tree cites that this repo backs
//! with neither a row nor a mention in a doc. A report, never a failure.
//!
//! An address is recognised as `0x5e6850`, `FUN_005e6850` or `DAT_00c81768` (with any leading
//! underscores, as in `_DAT_008029d4`), and kept only when it is one benilla writes as a
//! citation:
//!
//! - inside the image after its headers, from `.text` at 0x401000 to the image's end at
//!   0xd06000 (base 0x400000 plus `SizeOfImage` 0x906000), which drops masks, colours and
//!   counts below or above it;
//! - in lowercase hex, as benilla writes every address; its uppercase literals are colours and
//!   test values;
//! - in prose: in a Rust or WGSL file, inside a comment or a string literal. A literal in code
//!   is a value the code computes with (a flag word, a mask), never a citation.
//!
//! What it drops is counted by reason, so the filter can be audited on every run.

use crate::check::Checked;
use crate::tree;
use regex::Regex;
use std::collections::{BTreeMap, HashSet};
use std::io;
use std::ops::Range;
use std::path::Path;
use std::sync::LazyLock;

/// The image's addresses after its headers.
pub const IMAGE: Range<u64> = 0x401000..0xd06000;

static CANDIDATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:0x([0-9a-fA-F]{6})|_*(?:FUN|DAT)_([0-9a-fA-F]{8}))\b")
        .expect("a valid pattern")
});
static DOC_ADDRESS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b0x([0-9a-f]{6})\b").expect("a valid pattern"));

/// Candidates dropped as not addresses, by reason.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Dropped {
    pub outside: usize,
    pub uppercase: usize,
    pub code: usize,
}

/// One file's citations: each address with the line it is on.
pub fn scan(path: &str, text: &str, dropped: &mut Dropped) -> Vec<(u64, usize)> {
    let prose = if path.ends_with(".rs") || path.ends_with(".wgsl") {
        Some(prose_spans(text.as_bytes()))
    } else {
        None
    };
    let mut found = Vec::new();
    for candidate in CANDIDATE.captures_iter(text) {
        let digits = candidate
            .get(1)
            .or_else(|| candidate.get(2))
            .expect("one form matched")
            .as_str();
        let start = candidate.get(0).expect("the match").start();
        if let Some(spans) = &prose {
            let i = spans.partition_point(|s| s.end <= start);
            if spans.get(i).is_none_or(|s| s.start > start) {
                dropped.code += 1;
                continue;
            }
        }
        if digits.bytes().any(|b| b.is_ascii_uppercase()) {
            dropped.uppercase += 1;
            continue;
        }
        let address = u64::from_str_radix(digits, 16).expect("hex digits");
        if !IMAGE.contains(&address) {
            dropped.outside += 1;
            continue;
        }
        let line = text[..start].matches('\n').count() + 1;
        found.push((address, line));
    }
    found
}

/// The byte ranges of Rust or WGSL source that are prose: comments, nested block comments
/// included, and string literals, raw ones included. Character literals and lifetimes are code.
pub fn prose_spans(b: &[u8]) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let rest = &b[i..];
        let end = if rest.starts_with(b"//") {
            rest.iter()
                .position(|&c| c == b'\n')
                .map_or(b.len(), |n| i + n)
        } else if rest.starts_with(b"/*") {
            let (mut depth, mut j) = (0, i);
            while j < b.len() {
                if b[j..].starts_with(b"/*") {
                    depth += 1;
                    j += 2;
                } else if b[j..].starts_with(b"*/") {
                    depth -= 1;
                    j += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    j += 1;
                }
            }
            j
        } else if let Some(end) = raw_string_end(b, i) {
            end
        } else if b[i] == b'"' {
            let mut j = i + 1;
            while j < b.len() && b[j] != b'"' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            (j + 1).min(b.len())
        } else {
            i = if b[i] == b'\'' {
                char_literal_end(b, i).unwrap_or(i + 1)
            } else {
                i + 1
            };
            continue;
        };
        spans.push(i..end);
        i = end;
    }
    spans
}

/// The end of a raw string literal (`r"…"`, `r#"…"#`, `br"…"`, `cr"…"`) starting at `i`.
fn raw_string_end(b: &[u8], i: usize) -> Option<usize> {
    if i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_') {
        return None;
    }
    let mut j = i;
    if matches!(b.get(j), Some(b'b' | b'c')) {
        j += 1;
    }
    if b.get(j) != Some(&b'r') {
        return None;
    }
    j += 1;
    let hashes = b[j..].iter().take_while(|&&c| c == b'#').count();
    j += hashes;
    if b.get(j) != Some(&b'"') {
        return None;
    }
    j += 1;
    while j < b.len() {
        if b[j] == b'"'
            && b[j + 1..]
                .iter()
                .take(hashes)
                .filter(|&&c| c == b'#')
                .count()
                == hashes
        {
            return Some(j + 1 + hashes);
        }
        j += 1;
    }
    Some(b.len())
}

/// The end of a character literal starting at `i`, or `None` for a lifetime or a label.
fn char_literal_end(b: &[u8], i: usize) -> Option<usize> {
    let j = i + 1;
    match *b.get(j)? {
        b'\\' => {
            let close = b.get(j + 2..)?.iter().take(10).position(|&c| c == b'\'')?;
            Some(j + 2 + close + 1)
        }
        b'\'' | b'\n' => None,
        lead => {
            let width = match lead {
                0x00..=0x7f => 1,
                0xc0..=0xdf => 2,
                0xe0..=0xef => 3,
                _ => 4,
            };
            (b.get(j + width) == Some(&b'\'')).then_some(j + width + 1)
        }
    }
}

/// Prints the report for the benilla checkout at `benilla`.
/// Prints the report and returns the addresses with neither a row nor a doc mention.
pub fn run(checked: &Checked, benilla: &Path) -> io::Result<Vec<u64>> {
    let mut mentioned = HashSet::new();
    for text in checked
        .docs
        .iter()
        .map(|d| &d.text)
        .chain(checked.method.as_ref())
    {
        for found in DOC_ADDRESS.captures_iter(text) {
            mentioned.insert(u64::from_str_radix(&found[1], 16).expect("hex digits"));
        }
    }

    // Each address with its citation count and first place.
    let mut cited: BTreeMap<u64, (usize, String)> = BTreeMap::new();
    let mut dropped = Dropped::default();
    let (mut files, mut citations) = (0, 0);
    for top in ["crates", "assets"] {
        let dir = benilla.join(top);
        if !dir.is_dir() {
            continue;
        }
        for path in tree::files(&dir, &[])? {
            let Ok(text) = std::fs::read_to_string(dir.join(&path)) else {
                continue;
            };
            files += 1;
            let path = format!("{top}/{path}");
            for (address, line) in scan(&path, &text, &mut dropped) {
                citations += 1;
                cited
                    .entry(address)
                    .or_insert_with(|| (0, format!("{path}:{line}")))
                    .0 += 1;
            }
        }
    }
    if citations == 0 {
        return Err(io::Error::other(format!(
            "{}: no citation in {files} files under crates/ and assets/; is it a benilla checkout?",
            benilla.display()
        )));
    }

    let (mut rows, mut docs_only, mut neither) = (0, 0, 0);
    let mut missing = Vec::new();
    for (&address, (count, first)) in &cited {
        if checked.extents.contains(address) {
            rows += 1;
        } else if mentioned.contains(&address) {
            docs_only += 1;
        } else {
            neither += 1;
            missing.push(address);
            println!("{address:#08x}  {count:>5}  {first}");
        }
    }
    println!(
        "{} addresses cited {citations} times in {files} files; {rows} have a row, {docs_only} \
         more are mentioned in a doc, {neither} have neither",
        cited.len()
    );
    println!(
        "dropped as not addresses: {} outside the image, {} in uppercase hex, {} in code",
        dropped.outside, dropped.uppercase, dropped.code
    );
    Ok(missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST: &str = r##"//! Cites `0x5e6850` and FUN_005e6850, then DAT_00c81768 and _DAT_008029d4.
/* outer /* nested 0x401000 */ still 0x401001 */
fn f() {
    let flags = 0x410012; // a flag word in code, while 0x7400e5 is cited
    let s = "the 0x514490 handler";
    let r = r#"raw "0x514491" text"#;
    let q = '"'; let t = 0x514492; // a character literal holding a quote opens no string
    let l: &'static str = "0x514493";
    let colour = 0x8CB9FD; // 0x5393A8 is a colour in uppercase hex
    // 0xffffff 0x400fff 0xd06000 0xd05fff 0x1234567
}
"##;

    #[test]
    fn citations_are_lowercase_image_addresses_in_prose() {
        let mut dropped = Dropped::default();
        let found = scan("crates/x/src/lib.rs", RUST, &mut dropped);
        assert_eq!(
            found,
            [
                (0x5e6850, 1),
                (0x5e6850, 1),
                (0xc81768, 1),
                (0x8029d4, 1),
                (0x401000, 2),
                (0x401001, 2),
                (0x7400e5, 4),
                (0x514490, 5),
                (0x514491, 6),
                (0x514493, 8),
                (0xd05fff, 10),
            ]
        );
        assert_eq!(
            dropped,
            Dropped {
                outside: 3,
                uppercase: 1,
                code: 3
            }
        );
    }

    #[test]
    fn other_files_are_prose_throughout() {
        let mut dropped = Dropped::default();
        let xml = "<!-- the broadcast 0x6d2914 -->\n<Flags value=\"0x410012\"/>\n";
        let found = scan("crates/x/assets/a.xml", xml, &mut dropped);
        assert_eq!(found, [(0x6d2914, 1), (0x410012, 2)]);
        assert_eq!(dropped, Dropped::default());
    }
}
