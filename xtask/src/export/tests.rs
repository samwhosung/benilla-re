//! The export's controls: the header generator on small tables, and the outputs of the real tree
//! (the bundle's rows, the header compiled by clang, the scripts run against a stand-in API).

use super::header::{generate, Origin, Symbols};
use super::table::Table;
use super::{build, read, Built};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the repo")
        .to_path_buf()
}

fn origin() -> Origin {
    Origin {
        exe_size: 1,
        exe_sha256: "0".repeat(64),
        commit: "test".to_string(),
        rebased: false,
    }
}

const STRUCTS: &str =
    "name\tname_from\tsize\tvtable\tsystem\tmeaning\tconfidence\tproof\tfinding\n";
const FIELDS: &str =
    "struct\toffset\tsize\ttype\tname\tsystem\tmeaning\tconfidence\tproof\tfinding\n";
const FUNCTIONS: &str = "address\tsize\tname\tname_from\tsystem\tclass\tsignature\tmeaning\tconfidence\tproof\tfinding\ttest\n";
const GLOBALS: &str =
    "address\ttype\tname\tname_from\tsystem\tmeaning\tconfidence\tproof\tfinding\n";

/// A struct row.
fn s(name: &str, size: u32, vtable: &str) -> String {
    format!("{name}\t\t{size}\t{vtable}\tx\tHolds a thing\tverified\tone\tx#y\n")
}

/// A field row.
fn f(owner: &str, offset: u32, size: u32, ty: &str, name: &str) -> String {
    format!("{owner}\t{offset:#06x}\t{size}\t{ty}\t{name}\tx\tHolds a field\tverified\tone\tx#y\n")
}

/// A function row.
fn func(address: u32, name: &str, signature: &str) -> String {
    format!("{address:#08x}\t4\t{name}\t\tx\t\t{signature}\tCalls\tverified\tone\tx#y\t\n")
}

/// A global row.
fn global(address: u32, ty: &str, name: &str) -> String {
    format!("{address:#08x}\t{ty}\t{name}\t\tx\tHolds\tverified\tone\tx#y\n")
}

fn table(head: &str, rows: &[String]) -> Table {
    Table::parse("t.tsv", &format!("{head}{}", rows.concat())).expect("a table")
}

fn symbols(
    structs: &[String],
    fields: &[String],
    functions: &[String],
    globals: &[String],
) -> Symbols {
    Symbols {
        structs: table(STRUCTS, structs),
        fields: table(FIELDS, fields),
        functions: table(FUNCTIONS, functions),
        globals: table(GLOBALS, globals),
    }
}

fn header(
    structs: &[String],
    fields: &[String],
    functions: &[String],
    globals: &[String],
) -> String {
    generate(&symbols(structs, fields, functions, globals), &origin())
        .expect("a header")
        .text
}

#[test]
fn gaps_and_the_tail_are_padding_and_a_vtable_is_a_member() {
    let text = header(
        &[s("Thing", 24, "0x801000")],
        &[
            f("Thing", 8, 4, "uint32_t", "m_a"),
            f("Thing", 16, 2, "uint16_t", "m_b"),
        ],
        &[],
        &[],
    );
    let body: Vec<&str> = text
        .lines()
        .skip_while(|l| !l.starts_with("struct Thing {"))
        .skip(1)
        .take_while(|l| *l != "};")
        .collect();
    let members: Vec<&str> = body
        .iter()
        .map(|l| l.split(" /*").next().unwrap().trim())
        .collect();
    assert_eq!(
        members,
        [
            "void **__vfptr;",
            "uint8_t _pad0004[4];",
            "uint32_t m_a;",
            "uint8_t _pad000c[4];",
            "uint16_t m_b;",
            "uint8_t _pad0012[6];",
        ]
    );
    for expected in [
        "BRE_ASSERT(sizeof(struct Thing) == 24, \"Thing size\");",
        "BRE_ASSERT(offsetof(struct Thing, m_b) == 0x10, \"Thing.m_b offset\");",
        "BRE_ASSERT(offsetof(struct Thing, __vfptr) == 0x0, \"Thing.__vfptr offset\");",
        "sizeof(((struct Thing *)0)->m_a) == 4",
        "#pragma pack(push, 1)",
    ] {
        assert!(text.contains(expected), "{expected}");
    }
}

#[test]
fn a_member_at_offset_zero_holds_the_vtable_and_comes_after_what_it_holds() {
    let text = header(
        &[s("Derived", 12, "0x801010"), s("Base", 8, "0x801000")],
        &[
            f("Base", 0, 4, "void **", "__vfptr"),
            f("Derived", 0, 8, "Base", "m_base"),
            f("Derived", 8, 4, "uint32_t", "m_x"),
        ],
        &[],
        &[],
    );
    assert_eq!(text.matches("__vfptr;").count(), 1, "only Base has one");
    let base = text.find("struct Base {").expect("Base is defined");
    let derived = text.find("struct Derived {").expect("Derived is defined");
    assert!(base < derived);
    assert!(text.contains("    Base m_base;"));
}

#[test]
fn arrays_and_function_pointers_carry_their_name_inside() {
    let text = header(
        &[s("Thing", 36, "")],
        &[
            f("Thing", 0, 16, "uint8_t[16]", "m_digest"),
            f("Thing", 16, 8, "void *[2]", "m_links"),
            f("Thing", 24, 12, "float[3]", "m_position"),
        ],
        &[func(
            0x401000,
            "Apply",
            "void __cdecl (void (*callback)(int, int), uint8_t digest[16], uint32_t *out[6])",
        )],
        &[
            global(0x801000, "void (*[4])(void)", "s_hooks"),
            global(0x801010, "const char *[3]", "s_names"),
            global(0x801020, "char[8][4]", "s_grid"),
            global(0x801040, "const float", "s_scale"),
            global(0x801050, "CCamera **", "s_cameras"),
        ],
    );
    for expected in [
        "uint8_t m_digest[16];",
        "void *m_links[2];",
        "float m_position[3];",
        "extern void (*s_hooks[4])(void);",
        "extern const char *s_names[3];",
        "extern char s_grid[8][4];",
        "extern const float s_scale;",
        "extern CCamera **s_cameras;",
        "void BRE_CDECL Apply(void (*callback)(int, int), uint8_t digest[16], uint32_t *out[6]);",
        "typedef struct CCamera CCamera;",
        "#define BRE_ADDR_Apply 0x401000",
        "#define BRE_ADDR_s_hooks 0x801000",
        "#define BRE_G_s_hooks (*(void (*(*)[4])(void))BRE_ADDR_s_hooks)",
        "#define BRE_G_s_names (*(const char *(*)[3])BRE_ADDR_s_names)",
        "#define BRE_G_s_grid (*(char (*)[8][4])BRE_ADDR_s_grid)",
        "#define BRE_G_s_scale (*(const float *)BRE_ADDR_s_scale)",
        "#define BRE_G_s_cameras (*(CCamera ***)BRE_ADDR_s_cameras)",
        "typedef void (BRE_CDECL *BRE_FN_Apply)(void (*callback)(int, int), uint8_t digest[16], uint32_t *out[6]);",
    ] {
        assert!(text.contains(expected), "{expected}");
    }
    // The declarations of the globals are behind the prototypes' guard; the accessors are not.
    let guard = text.find("#ifdef BENILLA_RE_PROTOTYPES").unwrap();
    let end = guard + text[guard..].find("#endif\n#endif\n").unwrap();
    let declared = text.find("extern const float s_scale;").unwrap();
    let accessor = text.find("#define BRE_G_s_scale").unwrap();
    assert!(guard < declared && declared < end);
    assert!(accessor > end);
}

#[test]
fn names_of_forty_eight_characters_pass_whole() {
    let plain = "A".repeat(48);
    let method = format!("{}::{}", "B".repeat(23), "C".repeat(23));
    assert_eq!(method.len(), 48);
    let text = header(
        &[],
        &[],
        &[
            func(0x401000, &plain, "void __cdecl (void)"),
            func(0x401010, &method, "void __thiscall (void *this)"),
        ],
        &[global(
            0x801000,
            "uint32_t",
            &format!("s_{}", "d".repeat(46)),
        )],
    );
    assert!(text.contains(&format!("void BRE_CDECL {plain}(void);")));
    assert!(text.contains(&format!(
        "void BRE_THISCALL {}__{}(void *this_);",
        "B".repeat(23),
        "C".repeat(23)
    )));
    assert!(text.contains(&format!("extern uint32_t s_{};", "d".repeat(46))));
}

#[test]
fn a_prototype_the_header_cannot_state_is_left_out_and_counted() {
    let made = generate(
        &symbols(
            &[s("Thing", 4, "")],
            &[f("Thing", 0, 4, "uint32_t", "m_a")],
            &[
                func(
                    0x401000,
                    "Fine",
                    "Thing * __fastcall (Thing *thing, lua_State *L)",
                ),
                func(0x401010, "ByValue", "void __cdecl (HSLOG log)"),
                func(0x401020, "Commented", "void __cdecl (/* four */ int x)"),
                func(0x401030, "NoConvention", "void (int x)"),
                func(0x401040, "_WinMain@16", "int __stdcall (void)"),
                func(
                    0x401050,
                    "Decorated::~Decorated",
                    "void __thiscall (void *this)",
                ),
                func(
                    0x401060,
                    "Variadic",
                    "void __cdecl (const char *format, ...)",
                ),
                func(
                    0x401070,
                    "VariadicThis",
                    "void __thiscall (void *this, ...)",
                ),
                func(
                    0x401080,
                    "Names",
                    "void __cdecl (int a, int a, int register, Thing *Thing)",
                ),
                func(0x401090, "Userpurge", "void __userpurge (char *name@<ecx>)"),
                func(0x4010a0, "Empty", "void __thiscall ()"),
            ],
            &[],
        ),
        &origin(),
    )
    .unwrap();
    let text = &made.text;
    for expected in [
        "Thing *BRE_FASTCALL Fine(Thing *thing, lua_State *L);",
        "typedef struct lua_State lua_State;",
        "void BRE_CDECL Variadic(const char *format, ...);",
        "void BRE_THISCALL Decorated__dtor(void *this_);",
        "void BRE_CDECL Names(int a, int a_, int register_, Thing *Thing_);",
        "void BRE_THISCALL Empty(void);",
        "#define BRE_ADDR_Fine 0x401000",
        "typedef Thing *(BRE_FASTCALL *BRE_FN_Fine)(Thing *thing, lua_State *L);",
        "typedef void (BRE_THISCALL *BRE_FN_Decorated__dtor)(void *this_);",
        "typedef void (BRE_FASTCALL *BRE_FNX_Decorated__dtor)(void *this_, void *bre_edx);",
        "typedef void (BRE_CDECL *BRE_FN_Names)(int a, int a_, int register_, Thing *Thing_);",
    ] {
        assert!(text.contains(expected), "{expected}");
    }
    assert!(
        !text.contains("HSLOG"),
        "an incomplete type by value is left out"
    );
    let omitted: Vec<(&str, usize)> = made
        .report
        .omitted
        .iter()
        .map(|(reason, rows)| (reason.as_str(), rows.len()))
        .collect();
    assert_eq!(
        omitted,
        [
            ("incomplete type by value", 1),
            ("name is not a C identifier", 1),
            ("signature holds a comment", 1),
            ("signature is not RET CONVENTION (PARAMETERS)", 2),
            ("variadic outside __cdecl", 1),
        ]
    );
    assert!(!text.contains("VariadicThis"));
    assert_eq!(made.report.prototypes, 5);
}

#[test]
fn a_field_of_an_unknown_type_by_value_is_bytes() {
    let text = header(
        &[s("Thing", 12, "")],
        &[f("Thing", 4, 8, "Unknown", "m_x")],
        &[],
        &[],
    );
    assert!(text.contains("uint8_t m_x[8];"));
    assert!(!text.contains("sizeof(((struct Thing *)0)->m_x)"));
}

#[test]
fn a_field_that_overlaps_or_overruns_is_an_error() {
    for fields in [
        vec![
            f("T", 0, 4, "uint32_t", "m_a"),
            f("T", 2, 4, "uint32_t", "m_b"),
        ],
        vec![f("T", 6, 4, "uint32_t", "m_a")],
    ] {
        let symbols = symbols(&[s("T", 8, "")], &fields, &[], &[]);
        assert!(generate(&symbols, &origin()).is_err());
    }
}

/// The export of this tree, built once.
fn built() -> &'static Built {
    static BUILT: OnceLock<Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        build(&read(&root(), None).expect("the tree reads")).expect("the tree exports")
    })
}

#[test]
fn a_bundle_of_the_tree_round_trips_to_its_rows() {
    let built = built();
    let json = &built.files["bundle/benilla-re.json"];
    let files = super::bundle::files_of(json).expect("the bundle reads");
    assert!(files.len() >= 18, "{} tables", files.len());
    for (path, text) in &files {
        let source = std::fs::read_to_string(root().join(path)).expect("a table of the tree");
        assert_eq!(text, &source, "{path}");
        assert_eq!(&built.files[&format!("bundle/{path}")], &source, "{path}");
    }
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["exe"]["size"], crate::exe::size());
    assert_eq!(value["exe"]["sha256"], crate::exe::sha256());
    assert!(value["commit"].as_str().is_some_and(|c| c.len() >= 40));
}

#[test]
fn the_export_has_a_readme_the_docs_and_the_columns_docs() {
    let files = &built().files;
    let readme = &files["README.md"];
    for expected in [
        "https://github.com/samwhosung/benilla-re",
        "https://samwhosung.github.io/benilla-re/?0x5e6853",
        "d/systems/net/friends.html#who-query-tags",
        "systems/net/friends.md",
        "`db` folder",
        "<ghidra install>/support/analyzeHeadless",
        "`signature.tsv`",
    ] {
        assert!(readme.contains(expected), "{expected}");
    }
    assert!(!readme.contains("{{"));
    // Every folder and file the README names is in the export.
    for path in [
        "benilla-re.h",
        "ida/benilla_re.py",
        "binja/benilla_re.py",
        "ghidra/README.md",
        "x64dbg/WoW.exe.dd32",
        "bundle/benilla-re.json",
        "bundle/signature.tsv",
        "bundle/symbols/README.md",
        "bundle/tables/README.md",
        "bundle/tables/opcode_sends.tsv",
        "docs/METHOD.md",
        "docs/CONTRIBUTING.md",
        "systems/README.md",
        "systems/net/friends.md",
    ] {
        assert!(files.contains_key(path), "{path}");
    }
    assert!(!files.contains_key("bundle/signature"));
    let json: serde_json::Value = serde_json::from_str(&files["bundle/benilla-re.json"]).unwrap();
    assert_eq!(json["repo"], "https://github.com/samwhosung/benilla-re");
    assert_eq!(json["site"], "https://samwhosung.github.io/benilla-re/");
}

#[test]
fn every_finding_of_a_function_row_is_a_doc_of_the_export() {
    let files = &built().files;
    let functions = Table::parse(
        "symbols/functions.tsv",
        &std::fs::read_to_string(root().join("symbols/functions.tsv")).unwrap(),
    )
    .unwrap();
    let finding = functions.column("finding").unwrap();
    let mut docs = std::collections::BTreeSet::new();
    for row in &functions.rows {
        if let Some((doc, _)) = row[finding].split_once('#') {
            docs.insert(doc.to_string());
        }
    }
    assert!(docs.len() > 100);
    for doc in docs {
        let path = if doc == "METHOD" {
            "docs/METHOD.md".to_string()
        } else {
            format!("systems/{doc}.md")
        };
        assert!(files.contains_key(&path), "{doc}: no {path}");
    }
}

#[test]
fn the_signature_file_has_a_row_for_every_function_row_with_the_name_or_a_fun_name() {
    let files = &built().files;
    let signature = &files["bundle/signature.tsv"];
    let mut lines = signature.lines();
    assert_eq!(lines.next(), Some("address\tname\tpattern\tunique"));
    let rows: Vec<Vec<&str>> = lines.map(|l| l.split('\t').collect()).collect();
    let functions = Table::parse("f", &files["bundle/symbols/functions.tsv"]).unwrap();
    assert_eq!(rows.len(), functions.rows.len());
    for row in &rows {
        assert_eq!(row.len(), 4);
        assert!(row[3] == "yes" || row[3] == "no");
        assert!(!row[1].is_empty());
    }
}

#[test]
fn the_same_tree_exports_the_same_bytes() {
    let again = build(&read(&root(), None).unwrap()).unwrap();
    assert_eq!(&again.files, &built().files);
}

#[test]
fn the_ghidra_folder_holds_the_tools_own_scripts_and_the_tables_they_read() {
    let built = built();
    for (source, output) in [
        ("tools/ghidra/ApplySymbols.java", "ghidra/ApplySymbols.java"),
        (
            "tools/ghidra/CreateFunctions.java",
            "ghidra/CreateFunctions.java",
        ),
    ] {
        let text = std::fs::read_to_string(root().join(source)).unwrap();
        assert_eq!(built.files[output], text);
    }
    for name in ["functions", "globals", "structs", "fields"] {
        let text = std::fs::read_to_string(root().join(format!("symbols/{name}.tsv"))).unwrap();
        assert_eq!(built.files[&format!("ghidra/symbols/{name}.tsv")], text);
    }
}

fn clang() -> bool {
    Command::new("clang")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// One compile of the header: the language, the flags and what comes before the include.
struct Job<'a> {
    compiler: &'a str,
    lang: &'a str,
    flags: &'a [&'a str],
    before: &'a str,
}

/// Compiles the header with every layout assertion on.
fn compile(dir: &Path, header: &str, job: &Job) -> (bool, String) {
    std::fs::write(dir.join("benilla-re.h"), header).unwrap();
    // A call through an address, as injected code makes it, and a global's address.
    let plain_msvc = job.flags.contains(&"-DBRE_CDECL=__cdecl");
    let call = if !plain_msvc {
        "BRE_FN_CDataStore__GetUInt8 put = (BRE_FN_CDataStore__GetUInt8)BRE_ADDR_CDataStore__GetUInt8;\n\
         BRE_FNX_CDataStore__GetUInt8 put_x = (BRE_FNX_CDataStore__GetUInt8)BRE_ADDR_CDataStore__GetUInt8;\n"
    } else {
        "BRE_FNX_CDataStore__GetUInt8 put_x = (BRE_FNX_CDataStore__GetUInt8)BRE_ADDR_CDataStore__GetUInt8;\n"
    };
    // Every shape of global: a scalar, an array, a two-dimensional array and an array of function
    // pointers, read and written through the accessor.
    let globals = "void bre_globals(void) {\n\
         uint32_t sent = BRE_G_s_netBytesSent;\n\
         uint32_t first = BRE_G_s_netClientList[0];\n\
         uint8_t cell = BRE_G_s_chatTypeInfo[1][2];\n\
         void (*hook)(void) = BRE_G___xc_a[0];\n\
         BRE_G_s_netBytesSent = sent + first + cell;\n\
         (void)hook;\n\
         }\n";
    let layout = if job.flags.contains(&"-DBRE_NO_LAYOUT_CHECK") {
        ""
    } else {
        "#define BENILLA_RE_LAYOUT_CHECK\n"
    };
    let file = if job.lang == "c" {
        "check.c"
    } else {
        "check.cpp"
    };
    std::fs::write(
        dir.join(file),
        format!(
            "{}\n{layout}#include \"benilla-re.h\"\n{call}\
             BRE_ASSERT(BRE_ADDR_CDataStore__GetUInt8 == 0x418cb0, \"address\");\n\
             {globals}",
            job.before
        ),
    )
    .unwrap();
    let output = Command::new(job.compiler)
        .args([
            "-fsyntax-only",
            "-Wall",
            "-Wextra",
            "-pedantic",
            "-Werror",
            "-x",
            job.lang,
        ])
        .arg(if job.lang == "c" {
            "-std=c11"
        } else {
            "-std=c++17"
        })
        .args(job.flags)
        .arg("-I")
        .arg(dir)
        .arg(dir.join(file))
        .output()
        .expect("the compiler runs");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// What the Windows SDK declares of the names the table also declares, the SDK's way.
const SDK_STUB: &str = "\
typedef struct tagRECT { long left, top, right, bottom; } RECT;\n\
__declspec(dllimport) unsigned long __stdcall GetACP(void);\n\
double __cdecl ceil(double);\n\
int __cdecl _finite(double);\n\
int __cdecl _isnan(double);\n\
#define cdecl _cdecl\n";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("benilla-re-export-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn clang_compiles_the_header_and_every_layout_assertion_holds() {
    if !clang() {
        println!("skipped: no clang on the PATH");
        return;
    }
    let text = &built().files["benilla-re.h"];
    let dir = scratch("clang");
    let mut failures = Vec::new();
    let mut run = |job: Job| {
        let (ok, errors) = compile(&dir, text, &job);
        if !ok {
            failures.push(format!(
                "{} {} {:?}:\n{}",
                job.compiler,
                job.lang,
                job.flags,
                errors.lines().take(8).collect::<Vec<_>>().join("\n")
            ));
        }
    };
    // The Microsoft spelling for 32-bit x86, as C and as C++, with each way a compiler of that
    // family is told to accept Microsoft's language, with and without the prototypes, and after
    // an SDK that declares the same names.
    for lang in ["c", "c++"] {
        for mode in [
            "-fno-ms-extensions",
            "-fms-extensions",
            "-fms-compatibility",
        ] {
            for prototypes in ["-DBENILLA_RE_PROTOTYPES", "-DBRE_NO_PROTOTYPES"] {
                for before in ["", SDK_STUB] {
                    if before == SDK_STUB && prototypes == "-DBENILLA_RE_PROTOTYPES" {
                        continue;
                    }
                    run(Job {
                        compiler: "clang",
                        lang,
                        flags: &[
                            "--target=i686-pc-windows-msvc",
                            "-fdeclspec",
                            mode,
                            prototypes,
                        ],
                        before,
                    });
                }
            }
        }
        // Plain MSVC has no __thiscall outside a member: the header leaves those out.
        run(Job {
            compiler: "clang",
            lang,
            flags: &[
                "--target=i686-pc-windows-msvc",
                "-DBRE_CDECL=__cdecl",
                "-DBRE_STDCALL=__stdcall",
                "-DBRE_FASTCALL=__fastcall",
                "-DBENILLA_RE_PROTOTYPES",
            ],
            before: "",
        });
    }
    // The GNU spelling, with a `cdecl` macro as mingw's headers define it. Freestanding, so the
    // compile needs no 32-bit C library headers on the machine that runs it.
    run(Job {
        compiler: "clang",
        lang: "c",
        flags: &[
            "--target=i386-pc-linux-gnu",
            "-ffreestanding",
            "-DBENILLA_RE_PROTOTYPES",
        ],
        before: "#define cdecl _cdecl\n",
    });
    // The GNU spelling for Windows, with and without the prototypes and the layout assertions.
    for lang in ["c", "c++"] {
        for prototypes in ["-DBENILLA_RE_PROTOTYPES", "-DBRE_NO_PROTOTYPES"] {
            for layout in ["-DBRE_LAYOUT_ON", "-DBRE_NO_LAYOUT_CHECK"] {
                run(Job {
                    compiler: "clang",
                    lang,
                    flags: &[
                        "--target=i686-w64-windows-gnu",
                        "-ffreestanding",
                        prototypes,
                        layout,
                    ],
                    before: "",
                });
            }
        }
    }
    // The real headers of mingw, when this machine has them.
    for (compiler, lang) in [
        ("i686-w64-mingw32-gcc", "c"),
        ("i686-w64-mingw32-g++", "c++"),
    ] {
        if Command::new(compiler)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
        {
            run(Job {
                compiler,
                lang,
                flags: &[],
                before: "#include <windows.h>\n#include <math.h>\n#include <stdlib.h>",
            });
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // The control: a wrong offset and a wrong size each fail the same compile.
    for (from, to) in [
        (
            "BRE_ASSERT(offsetof(struct CCamera, m_slots) == 0x10,",
            "BRE_ASSERT(offsetof(struct CCamera, m_slots) == 0x11,",
        ),
        (
            "BRE_ASSERT(sizeof(struct CCamera) == 368,",
            "BRE_ASSERT(sizeof(struct CCamera) == 369,",
        ),
    ] {
        assert_eq!(text.matches(from).count(), 1, "{from}");
        let job = Job {
            compiler: "clang",
            lang: "c",
            flags: &["--target=i386-pc-linux-gnu", "-ffreestanding"],
            before: "",
        };
        let (ok, errors) = compile(&dir, &text.replacen(from, to, 1), &job);
        assert!(
            !ok && errors.contains("static assertion failed"),
            "{from}: {errors}"
        );
    }
    // The control: without the prototypes a global is not declared, so a bare use of its name fails
    // to compile and its accessor is the way to read it.
    std::fs::write(
        dir.join("bare.c"),
        "#include \"benilla-re.h\"\nuint32_t read(void) { return s_netBytesSent; }\n",
    )
    .unwrap();
    for (flag, declared) in [
        ("-DBRE_NO_PROTOTYPES", false),
        ("-DBENILLA_RE_PROTOTYPES", true),
    ] {
        let output = Command::new("clang")
            .args([
                "-fsyntax-only",
                "--target=i386-pc-linux-gnu",
                "-ffreestanding",
                flag,
            ])
            .arg("-I")
            .arg(&dir)
            .arg(dir.join("bare.c"))
            .output()
            .expect("clang runs");
        assert_eq!(output.status.success(), declared, "{flag}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_sdk_type_is_declared_with_the_sdk_tag_and_prototypes_are_opt_in() {
    let made = generate(
        &symbols(
            &[s("Thing", 4, "")],
            &[f("Thing", 0, 4, "uint32_t", "m_a")],
            &[func(
                0x401000,
                "Take",
                "void __cdecl (RECT *rect, Thing *this)",
            )],
            &[],
        ),
        &origin(),
    )
    .unwrap();
    let text = &made.text;
    assert!(text.contains("typedef struct tagRECT RECT;"));
    assert!(text.contains("void BRE_CDECL Take(RECT *rect, Thing *this_);"));
    let guard = text.find("#ifdef BENILLA_RE_PROTOTYPES").unwrap();
    assert!(guard < text.find("BRE_CDECL Take(").unwrap());
    assert!(text.contains("__cdecl__"));
    assert!(!text.contains("__attribute__((cdecl))"));
}

#[test]
fn the_ida_and_binary_ninja_scripts_walk_every_row_against_stand_in_apis() {
    if !Command::new("python3")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        println!("skipped: no python3 on the PATH");
        return;
    }
    let dir = scratch("scripts");
    for (path, text) in &built().files {
        let target = dir.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, text).unwrap();
    }
    let output = Command::new("python3")
        .arg("-I")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/export/scripts_test.py"))
        .arg(&dir)
        .output()
        .expect("python3 runs");
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{said}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        said.contains("IDA ok") && said.contains("Binja ok"),
        "{said}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_x64dbg_database_labels_every_named_row_at_its_rva() {
    let built = built();
    let text = &built.files["x64dbg/WoW.exe.dd32"];
    let value: serde_json::Value = serde_json::from_str(text).expect("x64dbg reads JSON");
    let named = |file: &str| {
        let t = Table::parse(file, &std::fs::read_to_string(root().join(file)).unwrap()).unwrap();
        let name = t.column("name").unwrap();
        t.rows.iter().filter(|r| !r[name].is_empty()).count()
    };
    let labels = value["labels"].as_array().unwrap();
    assert_eq!(
        labels.len(),
        named("symbols/functions.tsv") + named("symbols/globals.tsv")
    );
    let comments = value["comments"].as_array().unwrap();
    assert!(comments.len() > labels.len());
    for entry in labels.iter().chain(comments) {
        assert_eq!(entry["module"], "wow.exe");
        assert_eq!(entry["manual"], true);
        let rva = entry["address"].as_str().unwrap();
        assert!(rva.starts_with("0x") && rva[2..].chars().all(|c| c.is_ascii_hexdigit()));
        assert!(
            u32::from_str_radix(&rva[2..], 16).unwrap() < 0x906000,
            "{rva}"
        );
        assert!(entry["text"]
            .as_str()
            .is_some_and(|t| !t.is_empty() && t.len() < 512));
    }
    // CDataStore::PutInt8 is at 0x418070, an RVA of 0x18070.
    assert!(labels
        .iter()
        .any(|l| l["address"] == "0x18070" && l["text"] == "CDataStore::PutInt8"));
}
