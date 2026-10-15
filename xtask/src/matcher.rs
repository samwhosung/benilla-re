//! `cargo xtask match <exe> <out>`: finds the function rows of another build by their signatures
//! (`signature.rs`) and writes the symbol table rebased onto it.
//!
//! - `<out>/match.tsv`: `address`, `name`, `found`: the row's address in 5875, its name, and its
//!   address in the build, `missing` when the pattern occurs nowhere, `ambiguous` when it occurs
//!   twice or more, or when another row lands on the same address.
//! - `<out>/symbols/`: the symbol tables with each found function row at its new address and the
//!   rows not found left out. Structs and fields carry no address and are copied. Globals have
//!   no signature, so that table is empty. `cargo xtask export <dir> --symbols <out>/symbols`
//!   writes the export scripts' inputs from it.
//!
//! The build is any PE; its executable sections are searched, and nothing else of it is read.

use crate::export::Table;
use crate::signature::{self, Pe, Scanner, Signature};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Found {
    At(u32),
    Missing,
    Ambiguous,
}

/// How many rows each outcome took.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub rows: usize,
    pub found: usize,
    pub missing: usize,
    pub ambiguous: usize,
    /// Rows whose signature is unique in 5875.
    pub unique_rows: usize,
    /// Of those, the rows found.
    pub unique_found: usize,
}

impl Counts {
    pub fn summary(&self) -> String {
        format!(
            "{} rows: {} found, {} missing, {} ambiguous; {} of {} rows with a unique signature \
             found",
            self.rows,
            self.found,
            self.missing,
            self.ambiguous,
            self.unique_found,
            self.unique_rows
        )
    }
}

/// Finds every signature in the build, then drops the addresses two rows share.
pub fn locate(pe: &Pe, signatures: &[Signature]) -> Vec<Found> {
    let scanner = Scanner::new(pe);
    let mut cache: HashMap<&signature::Pattern, Found> = HashMap::new();
    let mut found: Vec<Found> = signatures
        .iter()
        .map(|s| {
            *cache
                .entry(&s.pattern)
                .or_insert_with(|| match scanner.find(&s.pattern, 2)[..] {
                    [] => Found::Missing,
                    [at] => Found::At(at),
                    _ => Found::Ambiguous,
                })
        })
        .collect();
    let mut seen: HashMap<u32, usize> = HashMap::new();
    for f in &found {
        if let Found::At(a) = f {
            *seen.entry(*a).or_default() += 1;
        }
    }
    for f in &mut found {
        if let Found::At(a) = f {
            if seen[a] > 1 {
                *f = Found::Ambiguous;
            }
        }
    }
    found
}

pub fn count(signatures: &[Signature], found: &[Found]) -> Counts {
    let mut c = Counts {
        rows: signatures.len(),
        ..Counts::default()
    };
    for (s, f) in signatures.iter().zip(found) {
        match f {
            Found::At(_) => c.found += 1,
            Found::Missing => c.missing += 1,
            Found::Ambiguous => c.ambiguous += 1,
        }
        if s.unique {
            c.unique_rows += 1;
            if matches!(f, Found::At(_)) {
                c.unique_found += 1;
            }
        }
    }
    c
}

/// What the match writes, by path under the output directory.
pub fn build(
    pe: &Pe,
    signatures: &[Signature],
    symbols: &[(String, String)],
) -> Result<(BTreeMap<String, String>, Counts), String> {
    let found = locate(pe, signatures);
    let counts = count(signatures, &found);
    let by_address: HashMap<u32, Found> = signatures
        .iter()
        .map(|s| s.address)
        .zip(found.iter().copied())
        .collect();
    let mut files = BTreeMap::new();
    let mut report = String::from("address\tname\tfound\n");
    for (name, text) in symbols {
        let table = Table::parse(name, text)?;
        let mut out = table.columns.join("\t");
        out.push('\n');
        if name == "functions" {
            let a = table.column("address")?;
            let n = table.column("name")?;
            let mut moved: Vec<(u32, String)> = Vec::new();
            for row in &table.rows {
                let old = u32::from_str_radix(row[a].trim_start_matches("0x"), 16)
                    .map_err(|_| format!("functions: bad address {}", row[a]))?;
                let Some(&f) = by_address.get(&old) else {
                    continue;
                };
                let shown = match f {
                    Found::At(at) => format!("{at:#08x}"),
                    Found::Missing => "missing".to_string(),
                    Found::Ambiguous => "ambiguous".to_string(),
                };
                let _ = writeln!(report, "{}\t{}\t{shown}", row[a], row[n]);
                if let Found::At(at) = f {
                    let mut row = row.clone();
                    row[a] = format!("{at:#08x}");
                    moved.push((at, row.join("\t")));
                }
            }
            moved.sort();
            for (_, line) in &moved {
                out.push_str(line);
                out.push('\n');
            }
        } else if name != "globals" {
            out = text.clone();
        }
        files.insert(format!("symbols/{name}.tsv"), out);
    }
    files.insert("match.tsv".to_string(), report);
    let _ = pe;
    Ok((files, counts))
}

/// Reads `tables/signatures.tsv` and the symbol tables of `root`, matches them against the exe at
/// `exe`, writes under `out`, and prints the counts.
pub fn run(root: &Path, exe: &Path, out: &Path) -> std::io::Result<bool> {
    let io = std::io::Error::other;
    let bytes = fs::read(exe)?;
    let pe = Pe::parse(&bytes).map_err(|e| io(format!("{}: {e}", exe.display())))?;
    let signatures =
        signature::parse(&fs::read_to_string(root.join("tables/signatures.tsv"))?).map_err(io)?;
    let mut symbols = Vec::new();
    for name in crate::export::SYMBOLS {
        symbols.push((
            name.to_string(),
            fs::read_to_string(root.join("symbols").join(format!("{name}.tsv")))?,
        ));
    }
    let (files, counts) = build(&pe, &signatures, &symbols).map_err(io)?;
    for (path, text) in &files {
        let target = out.join(path);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(target, text)?;
    }
    println!("{}", counts.summary());
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bodies::Listing;
    use crate::exe;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask sits in the repo")
            .to_path_buf()
    }

    /// The reader's exe, or `None` to skip; with `BENILLA_RE_REQUIRE_EXE` set, a missing one
    /// panics.
    fn reference() -> Option<Vec<u8>> {
        let path = exe::locate(&root());
        match exe::identify(&path) {
            Ok(exe::Identity::Reference) => Some(std::fs::read(&path).expect("the exe reads")),
            _ => {
                assert!(
                    std::env::var_os(exe::REQUIRE_VARIABLE).is_none(),
                    "{} is not WoW.exe 1.12.1.5875",
                    path.display()
                );
                println!("skipped: no WoW.exe 1.12.1.5875");
                None
            }
        }
    }

    fn committed() -> Vec<Signature> {
        signature::parse(&std::fs::read_to_string(root().join("tables/signatures.tsv")).unwrap())
            .expect("the table parses")
    }

    fn symbols() -> Vec<(String, String)> {
        crate::export::SYMBOLS
            .iter()
            .map(|n| {
                (
                    n.to_string(),
                    std::fs::read_to_string(root().join(format!("symbols/{n}.tsv"))).unwrap(),
                )
            })
            .collect()
    }

    /// The file offset of a `.text` address: the section's raw data starts at its address.
    fn at(address: u32) -> usize {
        (address - 0x400000) as usize
    }

    #[test]
    fn every_unique_signature_finds_its_own_row_in_5875() {
        let Some(bytes) = reference() else { return };
        let pe = Pe::parse(&bytes).unwrap();
        let signatures = committed();
        let found = locate(&pe, &signatures);
        let counts = count(&signatures, &found);
        assert_eq!(counts.unique_found, counts.unique_rows);
        assert!(counts.unique_rows > 10_000);
        assert_eq!(counts.missing, 0);
        for (s, f) in signatures.iter().zip(&found) {
            if s.unique {
                assert_eq!(*f, Found::At(s.address), "{:#08x}", s.address);
            } else {
                assert_eq!(*f, Found::Ambiguous, "{:#08x}", s.address);
            }
        }
    }

    #[test]
    fn a_patched_swapped_and_copied_build_shows_the_expected_counts() {
        let Some(mut bytes) = reference() else { return };
        let signatures = committed();
        let sizes: HashMap<u32, u32> = {
            let functions = symbols();
            let t = Table::parse("functions", &functions[0].1).unwrap();
            t.rows
                .iter()
                .map(|r| {
                    (
                        u32::from_str_radix(&r[0][2..], 16).unwrap(),
                        r[1].parse().unwrap(),
                    )
                })
                .collect()
        };
        let before = {
            let pe = Pe::parse(&bytes).unwrap();
            assert_eq!(pe.sections[0].va, 0x401000);
            count(&signatures, &locate(&pe, &signatures))
        };
        // Long unique rows that no other row's patch touches.
        let long: Vec<&Signature> = signatures
            .iter()
            .filter(|s| s.unique && s.pattern.len() >= 24 && sizes[&s.address] >= 48)
            .collect();
        let pick = |n: usize| long[n * long.len() / 7];
        let (patched, swapped_a, swapped_b, copied, overwritten) =
            (pick(1), pick(2), pick(3), pick(4), pick(5));
        // Swapped bodies must have the same length.
        let twin = long
            .iter()
            .find(|s| {
                s.address != swapped_a.address && sizes[&s.address] == sizes[&swapped_a.address]
            })
            .copied()
            .unwrap_or(swapped_b);
        assert_eq!(sizes[&twin.address], sizes[&swapped_a.address]);

        // Patched: a fixed byte of the pattern changes, so the row is not found.
        let fixed = patched.pattern.iter().position(Option::is_some).unwrap() + 3;
        let p = at(patched.address) + fixed;
        bytes[p] ^= 0xff;
        // Swapped: two bodies exchange places, so each row is found at the other's address.
        let size = sizes[&swapped_a.address] as usize;
        let (x, y) = (at(swapped_a.address), at(twin.address));
        for i in 0..size {
            bytes.swap(x + i, y + i);
        }
        // Copied: one row's pattern bytes overwrite the start of another row's body, so the first
        // occurs twice and the second is not found.
        let source: Vec<u8> = bytes[at(copied.address)..][..copied.pattern.len()].to_vec();
        let d = at(overwritten.address);
        bytes[d..d + source.len()].copy_from_slice(&source);

        let pe = Pe::parse(&bytes).unwrap();
        let found = locate(&pe, &signatures);
        let after = count(&signatures, &found);
        let result = |s: &Signature| found[signatures.iter().position(|t| t == s).unwrap()];
        assert_eq!(result(patched), Found::Missing);
        assert_eq!(result(swapped_a), Found::At(twin.address));
        assert_eq!(result(twin), Found::At(swapped_a.address));
        assert_eq!(result(copied), Found::Ambiguous);
        assert_eq!(result(overwritten), Found::Missing);
        assert_eq!(after.missing, before.missing + 2);
        assert_eq!(after.ambiguous, before.ambiguous + 1);
        assert_eq!(after.found, before.found - 3);
        assert_eq!(after.rows, before.rows);

        // The rebased tables hold the swapped rows at their new addresses and no missing row.
        let (files, _) = build(&pe, &signatures, &symbols()).unwrap();
        let moved = Table::parse("functions", &files["symbols/functions.tsv"]).unwrap();
        let rows: HashMap<&str, &Vec<String>> =
            moved.rows.iter().map(|r| (r[0].as_str(), r)).collect();
        assert!(rows.contains_key(format!("{:#08x}", twin.address).as_str()));
        assert_eq!(moved.rows.len(), after.found);
        let report = &files["match.tsv"];
        let line = |s: &Signature| {
            let start = format!("{:#08x}\t", s.address);
            report
                .lines()
                .find(|l| l.starts_with(&start))
                .unwrap()
                .rsplit('\t')
                .next()
                .unwrap()
                .to_string()
        };
        assert_eq!(line(patched), "missing");
        assert_eq!(line(copied), "ambiguous");
        assert_eq!(line(swapped_a), format!("{:#08x}", twin.address));
    }

    #[test]
    fn the_rebased_tables_of_5875_import_with_the_export_scripts() {
        let Some(bytes) = reference() else { return };
        let pe = Pe::parse(&bytes).unwrap();
        let (files, counts) = build(&pe, &committed(), &symbols()).unwrap();
        assert_eq!(counts.unique_found, counts.unique_rows);
        let dir = std::env::temp_dir().join(format!("xtask-match-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (path, text) in &files {
            let target = dir.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, text).unwrap();
        }
        let sources = crate::export::read(&root(), Some(&dir.join("symbols"))).unwrap();
        let built = crate::export::build(&sources).unwrap();
        assert!(!built.files.contains_key("bundle/signature"));
        assert!(built.files["benilla-re.h"].contains("struct"));
        let out = dir.join("export");
        for (path, text) in &built.files {
            let target = out.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, text).unwrap();
        }
        if std::process::Command::new("python3")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
        {
            let run = std::process::Command::new("python3")
                .arg("-I")
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/export/scripts_test.py"))
                .arg(&out)
                .output()
                .unwrap();
            let said = String::from_utf8_lossy(&run.stdout);
            assert!(
                run.status.success() && said.contains("IDA ok") && said.contains("Binja ok"),
                "{said}{}",
                String::from_utf8_lossy(&run.stderr)
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_same_rows_write_the_same_signatures() {
        let Some(bytes) = reference() else { return };
        let listing_path = root().join("dumps/disasm.txt");
        if !listing_path.exists() {
            assert!(
                std::env::var_os(exe::REQUIRE_VARIABLE).is_none(),
                "no dumps/disasm.txt"
            );
            println!("skipped: no dumps/disasm.txt");
            return;
        }
        let listing = Listing::read(&listing_path).unwrap();
        let pe = Pe::parse(&bytes).unwrap();
        let committed = committed();
        // A spread of rows, twice: the output is the same and is the committed one.
        let rows: Vec<(u32, u32)> = {
            let t = Table::parse("functions", &symbols()[0].1).unwrap();
            t.rows
                .iter()
                .step_by(23)
                .map(|r| {
                    (
                        u32::from_str_radix(&r[0][2..], 16).unwrap(),
                        r[1].parse().unwrap(),
                    )
                })
                .collect()
        };
        let a = signature::generate(&pe, &listing, &rows).unwrap();
        let b = signature::generate(&pe, &listing, &rows).unwrap();
        assert_eq!(signature::table(&a), signature::table(&b));
        for s in &a {
            let c = committed.iter().find(|c| c.address == s.address).unwrap();
            assert_eq!(s, c, "{:#08x}", s.address);
        }
    }
}
