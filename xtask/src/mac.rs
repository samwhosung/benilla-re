//! The Mac client's rows against the reader's own slice (`symbols/README.md`, The Mac client):
//! the i386 slice of the 1.12.1 (5875) `World of Warcraft.app`, identified by its hash. Run by
//! `cargo xtask check --exe` when there are Mac rows; without the slice the rows are left
//! unchecked and the summary says so, unless `BENILLA_RE_REQUIRE_MAC=1` makes that a problem.
//! Nothing here reads `WoW.exe`, and nothing in the `WoW.exe` half reads the slice.
//!
//! A function row lies in `__TEXT,__text` and starts with the frame prologue `55 89 e5` every
//! function with an unwind entry opens with. Its size is the range of the slice's own unwind entry
//! for that start (an FDE of `__TEXT,__eh_frame`) when there is one; a function with none ends where
//! at most 15 padding bytes (`00` or `90`) lead to the next prologue or the section's end. A slice
//! whose unwind entries do not read is a problem, never a fall-back to that weaker test. A global
//! row lies inside one section other than `__text`.

use crate::check::{Checked, Problem, Row};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The slice every Mac finding is read from, as `tools/exe.env` gives it.
const IDENTITY: &str = include_str!("../../tools/exe.env");

fn identity(key: &str) -> &'static str {
    IDENTITY
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("tools/exe.env sets {key}"))
}

/// Names the slice in place of `WoW/WoW-mac-i386`.
pub const MAC_VARIABLE: &str = "BENILLA_RE_MAC_SLICE";
/// Set (to `1`), a missing slice is a problem and a test that needs it panics.
pub const REQUIRE_MAC_VARIABLE: &str = "BENILLA_RE_REQUIRE_MAC";

/// Where the slice is: `$BENILLA_RE_MAC_SLICE` when set, else `WoW/WoW-mac-i386` in the repo.
pub fn locate(root: &Path) -> PathBuf {
    match env::var_os(MAC_VARIABLE) {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => root.join("WoW").join("WoW-mac-i386"),
    }
}

fn required() -> bool {
    env::var(REQUIRE_MAC_VARIABLE).is_ok_and(|v| !v.is_empty() && v != "0")
}

/// One section of the slice: its segment and section names, addresses and file bytes.
#[derive(Debug, PartialEq, Eq)]
pub struct Section {
    pub segment: String,
    pub name: String,
    pub address: u64,
    pub size: u64,
    /// The file offset of its bytes; `None` for a zero-filled section.
    pub offset: Option<u64>,
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn name_at(bytes: &[u8], at: usize) -> Option<String> {
    let raw = bytes.get(at..at + 16)?;
    let end = raw.iter().position(|&b| b == 0).unwrap_or(16);
    Some(String::from_utf8_lossy(&raw[..end]).into_owned())
}

/// The sections of a 32-bit little-endian Mach-O, from its `LC_SEGMENT` commands.
pub fn sections(bytes: &[u8]) -> Option<Vec<Section>> {
    if u32_at(bytes, 0)? != 0xfeed_face {
        return None;
    }
    let commands = u32_at(bytes, 16)?;
    let mut at = 28;
    let mut found = Vec::new();
    for _ in 0..commands {
        let (command, size) = (u32_at(bytes, at)?, u32_at(bytes, at + 4)? as usize);
        if command == 1 {
            for i in 0..u32_at(bytes, at + 48)? as usize {
                let s = at + 56 + 68 * i;
                let flags = u32_at(bytes, s + 56)?;
                // S_ZEROFILL, S_GB_ZEROFILL: no file bytes.
                let zero_fill = matches!(flags & 0xff, 1 | 0xc);
                found.push(Section {
                    segment: name_at(bytes, s + 16)?,
                    name: name_at(bytes, s)?,
                    address: u64::from(u32_at(bytes, s + 32)?),
                    size: u64::from(u32_at(bytes, s + 36)?),
                    offset: (!zero_fill).then_some(u64::from(u32_at(bytes, s + 40)?)),
                });
            }
        }
        at += size;
    }
    Some(found)
}

fn uleb(bytes: &[u8], at: &mut usize) -> Option<u64> {
    let (mut value, mut shift) = (0u64, 0);
    loop {
        let byte = *bytes.get(*at)?;
        *at += 1;
        value |= u64::from(byte & 0x7f) << shift;
        shift += 7;
        if byte < 0x80 {
            return Some(value);
        }
    }
}

/// The function ranges of the unwind entries in `__TEXT,__eh_frame`: each FDE's start and length,
/// by start. The slice's CIEs encode an FDE's start relative to where it is stored (`0x10`).
pub fn unwind_ranges(bytes: &[u8], sections: &[Section]) -> Option<HashMap<u64, u64>> {
    let frame = sections
        .iter()
        .find(|s| s.segment == "__TEXT" && s.name == "__eh_frame")?;
    let base = frame.offset? as usize;
    let data = bytes.get(base..base + frame.size as usize)?;
    let mut encodings: HashMap<usize, u8> = HashMap::new();
    let mut ranges = HashMap::new();
    let mut at = 0;
    while at + 4 <= data.len() {
        let length = u32_at(data, at)? as usize;
        if length == 0 {
            at += 4;
            continue;
        }
        let start = at + 4;
        let id = u32_at(data, start)? as usize;
        if id == 0 {
            let mut p = start + 5;
            let end = p + data.get(p..)?.iter().position(|&b| b == 0)?;
            let augmentation = data.get(p..end)?.to_vec();
            p = end + 1;
            uleb(data, &mut p)?; // code alignment
            uleb(data, &mut p)?; // data alignment, as an unsigned read: only its length matters
            p += 1; // return register
            let mut encoding = 0;
            if augmentation.first() == Some(&b'z') {
                uleb(data, &mut p)?;
                for &c in &augmentation[1..] {
                    match c {
                        b'R' => {
                            encoding = *data.get(p)?;
                            p += 1;
                        }
                        b'P' => p += 5,
                        b'L' => p += 1,
                        _ => return None,
                    }
                }
            }
            encodings.insert(at, encoding);
        } else {
            let encoding = *encodings.get(&(start - id))?;
            if encoding & 0x0f != 0 && encoding & 0x0f != 0x0b {
                return None; // only absolute and signed 4-byte pointers occur
            }
            let raw = u32_at(data, start + 4)?;
            let begin = if encoding & 0x70 == 0x10 {
                (frame.address + (start + 4) as u64).wrapping_add(u64::from(raw) as i32 as u64)
                    & 0xffff_ffff
            } else {
                u64::from(raw)
            };
            ranges.insert(begin, u64::from(u32_at(data, start + 8)?));
        }
        at = start + length;
    }
    Some(ranges)
}

fn problem(place: impl Into<String>, message: impl Into<String>) -> Problem {
    Problem {
        place: place.into(),
        message: message.into(),
    }
}

fn address(row: &Row) -> Option<u64> {
    u64::from_str_radix(row.get("address").strip_prefix("mac:")?, 16).ok()
}

/// The bytes of the slice at `address`, when a file-backed section holds all `n` of them.
fn bytes_at<'a>(bytes: &'a [u8], sections: &[Section], address: u64, n: u64) -> Option<&'a [u8]> {
    let section = sections
        .iter()
        .find(|s| s.address <= address && address + n <= s.address + s.size)?;
    let at = (section.offset? + address - section.address) as usize;
    bytes.get(at..at + n as usize)
}

const PROLOGUE: [u8; 3] = [0x55, 0x89, 0xe5];

/// Every Mac row against the slice's sections and unwind entries.
pub fn check_rows(checked: &Checked, bytes: &[u8]) -> Vec<Problem> {
    let Some(sections) = sections(bytes) else {
        return vec![problem("WoW-mac-i386", "is not a 32-bit Mach-O")];
    };
    let Some(ranges) = unwind_ranges(bytes, &sections).filter(|r| !r.is_empty()) else {
        return vec![problem(
            "WoW-mac-i386",
            "has no unwind entries this check reads in __TEXT,__eh_frame",
        )];
    };
    let text = sections
        .iter()
        .find(|s| s.segment == "__TEXT" && s.name == "__text");
    let mut found = Vec::new();
    for row in &checked.mac_functions {
        let (Some(start), Ok(size)) = (address(row), row.get("size").parse::<u64>()) else {
            continue; // the exe-free check reports the address and the size
        };
        let end = start + size;
        let Some(text) = text.filter(|t| t.address <= start && end <= t.address + t.size) else {
            found.push(problem(row.place(), "does not lie in __TEXT,__text"));
            continue;
        };
        if bytes_at(bytes, &sections, start, 3) != Some(&PROLOGUE[..]) {
            found.push(problem(
                row.place(),
                "does not start with the frame prologue 55 89 e5",
            ));
            continue;
        }
        match ranges.get(&start) {
            Some(&range) if range != size => found.push(problem(
                row.place(),
                format!("size {size} differs from the slice's unwind entry, {range}"),
            )),
            Some(_) => {}
            None => {
                let limit = text.address + text.size;
                let mut at = end;
                while at < limit
                    && at < end + 16
                    && matches!(bytes_at(bytes, &sections, at, 1), Some([0x00 | 0x90]))
                {
                    at += 1;
                }
                let next = at == limit
                    || (at < end + 16 && bytes_at(bytes, &sections, at, 3) == Some(&PROLOGUE[..]));
                if !next {
                    found.push(problem(
                        row.place(),
                        format!(
                            "has no unwind entry, and no prologue follows its end mac:{end:06x}"
                        ),
                    ));
                }
            }
        }
    }
    for row in &checked.mac_globals {
        let Some(start) = address(row) else { continue };
        let Some(size) = crate::check::type_size(row.get("type"), &HashMap::new()) else {
            continue; // the exe-free check reports the type
        };
        let holder = sections
            .iter()
            .find(|s| s.address <= start && start + size <= s.address + s.size);
        match holder {
            None => found.push(problem(
                row.place(),
                "lies in no single section of the slice",
            )),
            Some(s) if s.name == "__text" => {
                found.push(problem(row.place(), "lies in __TEXT,__text, which is code"))
            }
            Some(_) => {}
        }
    }
    found
}

/// Why `bytes` are not the slice `tools/exe.env` names, or `None` when they are.
fn refusal(bytes: &[u8]) -> Option<String> {
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let size: u64 = identity("MAC_SIZE").parse().expect("MAC_SIZE is a decimal");
    (bytes.len() as u64 != size || digest != identity("MAC_SHA256")).then(|| {
        format!(
            "refused: {} bytes, SHA-256 {digest}; the Mac findings are read from the 1.12.1.5875 \
             i386 slice, {size} bytes, SHA-256 {}",
            bytes.len(),
            identity("MAC_SHA256")
        )
    })
}

/// Identifies the slice and checks the Mac rows against it: the problems, and a summary.
pub fn run(root: &Path, checked: &Checked) -> io::Result<(Vec<Problem>, String)> {
    let rows = checked.mac_functions.len() + checked.mac_globals.len();
    if rows == 0 {
        return Ok((Vec::new(), String::new()));
    }
    let path = locate(root);
    let shown = path
        .strip_prefix(root)
        .unwrap_or(&path)
        .display()
        .to_string();
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let message = format!(
                "no Mac slice; {rows} Mac rows not checked against it (write the i386 slice of \
                 the 1.12.1 World of Warcraft.app to WoW/WoW-mac-i386, or set {MAC_VARIABLE}; \
                 {REQUIRE_MAC_VARIABLE}=1 forbids the skip)"
            );
            return Ok(if required() {
                (
                    vec![problem(shown, message)],
                    "Mac slice missing".to_string(),
                )
            } else {
                (Vec::new(), message)
            });
        }
        Err(error) => return Err(error),
    };
    if let Some(refused) = refusal(&bytes) {
        return Ok((
            vec![problem(shown, refused)],
            "Mac slice refused, no Mac row checked".to_string(),
        ));
    }
    Ok((
        check_rows(checked, &bytes),
        format!("the Mac slice is 1.12.1.5875 i386; {rows} Mac rows against it"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exe::SKIP_LOG_VARIABLE;
    use crate::tree::{load, Memory};

    /// A 32-bit Mach-O of one `__TEXT` segment with `__text` at 0x1000 (16 bytes: a function of 7
    /// bytes, padding, a second function) and `__eh_frame` after it, holding one CIE and one FDE
    /// for the first function, its start stored relative to itself.
    fn synthetic() -> Vec<u8> {
        let mut b = vec![0u8; 0x200];
        let put =
            |b: &mut Vec<u8>, at: usize, v: u32| b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut b, 0, 0xfeed_face);
        put(&mut b, 16, 1); // one load command
        put(&mut b, 28, 1); // LC_SEGMENT
        put(&mut b, 32, 56 + 2 * 68);
        b[36..42].copy_from_slice(b"__TEXT");
        put(&mut b, 28 + 48, 2); // two sections
        let section = |b: &mut Vec<u8>, i: usize, name: &[u8], addr: u32, size: u32, off: u32| {
            let s = 28 + 56 + 68 * i;
            b[s..s + name.len()].copy_from_slice(name);
            b[s + 16..s + 22].copy_from_slice(b"__TEXT");
            b[s + 32..s + 36].copy_from_slice(&addr.to_le_bytes());
            b[s + 36..s + 40].copy_from_slice(&size.to_le_bytes());
            b[s + 40..s + 44].copy_from_slice(&off.to_le_bytes());
        };
        section(&mut b, 0, b"__text", 0x1000, 16, 0x100);
        section(&mut b, 1, b"__eh_frame", 0x1010, 0x30, 0x110);
        b[0x100..0x110].copy_from_slice(&[
            0x55, 0x89, 0xe5, 0x5d, 0xc3, 0x90, 0x90, 0x90, // 0x1000, then padding
            0x55, 0x89, 0xe5, 0x5d, 0xc3, 0x00, 0x00, 0x00, // 0x1008, a leaf
        ]);
        // CIE: length 0x10, id 0, version 1, "zR", code 1, data -4, register 8, aug length 1,
        // encoding 0x10 (relative), then padding.
        b[0x110..0x124].copy_from_slice(&[
            0x10, 0, 0, 0, 0, 0, 0, 0, 1, b'z', b'R', 0, 1, 0x7c, 8, 1, 0x10, 0, 0, 0,
        ]);
        // FDE at 0x124 (address 0x1024): length 0x10, CIE pointer 0x18 back to 0x110, start
        // relative to 0x102c, range 5.
        let start = 0x1000i32 - 0x102c;
        b[0x124..0x128].copy_from_slice(&0x10u32.to_le_bytes());
        b[0x128..0x12c].copy_from_slice(&0x18u32.to_le_bytes());
        b[0x12c..0x130].copy_from_slice(&start.to_le_bytes());
        b[0x130..0x134].copy_from_slice(&5u32.to_le_bytes());
        b
    }

    fn checked(functions: &str, globals: &str) -> Checked {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/clean");
        let mut files = load(&fixture);
        files.insert("symbols/mac/functions.tsv".into(), functions.into());
        files.insert("symbols/mac/globals.tsv".into(), globals.into());
        crate::check::run(&Memory(files)).expect("reads")
    }

    const F: &str = "address\tsize\tname\tname_from\tsystem\tclass\tsignature\tmeaning\tconfidence\tproof\tfinding\ttest\n";
    const G: &str = "address\ttype\tname\tname_from\tsystem\tmeaning\tconfidence\tproof\tfinding\n";

    fn row(address: &str, size: u64) -> String {
        format!("{address}\t{size}\t\t\tunassigned\t\t\tReturns\tinferred\tnone\t\t\n")
    }

    #[test]
    fn the_sections_and_the_unwind_entries_read() {
        let bytes = synthetic();
        let sections = sections(&bytes).expect("a Mach-O");
        assert_eq!(sections.len(), 2);
        assert_eq!(
            (sections[0].address, sections[0].offset),
            (0x1000, Some(0x100))
        );
        let ranges = unwind_ranges(&bytes, &sections).expect("an eh_frame");
        assert_eq!(ranges, HashMap::from([(0x1000, 5)]));
    }

    #[test]
    fn each_row_is_held_to_the_slice() {
        let bytes = synthetic();
        // Right: the unwind entry's 5, and the leaf's 5 before its padding.
        let good = format!("{F}{}{}", row("mac:001000", 5), row("mac:001008", 5));
        let g = format!("{G}mac:001010\tuint32_t\ts_x\t\tunassigned\tHolds\tinferred\tnone\t\n");
        assert_eq!(check_rows(&checked(&good, &g), &bytes), Vec::new());
        // Wrong: a size against the unwind entry, a leaf that stops short, a start off the
        // prologue, a row past __text and a global in code.
        let bad = format!(
            "{F}{}{}{}",
            row("mac:001000", 6),
            row("mac:001008", 3),
            row("mac:00100a", 2)
        );
        let g = format!("{G}mac:001004\tuint32_t\ts_x\t\tunassigned\tHolds\tinferred\tnone\t\n");
        let found: Vec<String> = check_rows(&checked(&bad, &g), &bytes)
            .iter()
            .map(|p| p.message.clone())
            .collect();
        assert_eq!(
            found,
            [
                "size 6 differs from the slice's unwind entry, 5",
                "has no unwind entry, and no prologue follows its end mac:00100b",
                "does not start with the frame prologue 55 89 e5",
                "lies in __TEXT,__text, which is code",
            ]
        );
    }

    /// The unwind entries read, and a slice without them is one problem, never the padding test.
    #[test]
    fn a_slice_without_unwind_entries_is_a_problem() {
        let mut bytes = synthetic();
        bytes[0x124..0x134].fill(0); // the FDE gone
        let good = format!("{F}{}", row("mac:001000", 5));
        let found: Vec<String> = check_rows(&checked(&good, G), &bytes)
            .iter()
            .map(|p| p.message.clone())
            .collect();
        assert_eq!(
            found,
            ["has no unwind entries this check reads in __TEXT,__eh_frame"]
        );
    }

    /// The reader's slice, where `cargo xtask check --exe` finds it, gives the handler the length
    /// of its unwind entry: the control for the check's reading of them. Without the slice it skips,
    /// logged as a Mac skip, and under `BENILLA_RE_REQUIRE_MAC` it fails.
    #[test]
    fn the_slice_gives_the_handler_its_length() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the repo");
        let path = locate(root);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                assert!(
                    !required(),
                    "no Mac slice at {}, and {REQUIRE_MAC_VARIABLE} forbids skipping",
                    path.display()
                );
                let thread = std::thread::current();
                let test = thread.name().unwrap_or("test");
                eprintln!("skip {test}: no Mac slice at {}", path.display());
                if let Some(log) = env::var_os(SKIP_LOG_VARIABLE).filter(|p| !p.is_empty()) {
                    use std::io::Write;
                    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log)
                    {
                        let _ = file.write_all(format!("mac {test}\n").as_bytes());
                    }
                }
                return;
            }
            Err(error) => panic!("the Mac slice at {}: {error}", path.display()),
        };
        assert_eq!(refusal(&bytes), None, "{}", path.display());
        let sections = sections(&bytes).expect("a Mach-O");
        let ranges = unwind_ranges(&bytes, &sections).expect("an eh_frame");
        assert_eq!(ranges.get(&0x89e3e), Some(&3494));
        assert!(ranges.len() > 10_000);
    }
}
