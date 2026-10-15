//! The pages: one per address, struct, system and doc, one per table, and the front page.

use super::markdown::{address_page, escape, root_for, struct_page, Linker, REPO};
use super::model::{Doc, Item, Model};
use super::resolve::Coverage;
use crate::check::{sections, slug};
use regex::Regex;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::LazyLock;

static ROOTS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?ms)^Roots: (.+?)(?:\n\n|\z)").expect("a valid pattern"));
static CITED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`(0x[0-9a-f]{6})`").expect("a valid pattern"));
static WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z_][A-Za-z0-9_:]*").expect("a valid pattern"));

/// A table with more rows than this shows only its first rows; the file has the rest.
const HEAVY_TABLE: usize = 5000;
const SHOWN_OF_HEAVY: usize = 200;
/// Rows of each kind on one page of a system.
const PER_PAGE: usize = 500;

pub mod names;

pub struct Pages<'a> {
    pub model: &'a Model,
    pub names: names::Names<'a>,
    cites: names::Cites,
    pub linker: Linker<'a>,
    struct_names: HashSet<&'a str>,
    findings: RefCell<HashMap<(String, String), String>>,
}

/// The page chrome around a body. `path` is where the page is written.
pub fn shell(path: &str, title: &str, body: &str) -> String {
    let root = root_for(path);
    format!(
        "<!doctype html>\n<html lang=\"en\" data-root=\"{root}\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{} · benilla-re</title>\n<link rel=\"stylesheet\" href=\"{root}style.css\">\n</head>\n<body>\n\
<header>\n<a class=\"brand\" href=\"{root}index.html\">benilla-re</a>\n\
<a href=\"{root}index.html#systems\">systems</a>\n<a href=\"{root}t/index.html\">structs</a>\n\
<a href=\"{root}d/index.html\">docs</a>\n\
<div class=\"search\"><input id=\"q\" type=\"search\" placeholder=\"name or 0x address\" \
autocomplete=\"off\" spellcheck=\"false\"><ul id=\"hits\"></ul></div>\n</header>\n\
<main>\n{body}</main>\n<script src=\"{root}site.js\" defer></script>\n</body>\n</html>\n",
        escape(title)
    )
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        return "–".into();
    }
    match part * 100 / whole {
        0 if part > 0 => format!("{part} (<1%)"),
        pct => format!("{part} ({pct}%)"),
    }
}

/// A coverage row. `tested_of` is the number of rows a test can cover (the functions), or
/// `None` where the kind has none.
fn coverage_row(label: &str, c: &Coverage, tested_of: Option<usize>) -> String {
    format!(
        "<tr><th>{label}</th><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
        c.rows,
        percent(c.named, c.rows),
        percent(c.verified, c.rows),
        percent(c.proof_two, c.rows),
        tested_of.map_or_else(|| "–".into(), |of| percent(c.tested, of))
    )
}

const COVERAGE_HEAD: &str = "<tr><th></th><th>rows</th><th>named</th><th>verified</th>\
<th>proof two</th><th>tested</th></tr>\n";

/// The coverage of a set of rows, by kind.
#[derive(Default)]
pub struct Counts {
    pub functions: Coverage,
    pub globals: Coverage,
    pub structs: Coverage,
    pub fields: Coverage,
}

impl Counts {
    pub fn total(&self) -> Coverage {
        let mut t = Coverage::default();
        for c in [&self.functions, &self.globals, &self.structs, &self.fields] {
            t.merge(c);
        }
        t
    }

    fn table(&self) -> String {
        format!(
            "<div class=\"scroll\"><table class=\"cov\">\n{COVERAGE_HEAD}{}{}{}{}{}</table></div>\n",
            coverage_row("functions", &self.functions, Some(self.functions.rows)),
            coverage_row("globals", &self.globals, None),
            coverage_row("structs", &self.structs, None),
            coverage_row("fields", &self.fields, None),
            coverage_row("all", &self.total(), Some(self.functions.rows)),
        )
    }
}

/// The counts for every system, and for the whole table.
pub fn count(model: &Model) -> (BTreeMap<String, Counts>, Counts) {
    let mut by_system: BTreeMap<String, Counts> = BTreeMap::new();
    let mut all = Counts::default();
    for i in &model.items {
        let named = !i.name.is_empty();
        let tested = !i.test.is_empty();
        let slot = by_system.entry(i.system.clone()).or_default();
        let (a, b) = if i.is_function {
            (&mut slot.functions, &mut all.functions)
        } else {
            (&mut slot.globals, &mut all.globals)
        };
        a.add(named, &i.confidence, &i.proof, tested);
        b.add(named, &i.confidence, &i.proof, tested);
    }
    for s in &model.structs {
        for c in [
            &mut by_system.entry(s.system.clone()).or_default().structs,
            &mut all.structs,
        ] {
            c.add(true, &s.confidence, &s.proof, false);
        }
    }
    for f in &model.fields {
        for c in [
            &mut by_system.entry(f.system.clone()).or_default().fields,
            &mut all.fields,
        ] {
            c.add(true, &f.confidence, &f.proof, false);
        }
    }
    (by_system, all)
}

impl<'a> Pages<'a> {
    pub fn new(model: &'a Model, linker: Linker<'a>) -> Pages<'a> {
        Pages {
            model,
            names: names::Names::new(model),
            cites: names::Cites::new(model),
            linker,
            struct_names: model.structs.iter().map(|s| s.name.as_str()).collect(),
            findings: RefCell::new(HashMap::new()),
        }
    }

    fn label(item: &Item) -> String {
        if item.name.is_empty() {
            format!("0x{:06x}", item.address)
        } else {
            item.name.clone()
        }
    }

    fn item_link(&self, item: &Item, root: &str) -> String {
        format!(
            "<a href=\"{root}{}\">{}</a>",
            address_page(item.address),
            escape(&Self::label(item))
        )
    }

    fn address_list(&self, addresses: &[u32], root: &str) -> String {
        let mut out = String::from("<ul class=\"cols\">\n");
        for &a in addresses {
            match self.model.item(a) {
                Some(i) => out.push_str(&format!(
                    "<li>{} <span class=\"dim\">{}</span></li>\n",
                    self.item_link(i, root),
                    escape(&i.system)
                )),
                None => out.push_str(&format!("<li><code>0x{a:06x}</code></li>\n")),
            }
        }
        out.push_str("</ul>\n");
        out
    }

    /// A type with each struct it names linked.
    fn type_html(&self, ty: &str, root: &str) -> String {
        let mut out = String::new();
        let mut last = 0;
        for w in WORD.find_iter(ty) {
            if self.struct_names.contains(w.as_str()) {
                out.push_str(&escape(&ty[last..w.start()]));
                out.push_str(&format!(
                    "<a href=\"{root}{}\">{}</a>",
                    struct_page(w.as_str()),
                    escape(w.as_str())
                ));
                last = w.end();
            }
        }
        out.push_str(&escape(&ty[last..]));
        out
    }

    fn doc_of_finding<'b>(&self, finding: &'b str) -> Option<(&'a Doc, &'b str)> {
        let (key, anchor) = finding.split_once('#')?;
        let path = if key == "METHOD" {
            "docs/METHOD.md".to_string()
        } else {
            format!("systems/{key}.md")
        };
        let doc = self.model.docs.iter().find(|d| d.path == path)?;
        Some((doc, anchor))
    }

    /// The entry a finding names, rendered, under a link to it in its doc.
    fn finding_html(&self, finding: &str, root: &str) -> String {
        if finding.is_empty() {
            return String::new();
        }
        let cache_key = (finding.to_string(), root.to_string());
        if let Some(found) = self.findings.borrow().get(&cache_key) {
            return found.clone();
        }
        let html = match self.doc_of_finding(finding) {
            None => String::new(),
            Some((doc, anchor)) => {
                let entry = sections(&doc.text)
                    .into_iter()
                    .find(|(title, _, _)| slug(title) == anchor);
                match entry {
                    None => String::new(),
                    Some((title, body, _)) => format!(
                        "<section class=\"finding\">\n<h2>Finding: <a href=\"{root}d/{}.html#{anchor}\">{}</a> \
<span class=\"dim\">in {}</span></h2>\n<div class=\"md\">{}</div>\n\
<p class=\"dim\"><a href=\"{root}d/{}.html#{anchor}\">The derivation</a> is in the doc.</p>\n</section>\n",
                        doc.key,
                        escape(title),
                        escape(&doc.title),
                        self.linker.render(&claim(body), &doc.path, root),
                        doc.key
                    ),
                }
            }
        };
        self.findings.borrow_mut().insert(cache_key, html.clone());
        html
    }

    fn finding_link(&self, finding: &str, root: &str) -> String {
        match self.doc_of_finding(finding) {
            Some((doc, anchor)) => format!(
                "<a href=\"{root}d/{}.html#{anchor}\">{}</a>",
                doc.key,
                escape(finding)
            ),
            None => escape(finding),
        }
    }

    fn tests_html(&self, test: &str) -> String {
        test.split(", ")
            .filter(|t| !t.is_empty())
            .map(|t| match self.model.tests.get(t) {
                Some((file, line)) => {
                    format!("<a href=\"{REPO}/blob/main/{file}#L{line}\"><code>{t}</code></a>")
                }
                None => format!("<code>{t}</code>"),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn proof_html(confidence: &str, proof: &str) -> String {
        if confidence == "verified" {
            format!("verified, proof {proof}")
        } else {
            confidence.to_string()
        }
    }

    pub fn address(&self, item: &Item) -> (String, String) {
        let root = "../";
        let path = address_page(item.address);
        let kind = if item.is_function {
            "function"
        } else {
            "global"
        };
        let mut b = String::new();
        b.push_str(&format!("<h1>{}</h1>\n", escape(&Self::label(item))));
        b.push_str(&format!(
            "<p class=\"sub\">{kind} · <a href=\"{root}s/{0}.html\">{0}</a>{1}</p>\n",
            escape(&item.system),
            if item.class.is_empty() {
                String::new()
            } else {
                format!(" · {}", escape(&item.class))
            }
        ));
        b.push_str("<dl>\n");
        let end = item.address.saturating_add(item.size);
        b.push_str(&format!(
            "<dt>Address</dt><dd><code>0x{:06x}</code>{}</dd>\n",
            item.address,
            if item.size > 0 {
                format!(
                    " · {} bytes, up to <code>0x{:06x}</code>",
                    item.size,
                    end.saturating_sub(1)
                )
            } else {
                String::new()
            }
        ));
        if !item.shape.is_empty() {
            let label = if item.is_function {
                "Signature"
            } else {
                "Type"
            };
            b.push_str(&format!(
                "<dt>{label}</dt><dd><code>{}</code></dd>\n",
                self.type_html(&item.shape, root)
            ));
        }
        b.push_str(&format!(
            "<dt>Meaning</dt><dd>{}</dd>\n",
            self.linker.text(&item.meaning, root)
        ));
        b.push_str(&format!(
            "<dt>Confidence</dt><dd class=\"{}\">{}</dd>\n",
            item.confidence,
            Self::proof_html(&item.confidence, &item.proof)
        ));
        if item.name_from == "idb" {
            b.push_str("<dt>Name from</dt><dd>the community database (idb)</dd>\n");
        }
        if !item.finding.is_empty() {
            b.push_str(&format!(
                "<dt>Finding</dt><dd>{}</dd>\n",
                self.finding_link(&item.finding, root)
            ));
        }
        if !item.test.is_empty() {
            b.push_str(&format!(
                "<dt>Test</dt><dd>{}</dd>\n",
                self.tests_html(&item.test)
            ));
        }
        b.push_str("</dl>\n<p id=\"at\" class=\"at\" hidden></p>\n");
        b.push_str(&self.finding_html(&item.finding, root));
        if item.is_function {
            let empty = Vec::new();
            let callers = self.model.callers.get(&item.address).unwrap_or(&empty);
            let callees = self.model.callees.get(&item.address).unwrap_or(&empty);
            b.push_str(&format!(
                "<h2>Callers <span class=\"dim\">{}</span></h2>\n",
                callers.len()
            ));
            if callers.is_empty() {
                b.push_str("<p class=\"dim\">None by a direct call or jump.</p>\n");
            } else {
                b.push_str(&self.address_list(callers, root));
            }
            b.push_str(&format!(
                "<h2>Callees <span class=\"dim\">{}</span></h2>\n",
                callees.len()
            ));
            if callees.is_empty() {
                b.push_str("<p class=\"dim\">None by a direct call or jump.</p>\n");
            } else {
                b.push_str(&self.address_list(callees, root));
            }
            b.push_str(
                "<p class=\"dim\">Calls and jumps by a bare address, from <a href=\"../tb/calls.html\">tables/calls.tsv</a>.</p>\n",
            );
        }
        let title = format!("{} 0x{:06x}", Self::label(item), item.address);
        (path.clone(), shell(&path, &title, &b))
    }

    pub fn structure(&self, name: &str) -> Option<(String, String)> {
        let s = self.model.structs.iter().find(|s| s.name == name)?;
        let path = struct_page(name);
        let root = "../";
        let mut b = format!("<h1>{}</h1>\n", escape(name));
        b.push_str(&format!(
            "<p class=\"sub\">struct · <a href=\"{root}s/{0}.html\">{0}</a> · {1} bytes</p>\n<dl>\n",
            escape(&s.system),
            escape(&s.size)
        ));
        if !s.vtable.is_empty() {
            let link = self
                .linker
                .href(super::model::hex(&s.vtable), root)
                .map_or_else(
                    || escape(&s.vtable),
                    |h| format!("<a href=\"{h}\">{}</a>", escape(&s.vtable)),
                );
            b.push_str(&format!("<dt>Vtable</dt><dd><code>{link}</code></dd>\n"));
        }
        b.push_str(&format!(
            "<dt>Meaning</dt><dd>{}</dd>\n<dt>Confidence</dt><dd class=\"{}\">{}</dd>\n",
            self.linker.text(&s.meaning, root),
            s.confidence,
            Self::proof_html(&s.confidence, &s.proof)
        ));
        match s.name_from.as_str() {
            "idb" => b.push_str("<dt>Name from</dt><dd>the community database (idb)</dd>\n"),
            "later-build" => {
                b.push_str("<dt>Name from</dt><dd>a later build's public symbols</dd>\n")
            }
            _ => {}
        }
        if !s.finding.is_empty() {
            b.push_str(&format!(
                "<dt>Finding</dt><dd>{}</dd>\n",
                self.finding_link(&s.finding, root)
            ));
        }
        b.push_str("</dl>\n");
        b.push_str(&self.finding_html(&s.finding, root));
        let fields: Vec<_> = self
            .model
            .fields
            .iter()
            .filter(|f| f.owner == name)
            .collect();
        b.push_str(&format!(
            "<h2>Fields <span class=\"dim\">{}</span></h2>\n<div class=\"scroll\"><table>\n\
<tr><th>offset</th><th>size</th><th>type</th><th>name</th><th>meaning</th><th>confidence</th><th>finding</th></tr>\n",
            fields.len()
        ));
        for f in fields {
            b.push_str(&format!(
                "<tr id=\"{}\"><td><code>{}</code></td><td>{}</td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                escape(&f.offset),
                escape(&f.offset),
                escape(&f.size),
                self.type_html(&f.ty, root),
                escape(&f.name),
                self.linker.text(&f.meaning, root),
                Self::proof_html(&f.confidence, &f.proof),
                self.finding_link(&f.finding, root)
            ));
        }
        b.push_str("</table></div>\n");
        let globals: Vec<&Item> = self
            .model
            .items
            .iter()
            .filter(|i| !i.is_function && self.type_names(&i.shape).any(|w| w == name))
            .collect();
        if !globals.is_empty() {
            b.push_str("<h2>Globals of this type</h2>\n<ul class=\"cols\">\n");
            for g in globals {
                b.push_str(&format!(
                    "<li><code>0x{:06x}</code> {}</li>\n",
                    g.address,
                    self.item_link(g, root)
                ));
            }
            b.push_str("</ul>\n");
        }
        Some((path.clone(), shell(&path, name, &b)))
    }

    fn type_names<'t>(&self, ty: &'t str) -> impl Iterator<Item = &'t str> {
        WORD.find_iter(ty).map(|m| m.as_str())
    }

    pub fn structs_index(&self) -> (String, String) {
        let path = "t/index.html".to_string();
        let mut b = format!(
            "<h1>Structs <span class=\"dim\">{}</span></h1>\n<ul class=\"cols\">\n",
            self.model.structs.len()
        );
        for s in &self.model.structs {
            b.push_str(&format!(
                "<li><a href=\"{}\">{}</a> <span class=\"dim\">{} bytes · {}</span></li>\n",
                struct_page(&s.name).trim_start_matches("t/"),
                escape(&s.name),
                escape(&s.size),
                escape(&s.system)
            ));
        }
        b.push_str("</ul>\n");
        (path.clone(), shell(&path, "Structs", &b))
    }

    pub fn doc(&self, doc: &Doc) -> (String, String) {
        let path = format!("d/{}.html", doc.key);
        let root = root_for(&path);
        let mut b = String::new();
        if !doc.system.is_empty() {
            b.push_str(&format!(
                "<p class=\"sub\">system <a href=\"{root}s/{0}.html\">{0}</a> · <a href=\"{REPO}/blob/main/{1}\">source</a></p>\n",
                escape(&doc.system),
                doc.path
            ));
        } else {
            b.push_str(&format!(
                "<p class=\"sub\"><a href=\"{REPO}/blob/main/{}\">source</a></p>\n",
                doc.path
            ));
        }
        b.push_str(&format!(
            "<div class=\"md\">{}</div>\n",
            self.linker.render(&doc.text, &doc.path, &root)
        ));
        (path.clone(), shell(&path, &doc.title, &b))
    }

    pub fn docs_index(&self) -> (String, String) {
        let path = "d/index.html".to_string();
        let mut b = String::from("<h1>Docs</h1>\n<ul>\n");
        for d in self.model.docs.iter().filter(|d| d.system.is_empty()) {
            b.push_str(&format!(
                "<li><a href=\"{}.html\">{}</a> <span class=\"dim\">{}</span></li>\n",
                d.key,
                escape(&d.title),
                d.path
            ));
        }
        b.push_str("</ul>\n<p>The system docs are listed on each <a href=\"../index.html#systems\">system</a>'s page.</p>\n");
        (path.clone(), shell(&path, "Docs", &b))
    }

    pub fn table(&self, table: &super::model::Table) -> (String, String) {
        let path = format!("tb/{}.html", table.name);
        let root = "../";
        let mut b = format!(
            "<h1>tables/{}.tsv <span class=\"dim\">{} rows</span></h1>\n\
<p class=\"sub\"><a href=\"{REPO}/blob/main/tables/{}.tsv\">source</a> · <a href=\"{root}d/tables/README.html#{}\">what the columns mean</a></p>\n",
            escape(&table.name),
            table.rows.len(),
            escape(&table.name),
            slug(&format!("{}.tsv", table.name))
        );
        let shown = if table.rows.len() > HEAVY_TABLE {
            b.push_str(&format!(
                "<p class=\"dim\">The first {SHOWN_OF_HEAVY} rows of {} are shown. The whole table is \
<a href=\"{REPO}/blob/main/tables/{}.tsv\">the file</a>.</p>\n",
                table.rows.len(),
                escape(&table.name)
            ));
            SHOWN_OF_HEAVY
        } else {
            table.rows.len()
        };
        b.push_str("<div class=\"scroll\"><table>\n<tr>");
        for h in &table.header {
            b.push_str(&format!("<th>{}</th>", escape(h)));
        }
        b.push_str("</tr>\n");
        for row in table.rows.iter().take(shown) {
            b.push_str("<tr>");
            for cell in row {
                b.push_str(&format!("<td>{}</td>", self.linker.text(cell, root)));
            }
            b.push_str("</tr>\n");
        }
        b.push_str("</table></div>\n");
        (
            path.clone(),
            shell(&path, &format!("{}.tsv", table.name), &b),
        )
    }

    /// The roots a system's docs list, in doc order.
    pub fn roots(&self, system: &str) -> Vec<u32> {
        let mut roots = Vec::new();
        for d in self.model.docs.iter().filter(|d| d.system == system) {
            for line in ROOTS.captures_iter(&d.text) {
                for a in CITED.captures_iter(&line[1]) {
                    let a = super::model::hex(&a[1]);
                    if !roots.contains(&a) {
                        roots.push(a);
                    }
                }
            }
        }
        roots
    }

    /// The functions of a system reached from its roots by direct calls and jumps, and the
    /// functions of other systems it stops at.
    pub fn closure(&self, system: &str) -> (BTreeSet<u32>, BTreeSet<u32>) {
        let owned = |a: u32| {
            self.model
                .item(a)
                .is_some_and(|i| i.is_function && i.system == system)
        };
        let mut reached = BTreeSet::new();
        let mut boundary = BTreeSet::new();
        let mut queue = VecDeque::new();
        for r in self.roots(system) {
            if owned(r) {
                if reached.insert(r) {
                    queue.push_back(r);
                }
            } else {
                boundary.insert(r);
            }
        }
        while let Some(f) = queue.pop_front() {
            for &c in self.model.callees.get(&f).into_iter().flatten() {
                if owned(c) {
                    if reached.insert(c) {
                        queue.push_back(c);
                    }
                } else {
                    boundary.insert(c);
                }
            }
        }
        (reached, boundary)
    }

    /// The pages of a system: its first page, then any further pages of rows.
    pub fn system(&self, system: &str, counts: &Counts) -> Vec<(String, String)> {
        let root = "../";
        let mut b = format!("<h1>{}</h1>\n", escape(system));
        b.push_str(&counts.table());
        let docs: Vec<&Doc> = self
            .model
            .docs
            .iter()
            .filter(|d| d.system == system)
            .collect();
        b.push_str("<h2>Docs</h2>\n");
        if docs.is_empty() {
            b.push_str("<p class=\"dim\">No doc yet; its rows are in the tables below.</p>\n");
        }
        for d in &docs {
            b.push_str(&format!(
                "<h3><a href=\"{root}d/{}.html\">{}</a></h3>\n",
                d.key,
                escape(&d.title)
            ));
            if let Some(overview) = overview(&d.text) {
                b.push_str(&format!(
                    "<div class=\"md\">{}</div>\n",
                    self.linker.render(&overview, &d.path, root)
                ));
            }
        }
        let roots = self.roots(system);
        let (reached, boundary) = self.closure(system);
        let functions: Vec<&Item> = self
            .model
            .items
            .iter()
            .filter(|i| i.system == system && i.is_function)
            .collect();
        b.push_str("<h2>Closure</h2>\n");
        if roots.is_empty() {
            b.push_str("<p class=\"dim\">The docs list no roots.</p>\n");
        } else {
            b.push_str(&format!(
                "<p>{} roots. A walk of direct calls and jumps from them reaches {} of the {} functions the system owns, \
and stops at {} functions of other systems. Vtable slots and stored addresses are not followed, so a function \
reached only that way is not counted.</p>\n",
                roots.len(),
                reached.len(),
                functions.len(),
                boundary.len()
            ));
            b.push_str("<h3>Roots</h3>\n");
            b.push_str(&self.address_list(&roots, root));
            if !boundary.is_empty() {
                b.push_str("<h3>Boundary</h3>\n");
                let boundary: Vec<u32> = boundary.into_iter().collect();
                b.push_str(&self.address_list(&boundary, root));
            }
        }
        let globals: Vec<&Item> = self
            .model
            .items
            .iter()
            .filter(|i| i.system == system && !i.is_function)
            .collect();
        let structs: Vec<_> = self
            .model
            .structs
            .iter()
            .filter(|s| s.system == system)
            .collect();
        let count = functions
            .len()
            .div_ceil(PER_PAGE)
            .max(globals.len().div_ceil(PER_PAGE))
            .max(1);
        let page_path = |n: usize| {
            if n == 1 {
                format!("s/{system}.html")
            } else {
                format!("s/{system}/{n}.html")
            }
        };
        let head = b;
        let mut out = Vec::new();
        for n in 1..=count {
            let path = page_path(n);
            let root = root_for(&path);
            let mut b = if n == 1 {
                head.clone()
            } else {
                format!(
                    "<h1>{} <span class=\"dim\">rows, page {n} of {count}</span></h1>\n",
                    escape(system)
                )
            };
            if count > 1 {
                b.push_str("<p class=\"dim\">Rows, in pages of at most 500 of each kind: ");
                for k in 1..=count {
                    if k == n {
                        b.push_str(&format!("{k} "));
                    } else {
                        b.push_str(&format!("<a href=\"{root}{}\">{k}</a> ", page_path(k)));
                    }
                }
                b.push_str("</p>\n");
            }
            let from = (n - 1) * PER_PAGE;
            let slice = |len: usize| from.min(len)..(from + PER_PAGE).min(len);
            let fs = &functions[slice(functions.len())];
            if !fs.is_empty() || n == 1 {
                b.push_str(&format!(
                    "<h2>Functions <span class=\"dim\">{}</span></h2>\n<div class=\"scroll\"><table>\n\
<tr><th>address</th><th>name</th><th>class</th><th>confidence</th><th>test</th><th>closure</th></tr>\n",
                    functions.len()
                ));
                for i in fs {
                    b.push_str(&format!(
                        "<tr><td><code>0x{:06x}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                        i.address,
                        self.item_link(i, &root),
                        escape(&i.class),
                        Self::proof_html(&i.confidence, &i.proof),
                        if i.test.is_empty() { "" } else { "✓" },
                        if reached.contains(&i.address) { "✓" } else { "" }
                    ));
                }
                b.push_str("</table></div>\n");
            }
            let gs = &globals[slice(globals.len())];
            if !gs.is_empty() {
                b.push_str(&format!(
                    "<h2>Globals <span class=\"dim\">{}</span></h2>\n<div class=\"scroll\"><table>\n\
<tr><th>address</th><th>name</th><th>type</th><th>confidence</th></tr>\n",
                    globals.len()
                ));
                for i in gs {
                    b.push_str(&format!(
                        "<tr><td><code>0x{:06x}</code></td><td>{}</td><td><code>{}</code></td><td>{}</td></tr>\n",
                        i.address,
                        self.item_link(i, &root),
                        self.type_html(&i.shape, &root),
                        Self::proof_html(&i.confidence, &i.proof)
                    ));
                }
                b.push_str("</table></div>\n");
            }
            if n == 1 && !structs.is_empty() {
                b.push_str("<h2>Structs</h2>\n<ul class=\"cols\">\n");
                for s in &structs {
                    b.push_str(&format!(
                        "<li><a href=\"{root}{}\">{}</a></li>\n",
                        struct_page(&s.name),
                        escape(&s.name)
                    ));
                }
                b.push_str("</ul>\n");
            }
            out.push((path.clone(), shell(&path, system, &b)));
        }
        out
    }

    pub fn front(&self, by_system: &BTreeMap<String, Counts>, all: &Counts) -> (String, String) {
        let path = "index.html".to_string();
        let mut b = String::from(
            "<h1>benilla-re</h1>\n<p>The reverse engineering of the World of Warcraft 1.12.1 client \
(<code>WoW.exe</code>, build 5875), by address. Search a name or an address above, or open \
<code>?0x5e6850</code> after this page's address for the row that holds it.</p>\n\
<p id=\"resolve\" class=\"at\" hidden></p>\n",
        );
        b.push_str(&self.name_links());
        b.push_str("<h2>Coverage</h2>\n");
        b.push_str(&all.table());
        b.push_str(
            "<p class=\"dim\">Named: the row has a name. Verified: read at the bytes. Proof two: two independent readings. \
Tested: a bit-exact test runs the function; the share is of the functions. <a href=\"d/docs/METHOD.html\">The method</a> says what each means.</p>\n",
        );
        b.push_str(
            "<h2 id=\"systems\">Systems</h2>\n<div class=\"scroll\"><table>\n\
<tr><th>system</th><th>rows</th><th>named</th><th>verified</th><th>proof two</th><th>tested</th><th>doc</th></tr>\n",
        );
        for (system, c) in by_system {
            let t = c.total();
            let docs = self
                .model
                .docs
                .iter()
                .filter(|d| &d.system == system)
                .count();
            b.push_str(&format!(
                "<tr><th><a href=\"s/{0}.html\">{0}</a></th><td>{1}</td><td>{2}</td><td>{3}</td><td>{4}</td><td>{5}</td><td>{6}</td></tr>\n",
                escape(system),
                t.rows,
                percent(t.named, t.rows),
                percent(t.verified, t.rows),
                percent(t.proof_two, t.rows),
                percent(t.tested, c.functions.rows),
                if docs == 0 { String::new() } else { docs.to_string() }
            ));
        }
        b.push_str("</table></div>\n<h2>Tables</h2>\n<ul class=\"cols\">\n");
        for t in &self.model.tables {
            b.push_str(&format!(
                "<li><a href=\"tb/{0}.html\">{0}</a> <span class=\"dim\">{1} rows</span></li>\n",
                escape(&t.name),
                t.rows.len()
            ));
        }
        b.push_str("</ul>\n<h2>Docs</h2>\n<ul>\n");
        for d in self.model.docs.iter().filter(|d| d.system.is_empty()) {
            b.push_str(&format!(
                "<li><a href=\"d/{}.html\">{}</a></li>\n",
                d.key,
                escape(&d.title)
            ));
        }
        b.push_str("</ul>\n");
        (
            path.clone(),
            shell(&path, "Addresses of WoW.exe 1.12.1", &b),
        )
    }
}

/// The claim of an entry: its text up to the `Addresses:` line. A section with no such line
/// (a spec doc's) gives its first paragraphs, up to about 1,200 characters.
fn claim(body: &str) -> String {
    if let Some((claim, _)) = body.split_once("\nAddresses:") {
        return claim.trim().to_string();
    }
    let mut out = String::new();
    for paragraph in body.trim().split("\n\n") {
        if !out.is_empty() && out.len() + paragraph.len() > 1200 {
            break;
        }
        out.push_str(paragraph);
        out.push_str("\n\n");
    }
    out.trim().to_string()
}

/// The overview of a doc: the text between its title and its first `Roots:` line or `##`
/// heading.
fn overview(text: &str) -> Option<String> {
    let mut out = String::new();
    let mut seen_title = false;
    for line in text.lines() {
        if !seen_title {
            seen_title = line.starts_with("# ");
            continue;
        }
        if line.starts_with("Roots:") || line.starts_with("## ") {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    let out = out.trim().to_string();
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_overview_stops_before_the_roots() {
        let text = "# demo\n\nIt does a thing.\n\nRoots: `0x401000`\n\n## Entry\nbody\n";
        assert_eq!(overview(text).as_deref(), Some("It does a thing."));
        assert_eq!(overview("# demo\n\n## Entry\n"), None);
    }

    #[test]
    fn a_claim_stops_at_the_addresses_line() {
        let body = "\nThe claim.\n\nAddresses: `0x401000`\nConfidence: verified\n";
        assert_eq!(claim(body), "The claim.");
        assert_eq!(claim("\nOne.\n\nTwo.\n"), "One.\n\nTwo.");
    }

    #[test]
    fn percent_is_whole_numbers_and_a_dash_for_nothing() {
        assert_eq!(percent(1, 3), "1 (33%)");
        assert_eq!(percent(99, 20000), "99 (<1%)");
        assert_eq!(percent(0, 5), "0 (0%)");
        assert_eq!(percent(0, 0), "–");
    }
}
