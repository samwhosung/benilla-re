//! `cargo xtask`: the repo's own tooling.

mod bodies;
mod census;
mod check;
mod cited;
mod exe;
mod export;
mod fill;
mod globals;
mod listing;
mod matcher;
mod objfiles;
mod sig;
mod signature;
mod site;
mod tables;
mod tree;
mod vendored;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
usage: cargo xtask check [--exe]
       cargo xtask census [--all | --write | --why <address> | --fill [--idb <names.tsv>] | --assign [--idb <names.tsv>]]
       cargo xtask cited <benilla checkout> [--write | --strict]
       cargo xtask vendored <benilla checkout> [--strict]
       cargo xtask listing <GNU objdump> <output>
       cargo xtask site <output directory>
       cargo xtask export <output directory> [--symbols <symbols folder>]
       cargo xtask signatures
       cargo xtask match <exe> <output directory>

check         the symbol table and the system docs against their formats; runs anywhere
check --exe   also identifies WoW/WoW.exe (or $BENILLA_RE_WOW_EXE), checks every function
              row's body against dumps/disasm.txt (written when missing), regenerates the
              tables, runs every test with BENILLA_RE_REQUIRE_EXE=1, and requires each test
              the table and the docs name to pass
census        finds every function start in .text by docs/METHOD.md, Completeness, against
              dumps/disasm.txt; prints each start with no row (its size, how it was found,
              whether live code reaches it, its class), what it turned down, and its controls;
              --all prints the rows' starts too, --why the chain that reaches an address
census --write
              adds a row for each start with none and sets each row's size to its body's; run
              tools/disasm.sh after it, and again until the census finds nothing new
census --fill gives each row --write added its system, a name, a meaning and its class;
              --idb names the community names, address<TAB>func<TAB>name lines
census --assign
              settles the system of every row that has none, by the object-file rule of
              systems/README.md (a path the function passes, an initializer run, a stretch
              between two functions of one file) and sets the class a system with a doc requires;
              prints how many rows each kind of evidence settled and how many stay unassigned
site          writes the address website into the directory, from the symbol table, the docs and
              tables/; runs anywhere, with no exe
cited         lists the client addresses a benilla tree cites that no row or doc here backs;
              --write adds a globals.tsv row for each such data address and code table;
              --strict exits nonzero when any address has neither, the standing check that
              every address benilla cites is backed here (the report alone never fails)
vendored      compares the table copies under <checkout>/reference with the tables at the
              commit its reference/PIN names: each file named like a table is `same`,
              `differs` or `missing` (the pinned commit has no such table) against
              `git show <commit>:tables/<name>` byte for byte, and the report says whether the
              pin is behind main; --strict exits nonzero when any file differs; a checkout with
              no reference/PIN is reported as having none and passes
listing       writes the disassembly of WoW/WoW.exe (or $BENILLA_RE_WOW_EXE) the findings
              cite, as tools/disasm.sh does, with that objdump
export        writes the symbol table as files other tools start from, the same bytes
              for the same tree: benilla-re.h (a C header of the structs, prototypes and
              globals), ghidra/ (the Ghidra scripts and the tables they read), ida/ and
              binja/ (a script each, applying the names, types and comments), x64dbg/ (a
              database of labels and comments), and bundle/ (the symbols/ and tables/ TSVs,
              and one benilla-re.json holding them all, and signature, the byte patterns);
              needs no exe; --symbols reads the symbol tables from a folder, as `match` writes
signatures    writes tables/signatures.tsv: each function row's masked byte pattern, from your
              WoW.exe and dumps/disasm.txt
match         finds the function rows in another build's executable sections by their patterns
              and writes match.tsv (found, missing, ambiguous or conflict per row, and how it was
              found: by pattern, by the call targets of found rows, or by order), conflicts.tsv
              and a symbols/ folder
              rebased onto that build, into the output directory; reads no 5875 exe
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let root = root();
    let result = match args.as_slice() {
        ["check"] => check(&root, false),
        ["check", "--exe"] => check(&root, true),
        ["census"] => census(&root, false, None, false),
        ["census", "--all"] => census(&root, true, None, false),
        ["census", "--write"] => census(&root, false, None, true),
        ["census", "--fill"] => fill(&root, None),
        ["census", "--fill", "--idb", names] => fill(&root, Some(Path::new(names))),
        ["census", "--assign"] => assign(&root, None),
        ["census", "--assign", "--idb", names] => assign(&root, Some(Path::new(names))),
        ["census", "--why", address] => {
            match u32::from_str_radix(address.trim_start_matches("0x"), 16) {
                Ok(address) => census(&root, false, Some(address), false),
                Err(_) => {
                    eprint!("{USAGE}");
                    return ExitCode::from(2);
                }
            }
        }
        ["cited", benilla] => cited(&root, Path::new(benilla), false, false),
        ["cited", benilla, "--write"] => cited(&root, Path::new(benilla), true, false),
        ["cited", benilla, "--strict"] => cited(&root, Path::new(benilla), false, true),
        ["vendored", benilla] => vendored::run(&root, Path::new(benilla), false),
        ["vendored", benilla, "--strict"] => vendored::run(&root, Path::new(benilla), true),
        ["export", out] => export::run(&root, Path::new(out), None),
        ["export", out, "--symbols", dir] => {
            export::run(&root, Path::new(out), Some(Path::new(dir)))
        }
        ["signatures"] => signatures(&root),
        ["match", build, out] => matcher::run(&root, Path::new(build), Path::new(out)),
        ["listing", objdump, out] => listing(&root, Path::new(objdump), Path::new(out)),
        ["site", out] => site(&root, Path::new(out)),
        _ => {
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::from(2)
        }
    }
}

/// The repo root: the nearest directory, from where the command runs upwards, that holds
/// `docs/METHOD.md` and `symbols/`. Found at run time, so a binary built in one checkout checks the
/// checkout it is run in.
fn root() -> PathBuf {
    let start = std::env::current_dir().expect("the current directory reads");
    start
        .ancestors()
        .find(|dir| dir.join("docs/METHOD.md").is_file() && dir.join("symbols").is_dir())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| {
            eprintln!(
                "xtask: no benilla-re checkout at or above {}; run it inside one",
                start.display()
            );
            std::process::exit(2)
        })
}

/// Prints one line per problem and a summary; true when there is none.
fn check(root: &Path, with_exe: bool) -> std::io::Result<bool> {
    let checked = check::run(&tree::Disk::new(root))?;
    let mut problems = checked.problems.clone();
    let mut summary = checked.summary();
    if with_exe {
        let outcome = exe::run(root, &checked)?;
        problems.extend(outcome.problems);
        summary = format!("{summary}; {}", outcome.summary);
    }
    for problem in &problems {
        println!("{problem}");
    }
    println!("{summary}: {} problems", problems.len());
    Ok(problems.is_empty())
}

fn census(root: &Path, all: bool, why: Option<u32>, write: bool) -> std::io::Result<bool> {
    let exe = exe::locate(root);
    if exe::identify(&exe)? != exe::Identity::Reference {
        eprintln!("xtask: {} is not WoW.exe 1.12.1.5875", exe.display());
        return Ok(false);
    }
    let listing_path = root.join("dumps").join("disasm.txt");
    if !listing_path.exists() {
        eprintln!("xtask: no dumps/disasm.txt; run tools/disasm.sh");
        return Ok(false);
    }
    let checked = check::run(&tree::Disk::new(root))?;
    let listing = bodies::Listing::read(&listing_path)?;
    let bytes = std::fs::read(&exe)?;
    let table = root.join("symbols").join("functions.tsv");
    census::report(
        &checked,
        &listing,
        &bytes,
        all,
        why,
        write.then_some(table.as_path()),
    )
}

fn fill(root: &Path, idb: Option<&Path>) -> std::io::Result<bool> {
    let exe = exe::locate(root);
    if exe::identify(&exe)? != exe::Identity::Reference {
        eprintln!("xtask: {} is not WoW.exe 1.12.1.5875", exe.display());
        return Ok(false);
    }
    let checked = check::run(&tree::Disk::new(root))?;
    let listing = bodies::Listing::read(&root.join("dumps").join("disasm.txt"))?;
    let bytes = std::fs::read(&exe)?;
    let rows: Vec<u32> = exe::functions(&checked).iter().map(|f| f.address).collect();
    let census = census::run(&listing, &bytes, &rows)
        .ok_or_else(|| std::io::Error::other("the census did not settle"))?;
    let names = match idb {
        Some(path) => fill::read_idb(path)?,
        None => Default::default(),
    };
    let summary = fill::fill(root, &census, &listing, &bytes, &names)?;
    for line in &summary.conflicts {
        println!("conflict\t{line}");
    }
    for line in &summary.refused {
        println!("refused\t{line}");
    }
    let show = |m: &std::collections::BTreeMap<String, usize>| {
        m.iter()
            .map(|(k, v)| format!("{v} {k}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let show_static = |m: &std::collections::BTreeMap<&str, usize>| {
        m.iter()
            .map(|(k, v)| format!("{v} {k}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!(
        "filled {} rows; {} initializer runs; systems: {}; settled by: {}; named by: {}",
        summary.filled,
        summary.runs,
        show(&summary.systems),
        show_static(&summary.sources),
        show_static(&summary.names)
    );
    Ok(true)
}

fn assign(root: &Path, idb: Option<&Path>) -> std::io::Result<bool> {
    let exe = exe::locate(root);
    if exe::identify(&exe)? != exe::Identity::Reference {
        eprintln!("xtask: {} is not WoW.exe 1.12.1.5875", exe.display());
        return Ok(false);
    }
    let checked = check::run(&tree::Disk::new(root))?;
    let listing = bodies::Listing::read(&root.join("dumps").join("disasm.txt"))?;
    let bytes = std::fs::read(&exe)?;
    let rows: Vec<u32> = exe::functions(&checked).iter().map(|f| f.address).collect();
    let census = census::run(&listing, &bytes, &rows)
        .ok_or_else(|| std::io::Error::other("the census did not settle"))?;
    let names = match idb {
        Some(path) => fill::read_idb(path)?,
        None => Default::default(),
    };
    let done = fill::assign_unassigned(root, &census, &listing, &bytes, &names)?;
    for line in &done.conflicts {
        println!("conflict\t{line}");
    }
    let settled: usize = done.settled.values().sum();
    let by = |m: &std::collections::BTreeMap<&str, usize>| {
        m.iter()
            .map(|(k, v)| format!("{v} by {k}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!(
        "{} rows had no system; settled {settled} ({}); {} stay unassigned",
        done.unassigned,
        by(&done.settled),
        done.unassigned - settled
    );
    for (system, n) in &done.systems {
        println!("  {n} {system}");
    }
    Ok(true)
}

fn signatures(root: &Path) -> std::io::Result<bool> {
    let exe = exe::locate(root);
    if exe::identify(&exe)? != exe::Identity::Reference {
        eprintln!("xtask: {} is not WoW.exe 1.12.1.5875", exe.display());
        return Ok(false);
    }
    let checked = check::run(&tree::Disk::new(root))?;
    let listing = bodies::Listing::read(&root.join("dumps").join("disasm.txt"))?;
    let text = exe::signature_table(&checked, &listing, &exe)?;
    std::fs::write(root.join("tables").join("signatures.tsv"), &text)?;
    println!("{} signatures", text.lines().count() - 1);
    Ok(true)
}

fn cited(root: &Path, benilla: &Path, write: bool, strict: bool) -> std::io::Result<bool> {
    let checked = check::run(&tree::Disk::new(root))?;
    let missing = cited::run(&checked, benilla)?;
    if write {
        let exe = exe::locate(root);
        if exe::identify(&exe)? != exe::Identity::Reference {
            eprintln!("xtask: {} is not WoW.exe 1.12.1.5875", exe.display());
            return Ok(false);
        }
        let listing = bodies::Listing::read(&root.join("dumps").join("disasm.txt"))?;
        let bytes = std::fs::read(&exe)?;
        let addresses: Vec<u32> = missing.iter().map(|&a| a as u32).collect();
        let added = globals::write(root, &checked, &listing, &bytes, &addresses)?;
        println!("added {added} rows to symbols/globals.tsv");
    }
    Ok(cited::passes(&missing, strict))
}

fn listing(root: &Path, objdump: &Path, out: &Path) -> std::io::Result<bool> {
    let exe = exe::locate(root);
    if exe::identify(&exe)? != exe::Identity::Reference {
        eprintln!(
            "xtask: {} is not WoW.exe 1.12.1.5875 (tools/exe.env)",
            exe.display()
        );
        return Ok(false);
    }
    let checked = check::run(&tree::Disk::new(root))?;
    let starts: Vec<u32> = exe::functions(&checked).iter().map(|f| f.address).collect();
    println!("{}", listing::write(&starts, &exe, objdump, out)?);
    Ok(true)
}

/// Empties `out` so no page of a deleted row survives, and refuses a folder that does not hold a
/// previous site (an `index.html`, an `index.json` and a `.nojekyll`).
fn clear_output(out: &Path) -> std::io::Result<bool> {
    if out.exists() {
        let empty = out.is_dir() && std::fs::read_dir(out)?.next().is_none();
        let previous = ["index.html", "index.json", ".nojekyll"]
            .iter()
            .all(|f| out.join(f).is_file());
        if !empty && !previous {
            eprintln!(
                "xtask: {} is not empty and does not look like a site output; give an empty or new folder",
                out.display()
            );
            return Ok(false);
        }
        if !empty {
            for entry in std::fs::read_dir(out)? {
                let path = entry?.path();
                if path.is_dir() {
                    std::fs::remove_dir_all(&path)?;
                } else {
                    std::fs::remove_file(&path)?;
                }
            }
        }
    }
    std::fs::create_dir_all(out)?;
    Ok(true)
}

fn site(root: &Path, out: &Path) -> std::io::Result<bool> {
    let tree = tree::Disk::new(root);
    let checked = check::run(&tree)?;
    if !checked.problems.is_empty() {
        eprintln!(
            "xtask: the check finds {} problems; fix them before writing the site",
            checked.problems.len()
        );
        return Ok(false);
    }
    if !clear_output(out)? {
        return Ok(false);
    }
    let report = site::build(&tree, &checked, &mut site::Sink::disk(out))?;
    println!(
        "{} pages, {} bytes of HTML, written to {}",
        report.pages,
        report.bytes,
        out.display()
    );
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("xtask-site-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_previous_site_output_is_cleared_and_another_folder_is_refused() {
        let dir = scratch("clear");
        std::fs::create_dir_all(dir.join("a")).expect("a folder");
        for f in ["index.html", "index.json", ".nojekyll", "a/old.html"] {
            std::fs::write(dir.join(f), "x").expect("a file");
        }
        assert!(clear_output(&dir).expect("clears"));
        assert!(dir.is_dir());
        assert_eq!(std::fs::read_dir(&dir).expect("reads").count(), 0);

        std::fs::write(dir.join("notes.txt"), "keep").expect("a file");
        assert!(!clear_output(&dir).expect("refuses"));
        assert!(dir.join("notes.txt").is_file());
        std::fs::remove_dir_all(&dir).expect("removes");

        assert!(clear_output(&dir).expect("makes a new folder"));
        std::fs::remove_dir_all(&dir).expect("removes");
    }
}
