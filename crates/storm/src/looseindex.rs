//! The loose-file index: Storm's table of the files on disk under the install, which the open path
//! asks before the archives when the caller's flags allow it.
//!
//! `0x646ea0` builds it on the first open that reaches the resolver (`0x647e60`, at `0x647ebc`):
//! `0x646910` walks the base folder and then the base folder's `Data\`, depth first in listing
//! order, and keys every regular file by its path below the folder it walked. `0x646fc0` empties it
//! and the next open builds it again. The resolver asks it for the file name alone (flag bit 1),
//! then for the whole name (bit 0 or 1), and only on a miss goes on to the archives.

use std::collections::HashMap;

/// What an index entry opens: the kind `0x646910` stores at `+0x1c` and `SFile::OpenEx`
/// (`0x6477c0`) switches on (`0x64783d`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LooseKind {
    /// Any file but the two below: opened with `fopen(path, "rb")` (`0x647844`).
    Plain = 0,
    /// A `.bz` file, keyed without the suffix: opened with `fopen` and refused unless its 24-byte
    /// header holds `BZ00` in its second word (`0x647894`).
    Bz = 1,
    /// A `.MPQ` file, keyed without the suffix: an archive from which the requested name's last
    /// path component is opened (`0x647a30`).
    Mpq = 2,
}

/// One entry: the full disk path, extension kept, and its kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LooseEntry {
    pub disk_path: String,
    pub kind: LooseKind,
}

/// What a walk sees: the names a folder lists, in its own order, and whether a path is a folder,
/// a regular file or neither.
pub trait LooseSource {
    /// The entries of `dir` (a path without a trailing backslash), `.` and `..` included as the
    /// system lists them, or `None` when it is not a listable folder.
    fn list(&self, dir: &str) -> Option<Vec<String>>;
    /// `Some(true)` for a folder, `Some(false)` for a regular file, `None` when `stat` fails or the
    /// path is neither.
    fn kind(&self, path: &str) -> Option<bool>;
}

/// The data folder the second walk takes, `0xc520c8`, which start-up sets to `Data\` from the
/// root table's first entry (`0x82e124`).
pub const DATA_DIR: &str = "Data\\";

/// The index: keys compared without ASCII case (`SStrCmpI`), the first entry of a key kept.
#[derive(Default, Debug)]
pub struct LooseIndex {
    map: HashMap<Vec<u8>, LooseEntry>,
}

/// Where a resolved name comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved {
    /// The index answered: the caller opens this entry and nothing else.
    Loose(LooseEntry),
    /// The index did not answer, or was not asked: the archive walk runs with flag bits 0 and 1
    /// cleared.
    Archives,
}

fn fold(key: &str) -> Vec<u8> {
    key.bytes().map(|c| c.to_ascii_lowercase()).collect()
}

fn with_backslash(mut s: String) -> String {
    if !s.ends_with('\\') {
        s.push('\\');
    }
    s
}

impl LooseIndex {
    /// Builds the index as `0x646ea0` does from the base path `base` (`0xc52418`): the base folder
    /// (or `.` when it cannot be listed), then that string with [`DATA_DIR`] appended.
    pub fn build(src: &impl LooseSource, base: &str) -> LooseIndex {
        let mut ix = LooseIndex::default();
        let trimmed = base.trim_end_matches('\\');
        let root = if src.list(trimmed).is_some() {
            base.to_string()
        } else {
            ".".to_string()
        };
        ix.walk(src, &root, "");
        let data_root = format!("{root}{DATA_DIR}");
        if src.list(data_root.trim_end_matches('\\')).is_some() {
            ix.walk(src, &data_root, "");
        }
        ix
    }

    /// `0x646910(root, prefix)`: every entry of `root\prefix` but `.` and `..`; a folder is
    /// skipped when its path below the root is a prefix of [`DATA_DIR`] (case-sensitive), and
    /// walked otherwise; a regular file is inserted.
    fn walk(&mut self, src: &impl LooseSource, root: &str, prefix: &str) {
        let root = with_backslash(root.to_string());
        let dir_rel = if prefix.is_empty() {
            String::new()
        } else {
            with_backslash(prefix.to_string())
        };
        let dir = format!("{root}{dir_rel}");
        let Some(names) = src.list(dir.trim_end_matches('\\')) else {
            return;
        };
        for name in names {
            if name == "." || name == ".." {
                continue;
            }
            let rel = format!("{dir_rel}{name}");
            let full = format!("{root}{rel}");
            match src.kind(&full) {
                Some(true) => {
                    if DATA_DIR.as_bytes().starts_with(rel.as_bytes()) {
                        continue;
                    }
                    self.walk(src, &root, &rel);
                }
                Some(false) => self.insert_file(&full, &rel),
                None => {}
            }
        }
    }

    fn insert_file(&mut self, full: &str, rel: &str) {
        let suffix = full.rfind('.').map(|i| &full[i..]);
        let (key, kind) = match suffix {
            Some(s) if s.eq_ignore_ascii_case(".bz") => (&rel[..rel.len() - 3], LooseKind::Bz),
            Some(s) if s.eq_ignore_ascii_case(".MPQ") => (&rel[..rel.len() - 4], LooseKind::Mpq),
            _ => (rel, LooseKind::Plain),
        };
        self.map.entry(fold(key)).or_insert(LooseEntry {
            disk_path: full.to_string(),
            kind,
        });
    }

    /// The entry whose key equals `name` without ASCII case. A `/` does not match a `\`.
    pub fn get(&self, name: &str) -> Option<&LooseEntry> {
        self.map.get(&fold(name))
    }

    /// The number of keys.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// `0x647e60`'s first two probes for an open with `flags`: with bit 1 the part of `name` after
    /// its last backslash, then with bit 0 or 1 the whole name; a hit ends the open's search.
    pub fn resolve(&self, name: &str, flags: u32) -> Resolved {
        if flags & 3 == 0 {
            return Resolved::Archives;
        }
        if flags & 2 != 0 {
            let base = name.rsplit('\\').next().unwrap_or(name);
            if let Some(e) = self.get(base) {
                return Resolved::Loose(e.clone());
            }
        }
        match self.get(name) {
            Some(e) => Resolved::Loose(e.clone()),
            None => Resolved::Archives,
        }
    }
}
