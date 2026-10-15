//! Markdown to HTML, with every `0x` address in prose linked to its address page and every
//! relative link pointed at the page that renders its target.

use super::model::{Item, Model};
use super::resolve::Spans;
use crate::check::slug;
use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::sync::LazyLock;

pub const REPO: &str = "https://github.com/samwhosung/benilla-re";

static ADDRESS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b0x[0-9a-f]{6}\b").expect("a valid pattern"));

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// `../` once for each directory a page sits in.
pub fn root_for(page: &str) -> String {
    "../".repeat(page.matches('/').count())
}

/// The path of an address page: stable, from the row's start address alone.
pub fn address_page(address: u32) -> String {
    format!("a/{address:06x}.html")
}

/// The path of a struct's page; `::` becomes `--` so the name is a file name.
pub fn struct_page(name: &str) -> String {
    format!("t/{}.html", name.replace("::", "--"))
}

pub struct Linker<'a> {
    pub model: &'a Model,
    pub spans: &'a Spans,
}

impl Linker<'_> {
    pub fn row(&self, address: u32) -> Option<&Item> {
        self.spans.holding(address).map(|k| &self.model.items[k])
    }

    /// The href of an address: its row's page, and for an address inside the row, the
    /// instruction as the fragment.
    pub fn href(&self, address: u32, root: &str) -> Option<String> {
        let row = self.row(address)?;
        let fragment = if row.address == address {
            String::new()
        } else {
            format!("#0x{address:06x}")
        };
        Some(format!("{root}{}{fragment}", address_page(row.address)))
    }

    /// Text with each address that has a row as a link, everything else escaped.
    pub fn text(&self, text: &str, root: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        for found in ADDRESS.find_iter(text) {
            let address = u32::from_str_radix(&found.as_str()[2..], 16).unwrap_or(0);
            let Some(href) = self.href(address, root) else {
                continue;
            };
            out.push_str(&escape(&text[last..found.start()]));
            out.push_str(&format!(
                "<a class=\"addr\" href=\"{href}\">{}</a>",
                found.as_str()
            ));
            last = found.end();
        }
        out.push_str(&escape(&text[last..]));
        out
    }

    /// Where a link in the doc at `doc_path` points on the site.
    pub fn destination(&self, doc_path: &str, dest: &str, root: &str) -> String {
        if dest.starts_with('#') || dest.contains("://") || dest.starts_with("mailto:") {
            return dest.to_string();
        }
        let (path, fragment) = match dest.split_once('#') {
            Some((p, f)) => (p, format!("#{f}")),
            None => (dest, String::new()),
        };
        let mut parts: Vec<&str> = doc_path.split('/').collect();
        parts.pop();
        for segment in path.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                s => parts.push(s),
            }
        }
        let joined = parts.join("/");
        if self.model.docs.iter().any(|d| d.path == joined) {
            return format!("{root}d/{}.html{fragment}", joined.trim_end_matches(".md"));
        }
        if let Some(system) = joined.strip_prefix("systems/") {
            if self.model.systems.contains(system) {
                return format!("{root}s/{system}.html{fragment}");
            }
        }
        if joined == "systems" {
            return format!("{root}index.html#systems");
        }
        if let Some(name) = joined
            .strip_prefix("tables/")
            .and_then(|r| r.strip_suffix(".tsv"))
        {
            if self.model.tables.iter().any(|t| t.name == name) {
                return format!("{root}tb/{name}.html{fragment}");
            }
        }
        let kind = if self.model.files.contains(&joined) || joined.is_empty() {
            "blob"
        } else if self
            .model
            .files
            .iter()
            .any(|f| f.starts_with(&format!("{joined}/")))
        {
            "tree"
        } else {
            "blob"
        };
        format!("{REPO}/{kind}/main/{joined}{fragment}")
    }

    /// The HTML of a markdown text that lives in `doc_path`, for a page whose root is `root`.
    pub fn render(&self, markdown: &str, doc_path: &str, root: &str) -> String {
        let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
        let events: Vec<Event> = Parser::new_ext(markdown, options).collect();
        let mut out: Vec<Event> = Vec::with_capacity(events.len());
        let mut in_link = 0usize;
        let mut in_block = false;
        let mut k = 0;
        while k < events.len() {
            let event = events[k].clone();
            k += 1;
            out.push(match event {
                Event::Start(Tag::Heading {
                    level,
                    classes,
                    attrs,
                    ..
                }) => {
                    let mut title = String::new();
                    for e in &events[k..] {
                        match e {
                            Event::End(TagEnd::Heading(_)) => break,
                            Event::Text(t) | Event::Code(t) => title.push_str(t),
                            _ => {}
                        }
                    }
                    Event::Start(Tag::Heading {
                        level,
                        id: Some(CowStr::from(slug(&title))),
                        classes,
                        attrs,
                    })
                }
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) => {
                    in_link += 1;
                    Event::Start(Tag::Link {
                        link_type,
                        dest_url: CowStr::from(self.destination(doc_path, &dest_url, root)),
                        title,
                        id,
                    })
                }
                Event::End(TagEnd::Link) => {
                    in_link = in_link.saturating_sub(1);
                    Event::End(TagEnd::Link)
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_block = true;
                    Event::Start(Tag::CodeBlock(kind))
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_block = false;
                    Event::End(TagEnd::CodeBlock)
                }
                Event::Text(t) if in_link == 0 && !in_block => {
                    Event::Html(CowStr::from(self.text(&t, root)))
                }
                Event::Code(c) if in_link == 0 => Event::Html(CowStr::from(format!(
                    "<code>{}</code>",
                    self.text(&c, root)
                ))),
                Event::Html(h) | Event::InlineHtml(h) => Event::Text(h),
                other => other,
            });
        }
        let mut html_out = String::new();
        html::push_html(&mut html_out, out.into_iter());
        html_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::site::model::{Doc, Item};
    use std::collections::{BTreeSet, HashMap};

    fn item(address: u32, size: u32, name: &str) -> Item {
        Item {
            address,
            size,
            is_function: true,
            name: name.into(),
            name_from: String::new(),
            system: "demo".into(),
            class: String::new(),
            shape: String::new(),
            meaning: String::new(),
            confidence: "inferred".into(),
            proof: "none".into(),
            finding: String::new(),
            test: String::new(),
        }
    }

    fn model() -> Model {
        Model {
            items: vec![item(0x5e6850, 0x30, "Advance"), item(0x800000, 4, "")],
            structs: vec![],
            fields: vec![],
            docs: vec![Doc {
                path: "systems/demo.md".into(),
                key: "systems/demo".into(),
                title: "demo".into(),
                text: String::new(),
                system: "demo".into(),
            }],
            tables: vec![],
            callees: Default::default(),
            callers: Default::default(),
            files: BTreeSet::from(["symbols/README.md".to_string()]),
            tests: HashMap::new(),
            systems: BTreeSet::from(["demo".to_string(), "ui".to_string()]),
        }
    }

    fn with<T>(f: impl FnOnce(&Linker) -> T) -> T {
        let m = model();
        let spans = Spans::new(m.items.iter().map(|i| (i.address, i.size)));
        f(&Linker {
            model: &m,
            spans: &spans,
        })
    }

    #[test]
    fn a_row_address_in_text_links_to_its_page() {
        let html = with(|l| l.text("calls 0x5e6850 and 0x123456.", "../"));
        assert_eq!(
            html,
            "calls <a class=\"addr\" href=\"../a/5e6850.html\">0x5e6850</a> and 0x123456."
        );
    }

    #[test]
    fn an_instruction_address_links_to_its_function_at_the_instruction() {
        let html = with(|l| l.text("0x5e6853", ""));
        assert_eq!(
            html,
            "<a class=\"addr\" href=\"a/5e6850.html#0x5e6853\">0x5e6853</a>"
        );
    }

    #[test]
    fn a_longer_hex_constant_is_not_an_address() {
        assert_eq!(with(|l| l.text("0x5e68500", "")), "0x5e68500");
    }

    #[test]
    fn prose_and_inline_code_link_and_a_code_block_does_not() {
        let html = with(|l| {
            l.render(
                "At 0x5e6850 and `0x800000`.\n\n```\n0x5e6850\n```\n",
                "systems/demo.md",
                "../../",
            )
        });
        assert!(html.contains("<a class=\"addr\" href=\"../../a/5e6850.html\">0x5e6850</a> and"));
        assert!(html
            .contains("<code><a class=\"addr\" href=\"../../a/800000.html\">0x800000</a></code>"));
        assert!(html.contains("<pre><code>0x5e6850\n</code></pre>"));
    }

    #[test]
    fn an_address_inside_link_text_stays_one_link() {
        let html = with(|l| l.render("[see 0x5e6850](x.md)", "systems/demo.md", ""));
        assert_eq!(html.matches("<a ").count(), 1);
    }

    #[test]
    fn relative_links_reach_the_page_that_renders_the_target() {
        with(|l| {
            let d = |dest: &str| l.destination("systems/ui/frames.md", dest, "../../../");
            assert_eq!(d("#entry"), "#entry");
            assert_eq!(d("../demo.md#x"), "../../../d/systems/demo.html#x");
            assert_eq!(d("../ui"), "../../../s/ui.html");
            assert_eq!(
                d("../../symbols/README.md#names"),
                "https://github.com/samwhosung/benilla-re/blob/main/symbols/README.md#names"
            );
            assert_eq!(
                d("../../crates/x"),
                "https://github.com/samwhosung/benilla-re/blob/main/crates/x"
            );
            assert_eq!(d("https://example.org/"), "https://example.org/");
        });
    }

    #[test]
    fn a_heading_gets_the_anchor_rows_link_to() {
        let html = with(|l| l.render("## State advance\n\nx", "systems/demo.md", ""));
        assert!(html.contains("<h2 id=\"state-advance\">State advance</h2>"));
    }
}
