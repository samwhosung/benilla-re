//! The world queries' arithmetic, as slices: `GatherWorldFacets` (`0x6aa8b0`), `GetSurfaceHeight`
//! (`0x6b7070`), `TraceMapObjectFloor` (`0x6b7790`) and `TestChunkBox` (`0x6ad610`).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::worldquery::{
    aabb_query_cell_indices, aabb_query_negates, chunk_box_translate, ray_aabb_inv,
    ray_t_attenuation, sphere_cell_indices, sphere_compose_z, sphere_seg_height, sphere_seg_top,
};
use common::{assert_f32, next};

/// An EBP in the middle of the harness's stack, so every frame access and push of a slice lands in
/// it.
const SLICE_EBP: u64 = 0x7008_0000;

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// The `i32` a `fistp dword` stored.
fn read_i32(e: &Emu, va: u64) -> i32 {
    e.read_u32(va) as i32
}

/// 17066.666 (`0x46855555`), the map's centre.
fn map_center() -> f32 {
    f32::from_bits(0x4685_5555)
}

/// `GatherWorldFacets` (`0x6aa8b0`) from `0x6aa930` to the first bounds compare at `0x6aa967`, eax =
/// the box: the four stored differences from `[ebp-0x34]` up.
#[test]
fn aabb_query_negates_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let corners = e.alloc(0x20);
    let mut s = 0x6aa9_0000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, c: [f32; 4], ctx: &str| {
        e.write_u32(corners + 0xc, c[0].to_bits());
        e.write_u32(corners + 0x10, c[1].to_bits());
        e.write_u32(corners, c[2].to_bits());
        e.write_u32(corners + 4, c[3].to_bits());
        e.set_reg(RegisterX86::EAX, corners);
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.run(0x6aa930, 0x6aa967);
        let bin = [
            e.read_f32(SLICE_EBP - 0x34),
            e.read_f32(SLICE_EBP - 0x30),
            e.read_f32(SLICE_EBP - 0x2c),
            e.read_f32(SLICE_EBP - 0x28),
        ];
        let ours = aabb_query_negates(c);
        for k in 0..4 {
            assert_f32(bin[k], ours[k], &format!("{ctx} neg[{k}]"));
        }
    };

    // a corner at the centre gives −0.0 after `fchs`
    case(
        &mut e,
        [map_center(), map_center(), map_center(), map_center()],
        "corner at the centre",
    );
    case(&mut e, [0.0, 0.0, 0.0, 0.0], "all-zero (neg = +C)");
    case(
        &mut e,
        [1e30, -1e30, f32::INFINITY, f32::NAN],
        "overflow/inf/nan",
    );

    for i in 0..6000u32 {
        let c: [f32; 4] = if i % 4 == 0 {
            std::array::from_fn(|_| frand_bits(&mut s))
        } else {
            std::array::from_fn(|_| frand(&mut s, -2000.0, 36000.0))
        };
        case(&mut e, c, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("aabb_query_negates: {checked} cases bit-exact vs WoW.exe @ 0x6aa8b0");
}

/// `0x6aa8b0` from `0x6aa9b4` to `0x6aaa14`: the four stored differences at `[ebp-0x34]` up to the
/// cell indices at `[ebp-0x8]`, `[ebp-0xc]`, `[ebp-0x10]` and `[ebp-0x14]`, each product stored
/// through `[ebp+0xc]`.
#[test]
fn aabb_query_cell_indices_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s = 0x6aaa_0000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, negs: [f32; 4], ctx: &str| {
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.write_u32(SLICE_EBP - 0x34, negs[0].to_bits());
        e.write_u32(SLICE_EBP - 0x30, negs[1].to_bits());
        e.write_u32(SLICE_EBP - 0x2c, negs[2].to_bits());
        e.write_u32(SLICE_EBP - 0x28, negs[3].to_bits());
        e.run(0x6aa9b4, 0x6aaa14);
        let bin = [
            read_i32(e, SLICE_EBP - 0x8),
            read_i32(e, SLICE_EBP - 0xc),
            read_i32(e, SLICE_EBP - 0x10),
            read_i32(e, SLICE_EBP - 0x14),
        ];
        let ours = aabb_query_cell_indices(negs);
        assert_eq!(bin, ours, "{ctx}: binary={bin:?} ours={ours:?}");
    };

    // 0 less 0.5 is a tie, which `fistp` takes to the even 0
    case(&mut e, [0.0, 0.0, 0.0, 0.0], "tie at -0.5");
    case(&mut e, [10.0, -10.0, 100.0, -100.0], "mixed mid-range");
    case(
        &mut e,
        [1e30, -1e30, f32::INFINITY, f32::NAN],
        "overflow, infinity and NaN",
    );

    for i in 0..6000u32 {
        let negs: [f32; 4] = if i % 4 == 0 {
            std::array::from_fn(|_| frand_bits(&mut s))
        } else {
            std::array::from_fn(|_| frand(&mut s, -20000.0, 20000.0))
        };
        case(&mut e, negs, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("aabb_query_cell_indices: {checked} cases bit-exact vs WoW.exe @ 0x6aa8b0");
}

/// `GetSurfaceHeight` (`0x6b7070`) from `0x6b70c1` to `0x6b7100`, esi = the point: the cell
/// indices at `[ebp-0x1c]` (y) and `[ebp-0xc]` (x).
#[test]
fn sphere_cell_indices_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pt = e.alloc(0x20);
    let mut s = 0x6b70_0000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, px: f32, py: f32, ctx: &str| {
        e.write_u32(pt, px.to_bits());
        e.write_u32(pt + 4, py.to_bits());
        e.set_reg(RegisterX86::ESI, pt);
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.run(0x6b70c1, 0x6b7100);
        let bin = [read_i32(e, SLICE_EBP - 0x1c), read_i32(e, SLICE_EBP - 0xc)];
        let ours = sphere_cell_indices(px, py);
        assert_eq!(bin, ours, "{ctx}: binary={bin:?} ours={ours:?}");
    };

    // the centre gives 0 less 0.5, a tie
    case(&mut e, map_center(), map_center(), "tie at -0.5");
    case(&mut e, 0.0, 0.0, "px=py=0");
    case(&mut e, 1e30, f32::NAN, "overflow and NaN");

    for i in 0..6000u32 {
        let (px, py) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -2000.0, 36000.0),
                frand(&mut s, -2000.0, 36000.0),
            )
        };
        case(&mut e, px, py, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("sphere_cell_indices: {checked} cases bit-exact vs WoW.exe @ 0x6b7070");
}

/// `0x6b7070` from `0x6b71a9` to the call at `0x6b71c6`, esi = the point, `[ebp+8]` = the range: the
/// segment's top at `[ebp-0x24]` and bottom at `[ebp-0x30]`.
#[test]
fn sphere_seg_prep_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pt = e.alloc(0x20);
    let mut s = 0x6b71_0000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, z: f32, pt_z: f32, ctx: &str| {
        e.write_u32(pt + 8, pt_z.to_bits());
        // x and y are only copied
        e.write_u32(pt, 0);
        e.write_u32(pt + 4, 0);
        e.set_reg(RegisterX86::ESI, pt);
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.write_u32(SLICE_EBP + 8, z.to_bits());
        e.run(0x6b71a9, 0x6b71c6);
        let bin = [e.read_f32(SLICE_EBP - 0x24), e.read_f32(SLICE_EBP - 0x30)];
        let ours = [sphere_seg_top(z, pt_z), sphere_seg_height(z, pt_z)];
        assert_f32(bin[0], ours[0], &format!("{ctx} seg_top"));
        assert_f32(bin[1], ours[1], &format!("{ctx} seg_height"));
    };

    case(&mut e, 1234.5, 1234.5, "equal, the bottom +0.0");
    case(&mut e, 0.0, 0.0, "zeros");
    case(&mut e, -3.0, 7.0, "mixed");
    case(&mut e, f32::INFINITY, f32::NEG_INFINITY, "infinities");

    for i in 0..6000u32 {
        let (z, pt_z) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -5000.0, 5000.0),
                frand(&mut s, -5000.0, 5000.0),
            )
        };
        case(&mut e, z, pt_z, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("sphere_seg_prep: {checked} cases bit-exact vs WoW.exe @ 0x6b7070");
}

/// `0x6b7070` from `0x6b7225` to `0x6b7237`, after the store: the range `[ebp+8]`, t `[ebp-0x8]` and
/// the top `[ebp-0x24]` to the output that `[ebp-0x18]` points at.
#[test]
fn sphere_compose_z_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out = e.alloc(0x10);
    let mut s = 0x6b72_0000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, z: f32, t: f32, seg_top: f32, ctx: &str| {
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.write_u32(SLICE_EBP + 8, z.to_bits());
        e.write_u32(SLICE_EBP - 0x8, t.to_bits());
        e.write_u32(SLICE_EBP - 0x24, seg_top.to_bits());
        e.write_u32(SLICE_EBP - 0x18, out as u32);
        e.run(0x6b7225, 0x6b7237);
        let bin = e.read_f32(out);
        let ours = sphere_compose_z(z, t, seg_top);
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 50.0, 1.0, 150.0, "t = 1.0");
    case(&mut e, 50.0, 0.0, 150.0, "t = 0.0");
    case(&mut e, 0.0, 0.5, 0.0, "zeros");
    case(&mut e, f32::INFINITY, 0.5, 100.0, "infinite range");

    for i in 0..6000u32 {
        let (z, t, seg_top) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -200.0, 200.0),
                frand(&mut s, 0.0, 1.0),
                frand(&mut s, -5000.0, 5000.0),
            )
        };
        case(&mut e, z, t, seg_top, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("sphere_compose_z: {checked} cases bit-exact vs WoW.exe @ 0x6b7070");
}

/// `TraceMapObjectFloor` (`0x6b7790`) from `0x6b779f` to the `jne` at `0x6b77b5`, esi = the top and
/// eax = the bottom: the reciprocal at `[ebp-0x18]` and `[ebp-0x8]`, the same bits in both.
#[test]
fn ray_aabb_inv_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ray_a = e.alloc(0x20);
    let ray_b = e.alloc(0x20);
    let mut s = 0x6b77_0000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, a_z: f32, b_z: f32, ctx: &str| {
        e.write_u32(ray_a + 8, a_z.to_bits());
        e.write_u32(ray_b + 8, b_z.to_bits());
        e.set_reg(RegisterX86::EAX, ray_a);
        e.set_reg(RegisterX86::ESI, ray_b);
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.run(0x6b779f, 0x6b77b5);
        let s18 = e.read_f32(SLICE_EBP - 0x18);
        let s08 = e.read_f32(SLICE_EBP - 0x8);
        let ours = ray_aabb_inv(a_z, b_z);
        assert_f32(s18, ours, &format!("{ctx} [ebp-0x18]"));
        assert_f32(s08, ours, &format!("{ctx} [ebp-0x8]"));
        assert_eq!(
            s18.to_bits(),
            s08.to_bits(),
            "{ctx}: [ebp-0x18]={:#010x} != [ebp-0x8]={:#010x}",
            s18.to_bits(),
            s08.to_bits()
        );
    };

    // a zero height divides to +inf
    case(&mut e, 100.0, 100.0, "zero height");
    case(&mut e, 10.0, 20.0, "height 10");
    case(&mut e, 20.0, 10.0, "height -10");
    case(&mut e, f32::NAN, 1.0, "nan a_z");

    for i in 0..6000u32 {
        let (a_z, b_z) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -1000.0, 1000.0),
                frand(&mut s, -1000.0, 1000.0),
            )
        };
        case(&mut e, a_z, b_z, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("ray_aabb_inv: {checked} cases bit-exact vs WoW.exe @ 0x6b7790");
}

/// `0x6b7790` from `0x6b7885` to `0x6b789c` (t at `[ebp-0xc]`, maxt at `[ebp-0x4]`) and from
/// `0x6b7a2c` to `0x6b7a43` (t at `[ebp-0x14]`, maxt at `[ebp-0xc]`), the reciprocal at `[ebp-0x18]`
/// and `[ebp-0x8]` in both: the result at `[edi]`, before the `fcomp` that follows.
#[test]
fn ray_t_attenuation_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out = e.alloc(0x10);
    let mut s = 0x6b78_0000u32;
    let mut checked = 0u32;

    let case1 = |e: &mut Emu, inv: f32, t: f32, maxt: f32, ctx: &str| {
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.set_reg(RegisterX86::EDI, out);
        e.write_u32(SLICE_EBP - 0x18, inv.to_bits());
        e.write_u32(SLICE_EBP - 0x8, inv.to_bits());
        e.write_u32(SLICE_EBP - 0xc, t.to_bits());
        e.write_u32(SLICE_EBP - 0x4, maxt.to_bits());
        e.run(0x6b7885, 0x6b789c);
        let bin = e.read_f32(out);
        let ours = ray_t_attenuation(inv, t, maxt);
        assert_f32(bin, ours, &format!("{ctx} at 0x6b7885"));
    };
    let case2 = |e: &mut Emu, inv: f32, t: f32, maxt: f32, ctx: &str| {
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.set_reg(RegisterX86::EDI, out);
        e.write_u32(SLICE_EBP - 0x18, inv.to_bits());
        e.write_u32(SLICE_EBP - 0x8, inv.to_bits());
        e.write_u32(SLICE_EBP - 0x14, t.to_bits());
        e.write_u32(SLICE_EBP - 0xc, maxt.to_bits());
        e.run(0x6b7a2c, 0x6b7a43);
        let bin = e.read_f32(out);
        let ours = ray_t_attenuation(inv, t, maxt);
        assert_f32(bin, ours, &format!("{ctx} at 0x6b7a2c"));
    };

    case1(&mut e, 0.0, 0.5, 0.75, "inv = 0");
    case2(&mut e, 0.0, 0.5, 0.75, "inv = 0");
    case1(&mut e, 1.0, 0.0, 5.0, "t = 0, inv = 1");
    case2(&mut e, 1.0, 0.0, 5.0, "t = 0, inv = 1");
    case1(&mut e, 0.1, 0.5, 1.0, "typical");
    case2(&mut e, 0.1, 0.5, 1.0, "typical");

    for i in 0..6000u32 {
        let (inv, t, maxt) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, 0.0, 1.0),
                frand(&mut s, 0.0, 1.0),
            )
        };
        case1(&mut e, inv, t, maxt, &format!("fuzz{i}"));
        case2(&mut e, inv, t, maxt, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("ray_t_attenuation: {checked} cases bit-exact vs WoW.exe @ 0x6b7790");
}

/// `TestChunkBox` (`0x6ad610`) from `0x6ad676` to the call at `0x6ad6b0`, ebx = the chunk: the box
/// at `[ebp-0x40]` up, translated in place.
#[test]
fn chunk_box_translate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let origin = e.alloc(0x100);
    let mut s = 0x6ad6_0000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, box3: [f32; 3], org: [f32; 3], ctx: &str| {
        e.write_u32(origin + 0x6c, org[0].to_bits());
        e.write_u32(origin + 0x70, org[1].to_bits());
        e.write_u32(origin + 0x74, org[2].to_bits());
        e.set_reg(RegisterX86::EBX, origin);
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.write_u32(SLICE_EBP - 0x40, box3[0].to_bits());
        e.write_u32(SLICE_EBP - 0x3c, box3[1].to_bits());
        e.write_u32(SLICE_EBP - 0x38, box3[2].to_bits());
        e.run(0x6ad676, 0x6ad6b0);
        let bin = [
            e.read_f32(SLICE_EBP - 0x40),
            e.read_f32(SLICE_EBP - 0x3c),
            e.read_f32(SLICE_EBP - 0x38),
        ];
        let ours = chunk_box_translate(box3, org);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} box[{k}]"));
        }
    };

    // a zero origin negates to −0.0, and +0.0 plus −0.0 is +0.0
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "zero box and origin",
    );
    case(
        &mut e,
        [123.5, -456.25, 789.0],
        [123.5, -456.25, 789.0],
        "box at the origin",
    );
    case(
        &mut e,
        [10.0, 20.0, 30.0],
        [1.0, 2.0, 3.0],
        "typical translate",
    );
    case(
        &mut e,
        [1e30, -1e30, f32::INFINITY],
        [f32::NAN, 1e30, 1.0],
        "extremes and NaN",
    );

    for i in 0..6000u32 {
        let (box3, org): ([f32; 3], [f32; 3]) = if i % 4 == 0 {
            (
                std::array::from_fn(|_| frand_bits(&mut s)),
                std::array::from_fn(|_| frand_bits(&mut s)),
            )
        } else {
            (
                std::array::from_fn(|_| frand(&mut s, -36000.0, 36000.0)),
                std::array::from_fn(|_| frand(&mut s, -36000.0, 36000.0)),
            )
        };
        case(&mut e, box3, org, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("chunk_box_translate: {checked} cases bit-exact vs WoW.exe @ 0x6ad610");
}
