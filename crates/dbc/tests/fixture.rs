//! The reader against the install's tables: parse the real `DBFilesClient\*.dbc` tables the 1.12.1
//! client loads (read through the archive mount of `benilla-re-storm`) and check the reader reproduces
//! the WDBC container the client sees. The tables are a fixture (`common::data_fixture`): each test
//! skips without them, and no row names these tests.

mod common;

use benilla_re_dbc::{Dbc, LOCALES};
use benilla_re_storm::Vfs;
use common::data_fixture;

/// `Startup_Strings.dbc` is the first DBC the start-up path (`0x402350`) loads, 11 fields and 44-byte
/// records. Parse it through the VFS, check the header constants, and resolve a localized
/// string (proving the string block + the locale-column selection work against real data).
#[test]
fn parses_startup_strings() {
    let Some(dir) = data_fixture() else {
        return;
    };
    let mut vfs = Vfs::mount_data_dir(&dir).expect("mount ./WoW/Data");
    let bytes = vfs
        .read(r"DBFilesClient\Startup_Strings.dbc")
        .expect("read Startup_Strings.dbc");
    let dbc = Dbc::parse(bytes).expect("parse Startup_Strings.dbc as WDBC");

    // The container constants for this table, as the exe's inline parse strides them.
    assert_eq!(dbc.field_count(), 11, "Startup_Strings field count");
    assert_eq!(
        dbc.record_size(),
        44,
        "Startup_Strings record size (11 × 4)"
    );
    assert!(dbc.record_count() > 0, "Startup_Strings has rows");
    assert!(
        dbc.string_block_size() > 1,
        "Startup_Strings has a string block"
    );

    // Localized strings: the per-locale offset columns begin at field 2 (the exe's
    // `record + locale*4 + 8` pick at `0x4027c0`). enUS (locale 0) must resolve to a non-empty string.
    let en = dbc.loc_string(0, 2, 0).into_owned();
    assert!(
        !en.is_empty(),
        "record 0 enUS loc-string (field 2) should be non-empty"
    );
    let other = LOCALES
        .iter()
        .enumerate()
        .map(|(i, loc)| format!("{loc}={:?}", dbc.loc_string(0, 2, i)))
        .collect::<Vec<_>>()
        .join("  ");
    eprintln!(
        "Startup_Strings: {} records × {} fields ({} B); record 0 loc strings: {other}",
        dbc.record_count(),
        dbc.field_count(),
        dbc.record_size()
    );
}

/// A large real table exercising the generic container path: `Spell.dbc`. The size invariant in
/// `Dbc::parse` (`20 + records*record_size + string_block == file len`) validates the layout; assert
/// it has the thousands of rows / many columns a vanilla spell table does.
#[test]
fn parses_spell_dbc_generic_path() {
    let Some(dir) = data_fixture() else {
        return;
    };
    let mut vfs = Vfs::mount_data_dir(&dir).expect("mount");
    let bytes = vfs
        .read(r"DBFilesClient\Spell.dbc")
        .expect("read Spell.dbc");
    let dbc = Dbc::parse(bytes).expect("parse Spell.dbc as WDBC (size invariant holds)");
    assert!(
        dbc.record_count() > 100,
        "Spell.dbc should have thousands of spells, got {}",
        dbc.record_count()
    );
    assert!(dbc.field_count() > 10, "Spell.dbc is a wide table");
    // field 0 is the spell id (the WowClientDB index key); record 0's id is a small positive number.
    let id0 = dbc.field_u32(0, 0);
    assert!(id0 > 0, "Spell.dbc record 0 id should be set");
    eprintln!(
        "Spell.dbc: {} records × {} fields ({} B each), id[0]={id0}, string block {} B",
        dbc.record_count(),
        dbc.field_count(),
        dbc.record_size(),
        dbc.string_block_size()
    );
}
