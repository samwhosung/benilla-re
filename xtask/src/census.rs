//! `cargo xtask census`: every function start in `.text` by the rules of `docs/METHOD.md`,
//! Completeness, each with its body's size by the rule in `symbols/README.md`, whether live code
//! reaches it, and the class its body gives it. `--write` adds a row for each start with none and
//! sets each row's size to its body's; `--fill` completes the rows it added (`fill.rs`).
//!
//! A start is a `call` target, except a `call` to the next instruction, which only pushes its
//! address; a stored address, a dword of `.rdata` or `.data` or an immediate operand pushed or
//! moved; the target of a `jmp` that lands on the first instruction after padding, or right after
//! a `ret` or `jmp` when a `jmp` from another function than the one above lands there; or the
//! first instruction no body covers after a path's end, padding or a table's data. It never is an
//! address the instruction before it falls through to, a conditional jump's target or a jump
//! table's entry: those are the code of the function above. An address that reached code pushes
//! or moves is the exception where the bytes before it that no body reaches decode as data
//! (printable text, as the C runtime's `VC20XC00` before `__except_handler3`, or an instruction
//! that does not decode): they are not code falling through, and a candidate found only after
//! padding that runs through them into the start is turned down. Padding starts nothing, except
//! the `mov edi,edi` a hot-patchable function opens with, which is padding before a branch target,
//! a `call` target or a start. A filter or handler in a scope table that a function's exception
//! frame pushes is that function's code. A stored address that the extent of a referenced start
//! holds is that function's own code (a continuation, a catch block), unless a code address sits
//! in the dword beside it, a table of entries, and so is an instruction
//! inside an extent that no padding precedes; a start found only after padding owns no stored
//! address. A candidate whose body is the decode of data (an undecodable first instruction, a table
//! of code addresses, an instruction compiled code never holds) is turned down. The jump tables
//! and the byte index tables the bodies read are data. The scans repeat, walking every body
//! afresh, until no start is new, since each new start ends the tail jumps to it and can uncover
//! code; the listing restarts at every row, so the census and `tools/disasm.sh` alternate until
//! both settle.
//!
//! A start whose body the check refuses (another entry or a dead instruction inside its extent,
//! or code above falling through into it) can hold no row; it is listed as refused.
//!
//! Whether anything reaches a start is the two scans of `docs/METHOD.md`: every `e8` or `e9` byte that
//! starts an instruction of `.text` and whose rel32 lands on it, and every little-endian
//! occurrence of its address in `.text`, `.rdata` and `.data`, inside one instruction in code and
//! at even offsets in data, outside the import tables, whose dwords are relative addresses of
//! import names; and every direct `jmp` or `jcc`, whose short forms name their target in a byte.
//! A vtable or handler table is live only when something live names it (`reach`). The entry point
//! and the hits nothing holds are the roots; `--why <address>` prints the chain from a root.

use crate::bodies::{
    self, end_of, in_ranges, is_filler, is_trap, Image, Listing, NoReturn, Tables, Walker, DIRECT,
    IMMEDIATE,
};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::LazyLock;

static CALL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^call\s+0x([0-9a-f]+)$").expect("a valid pattern"));
/// What compiled user-mode code never holds: the decode of data. `sahf` and `wait` are real.
static GARBAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:in |out |ins[bdw]|outs[bdw]|hlt|iret|lret|retf|ljmp|lcall|les |lds |arpl|bound|into|aaa|aas|aad|aam|daa|das|salc|icebp|int1|push +(?:es|cs|ss|ds)$|pop +(?:es|ss|ds)$|cli|sti)",
    )
    .expect("a valid pattern")
});

/// Whether a body is the decode of data: it opens with an instruction objdump cannot decode or
/// cuts short, or holds one compiled code never holds. One undecodable instruction inside a body
/// is no evidence: `dc d8`, an alias of `fcomp`, is real and objdump calls it `(bad)`.
fn decodes_data(listing: &Listing, image: &Image, entry: u32, reached: &[u32]) -> bool {
    let first = listing.get(entry).unwrap_or_default();
    if first == "(bad)" || first.starts_with(".byte") {
        return true;
    }
    // A table of code addresses: two aligned dwords from the entry on that are instructions
    // within 64 KiB.
    let near = |a: u32| {
        image
            .dword(a)
            .is_some_and(|v| listing.contains(v) && v.abs_diff(entry) < 0x10000)
    };
    if entry.is_multiple_of(4) && near(entry) && near(entry + 4) {
        return true;
    }
    reached
        .iter()
        .any(|&a| GARBAGE.is_match(listing.get(a).unwrap_or_default()))
}

/// Whether the bytes from `from` to `to` are data the listing decodes: a run of printable text of
/// four bytes or more (the C runtime's `VC20XC00` before `__except_handler3`), or one instruction
/// that does not decode.
fn data_signature(listing: &Listing, image: &Image, from: u32, to: u32) -> bool {
    let text = (from..to).all(|a| image.byte(a).is_some_and(|b| (0x20..0x7f).contains(&b)));
    let undecodable = listing
        .addresses
        .iter()
        .skip(listing.addresses.partition_point(|&a| a < from))
        .take_while(|&&a| a < to)
        .any(|&a| {
            let t = listing.get(a).unwrap_or_default();
            t == "(bad)" || t.starts_with(".byte")
        });
    to > from && ((to - from >= 4 && text) || undecodable)
}

/// Any absolute operand: `ds:0x7ff1a0`, `[eax*4+0x403470]`, `0x82e94c`.
static OPERAND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"0x([0-9a-f]{6,8})\b").expect("a valid pattern"));

/// How a start was found. A row's address keeps `Row` whatever else finds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum How {
    Row,
    Call,
    Stored,
    TailCall,
    AfterPadding,
}

impl How {
    pub fn word(self) -> &'static str {
        match self {
            How::Row => "row",
            How::Call => "call",
            How::Stored => "stored",
            How::TailCall => "tail-call",
            How::AfterPadding => "after-padding",
        }
    }
}

/// One start: how it was found, its body and what the body says.
pub struct Start {
    pub address: u32,
    pub how: How,
    /// From the entry to the end of the body's last instruction, or none when the body runs to
    /// the end of the listing.
    pub size: Option<u32>,
    pub live: bool,
    pub class: &'static str,
    /// The direct `call` targets of the body, in address order of the calls, each once.
    pub callees: Vec<u32>,
    /// Sites of the `e8`/`e9` and stored-dword hits on it.
    pub hits: usize,
}

/// A candidate turned down, with the reason.
pub struct Rejected {
    pub address: u32,
    pub how: How,
    pub reason: String,
}

pub struct Census {
    pub starts: BTreeMap<u32, Start>,
    pub rejected: Vec<Rejected>,
    pub rounds: usize,
    /// Byte index tables whose length no bound gave, by the read's address.
    pub unbounded: Vec<u32>,
    /// Unaligned matches in data of a start nothing live reaches: the site and the start.
    pub unaligned: Vec<(u32, u32)>,
    pub parents: Parents,
    /// Rows that are no function start, with the reason.
    pub not_starts: Vec<(u32, String)>,
}

/// The listing's facts that depend on which bytes are data.
struct Scan {
    calls: BTreeMap<u32, Vec<u32>>,
    jmps: BTreeMap<u32, Vec<u32>>,
    jccs: HashSet<u32>,
    branch_targets: HashSet<u32>,
    immediates: Vec<(u32, u32)>,
}

fn scan(listing: &Listing, data: &[(u32, u32)]) -> Scan {
    let mut s = Scan {
        calls: BTreeMap::new(),
        jmps: BTreeMap::new(),
        jccs: HashSet::new(),
        branch_targets: HashSet::new(),
        immediates: Vec::new(),
    };
    for i in 0..listing.len() {
        let address = listing.addresses[i];
        if in_ranges(data, address) {
            continue;
        }
        let instruction = listing.at(i);
        if let Some(found) = CALL.captures(instruction) {
            // A `call` to the next instruction pushes its own address and enters nothing.
            if let Ok(target) = u32::from_str_radix(&found[1], 16) {
                if listing.addresses.get(i + 1) != Some(&target) {
                    s.calls.entry(target).or_default().push(address);
                }
            }
        } else if let Some(found) = DIRECT.captures(instruction) {
            if let Ok(target) = u32::from_str_radix(&found[2], 16) {
                s.branch_targets.insert(target);
                if &found[1] == "jmp" {
                    s.jmps.entry(target).or_default().push(address);
                } else {
                    s.jccs.insert(target);
                }
            }
        } else if let Some(found) = IMMEDIATE.captures(instruction) {
            if let Ok(value) = u32::from_str_radix(&found[1], 16) {
                s.immediates.push((address, value));
            }
        }
    }
    s
}

/// Where the image keeps its code, its import slots and its entry.
struct Layout {
    text: (u32, u32),
    iat: (u32, u32),
    entry: u32,
}

/// The `.text` section's start and end, the import address table's range and the entry point.
fn layout(bytes: &[u8]) -> Option<Layout> {
    let u16_at = |o: usize| Some(u16::from_le_bytes(bytes.get(o..o + 2)?.try_into().ok()?));
    let u32_at = |o: usize| Some(u32::from_le_bytes(bytes.get(o..o + 4)?.try_into().ok()?));
    let pe = u32_at(0x3c)? as usize;
    let optional = pe + 24;
    let base = u32_at(optional + 28)?;
    let entry = base + u32_at(optional + 16)?;
    let iat = (
        base + u32_at(optional + 96 + 12 * 8)?,
        u32_at(optional + 100 + 12 * 8)?,
    );
    let table = optional + u16_at(pe + 20)? as usize;
    let count = u16_at(pe + 6)? as usize;
    for i in 0..count {
        let header = table + 40 * i;
        if bytes.get(header..header + 5)? == b".text" {
            let start = base + u32_at(header + 12)?;
            let size = u32_at(header + 8)?;
            return Some(Layout {
                text: (start, start + size),
                iat: (iat.0, iat.0 + iat.1),
                entry,
            });
        }
    }
    None
}

/// The class a body's instructions give it (`symbols/README.md`): `host` when it reads an
/// import's slot, `primitive` when it computes on values, else `orchestration`. Computing is any
/// arithmetic, logic, shift or floating-point operation, or a conversion, except adjusting or
/// aligning `esp` and the idioms that only load a constant: `xor r,r`, `sub r,r`,
/// `or r,0xffffffff` and `and r,0x0`.
pub fn class_of(texts: &[&str], iat: (u32, u32)) -> &'static str {
    let mut host = false;
    let mut computes = false;
    for (i, text) in texts.iter().enumerate() {
        for found in OPERAND.captures_iter(text) {
            if let Ok(value) = u32::from_str_radix(&found[1], 16) {
                host |= iat.0 <= value && value < iat.1;
            }
        }
        computes |= computes_on_values(text) && !address_arithmetic(texts, i);
    }
    if host {
        "host"
    } else if computes {
        "primitive"
    } else {
        "orchestration"
    }
}

/// Whether `texts[i]` adds to or steps a register (`add ecx,0x8`, `add eax,DWORD PTR [ecx+0x10]`,
/// `inc esi`) that the code after it uses only as an address: inside a memory operand, or as
/// `this` (`ecx`) of a call. Computing an address is not computing on a value.
fn address_arithmetic(texts: &[&str], i: usize) -> bool {
    let (mnemonic, operands) = texts[i].split_once(' ').unwrap_or((texts[i], ""));
    let register = match mnemonic {
        "add" | "sub" => match operands.split_once(',') {
            Some((r, _)) => r,
            _ => return false,
        },
        "inc" | "dec" => operands,
        _ => return false,
    };
    if !(register.len() == 3 && register.starts_with('e') && register != "esp") {
        return false;
    }
    let mentions = |t: &str| {
        t.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w == register)
    };
    for text in texts.iter().skip(i + 1).take(8) {
        let inside = text
            .split_once('[')
            .and_then(|(_, rest)| rest.split_once(']'))
            .is_some_and(|(memory, _)| mentions(memory));
        if inside || (register == "ecx" && text.starts_with("call")) {
            return true;
        }
        if mentions(text) || text.starts_with("ret") || text.starts_with('j') {
            return false;
        }
        if text.starts_with("call") {
            return false;
        }
    }
    false
}

const INTEGER: [&str; 27] = [
    "add", "adc", "sub", "sbb", "imul", "mul", "div", "idiv", "neg", "not", "inc", "dec", "and",
    "or", "xor", "shl", "sal", "shr", "sar", "rol", "ror", "rcl", "rcr", "shld", "shrd", "bswap",
    "xadd",
];
const FLOAT: [&str; 22] = [
    "fadd", "fiadd", "fsub", "fisub", "fmul", "fimul", "fdiv", "fidiv", "fsqrt", "fabs", "fchs",
    "fsin", "fcos", "fsincos", "fptan", "fpatan", "f2xm1", "fyl2x", "fscale", "frndint", "fprem",
    "fxtract",
];

/// SSE, MMX and 3DNow operations that compute, by mnemonic prefix.
const VECTOR: [&str; 34] = [
    "add", "sub", "mul", "div", "sqrt", "rsqrt", "rcp", "min", "max", "and", "or", "xor", "cvt",
    "padd", "psub", "pmul", "pmadd", "psll", "psrl", "psra", "pand", "por", "pxor", "pf", "pi2f",
    "pavg", "pmin", "pmax", "psad", "shuf", "unpck", "pshuf", "punpck", "pack",
];

/// Whether one instruction computes on values (`class_of`).
pub fn computes_on_values(text: &str) -> bool {
    let (mnemonic, operands) = text.split_once(' ').unwrap_or((text, ""));
    let (left, right) = operands
        .trim()
        .split_once(',')
        .unwrap_or((operands.trim(), ""));
    if INTEGER.contains(&mnemonic) {
        let constant = (matches!(mnemonic, "xor" | "sub") && left == right)
            || (mnemonic == "or" && right == "0xffffffff")
            || (mnemonic == "and" && right == "0x0");
        let stack = left == "esp" && matches!(mnemonic, "add" | "sub" | "and");
        return !constant && !stack;
    }
    if mnemonic.starts_with('f') {
        // `fsubrp` is `fsub` reversed and popping; `fistp` and `fisttp` convert to integers.
        let base = mnemonic.trim_end_matches('p');
        let base = base.strip_suffix('r').unwrap_or(base);
        return FLOAT.contains(&base)
            || mnemonic.starts_with("fist")
            || mnemonic.starts_with("fyl2x")
            || mnemonic.starts_with("fprem");
    }
    let packed = ["ss", "ps", "sd", "pd"]
        .iter()
        .any(|s| mnemonic.ends_with(s))
        || mnemonic.starts_with('p')
        || mnemonic.starts_with("cvt");
    let moves = [
        "mov", "push", "pop", "prefetch", "pause", "comis", "ucomis", "pcmp",
    ]
    .iter()
    .any(|m| mnemonic.starts_with(m));
    packed
        && !moves
        && VECTOR.iter().any(|v| mnemonic.starts_with(v))
        && !(mnemonic.contains("xor") && left == right)
}

struct Context<'a> {
    listing: &'a Listing,
    data: Vec<(u32, u32)>,
    scan: Scan,
    cases: HashSet<u32>,
    /// The starts so far: a filler that is one is an entry, not padding.
    starts: HashSet<u32>,
    no_return: &'a NoReturn,
}

impl Context<'_> {
    /// Whether an instruction ends a path, so the next one is never reached by falling through:
    /// `ret`, `jmp`, `int3`, or a call that never returns.
    fn ends(&self, instruction: &str) -> bool {
        instruction.starts_with("ret")
            || instruction.starts_with("jmp")
            || is_trap(instruction)
            || self.no_return.ends(instruction)
    }

    /// A hot-patchable entry: `mov edi,edi` followed by code, not by more padding.
    fn hot_patch_entry(&self, address: u32) -> bool {
        self.listing.get(address) == Some("mov edi,edi")
            && self.listing.following(address).is_some_and(|n| {
                let next = self.listing.get(n).unwrap_or_default();
                // Before a table, a branch target, a `call` target or another start the two bytes
                // are padding: what reaches the code enters after them.
                n == address + 2
                    && !in_ranges(&self.data, n)
                    && !is_trap(next)
                    && !is_filler(next)
                    && !self.scan.calls.contains_key(&n)
                    && !self.scan.branch_targets.contains(&n)
                    && !self.starts.contains(&n)
            })
    }

    /// A hot-patchable entry after a path's end, padding between: the function starts at its
    /// `mov edi,edi`, which is no padding.
    fn opens_function(&self, address: u32) -> bool {
        if !self.hot_patch_entry(address) {
            return false;
        }
        let mut at = self.listing.preceding(address);
        while let Some(b) = at {
            let text = self.listing.get(b).unwrap_or_default();
            if in_ranges(&self.data, b) || is_trap(text) || is_filler(text) {
                at = self.listing.preceding(b);
                continue;
            }
            return self.ends(text);
        }
        true
    }

    fn padding(&self, address: u32) -> bool {
        let instruction = self.listing.get(address).unwrap_or_default();
        is_trap(instruction)
            || (is_filler(instruction)
                && !self.scan.branch_targets.contains(&address)
                && !self.opens_function(address))
    }

    /// The instruction before `address` that is neither padding nor data, and whether padding
    /// or data lay between them.
    fn before(&self, address: u32) -> (Option<u32>, bool, bool) {
        let (mut skipped_padding, mut skipped_data) = (false, false);
        let mut at = self.listing.preceding(address);
        while let Some(b) = at {
            if in_ranges(&self.data, b) {
                skipped_data = true;
            } else if self.padding(b) && !self.starts.contains(&b) {
                skipped_padding = true;
            } else {
                break;
            }
            at = self.listing.preceding(b);
        }
        // Data the listing leaves out sits between `b` and `address` without an instruction.
        if let Some(b) = at {
            let i = self.data.partition_point(|r| r.0 <= b);
            if self.data.get(i).is_some_and(|r| r.0 < address) {
                skipped_data = true;
            }
        }
        (at, skipped_padding, skipped_data)
    }

    /// Why `address` is not a function start, or none when nothing reaches it from above. A
    /// `call` target is a start whatever reaches it from above, so long as it is code.
    fn not_a_start(&self, address: u32) -> Option<String> {
        if !self.listing.contains(address) {
            return Some("not an instruction start in the listing".to_string());
        }
        if in_ranges(&self.data, address) {
            return Some("inside a jump or index table".to_string());
        }
        if self.cases.contains(&address) {
            return Some("a jump table's entry".to_string());
        }
        if self.scan.calls.contains_key(&address) {
            return None;
        }
        if self.scan.jccs.contains(&address) {
            return Some("a conditional jump's target".to_string());
        }
        let (before, _, data) = self.before(address);
        if let Some(b) = before {
            if !data && !self.ends(self.listing.get(b).unwrap_or_default()) {
                return Some(format!("falls through from {b:#x}"));
            }
        }
        None
    }

    /// Where `address` sits after the end of a path: `Some(true)` after padding or data,
    /// `Some(false)` right after a `ret` or `jmp` with no padding between, none when the code
    /// before it can fall through into it.
    fn after_end(&self, address: u32) -> Option<bool> {
        let (before, padding, data) = self.before(address);
        if data || before.is_none() {
            return Some(true);
        }
        let ended = before.is_some_and(|b| self.ends(self.listing.get(b).unwrap_or_default()));
        ended.then_some(padding)
    }
}

/// The sections that hold code and the data it points at, `.text`, `.rdata` and `.data`, as
/// their virtual address and file bytes. The resources and the safe-handler table hold no
/// address.
fn program_sections(bytes: &[u8]) -> Vec<(u32, &[u8])> {
    let u16_at = |o: usize| Some(u16::from_le_bytes(bytes.get(o..o + 2)?.try_into().ok()?));
    let u32_at = |o: usize| Some(u32::from_le_bytes(bytes.get(o..o + 4)?.try_into().ok()?));
    let mut out = Vec::new();
    let Some(pe) = u32_at(0x3c).map(|p| p as usize) else {
        return out;
    };
    let (Some(count), Some(optional), Some(base)) =
        (u16_at(pe + 6), u16_at(pe + 20), u32_at(pe + 52))
    else {
        return out;
    };
    let table = pe + 24 + optional as usize;
    for i in 0..count as usize {
        let header = table + 40 * i;
        let Some(name) = bytes.get(header..header + 8) else {
            continue;
        };
        let name = String::from_utf8_lossy(name);
        let name = name.trim_end_matches('\0');
        if !matches!(name, ".text" | ".rdata" | ".data") {
            continue;
        }
        let (Some(address), Some(size), Some(offset)) = (
            u32_at(header + 12),
            u32_at(header + 16),
            u32_at(header + 20),
        ) else {
            continue;
        };
        if let Some(raw) = bytes.get(offset as usize..(offset + size) as usize) {
            out.push((base + address, raw));
        }
    }
    out
}

/// The import directory's own tables, as address ranges: the descriptors, and each library's
/// name table and address table. Their dwords are relative addresses of import names, which can
/// read as code addresses, never pointers to code.
fn import_tables(bytes: &[u8]) -> Vec<(u32, u32)> {
    let u16_at = |o: usize| Some(u16::from_le_bytes(bytes.get(o..o + 2)?.try_into().ok()?));
    let u32_at = |o: usize| Some(u32::from_le_bytes(bytes.get(o..o + 4)?.try_into().ok()?));
    let mut out = Vec::new();
    let Some(pe) = u32_at(0x3c).map(|p| p as usize) else {
        return out;
    };
    let optional = pe + 24;
    let (Some(base), Some(directory), Some(size), Some(count), Some(header)) = (
        u32_at(optional + 28),
        u32_at(optional + 104),
        u32_at(optional + 108),
        u16_at(pe + 6),
        u16_at(pe + 20),
    ) else {
        return out;
    };
    let table = optional + header as usize;
    let offset = |rva: u32| -> Option<usize> {
        (0..count as usize).find_map(|i| {
            let h = table + 40 * i;
            let (va, raw, at) = (u32_at(h + 12)?, u32_at(h + 16)?, u32_at(h + 20)?);
            (va <= rva && rva < va + raw).then(|| (at + rva - va) as usize)
        })
    };
    out.push((base + directory, base + directory + size));
    let mut descriptor = directory;
    while let Some(at) = offset(descriptor) {
        let (Some(names), Some(slots)) = (u32_at(at), u32_at(at + 16)) else {
            break;
        };
        if names == 0 && slots == 0 {
            break;
        }
        for array in [names, slots].into_iter().filter(|&a| a != 0) {
            let mut k = 0;
            while offset(array + 4 * k)
                .and_then(u32_at)
                .is_some_and(|v| v != 0)
            {
                k += 1;
            }
            out.push((base + array, base + array + 4 * (k + 1)));
        }
        descriptor += 20;
    }
    bodies::join(out)
}

/// Runs the census over `listing` from the function rows at `rows`.
pub fn run(listing: &Listing, bytes: &[u8], rows: &[u32]) -> Option<Census> {
    let image = Image::new(bytes)?;
    let Layout {
        text: (text_start, text_end),
        iat,
        entry,
    } = layout(bytes)?;
    let mut how: BTreeMap<u32, How> = rows.iter().map(|&r| (r, How::Row)).collect();
    let mut rejected: BTreeMap<u32, Rejected> = BTreeMap::new();
    // Dwords of the sections other than `.text`, at even offsets: vtables, handler tables and
    // packed records.
    let imports = import_tables(bytes);
    let mut stored_data: Vec<u32> = program_sections(bytes)
        .into_iter()
        .filter(|&(start, _)| start != text_start)
        .flat_map(|(start, raw)| {
            (0..raw.len().saturating_sub(3))
                .step_by(2)
                .filter(|&k| !in_ranges(&imports, start + k as u32))
                .map(|k| u32::from_le_bytes(raw[k..k + 4].try_into().expect("four bytes")))
                .filter(|v| (text_start..text_end).contains(v))
                .collect::<Vec<u32>>()
        })
        .collect();
    stored_data.sort_unstable();
    stored_data.dedup();
    // The stored addresses with a code address in the dword before or after: a table of entries,
    // such as the run-time library's packed records of `cosh` and `tanh`, whose entries are
    // functions however near another function's code they lie.
    let tabled: HashSet<u32> = program_sections(bytes)
        .into_iter()
        .filter(|&(start, _)| start != text_start)
        .flat_map(|(start, raw)| {
            let code = |k: usize| {
                raw.get(k..k + 4)
                    .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")))
                    .filter(|v| (text_start..text_end).contains(v) && listing.contains(*v))
            };
            (4..raw.len().saturating_sub(3))
                .step_by(2)
                .filter(|&k| !in_ranges(&imports, start + k as u32))
                .filter_map(|k| {
                    let v = code(k)?;
                    (code(k - 4).is_some() || code(k + 4).is_some()).then_some(v)
                })
                .collect::<Vec<u32>>()
        })
        .collect();
    let exception_code = bodies::exception_code(listing, &image, (text_start, text_end));
    let mut rounds = 0;
    let mut inner_rows: Vec<(u32, String)>;
    loop {
        rounds += 1;
        // What a round turns down is the last round's word: a later start can own it.
        rejected.clear();
        let addresses: Vec<u32> = how.keys().copied().collect();
        let no_return = bodies::no_return(&addresses, listing, &image);
        let walker = Walker {
            listing,
            image: &image,
            starts: addresses.iter().copied().collect(),
            no_return: &no_return,
        };
        let bodies: BTreeMap<u32, (Vec<u32>, Tables)> = how
            .keys()
            .filter(|&&a| listing.contains(a))
            .map(|&a| (a, walker.body(a)))
            .collect();
        let mut tables = Tables::new();
        let mut index_ranges = Vec::new();
        let mut unbounded = Vec::new();
        for (entry, (reached, found)) in &bodies {
            bodies::merge_tables(&mut tables, found.clone());
            let (ranges, missing) = bodies::index_tables(listing, &image, *entry, reached);
            index_ranges.extend(ranges);
            unbounded.extend(missing);
        }
        let mut data = bodies::table_ranges(&tables);
        data.extend(index_ranges);
        let data = bodies::join(data);
        let cases: HashSet<u32> = tables.values().flatten().copied().collect();
        let context = Context {
            listing,
            scan: scan(listing, &data),
            data,
            cases,
            starts: how.keys().copied().collect(),
            no_return: &no_return,
        };
        let extent = |entry: u32, reached: &[u32]| -> Option<u32> {
            let last = reached.last().copied().unwrap_or(entry);
            end_of(listing, &context.data, last, image.section_end(last))
        };
        let mut extents: Vec<(u32, u32)> = bodies
            .iter()
            .filter_map(|(&a, (reached, _))| Some((a, extent(a, reached)?)))
            .collect();
        extents.sort_unstable();

        // A start found only as a stored address that another start's extent holds is that
        // function's own code: a catch block, a continuation, a coincidence of data.
        // An extent that holds a row or a `call` target is a body the check refuses, and owns
        // nothing; nor does the extent of a start that nothing references, found only after
        // padding, against an address something stores.
        let firm: Vec<u32> = how
            .iter()
            .filter(|(_, h)| matches!(h, How::Row | How::Call))
            .map(|(&a, _)| a)
            .collect();
        let sound: Vec<(u32, u32)> = extents
            .iter()
            .copied()
            .filter(|&(start, end)| {
                let i = firm.partition_point(|&f| f <= start);
                firm.get(i).is_none_or(|&f| f >= end)
            })
            .collect();
        let owning: Vec<(u32, u32)> = sound
            .iter()
            .copied()
            .filter(|(start, _)| how.get(start) != Some(&How::AfterPadding))
            .collect();
        let owns = |address: u32| {
            let i = owning.partition_point(|e| e.0 < address);
            owning[..i].iter().rev().take(64).any(|e| address < e.1)
        };
        let within = |address: u32| {
            let i = sound.partition_point(|e| e.0 < address);
            sound[..i].iter().rev().take(64).any(|e| address < e.1)
        };
        let owned: Vec<u32> = how
            .iter()
            .filter(|(_, h)| matches!(h, How::Stored))
            .map(|(&a, _)| a)
            .filter(|&a| {
                (owns(a) && !tabled.contains(&a))
                    || context.cases.contains(&a)
                    || in_ranges(&context.data, a)
            })
            .collect();
        if !owned.is_empty() {
            for a in owned {
                how.remove(&a);
            }
            continue;
        }
        // An address that reached code pushes or moves is a start even where the bytes before
        // it would fall through into it, so long as no body reaches those bytes: they are data
        // (the C runtime puts `VC20XC00` before `__except_handler3`).
        let covered_by_bodies: HashSet<u32> = bodies
            .values()
            .flat_map(|(reached, _)| reached.iter().copied())
            .collect();
        let pushed: HashSet<u32> = context
            .scan
            .immediates
            .iter()
            .filter(|(site, _)| covered_by_bodies.contains(site))
            .map(|&(_, value)| value)
            .collect();
        // The run of instructions no body reaches right before `address`, back to padding or a
        // path's end.
        let run_before = |address: u32| {
            let mut from = address;
            while let Some(p) = listing.preceding(from) {
                let text = listing.get(p).unwrap_or_default();
                if covered_by_bodies.contains(&p) || context.padding(p) || context.ends(text) {
                    break;
                }
                from = p;
            }
            from
        };
        let not_a_start = |address: u32| {
            context.not_a_start(address).filter(|why| {
                !(why.starts_with("falls through from")
                    && pushed.contains(&address)
                    && data_signature(listing, &image, run_before(address), address))
            })
        };
        // A row the same holds, or that is no start, is listed.
        // Every instruction another start's walk reaches, with that start.
        let mut reached_by: HashMap<u32, u32> = HashMap::new();
        for (&entry, (reached, _)) in &bodies {
            for &a in reached {
                if a != entry {
                    reached_by.entry(a).or_insert(entry);
                }
            }
        }
        inner_rows = how
            .iter()
            .filter(|(_, h)| **h == How::Row)
            .map(|(&a, _)| a)
            .filter_map(|a| {
                let owner = reached_by
                    .get(&a)
                    .filter(|_| !context.scan.calls.contains_key(&a));
                if let Some(owner) = owner {
                    Some((a, format!("inside the body of {owner:#x}")))
                } else if let Some(table) = exception_code
                    .get(&a)
                    .filter(|_| !context.scan.calls.contains_key(&a))
                {
                    Some((
                        a,
                        format!("exception code in the scope table at {table:#x}"),
                    ))
                } else if within(a) {
                    Some((a, "inside another start's extent".to_string()))
                } else if context.padding(a)
                    && !context.scan.calls.contains_key(&a)
                    && !context.hot_patch_entry(a)
                {
                    Some((a, "padding that nothing calls".to_string()))
                } else if bodies
                    .get(&a)
                    .is_some_and(|(reached, _)| decodes_data(listing, &image, a, reached))
                {
                    Some((a, "its body decodes data".to_string()))
                } else {
                    not_a_start(a).map(|why| (a, why))
                }
            })
            .collect();

        let mut new: BTreeMap<u32, How> = BTreeMap::new();
        let mut consider = |address: u32, found: How, new: &mut BTreeMap<u32, How>| {
            if how.contains_key(&address) || new.contains_key(&address) {
                return;
            }
            if !(text_start..text_end).contains(&address) {
                return;
            }
            // Padding starts nothing: a stored address on it is a pointer into the gap between
            // functions, never an entry, unless it is the `mov edi,edi; push ebp` a hot-patchable
            // function opens with.
            let unnamed_padding = match found {
                How::AfterPadding | How::TailCall => context.padding(address),
                How::Stored => context.padding(address) && !context.hot_patch_entry(address),
                _ => false,
            };
            let garbage = || {
                (found != How::Call
                    && decodes_data(listing, &image, address, &walker.body(address).0))
                .then(|| "its body decodes data".to_string())
            };
            // Code found only after padding that falls through into a start is the data before
            // that start, decoded.
            let runs_into = || {
                if found != How::AfterPadding {
                    return None;
                }
                let (reached, _) = walker.body(address);
                reached
                    .iter()
                    .copied()
                    .find(|&a| {
                        a != address
                            && (how.contains_key(&a) || new.contains_key(&a))
                            && listing.preceding(a).is_some_and(|p| {
                                reached.binary_search(&p).is_ok()
                                    && !context.ends(listing.get(p).unwrap_or_default())
                            })
                    })
                    .filter(|&a| data_signature(listing, &image, address, a))
                    .map(|a| format!("its bytes before the start at {a:#x} are data"))
            };
            let refused = if found == How::Stored {
                not_a_start(address)
            } else {
                context.not_a_start(address)
            };
            match refused
                .or_else(|| unnamed_padding.then(|| "padding".to_string()))
                .or_else(runs_into)
                .or_else(garbage)
            {
                None => {
                    rejected.remove(&address);
                    new.insert(address, found);
                }
                Some(reason) => {
                    rejected.entry(address).or_insert(Rejected {
                        address,
                        how: found,
                        reason,
                    });
                }
            }
        };
        for &target in context.scan.calls.keys() {
            consider(target, How::Call, &mut new);
        }
        let immediates = context.scan.immediates.iter().map(|&(_, v)| v);
        // A stored address an extent holds is its function's own code; listed, not a start.
        let mut owned_stored: Vec<Rejected> = Vec::new();
        for value in stored_data.iter().copied().chain(immediates) {
            if let Some(table) = exception_code.get(&value) {
                owned_stored.push(Rejected {
                    address: value,
                    how: How::Stored,
                    reason: format!("exception code in the scope table at {table:#x}"),
                });
                continue;
            }
            if (text_start..text_end).contains(&value)
                && listing.contains(value)
                && !how.contains_key(&value)
                && owns(value)
                && !tabled.contains(&value)
            {
                let i = owning.partition_point(|e| e.0 < value);
                let owner = owning[..i]
                    .iter()
                    .rev()
                    .find(|e| value < e.1)
                    .map_or(0, |e| e.0);
                owned_stored.push(Rejected {
                    address: value,
                    how: How::Stored,
                    reason: format!("inside the extent of {owner:#x}"),
                });
                continue;
            }
            if listing.contains(value) || how.contains_key(&value) {
                consider(value, How::Stored, &mut new);
            }
        }
        // A `jmp` to the first instruction after padding is a tail call. Right after a `ret`
        // with no padding, it is one only when a `jmp` from another function than the one
        // above lands there; one the function above alone jumps to is its exit block.
        for (&target, sites) in &context.scan.jmps {
            match context.after_end(target) {
                Some(true) => consider(target, How::TailCall, &mut new),
                Some(false) => {
                    // The function a site lies in is the nearest start at or before it; a jump
                    // from the function above is its own, whether or not its walk reaches it yet.
                    let owner = |a: u32| how.range(..=a).next_back().map(|(&s, _)| s);
                    let above = how.range(..target).next_back().map(|(&a, _)| a);
                    let outside = sites.iter().any(|&s| owner(s) != above);
                    if outside {
                        consider(target, How::TailCall, &mut new);
                    }
                }
                None => {}
            }
        }
        if new.is_empty() {
            // The first instruction no body covers after a path's end, padding or data. A body
            // covers what its walk reaches and its own exception code: what the stored addresses
            // inside its extent reach.
            let mut stored: Vec<u32> = stored_data.clone();
            stored.extend(context.scan.immediates.iter().map(|&(_, v)| v));
            stored.sort_unstable();
            stored.dedup();
            let mut covered: HashSet<u32> = HashSet::new();
            for &handler in exception_code.keys() {
                covered.extend(walker.walk(handler, vec![handler]).0);
            }
            for (&entry, (reached, _)) in &bodies {
                covered.extend(reached.iter().copied());
                let Some(end) = extent(entry, reached) else {
                    continue;
                };
                let seeds: Vec<u32> = stored[stored.partition_point(|&a| a <= entry)..]
                    .iter()
                    .copied()
                    .take_while(|&a| a < end)
                    .filter(|a| reached.binary_search(a).is_err() && !how.contains_key(a))
                    .collect();
                if !seeds.is_empty() {
                    covered.extend(walker.walk(entry, seeds).0);
                }
            }
            let mut turned_down_run = 0u32;
            let mut found_data: Vec<(u32, u32)> = Vec::new();
            for i in 0..listing.len() {
                let address = listing.addresses[i];
                if covered.contains(&address)
                    || in_ranges(&context.data, address)
                    || in_ranges(&found_data, address)
                    || context.padding(address)
                    || !(text_start..text_end).contains(&address)
                {
                    continue;
                }
                // Inside another start's extent, only code after padding starts a function; an
                // instruction right after a path's end there is that function's dead code.
                if within(address) && context.after_end(address) != Some(true) {
                    continue;
                }
                let (before, _, _) = context.before(address);
                if before.is_some_and(|b| b == turned_down_run) {
                    // Falls through from code just turned down: the same run.
                    turned_down_run = address;
                    continue;
                }
                consider(address, How::AfterPadding, &mut new);
                if new.contains_key(&address) {
                    // Its body is walked next round; what it covers is not a start, and neither
                    // are its tables, which follow it.
                    let (reached, tables) = walker.body(address);
                    let mut more = bodies::table_ranges(&tables);
                    more.extend(bodies::index_tables(listing, &image, address, &reached).0);
                    covered.extend(reached);
                    found_data.extend(more);
                    found_data = bodies::join(found_data);
                } else {
                    turned_down_run = address;
                }
            }
        }
        for r in owned_stored {
            rejected.entry(r.address).or_insert(r);
        }
        if new.is_empty() {
            let mut census = finish(
                listing, &image, bytes, how, rejected, rounds, unbounded, context, bodies, extents,
                iat, entry, text_start,
            );
            census.not_starts = inner_rows;
            return Some(census);
        }
        how.extend(new);
        if rounds > 64 {
            return None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn finish(
    listing: &Listing,
    image: &Image,
    bytes: &[u8],
    how: BTreeMap<u32, How>,
    rejected: BTreeMap<u32, Rejected>,
    rounds: usize,
    unbounded: Vec<u32>,
    context: Context,
    bodies: BTreeMap<u32, (Vec<u32>, Tables)>,
    extents: Vec<(u32, u32)>,
    iat: (u32, u32),
    entry: u32,
    text_start: u32,
) -> Census {
    let starts: HashSet<u32> = how.keys().copied().collect();
    let Reach {
        live,
        hits,
        unaligned,
        parents,
    } = reach(
        listing,
        &context.data,
        bytes,
        &starts,
        &extents,
        entry,
        text_start,
    );

    let mut out = BTreeMap::new();
    for (&address, &found) in &how {
        let body = bodies.get(&address);
        let size = body.and_then(|(reached, _)| {
            let last = reached.last().copied().unwrap_or(address);
            Some(end_of(listing, &context.data, last, image.section_end(last))? - address)
        });
        let texts: Vec<&str> = body
            .map(|(reached, _)| reached.iter().filter_map(|&a| listing.get(a)).collect())
            .unwrap_or_default();
        let mut callees = Vec::new();
        for text in &texts {
            if let Some(c) = CALL.captures(text) {
                if let Ok(t) = u32::from_str_radix(&c[1], 16) {
                    if !callees.contains(&t) {
                        callees.push(t);
                    }
                }
            }
        }
        let is_live = live.contains(&address);
        let class = if is_live {
            class_of(&texts, iat)
        } else {
            "unreached"
        };
        out.insert(
            address,
            Start {
                address,
                how: found,
                size,
                live: is_live,
                class,
                callees,
                hits: hits.get(&address).copied().unwrap_or(0),
            },
        );
    }
    Census {
        starts: out,
        rejected: rejected.into_values().collect(),
        rounds,
        unbounded,
        unaligned,
        parents,
        not_starts: Vec::new(),
    }
}

/// Prints the census against the table: every start with no row, every row whose size is not its
/// body's, what was turned down, and the controls. True when every start has a row at its size.
pub fn report(
    checked: &crate::check::Checked,
    listing: &Listing,
    bytes: &[u8],
    all: bool,
    why: Option<u32>,
    write: Option<&std::path::Path>,
) -> std::io::Result<bool> {
    let rows: Vec<(u32, u32, String)> = crate::exe::functions(checked)
        .into_iter()
        .map(|f| (f.address, f.size, f.place))
        .collect();
    let classes: HashMap<u32, String> = checked
        .functions
        .iter()
        .filter_map(|r| {
            let a = u32::from_str_radix(r.get("address").strip_prefix("0x")?, 16).ok()?;
            Some((a, r.get("class").to_string()))
        })
        .collect();
    let addresses: Vec<u32> = rows.iter().map(|r| r.0).collect();
    let Some(census) = run(listing, bytes, &addresses) else {
        return Err(std::io::Error::other("the census did not settle"));
    };
    if let Some(address) = why {
        let mut node = Node::Function(address);
        if !census.parents.contains_key(&node) {
            println!("{address:#x}: nothing live reaches it");
        }
        while let Some(&(site, from)) = census.parents.get(&node) {
            let shown = |n: Node| match n {
                Node::Function(a) => format!("function {a:#x}"),
                Node::Segment(a) => format!("data from {a:#x}"),
            };
            match from {
                Some(f) => println!(
                    "{} <- the reference at {site:#x} in {}",
                    shown(node),
                    shown(f)
                ),
                None => println!("{} <- the reference at {site:#x}, a root", shown(node)),
            }
            match from {
                Some(f) if f != node => node = f,
                _ => break,
            }
        }
        return Ok(true);
    }
    // The body check over every start at its census size: what a row there would fail. A start
    // with no row whose body it refuses can hold no row: it is listed, not owed.
    let image = Image::new(bytes).ok_or_else(|| std::io::Error::other("no section table"))?;
    let proposed: Vec<bodies::Function> = census
        .starts
        .values()
        .filter_map(|s| {
            Some(bodies::Function {
                place: format!("{:#x}", s.address),
                address: s.address,
                size: s.size?,
                pops: None,
            })
        })
        .collect();
    let failing = bodies::check(&proposed, listing, &image);
    let refused: HashSet<u32> = failing
        .iter()
        .filter_map(|p| u32::from_str_radix(p.place.trim_start_matches("0x"), 16).ok())
        .filter(|a| census.starts.get(a).is_some_and(|s| s.how != How::Row))
        .collect();
    let mut clean = true;
    println!("# starts with no row: address, size, how found, live, class, hits");
    let mut by_how: BTreeMap<&str, usize> = BTreeMap::new();
    for start in census.starts.values().filter(|s| all || s.how != How::Row) {
        let kind = match start.how {
            How::Row => "row",
            _ if refused.contains(&start.address) => "refused",
            _ => {
                clean = false;
                *by_how.entry(start.how.word()).or_default() += 1;
                "new"
            }
        };
        println!(
            "{kind}\t{:#x}\t{}\t{}\t{}\t{}\t{}",
            start.address,
            start.size.map_or("-".to_string(), |s| s.to_string()),
            start.how.word(),
            if start.live { "live" } else { "unreached" },
            start.class,
            start.hits
        );
    }
    println!("# what the body check finds over every start at its census size");
    for problem in &failing {
        let address = u32::from_str_radix(problem.place.trim_start_matches("0x"), 16).ok();
        clean &= address.is_some_and(|a| refused.contains(&a));
        println!("body\t{problem}");
    }
    println!("# rows that are no function start");
    for (address, why) in &census.not_starts {
        clean = false;
        println!("notstart\t{address:#x}\t{why}");
    }
    println!("# rows whose size is not their body's");
    let mut resized = 0;
    for (address, size, place) in &rows {
        if let Some(start) = census.starts.get(address) {
            if start.size != Some(*size) {
                clean = false;
                resized += 1;
                println!(
                    "size\t{address:#x}\t{size}\t{}\t{place}",
                    start.size.map_or("-".to_string(), |s| s.to_string())
                );
            }
        }
    }
    println!("# turned down: address, how found, reason");
    let mut by_reason: BTreeMap<String, usize> = BTreeMap::new();
    for r in &census.rejected {
        let reason = r
            .reason
            .split(" 0x")
            .next()
            .unwrap_or(&r.reason)
            .to_string();
        *by_reason
            .entry(format!("{} ({})", reason, r.how.word()))
            .or_default() += 1;
        println!("rejected\t{:#x}\t{}\t{}", r.address, r.how.word(), r.reason);
    }
    for (site, value) in &census.unaligned {
        println!("unaligned\t{value:#x}\tunreached; its only data match at {site:#x} is unaligned");
    }
    for read in &census.unbounded {
        println!("unbounded\t{read:#x}\tan index table read with no bound before it, sized by its jump table");
    }
    // Controls: each row against the census's own verdicts.
    let (mut agree, mut differ) = (BTreeMap::new(), BTreeMap::new());
    for (address, class) in &classes {
        let Some(start) = census.starts.get(address) else {
            continue;
        };
        if class.is_empty() {
            continue;
        }
        let key = format!("{class} -> {}", start.class);
        if *class == start.class {
            *agree.entry(key).or_insert(0usize) += 1;
        } else {
            *differ.entry(key).or_insert(0usize) += 1;
        }
    }
    println!("# controls");
    println!(
        "control\t{} rows; every row is a start the scans keep: {}",
        rows.len(),
        rows.iter().all(|r| census.starts.contains_key(&r.0))
    );
    for (key, n) in agree.iter().chain(differ.iter()) {
        println!("control\tclass of rows, table -> body: {key}: {n}");
    }
    if let Some(table) = write {
        let skip: HashSet<u32> = failing
            .iter()
            .filter(|_| std::env::var_os("CENSUS_TRIAL").is_none())
            .filter_map(|p| u32::from_str_radix(p.place.trim_start_matches("0x"), 16).ok())
            .collect();
        let (added, resized, removed) = write_rows(table, &census, &skip)?;
        println!(
            "wrote {added} rows and {resized} sizes to {}; removed {removed} unread rows that are \
             no start",
            table.display()
        );
    }
    println!(
        "census: {} starts after {} rounds; {} new ({}); {} refused by the body rule; {} rows \
         resized; {} body problems; {} turned down ({}); {} index tables unbounded",
        census.starts.len(),
        census.rounds,
        by_how.values().sum::<usize>(),
        by_how
            .iter()
            .map(|(k, v)| format!("{v} {k}"))
            .collect::<Vec<_>>()
            .join(", "),
        refused.len(),
        resized,
        failing.len(),
        census.rejected.len(),
        by_reason
            .iter()
            .map(|(k, v)| format!("{v} {k}"))
            .collect::<Vec<_>>()
            .join(", "),
        census.unbounded.len()
    );
    Ok(clean)
}

/// The placeholder meaning of a row added unread.
pub const UNREAD: &str = "Runs code not read here";

/// Adds a row for every start with none, at its census size and class, and sets every row's size
/// to its body's; the starts in `skip`, whose bodies the check refuses, are left out. Removes a
/// row added unread that is no start. Returns the rows added, the sizes changed and the rows
/// removed.
pub fn write_rows(
    table: &std::path::Path,
    census: &Census,
    skip: &HashSet<u32>,
) -> std::io::Result<(usize, usize, usize)> {
    let text = std::fs::read_to_string(table)?;
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default().to_string();
    let columns: Vec<&str> = header.split('\t').collect();
    let column = |name: &str| columns.iter().position(|c| *c == name).expect("a column");
    let (address_at, size_at) = (column("address"), column("size"));
    let mut rows: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for line in lines.filter(|l| !l.is_empty()) {
        let cells: Vec<String> = line.split('\t').map(str::to_string).collect();
        let address = u32::from_str_radix(cells[address_at].trim_start_matches("0x"), 16)
            .map_err(std::io::Error::other)?;
        rows.insert(address, cells);
    }
    let (mut added, mut resized) = (0, 0);
    for start in census.starts.values() {
        let Some(size) = start.size else { continue };
        if skip.contains(&start.address) {
            continue;
        }
        match rows.get_mut(&start.address) {
            Some(cells) => {
                if cells[size_at] != size.to_string() {
                    cells[size_at] = size.to_string();
                    resized += 1;
                }
            }
            None => {
                let mut cells = vec![String::new(); columns.len()];
                cells[address_at] = format!("{:#08x}", start.address);
                cells[size_at] = size.to_string();
                cells[column("system")] = "unassigned".to_string();
                cells[column("class")] = start.class.to_string();
                cells[column("meaning")] = UNREAD.to_string();
                cells[column("confidence")] = "inferred".to_string();
                cells[column("proof")] = "none".to_string();
                rows.insert(start.address, cells);
                added += 1;
            }
        }
    }
    // A row added unread that is no start, or whose body the check refuses, goes; any other is
    // only listed.
    let mut removed = 0;
    let refused = skip.iter().map(|&a| (a, String::new()));
    for (address, _) in census.not_starts.iter().cloned().chain(refused) {
        if let Some(cells) = rows.get(&address) {
            if cells[column("meaning")] == UNREAD && cells[column("name")].is_empty() {
                rows.remove(&address);
                removed += 1;
            }
        }
    }
    let mut out = header;
    out.push('\n');
    for cells in rows.values() {
        out.push_str(&cells.join("\t"));
        out.push('\n');
    }
    std::fs::write(table, out)?;
    Ok((added, resized, removed))
}

/// A node of the reach graph: a function by its start, a data segment by its first address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Node {
    Function(u32),
    Segment(u32),
}

/// How a live node was reached: the site of the reference and the node that holds it, none for
/// a root.
pub type Parents = HashMap<Node, (u32, Option<Node>)>;

/// Which starts live code reaches, and each start's count of hits.
///
/// Every dword at every alignment of every section is a reference to the start or the data run
/// it names, and in `.text` so is every `e8` or `e9` rel32. A data run is a stretch of aligned
/// dwords outside `.text` that each point into the image (a vtable and its locator, a handler or
/// name table). A run is cut into segments at each address something references, the head joined
/// to the first, so a vtable's slots live only when something live references the vtable. A
/// site's owner is the function whose extent holds it, or the segment that holds it; a site no
/// function or segment holds is a root, and so is a run nothing references.
/// What the reach scans found: the live starts, each start's hits, the unaligned data matches of
/// the starts nothing live reaches, and how each live node was reached.
struct Reach {
    live: HashSet<u32>,
    hits: HashMap<u32, usize>,
    unaligned: Vec<(u32, u32)>,
    parents: Parents,
}

fn reach(
    listing: &Listing,
    data: &[(u32, u32)],
    bytes: &[u8],
    starts: &HashSet<u32>,
    extents: &[(u32, u32)],
    entry: u32,
    text_start: u32,
) -> Reach {
    let in_image = |v: u32| (0x401000..0xd06000).contains(&v);
    let sections = program_sections(bytes);
    let imports = import_tables(bytes);
    let dword = |s: &[u8], i: usize| u32::from_le_bytes(s[i..i + 4].try_into().expect("four"));
    // The runs.
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for &(start, section) in &sections {
        if start == text_start {
            continue;
        }
        let mut i = 0;
        let pointer =
            |i: usize| in_image(dword(section, i)) && !in_ranges(&imports, start + i as u32);
        while i + 4 <= section.len() {
            if pointer(i) {
                let from = i;
                while i + 4 <= section.len() && pointer(i) {
                    i += 4;
                }
                runs.push((start + from as u32, start + i as u32));
            } else {
                i += 4;
            }
        }
    }
    // Up to two sentinel dwords, 0 or -1, right before a run open it: `__xc_a`, the null slot,
    // opens the initializer table, and a scope table's first record opens with the enclosing
    // level -1 (`0x7ffe60`, which `_WinMainCRTStartup` pushes at `0x4098c6`).
    let sentinel_at = |v: u32| {
        sections.iter().any(|&(start, section)| {
            v >= start
                && ((v - start) as usize + 4) <= section.len()
                && matches!(dword(section, (v - start) as usize), 0 | 0xffff_ffff)
        })
    };
    let run_of = |v: u32| -> Option<usize> {
        let i = runs.partition_point(|r| r.0 <= v.wrapping_add(8));
        let r = i.checked_sub(1)?;
        let head = runs[r].0;
        let opens = head <= v
            || (v + 4 == head && sentinel_at(v))
            || (v + 8 == head && sentinel_at(v) && sentinel_at(v + 4));
        (v < runs[r].1 && opens).then_some(r)
    };
    // Whether the four bytes at `site` lie inside one instruction, or outside every one.
    let within_one = |site: u32| match listing
        .index(site)
        .map(|_| site)
        .or_else(|| listing.preceding(site))
    {
        Some(start) => listing
            .following(start)
            .is_none_or(|next| next >= site + 4 || in_ranges(data, site)),
        None => true,
    };
    // Every reference: its site and the value it names. A start is named by any dword at any
    // alignment and by any `e8`/`e9`; a run by an operand of an instruction or an aligned dword
    // outside `.text`.
    let mut references: Vec<(u32, u32)> = Vec::new();
    let mut unaligned: Vec<(u32, u32)> = Vec::new();
    for &(start, section) in &sections {
        let is_text = start == text_start;
        for i in 0..section.len().saturating_sub(3) {
            let site = start + i as u32;
            let value = dword(section, i);
            if in_ranges(&imports, site) {
                continue;
            }
            if !is_text && !site.is_multiple_of(2) {
                // Data aligns its pointers at least to two, as the C runtime's packed math
                // table at `0x8763da` does: a match at an odd offset is the bytes of two values.
                if starts.contains(&value) {
                    unaligned.push((site, value));
                }
            } else if is_text {
                // In code, a dword is an operand only inside one instruction; the listing
                // leaves data out, so a dword in a table is inside none and counts too.
                if starts.contains(&value) && within_one(site) {
                    references.push((site, value));
                }
            } else if starts.contains(&value) || run_of(value).is_some() {
                references.push((site, value));
            }
            // An `e8` or `e9` byte calls or jumps only where an instruction starts.
            if is_text
                && matches!(section[i], 0xe8 | 0xe9)
                && i + 5 <= section.len()
                && listing.contains(site)
            {
                let target = site.wrapping_add(5).wrapping_add(dword(section, i + 1));
                if starts.contains(&target) {
                    references.push((site, target));
                }
            }
        }
    }
    for i in 0..listing.len() {
        let site = listing.addresses[i];
        if in_ranges(data, site) {
            continue;
        }
        let text = listing.at(i);
        for found in OPERAND.captures_iter(text) {
            if let Ok(value) = u32::from_str_radix(&found[1], 16) {
                if !starts.contains(&value) && run_of(value).is_some() {
                    references.push((site, value));
                }
            }
        }
        // A short `jmp` or a `jcc` names its target in a byte, which the dword scan misses.
        if let Some(found) = DIRECT.captures(text) {
            if let Ok(target) = u32::from_str_radix(&found[2], 16) {
                if starts.contains(&target) {
                    references.push((site, target));
                }
            }
        }
    }
    // The segments: each run cut at the addresses referenced inside it.
    let mut cuts: BTreeSet<u32> = BTreeSet::new();
    for &(_, value) in &references {
        if !starts.contains(&value) {
            if let Some(r) = run_of(value) {
                if value > runs[r].0 {
                    cuts.insert(value);
                }
            }
        }
    }
    let mut referenced_runs = vec![false; runs.len()];
    for &(_, value) in &references {
        if !starts.contains(&value) {
            if let Some(r) = run_of(value) {
                referenced_runs[r] = true;
            }
        }
    }
    let segment_of = |v: u32| -> Option<u32> {
        let r = run_of(v)?;
        let head = runs[r].0;
        Some(
            cuts.range(head..=v.max(head))
                .next_back()
                .copied()
                .unwrap_or(head),
        )
    };
    let owner = |site: u32| -> Option<Node> {
        let i = extents.partition_point(|e| e.0 <= site);
        if let Some(e) = extents[..i].iter().rev().take(64).find(|e| site < e.1) {
            return Some(Node::Function(e.0));
        }
        if site.is_multiple_of(4) {
            if let Some(r) = run_of(site) {
                let _ = r;
                return segment_of(site).map(Node::Segment);
            }
        }
        None
    };
    let mut edges: HashMap<Node, Vec<(Node, u32)>> = HashMap::new();
    let mut hits: HashMap<u32, usize> = HashMap::new();
    let mut work: Vec<(Node, u32, Option<Node>)> = vec![(Node::Function(entry), entry, None)];
    for (r, &referenced) in referenced_runs.iter().enumerate() {
        if !referenced {
            work.push((Node::Segment(runs[r].0), runs[r].0, None));
        }
    }
    for &(site, value) in &references {
        let target = if starts.contains(&value) {
            *hits.entry(value).or_default() += 1;
            Node::Function(value)
        } else {
            match segment_of(value) {
                Some(s) => Node::Segment(s),
                None => continue,
            }
        };
        match owner(site) {
            Some(from) if from != target => edges.entry(from).or_default().push((target, site)),
            Some(_) => {}
            None => work.push((target, site, None)),
        }
    }
    // Breadth first, so each parent chain is a shortest one.
    let mut parents: Parents = HashMap::new();
    let mut queue: std::collections::VecDeque<(Node, u32, Option<Node>)> = work.into();
    while let Some((node, site, from)) = queue.pop_front() {
        if parents.contains_key(&node) {
            continue;
        }
        parents.insert(node, (site, from));
        if let Some(to) = edges.get(&node) {
            for &(target, site) in to {
                if !parents.contains_key(&target) {
                    queue.push_back((target, site, Some(node)));
                }
            }
        }
    }
    let seen: HashSet<Node> = parents.keys().copied().collect();
    let live = seen
        .into_iter()
        .filter_map(|n| match n {
            Node::Function(a) => Some(a),
            Node::Segment(_) => None,
        })
        .collect::<HashSet<u32>>();
    unaligned.retain(|(_, value)| !live.contains(value));
    Reach {
        live,
        hits,
        unaligned,
        parents,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PE with `.text` at 0x401000 holding `code` and `.rdata` at 0x402000 holding `stored`,
    /// entered at 0x401000.
    fn image(stored: &[u32]) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x600];
        let put = |bytes: &mut Vec<u8>, at: usize, value: u32| {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        };
        put(&mut bytes, 0x3c, 0x40);
        bytes[0x46] = 2; // two sections
        bytes[0x54] = 0xe0; // the optional header's size
        let optional = 0x40 + 24;
        put(&mut bytes, optional + 16, 0x1000); // the entry point
        put(&mut bytes, optional + 28, 0x400000);
        let sections = optional + 0xe0;
        for (i, (name, address, offset)) in [(b".text", 0x1000, 0x200), (b".rdat", 0x2000, 0x400)]
            .into_iter()
            .enumerate()
        {
            let h = sections + 40 * i;
            bytes[h..h + 5].copy_from_slice(name);
            put(&mut bytes, h + 8, 0x50); // the virtual size
            put(&mut bytes, h + 12, address);
            put(&mut bytes, h + 16, 0x200);
            put(&mut bytes, h + 20, offset);
        }
        // `.rdata` by its full name; the stored dwords apart, so that none is a table.
        bytes[sections + 40 + 5] = b'a';
        for (k, &value) in stored.iter().enumerate() {
            put(&mut bytes, 0x400 + 8 * k, value);
        }
        // The code bytes the reach scan reads: the call at 0x401001 and the jump at 0x401006.
        bytes[0x201] = 0xe8;
        put(&mut bytes, 0x202, 0x401020 - 0x401006);
        bytes[0x206] = 0xe9;
        put(&mut bytes, 0x207, 0x401040 - 0x40100b);
        bytes
    }

    /// The entry calls one function and tail-jumps to another after padding; a third is stored
    /// in `.rdata`; a fourth follows padding with nothing reaching it; and the entry's exit block
    /// past its `ret`, which a conditional jump reaches, is no start.
    const LISTING: &str = "
  401000:\tpush   ebp
  401001:\tcall   0x401020
  401006:\tjmp    0x401040
  40100b:\tint3
  40100c:\tint3
  40100d:\tint3
  40100e:\tint3
  40100f:\tint3
  401010:\tmov    eax,0x1
  401015:\tret
  401016:\tint3
  401017:\tint3
  401018:\tint3
  401019:\tint3
  40101a:\tint3
  40101b:\tint3
  40101c:\tint3
  40101d:\tint3
  40101e:\tint3
  40101f:\tint3
  401020:\ttest   eax,eax
  401022:\tje     0x401025
  401024:\tret
  401025:\txor    eax,eax
  401027:\tret
  401028:\tint3
  401029:\tint3
  40102a:\tint3
  40102b:\tint3
  40102c:\tint3
  40102d:\tint3
  40102e:\tint3
  40102f:\tint3
  401030:\tmov    eax,ecx
  401032:\tret
  401033:\tint3
  401034:\tint3
  401035:\tint3
  401036:\tint3
  401037:\tint3
  401038:\tint3
  401039:\tint3
  40103a:\tint3
  40103b:\tint3
  40103c:\tint3
  40103d:\tint3
  40103e:\tint3
  40103f:\tint3
  401040:\tadd    eax,ecx
  401042:\tret
  401043:\tint3
";

    #[test]
    fn each_scan_finds_its_starts_and_none_else() {
        let listing = Listing::parse(LISTING.as_bytes()).expect("parses");
        let bytes = image(&[0x401030]);
        let census = run(&listing, &bytes, &[0x401000]).expect("settles");
        let found: Vec<(u32, How, Option<u32>, bool, &str)> = census
            .starts
            .values()
            .map(|s| (s.address, s.how, s.size, s.live, s.class))
            .collect();
        assert_eq!(
            found,
            [
                (0x401000, How::Row, Some(11), true, "orchestration"),
                (0x401010, How::AfterPadding, Some(6), false, "unreached"),
                (0x401020, How::Call, Some(8), true, "orchestration"),
                (0x401030, How::Stored, Some(3), true, "orchestration"),
                (0x401040, How::TailCall, Some(3), true, "primitive"),
            ]
        );
        assert!(census.rejected.is_empty(), "nothing to turn down");
        assert!(census.not_starts.is_empty());
    }

    #[test]
    fn a_stored_pointer_into_padding_or_a_body_is_no_start() {
        let listing = Listing::parse(LISTING.as_bytes()).expect("parses");
        // 0x40100c is padding; 0x401025 lies in 0x401020's body.
        let bytes = image(&[0x401030, 0x40100c, 0x401025]);
        let census = run(&listing, &bytes, &[0x401000]).expect("settles");
        assert!(!census.starts.contains_key(&0x40100c));
        assert!(!census.starts.contains_key(&0x401025));
        let rejected: Vec<(u32, &str)> = census
            .rejected
            .iter()
            .map(|r| (r.address, r.reason.as_str()))
            .collect();
        assert_eq!(
            rejected,
            [
                (0x40100c, "padding"),
                (0x401025, "inside the extent of 0x401020")
            ]
        );
    }

    #[test]
    fn a_row_inside_a_body_is_no_start() {
        let listing = Listing::parse(LISTING.as_bytes()).expect("parses");
        let bytes = image(&[0x401030]);
        let census = run(&listing, &bytes, &[0x401000, 0x401025]).expect("settles");
        let reasons: Vec<(u32, &str)> = census
            .not_starts
            .iter()
            .map(|(a, why)| (*a, why.as_str()))
            .collect();
        assert_eq!(reasons, [(0x401025, "inside the body of 0x401020")]);
    }

    /// The starts no row can hold: the body rule (`symbols/README.md`) cannot size them, since
    /// another entry or an instruction no path reaches lies inside their extent, or code above
    /// falls through into them. All are the C runtime's: hand-written routines that share tails and
    /// branches, and the `__finally` blocks its functions both call and enter from their unwind
    /// code.
    const REFUSED: [u32; 33] = [
        0x40b115, 0x40b9ff, 0x40bb4d, 0x40bc0d, 0x40bc88, 0x40bccf, 0x40bd95, 0x40beee, 0x40d510,
        0x40da4e, 0x40dad7, 0x73f959, 0x73f962, 0x7405b4, 0x7405bd, 0x741774, 0x74177d, 0x741834,
        0x74183d, 0x741b02, 0x74378a, 0x743a3d, 0x743ae9, 0x744890, 0x744944, 0x744949, 0x7449d1,
        0x744aa0, 0x744bd5, 0x746a60, 0x746a79, 0x746ab2, 0x7470ad,
    ];

    /// The real exe, listing and table: the census is at its fixed point, so every start but the
    /// refused ones has a row at its body's size and every row is a start; `WinMain` is live, and
    /// `0x41fa80`, whose only match is unaligned in a float table, is not.
    #[test]
    fn the_table_holds_every_start_of_the_real_exe() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the repo");
        let exe = crate::exe::locate(root);
        let path = root.join("dumps").join("disasm.txt");
        let present = crate::exe::identify(&exe).expect("the exe reads")
            == crate::exe::Identity::Reference
            && path.exists();
        if !present {
            assert!(
                !std::env::var(crate::exe::REQUIRE_VARIABLE)
                    .is_ok_and(|v| !v.is_empty() && v != "0"),
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
        let rows = crate::exe::functions(&checked);
        let listing = Listing::read(&path).expect("the listing reads");
        let bytes = std::fs::read(&exe).expect("the exe reads");
        let addresses: Vec<u32> = rows.iter().map(|r| r.address).collect();
        let census = run(&listing, &bytes, &addresses).expect("settles");
        let unrowed: Vec<u32> = census
            .starts
            .values()
            .filter(|s| s.how != How::Row)
            .map(|s| s.address)
            .collect();
        assert_eq!(unrowed, REFUSED);
        assert_eq!(census.not_starts, Vec::<(u32, String)>::new());
        let resized: Vec<u32> = rows
            .iter()
            .filter(|r| census.starts[&r.address].size != Some(r.size))
            .map(|r| r.address)
            .collect();
        assert_eq!(resized, Vec::<u32>::new());
        assert!(census.starts[&0x404100].live);
        assert!(!census.starts[&0x41fa80].live);
    }

    #[test]
    fn computing_is_told_from_moving() {
        for text in [
            "add eax,ecx",
            "sub ecx,DWORD PTR [edx+0x4]",
            "or edx,0x1",
            "fmul DWORD PTR [ebp-0x4]",
            "fsubrp st(1),st",
            "fistp DWORD PTR [esp]",
            "mulss xmm0,xmm1",
            "cvttss2si eax,xmm0",
            "xor eax,ecx",
            "inc esi",
        ] {
            assert!(computes_on_values(text), "{text}");
        }
        for text in [
            "mov eax,ecx",
            "xor eax,eax",
            "sub esp,0x10",
            "add esp,0x8",
            "and esp,0xfffffff0",
            "or eax,0xffffffff",
            "lea eax,[ecx+0x4]",
            "cmp eax,0x36",
            "test al,0x1",
            "fld DWORD PTR [eax]",
            "fstp DWORD PTR [eax]",
            "fild DWORD PTR [eax]",
            "fcomp DWORD PTR [eax]",
            "movss xmm0,DWORD PTR [eax]",
            "xorps xmm0,xmm0",
            "push 0x0",
            "call 0x646600",
            "comiss xmm0,xmm1",
        ] {
            assert!(!computes_on_values(text), "{text}");
        }
    }

    #[test]
    fn an_import_slot_makes_host() {
        let iat = (0x7ff000, 0x7ff400);
        assert_eq!(
            class_of(&["push 0x1", "call DWORD PTR ds:0x7ff1a0"], iat),
            "host"
        );
        assert_eq!(
            class_of(&["mov esi,DWORD PTR ds:0x7ff004", "add eax,0x1"], iat),
            "host"
        );
        assert_eq!(class_of(&["add eax,0x1", "ret"], iat), "primitive");
        assert_eq!(
            class_of(&["push 0x82e94c", "call 0x64ae50"], iat),
            "orchestration"
        );
    }
}
