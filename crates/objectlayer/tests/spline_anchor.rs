//! `DotVector3` (`0x602630`) runs whole through `call_f64`; the pieces of `AnchorSplinePath`
//! (`0x6021c0`) run as slices: the one-vertex distance `0x6021da` to `0x60221e`, the edge and
//! bracket `0x602252` to `0x6022bb`, the first vertex's 2D distance `0x602324` to `0x602342`, the
//! matched vertex's distance `0x60237b` to `0x6023ad`, the unit edge and projections `0x602426` to
//! `0x6024c3`, and with straddling brackets on to the cut point, `0x602426` to `0x60257b`. A slice
//! that ends at a gate stops after `fnstsw` and compares `ah`, and the jump it decides, with the
//! transcription. The last vertex's 2D distance (`0x6025d7`) is the same code as the first's.
//! `SetVector3` (`0x4549a0`) runs unstubbed.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::spline_anchor::*;

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

/// `ah` after `fnstsw ax`: C0 is bit 0, C2 bit 2, C3 bit 6.
fn read_ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// Whether `test ah,5; jp` jumps for `v` against `t`: at least `t`, or NaN.
fn jp_taken_5(v: f64, t: f64) -> bool {
    v.is_nan() || v >= t
}

/// Whether `jp` jumps on the emulated `ah`: even parity of `ah & 5`.
fn ah5_jp_taken(ah: u8) -> bool {
    (ah & 5).count_ones().is_multiple_of(2)
}

/// Simple, zero and cancelling cases, and random vectors over all bit patterns and over ±1000.
#[test]
fn dot3_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pa = e.alloc(0x20);
    let pb = e.alloc(0x20);
    let mut s = 0x6026_3000u32;

    let case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(pa + 4 * k as u64, a[k].to_bits());
            e.write_u32(pb + 4 * k as u64, b[k].to_bits());
        }

        let bin = e.call_f64(0x602630, pa as u32, pb as u32, &[]);
        let ours = dot3(a, b);
        assert_eq!(
            bin.to_bits(),
            ours.to_bits(),
            "{ctx}: binary={:#018x} ours={:#018x}",
            bin.to_bits(),
            ours.to_bits()
        );
    };

    case(&mut e, [1.0, 2.0, 3.0], [4.0, 5.0, 6.0], "simple");
    case(&mut e, [0.0; 3], [0.0; 3], "zeros");

    case(&mut e, [1.0, 1.0, 1.0], [1.0, -1.0, 0.0], "1-1+0 cancel");
    case(&mut e, [3.0, 0.0, 0.0], [-1.0, 0.0, 0.0], "neg");
    for i in 0..6000u32 {
        let a = if i % 4 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e3, 1e3),
                frand(&mut s, -1e3, 1e3),
                frand(&mut s, -1e3, 1e3),
            ]
        };
        let b = if i % 4 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e3, 1e3),
                frand(&mut s, -1e3, 1e3),
                frand(&mut s, -1e3, 1e3),
            ]
        };
        case(&mut e, a, b, &format!("fuzz{i}"));
    }
}

/// The stored differences and the eps² gate, with points around the gate.
#[test]
fn anchor_region_a_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let v0 = e.alloc(0x20);
    let pp = e.alloc(0x20);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x6021_da00u32;
    let mut seen_ge = false;
    let mut seen_lt = false;

    let mut case = |e: &mut Emu, a: [f32; 3], p: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(v0 + 4 * k as u64, a[k].to_bits());
            e.write_u32(pp + 4 * k as u64, p[k].to_bits());
        }
        e.set_reg(RegisterX86::EBX, v0);
        e.set_reg(RegisterX86::ESI, pp);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6021da, 0x60221e);
        let bin_d = [
            e.read_f32(ebp - 0x2c),
            e.read_f32(ebp - 0x28),
            e.read_f32(ebp - 0x24),
        ];
        let ah = read_ah(e);

        let our_d = diff3(a, p);
        for k in 0..3 {
            assert_f32(bin_d[k], our_d[k], &format!("{ctx} diff[{k}]"));
        }

        let our_dist2 = dist2_3d_a(our_d);
        let want_taken = jp_taken_5(our_dist2, f64::from(EPS2));
        assert_eq!(
            ah5_jp_taken(ah),
            want_taken,
            "{ctx}: eps² gate AH={ah:#04x} dist2={our_dist2} (binary jp != ours)"
        );
        if want_taken {
            seen_ge = true;
        } else {
            seen_lt = true;
        }
    };

    let de = f64::from(EPS2).sqrt() as f32;
    case(
        &mut e,
        [de, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "dist²≈eps² (==threshold)",
    );
    case(&mut e, [0.0; 3], [0.0; 3], "dist²=0 (<eps²)");
    case(&mut e, [10.0, 0.0, 0.0], [0.0; 3], "dist²=100 (>eps²)");
    case(
        &mut e,
        [f32::NAN, 0.0, 0.0],
        [0.0; 3],
        "NaN diff → gate-taken",
    );

    for i in 0..4000u32 {
        let (a, p) = if i % 4 == 0 {
            (
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
            )
        } else {
            let p = [
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
            ];
            let scale = match i % 3 {
                0 => frand(&mut s, -0.05, 0.05),
                1 => frand(&mut s, -5.0, 5.0),
                _ => frand(&mut s, -0.03, 0.03),
            };
            let a = [p[0] + scale, p[1] + scale, p[2] + scale];
            (a, p)
        };
        case(&mut e, a, p, &format!("fuzz{i}"));
    }
    assert!(
        seen_ge && seen_lt,
        "both eps² gate arms exercised (>=eps²={seen_ge} <eps²={seen_lt})"
    );
}

/// The 2D distance and the 1.0 gate, with points around it.
#[test]
fn anchor_region_c_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let v0 = e.alloc(0x20);
    let pp = e.alloc(0x20);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x6023_2400u32;
    let mut seen_lt = false;
    let mut seen_ge = false;

    let mut case = |e: &mut Emu, v: [f32; 3], p: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(v0 + 4 * k as u64, v[k].to_bits());
            e.write_u32(pp + 4 * k as u64, p[k].to_bits());
        }
        e.set_reg(RegisterX86::EBX, v0);
        e.set_reg(RegisterX86::ESI, pp);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x602324, 0x602342);
        let ah = read_ah(e);

        let our_dist2 = dist2_2d(v, p);

        let bin_jnp = !ah5_jp_taken(ah);
        let our_jnp = !(our_dist2.is_nan() || our_dist2 >= f64::from(ONE));
        assert_eq!(
            bin_jnp, our_jnp,
            "{ctx}: 1.0 gate AH={ah:#04x} dist2={our_dist2} (binary jnp != ours)"
        );
        if our_jnp {
            seen_lt = true;
        } else {
            seen_ge = true;
        }
    };

    case(
        &mut e,
        [1.0, 0.0, 99.0],
        [0.0, 0.0, 0.0],
        "dist²==1.0 (==threshold, jnp not taken)",
    );
    let one_minus = f32::from_bits(1.0f32.to_bits() - 1);
    case(
        &mut e,
        [one_minus, 0.0, 0.0],
        [0.0; 3],
        "dist²<1.0 (jnp taken)",
    );
    case(&mut e, [0.0; 3], [0.0; 3], "dist²=0 (<1.0)");
    case(&mut e, [5.0, 0.0, 0.0], [0.0; 3], "dist²=25 (>1.0)");
    case(
        &mut e,
        [f32::NAN, 0.0, 0.0],
        [0.0; 3],
        "NaN → jnp not taken",
    );

    for i in 0..4000u32 {
        let (v, p) = if i % 4 == 0 {
            (
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
            )
        } else {
            let p = [
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
            ];

            let d = match i % 3 {
                0 => frand(&mut s, -1.5, 1.5),
                1 => frand(&mut s, -10.0, 10.0),
                _ => frand(&mut s, -0.9, 0.9),
            };
            ([p[0] + d, p[1] + d, frand(&mut s, -99.0, 99.0)], p)
        };
        case(&mut e, v, p, &format!("fuzz{i}"));
    }
    assert!(
        seen_lt && seen_ge,
        "both 1.0 gate arms exercised (<1.0={seen_lt} >=1.0={seen_ge})"
    );
}

/// The 3D distance and the eps² gate, with points around it.
#[test]
fn anchor_region_d_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    let arr = e.alloc(0x40);
    let pp = e.alloc(0x20);
    let mut s = 0x6023_7b00u32;
    let mut seen_ge = false;
    let mut seen_lt = false;

    let mut case = |e: &mut Emu, v: [f32; 3], p: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(arr + 4 * k as u64, v[k].to_bits());
            e.write_u32(pp + 4 * k as u64, p[k].to_bits());
        }
        e.set_reg(RegisterX86::EBX, arr);
        e.set_reg(RegisterX86::EAX, 1);
        e.set_reg(RegisterX86::ESI, pp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);

        e.set_reg(RegisterX86::EAX, 3);
        e.run(0x60237b, 0x6023ad);
        let ah = read_ah(e);

        let our_dist2 = dist2_3d_live(v, p);
        let want_taken = jp_taken_5(our_dist2, f64::from(EPS2));
        assert_eq!(
            ah5_jp_taken(ah),
            want_taken,
            "{ctx}: eps² gate AH={ah:#04x} dist2={our_dist2} (binary jp != ours)"
        );
        if want_taken {
            seen_ge = true;
        } else {
            seen_lt = true;
        }
    };

    let de = f64::from(EPS2).sqrt() as f32;
    case(&mut e, [de, 0.0, 0.0], [0.0; 3], "dist²≈eps² (==threshold)");
    case(&mut e, [0.0; 3], [0.0; 3], "dist²=0 (<eps²)");
    case(&mut e, [10.0, 0.0, 0.0], [0.0; 3], "dist²=100 (>eps²)");
    case(
        &mut e,
        [0.0, f32::NAN, 0.0],
        [0.0; 3],
        "NaN diff → gate-taken",
    );

    for i in 0..4000u32 {
        let (v, p) = if i % 4 == 0 {
            (
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
            )
        } else {
            let p = [
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
            ];
            let scale = match i % 3 {
                0 => frand(&mut s, -0.05, 0.05),
                1 => frand(&mut s, -5.0, 5.0),
                _ => frand(&mut s, -0.03, 0.03),
            };
            ([p[0] + scale, p[1] + scale, p[2] + scale], p)
        };
        case(&mut e, v, p, &format!("fuzz{i}"));
    }
    assert!(
        seen_ge && seen_lt,
        "both eps² gate arms exercised (>=eps²={seen_ge} <eps²={seen_lt})"
    );
}

/// The edge, `c`, `dA` and `dB` through three calls to `0x602630`.
#[test]
fn anchor_region_b_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    let verts = e.alloc(0x40);
    let edi = verts + 0xc;
    let pp = e.alloc(0x20);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x6022_5200u32;

    let case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], p: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(verts + 4 * k as u64, a[k].to_bits());
            e.write_u32(verts + 0xc + 4 * k as u64, b[k].to_bits());
            e.write_u32(pp + 4 * k as u64, p[k].to_bits());
        }
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::ESI, pp);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x602252, 0x6022bb);
        let bin_c = e.read_f32(ebp - 0x30);
        let bin_da = e.read_f32(ebp + 0xc);

        let edge = segment_edge(b, a);
        let our_c = bracket_c(edge, p);
        let our_da = bracket_d(edge, a, our_c);
        assert_f32(bin_c, our_c, &format!("{ctx} c"));
        assert_f32(bin_da, our_da, &format!("{ctx} dA"));
    };

    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.5, 1.0, 0.0],
        "unit-x seg",
    );
    case(&mut e, [0.0; 3], [0.0; 3], [0.0; 3], "degenerate edge");
    case(
        &mut e,
        [-3.0, 2.0, 1.0],
        [4.0, -1.0, 5.0],
        [0.0, 0.0, 0.0],
        "mixed",
    );

    case(
        &mut e,
        [1.0, 2.0, 3.0],
        [4.0, 6.0, 8.0],
        [1.0, 2.0, 3.0],
        "P==A (dA exact 0)",
    );

    for i in 0..4000u32 {
        let (a, b, p) = if i % 4 == 0 {
            (
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
            )
        } else {
            (
                [
                    frand(&mut s, -1e2, 1e2),
                    frand(&mut s, -1e2, 1e2),
                    frand(&mut s, -1e2, 1e2),
                ],
                [
                    frand(&mut s, -1e2, 1e2),
                    frand(&mut s, -1e2, 1e2),
                    frand(&mut s, -1e2, 1e2),
                ],
                [
                    frand(&mut s, -1e2, 1e2),
                    frand(&mut s, -1e2, 1e2),
                    frand(&mut s, -1e2, 1e2),
                ],
            )
        };
        case(&mut e, a, b, p, &format!("fuzz{i}"));
    }
}

/// The unit edge, `c` and `dB`.
#[test]
fn anchor_region_e_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    let block = e.alloc(0x40);
    let ecx = block + 0x10;
    let pp = e.alloc(0x20);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x6024_2600u32;

    let case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], p: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(block + 4 * k as u64, a[k].to_bits());
            e.write_u32(block + 0xc + 4 * k as u64, b[k].to_bits());
            e.write_u32(pp + 4 * k as u64, p[k].to_bits());
        }
        e.set_reg(RegisterX86::ECX, ecx);
        e.set_reg(RegisterX86::ESI, pp);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x602426, 0x6024c3);
        let bin_n = [
            e.read_f32(ebp - 0x20),
            e.read_f32(ebp - 0x1c),
            e.read_f32(ebp - 0x18),
        ];
        let bin_c = e.read_f32(ebp - 0x40);
        let bin_db = e.read_f32(ebp + 0xc);

        let edge = [
            f64::from(b[0]) - f64::from(a[0]),
            f64::from(b[1]) - f64::from(a[1]),
            f64::from(b[2]) - f64::from(a[2]),
        ];
        let mag2 = edge_len2(edge);
        let our_n = unit_dir(edge, mag2);
        let our_c = proj_c(our_n, p);
        let our_db = proj_d(our_n, b, our_c) as f32;
        for k in 0..3 {
            assert_f32(bin_n[k], our_n[k], &format!("{ctx} n[{k}]"));
        }
        assert_f32(bin_c, our_c, &format!("{ctx} c"));
        assert_f32(bin_db, our_db, &format!("{ctx} dB"));
    };

    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [3.0, 4.0, 0.0],
        [1.0, 1.0, 1.0],
        "3-4-5 edge",
    );
    case(
        &mut e,
        [1.0, 2.0, 3.0],
        [4.0, 6.0, 8.0],
        [2.0, 2.0, 2.0],
        "diag edge",
    );

    case(
        &mut e,
        [0.0; 3],
        [1.0, 0.0, 0.0],
        [0.5, 1.0, 0.0],
        "unit edge (|e|=1, ==threshold)",
    );
    case(
        &mut e,
        [-5.0, 3.0, -2.0],
        [2.0, -7.0, 6.0],
        [0.0, 0.0, 0.0],
        "mixed",
    );

    for i in 0..3000u32 {
        let a = [
            frand(&mut s, -1e2, 1e2),
            frand(&mut s, -1e2, 1e2),
            frand(&mut s, -1e2, 1e2),
        ];
        let off = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let b = if i % 50 == 0 {
            a
        } else {
            [a[0] + off[0], a[1] + off[1], a[2] + off[2]]
        };
        let p = [
            frand(&mut s, -1e2, 1e2),
            frand(&mut s, -1e2, 1e2),
            frand(&mut s, -1e2, 1e2),
        ];
        case(&mut e, a, b, p, &format!("fuzz{i}"));
    }
}

/// The cut point and its eps² gate, with brackets of opposite sign.
#[test]
fn anchor_region_f_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let block = e.alloc(0x40);
    let ecx = block + 0x10;
    let pp = e.alloc(0x20);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x6025_1c00u32;
    let mut seen_ge = false;
    let mut seen_lt = false;
    let mut ran = 0u32;

    let mut case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], sp: f32, perp: [f32; 3], ctx: &str| {
        let p = [
            a[0] + sp * (b[0] - a[0]) + perp[0],
            a[1] + sp * (b[1] - a[1]) + perp[1],
            a[2] + sp * (b[2] - a[2]) + perp[2],
        ];

        let edge = [
            f64::from(b[0]) - f64::from(a[0]),
            f64::from(b[1]) - f64::from(a[1]),
            f64::from(b[2]) - f64::from(a[2]),
        ];
        let mag2 = edge_len2(edge);
        let n = unit_dir(edge, mag2);
        let c = proj_c(n, p);
        let da = proj_d(n, a, c);
        let db_live = proj_d(n, b, c);
        let db_f32 = db_live as f32;

        if !(da <= 0.0 && f64::from(db_f32) >= 0.0) {
            return;
        }
        for k in 0..3 {
            e.write_u32(block + 4 * k as u64, a[k].to_bits());
            e.write_u32(block + 0xc + 4 * k as u64, b[k].to_bits());
            e.write_u32(pp + 4 * k as u64, p[k].to_bits());
        }
        e.set_reg(RegisterX86::ECX, ecx);
        e.set_reg(RegisterX86::ESI, pp);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x602426, 0x60257b);
        ran += 1;
        let bin_c = [
            e.read_f32(ebp - 0x2c),
            e.read_f32(ebp - 0x28),
            e.read_f32(ebp - 0x24),
        ];
        let ah = read_ah(e);

        let t = lerp_param(da, f64::from(db_f32));
        let our_c = lerp_point(a, b, t);
        for k in 0..3 {
            assert_f32(bin_c[k], our_c[k], &format!("{ctx} C[{k}]"));
        }
        let our_dist2 = dist2_bc(b, our_c);
        let want_taken = jp_taken_5(our_dist2, f64::from(EPS2));
        assert_eq!(
            ah5_jp_taken(ah),
            want_taken,
            "{ctx}: |B-C|² eps² gate AH={ah:#04x} dist2={our_dist2} (binary jp != ours)"
        );
        if want_taken {
            seen_ge = true;
        } else {
            seen_lt = true;
        }
    };

    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        0.5,
        [0.0, 1.0, 0.0],
        "mid, perp 1",
    );
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        0.3,
        [0.0, 0.0, 2.0],
        "0.3, perp 2",
    );
    case(
        &mut e,
        [1.0, 2.0, 3.0],
        [5.0, -4.0, 7.0],
        0.7,
        [0.5, 0.5, 0.5],
        "diag, perp .5",
    );

    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        0.999,
        [0.0, 0.02, 0.0],
        "near-B, perp .02 (eps² edge)",
    );

    for i in 0..3000u32 {
        let a = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let dir = [
            frand(&mut s, -30.0, 30.0),
            frand(&mut s, -30.0, 30.0),
            frand(&mut s, -30.0, 30.0),
        ];
        let b = [a[0] + dir[0], a[1] + dir[1], a[2] + dir[2]];
        let sp = frand(&mut s, 0.05, 0.95);

        let raw = [
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
        ];
        let dd = dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2];
        let k = if dd > 1e-6 {
            (raw[0] * dir[0] + raw[1] * dir[1] + raw[2] * dir[2]) / dd
        } else {
            0.0
        };
        let perp = [
            raw[0] - k * dir[0],
            raw[1] - k * dir[1],
            raw[2] - k * dir[2],
        ];
        case(&mut e, a, b, sp, perp, &format!("fuzz{i}"));
    }
    assert!(ran > 100, "the E→F straddle path ran ({ran} cases)");
    assert!(
        seen_ge && seen_lt,
        "both |B-C|² eps² gate arms exercised (>=eps²={seen_ge} <eps²={seen_lt})"
    );
}
