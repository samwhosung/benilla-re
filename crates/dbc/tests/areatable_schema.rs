//! The `AreaTable.dbc` schema against the install's table. The schema is read from `AreaTableRec::Read`
//! (`0x574070`, called from the loader loop of `StaticDBLoadAll` at `0x53fe1a`, which strides
//! `add edi,0x64` at `0x53fe40`) and checked against the real table read through the archive mount.
//! The table is a fixture (`common::data_fixture`): the test skips without it, and no row names it.
//!
//! The schema, from the 22 typed-column reads (`call 0x648460`) of `0x574070` into the 100-byte record:
//!   fields  0..10  : 11 x u32                          -> struct [0x00..0x28]
//!   fields 11..19  : localized string `AreaName`       -> struct [0x2c..0x48] (8 locale pointers)
//!                    + flags column (field 19)         -> struct [0x4c]
//!   field  20      : 1 x u32                           -> struct [0x50]
//!   fields 21..24  : 4 x u32 (a 16-byte tail block)    -> struct [0x54..0x63]
//!   => 25 columns, 100-byte records.

mod common;

use benilla_re_dbc::{Dbc, LOCALES};
use benilla_re_storm::Vfs;
use common::data_fixture;

/// The `AreaTable.dbc` column schema of `0x574070` holds for the real table: `Read` decodes exactly
/// these columns.
#[test]
fn areatable_schema_matches_binary_read() {
    let Some(dir) = data_fixture() else {
        return;
    };
    let mut vfs = Vfs::mount_data_dir(&dir).expect("mount the install's Data folder");
    let bytes = vfs
        .read(r"DBFilesClient\AreaTable.dbc")
        .expect("read AreaTable.dbc");
    let dbc = Dbc::parse(bytes).expect("parse AreaTable.dbc as WDBC");

    eprintln!(
        "AreaTable.dbc: field_count={} record_size={} record_count={} string_block={}",
        dbc.field_count(),
        dbc.record_size(),
        dbc.record_count(),
        dbc.string_block_size()
    );

    // The structural schema, from the 22 typed-column reads of 0x574070.
    // 25 columns, all 4-byte -> 100-byte records (the loader's `add edi,0x64` stride, `0x53fe40`).
    assert_eq!(
        dbc.field_count(),
        25,
        "AreaTable column count (0x574070 reads 25)"
    );
    assert_eq!(
        dbc.record_size(),
        100,
        "AreaTable record size (25 × 4 = 0x64)"
    );
    assert_eq!(
        dbc.record_size(),
        dbc.field_count() * 4,
        "every AreaTable column is a 4-byte cell"
    );
    assert!(
        dbc.record_count() > 1000,
        "vanilla AreaTable has ~1081 zones"
    );

    // --- field 0 = the WowClientDB index key (id); record 0 is Dun Morogh (id 1, continent 0) ---
    assert_eq!(dbc.field_u32(0, 0), 1, "rec0 id");
    assert_eq!(
        dbc.field_u32(0, 1),
        0,
        "rec0 ContinentID = Eastern Kingdoms (map 0)"
    );
    assert_eq!(
        dbc.field_u32(0, 2),
        0,
        "rec0 ParentAreaID = 0 (top-level zone)"
    );

    // --- AreaName localized string at base field 11 (8 locale-offset cols + flags @ field 19) ---
    // The loc-string fixup at 0x57435c turns the 8 offsets into pointers via the string block.
    assert_eq!(
        dbc.loc_string(0, 11, 0).into_owned(),
        "Dun Morogh",
        "rec0 AreaName enUS (loc-string base field 11)"
    );
    assert_eq!(
        dbc.loc_string(1, 11, 0).into_owned(),
        "Longshore",
        "rec1 AreaName enUS"
    );
    // Every record's enUS AreaName resolves into the string block (no out-of-range offset).
    for rec in 0..dbc.record_count() {
        let _ = dbc.loc_string(rec, 11, 0);
        // the 7 non-enUS locale columns exist (this enUS client leaves them empty).
        for loc in 1..LOCALES.len() {
            assert!(
                dbc.loc_string(rec, 11, loc).is_empty(),
                "enUS client: locale {loc} AreaName empty at rec {rec}"
            );
        }
    }
}
