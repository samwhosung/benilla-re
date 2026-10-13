//! `cargo xtask`: the repo's own tooling.

mod bodies;
mod check;
mod cited;
mod exe;
mod listing;
mod tables;
mod tree;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
usage: cargo xtask check [--exe]
       cargo xtask cited <benilla checkout>
       cargo xtask listing <GNU objdump> <output>

check         the symbol table and the system docs against their formats; runs anywhere
check --exe   also identifies WoW/WoW.exe (or $BENILLA_RE_WOW_EXE), checks every function
              row's body against dumps/disasm.txt (written when missing), regenerates the
              tables, runs every test with BENILLA_RE_REQUIRE_EXE=1, and requires each test
              the table and the docs name to pass
cited         lists the client addresses a benilla tree cites that no row or doc here backs
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
        ["cited", benilla] => cited(&root, Path::new(benilla)),
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

fn cited(root: &Path, benilla: &Path) -> std::io::Result<bool> {
    let checked = check::run(&tree::Disk::new(root))?;
    cited::run(&checked, benilla)?;
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
