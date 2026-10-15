//! The grammar of a function row's `signature` and the stack bytes it implies.
//!
//! `RET CONV (PARAMS)`: a return type, one of four conventions, then the parameters. A parameter
//! is a type with an optional name. The grammar is in `symbols/README.md`.

use crate::export::cdecl::{self, Decl};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conv {
    Cdecl,
    Stdcall,
    Fastcall,
    Thiscall,
}

impl Conv {
    fn from_word(word: &str) -> Option<Conv> {
        match word {
            "__cdecl" => Some(Conv::Cdecl),
            "__stdcall" => Some(Conv::Stdcall),
            "__fastcall" => Some(Conv::Fastcall),
            "__thiscall" => Some(Conv::Thiscall),
            _ => None,
        }
    }
}

/// A signature that follows the grammar.
#[derive(Debug, PartialEq, Eq)]
pub struct Signature<'a> {
    pub conv: Conv,
    pub ret: &'a str,
    /// The parameters, without a trailing `...`.
    pub params: Vec<&'a str>,
    pub variadic: bool,
}

/// The struct sizes of `structs.tsv`, by name.
pub type Structs<'a> = HashMap<&'a str, u32>;

const CONVENTIONS: [&str; 4] = ["__cdecl", "__stdcall", "__fastcall", "__thiscall"];

/// Reads a signature, or says what is wrong with it.
pub fn parse<'a>(text: &'a str, structs: &Structs) -> Result<Signature<'a>, String> {
    if text.contains("/*") || text.contains("*/") || text.contains("//") {
        return Err("signature holds a comment".to_string());
    }
    if text.contains('@') {
        return Err("signature holds a register annotation".to_string());
    }
    let Some(open) = text.find('(') else {
        return Err("signature has no parameter list".to_string());
    };
    let Some(inner) = text[open + 1..].strip_suffix(')') else {
        return Err("signature does not end with its parameter list".to_string());
    };
    let head = text[..open].trim_end();
    let Some((ret, word)) = split_convention(head) else {
        return Err(match head.split_whitespace().last() {
            Some(last) if last.starts_with("__") => format!("{last} is not a convention"),
            _ => "signature is not RET CONV (PARAMS): no convention, or a name before the \
                  parameter list"
                .to_string(),
        });
    };
    let Some(conv) = Conv::from_word(word) else {
        return Err(format!("{word} is not a convention"));
    };
    let ret = ret.trim();
    if ret.is_empty() {
        return Err("signature has no return type".to_string());
    }
    check_type(ret, true, structs).map_err(|e| format!("return type: {e}"))?;
    let list = cdecl::split_parameters(inner);
    let mut params = Vec::new();
    let mut variadic = false;
    for (i, p) in list.iter().enumerate() {
        if CONVENTIONS.iter().any(|c| p.contains(c)) {
            return Err("a convention inside a parameter".to_string());
        }
        if *p == "..." {
            if i + 1 != list.len() {
                return Err("... is not last".to_string());
            }
            if conv != Conv::Cdecl {
                return Err("... outside __cdecl".to_string());
            }
            variadic = true;
            continue;
        }
        if *p == "void" {
            if list.len() != 1 {
                return Err("void among other parameters".to_string());
            }
            continue;
        }
        check_type(p, false, structs).map_err(|e| format!("parameter {}: {e}", i + 1))?;
        params.push(*p);
    }
    if conv == Conv::Thiscall {
        match params.first() {
            Some(first) if is_this(first) => {}
            _ => return Err("__thiscall lists `T *this` first".to_string()),
        }
    }
    let mut names: Vec<&str> = Vec::new();
    for p in &params {
        if let Some(name) = param_name(p) {
            if names.contains(&name) {
                return Err(format!("parameter name {name} used twice"));
            }
            names.push(name);
        }
    }
    Ok(Signature {
        conv,
        ret,
        params,
        variadic,
    })
}

/// `RET CONV` split at the convention word that ends the head, `None` when it has none.
fn split_convention(head: &str) -> Option<(&str, &str)> {
    let start = head
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map_or(0, |i| i + 1);
    let word = &head[start..];
    if word.starts_with("__") && !head[..start].trim().is_empty() {
        Some((&head[..start], word))
    } else {
        None
    }
}

fn is_this(param: &str) -> bool {
    param.ends_with("this")
        && !param.contains('(')
        && param[..param.len() - 4].trim_end().ends_with('*')
        && param_name(param) == Some("this")
}

fn param_name(param: &str) -> Option<&str> {
    let decl = cdecl::parse(param).ok()?;
    decl.name.map(|(s, e)| &param[s..e])
}

/// A type of the grammar: every name it mentions is built in, a struct, or (behind a pointer
/// only) a name the header declares opaque.
fn check_type(text: &str, ret: bool, structs: &Structs) -> Result<(), String> {
    let decl: Decl = cdecl::parse(text).map_err(|e| format!("outside the grammar ({e})"))?;
    if ret && decl.name.is_some() {
        return Err("holds a name".to_string());
    }
    for m in &decl.mentions {
        if m.by_value && !structs.contains_key(m.name.as_str()) {
            return Err(format!("{} is not declared and is passed by value", m.name));
        }
    }
    Ok(())
}

/// The size a parameter takes on the stack, rounded up to 4, and whether the first two
/// register-eligible parameters of `__fastcall` take it into a register.
fn param_class(param: &str, structs: &Structs) -> Option<(u32, bool)> {
    let decl = cdecl::parse(param).ok()?;
    let bare = match decl.name {
        Some((s, e)) => format!("{}{}", &param[..s], &param[e..]),
        None => param.to_string(),
    };
    if bare.contains('*') || bare.contains('(') || bare.contains('[') {
        return Some((4, true));
    }
    let words: Vec<&str> = bare
        .split_whitespace()
        .filter(|w| !matches!(*w, "const" | "volatile" | "struct" | "unsigned" | "signed"))
        .collect();
    let ty = words.join(" ");
    match ty.as_str() {
        "double" | "int64_t" | "uint64_t" | "long long" => Some((8, false)),
        "float" => Some((4, false)),
        _ => {
            if let Some(&size) = structs.get(ty.as_str()) {
                return Some((size.div_ceil(4) * 4, false));
            }
            Some((4, true))
        }
    }
}

/// Whether the return value is a struct by value that MSVC returns through a hidden pointer
/// argument: any size but 1, 2, 4 and 8.
fn returns_through_pointer(ret: &str, structs: &Structs) -> bool {
    let words: Vec<&str> = ret
        .split_whitespace()
        .filter(|w| !matches!(*w, "const" | "volatile" | "struct"))
        .collect();
    structs
        .get(words.join(" ").as_str())
        .is_some_and(|size| !matches!(size, 1 | 2 | 4 | 8))
}

/// The bytes the callee pops: what `ret N` says. `None` when a size is unknown. A struct
/// returned through a hidden pointer adds that pointer's four bytes to `__stdcall` and
/// `__thiscall`; `__cdecl` pops nothing, and `__fastcall` is left out since the pointer's place
/// among its registers is not settled.
pub fn pops(sig: &Signature, structs: &Structs) -> Option<u32> {
    let mut total = 0;
    if matches!(sig.conv, Conv::Stdcall | Conv::Thiscall)
        && returns_through_pointer(sig.ret, structs)
    {
        total += 4;
    }
    let mut registers = match sig.conv {
        Conv::Cdecl => return Some(0),
        Conv::Stdcall => 0,
        Conv::Fastcall => 2,
        Conv::Thiscall => 1,
    };
    for p in &sig.params {
        let (size, eligible) = param_class(p, structs)?;
        if eligible && registers > 0 {
            registers -= 1;
        } else {
            total += size;
        }
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structs() -> Structs<'static> {
        HashMap::from([("C3Vector", 12)])
    }

    fn bad(text: &str) -> String {
        parse(text, &structs()).expect_err(text)
    }

    #[test]
    fn well_formed() {
        for text in [
            "void __cdecl (void)",
            "void __cdecl ()",
            "int __cdecl (const char *format, ...)",
            "void * __thiscall (CMovement *this, float x)",
            "uint32_t *__stdcall (uint32_t a, uint64_t b)",
            "void __fastcall (void *a, void *b, C3Vector v, int (*f)(int, void *))",
            "NTempest::CRndSeed * __cdecl (NTempest::CRndSeed *seed)",
            "void __thiscall (uint32_t **this, int a)",
        ] {
            assert!(parse(text, &structs()).is_ok(), "{text}");
        }
    }

    #[test]
    fn malformed() {
        assert_eq!(
            bad("void __fastcall (double *a /*ecx*/, int n)"),
            "signature holds a comment"
        );
        assert!(bad("void __stdcall SetHealer(uint64_t guid)").contains("no convention"));
        assert!(bad("int Handler(uint32_t opcode)").contains("no convention"));
        assert!(bad("int (void)").contains("no convention"));
        assert_eq!(
            bad("void __userpurge (char *name@<ecx>)"),
            "signature holds a register annotation"
        );
        assert_eq!(bad("void __foo (int a)"), "__foo is not a convention");
        assert!(bad("void __cdecl Name (int a)").contains("no convention"));
        assert_eq!(
            bad("void __fastcall (uint32_t (__fastcall *f)(uint32_t))"),
            "a convention inside a parameter"
        );
        assert!(bad("void __cdecl (HANDLE h)").contains("HANDLE is not declared"));
        assert!(bad("void __cdecl (RECT r)").contains("RECT is not declared"));
        assert!(bad("RECT __cdecl (void)").contains("return type"));
        assert!(bad("void __cdecl (int a, ..., int b)").contains("not last"));
        assert!(bad("void __stdcall (int a, ...)").contains("outside __cdecl"));
        assert!(bad("void __thiscall (int a)").contains("`T *this` first"));
        assert!(bad("void __thiscall (void *a, int b)").contains("`T *this` first"));
        assert!(bad("void __thiscall (void)").contains("`T *this` first"));
        assert!(bad("void __cdecl (int a, int a)").contains("used twice"));
        assert!(bad("void __cdecl (void, int a)").contains("void among"));
        assert!(bad("void __cdecl (int a").contains("parameter list"));
    }

    #[test]
    fn pointers_to_undeclared_names_are_opaque() {
        assert!(parse("void __cdecl (Unknown *p, Unknown **q)", &structs()).is_ok());
    }

    #[test]
    fn stack_bytes() {
        let s = structs();
        let of = |text: &str| pops(&parse(text, &s).unwrap(), &s);
        assert_eq!(of("void __cdecl (int a, int b)"), Some(0));
        assert_eq!(of("void __stdcall (int a, uint64_t b)"), Some(12));
        assert_eq!(of("void __thiscall (T *this, int a, double b)"), Some(12));
        assert_eq!(of("void __thiscall (T *this)"), Some(0));
        assert_eq!(of("void __fastcall (int a, int b, int c)"), Some(4));
        assert_eq!(of("void __fastcall (int a, float f)"), Some(4));
        assert_eq!(
            of("void __fastcall (float f, int a, int b, int c)"),
            Some(8)
        );
        assert_eq!(of("void __fastcall (C3Vector v, int a)"), Some(12));
        assert_eq!(of("void __stdcall (C3Vector v)"), Some(12));
        assert_eq!(
            of("void __stdcall (const char *s, void (*f)(int, int))"),
            Some(8)
        );
        assert_eq!(
            of("void __stdcall (uint8_t a, bool b, uint16_t c)"),
            Some(12)
        );
        assert_eq!(of("void __stdcall (void)"), Some(0));
    }

    #[test]
    fn hidden_return_pointer() {
        let s = structs();
        let of = |text: &str| pops(&parse(text, &s).unwrap(), &s);
        assert_eq!(of("C3Vector __stdcall (int a)"), Some(8));
        assert_eq!(of("C3Vector __thiscall (T *this, int a)"), Some(8));
        assert_eq!(of("C3Vector __cdecl (int a)"), Some(0));
        assert_eq!(of("C3Vector * __stdcall (int a)"), Some(4));
        assert_eq!(of("uint64_t __stdcall (int a)"), Some(4));
    }
}
