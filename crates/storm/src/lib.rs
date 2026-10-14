//! The client's MPQ archive set, mounted in the order the exe mounts it.
//!
//! [`wow_mpq`] reads the MPQ format and its codecs. What this crate adds is the client's mount: the
//! start-up routine `0x403740` opens the base archives at fixed priorities `0x3f` (model) down to
//! `0x36` (dbc), the patch chain above them from `0x40`, and `speech2.MPQ` above the patches. A name
//! resolves by the flat priority walk: the first archive that holds it wins, patches over base, a later
//! patch over an earlier one.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use wow_mpq::{FileEntry, PatchChain, Result};

/// The 1.12.1 base asset archives, the table at `0x82e12c` in table order. The exe gives them fixed
/// priorities descending from `0x3f` (model) to `0x36` (dbc), whether or not an open succeeds
/// (`edx` starts at `0x3f` at `0x40389f` and is decremented at `0x403908` for every entry), so among
/// base archives an earlier table entry wins a name collision. `base.MPQ` is absent: the client opens
/// it for `telemetry.dat` only and closes it again (`0x5aa2d0`, at `0x5aa35c`) before the asset chain
/// exists, so an asset held only in `base.MPQ` is unreachable in the running client.
const BASE_ARCHIVES: &[&str] = &[
    "model.MPQ",
    "texture.MPQ",
    "terrain.MPQ",
    "wmo.MPQ",
    "sound.MPQ",
    "misc.MPQ",
    "interface.MPQ",
    "fonts.MPQ",
    "speech.MPQ",
    "dbc.MPQ",
];

/// The priority of base table entry `i` is `0x3f - i`: model `0x3f` down to dbc `0x36`.
const BASE_PRIORITY_TOP: i32 = 0x3f;

/// The first patch-chain priority (`mov ebx,0x40` at `0x40374e`); it increments per mounted archive:
/// `patch.MPQ` is `0x40`, then the `patch-?.MPQ` matches ascending, then `speech2.MPQ`.
const PATCH_PRIORITY_BASE: i32 = 0x40;

/// The MPQ block flag that marks a patch tombstone: the path is deleted from the composite chain
/// (`wow_mpq::BlockEntry::FLAG_DELETE_MARKER`). A tombstone is a block with this flag and a zero
/// length.
const FLAG_DELETE_MARKER: u32 = 0x0200_0000;

/// The mounted archive set: every asset request resolves through here, base then patches.
pub struct Vfs {
    chain: PatchChain,
}

impl Vfs {
    /// Mounts the client's archive set from a `Data` directory, skipping any archive that is not
    /// present, with the exe's priorities (`0x403740`):
    ///
    /// - the base archives at `0x3f` (model) down to `0x36` (dbc), an earlier table entry winning;
    /// - `patch.MPQ` at `0x40`, then every `patch-?.MPQ` (one wildcard character) ascending and
    ///   case-insensitively. The exe sorts its glob matches descending (the comparator `0x403ae0`
    ///   negates the case-insensitive compare `0x64a4c0`), appends the `patch.MPQ` match, and opens the
    ///   list backwards (`0x4037e0`), which composes to ascending, so the highest-sorting patch wins
    ///   whatever order the file system lists;
    /// - `speech2.MPQ` above the patches, which the exe mounts right after the patch chain.
    ///
    /// Not modelled: the per-patch child records with the `speech2\` prefix (`0x648e80`) and the
    /// hidden-file test of the glob (`0x42ad10`).
    pub fn mount_data_dir(dir: impl AsRef<Path>) -> Result<Vfs> {
        let dir = dir.as_ref();
        let mut chain = PatchChain::new();
        for (i, name) in BASE_ARCHIVES.iter().enumerate() {
            let p = dir.join(name);
            if p.is_file() {
                chain.add_archive(&p, BASE_PRIORITY_TOP - i as i32)?;
            }
        }
        let mut prio = PATCH_PRIORITY_BASE;
        let patch = dir.join("patch.MPQ");
        if patch.is_file() {
            chain.add_archive(&patch, prio)?;
            prio += 1;
        }
        for p in discover_patch_wildcards(dir) {
            chain.add_archive(&p, prio)?;
            prio += 1;
        }
        let speech2 = dir.join("speech2.MPQ");
        if speech2.is_file() {
            chain.add_archive(&speech2, prio)?;
        }
        Ok(Vfs { chain })
    }

    /// Reads a file by its archive path (`DBFilesClient\Startup_Strings.dbc`) from the highest-priority
    /// archive that holds it. The name is case-insensitive and takes `/` or `\`.
    pub fn read(&mut self, name: &str) -> Result<Vec<u8>> {
        self.chain.read_file(name)
    }

    /// The mounted archive a name resolves to, the winner of the priority chain, or `None`.
    pub fn which_archive(&self, name: &str) -> Option<&Path> {
        self.chain.find_file_archive(name)
    }

    /// Whether any mounted archive holds `name`.
    pub fn contains(&self, name: &str) -> bool {
        self.chain.contains_file(name)
    }

    /// The number of archives mounted.
    pub fn archive_count(&self) -> usize {
        self.chain.archive_count()
    }

    /// The deduplicated file list across the chain, the highest-priority copy of each name. It needs
    /// the archives to carry a `(listfile)`; an archive without one contributes only hash-indexed files.
    ///
    /// Tombstoned paths are dropped, and deduplication folds case and path separators, as
    /// [`Vfs::read`] resolves them. [`PatchChain::list`] takes the highest-priority listfile entry per
    /// name whatever its flags, so it lists a tombstone as if the file were present, even though a lower
    /// archive still lists real content for it. It also dedups by exact name, while the patch chain
    /// ships the same asset under different casings (`SPELLS\...` and `Spells\...`) that `read` and
    /// `contains` resolve to one file. A listing built from the raw chain therefore has more entries
    /// than there are files. The entries come back sorted by the normalized name, one per resolved,
    /// non-tombstoned file.
    ///
    /// [`Vfs::read`] on a tombstoned path returns an empty buffer, not an error: the chain serves the
    /// tombstone's zero-length block, never the lower archive's copy. [`Vfs::contains`] still reports
    /// such a path as present.
    pub fn list(&mut self) -> Result<Vec<FileEntry>> {
        let mut v = self.chain.list()?;
        v.retain(|e| e.flags & FLAG_DELETE_MARKER == 0);
        v.sort_by_key(|e| e.name.to_ascii_lowercase().replace('/', "\\"));
        v.dedup_by_key(|e| e.name.to_ascii_lowercase().replace('/', "\\"));
        Ok(v)
    }
}

/// The `patch-?.MPQ` matches in mount order, ascending and case-insensitive by name. The exe globs
/// `Data\patch-?.MPQ` (`0x82edbc`), where `?` is exactly one character, so `patch-10.MPQ` does not
/// match. Only names of the shape `patch-X.MPQ` (11 characters, one wildcard character, ASCII
/// case-insensitive) qualify; `patch.MPQ` is a separate glob (`0x82edb0`) that the caller mounts first.
fn discover_patch_wildcards(dir: &Path) -> Vec<PathBuf> {
    let mut named: Vec<(String, PathBuf)> = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    for entry in rd.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let lower = name.to_ascii_lowercase();
        let is_match = lower.len() == "patch-x.mpq".len()
            && lower.starts_with("patch-")
            && lower.ends_with(".mpq")
            && entry.path().is_file();
        if is_match {
            named.push((lower, entry.path()));
        }
    }
    named.sort();
    named.into_iter().map(|(_, p)| p).collect()
}
