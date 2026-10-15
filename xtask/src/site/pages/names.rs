//! The name-first pages: a page for each Lua function, event, CVar and opcode, and an index of
//! each. Their URLs come from the name alone (`lua/<name>.html`, `event/<NAME>.html`,
//! `cvar/<name>.html`, `op/<0x...>.html`).

use super::{shell, Pages};
use crate::check::{sections, slug};
use crate::site::markdown::escape;
use crate::site::model::{hex, Model};
use regex::Regex;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

static TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b0x(?:[0-9a-fA-F]{6}|[0-9a-fA-F]{3})\b").expect("a valid pattern")
});

/// Entries listed under a page; the rest are counted.
const CITES_SHOWN: usize = 40;

/// A table read by column name. A column the table lacks reads as empty.
pub struct View<'a> {
    header: &'a [String],
    rows: &'a [Vec<String>],
}

impl<'a> View<'a> {
    fn of(model: &'a Model, name: &str) -> Option<View<'a>> {
        let t = model.tables.iter().find(|t| t.name == name)?;
        Some(View {
            header: &t.header,
            rows: &t.rows,
        })
    }

    fn get<'r>(&self, row: &'r [String], column: &str) -> &'r str {
        self.header
            .iter()
            .position(|h| h == column)
            .and_then(|i| row.get(i))
            .map_or("", String::as_str)
    }

    fn has(&self, column: &str) -> bool {
        self.header.iter().any(|h| h == column)
    }
}

/// The rows of one name, and the file stem of its page.
pub struct Group<'a> {
    pub name: &'a str,
    pub stem: String,
    pub rows: Vec<&'a Vec<String>>,
}

/// A name as a file stem: letters, digits and `_-.` stay, any other byte is `%XX`; `index` is
/// `index~` so it never meets an index page.
fn stem_of(name: &str) -> String {
    let mut out = String::new();
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    if out == "index" {
        out.push('~');
    }
    out
}

/// Groups the rows by the `column`, in name order. Names that differ only by case would share
/// a file on a case-insensitive disk: each of those that has a capital writes it `~` and the
/// lowercase letter (`Lower` is `~lower`), and the all-lowercase one keeps its plain name.
fn group<'a>(
    view: &View<'a>,
    column: &str,
    keep: impl Fn(&str) -> bool,
    fold: bool,
) -> Vec<Group<'a>> {
    let mut by_name: BTreeMap<&'a str, Vec<&'a Vec<String>>> = BTreeMap::new();
    for row in view.rows {
        let name = view.get(row, column);
        if !name.is_empty() && keep(name) {
            by_name.entry(name).or_default().push(row);
        }
    }
    let mut twins: HashMap<String, usize> = HashMap::new();
    for name in by_name.keys() {
        *twins.entry(stem_of(name).to_lowercase()).or_default() += 1;
    }
    by_name
        .into_iter()
        .map(|(name, rows)| {
            let plain = stem_of(name);
            let stem = if fold && twins[&plain.to_lowercase()] > 1 {
                plain
                    .chars()
                    .map(|c| {
                        if c.is_ascii_uppercase() {
                            format!("~{}", c.to_ascii_lowercase())
                        } else {
                            c.to_string()
                        }
                    })
                    .collect()
            } else {
                plain
            };
            Group { name, stem, rows }
        })
        .collect()
}

/// The groups of each kind of name, from the tables that have them.
pub struct Names<'a> {
    pub lua: Option<(View<'a>, Vec<Group<'a>>)>,
    pub event: Option<(View<'a>, Vec<Group<'a>>)>,
    pub cvar: Option<(View<'a>, Vec<Group<'a>>)>,
    pub op: Option<(View<'a>, Vec<Group<'a>>)>,
    /// The opcodes the client sends and registers no handler for, from `opcode_sends`.
    pub sent: Option<(View<'a>, Vec<Group<'a>>)>,
    sends: Option<View<'a>>,
    entries: Option<View<'a>>,
    sites: Option<View<'a>>,
}

impl<'a> Names<'a> {
    pub fn new(model: &'a Model) -> Names<'a> {
        let make = |table: &str, column: &str, keep: &dyn Fn(&str) -> bool, fold: bool| {
            let view = View::of(model, table)?;
            let groups = group(&view, column, keep, fold);
            Some((view, groups))
        };
        let op = make("opcodes", "opcode", &|_| true, false);
        let handled: HashSet<&str> = op
            .iter()
            .flat_map(|(_, groups)| groups.iter().map(|g| g.name))
            .collect();
        let sent = make(
            "opcode_sends",
            "opcode",
            &|n| n != "?" && !handled.contains(n),
            false,
        );
        Names {
            lua: make("bindings", "name", &|_| true, true),
            event: make("events", "name", &|_| true, true),
            cvar: make("cvars", "name", &|n| n != "?", true),
            op,
            sent,
            sends: View::of(model, "opcode_sends"),
            entries: View::of(model, "opcode_entries"),
            sites: View::of(model, "event_sites"),
        }
    }

    /// `(label, path, kind)` for the search index.
    pub fn search(&self) -> Vec<(String, String, &'static str)> {
        let mut out = Vec::new();
        let mut add = |part: &Option<(View, Vec<Group>)>, dir: &str, kind: &'static str| {
            if let Some((view, groups)) = part {
                for g in groups {
                    let mut label = g.name.to_string();
                    if dir == "op" {
                        let name = view.get(g.rows[0], "name");
                        if !name.is_empty() {
                            label = format!("{label} {name}");
                        }
                    }
                    out.push((label, format!("{dir}/{}.html", g.stem), kind));
                }
            }
        };
        add(&self.lua, "lua", "Lua function");
        add(&self.event, "event", "event");
        add(&self.cvar, "cvar", "CVar");
        add(&self.op, "op", "opcode");
        add(&self.sent, "op", "opcode");
        out
    }
}

/// The doc entries that cite an address or an opcode, by the token as written.
pub struct Cites {
    entries: Vec<Cite>,
    by_token: HashMap<String, Vec<usize>>,
}

struct Cite {
    doc_key: String,
    doc_title: String,
    entry: String,
    anchor: String,
}

impl Cites {
    pub fn new(model: &Model) -> Cites {
        let mut entries = Vec::new();
        let mut by_token: HashMap<String, Vec<usize>> = HashMap::new();
        for doc in &model.docs {
            for (title, body, _) in sections(&doc.text) {
                let mut seen = HashSet::new();
                let mut tokens = Vec::new();
                for t in TOKEN.find_iter(title).chain(TOKEN.find_iter(body)) {
                    let token = t.as_str().to_lowercase();
                    if seen.insert(token.clone()) {
                        tokens.push(token);
                    }
                }
                if tokens.is_empty() {
                    continue;
                }
                entries.push(Cite {
                    doc_key: doc.key.clone(),
                    doc_title: doc.title.clone(),
                    entry: title.trim().to_string(),
                    anchor: slug(title),
                });
                for token in tokens {
                    by_token.entry(token).or_default().push(entries.len() - 1);
                }
            }
        }
        Cites { entries, by_token }
    }
}

fn rows_of(pairs: &[(&str, String)]) -> String {
    let mut out = String::from("<dl>\n");
    for (label, value) in pairs {
        if !value.is_empty() {
            out.push_str(&format!("<dt>{label}</dt><dd>{value}</dd>\n"));
        }
    }
    out.push_str("</dl>\n");
    out
}

fn dim(text: &str) -> String {
    format!("<span class=\"dim\">{}</span>", escape(text))
}

fn is_address(cell: &str) -> bool {
    cell.len() == 8 && cell.starts_with("0x") && cell[2..].bytes().all(|b| b.is_ascii_hexdigit())
}

impl Pages<'_> {
    /// The function or global that holds an address, linked; the address inside it when it is
    /// not the row's start.
    fn place(&self, address: u32, root: &str) -> String {
        let Some(item) = self.linker.row(address) else {
            return format!("<code>0x{address:06x}</code>");
        };
        let name = self.item_link(item, root);
        if item.address == address {
            format!("{name} {}", dim(&item.system))
        } else {
            let href = self.linker.href(address, root).unwrap_or_default();
            format!(
                "{name} at <a class=\"addr\" href=\"{href}\">0x{address:06x}</a> {}",
                dim(&item.system)
            )
        }
    }

    /// A cell of addresses joined by `|`, each as a place.
    fn places(&self, cell: &str, root: &str) -> String {
        cell.split('|')
            .map(|a| {
                if is_address(a) {
                    self.place(hex(a), root)
                } else {
                    escape(a)
                }
            })
            .collect::<Vec<_>>()
            .join("<br>")
    }

    /// The doc entries that cite any of `tokens`.
    fn cited(&self, tokens: &[String], root: &str) -> String {
        let mut seen = HashSet::new();
        let mut found = Vec::new();
        for t in tokens {
            for &k in self.cites.by_token.get(t).into_iter().flatten() {
                if seen.insert(k) {
                    found.push(k);
                }
            }
        }
        if found.is_empty() {
            return String::new();
        }
        let mut out = format!(
            "<h2>Cited in <span class=\"dim\">{}</span></h2>\n<ul>\n",
            found.len()
        );
        for &k in found.iter().take(CITES_SHOWN) {
            let c = &self.cites.entries[k];
            out.push_str(&format!(
                "<li><a href=\"{root}d/{}.html#{}\">{}</a> {}</li>\n",
                c.doc_key,
                c.anchor,
                escape(&c.entry),
                dim(&format!("in {}", c.doc_title))
            ));
        }
        if found.len() > CITES_SHOWN {
            out.push_str(&format!(
                "<li class=\"dim\">and {} more</li>\n",
                found.len() - CITES_SHOWN
            ));
        }
        out.push_str("</ul>\n");
        out
    }

    fn columns_link(root: &str, table: &str) -> String {
        format!(
            "<a href=\"{root}d/tables/README.html#{}\">what the columns mean</a>",
            slug(&format!("{table}.tsv"))
        )
    }

    /// Every page of the name-first set, indexes first, in a fixed order.
    pub fn name_pages(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        if let Some((view, groups)) = &self.names.lua {
            out.push(self.lua_index(view, groups));
            out.extend(groups.iter().map(|g| self.lua_page(view, g)));
        }
        if let Some((view, groups)) = &self.names.event {
            out.push(self.event_index(view, groups));
            out.extend(groups.iter().map(|g| self.event_page(view, g)));
        }
        if let Some((view, groups)) = &self.names.cvar {
            out.push(self.cvar_index(view, groups));
            out.extend(groups.iter().map(|g| self.cvar_page(view, g)));
        }
        if let Some((view, groups)) = &self.names.op {
            out.push(self.op_index(view, groups));
            out.extend(groups.iter().map(|g| self.op_page(view, g)));
        }
        if let Some((view, groups)) = &self.names.sent {
            out.extend(groups.iter().map(|g| self.op_sent_page(view, g)));
        }
        out
    }

    /// The "by name" list on the front page.
    pub fn name_links(&self) -> String {
        let mut out = String::new();
        let mut add = |part: &Option<(View, Vec<Group>)>, href: &str, label: &str| {
            if let Some((_, groups)) = part {
                out.push_str(&format!(
                    "<li><a href=\"{href}\">{label}</a> <span class=\"dim\">{}</span></li>\n",
                    groups.len()
                ));
            }
        };
        add(&self.names.lua, "lua/index.html", "Lua functions");
        add(&self.names.event, "event/index.html", "Events");
        add(&self.names.cvar, "cvar/index.html", "CVars");
        add(&self.names.op, "op/index.html", "Opcodes");
        if out.is_empty() {
            return out;
        }
        format!("<h2>By name</h2>\n<ul class=\"cols\">\n{out}</ul>\n")
    }

    fn lua_page(&self, v: &View, g: &Group) -> (String, String) {
        let path = format!("lua/{}.html", g.stem);
        let root = "../";
        let mut b = format!("<h1>{}</h1>\n", escape(g.name));
        let kinds: Vec<&str> = g.rows.iter().map(|r| v.get(r, "table_kind")).collect();
        b.push_str(&format!(
            "<p class=\"sub\">Lua {}{}</p>\n",
            kind_noun(kinds[0]),
            if g.rows.len() > 1 {
                format!(" · registered {} times", g.rows.len())
            } else {
                String::new()
            }
        ));
        let mut tokens = Vec::new();
        for (k, r) in g.rows.iter().enumerate() {
            if g.rows.len() > 1 {
                b.push_str(&format!("<h2>Registration {}</h2>\n", k + 1));
            }
            let fn_cell = v.get(r, "fn");
            tokens.push(fn_cell.to_lowercase());
            let argc = v.get(r, "argc");
            let args = match v.get(r, "argc_conf") {
                "exact" => format!("{argc} {}", dim("exact")),
                "lower-bound" => format!("at least {argc} {}", dim("lower bound")),
                other => format!("{argc} {}", dim(other)),
            };
            let meaning = self
                .linker
                .row(hex(fn_cell))
                .filter(|i| i.address == hex(fn_cell) && !i.meaning.is_empty())
                .map(|i| {
                    format!(
                        "{} {}",
                        self.linker.text(&i.meaning, root),
                        dim(&Self::proof_html(&i.confidence, &i.proof))
                    )
                })
                .unwrap_or_default();
            let kinds = v.get(r, "kinds");
            b.push_str(&rows_of(&[
                ("Function", self.places(fn_cell, root)),
                ("Meaning", meaning),
                (
                    "Registered in",
                    format!(
                        "{} table {}",
                        escape(kind_noun_plural(v.get(r, "table_kind"))),
                        self.places(v.get(r, "table"), root)
                    ),
                ),
                ("Registrar pair", self.linker.text(v.get(r, "pair"), root)),
                ("Arguments", args),
                (
                    "Returns",
                    format!(
                        "<code>{}</code> values {}, kinds <code>{}</code> {}",
                        escape(v.get(r, "arity")),
                        dim(v.get(r, "arity_conf")),
                        escape(kinds),
                        dim(v.get(r, "kinds_conf"))
                    ),
                ),
                ("Notes", escape(&v.get(r, "notes").replace(';', ", "))),
            ]));
        }
        b.push_str(&format!(
            "<p class=\"dim\">Arguments is the largest Lua stack index the function reads: a floor, never a check. \
Returns lists the number and the kinds of the values it pushes; <code>?</code> is unknown. {}.</p>\n",
            Self::columns_link(root, "bindings")
        ));
        b.push_str(&self.cited(&tokens, root));
        (path.clone(), shell(&path, g.name, &b))
    }

    fn lua_index(&self, v: &View, groups: &[Group]) -> (String, String) {
        let path = "lua/index.html".to_string();
        let root = "../";
        let mut b = format!(
            "<h1>Lua functions <span class=\"dim\">{} names</span></h1>\n\
<p>Every function the client registers for Lua, by the table that registers it. A name some tables share \
has one page for all of them. {}.</p>\n",
            groups.len(),
            Self::columns_link(root, "bindings")
        );
        let mut tables: BTreeMap<(usize, &str), Vec<&Group>> = BTreeMap::new();
        for g in groups {
            let mut seen = HashSet::new();
            for r in &g.rows {
                let kind = v.get(r, "table_kind");
                let rank = ["global", "widget", "baselib", "stdlib"]
                    .iter()
                    .position(|k| *k == kind)
                    .unwrap_or(4);
                if seen.insert((rank, v.get(r, "table"))) {
                    tables.entry((rank, v.get(r, "table"))).or_default().push(g);
                }
            }
        }
        let kind_of: HashMap<&str, &str> = v
            .rows
            .iter()
            .map(|r| (v.get(r, "table"), v.get(r, "table_kind")))
            .collect();
        for ((_, table), list) in &tables {
            let heading = match kind_of.get(table).copied().unwrap_or("") {
                "global" => "Global functions",
                "widget" => "Widget methods",
                "baselib" => "Base library",
                "stdlib" => "Standard library",
                _ => "Other",
            };
            b.push_str(&format!(
                "<h2>{heading} <span class=\"dim\">{} · table</span> {}</h2>\n<ul class=\"cols\">\n",
                list.len(),
                self.places(table, root)
            ));
            for g in list {
                b.push_str(&format!(
                    "<li><a href=\"{}.html\">{}</a></li>\n",
                    g.stem,
                    escape(g.name)
                ));
            }
            b.push_str("</ul>\n");
        }
        (path.clone(), shell(&path, "Lua functions", &b))
    }

    fn event_page(&self, v: &View, g: &Group) -> (String, String) {
        let path = format!("event/{}.html", g.stem);
        let root = "../";
        let mut b = format!("<h1>{}</h1>\n", escape(g.name));
        let ids: Vec<&str> = g.rows.iter().map(|r| v.get(r, "id")).collect();
        b.push_str(&format!(
            "<p class=\"sub\">event · id {}</p>\n",
            escape(&ids.join(", "))
        ));
        b.push_str("<h2>Name table</h2>\n<div class=\"scroll\"><table>\n<tr><th>id</th><th>name string</th><th>filled by</th></tr>\n");
        for r in &g.rows {
            b.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                escape(v.get(r, "id")),
                self.linker.text(v.get(r, "name_address"), root),
                self.places(v.get(r, "store"), root)
            ));
        }
        b.push_str("</table></div>\n");
        let sites: Vec<&Vec<String>> = self
            .names
            .sites
            .as_ref()
            .map(|s| {
                s.rows
                    .iter()
                    .filter(|r| {
                        s.get(r, "space") == "game"
                            && s.get(r, "event_names").split('|').any(|n| n == g.name)
                    })
                    .collect()
            })
            .unwrap_or_default();
        b.push_str(&format!(
            "<h2>Fire sites <span class=\"dim\">{}</span></h2>\n",
            sites.len()
        ));
        if sites.is_empty() {
            b.push_str(
                "<p class=\"dim\">No fire site names this event; the doc entry ui/script, Events no site fires, gives the census.</p>\n",
            );
        } else if let Some(s) = &self.names.sites {
            b.push_str(
                "<div class=\"scroll\"><table>\n<tr><th>site</th><th>dispatcher</th><th>argument format</th><th>other events at the site</th></tr>\n",
            );
            for r in sites {
                let others: Vec<&str> = s
                    .get(r, "event_names")
                    .split('|')
                    .filter(|n| !n.is_empty() && *n != g.name)
                    .collect();
                let format = s.get(r, "arg_format");
                b.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                    self.places(s.get(r, "site"), root),
                    escape(s.get(r, "dispatcher")),
                    if format.is_empty() {
                        dim("none")
                    } else {
                        format!("<code>{}</code>", escape(format))
                    },
                    escape(&others.join(", "))
                ));
            }
            b.push_str(&format!(
                "</table></div>\n<p class=\"dim\">The argument format is the declaration <code>SignalEvent2</code> is given, \
not a count of what the caller pushed. {}.</p>\n",
                Self::columns_link(root, "event_sites")
            ));
        }
        (path.clone(), shell(&path, g.name, &b))
    }

    fn event_index(&self, v: &View, groups: &[Group]) -> (String, String) {
        let path = "event/index.html".to_string();
        let root = "../";
        let mut fired: HashMap<&str, usize> = HashMap::new();
        if let Some(s) = &self.names.sites {
            for r in s.rows.iter().filter(|r| s.get(r, "space") == "game") {
                for n in s.get(r, "event_names").split('|') {
                    *fired.entry(n).or_default() += 1;
                }
            }
        }
        let mut b = format!(
            "<h1>Events <span class=\"dim\">{} names</span></h1>\n\
<p>The C-to-Lua events, with the sites that fire them. {}.</p>\n\
<div class=\"scroll\"><table>\n<tr><th>event</th><th>id</th><th>fire sites</th></tr>\n",
            groups.len(),
            Self::columns_link(root, "events")
        );
        for g in groups {
            let ids: Vec<&str> = g.rows.iter().map(|r| v.get(r, "id")).collect();
            b.push_str(&format!(
                "<tr><td><a href=\"{}.html\">{}</a></td><td>{}</td><td>{}</td></tr>\n",
                g.stem,
                escape(g.name),
                escape(&ids.join(", ")),
                fired.get(g.name).copied().unwrap_or(0)
            ));
        }
        b.push_str("</table></div>\n");
        (path.clone(), shell(&path, "Events", &b))
    }

    fn cvar_page(&self, v: &View, g: &Group) -> (String, String) {
        let path = format!("cvar/{}.html", g.stem);
        let root = "../";
        let mut b = format!("<h1>{}</h1>\n<p class=\"sub\">CVar", escape(g.name));
        if g.rows.len() > 1 {
            b.push_str(&format!(" · registered {} times", g.rows.len()));
        }
        b.push_str("</p>\n");
        let mut tokens = Vec::new();
        for (k, r) in g.rows.iter().enumerate() {
            if g.rows.len() > 1 {
                b.push_str(&format!("<h2>Registration {}</h2>\n", k + 1));
            }
            tokens.push(v.get(r, "site").to_lowercase());
            let callback = v.get(r, "callback");
            let hidden = match v.get(r, "script_hidden") {
                "0" => "Lua can read it".to_string(),
                "1" => "Lua cannot read it".to_string(),
                other => escape(other),
            };
            b.push_str(&rows_of(&[
                ("Help", escape(v.get(r, "help"))),
                (
                    "Default",
                    format!(
                        "<code>{}</code> {}",
                        escape(v.get(r, "default")),
                        self.linker.text(v.get(r, "default_address"), root)
                    ),
                ),
                (
                    "Flags",
                    format!("<code>{}</code>", escape(v.get(r, "flags"))),
                ),
                ("Category", escape(v.get(r, "category"))),
                ("Script", hidden),
                (
                    "Change callback",
                    if callback.is_empty() {
                        dim("none")
                    } else {
                        format!(
                            "{} {}",
                            self.places(callback, root),
                            dim(&format!("argument {}", v.get(r, "callback_arg")))
                        )
                    },
                ),
                ("Registered at", self.places(v.get(r, "site"), root)),
                (
                    "Name string",
                    self.linker.text(v.get(r, "name_address"), root),
                ),
                ("Record", self.places(v.get(r, "record"), root)),
                ("Notes", escape(v.get(r, "notes"))),
            ]));
        }
        b.push_str(&format!(
            "<p class=\"dim\">{}.</p>\n",
            Self::columns_link(root, "cvars")
        ));
        b.push_str(&self.cited(&tokens, root));
        (path.clone(), shell(&path, g.name, &b))
    }

    fn cvar_index(&self, v: &View, groups: &[Group]) -> (String, String) {
        let path = "cvar/index.html".to_string();
        let root = "../";
        let mut b = format!(
            "<h1>CVars <span class=\"dim\">{} names</span></h1>\n\
<p>Every variable the client registers. {}.</p>\n\
<div class=\"scroll\"><table>\n<tr><th>name</th><th>default</th><th>help</th></tr>\n",
            groups.len(),
            Self::columns_link(root, "cvars")
        );
        for g in groups {
            let r = g.rows[0];
            b.push_str(&format!(
                "<tr><td><a href=\"{}.html\">{}</a></td><td><code>{}</code></td><td>{}</td></tr>\n",
                g.stem,
                escape(g.name),
                escape(v.get(r, "default")),
                escape(v.get(r, "help"))
            ));
        }
        b.push_str("</table></div>\n");
        let unresolved: Vec<&Vec<String>> =
            v.rows.iter().filter(|r| v.get(r, "name") == "?").collect();
        if !unresolved.is_empty() {
            b.push_str(&format!(
                "<h2>Registered under a name the table could not resolve <span class=\"dim\">{}</span></h2>\n<ul class=\"cols\">\n",
                unresolved.len()
            ));
            for r in unresolved {
                b.push_str(&format!(
                    "<li>{}</li>\n",
                    self.places(v.get(r, "site"), root)
                ));
            }
            b.push_str("</ul>\n");
        }
        (path.clone(), shell(&path, "CVars", &b))
    }

    fn op_page(&self, v: &View, g: &Group) -> (String, String) {
        let path = format!("op/{}.html", g.stem);
        let root = "../";
        let name = v.get(g.rows[0], "name");
        let mut b = format!(
            "<h1>Opcode {}{}</h1>\n<p class=\"sub\">network message</p>\n",
            escape(g.name),
            if name.is_empty() {
                String::new()
            } else {
                format!(" {}", escape(name))
            }
        );
        let mut tokens = vec![g.name.to_lowercase()];
        for r in &g.rows {
            let handler = v.get(r, "handler");
            tokens.push(handler.to_lowercase());
            let path_note = match v.get(r, "path") {
                "A" => "through the wrapper 0x5ab650".to_string(),
                "B" => "a direct call to NetClient::SetMessageHandler 0x537a60".to_string(),
                other => other.to_string(),
            };
            let shared: Vec<String> = v
                .rows
                .iter()
                .filter(|o| v.get(o, "handler") == handler && v.get(o, "opcode") != g.name)
                .map(|o| {
                    let code = v.get(o, "opcode");
                    format!("<a href=\"{code}.html\">{}</a>", escape(code))
                })
                .collect();
            b.push_str(&rows_of(&[
                ("Handler", self.places(handler, root)),
                ("Registered at", self.places(v.get(r, "site"), root)),
                ("Registered by", self.linker.text(&path_note, root)),
                ("Same handler", shared.join(", ")),
            ]));
        }
        b.push_str(&format!(
            "<p class=\"dim\">{}.</p>\n",
            Self::columns_link(root, "opcodes")
        ));
        b.push_str(&self.op_extras(g.name, root));
        b.push_str(&self.cited(&tokens, root));
        (
            path.clone(),
            shell(&path, &format!("opcode {}", g.name), &b),
        )
    }

    /// The page of an opcode the client sends and registers no handler for.
    fn op_sent_page(&self, v: &View, g: &Group) -> (String, String) {
        let path = format!("op/{}.html", g.stem);
        let root = "../";
        let name = v.get(g.rows[0], "name");
        let mut b = format!(
            "<h1>Opcode {}{}</h1>\n<p class=\"sub\">network message the client sends</p>\n",
            escape(g.name),
            if name.is_empty() {
                String::new()
            } else {
                format!(" {}", escape(name))
            }
        );
        b.push_str(&self.op_extras(g.name, root));
        b.push_str(&format!(
            "<p class=\"dim\">{}.</p>\n",
            Self::columns_link(root, "opcode_sends")
        ));
        b.push_str(&self.cited(&[g.name.to_lowercase()], root));
        (
            path.clone(),
            shell(&path, &format!("opcode {}", g.name), &b),
        )
    }

    /// Where the client sends an opcode, and the entries that cover its handler and its sends.
    fn op_extras(&self, code: &str, root: &str) -> String {
        let mut out = String::new();
        if let Some(sends) = &self.names.sends {
            let rows: Vec<&Vec<String>> = sends
                .rows
                .iter()
                .filter(|r| sends.get(r, "opcode") == code)
                .collect();
            if !rows.is_empty() {
                out.push_str(&format!(
                    "<h2>Sent at <span class=\"dim\">{}</span></h2>\n<ul>\n",
                    rows.len()
                ));
                for r in rows {
                    let notes = sends.get(r, "notes");
                    out.push_str(&format!(
                        "<li>{}{}</li>\n",
                        self.places(sends.get(r, "site"), root),
                        if notes.is_empty() {
                            String::new()
                        } else {
                            format!(" {}", dim(notes))
                        }
                    ));
                }
                out.push_str("</ul>\n");
            }
        }
        if let Some(entries) = &self.names.entries {
            let rows: Vec<&Vec<String>> = entries
                .rows
                .iter()
                .filter(|r| entries.get(r, "opcode") == code)
                .collect();
            if !rows.is_empty() {
                out.push_str("<h2>Entries</h2>\n<ul>\n");
                for r in rows {
                    let links: Vec<String> = entries
                        .get(r, "entries")
                        .split('|')
                        .filter_map(|e| e.split_once('#'))
                        .map(|(doc, anchor)| {
                            format!(
                                "<a href=\"{root}d/systems/{doc}.html#{anchor}\">{}</a>",
                                escape(&anchor.replace('-', " "))
                            )
                        })
                        .collect();
                    let basis = entries.get(r, "basis");
                    out.push_str(&format!(
                        "<li>{} at {}: {} {}</li>\n",
                        escape(entries.get(r, "role")),
                        self.places(entries.get(r, "site"), root),
                        if links.is_empty() {
                            "no entry".to_string()
                        } else {
                            links.join(", ")
                        },
                        dim(basis)
                    ));
                }
                out.push_str("</ul>\n");
            }
        }
        out
    }

    fn op_index(&self, v: &View, groups: &[Group]) -> (String, String) {
        let path = "op/index.html".to_string();
        let root = "../";
        let named = v.has("name");
        let mut b = format!(
            "<h1>Opcodes <span class=\"dim\">{} messages</span></h1>\n\
<p>Every network message the client registers a handler for. {}.</p>\n\
<div class=\"scroll\"><table>\n<tr><th>opcode</th>{}<th>handler</th></tr>\n",
            groups.len(),
            Self::columns_link(root, "opcodes"),
            if named { "<th>name</th>" } else { "" }
        );
        for g in groups {
            let r = g.rows[0];
            b.push_str(&format!(
                "<tr><td><a href=\"{}.html\"><code>{}</code></a></td>{}<td>{}</td></tr>\n",
                g.stem,
                escape(g.name),
                if named {
                    format!("<td>{}</td>", escape(v.get(r, "name")))
                } else {
                    String::new()
                },
                self.places(v.get(r, "handler"), root)
            ));
        }
        b.push_str("</table></div>\n");
        if let Some((sv, sent)) = self.names.sent.as_ref().filter(|(_, g)| !g.is_empty()) {
            let named = sv.has("name");
            b.push_str(&format!(
                "<h2>Sent only <span class=\"dim\">{}</span></h2>\n\
<p>Messages the client sends and registers no handler for. {}.</p>\n\
<div class=\"scroll\"><table>\n<tr><th>opcode</th>{}<th>send sites</th></tr>\n",
                sent.len(),
                Self::columns_link(root, "opcode_sends"),
                if named { "<th>name</th>" } else { "" }
            ));
            for g in sent {
                b.push_str(&format!(
                    "<tr><td><a href=\"{}.html\"><code>{}</code></a></td>{}<td>{}</td></tr>\n",
                    g.stem,
                    escape(g.name),
                    if named {
                        format!("<td>{}</td>", escape(sv.get(g.rows[0], "name")))
                    } else {
                        String::new()
                    },
                    g.rows.len()
                ));
            }
            b.push_str("</table></div>\n");
        }
        (path.clone(), shell(&path, "Opcodes", &b))
    }
}

fn kind_noun(kind: &str) -> &'static str {
    match kind {
        "global" => "global function",
        "widget" => "widget method",
        "baselib" => "base library function",
        "stdlib" => "standard library function",
        _ => "function",
    }
}

fn kind_noun_plural(kind: &str) -> &'static str {
    match kind {
        "global" => "global",
        "widget" => "widget method",
        "baselib" => "base library",
        "stdlib" => "standard library",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stem_is_a_safe_file_name() {
        assert_eq!(stem_of("GetItemInfo"), "GetItemInfo");
        assert_eq!(stem_of("a/b c"), "a%2Fb%20c");
        assert_eq!(stem_of("index"), "index~");
    }
}
