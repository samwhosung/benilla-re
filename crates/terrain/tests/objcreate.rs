//! The light objects and the plane pieces against `0x671010`, `0x671380`, `0x685a90` and `0x686640`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::spline_anchor::dot3;
use benilla_re_terrain::objcreate::{
    color_clamp_scale, corner_ray_negate, dir_len2, dir_normalize, dir_normalizes, dir_subtract,
    plane5_from_plane4, plane_d_negate, plane_offset_negate, pos_magnitude, NORMALIZE_EPS2,
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

/// The camera, which `0x685a90` negates.
const CORNER_X: u64 = 0xc7cf20;

const CORNER_Y: u64 = 0xc7cf24;

const CORNER_Z: u64 = 0xc7cf28;

/// The target's x; the slice copies y and z there from `[eax+4]`.
const TGT_X: u64 = 0xca8018;

/// The base the direction starts from.
const BASE_X: u64 = 0xca7edc;

const BASE_Y: u64 = 0xca7ee0;

const BASE_Z: u64 = 0xca7ee4;

/// Sets up `0x685a90` from `0x685c5b`: the target at eax and `0xca8018`, the base at `0xca7edc`.
/// Each case is one run from `0x685c5b`, to the `jne` or to its join.
fn dir_setup(e: &mut Emu, tgt: u64, ebp: u64, target: [f32; 3], base: [f32; 3]) {
    e.write_u32(tgt, target[0].to_bits());
    e.write_u32(tgt + 4, target[1].to_bits());
    e.write_u32(tgt + 8, target[2].to_bits());
    e.write_u32(TGT_X, target[0].to_bits());
    e.write_u32(BASE_X, base[0].to_bits());
    e.write_u32(BASE_Y, base[1].to_bits());
    e.write_u32(BASE_Z, base[2].to_bits());
    e.set_reg(RegisterX86::EAX, tgt);
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.set_reg(RegisterX86::EBP, ebp);
}

/// A target and a base: a quarter of them close, about the 0.0001 gate, the rest far apart.
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

/// `CWorld::CreateLightObjectB` (`0x671010`) from `0x671072` to `0x6710b7`: the bytes copied out of
/// the client with `fstp dword [out]` appended (the short jumps land on it), `[ebp-0xc]` the
/// component. The other components and `0x671380` repeat these bytes.
#[test]
fn color_clamp_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const LADDER_VA: u64 = 0x671072;
    const LADDER_LEN: usize = 0x6710b7 - 0x671072;
    let mut ladder = e.read(LADDER_VA, LADDER_LEN);

    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut code = std::mem::take(&mut ladder);
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + code.len() as u64;
    e.write(isl, &code);

    let run_case = |e: &mut Emu, v: f32, ctx: &str| {
        e.write_u32(ebp - 0xc, v.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let bin = e.read_f32(out);
        // the appended store rounds the register to `f32`
        let ours = color_clamp_scale(v) as f32;
        assert_f32(bin, ours, ctx);
    };

    // the bounds themselves
    run_case(&mut e, 0.0, "clamp boundary v==0.0");
    run_case(&mut e, -0.0, "clamp boundary v==-0.0");
    run_case(&mut e, 1.0, "clamp boundary v==1.0");
    run_case(
        &mut e,
        f32::from_bits(1.0f32.to_bits() - 1),
        "clamp just-below 1.0",
    );
    run_case(&mut e, f32::from_bits(0x0000_0001), "clamp tiny positive");
    run_case(&mut e, -1e-30, "clamp tiny negative");

    let mut s = 0x6710_1010u32;
    for i in 0..4000u32 {
        let v = match (next(&mut s) >> 8) % 3 {
            0 => frand(&mut s, -2.0, 0.0),
            1 => frand(&mut s, 0.0, 1.0),
            _ => frand(&mut s, 1.0, 3.0),
        };
        run_case(&mut e, v, &format!("clamp fuzz[{i}] v={v}"));
    }
}

/// `0x671010` from `0x6712fc` and `0x671380` from `0x67165c`, through the store to `[esi+0x30]`,
/// eax the position.
#[test]
fn pos_magnitude_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pos = e.alloc(0x20);
    let this = e.alloc(0x100);

    let run_b = |e: &mut Emu, p: [f32; 3], ctx: &str| {
        for (k, c) in p.iter().enumerate() {
            e.write_u32(pos + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::EAX, pos);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(0x6712fc, 0x671322);
        let bin = e.read_f32(this + 0x30);
        let ours = pos_magnitude(p);
        assert_f32(bin, ours, ctx);
    };
    // `0x671380` reloads eax from `[ebp+0x10]` after the loads
    let run_a = |e: &mut Emu, p: [f32; 3], ctx: &str| {
        for (k, c) in p.iter().enumerate() {
            e.write_u32(pos + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::EAX, pos);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.write_u32(SLICE_ESP + 0x80 + 0x10, pos as u32);
        e.run(0x67165c, 0x671684);
        let bin = e.read_f32(this + 0x30);
        let ours = pos_magnitude(p);
        assert_f32(bin, ours, ctx);
    };

    // zero, an axis, and exact squares
    run_b(&mut e, [0.0, 0.0, 0.0], "mag B zero");
    run_b(&mut e, [3.0, 4.0, 0.0], "mag B 3-4-5");
    run_b(&mut e, [1.0, 0.0, 0.0], "mag B unit-x");
    run_a(&mut e, [0.0, 0.0, 0.0], "mag A zero");
    run_a(&mut e, [3.0, 4.0, 0.0], "mag A 3-4-5");
    run_a(&mut e, [1.0, 2.0, 2.0], "mag A 1-2-2");

    let mut s = 0x6713_0707u32;
    for i in 0..3000u32 {
        let p = [
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
        ];
        run_b(&mut e, p, &format!("mag B fuzz[{i}]"));
        run_a(&mut e, p, &format!("mag A fuzz[{i}]"));
    }
}

/// `CullInteriorMapObject` (`0x685a90`) from `0x685a99` to `0x685ae3`: the camera negated to `[esp]`
/// up, after two pushes and `sub esp,0xc`.
#[test]
fn corner_ray_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let run_case = |e: &mut Emu, g: [f32; 3], ctx: &str| {
        e.write_u32(CORNER_X, g[0].to_bits());
        e.write_u32(CORNER_Y, g[1].to_bits());
        e.write_u32(CORNER_Z, g[2].to_bits());
        e.set_reg(RegisterX86::ECX, this);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(0x685a99, 0x685ae3);
        let base = SLICE_ESP - 0x14;
        let bin = [e.read_f32(base), e.read_f32(base + 4), e.read_f32(base + 8)];
        for k in 0..3 {
            assert_f32(bin[k], corner_ray_negate(g[k]), &format!("{ctx} neg[{k}]"));
        }
    };
    run_case(&mut e, [0.0, 0.0, 0.0], "corner zero");
    run_case(&mut e, [1.0, -1.0, 1.0], "corner ±1");
    let mut s = 0x685a_9999u32;
    for i in 0..2000u32 {
        let g = [
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
        ];
        run_case(&mut e, g, &format!("corner fuzz[{i}]"));
    }
}

/// `0x685a90` from `0x685c5b` to the `jne` at `0x685cb8`: the stored direction and the decision.
#[test]
fn dir_len2_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let tgt = e.alloc(0x20);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, target: [f32; 3], base: [f32; 3], ctx: &str| {
        dir_setup(e, tgt, ebp, target, base);
        e.run(0x685c5b, 0x685cb8);
        let d = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let ah = read_ah(e);
        let our_d = dir_subtract(target, base);
        for k in 0..3 {
            assert_f32(d[k], our_d[k], &format!("{ctx} d[{k}]"));
        }
        let dz_live = f64::from(target[2]) - f64::from(base[2]);
        let our_len2 = dir_len2(our_d, dz_live);
        // `jne` skips the scale when C0 or C3 is set
        assert_eq!(
            ah & 0x41 == 0,
            dir_normalizes(our_len2),
            "{ctx}: AH {:#04x}, len² {our_len2}",
            ah & 0x41
        );
    };

    // about the gate
    let eps = f64::from(NORMALIZE_EPS2);
    let dx_eps = eps.sqrt() as f32;
    run_case(&mut e, [dx_eps, 0.0, 0.0], [0.0, 0.0, 0.0], "gate ~=eps");
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "gate zero (len²=0<eps)",
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

    let mut s = 0x685c_5b5bu32;
    for i in 0..3000u32 {
        let (target, base) = dir_fuzz_case(&mut s);
        run_case(&mut e, target, base, &format!("gate fuzz[{i}]"));
    }
}

/// `0x685a90` from `0x685c5b` to the join at `0x685cda`: the direction, scaled or not.
#[test]
fn dir_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let tgt = e.alloc(0x20);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, target: [f32; 3], base: [f32; 3], ctx: &str| {
        dir_setup(e, tgt, ebp, target, base);
        e.run(0x685c5b, 0x685cda);
        let bin = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let d = dir_subtract(target, base);
        let dz_live = f64::from(target[2]) - f64::from(base[2]);
        let len2 = dir_len2(d, dz_live);
        let ours = dir_normalize(d, len2);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} norm[{k}]"));
        }
    };

    let eps = f64::from(NORMALIZE_EPS2);
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

    let mut s = 0x685c_dada_u32;
    for i in 0..3000u32 {
        let (target, base) = dir_fuzz_case(&mut s);
        run_case(&mut e, target, base, &format!("norm fuzz[{i}]"));
    }
}

/// `0x685a90` from `0x685cf1` through the store at `0x685d07`, `DotVector3` (`0x602630`) run: the
/// direction at `[ebp-0xc]`, the base at `0xca7edc`, the result at `[0xca7efc]`.
#[test]
fn plane_offset_negate_difftest() {
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
        e.run(0x685cf1, 0x685d0d);
        let bin = e.read_f32(OUT);
        let ours = plane_offset_negate(dot3(dir, base));
        assert_f32(bin, ours, ctx);
    };
    run_case(&mut e, [0.0, 0.0, 0.0], [1.0, 2.0, 3.0], "plane-off zero");
    run_case(&mut e, [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], "plane-off dot=1");
    let mut s = 0x685c_fefeu32;
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

/// `BuildConvexPlanes` (`0x686640`): the four side planes' `d`, each from its call to `DotVector3`
/// (`0x602630`, run) through the `fchs` and the store: from `0x6866b1` to `[esi+0x1c]`, `0x686708`
/// to `[esi+0x2c]`, `0x686768` to `[esi+0x3c]` and `0x6867bd` to `[esi+0x4c]`.
#[test]
fn side_plane_d_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);

    // the normal at `esi+0x10`, the corner at edi = `esi+0x60`
    let run_site0 = |e: &mut Emu, n: [f32; 3], a: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(this + 0x10 + 4 * k as u64, n[k].to_bits());
            e.write_u32(this + 0x60 + 4 * k as u64, a[k].to_bits());
        }
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, this + 0x60);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(0x6866b1, 0x6866c1);
        let bin = e.read_f32(this + 0x1c);
        assert_f32(bin, plane_d_negate(dot3(n, a)), ctx);
    };
    // the normal at `esi+0x20`, the corner at edi = `esi+0x60`
    let run_site1 = |e: &mut Emu, n: [f32; 3], a: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(this + 0x20 + 4 * k as u64, n[k].to_bits());
            e.write_u32(this + 0x60 + 4 * k as u64, a[k].to_bits());
        }
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, this + 0x60);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(0x686708, 0x68671f);
        let bin = e.read_f32(this + 0x2c);
        assert_f32(bin, plane_d_negate(dot3(n, a)), ctx);
    };
    // the normal at edi = `esi+0x30`, the corner at `esi+0x84`
    let run_site2 = |e: &mut Emu, n: [f32; 3], a: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(this + 0x30 + 4 * k as u64, n[k].to_bits());
            e.write_u32(this + 0x84 + 4 * k as u64, a[k].to_bits());
        }
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, this + 0x30);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(0x686768, 0x68677a);
        let bin = e.read_f32(this + 0x3c);
        assert_f32(bin, plane_d_negate(dot3(n, a)), ctx);
    };
    // the normal at edi = `esi+0x40`, the corner at ebx = `esi+0x9c`
    let run_site3 = |e: &mut Emu, n: [f32; 3], a: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(this + 0x40 + 4 * k as u64, n[k].to_bits());
            e.write_u32(this + 0x9c + 4 * k as u64, a[k].to_bits());
        }
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, this + 0x40);
        e.set_reg(RegisterX86::EBX, this + 0x9c);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(0x6867bd, 0x6867cb);
        let bin = e.read_f32(this + 0x4c);
        assert_f32(bin, plane_d_negate(dot3(n, a)), ctx);
    };

    let probes: [([f32; 3], [f32; 3]); 3] = [
        ([0.0, 0.0, 0.0], [1.0, 2.0, 3.0]),
        ([1.0, 0.0, 0.0], [5.0, 0.0, 0.0]),
        ([0.577, 0.577, 0.577], [10.0, 10.0, 10.0]),
    ];
    for (n, a) in probes {
        run_site0(&mut e, n, a, "site0 probe");
        run_site1(&mut e, n, a, "site1 probe");
        run_site2(&mut e, n, a, "site2 probe");
        run_site3(&mut e, n, a, "site3 probe");
    }
    let mut s = 0x6866_bbbbu32;
    for i in 0..1500u32 {
        let n = [
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
        ];
        let a = [
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        run_site0(&mut e, n, a, &format!("site0 fuzz[{i}]"));
        run_site1(&mut e, n, a, &format!("site1 fuzz[{i}]"));
        run_site2(&mut e, n, a, &format!("site2 fuzz[{i}]"));
        run_site3(&mut e, n, a, &format!("site3 fuzz[{i}]"));
    }
}

/// `0x686640` from `0x6867cb` to `0x686819`: the fourth normal at edi = `esi+0x40`, the point at
/// `esi+0x78`; the fifth plane at `esi+0x50`.
#[test]
fn plane5_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let edi = this + 0x40;
    let eax = this + 0x50;

    let run_case = |e: &mut Emu, n4: [f32; 3], vp: [f32; 3], ctx: &str| {
        for (k, c) in n4.iter().enumerate() {
            e.write_u32(edi + 4 * k as u64, c.to_bits());
        }
        e.write_u32(this + 0x78, vp[0].to_bits());
        e.write_u32(this + 0x7c, vp[1].to_bits());
        e.write_u32(this + 0x80, vp[2].to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(0x6867cb, 0x686819);
        let bin_n5 = [e.read_f32(eax), e.read_f32(eax + 4), e.read_f32(eax + 8)];
        let bin_d5 = e.read_f32(eax + 0xc);
        let (n5, d5) = plane5_from_plane4(n4, vp);
        for k in 0..3 {
            assert_f32(bin_n5[k], n5[k], &format!("{ctx} n5[{k}]"));
        }
        assert_f32(bin_d5, d5, &format!("{ctx} d5"));
    };

    run_case(&mut e, [0.0, 0.0, 0.0], [1.0, 2.0, 3.0], "plane5 zero-n");
    run_case(&mut e, [1.0, 0.0, 0.0], [5.0, 0.0, 0.0], "plane5 unit-x");
    run_case(
        &mut e,
        [-1.0, -1.0, -1.0],
        [0.0, 0.0, 0.0],
        "plane5 vp-zero",
    );
    let mut s = 0x6867_cece_u32;
    for i in 0..2000u32 {
        let n4 = [
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
        ];
        let vp = [
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
        ];
        run_case(&mut e, n4, vp, &format!("plane5 fuzz[{i}]"));
    }
}
