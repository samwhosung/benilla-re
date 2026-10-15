//! `cargo xtask export <out dir>`: the symbol table as files other tools start from.
//!
//! - `benilla-re.h`: a C header of the structs, prototypes and globals (`header`).
//! - `ghidra/`: the Ghidra scripts `tools/ghidra/setup.sh` runs, with the tables they read.
//! - `ida/benilla_re.py`, `binja/benilla_re.py`: scripts for IDA 9 and Binary Ninja that apply
//!   the names, types and comments to the reader's own program.
//! - `bundle/`: the tables as TSV and as one `benilla-re.json` (`bundle`).
//! - `x64dbg/WoW.exe.dd32`: the labels and comments as an x64dbg database (`x64dbg`).
//!
//! The scripts are read from `tools/`; every output is a function of the tree, so the same tree
//! writes the same bytes.

mod bundle;
mod cdecl;
mod header;
mod table;
mod x64dbg;

use crate::exe;
use header::{Origin, Report, Symbols};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use table::Table;

/// The `symbols/` tables, in the order the bundle lists them.
const SYMBOLS: [&str; 4] = ["functions", "globals", "structs", "fields"];

/// Files of `tools/` the export copies, as `(source, output)`.
const SCRIPTS: [(&str, &str); 5] = [
    ("tools/ghidra/ApplySymbols.java", "ghidra/ApplySymbols.java"),
    (
        "tools/ghidra/CreateFunctions.java",
        "ghidra/CreateFunctions.java",
    ),
    ("tools/export/ghidra-README.md", "ghidra/README.md"),
    ("tools/export/benilla_re_ida.py", "ida/benilla_re.py"),
    ("tools/export/benilla_re_binja.py", "binja/benilla_re.py"),
];

/// What the export reads.
pub struct Sources {
    /// `symbols/<name>.tsv`, in `SYMBOLS` order.
    pub symbols: Vec<(String, String)>,
    /// `tables/<name>.tsv`, by name.
    pub tables: Vec<(String, String)>,
    /// The scripts, by output path.
    pub scripts: Vec<(String, String)>,
    pub origin: Origin,
}

/// What the export writes, by path under the output directory.
pub struct Built {
    pub files: BTreeMap<String, String>,
    pub report: Report,
}

fn tsv_files(dir: &Path) -> io::Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "tsv") {
            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .expect("a file name")
                .to_string();
            out.push((name, fs::read_to_string(&path)?));
        }
    }
    out.sort();
    Ok(out)
}

/// The commit the tables are read at, with `+dirty` when a file the export reads has changed.
fn commit(root: &Path) -> Result<String, String> {
    let git = |args: &[&str]| -> Result<String, String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|e| format!("git: {e}"))?;
        if !output.status.success() {
            return Err(format!("git {}: failed", args.join(" ")));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let head = git(&["rev-parse", "HEAD"])?;
    let changed = git(&["status", "--porcelain", "--", "symbols", "tables", "tools"])?;
    Ok(if changed.is_empty() {
        head
    } else {
        format!("{head}+dirty")
    })
}

/// Reads the tree.
pub fn read(root: &Path) -> Result<Sources, String> {
    let io = |e: io::Error| e.to_string();
    let mut symbols = Vec::new();
    for name in SYMBOLS {
        let path = root.join("symbols").join(format!("{name}.tsv"));
        symbols.push((
            name.to_string(),
            fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?,
        ));
    }
    let mut scripts = Vec::new();
    for (source, output) in SCRIPTS {
        let path = root.join(source);
        scripts.push((
            output.to_string(),
            fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?,
        ));
    }
    Ok(Sources {
        symbols,
        tables: tsv_files(&root.join("tables")).map_err(io)?,
        scripts,
        origin: Origin {
            exe_size: exe::size(),
            exe_sha256: exe::sha256().to_string(),
            commit: commit(root)?,
        },
    })
}

/// Builds every output from what was read.
pub fn build(sources: &Sources) -> Result<Built, String> {
    let mut parsed: BTreeMap<&str, Table> = BTreeMap::new();
    let mut listed: Vec<(String, Table)> = Vec::new();
    let mut files = BTreeMap::new();
    for (name, text) in &sources.symbols {
        let table = Table::parse(&format!("symbols/{name}.tsv"), text)?;
        files.insert(format!("bundle/symbols/{name}.tsv"), text.clone());
        files.insert(format!("ghidra/symbols/{name}.tsv"), text.clone());
        listed.push((format!("symbols/{name}"), table.clone()));
        parsed.insert(name.as_str(), table);
    }
    for (name, text) in &sources.tables {
        let table = Table::parse(&format!("tables/{name}.tsv"), text)?;
        files.insert(format!("bundle/tables/{name}.tsv"), text.clone());
        listed.push((format!("tables/{name}"), table));
    }
    let take = |name: &str| parsed.get(name).cloned().ok_or(format!("no {name} table"));
    let symbols = Symbols {
        structs: take("structs")?,
        fields: take("fields")?,
        functions: take("functions")?,
        globals: take("globals")?,
    };
    let header = header::generate(&symbols, &sources.origin)?;
    files.insert("benilla-re.h".to_string(), header.text);
    files.insert(
        "bundle/benilla-re.json".to_string(),
        bundle::json(&sources.origin, &listed),
    );
    files.insert(
        "x64dbg/WoW.exe.dd32".to_string(),
        x64dbg::database(&symbols.functions, &symbols.globals)?,
    );
    for (path, text) in &sources.scripts {
        files.insert(path.clone(), text.clone());
    }
    Ok(Built {
        files,
        report: header.report,
    })
}

/// Writes the export under `out`.
pub fn run(root: &Path, out: &Path) -> io::Result<bool> {
    let built = read(root)
        .and_then(|sources| build(&sources))
        .map_err(io::Error::other)?;
    for (path, text) in &built.files {
        let target = out.join(path);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&target, text)?;
    }
    for (path, text) in &built.files {
        println!("{:>10}  {path}", text.len());
    }
    println!("benilla-re.h: {}", built.report.summary());
    Ok(true)
}

#[cfg(test)]
mod tests;
