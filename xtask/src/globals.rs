//! `cargo xtask cited <benilla> --write`: a `globals.tsv` row for each data address benilla cites
//! that no row holds, and for each jump or index table in `.text` it cites.
//!
//! The type is read from the instructions that name the address: a floating-point load of a
//! dword or a qword (`float`, `double`), an integer load (`fild`, `int32_t`), a byte or word
//! access, a call through it (`void *`), a dword otherwise; an address only taken is a C string
//! when its bytes are one (`char[n]`), a vtable when the run-time type information before it names
//! its class (`Class::vftable`, `void *[n]`), else a dword. A `.rdata` scalar is `const`. A type
//! that would reach another row's address is cut to the largest scalar that fits.
//!
//! The name is a string's words (`s_textInterfaceIcons…`), a float's or a double's bits
//! (`s_float3f000000`), a table's function (`s_cases494e20`) or a vtable's class; a datum read
//! from nothing more keeps no name and a placeholder meaning naming the functions that read it.
//!
//! The system is the object-file rule for data: the system of every function that names it, when
//! those agree and so do the nearest data on each side that one system's functions alone name or
//! that is a source path of one system's file; a table's is its function's.

use crate::bodies::{self, Image, Listing, TABLE};
use crate::check::Checked;
use crate::objfiles::{c_string, system_of_path};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io;
use std::path::Path;
use std::sync::LazyLock;

static OPERAND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"0x([0-9a-f]{6})\b").expect("a valid pattern"));
static BANNED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\?|\bTODO\b|\b(?:probably|maybe|seems|likely)\b").expect("a valid pattern")
});
static OBJECT_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^[ -~]*[\\/][ -~]*\.(cpp|c)$").expect("a valid pattern"));
static RTTI_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\.\?A[UV]([A-Za-z0-9_]+(?:@[A-Za-z0-9_]+)*)@@$").expect("a valid pattern")
});
static GLOBAL_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[A-Z][A-Za-z0-9]*(?:_[A-Z][A-Za-z0-9]*)*::)+vftable$").expect("valid")
});

const MEANING: usize = 120;
const NAME: usize = 48;
/// `.rdata`, whose values are constants.
const RDATA: (u32, u32) = (0x7ff000, 0x827000);

/// How one instruction uses an address it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Use {
    Float,
    Double,
    Int32,
    Int16,
    Int64,
    Byte,
    Word,
    Dword,
    Qword,
    Call,
    JumpTable,
    IndexTable,
    Indexed(u8),
    Address,
}

/// What one instruction does with `address`, which it names.
fn use_of(text: &str, address: u32) -> Option<Use> {
    let hex = format!("0x{address:x}");
    let direct = text.contains(&format!("ds:{hex}"));
    let indexed = text.contains(&format!("+{hex}]"));
    let mnemonic = text.split(' ').next().unwrap_or_default();
    let width = |t: &str| {
        if t.contains("QWORD PTR") {
            8
        } else if t.contains("DWORD PTR") {
            4
        } else if t.contains("TBYTE PTR") {
            10
        } else if t.contains("WORD PTR") {
            2
        } else if t.contains("BYTE PTR") {
            1
        } else {
            0
        }
    };
    if direct {
        let register8 = Regex::new(r"(?:^|[ ,])(?:al|bl|cl|dl|ah|bh|ch|dh)(?:,|$)").ok()?;
        let register16 = Regex::new(r"(?:^|[ ,])(?:ax|bx|cx|dx|si|di|bp)(?:,|$)").ok()?;
        return Some(if mnemonic == "call" || mnemonic == "jmp" {
            Use::Call
        } else if mnemonic.starts_with("fi") {
            // `fild`, `fiadd`, `fist`, `fistp` and the like read or write an integer.
            match width(text) {
                2 => Use::Int16,
                8 => Use::Int64,
                _ => Use::Int32,
            }
        } else if mnemonic.starts_with('f') {
            if width(text) == 8 {
                Use::Double
            } else {
                Use::Float
            }
        } else if mnemonic.ends_with("ss") && !mnemonic.starts_with("movs") || mnemonic == "movss" {
            Use::Float
        } else if mnemonic.ends_with("sd") && mnemonic != "movsd" || mnemonic == "movsd" {
            Use::Double
        } else if width(text) == 1 || register8.is_match(text.split_once(' ')?.1) {
            Use::Byte
        } else if width(text) == 2 || register16.is_match(text.split_once(' ')?.1) {
            Use::Word
        } else if width(text) == 8 {
            Use::Qword
        } else {
            Use::Dword
        });
    }
    if indexed {
        if mnemonic == "jmp" && TABLE.is_match(text) {
            return Some(Use::JumpTable);
        }
        if mnemonic == "movzx" && text.contains("BYTE PTR") {
            return Some(Use::IndexTable);
        }
        return Some(Use::Indexed(
            match (mnemonic.starts_with('f'), width(text)) {
                (true, 8) => 9,
                (true, _) => 5,
                (false, w) => w,
            },
        ));
    }
    Some(Use::Address)
}

/// The x87 instructions that store to their memory operand; every other one reads it.
const X87_STORES: [&str; 14] = [
    "fst", "fstp", "fist", "fistp", "fisttp", "fbstp", "fstcw", "fnstcw", "fstsw", "fnstsw",
    "fstenv", "fnstenv", "fsave", "fnsave",
];
/// The instructions whose first operand they only read.
const READS_FIRST: [&str; 12] = [
    "cmp", "test", "push", "bt", "call", "jmp", "comiss", "comisd", "ucomiss", "ucomisd", "ptest",
    "prefetch",
];

/// Whether `text` writes the memory at `address` it names, read from the operand's position at
/// any width: a store, a read-modify-write (`add`, `inc`, `or`, `setne`), `xchg`, and the x87
/// stores (`fst`, `fstp`, `fistp`).
fn writes(text: &str, address: u32) -> bool {
    let hex = format!("0x{address:x}");
    let (mnemonic, operands) = text.split_once(' ').unwrap_or((text, ""));
    let names = |operand: &str| {
        operand.contains(&format!("ds:{hex}")) || operand.contains(&format!("+{hex}]"))
    };
    let mut parts = operands.split(',');
    let first = parts.next().unwrap_or_default();
    if !names(first) {
        return mnemonic == "xchg" && parts.any(names);
    }
    if mnemonic.starts_with('f') {
        return X87_STORES.contains(&mnemonic);
    }
    !READS_FIRST.iter().any(|m| mnemonic.starts_with(m))
}

/// A proposed row.
pub struct Proposal {
    pub address: u32,
    pub ty: String,
    pub size: u32,
    pub name: String,
    pub system: String,
    pub meaning: String,
}

/// The function rows' extents with their names and systems, for owners.
struct Functions {
    rows: Vec<(u32, u32, String, String)>,
}

impl Functions {
    fn of(&self, address: u32) -> Option<&(u32, u32, String, String)> {
        let i = self.rows.partition_point(|r| r.0 <= address);
        self.rows[..i]
            .iter()
            .rev()
            .take(64)
            .find(|r| address < r.0 + r.1)
    }
}

/// The class a vtable at `address` belongs to, from the run-time type information its
/// complete-object locator, the dword before it, points at.
fn vtable_class(image: &Image, address: u32) -> Option<String> {
    let locator = image.dword(address.checked_sub(4)?)?;
    let descriptor = image.dword(locator.checked_add(12)?)?;
    let name = c_string(image, descriptor.checked_add(8)?)?;
    let found = RTTI_NAME.captures(&name)?;
    let mut parts: Vec<&str> = found[1].split('@').collect();
    parts.reverse();
    let class = format!("{}::vftable", parts.join("::"));
    GLOBAL_CLASS.is_match(&class).then_some(class)
}

/// A string's words as a name: `s_text` and each word capitalised, cut to the name's limit.
fn text_name(text: &str) -> Option<String> {
    let mut name = "s_text".to_string();
    for word in text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        let mut chars = word.chars();
        let first = chars.next()?;
        let next = format!("{}{}", first.to_ascii_uppercase(), chars.as_str());
        if name.len() + next.len() > NAME {
            break;
        }
        name.push_str(&next);
    }
    (name.len() > "s_text".len()).then_some(name)
}

/// `Holds the string "…"`, cut to the cell; a string the meaning cannot quote is described.
fn text_meaning(text: &str) -> String {
    let quoted: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let full = format!("Holds the string \"{quoted}\"");
    if BANNED.is_match(&quoted) {
        return format!("Holds a string of {} characters", text.chars().count());
    }
    if full.chars().count() <= MEANING {
        return full;
    }
    let cut: String = quoted.chars().take(MEANING - 30).collect();
    format!("Holds a string that opens \"{cut}…\"")
}

/// `Holds a … that A and B read; not read here`, the readers named that fit.
fn placeholder(what: &str, readers: &[String], writes: bool) -> String {
    let mut n = readers.len().min(3);
    loop {
        let verb = match (n, writes) {
            (1, false) => "reads",
            (1, true) => "reads and writes",
            (_, false) => "read",
            (_, true) => "read and write",
        };
        let text = if n == 0 {
            let verb = if writes { "reads and writes" } else { "reads" };
            format!("Holds {what} its object file's code {verb}; not read here")
        } else {
            format!(
                "Holds {what} that {} {verb}; not read here",
                readers[..n].join(", ")
            )
        };
        if text.chars().count() <= MEANING || n == 0 {
            return text;
        }
        n -= 1;
    }
}

fn scalar(size: u32) -> &'static str {
    match size {
        8.. => "uint64_t",
        4..=7 => "uint32_t",
        2..=3 => "uint16_t",
        _ => "uint8_t",
    }
}

/// The last vtable-like address (two function starts or more) a function stores as an
/// immediate at an object's start, by instruction address: the vtable a constructor finally
/// stores.
fn last_vtable_stored(
    listing: &Listing,
    image: &Image,
    functions: &Functions,
    function: u32,
) -> Option<u32> {
    let (start, size, _, _) = functions.of(function)?;
    let end = start + size;
    let mut at = Some(*start);
    let mut last = None;
    while let Some(a) = at.filter(|&a| a < end) {
        let text = listing.get(a).unwrap_or_default();
        // A store to the object's own start, `[reg]`; one at a displacement is a member's.
        let own = text
            .strip_prefix("mov DWORD PTR [")
            .and_then(|t| t.split_once("],0x"))
            .filter(|(register, _)| register.len() == 3 && register.starts_with('e'));
        if let Some(value) = own.and_then(|(_, v)| u32::from_str_radix(v, 16).ok()) {
            let slot = |k: u32| {
                image
                    .dword(value + 4 * k)
                    .is_some_and(|v| functions.of(v).is_some_and(|f| f.0 == v))
            };
            if value >= RDATA.0 && slot(0) && slot(1) {
                last = Some(value);
            }
        }
        at = listing.following(a);
    }
    last
}

/// Proposes a row for each of `addresses`.
pub fn propose(
    checked: &Checked,
    listing: &Listing,
    image: &Image,
    addresses: &[u32],
    index: &[(u32, u32)],
) -> Vec<Proposal> {
    let functions = Functions {
        rows: checked
            .functions
            .iter()
            .filter_map(|r| {
                Some((
                    u32::from_str_radix(r.get("address").strip_prefix("0x")?, 16).ok()?,
                    r.get("size").parse().ok()?,
                    r.get("name").to_string(),
                    r.get("system").to_string(),
                ))
            })
            .collect(),
    };
    let targets: HashSet<u32> = addresses.iter().copied().collect();
    // Every instruction's data operands: who names what, and how.
    let mut uses: HashMap<u32, Vec<(u32, Use)>> = HashMap::new();
    let mut named_by: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut anchors: BTreeMap<u32, String> = BTreeMap::new();
    let mut written: HashSet<u32> = HashSet::new();
    for i in 0..listing.len() {
        let site = listing.addresses[i];
        let text = listing.at(i);
        for found in OPERAND.captures_iter(text) {
            let Ok(value) = u32::from_str_radix(&found[1], 16) else {
                continue;
            };
            if value < RDATA.0 && !targets.contains(&value) {
                continue;
            }
            let owner = functions.of(site);
            if value >= RDATA.0 {
                if let Some(o) = owner {
                    named_by.entry(value).or_default().insert(o.3.clone());
                }
            }
            if targets.contains(&value) {
                if let (Some(u), Some(o)) = (use_of(text, value), owner) {
                    uses.entry(value).or_default().push((o.0, u));
                    if writes(text, value) {
                        written.insert(value);
                    }
                }
            }
            if let Some(path) = c_string(image, value).filter(|t| OBJECT_PATH.is_match(t)) {
                if let Some(system) = system_of_path(&path) {
                    anchors.insert(value, system.to_string());
                }
            }
        }
    }
    for (&value, systems) in &named_by {
        if systems.len() == 1 && !anchors.contains_key(&value) {
            let system = systems.iter().next().expect("one");
            if system != "unassigned" {
                anchors.insert(value, system.clone());
            }
        }
    }
    let name_of = |function: u32| -> Option<String> {
        functions
            .of(function)
            .map(|f| f.2.clone())
            .filter(|n| !n.is_empty())
    };

    let mut out = Vec::new();
    for &address in addresses {
        let found = uses.get(&address).cloned().unwrap_or_default();
        let kinds: BTreeSet<Use> = found.iter().map(|(_, u)| *u).collect();
        let readers: Vec<u32> = {
            let mut r: Vec<u32> = found.iter().map(|(f, _)| *f).collect();
            r.sort_unstable();
            r.dedup();
            r
        };
        let in_code = address < RDATA.0;
        let constant = (RDATA.0..RDATA.1).contains(&address);
        let systems: BTreeSet<String> = readers
            .iter()
            .filter_map(|&f| functions.of(f).map(|o| o.3.clone()))
            .collect();
        // The system: the readers' one system, which the nearest anchors on each side share.
        let system = if in_code {
            readers
                .first()
                .and_then(|&f| functions.of(f))
                .map(|o| o.3.clone())
        } else if systems.len() == 1 {
            let s = systems.iter().next().expect("one").clone();
            let before = anchors.range(..address).next_back().map(|(_, v)| v);
            let after = anchors.range(address + 1..).next().map(|(_, v)| v);
            (before == Some(&s) && after == Some(&s) && s != "unassigned").then_some(s)
        } else {
            None
        }
        .unwrap_or_else(|| "unassigned".to_string());
        let reader_names: Vec<String> = readers.iter().filter_map(|&f| name_of(f)).collect();
        let writes = written.contains(&address);
        let cst = if constant { "const " } else { "" };

        let proposal = |ty: String, size: u32, name: String, meaning: String| Proposal {
            address,
            ty,
            size,
            name,
            system: system.clone(),
            meaning,
        };
        // Tables in code.
        if kinds.contains(&Use::JumpTable) {
            let owner = readers.first().copied().unwrap_or(0);
            let n = image.table_entries(owner, address).len() as u32;
            if n > 0 {
                out.push(proposal(
                    format!("void *[{n}]"),
                    4 * n,
                    format!("s_cases{owner:06x}"),
                    format!("Holds the {n} case addresses of the switch in {owner:#08x}"),
                ));
                continue;
            }
        }
        if kinds.contains(&Use::IndexTable) && in_code {
            let owner = readers.first().copied().unwrap_or(0);
            if let Some(&(_, end)) = index.iter().find(|t| t.0 == address) {
                let n = end - address;
                out.push(proposal(
                    format!("uint8_t[{n}]"),
                    n,
                    format!("s_caseIndexes{owner:06x}"),
                    format!("Holds the {n} case indexes of the switch in {owner:#08x}"),
                ));
                continue;
            }
        }
        if in_code {
            continue;
        }
        // Constants and variables by their loads.
        let float = kinds.contains(&Use::Float);
        let double = kinds.contains(&Use::Double);
        if float || double {
            let (ty, size) = if double { ("double", 8) } else { ("float", 4) };
            let bits = if double {
                image
                    .read(address, 8)
                    .map(|b| format!("{:016x}", u64::from_le_bytes(b.try_into().expect("8"))))
            } else {
                image.dword(address).map(|v| format!("{v:08x}"))
            };
            match bits.filter(|_| constant) {
                Some(bits) => {
                    let value = if double {
                        f64::from_bits(u64::from_str_radix(&bits, 16).unwrap_or(0)).to_string()
                    } else {
                        f32::from_bits(u32::from_str_radix(&bits, 16).unwrap_or(0)).to_string()
                    };
                    out.push(proposal(
                        format!("{cst}{ty}"),
                        size,
                        format!("s_{ty}{bits}"),
                        format!("Holds the {ty} {value} (0x{bits})"),
                    ));
                }
                None => out.push(proposal(
                    format!("{cst}{ty}"),
                    size,
                    String::new(),
                    placeholder(&format!("a {ty}"), &reader_names, writes),
                )),
            }
            continue;
        }
        let integer = [
            (Use::Int32, "int32_t", 4),
            (Use::Int16, "int16_t", 2),
            (Use::Int64, "int64_t", 8),
        ]
        .into_iter()
        .find(|(u, _, _)| kinds.contains(u));
        if let Some((_, ty, size)) = integer {
            out.push(proposal(
                format!("{cst}{ty}"),
                size,
                String::new(),
                placeholder(&format!("an {ty}"), &reader_names, writes),
            ));
            continue;
        }
        if kinds.contains(&Use::Call) {
            out.push(proposal(
                format!("{cst}void *"),
                4,
                String::new(),
                placeholder("a function address", &reader_names, writes),
            ));
            continue;
        }
        // An address only taken, or named by no instruction: a string, a vtable, or data.
        let only_taken = kinds.iter().all(|u| *u == Use::Address);
        if only_taken {
            if let Some(text) = c_string(image, address).filter(|t| !t.is_empty()) {
                let n = text.len() as u32 + 1;
                out.push(proposal(
                    format!("{cst}char[{n}]"),
                    n,
                    text_name(&text).unwrap_or_default(),
                    text_meaning(&text),
                ));
                continue;
            }
            // A vtable: a run of function starts that code stores, up to the next address an
            // instruction names. Its class is the one whose constructor stores it last.
            let next_named = named_by
                .range(address + 1..)
                .next()
                .map_or(u32::MAX, |(&a, _)| a);
            let slots = (0..)
                .take_while(|&k| {
                    address + 4 * k < next_named
                        && image
                            .dword(address + 4 * k)
                            .is_some_and(|v| functions.of(v).is_some_and(|f| f.0 == v))
                })
                .count() as u32;
            let class = vtable_class(image, address).or_else(|| {
                readers.iter().find_map(|&f| {
                    let n = name_of(f)?;
                    let (scope, own) = n.rsplit_once("::")?;
                    let class = scope.rsplit("::").next()?;
                    let name = format!("{scope}::vftable");
                    let last = last_vtable_stored(listing, image, &functions, f);
                    (own == class && GLOBAL_CLASS.is_match(&name) && last == Some(address))
                        .then_some(name)
                })
            });
            if slots >= 2 && class.is_none() {
                out.push(proposal(
                    format!("void *[{slots}]"),
                    4 * slots,
                    String::new(),
                    placeholder(&format!("{slots} function addresses"), &reader_names, false)
                        .replace(" reads; not", " stores; not")
                        .replace(" read; not", " store; not"),
                ));
                continue;
            }
            if let Some(class) = class.filter(|_| slots >= 2) {
                let owner = class.trim_end_matches("::vftable").to_string();
                out.push(proposal(
                    format!("void *[{slots}]"),
                    4 * slots,
                    class,
                    format!(
                        "Holds the {slots} virtual function slots of {owner}, which its \
                         constructor stores"
                    ),
                ));
                continue;
            }
        }
        let (ty, size, what) = if kinds.contains(&Use::Byte) || kinds.contains(&Use::Indexed(1)) {
            ("uint8_t", 1, "a byte")
        } else if kinds.contains(&Use::Word) || kinds.contains(&Use::Indexed(2)) {
            ("uint16_t", 2, "a word")
        } else if kinds.contains(&Use::Qword) {
            ("uint64_t", 8, "a qword")
        } else if kinds.contains(&Use::Indexed(5)) {
            ("float", 4, "the first float of a table")
        } else if kinds.contains(&Use::Indexed(9)) {
            ("double", 8, "the first double of a table")
        } else if found.is_empty() {
            ("uint32_t", 4, "a dword no instruction names, that")
        } else if only_taken {
            ("uint32_t", 4, "data whose address")
        } else {
            ("uint32_t", 4, "a dword")
        };
        let ty = format!("{cst}{ty}");
        let meaning = if found.is_empty() {
            "Holds a dword no instruction names directly; not read here".to_string()
        } else if only_taken {
            placeholder("data whose address", &reader_names, false)
                .replace("address that ", "address ")
                .replace(" reads; not", " takes; not")
                .replace(" read; not", " take; not")
        } else {
            placeholder(what, &reader_names, writes)
        };
        out.push(proposal(ty, size, String::new(), meaning));
    }
    out
}

/// Adds the proposals to `symbols/globals.tsv`, each cut so that it reaches no other row, and
/// each named uniquely; returns the rows added.
pub fn write(
    root: &Path,
    checked: &Checked,
    listing: &Listing,
    bytes: &[u8],
    addresses: &[u32],
) -> io::Result<usize> {
    let image = Image::new(bytes).ok_or_else(|| io::Error::other("no section table"))?;
    // A cited code address is a table's, or no row's (padding, a range's end): each table once,
    // from its start.
    let starts: Vec<u32> = checked
        .functions
        .iter()
        .filter_map(|r| u32::from_str_radix(r.get("address").strip_prefix("0x")?, 16).ok())
        .collect();
    let no_return = bodies::no_return(&starts, listing, &image);
    let jumps = bodies::jump_tables(&starts, listing, &image, &no_return);
    let mut tables: Vec<(u32, u32)> = bodies::table_ranges(&jumps);
    let index = bodies::all_index_tables(&starts, listing, &image, &no_return);
    tables.extend(index.iter().copied());
    tables.sort_unstable();
    let mut wanted: BTreeSet<u32> = BTreeSet::new();
    for &a in addresses {
        if a >= RDATA.0 {
            wanted.insert(a);
        } else if let Some(&(start, _)) = tables.iter().find(|t| t.0 <= a && a < t.1) {
            wanted.insert(start);
        }
    }
    let wanted: Vec<u32> = wanted.into_iter().collect();
    let mut proposals = propose(checked, listing, &image, &wanted, &index);
    proposals.sort_by_key(|p| p.address);
    let path = root.join("symbols").join("globals.tsv");
    let text = std::fs::read_to_string(&path)?;
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default().to_string();
    let mut rows: BTreeMap<u32, String> = BTreeMap::new();
    for line in lines.filter(|l| !l.is_empty()) {
        let address = line.split('\t').next().unwrap_or_default();
        if let Ok(a) = u32::from_str_radix(address.trim_start_matches("0x"), 16) {
            rows.insert(a, line.to_string());
        }
    }
    let mut taken: HashSet<String> = checked
        .functions
        .iter()
        .chain(&checked.globals)
        .map(|r| r.get("name").to_string())
        .filter(|n| !n.is_empty())
        .collect();
    // Every row's address, so a new one reaches none.
    let mut starts: BTreeSet<u32> = checked
        .functions
        .iter()
        .chain(&checked.globals)
        .filter_map(|r| u32::from_str_radix(r.get("address").strip_prefix("0x")?, 16).ok())
        .collect();
    starts.extend(proposals.iter().map(|p| p.address));
    let mut added = 0;
    let mut covered_until = 0u32;
    for mut p in proposals {
        if p.address < covered_until || checked.extents.contains(u64::from(p.address)) {
            continue;
        }
        let next = starts.range(p.address + 1..).next().copied();
        if let Some(next) = next.filter(|&n| n < p.address + p.size) {
            let room = next - p.address;
            p.size = room.clamp(1, 8);
            let base = scalar(p.size);
            let cst = if p.ty.starts_with("const ") {
                "const "
            } else {
                ""
            };
            if p.ty.contains("char[") {
                p.ty = format!("{cst}char[{room}]");
                p.size = room;
            } else {
                p.ty = format!("{cst}{base}");
                p.size = match base {
                    "uint64_t" => 8,
                    "uint32_t" => 4,
                    "uint16_t" => 2,
                    _ => 1,
                };
                if !p.name.is_empty() && !p.name.contains("::") {
                    p.name.clear();
                    p.meaning = format!(
                        "Holds data {} bytes long before the next row; not read here",
                        p.size
                    );
                }
            }
        }
        if !p.name.is_empty() && !taken.insert(p.name.clone()) {
            let unique = format!("{}At{:06x}", p.name, p.address);
            if unique.len() <= NAME && taken.insert(unique.clone()) {
                p.name = unique;
            } else {
                p.name.clear();
            }
        }
        covered_until = p.address + p.size;
        rows.insert(
            p.address,
            [
                format!("{:#08x}", p.address),
                p.ty,
                p.name,
                String::new(),
                p.system,
                p.meaning,
                "inferred".to_string(),
                "none".to_string(),
                String::new(),
            ]
            .join("\t"),
        );
        added += 1;
    }
    let mut out = header;
    out.push('\n');
    for line in rows.values() {
        out.push_str(line);
        out.push('\n');
    }
    std::fs::write(&path, out)?;
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-section image at 0x401000 whose bytes from 0x402000 are `data`.
    fn image_with(data: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x200 + 0x2000];
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xe0u16.to_le_bytes());
        bytes[0x80 + 52..0x80 + 56].copy_from_slice(&0x400000u32.to_le_bytes());
        let header = 0x80 + 24 + 0xe0;
        bytes[header + 8..header + 12].copy_from_slice(&0x2000u32.to_le_bytes());
        bytes[header + 12..header + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes[header + 16..header + 20].copy_from_slice(&0x2000u32.to_le_bytes());
        bytes[header + 20..header + 24].copy_from_slice(&0x200u32.to_le_bytes());
        for (address, text) in data {
            let at = 0x200 + (address - 0x401000) as usize;
            bytes[at..at + text.len()].copy_from_slice(text);
        }
        bytes
    }

    #[test]
    fn a_vtable_class_is_read_from_its_locator_without_overflow() {
        let word = |v: u32| v.to_le_bytes().to_vec();
        // A locator at 0x402100 whose type descriptor at 0x402200 names `CFoo`.
        let good = image_with(&[
            (0x40200c, word(0x402100)),
            (0x40210c, word(0x402200)),
            (0x402208, b".?AVCFoo@@\0".to_vec()),
        ]);
        let image = Image::new(&good).expect("an image");
        assert_eq!(
            vtable_class(&image, 0x402010).as_deref(),
            Some("CFoo::vftable")
        );
        // The dword before the vtable is 0xfffffff8: the locator plus 12 wraps, and no class is read.
        let wrapped = image_with(&[(0x40200c, word(0xffff_fff8))]);
        let image = Image::new(&wrapped).expect("an image");
        assert_eq!(vtable_class(&image, 0x402010), None);
        // A descriptor at 0xfffffffc: its name at +8 wraps too.
        let wrapped = image_with(&[(0x40200c, word(0x402100)), (0x40210c, word(0xffff_fffc))]);
        let image = Image::new(&wrapped).expect("an image");
        assert_eq!(vtable_class(&image, 0x402010), None);
    }

    #[test]
    fn each_use_is_read_from_its_instruction() {
        let a = 0x801360;
        let cases = [
            ("fld DWORD PTR ds:0x801360", Use::Float),
            ("fmul QWORD PTR ds:0x801360", Use::Double),
            ("fild DWORD PTR ds:0x801360", Use::Int32),
            ("movss xmm0,DWORD PTR ds:0x801360", Use::Float),
            ("mov eax,ds:0x801360", Use::Dword),
            ("mov ds:0x801360,eax", Use::Dword),
            ("mov al,ds:0x801360", Use::Byte),
            ("movzx eax,BYTE PTR ds:0x801360", Use::Byte),
            ("mov ax,ds:0x801360", Use::Word),
            ("call DWORD PTR ds:0x801360", Use::Call),
            ("push 0x801360", Use::Address),
            ("mov eax,DWORD PTR [ecx*4+0x801360]", Use::Indexed(4)),
            ("fld DWORD PTR [eax*4+0x801360]", Use::Indexed(5)),
        ];
        for (text, expected) in cases {
            assert_eq!(use_of(text, a), Some(expected), "{text}");
        }
    }

    #[test]
    fn a_write_is_read_from_the_operand_position() {
        let a = 0x835a4c;
        for text in [
            "mov ds:0x835a4c,eax",
            "mov BYTE PTR ds:0x835a4c,0x1",
            "mov WORD PTR ds:0x835a4c,ax",
            "fstp DWORD PTR ds:0x835a4c",
            "fst QWORD PTR ds:0x835a4c",
            "fistp DWORD PTR ds:0x835a4c",
            "add DWORD PTR ds:0x835a4c,0x1",
            "inc DWORD PTR ds:0x835a4c",
            "or BYTE PTR ds:0x835a4c,0x4",
            "setne BYTE PTR ds:0x835a4c",
            "xchg ecx,DWORD PTR ds:0x835a4c",
            "movss DWORD PTR ds:0x835a4c,xmm0",
            "mov DWORD PTR [eax*4+0x835a4c],ecx",
        ] {
            assert!(writes(text, a), "{text}");
        }
        for text in [
            "mov eax,ds:0x835a4c",
            "fld DWORD PTR ds:0x835a4c",
            "fcomp DWORD PTR ds:0x835a4c",
            "cmp DWORD PTR ds:0x835a4c,0x0",
            "test BYTE PTR ds:0x835a4c,0x1",
            "push DWORD PTR ds:0x835a4c",
            "push 0x835a4c",
            "movss xmm0,DWORD PTR ds:0x835a4c",
            "call DWORD PTR ds:0x835a4c",
        ] {
            assert!(!writes(text, a), "{text}");
        }
    }

    #[test]
    fn strings_name_and_mean_their_text() {
        assert_eq!(
            text_name(r"Interface\Icons\INV_Misc_QuestionMark").as_deref(),
            Some("s_textInterfaceIconsINVMiscQuestionMark")
        );
        assert_eq!(text_name("%s"), Some("s_textS".to_string()));
        assert_eq!(text_name("%%"), None);
        assert_eq!(text_meaning("Cancel"), "Holds the string \"Cancel\"");
        assert_eq!(
            text_meaning("Are you sure?"),
            "Holds a string of 13 characters"
        );
    }
}
