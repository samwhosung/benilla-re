//! The pass negations against `0x684840`, `0x684cd0`, `0x6841a0`, `0x6aaab0` and `0x6c0cf0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::passneg::{coord_negate, far_fsub1, far_fsub2, FAR_FSUB2_MINUEND};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes).
const SLICE_ESP: u64 = 0x7008_0000;

/// The camera the passes negate.
const CORNER_X: u64 = 0xc7cf20;

const CORNER_Y: u64 = 0xc7cf24;

const CORNER_Z: u64 = 0xc7cf28;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Signed zeros, ±1, NaN of each sign, the infinities, subnormals and ±33.
const NEG_PROBES: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    f32::from_bits(0x7fc0_0000),
    f32::from_bits(0xffc0_0000),
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::from_bits(0x0000_0001),
    f32::from_bits(0x8000_0001),
    33.0,
    -33.0,
];

/// `DrawMapObjectPass` (`0x684840`) from `0x6849ff` to `0x684a4e`: the camera negated to `[esp]` up,
/// after `sub esp,0xc`.
#[test]
fn wmo_pass_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x400);

    let run_case = |e: &mut Emu, g: [f32; 3], ctx: &str| {
        e.write_u32(CORNER_X, g[0].to_bits());
        e.write_u32(CORNER_Y, g[1].to_bits());
        e.write_u32(CORNER_Z, g[2].to_bits());
        e.set_reg(RegisterX86::ECX, this);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x300);
        e.run(0x6849ff, 0x684a4e);
        let base = SLICE_ESP - 0xc;
        let bin = [
            e.read_f32(base),
            e.read_f32(base + 4), // [esp+4] = -y
            e.read_f32(base + 8), // [esp+8] = -z
        ];
        for k in 0..3 {
            assert_f32(bin[k], coord_negate(g[k]), &format!("{ctx} neg[{k}]"));
        }
    };

    for &p in NEG_PROBES {
        run_case(&mut e, [p, p, p], &format!("wmo probe {p:?}"));
    }
    run_case(&mut e, [1.0, -1.0, 0.0], "wmo mix ±1/0");
    let mut s = 0x6849_ffffu32;
    for i in 0..2000u32 {
        let g = [
            frand(&mut s, -10000.0, 10000.0),
            frand(&mut s, -10000.0, 10000.0),
            frand(&mut s, -10000.0, 10000.0),
        ];
        run_case(&mut e, g, &format!("wmo fuzz[{i}]"));
    }
}

/// `DrawGroupLiquidPass` (`0x684cd0`) from `0x684e5a` to `0x684e93`: the camera negated to
/// `[ebp-0x50]` up; eax a buffer for the `mov esi,[eax+8]` inside.
#[test]
fn wmo_liquid_pass_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let buf = e.alloc(0x40);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, g: [f32; 3], ctx: &str| {
        e.write_u32(CORNER_X, g[0].to_bits());
        e.write_u32(CORNER_Y, g[1].to_bits());
        e.write_u32(CORNER_Z, g[2].to_bits());
        e.set_reg(RegisterX86::EAX, buf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x684e5a, 0x684e93);
        let bin = [
            e.read_f32(ebp - 0x50),
            e.read_f32(ebp - 0x4c),
            e.read_f32(ebp - 0x48),
        ];
        for k in 0..3 {
            assert_f32(bin[k], coord_negate(g[k]), &format!("{ctx} neg[{k}]"));
        }
    };

    for &p in NEG_PROBES {
        run_case(&mut e, [p, p, p], &format!("wmo-liquid probe {p:?}"));
    }
    run_case(&mut e, [1.0, -1.0, 0.0], "wmo-liquid mix ±1/0");
    let mut s = 0x684e_5a5au32;
    for i in 0..2000u32 {
        let g = [
            frand(&mut s, -10000.0, 10000.0),
            frand(&mut s, -10000.0, 10000.0),
            frand(&mut s, -10000.0, 10000.0),
        ];
        run_case(&mut e, g, &format!("wmo-liquid fuzz[{i}]"));
    }
}

/// `DrawFarBand` (`0x6841a0`) from `0x684253` to `0x684287`: `[ebp-0x1c] − 33.0` to `[esp]`.
#[test]
fn far_fsub1_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // the slice writes down to `[ebp-0xa4]`
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, depth: f32, ctx: &str| {
        e.write_u32(ebp - 0x1c, depth.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x684253, 0x684287);
        let bin = e.read_f32(SLICE_ESP);
        assert_f32(bin, far_fsub1(depth), ctx);
    };

    // 33.0 and its neighbours, zeros, NaN, the infinities, a subnormal
    for &p in &[
        0.0f32,
        -0.0,
        33.0,
        f32::from_bits(33.0f32.to_bits() - 1),
        f32::from_bits(33.0f32.to_bits() + 1),
        66.0,
        -33.0,
        f32::from_bits(0x7fc0_0000),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x0000_0001),
    ] {
        run_case(&mut e, p, &format!("far-fsub1 probe {p:?}"));
    }
    let mut s = 0x6842_6060u32;
    for i in 0..3000u32 {
        let depth = frand(&mut s, -1000.0, 5000.0);
        run_case(&mut e, depth, &format!("far-fsub1 fuzz[{i}]"));
    }
}

/// `0x6841a0` from `0x68434d` to `0x684367`: `[0x80febc] − 0.005` to `[esp]` after two pushes.
#[test]
fn far_fsub2_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let bin_minuend = e.read_f32(0x80febc);
    assert_f32(
        bin_minuend,
        FAR_FSUB2_MINUEND,
        "far-fsub2 minuend [0x80febc] vs documented constant",
    );

    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ECX, 0);
    e.run(0x68434d, 0x684367);
    let bin = e.read_f32(SLICE_ESP - 8);
    assert_f32(bin, far_fsub2(bin_minuend), "far-fsub2 (0.96 − 0.005)");
}

/// `0x6841a0` from `0x684373` to `0x6843d1`: the camera negated to `[esp]` up, after `sub esp,0xc`.
#[test]
fn far_pass_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    let run_case = |e: &mut Emu, g: [f32; 3], ctx: &str| {
        e.write_u32(CORNER_X, g[0].to_bits());
        e.write_u32(CORNER_Y, g[1].to_bits());
        e.write_u32(CORNER_Z, g[2].to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x200);
        e.run(0x684373, 0x6843d1);
        let base = SLICE_ESP - 0xc;
        let bin = [
            e.read_f32(base),
            e.read_f32(base + 4), // [esp+4] = -y
            e.read_f32(base + 8), // [esp+8] = -z
        ];
        for k in 0..3 {
            assert_f32(bin[k], coord_negate(g[k]), &format!("{ctx} neg[{k}]"));
        }
    };

    for &p in NEG_PROBES {
        run_case(&mut e, [p, p, p], &format!("far-pass probe {p:?}"));
    }
    run_case(&mut e, [1.0, -1.0, 0.0], "far-pass mix ±1/0");
    let mut s = 0x6843_7373u32;
    for i in 0..2000u32 {
        let g = [
            frand(&mut s, -10000.0, 10000.0),
            frand(&mut s, -10000.0, 10000.0),
            frand(&mut s, -10000.0, 10000.0),
        ];
        run_case(&mut e, g, &format!("far-pass fuzz[{i}]"));
    }
}

/// `GatherMapObjectFacets` (`0x6aaab0`) from `0x6aaaf5` to `0x6aab1c`: `[ebp-0x24]`, `[ebp-0x28]` and
/// `[ebp-0x2c]` negated to `[esp+8]` down, after `sub esp,0xc`; esi a buffer for the `rep movsd`
/// inside.
#[test]
fn collectinbox_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let movsd_src = e.alloc(0x40);

    let run_case = |e: &mut Emu, g: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x24, g[0].to_bits());
        e.write_u32(ebp - 0x28, g[1].to_bits());
        e.write_u32(ebp - 0x2c, g[2].to_bits());
        e.set_reg(RegisterX86::ESI, movsd_src);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6aaaf5, 0x6aab1c);
        let base = SLICE_ESP - 0xc;
        let bin = [e.read_f32(base + 8), e.read_f32(base + 4), e.read_f32(base)];
        for k in 0..3 {
            assert_f32(bin[k], coord_negate(g[k]), &format!("{ctx} neg[{k}]"));
        }
    };

    for &p in NEG_PROBES {
        run_case(&mut e, [p, p, p], &format!("collectinbox probe {p:?}"));
    }
    run_case(&mut e, [1.0, -1.0, 0.0], "collectinbox mix ±1/0");
    let mut s = 0x6aaa_f5f5u32;
    for i in 0..2000u32 {
        let g = [
            frand(&mut s, -20000.0, 20000.0),
            frand(&mut s, -20000.0, 20000.0),
            frand(&mut s, -20000.0, 20000.0),
        ];
        run_case(&mut e, g, &format!("collectinbox fuzz[{i}]"));
    }
}

/// `LinkLightToGroups` (`0x6c0cf0`) from `0x6c0d35` to `0x6c0d5c`: `[ebp-0x20]`, `[ebp-0x24]` and
/// `[ebp-0x28]` negated to `[esp+8]` down, as in `0x6aaab0`.
#[test]
fn lightbox_attach_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let movsd_src = e.alloc(0x40);

    let run_case = |e: &mut Emu, g: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x20, g[0].to_bits());
        e.write_u32(ebp - 0x24, g[1].to_bits());
        e.write_u32(ebp - 0x28, g[2].to_bits());
        e.set_reg(RegisterX86::ESI, movsd_src);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6c0d35, 0x6c0d5c);
        let base = SLICE_ESP - 0xc;
        let bin = [e.read_f32(base + 8), e.read_f32(base + 4), e.read_f32(base)];
        for k in 0..3 {
            assert_f32(bin[k], coord_negate(g[k]), &format!("{ctx} neg[{k}]"));
        }
    };

    for &p in NEG_PROBES {
        run_case(&mut e, [p, p, p], &format!("lightbox probe {p:?}"));
    }
    run_case(&mut e, [1.0, -1.0, 0.0], "lightbox mix ±1/0");
    let mut s = 0x6c0d_3535u32;
    for i in 0..2000u32 {
        let g = [
            frand(&mut s, -20000.0, 20000.0),
            frand(&mut s, -20000.0, 20000.0),
            frand(&mut s, -20000.0, 20000.0),
        ];
        run_case(&mut e, g, &format!("lightbox fuzz[{i}]"));
    }
}
