//! `CMovement::ClassifyContactTriangle` (`0x6334a0`) runs whole with `IsUnitPlayerDriven`
//! (`0x5fa550`) stubbed to each answer, so both slope limits are covered: `eax`, the steepness
//! output and the flags. The inside test `0x6335d0` and the box test `0x6333d0` it calls run
//! unstubbed.

use benilla_re_collision::resolve_clip::point_in_tri;
use benilla_re_collision::resolve_slope::walkable_slope;
use benilla_re_collision::resolve_state::aabb_top_flush_test;
use benilla_re_difftest::{Emu, Wow};

const COS50: f32 = f32::from_bits(0x3f24_8dbb);
const COS80: f32 = f32::from_bits(0x3e31_d0d4);

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// A random `f32` in `[-4, 4)`.
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}

#[derive(Clone)]
struct Case {
    foot: [f32; 3],
    radius: f32,
    height: f32,
    flags: u32,
    normal: [f32; 3],
    plane_d: f32,
    basis: [[f32; 3]; 3],
}

/// The plane `d` that puts a box corner a given distance from the plane.
fn flush_d(n: &[f32; 3], foot: &[f32; 3], r: f32, h: f32) -> f32 {
    let c = [
        (foot[0] as f64 - r as f64) as f32,
        (foot[1] as f64 - r as f64) as f32,
        (foot[2] as f64 + h as f64) as f32,
    ];
    let dot = (n[2] as f64 * c[2] as f64 + n[1] as f64 * c[1] as f64) + n[0] as f64 * c[0] as f64;
    (-dot) as f32
}

/// A triangle record.
fn tri(c: [f32; 2], r: f32, z: f32) -> [[f32; 3]; 3] {
    [
        [c[0] + r, c[1], z],
        [c[0] - r * 0.5, c[1] + r, z],
        [c[0] - r * 0.5, c[1] - r, z],
    ]
}

/// Writes a triangle record to the list.
fn build(s: &mut u32, scenario: u32) -> Case {
    let foot = [fz(s) * 0.5, fz(s) * 0.5, fz(s) * 0.5];
    let radius = fz(s).abs() + 0.3;
    let height = fz(s).abs() + 0.2;
    let bit = if (next(s) >> 8) & 1 == 0 {
        0x0400_0000
    } else {
        0
    };
    let flags = bit | ((next(s) >> 24) & 0x55);
    let nxy = [fz(s) * 0.2, fz(s) * 0.2];
    let nz = match scenario {
        0 | 1 => 0.85 + fz(s).abs() * 0.05,
        2 => fz(s).abs() * 0.10,
        3 => 0.30 + fz(s).abs() * 0.05,
        4 => -(0.30 + fz(s).abs() * 0.05),
        5 => -(0.80 + fz(s).abs() * 0.05),
        _ => -(fz(s).abs() * 0.10 + 0.01),
    };
    let normal = [nxy[0], nxy[1], nz];
    let inside = scenario != 1;
    let centre = [foot[0], foot[1]];
    let basis = if inside {
        tri(centre, 2.0, foot[2])
    } else {
        tri([centre[0] + 20.0, centre[1] + 20.0], 2.0, foot[2])
    };
    let plane_d = match scenario {
        4 => fz(s).abs() + 100.0,
        5 | 6 => flush_d(&normal, &foot, radius, height),
        _ => fz(s),
    };
    Case {
        foot,
        radius,
        height,
        flags,
        normal,
        plane_d,
        basis,
    }
}

/// Runs one case and compares the three outputs.
#[allow(clippy::too_many_arguments)]
fn run_case(
    emu: &mut Emu,
    this_va: u64,
    hits: u64,
    out_va: u64,
    pred: bool,
    c: &Case,
    idx: u64,
    label: &str,
    tally: &mut [u32; 8],
    checked: &mut u32,
) {
    for (k, v) in c.foot.iter().enumerate() {
        emu.write_u32(this_va + 0x10 + (k as u64) * 4, v.to_bits());
    }
    emu.write_u32(this_va + 0xb0, c.radius.to_bits());
    emu.write_u32(this_va + 0xb4, c.height.to_bits());
    emu.write_u32(this_va + 0x40, c.flags);
    let rv = hits + idx * 0x34;
    for (k, v) in c.normal.iter().enumerate() {
        emu.write_u32(rv + (k as u64) * 4, v.to_bits());
    }
    emu.write_u32(rv + 0xc, c.plane_d.to_bits());
    for (r, row) in c.basis.iter().enumerate() {
        for (k, v) in row.iter().enumerate() {
            emu.write_u32(rv + 0x10 + (r as u64) * 0xc + (k as u64) * 4, v.to_bits());
        }
    }
    let sent = 0x7fc0_0001u32;
    emu.write_u32(out_va, sent);

    let bin_eax = emu.call(0x6334a0, this_va as u32, 0, &[idx as u32, out_va as u32]);
    let bin_out = emu.read_u32(out_va);
    let bin_flags = emu.read_u32(this_va + 0x40);

    let mut flags = c.flags;
    let mut out_steep = sent;
    let our_eax = walkable_slope(
        &mut flags,
        &c.foot,
        c.radius,
        c.height,
        pred,
        &c.normal,
        c.plane_d,
        &c.basis,
        &mut out_steep,
    );

    assert_eq!(
        bin_eax, our_eax,
        "p{pred} {label}: eax binary={bin_eax:#010x} ours={our_eax:#010x}"
    );
    assert_eq!(
        bin_out, out_steep,
        "p{pred} {label}: *out_steep binary={bin_out:#010x} ours={out_steep:#010x}"
    );
    assert_eq!(
        bin_flags, flags,
        "p{pred} {label}: flags binary={bin_flags:#010x} ours={flags:#010x}"
    );

    let thr = if pred { COS50 } else { COS80 };
    let nz = c.normal[2] as f64;
    if nz > thr as f64 || nz.is_nan() {
        tally[if point_in_tri(&c.basis, &c.foot) {
            0
        } else {
            1
        }] += 1;
    } else if nz >= 0.0 {
        tally[2 + (c.flags & 0x0400_0000 == 0) as usize] += 1;
    } else if !aabb_top_flush_test(&c.foot, c.radius, c.height, &c.normal, c.plane_d) {
        tally[4] += 1;
    } else {
        tally[if (-nz) > COS50 as f64 { 5 } else { 6 }] += 1;
        if c.flags & 0x0400_0000 != 0 {
            tally[7] += 1;
        }
    }
    *checked += 1;
}

/// Normals at and either side of both limits, 0 and NaN, and random triangles.
#[test]
fn slope_classify_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };

    let mut tally = [0u32; 8];
    let mut checked = 0u32;

    for pred in [false, true] {
        let mut emu = Emu::new(&wow);
        emu.stub(0x5fa550, pred as u32, 0);
        let this_va = emu.alloc(0x200);
        let hits = emu.alloc(0x400);
        let out_va = emu.alloc(0x10);
        emu.write_u32(0xc4e534, hits as u32);
        let obj_va = emu.alloc(0x10);
        emu.write_u32(this_va + 0x15c, obj_va as u32);

        let gate = if pred { COS50 } else { COS80 };
        let edges = [
            ("gate", gate),
            ("gate+ulp", f32::from_bits(gate.to_bits() + 1)),
            ("gate-ulp", f32::from_bits(gate.to_bits() - 1)),
            ("zero", 0.0),
            ("nan", f32::NAN),
            ("-cos50", -COS50),
            ("-cos50+ulp", f32::from_bits((-COS50).to_bits() + 1)),
            ("-cos50-ulp", f32::from_bits((-COS50).to_bits() - 1)),
        ];
        let mut es = 0x1234_5678u32 ^ ((pred as u32) << 8);
        for (name, nz) in edges {
            for variant in 0..4u32 {
                let mut c = build(&mut es, 5);
                c.normal[2] = nz;
                c.plane_d = flush_d(&c.normal, &c.foot, c.radius, c.height);
                if variant & 1 == 0 {
                    c.flags |= 0x0400_0000;
                } else {
                    c.flags &= !0x0400_0000;
                }
                if variant & 2 == 0 {
                    c.plane_d = (c.plane_d as f64 + 100.0) as f32;
                }
                let idx = (variant % 6) as u64;
                let label = format!("edge:{name}/v{variant}");
                run_case(
                    &mut emu,
                    this_va,
                    hits,
                    out_va,
                    pred,
                    &c,
                    idx,
                    &label,
                    &mut tally,
                    &mut checked,
                );
            }
        }

        let mut s = 0x5109_e2a1_u32 ^ ((pred as u32) << 16);
        for case in 0..160u32 {
            let scenario = if case < 7 {
                case
            } else {
                (next(&mut s) >> 8) % 7
            };
            let c = build(&mut s, scenario);
            let idx = ((next(&mut s) >> 8) % 6) as u64;
            let label = format!("case{case}/sc{scenario}");
            run_case(
                &mut emu,
                this_va,
                hits,
                out_va,
                pred,
                &c,
                idx,
                &label,
                &mut tally,
                &mut checked,
            );
        }
    }

    eprintln!(
        "slope_classify: {checked} rounds bit-exact vs WoW.exe | legs \
         walk-inface={} walk-outface={} wall-bitset={} wall-bitclear={} \
         down-noflush={} down-flush-steep={} down-flush-shallow={} out_steep-written={}",
        tally[0], tally[1], tally[2], tally[3], tally[4], tally[5], tally[6], tally[7]
    );
    for (leg, n) in tally.iter().enumerate() {
        assert!(*n > 0, "leg {leg} never exercised — fuzz gap");
    }
}
