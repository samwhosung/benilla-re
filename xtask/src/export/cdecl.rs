//! The C declarations of the tables, read just far enough to export them: which type names a
//! declaration mentions and whether it needs each one complete, and where its name stands or goes.
//!
//! The grammar is the one the tables use: specifiers (`const`, `struct X`, `unsigned int`, a type
//! name), stars, then a name or a function pointer group `(*name[4])(params)`, then array
//! dimensions. A declaration outside it is an error, and the row that holds it is left out of
//! the header.

use std::collections::BTreeSet;
use std::sync::LazyLock;

/// The words that spell a built-in type.
const WORDS: [&str; 9] = [
    "void", "char", "short", "int", "long", "float", "double", "unsigned", "signed",
];

/// The type names the header's includes define.
const DEFINED: [&str; 14] = [
    "bool",
    "int8_t",
    "int16_t",
    "int32_t",
    "int64_t",
    "uint8_t",
    "uint16_t",
    "uint32_t",
    "uint64_t",
    "intptr_t",
    "uintptr_t",
    "size_t",
    "ptrdiff_t",
    "va_list",
];

/// Whether a name is a built-in or one of the header's own includes defines it.
pub fn is_defined(name: &str) -> bool {
    WORDS.contains(&name) || DEFINED.contains(&name) || name == "wchar_t"
}

/// The words of C and C++ that a name cannot be, and the names the includes define as macros.
pub static RESERVED: LazyLock<BTreeSet<&'static str>> = LazyLock::new(|| {
    [
        "auto",
        "break",
        "case",
        "char",
        "const",
        "continue",
        "default",
        "do",
        "double",
        "else",
        "enum",
        "extern",
        "float",
        "for",
        "goto",
        "if",
        "inline",
        "int",
        "long",
        "register",
        "restrict",
        "return",
        "short",
        "signed",
        "sizeof",
        "static",
        "struct",
        "switch",
        "typedef",
        "union",
        "unsigned",
        "void",
        "volatile",
        "while",
        "bool",
        "true",
        "false",
        "NULL",
        "offsetof",
        "va_list",
        "va_start",
        "va_end",
        "va_arg",
        "wchar_t",
        "alignas",
        "alignof",
        "and",
        "and_eq",
        "asm",
        "bitand",
        "bitor",
        "catch",
        "class",
        "compl",
        "concept",
        "const_cast",
        "constexpr",
        "decltype",
        "delete",
        "dynamic_cast",
        "explicit",
        "export",
        "friend",
        "mutable",
        "namespace",
        "new",
        "noexcept",
        "not",
        "not_eq",
        "nullptr",
        "operator",
        "or",
        "or_eq",
        "private",
        "protected",
        "public",
        "reinterpret_cast",
        "requires",
        "static_assert",
        "static_cast",
        "template",
        "this",
        "thread_local",
        "throw",
        "try",
        "typeid",
        "typename",
        "using",
        "virtual",
        "xor",
        "xor_eq",
    ]
    .into_iter()
    .collect()
});

/// The C identifier of a table name: `::` becomes `__`, and a destructor `Class::~Class` becomes
/// `Class__dtor`. None for a name that is no identifier after that, such as `_WinMain@16`.
pub fn c_name(name: &str) -> Option<String> {
    let mut segments: Vec<&str> = name.split("::").collect();
    if let Some(last) = segments.last_mut() {
        if last.starts_with('~') {
            *last = "dtor";
        }
    }
    let valid = segments.iter().all(|segment| {
        let mut chars = segment.chars();
        matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    });
    valid.then(|| segments.join("__"))
}

/// A type name a declaration mentions, and whether the declaration holds one by value, so that
/// the type must be complete. A pointer to it, or any use inside a function type, needs only the
/// name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mention {
    pub name: String,
    pub by_value: bool,
}

/// What one declaration says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decl {
    /// The type names it mentions that are not built in, in order, nested ones included.
    pub mentions: Vec<Mention>,
    /// The byte range of the declared name, when it has one.
    pub name: Option<(usize, usize)>,
    /// Where a name goes when there is none: after the stars, inside a function pointer's group.
    pub insert_at: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok {
    Ident(String),
    Number,
    Star,
    Open,
    Close,
    OpenIndex,
    CloseIndex,
    Comma,
    Ellipsis,
}

#[derive(Clone, Debug)]
struct Token {
    tok: Tok,
    start: usize,
    end: usize,
}

fn tokens(s: &str) -> Result<Vec<Token>, String> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let start = i;
        let tok = match c {
            b' ' => {
                i += 1;
                continue;
            }
            b'*' => Tok::Star,
            b'(' => Tok::Open,
            b')' => Tok::Close,
            b'[' => Tok::OpenIndex,
            b']' => Tok::CloseIndex,
            b',' => Tok::Comma,
            b'.' if s[i..].starts_with("...") => {
                i += 2;
                Tok::Ellipsis
            }
            b'0'..=b'9' => {
                while i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
                    i += 1;
                }
                Tok::Number
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                loop {
                    while i + 1 < bytes.len()
                        && (bytes[i + 1].is_ascii_alphanumeric() || bytes[i + 1] == b'_')
                    {
                        i += 1;
                    }
                    let next = s.get(i + 1..i + 4).unwrap_or_default();
                    let joined = next.starts_with("::")
                        && matches!(bytes.get(i + 3), Some(b) if b.is_ascii_alphabetic() || *b == b'_');
                    if !joined {
                        break;
                    }
                    i += 3;
                }
                Tok::Ident(s[start..=i].to_string())
            }
            _ => {
                let rest: String = s[i..].chars().take(8).collect();
                return Err(format!("cannot read {rest:?}"));
            }
        };
        i += 1;
        out.push(Token { tok, start, end: i });
    }
    Ok(out)
}

struct Parser<'a> {
    s: &'a str,
    toks: Vec<Token>,
    pos: usize,
    mentions: Vec<Mention>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.tok)
    }

    fn eat(&mut self, tok: &Tok) -> bool {
        if self.peek() == Some(tok) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn qualifier(&self) -> bool {
        matches!(self.peek(), Some(Tok::Ident(w)) if w == "const" || w == "volatile")
    }

    /// The specifiers; the type name they name, if any.
    fn specifiers(&mut self) -> Result<Option<String>, String> {
        let mut have_base = false;
        let mut named = None;
        while let Some(Tok::Ident(word)) = self.peek().cloned() {
            if word == "const" || word == "volatile" {
                self.pos += 1;
            } else if word == "struct" || word == "union" || word == "enum" {
                if have_base {
                    break;
                }
                self.pos += 1;
                match self.peek().cloned() {
                    Some(Tok::Ident(tag)) => {
                        named = Some(tag);
                        have_base = true;
                        self.pos += 1;
                    }
                    _ => return Err(format!("no tag after {word}")),
                }
            } else if WORDS.contains(&word.as_str()) {
                have_base = true;
                self.pos += 1;
            } else if have_base {
                break;
            } else {
                if !is_defined(&word) {
                    named = Some(word);
                }
                have_base = true;
                self.pos += 1;
            }
        }
        if have_base {
            Ok(named)
        } else {
            Err("no type".to_string())
        }
    }

    fn dimensions(&mut self) -> Result<(), String> {
        while self.eat(&Tok::OpenIndex) {
            self.eat(&Tok::Number);
            if !self.eat(&Tok::CloseIndex) {
                return Err("unclosed [".to_string());
            }
        }
        Ok(())
    }

    /// The parameter list of a function type, its `(` already read: the mentions of each
    /// parameter, none of them by value.
    fn parameters(&mut self) -> Result<(), String> {
        let first = self.pos;
        let mut depth = 1;
        while depth > 0 {
            match self.toks.get(self.pos).map(|t| &t.tok) {
                Some(Tok::Open) => depth += 1,
                Some(Tok::Close) => depth -= 1,
                Some(_) => {}
                None => return Err("unclosed (".to_string()),
            }
            self.pos += 1;
        }
        let last = self.pos - 1;
        let mut start = first;
        let mut depth = 0;
        for i in first..=last {
            match &self.toks[i].tok {
                Tok::Open => depth += 1,
                Tok::Close if i < last => depth -= 1,
                Tok::Comma if depth == 0 => {
                    self.nested(start, i)?;
                    start = i + 1;
                }
                _ => {}
            }
        }
        self.nested(start, last)
    }

    fn nested(&mut self, from: usize, to: usize) -> Result<(), String> {
        if from == to {
            return Ok(());
        }
        let text = &self.s[self.toks[from].start..self.toks[to - 1].end];
        if text == "..." || text == "void" {
            return Ok(());
        }
        for mention in parse(text)?.mentions {
            self.mentions.push(Mention {
                name: mention.name,
                by_value: false,
            });
        }
        Ok(())
    }

    fn declaration(mut self) -> Result<Decl, String> {
        let named = self.specifiers()?;
        let mut stars = 0;
        let mut insert_at = self.toks[self.pos - 1].end;
        while matches!(self.peek(), Some(Tok::Star)) || self.qualifier() {
            if self.peek() == Some(&Tok::Star) {
                stars += 1;
            }
            insert_at = self.toks[self.pos].end;
            self.pos += 1;
        }
        let mut name = None;
        let mut by_value = stars == 0;
        if self.eat(&Tok::Open) {
            by_value = false;
            let mut inner = 0;
            insert_at = self.toks[self.pos - 1].end;
            while matches!(self.peek(), Some(Tok::Star)) || self.qualifier() {
                if self.peek() == Some(&Tok::Star) {
                    inner += 1;
                }
                insert_at = self.toks[self.pos].end;
                self.pos += 1;
            }
            if inner == 0 {
                return Err("a function type, not a pointer to one".to_string());
            }
            if let Some(Tok::Ident(_)) = self.peek() {
                name = Some((self.toks[self.pos].start, self.toks[self.pos].end));
                self.pos += 1;
            }
            self.dimensions()?;
            if !self.eat(&Tok::Close) || !self.eat(&Tok::Open) {
                return Err("a function pointer without its parameters".to_string());
            }
            self.parameters()?;
        } else {
            if let Some(Tok::Ident(_)) = self.peek() {
                name = Some((self.toks[self.pos].start, self.toks[self.pos].end));
                self.pos += 1;
            }
            self.dimensions()?;
        }
        if self.pos != self.toks.len() {
            return Err(format!("unread: {}", &self.s[self.toks[self.pos].start..]));
        }
        if let Some(type_name) = named {
            self.mentions.insert(
                0,
                Mention {
                    name: type_name,
                    by_value,
                },
            );
        }
        Ok(Decl {
            mentions: self.mentions,
            name,
            insert_at,
        })
    }
}

/// Reads one declaration: a field's or global's type, a parameter, a return type.
pub fn parse(s: &str) -> Result<Decl, String> {
    if s.trim().is_empty() {
        return Err("empty".to_string());
    }
    let toks = tokens(s)?;
    Parser {
        s,
        toks,
        pos: 0,
        mentions: Vec::new(),
    }
    .declaration()
}

/// The text of `decl` with `name` as its declared name, in place of the one it has or where it
/// goes; `::` is `__` throughout.
pub fn with_name(text: &str, decl: &Decl, name: &str) -> String {
    let mut out = String::with_capacity(text.len() + name.len() + 1);
    match decl.name {
        Some((start, end)) => {
            out.push_str(&text[..start]);
            out.push_str(name);
            out.push_str(&text[end..]);
        }
        None => {
            out.push_str(&text[..decl.insert_at]);
            if !out.ends_with(['*', '(']) {
                out.push(' ');
            }
            out.push_str(name);
            out.push_str(&text[decl.insert_at..]);
        }
    }
    out.replace("::", "__")
}

/// The text of a declaration with its `::` written `__`.
pub fn mangled(text: &str) -> String {
    text.replace("::", "__")
}

/// The top-level, comma-separated parts of a parameter list.
pub fn split_parameters(list: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in list.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(list[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if !list[start..].trim().is_empty() {
        parts.push(list[start..].trim());
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mention(name: &str, by_value: bool) -> Mention {
        Mention {
            name: name.to_string(),
            by_value,
        }
    }

    #[test]
    fn a_name_is_a_c_identifier_or_nothing() {
        assert_eq!(
            c_name("CDataStore::PutInt8").unwrap(),
            "CDataStore__PutInt8"
        );
        assert_eq!(c_name("CCamera::~CCamera").unwrap(), "CCamera__dtor");
        assert_eq!(c_name("NTempest::CRndSeed").unwrap(), "NTempest__CRndSeed");
        assert_eq!(c_name("_srand").unwrap(), "_srand");
        assert_eq!(c_name("_WinMain@16"), None);
        assert_eq!(c_name("A::"), None);
        assert_eq!(c_name("1abc"), None);
        assert_eq!(c_name(""), None);
    }

    #[test]
    fn a_pointer_needs_the_name_and_a_value_needs_the_type() {
        let d = parse("CMovement *this").unwrap();
        assert_eq!(d.mentions, [mention("CMovement", false)]);
        assert_eq!(d.name, Some((11, 15)));
        let d = parse("const NTempest::CRndSeed seed").unwrap();
        assert_eq!(d.mentions, [mention("NTempest::CRndSeed", true)]);
        let d = parse("uint32_t time").unwrap();
        assert!(d.mentions.is_empty());
        assert_eq!(d.name, Some((9, 13)));
        let d = parse("struct sockaddr_in *addr").unwrap();
        assert_eq!(d.mentions, [mention("sockaddr_in", false)]);
        let d = parse("unsigned int x").unwrap();
        assert!(d.mentions.is_empty());
        assert_eq!(d.name, Some((13, 14)));
    }

    #[test]
    fn arrays_put_the_name_before_the_dimensions() {
        let d = parse("uint8_t[16]").unwrap();
        assert_eq!(
            with_name("uint8_t[16]", &d, "m_digest"),
            "uint8_t m_digest[16]"
        );
        let d = parse("char[8][4]").unwrap();
        assert_eq!(with_name("char[8][4]", &d, "s_x"), "char s_x[8][4]");
        let d = parse("const char *[3]").unwrap();
        assert_eq!(
            with_name("const char *[3]", &d, "s_x"),
            "const char *s_x[3]"
        );
        let d = parse("uint32_t *out[6]").unwrap();
        assert_eq!(
            with_name("uint32_t *out[6]", &d, "out_2"),
            "uint32_t *out_2[6]"
        );
        let d = parse("void **").unwrap();
        assert_eq!(with_name("void **", &d, "m_vtable"), "void **m_vtable");
    }

    #[test]
    fn a_function_pointer_takes_its_name_inside_the_group() {
        let d = parse("void (*[4])(void)").unwrap();
        assert_eq!(
            with_name("void (*[4])(void)", &d, "s_hooks"),
            "void (*s_hooks[4])(void)"
        );
        let d = parse("void (*function)(void)").unwrap();
        assert_eq!(d.name, Some((7, 15)));
        let d = parse("void (**begin)(void)").unwrap();
        assert_eq!(
            with_name("void (**begin)(void)", &d, "end"),
            "void (**end)(void)"
        );
        let d = parse("void (*)(CGxFont *font, uint32_t, ...)").unwrap();
        assert_eq!(d.mentions, [mention("CGxFont", false)]);
        assert_eq!(
            with_name("void (*)(CGxFont *font, uint32_t, ...)", &d, "cb"),
            "void (*cb)(CGxFont *font, uint32_t, ...)"
        );
        let d = parse("Thing (*callback)(Other value)").unwrap();
        assert_eq!(
            d.mentions,
            [mention("Thing", false), mention("Other", false)]
        );
    }

    #[test]
    fn what_the_grammar_does_not_cover_is_an_error() {
        for bad in [
            "char *name@<ecx>",
            "/* nine record fields */ void *outHandle",
            "void (void)",
            "",
            "int *p q",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn parameters_split_at_the_top_level() {
        assert_eq!(
            split_parameters("int a, void (*f)(int, int), uint8_t d[16], ..."),
            ["int a", "void (*f)(int, int)", "uint8_t d[16]", "..."]
        );
        assert!(split_parameters("").is_empty());
    }
}
