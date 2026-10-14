//! The DBC schema table against the install's tables, for all 151 client DBC types.
//!
//! `tables/dbc_schemas.tsv` holds, for each table, the field count and record size from the sum of
//! its `T::Read`'s typed-column reads (`call 0x648460`) and the string columns: the field indices
//! read into a stack temporary and fixed up with `+stringBlockBase`. A run of 8 is a localized
//! string; a single is a `char *`. Both tests check that table against the real
//! `DBFilesClient\*.dbc` read through the archive mount. The tables are a fixture
//! (`common::data_fixture`): each test skips without them, and no row names these tests.

mod common;

use benilla_re_dbc::Dbc;
use benilla_re_storm::Vfs;
use common::data_fixture;

struct Entry {
    file: String,
    field_count: u32,
    record_size: u32,
    string_cols: Vec<u32>,
}

/// The rows of `tables/dbc_schemas.tsv`: file, field count, record size, string columns, fix-ups.
fn catalog() -> Vec<Entry> {
    let path = benilla_re_difftest::repo_root()
        .join("tables")
        .join("dbc_schemas.tsv");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            Entry {
                file: c[0].to_string(),
                field_count: c[1].parse().unwrap_or(0),
                record_size: c[2].parse().unwrap_or(0),
                string_cols: c
                    .get(3)
                    .copied()
                    .unwrap_or("")
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.parse().unwrap())
                    .collect(),
            }
        })
        .collect()
}

/// Structural layer: every table's `(field_count, record_size)` in the schema table matches its real
/// `.dbc` header.
#[test]
fn dbc_schema_catalog_matches_real_tables() {
    let Some(dir) = data_fixture() else {
        return;
    };
    let mut vfs = Vfs::mount_data_dir(&dir).expect("mount the install's Data folder");

    let mut validated = 0usize;
    let (mut absent, mut mismatches): (Vec<String>, Vec<String>) = (vec![], vec![]);

    for e in catalog() {
        let Ok(bytes) = vfs.read(&format!(r"DBFilesClient\{}.dbc", e.file)) else {
            absent.push(e.file.clone());
            continue;
        };
        let dbc = match Dbc::parse(bytes) {
            Ok(d) => d,
            Err(err) => {
                mismatches.push(format!("{}: parse failed: {err}", e.file));
                continue;
            }
        };
        if dbc.field_count() != e.field_count || dbc.record_size() != e.record_size {
            mismatches.push(format!(
                "{}: table {}c/{}B, real {}c/{}B",
                e.file,
                e.field_count,
                e.record_size,
                dbc.field_count(),
                dbc.record_size()
            ));
        } else {
            validated += 1;
        }
    }
    eprintln!(
        "DBC structural: {validated} validated, {} absent, {} MISMATCH",
        absent.len(),
        mismatches.len()
    );
    for m in &mismatches {
        eprintln!("  MISMATCH {m}");
    }
    assert!(
        mismatches.is_empty(),
        "{} schema mismatches",
        mismatches.len()
    );
}

/// Type layer: every RE'd string column holds a valid string-block offset in every record of the
/// real table — corroborating the exe's `+stringBlockBase` fix-ups. A scalar column wrongly listed as a string
/// would carry out-of-range values (e.g. AreaTable's flags = 4128894 ≫ its 15838-byte block).
#[test]
fn dbc_string_columns_resolve() {
    let Some(dir) = data_fixture() else {
        return;
    };
    let mut vfs = Vfs::mount_data_dir(&dir).expect("mount the install's Data folder");

    let mut tables = 0usize;
    let mut cols_checked = 0usize;
    let mut nonempty = 0usize;
    let mut bad: Vec<String> = Vec::new();

    for e in catalog() {
        if e.string_cols.is_empty() {
            continue;
        }
        let Ok(bytes) = vfs.read(&format!(r"DBFilesClient\{}.dbc", e.file)) else {
            continue;
        };
        let dbc = Dbc::parse(bytes).expect("parse");
        let block = dbc.string_block_size();
        tables += 1;
        for &col in &e.string_cols {
            if col >= dbc.field_count() {
                bad.push(format!("{}: string col {col} >= field_count", e.file));
                continue;
            }
            cols_checked += 1;
            for rec in 0..dbc.record_count() {
                let off = dbc.field_u32(rec, col);
                if off >= block {
                    bad.push(format!(
                        "{} col {col} rec {rec}: offset {off} >= string block {block}",
                        e.file
                    ));
                    break;
                }
                if off != 0 && !dbc.string_bytes(off).is_empty() {
                    nonempty += 1;
                }
            }
        }
    }
    eprintln!(
        "DBC string cols: {tables} tables, {cols_checked} string columns all in-range, {nonempty} non-empty cells resolved, {} bad",
        bad.len()
    );
    // a couple of resolved samples for visibility
    if let Ok(bytes) = vfs.read(r"DBFilesClient\Map.dbc") {
        let dbc = Dbc::parse(bytes).unwrap();
        eprintln!(
            "  sample Map.dbc rec0: Directory(char* @1)={:?}  MapName(loc @4 enUS)={:?}",
            dbc.string_field(0, 1),
            dbc.loc_string(0, 4, 0)
        );
    }
    for b in bad.iter().take(20) {
        eprintln!("  BAD {b}");
    }
    assert!(bad.is_empty(), "{} string-column anomalies", bad.len());
}
