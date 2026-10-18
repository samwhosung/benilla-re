//! The Region size and edge methods and the game's screen-size and cursor globals against the exe,
//! each run whole: `GetWidth` (`0x7a1e00`), `GetHeight` (`0x7a2030`), `SetWidth` (`0x7a1f20`),
//! `SetHeight` (`0x7a2150`), `GetLeft`, `GetRight`, `GetTop`, `GetBottom` (`0x7a1980` to
//! `0x7a1ce0`), `GetCenter` (`0x7a1810`), `GetScreenWidth` and `GetScreenHeight` (`0x48b480`,
//! `0x48b4d0`) and `GetCursorPosition` (`0x48b820`).
//!
//! Each case first runs `SetScreenAspect` (`0x41ad10`) on the aspect, so the cells are the
//! client's own. The exe's code runs from the method's entry to its `ret`: the screen helpers, the
//! layout part's size slots `0x768420` and `0x768410`, `0x768310`, `0x768320`, `0x494590`,
//! `lua_pushnumber` (`0x6f3810`) and `lua_pushnil` (`0x6f37f0`). Stubs stand in for the Lua stack
//! reads (`lua_type` answers a table, `lua_rawgeti` and `lua_settop` do nothing, `lua_touserdata`
//! answers the region, `lua_isnumber` answers 1, `lua_tonumber` answers the case's number), the
//! region's type test (slot 0x10 answers 1), the layout's size setters (slots 0x14 and 0x18, which
//! the test reads the argument of), `CLayoutFrame::Resize`, which the test checks is called for a
//! pending layout, and `luaL_error`, which no case reaches.
//!
//! Probes: six aspects (4:3, 16:9, 16:10, 5:4, 2560:1080 and 1); the layout scales 1, the floor
//! 0.64, 0.9, 1.5 and 0.3, and for the edge getters also 0, ±∞ and NaN; zero, negative and NaN
//! sizes, NaN and infinite edges, an invalid rectangle, a pending layout; and random values.

#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_screencoord::{set_scale, ScaleState};
use benilla_re_ui::region_units::{
    cursor_position, get_center, get_edge, get_height, get_width, set_size, ui_screen_size,
    LayoutPart, RECT_VALID,
};

const LUA_TYPE: u64 = 0x6f3400;
const LUA_RAWGETI: u64 = 0x6f3bc0;
const LUA_TOUSERDATA: u64 = 0x6f3740;
const LUA_SETTOP: u64 = 0x6f3080;
const LUA_ISNUMBER: u64 = 0x6f34d0;
const LUA_TONUMBER: u64 = 0x6f3620;
const LUA_ERROR: u64 = 0x6f4940;
const RESIZE: u64 = 0x7680e0;
const UI_PARENT: u64 = 0xb4b44c;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

/// Equal bits, or both NaN: the x87's default NaN is negative where the host's is positive.
fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

const ASPECTS: [f32; 6] = [
    4.0 / 3.0,
    16.0 / 9.0,
    16.0 / 10.0,
    5.0 / 4.0,
    2560.0 / 1080.0,
    1.0,
];
const SCALES: [f32; 5] = [1.0, 0.64, 0.9, 1.5, 0.3];

/// The emulated state: a Lua state whose stack the real `lua_pushnumber` writes, a region with a
/// fake script vtable and a layout vtable over the exe's own size slots.
struct Rig<'a> {
    emu: Emu<'a>,
    lua: u64,
    stack: u64,
    region: u64,
    number: u64,
    set_w: u64,
    set_h: u64,
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut emu = Emu::new(wow);
        let lua = emu.alloc(0x40);
        let stack = emu.alloc(0x100);
        let region = emu.alloc(0x200);
        let number = emu.alloc(0x10);
        let code = emu.alloc(0x40);
        emu.write(code, &[0xc3; 0x40]);
        let (is_type, set_w, set_h) = (code, code + 0x10, code + 0x20);
        let script_vt = emu.alloc(0x40);
        emu.write_u32(script_vt + 0x10, is_type as u32);
        let layout_vt = emu.alloc(0x40);
        emu.write_u32(layout_vt + 0x14, set_w as u32);
        emu.write_u32(layout_vt + 0x18, set_h as u32);
        emu.write_u32(layout_vt + 0x1c, 0x768420);
        emu.write_u32(layout_vt + 0x20, 0x768410);
        emu.write_u32(region, script_vt as u32);
        emu.write_u32(region + 0x24, layout_vt as u32);
        emu.stub(is_type, 1, 1);
        emu.stub(set_w, 0, 1);
        emu.stub(set_h, 0, 1);
        emu.stub(LUA_TYPE, 5, 0);
        emu.stub(LUA_RAWGETI, 0, 1);
        emu.stub(LUA_TOUSERDATA, region as u32, 0);
        emu.stub(LUA_SETTOP, 0, 0);
        emu.stub(LUA_ISNUMBER, 1, 0);
        emu.stub_f64(LUA_TONUMBER, number, 0);
        emu.stub(LUA_ERROR, 0, 0);
        emu.stub(RESIZE, 0, 1);
        Rig {
            emu,
            lua,
            stack,
            region,
            number,
            set_w,
            set_h,
        }
    }

    /// Sets the cells through the exe's `SetScreenAspect` and returns them as the exe stored them.
    fn aspect(&mut self, a: f32) -> ScaleState {
        self.emu.call(0x41ad10, 0, 0, &[a.to_bits()]);
        let st = ScaleState {
            s: self.emu.read_f32(0x832a40),
            g44: self.emu.read_f32(0x832a44),
            g48: self.emu.read_f32(0x832a48),
            s075: self.emu.read_f32(0x832a4c),
        };
        assert_eq!(st, set_scale(a), "cells a={a}");
        st
    }

    fn layout(&mut self, l: &LayoutPart) {
        let base = self.region + 0x24;
        self.emu.write_u32(base + 0x3c, l.flags);
        for (i, v) in l.rect.iter().enumerate() {
            self.emu.write_u32(base + 0x40 + 4 * i as u64, v.to_bits());
        }
        self.emu.write_u32(base + 0x50, l.width.to_bits());
        self.emu.write_u32(base + 0x54, l.height.to_bits());
        self.emu.write_u32(base + 0x58, l.scale.to_bits());
    }

    /// Runs a binding whole and returns what it pushed: `Some(number)` or `None` for nil, per value.
    fn call(&mut self, f: u64) -> Vec<Option<f64>> {
        self.emu.write_u32(self.lua + 0x8, self.stack as u32);
        self.emu.write(self.stack, &[0xaa; 0x40]);
        self.emu.clear_calls();
        let n = self.emu.call(f, self.lua as u32, 0, &[]);
        assert!(
            !self.emu.calls().iter().any(|c| c.addr == LUA_ERROR),
            "{f:#x} raised an error"
        );
        let top = u64::from(self.emu.read_u32(self.lua + 0x8));
        let pushed = ((top - self.stack) / 0x10) as u32;
        assert_eq!(pushed, n, "{f:#x} returns its push count");
        (0..u64::from(n))
            .map(|i| {
                let tv = self.stack + 0x10 * i;
                match self.emu.read_u32(tv) {
                    3 => {
                        let lo = u64::from(self.emu.read_u32(tv + 8));
                        let hi = u64::from(self.emu.read_u32(tv + 12));
                        Some(f64::from_bits(lo | hi << 32))
                    }
                    0 => None,
                    t => panic!("{f:#x} pushed type {t}"),
                }
            })
            .collect()
    }

    fn resized(&self) -> bool {
        self.emu
            .calls()
            .iter()
            .any(|c| c.addr == RESIZE && c.args[0] == 1)
    }

    /// Runs a setter whole on the number `v` and returns the float it passed to `slot`.
    fn set(&mut self, f: u64, slot: u64, v: f64) -> f32 {
        let b = v.to_bits();
        self.emu.write_u32(self.number, b as u32);
        self.emu.write_u32(self.number + 4, (b >> 32) as u32);
        let pushed = self.call(f);
        assert!(pushed.is_empty(), "{f:#x} returns nothing");
        let c = self
            .emu
            .calls()
            .iter()
            .find(|c| c.addr == slot)
            .unwrap_or_else(|| panic!("{f:#x} did not call its size slot"));
        f32::from_bits(c.args[0])
    }
}

fn part(flags: u32, rect: [f32; 4], width: f32, height: f32, scale: f32) -> LayoutPart {
    LayoutPart {
        flags,
        rect,
        width,
        height,
        scale,
    }
}

/// The layout parts the getters are probed on, for one aspect and scale.
fn parts(seed: &mut u32, scale: f32) -> Vec<LayoutPart> {
    let r = [100.0f32, -20.5, 612.25, 383.0];
    let mut v = vec![
        part(RECT_VALID, r, 0.0, 0.0, scale),
        part(RECT_VALID, r, -0.0, -0.0, scale),
        part(RECT_VALID, r, 0.25, 0.1875, scale),
        part(RECT_VALID, r, f32::NAN, -3.0, scale),
        part(0, r, 0.0, 0.0, scale),
        part(0, r, 0.4, 0.3, scale),
        part(RECT_VALID | 4, r, 0.0, 0.5, scale),
        part(RECT_VALID, [0.0; 4], 0.0, 0.0, scale),
        part(
            RECT_VALID,
            [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::NAN],
            0.0,
            0.0,
            scale,
        ),
        part(
            RECT_VALID,
            [f32::NEG_INFINITY, 2.0, f32::INFINITY, f32::INFINITY],
            0.0,
            0.0,
            scale,
        ),
        part(
            RECT_VALID,
            [1.0e-30, -1.0e30, 3.0e30, 2.0e-30],
            0.0,
            0.0,
            scale,
        ),
    ];
    for _ in 0..200 {
        let rect = [
            rng(seed, -0.2, 0.8),
            rng(seed, -0.2, 1.0),
            rng(seed, -0.2, 0.8),
            rng(seed, -0.2, 1.0),
        ];
        let w = if next(seed) & 3 == 0 {
            0.0
        } else {
            rng(seed, -1.0, 1.0)
        };
        let h = if next(seed) & 3 == 0 {
            0.0
        } else {
            rng(seed, -1.0, 1.0)
        };
        v.push(part(RECT_VALID, rect, w, h, scale));
    }
    v
}

#[test]
fn region_size_getters_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut seed = 0x7a1e_0001u32;
    let mut total = 0u32;
    for a in ASPECTS {
        let st = rig.aspect(a);
        for scale in SCALES {
            for l in parts(&mut seed, scale) {
                rig.layout(&l);
                let w = rig.call(0x7a1e00);
                assert_eq!(rig.resized(), l.width == 0.0 && l.flags & 4 != 0);
                let h = rig.call(0x7a2030);
                assert_eq!(rig.resized(), l.height == 0.0 && l.flags & 4 != 0);
                let (w, h) = (w[0].expect("width"), h[0].expect("height"));
                assert!(same(w, get_width(st, &l)), "GetWidth a={a} {l:?}");
                assert!(same(h, get_height(st, &l)), "GetHeight a={a} {l:?}");
                total += 2;
            }
        }
    }
    eprintln!("region_size_getters_whole: {total} bit-exact");
}

#[test]
fn region_edge_getters_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut seed = 0x7a19_8001u32;
    let mut total = 0u32;
    // The edge getters divide by the scale: zero, the infinities and NaN as well.
    let scales =
        SCALES
            .iter()
            .copied()
            .chain([0.0, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN]);
    for a in ASPECTS {
        let st = rig.aspect(a);
        for scale in scales.clone() {
            for l in parts(&mut seed, scale) {
                rig.layout(&l);
                for (f, index) in [(0x7a1980, 1), (0x7a1aa0, 3), (0x7a1bc0, 2), (0x7a1ce0, 0)] {
                    let got = rig.call(f);
                    assert_eq!(rig.resized(), l.flags & 4 != 0);
                    let want = get_edge(st, &l, index);
                    assert_eq!(got.len(), 1);
                    match (got[0], want) {
                        (Some(g), Some(w)) => assert!(same(g, w), "{f:#x} a={a} {l:?}"),
                        (None, None) => {}
                        _ => panic!("{f:#x} nil mismatch a={a} {l:?}"),
                    }
                    total += 1;
                }
                let got = rig.call(0x7a1810);
                match get_center(st, &l) {
                    Some((x, y)) => {
                        let (gx, gy) = (got[0].expect("x"), got[1].expect("y"));
                        assert!(same(gx, x) && same(gy, y), "GetCenter a={a} {l:?}");
                    }
                    None => assert_eq!(got, vec![None, None]),
                }
                total += 2;
            }
        }
    }
    eprintln!("region_edge_getters_whole: {total} bit-exact");
}

#[test]
fn region_size_setters_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut seed = 0x7a1f_2001u32;
    let mut total = 0u32;
    let fixed = [
        0.0f64,
        -0.0,
        1.0,
        100.0,
        768.0,
        1024.0,
        -250.5,
        0.1,
        1.0e-40,
        3.4e38,
        1.0e300,
        f64::INFINITY,
        f64::NAN,
        // a double that rounds to a float tie, then halfway cases of the quotient
        1.0 + f64::from(f32::EPSILON) / 2.0,
        1536.0 / 3.0,
    ];
    for a in ASPECTS {
        let st = rig.aspect(a);
        let cases: Vec<f64> = fixed
            .iter()
            .copied()
            .chain((0..1500).map(|_| f64::from(rng(&mut seed, -4096.0, 4096.0)) * 1.000_000_1))
            .collect();
        for v in cases {
            let w = rig.set(0x7a1f20, rig.set_w, v);
            let h = rig.set(0x7a2150, rig.set_h, v);
            let want = set_size(st, v);
            let same32 = |g: f32| g.to_bits() == want.to_bits() || (g.is_nan() && want.is_nan());
            assert!(same32(w), "SetWidth a={a} v={v}: {w} != {want}");
            assert!(same32(h), "SetHeight a={a} v={v}: {h} != {want}");
            total += 2;
        }
    }
    eprintln!("region_size_setters_whole: {total} bit-exact");
}

#[test]
fn ui_screen_size_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let parent = rig.emu.alloc(0x100);
    let mut total = 0u32;
    for a in ASPECTS {
        let st = rig.aspect(a);
        for s in [
            None,
            Some(1.0f32),
            Some(0.9),
            Some(0.64),
            Some(768.0 / 1080.0),
            Some(1.5),
        ] {
            match s {
                Some(v) => {
                    rig.emu.write_u32(UI_PARENT, parent as u32);
                    rig.emu.write_u32(parent + 0x7c, v.to_bits());
                }
                None => rig.emu.write_u32(UI_PARENT, 0),
            }
            for (f, height) in [(0x48b480, false), (0x48b4d0, true)] {
                let got = rig.call(f)[0].expect("size");
                let want = ui_screen_size(st, s, height);
                assert!(same(got, want), "{f:#x} a={a} s={s:?}: {got} != {want}");
                total += 1;
            }
        }
    }
    // The height a 4:3 window answers with no UIParent, and with UIParent at scale 1.
    let st = rig.aspect(4.0 / 3.0);
    rig.emu.write_u32(UI_PARENT, 0);
    assert_eq!(rig.call(0x48b4d0)[0], Some(767.999_942_779_541_8));
    assert_eq!(ui_screen_size(st, Some(1.0), true), 767.999_942_779_541_8);
    eprintln!("ui_screen_size_whole: {total} bit-exact");
}

#[test]
fn ui_cursor_position_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let top = rig.emu.alloc(0x1200);
    rig.emu.write_u32(0xcf0bd8, top as u32);
    let mut seed = 0x48b8_2001u32;
    let mut total = 0u32;
    for a in ASPECTS {
        let st = rig.aspect(a);
        let fixed = [
            (0.0f32, 0.0f32),
            (1.0, 1.0),
            (0.5, 0.25),
            (-0.0, 1.0e-30),
            (f32::INFINITY, f32::NAN),
            (-1.0, 2.0),
        ];
        let cases: Vec<(f32, f32)> = fixed
            .into_iter()
            .chain((0..500).map(|_| (rng(&mut seed, -0.5, 1.5), rng(&mut seed, -0.5, 1.5))))
            .collect();
        for (fx, fy) in cases {
            rig.emu.write_u32(top + 0x1118, fx.to_bits());
            rig.emu.write_u32(top + 0x111c, fy.to_bits());
            let got = rig.call(0x48b820);
            let (x, y) = cursor_position(st, fx, fy);
            let (gx, gy) = (got[0].expect("x"), got[1].expect("y"));
            assert!(
                same(gx, x) && same(gy, y),
                "GetCursorPosition a={a} {fx} {fy}"
            );
            total += 2;
        }
    }
    eprintln!("ui_cursor_position_whole: {total} bit-exact");
}
