//! The volume box and the ground normal. `CMovement::IsVolumeObstructed` (`0x631670`) runs from
//! `0x631685` to `0x63172f`, with `CMovement::TransportToWorld` (`0x7c4930`) stubbed to return the
//! chosen position; `CMovement::RefreshGroundNormal` (`0x6332a0`) from `0x6332db` to `0x6333ad`;
//! `AverageGroundNormal` (`0x637140`) runs whole with `ClipPolygonByPlanes` (`0x631870`) stubbed to
//! accept every face, on faces the test writes to the list at `0xc4e52c`.

use benilla_re_collision::capsule::*;
use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// Asserts the same bits, or NaN on both sides.
fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// `0x631685` to `0x63172f`: the six box components.
#[test]
fn query_aabb_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pos = e.alloc(0x20);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;

    e.stub(0x7c4930, pos as u32, 2);
    let mut s = 0x6316_7000u32;

    let case = |e: &mut Emu, p: [f32; 3], xy_radius: f32, z_extent: f32, ctx: &str| {
        e.write_u32(pos, p[0].to_bits());
        e.write_u32(pos + 4, p[1].to_bits());
        e.write_u32(pos + 8, p[2].to_bits());
        e.write_u32(ebp + 0xc, xy_radius.to_bits());
        e.write_u32(ebp + 8, z_extent.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x631685, 0x63172f);
        let bin_min = [
            e.read_f32(0xc4e5a0),
            e.read_f32(0xc4e5a4),
            e.read_f32(0xc4e5a8),
        ];
        let bin_max = [
            e.read_f32(0xc4e5ac),
            e.read_f32(0xc4e5b0),
            e.read_f32(0xc4e5b4),
        ];
        let (our_min, our_max) = query_aabb(p, xy_radius, z_extent);
        for k in 0..3 {
            assert_f32(bin_min[k], our_min[k], &format!("{ctx} min[{k}]"));
            assert_f32(bin_max[k], our_max[k], &format!("{ctx} max[{k}]"));
        }
    };

    case(&mut e, [10.0, 20.0, 30.0], 2.0, 5.0, "simple");
    case(&mut e, [0.0; 3], 0.0, 0.0, "zeros");
    case(&mut e, [-100.0, 5.0, -3.0], 1.5, -1.0, "mixed/neg-extent");

    case(
        &mut e,
        [1.0, 1.0, 1.0],
        f32::from_bits(0x3400_0001),
        0.0,
        "1±(2⁻²³+1ulp)",
    );
    for i in 0..4000u32 {
        let p = if i % 4 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
            ]
        };
        let xy_radius = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -10.0, 50.0)
        };
        let z_extent = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -10.0, 50.0)
        };
        case(&mut e, p, xy_radius, z_extent, &format!("fuzz{i}"));
    }
}

/// `0x6332db` to `0x6333ad`: the six plane distances.
#[test]
fn capsule_kdop_dists_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x100);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x6332_a000u32;

    let case = |e: &mut Emu, c: [f32; 3], radius: f32, height: f32, ctx: &str| {
        e.write_u32(this + 0x10, c[0].to_bits());
        e.write_u32(this + 0x14, c[1].to_bits());
        e.write_u32(this + 0x18, c[2].to_bits());
        e.write_u32(this + 0xb0, radius.to_bits());
        e.write_u32(this + 0xb4, height.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6332db, 0x6333ad);
        let bin = [
            e.read_f32(ebp - 0x60),
            e.read_f32(ebp - 0x50),
            e.read_f32(ebp - 0x40),
            e.read_f32(ebp - 0x30),
            e.read_f32(ebp - 0x20),
            e.read_f32(ebp - 0x10),
        ];
        let ours = capsule_kdop_dists(c, radius, height);
        for k in 0..6 {
            assert_f32(bin[k], ours[k], &format!("{ctx} plane[{k}]"));
        }
    };

    case(&mut e, [1.0, 2.0, 3.0], 0.5, 2.0, "simple");
    case(&mut e, [0.0; 3], 0.0, 0.0, "zeros");
    case(&mut e, [-50.0, 7.5, -12.0], 1.0, 3.0, "mixed");

    case(
        &mut e,
        [1.0, -1.0, f32::from_bits(0x3f80_0001)],
        f32::from_bits(0x3400_0001),
        f32::from_bits(0x3400_0001),
        "1±(2⁻²³+1ulp) boundary",
    );
    for i in 0..4000u32 {
        let c = if i % 4 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -200.0, 200.0),
                frand(&mut s, -200.0, 200.0),
                frand(&mut s, -200.0, 200.0),
            ]
        };
        let radius = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 30.0)
        };
        let height = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 30.0)
        };
        case(&mut e, c, radius, height, &format!("fuzz{i}"));
    }
}

/// `0x637140` whole: the normal, over random faces either side of cos 89°.
#[test]
fn walkable_normal_avg_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const COUNT_GLOBAL: u64 = 0x00c4_e530;
    const ARRAY_GLOBAL: u64 = 0x00c4_e534;
    const STRIDE: u64 = 0x34;
    let recs = e.alloc(STRIDE * 64);
    let out = e.alloc(0x10);
    let ctx = e.alloc(0x10);
    let scratch = e.alloc(0x10);

    e.stub(0x631870, 1, 1);
    let mut s = 0x6371_4000u32;
    let mut seen_default = false;
    let mut seen_avg = false;

    let mut case = |e: &mut Emu, normals: &[[f32; 3]], ctx_str: &str| {
        for (i, n) in normals.iter().enumerate() {
            let base = recs + STRIDE * i as u64;
            e.write_u32(base, n[0].to_bits());
            e.write_u32(base + 4, n[1].to_bits());
            e.write_u32(base + 8, n[2].to_bits());
        }
        e.write_u32(COUNT_GLOBAL, normals.len() as u32);
        e.write_u32(ARRAY_GLOBAL, recs as u32);
        e.write_u32(out, 0x7fc0_0000);
        e.write_u32(out + 4, 0x7fc0_0000);
        e.write_u32(out + 8, 0x7fc0_0000);
        e.call(0x637140, 0, 0, &[out as u32, ctx as u32, scratch as u32]);
        let bin = [e.read_f32(out), e.read_f32(out + 4), e.read_f32(out + 8)];

        let mut sum = [0.0f32; 3];
        let mut count = 0i32;
        for n in normals {
            if walkable_normal_accepts(n[2]) {
                sum[0] = normal_sum_step(sum[0], n[0]);
                sum[1] = normal_sum_step(sum[1], n[1]);
                sum[2] = normal_sum_step(sum[2], n[2]);
                count += 1;
            }
        }
        let want = if count == 0 {
            seen_default = true;
            [0.0f32, 0.0f32, 1.0f32]
        } else {
            seen_avg = true;
            let (mag2, mx, my, mz) = mean_and_mag2(sum, count);
            renormalize(mag2, mx, my, mz)
        };
        for k in 0..3 {
            assert_f32(bin[k], want[k], &format!("{ctx_str} out[{k}]"));
        }
    };

    case(&mut e, &[], "empty → (0,0,1)");
    case(
        &mut e,
        &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        "all nz<sin1° rejected → (0,0,1)",
    );

    case(&mut e, &[[0.0, 0.0, 1.0]], "single up");
    case(&mut e, &[[0.1, 0.2, 0.9], [0.0, 0.1, 0.99]], "two walkable");

    case(
        &mut e,
        &[[0.5, 0.5, SIN_1DEG]],
        "nz==sin1° (==threshold, rejected → count 0 → default)",
    );
    let sin1_plus = f32::from_bits(SIN_1DEG.to_bits() + 1);
    case(
        &mut e,
        &[[0.5, 0.5, sin1_plus]],
        "nz==sin1°+1ulp (accepted)",
    );

    case(
        &mut e,
        &[[0.2, 0.2, 0.6], [0.2, 0.2, 0.6], [0.2, 0.2, 0.6]],
        "0.6/3 mean (live-vs-f32 mx/my/mz)",
    );

    for i in 0..2000u32 {
        let total = 1 + (next(&mut s) >> 8) % 12;
        let normals: Vec<[f32; 3]> = (0..total)
            .map(|_| {
                let nx = frand(&mut s, -1.0, 1.0);
                let ny = frand(&mut s, -1.0, 1.0);

                let nz = match (next(&mut s) >> 8) % 8 {
                    0 => frand_bits(&mut s),
                    1 => frand(&mut s, -0.02, 0.02),
                    _ => frand(&mut s, -1.0, 1.0),
                };
                [nx, ny, nz]
            })
            .collect();
        case(&mut e, &normals, &format!("fuzz{i}"));
    }
    assert!(
        seen_default && seen_avg,
        "both count==0 default and count>0 average exercised (default={seen_default} avg={seen_avg})"
    );
}
