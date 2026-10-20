//! The object file, and so the system, of every function start (`systems/README.md`, "Object
//! files"; `symbols/README.md`, "One row per address").
//!
//! A function's system is its object file's, and a file is proved by the first of these that
//! applies; nothing else settles it:
//!
//! - a row's own system, for a row this pass does not fill;
//! - a span a written system's doc lists by address (`DECLARED`);
//! - a source path the function itself passes (`__FILE__` in an assertion or an allocation),
//!   mapped to its system by the directory and file rules in `RULES`; a header's path names no
//!   file, since headers are inlined anywhere;
//! - an initializer run: the C runtime's table `0x827000` lists each file's static initializers
//!   in link order, a file that includes the shared maths header opening its run with its +∞
//!   initializer. That initializer, its thunk and the functions that read the file's own copies of
//!   the header's constants are one file's;
//! - a stretch: code is contiguous per object file, so every function between the first and the
//!   last that a path or a run proves to be one file's is that file's too, unless a function in
//!   the stretch is proved another's. The file's system is the path's, or for a file only a run
//!   proves, the one system the evidence inside the stretch agrees on, and never a system with a
//!   written doc, which lists its own object files;
//! - the C runtime's own names in the community database (`_malloc`, `__aulldiv`), whose
//!   functions are the `crt` library's;
//! - a DBC file name (`DBFilesClient\…`), which only the DBC stores' loaders pass.
//!
//! Two files of one system with a function between them prove nothing about it: the stretch can
//! hold a file of another system. What none of these settles is `unassigned`.

use crate::bodies::{Image, Listing, IMMEDIATE};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::LazyLock;

/// The C runtime's initializer table and its end, `systems/README.md`, "Object files".
const INITIALIZERS: (u32, u32) = (0x827000, 0x82e030);

/// The object files the docs list by address where no path or row bounds them: `ui`'s runs
/// (`systems/ui/README.md`, `systems/ui/framexml.md`), the minigame's board file (`symbols/README.md`), and the object-file
/// spans of `gx` (`systems/gx/overview.md`), `lighting` (`systems/lighting/overview.md`), `cmath`
/// (`systems/cmath/overview.md`), `dbcache` (`systems/dbcache.md`) and `font`
/// (`systems/font/overview.md`). A span is that system's; a span whose end the doc leaves open is
/// one address, its first function. The spans are ordered by start.
const DECLARED: &[(u32, u32, &str)] = &[
    (0x4477d0, 0x44bfa0, "gx"),
    (0x44c040, 0x44ca10, "gx"),
    (0x455c00, 0x4567c2, "cmath"),
    (0x47a9a0, 0x47bec0, "gx"),
    (0x480380, 0x509ff0, "ui"),
    (0x515880, 0x536ea0, "ui"),
    (0x554b10, 0x5737b0, "dbcache"),
    (0x5894c0, 0x5a35d0, "gx"),
    (0x5c1560, 0x5c3c90, "font"),
    (0x5c6650, 0x5cfdef, "font"),
    (0x5d1100, 0x5d1400, "gx"),
    (0x674560, 0x67e350, "lighting"),
    (0x6cdf50, 0x6d6c20, "lighting"),
    (0x6d6df0, 0x6d8530, "lighting"),
    (0x6ed930, 0x6ed930, "ui"),
    (0x6f19e0, 0x6f2840, "ui"),
    (0x701940, 0x706430, "ui"),
    (0x728b10, 0x728ef3, "minigame"),
    (0x728f00, 0x730bbf, "ui"),
    (0x760be0, 0x762e20, "cmath"),
    (0x764160, 0x7a3a80, "ui"),
    (0x7af5c0, 0x7b0d90, "gx"),
    (0x7bc090, 0x7c4810, "cmath"),
    (0x7c8120, 0x7c8120, "dbcache"),
    (0x7cb0f0, 0x7cc310, "ui"),
    (0x7cc320, 0x7cc320, "ui"),
    (0x7e4f90, 0x7e6670, "lighting"),
];

/// Directory and file rules: the system a source path's object file belongs to. First match wins.
const RULES: &[(&str, &str)] = &[
    (r"\\Source\\Client\.cpp$", "client"),
    (r"\\LoadingScreen\.cpp$", "loadingscreen"),
    (r"\\Base\\Status\.cpp$", "client"),
    (r"\\engine\\Source\\Base\\", "base"),
    (r"\\Event\\", "core"),
    (r"\\TimeManager\.cpp$", "core"),
    (r"\\OS\\W32\\", "os"),
    (r"\\Services\\(Texture|Lightning)\.cpp$", "gx"),
    (r"\\Services\\SysMessage\.cpp$", "sysmsg"),
    (r"\\Services\\GfxSingletonManager\.cpp$", "models"),
    (r"\\SoundInterface\\", "sound"),
    (r"\\ObjectAlloc\\", "objectalloc"),
    (r"\\ObjectMgrClient\\", "objectlayer"),
    (r"\\Source\\Glue\\", "glue"),
    (
        r"\\ComponentCore\\CharacterComponent\.cpp$",
        "charactermodel",
    ),
    (r"\\Common\\TextureCache\.cpp$", "gx"),
    (r"\\Common\\AaBsp\.cpp$", "models"),
    (r"\\UIUtil\\Camera\.cpp$", "uicamera"),
    (r"\\UIUtil\\InputControl\.cpp$", "input"),
    (r"\\UIUtil\\", "ui"),
    (r"\\Source\\Ui\\", "ui"),
    (r"\\Source\\Net\\", "net"),
    (r"\\WowSvcs\\", "net"),
    (r"\\WowServices\\", "net"),
    (r"\\DBClient\\DBCache\.cpp$", "dbcache"),
    (
        r"\\Source\\Object\\(CreatureStats|GameObjectStats|ItemName|ItemStats|NPCText)\.cpp$",
        "dbcache",
    ),
    (r"\\Source\\gx\\", "gx"),
    (r"\\Images\\", "image"),
    (r"\\BLPFile\\", "image"),
    (r"\\Gxu\\IGxuFontGlyph\.cpp$", "gx"),
    (r"\\Gxu\\GxuFont", "font"),
    (r"\\ObjectClient\\Movement(_C)?\.cpp$", "collision"),
    (r"\\ObjectClient\\", "objectlayer"),
    (r"\\Console\\ConsoleClient\.cpp$", "console"),
    (r"\\Console\\ConsoleVar\.cpp$", "cvar"),
    (r"\\Storm\\Source\\", "storm"),
    (r"\\WorldClient\\MapWeather\.cpp$", "lighting"),
    (r"\\WorldClient\\MapObj(Group|Read)?\.cpp$", "models"),
    (r"\\WorldClient\\", "terrain"),
    (r"\\GameClient\\PlayerName\.cpp$", "playername"),
    (r"\\GameClient\\WardenClient\.cpp$", "warden"),
    (r"\\GameClient\\FFXEffects\.cpp$", "ffxeffects"),
    (r"\\GameClient\\Minimap\.cpp$", "minimap"),
    (r"\\MagicClient\\", "spell"),
    (r"\\FrameXML\\", "ui"),
    (r"\\FrameScript\\", "ui"),
    (r"\\lua\\src\\", "lua"),
    (r"\\Model2\\", "models"),
    (r"\\Minigame\\", "minigame"),
    (r"\\lgLCD\\", "ui"),
    (r"\\Source\\Frame\\", "ui"),
    (r"\\Tempest\\", "cmath"),
];

static COMPILED: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    RULES
        .iter()
        .map(|(p, s)| (Regex::new(&format!("(?i){p}")).expect("a valid rule"), *s))
        .collect()
});

/// A source path of an object file: ends in `.cpp` or `.c` and has a directory.
static OBJECT_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^[ -~]*[\\/][ -~]*\.(cpp|c)$").expect("a valid pattern"));
static INFINITY_LOAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^mov eax,ds:0x([0-9a-f]+)$").expect("a valid pattern"));
static GLOBAL_STORE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^mov ds:0x[0-9a-f]+,eax$").expect("a valid pattern"));
static BRANCH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:call|jmp)\s+0x([0-9a-f]+)$").expect("a valid pattern"));
static OPERAND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"0x([0-9a-f]{6})\b").expect("a valid pattern"));
/// A C runtime name in the community database: a leading underscore, no FLIRT duplicate suffix.
static CRT_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^_[A-Za-z_][A-Za-z0-9_]*(?:@[0-9]+)?$").expect("a valid pattern")
});
static DUPLICATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"_[0-9]+$").expect("a valid pattern"));
/// Database names with a leading underscore that are not the C runtime's: the game's own entry,
/// and FLIRT's match of game code to a runtime routine (a known false positive).
const NOT_CRT: [&str; 2] = ["_WinMain@16", "__cfltcvt_init"];

/// The system of a source path, by `RULES`.
pub fn system_of_path(path: &str) -> Option<&'static str> {
    COMPILED
        .iter()
        .find(|(r, _)| r.is_match(path))
        .map(|(_, s)| *s)
}

/// The kind of evidence that settled a function's system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    Row,
    Declared,
    Path,
    Stretch,
    Run,
    Library,
    DbcName,
}

impl Source {
    pub fn word(self) -> &'static str {
        match self {
            Source::Row => "row",
            Source::Declared => "a doc's span",
            Source::Path => "path",
            Source::Stretch => "stretch",
            Source::Run => "initializer run",
            Source::Library => "runtime name",
            Source::DbcName => "DBC file name",
        }
    }
}

pub struct Owner {
    pub system: String,
    pub source: Source,
}

pub struct Owners {
    /// Each settled function's system and why.
    pub owners: BTreeMap<u32, Owner>,
    /// The object file's name, where a path names it: `Player_C.cpp`.
    pub files: HashMap<u32, String>,
    /// Problems with the evidence: a path span holding another system's row, and the like.
    pub conflicts: Vec<String>,
    pub runs: usize,
}

/// A C string in the image, printable ASCII, up to 260 bytes.
pub fn c_string(image: &Image, address: u32) -> Option<String> {
    let mut out = Vec::new();
    for i in 0..260 {
        let b = image.byte(address.wrapping_add(i))?;
        if b == 0 {
            return String::from_utf8(out).ok();
        }
        if !(0x20..0x7f).contains(&b) {
            return None;
        }
        out.push(b);
    }
    None
}

/// The function whose extent holds `address`.
fn function_of(extents: &[(u32, u32)], address: u32) -> Option<u32> {
    let i = extents.partition_point(|e| e.0 <= address);
    extents[..i]
        .iter()
        .rev()
        .take(64)
        .find(|e| address < e.1)
        .map(|e| e.0)
}

/// Settles the system of every start in `extents` (start, end), from the rows' systems in `rows`
/// (the rows not being filled), the listing, the image and the community names `idb`. A system in
/// `documented` has a doc that lists its object files: an initializer run never extends it. The
/// import thunks in `thunks` are the linker's, from the import libraries, in no object file of
/// the client's: they settle nothing and no neighbour reaches across them.
pub fn assign(
    listing: &Listing,
    image: &Image,
    extents: &[(u32, u32)],
    rows: &HashMap<u32, String>,
    idb: &HashMap<u32, String>,
    documented: &BTreeSet<String>,
    thunks: &BTreeSet<u32>,
) -> Owners {
    assign_in(
        listing,
        image,
        extents,
        rows,
        idb,
        documented,
        thunks,
        INITIALIZERS,
    )
}

/// `assign` over the initializer table `table`.
#[allow(clippy::too_many_arguments)]
fn assign_in(
    listing: &Listing,
    image: &Image,
    extents: &[(u32, u32)],
    rows: &HashMap<u32, String>,
    idb: &HashMap<u32, String>,
    documented: &BTreeSet<String>,
    thunks: &BTreeSet<u32>,
    table: (u32, u32),
) -> Owners {
    let starts: Vec<u32> = extents.iter().map(|e| e.0).collect();
    let mut owners: BTreeMap<u32, Owner> = BTreeMap::new();
    let mut files: HashMap<u32, String> = HashMap::new();
    let mut conflicts = Vec::new();
    for (&a, s) in rows {
        owners.insert(
            a,
            Owner {
                system: s.clone(),
                source: Source::Row,
            },
        );
    }

    // The spans the written systems' docs list.
    for &(low, high, system) in DECLARED {
        let i = starts.partition_point(|&a| a < low);
        let j = starts.partition_point(|&a| a <= high);
        if let Some(a) = starts[i..j]
            .iter()
            .find(|a| rows.get(a).is_some_and(|s| s != system))
        {
            conflicts.push(format!(
                "{system}'s span {low:#x} to {high:#x} holds the {} row {a:#x}; it is not used",
                rows[a]
            ));
            continue;
        }
        for &a in &starts[i..j] {
            owners.entry(a).or_insert(Owner {
                system: system.to_string(),
                source: Source::Declared,
            });
        }
    }

    // What each function's instructions name: source paths, DBC names, operands.
    let mut paths: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    let mut own_paths: HashMap<u32, BTreeSet<String>> = HashMap::new();
    let mut dbc_named: BTreeSet<u32> = BTreeSet::new();
    let mut operands: HashMap<u32, Vec<u32>> = HashMap::new();
    for i in 0..listing.len() {
        let site = listing.addresses[i];
        let text = listing.at(i);
        let Some(function) = function_of(extents, site) else {
            continue;
        };
        for found in OPERAND.captures_iter(text) {
            if let Ok(v) = u32::from_str_radix(&found[1], 16) {
                operands.entry(v).or_default().push(function);
            }
        }
        let Some(found) = IMMEDIATE.captures(text) else {
            continue;
        };
        let Ok(value) = u32::from_str_radix(&found[1], 16) else {
            continue;
        };
        let Some(text) = c_string(image, value) else {
            continue;
        };
        if OBJECT_PATH.is_match(&text) {
            paths.entry(text.clone()).or_default().insert(function);
            own_paths.entry(function).or_default().insert(text);
        } else if text.starts_with("DBFilesClient\\") {
            dbc_named.insert(function);
        }
    }

    // A function that passes the paths of one system. One that passes a path no rule maps is a
    // file of a system not settled, which no neighbour reaches across.
    let mut barriers: BTreeSet<u32> = thunks.clone();
    for (&function, set) in &own_paths {
        let systems: BTreeSet<&str> = set.iter().filter_map(|p| system_of_path(p)).collect();
        if set.len() == 1 {
            let path = set.iter().next().expect("one path");
            files.insert(function, file_name(path));
        }
        if set.iter().any(|p| system_of_path(p).is_none()) {
            barriers.insert(function);
        } else if systems.len() == 1 && !owners.contains_key(&function) {
            owners.insert(
                function,
                Owner {
                    system: systems.into_iter().next().expect("one system").to_string(),
                    source: Source::Path,
                },
            );
        }
    }
    // The object file of each function a path or a run proves, then the stretch of each file.
    let mut file_of: BTreeMap<u32, String> = BTreeMap::new();
    for (&function, set) in &own_paths {
        if let (1, Some(path)) = (set.len(), set.iter().next()) {
            file_of.insert(function, file_name(path));
        }
    }
    let path_spans = file_spans(&file_of);
    let runs = initializer_runs(listing, image, &operands, table);
    let mut run_files: BTreeSet<String> = BTreeSet::new();
    for members in &runs {
        let (Some(&low), Some(&high)) = (members.first(), members.last()) else {
            continue;
        };
        let i = starts.partition_point(|&a| a < low);
        let j = starts.partition_point(|&a| a <= high);
        // The files already proved inside the run's stretch, or whose stretch holds a member.
        let mut named: BTreeSet<&String> =
            starts[i..j].iter().filter_map(|a| file_of.get(a)).collect();
        for m in members {
            named.extend(
                path_spans
                    .iter()
                    .filter(|(_, &(l, h))| l <= *m && *m <= h)
                    .map(|(f, _)| f),
            );
        }
        if named.len() > 1 {
            conflicts.push(format!(
                "the run at {low:#x} to {high:#x} crosses {} files; it is not used",
                named.len()
            ));
            continue;
        }
        let file = match named.iter().next() {
            Some(f) => (*f).clone(),
            None => {
                let f = format!("the run at {low:#x}");
                run_files.insert(f.clone());
                f
            }
        };
        for &m in members {
            file_of.entry(m).or_insert_with(|| file.clone());
        }
    }
    let runs_count = runs.len();
    let run_members: BTreeSet<u32> = runs.iter().flatten().copied().collect();
    let spans = file_spans(&file_of);
    for (file, &(low, high)) in &spans {
        let overlap = spans
            .iter()
            .find(|&(other, &(l, h))| other != file && l <= high && low <= h);
        if let Some((other, _)) = overlap {
            conflicts.push(format!(
                "{file} ({low:#x} to {high:#x}) overlaps {other}; its stretch is not used"
            ));
            continue;
        }
        let i = starts.partition_point(|&a| a < low);
        let j = starts.partition_point(|&a| a <= high);
        // A file's system: its paths', or for a file only a run proves, the one system the
        // evidence inside agrees on, and never a written system's: its doc lists its files.
        let by_path: BTreeSet<Option<&str>> = starts[i..j]
            .iter()
            .filter(|a| own_paths.contains_key(a))
            .flat_map(|a| own_paths[a].iter().map(|p| system_of_path(p)))
            .collect();
        let system = if run_files.contains(file) {
            let inside: BTreeSet<&str> = starts[i..j]
                .iter()
                .filter_map(|a| owners.get(a))
                .map(|o| o.system.as_str())
                .collect();
            let barred = barriers.range(low..=high).next().is_some();
            match inside.iter().next() {
                Some(&s) if inside.len() == 1 && !barred && !documented.contains(s) => {
                    s.to_string()
                }
                _ => continue,
            }
        } else {
            match by_path.iter().next() {
                Some(Some(s)) if by_path.len() == 1 => s.to_string(),
                _ => continue,
            }
        };
        let foreign = starts[i..j]
            .iter()
            .find(|a| rows.get(a).is_some_and(|s| *s != system));
        if let Some(a) = foreign {
            conflicts.push(format!(
                "{file} ({low:#x} to {high:#x}) holds the {} row {a:#x}; its stretch is not used",
                rows[a]
            ));
            continue;
        }
        for &a in &starts[i..j] {
            files.entry(a).or_insert_with(|| file.clone());
            let source = if own_paths.contains_key(&a) {
                Source::Path
            } else if run_members.contains(&a) {
                Source::Run
            } else {
                Source::Stretch
            };
            owners.entry(a).or_insert(Owner {
                system: system.clone(),
                source,
            });
        }
    }

    // The runtime's names, then the DBC loaders.
    for (&a, name) in idb {
        if starts.binary_search(&a).is_ok()
            && CRT_NAME.is_match(name)
            && !DUPLICATE.is_match(name)
            && !NOT_CRT.contains(&name.as_str())
        {
            owners.entry(a).or_insert(Owner {
                system: "crt".to_string(),
                source: Source::Library,
            });
        }
    }
    for &a in &dbc_named {
        owners.entry(a).or_insert(Owner {
            system: "dbc".to_string(),
            source: Source::DbcName,
        });
    }

    owners.retain(|a, o| o.source == Source::Row || !thunks.contains(a));
    Owners {
        owners,
        files,
        conflicts,
        runs: runs_count,
    }
}

/// The first and last function of each file in `file_of`.
fn file_spans(file_of: &BTreeMap<u32, String>) -> BTreeMap<String, (u32, u32)> {
    let mut spans: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for (&a, f) in file_of {
        let span = spans.entry(f.clone()).or_insert((a, a));
        span.0 = span.0.min(a);
        span.1 = span.1.max(a);
    }
    spans
}

/// The last component of a path: `Player_C.cpp`.
pub fn file_name(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
}

/// The instructions from `address` on, at most `n`.
fn instructions(listing: &Listing, address: u32, n: usize) -> Vec<(u32, &str)> {
    let mut out = Vec::new();
    let mut at = Some(address);
    while let Some(a) = at {
        if out.len() == n {
            break;
        }
        let Some(text) = listing.get(a) else { break };
        out.push((a, text));
        at = listing.following(a);
    }
    out
}

/// The +∞ initializer `mov eax,ds:C; mov ds:G,eax; ret` with the dword at `C` +∞: returns `C`.
fn infinity_constant(listing: &Listing, image: &Image, address: u32) -> Option<u32> {
    let code = instructions(listing, address, 3);
    let [(_, load), (_, store), (_, ret)] = code.as_slice() else {
        return None;
    };
    let constant = u32::from_str_radix(&INFINITY_LOAD.captures(load)?[1], 16).ok()?;
    (GLOBAL_STORE.is_match(store)
        && ret.starts_with("ret")
        && image.dword(constant)? == 0x7f80_0000)
        .then_some(constant)
}

/// The initializer table's runs, in link order: for each, the functions of the file that opens
/// it, the one with the +∞ initializer: its +∞ thunk and initializer, and the functions that read
/// its own copies of the header's +∞, π and 10, each only while no other file's +∞ lies at or
/// below it after the run's own: the next literal is often the next file's +∞. A run's later
/// thunks can be another file's, one
/// without the header, and are left out.
fn initializer_runs(
    listing: &Listing,
    image: &Image,
    operands: &HashMap<u32, Vec<u32>>,
    table: (u32, u32),
) -> Vec<Vec<u32>> {
    let mut opened: Vec<(u32, u32, u32)> = Vec::new();
    let mut slot = table.0;
    while slot < table.1 {
        let thunk = image.dword(slot).unwrap_or(0);
        slot += 4;
        if thunk == 0 || !listing.contains(thunk) {
            continue;
        }
        let mut targets = vec![thunk];
        for (_, text) in instructions(listing, thunk, 6) {
            if let Some(found) = BRANCH.captures(text) {
                if let Ok(t) = u32::from_str_radix(&found[1], 16) {
                    targets.push(t);
                }
                break;
            }
        }
        if let Some((initializer, c)) = targets
            .iter()
            .find_map(|&t| Some((t, infinity_constant(listing, image, t)?)))
        {
            opened.push((thunk, initializer, c));
        }
    }
    let infinities: BTreeSet<u32> = opened.iter().map(|o| o.2).collect();
    opened
        .iter()
        .map(|&(thunk, initializer, c)| {
            let mut members: BTreeSet<u32> = [thunk, initializer].into_iter().collect();
            members.extend(operands.get(&c).into_iter().flatten().copied());
            for constant in [c + 4, c + 8] {
                // The next constant at or below this one that is another file's +inf ends the
                // run's own copies.
                if infinities.range(c + 1..=constant).next().is_some() {
                    break;
                }
                members.extend(operands.get(&constant).into_iter().flatten().copied());
            }
            members.into_iter().collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_map_to_their_systems() {
        let cases = [
            (r"E:\build\buildWoW\WoW\Source\Ui\GameUI.cpp", Some("ui")),
            (
                r"E:\build\buildWoW\WoW\Source\UIUtil\Camera.cpp",
                Some("uicamera"),
            ),
            (
                r"E:\build\buildWoW\WoW\Source\UIUtil\Tooltip.cpp",
                Some("ui"),
            ),
            (
                r"E:\build\buildWoW\WoW\Source\Object\ObjectClient\Movement_C.cpp",
                Some("collision"),
            ),
            (
                r"E:\build\buildWoW\WoW\Source\Object\ObjectClient\Unit_C.cpp",
                Some("objectlayer"),
            ),
            (
                r"E:\build\buildWoW\WoW\Source\WorldClient\MapObjGroup.cpp",
                Some("models"),
            ),
            (
                r"E:\build\buildWoW\WoW\Source\WorldClient\Map.cpp",
                Some("terrain"),
            ),
            (
                r"E:\build\buildWoW\engine\Source\Base\Status.cpp",
                Some("client"),
            ),
            (
                r"E:\build\buildWoW\engine\Source\Base\Prop.cpp",
                Some("base"),
            ),
            (
                r"E:\build\buildWoW\ENGINE\SOURCE\lua\src\ldo.c",
                Some("lua"),
            ),
            (r"E:\build\buildWoW\Storm\Source\SFile.cpp", Some("storm")),
            (
                r"E:\build\buildWoW\ENGINE\Source\Services\Profile.cpp",
                None,
            ),
        ];
        for (path, system) in cases {
            assert_eq!(system_of_path(path), system, "{path}");
        }
        assert!(OBJECT_PATH.is_match(r"E:\build\x\GameUI.cpp"));
        assert!(OBJECT_PATH.is_match(r"E:\build\x\LCD.CPP"));
        assert!(!OBJECT_PATH.is_match(r"..\Object/ObjectClient/Player_C.h"));
        assert_eq!(file_name(r"E:\a\b/Player_C.cpp"), "Player_C.cpp");
    }

    #[test]
    fn runtime_names_exclude_duplicates_and_known_false_matches() {
        let crt = |n: &str| CRT_NAME.is_match(n) && !DUPLICATE.is_match(n) && !NOT_CRT.contains(&n);
        assert!(crt("_malloc"));
        assert!(crt("__aulldiv"));
        assert!(crt("_x64toa@20"));
        assert!(!crt("__cfltcvt_init"));
        assert!(!crt("__cfltcvt_init_2"));
        assert!(!crt("_WinMain@16"));
        assert!(!crt(
            "?_Get_deleter@_Ref_count_base@std@@UBEPAXABVtype_info@@@Z"
        ));
        assert!(!crt("SStrCopy"));
    }

    /// A one-section image at 0x401000 holding `data` at its offset 0x1000.
    fn image(data: &[(u32, &[u8])]) -> Vec<u8> {
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

    const GAME_UI: &str = r"E:\build\buildWoW\WoW\Source\Ui\GameUI.cpp";
    const OTHER_UI: &str = r"E:\build\buildWoW\WoW\Source\Ui\Other.cpp";

    /// Functions of 0x10 bytes from 0x401000, each a `push` of the string at the address given.
    fn owners_of_pushes(pushes: &[Option<u32>]) -> Owners {
        let mut text = String::new();
        let mut extents = Vec::new();
        for (i, push) in pushes.iter().enumerate() {
            let at = 0x401000 + 0x10 * i as u32;
            extents.push((at, at + 0x10));
            match push {
                Some(string) => text.push_str(&format!("  {at:x}:\tpush   0x{string:x}\n")),
                None => text.push_str(&format!("  {at:x}:\tnop\n")),
            }
        }
        let listing = Listing::parse(text.as_bytes()).expect("a listing");
        let bytes = image(&[
            (0x402000, GAME_UI.as_bytes()),
            (0x402100, OTHER_UI.as_bytes()),
        ]);
        let image = Image::new(&bytes).expect("an image");
        assign(
            &listing,
            &image,
            &extents,
            &HashMap::new(),
            &HashMap::new(),
            &BTreeSet::new(),
            &BTreeSet::new(),
        )
    }

    #[test]
    fn a_stretch_is_settled_only_between_functions_of_one_file() {
        let (a, b) = (Some(0x402000), Some(0x402100));
        // File A, an unproved function, file A again, an unproved function, file B.
        let owners = owners_of_pushes(&[a, None, a, None, b]);
        let of = |i: u32| owners.owners.get(&(0x401000 + 0x10 * i));
        assert!(matches!(of(0), Some(o) if o.system == "ui" && o.source == Source::Path));
        assert!(matches!(of(1), Some(o) if o.system == "ui" && o.source == Source::Stretch));
        assert!(matches!(of(2), Some(o) if o.system == "ui" && o.source == Source::Path));
        assert!(matches!(of(4), Some(o) if o.system == "ui" && o.source == Source::Path));
        // Two files of one system around it prove nothing about it.
        assert!(of(3).is_none());
    }

    #[test]
    fn overlapping_files_settle_nothing_between_them() {
        let (a, b) = (Some(0x402000), Some(0x402100));
        let owners = owners_of_pushes(&[a, b, None, a, b]);
        assert!(!owners.owners.contains_key(&0x401020));
        assert!(!owners.conflicts.is_empty());
    }

    /// Two files' initializer runs: each file's thunk jumps to its +∞ initializer, which stores
    /// the constant at `c`. The first file's constants are at 0x402300; `next` is where the second
    /// file's +∞ lies.
    fn two_runs(next: u32) -> Vec<Vec<u32>> {
        let mut text = String::new();
        for (thunk, init, c) in [
            (0x401000u32, 0x401010u32, 0x402300u32),
            (0x401100, 0x401110, next),
        ] {
            text.push_str(&format!("  {thunk:x}:\tjmp    0x{init:x}\n"));
            text.push_str(&format!("  {init:x}:\tmov    eax,ds:0x{c:x}\n"));
            text.push_str(&format!("  {:x}:\tmov    ds:0x403000,eax\n", init + 5));
            text.push_str(&format!("  {:x}:\tret\n", init + 10));
        }
        let listing = Listing::parse(text.as_bytes()).expect("a listing");
        let infinity = 0x7f80_0000u32.to_le_bytes();
        let table: Vec<u8> = [0x401000u32, 0x401100]
            .iter()
            .flat_map(|t| t.to_le_bytes())
            .collect();
        let bytes = image(&[(0x402300, &infinity), (next, &infinity), (0x402200, &table)]);
        let image = Image::new(&bytes).expect("an image");
        // Functions reading the constants at 0x402304 and 0x402308.
        let operands: HashMap<u32, Vec<u32>> =
            [(0x402304, vec![0x401040]), (0x402308, vec![0x401050])]
                .into_iter()
                .collect();
        initializer_runs(&listing, &image, &operands, (0x402200, 0x402208))
    }

    #[test]
    fn a_run_keeps_its_own_constants() {
        let runs = two_runs(0x402320);
        assert_eq!(runs[0], [0x401000, 0x401010, 0x401040, 0x401050]);
    }

    #[test]
    fn a_run_ends_at_the_next_files_infinity() {
        // The second file's +∞ is the first file's third constant, or its second: the readers
        // of it are the second file's.
        let runs = two_runs(0x402308);
        assert_eq!(runs[0], [0x401000, 0x401010, 0x401040]);
        let runs = two_runs(0x402304);
        assert_eq!(runs[0], [0x401000, 0x401010]);
    }

    #[test]
    fn a_run_across_a_proved_file_is_a_conflict() {
        // File A's functions at 0x401000 and 0x401050 hold file B's at 0x401030; A's run has a
        // member on each side of it.
        let lines = [
            (0x401000, "push   0x402000"),
            (0x401010, "jmp    0x401020"),
            (0x401020, "mov    eax,ds:0x402300"),
            (0x401025, "mov    ds:0x403000,eax"),
            (0x40102a, "ret"),
            (0x401030, "push   0x402100"),
            (0x401040, "mov    eax,ds:0x402304"),
            (0x401050, "push   0x402000"),
        ];
        let text: String = lines
            .iter()
            .map(|(a, t)| format!("  {a:x}:\t{t}\n"))
            .collect();
        let listing = Listing::parse(text.as_bytes()).expect("a listing");
        let extents: Vec<(u32, u32)> = [0x401000, 0x401010, 0x401020, 0x401030, 0x401040, 0x401050]
            .iter()
            .map(|&a| (a, a + 0x10))
            .collect();
        let infinity = 0x7f80_0000u32.to_le_bytes();
        let bytes = image(&[
            (0x402000, GAME_UI.as_bytes()),
            (0x402100, OTHER_UI.as_bytes()),
            (0x402300, &infinity),
            (0x402200, &0x401010u32.to_le_bytes()),
        ]);
        let image = Image::new(&bytes).expect("an image");
        let owners = assign_in(
            &listing,
            &image,
            &extents,
            &HashMap::new(),
            &HashMap::new(),
            &BTreeSet::new(),
            &BTreeSet::new(),
            (0x402200, 0x402204),
        );
        assert!(owners.conflicts.iter().any(|c| c.contains("crosses")));
        assert!(!owners.owners.contains_key(&0x401020));
    }
}
