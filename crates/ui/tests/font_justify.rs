//! The shared font methods `SetJustifyH` (`0x79fc20`), `SetJustifyV` (`0x79fce0`) and
//! `GetJustifyV` (`0x79fd70`) against the exe, each run whole
//! (`systems/ui/fontmethods.md#justify-and-the-followed-font`). `ParseJustify` (`0x6f1990`) and
//! `GetJustifyName` (`0x6f1a00`) run from the image with `SStrCmpI`. The Lua calls are stubbed:
//! `lua_isstring` (`0x6f3510`) and `lua_tostring` (`0x6f3690`) answer the test's argument 2,
//! `luaL_error` (`0x6f4940`) and `lua_pushstring` (`0x6f3890`) log what they get, and the font's
//! vtable slot 5, `PushChanges`, logs its call.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::font_justify::{
    justify_name, parse_justify, set_justify, JustifyState, H_MASK, V_MASK,
};
use std::cell::RefCell;
use std::rc::Rc;

const SET_H: u64 = 0x79fc20;
const SET_V: u64 = 0x79fce0;
const GET_V: u64 = 0x79fd70;
const LUA_ISSTRING: u64 = 0x6f3510;
const LUA_TOSTRING: u64 = 0x6f3690;
const LUAL_ERROR: u64 = 0x6f4940;
const LUA_PUSHSTRING: u64 = 0x6f3890;
const USAGE_H: u32 = 0x87c77c;
const USAGE_V: u32 = 0x87c7a0;
const L: u32 = 0x00c0_ffee;

#[derive(Debug, PartialEq, Eq)]
enum Ev {
    Error(u32, u32, u32),
    Push,
    PushString(Vec<u8>),
}

struct Rig<'a> {
    e: Emu<'a>,
    font: u64,
    name: u32,
    arg: Rc<RefCell<Option<u32>>>,
    log: Rc<RefCell<Vec<Ev>>>,
}

fn cstr(e: &mut Emu, s: &[u8]) -> u32 {
    let va = e.alloc(s.len() as u64 + 1);
    e.write(va, s);
    e.write(va + s.len() as u64, &[0]);
    va as u32
}

fn rig(wow: &Wow) -> Rig<'_> {
    let mut e = Emu::new(wow);
    let arg: Rc<RefCell<Option<u32>>> = Rc::default();
    let log: Rc<RefCell<Vec<Ev>>> = Rc::default();
    {
        let arg = arg.clone();
        e.stub_fn(LUA_ISSTRING, 0, move |io| {
            assert_eq!(
                (io.reg(RegisterX86::ECX) as u32, io.reg(RegisterX86::EDX)),
                (L, 2)
            );
            u32::from(arg.borrow().is_some())
        });
    }
    {
        let arg = arg.clone();
        e.stub_fn(LUA_TOSTRING, 0, move |io| {
            assert_eq!(
                (io.reg(RegisterX86::ECX) as u32, io.reg(RegisterX86::EDX)),
                (L, 2)
            );
            arg.borrow().expect("tostring of a string")
        });
    }
    {
        let log = log.clone();
        e.stub_fn(LUAL_ERROR, 0, move |io| {
            log.borrow_mut()
                .push(Ev::Error(io.arg(0), io.arg(1), io.arg(2)));
            0
        });
    }
    {
        let log = log.clone();
        e.stub_fn(LUA_PUSHSTRING, 0, move |io| {
            assert_eq!(io.reg(RegisterX86::ECX) as u32, L);
            let s = io.read_cstr(io.reg(RegisterX86::EDX));
            log.borrow_mut().push(Ev::PushString(s));
            0
        });
    }
    let push = e.alloc(0x10);
    e.write(push, &[0xc3]);
    {
        let log = log.clone();
        e.stub_fn(push, 0, move |_| {
            log.borrow_mut().push(Ev::Push);
            0
        });
    }
    let vtable = e.alloc(0x40);
    e.write_u32(vtable + 0x14, push as u32);
    let font = e.alloc(0x100);
    e.write_u32(font, vtable as u32);
    let name = cstr(&mut e, b"TestFont");
    Rig {
        e,
        font,
        name,
        arg,
        log,
    }
}

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// The argument 2 probes: the six names in several cases, near misses, and no string.
fn names() -> Vec<Option<Vec<u8>>> {
    let mut out = Vec::new();
    for n in ["LEFT", "CENTER", "RIGHT", "TOP", "MIDDLE", "BOTTOM"] {
        out.push(Some(n.as_bytes().to_vec()));
        out.push(Some(n.to_ascii_lowercase().into_bytes()));
    }
    for n in [
        "Center",
        "bOtToM",
        "",
        "LEFTX",
        "LEF",
        "UNKNOWN",
        " TOP",
        "JUSTIFY_LEFT",
    ] {
        out.push(Some(n.as_bytes().to_vec()));
    }
    out.push(None);
    out
}

fn states(seed: &mut u32) -> Vec<JustifyState> {
    let mut out = Vec::new();
    for word in [0u32, 0x01, 0x12, 0x21, 0x3f, 0x07, 0x38, 0xffff_ffc0] {
        for valid in [0u32, 0x200, 0x202, 0x1f1f, 0xffff_fdff] {
            for follow in [0u32, 0x2, 0x1f, 0xffff_ffff] {
                out.push(JustifyState {
                    word,
                    valid,
                    follow,
                });
            }
        }
    }
    for _ in 0..32 {
        out.push(JustifyState {
            word: lcg(seed),
            valid: lcg(seed),
            follow: lcg(seed),
        });
    }
    out
}

fn run_setter(f: u64, mask: u32, usage: u32) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = rig(&wow);
    let mut seed = 0xabcd_u32;
    for name in names() {
        let ptr = name.as_ref().map(|n| cstr(&mut r.e, n));
        for s in states(&mut seed) {
            *r.arg.borrow_mut() = ptr;
            r.e.write_u32(r.font + 0x54, s.word);
            r.e.write_u32(r.font + 0x38, s.valid);
            r.e.write_u32(r.font + 0x2c, s.follow);
            r.log.borrow_mut().clear();
            let ret = r.e.call(f, r.name, r.font as u32, &[L]);
            let got = JustifyState {
                word: r.e.read_u32(r.font + 0x54),
                valid: r.e.read_u32(r.font + 0x38),
                follow: r.e.read_u32(r.font + 0x2c),
            };
            let what = format!(
                "{:?} {s:x?}",
                name.as_ref().map(|n| String::from_utf8_lossy(n))
            );
            assert_eq!(ret, 0, "{what}: values");
            match name.as_deref().and_then(parse_justify) {
                Some(bit) => {
                    assert_eq!(got, set_justify(s, bit, mask), "{what}: state");
                    assert_eq!(r.log.borrow().as_slice(), &[Ev::Push], "{what}: push");
                }
                None => {
                    assert_eq!(got, s, "{what}: unchanged");
                    assert_eq!(
                        r.log.borrow().as_slice(),
                        &[Ev::Error(L, usage, r.name)],
                        "{what}: usage"
                    );
                }
            }
        }
    }
}

#[test]
fn font_justify_set_h_difftest() {
    run_setter(SET_H, H_MASK, USAGE_H);
}

#[test]
fn font_justify_set_v_difftest() {
    run_setter(SET_V, V_MASK, USAGE_V);
}

#[test]
fn font_justify_get_v_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = rig(&wow);
    let mut seed = 0x77_u32;
    let mut words: Vec<u32> = (0..0x80).collect();
    words.extend((0..64).map(|_| lcg(&mut seed)));
    for word in words {
        r.e.write_u32(r.font + 0x54, word);
        r.log.borrow_mut().clear();
        let ret = r.e.call(GET_V, 0, r.font as u32, &[L]);
        let want = justify_name(word & V_MASK).as_bytes().to_vec();
        assert_eq!(ret, 1, "{word:#x}: values");
        assert_eq!(
            r.log.borrow().as_slice(),
            &[Ev::PushString(want)],
            "{word:#x}: name"
        );
    }
}
