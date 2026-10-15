//! The loose-file index, run in the exe: `SFileResolveOpen` (`0x647e60`) builds it on its first
//! call through `0x646ea0` and `0x646910`, over a synthetic install, and answers names with every
//! flag value; `LooseIndex` is built over the same install and must answer alike.
//!
//! The table's static object is constructed by its initializer body `0x6466d0`, and the globals
//! start-up sets are written as it leaves them: the base path `0xc52418`, the data folder
//! `0xc520c8` (`Data\`), the extra flags `0x865b40` (0, `0x648be0(0)`) and the byte `0x865b44`
//! (0, `0x648c20`). What touches the system is stubbed, each with the callee's own `ret N`:
//!
//! - `0x6674a0` (`ret 4`), the folder open: a handle for a folder of the install, else 0.
//! - `0x667580` (`ret 4`), the next name of a handle, `.` and `..` first, else 0.
//! - `0x6676a0` (`ret 4`), the close.
//! - `0x646d90` (`ret`, path in `ecx`, `_stat` buffer in `edx`): `st_mode` at `+6`, `0x4000` for a
//!   folder and `0x8000` for a file, and 0; -1 for a path that is neither.
//! - `0x654920` (`ret 0x10`), the archive walk: 1 for a name the synthetic archives hold. Its
//!   flags argument is logged and checked.
//! - `SMemAlloc` (`0x6462e0`, `ret 0x10`) as a zero-filled bump arena, `SMemFree` (`0x646430`,
//!   `ret 0x10`) and `SErrSetLastError` (`0x64e850`, `ret 4`).
//!
//! The rest runs as the client's bytes: the walk, the key cuts, the inserts and the table code,
//! `SStrHashHT`, `SStrCmpI`, `SStrDupA` and the three probes of the resolver.

mod common;

use benilla_re_difftest::{Emu, RegisterX86};
use benilla_re_storm::{LooseIndex, LooseSource, Resolved};
use common::Wow;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

const RESOLVE: u64 = 0x647e60;
const TABLE_INIT: u64 = 0x6466d0;
const DIR_OPEN: u64 = 0x6674a0;
const DIR_NEXT: u64 = 0x667580;
const DIR_CLOSE: u64 = 0x6676a0;
const STAT: u64 = 0x646d90;
const ARCHIVE_WALK: u64 = 0x654920;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_FREE: u64 = 0x646430;
const SMEM_REALLOC: u64 = 0x646320;
const SERR_SET: u64 = 0x64e850;

const G_BASE_PATH: u64 = 0xc52418;
const G_DATA_DIR: u64 = 0xc520c8;
const G_EXTRA_FLAGS: u64 = 0x865b40;
const G_KEEP_FLAGS: u64 = 0x865b44;

const BASE: &str = "C:\\W\\";

/// A synthetic install: every path below `BASE` (without it), a trailing backslash for a folder,
/// listed in the order a folder returns them.
const TREE: &[&str] = &[
    "x.bz",
    "x",
    "readme.BZ",
    "FRIZQT__.TTF",
    "Fonts\\",
    "Fonts\\ARIALN.TTF",
    "Interface\\",
    "Interface\\AddOns\\",
    "Interface\\AddOns\\Foo\\",
    "Interface\\AddOns\\Foo\\Foo.toc",
    "Interface\\AddOns\\Foo\\art.tga",
    "Interface\\AddOns\\Foo\\art.blp.bz",
    "Interface\\AddOns\\Foo\\art.blp",
    "Interface\\AddOns\\Foo\\pack.MPQ",
    "my.addon\\",
    "my.addon\\file",
    "Dat\\",
    "Dat\\hidden.txt",
    "D\\",
    "D\\hidden2.txt",
    "data2\\",
    "data2\\seen.txt",
    "data\\",
    "data\\low.txt",
    "dat\\",
    "dat\\low2.txt",
    "Lower.mpq",
    "Sound\\",
    "Sound\\s.wav",
    "WTF\\",
    "WTF\\Config.wtf",
    "Data\\",
    "Data\\model.MPQ",
    "Data\\patch.MPQ",
    "Data\\Interface\\",
    "Data\\Interface\\AddOns\\",
    "Data\\Interface\\AddOns\\Foo\\",
    "Data\\Interface\\AddOns\\Foo\\Foo.toc",
    "Data\\Interface\\AddOns\\Foo\\only.lua",
    "Data\\SOUND\\",
    "Data\\SOUND\\S.WAV",
    "Data\\Textures\\",
    "Data\\Textures\\x.blp",
    "Data\\Data\\",
    "Data\\Data\\deep.txt",
];

/// Names the synthetic archives hold.
const ARCHIVE: &[&str] = &[
    "Interface\\AddOns\\Foo\\Foo.toc",
    "Fonts\\FRIZQT__.TTF",
    "Fonts\\ARIALN.TTF",
    "Textures\\y.blp",
    "DBFilesClient\\Spell.dbc",
];

struct Install {
    /// Folder (full path, no trailing backslash) to its names in order.
    dirs: BTreeMap<String, Vec<String>>,
    files: HashSet<String>,
}

impl Install {
    /// The tree under `root`, a folder path without its trailing backslash.
    fn new(root: &str) -> Install {
        let mut dirs: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut files = HashSet::new();
        let root = root.to_string();
        dirs.insert(root.clone(), vec![".".into(), "..".into()]);
        for p in TREE {
            let is_dir = p.ends_with('\\');
            let rel = p.trim_end_matches('\\');
            let full = format!("{root}\\{rel}");
            let (parent, name) = full.rsplit_once('\\').unwrap();
            dirs.get_mut(parent)
                .unwrap_or_else(|| panic!("parent of {p} listed first"))
                .push(name.to_string());
            if is_dir {
                dirs.insert(full, vec![".".into(), "..".into()]);
            } else {
                files.insert(full);
            }
        }
        Install { dirs, files }
    }
}

impl LooseSource for Install {
    fn list(&self, dir: &str) -> Option<Vec<String>> {
        self.dirs.get(dir).cloned()
    }
    fn kind(&self, path: &str) -> Option<bool> {
        if self.dirs.contains_key(path) {
            Some(true)
        } else if self.files.contains(path) {
            Some(false)
        } else {
            None
        }
    }
}

#[derive(Default)]
struct Handles {
    /// Handle address to (names, next index).
    open: BTreeMap<u32, (Vec<String>, usize)>,
    next_handle: u32,
    /// The flags the archive walk received.
    walk_flags: Vec<u32>,
}

fn write_cstr(emu: &mut Emu, va: u64, s: &str) {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    emu.write(va, &b);
}

fn read_cstr(emu: &Emu, va: u64) -> String {
    let b = emu.read(va, 0x104);
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..n]).into_owned()
}

/// Sets up the emulator: globals, the table, the stubs over `install`.
fn setup<'a>(wow: &'a Wow, base: &str, install: Rc<Install>, st: Rc<RefCell<Handles>>) -> Emu<'a> {
    let mut emu = Emu::new(wow);
    write_cstr(&mut emu, G_BASE_PATH, base);
    write_cstr(&mut emu, G_DATA_DIR, "Data\\");
    emu.write_u32(G_EXTRA_FLAGS, 0);
    emu.write(G_KEEP_FLAGS, &[0]);
    emu.call(TABLE_INIT, 0, 0, &[]);

    let arena = emu.alloc(0x40_0000);
    emu.stub_alloc_sized(SMEM_ALLOC, arena, arena + 0x40_0000, 0, 4);
    emu.stub_realloc_sized(SMEM_REALLOC, 5);
    emu.stub(SMEM_FREE, 1, 4);
    emu.stub(SERR_SET, 0, 1);
    emu.stub(DIR_CLOSE, 0, 1);

    let handles = emu.alloc(0x1000);
    let name_buf = emu.alloc(0x200);
    st.borrow_mut().next_handle = handles as u32;
    {
        let (install, st) = (install.clone(), st.clone());
        emu.stub_fn(DIR_OPEN, 1, move |io| {
            let path = String::from_utf8(io.read_cstr(u64::from(io.arg(0)))).unwrap();
            match install.list(path.trim_end_matches('\\')) {
                Some(names) => {
                    let mut s = st.borrow_mut();
                    let h = s.next_handle;
                    s.next_handle += 0x10;
                    s.open.insert(h, (names, 0));
                    h
                }
                None => 0,
            }
        });
    }
    {
        let st = st.clone();
        emu.stub_fn(DIR_NEXT, 1, move |io| {
            let h = io.arg(0);
            let mut s = st.borrow_mut();
            let (names, i) = s.open.get_mut(&h).expect("an open handle");
            if *i >= names.len() {
                return 0;
            }
            let mut b = names[*i].as_bytes().to_vec();
            *i += 1;
            b.push(0);
            io.write(name_buf, &b);
            name_buf as u32
        });
    }
    {
        let install = install.clone();
        emu.stub_fn(STAT, 0, move |io| {
            let path = String::from_utf8(io.read_cstr(io.reg(RegisterX86::ECX))).unwrap();
            let buf = io.reg(RegisterX86::EDX);
            io.write(buf, &[0u8; 0x24]);
            let mode: u16 = match install.kind(&path) {
                Some(true) => 0x41ff,
                Some(false) => 0x81b6,
                None => return u32::MAX,
            };
            io.write(buf + 6, &mode.to_le_bytes());
            0
        });
    }
    {
        let st = st.clone();
        emu.stub_fn(ARCHIVE_WALK, 4, move |io| {
            let name = String::from_utf8(io.read_cstr(u64::from(io.arg(1)))).unwrap();
            st.borrow_mut().walk_flags.push(io.arg(2));
            let hit = ARCHIVE.iter().any(|a| a.eq_ignore_ascii_case(&name));
            u32::from(hit)
        });
    }
    emu
}

/// The resolver's answer: its return, the path it wrote and the kind (`[ebp+0x10]`).
fn resolve(
    emu: &mut Emu,
    name_va: u64,
    out: u64,
    kind: u64,
    name: &str,
    flags: u32,
) -> (u32, String, u32) {
    write_cstr(emu, name_va, name);
    emu.write(out, &[0u8; 0x104]);
    emu.write_u32(kind, 0xdead);
    let arch = emu.alloc(4);
    let r = emu.call(
        RESOLVE,
        name_va as u32,
        out as u32,
        &[0x104, flags, kind as u32, arch as u32],
    );
    (r, read_cstr(emu, out), emu.read_u32(kind))
}

/// Builds the index in the exe on the first resolve and compares every name and flag value
/// with `LooseIndex`: a hit's path and kind, and on a miss the archive walk's answer, called with
/// flag bits 0 and 1 cleared.
#[test]
fn resolve_matches_index() {
    let Some(wow) = Wow::load() else {
        return;
    };
    compare(&wow, BASE, BASE.trim_end_matches('\\'), 18);
}

/// The same with a base path that does not list: the exe walks `.` and then `.Data\`, so the
/// install's `Data` folder is not walked at all.
#[test]
fn index_falls_back_to_dot() {
    let Some(wow) = Wow::load() else {
        return;
    };
    compare(&wow, "C:\\Gone\\", ".", 14);
}

/// Builds the index in the exe for base path `base` over the tree rooted at `root`, and compares
/// every name and flag value with `LooseIndex`, which must hold more than `min_keys` keys.
fn compare(wow: &Wow, base: &str, root: &str, min_keys: usize) {
    let install = Rc::new(Install::new(root));
    let st = Rc::new(RefCell::new(Handles::default()));
    let mut emu = setup(wow, base, install.clone(), st.clone());
    let model = LooseIndex::build(install.as_ref(), base);

    let name_va = emu.alloc(0x400);
    let out = emu.alloc(0x104);
    let kind = emu.alloc(4);

    let mut names: Vec<String> = TREE
        .iter()
        .filter(|p| !p.ends_with('\\'))
        .flat_map(|p| {
            let below_data = p.strip_prefix("Data\\").map(str::to_string);
            let base = p.rsplit('\\').next().unwrap().to_string();
            let mut v = vec![
                p.to_string(),
                p.to_ascii_uppercase(),
                p.replace('\\', "/"),
                base,
            ];
            v.extend(below_data);
            v
        })
        .collect();
    names.extend(
        [
            "x",
            "readme",
            "Interface\\AddOns\\Foo\\art.blp",
            "Interface\\AddOns\\Foo\\pack",
            "Interface\\AddOns\\Foo\\pack\\inner.blp",
            "Fonts\\FRIZQT__.TTF",
            "fonts\\frizqt__.ttf",
            "Dat\\hidden.txt",
            "D\\hidden2.txt",
            "data2\\seen.txt",
            "data\\low.txt",
            "dat\\low2.txt",
            "low.txt",
            "Lower",
            "Data\\deep.txt",
            "model",
            "patch",
            "Textures\\y.blp",
            "DBFilesClient\\Spell.dbc",
            "nothing\\here.txt",
            "",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    names.extend(ARCHIVE.iter().map(|s| s.to_string()));

    let mut checked = 0;
    for name in &names {
        for flags in [0u32, 1, 2, 3, 0x10000, 0x10001] {
            st.borrow_mut().walk_flags.clear();
            let (r, path, k) = resolve(&mut emu, name_va, out, kind, name, flags);
            let walked = st.borrow().walk_flags.clone();
            let in_archive = ARCHIVE.iter().any(|a| a.eq_ignore_ascii_case(name));
            match model.resolve(name, flags) {
                Resolved::Loose(e) => {
                    assert_eq!(
                        (r, path.as_str(), k),
                        (1, e.disk_path.as_str(), e.kind as u32),
                        "{name:?} flags {flags:#x}: the index answers"
                    );
                    assert!(
                        walked.is_empty(),
                        "{name:?} flags {flags:#x}: no archive walk after a hit"
                    );
                }
                Resolved::Archives => {
                    assert_eq!(
                        walked,
                        vec![flags & !3],
                        "{name:?} flags {flags:#x}: one archive walk, bits 0 and 1 cleared"
                    );
                    if in_archive {
                        assert_eq!(
                            (r, path.as_str(), k),
                            (1, name.as_str(), 3),
                            "{name:?} flags {flags:#x}: the archives answer"
                        );
                    } else {
                        assert_eq!(r, 0, "{name:?} flags {flags:#x}: nothing answers");
                    }
                }
            }
            checked += 1;
        }
    }
    assert!(
        model.len() > min_keys,
        "the model indexed {} keys",
        model.len()
    );
    eprintln!(
        "{checked} resolves compared over {} index keys",
        model.len()
    );
}

const OPEN_EX: u64 = 0x6477c0;
const FILE_RECORD_CTOR: u64 = 0x647290;
const FILE_RECORD_DTOR: u64 = 0x6472d0;
const FOPEN: u64 = 0x647db0;
const FREAD: u64 = 0x740c97;
const ARCHIVE_OPEN: u64 = 0x655670;
const ARCHIVE_FILE_OPEN: u64 = 0x656690;
const ARCHIVE_CLOSE: u64 = 0x653780;

/// How each stubbed disk call answers in one case of `index_hit_has_no_fallback`.
#[derive(Clone, Copy, Default)]
struct DiskAnswers {
    /// `fopen`'s `FILE *`, 0 for a failure.
    fopen: u32,
    /// The second word of the 24-byte header `fread` fills.
    magic: u32,
    /// Whether the archive open succeeds.
    archive: bool,
}

/// `SFile::OpenEx(0, name, 1, &file)` on names the index holds, one of which the archives hold too,
/// with each disk open failing in turn: `fopen` (`0x647db0`, `ret 4`) for a plain entry
/// (`0x647865`) and a `.bz` entry (`0x6478d5`); a `.bz` header whose magic is not `BZ00`, read by
/// `fread` (`0x740c97`, `__cdecl`; `0x647964`); the archive open (`0x655670`, `ret 0x10`) of a
/// `.MPQ` entry (`0x647a63`); and the open of the name inside it (`0x656690`, `ret 0x10`;
/// `0x647a94`), the archive then closed (`0x653780`, `ret 4`). Each open returns 0 and the archive
/// walk is never called. With `fopen` succeeding the plain entry opens as kind 2, again without
/// the walk. The file record's constructor (`0x647290`, `ret 4`) is stubbed to return its `this`,
/// and its destructor (`0x6472d0`, `ret`) to do nothing.
#[test]
fn index_hit_has_no_fallback() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let install = Rc::new(Install::new(BASE.trim_end_matches('\\')));
    let st = Rc::new(RefCell::new(Handles::default()));
    let mut emu = setup(&wow, BASE, install, st.clone());
    emu.stub_fn(FILE_RECORD_CTOR, 1, |io| io.reg(RegisterX86::ECX) as u32);
    emu.stub(FILE_RECORD_DTOR, 0, 0);
    emu.stub(ARCHIVE_FILE_OPEN, 0, 4);
    emu.stub(ARCHIVE_CLOSE, 0, 1);
    let answers = Rc::new(RefCell::new(DiskAnswers::default()));
    {
        let a = answers.clone();
        emu.stub_fn(FOPEN, 1, move |_| a.borrow().fopen);
    }
    {
        let a = answers.clone();
        emu.stub_fn(FREAD, 0, move |io| {
            let mut header = [0u8; 0x18];
            header[4..8].copy_from_slice(&a.borrow().magic.to_le_bytes());
            io.write(u64::from(io.arg(0)), &header);
            0x18
        });
    }
    {
        let a = answers.clone();
        emu.stub_fn(ARCHIVE_OPEN, 4, move |io| {
            if !a.borrow().archive {
                return 0;
            }
            io.write_u32(u64::from(io.arg(3)), 0x7300_1000);
            1
        });
    }
    let name_va = emu.alloc(0x400);
    let file = emu.alloc(4);
    let bz00 = u32::from_le_bytes(*b"BZ00");
    let file_ok = 0x7300_0000;
    let fail = |fopen, magic, archive| DiskAnswers {
        fopen,
        magic,
        archive,
    };
    for (name, case, expect) in [
        ("Interface\\AddOns\\Foo\\Foo.toc", fail(0, 0, false), 0),
        ("Interface\\AddOns\\Foo\\art.blp", fail(0, 0, false), 0),
        (
            "Interface\\AddOns\\Foo\\art.blp",
            fail(file_ok, bz00 ^ 1, false),
            0,
        ),
        ("Interface\\AddOns\\Foo\\pack", fail(0, 0, false), 0),
        ("Interface\\AddOns\\Foo\\pack", fail(0, 0, true), 0),
        (
            "Interface\\AddOns\\Foo\\Foo.toc",
            fail(file_ok, 0, false),
            2,
        ),
    ] {
        *answers.borrow_mut() = case;
        st.borrow_mut().walk_flags.clear();
        emu.clear_calls();
        write_cstr(&mut emu, name_va, name);
        emu.write_u32(file, 0xdead);
        let r = emu.call(OPEN_EX, 0, 0, &[0, name_va as u32, 1, file as u32]);
        assert_eq!(r, expect, "{name}: OpenEx");
        assert!(
            st.borrow().walk_flags.is_empty(),
            "{name}: an index hit never reaches the archive walk"
        );
        if case.archive {
            assert!(
                emu.calls().iter().any(|c| c.addr == ARCHIVE_FILE_OPEN),
                "{name}: the open inside the archive ran"
            );
        }
        if case.magic != 0 {
            assert!(
                emu.calls().iter().any(|c| c.addr == FREAD),
                "{name}: the header was read"
            );
        }
    }
}
