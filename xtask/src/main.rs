//! `cargo xtask`: the repo's own tooling.

mod bodies;
mod census;
mod check;
mod cited;
mod exe;
mod fill;
mod globals;
mod listing;
mod objfiles;
mod tables;
mod tree;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
usage: cargo xtask check [--exe]
       cargo xtask census [--all | --write | --why <address> | --fill [--idb <names.tsv>]]
       cargo xtask cited <benilla checkout> [--write]
       cargo xtask listing <GNU objdump> <output>

check         the symbol table and the system docs against their formats; runs anywhere
check --exe   also identifies WoW/WoW.exe (or $BENILLA_RE_WOW_EXE), checks every function
              row's body against dumps/disasm.txt (written when missing), regenerates the
              tables, runs every test with BENILLA_RE_REQUIRE_EXE=1, and requires each test
              the table and the docs name to pass
census        finds every function start in .text by METHOD.md, Completeness, against
              dumps/disasm.txt; prints each start with no row (its size, how it was found,
              whether live code reaches it, its class), what it turned down, and its controls;
              --all prints the rows' starts too, --why the chain that reaches an address
census --write
              adds a row for each start with none and sets each row's size to its body's; run
              tools/disasm.sh after it, and again until the census finds nothing new
census --fill gives each row --write added its system, a name, a meaning and its class;
              --idb names the community names, address<TAB>func<TAB>name lines
cited         lists the client addresses a benilla tree cites that no row or doc here backs;
              --write adds a globals.tsv row for each such data address and code table
listing       writes the disassembly of WoW/WoW.exe (or $BENILLA_RE_WOW_EXE) the findings
              cite, as tools/disasm.sh does, with that objdump
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
        ["census", "--why", address] => {
            match u32::from_str_radix(address.trim_start_matches("0x"), 16) {
                Ok(address) => census(&root, false, Some(address), false),
                Err(_) => {
                    eprint!("{USAGE}");
                    return ExitCode::from(2);
                }
            }
        }
        ["cited", benilla] => cited(&root, Path::new(benilla), false),
        ["cited", benilla, "--write"] => cited(&root, Path::new(benilla), true),
        ["listing", objdump, out] => listing(&root, Path::new(objdump), Path::new(out)),
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
/// `METHOD.md` and `symbols/`. Found at run time, so a binary built in one checkout checks the
/// checkout it is run in.
fn root() -> PathBuf {
    let start = std::env::current_dir().expect("the current directory reads");
    start
        .ancestors()
        .find(|dir| dir.join("METHOD.md").is_file() && dir.join("symbols").is_dir())
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

fn cited(root: &Path, benilla: &Path, write: bool) -> std::io::Result<bool> {
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
    Ok(true)
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
