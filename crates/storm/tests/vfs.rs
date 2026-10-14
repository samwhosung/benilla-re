//! The mounted archive set against the install: mount `Data` and check that the VFS reads the bytes
//! the 1.12.1 client would. The archives are a fixture (`common::data_fixture`): each test skips
//! without them, and no row names these tests.

mod common;

use benilla_re_storm::Vfs;
use common::data_fixture;

/// The MPQ block flag of a patch tombstone.
const FLAG_DELETE_MARKER: u32 = 0x0200_0000;

/// The client reads DBC tables through the VFS as `DBFilesClient\<Name>.dbc`; the start-up path
/// (`0x402350`) loads `Startup_Strings.dbc` early. It must mount, resolve, and be a real WDBC blob:
/// the DBC magic and at least the 20-byte header.
#[test]
fn reads_startup_strings_dbc() {
    let Some(dir) = data_fixture() else {
        return;
    };
    let mut vfs = Vfs::mount_data_dir(&dir).expect("mount the install's Data folder");
    assert!(
        vfs.archive_count() >= 2,
        "expected the base archive set to mount, got {}",
        vfs.archive_count()
    );
    let name = r"DBFilesClient\Startup_Strings.dbc";
    assert!(vfs.contains(name), "{name} should exist in the mounted set");
    let bytes = vfs.read(name).expect("read Startup_Strings.dbc");
    assert_eq!(&bytes[..4], b"WDBC", "the WDBC magic");
    assert!(
        bytes.len() > 20,
        "at least the 20-byte WDBC header, got {}",
        bytes.len()
    );
    eprintln!(
        "Startup_Strings.dbc: {} bytes, WDBC ok, from {:?}",
        bytes.len(),
        vfs.which_archive(name)
    );
}

/// A file present in both a patch archive and a base archive resolves to the patch. The test lists a
/// base archive and a patch (the listing needs a `(listfile)`), intersects them, and checks the
/// winner is a patch archive.
#[test]
fn patch_overrides_base() {
    use std::collections::HashSet;
    use wow_mpq::Archive;

    let Some(dir) = data_fixture() else {
        return;
    };
    let list_names = |arch: &str| -> Vec<String> {
        match Archive::open(dir.join(arch)) {
            Ok(mut a) => a
                .list()
                .map(|v| v.into_iter().map(|f| f.name).collect())
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    };
    let base: HashSet<String> = [
        "dbc.MPQ",
        "interface.MPQ",
        "misc.MPQ",
        "texture.MPQ",
        "sound.MPQ",
    ]
    .iter()
    .flat_map(|&a| list_names(a))
    .map(|n| n.to_uppercase())
    .collect();
    let common = ["patch.MPQ", "patch-2.MPQ"]
        .iter()
        .flat_map(|&a| list_names(a))
        .find(|n| base.contains(&n.to_uppercase()));
    let Some(common) = common else {
        eprintln!("skip the precedence assertion: no (listfile) or no overlap in the archives");
        return;
    };
    let vfs = Vfs::mount_data_dir(&dir).expect("mount");
    let winner = vfs
        .which_archive(&common)
        .expect("the common file must resolve through the chain");
    let w = winner.to_string_lossy().to_lowercase();
    assert!(
        w.contains("patch"),
        "patch must override base for `{common}`, but it resolved to {winner:?}"
    );
    eprintln!(
        "precedence: `{common}` (present in base and patch) resolves to {winner:?}, the patch wins"
    );
}

/// `Vfs::list` serves the composite chain, not the raw union of the listfiles: a path tombstoned by a
/// higher-priority patch archive (a zero-length block flagged `FLAG_DELETE_MARKER`) does not exist in
/// the composite, and `PatchChain::list` enumerates it anyway.
///
/// A tombstone in a listing makes a census count files that do not exist. The test gates the filter
/// two ways: no listed entry carries the marker, and every listed entry reads back non-empty, since on
/// the 1.12.1 chain the tombstones are exactly the zero-length set.
#[test]
fn list_excludes_tombstoned_paths() {
    let Some(dir) = data_fixture() else {
        return;
    };
    let mut vfs = Vfs::mount_data_dir(&dir).expect("mount the install's Data folder");
    let entries = vfs.list().expect("list the MPQ VFS");
    assert!(
        entries.len() > 10_000,
        "expected a large listing, got {}: the positive control for the zero below",
        entries.len()
    );

    let marked: Vec<&str> = entries
        .iter()
        .filter(|e| e.flags & FLAG_DELETE_MARKER != 0)
        .map(|e| e.name.as_str())
        .collect();
    assert!(
        marked.is_empty(),
        "{} tombstoned path(s) survived the filter, e.g. {:?}",
        marked.len(),
        &marked[..marked.len().min(5)]
    );

    let zero_len = entries.iter().filter(|e| e.size == 0).count();
    assert_eq!(
        zero_len, 0,
        "listed entries with a zero-length block: on the 1.12.1 chain these are the tombstones the \
         filter must have removed"
    );
    eprintln!(
        "Vfs::list: {} composite entries, 0 tombstoned",
        entries.len()
    );
}

/// A tombstoned read does not fall through to the stale base copy. Patch archives delete files that
/// still hold real content in `model.MPQ`; a chain that resolved past the tombstone to the lower
/// archive would serve cut content as live. `read` returns the tombstone's own zero-length block,
/// never the base bytes. (`contains` still reports a tombstoned path as present; this test pins the
/// read.)
#[test]
fn tombstoned_read_does_not_fall_through_to_base() {
    use wow_mpq::Archive;

    let Some(dir) = data_fixture() else {
        return;
    };
    // Find a real tombstone in the patch archive (`Vfs::list` filters them) and pair it with a base
    // archive that still holds real content for the same name.
    let Ok(mut patch) = Archive::open(dir.join("patch.MPQ")) else {
        eprintln!("skip: patch.MPQ is absent");
        return;
    };
    let Ok(patch_files) = patch.list() else {
        eprintln!("skip: patch.MPQ carries no (listfile)");
        return;
    };
    let tombstones: Vec<String> = patch_files
        .iter()
        .filter(|f| f.flags & FLAG_DELETE_MARKER != 0)
        .map(|f| f.name.clone())
        .collect();
    assert!(
        !tombstones.is_empty(),
        "expected delete markers in patch.MPQ: the positive control for this test"
    );

    let mut vfs = Vfs::mount_data_dir(&dir).expect("mount the install's Data folder");
    let mut checked = 0usize;
    for base_name in ["model.MPQ", "texture.MPQ", "misc.MPQ", "interface.MPQ"] {
        let Ok(base) = Archive::open(dir.join(base_name)) else {
            continue;
        };
        for name in &tombstones {
            let Ok(Some(info)) = base.find_file(name) else {
                continue;
            };
            if info.file_size == 0 || info.flags & FLAG_DELETE_MARKER != 0 {
                continue;
            }
            // The chain must serve the tombstone, not this live-looking base content.
            let served = vfs.read(name).map(|b| b.len()).unwrap_or(0);
            assert_eq!(
                served, 0,
                "{name}: tombstoned by patch.MPQ but the chain served {served} bytes; \
                 {base_name} holds {} bytes of stale content it must not fall through to",
                info.file_size
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "no tombstone had live base content to fall through to: this test proved nothing"
    );
    eprintln!(
        "tombstone semantics: {checked} tombstoned path(s) with live base content, none served"
    );
}
