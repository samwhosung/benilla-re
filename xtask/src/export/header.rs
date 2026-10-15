//! `benilla-re.h`: the structs, prototypes and globals of the symbol table as a C header.
//!
//! Every struct is written packed with explicit padding, so a member sits at its table offset on
//! any compiler, and every size and offset is asserted behind `BENILLA_RE_LAYOUT_CHECK`. A
//! prototype or a global whose declaration the header cannot state is left out and counted.

use super::cdecl::{self, c_name, Decl, RESERVED};
use super::table::Table;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;

/// The four tables of `symbols/`.
pub struct Symbols {
    pub structs: Table,
    pub fields: Table,
    pub functions: Table,
    pub globals: Table,
}

/// The client the tables describe and the tree they come from, for the header's banner.
pub struct Origin {
    pub exe_size: u64,
    pub exe_sha256: String,
    pub commit: String,
    /// The addresses are matches found in another build, not 5875's.
    pub rebased: bool,
}

/// What the header holds and what it leaves out.
#[derive(Debug, Default)]
pub struct Report {
    pub structs: usize,
    pub fields: usize,
    /// Fields written as bytes because their type is not complete here.
    pub bytes_for_fields: Vec<String>,
    pub prototypes: usize,
    pub globals: usize,
    /// Type names the header declares without defining.
    pub opaque: Vec<String>,
    /// Rows left out, by reason: each row as its address and name.
    pub omitted: BTreeMap<String, Vec<String>>,
    /// Function rows with no signature.
    pub unsigned_functions: usize,
}

impl Report {
    pub fn omitted_total(&self) -> usize {
        self.omitted.values().map(Vec::len).sum()
    }

    /// The summary the export prints.
    pub fn summary(&self) -> String {
        let mut text = format!(
            "{} structs, {} fields, {} prototypes, {} globals, {} opaque types",
            self.structs,
            self.fields,
            self.prototypes,
            self.globals,
            self.opaque.len()
        );
        if !self.bytes_for_fields.is_empty() {
            let _ = write!(
                text,
                "; {} fields written as bytes",
                self.bytes_for_fields.len()
            );
        }
        let _ = write!(
            text,
            "; {} functions without a signature; {} rows left out",
            self.unsigned_functions,
            self.omitted_total()
        );
        for (reason, rows) in &self.omitted {
            let _ = write!(text, "\n  {} {reason}", rows.len());
        }
        text
    }
}

pub struct Header {
    pub text: String,
    pub report: Report,
}

struct Field {
    offset: u32,
    size: u32,
    ty: String,
    name: String,
    meaning: String,
}

struct Struct {
    name: String,
    c: String,
    size: u32,
    vtable: String,
    meaning: String,
    fields: Vec<Field>,
}

fn number(file: &str, line: usize, cell: &str) -> Result<u32, String> {
    let parsed = match cell.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => cell.parse(),
    };
    parsed.map_err(|_| format!("{file}:{line}: {cell:?} is not a number"))
}

fn structs_of(symbols: &Symbols) -> Result<Vec<Struct>, String> {
    let t = &symbols.structs;
    let (name, size, vtable, meaning) = (
        t.column("name")?,
        t.column("size")?,
        t.column("vtable")?,
        t.column("meaning")?,
    );
    let mut out: Vec<Struct> = Vec::new();
    for (i, row) in t.rows.iter().enumerate() {
        let c = c_name(&row[name])
            .ok_or_else(|| format!("structs.tsv:{}: {} is no C name", i + 2, row[name]))?;
        out.push(Struct {
            name: row[name].clone(),
            c,
            size: number("structs.tsv", i + 2, &row[size])?,
            vtable: row[vtable].clone(),
            meaning: row[meaning].clone(),
            fields: Vec::new(),
        });
    }
    let index: HashMap<String, usize> = out
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.clone(), i))
        .collect();
    let f = &symbols.fields;
    let (owner, offset, fsize, ty, fname, fmeaning) = (
        f.column("struct")?,
        f.column("offset")?,
        f.column("size")?,
        f.column("type")?,
        f.column("name")?,
        f.column("meaning")?,
    );
    for (i, row) in f.rows.iter().enumerate() {
        let line = i + 2;
        let at = *index
            .get(&row[owner])
            .ok_or_else(|| format!("fields.tsv:{line}: no struct {}", row[owner]))?;
        out[at].fields.push(Field {
            offset: number("fields.tsv", line, &row[offset])?,
            size: number("fields.tsv", line, &row[fsize])?,
            ty: row[ty].clone(),
            name: row[fname].clone(),
            meaning: row[fmeaning].clone(),
        });
    }
    for s in &mut out {
        s.fields.sort_by_key(|f| f.offset);
    }
    Ok(out)
}

/// A comment's text with nothing in it that ends the comment.
fn comment(text: &str) -> String {
    text.replace("*/", "* /").replace("/*", "/ *")
}

/// One member of a struct body.
struct Member {
    text: String,
}

/// A struct's members, the structs it holds by value, and its layout assertions.
struct Body {
    members: Vec<Member>,
    deps: Vec<usize>,
    asserts: Vec<String>,
}

/// The type names every declaration of the tables mentions, as C names: the ones a function or
/// global cannot also be.
fn type_names(symbols: &Symbols, structs: &[Struct]) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = structs.iter().map(|s| s.c.clone()).collect();
    let mut note = |text: &str| {
        if let Ok(decl) = cdecl::parse(text) {
            names.extend(decl.mentions.iter().map(|m| cdecl::mangled(&m.name)));
        }
    };
    for row in &symbols.globals.rows {
        note(&row[1]);
    }
    for s in structs {
        for f in &s.fields {
            note(&f.ty);
        }
    }
    for row in &symbols.functions.rows {
        let signature = &row[6];
        if let Some(parts) = split_signature(signature) {
            note(&parts.ret);
            for p in cdecl::split_parameters(&parts.params) {
                if p != "..." && p != "void" {
                    note(p);
                }
            }
        }
    }
    names
}

struct Signature {
    ret: String,
    convention: String,
    params: String,
}

/// `RET CONVENTION (PARAMETERS)`, or None.
fn split_signature(signature: &str) -> Option<Signature> {
    use regex::Regex;
    use std::sync::LazyLock;
    static SHAPE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(.+?)\s*\b(__cdecl|__stdcall|__fastcall|__thiscall)\s*\((.*)\)$")
            .expect("a valid pattern")
    });
    let c = SHAPE.captures(signature)?;
    Some(Signature {
        ret: c[1].trim().to_string(),
        convention: c[2].to_string(),
        params: c[3].to_string(),
    })
}

/// SDK types the header can declare opaquely, with the tag the SDK gives each: the typedef must
/// repeat the tag, or it redefines the SDK's after `windows.h`.
const SDK_TAGS: [(&str, &str); 4] = [
    ("RECT", "tagRECT"),
    ("POINT", "tagPOINT"),
    ("SIZE", "tagSIZE"),
    ("MSG", "tagMSG"),
];

/// The state of one export of the header.
struct Writer<'a> {
    by_name: HashMap<&'a str, usize>,
    type_names: BTreeSet<String>,
    used: BTreeSet<String>,
    opaque: BTreeSet<String>,
    report: Report,
}

impl Writer<'_> {
    /// Records a type name a declaration needs: Err when it must be complete and is not.
    fn need(&mut self, decl: &Decl) -> Result<(), String> {
        for m in &decl.mentions {
            if self.by_name.contains_key(m.name.as_str()) {
                continue;
            }
            if m.by_value {
                return Err(format!("incomplete type {} by value", m.name));
            }
        }
        Ok(())
    }

    /// Records the opaque names of a declaration that is going into the header.
    fn commit(&mut self, decl: &Decl) {
        for m in &decl.mentions {
            if !self.by_name.contains_key(m.name.as_str()) {
                self.opaque.insert(cdecl::mangled(&m.name));
            }
        }
    }

    fn leave_out(&mut self, reason: &str, address: &str, name: &str) {
        self.report
            .omitted
            .entry(reason.to_string())
            .or_default()
            .push(format!("{address} {name}"));
    }

    /// The body of a struct, member by member, and the structs it holds by value.
    fn body(&mut self, s: &Struct) -> Result<Body, String> {
        let mut members = Vec::new();
        let mut deps = Vec::new();
        let mut asserts = Vec::new();
        let mut cursor = 0u32;
        let mut fields: Vec<(u32, u32, String, String, String)> = s
            .fields
            .iter()
            .map(|f| {
                (
                    f.offset,
                    f.size,
                    f.ty.clone(),
                    f.name.clone(),
                    f.meaning.clone(),
                )
            })
            .collect();
        if !s.vtable.is_empty() && !fields.iter().any(|f| f.0 < 4) {
            fields.insert(
                0,
                (
                    0,
                    4,
                    "void **".to_string(),
                    "__vfptr".to_string(),
                    format!("Holds the vtable {}", s.vtable),
                ),
            );
        }
        for (offset, size, ty, name, meaning) in fields {
            if offset < cursor {
                return Err(format!(
                    "{}.{name}: offset {offset:#x} overlaps the member before it",
                    s.name
                ));
            }
            if offset + size > s.size {
                return Err(format!("{}.{name}: runs past the struct's size", s.name));
            }
            if offset > cursor {
                members.push(Member {
                    text: format!("uint8_t _pad{cursor:04x}[{}];", offset - cursor),
                });
            }
            let by_value = match cdecl::parse(&ty) {
                Ok(decl) if self.need(&decl).is_ok() => Some(decl),
                _ => None,
            };
            let text = match by_value {
                Some(decl) => {
                    for m in &decl.mentions {
                        if m.by_value {
                            deps.push(self.by_name[m.name.as_str()]);
                        }
                    }
                    self.commit(&decl);
                    asserts.push(format!(
                        "BRE_ASSERT(sizeof(((struct {} *)0)->{name}) == {size}, \"{}.{name} size\");",
                        s.c, s.name
                    ));
                    format!("{};", cdecl::with_name(&ty, &decl, &name))
                }
                None => {
                    self.report
                        .bytes_for_fields
                        .push(format!("{}.{name}", s.name));
                    format!("uint8_t {name}[{size}];")
                }
            };
            asserts.push(format!(
                "BRE_ASSERT(offsetof(struct {}, {name}) == {offset:#x}, \"{}.{name} offset\");",
                s.c, s.name
            ));
            members.push(Member {
                text: format!("{text} /* {offset:#06x}: {} */", comment(&meaning)),
            });
            cursor = offset + size;
            self.report.fields += 1;
        }
        if cursor < s.size {
            members.push(Member {
                text: format!("uint8_t _pad{cursor:04x}[{}];", s.size - cursor),
            });
        }
        Ok(Body {
            members,
            deps,
            asserts,
        })
    }
}

/// Generates the header.
pub fn generate(symbols: &Symbols, origin: &Origin) -> Result<Header, String> {
    let structs = structs_of(symbols)?;
    let mut writer = Writer {
        by_name: structs
            .iter()
            .enumerate()
            .map(|(i, s)| (s.name.as_str(), i))
            .collect(),
        type_names: type_names(symbols, &structs),
        used: structs.iter().map(|s| s.c.clone()).collect(),
        opaque: BTreeSet::new(),
        report: Report::default(),
    };
    if writer.used.len() != structs.len() {
        return Err("two structs have one C name".to_string());
    }
    for s in &structs {
        if RESERVED.contains(s.c.as_str()) {
            return Err(format!("struct {} is a reserved word", s.name));
        }
    }

    // The struct bodies, then their order: a struct after the ones it holds by value.
    let mut bodies = Vec::new();
    for s in &structs {
        bodies.push(writer.body(s)?);
    }
    let mut order = Vec::new();
    let mut state = vec![0u8; structs.len()];
    fn visit(
        i: usize,
        bodies: &[Body],
        structs: &[Struct],
        state: &mut [u8],
        order: &mut Vec<usize>,
    ) -> Result<(), String> {
        match state[i] {
            2 => return Ok(()),
            1 => return Err(format!("{} holds itself by value", structs[i].name)),
            _ => {}
        }
        state[i] = 1;
        let mut deps = bodies[i].deps.clone();
        deps.sort_by(|a, b| structs[*a].name.cmp(&structs[*b].name));
        for d in deps {
            visit(d, bodies, structs, state, order)?;
        }
        state[i] = 2;
        order.push(i);
        Ok(())
    }
    for i in 0..structs.len() {
        visit(i, &bodies, &structs, &mut state, &mut order)?;
    }

    // Globals and prototypes.
    let mut globals = Vec::new();
    let mut addresses = Vec::new();
    {
        let t = &symbols.globals;
        let (address, ty, name) = (t.column("address")?, t.column("type")?, t.column("name")?);
        for row in &t.rows {
            if row[name].is_empty() {
                continue;
            }
            let (a, n) = (&row[address], &row[name]);
            let Some(c) = c_name(n) else {
                writer.leave_out("name is not a C identifier", a, n);
                continue;
            };
            if RESERVED.contains(c.as_str()) || writer.type_names.contains(&c) {
                writer.leave_out("name is also a type or a reserved word", a, n);
                continue;
            }
            let decl = match cdecl::parse(&row[ty]) {
                Ok(decl) => decl,
                Err(_) => {
                    writer.leave_out("type outside the grammar", a, n);
                    continue;
                }
            };
            if writer.need(&decl).is_err() {
                writer.leave_out("incomplete type by value", a, n);
                continue;
            }
            if !writer.used.insert(c.clone()) {
                writer.leave_out("name used twice", a, n);
                continue;
            }
            writer.commit(&decl);
            writer.report.globals += 1;
            addresses.push(format!("#define BRE_ADDR_{c} {a}"));
            globals.push(format!(
                "/* {a} */ extern {};",
                cdecl::with_name(&row[ty], &decl, &c)
            ));
        }
    }
    // Prototypes and pointer types: the ones of any compiler, then the `__thiscall` ones.
    let (mut prototypes, mut this_prototypes) = (Vec::new(), Vec::new());
    let (mut pointers, mut this_pointers, mut fastcall_pointers) =
        (Vec::new(), Vec::new(), Vec::new());
    {
        let t = &symbols.functions;
        let (address, name, signature) = (
            t.column("address")?,
            t.column("name")?,
            t.column("signature")?,
        );
        for row in &t.rows {
            let (a, n) = (&row[address], &row[name]);
            if row[signature].is_empty() {
                writer.report.unsigned_functions += 1;
                continue;
            }
            if n.is_empty() {
                writer.leave_out("no name", a, n);
                continue;
            }
            match prototype(&mut writer, n, &row[signature]) {
                Ok(proto) => {
                    writer.report.prototypes += 1;
                    let line = format!("/* {a} */ {}", proto.line);
                    addresses.push(format!("#define BRE_ADDR_{} {a}", proto.c));
                    if proto.thiscall {
                        this_prototypes.push(line);
                        this_pointers.push(proto.pointer);
                    } else {
                        prototypes.push(line);
                        pointers.push(proto.pointer);
                    }
                    fastcall_pointers.extend(proto.fastcall_pointer);
                }
                Err(reason) => writer.leave_out(&reason, a, n),
            }
        }
    }
    writer.report.structs = structs.len();
    writer.report.opaque = writer.opaque.iter().cloned().collect();

    let mut h = String::new();
    banner(&mut h, origin);
    h.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n/* BRE-TYPES-BEGIN */\n");
    for s in &structs {
        let _ = writeln!(h, "typedef struct {0} {0};", s.c);
    }
    for name in &writer.report.opaque {
        let _ = match SDK_TAGS.iter().find(|(sdk, _)| sdk == name) {
            Some((_, tag)) => writeln!(h, "typedef struct {tag} {name};"),
            None => writeln!(h, "typedef struct {name} {name};"),
        };
    }
    h.push_str("\n#pragma pack(push, 1)\n");
    for &i in &order {
        let s = &structs[i];
        let _ = write!(
            h,
            "\n/* {} bytes: {} */\nstruct {} {{\n",
            s.size,
            comment(&s.meaning),
            s.c
        );
        for m in &bodies[i].members {
            let _ = writeln!(h, "    {}", m.text);
        }
        h.push_str("};\n");
    }
    h.push_str("#pragma pack(pop)\n/* BRE-TYPES-END */\n\n");
    h.push_str("#ifdef BENILLA_RE_LAYOUT_CHECK\n");
    for (i, s) in structs.iter().enumerate() {
        let _ = writeln!(
            h,
            "BRE_ASSERT(sizeof(struct {}) == {}, \"{} size\");",
            s.c, s.size, s.name
        );
        for line in &bodies[i].asserts {
            let _ = writeln!(h, "{line}");
        }
    }
    h.push_str("#endif\n\n");
    h.push_str("/* Globals */\n");
    for line in &globals {
        let _ = writeln!(h, "{line}");
    }
    h.push_str(
        "\n/* Functions: prototypes, behind BENILLA_RE_PROTOTYPES. The functions are in WoW.exe, so\n \
         * none can be linked; a call goes through BRE_FN_name and BRE_ADDR_name below. */\n\
         #ifdef BENILLA_RE_PROTOTYPES\n",
    );
    for line in &prototypes {
        let _ = writeln!(h, "{line}");
    }
    h.push_str("#ifdef BRE_HAS_THISCALL\n");
    for line in &this_prototypes {
        let _ = writeln!(h, "{line}");
    }
    h.push_str("#endif\n#endif\n");
    h.push_str(
        "\n/* Addresses: BRE_ADDR_<name>, the virtual address of a function or a global */\n",
    );
    for line in &addresses {
        let _ = writeln!(h, "{line}");
    }
    h.push_str(
        "\n/* Function pointer types: BRE_FN_<name> has the function's convention;\n \
         * BRE_FNX_<name>, for a __thiscall function, is the __fastcall form that passes the dead\n \
         * edx second. Where BRE_HAS_THISCALL is not defined, only BRE_FNX_<name> exists for a\n \
         * __thiscall function. */\n",
    );
    for line in &pointers {
        let _ = writeln!(h, "{line}");
    }
    h.push_str("#ifdef BRE_HAS_THISCALL\n");
    for line in &this_pointers {
        let _ = writeln!(h, "{line}");
    }
    h.push_str("#endif\n");
    for line in &fastcall_pointers {
        let _ = writeln!(h, "{line}");
    }
    h.push('\n');
    let _ = writeln!(
        h,
        "/* {} prototypes, {} globals and {} structs; left out: {} rows. */",
        writer.report.prototypes,
        writer.report.globals,
        structs.len(),
        writer.report.omitted_total()
    );
    h.push_str("\n#ifdef __cplusplus\n}\n#endif\n#endif\n");
    Ok(Header {
        text: h,
        report: writer.report,
    })
}

/// A function the header declares.
struct Proto {
    /// The C name.
    c: String,
    /// The prototype without its address: `RET CONVENTION name(PARAMETERS);`.
    line: String,
    /// The type of a pointer to it, with its convention.
    pointer: String,
    /// For a `__thiscall` function with a parameter: the `__fastcall` form of the same call, with
    /// `this` first and the dead `edx` second.
    fastcall_pointer: Option<String>,
    /// Whether the convention is `__thiscall`, which MSVC spells only on a member function.
    thiscall: bool,
}

fn prototype(writer: &mut Writer, name: &str, signature: &str) -> Result<Proto, String> {
    if signature.contains("/*") {
        return Err("signature holds a comment".to_string());
    }
    let parts = split_signature(signature)
        .ok_or_else(|| "signature is not RET CONVENTION (PARAMETERS)".to_string())?;
    let c = c_name(name).ok_or_else(|| "name is not a C identifier".to_string())?;
    if RESERVED.contains(c.as_str()) || writer.type_names.contains(&c) {
        return Err("name is also a type or a reserved word".to_string());
    }
    let ret =
        cdecl::parse(&parts.ret).map_err(|_| "return type outside the grammar".to_string())?;
    if ret.name.is_some() {
        return Err("return type outside the grammar".to_string());
    }
    writer
        .need(&ret)
        .map_err(|_| "incomplete type by value".to_string())?;
    let mut decls = vec![ret];
    let mut variadic = false;
    let mut texts = Vec::new();
    let list = cdecl::split_parameters(&parts.params);
    for (i, p) in list.iter().enumerate() {
        if *p == "..." {
            if i + 1 != list.len() {
                return Err("parameter outside the grammar".to_string());
            }
            variadic = true;
            continue;
        }
        if *p == "void" && list.len() == 1 {
            continue;
        }
        let decl = cdecl::parse(p).map_err(|_| "parameter outside the grammar".to_string())?;
        writer
            .need(&decl)
            .map_err(|_| "incomplete type by value".to_string())?;
        texts.push((*p, decl.clone()));
        decls.push(decl);
    }
    if variadic && parts.convention != "__cdecl" {
        return Err("variadic outside __cdecl".to_string());
    }
    if variadic && texts.is_empty() {
        return Err("variadic without a named parameter".to_string());
    }
    if !writer.used.insert(c.clone()) {
        return Err("name used twice".to_string());
    }
    for d in &decls {
        writer.commit(d);
    }
    // Parameter names: unique, and none a word of C or a type.
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut params = Vec::new();
    for (text, decl) in &texts {
        let rendered = match decl.name {
            Some((start, end)) => {
                let mut name = text[start..end].to_string();
                while RESERVED.contains(name.as_str())
                    || writer.type_names.contains(&name)
                    || taken.contains(&name)
                {
                    name.push('_');
                }
                taken.insert(name.clone());
                cdecl::with_name(text, decl, &name)
            }
            None => cdecl::mangled(text),
        };
        params.push(rendered);
    }
    let mut list = params.join(", ");
    if variadic {
        list.push_str(", ...");
    } else if params.is_empty() {
        list.push_str("void");
    }
    let ret = cdecl::mangled(&parts.ret);
    let star = if ret.ends_with('*') { "" } else { " " };
    let keyword =
        |convention: &str| format!("BRE_{}", convention.trim_start_matches('_').to_uppercase());
    let pointer_to = |convention: Option<String>, list: &str, tag: &str| {
        let convention = convention.map(|m| format!("{m} ")).unwrap_or_default();
        format!("typedef {ret}{star}({convention}*BRE_{tag}_{c})({list});")
    };
    let macro_name = keyword(&parts.convention);
    let fastcall_pointer = (parts.convention == "__thiscall" && !variadic && !params.is_empty())
        .then(|| {
            let mut all = vec![params[0].clone(), "void *bre_edx".to_string()];
            all.extend(params[1..].iter().cloned());
            pointer_to(Some(keyword("__fastcall")), &all.join(", "), "FNX")
        });
    Ok(Proto {
        line: format!("{ret}{star}{macro_name} {c}({list});"),
        pointer: pointer_to(Some(macro_name), &list, "FN"),
        fastcall_pointer,
        thiscall: parts.convention == "__thiscall",
        c,
    })
}

fn banner(h: &mut String, origin: &Origin) {
    let client = if origin.rebased {
        "WoW.exe 1.12.1.5875, with the function addresses rebased onto\n * another build by byte patterns (`cargo xtask match`): a match of bytes and position,\n * not a proof that the function is the same, and a global has no address here"
            .to_string()
    } else {
        format!(
            "WoW.exe 1.12.1.5875 (size {},\n * SHA-256 {}),",
            origin.exe_size, origin.exe_sha256
        )
    };
    let _ = write!(
        h,
        "\
/*
 * benilla-re: the structs, globals and functions of {client}
 * as the symbol table at commit
 * {commit}
 * names them.
 *
 * Generated by `cargo xtask export`; edits are lost.
 *
 * - A name is the table's with `::` written `__`, and a destructor Class::~Class written
 *   Class__dtor. A row whose name is no C identifier is left out.
 * - A struct is packed, with its gaps written as `_pad<offset>` bytes, so that every member
 *   sits at its table offset. Define BENILLA_RE_LAYOUT_CHECK to assert every size and offset.
 * - A type the tables name but do not define is declared and left incomplete.
 * - Each prototype and global carries its address in the comment before it.
 * - For a call from injected code, `((BRE_FN_name)BRE_ADDR_name)(args)`: every function has its
 *   address and a pointer type with its convention; a global has its address. Where a compiler
 *   has no __thiscall, BRE_FNX_name is the same call as a __fastcall with `this` first and the
 *   unused edx second.
 * - Define BENILLA_RE_PROTOTYPES for the free prototypes. They are off by default: the
 *   functions are in WoW.exe and cannot be linked, and a prototype of a name the SDK or the C
 *   library also declares (GetACP, ceil, _isnan) collides with it after <windows.h>.
 * - A function that takes variable arguments under __stdcall or __thiscall, which no compiler
 *   can call as written, has no prototype and no pointer type; its address is in the table.
 * - The header is C and C++. MSVC takes no __thiscall outside a member function, so there it has
 *   no __thiscall prototype and no BRE_FN_ type for one; BRE_FNX_ is there.
 */
#ifndef BENILLA_RE_H
#define BENILLA_RE_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#if !defined(BRE_CDECL)
#  if defined(_MSC_VER)
#    define BRE_CDECL __cdecl
#    define BRE_STDCALL __stdcall
#    define BRE_FASTCALL __fastcall
#    if defined(__clang__)
#      define BRE_THISCALL __thiscall
#      define BRE_HAS_THISCALL
#    endif
#  elif defined(__i386__)
#    define BRE_CDECL __attribute__((__cdecl__))
#    define BRE_STDCALL __attribute__((__stdcall__))
#    define BRE_FASTCALL __attribute__((__fastcall__))
#    define BRE_THISCALL __attribute__((__thiscall__))
#    define BRE_HAS_THISCALL
#  else
#    define BRE_CDECL
#    define BRE_STDCALL
#    define BRE_FASTCALL
#    define BRE_THISCALL
#    define BRE_HAS_THISCALL
#  endif
#endif

#ifdef __cplusplus
#  define BRE_ASSERT(condition, message) static_assert(condition, message)
#else
#  define BRE_ASSERT(condition, message) _Static_assert(condition, message)
#endif

",
        commit = origin.commit
    );
}
