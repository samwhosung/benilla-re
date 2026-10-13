//! The object file, and so the system, of every function start (`systems/README.md`, "Object
//! files"; `symbols/README.md`, "One row per address").
//!
//! The evidence, strongest first:
//!
//! - a row's own system, for a row this pass does not fill;
//! - a span a written system's doc lists by address (`DECLARED`);
//! - a source path the function passes (`__FILE__` in an assertion or an allocation), mapped to
//!   its system by the directory and file rules in `RULES`; a header's path names no file, since
//!   headers are inlined anywhere;
//! - the span of one path: code is contiguous per object file, so every function between the
//!   first and the last that pass the same `.cpp` path is that file's, unless a row inside belongs
//!   to another system;
//! - an initializer run: the C runtime's table `0x827000` lists each file's static initializers
//!   in link order, a file that includes the shared maths header opening its run with its +∞
//!   initializer. That initializer, its thunk and the functions that read the file's own copies of
//!   the header's constants are one file's, and so is everything between them; that span is the
//!   system of the evidence inside it, when the evidence agrees;
//! - the C runtime's own names in the community database (`_malloc`, `__aulldiv`), whose
//!   functions are the `crt` library's;
//! - a DBC file name (`DBFilesClient\…`), which only the DBC stores' loaders pass;
//! - neighbours: a function between two functions of one system, with no evidence of another
//!   between them, is that system's.
//!
//! What none of these settles is `unassigned`.

use crate::bodies::{Image, Listing, IMMEDIATE};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::LazyLock;

/// The C runtime's initializer table and its end, `systems/README.md`, "Object files".
const INITIALIZERS: (u32, u32) = (0x827000, 0x82e030);

/// The object files the docs list by address where no path or row bounds them: `ui`'s runs
/// (`systems/ui/README.md`) and the minigame's board file (`symbols/README.md`). A span is that
/// system's; a span whose end the doc leaves open is one address, its first function.
const DECLARED: &[(u32, u32, &str)] = &[
    (0x480380, 0x509ff0, "ui"),
    (0x515880, 0x536ea0, "ui"),
    (0x6ed930, 0x6ed930, "ui"),
    (0x701940, 0x706430, "ui"),
    (0x728b10, 0x728ef3, "minigame"),
    (0x728f00, 0x730bbf, "ui"),
    (0x764160, 0x7a3a80, "ui"),
    (0x7cb0f0, 0x7cc310, "ui"),
    (0x7cc320, 0x7cc320, "ui"),
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
    PathSpan,
    Run,
    Library,
    DbcName,
    Neighbours,
}

impl Source {
    pub fn word(self) -> &'static str {
        match self {
            Source::Row => "row",
            Source::Declared => "a doc's span",
            Source::Path => "path",
            Source::PathSpan => "path span",
            Source::Run => "initializer run",
            Source::Library => "runtime name",
            Source::DbcName => "DBC file name",
            Source::Neighbours => "neighbours",
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
    // Each path's span.
    let spans: Vec<(&String, u32, u32)> = paths
        .iter()
        .filter_map(|(p, fs)| Some((p, *fs.first()?, *fs.last()?)))
        .collect();
    for (k, &(path, low, high)) in spans.iter().enumerate() {
        let Some(system) = system_of_path(path) else {
            continue;
        };
        let overlap = spans.iter().enumerate().find(|&(j, &(other, l, h))| {
            j != k && file_name(other) != file_name(path) && l <= high && low <= h
        });
        if let Some((_, (other, _, _))) = overlap {
            conflicts.push(format!(
                "{} ({low:#x} to {high:#x}) overlaps {}; its span is not used",
                file_name(path),
                file_name(other)
            ));
            continue;
        }
        let i = starts.partition_point(|&a| a < low);
        let j = starts.partition_point(|&a| a <= high);
        let foreign = starts[i..j]
            .iter()
            .find(|a| rows.get(a).is_some_and(|s| *s != system));
        if let Some(a) = foreign {
            conflicts.push(format!(
                "{} ({low:#x} to {high:#x}) holds the {} row {a:#x}; its span is not used",
                file_name(path),
                rows[a]
            ));
            continue;
        }
        for &a in &starts[i..j] {
            files.entry(a).or_insert_with(|| file_name(path));
            owners.entry(a).or_insert(Owner {
                system: system.to_string(),
                source: Source::PathSpan,
            });
        }
    }

    // The initializer runs. A run's +∞ initializer and the functions that read the run's own
    // copies of the header's constants are one file's, so every function between the first and
    // the last of them is that file's: its system, when the evidence inside agrees.
    let runs = initializer_runs(listing, image, &operands);
    for members in &runs {
        let (Some(&low), Some(&high)) = (members.first(), members.last()) else {
            continue;
        };
        let i = starts.partition_point(|&a| a < low);
        let j = starts.partition_point(|&a| a <= high);
        let systems: BTreeSet<&str> = starts[i..j]
            .iter()
            .filter_map(|a| owners.get(a))
            .map(|o| o.system.as_str())
            .collect();
        let barred = barriers.range(low..=high).next().is_some();
        if systems.len() == 1 && !barred {
            let system = systems.into_iter().next().expect("one system").to_string();
            // A written system lists its own object files; a run adds none to it.
            if documented.contains(&system) {
                continue;
            }
            for &a in &starts[i..j] {
                owners.entry(a).or_insert(Owner {
                    system: system.clone(),
                    source: Source::Run,
                });
            }
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

    // Neighbours: between two settled functions of one system.
    let mut settled: Vec<(u32, String)> = owners
        .iter()
        .map(|(&a, o)| (a, o.system.clone()))
        .chain(barriers.iter().map(|&a| (a, String::new())))
        .collect();
    settled.sort();
    for &a in &starts {
        if owners.contains_key(&a) || barriers.contains(&a) {
            continue;
        }
        let i = settled.partition_point(|s| s.0 < a);
        let (Some(before), Some(after)) = (i.checked_sub(1).map(|k| &settled[k]), settled.get(i))
        else {
            continue;
        };
        if before.1 == after.1 && !before.1.is_empty() {
            let system = before.1.clone();
            owners.insert(
                a,
                Owner {
                    system,
                    source: Source::Neighbours,
                },
            );
        }
    }
    owners.retain(|a, o| o.source == Source::Row || !thunks.contains(a));
    Owners {
        owners,
        files,
        conflicts,
        runs: runs.len(),
    }
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
/// its own copies of the header's +∞, π and 10. A run's later thunks can be another file's, one
/// without the header, and are left out.
fn initializer_runs(
    listing: &Listing,
    image: &Image,
    operands: &HashMap<u32, Vec<u32>>,
) -> Vec<Vec<u32>> {
    let mut runs: Vec<Vec<u32>> = Vec::new();
    let mut slot = INITIALIZERS.0;
    while slot < INITIALIZERS.1 {
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
        let Some((initializer, c)) = targets
            .iter()
            .find_map(|&t| Some((t, infinity_constant(listing, image, t)?)))
        else {
            continue;
        };
        let mut members: BTreeSet<u32> = [thunk, initializer].into_iter().collect();
        for constant in [c, c + 4, c + 8] {
            members.extend(operands.get(&constant).into_iter().flatten().copied());
        }
        runs.push(members.into_iter().collect());
    }
    runs
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
}
