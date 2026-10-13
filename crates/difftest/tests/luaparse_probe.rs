//! How the client's Lua 5.0 parser treats a bare `;`, settled by running its own compiler:
//!
//! - at statement level there is no empty statement: `x = 1;;`, a leading `;` and a lone `;`
//!   raise `` unexpected symbol near `;' `` (`lua50_empty_statement_probe`);
//! - inside a table constructor the field loop opens each iteration with `testnext(ls, ';')`,
//!   Lua 5.0's compatibility skip, so one extra `;` per field is eaten: `t = { a = 1;; }`
//!   compiles and `t = { a = 1;;; }` raises (`lua50_constructor_semicolon_probe`). AtlasLoot's
//!   `Core/ButtonRegistry.lua` has twenty `;;` field endings, all inside constructors, and
//!   `atlasloot_buttonregistry_compiles` compiles the real file from `BENILLA_RE_BUTTONREGISTRY_LUA`.
//!
//! `lua_open` (`0x6f6d20`) builds a real `lua_State`, then `luaL_loadbuffer` (`0x6f5690`,
//! `__fastcall(L, buff, [size, name])`, the entry that strips a byte-order mark) compiles a source.
//! Stubbed:
//!
//! - the Storm allocators under Lua's: `SMemAlloc` (`0x6462e0`, `(size, file, line, flags)`,
//!   `ret 0x10`) by size, since its `flags & 8` fill at `0x646302` writes `size` bytes through the
//!   result, and a 46 KB file makes thousands of allocations; `SMemReAlloc` (`0x646320`,
//!   `(ptr, size, …)`, `ret 0x14`) the same way; `SMemFree` (`0x646430`, `ret 0x10`);
//! - CRT `_setjmp3` (`0x742090`), returning 0. A raise is never unwound: the run stops at
//!   `luaD_throw` (`0x6f5d80`) instead and reads the error code from `edx`.
//!
//! Each input runs twice in fresh emulators: a call that returns means the chunk compiled (status
//! in `eax`); a raise longjmps through the stubbed buffer and panics, so the second run stops at
//! `luaD_throw` and reports its code (3, `LUA_ERRSYNTAX`). The controls `x = 1` and `x = 1 1` must
//! take each branch.

use benilla_re_difftest::{fixture, Emu, RegisterX86, Wow};
use std::panic::{catch_unwind, AssertUnwindSafe};

const LUA_OPEN: u64 = 0x6f6d20;
const LUAL_LOADBUFFER: u64 = 0x6f5690;
const LUAD_THROW: u64 = 0x6f5d80;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const SMEM_FREE: u64 = 0x646430;
const SETJMP3: u64 = 0x742090;
/// The helper `luaD_rawrunprotected` calls (`ecx` = the function) before `call esi`. Its result is
/// unused there, and its first-use setup reads zeroed `.bss`, so it is stubbed.
const RAWRUN_INSTRUMENT: u64 = 0x42a320;
/// CRT `sprintf`, reached only when a raise formats its line number (`0x6f7cac`), and stopped by
/// the CRT's float support check without `_cfltcvt_init`. Stubbed to write one digit: it can change
/// the message text, never the outcome.
const CRT_SPRINTF: u64 = 0x73fec4;

const ARENA_BYTES: u64 = 0xE0_0000; // 14 MB of the 16 MB scratch arena

#[derive(Debug, PartialEq)]
enum Outcome {
    /// `luaL_loadbuffer` returned its status (0: compiled).
    Returned(u32),
    /// The run reached `luaD_throw` with this error code in `edx` (3: `LUA_ERRSYNTAX`).
    Threw(u32),
}

fn fresh(wow: &Wow) -> (Emu<'_>, u32) {
    let mut emu = Emu::new(wow);
    let arena = emu.alloc(ARENA_BYTES);
    emu.stub_alloc_sized(SMEM_ALLOC, arena, arena + ARENA_BYTES, 0, 4);
    emu.stub_realloc_sized(SMEM_REALLOC, 5);
    emu.stub(SMEM_FREE, 0, 4);
    emu.stub(SETJMP3, 0, 0);
    emu.stub(RAWRUN_INSTRUMENT, 0, 0);
    let digit = emu.alloc(2);
    emu.write(digit, b"1\0");
    emu.stub_write_through(CRT_SPRINTF, 0, digit, 2, 1, 0);
    let l = emu.call(LUA_OPEN, 0, 0, &[]);
    assert_ne!(l, 0, "lua_open returned NULL");
    (emu, l)
}

fn stage_source(emu: &mut Emu, src: &[u8]) -> (u64, u64) {
    let buf = emu.alloc(src.len() as u64 + 1);
    emu.write(buf, src);
    let nm = emu.alloc(8);
    emu.write(nm, b"probe\0");
    (buf, nm)
}

fn run_case(wow: &Wow, src: &[u8]) -> Outcome {
    // A call returns only if the chunk compiles; a raise longjmps through the stub and panics.
    let clean = catch_unwind(AssertUnwindSafe(|| {
        let (mut emu, l) = fresh(wow);
        let (buf, nm) = stage_source(&mut emu, src);
        emu.call(
            LUAL_LOADBUFFER,
            l,
            buf as u32,
            &[src.len() as u32, nm as u32],
        )
    }));
    if let Ok(status) = clean {
        return Outcome::Returned(status);
    }
    // The same setup, stopped at luaD_throw.
    let (mut emu, l) = fresh(wow);
    let (buf, nm) = stage_source(&mut emu, src);
    let esp = 0x7000_0000u64 + 0x0010_0000 - 0x2000;
    emu.write_u32(esp, 0xdead_beef); // a return address this run never reaches
    emu.write_u32(esp + 4, src.len() as u32);
    emu.write_u32(esp + 8, nm as u32);
    emu.set_reg(RegisterX86::ESP, esp);
    emu.set_reg(RegisterX86::ECX, u64::from(l));
    emu.set_reg(RegisterX86::EDX, buf);
    emu.run(LUAL_LOADBUFFER, LUAD_THROW);
    Outcome::Threw(emu.get_reg(RegisterX86::EDX) as u32)
}

#[test]
fn lua50_empty_statement_probe() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let cases: &[(&str, &str)] = &[
        ("control-clean", "x = 1"),
        ("control-error", "x = 1 1"),
        ("single-semi", "x = 1;"),
        ("double-semi", "x = 1;;"),
        ("lone-semi", ";"),
        ("leading-semi", ";x = 1"),
        ("atlasloot-line", "Back_Title = AL[\"Factions\"];;"),
        ("empty-chunk", ""),
    ];
    for (label, src) in cases {
        let outcome = run_case(&wow, src.as_bytes());
        println!("PROBE {label}: {src:?} -> {outcome:?}");
    }
    // Controls: each outcome is reachable.
    assert_eq!(run_case(&wow, b"x = 1"), Outcome::Returned(0), "accept leg");
    assert_eq!(run_case(&wow, b"x = 1 1"), Outcome::Threw(3), "reject leg");
    // At statement level there is no empty statement.
    assert_eq!(run_case(&wow, b"x = 1;;"), Outcome::Threw(3), "stmt `;;`");
}

/// `constructor` (`0x6fd430`) opens each field iteration with `testnext(ls, ';')` (`0x6fd4a0 mov
/// edx,0x3b; call 0x6fccf0`, result discarded; Lua 5.1 removed it) and continues on
/// `testnext(',') || testnext(';')` (`0x6fd4f5`, `0x6fd505`). So at most two `;` pass between
/// fields or before `}`; a third reaches `prefixexp` (`0x6fde40`) and raises. `chunk` (`0x6fcc90`)
/// tests one `;` after a statement and has no such skip, so `;;` there still raises.
#[test]
fn lua50_constructor_semicolon_probe() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let cases: &[(&str, &str, Outcome)] = &[
        // AtlasLoot's shape: a field ended by `;;` before `}` compiles.
        (
            "ctor-atlasloot-shape",
            "return { Title = \"x\"; Back_Page = \"y\"; Back_Title = \"z\";; }",
            Outcome::Returned(0),
        ),
        (
            "ctor-atlasloot-line",
            "t = { Back_Title = AL[\"Factions\"];; }",
            Outcome::Returned(0),
        ),
        (
            "ctor-double-semi-mid",
            "t = { a = 1;; b = 2 }",
            Outcome::Returned(0),
        ),
        (
            "ctor-comma-semi",
            "t = { a = 1,; b = 2 }",
            Outcome::Returned(0),
        ),
        ("ctor-single-semi", "t = { a = 1; }", Outcome::Returned(0)),
        // The skip eats one extra `;` per iteration; a third raises.
        ("ctor-triple-semi", "t = { a = 1;;; }", Outcome::Threw(3)),
        // It runs on the first iteration too: `{ ; }` is an empty table,
        ("ctor-lone-semi", "t = { ; }", Outcome::Returned(0)),
        // but `{ ;; }` raises.
        ("ctor-double-lone-semi", "t = { ;; }", Outcome::Threw(3)),
        // Control: empty constructor.
        ("ctor-empty", "t = { }", Outcome::Returned(0)),
        // At statement level `;;` still raises.
        ("stmt-double-semi", "x = 1;;", Outcome::Threw(3)),
    ];
    for (label, src, expect) in cases {
        let outcome = run_case(&wow, src.as_bytes());
        println!("PROBE {label}: {src:?} -> {outcome:?}");
        assert_eq!(outcome, *expect, "{label}: {src:?}");
    }
}

/// AtlasLoot Enhanced v4.07.01's `Core/ButtonRegistry.lua`, 46 KB with twenty `;;` field endings
/// inside table constructors, compiles in the client's `luaL_loadbuffer`, as the addon runs on the
/// real client. The chunk is only compiled, so its globals do not matter. The file is a third
/// party's and is read from `BENILLA_RE_BUTTONREGISTRY_LUA`.
#[test]
fn atlasloot_buttonregistry_compiles() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let Some(src) = fixture("BENILLA_RE_BUTTONREGISTRY_LUA") else {
        return;
    };
    // The file is the one meant: its twenty `;;` are present.
    let semis = src.windows(2).filter(|&w| w == b";;").count();
    assert_eq!(semis, 20, "fixture carries the twenty `;;` sites");
    let outcome = run_case(&wow, &src);
    println!(
        "PROBE atlasloot-buttonregistry ({} bytes, {semis} `;;` sites) -> {outcome:?}",
        src.len()
    );
    assert_eq!(outcome, Outcome::Returned(0), "ButtonRegistry.lua compiles");
}

/// The lexer has no long-string levels: Lua 5.1's `[=[ … ]=]` is not in 5.0. `luaX_lex`
/// (`0x6ff610`) tests the character after `[` once (`0x6ff793 cmp eax,0x5b`): anything else gives
/// the token `[` (`0x6ff79c`), and `[` calls `read_long_string` (`0x700010`), which never compares
/// with `=` (its dispatch at `0x700234` sends `=` to the default arm `0x7001bc`) and counts only
/// `[[`/`]]` nesting (`0x70011b inc ebx`, `0x700196 dec ebx`). A long comment is entered only by
/// `--[[` (`0x6ff68b`, `0x6ff6b4`); `--[=` falls to the line comment at `0x6ff6c9`, so `--[==[`
/// comments out one line instead of raising.
#[test]
fn lua50_long_string_level_probe() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let cases: &[(&str, &str, Outcome)] = &[
        // Controls for both outcomes.
        ("control-clean", "x = 1", Outcome::Returned(0)),
        ("control-error", "x = 1 1", Outcome::Threw(3)),
        // A plain long string and a nested one compile.
        ("long-plain", "x = [[hello]]", Outcome::Returned(0)),
        ("long-nested", "x = [[a[[b]]c]]", Outcome::Returned(0)),
        // Level syntax raises: `[=` lexes as the token `[`, then `=`.
        ("level-1", "x = [=[hello]=]", Outcome::Threw(3)),
        ("level-2", "x = [==[hello]==]", Outcome::Threw(3)),
        // `--[[ ]]` is a long comment; `--[==[` comments out one line, so the body is code and
        // the closing `]==]` raises.
        ("comment-plain", "--[[ c ]]\nx = 1", Outcome::Returned(0)),
        (
            "comment-level",
            "--[==[\nx = 1\n]==]\ny = 2",
            Outcome::Threw(3),
        ),
        // With the body on the same line as `--[==[`, the line comment takes it all.
        (
            "comment-level-oneline",
            "--[==[ x = 1 ]==]",
            Outcome::Returned(0),
        ),
    ];
    for (label, src, expect) in cases {
        let outcome = run_case(&wow, src.as_bytes());
        println!("PROBE {label}: {src:?} -> {outcome:?}");
        assert_eq!(outcome, *expect, "{label}: {src:?}");
    }
}

const LUA_PUSHSTRING: u64 = 0x6f3890;
const LUA_GETINFO: u64 = 0x6fbc70;
const LUAO_CHUNKID: u64 = 0x6f5c40;
const LOADSTRING: u64 = 0x703280;
/// `lua_Debug`'s `short_src` offset and size: `lua_getstack` (`0x6fbaa0`) writes `ar->i_ci`, the
/// field after `short_src`, at `0x6fbade mov dword [ecx+0x5c],0`, and 0x20 + 60 = 0x5c.
const AR_SHORT_SRC: u64 = 0x20;
const LUA_IDSIZE: u64 = 60;

fn read_cstr(emu: &Emu, va: u64, cap: usize) -> String {
    let bytes = emu.read(va, cap);
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// `luaO_chunkid` (`0x6f5c40`, `__fastcall(ecx = out, edx = source, [ebp+8] = bufflen)`, `ret 4`)
/// renders a chunk's `source` into the 60-byte `short_src` that error prefixes and `debugstack`
/// frames carry, as stock Lua 5.0 does:
///
/// - `'='` (`0x6f5c49`): `source+1` as it is;
/// - `'@'` (`0x6f5c6b`): the budget less 8 (`0x6f5c78`); a longer name keeps its tail after `...`
///   (the dword at `0x800188`, stored at `0x6f5c9a` and `0x6f5ca0`);
/// - otherwise: cut at the first newline (`strcspn`, `0x4125d0`, with `"\n"` at `0x835144`), the
///   budget less 0x11 (`0x6f5cdf`), wrapped in `[string "` and `"]` (`0x871714`, `0x871710`), with
///   `...` when cut.
///
/// It needs no `lua_State`.
#[test]
fn lua50_chunkid_render_probe() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let cases: &[(&str, &str, &str)] = &[
        // `=`: the rest as it is, as the client names its own chunks.
        ("eq-literal", "=(loadstring)", "(loadstring)"),
        // `@`: a file name, short enough to keep whole.
        ("at-short", "@Foo.lua", "Foo.lua"),
        // `@` over budget: the last 52 bytes after `...`, cut by count and not at a path
        // separator, so 57 characters lose `Inter`.
        (
            "at-long",
            "@Interface\\AddOns\\FooBarBaz\\Libs\\AceLibrary\\AceLibrary.lua",
            "...face\\AddOns\\FooBarBaz\\Libs\\AceLibrary\\AceLibrary.lua",
        ),
        // 61 characters is the longest path that keeps its `\AddOns\`: the `\` sits at index 9.
        (
            "at-boundary-keeps-addons",
            "@Interface\\AddOns\\FuBar_QuestFu\\libs\\AceLibrary\\AceLibrary.lua",
            "...\\AddOns\\FuBar_QuestFu\\libs\\AceLibrary\\AceLibrary.lua",
        ),
        // One more, and `\AddOns\` is cut.
        (
            "at-boundary-loses-addons",
            "@Interface\\AddOns\\FuBar_QuestsFu\\libs\\AceLibrary\\AceLibrary.lua",
            "...AddOns\\FuBar_QuestsFu\\libs\\AceLibrary\\AceLibrary.lua",
        ),
        // Source text: wrapped, cut at the first newline, with `...` when cut.
        ("src-short", "return 1+", "[string \"return 1+\"]"),
        (
            "src-newline",
            "local a = 1\nreturn 1+",
            "[string \"local a = 1...\"]",
        ),
        (
            "src-long",
            "return 1+ -- padding padding padding padding padding padding",
            "[string \"return 1+ -- padding padding padding paddin...\"]",
        ),
    ];
    let mut emu = Emu::new(&wow);
    for (label, source, expect) in cases {
        let out = emu.alloc(256);
        emu.write(out, &[0u8; 256]);
        let src = emu.alloc(source.len() as u64 + 1);
        emu.write(src, source.as_bytes());
        emu.write_u32(src + source.len() as u64, 0);
        emu.call(LUAO_CHUNKID, out as u32, src as u32, &[LUA_IDSIZE as u32]);
        let got = read_cstr(&emu, out, 128);
        println!("PROBE {label}: {source:?} -> {got:?} ({} bytes)", got.len());
        assert_eq!(&got, expect, "{label}");
        assert!(got.len() < LUA_IDSIZE as usize, "{label} fits short_src");
    }
}

/// `loadstring`'s default chunk name is the source itself: `luaB_loadstring` (`0x703280`) calls
/// `luaL_checklstring(L, 1, &l)` (`0x703291`), then `luaL_optlstring(L, 2, s, NULL)` with the
/// default `edi = s` (`0x70329a push edi`, `0x7032a2 call 0x6f4c20`), then `luaL_loadbuffer`
/// (`0x7032b0`). The test loads a chunk and reads its name back with `lua_getinfo(L, ">S", &ar)`.
#[test]
fn lua50_loadstring_default_chunkname_probe() {
    let Some(wow) = Wow::load() else {
        return;
    };
    // Sources that compile: a raise would longjmp through the stubbed `_setjmp3`.
    let table: &[(&str, &str, Option<&str>, &str)] = &[
        ("default", "return 1", None, "[string \"return 1\"]"),
        ("explicit-eq", "return 2", Some("=myname"), "myname"),
        (
            "explicit-at",
            "return 3",
            Some("@some/file.lua"),
            "some/file.lua",
        ),
    ];
    for (label, source, name, expect) in table {
        let (mut emu, l) = fresh(&wow);
        let s = emu.alloc(source.len() as u64 + 1);
        emu.write(s, source.as_bytes());
        emu.write_u32(s + source.len() as u64, 0);
        emu.call(LUA_PUSHSTRING, l, s as u32, &[]);
        if let Some(n) = name {
            let np = emu.alloc(n.len() as u64 + 1);
            emu.write(np, n.as_bytes());
            emu.write_u32(np + n.len() as u64, 0);
            emu.call(LUA_PUSHSTRING, l, np as u32, &[]);
        }
        let nres = emu.call(LOADSTRING, l, 0, &[]);
        assert_eq!(
            nres, 1,
            "{label}: the chunk compiles, so loadstring returns 1"
        );
        let ar = emu.alloc(0x60);
        emu.write(ar, &[0u8; 0x60]);
        let what = emu.alloc(4);
        emu.write(what, b">S\0");
        let ok = emu.call(LUA_GETINFO, l, what as u32, &[ar as u32]);
        assert_ne!(ok, 0, "{label}: lua_getinfo(\">S\") succeeded");
        let short_src = read_cstr(&emu, ar + AR_SHORT_SRC, LUA_IDSIZE as usize);
        println!("PROBE {label}: loadstring({source:?}, {name:?}) -> short_src={short_src:?}");
        assert_eq!(&short_src, expect, "{label}");
    }
}
