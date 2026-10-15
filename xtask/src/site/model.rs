//! What the site is built from: the symbol table, the docs, the call table and the files of the
//! repo, read once into plain structures.

use crate::check::{type_size, Checked, Row};
use crate::tree::Tree;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io;

/// A function or a global.
pub struct Item {
    pub address: u32,
    /// Bytes: a function's size, or the size of a global's type; 0 when unknown.
    pub size: u32,
    pub is_function: bool,
    pub name: String,
    pub name_from: String,
    pub system: String,
    pub class: String,
    /// A function's signature, or a global's type.
    pub shape: String,
    pub meaning: String,
    pub confidence: String,
    pub proof: String,
    pub finding: String,
    pub test: String,
}

pub struct Struct {
    pub name: String,
    pub size: String,
    pub vtable: String,
    pub system: String,
    pub meaning: String,
    pub confidence: String,
    pub proof: String,
    pub finding: String,
}

pub struct Field {
    pub owner: String,
    pub offset: String,
    pub size: String,
    pub ty: String,
    pub name: String,
    pub system: String,
    pub meaning: String,
    pub confidence: String,
    pub proof: String,
    pub finding: String,
}

/// A markdown file the site renders. `path` is the repo path, `key` the path without `.md`.
pub struct Doc {
    pub path: String,
    pub key: String,
    pub title: String,
    pub text: String,
    /// The system a `systems/` doc belongs to; empty for the others.
    pub system: String,
}

pub struct Table {
    pub name: String,
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub struct Model {
    pub items: Vec<Item>,
    pub structs: Vec<Struct>,
    pub fields: Vec<Field>,
    pub docs: Vec<Doc>,
    pub tables: Vec<Table>,
    /// The functions each function calls, from `tables/calls.tsv`.
    pub callees: BTreeMap<u32, Vec<u32>>,
    pub callers: BTreeMap<u32, Vec<u32>>,
    /// Every file of the repo, for links to the ones the site does not render.
    pub files: BTreeSet<String>,
    /// `crate::test_fn` to the file and line that define it.
    pub tests: HashMap<String, (String, usize)>,
    pub systems: BTreeSet<String>,
}

pub fn hex(cell: &str) -> u32 {
    u32::from_str_radix(cell.trim_start_matches("0x"), 16).unwrap_or(0)
}

fn doc_title(text: &str, fallback: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or(fallback)
        .trim()
        .to_string()
}

/// The spec docs the site renders besides `systems/`.
const SPEC_DOCS: [&str; 6] = [
    "README.md",
    "METHOD.md",
    "CONTRIBUTING.md",
    "symbols/README.md",
    "systems/README.md",
    "tables/README.md",
];

impl Model {
    pub fn read(tree: &dyn Tree, checked: &Checked) -> io::Result<Model> {
        let structs_by_name: HashMap<&str, &Row> =
            checked.structs.iter().map(|r| (r.get("name"), r)).collect();
        let mut items = Vec::new();
        for row in &checked.functions {
            items.push(Item {
                address: hex(row.get("address")),
                size: row.get("size").parse().unwrap_or(0),
                is_function: true,
                name: row.get("name").into(),
                name_from: row.get("name_from").into(),
                system: row.get("system").into(),
                class: row.get("class").into(),
                shape: row.get("signature").into(),
                meaning: row.get("meaning").into(),
                confidence: row.get("confidence").into(),
                proof: row.get("proof").into(),
                finding: row.get("finding").into(),
                test: row.get("test").into(),
            });
        }
        for row in &checked.globals {
            items.push(Item {
                address: hex(row.get("address")),
                size: type_size(row.get("type"), &structs_by_name)
                    .and_then(|s| u32::try_from(s).ok())
                    .unwrap_or(0),
                is_function: false,
                name: row.get("name").into(),
                name_from: row.get("name_from").into(),
                system: row.get("system").into(),
                class: String::new(),
                shape: row.get("type").into(),
                meaning: row.get("meaning").into(),
                confidence: row.get("confidence").into(),
                proof: row.get("proof").into(),
                finding: row.get("finding").into(),
                test: String::new(),
            });
        }
        items.sort_by_key(|i| (i.address, !i.is_function));
        let structs = checked
            .structs
            .iter()
            .map(|r| Struct {
                name: r.get("name").into(),
                size: r.get("size").into(),
                vtable: r.get("vtable").into(),
                system: r.get("system").into(),
                meaning: r.get("meaning").into(),
                confidence: r.get("confidence").into(),
                proof: r.get("proof").into(),
                finding: r.get("finding").into(),
            })
            .collect();
        let fields = checked
            .fields
            .iter()
            .map(|r| Field {
                owner: r.get("struct").into(),
                offset: r.get("offset").into(),
                size: r.get("size").into(),
                ty: r.get("type").into(),
                name: r.get("name").into(),
                system: r.get("system").into(),
                meaning: r.get("meaning").into(),
                confidence: r.get("confidence").into(),
                proof: r.get("proof").into(),
                finding: r.get("finding").into(),
            })
            .collect();

        let files = tree.files()?;
        let mut docs = Vec::new();
        let mut paths: Vec<&str> = SPEC_DOCS.to_vec();
        paths.extend(
            files
                .iter()
                .filter(|f| f.starts_with("systems/") && f.ends_with(".md"))
                .map(String::as_str)
                .filter(|f| *f != "systems/README.md"),
        );
        paths.sort();
        for path in paths {
            let Some(text) = tree.read(path)? else {
                continue;
            };
            let key = path.trim_end_matches(".md").to_string();
            let system = path
                .strip_prefix("systems/")
                .filter(|_| path != "systems/README.md")
                .map(|rest| {
                    rest.split('/')
                        .next()
                        .unwrap_or_default()
                        .trim_end_matches(".md")
                        .to_string()
                })
                .unwrap_or_default();
            docs.push(Doc {
                title: doc_title(&text, &key),
                path: path.into(),
                key,
                text,
                system,
            });
        }

        let mut tables = Vec::new();
        for file in &files {
            let Some(name) = file
                .strip_prefix("tables/")
                .and_then(|r| r.strip_suffix(".tsv"))
            else {
                continue;
            };
            if name.contains('/') {
                continue;
            }
            let text = tree.read(file)?.unwrap_or_default();
            let mut lines = text.lines();
            let header: Vec<String> = lines
                .next()
                .unwrap_or_default()
                .split('\t')
                .map(String::from)
                .collect();
            let rows = lines
                .map(|l| l.split('\t').map(String::from).collect())
                .collect();
            tables.push(Table {
                name: name.into(),
                header,
                rows,
            });
        }

        let mut callees: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        let mut callers: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        if let Some(table) = tables.iter().find(|t| t.name == "calls") {
            for row in &table.rows {
                if row.len() < 2 {
                    continue;
                }
                let from = hex(&row[0]);
                let to: Vec<u32> = row[1].split('|').map(hex).collect();
                for &t in &to {
                    callers.entry(t).or_default().push(from);
                }
                callees.insert(from, to);
            }
        }

        let mut tests = HashMap::new();
        for file in &files {
            let Some(rest) = file.strip_prefix("crates/") else {
                continue;
            };
            let Some((krate, tail)) = rest.split_once("/tests/") else {
                continue;
            };
            if tail.contains('/') || !tail.ends_with(".rs") {
                continue;
            }
            let text = tree.read(file)?.unwrap_or_default();
            for (n, line) in text.lines().enumerate() {
                if let Some(name) = line
                    .strip_prefix("fn ")
                    .and_then(|r| r.split('(').next())
                    .map(str::trim)
                {
                    tests
                        .entry(format!("{krate}::{name}"))
                        .or_insert((file.clone(), n + 1));
                }
            }
        }

        let mut systems: BTreeSet<String> = items.iter().map(|i| i.system.clone()).collect();
        systems.extend(structs_systems(&checked.structs));
        systems.extend(
            docs.iter()
                .filter(|d| !d.system.is_empty())
                .map(|d| d.system.clone()),
        );
        systems.remove("");
        Ok(Model {
            items,
            structs,
            fields,
            docs,
            tables,
            callees,
            callers,
            files: files.into_iter().collect(),
            tests,
            systems,
        })
    }

    pub fn item(&self, address: u32) -> Option<&Item> {
        self.items
            .binary_search_by_key(&(address, false), |i| (i.address, !i.is_function))
            .ok()
            .map(|k| &self.items[k])
            .or_else(|| {
                self.items
                    .binary_search_by_key(&(address, true), |i| (i.address, !i.is_function))
                    .ok()
                    .map(|k| &self.items[k])
            })
    }
}

fn structs_systems(rows: &[Row]) -> Vec<String> {
    rows.iter().map(|r| r.get("system").to_string()).collect()
}
