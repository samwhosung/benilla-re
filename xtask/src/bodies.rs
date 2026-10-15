//! Every function row against the disassembly `tools/disasm.sh` writes: that the row's address
//! starts a function (`docs/METHOD.md`, Completeness) and that its size is its body's, by the rule in
//! `symbols/README.md`. The exe supplies the jump tables.

use crate::check::Problem;
use regex::Regex;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::sync::LazyLock;

macro_rules! pattern {
    ($name:ident, $source:expr) => {
        pub(crate) static $name: LazyLock<Regex> =
            LazyLock::new(|| Regex::new($source).expect("a valid pattern"));
    };
}

pattern!(DIRECT, r"^(j[a-z]+|loop[a-z]*)\s+0x([0-9a-f]+)$");
pattern!(
    TABLE,
    r"^jmp\s+DWORD PTR \[(e[a-z]{2})\*4\+0x([0-9a-f]+)\]$"
);
pattern!(
    LEA_FILLER,
    r"^lea (e[a-z]{2}),\[(e[a-z]{2})(?:\+eiz\*1)?\+0x0\]$"
);
pattern!(MOV_FILLER, r"^mov (e[a-z]{2}),(e[a-z]{2})$");
pattern!(IMMEDIATE, r"^(?:push|mov [^,]+,)\s*0x([0-9a-f]+)$");

/// A byte index table read, `movzx edx,BYTE PTR [eax+0x403494]`, with its index register.
pub(crate) static BYTE_TABLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^movzx\s+e[a-z]{2},BYTE PTR \[(e[a-z]{2})\+0x([0-9a-f]+)\]$")
        .expect("a valid pattern")
});
pub(crate) static BOUND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^cmp\s+(e[a-z]{2}),0x([0-9a-f]+)$").expect("a valid pattern"));
/// How far past its jump table a case can lie: hand-written code such as the C runtime's `memcpy`
/// puts its cases after its tables.
const TABLE_AFTER: u32 = 0x400;

/// The listing: each instruction's address and its text, whitespace runs collapsed.
pub struct Listing {
    pub(crate) addresses: Vec<u32>,
    ends: Vec<usize>,
    text: String,
}

impl Listing {
    pub fn read(path: &Path) -> io::Result<Listing> {
        Listing::parse(BufReader::new(File::open(path)?))
    }

    pub fn parse(reader: impl BufRead) -> io::Result<Listing> {
        let mut pairs: Vec<(u32, String)> = Vec::new();
        for line in reader.split(b'\n') {
            let line = line?;
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_start();
            let Some((address, rest)) = line.split_once(':') else {
                continue;
            };
            let Some(rest) = rest.strip_prefix('\t') else {
                continue;
            };
            if address.is_empty()
                || !address
                    .bytes()
                    .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            {
                continue;
            }
            let Ok(address) = u32::from_str_radix(address, 16) else {
                continue;
            };
            let words: Vec<&str> = rest.split_whitespace().collect();
            pairs.push((address, words.join(" ")));
        }
        // A repeated address keeps its last text.
        pairs.reverse();
        pairs.sort_by_key(|p| p.0);
        pairs.dedup_by_key(|p| p.0);
        let mut listing = Listing {
            addresses: Vec::with_capacity(pairs.len()),
            ends: Vec::with_capacity(pairs.len()),
            text: String::new(),
        };
        for (address, text) in pairs {
            listing.text.push_str(&text);
            listing.addresses.push(address);
            listing.ends.push(listing.text.len());
        }
        Ok(listing)
    }

    pub fn len(&self) -> usize {
        self.addresses.len()
    }

    pub fn contains(&self, address: u32) -> bool {
        self.index(address).is_some()
    }

    pub(crate) fn index(&self, address: u32) -> Option<usize> {
        self.addresses.binary_search(&address).ok()
    }

    pub(crate) fn at(&self, index: usize) -> &str {
        let start = if index == 0 { 0 } else { self.ends[index - 1] };
        &self.text[start..self.ends[index]]
    }

    pub(crate) fn get(&self, address: u32) -> Option<&str> {
        self.index(address).map(|i| self.at(i))
    }

    /// The first instruction after `address`.
    pub(crate) fn following(&self, address: u32) -> Option<u32> {
        let i = self.addresses.partition_point(|&a| a <= address);
        self.addresses.get(i).copied()
    }

    /// The last instruction before `address`.
    pub(crate) fn preceding(&self, address: u32) -> Option<u32> {
        let i = self.addresses.partition_point(|&a| a < address);
        i.checked_sub(1).map(|i| self.addresses[i])
    }
}

/// The exe's sections as file bytes, for reading jump tables.
pub struct Image<'a> {
    bytes: &'a [u8],
    /// Each section's virtual address, raw size and file offset.
    pub(crate) sections: Vec<(u32, u32, u32)>,
    /// Each section's virtual size, in the order of `sections`.
    virtual_sizes: Vec<u32>,
}

impl<'a> Image<'a> {
    pub fn new(bytes: &'a [u8]) -> Option<Image<'a>> {
        let u16_at = |o: usize| Some(u16::from_le_bytes(bytes.get(o..o + 2)?.try_into().ok()?));
        let u32_at = |o: usize| Some(u32::from_le_bytes(bytes.get(o..o + 4)?.try_into().ok()?));
        let pe = u32_at(0x3c)? as usize;
        let count = u16_at(pe + 6)? as usize;
        let optional = u16_at(pe + 20)? as usize;
        let base = u32_at(pe + 52)?;
        let table = pe + 24 + optional;
        let mut sections = Vec::new();
        let mut virtual_sizes = Vec::new();
        for i in 0..count {
            let header = table + 40 * i;
            virtual_sizes.push(u32_at(header + 8)?);
            let address = u32_at(header + 12)?;
            let raw_size = u32_at(header + 16)?;
            let raw_offset = u32_at(header + 20)?;
            sections.push((base.wrapping_add(address), raw_size, raw_offset));
        }
        Some(Image {
            bytes,
            sections,
            virtual_sizes,
        })
    }

    /// The file bytes at `address`, `length` of them, when one section holds them all.
    pub(crate) fn read(&self, address: u32, length: u32) -> Option<&[u8]> {
        let (address, length) = (u64::from(address), u64::from(length));
        for &(start, size, offset) in &self.sections {
            let (start, size) = (u64::from(start), u64::from(size));
            if start <= address && address + length <= start + size {
                let at = (u64::from(offset) + address - start) as usize;
                return self.bytes.get(at..at + length as usize);
            }
        }
        None
    }

    /// The end of the section that holds `address`, by its virtual size.
    pub(crate) fn section_end(&self, address: u32) -> Option<u32> {
        self.sections
            .iter()
            .zip(&self.virtual_sizes)
            .map(|(&(start, _, _), &size)| (start, start.wrapping_add(size)))
            .find(|&(start, end)| start <= address && address < end)
            .map(|(_, end)| end)
    }

    pub fn byte(&self, address: u32) -> Option<u8> {
        self.read(address, 1).map(|b| b[0])
    }

    pub(crate) fn dword(&self, address: u32) -> Option<u32> {
        Some(u32::from_le_bytes(self.read(address, 4)?.try_into().ok()?))
    }

    /// Every dword-aligned value in the sections other than the one holding `code` that lies in
    /// `[low, high]`: the code addresses the exe's data stores, such as exception handlers.
    pub(crate) fn stored_addresses(&self, code: u32, low: u32, high: u32) -> HashSet<u32> {
        let mut found = HashSet::new();
        for &(start, size, offset) in &self.sections {
            if start <= code && u64::from(code) < u64::from(start) + u64::from(size) {
                continue;
            }
            let (offset, size) = (offset as usize, size as usize);
            let Some(bytes) = self.bytes.get(offset..offset + size) else {
                continue;
            };
            for word in bytes.as_chunks::<4>().0 {
                let value = u32::from_le_bytes(*word);
                if low <= value && value <= high {
                    found.insert(value);
                }
            }
        }
        found
    }

    /// The entries of the jump table at `table` that a function entered at `entry` indexes: the
    /// dwords from `table` on while each is an address in `[entry, table)`, since the table
    /// follows its function's code, or one past the table within `TABLE_AFTER`, where
    /// hand-written code keeps its cases.
    /// A jump table read from `base`, or, when hand-written code biases its index so that the
    /// first dwords from `base` are never read (`jmp [eax*4+base]` with `eax` from 1, as the C
    /// runtime's `memcpy` does), from the first of the next three dwords that is an entry: that
    /// table's address and entries.
    pub fn biased_table(&self, entry: u32, base: u32) -> (u32, Vec<u32>) {
        for skip in 0..4 {
            let table = base.wrapping_add(4 * skip);
            let entries = self.table_entries(entry, table);
            if !entries.is_empty() {
                return (table, entries);
            }
        }
        (base, Vec::new())
    }

    /// A jump table indexed by a register negated just before the jump (`neg ecx; jmp
    /// [ecx*4+base]`, as the C runtime's `memmove` copies backward): the dwords from `base` down
    /// while each is an entry, read as `table_entries` reads them. Its lowest address and its
    /// entries in address order.
    pub fn negated_table(&self, entry: u32, base: u32) -> (u32, Vec<u32>) {
        let mut entries = Vec::new();
        let mut at = base;
        while let Some(value) = self.dword(at) {
            let before = entry <= value && value < at;
            let after = base + 4 <= value && value < base.saturating_add(TABLE_AFTER);
            if !(before || after) {
                break;
            }
            entries.push(value);
            at = at.wrapping_sub(4);
        }
        entries.reverse();
        (base - 4 * (entries.len() as u32).saturating_sub(1), entries)
    }

    pub fn table_entries(&self, entry: u32, table: u32) -> Vec<u32> {
        let mut entries = Vec::new();
        loop {
            let at = table.wrapping_add(4 * entries.len() as u32);
            let Some(value) = self.dword(at) else { break };
            let before = entry <= value && value < table;
            let after = at + 4 <= value && value < table.saturating_add(TABLE_AFTER);
            if !(before || after) {
                break;
            }
            entries.push(value);
        }
        entries
    }
}

/// The functions that never return, read at their bodies. A `call` to one ends the path where
/// padding or another function follows it, and the instruction after it is never reached by
/// falling through.
///
/// - `__amsg_exit` `0x4099c8` calls through `0x82f264`, which holds `__exit` and which no
///   instruction writes;
/// - `_exit` `0x40a30d` and `__exit` `0x40a31e` call `_doexit` `0x40a34d` with its third argument
///   0, the path that ends in `ExitProcess` (`0x40a3ea`);
/// - `_longjmp` `0x742014` ends in `jmp DWORD PTR [edx+0x14]`, to the frame `setjmp` saved.
///
/// Besides these, a function whose body reaches no `ret`, no indirect `jmp` and no tail jump to a
/// function that returns never returns either; so does a call through the import slot of
/// `ExitProcess`, `ExitThread` or `FatalAppExitA`.
pub const NO_RETURN: [u32; 4] = [0x4099c8, 0x40a30d, 0x40a31e, 0x742014];
const NO_RETURN_IMPORTS: [&str; 3] = ["ExitProcess", "ExitThread", "FatalAppExitA"];

pattern!(CALL_DIRECT, r"^call\s+0x([0-9a-f]+)$");
pattern!(CALL_SLOT, r"^call\s+DWORD PTR ds:0x([0-9a-f]+)$");

/// The functions and import slots a call to which never returns.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct NoReturn {
    pub functions: HashSet<u32>,
    pub slots: HashSet<u32>,
}

impl NoReturn {
    /// Whether `instruction` is a call that never returns.
    pub fn ends(&self, instruction: &str) -> bool {
        if !instruction.starts_with("call") {
            return false;
        }
        if let Some(found) = CALL_DIRECT.captures(instruction) {
            return u32::from_str_radix(&found[1], 16).is_ok_and(|t| self.functions.contains(&t));
        }
        CALL_SLOT
            .captures(instruction)
            .and_then(|f| u32::from_str_radix(&f[1], 16).ok())
            .is_some_and(|s| self.slots.contains(&s))
    }
}

/// The functions entered at `starts` that never return, with `NO_RETURN` and the import slots:
/// repeated until no function is new, since each one found cuts its callers' paths.
pub fn no_return(starts: &[u32], listing: &Listing, image: &Image) -> NoReturn {
    let mut found = NoReturn {
        functions: NO_RETURN.iter().copied().collect(),
        slots: imports(image.bytes)
            .into_iter()
            .filter(|(_, (name, _))| NO_RETURN_IMPORTS.contains(&name.as_str()))
            .map(|(slot, _)| slot)
            .collect(),
    };
    let set: HashSet<u32> = starts.iter().copied().collect();
    loop {
        let walker = Walker {
            listing,
            image,
            starts: set.clone(),
            no_return: &found,
        };
        let mut more = Vec::new();
        for &entry in starts {
            if found.functions.contains(&entry) || !listing.contains(entry) {
                continue;
            }
            let (reached, _) = walker.body(entry);
            let returns = reached.iter().any(|&a| {
                let text = listing.get(a).unwrap_or_default();
                if text.starts_with("ret") {
                    return true;
                }
                if text.starts_with("jmp") && !TABLE.is_match(text) {
                    return match DIRECT.captures(text) {
                        Some(f) => u32::from_str_radix(&f[2], 16).is_ok_and(|t| {
                            t != entry && set.contains(&t) && !found.functions.contains(&t)
                        }),
                        None => true,
                    };
                }
                false
            });
            if !returns {
                more.push(entry);
            }
        }
        if more.is_empty() {
            return found;
        }
        found.functions.extend(more);
    }
}

/// The import address table's slots: each slot's import name and its library.
pub fn imports(bytes: &[u8]) -> HashMap<u32, (String, String)> {
    let mut out = HashMap::new();
    let u16_at = |o: usize| Some(u16::from_le_bytes(bytes.get(o..o + 2)?.try_into().ok()?));
    let u32_at = |o: usize| Some(u32::from_le_bytes(bytes.get(o..o + 4)?.try_into().ok()?));
    let Some(pe) = u32_at(0x3c).map(|p| p as usize) else {
        return out;
    };
    let optional = pe + 24;
    let (Some(base), Some(directory), Some(count), Some(size)) = (
        u32_at(optional + 28),
        u32_at(optional + 96 + 8),
        u16_at(pe + 6),
        u16_at(pe + 20),
    ) else {
        return out;
    };
    let table = optional + size as usize;
    // An RVA's file offset, through the section table.
    let offset = |rva: u32| -> Option<usize> {
        (0..count as usize).find_map(|i| {
            let h = table + 40 * i;
            let (va, raw, at) = (u32_at(h + 12)?, u32_at(h + 16)?, u32_at(h + 20)?);
            (va <= rva && rva < va + raw).then(|| (at + rva - va) as usize)
        })
    };
    let string = |at: usize| -> Option<String> {
        let end = at + bytes.get(at..)?.iter().position(|&b| b == 0)?;
        String::from_utf8(bytes[at..end].to_vec()).ok()
    };
    let Some(mut descriptor) = offset(directory) else {
        return out;
    };
    while let (Some(names), Some(dll), Some(slots)) = (
        u32_at(descriptor),
        u32_at(descriptor + 12),
        u32_at(descriptor + 16),
    ) {
        if dll == 0 && slots == 0 {
            break;
        }
        let library = offset(dll).and_then(string).unwrap_or_default();
        let lookup = if names != 0 { names } else { slots };
        for k in 0.. {
            let Some(entry) = offset(lookup + 4 * k).and_then(u32_at) else {
                break;
            };
            if entry == 0 {
                break;
            }
            if entry & 0x8000_0000 == 0 {
                if let Some(name) = offset(entry + 2).and_then(string) {
                    out.insert(base + slots + 4 * k, (name, library.clone()));
                }
            }
        }
        descriptor += 20;
    }
    out
}

/// Never executed: `int3`, which pads between functions.
pub(crate) fn is_trap(instruction: &str) -> bool {
    instruction.starts_with("int3")
}

/// A no-op compilers emit to align code or data: `nop` in its one-byte and multi-byte forms,
/// `xchg ax,ax`, a 32-bit `mov` of a register to itself (`mov edi,edi`), and `lea` of a
/// register to itself plus zero (`lea ecx,[ecx+0x0]`, `lea esp,[esp+0x0]`).
pub(crate) fn is_filler(instruction: &str) -> bool {
    if instruction == "nop" || instruction.starts_with("nop ") || instruction == "xchg ax,ax" {
        return true;
    }
    [&*LEA_FILLER, &*MOV_FILLER].iter().any(|pattern| {
        pattern
            .captures(instruction)
            .is_some_and(|found| found[1] == found[2])
    })
}

/// The jump tables found by walking bodies: each table's address and entries.
pub type Tables = BTreeMap<u32, Vec<u32>>;

/// Adds `found` to `tables`, keeping the longer reading of a table two entries share.
pub fn merge_tables(tables: &mut Tables, found: Tables) {
    for (table, entries) in found {
        let known = tables.entry(table).or_default();
        if entries.len() > known.len() {
            *known = entries;
        }
    }
}

/// The byte ranges `tables` occupy, sorted, overlaps joined.
pub fn table_ranges(tables: &Tables) -> Vec<(u32, u32)> {
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for (&table, entries) in tables {
        let end = table.wrapping_add(4 * entries.len() as u32);
        if end <= table {
            continue;
        }
        match ranges.last_mut() {
            Some(last) if table <= last.1 => last.1 = last.1.max(end),
            _ => ranges.push((table, end)),
        }
    }
    ranges
}

pub(crate) fn in_ranges(ranges: &[(u32, u32)], address: u32) -> bool {
    let i = ranges.partition_point(|r| r.0 <= address);
    i > 0 && address < ranges[i - 1].1
}

/// Where the instruction at `address` ends: at the next instruction, or at a jump table that
/// comes first, since the listing leaves a table's bytes out. The last instruction ends where the
/// image's section that holds it ends, `section_end`.
pub(crate) fn end_of(
    listing: &Listing,
    ranges: &[(u32, u32)],
    address: u32,
    section_end: Option<u32>,
) -> Option<u32> {
    let next = listing.following(address);
    let i = ranges.partition_point(|r| r.0 <= address);
    match (next, ranges.get(i).map(|r| r.0)) {
        (Some(next), Some(table)) => Some(next.min(table)),
        (Some(next), None) => Some(next),
        (None, table) => section_end.filter(|&end| end > address).or(table),
    }
}

/// Walks bodies over a listing.
pub(crate) struct Walker<'a> {
    pub(crate) listing: &'a Listing,
    pub(crate) image: &'a Image<'a>,
    pub(crate) starts: HashSet<u32>,
    pub(crate) no_return: &'a NoReturn,
}

impl Walker<'_> {
    /// The instructions reached from `entry`, sorted, and the jump tables they index.
    pub(crate) fn body(&self, entry: u32) -> (Vec<u32>, Tables) {
        self.walk(entry, vec![entry])
    }

    /// The instructions reached from `seeds` by the body walk of the function entered at `entry`.
    pub(crate) fn walk(&self, entry: u32, seeds: Vec<u32>) -> (Vec<u32>, Tables) {
        let (mut reached, mut tables, mut work) = (HashSet::new(), Tables::new(), seeds);
        while let Some(start) = work.pop() {
            let mut address = Some(start);
            while let Some(here) = address {
                let Some(instruction) = self.listing.get(here) else {
                    break;
                };
                if !reached.insert(here) {
                    break;
                }
                if instruction.starts_with("ret") || is_trap(instruction) {
                    break;
                }
                // A call that never returns ends the path where padding or another function
                // follows it; code the compiler put after it is still the function's.
                if self.no_return.ends(instruction) {
                    let next = self.listing.following(here);
                    let after = next.and_then(|n| self.listing.get(n)).unwrap_or_default();
                    if next.is_none_or(|n| self.starts.contains(&n))
                        || is_trap(after)
                        || is_filler(after)
                    {
                        break;
                    }
                }
                if let Some(found) = DIRECT.captures(instruction) {
                    let Ok(target) = u32::from_str_radix(&found[2], 16) else {
                        break;
                    };
                    if &found[1] == "jmp" {
                        if self.starts.contains(&target) && target != entry {
                            break;
                        }
                        address = Some(target);
                        continue;
                    }
                    work.push(target);
                } else if let Some(found) = TABLE.captures(instruction) {
                    let base = u32::from_str_radix(&found[2], 16).unwrap_or(0);
                    let negated = self
                        .listing
                        .preceding(here)
                        .and_then(|p| self.listing.get(p))
                        .is_some_and(|p| p == format!("neg {}", &found[1]));
                    let (table, entries) = if negated {
                        self.image.negated_table(entry, base)
                    } else {
                        self.image.biased_table(entry, base)
                    };
                    work.extend(&entries);
                    tables.insert(table, entries);
                    break;
                } else if instruction.starts_with("jmp") {
                    break;
                }
                address = self.listing.following(here);
            }
        }
        let mut reached: Vec<u32> = reached.into_iter().collect();
        reached.sort_unstable();
        (reached, tables)
    }
}

/// The exception filters and handlers of the C runtime's frames, each with its scope table. A
/// frame opens `push 0xffffffff; push <scope table>; push <handler>; mov eax,fs:0x0`; the table's
/// records are (enclosing level, filter or 0, handler), and both code addresses are the framing
/// function's own code, entered by the handler through the table and never called.
pub fn exception_code(listing: &Listing, image: &Image, text: (u32, u32)) -> BTreeMap<u32, u32> {
    static PUSH: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^push 0x([0-9a-f]+)$").expect("pattern"));
    let code = |a: u32| (text.0..text.1).contains(&a);
    let mut found = BTreeMap::new();
    for i in 0..listing.len().saturating_sub(3) {
        if listing.at(i) != "push 0xffffffff" || !listing.at(i + 3).ends_with("fs:0x0") {
            continue;
        }
        let pushed = |k: usize| {
            PUSH.captures(listing.at(i + k))
                .and_then(|c| u32::from_str_radix(&c[1], 16).ok())
        };
        let (Some(table), Some(handler)) = (pushed(1), pushed(2)) else {
            continue;
        };
        if code(table) || !code(handler) {
            continue;
        }
        for record in 0u32.. {
            let at = table + 12 * record;
            let (Some(level), Some(filter), Some(handler)) =
                (image.dword(at), image.dword(at + 4), image.dword(at + 8))
            else {
                break;
            };
            if !(level == u32::MAX || level < record)
                || !(filter == 0 || code(filter))
                || !code(handler)
            {
                break;
            }
            if filter != 0 {
                found.insert(filter, table);
            }
            found.insert(handler, table);
        }
    }
    found
}

/// The jump tables the bodies of the functions entered at `starts` index, walked over `listing`.
pub fn jump_tables(
    starts: &[u32],
    listing: &Listing,
    image: &Image,
    no_return: &NoReturn,
) -> Tables {
    let walker = Walker {
        listing,
        image,
        starts: starts.iter().copied().collect(),
        no_return,
    };
    let mut tables = Tables::new();
    for &entry in starts {
        if listing.contains(entry) {
            merge_tables(&mut tables, walker.body(entry).1);
        }
    }
    tables
}

/// The byte index tables the bodies read: `movzx r,BYTE PTR [i+table]` after `cmp i,bound`
/// within the twelve instructions before it, with no `call`, `jmp` or `ret` between, gives
/// `bound + 1` bytes; with no bound, the table runs while its bytes index the jump table the
/// next instruction reads. Returns the ranges and the reads with no bound.
pub(crate) fn index_tables(
    listing: &Listing,
    image: &Image,
    entry: u32,
    reached: &[u32],
) -> (Vec<(u32, u32)>, Vec<u32>) {
    let (mut ranges, mut unbounded) = (Vec::new(), Vec::new());
    for &address in reached {
        let Some(instruction) = listing.get(address) else {
            continue;
        };
        if !instruction.starts_with("movzx") {
            continue;
        }
        let Some(found) = BYTE_TABLE.captures(instruction) else {
            continue;
        };
        let Ok(table) = u32::from_str_radix(&found[2], 16) else {
            continue;
        };
        // An index table sits in the code, after its function's jump table.
        let (Some(&first), Some(&last)) = (listing.addresses.first(), listing.addresses.last())
        else {
            continue;
        };
        if !(first..=last).contains(&table) {
            continue;
        }
        let register = &found[1];
        let mut back = Some(address);
        let mut bound = None;
        for _ in 0..12 {
            back = back.and_then(|b| listing.preceding(b));
            let Some(b) = back else { break };
            let text = listing.get(b).unwrap_or_default();
            if let Some(cmp) = BOUND.captures(text) {
                if &cmp[1] == register {
                    bound = u32::from_str_radix(&cmp[2], 16).ok();
                    break;
                }
            }
            if text.starts_with("ret") || text.starts_with("jmp") || text.starts_with("call") {
                break;
            }
        }
        // With no bound, the table runs while its bytes index the jump table that follows.
        let bound = bound.or_else(|| {
            let next = listing.following(address)?;
            let jump = TABLE.captures(listing.get(next)?)?;
            let entries = image.table_entries(entry, u32::from_str_radix(&jump[2], 16).ok()?);
            let mut n = 0;
            while n < 256
                && image
                    .byte(table + n)
                    .is_some_and(|b| (b as usize) < entries.len())
            {
                n += 1;
            }
            unbounded.push(address);
            n.checked_sub(1)
        });
        if let Some(bound) = bound {
            ranges.push((table, table + bound + 1));
        }
    }
    (ranges, unbounded)
}

/// Sorted ranges, overlaps joined.
pub(crate) fn join(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    ranges.sort_unstable();
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (start, end) in ranges {
        match out.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => out.push((start, end)),
        }
    }
    out
}

/// The byte index tables the bodies of the functions entered at `starts` read, as joined ranges.
pub fn all_index_tables(
    starts: &[u32],
    listing: &Listing,
    image: &Image,
    no_return: &NoReturn,
) -> Vec<(u32, u32)> {
    let walker = Walker {
        listing,
        image,
        starts: starts.iter().copied().collect(),
        no_return,
    };
    let mut ranges = Vec::new();
    for &entry in starts {
        if listing.contains(entry) {
            let (reached, _) = walker.body(entry);
            ranges.extend(index_tables(listing, image, entry, &reached).0);
        }
    }
    join(ranges)
}

/// A function row: its place in the table, its address and its size.
pub struct Function {
    pub place: String,
    pub address: u32,
    pub size: u32,
}

/// What the whole listing says about one address, read once for every row.
struct Context<'a> {
    listing: &'a Listing,
    jcc_targets: HashSet<u32>,
    /// Every address a direct branch or a jump table reaches.
    branch_targets: HashSet<u32>,
}

impl Context<'_> {
    /// Padding: an `int3`, or a no-op filler nothing branches to.
    fn padding(&self, address: u32) -> bool {
        let instruction = self.listing.get(address).unwrap_or_default();
        is_trap(instruction) || (is_filler(instruction) && !self.branch_targets.contains(&address))
    }
}

/// Checks every row; returns the problems.
pub fn check(rows: &[Function], listing: &Listing, image: &Image) -> Vec<Problem> {
    let (Some(&first), Some(&last)) = (listing.addresses.first(), listing.addresses.last()) else {
        return vec![Problem {
            place: "dumps/disasm.txt".to_string(),
            message: "holds no instruction".to_string(),
        }];
    };
    let addresses: Vec<u32> = rows.iter().map(|r| r.address).collect();
    let no_return = no_return(&addresses, listing, image);
    let walker = Walker {
        listing,
        image,
        starts: addresses.iter().copied().collect(),
        no_return: &no_return,
    };
    let bodies: Vec<Option<(Vec<u32>, Tables)>> = rows
        .iter()
        .map(|row| {
            listing
                .contains(row.address)
                .then(|| walker.body(row.address))
        })
        .collect();
    let mut tables = Tables::new();
    for (_, found) in bodies.iter().flatten() {
        merge_tables(&mut tables, found.clone());
    }
    // A table's bytes are data: an instruction the listing decodes from them is no evidence.
    let mut ranges = table_ranges(&tables);
    for (body, row) in bodies.iter().zip(rows) {
        if let Some((reached, _)) = body {
            ranges.extend(index_tables(listing, image, row.address, reached).0);
        }
    }
    let ranges = join(ranges);
    let mut jcc_targets = HashSet::new();
    let mut branch_targets: HashSet<u32> = tables.values().flatten().copied().collect();
    for i in 0..listing.len() {
        if in_ranges(&ranges, listing.addresses[i]) {
            continue;
        }
        let instruction = listing.at(i);
        if instruction.starts_with('j') || instruction.starts_with("loop") {
            if let Some(found) = DIRECT.captures(instruction) {
                if let Ok(target) = u32::from_str_radix(&found[2], 16) {
                    branch_targets.insert(target);
                    if &found[1] != "jmp" {
                        jcc_targets.insert(target);
                    }
                }
            }
        }
    }
    // The code addresses stored as data or as an immediate operand: exception filters, handlers,
    // catch funclets and the continuations they return.
    let mut stored = image.stored_addresses(first, first, last);
    for i in 0..listing.len() {
        if let Some(found) = IMMEDIATE.captures(listing.at(i)) {
            if let Ok(value) = u32::from_str_radix(&found[1], 16) {
                stored.insert(value);
            }
        }
    }
    let mut stored: Vec<u32> = stored.into_iter().collect();
    stored.sort_unstable();
    let context = Context {
        listing,
        jcc_targets,
        branch_targets,
    };

    // Code in a row's extent reached only through a stored address is the function's own
    // exception code; an address that has a row is another function's entry.
    let exceptions: Vec<Vec<u32>> = rows
        .iter()
        .zip(&bodies)
        .map(|(row, body)| {
            let Some((reached, _)) = body else {
                return Vec::new();
            };
            let end = u64::from(row.address) + u64::from(row.size);
            let seeds: Vec<u32> = stored[stored.partition_point(|&a| a <= row.address)..]
                .iter()
                .copied()
                .take_while(|&a| u64::from(a) < end)
                .filter(|a| reached.binary_search(a).is_err() && !walker.starts.contains(a))
                .collect();
            walker.walk(row.address, seeds).0
        })
        .collect();
    // The code some row's body or exception code reaches, and what the C runtime's scope tables
    // enter.
    let text = (first, image.section_end(first).unwrap_or(last + 1));
    let handlers: Vec<u32> = exception_code(listing, image, text).into_keys().collect();
    let covered: HashSet<u32> = bodies
        .iter()
        .flatten()
        .flat_map(|(reached, _)| reached.iter().copied())
        .chain(exceptions.iter().flatten().copied())
        .chain(handlers.iter().flat_map(|&h| walker.walk(h, vec![h]).0))
        .collect();
    // Why each row's address is no entry: code above falls through into it, or a conditional
    // jump lands on it.
    let not_entry: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            let entry = row.address;
            let mut why = Vec::new();
            let mut before = listing.preceding(entry);
            while let Some(b) = before.filter(|&b| context.padding(b)) {
                before = listing.preceding(b);
            }
            if let Some(b) = before {
                let instruction = listing.get(b).unwrap_or_default();
                let ends = ["ret", "jmp", "int3"]
                    .iter()
                    .any(|e| instruction.starts_with(e))
                    || no_return.ends(instruction);
                // Bytes no body reaches are no code that falls through: data before the entry,
                // such as a table the listing decodes.
                if !ends && covered.contains(&b) && !in_ranges(&ranges, b) {
                    why.push(format!(
                        "reached by fall-through from {b:#x}, so code of the function above"
                    ));
                }
            }
            if context.jcc_targets.contains(&entry) {
                why.push("a conditional jump lands on it".to_string());
            }
            why
        })
        .collect();
    // The entries a body may not hold: rows that are entries by the test above.
    let entries: HashSet<u32> = rows
        .iter()
        .zip(&not_entry)
        .filter(|(_, why)| why.is_empty())
        .map(|(row, _)| row.address)
        .collect();
    let mut problems = Vec::new();
    let mut add = |row: &Function, message: String| {
        problems.push(Problem {
            place: row.place.clone(),
            message,
        })
    };
    for (i, (row, body)) in rows.iter().zip(&bodies).enumerate() {
        let (entry, size) = (row.address, row.size);
        let Some((reached, tables)) = body else {
            add(row, "not an instruction start in the listing".to_string());
            continue;
        };
        for why in &not_entry[i] {
            add(row, why.clone());
        }
        let last_reached = reached.last().copied().unwrap_or(entry);
        match end_of(
            listing,
            &ranges,
            last_reached,
            image.section_end(last_reached),
        ) {
            Some(end) if end - entry != size => {
                add(row, format!("size {size}, body {}", end - entry));
            }
            Some(_) => {}
            None => add(row, "the body runs to the end of the listing".to_string()),
        }
        let exception = &exceptions[i];
        let own_tables = table_ranges(tables);
        let low = listing.addresses.partition_point(|&a| a < entry);
        let high = listing
            .addresses
            .partition_point(|&a| u64::from(a) < u64::from(entry) + u64::from(size));
        for &address in &listing.addresses[low..high] {
            if address != entry && entries.contains(&address) {
                add(
                    row,
                    format!("{address:#x}, another row's entry, lies inside the extent"),
                );
                break;
            }
            if reached.binary_search(&address).is_ok()
                || exception.binary_search(&address).is_ok()
                || context.padding(address)
                || in_ranges(&own_tables, address)
            {
                continue;
            }
            add(
                row,
                format!("{address:#x} lies inside the extent but outside the body"),
            );
            break;
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three functions: one with a conditional branch and a call, a switch through a jump table
    /// whose data follows its code, and a tail jump after the table.
    const LISTING: &str = "
WoW.exe:     file format pei-i386

00401000 <.text>:
  401000:\tpush   ebp
  401001:\tmov    ebp,esp
  401003:\tje     0x40100a
  401005:\tcall   0x401040
  40100a:\tpop    ebp
  40100b:\tret
  40100c:\tnop
  40100d:\tnop
  40100e:\tint3
  40100f:\tnop
  401010:\tmov    eax,ecx
  401012:\tjmp    DWORD PTR [eax*4+0x401030]
  401019:\tmov    eax,0x1
  40101e:\tret
  40101f:\tmov    eax,0x2
  401024:\tret
  401025:\tnop
  401030:\tsbb    BYTE PTR [eax+0x40],dl
  401034:\tsbb    BYTE PTR [eax+0x40],bl
  401038:\tadd    BYTE PTR [eax],al
  40103a:\tadd    BYTE PTR [eax],al
  401040:\tjmp    0x401010
  401045:\tnop
";

    /// A PE with one section at 0x401000 whose file bytes hold the jump table at 0x401030.
    fn image_bytes() -> Vec<u8> {
        let mut bytes = vec![0u8; 0x400];
        let put = |bytes: &mut Vec<u8>, at: usize, value: u32| {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        };
        put(&mut bytes, 0x3c, 0x40);
        bytes[0x46] = 1; // one section
        bytes[0x54] = 0xe0; // the optional header's size
        put(&mut bytes, 0x40 + 52, 0x400000);
        let section = 0x40 + 24 + 0xe0;
        put(&mut bytes, section + 12, 0x1000);
        put(&mut bytes, section + 16, 0x200);
        put(&mut bytes, section + 20, 0x200);
        put(&mut bytes, 0x230, 0x401019);
        put(&mut bytes, 0x234, 0x40101f);
        bytes
    }

    fn problems(rows: &[(u32, u32)]) -> Vec<String> {
        problems_in(LISTING, rows)
    }

    fn problems_in(listing: &str, rows: &[(u32, u32)]) -> Vec<String> {
        let listing = Listing::parse(listing.as_bytes()).expect("parses");
        let bytes = image_bytes();
        let image = Image::new(&bytes).expect("a PE");
        let rows: Vec<Function> = rows
            .iter()
            .map(|&(address, size)| Function {
                place: format!("{address:#x}"),
                address,
                size,
            })
            .collect();
        check(&rows, &listing, &image)
            .iter()
            .map(|p| p.to_string())
            .collect()
    }

    const CLEAN: [(u32, u32); 3] = [(0x401000, 12), (0x401010, 21), (0x401040, 5)];

    #[test]
    fn bodies_follow_branches_tables_and_tail_jumps() {
        assert_eq!(problems(&CLEAN), Vec::<String>::new());
    }

    #[test]
    fn each_misfit_is_named() {
        let with = |extra: (u32, u32)| {
            let mut rows = CLEAN.to_vec();
            rows.push(extra);
            rows.sort();
            problems(&rows)
        };
        let mut wrong_size = CLEAN;
        wrong_size[0].1 = 10;
        assert_eq!(problems(&wrong_size), ["0x401000: size 10, body 12"]);
        assert_eq!(
            with((0x401002, 1)),
            ["0x401002: not an instruction start in the listing"]
        );
        assert_eq!(
            with((0x401005, 7)),
            ["0x401005: reached by fall-through from 0x401003, so code of the function above"]
        );
        assert_eq!(
            with((0x40100a, 2)),
            [
                "0x40100a: reached by fall-through from 0x401005, so code of the function above",
                "0x40100a: a conditional jump lands on it",
            ]
        );
        let mut long = CLEAN;
        long[1].1 = 0x30;
        assert_eq!(
            problems(&long),
            [
                "0x401010: size 48, body 21",
                "0x401010: 0x401038 lies inside the extent but outside the body",
            ]
        );
    }

    /// The listing as `tools/disasm.sh` writes it, which leaves the jump table's bytes out.
    #[test]
    fn a_left_out_table_ends_the_instruction_before_it() {
        let listing: String = LISTING
            .lines()
            .filter(|l| !l.trim_start().starts_with("40103"))
            .map(|l| format!("{l}\n"))
            .collect();
        assert!(!listing.contains("401030:") && listing.contains("401024:"));
        assert_eq!(problems_in(&listing, &CLEAN), Vec::<String>::new());
        let without_nop = listing.replace("  401025:\tnop\n", "");
        let mut sizes = CLEAN;
        sizes[1].1 = 32;
        assert_eq!(problems_in(&without_nop, &sizes), Vec::<String>::new());
        assert_eq!(
            problems_in(&without_nop, &CLEAN),
            ["0x401010: size 21, body 32"]
        );
    }

    /// A conditional jump the listing decodes from the jump table's bytes lands on a row: no
    /// evidence. The same jump in the bytes after the table, which no table owns, is.
    #[test]
    fn a_jump_decoded_from_a_table_is_no_evidence() {
        let in_table = LISTING.replace(
            "401034:\tsbb    BYTE PTR [eax+0x40],bl",
            "401034:\tje     0x401040",
        );
        assert_eq!(problems_in(&in_table, &CLEAN), Vec::<String>::new());
        let after_table = LISTING.replace(
            "401038:\tadd    BYTE PTR [eax],al",
            "401038:\tje     0x401040",
        );
        assert_eq!(
            problems_in(&after_table, &CLEAN),
            ["0x401040: a conditional jump lands on it"]
        );
    }

    /// Alignment fillers: inside a function, between a return and the code a branch reaches,
    /// and after it, before the next entry.
    const FILLERS: &str = "
  402000:\tpush   ebp
  402001:\tje     0x402010
  402003:\tret
  402004:\tlea    esp,[esp+0x0]
  40200b:\tlea    ecx,[ecx+0x0]
  40200e:\tmov    edi,edi
  402010:\tpop    ebp
  402011:\tret
  402012:\txchg   ax,ax
  402014:\tnop    DWORD PTR [eax+0x0]
  402018:\tlea    esi,[esi+eiz*1+0x0]
  40201f:\tint3
";

    #[test]
    fn fillers_are_padding_unless_a_branch_lands_on_them() {
        let next = "  402020:\tret\n  402021:\tint3\n";
        let rows = [(0x402000, 18), (0x402020, 1)];
        assert_eq!(
            problems_in(&format!("{FILLERS}{next}"), &rows),
            Vec::<String>::new()
        );
        let branch = "  402020:\tjne    0x40200b\n  402022:\tret\n  402023:\tint3\n";
        assert_eq!(
            problems_in(
                &format!("{FILLERS}{branch}"),
                &[(0x402000, 18), (0x402020, 3)]
            ),
            ["0x402000: 0x40200b lies inside the extent but outside the body"]
        );
        // An instruction that is no filler and that no body reaches is data before the entry,
        // not code falling through into it.
        let not_filler = FILLERS.replace("lea    esi,[esi+eiz*1+0x0]", "lea    esi,[edi+0x0]");
        assert_eq!(
            problems_in(&format!("{not_filler}{next}"), &rows),
            Vec::<String>::new()
        );
        let reached = not_filler.replace("je     0x402010", "je     0x402018");
        assert_eq!(
            problems_in(
                &format!("{reached}{next}"),
                &[(0x402000, 32), (0x402020, 1)]
            ),
            [
                "0x402000: 0x402010 lies inside the extent but outside the body",
                "0x402020: reached by fall-through from 0x402018, so code of the function above"
            ]
        );
        assert!(!is_filler("mov edi,esi"));
        assert!(!is_filler("lea ecx,[ecx+0x4]"));
    }

    /// A PE with code at 0x403000 and a data section at 0x405000 that stores `stored`.
    fn image_storing(stored: &[u32]) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x600];
        let put = |bytes: &mut Vec<u8>, at: usize, value: u32| {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        };
        put(&mut bytes, 0x3c, 0x40);
        bytes[0x46] = 2; // two sections
        bytes[0x54] = 0xe0; // the optional header's size
        put(&mut bytes, 0x40 + 52, 0x400000);
        let sections = 0x40 + 24 + 0xe0;
        for (i, (address, offset)) in [(0x3000, 0x200), (0x5000, 0x400)].into_iter().enumerate() {
            put(&mut bytes, sections + 40 * i + 12, address);
            put(&mut bytes, sections + 40 * i + 16, 0x200);
            put(&mut bytes, sections + 40 * i + 20, offset);
        }
        bytes[0x200..0x400].fill(0x90);
        for (k, &value) in stored.iter().enumerate() {
            put(&mut bytes, 0x400 + 8 + 4 * k, value);
        }
        bytes
    }

    fn problems_storing(stored: &[u32], listing: &str, rows: &[(u32, u32)]) -> Vec<String> {
        let listing = Listing::parse(listing.as_bytes()).expect("parses");
        let bytes = image_storing(stored);
        let image = Image::new(&bytes).expect("a PE");
        let rows: Vec<Function> = rows
            .iter()
            .map(|&(address, size)| Function {
                place: format!("{address:#x}"),
                address,
                size,
            })
            .collect();
        check(&rows, &listing, &image)
            .iter()
            .map(|p| p.to_string())
            .collect()
    }

    /// The shape of a function with a catch funclet inside it: the funclet at 0x403003 is
    /// reached only through the exception data, and returns the continuation 0x40300d. After the
    /// body's `ret`, a handler at 0x403014 that only the data reaches, as after
    /// `_WinMainCRTStartup`.
    const EXCEPTION: &str = "
  403000:\tpush   ebp
  403001:\tjmp    0x403010
  403003:\tmov    eax,DWORD PTR [ebp-0x14]
  403006:\tmov    eax,0x40300d
  40300b:\tret
  40300c:\tnop
  40300d:\tjmp    0x403012
  40300f:\tnop
  403010:\txor    eax,eax
  403012:\tpop    ebp
  403013:\tret
  403014:\tmov    esp,DWORD PTR [ebp-0x18]
  403017:\tret
  403018:\tint3
";

    #[test]
    fn exception_code_inside_the_extent_is_the_functions_own() {
        let handlers = [0x403003, 0x403014];
        assert_eq!(
            problems_storing(&handlers, EXCEPTION, &[(0x403000, 20)]),
            Vec::<String>::new()
        );
        // The handler past the body's end is not pulled into the size.
        assert_eq!(
            problems_storing(&handlers, EXCEPTION, &[(0x403000, 24)]),
            ["0x403000: size 24, body 20"]
        );
        // Without the stored address the funclet is reached by nothing.
        assert_eq!(
            problems_storing(&[0x403014], EXCEPTION, &[(0x403000, 20)]),
            ["0x403000: 0x403003 lies inside the extent but outside the body"]
        );
        // A stored address that has a row is another function's entry, not exception code.
        assert_eq!(
            problems_storing(&handlers, EXCEPTION, &[(0x403000, 20), (0x403003, 9)]),
            ["0x403000: 0x403003, another row's entry, lies inside the extent"]
        );
    }

    /// Whether a missing exe or listing must fail a test rather than skip it.
    fn required() -> bool {
        std::env::var(crate::exe::REQUIRE_VARIABLE)
            .is_ok_and(|value| !value.is_empty() && value != "0")
    }

    /// The real exe and listing: rows of each kind a listing out of step or a narrow padding
    /// test once failed pass, and a size made wrong on purpose is caught.
    #[test]
    fn the_real_listing_passes_known_rows_and_catches_a_wrong_size() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the repo");
        let exe = crate::exe::locate(root);
        let path = root.join("dumps").join("disasm.txt");
        let present = crate::exe::identify(&exe).expect("the exe reads")
            == crate::exe::Identity::Reference
            && path.exists();
        if !present {
            assert!(
                !required(),
                "no reference exe and listing at {}",
                path.display()
            );
            eprintln!(
                "skipped: no reference exe at {} or no listing",
                exe.display()
            );
            return;
        }
        let checked = crate::check::run(&crate::tree::Disk::new(root)).expect("the table reads");
        let mut rows = crate::exe::functions(&checked);
        let known = [
            0x423240, 0x423b70, 0x46c2c0, 0x471080, 0x5aa920, 0x5aae20, 0x5b1e70, 0x5b24b0,
            0x5bcb00, 0x616620, 0x627240, 0x6e3b80, 0x704ae0, 0x764390, 0x7645a0, 0x768710,
            0x76c040, 0x74fa8a, 0x4098c0,
        ];
        // The CPUID helper, whose catch funclet 0x74fac4 and continuation 0x74fad6 lie in its
        // extent, at its body's size.
        if !rows.iter().any(|r| r.address == 0x74fa8a) {
            let at = rows.partition_point(|r| r.address < 0x74fa8a);
            rows.insert(
                at,
                Function {
                    place: "0x74fa8a".to_string(),
                    address: 0x74fa8a,
                    size: 161,
                },
            );
        }
        let wrong = 0x5bcb00;
        let row = rows.iter_mut().find(|r| r.address == wrong).expect("a row");
        let size = row.size;
        row.size -= 1;
        let place = row.place.clone();
        let listing = Listing::read(&path).expect("the listing reads");
        let bytes = std::fs::read(&exe).expect("the exe reads");
        let image = Image::new(&bytes).expect("a PE");
        let places: HashSet<String> = rows
            .iter()
            .filter(|r| {
                known
                    .iter()
                    .any(|&k| r.address <= k && k < r.address + r.size.max(1))
            })
            .map(|r| r.place.clone())
            .collect();
        assert_eq!(places.len(), known.len(), "the known rows are in the table");
        let found: Vec<String> = check(&rows, &listing, &image)
            .into_iter()
            .filter(|p| places.contains(&p.place))
            .map(|p| p.to_string())
            .collect();
        assert_eq!(found, [format!("{place}: size {}, body {size}", size - 1)]);
    }
}
