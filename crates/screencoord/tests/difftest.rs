//! The screen aspect scale against the exe. Each function runs whole: it calls nothing and reads
//! its `.data` globals, which each case writes. `SetScreenAspect` is checked through the four
//! globals it sets. Fixed cases (4/3, the default, and other exact ratios), then random ones. The
//! single-value functions are read at 53 bits and probed at zero, the infinities and NaN, the
//! divisor included; the pair and rect functions keep a divisor at least 0.05 from 0.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_screencoord::{
    scale_rect, scale_x, scale_xy, scale_y, set_scale, unscale_rect, unscale_x, unscale_xy,
    unscale_y,
};

const S40: u64 = 0x832a40;
const G44: u64 = 0x832a44;
const G48: u64 = 0x832a48;
const S4C: u64 = 0x832a4c;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

/// `SetScreenAspect` (`0x41ad10`, `ret 4`), read back from its four globals.
#[test]
fn sc_41ad10_set_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let run = |emu: &mut Emu, s: f32| -> (f32, f32, f32, f32) {
        emu.call(0x41ad10, 0, 0, &[s.to_bits()]);
        (
            emu.read_f32(S40),
            emu.read_f32(G44),
            emu.read_f32(G48),
            emu.read_f32(S4C),
        )
    };
    let mut total = 0u32;
    let check = |emu: &mut Emu, s: f32| {
        let (s40, g44, g48, s4c) = run(emu, s);
        let w = set_scale(s);
        // Equal bits, or both NaN (the x87's default NaN is negative, the host's positive).
        let same = |a: f32, b: f32| a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan());
        assert!(same(s40, w.s), "s40 s={s}");
        assert!(same(g44, w.g44), "g44 s={s}");
        assert!(same(g48, w.g48), "g48 s={s}");
        assert!(same(s4c, w.s075), "s4c s={s}");
    };
    for s in [
        4.0f32 / 3.0, // default aspect
        1.0,
        1.6,
        16.0 / 9.0,
        0.75,
        2.0,
        0.5,
        3.0,
        100.0,
        0.01,
        1.25,
        1.6,
        2560.0 / 1080.0,
        0.0,
        -0.0,
        -4.0 / 3.0,
        f32::INFINITY,
        f32::NAN,
    ] {
        check(&mut emu, s);
        total += 1;
    }
    let mut st = 0x41ad_1000u32;
    for _ in 0..4000 {
        check(&mut emu, rng(&mut st, 0.1, 10.0));
        total += 1;
    }
    eprintln!(
        "sc_41ad10_set_scale: {total} bit-exact (normalize(s,1) + s·0.75; fst-no-pop f64 invlen)"
    );
}

/// Equal bits, or both NaN: the x87 makes the negative default NaN where the host makes the
/// positive one, and an input NaN's payload passes through both.
fn same64(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

/// One of the single-value functions, which return in `st0`: read back at 53 bits through an
/// `fstp qword`, as a caller that keeps the result unrounded sees it.
fn leaf(name: &str, addr: u64, gva: u64, refn: fn(f32, f32) -> f64) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let run = |emu: &mut Emu, g: f32, v: f32| -> f64 {
        emu.write_u32(gva, g.to_bits());
        emu.call_f64(addr, 0, 0, &[v.to_bits()])
    };
    let mut total = 0u32;
    let probes = [
        1.0f32,
        0.6,
        0.8,
        2.0,
        0.5,
        -3.0,
        1024.0,
        0.1,
        1.0e6,
        -1.0e6,
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN_POSITIVE,
        f32::MAX,
    ];
    // The scales the client stores at 4:3 and 16:9, the file's defaults, and the zero, infinite and
    // NaN scales nothing guards against.
    let scales = [
        0.6f32,
        0.8,
        f32::from_bits(0x3f19_9999),
        f32::from_bits(0x3f5f_1f93),
        f32::from_bits(0x3efb_0385),
        1.0,
        1.3333334,
        -0.5,
        2.5,
        0.0,
        f32::INFINITY,
        f32::NAN,
    ];
    for &g in &scales {
        for &v in &probes {
            let (got, want) = (run(&mut emu, g, v), refn(g, v));
            assert!(same64(got, want), "{name} g={g} v={v}: {got:e} != {want:e}");
            total += 1;
        }
    }
    let mut st = (addr as u32).wrapping_mul(0x1000);
    for _ in 0..4000 {
        let g = rng(&mut st, -5.0, 5.0);
        let v = rng(&mut st, -50000.0, 50000.0);
        let (got, want) = (run(&mut emu, g, v), refn(g, v));
        assert!(same64(got, want), "{name} fuzz g={g} v={v}");
        total += 1;
    }
    eprintln!("{name}: {total} bit-exact at 53 bits");
}

#[test]
fn sc_41ae60_scale_x() {
    leaf("sc_41ae60_scale_x", 0x41ae60, G44, scale_x);
}
#[test]
fn sc_41ae70_scale_y() {
    leaf("sc_41ae70_scale_y", 0x41ae70, G48, scale_y);
}
#[test]
fn sc_41ae40_unscale_x() {
    leaf("sc_41ae40_unscale_x", 0x41ae40, G44, unscale_x);
}
#[test]
fn sc_41ae50_unscale_y() {
    leaf("sc_41ae50_unscale_y", 0x41ae50, G48, unscale_y);
}

// The pair and rect functions, which store through pointers.
#[test]
fn sc_41ad80_scale_xy() {
    pair_test("sc_41ad80_scale_xy", 0x41ad80, |g44, g48, x, y| {
        scale_xy(g44, g48, x, y)
    });
}
#[test]
fn sc_41ade0_unscale_xy() {
    pair_test("sc_41ade0_unscale_xy", 0x41ade0, |g44, g48, x, y| {
        unscale_xy(g44, g48, x, y)
    });
}

fn pair_test(name: &str, addr: u64, refn: fn(f32, f32, f32, f32) -> (f32, f32)) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let xo = emu.alloc(0x10);
    let yo = emu.alloc(0x10);
    let run = |emu: &mut Emu, g44: f32, g48: f32, x: f32, y: f32| -> (f32, f32) {
        emu.write_u32(G44, g44.to_bits());
        emu.write_u32(G48, g48.to_bits());
        emu.call(addr, xo as u32, yo as u32, &[x.to_bits(), y.to_bits()]);
        (emu.read_f32(xo), emu.read_f32(yo))
    };
    let mut total = 0u32;
    let mut st = (addr as u32).wrapping_mul(0x100);
    for _ in 0..5000 {
        let g44 = {
            let x = rng(&mut st, -3.0, 3.0);
            if x.abs() < 0.05 {
                0.05f32.copysign(x)
            } else {
                x
            }
        };
        let g48 = {
            let x = rng(&mut st, -3.0, 3.0);
            if x.abs() < 0.05 {
                0.05f32.copysign(x)
            } else {
                x
            }
        };
        let x = rng(&mut st, -50000.0, 50000.0);
        let y = rng(&mut st, -50000.0, 50000.0);
        let (ox, oy) = run(&mut emu, g44, g48, x, y);
        let (wx, wy) = refn(g44, g48, x, y);
        assert_eq!(ox.to_bits(), wx.to_bits(), "{name} x g44={g44} x={x}");
        assert_eq!(oy.to_bits(), wy.to_bits(), "{name} y g48={g48} y={y}");
        total += 2;
    }
    eprintln!("{name}: {total} bit-exact");
}

#[test]
fn sc_41adb0_scale_rect() {
    rect_test("sc_41adb0_scale_rect", 0x41adb0, scale_rect);
}
#[test]
fn sc_41ae10_unscale_rect() {
    rect_test("sc_41ae10_unscale_rect", 0x41ae10, unscale_rect);
}

fn rect_test(name: &str, addr: u64, refn: fn(f32, f32, [f32; 4]) -> [f32; 4]) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let inp = emu.alloc(0x10);
    let out = emu.alloc(0x10);
    let run = |emu: &mut Emu, g44: f32, g48: f32, r: [f32; 4]| -> [f32; 4] {
        emu.write_u32(G44, g44.to_bits());
        emu.write_u32(G48, g48.to_bits());
        for (i, v) in r.iter().enumerate() {
            emu.write_u32(inp + 4 * i as u64, v.to_bits());
        }
        emu.call(addr, inp as u32, out as u32, &[]);
        [
            emu.read_f32(out),
            emu.read_f32(out + 4),
            emu.read_f32(out + 8),
            emu.read_f32(out + 0xc),
        ]
    };
    let mut total = 0u32;
    let mut st = (addr as u32).wrapping_mul(0x100).wrapping_add(7);
    for _ in 0..5000 {
        let g44 = {
            let x = rng(&mut st, -3.0, 3.0);
            if x.abs() < 0.05 {
                0.05f32.copysign(x)
            } else {
                x
            }
        };
        let g48 = {
            let x = rng(&mut st, -3.0, 3.0);
            if x.abs() < 0.05 {
                0.05f32.copysign(x)
            } else {
                x
            }
        };
        let r = [
            rng(&mut st, -20000.0, 20000.0),
            rng(&mut st, -20000.0, 20000.0),
            rng(&mut st, -20000.0, 20000.0),
            rng(&mut st, -20000.0, 20000.0),
        ];
        let got = run(&mut emu, g44, g48, r);
        let want = refn(g44, g48, r);
        for k in 0..4 {
            assert_eq!(
                got[k].to_bits(),
                want[k].to_bits(),
                "{name}[{k}] g44={g44} g48={g48}"
            );
        }
        total += 4;
    }
    eprintln!("{name}: {total} bit-exact");
}
