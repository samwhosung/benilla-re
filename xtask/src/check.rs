//! The exe-free half of the check: the symbol table and the system docs against the formats and
//! limits in `symbols/README.md` and `systems/README.md`.

use crate::sig;
use crate::tree::Tree;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;
use std::sync::LazyLock;

macro_rules! pattern {
    ($name:ident, $source:expr) => {
        static $name: LazyLock<Regex> =
            LazyLock::new(|| Regex::new($source).expect("a valid pattern"));
    };
}

pub(crate) const LIBRARIES: [&str; 5] = ["crt", "storm", "lua", "zlib", "d3dx"];
/// The section of `symbols/README.md` that names the systems whose doc is not written yet.
const UNWRITTEN: &str = "## Systems without a doc";
const CLASSES: [&str; 5] = [
    "boundary",
    "host",
    "orchestration",
    "primitive",
    "unreached",
];

const MEANING: usize = 120;
const NAME: usize = 48;
const SIGNATURE: usize = 200;
const OVERVIEW: usize = 200;
const TITLE: usize = 6;
const CLAIM: usize = 120;
const TABLE: usize = 12;
const DERIVATION: usize = 200;
const CODE: usize = 8;
const DOC: usize = 4000;

pattern!(ADDRESS, r"^0x[0-9a-f]{6}$");
pattern!(OFFSET, r"^0x[0-9a-f]{4}$");
pattern!(DECIMAL, r"^[1-9][0-9]*$");
pattern!(SYSTEM, r"^[a-z][a-z0-9]*$");
pattern!(TEST, r"^[a-z][a-z0-9_]*::[a-z][a-z0-9_]*$");
// A class name keeps the client's own suffix, as its tags give it (`.?AVCGObject_C@@` is
// `CGObject_C`); a method's own part takes none (`method_name`).
pattern!(
    METHOD_NAME,
    r"^(?:[A-Z][A-Za-z0-9]*(?:_[A-Z][A-Za-z0-9]*)*::)+~?[A-Z][A-Za-z0-9]*$"
);
pattern!(FREE_NAME, r"^[A-Z][A-Za-z0-9]*$");
pattern!(
    LIBRARY_NAME,
    r"^[A-Za-z_][A-Za-z0-9_]*(?:::~?[A-Za-z_][A-Za-z0-9_]*)*(?:@[0-9]+)?$"
);
pattern!(
    GLOBAL_NAME,
    r"^(?:s_[a-z][A-Za-z0-9]*|(?:[A-Z][A-Za-z0-9]*(?:_[A-Z][A-Za-z0-9]*)*::)+vftable)$"
);
pattern!(FIELD_NAME, r"^(?:m_[a-z][A-Za-z0-9]*|__vfptr)$");
pattern!(
    STRUCT_NAME,
    r"^[A-Z][A-Za-z0-9]*(?:_[A-Z][A-Za-z0-9]*)*(?:::[A-Z][A-Za-z0-9]*(?:_[A-Z][A-Za-z0-9]*)*)*$"
);
pattern!(VERB, r"^[A-Z][A-Za-z]*s\b");
pattern!(
    MEANING_BANNED,
    r"(?i)\?|\bTODO\b|\b(?:probably|maybe|seems|likely)\b"
);
pattern!(
    HEDGES,
    r"(?i)\b(?:probably|likely|seems|appears to|roughly|maybe|should be|TODO|FIXME)\b"
);
pattern!(
    SHOUTING,
    r"NOTE:|\b(?:IMPORTANT|WARNING|CORRECTED|VERIFIED|INFERRED)\b"
);
pattern!(
    HISTORY,
    r"(?i)\b(?:previously|used to|no longer|originally|corrected|was wrong)\b"
);
pattern!(FIRST_PERSON, r"\b(?:[Ww]e|[Oo]ur|I)\b");
pattern!(CITED, r"`(0x[0-9a-f]{6})`");
pattern!(BARE_ADDRESS, r"\b0x[0-9a-f]{6}\b");
pattern!(ANY_HEADING, r"^#{1,6} (.+)$");
pattern!(LISTED_SYSTEM, r"^- `([a-z][a-z0-9]*)`: \S");

/// The image's virtual addresses: the base 0x400000 up to base plus `SizeOfImage` (0x906000).
const IMAGE: std::ops::Range<u64> = 0x400000..0xd06000;
pattern!(INLINE_CODE, r"`[^`]*`");
pattern!(CITED_ANCHOR, r"`([A-Za-z][A-Za-z0-9_/]*)#([a-z0-9_-]+)`");
pattern!(LINKED_ANCHOR, r"\]\(([^)#\s]*)#([a-z0-9_-]+)\)");
pattern!(HEADING, r"(?m)^## (.+)$");
pattern!(ROOTS, r"(?ms)^Roots: (.+?)(?:\n\n|\z)");
pattern!(
    SHAPE,
    r"(?m)^Addresses: (.+)\nConfidence: (.*)\nProof: (.*)\nTest: (.*)$"
);
pattern!(ADDRESS_LIST, r"^`0x[0-9a-f]{6}`(?:, `0x[0-9a-f]{6}`)*$");
pattern!(TEST_CITED, r"^`[a-z][a-z0-9_]*::[a-z][a-z0-9_]*`$");
pattern!(TABLE_RULE, r"^\|[-|: ]+\|$");
pattern!(FUNCTION_POINTERS, r"^.+\(\*\[([0-9]+)\]\)\(.*\)$");
pattern!(ARRAY, r"^(.+?)\s*\[([0-9]+)\]$");

/// One problem: where, as `file:line`, and what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub place: String,
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.place, self.message)
    }
}

#[derive(Default)]
struct Problems(Vec<Problem>);

impl Problems {
    fn add(&mut self, place: impl Into<String>, message: impl Into<String>) {
        self.0.push(Problem {
            place: place.into(),
            message: message.into(),
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Functions,
    Globals,
    Structs,
    Fields,
}

impl Kind {
    pub fn file(self) -> &'static str {
        match self {
            Kind::Functions => "symbols/functions.tsv",
            Kind::Globals => "symbols/globals.tsv",
            Kind::Structs => "symbols/structs.tsv",
            Kind::Fields => "symbols/fields.tsv",
        }
    }

    fn header(self) -> &'static [&'static str] {
        match self {
            Kind::Functions => &[
                "address",
                "size",
                "name",
                "name_from",
                "system",
                "class",
                "signature",
                "meaning",
                "confidence",
                "proof",
                "finding",
                "test",
            ],
            Kind::Globals => &[
                "address",
                "type",
                "name",
                "name_from",
                "system",
                "meaning",
                "confidence",
                "proof",
                "finding",
            ],
            Kind::Structs => &[
                "name",
                "size",
                "vtable",
                "system",
                "meaning",
                "confidence",
                "proof",
                "finding",
            ],
            Kind::Fields => &[
                "struct",
                "offset",
                "size",
                "type",
                "name",
                "system",
                "meaning",
                "confidence",
                "proof",
                "finding",
            ],
        }
    }

    /// The columns a table is sorted by, in byte order.
    fn sort_key(self) -> &'static [&'static str] {
        match self {
            Kind::Functions | Kind::Globals => &["address"],
            Kind::Structs => &["name"],
            Kind::Fields => &["struct", "offset"],
        }
    }
}

/// One row of a table, with its line number.
pub struct Row {
    kind: Kind,
    pub line: usize,
    cells: Vec<String>,
}

impl Row {
    pub fn get(&self, column: &str) -> &str {
        let index = self
            .kind
            .header()
            .iter()
            .position(|c| *c == column)
            .unwrap_or_else(|| panic!("{} has no column {column}", self.kind.file()));
        &self.cells[index]
    }

    pub fn place(&self) -> String {
        format!("{}:{}", self.kind.file(), self.line)
    }

    fn key(&self) -> Vec<&str> {
        self.kind.sort_key().iter().map(|c| self.get(c)).collect()
    }
}

/// A system doc: `systems/<key>.md`.
pub struct Doc {
    pub key: String,
    pub path: String,
    pub text: String,
}

/// The address ranges the rows of `functions.tsv` and `globals.tsv` cover.
#[derive(Default)]
pub struct Extents {
    spans: Vec<(u64, u64)>,
    /// The largest end among `spans[..=i]`, so a lookup can stop once nothing earlier reaches.
    reach: Vec<u64>,
}

impl Extents {
    fn new(mut spans: Vec<(u64, u64)>) -> Extents {
        spans.sort();
        let mut reach = Vec::with_capacity(spans.len());
        let mut furthest = 0;
        for &(_, end) in &spans {
            furthest = furthest.max(end);
            reach.push(furthest);
        }
        Extents { spans, reach }
    }

    /// Whether an address is a row's address or lies inside a row's extent.
    pub fn contains(&self, address: u64) -> bool {
        let mut i = self.spans.partition_point(|&(start, _)| start <= address);
        while i > 0 {
            i -= 1;
            if self.reach[i] <= address {
                return false;
            }
            if self.spans[i].1 > address {
                return true;
            }
        }
        false
    }

    /// Whether an address lies in the padding after a function: no row holds it and it is less
    /// than 16 bytes past the end of the nearest row before it. A system doc writes a function's
    /// end and its padding bytes in backticks like any address.
    pub fn pads(&self, address: u64) -> bool {
        let i = self.spans.partition_point(|&(start, _)| start <= address);
        i > 0 && self.reach[i - 1] <= address && address - self.reach[i - 1] < 16
    }
}

/// What the check read, and the problems it found.
pub struct Checked {
    pub problems: Vec<Problem>,
    pub functions: Vec<Row>,
    pub globals: Vec<Row>,
    pub structs: Vec<Row>,
    pub fields: Vec<Row>,
    pub docs: Vec<Doc>,
    pub method: Option<String>,
    pub extents: Extents,
    /// Every well-formed test a doc entry's `Test` line names, with the line's place.
    pub doc_tests: Vec<(String, String)>,
}

impl Checked {
    pub fn summary(&self) -> String {
        format!(
            "{} functions, {} globals, {} structs, {} fields, {} system docs",
            self.functions.len(),
            self.globals.len(),
            self.structs.len(),
            self.fields.len(),
            self.docs.len()
        )
    }

    /// Every well-formed test the table and the docs name, with the place that names it.
    pub fn named_tests(&self) -> Vec<(String, String)> {
        let mut named = Vec::new();
        for row in &self.functions {
            let tests = row.get("test");
            if !tests.is_empty() && tests.split(", ").all(|t| TEST.is_match(t)) {
                for test in tests.split(", ") {
                    named.push((row.place(), test.to_string()));
                }
            }
        }
        named.extend(self.doc_tests.iter().cloned());
        named
    }
}

/// Reads a file the check needs; a missing file or one that is not UTF-8 is a problem, not an
/// error.
fn read(tree: &dyn Tree, path: &str, problems: &mut Problems) -> io::Result<Option<String>> {
    match tree.read(path) {
        Ok(Some(text)) => Ok(Some(text)),
        Ok(None) => {
            problems.add(format!("{path}:1"), "missing");
            Ok(None)
        }
        Err(error) if error.kind() == io::ErrorKind::InvalidData => {
            problems.add(format!("{path}:1"), "is not UTF-8");
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub fn run(tree: &dyn Tree) -> io::Result<Checked> {
    let mut p = Problems::default();
    let files = tree.files()?;
    let markdown: Vec<&String> = files.iter().filter(|f| f.ends_with(".md")).collect();

    let mut docs = Vec::new();
    for path in &markdown {
        let Some(key) = path
            .strip_prefix("systems/")
            .and_then(|rest| rest.strip_suffix(".md"))
        else {
            continue;
        };
        if path.as_str() == "systems/README.md" {
            continue;
        }
        if let Some(text) = read(tree, path, &mut p)? {
            docs.push(Doc {
                key: key.to_string(),
                path: path.to_string(),
                text,
            });
        }
    }
    let written: HashSet<&str> = docs
        .iter()
        .map(|d| d.key.split('/').next().unwrap_or_default())
        .collect();

    let mut anchors: HashMap<String, String> = HashMap::new();
    for doc in &docs {
        for (title, body, _) in sections(&doc.text) {
            anchors.insert(format!("{}#{}", doc.key, slug(title)), body.to_string());
        }
    }
    let method = read(tree, "docs/METHOD.md", &mut p)?;
    if let Some(text) = &method {
        for (title, body, _) in sections(text) {
            anchors.insert(format!("METHOD#{}", slug(title)), body.to_string());
        }
    }

    let functions = read_table(tree, Kind::Functions, &mut p)?;
    let globals = read_table(tree, Kind::Globals, &mut p)?;
    let structs_rows = read_table(tree, Kind::Structs, &mut p)?;
    let fields = read_table(tree, Kind::Fields, &mut p)?;
    let mut structs: HashMap<&str, &Row> = HashMap::new();
    for row in &structs_rows {
        let name = row.get("name");
        if structs.contains_key(name) {
            p.add(row.place(), format!("struct {name} appears twice"));
        }
        structs.insert(name, row);
    }

    let struct_sizes: sig::Structs = structs_rows
        .iter()
        .filter_map(|r| Some((r.get("name"), r.get("size").parse().ok()?)))
        .collect();
    let mut names: HashMap<&str, &str> = HashMap::new();
    let mut extents = Vec::new();
    for (kind, rows) in [(Kind::Functions, &functions), (Kind::Globals, &globals)] {
        let mut spans: Vec<(u64, u64, &Row)> = Vec::new();
        for row in rows {
            let place = row.place();
            let address = row.get("address");
            if !ADDRESS.is_match(address) {
                p.add(
                    place,
                    format!("address '{address}' is not 0x and six lowercase hex digits"),
                );
                continue;
            }
            if !matches!(row.get("name_from"), "" | "idb") {
                p.add(&place, "name_from is idb or empty");
            }
            let name = row.get("name");
            if !name.is_empty() {
                if let Some(other) = names.get(name) {
                    p.add(&place, format!("name {name} is also used at {other}"));
                }
                names.insert(name, address);
            }
            check_common(row, &anchors, Some(address), &mut p);
            let start = u64::from_str_radix(&address[2..], 16).expect("six hex digits");
            let size = if kind == Kind::Functions {
                check_name(
                    row,
                    |n| method_name(n) || FREE_NAME.is_match(n),
                    "functions",
                    &mut p,
                );
                let size = decimal(row.get("size"));
                if size.is_none() {
                    p.add(
                        &place,
                        format!("size '{}' is not a positive decimal", row.get("size")),
                    );
                }
                let class = row.get("class");
                if !class.is_empty() && !CLASSES.contains(&class) {
                    p.add(
                        &place,
                        format!("class '{class}' is not one of {}", CLASSES.join(", ")),
                    );
                }
                let system = row.get("system");
                if class.is_empty() && written.contains(system) {
                    p.add(&place, format!("class is required: {system} has a doc"));
                }
                let test = row.get("test");
                if class == "unreached" && !test.is_empty() {
                    p.add(&place, "an unreached function has no test");
                }
                if !test.is_empty() && !test.split(", ").all(|t| TEST.is_match(t)) {
                    p.add(&place, format!("test '{test}' is not crate::test_fn"));
                }
                let signature = row.get("signature");
                if signature.chars().count() > SIGNATURE {
                    p.add(&place, format!("signature is over {SIGNATURE} characters"));
                }
                if !signature.is_empty() {
                    if let Err(message) = sig::parse(signature, &struct_sizes) {
                        p.add(&place, message);
                    }
                }
                size
            } else {
                check_name(row, |n| GLOBAL_NAME.is_match(n), "globals", &mut p);
                let size = type_size(row.get("type"), &structs);
                if size.is_none() {
                    p.add(
                        &place,
                        format!("type '{}' has no known size", row.get("type")),
                    );
                }
                size
            };
            if let Some(size) = size.filter(|&s| s > 0) {
                spans.push((start, start + size, row));
                extents.push((start, start + size));
            }
        }
        // Each row is compared with the row reaching furthest before it, not only its
        // neighbour, so a row inside a long one is caught after a short one between them.
        spans.sort_by_key(|s| s.0);
        let mut furthest: Option<(u64, &Row)> = None;
        for &(start, end, row) in &spans {
            if let Some((reach, outer)) = furthest {
                if start < reach {
                    p.add(
                        row.place(),
                        format!(
                            "{} lies inside {}",
                            row.get("address"),
                            outer.get("address")
                        ),
                    );
                }
            }
            if furthest.is_none_or(|(reach, _)| end > reach) {
                furthest = Some((end, row));
            }
        }
    }
    let mut seen: HashMap<&str, String> = HashMap::new();
    for row in functions.iter().chain(&globals) {
        let address = row.get("address");
        if let Some(other) = seen.get(address) {
            p.add(
                row.place(),
                format!("address {address} also has a row at {other}"),
            );
        }
        seen.insert(address, row.place());
    }
    let global_addresses: HashSet<&str> = globals.iter().map(|r| r.get("address")).collect();

    for row in &structs_rows {
        let place = row.place();
        let library = LIBRARIES.contains(&row.get("system"));
        let rule = if library { &LIBRARY_NAME } else { &STRUCT_NAME };
        if !rule.is_match(row.get("name")) {
            p.add(
                &place,
                format!("struct name '{}' breaks the naming rule", row.get("name")),
            );
        }
        if decimal(row.get("size")).is_none() {
            p.add(
                &place,
                format!("size '{}' is not a positive decimal", row.get("size")),
            );
        }
        let vtable = row.get("vtable");
        if !vtable.is_empty() && !global_addresses.contains(vtable) {
            p.add(
                &place,
                format!("vtable {vtable} is not a row of globals.tsv"),
            );
        }
        check_common(row, &anchors, None, &mut p);
    }

    // Each struct's field names so far, and the byte ranges they cover.
    type Occupied<'a> = (HashSet<&'a str>, Vec<(u64, u64)>);
    let mut occupied: HashMap<&str, Occupied> = HashMap::new();
    for row in &fields {
        let place = row.place();
        let struct_name = row.get("struct");
        let Some(owner) = structs.get(struct_name) else {
            p.add(
                &place,
                format!("struct {struct_name} is not a row of structs.tsv"),
            );
            continue;
        };
        let library = LIBRARIES.contains(&owner.get("system"));
        let rule = if library { &LIBRARY_NAME } else { &FIELD_NAME };
        let name = row.get("name");
        if !rule.is_match(name) {
            p.add(
                &place,
                format!("field name '{name}' breaks the naming rule"),
            );
        }
        let (Some(size), true) = (decimal(row.get("size")), OFFSET.is_match(row.get("offset")))
        else {
            p.add(
                &place,
                "offset is 0x and four hex digits, size a positive decimal",
            );
            continue;
        };
        let offset = u64::from_str_radix(&row.get("offset")[2..], 16).expect("four hex digits");
        if let Some(whole) = decimal(owner.get("size")) {
            if offset + size > whole {
                p.add(&place, format!("field ends past the {whole}-byte struct"));
            }
        }
        let ty = row.get("type");
        match type_size(ty, &structs) {
            None => p.add(&place, format!("type '{ty}' has no known size")),
            Some(expected) if expected != size => p.add(
                &place,
                format!("size {size} differs from the size of {ty}, {expected}"),
            ),
            Some(_) => {}
        }
        let (names_here, spans_here) = occupied.entry(struct_name).or_default();
        if !names_here.insert(name) {
            p.add(
                &place,
                format!("field {name} appears twice in {struct_name}"),
            );
        }
        if spans_here
            .iter()
            .any(|&(start, end)| offset < end && start < offset + size)
        {
            p.add(
                &place,
                format!("field overlaps another field of {struct_name}"),
            );
        }
        spans_here.push((offset, offset + size));
        check_common(row, &anchors, None, &mut p);
    }

    check_systems(
        tree,
        &written,
        [&functions, &globals, &structs_rows, &fields],
        &mut p,
    )?;

    let extents = Extents::new(extents);
    let mut doc_tests = Vec::new();
    for doc in &docs {
        check_doc(doc, &functions, &mut doc_tests, &mut p);
        check_citations(doc, &anchors, &mut p);
    }

    // A system doc's addresses and a row's meaning cite the table: each has a row. Every other
    // Markdown file is a spec doc, whose addresses are examples and pointers: each lies inside
    // the image.
    for row in functions
        .iter()
        .chain(&globals)
        .chain(&structs_rows)
        .chain(&fields)
    {
        for found in BARE_ADDRESS.find_iter(row.get("meaning")) {
            let value = u64::from_str_radix(&found.as_str()[2..], 16).expect("six hex digits");
            if !extents.contains(value) {
                p.add(
                    row.place(),
                    format!("meaning cites {}, which resolves to no row", found.as_str()),
                );
            }
        }
    }
    for path in &markdown {
        let Some(text) = read(tree, path, &mut p)? else {
            continue;
        };
        let system_doc = docs.iter().any(|d| &d.path == *path);
        let mut anchors_here: HashMap<String, usize> = HashMap::new();
        let mut in_code = false;
        for (number, line) in text.split('\n').enumerate() {
            let place = format!("{path}:{}", number + 1);
            if line.starts_with("```") {
                in_code = !in_code;
            } else if let Some(heading) = ANY_HEADING.captures(line).filter(|_| !in_code) {
                let anchor = slug(&heading[1]);
                if let Some(first) = anchors_here.get(&anchor) {
                    p.add(
                        &place,
                        format!("heading's anchor #{anchor} is also line {first}'s"),
                    );
                } else {
                    anchors_here.insert(anchor, number + 1);
                }
            }
            for cited in CITED.captures_iter(line) {
                let address = &cited[1];
                let value = u64::from_str_radix(&address[2..], 16).expect("six hex digits");
                if system_doc && !extents.contains(value) && !extents.pads(value) {
                    p.add(&place, format!("{address} resolves to no row"));
                } else if !system_doc && !IMAGE.contains(&value) {
                    p.add(
                        &place,
                        format!("{address} is not an address inside the image"),
                    );
                }
            }
        }
    }

    p.0.extend(crate::tables::check_headers(tree, &files)?);

    Ok(Checked {
        problems: p.0,
        functions,
        globals,
        structs: structs_rows,
        fields,
        docs,
        method,
        extents,
        doc_tests,
    })
}

fn decimal(text: &str) -> Option<u64> {
    if DECIMAL.is_match(text) {
        text.parse().ok()
    } else {
        None
    }
}

fn read_table(tree: &dyn Tree, kind: Kind, p: &mut Problems) -> io::Result<Vec<Row>> {
    let file = kind.file();
    let Some(text) = read(tree, file, p)? else {
        return Ok(Vec::new());
    };
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    } else {
        p.add(
            format!("{file}:{}", lines.len()),
            "does not end with a newline",
        );
    }
    let header = kind.header();
    if lines.is_empty() || !lines[0].split('\t').eq(header.iter().copied()) {
        p.add(
            format!("{file}:1"),
            format!("header is not {}", header.join(" ")),
        );
        return Ok(Vec::new());
    }
    let mut rows = Vec::new();
    for (index, line) in lines.iter().enumerate().skip(1) {
        let cells: Vec<String> = line.split('\t').map(str::to_string).collect();
        if cells.len() != header.len() {
            p.add(
                format!("{file}:{}", index + 1),
                format!("{} columns, not {}", cells.len(), header.len()),
            );
            continue;
        }
        rows.push(Row {
            kind,
            line: index + 1,
            cells,
        });
    }
    for pair in rows.windows(2) {
        if pair[1].key() <= pair[0].key() {
            p.add(pair[1].place(), "out of order or duplicate key");
        }
    }
    Ok(rows)
}

/// Every `## ` heading with the text up to the next one and the heading's line number.
pub(crate) fn sections(text: &str) -> Vec<(&str, &str, usize)> {
    let headings: Vec<_> = HEADING.captures_iter(text).collect();
    let mut found = Vec::new();
    for (i, heading) in headings.iter().enumerate() {
        let whole = heading.get(0).expect("the match");
        let end = headings
            .get(i + 1)
            .map_or(text.len(), |next| next.get(0).expect("the match").start());
        let line = text[..whole.start()].matches('\n').count() + 1;
        found.push((
            heading.get(1).expect("the title").as_str(),
            &text[whole.end()..end],
            line,
        ));
    }
    found
}

/// The anchor GitHub gives a heading: lowercase, punctuation dropped, spaces to hyphens.
pub fn slug(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter(|&c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

pub(crate) fn type_size(ty: &str, structs: &HashMap<&str, &Row>) -> Option<u64> {
    let ty = ty.trim();
    if let Some(found) = FUNCTION_POINTERS.captures(ty) {
        return found[1].parse::<u64>().ok()?.checked_mul(4);
    }
    if let Some(found) = ARRAY.captures(ty) {
        let unit = type_size(&found[1], structs)?;
        return unit.checked_mul(found[2].parse().ok()?);
    }
    if ty.ends_with('*') {
        return Some(4);
    }
    if let Some(rest) = ty.strip_prefix("const ") {
        return type_size(rest, structs);
    }
    let base = match ty {
        "char" | "int8_t" | "uint8_t" | "bool" => Some(1),
        "int16_t" | "uint16_t" => Some(2),
        "int" | "unsigned int" | "int32_t" | "uint32_t" | "float" => Some(4),
        "double" | "int64_t" | "uint64_t" => Some(8),
        _ => None,
    };
    base.or_else(|| structs.get(ty).and_then(|row| decimal(row.get("size"))))
}

/// Every row's system is a system with a doc, a library, `unassigned`, or a name listed in
/// `symbols/README.md` as a system whose doc is not written yet; a listed name has no doc.
fn check_systems(
    tree: &dyn Tree,
    written: &HashSet<&str>,
    tables: [&Vec<Row>; 4],
    p: &mut Problems,
) -> io::Result<()> {
    const README: &str = "symbols/README.md";
    let mut listed: HashMap<String, usize> = HashMap::new();
    if let Some(text) = read(tree, README, p)? {
        let mut inside = false;
        let mut found = false;
        for (number, line) in text.split('\n').enumerate() {
            if ANY_HEADING.is_match(line) {
                inside = line == UNWRITTEN;
                found |= inside;
            } else if inside {
                if let Some(name) = LISTED_SYSTEM.captures(line) {
                    let name = name[1].to_string();
                    let place = format!("{README}:{}", number + 1);
                    if written.contains(name.as_str()) {
                        p.add(&place, format!("{name} is listed but has a doc"));
                    } else if LIBRARIES.contains(&name.as_str()) || name == "unassigned" {
                        p.add(
                            &place,
                            format!("{name} is listed but is a library or unassigned"),
                        );
                    } else if listed.insert(name.clone(), number + 1).is_some() {
                        p.add(&place, format!("{name} is listed twice"));
                    }
                }
            }
        }
        if !found {
            p.add(
                format!("{README}:1"),
                format!("has no \"{UNWRITTEN}\" section"),
            );
        }
    }
    for rows in tables {
        for row in rows.iter() {
            let system = row.get("system");
            if SYSTEM.is_match(system)
                && !written.contains(system)
                && !LIBRARIES.contains(&system)
                && system != "unassigned"
                && !listed.contains_key(system)
            {
                p.add(
                    row.place(),
                    format!(
                        "system '{system}' has no doc, is no library and is not listed in {README}"
                    ),
                );
            }
        }
    }
    Ok(())
}

fn check_common(
    row: &Row,
    anchors: &HashMap<String, String>,
    mention: Option<&str>,
    p: &mut Problems,
) {
    let place = row.place();
    let meaning = row.get("meaning");
    if meaning.is_empty() {
        p.add(&place, "meaning is empty");
    } else {
        let length = meaning.chars().count();
        if length > MEANING {
            p.add(
                &place,
                format!("meaning is {length} characters, over {MEANING}"),
            );
        }
        if meaning.ends_with('.') {
            p.add(&place, "meaning ends with a period");
        }
        if !VERB.is_match(meaning) {
            p.add(&place, "meaning does not start with a verb");
        }
        if let Some(banned) = MEANING_BANNED.find(meaning) {
            p.add(
                &place,
                format!("meaning has a banned word: {}", banned.as_str()),
            );
        }
    }
    let system = row.get("system");
    if !SYSTEM.is_match(system) {
        p.add(&place, format!("system '{system}' is not a lowercase word"));
    }
    let (confidence, proof, finding) =
        (row.get("confidence"), row.get("proof"), row.get("finding"));
    match proofs(confidence) {
        None => p.add(
            &place,
            format!("confidence '{confidence}' is not verified or inferred"),
        ),
        Some(allowed) if !allowed.contains(&proof) => p.add(
            &place,
            format!("{confidence} takes proof {}", allowed.join(" or ")),
        ),
        Some(_) => {}
    }
    if confidence == "verified" && finding.is_empty() {
        p.add(&place, "verified needs a finding");
    }
    if !finding.is_empty() {
        match anchors.get(finding) {
            None => p.add(
                &place,
                format!("finding {finding} does not resolve to an entry"),
            ),
            Some(body) => {
                if let Some(address) = mention {
                    if !body.contains(address) {
                        p.add(
                            &place,
                            format!("finding {finding} does not mention {address}"),
                        );
                    }
                }
            }
        }
    }
}

/// The proofs a confidence takes, sorted.
fn proofs(confidence: &str) -> Option<&'static [&'static str]> {
    match confidence {
        "verified" => Some(&["one", "two"]),
        "inferred" => Some(&["none"]),
        _ => None,
    }
}

fn check_name(row: &Row, rule: impl Fn(&str) -> bool, what: &str, p: &mut Problems) {
    let name = row.get("name");
    if name.is_empty() {
        return;
    }
    let length = name.chars().count();
    if length > NAME {
        p.add(
            row.place(),
            format!("name is {length} characters, over {NAME}"),
        );
    }
    let library = row.get("name_from") == "idb" || LIBRARIES.contains(&row.get("system"));
    if library {
        if !LIBRARY_NAME.is_match(name) {
            p.add(row.place(), format!("name '{name}' is not an identifier"));
        }
    } else if !rule(name) {
        p.add(
            row.place(),
            format!("name '{name}' breaks the naming rule for {what}"),
        );
    }
}

/// Whether a function name passes the naming rule the check holds a row to: at most `NAME`
/// characters, an identifier for library code or a community name, else a method or free name.
pub(crate) fn valid_function_name(name: &str, library: bool) -> bool {
    name.chars().count() <= NAME
        && if library {
            LIBRARY_NAME.is_match(name)
        } else {
            method_name(name) || FREE_NAME.is_match(name)
        }
}

/// A method name: `Class::Method`, where only the class carries a `_X` suffix, or a constructor
/// or destructor, whose own part repeats its class, suffix and all (`CGObject_C::~CGObject_C`).
fn method_name(name: &str) -> bool {
    if METHOD_NAME.is_match(name) {
        return true;
    }
    let mut parts = name.rsplitn(2, "::");
    let (Some(own), Some(scope)) = (parts.next(), parts.next()) else {
        return false;
    };
    let class = scope.rsplit("::").next().unwrap_or(scope);
    own.strip_prefix('~').unwrap_or(own) == class
        && STRUCT_NAME.is_match(scope)
        && METHOD_NAME.is_match(&format!("{scope}::X"))
}

/// Every entry a doc cites in its prose resolves: `` `doc#anchor` `` (a bare doc name is first
/// read inside the citing doc's system) and `[text](file.md#anchor)` (relative to the doc).
fn check_citations(doc: &Doc, anchors: &HashMap<String, String>, p: &mut Problems) {
    let system = doc.key.split('/').next().unwrap_or_default();
    let folder = doc.path.rsplit_once('/').map_or("", |(dir, _)| dir);
    let mut in_code = false;
    for (index, line) in doc.text.split('\n').enumerate() {
        if line.starts_with("```") {
            in_code = !in_code;
        }
        if in_code {
            continue;
        }
        let place = format!("{}:{}", doc.path, index + 1);
        for cited in CITED_ANCHOR.captures_iter(line) {
            let (target, anchor) = (&cited[1], &cited[2]);
            let found = if target.contains('/') || target == "METHOD" {
                anchors.contains_key(&format!("{target}#{anchor}"))
            } else {
                anchors.contains_key(&format!("{system}/{target}#{anchor}"))
                    || anchors.contains_key(&format!("{target}#{anchor}"))
            };
            if !found {
                p.add(&place, format!("{target}#{anchor} cites no entry"));
            }
        }
        for linked in LINKED_ANCHOR.captures_iter(line) {
            let (file, anchor) = (&linked[1], &linked[2]);
            if file.contains("://") {
                continue;
            }
            let key = if file.is_empty() {
                Some(doc.key.clone())
            } else {
                linked_key(folder, file)
            };
            let found = key.is_some_and(|key| anchors.contains_key(&format!("{key}#{anchor}")));
            if !found {
                p.add(&place, format!("{file}#{anchor} links to no entry"));
            }
        }
    }
}

/// The anchor key of a Markdown file linked from a doc in `folder`: `systems/<key>.md`, or `METHOD`.
fn linked_key(folder: &str, file: &str) -> Option<String> {
    let mut parts: Vec<&str> = folder.split('/').filter(|s| !s.is_empty()).collect();
    for part in file.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    let path = parts.join("/");
    if path == "docs/METHOD.md" {
        return Some("METHOD".to_string());
    }
    path.strip_prefix("systems/")?
        .strip_suffix(".md")
        .map(str::to_string)
}

fn check_doc(
    doc: &Doc,
    functions: &[Row],
    doc_tests: &mut Vec<(String, String)>,
    p: &mut Problems,
) {
    let path = &doc.path;
    let text = &doc.text;
    let system = doc.key.split('/').next().unwrap_or_default();
    let total = words(text);
    if total > DOC {
        p.add(
            format!("{path}:1"),
            format!("{total} words, over {DOC}; split by topic"),
        );
    }
    let lines: Vec<&str> = text.split('\n').collect();
    if !lines[0].starts_with("# ") {
        p.add(format!("{path}:1"), "the doc starts with \"# <system>\"");
    }
    let (mut in_code, mut code_start) = (false, 0);
    for (index, line) in lines.iter().enumerate() {
        let number = index + 1;
        if line.starts_with("```") {
            if in_code && number - code_start - 1 > CODE {
                p.add(
                    format!("{path}:{code_start}"),
                    format!("code block over {CODE} lines"),
                );
            }
            in_code = !in_code;
            code_start = number;
        }
    }
    in_code = false;
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let prose = INLINE_CODE.replace_all(line, "");
        for (rule, what) in [
            (&HEDGES, "hedge"),
            (&SHOUTING, "shouting"),
            (&HISTORY, "history"),
            (&FIRST_PERSON, "first person"),
        ] {
            if let Some(found) = rule.find(&prose) {
                p.add(
                    format!("{path}:{}", index + 1),
                    format!("{what}: '{}'", found.as_str()),
                );
            }
        }
    }

    let head = text
        .split_once("\n## ")
        .map_or(text.as_str(), |(head, _)| head);
    let overview = match ROOTS.captures(head) {
        None => {
            p.add(format!("{path}:1"), "no Roots line before the first entry");
            head
        }
        Some(roots) => {
            let whole = roots.get(0).expect("the match");
            let line = head[..whole.start()].matches('\n').count() + 1;
            for root in CITED.captures_iter(&roots[1]) {
                let address = &root[1];
                let owned = functions
                    .iter()
                    .any(|r| r.get("system") == system && r.get("address") == address);
                if !owned {
                    p.add(
                        format!("{path}:{line}"),
                        format!("root {address} is not a function row of {system}"),
                    );
                }
            }
            &head[..whole.start()]
        }
    };
    let overview = overview.split_once('\n').map_or("", |(_, rest)| rest);
    let overview_words = words(overview);
    if overview_words > OVERVIEW {
        p.add(
            format!("{path}:1"),
            format!("overview is {overview_words} words, over {OVERVIEW}"),
        );
    }

    for (title, body, heading_line) in sections(text) {
        // An entry's body starts on the line after its heading.
        let Some(body) = body.strip_prefix('\n') else {
            continue;
        };
        let heading = format!("{path}:{heading_line}");
        let title_words = words(title);
        if title_words > TITLE {
            p.add(
                &heading,
                format!("title is {title_words} words, over {TITLE}"),
            );
        }
        let Some(shape) = SHAPE.captures(body) else {
            p.add(
                &heading,
                "no Addresses, Confidence, Proof and Test lines in that order",
            );
            continue;
        };
        let whole = shape.get(0).expect("the match");
        let addresses_line = heading_line + 1 + body[..whole.start()].matches('\n').count();
        let place = |below: usize| format!("{path}:{}", addresses_line + below);
        let (cited, confidence, proof, test) = (&shape[1], &shape[2], &shape[3], &shape[4]);
        if !ADDRESS_LIST.is_match(cited) {
            p.add(
                place(0),
                "Addresses is a comma-separated list of `0x……` addresses",
            );
        }
        match proofs(confidence) {
            None => p.add(
                place(1),
                format!("Confidence '{confidence}' is not verified or inferred"),
            ),
            Some(allowed) if !allowed.contains(&proof) => p.add(
                place(2),
                format!("{confidence} takes Proof {}", allowed.join(" or ")),
            ),
            Some(_) => {}
        }
        if test != "none" {
            if test.split(", ").all(|t| TEST_CITED.is_match(t)) {
                for t in test.split(", ") {
                    doc_tests.push((place(3), t.trim_matches('`').to_string()));
                }
            } else {
                p.add(place(3), "Test is none or a list of `crate::test_fn`");
            }
        }
        let (claim, derivation) = (&body[..whole.start()], &body[whole.end()..]);
        let prose: Vec<&str> = claim.split('\n').filter(|l| !l.starts_with('|')).collect();
        let claim_words = words(&prose.join("\n"));
        if claim_words > CLAIM {
            p.add(
                &heading,
                format!("claim is {claim_words} words, over {CLAIM}"),
            );
        }
        let mut table_rows = claim
            .split('\n')
            .enumerate()
            .filter(|(_, l)| l.starts_with('|') && !TABLE_RULE.is_match(l));
        if let Some((first, _)) = table_rows.next() {
            let rows = table_rows.count();
            if rows > TABLE {
                p.add(
                    format!("{path}:{}", heading_line + 1 + first),
                    format!("table has {rows} rows, over {TABLE}"),
                );
            }
        }
        let derivation_words = words(derivation);
        if derivation_words > DERIVATION {
            p.add(
                &heading,
                format!("derivation is {derivation_words} words, over {DERIVATION}"),
            );
        }
    }
}

#[cfg(test)]
mod tests;
