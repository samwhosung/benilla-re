//! `cargo xtask site <out dir>`: the address website, plain static HTML written from the symbol
//! table, the docs and `tables/`. The same input writes the same bytes.

mod markdown;
mod model;
mod pages;
mod resolve;

use crate::check::Checked;
use crate::tree::Tree;
use markdown::Linker;
use model::Model;
use pages::Pages;
use resolve::Spans;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const STYLE: &str = include_str!("style.css");
const SCRIPT: &str = include_str!("site.js");

/// Where pages go: a directory, or memory for a test.
pub enum Sink {
    Disk {
        dir: PathBuf,
        made: HashSet<PathBuf>,
    },
    #[cfg(test)]
    Memory(std::collections::BTreeMap<String, String>),
}

impl Sink {
    pub fn disk(dir: &Path) -> Sink {
        Sink::Disk {
            dir: dir.to_path_buf(),
            made: HashSet::new(),
        }
    }

    fn put(&mut self, path: &str, content: String) -> io::Result<()> {
        match self {
            Sink::Disk { dir, made } => {
                let file = dir.join(path);
                if let Some(parent) = file.parent() {
                    if made.insert(parent.to_path_buf()) {
                        fs::create_dir_all(parent)?;
                    }
                }
                fs::write(file, content)
            }
            #[cfg(test)]
            Sink::Memory(files) => {
                files.insert(path.to_string(), content);
                Ok(())
            }
        }
    }
}

pub struct Report {
    pub pages: usize,
    pub bytes: u64,
}

/// The search and resolver index: every row by address, extent and name, and the other pages by
/// label.
fn index(model: &Model) -> String {
    let rows: Vec<serde_json::Value> = model
        .items
        .iter()
        .map(|i| serde_json::json!([i.address, i.size, i.name, u8::from(!i.is_function)]))
        .collect();
    let mut others: Vec<serde_json::Value> = Vec::new();
    for s in &model.structs {
        others.push(serde_json::json!([
            s.name,
            markdown::struct_page(&s.name),
            "struct"
        ]));
    }
    for s in &model.systems {
        others.push(serde_json::json!([s, format!("s/{s}.html"), "system"]));
    }
    for d in &model.docs {
        others.push(serde_json::json!([
            d.title,
            format!("d/{}.html", d.key),
            "doc"
        ]));
    }
    serde_json::json!({ "a": rows, "o": others }).to_string()
}

/// Writes the site into `sink`.
pub fn build(tree: &dyn Tree, checked: &Checked, sink: &mut Sink) -> io::Result<Report> {
    let model = Model::read(tree, checked)?;
    let spans = Spans::new(model.items.iter().map(|i| (i.address, i.size)));
    let pages = Pages::new(
        &model,
        Linker {
            model: &model,
            spans: &spans,
        },
    );
    let (by_system, all) = pages::count(&model);
    let mut count = 0;
    let mut bytes = 0u64;
    let mut put = |sink: &mut Sink, (path, html): (String, String)| -> io::Result<()> {
        count += usize::from(path.ends_with(".html"));
        bytes += html.len() as u64;
        sink.put(&path, html)
    };
    put(sink, pages.front(&by_system, &all))?;
    put(sink, pages.structs_index())?;
    put(sink, pages.docs_index())?;
    for item in &model.items {
        put(sink, pages.address(item))?;
    }
    for s in &model.structs {
        if let Some(page) = pages.structure(&s.name) {
            put(sink, page)?;
        }
    }
    for system in &model.systems {
        let counts = by_system.get(system);
        let empty = pages::Counts::default();
        put(sink, pages.system(system, counts.unwrap_or(&empty)))?;
    }
    for doc in &model.docs {
        put(sink, pages.doc(doc))?;
    }
    for table in &model.tables {
        put(sink, pages.table(table))?;
    }
    put(sink, ("style.css".into(), STYLE.to_string()))?;
    put(sink, ("site.js".into(), SCRIPT.to_string()))?;
    put(sink, ("index.json".into(), index(&model)))?;
    put(sink, (".nojekyll".into(), String::new()))?;
    Ok(Report {
        pages: count,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::run;
    use crate::tree::{load, Memory};
    use std::collections::BTreeMap;

    fn built() -> BTreeMap<String, String> {
        let files = load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/clean"));
        let tree = Memory(files);
        let checked = run(&tree).expect("the fixture reads");
        let mut sink = Sink::Memory(BTreeMap::new());
        build(&tree, &checked, &mut sink).expect("the site builds");
        match sink {
            Sink::Memory(files) => files,
            Sink::Disk { .. } => unreachable!(),
        }
    }

    #[test]
    fn every_row_has_its_page_at_the_stable_address() {
        let site = built();
        assert_eq!(markdown::address_page(0x402000), "a/402000.html");
        let page = &site["a/402000.html"];
        assert!(page.contains("CDemo::Advance"));
        assert!(page.contains("demo::demo_advance"));
        assert!(page.contains("Finding:"));
        for path in [
            "a/401000.html",
            "a/401100.html",
            "a/402040.html",
            "a/402100.html",
        ] {
            assert!(site.contains_key(path), "{path}");
        }
    }

    #[test]
    fn the_index_carries_every_row_with_its_extent() {
        let site = built();
        let index: serde_json::Value = serde_json::from_str(&site["index.json"]).expect("json");
        let rows = index["a"].as_array().expect("rows");
        assert!(rows
            .iter()
            .any(|r| r[0] == 0x402000 && r[1] == 64 && r[2] == "CDemo::Advance"));
    }

    #[test]
    fn the_site_is_the_same_each_time() {
        assert_eq!(built(), built());
    }

    #[test]
    fn the_front_page_and_a_system_page_state_their_coverage() {
        let site = built();
        assert!(site["index.html"].contains("named"));
        assert!(site["s/demo.html"].contains("proof two"));
        assert!(site["d/systems/demo.html"].contains("<h2 id="));
    }
}
