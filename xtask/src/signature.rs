//! Masked byte signatures: for each function row, the bytes of its body with every byte that
//! depends on where the image sits replaced by a wildcard, cut to the shortest prefix that
//! occurs once in the executable sections. `cargo xtask signatures` writes them as
//! `tables/signatures.tsv`; `matcher.rs` finds them in another build.
//!
//! A pattern is hex bytes separated by spaces, `??` for a wildcard: `55 8b ec 83 ?? ?? ?? ?? ??`.
//!
//! The 5875 image has no base relocation table, so the wildcards come from the decoded
//! instructions (`dumps/disasm.txt`):
//!
//! - the rel32 of a `call`, `jmp` or conditional jump whose target lies outside the function
//!   (a rel8 that leaves the function too);
//! - every four-byte operand whose value is an address in the image: a global, a string, a
//!   vtable, a jump table;
//! - in the bytes the listing leaves out after a jump, every aligned dword that is an address in
//!   the image: the jump table.

use crate::bodies::Listing;
use regex::Regex;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::LazyLock;

/// The section flag for code that runs.
const EXECUTE: u32 = 0x2000_0000;
/// The longest an x86 instruction is; a listing gap longer than this holds data.
const LONGEST: u32 = 15;
/// A pattern is at least this long when the body is.
pub const MINIMUM: usize = 16;

/// A pattern: a byte, or `None` for a wildcard.
pub type Pattern = Vec<Option<u8>>;

/// One section of a PE: where it is mapped, its flags and the bytes the file holds for it.
pub struct Section<'a> {
    pub va: u32,
    pub flags: u32,
    pub data: &'a [u8],
}

/// The parts of a PE the signatures need.
pub struct Pe<'a> {
    pub base: u32,
    pub image_size: u32,
    pub sections: Vec<Section<'a>>,
}

impl<'a> Pe<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Pe<'a>, String> {
        let u16_at = |o: usize| -> Option<u16> {
            Some(u16::from_le_bytes(bytes.get(o..o + 2)?.try_into().ok()?))
        };
        let u32_at = |o: usize| -> Option<u32> {
            Some(u32::from_le_bytes(bytes.get(o..o + 4)?.try_into().ok()?))
        };
        let bad = || "not a PE file".to_string();
        if bytes.get(..2) != Some(b"MZ") {
            return Err(bad());
        }
        let pe = u32_at(0x3c).ok_or_else(bad)? as usize;
        if bytes.get(pe..pe + 4) != Some(b"PE\0\0") {
            return Err(bad());
        }
        let count = u16_at(pe + 6).ok_or_else(bad)? as usize;
        let optional = u16_at(pe + 20).ok_or_else(bad)? as usize;
        let base = u32_at(pe + 52).ok_or_else(bad)?;
        let image_size = u32_at(pe + 80).ok_or_else(bad)?;
        let table = pe + 24 + optional;
        let mut sections = Vec::new();
        for i in 0..count {
            let h = table + 40 * i;
            let virtual_size = u32_at(h + 8).ok_or_else(bad)?;
            let rva = u32_at(h + 12).ok_or_else(bad)?;
            let raw_size = u32_at(h + 16).ok_or_else(bad)?;
            let raw = u32_at(h + 20).ok_or_else(bad)? as usize;
            let flags = u32_at(h + 36).ok_or_else(bad)?;
            let length = raw_size.min(virtual_size) as usize;
            let data = bytes.get(raw..raw + length).ok_or_else(bad)?;
            sections.push(Section {
                va: base.wrapping_add(rva),
                flags,
                data,
            });
        }
        Ok(Pe {
            base,
            image_size,
            sections,
        })
    }

    /// The sections that hold code.
    pub fn executable(&self) -> Vec<&Section<'a>> {
        self.sections
            .iter()
            .filter(|s| s.flags & EXECUTE != 0)
            .collect()
    }

    /// The bytes `[address, address + length)` when one section holds them.
    pub fn read(&self, address: u32, length: u32) -> Option<&'a [u8]> {
        let (address, length) = (u64::from(address), u64::from(length));
        self.sections.iter().find_map(|s| {
            let start = u64::from(s.va);
            if start <= address && address + length <= start + s.data.len() as u64 {
                let at = (address - start) as usize;
                s.data.get(at..at + length as usize)
            } else {
                None
            }
        })
    }
}

/// An index over the bytes of one section: every position by the four bytes there, and by the
/// byte there.
struct Index<'a> {
    va: u32,
    data: &'a [u8],
    words: Vec<(u32, u32)>,
    bytes: Vec<Vec<u32>>,
}

impl<'a> Index<'a> {
    fn new(section: &Section<'a>) -> Index<'a> {
        let data = section.data;
        let mut words: Vec<(u32, u32)> = data
            .windows(4)
            .enumerate()
            .map(|(i, w)| (u32::from_le_bytes([w[0], w[1], w[2], w[3]]), i as u32))
            .collect();
        words.sort_unstable();
        let mut bytes = vec![Vec::new(); 256];
        for (i, &b) in data.iter().enumerate() {
            bytes[b as usize].push(i as u32);
        }
        Index {
            va: section.va,
            data,
            words,
            bytes,
        }
    }

    fn at_start(&self, pattern: &[Option<u8>], p: usize) -> bool {
        p + pattern.len() <= self.data.len()
            && pattern
                .iter()
                .zip(&self.data[p..])
                .all(|(want, &have)| want.is_none_or(|w| w == have))
    }

    /// Addresses where `pattern` matches, at most `cap`, in address order.
    fn find(&self, pattern: &[Option<u8>], cap: usize, out: &mut Vec<u32>) {
        let at_start = |p: usize| self.at_start(pattern, p);
        let run = (0..pattern.len().saturating_sub(3))
            .find(|&k| pattern[k..k + 4].iter().all(Option::is_some));
        let mut found = Vec::new();
        if let Some(k) = run {
            let key = u32::from_le_bytes([
                pattern[k].unwrap_or(0),
                pattern[k + 1].unwrap_or(0),
                pattern[k + 2].unwrap_or(0),
                pattern[k + 3].unwrap_or(0),
            ]);
            let first = self.words.partition_point(|&(w, _)| w < key);
            for &(w, pos) in &self.words[first..] {
                if w != key {
                    break;
                }
                if let Some(p) = (pos as usize).checked_sub(k) {
                    if at_start(p) {
                        found.push(p as u32);
                    }
                }
            }
        } else if let Some(j) = pattern.iter().position(Option::is_some) {
            let b = pattern[j].unwrap_or(0);
            for &pos in &self.bytes[b as usize] {
                if let Some(p) = (pos as usize).checked_sub(j) {
                    if at_start(p) {
                        found.push(p as u32);
                    }
                }
            }
        }
        found.sort_unstable();
        for p in found {
            if out.len() >= cap {
                break;
            }
            out.push(self.va + p);
        }
    }
}

/// The executable sections of an image, ready to be searched.
pub struct Scanner<'a> {
    indexes: Vec<Index<'a>>,
}

impl<'a> Scanner<'a> {
    pub fn new(pe: &Pe<'a>) -> Scanner<'a> {
        Scanner {
            indexes: pe.executable().into_iter().map(Index::new).collect(),
        }
    }

    /// Whether the pattern matches at `address`.
    pub fn matches_at(&self, pattern: &[Option<u8>], address: u32) -> bool {
        self.indexes.iter().any(|i| {
            address
                .checked_sub(i.va)
                .is_some_and(|p| i.at_start(pattern, p as usize))
        })
    }

    /// Where the pattern matches, the first `cap` addresses.
    pub fn find(&self, pattern: &[Option<u8>], cap: usize) -> Vec<u32> {
        let mut out = Vec::new();
        for index in &self.indexes {
            if out.len() >= cap {
                break;
            }
            index.find(pattern, cap, &mut out);
        }
        out
    }
}

static HEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"0x([0-9a-f]{1,8})\b").expect("valid"));
static DIRECT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^0x([0-9a-f]{1,8})$").expect("valid"));
const PREFIXES: [&str; 8] = [
    "rep", "repz", "repnz", "repe", "repne", "lock", "data16", "addr16",
];

fn is_branch(mnemonic: &str) -> bool {
    mnemonic == "call"
        || mnemonic == "jmp"
        || mnemonic.starts_with('j')
        || mnemonic.starts_with("loop")
}

/// The body `[start, start + size)` as a pattern, with its wildcards.
pub fn mask(pe: &Pe, listing: &Listing, start: u32, size: u32) -> Option<Pattern> {
    let body = pe.read(start, size)?;
    let end = start + size;
    let mut pattern: Pattern = body.iter().map(|&b| Some(b)).collect();
    let image = pe.base..pe.base.wrapping_add(pe.image_size);
    let in_image = |v: u32| image.contains(&v);
    let wild = |pattern: &mut Pattern, from: usize, to: usize| {
        for slot in pattern.iter_mut().take(to).skip(from) {
            *slot = None;
        }
    };
    let mut k = listing.index(start)?;
    while let Some(&a) = listing.addresses.get(k) {
        if a >= end {
            break;
        }
        let next = listing.addresses.get(k + 1).copied().unwrap_or(end);
        let lo = (a - start) as usize;
        let hi = (next.min(end) - start) as usize;
        let text = listing.at(k);
        let mut rest = text;
        let mut mnemonic;
        let mut operands;
        loop {
            let (m, o) = rest.split_once(' ').unwrap_or((rest, ""));
            mnemonic = m;
            operands = o;
            if PREFIXES.contains(&m) && !o.is_empty() {
                rest = o;
            } else {
                break;
            }
        }
        let code = &body[lo..hi.min(lo + LONGEST as usize)];
        if is_branch(mnemonic) {
            if let Some(target) = DIRECT
                .captures(operands)
                .and_then(|c| u32::from_str_radix(&c[1], 16).ok())
            {
                if target < start || target >= end {
                    let rel32 = match code {
                        [0xe8 | 0xe9, ..] if code.len() >= 5 => Some(lo + 1),
                        [0x0f, b, ..] if b & 0xf0 == 0x80 && code.len() >= 6 => Some(lo + 2),
                        _ => None,
                    };
                    if let Some(at) = rel32 {
                        wild(&mut pattern, at, at + 4);
                    } else if code.len() == 2 {
                        wild(&mut pattern, lo + 1, lo + 2);
                    }
                }
            }
        } else {
            for cap in HEX.captures_iter(operands) {
                let Ok(v) = u32::from_str_radix(&cap[1], 16) else {
                    continue;
                };
                if !in_image(v) {
                    continue;
                }
                let needle = v.to_le_bytes();
                let mut j = 0;
                while j + 4 <= code.len() {
                    if code[j..j + 4] == needle {
                        wild(&mut pattern, lo + j, lo + j + 4);
                    }
                    j += 1;
                }
            }
        }
        if (hi - lo) as u32 > LONGEST {
            for at in lo..hi.saturating_sub(3) {
                if (start + at as u32).is_multiple_of(4) {
                    let v = u32::from_le_bytes(body[at..at + 4].try_into().ok()?);
                    if in_image(v) {
                        wild(&mut pattern, at, at + 4);
                    }
                }
            }
        }
        k += 1;
    }
    Some(pattern)
}

/// One row's signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    pub address: u32,
    /// The pattern occurs once in the executable sections.
    pub unique: bool,
    pub pattern: Pattern,
}

/// The pattern in text.
pub fn show(pattern: &[Option<u8>]) -> String {
    let mut out = String::new();
    for (i, b) in pattern.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        match b {
            Some(b) => {
                let _ = write!(out, "{b:02x}");
            }
            None => out.push_str("??"),
        }
    }
    out
}

/// A pattern from its text.
pub fn read_pattern(text: &str) -> Result<Pattern, String> {
    text.split(' ')
        .map(|w| {
            if w == "??" {
                Ok(None)
            } else if w.len() == 2 {
                u8::from_str_radix(w, 16)
                    .map(Some)
                    .map_err(|_| format!("'{w}' is not a byte"))
            } else {
                Err(format!("'{w}' is not a byte"))
            }
        })
        .collect()
}

pub const HEADER: &str = "address\tunique\tpattern\n";

/// The signature of every `(address, size)` row, in the order given.
pub fn generate(pe: &Pe, listing: &Listing, rows: &[(u32, u32)]) -> Result<Vec<Signature>, String> {
    let scanner = Scanner::new(pe);
    let mut out = Vec::new();
    let mut seen: HashMap<Pattern, bool> = HashMap::new();
    for &(address, size) in rows {
        let whole = mask(pe, listing, address, size).ok_or_else(|| {
            format!("{address:#08x}: the body is not in the image or the listing")
        })?;
        if !scanner.matches_at(&whole, address) {
            return Err(format!("{address:#08x}: its own pattern does not match it"));
        }
        let unique = *seen
            .entry(whole.clone())
            .or_insert_with(|| scanner.find(&whole, 2).len() == 1);
        let mut length = whole.len();
        if unique && whole.len() > MINIMUM {
            let (mut low, mut high) = (MINIMUM, whole.len());
            while low < high {
                let mid = (low + high) / 2;
                if scanner.find(&whole[..mid], 2).len() == 1 {
                    high = mid;
                } else {
                    low = mid + 1;
                }
            }
            length = low;
        }
        out.push(Signature {
            address,
            unique,
            pattern: whole[..length].to_vec(),
        });
    }
    Ok(out)
}

/// The table `tables/signatures.tsv`.
pub fn table(signatures: &[Signature]) -> String {
    let mut out = HEADER.to_string();
    for s in signatures {
        let _ = writeln!(
            out,
            "{:#08x}\t{}\t{}",
            s.address,
            if s.unique { "yes" } else { "no" },
            show(&s.pattern)
        );
    }
    out
}

/// The signatures of a table's text.
pub fn parse(text: &str) -> Result<Vec<Signature>, String> {
    let mut lines = text.lines();
    if lines.next().map(|l| format!("{l}\n")).as_deref() != Some(HEADER) {
        return Err("signatures: unexpected header".to_string());
    }
    let mut out = Vec::new();
    for (i, line) in lines.enumerate() {
        let cells: Vec<&str> = line.split('\t').collect();
        let [address, unique, pattern] = cells[..] else {
            return Err(format!("signatures:{}: not three cells", i + 2));
        };
        out.push(Signature {
            address: u32::from_str_radix(address.trim_start_matches("0x"), 16)
                .map_err(|_| format!("signatures:{}: bad address", i + 2))?,
            unique: match unique {
                "yes" => true,
                "no" => false,
                _ => return Err(format!("signatures:{}: unique is yes or no", i + 2)),
            },
            pattern: read_pattern(pattern).map_err(|e| format!("signatures:{}: {e}", i + 2))?,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pattern_prints_back() {
        let p = read_pattern("55 8b ?? 00").unwrap();
        assert_eq!(p, [Some(0x55), Some(0x8b), None, Some(0)]);
        assert_eq!(show(&p), "55 8b ?? 00");
        assert!(read_pattern("5").is_err());
        assert!(read_pattern("zz").is_err());
    }

    #[test]
    fn a_table_parses_back() {
        let s = vec![
            Signature {
                address: 0x401000,
                unique: true,
                pattern: read_pattern("e8 ?? ?? ?? ??").unwrap(),
            },
            Signature {
                address: 0x401010,
                unique: false,
                pattern: read_pattern("c3").unwrap(),
            },
        ];
        assert_eq!(parse(&table(&s)).unwrap(), s);
    }

    #[test]
    fn an_index_finds_wildcards_and_short_patterns() {
        let data = [1u8, 2, 3, 4, 5, 6, 7, 8, 2, 9, 4, 5, 6, 7];
        let section = Section {
            va: 0x1000,
            flags: EXECUTE,
            data: &data,
        };
        let pe = Pe {
            base: 0x1000,
            image_size: 0x100,
            sections: vec![section],
        };
        let scanner = Scanner::new(&pe);
        let p = |t: &str| read_pattern(t).unwrap();
        assert_eq!(scanner.find(&p("02 ?? 04 05"), 9), [0x1001, 0x1008]);
        assert_eq!(scanner.find(&p("04 05 06 07"), 9), [0x1003, 0x100a]);
        assert_eq!(scanner.find(&p("?? 09"), 9), [0x1008]);
        assert_eq!(scanner.find(&p("02 ?? 04 05"), 1), [0x1001]);
        assert!(scanner.find(&p("07 08 02 09 04 05 06 07 08"), 9).is_empty());
    }
}
