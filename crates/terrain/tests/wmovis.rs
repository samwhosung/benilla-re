//! The map object visibility arithmetic against `0x6856c0`, `0x6b41c0` and `0x699210`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::spline_anchor::dot3;
use benilla_re_terrain::wmovis::{
    portal_ndc_to_screen, portal_plane_dot, portal_rect_extent, wmo_corner_ray_negate,
    wmo_dir_len2, wmo_dir_normalize, wmo_dir_normalizes, wmo_dir_subtract, wmo_group_double,
    wmo_plane_offset_negate, WMO_NORMALIZE_EPS2,
};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes).
const SLICE_ESP: u64 = 0x7008_0000;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// AH after `fnstsw ax`.
fn read_ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// Asserts the copied bytes at `island` are the exe's at `src`, so the copy runs the client's own
/// instructions.
fn island_bytes_match(e: &Emu, island: u64, src: u64, len: usize) {
    let got = e.read(island, len);
    let want = e.read(src, len);
    assert_eq!(
        got, want,
        "island bytes at {island:#x} != WoW.exe[{src:#x}..] (len {len})"
    );
}

const CORNER_X: u64 = 0xc7cf20;

const CORNER_Y: u64 = 0xc7cf24;

const CORNER_Z: u64 = 0xc7cf28;

const TGT_X: u64 = 0xca8018;

const TGT_Y: u64 = 0xca801c;

const TGT_Z: u64 = 0xca8020;

const BASE_X: u64 = 0xca7edc;

const BASE_Y: u64 = 0xca7ee0;

const BASE_Z: u64 = 0xca7ee4;

fn dir_setup(e: &mut Emu, ebp: u64, target: [f32; 3], base: [f32; 3]) {
    e.write_u32(TGT_X, target[0].to_bits());
    e.write_u32(TGT_Y, target[1].to_bits());
    e.write_u32(TGT_Z, target[2].to_bits());
    e.write_u32(BASE_X, base[0].to_bits());
    e.write_u32(BASE_Y, base[1].to_bits());
    e.write_u32(BASE_Z, base[2].to_bits());
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.set_reg(RegisterX86::EBP, ebp);
}

/// A random target and base: a quarter close together, so the squared length is near or below the
/// least it normalizes, the rest far apart.
fn dir_fuzz_case(s: &mut u32) -> ([f32; 3], [f32; 3]) {
    if (next(s) >> 8).is_multiple_of(4) {
        let b = [
            frand(s, -100.0, 100.0),
            frand(s, -100.0, 100.0),
            frand(s, -100.0, 100.0),
        ];
        let t = [
            b[0] + frand(s, -0.005, 0.005),
            b[1] + frand(s, -0.005, 0.005),
            b[2] + frand(s, -0.005, 0.005),
        ];
        (t, b)
    } else {
        let b = [
            frand(s, -500.0, 500.0),
            frand(s, -500.0, 500.0),
            frand(s, -500.0, 500.0),
        ];
        let t = [
            frand(s, -500.0, 500.0),
            frand(s, -500.0, 500.0),
            frand(s, -500.0, 500.0),
        ];
        (t, b)
    }
}

const PT_X: u64 = 0xca7edc;

const PT_Y: u64 = 0xca7ee0;

const PT_Z: u64 = 0xca7ee4;

/// `CullExteriorMapObjects` (`0x6856c0`) from `0x68585b` to the call at `0x6858a6`: the camera negated
/// to `[esp]` up.
#[test]
fn wmo_corner_ray_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, g: [f32; 3], ctx: &str| {
        e.write_u32(CORNER_X, g[0].to_bits());
        e.write_u32(CORNER_Y, g[1].to_bits());
        e.write_u32(CORNER_Z, g[2].to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x68585b, 0x6858a9);
        // after `sub esp,0xc`
        let base = SLICE_ESP - 0xc;
        let bin = [e.read_f32(base), e.read_f32(base + 4), e.read_f32(base + 8)];
        for k in 0..3 {
            assert_f32(
                bin[k],
                wmo_corner_ray_negate(g[k]),
                &format!("{ctx} neg[{k}]"),
            );
        }
    };

    // zero and ±1
    run_case(&mut e, [0.0, 0.0, 0.0], "corner zero");
    run_case(&mut e, [1.0, -1.0, 1.0], "corner ±1");
    run_case(
        &mut e,
        [f32::from_bits(0x0000_0001), -1024.0, 12345.678],
        "corner subnormal/large",
    );
    let mut s = 0x6858_5b5bu32;
    for i in 0..2000u32 {
        let g = [
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
        ];
        run_case(&mut e, g, &format!("corner fuzz[{i}]"));
    }
}

/// `0x6856c0` from `0x685974` to the `jne` at `0x6859cf`: the stored direction and the decision.
#[test]
fn wmo_dir_len2_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, target: [f32; 3], base: [f32; 3], ctx: &str| {
        dir_setup(e, ebp, target, base);
        e.run(0x685974, 0x6859cf);
        let d = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let ah = read_ah(e);
        let our_d = wmo_dir_subtract(target, base);
        for k in 0..3 {
            assert_f32(d[k], our_d[k], &format!("{ctx} d[{k}]"));
        }
        let our_len2 = wmo_dir_len2(our_d);
        // `jne` skips the scale when C0 or C3 is set
        assert_eq!(
            ah & 0x41 == 0,
            wmo_dir_normalizes(our_len2),
            "{ctx}: AH {:#04x}, len² {our_len2}",
            ah & 0x41
        );
    };

    let eps = f64::from(WMO_NORMALIZE_EPS2);
    let dx_eps = eps.sqrt() as f32;
    run_case(&mut e, [dx_eps, 0.0, 0.0], [0.0, 0.0, 0.0], "gate ~=eps");
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "gate zero (len²=0<eps skip)",
    );
    run_case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0, 0.0, 0.0],
        "gate 3-4-0 (len²=25)",
    );
    run_case(
        &mut e,
        [0.005, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "gate 0.005 (len²=2.5e-5<eps)",
    );
    run_case(
        &mut e,
        [0.011, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "gate 0.011 (len²=1.21e-4>eps)",
    );

    let mut s = 0x6859_7474u32;
    for i in 0..3000u32 {
        let (target, base) = dir_fuzz_case(&mut s);
        run_case(&mut e, target, base, &format!("gate fuzz[{i}]"));
    }
}

/// `0x6856c0` from `0x685974` to the join at `0x6859f1`: the direction, scaled or not.
#[test]
fn wmo_dir_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, target: [f32; 3], base: [f32; 3], ctx: &str| {
        dir_setup(e, ebp, target, base);
        e.run(0x685974, 0x6859f1);
        let bin = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let d = wmo_dir_subtract(target, base);
        let len2 = wmo_dir_len2(d);
        let ours = wmo_dir_normalize(d, len2);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} norm[{k}]"));
        }
    };

    let eps = f64::from(WMO_NORMALIZE_EPS2);
    let dx_eps = eps.sqrt() as f32;
    run_case(&mut e, [dx_eps, 0.0, 0.0], [0.0, 0.0, 0.0], "norm ~=eps");
    run_case(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "norm zero (skip)");
    run_case(&mut e, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0], "norm 3-4-0");
    run_case(&mut e, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], "norm unit-x");
    run_case(
        &mut e,
        [0.005, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "norm 0.005 (skip)",
    );
    run_case(
        &mut e,
        [0.011, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "norm 0.011 (scale)",
    );

    let mut s = 0x6859_f1f1u32;
    for i in 0..3000u32 {
        let (target, base) = dir_fuzz_case(&mut s);
        run_case(&mut e, target, base, &format!("norm fuzz[{i}]"));
    }
}

/// `0x6856c0` from `0x685a08` to `0x685a24`, `DotVector3` (`0x602630`) run: `[0xca7efc]`.
#[test]
fn wmo_plane_offset_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;
    const OUT: u64 = 0xca7efc;

    let run_case = |e: &mut Emu, dir: [f32; 3], base: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0xc, dir[0].to_bits());
        e.write_u32(ebp - 0x8, dir[1].to_bits());
        e.write_u32(ebp - 0x4, dir[2].to_bits());
        e.write_u32(BASE_X, base[0].to_bits());
        e.write_u32(BASE_Y, base[1].to_bits());
        e.write_u32(BASE_Z, base[2].to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x685a08, 0x685a24);
        let bin = e.read_f32(OUT);
        let ours = wmo_plane_offset_negate(dot3(dir, base));
        assert_f32(bin, ours, ctx);
    };
    run_case(&mut e, [0.0, 0.0, 0.0], [1.0, 2.0, 3.0], "plane-off zero");
    run_case(&mut e, [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], "plane-off dot=1");
    let mut s = 0x685a_0808u32;
    for i in 0..2000u32 {
        let dir = [
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
        ];
        let base = [
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
        ];
        run_case(&mut e, dir, base, &format!("plane-off fuzz[{i}]"));
    }
}

/// `WalkPortals` (`0x6b41c0`) from `0x6b43d2`: the bytes through the `fadd` copied out, with `fchs`
/// appended when flipped and `fstp dword [out]` after.
#[test]
fn portal_plane_dot_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const SRC: u64 = 0x6b43d2;
    const FP_LEN: usize = 0x6b43f4 - 0x6b43d2;

    let plane = e.alloc(0x40);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);

    let run_case = |e: &mut Emu, pt: [f32; 3], pl: [f32; 3], d: f32, flip: bool, ctx: &str| {
        let mut code = e.read(SRC, FP_LEN);
        if flip {
            code.push(0xd9);
            code.push(0xe0);
        }
        code.extend_from_slice(&[0xd9, 0x1d]);
        code.extend_from_slice(&(out as u32).to_le_bytes());
        e.write(isl, &code);
        island_bytes_match(e, isl, SRC, FP_LEN);
        let end = isl + code.len() as u64;

        e.write_u32(PT_X, pt[0].to_bits());
        e.write_u32(PT_Y, pt[1].to_bits());
        e.write_u32(PT_Z, pt[2].to_bits());
        e.write_u32(plane + 0x4, pl[0].to_bits());
        e.write_u32(plane + 0x8, pl[1].to_bits());
        e.write_u32(plane + 0xc, pl[2].to_bits());
        e.write_u32(plane + 0x10, d.to_bits());
        e.set_reg(RegisterX86::EBX, plane);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(isl, end);
        let bin = e.read_f32(out);
        let ours = portal_plane_dot(pt, pl, d, flip);
        assert_f32(bin, ours, ctx);
    };

    // both signs, a distance of 0, exact values
    run_case(
        &mut e,
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        0.0,
        false,
        "dot=1 noflip",
    );
    run_case(
        &mut e,
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        0.0,
        true,
        "dot=1 flip",
    );
    run_case(
        &mut e,
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        -3.0,
        false,
        "dot==0 (3-3=0) noflip",
    );
    run_case(
        &mut e,
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        -3.0,
        true,
        "dot==0 flip",
    );
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        0.0,
        false,
        "all-zero",
    );

    let mut s = 0x6b43_d2d2u32;
    for i in 0..3000u32 {
        let pt = [
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
        ];
        let pl = [
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
        ];
        let d = frand(&mut s, -100.0, 100.0);
        let flip = (next(&mut s) >> 8).is_multiple_of(2);
        run_case(&mut e, pt, pl, d, flip, &format!("dot fuzz[{i}]"));
    }
}

/// `0x6b41c0` at `0x6b44b1` and `0x6b44ca`: the `fld; fsub; fabs` copied out with `fstp dword [out]`.
#[test]
fn portal_rect_extent_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const SRC: u64 = 0x6b44b1;
    const LEN: usize = 0x6b44b9 - 0x6b44b1;
    let mut code = e.read(SRC, LEN);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl, SRC, LEN);
    let end = isl + code.len() as u64;

    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, c1: f32, c0: f32, ctx: &str| {
        e.write_u32(ebp - 0x1c, c1.to_bits());
        e.write_u32(ebp - 0x24, c0.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let bin = e.read_f32(out);
        let ours = portal_rect_extent(c1, c0);
        assert_f32(bin, ours, ctx);
    };

    // equal sides, both orders, exact values
    run_case(&mut e, 5.0, 5.0, "extent c1==c0 (0)");
    run_case(&mut e, 5.0, 3.0, "extent +2");
    run_case(&mut e, 3.0, 5.0, "extent -2 → fabs 2");
    run_case(&mut e, 0.0, -0.0, "extent 0 - -0");
    run_case(&mut e, -1.0, 1.0, "extent -2 → 2");

    let mut s = 0x6b44_b1b1u32;
    for i in 0..3000u32 {
        let c1 = frand(&mut s, -2.0, 2.0);
        let c0 = frand(&mut s, -2.0, 2.0);
        run_case(&mut e, c1, c0, &format!("extent fuzz[{i}]"));
    }
}

/// `0x6b41c0` from `0x6b45ba` to the call at `0x6b4610`: the corners at `[ebp-0x28]` up mapped to
/// `[ebp-0x48]` up.
#[test]
fn portal_ndc_to_screen_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, coords: [f32; 4], ctx: &str| {
        e.write_u32(ebp - 0x28, coords[0].to_bits());
        e.write_u32(ebp - 0x24, coords[1].to_bits());
        e.write_u32(ebp - 0x20, coords[2].to_bits());
        e.write_u32(ebp - 0x1c, coords[3].to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b45ba, 0x6b4610);
        let bin = [
            e.read_f32(ebp - 0x48),
            e.read_f32(ebp - 0x44),
            e.read_f32(ebp - 0x40),
            e.read_f32(ebp - 0x3c),
        ];
        let ours = portal_ndc_to_screen(coords);
        for k in 0..4 {
            assert_f32(bin[k], ours[k], &format!("{ctx} c{k}'"));
        }
    };

    // ±1, 0, and the same value in all four corners
    run_case(&mut e, [-1.0, -1.0, -1.0, -1.0], "ndc all -1 (→0)");
    run_case(&mut e, [1.0, 1.0, 1.0, 1.0], "ndc all 1 (→1)");
    run_case(&mut e, [0.0, 0.0, 0.0, 0.0], "ndc all 0 (→0.5)");
    let split = f32::from_bits(0x4000_0001);
    run_case(
        &mut e,
        [split, split, split, split],
        "ndc equal (live==reload, both paths)",
    );
    run_case(
        &mut e,
        [1.0e7, -1.0e7, 1.0e30, -1.0e30],
        "ndc large (reload-band)",
    );
    run_case(
        &mut e,
        [f32::from_bits(0x3f80_0001), -0.5, 0.999_999, 12345.6],
        "ndc mixed",
    );

    let mut s = 0x6b45_baba_u32;
    for i in 0..3000u32 {
        let wide = (next(&mut s) >> 8).is_multiple_of(4);
        let lim = if wide { 1.0e7 } else { 2.0 };
        let coords = [
            frand(&mut s, -lim, lim),
            frand(&mut s, -lim, lim),
            frand(&mut s, -lim, lim),
            frand(&mut s, -lim, lim),
        ];
        run_case(&mut e, coords, &format!("ndc fuzz[{i}]"));
    }
}

/// `InitTerrainTwoPi8` (`0x699210`) called whole: `[0xca0538]`.
#[test]
fn wmo_group_double_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x810618;
    const OUT: u64 = 0xca0538;

    let run_case = |e: &mut Emu, x: f32, ctx: &str| {
        e.write_u32(SRC, x.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(0x699210, 0x69921e);
        let bin = e.read_f32(OUT);
        let ours = wmo_group_double(x);
        assert_f32(bin, ours, ctx);
    };

    // zeros, exact values, an odd mantissa, large values
    run_case(&mut e, 0.0, "double zero");
    run_case(&mut e, -0.0, "double -zero");
    run_case(&mut e, 1.0, "double 1.0");
    run_case(&mut e, f32::from_bits(0x0000_0001), "double tiny-subnormal");
    run_case(&mut e, 1e38, "double 1e38 (→ overflow inf)");
    run_case(&mut e, -1234.5678, "double negative");

    let mut s = 0x6992_1010u32;
    for i in 0..2000u32 {
        let x = frand(&mut s, -1e6, 1e6);
        run_case(&mut e, x, &format!("double fuzz[{i}]"));
    }
}
