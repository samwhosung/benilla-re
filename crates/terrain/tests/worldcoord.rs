//! The shadow test's indices (`IsInChunkShadow`, `0x69b350`), the ground probe (`ProbeGroundBelow`,
//! `0x672b60`) and the projector basis (`BuildProjectorBasis`, `0x687080`), as slices and whole calls.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::worldcoord::{
    probe_lerp_z, probe_segment_bottom_z, projector_basis_rows, tile_index_coarse, tile_index_fine,
    tile_neg_axis,
};
use common::{assert_f32, next};

/// An ESP in the middle of the harness's stack, so every push of a slice lands in it.
const SLICE_ESP: u64 = 0x7008_0000;

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// `IsInChunkShadow` (`0x69b350`, ecx = the position): the negated axes from `0x69b356` to
/// `0x69b371`, the chunk indices from `0x69b3c1` to `0x69b3f2` and the texel indices from `0x69b42f`
/// to `0x69b466`, each slice run alone on a frame whose `[ebp-N]` slots the test fills and reads.
#[test]
fn tile_index_conversion_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let pos = e.alloc(0x10);
    let ebp = frame + 0x20;
    let mut s = 0x69b3_0000u32;

    let slot = |disp: u64| ebp - disp;
    let (s_neg_y, s_neg_x) = (slot(0x4), slot(0x8)); // [ebp-4] is y, [ebp-8] is x
    let (s_tmp, s_idx, s_idy) = (slot(0xc), slot(0x14), slot(0x10));

    // 0x69b356 to 0x69b371: both axes from [ecx] and [ecx+4]
    let neg_case = |e: &mut Emu, px: f32, py: f32, ctx: &str| {
        e.write_u32(pos, px.to_bits());
        e.write_u32(pos + 4, py.to_bits());
        e.set_reg(RegisterX86::ECX, pos);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x69b356, 0x69b371);
        assert_f32(
            e.read_f32(s_neg_x),
            tile_neg_axis(px),
            &format!("{ctx} neg-x"),
        );
        assert_f32(
            e.read_f32(s_neg_y),
            tile_neg_axis(py),
            &format!("{ctx} neg-y"),
        );
    };
    // 0x69b3c1 to 0x69b3da: [ebp-4] to [ebp-0x14]
    let coarse_x = |e: &mut Emu, neg: f32, ctx: &str| -> i32 {
        e.write_u32(s_neg_y, neg.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x69b3c1, 0x69b3da);
        let bin = e.read_u32(s_idx) as i32;
        assert_eq!(bin, tile_index_coarse(neg), "{ctx} coarse-x (neg={neg})");
        bin
    };
    // 0x69b3da to 0x69b3f2: [ebp-8] to [ebp-0x10]
    let coarse_y = |e: &mut Emu, neg: f32, ctx: &str| {
        e.write_u32(s_neg_x, neg.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x69b3da, 0x69b3f2);
        let bin = e.read_u32(s_idy) as i32;
        assert_eq!(bin, tile_index_coarse(neg), "{ctx} coarse-y (neg={neg})");
    };
    // 0x69b42f to 0x69b448: [ebp-4] to [ebp-0x10]
    let fine_x = |e: &mut Emu, neg: f32, ctx: &str| {
        e.write_u32(s_neg_y, neg.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x69b42f, 0x69b448);
        let bin = e.read_u32(s_idy) as i32;
        assert_eq!(bin, tile_index_fine(neg), "{ctx} fine-x (neg={neg})");
    };
    // 0x69b448 to 0x69b466: [ebp-8] to [ebp-0x10], through [ebp-0xc]
    let fine_y = |e: &mut Emu, neg: f32, ctx: &str| {
        e.write_u32(s_neg_x, neg.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.write_u32(s_tmp, 0xdead_beef);
        e.run(0x69b448, 0x69b466);
        let bin = e.read_u32(s_idy) as i32;
        assert_eq!(bin, tile_index_fine(neg), "{ctx} fine-y (neg={neg})");
    };

    neg_case(&mut e, 0.0, 0.0, "neg zeros");
    neg_case(&mut e, 17066.666, 17066.666, "neg at-center (=> 0)");
    neg_case(&mut e, -17066.666, 34133.332, "neg span");
    // ties for `fistp` after the bias, and the signed zeros
    for &neg in &[0.0f32, 16.6875, 33.375, 50.0, -50.0, 16.5, -0.0] {
        coarse_x(&mut e, neg, "edge");
        coarse_y(&mut e, neg, "edge");
        fine_x(&mut e, neg, "edge");
        fine_y(&mut e, neg, "edge");
    }
    // a product near 1.0, so the biased value is near the tie at 0.5
    {
        let neg = 1.0_f32 / 0.03_f32;
        coarse_x(&mut e, neg, "tie-probe prod~1.0");
        coarse_y(&mut e, neg, "tie-probe prod~1.0");
    }

    let mut checked = 0u32;
    for i in 0..3000u32 {
        let (px, py) = if i % 5 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -40000.0, 40000.0),
                frand(&mut s, -40000.0, 40000.0),
            )
        };
        neg_case(&mut e, px, py, &format!("fuzz{i}"));
        let nx = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -2000.0, 36000.0)
        };
        let ny = if i % 4 == 1 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -2000.0, 36000.0)
        };
        coarse_x(&mut e, nx, &format!("fuzz{i}"));
        coarse_y(&mut e, ny, &format!("fuzz{i}"));
        fine_x(&mut e, nx, &format!("fuzz{i}"));
        fine_y(&mut e, ny, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "tile_index_conversion: {checked} cases bit-exact vs WoW.exe @ 0x69b350 \
         slices [356,371)/[3c1,3da)/[3da,3f2)/[42f,448)/[448,466)"
    );
}

/// `ProbeGroundBelow` (`0x672b60`): the trace's end `z − 20` from `0x672b77` to `0x672b96`, and the
/// height at the hit from `0x672c1c` to `0x672c2f`, each slice run alone; the segment query
/// (`0x672190`) between them is not run.
#[test]
fn ground_z_probe_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pt = e.alloc(0x10);
    let frame = e.alloc(0x60);
    let out = e.alloc(0x10);
    let ebp = frame + 0x40;
    let slot = |disp: u64| ebp - disp;
    let s_bottom = slot(0x8);
    let mut s = 0x672b_0000u32;

    // 0x672b77 to 0x672b96: [esi+8] less 20 to [ebp-8]
    let bottom_case = |e: &mut Emu, pz: f32, ctx: &str| {
        e.write_u32(pt + 8, pz.to_bits());
        e.set_reg(RegisterX86::ESI, pt);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x672b77, 0x672b96);
        assert_f32(
            e.read_f32(s_bottom),
            probe_segment_bottom_z(pz),
            &format!("{ctx} bottom-z"),
        );
    };
    // 0x672c1c to 0x672c2f: the end [ebp-0x24], the start [ebp-0x30] and t [ebp-4] to [edi]
    let s_end = slot(0x24);
    let s_start = slot(0x30);
    let s_t = slot(0x4);
    let lerp_case = |e: &mut Emu, end_z: f32, start_z: f32, t: f32, ctx: &str| {
        e.write_u32(s_end, end_z.to_bits());
        e.write_u32(s_start, start_z.to_bits());
        e.write_u32(s_t, t.to_bits());
        e.write_u32(out, 0xdead_beef);
        e.set_reg(RegisterX86::EDI, out);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x672c1c, 0x672c2f);
        assert_f32(e.read_f32(out), probe_lerp_z(end_z, start_z, t), ctx);
    };

    for &pz in &[
        0.0f32,
        20.0,
        -20.0,
        100.5,
        -1e6,
        1e6,
        f32::from_bits(0x3f800000),
    ] {
        bottom_case(&mut e, pz, "edge");
    }
    lerp_case(&mut e, 80.0, 100.0, 0.0, "t=0 -> startZ");
    lerp_case(&mut e, 80.0, 100.0, 1.0, "t=1 -> endZ");
    lerp_case(&mut e, 80.0, 100.0, 0.5, "t=0.5 midpoint");
    lerp_case(&mut e, 80.0, 100.0, 0.25, "composite z-20t (t=0.25)");

    let mut checked = 0u32;
    for i in 0..3000u32 {
        let pz = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -2000.0, 2000.0)
        };
        bottom_case(&mut e, pz, &format!("fuzz{i}"));
        let (end_z, start_z, t) = if i % 7 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -2000.0, 2000.0),
                frand(&mut s, -2000.0, 2000.0),
                frand(&mut s, 0.0, 1.0),
            )
        };
        lerp_case(&mut e, end_z, start_z, t, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "ground_z_probe: {checked} cases bit-exact vs WoW.exe @ 0x672b60 \
         slices [672b77,672b96)/[672c1c,672c2f)"
    );
}

/// `BuildProjectorBasis` (`0x687080`, ecx = this, edx = the matrix): from the entry to `0x687189`,
/// before the call to the determinant (`0x7bcf90`), the nine differences against
/// `projector_basis_rows`; and whole calls, where a basis with a determinant of 1 returns AL = 1 and
/// a degenerate one returns AL = 0 with the identity written.
#[test]
fn projector_basis_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x100);
    let outm = e.alloc(0x40);
    let mut s = 0x6870_0000u32;

    // the rows of `this` in the order the matrix takes them
    const CORNER_OFF: [[u64; 3]; 3] = [[0x84, 0x88, 0x8c], [0x6c, 0x70, 0x74], [0x90, 0x94, 0x98]];
    const BASE_OFF: [u64; 3] = [0x60, 0x64, 0x68];
    const OUT_OFF: [[u64; 3]; 3] = [[0x0, 0x4, 0x8], [0x10, 0x14, 0x18], [0x20, 0x24, 0x28]];

    let mut checked = 0u32;
    let mut proceeded = 0u32;
    let mut fell_back = 0u32;

    let case = |e: &mut Emu, corners: [[f32; 3]; 3], base: [f32; 3], ctx: &str| {
        for (r, offs) in CORNER_OFF.iter().enumerate() {
            for (c, &off) in offs.iter().enumerate() {
                e.write_u32(this + off, corners[r][c].to_bits());
            }
        }
        for (c, &off) in BASE_OFF.iter().enumerate() {
            e.write_u32(this + off, base[c].to_bits());
        }
        for i in 0..0x10u64 {
            e.write_u32(outm + i * 4, 0xdead_beef);
        }
        e.set_reg(RegisterX86::ECX, this);
        e.set_reg(RegisterX86::EDX, outm);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x687080, 0x687189);
        let ours = projector_basis_rows(corners, base);
        for (r, offs) in OUT_OFF.iter().enumerate() {
            for (c, &off) in offs.iter().enumerate() {
                assert_f32(
                    e.read_f32(outm + off),
                    ours[r][c],
                    &format!("{ctx} row{r} col{c}"),
                );
            }
        }
    };

    case(
        &mut e,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        [0.0, 0.0, 0.0],
        "identity-corners zero-base",
    );
    case(
        &mut e,
        [[10.0, 11.0, 12.0], [20.0, 21.0, 22.0], [30.0, 31.0, 32.0]],
        [5.0, 6.0, 7.0],
        "nontrivial",
    );
    for i in 0..2000u32 {
        let corners: [[f32; 3]; 3] = std::array::from_fn(|_| {
            std::array::from_fn(|_| {
                if i % 6 == 0 {
                    frand_bits(&mut s)
                } else {
                    frand(&mut s, -1000.0, 1000.0)
                }
            })
        });
        let base: [f32; 3] = std::array::from_fn(|_| {
            if i % 6 == 0 {
                frand_bits(&mut s)
            } else {
                frand(&mut s, -1000.0, 1000.0)
            }
        });
        case(&mut e, corners, base, &format!("fuzz{i}"));
        checked += 1;
    }

    let whole =
        |e: &mut Emu, corners: [[f32; 3]; 3], base: [f32; 3], expect_proceed: bool, ctx: &str| {
            for (r, offs) in CORNER_OFF.iter().enumerate() {
                for (c, &off) in offs.iter().enumerate() {
                    e.write_u32(this + off, corners[r][c].to_bits());
                }
            }
            for (c, &off) in BASE_OFF.iter().enumerate() {
                e.write_u32(this + off, base[c].to_bits());
            }
            for i in 0..0x10u64 {
                e.write_u32(outm + i * 4, 0xdead_beef);
            }
            // seven stack arguments (`ret 0x1c`), none read by the arithmetic
            let al = e.call(0x687080, this as u32, outm as u32, &[0; 7]) & 0xff;
            assert_eq!(
                al != 0,
                expect_proceed,
                "{ctx}: binary AL={al} expect_proceed={expect_proceed}"
            );
            if al == 0 {
                let ident: [f32; 16] = {
                    let mut m = [0.0f32; 16];
                    m[0] = 1.0;
                    m[5] = 1.0;
                    m[10] = 1.0;
                    m[15] = 1.0;
                    m
                };
                for (i, v) in ident.iter().enumerate() {
                    assert_f32(
                        e.read_f32(outm + i as u64 * 4),
                        *v,
                        &format!("{ctx} identity[{i}]"),
                    );
                }
            }
        };
    whole(
        &mut e,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        [0.0, 0.0, 0.0],
        true,
        "non-degenerate (det=1) proceeds",
    );
    proceeded += 1;
    whole(
        &mut e,
        [[2.0, 3.0, 4.0], [2.0, 3.0, 4.0], [2.0, 3.0, 4.0]],
        [1.0, 1.0, 1.0],
        false,
        "degenerate (coplanar, det=0) -> identity",
    );
    fell_back += 1;
    whole(
        &mut e,
        [[5.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        [0.0, 0.0, 0.0],
        false,
        "degenerate (collinear) -> identity",
    );
    fell_back += 1;

    assert!(
        proceeded > 0 && fell_back > 0,
        "both det-gate branches exercised (proceed={proceeded} fallback={fell_back})"
    );
    eprintln!(
        "projector_basis: {checked} slice cases + 3 whole-fn (proceed={proceeded} fallback={fell_back}) \
         bit-exact vs WoW.exe @ 0x687080 slice [687080,687189) and whole calls"
    );
}
