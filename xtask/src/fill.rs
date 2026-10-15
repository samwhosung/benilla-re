//! `cargo xtask census --fill [--idb <names.tsv>]`: completes the rows the census added unread
//! (meaning `Runs code not read here`, no name, system `unassigned`): the system by the object-file
//! rule (`objfiles`), a name where a string or the community database gives one, a meaning, and
//! the class the census derives from the body. Rows anyone has written are never touched.
//!
//! Names, first that applies and passes the naming rule, unique among functions and globals:
//!
//! - an import thunk, `jmp DWORD PTR ds:<slot>`, takes the import's name from the import table;
//! - a Lua binding (`tables/bindings.tsv`) takes `Script::<name>` for a global function and
//!   `<Class>::<name>` for a widget method, the class read from the community names of the other
//!   methods in its table; a community name that names the same binding is kept as it is;
//! - otherwise the community name (`vmangos/idb`, given as `address<TAB>kind<TAB>name`), except
//!   FLIRT's duplicate matches (`_<n>` suffixes, mangled `?` names), the disassembler's own
//!   `sub_<address>` labels, the entry label `start`, and the names the bytes contradict
//!   (`CONTRADICTED`), which take the name the bytes give where they give one.
//!
//! A row with no name keeps a placeholder meaning: `Runs <file> code that calls <callees>; not read
//! here`, the file being the one a source path names, else `its object file`.

use crate::bodies::{imports, Image, Listing};
use crate::census::{Census, UNREAD};
use crate::check::{valid_function_name, LIBRARIES};
use crate::objfiles;
use regex::Regex;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;
use std::path::Path;
use std::sync::LazyLock;

static IMPORT_JUMP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^jmp\s+DWORD PTR ds:0x([0-9a-f]+)$").expect("a valid pattern"));
static NOISE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"_[0-9]+$|^\?|^start$|^__cfltcvt_init$|(?:^|::)sub_[0-9A-Fa-f]+$")
        .expect("a valid pattern")
});
static METHOD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z][A-Za-z0-9_]*)::([A-Za-z0-9_]+)$").expect("valid"));

const MEANING: usize = 120;

/// The name and meaning the bytes give a function.
type Given = Option<(&'static str, &'static str)>;

/// Community names the bytes contradict, checked at the bytes and never used: the address, why,
/// and the name and meaning the bytes give where they give one.
pub const CONTRADICTED: &[(u32, &str, Given)] = &[
    (
        0x429a50,
        "___setargv is the C runtime's argument parser; this sets the flag 0x884c04 that 0x429a60 \
         reads, called from the client's start at 0x402156",
        None,
    ),
    (
        0x6395c0,
        "ConsoleCommand_BufferSize: the table at 0x80e148 registers it as the console command \
         consolelines, which sets the console's height in lines (0x864560, 0xc4ec10), no buffer",
        Some((
            "ConsoleCommandConsoleLines",
            "Implements the console command consolelines: the console's height in lines",
        )),
    ),
    (
        0x6f3bc0,
        "GetErrorFunction: it resolves a stack index and the pseudo-indices, calls luaH_getnum \
         0x6fa700 and copies the value to the top, which is lua_rawgeti",
        Some((
            "lua_rawgeti",
            "Pushes t[n] of the table at a stack index, without metamethods",
        )),
    ),
];

/// The community names: `address<TAB>func<TAB>name` lines; others are skipped, and so are the
/// names in `CONTRADICTED`.
pub fn read_idb(path: &Path) -> io::Result<HashMap<u32, String>> {
    let mut out = HashMap::new();
    for line in std::fs::read_to_string(path)?.lines() {
        let cells: Vec<&str> = line.split('\t').collect();
        let [address, kind, name, ..] = cells.as_slice() else {
            continue;
        };
        if *kind != "func" {
            continue;
        }
        if let Ok(a) = u32::from_str_radix(address.trim_start_matches("0x"), 16) {
            if !CONTRADICTED.iter().any(|c| c.0 == a) {
                out.insert(a, name.to_string());
            }
        }
    }
    Ok(out)
}

/// A Lua binding: its name, its table and the table's kind.
struct Binding {
    name: String,
    table: String,
    kind: String,
}

fn read_bindings(root: &Path) -> io::Result<HashMap<u32, Vec<Binding>>> {
    let text = std::fs::read_to_string(root.join("tables").join("bindings.tsv"))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap_or_default().split('\t').collect();
    let at = |c: &str| header.iter().position(|h| *h == c);
    let (Some(name), Some(function), Some(table), Some(kind)) =
        (at("name"), at("fn"), at("table"), at("table_kind"))
    else {
        return Err(io::Error::other("tables/bindings.tsv lacks its columns"));
    };
    let mut out: HashMap<u32, Vec<Binding>> = HashMap::new();
    for line in lines {
        let cells: Vec<&str> = line.split('\t').collect();
        let Some(Ok(address)) = cells
            .get(function)
            .map(|f| u32::from_str_radix(f.trim_start_matches("0x"), 16))
        else {
            continue;
        };
        out.entry(address).or_default().push(Binding {
            name: cells[name].to_string(),
            table: cells[table].to_string(),
            kind: cells[kind].to_string(),
        });
    }
    Ok(out)
}

/// What the fill did, for the report.
#[derive(Default)]
pub struct Summary {
    pub filled: usize,
    pub systems: BTreeMap<String, usize>,
    pub sources: BTreeMap<&'static str, usize>,
    pub names: BTreeMap<&'static str, usize>,
    pub refused: Vec<String>,
    pub conflicts: Vec<String>,
    pub runs: usize,
}

/// The columns of `symbols/functions.tsv` the fill reads and writes.
struct Columns {
    address: usize,
    size: usize,
    name: usize,
    from: usize,
    system: usize,
    class: usize,
    meaning: usize,
}

impl Columns {
    fn of(header: &str) -> Columns {
        let columns: Vec<&str> = header.split('\t').collect();
        let at = |name: &str| columns.iter().position(|c| *c == name).expect("a column");
        Columns {
            address: at("address"),
            size: at("size"),
            name: at("name"),
            from: at("name_from"),
            system: at("system"),
            class: at("class"),
            meaning: at("meaning"),
        }
    }
}

/// The rows of `symbols/functions.tsv` under `root`, by address.
fn read_rows(root: &Path, c: &Columns) -> io::Result<(String, BTreeMap<u32, Vec<String>>)> {
    let text = std::fs::read_to_string(root.join("symbols").join("functions.tsv"))?;
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default().to_string();
    let mut rows: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for line in lines.filter(|l| !l.is_empty()) {
        let cells: Vec<String> = line.split('\t').map(str::to_string).collect();
        let address = u32::from_str_radix(cells[c.address].trim_start_matches("0x"), 16)
            .map_err(io::Error::other)?;
        rows.insert(address, cells);
    }
    Ok((header, rows))
}

/// The systems with a doc under `systems/`.
fn documented_systems(root: &Path) -> io::Result<std::collections::BTreeSet<String>> {
    Ok(std::fs::read_dir(root.join("systems"))?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let key = name.strip_suffix(".md").unwrap_or(&name).to_string();
            (key != "README").then_some(key)
        })
        .collect())
}

/// The system of every function the object-file rule settles, with the rows in `fill` (the rows
/// being completed) left out of the evidence.
fn owners_of(
    root: &Path,
    rows: &BTreeMap<u32, Vec<String>>,
    c: &Columns,
    fill: &HashSet<u32>,
    listing: &Listing,
    bytes: &[u8],
    idb: &HashMap<u32, String>,
) -> io::Result<objfiles::Owners> {
    let image = Image::new(bytes).ok_or_else(|| io::Error::other("no section table"))?;
    let extents: Vec<(u32, u32)> = rows
        .iter()
        .filter_map(|(&a, r)| Some((a, a + r[c.size].parse::<u32>().ok()?)))
        .collect();
    let known: HashMap<u32, String> = rows
        .iter()
        .filter(|(a, r)| !fill.contains(a) && r[c.system] != "unassigned")
        .map(|(&a, r)| (a, r[c.system].clone()))
        .collect();
    let documented = documented_systems(root)?;
    let imports = imports(bytes);
    let thunks: std::collections::BTreeSet<u32> = extents
        .iter()
        .map(|e| e.0)
        .filter(|&a| {
            IMPORT_JUMP
                .captures(listing.get(a).unwrap_or_default())
                .and_then(|f| u32::from_str_radix(&f[1], 16).ok())
                .is_some_and(|slot| imports.contains_key(&slot))
        })
        .collect();
    Ok(objfiles::assign(
        listing,
        &image,
        &extents,
        &known,
        idb,
        &documented,
        &thunks,
    ))
}

/// What `--assign` did, for the report.
#[derive(Default)]
pub struct Assigned {
    pub unassigned: usize,
    pub settled: BTreeMap<&'static str, usize>,
    pub systems: BTreeMap<String, usize>,
    pub conflicts: Vec<String>,
}

/// Settles the system of every row that has none, by the object-file rule, leaving every other
/// cell alone except the class, which a system with a doc requires and the census gives.
pub fn assign_unassigned(
    root: &Path,
    census: &Census,
    listing: &Listing,
    bytes: &[u8],
    idb: &HashMap<u32, String>,
) -> io::Result<Assigned> {
    let table = root.join("symbols").join("functions.tsv");
    let text = std::fs::read_to_string(&table)?;
    let header = text.lines().next().unwrap_or_default().to_string();
    let c = Columns::of(&header);
    let (_, mut rows) = read_rows(root, &c)?;
    let targets: HashSet<u32> = rows
        .iter()
        .filter(|(_, r)| r[c.system] == "unassigned")
        .map(|(&a, _)| a)
        .collect();
    let owners = owners_of(root, &rows, &c, &targets, listing, bytes, idb)?;
    let documented = documented_systems(root)?;
    let mut out = Assigned {
        unassigned: targets.len(),
        conflicts: owners.conflicts.clone(),
        ..Assigned::default()
    };
    for &a in &targets {
        let Some(owner) = owners.owners.get(&a).filter(|o| o.system != "unassigned") else {
            continue;
        };
        let row = rows.get_mut(&a).expect("a target row");
        row[c.system] = owner.system.clone();
        if documented.contains(&owner.system) && row[c.class].is_empty() {
            let class = census.starts.get(&a).map_or("", |s| s.class);
            row[c.class] = class.to_string();
        }
        *out.settled.entry(owner.source.word()).or_default() += 1;
        *out.systems.entry(owner.system.clone()).or_default() += 1;
    }
    let mut text = header;
    text.push('\n');
    for cells in rows.values() {
        text.push_str(&cells.join("\t"));
        text.push('\n');
    }
    std::fs::write(&table, text)?;
    Ok(out)
}

/// Fills the unread rows of `symbols/functions.tsv` under `root`.
pub fn fill(
    root: &Path,
    census: &Census,
    listing: &Listing,
    bytes: &[u8],
    idb: &HashMap<u32, String>,
) -> io::Result<Summary> {
    let table = root.join("symbols").join("functions.tsv");
    let header = std::fs::read_to_string(&table)?
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    let c = Columns::of(&header);
    let (_, mut rows) = read_rows(root, &c)?;
    let (name_at, from_at, system_at, class_at, meaning_at) =
        (c.name, c.from, c.system, c.class, c.meaning);
    let targets: Vec<u32> = rows
        .iter()
        .filter(|(_, c)| {
            c[meaning_at] == UNREAD && c[name_at].is_empty() && c[system_at] == "unassigned"
        })
        .map(|(&a, _)| a)
        .collect();
    let target_set: HashSet<u32> = targets.iter().copied().collect();
    let documented = documented_systems(root)?;
    let imports = imports(bytes);
    let owners = owners_of(root, &rows, &c, &target_set, listing, bytes, idb)?;

    // Names.
    let mut taken: HashSet<String> = rows
        .values()
        .map(|c| c[name_at].clone())
        .filter(|n| !n.is_empty())
        .collect();
    let globals = std::fs::read_to_string(root.join("symbols").join("globals.tsv"))?;
    for line in globals.lines().skip(1) {
        if let Some(name) = line.split('\t').nth(2).filter(|n| !n.is_empty()) {
            taken.insert(name.to_string());
        }
    }
    let bindings = read_bindings(root)?;
    // Each widget table's class, from the community names of its methods.
    let mut classes: HashMap<&str, BTreeMap<String, usize>> = HashMap::new();
    for (address, list) in &bindings {
        let name = idb.get(address).or_else(|| {
            rows.get(address)
                .map(|c| &c[name_at])
                .filter(|n| !n.is_empty())
        });
        for b in list.iter().filter(|b| b.kind == "widget") {
            if let Some(found) = name.and_then(|n| METHOD.captures(n)) {
                if found[2] == b.name {
                    *classes
                        .entry(b.table.as_str())
                        .or_default()
                        .entry(found[1].to_string())
                        .or_default() += 1;
                }
            }
        }
    }
    let class_of = |table: &str| -> Option<String> {
        let counts = classes.get(table)?;
        let (class, _) = counts.iter().max_by_key(|(_, n)| **n)?;
        Some(class.clone())
    };

    let mut summary = Summary {
        conflicts: owners.conflicts.clone(),
        runs: owners.runs,
        ..Summary::default()
    };
    let mut named: HashMap<u32, (String, bool, String)> = HashMap::new();
    for &a in &targets {
        let system = owners
            .owners
            .get(&a)
            .map_or("unassigned".to_string(), |o| o.system.clone());
        let library = LIBRARIES.contains(&system.as_str());
        let idb_name = idb.get(&a).filter(|n| !NOISE.is_match(n));
        // (name, from the community database, meaning, source)
        let mut candidates: Vec<(String, bool, String, &'static str)> = Vec::new();
        if let Some((_, _, Some((name, meaning)))) = CONTRADICTED.iter().find(|c| c.0 == a) {
            candidates.push((name.to_string(), false, meaning.to_string(), "read"));
        }
        let first = listing.get(a).unwrap_or_default();
        if let Some(found) = IMPORT_JUMP.captures(first) {
            let slot = u32::from_str_radix(&found[1], 16).unwrap_or(0);
            if let Some((name, dll)) = imports.get(&slot) {
                candidates.push((
                    name.clone(),
                    false,
                    format!("Jumps to {name}, imported from {dll}"),
                    "import",
                ));
            }
        }
        for b in bindings.get(&a).into_iter().flatten() {
            let agrees =
                idb_name.filter(|n| n.ends_with(&format!("::{}", b.name)) || **n == b.name);
            let (name, meaning) = match b.kind.as_str() {
                "global" => (
                    format!("Script::{}", b.name),
                    format!("Implements the Lua function {}", b.name),
                ),
                "widget" => match class_of(&b.table) {
                    Some(class) => (
                        format!("{class}::{}", b.name),
                        format!("Implements the {class} method {}", b.name),
                    ),
                    None => continue,
                },
                _ => match agrees {
                    Some(n) => (
                        n.clone(),
                        format!("Implements the Lua library function {}", b.name),
                    ),
                    None => continue,
                },
            };
            match agrees {
                Some(n) => candidates.push((n.clone(), true, meaning, "binding")),
                None => candidates.push((name, false, meaning, "binding")),
            }
        }
        if let Some(n) = idb_name {
            candidates.push((n.clone(), true, String::new(), "community"));
        }
        for (name, from_idb, meaning, source) in candidates {
            if taken.contains(&name) {
                summary.refused.push(format!("{a:#x}: {name} is taken"));
                continue;
            }
            if !valid_function_name(&name, from_idb || library) {
                summary
                    .refused
                    .push(format!("{a:#x}: {name} breaks the naming rule"));
                continue;
            }
            taken.insert(name.clone());
            *summary.names.entry(source).or_default() += 1;
            named.insert(a, (name, from_idb, meaning));
            break;
        }
    }

    // Meanings, classes and systems.
    let name_of = |a: u32| -> Option<String> {
        named.get(&a).map(|n| n.0.clone()).or_else(|| {
            rows.get(&a)
                .map(|c| c[name_at].clone())
                .filter(|n| !n.is_empty())
        })
    };
    let mut updates: Vec<(u32, Vec<(usize, String)>)> = Vec::new();
    for &a in &targets {
        let owner = owners.owners.get(&a);
        let system = owner.map_or("unassigned".to_string(), |o| o.system.clone());
        let source = owner.map_or("none", |o| o.source.word());
        *summary.systems.entry(system.clone()).or_default() += 1;
        *summary.sources.entry(source).or_default() += 1;
        let start = census.starts.get(&a);
        let mut class = start.map_or("", |s| s.class).to_string();
        // Library code is a boundary, except D3DX's math, whose exact output matters.
        if LIBRARIES.contains(&system.as_str())
            && system != "d3dx"
            && class != "unreached"
            && !class.is_empty()
        {
            class = "boundary".to_string();
        }
        // Outside a system with a doc, where nothing requires a class, the classifier's is left
        // out; the census's `unreached` and a library's `boundary` stay.
        if !documented.contains(&system) && class != "unreached" && class != "boundary" {
            class.clear();
        }
        let mut cells = vec![(system_at, system), (class_at, class)];
        let explicit = named.get(&a).map(|n| n.2.clone()).filter(|m| !m.is_empty());
        let meaning = match explicit {
            Some(m) => m,
            None => {
                let file = owners
                    .files
                    .get(&a)
                    .cloned()
                    .unwrap_or_else(|| "its object file".to_string());
                // A callee with no row (a start the body rule refuses) is counted, not cited.
                let callees: Vec<u32> = start.map(|s| s.callees.clone()).unwrap_or_default();
                let cited: Vec<String> = callees
                    .iter()
                    .filter(|c| rows.contains_key(c))
                    .map(|&c| name_of(c).unwrap_or_else(|| format!("{c:#08x}")))
                    .collect();
                let mut rowless: Vec<u32> = callees
                    .iter()
                    .copied()
                    .filter(|c| !rows.contains_key(c))
                    .collect();
                rowless.sort_unstable();
                rowless.dedup();
                placeholder(&file, &cited, rowless.len())
            }
        };
        cells.push((meaning_at, meaning));
        if let Some((name, from_idb, _)) = named.get(&a) {
            cells.push((name_at, name.clone()));
            cells.push((from_at, if *from_idb { "idb" } else { "" }.to_string()));
        }
        updates.push((a, cells));
    }
    for (a, cells) in updates {
        if let Some(row) = rows.get_mut(&a) {
            for (k, v) in cells {
                row[k] = v;
            }
        }
    }
    summary.filled = targets.len();
    let mut out = header;
    out.push('\n');
    for cells in rows.values() {
        out.push_str(&cells.join("\t"));
        out.push('\n');
    }
    std::fs::write(&table, out)?;
    Ok(summary)
}

/// `Runs <file> code that calls A, B, C and 2 more; not read here`: the callees in the order
/// the body calls them, each once, named or by address, as many as fit and the count of the rest,
/// which counts the `uncited` callees no row holds.
pub fn placeholder(file: &str, callees: &[String], uncited: usize) -> String {
    let mut unique: Vec<&String> = Vec::new();
    for c in callees {
        if !unique.contains(&c) {
            unique.push(c);
        }
    }
    let mut n = unique.len().min(3);
    loop {
        let rest = unique.len() - n + uncited;
        let text = if unique.is_empty() && uncited == 0 {
            format!("Runs {file} code not read here")
        } else if n == 0 && rest == 1 {
            format!("Runs {file} code that calls one function; not read here")
        } else if n == 0 {
            format!("Runs {file} code that calls {rest} functions; not read here")
        } else {
            let list: Vec<&str> = unique[..n].iter().map(|s| s.as_str()).collect();
            let more = if rest > 0 {
                format!(" and {rest} more")
            } else {
                String::new()
            };
            format!(
                "Runs {file} code that calls {}{more}; not read here",
                list.join(", ")
            )
        };
        if text.chars().count() <= MEANING || n == 0 {
            return text;
        }
        n -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placeholder_fits_its_cell() {
        assert_eq!(
            placeholder("its object file", &[], 0),
            "Runs its object file code not read here"
        );
        assert_eq!(
            placeholder("its object file", &[], 1),
            "Runs its object file code that calls one function; not read here"
        );
        let callees: Vec<String> = [
            "SMemAlloc",
            "SMemFree",
            "SMemAlloc",
            "SStrCopy",
            "SErrDisplayError",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            placeholder("Player_C.cpp", &callees, 0),
            "Runs Player_C.cpp code that calls SMemAlloc, SMemFree, SStrCopy and 1 more; not read \
             here"
        );
        assert_eq!(
            placeholder("Player_C.cpp", &callees[..2], 2),
            "Runs Player_C.cpp code that calls SMemAlloc, SMemFree and 2 more; not read here"
        );
        let long: Vec<String> = (0..3)
            .map(|i| format!("AVeryLongCalleeNameNumber{i}ForTesting"))
            .collect();
        assert!(placeholder("UnitCombatLog_C.cpp", &long, 0).chars().count() <= MEANING);
    }

    #[test]
    fn noise_is_known() {
        for n in [
            "??0Iostream_init@@QAE@XZ_22",
            "__cfltcvt_init_2",
            "start",
            "?_Get_deleter@x",
            "WowTime::sub_642A10",
            "sub_401000",
        ] {
            assert!(NOISE.is_match(n), "{n}");
        }
        assert!(!NOISE.is_match("CGUnit_C::GetAura"));
    }
}
