//! `cargo xtask export <out dir>`: the symbol table as files other tools start from.
//!
//! - `benilla-re.h`: a C header of the structs, prototypes and globals (`header`).
//! - `ghidra/`: the Ghidra scripts `tools/ghidra/setup.sh` runs, with the tables they read.
//! - `ida/benilla_re.py`, `binja/benilla_re.py`: scripts for IDA 9 and Binary Ninja that apply
//!   the names, types and comments to the reader's own program.
//! - `bundle/`: the tables as TSV and as one `benilla-re.json` (`bundle`), their READMEs, and
//!   `signature.tsv`, each function row's byte pattern with its name (`signature.rs`).
//! - `x64dbg/WoW.exe.dd32`: the labels and comments as an x64dbg database (`x64dbg`).
//! - `docs/` and `systems/`: the Markdown a `finding` cell names, so that it resolves offline.
//! - `README.md`: where to start, from `tools/export/README.md` with the build and the commit
//!   filled in (`readme`).
//!
//! The scripts are read from `tools/`; every output is a function of the tree, so the same tree
//! writes the same bytes.

mod bundle;
pub(crate) mod cdecl;
mod header;
mod readme;
mod table;
mod x64dbg;

use crate::exe;
use header::{Origin, Report, Symbols};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
pub use table::Table;

/// The `symbols/` tables, in the order the bundle lists them.
pub const SYMBOLS: [&str; 4] = ["functions", "globals", "structs", "fields"];

/// Files of `tools/` the export copies, as `(source, output)`.
const SCRIPTS: [(&str, &str); 6] = [
    ("tools/export/README.md", "README.md"),
    ("tools/ghidra/ApplySymbols.java", "ghidra/ApplySymbols.java"),
    (
        "tools/ghidra/CreateFunctions.java",
        "ghidra/CreateFunctions.java",
    ),
    ("tools/export/ghidra-README.md", "ghidra/README.md"),
    ("tools/export/benilla_re_ida.py", "ida/benilla_re.py"),
    ("tools/export/benilla_re_binja.py", "binja/benilla_re.py"),
];

/// The repository and the site, as `benilla-re.json` and the README give them.
pub const REPO: &str = crate::site::REPO;
pub const SITE: &str = "https://samwhosung.github.io/benilla-re/";

/// Docs of the tree the export carries, as `(source, output)`: the specs, and the READMEs that
/// say what the columns of the bundle's tables mean.
const DOCS: [(&str, &str); 4] = [
    ("docs/METHOD.md", "docs/METHOD.md"),
    ("docs/CONTRIBUTING.md", "docs/CONTRIBUTING.md"),
    ("symbols/README.md", "bundle/symbols/README.md"),
    ("tables/README.md", "bundle/tables/README.md"),
];

/// What the export reads.
pub struct Sources {
    /// `symbols/<name>.tsv`, in `SYMBOLS` order.
    pub symbols: Vec<(String, String)>,
    /// `tables/<name>.tsv`, by name.
    pub tables: Vec<(String, String)>,
    /// The scripts and the README template, by output path.
    pub scripts: Vec<(String, String)>,
    /// The Markdown docs, by output path: `docs/`, `systems/` and the tables' READMEs.
    pub docs: Vec<(String, String)>,
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

/// The Markdown files under `dir` (relative to `root`), recursively, as `(path, text)` sorted by path.
fn markdown_files(root: &Path, dir: &str) -> io::Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut pending = vec![dir.to_string()];
    while let Some(folder) = pending.pop() {
        for entry in fs::read_dir(root.join(&folder))? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = format!("{folder}/{name}");
            if entry.file_type()?.is_dir() {
                pending.push(path);
            } else if name.ends_with(".md") {
                out.push((path.clone(), fs::read_to_string(root.join(&path))?));
            }
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
pub fn read(root: &Path, symbols_dir: Option<&Path>) -> Result<Sources, String> {
    let io = |e: io::Error| e.to_string();
    let mut symbols = Vec::new();
    for name in SYMBOLS {
        let dir = symbols_dir.map_or_else(|| root.join("symbols"), Path::to_path_buf);
        let path = dir.join(format!("{name}.tsv"));
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
    let mut tables = tsv_files(&root.join("tables")).map_err(io)?;
    if symbols_dir.is_some() {
        // Every other table holds 5875's addresses, which the folder's rows do not have.
        tables.clear();
    }
    let mut docs = Vec::new();
    for (source, output) in DOCS {
        if symbols_dir.is_some() && output == "bundle/tables/README.md" {
            continue;
        }
        let path = root.join(source);
        docs.push((
            output.to_string(),
            fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?,
        ));
    }
    docs.extend(markdown_files(root, "systems").map_err(io)?);
    Ok(Sources {
        symbols,
        tables,
        scripts,
        docs,
        origin: Origin {
            exe_size: exe::size(),
            exe_sha256: exe::sha256().to_string(),
            commit: commit(root)?,
            rebased: symbols_dir.is_some(),
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
        let text = if path == "README.md" {
            readme::fill(text, &sources.origin)
        } else {
            text.clone()
        };
        files.insert(path.clone(), text);
    }
    for (path, text) in &sources.docs {
        files.insert(path.clone(), text.clone());
    }
    if let Some((_, text)) = sources.tables.iter().find(|(name, _)| name == "signatures") {
        let functions = parsed.get("functions").ok_or("no functions table")?;
        files.insert(
            "bundle/signature.tsv".to_string(),
            signature_file(functions, text)?,
        );
    }
    Ok(Built {
        files,
        report: header.report,
    })
}

/// `bundle/signature.tsv`: `address`, `name`, `pattern` and `unique` of every function row, from the
/// signatures table and the names in the functions table. A row with no name is `FUN_` and its
/// address, as the other exports name it.
fn signature_file(functions: &Table, signatures: &str) -> Result<String, String> {
    let address = functions.column("address")?;
    let name = functions.column("name")?;
    let names: BTreeMap<&str, &str> = functions
        .rows
        .iter()
        .map(|r| (r[address].as_str(), r[name].as_str()))
        .collect();
    let table = Table::parse("tables/signatures.tsv", signatures)?;
    let mut out = String::from("address\tname\tpattern\tunique\n");
    for row in &table.rows {
        let Some(&n) = names.get(row[0].as_str()) else {
            continue;
        };
        let n = if n.is_empty() {
            format!("FUN_{:0>8}", row[0].trim_start_matches("0x"))
        } else {
            n.to_string()
        };
        out.push_str(&format!("{}\t{n}\t{}\t{}\n", row[0], row[2], row[1]));
    }
    Ok(out)
}

/// Writes the export under `out`.
pub fn run(root: &Path, out: &Path, symbols_dir: Option<&Path>) -> io::Result<bool> {
    let built = read(root, symbols_dir)
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
