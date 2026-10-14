//! The effect, equipment and unit sound kernels of `src/objf2_fp.rs` against the exe. Each runs the
//! slice of the function it covers, with the frame, registers and fields it reads set directly,
//! reading the store it ends in:
//! - `CEffect::Update` (`0x620580`): the re-resolve test `[0x620641, 0x620676)` and the half-scale
//!   `[0x6206da, 0x620705)`;
//! - `UpdateSpellEffectObject` (`0x620be0`): the anchor add `[0x620d51, 0x620d74)`, the half-scale
//!   `[0x620d79, 0x620da5)` and the scale divide `[0x620f21, 0x620fe7)`;
//! - `GetDurabilityPercent` (`0x6229d0`): the percentage through `__ftol`, `[0x622a35, 0x622a46)`;
//! - `CGUnit_C::PlayFoley` (`0x623440`): the height bias `[0x62346a, 0x623482)`;
//! - `CGUnit_C::GetAttachmentPoint` (`0x623b90`): the table bias `[0x623bdf, 0x623bec)`, which
//!   also checks the table in the source against the image, and the function whole on the path
//!   without the attachment, which runs the index guard.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either
//! side, to its random inputs; the percentage adds the integers' extremes.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::objf2_fp::*;
use common::*;

// ---- shared helpers (the crate float idiom, per dynobj_fp.rs) -------------

#[test]
fn f2_effect_track_reresolve_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let mut s = 0x6205_8000u32;
    let mut seen_rr = false;
    let mut seen_cull = false;

    let mut case = |e: &mut Emu, np: [f32; 3], cur: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x94, np[0].to_bits());
        e.write_u32(ebp - 0x90, np[1].to_bits());
        e.write_u32(ebp - 0x8c, np[2].to_bits());
        e.write_u32(ebp - 0x54, cur[0].to_bits());
        e.write_u32(ebp - 0x50, cur[1].to_bits());
        e.write_u32(ebp - 0x4c, cur[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x620641, 0x620676);
        let m = ah(e) & 0x41;
        // jne 0x620748 (CULL) taken ⟺ (ah&0x41)!=0. RE-RESOLVE (jne not taken) ⟺ m==0 ⟺ dist²>eps.
        let bin_rr = m == 0;
        if bin_rr {
            seen_rr = true;
        } else {
            seen_cull = true;
        }
        assert_eq!(
            bin_rr,
            effect_track_reresolve(np, cur),
            "{ctx}: bin_rr={bin_rr} ah&0x41={m:#04x}"
        );
    };

    // Deterministic anchors + the eps-boundary probe. eps(0x801360)=0.0010000000474974513; no f32
    // delta makes dist² land exactly on it, so probe the two adjacent reachable dist² values either
    // side of eps (a strict-vs-inclusive error would surface here). dx bits 0x3d0186e3 → dx²≈
    // 1.000000127e-3 (> eps, RE-RESOLVE); 0x3d0186e2 → dx²≈9.99999892e-4 (< eps, CULL).
    case(
        &mut e,
        [10.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "big → re-resolve",
    );
    case(&mut e, [0.0; 3], [0.0; 3], "zero → cull");
    let dx_above = f32::from_bits(0x3d01_86e3);
    let dx_below = f32::from_bits(0x3d01_86e2);
    case(
        &mut e,
        [dx_above, 0.0, 0.0],
        [0.0; 3],
        "dist² just-above eps → re-resolve",
    );
    case(
        &mut e,
        [dx_below, 0.0, 0.0],
        [0.0; 3],
        "dist² just-below eps → cull",
    );
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], "nan → cull");
    case(
        &mut e,
        [1e-3, 0.0, 0.0],
        [0.0; 3],
        "tiny (1e-6) < eps → cull",
    );

    for i in 0..5000u32 {
        let scale = match i % 3 {
            0 => 0.05, // near the eps boundary (dist² ≈ eps-ish)
            1 => 5.0,
            _ => 1e4,
        };
        let np = if i % 7 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -scale, scale),
                frand(&mut s, -scale, scale),
                frand(&mut s, -scale, scale),
            ]
        };
        let cur = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        case(&mut e, np, cur, &format!("fuzz{i}"));
    }
    // The squared displacement at 0.001 (0.0316227766 along x) and a few steps either side, then
    // the specials in each coordinate of both positions.
    let edge = (f64::from(DIST2_EPS)).sqrt() as f32;
    let mut x = edge;
    for _ in 0..4 {
        x = next_down(x);
    }
    for _ in 0..9 {
        case(&mut e, [x, 0.0, 0.0], [0.0; 3], "eps edge");
        x = next_up(x);
    }
    for &v in &specials(&[edge]) {
        for k in 0..3 {
            let mut a = [0.1f32, 0.2, 0.3];
            a[k] = v;
            case(&mut e, a, [0.0; 3], "special new");
            case(&mut e, [0.1, 0.2, 0.3], a, "special cur");
        }
    }
    assert!(seen_rr, "re-resolve (dist²>eps) never seen");
    assert!(seen_cull, "cull (dist²<=eps / NaN) never seen");
}

#[test]
fn f2_vec3_half_scale_580_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let vec = e.alloc(0x20);
    let mut s = 0x6206_da00u32;
    let base = SLICE_ESP - 0xc;

    let case = |e: &mut Emu, v: [f32; 3], ctx: &str| {
        e.write_u32(vec, v[0].to_bits());
        e.write_u32(vec + 4, v[1].to_bits());
        e.write_u32(vec + 8, v[2].to_bits());
        e.set_reg(RegisterX86::EAX, vec);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6206da, 0x620705);
        // store order: [esp+8]=v[2]*0.5, [esp+4]=v[1]*0.5, [esp]=v[0]*0.5.
        let bin = [e.read_f32(base), e.read_f32(base + 4), e.read_f32(base + 8)];
        let ours = vec3_half_scale(v);
        assert_f32(bin[0], ours[0], &format!("{ctx} [esp]=v0*.5"));
        assert_f32(bin[1], ours[1], &format!("{ctx} [esp+4]=v1*.5"));
        assert_f32(bin[2], ours[2], &format!("{ctx} [esp+8]=v2*.5"));
    };

    case(&mut e, [4.0, 6.0, 8.0], "simple");
    case(&mut e, [0.0; 3], "zeros");
    case(&mut e, [-3.0, 1.0, -7.5], "mixed");
    case(&mut e, [1.0, 1.0, 1.0], "ones");
    for i in 0..4000u32 {
        let v = if i % 4 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e5, 1e5),
                frand(&mut s, -1e5, 1e5),
                frand(&mut s, -1e5, 1e5),
            ]
        };
        case(&mut e, v, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut a = [1.0f32, 2.0, 3.0];
            a[k] = v;
            case(&mut e, a, "special");
        }
    }
}

#[test]
fn f2_effect_anchor_add_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let mut s = 0x620b_e000u32;
    let base = SLICE_ESP - 0xc;

    let case = |e: &mut Emu, local: [f32; 3], parent: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x10, local[0].to_bits());
        e.write_u32(ebp - 0x14, local[1].to_bits());
        e.write_u32(ebp - 0x18, local[2].to_bits());
        e.write_u32(ebp - 0x1c, parent[0].to_bits());
        e.write_u32(ebp - 0x20, parent[1].to_bits());
        e.write_u32(ebp - 0x24, parent[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x620d51, 0x620d74);
        // store order: [esp+8]=local0+parent0, [esp+4]=local1+parent1, [esp]=local2+parent2.
        let bin = [e.read_f32(base + 8), e.read_f32(base + 4), e.read_f32(base)];
        let ours = effect_anchor_add(local, parent);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} comp[{k}]"));
        }
    };

    case(&mut e, [1.0, 2.0, 3.0], [10.0, 20.0, 30.0], "simple");
    case(&mut e, [0.0; 3], [0.0; 3], "zeros");
    case(&mut e, [-100.0, 5.0, 0.0], [1.0, -1.0, 0.5], "mixed");
    for i in 0..4000u32 {
        let (local, parent) = if i % 4 == 0 {
            (
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
            )
        } else {
            (
                [
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                ],
                [
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                ],
            )
        };
        case(&mut e, local, parent, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut a = [1.0f32, 2.0, 3.0];
            a[k] = v;
            case(&mut e, a, [0.5, 0.25, 0.125], "special local");
            case(&mut e, [0.5, 0.25, 0.125], a, "special parent");
        }
    }
}

#[test]
fn f2_vec3_half_scale_be0_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let mut s = 0x620b_e100u32;
    let base = SLICE_ESP - 0xc;

    let case = |e: &mut Emu, v: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x28, v[0].to_bits());
        e.write_u32(ebp - 0x2c, v[1].to_bits());
        e.write_u32(ebp - 0x30, v[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x620d79, 0x620da5);
        // store order: [esp+8]=v[0]*0.5, [esp+4]=v[1]*0.5, [esp]=v[2]*0.5.
        let bin = [e.read_f32(base + 8), e.read_f32(base + 4), e.read_f32(base)];
        let ours = vec3_half_scale(v);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} comp[{k}]"));
        }
    };

    case(&mut e, [4.0, 6.0, 8.0], "simple");
    case(&mut e, [0.0; 3], "zeros");
    case(&mut e, [-3.0, 1.0, -7.5], "mixed");
    for i in 0..4000u32 {
        let v = if i % 4 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e5, 1e5),
                frand(&mut s, -1e5, 1e5),
                frand(&mut s, -1e5, 1e5),
            ]
        };
        case(&mut e, v, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut a = [1.0f32, 2.0, 3.0];
            a[k] = v;
            case(&mut e, a, "special");
        }
    }
}

#[test]
fn f2_attach_scale_div_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let mut s = 0x620f_2100u32;
    let result_slot = SLICE_ESP - 4; // after `push ecx` (0x620f62), `fstp [esp]` writes here

    let case = |e: &mut Emu, child: f32, parent: f32, ctx: &str| {
        e.write_u32(ebp - 0x98, parent.to_bits());
        e.write_u32(ebp - 4, child.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x620f21, 0x620fe7);
        let bin = e.read_f32(result_slot);
        assert_f32(bin, attach_scale_div(child, parent), ctx);
    };

    // the parent>0 branch is taken iff parent ordered-and->0 (fcomp 0.0; test ah,0x44; jp). Keep
    // parent strictly positive so the divisor is [ebp-0x98] (the slice's entry assumption).
    case(&mut e, 10.0, 2.0, "10/2");
    case(&mut e, 1.0, 3.0, "1/3");
    case(&mut e, 0.0, 1.0, "0/1");
    case(&mut e, -5.0, 2.5, "neg child");
    case(&mut e, 1.5, 1.0, "1.5/1");
    // child==parent boundary (exact 1.0 result) — a deterministic equality probe.
    case(&mut e, 3.0, 3.0, "3/3==1.0 (equality probe)");
    case(&mut e, 7.0, 7.0, "7/7==1.0 (equality probe)");
    for i in 0..4000u32 {
        let child = if i % 5 == 0 {
            frand_bits(&mut s).abs() // keep finite-ish; full-space child below in another arm
        } else {
            frand(&mut s, -50.0, 50.0)
        };
        // parent strictly positive (the branch's precondition); span tiny..large.
        let parent = match i % 3 {
            0 => frand(&mut s, 1e-3, 0.1),
            1 => frand(&mut s, 0.1, 5.0),
            _ => frand(&mut s, 5.0, 1e4),
        };
        case(&mut e, child, parent, &format!("fuzz{i}"));
    }
    // The slice picks the divisor: `[ebp-0x98]` when it is above 0, else a model bound read through
    // `eax`, which the test does not set, so the divisor specials are the positive ones.
    for &v in &specials(&[]) {
        case(&mut e, v, 2.0, "special child");
        if v > 0.0 {
            case(&mut e, 2.0, v, "special divisor");
        }
    }
}

#[test]
fn f2_durability_percent_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x6229_d000u32;

    let case = |e: &mut Emu, cur: i32, max: i32, ctx: &str| {
        e.write_u32(ebp - 8, cur as u32);
        e.write_u32(ebp - 4, max as u32);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x622a35, 0x622a46);
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        assert_eq!(
            bin,
            durability_percent(cur, max),
            "{ctx}: cur={cur} max={max} binary={bin} ours={}",
            durability_percent(cur, max)
        );
    };

    // max != 0 always (the divisor guard). Cover full/partial/empty + the truncation-toward-zero
    // edges and a signed-current path (sums are signed int sums).
    case(&mut e, 50, 200, "25%");
    case(&mut e, 1, 3, "33% trunc");
    case(&mut e, 2, 3, "66% trunc");
    case(&mut e, 100, 100, "100%");
    case(&mut e, 0, 5, "0%");
    case(&mut e, 5, 1, "500% (over-full)");
    case(&mut e, -7, 2, "neg current → -350 (signed)");
    case(&mut e, 7, -2, "neg max → -350 (signed)");
    case(&mut e, 199, 200, "99.5% → 99 trunc");
    for i in 0..4000u32 {
        // mostly plausible counts (0..4096), with a signed arm.
        let max = {
            let m = 1 + (next(&mut s) >> 8) % 4096;
            if next(&mut s) >> 31 == 1 {
                -(m as i32)
            } else {
                m as i32
            }
        };
        let cur = {
            let c = (next(&mut s) >> 8) % 8192;
            if next(&mut s) >> 31 == 1 {
                -(c as i32)
            } else {
                c as i32
            }
        };
        case(&mut e, cur, max, &format!("fuzz{i}"));
    }
    // Sums whose percentage is past `i32` keep the low dword of the 64-bit result, negative sums,
    // and the extremes; the divisor is never 0 (the function returns before the divide).
    for &(c, m) in &[
        (i32::MAX, 1),
        (i32::MIN, 1),
        (1, -1),
        (-5, 3),
        (5, -3),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MAX),
        (0, 7),
        (7, 7),
        (1, 3),
        (2, 3),
    ] {
        case(&mut e, c, m, "extremes");
    }
}

#[test]
fn f2_sound_pos_z_bias_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x6234_4000u32;

    let case = |e: &mut Emu, z: f32, ctx: &str| {
        e.write_u32(ebp - 4, z.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x62346a, 0x623482);
        let bin = e.read_f32(ebp - 4);
        assert_f32(bin, sound_pos_z_bias(z), ctx);
    };

    case(&mut e, 5.0, "5+2");
    case(&mut e, 0.0, "0+2");
    case(&mut e, -2.0, "-2+2 → 0");
    case(&mut e, 100.5, "100.5+2");
    for i in 0..4000u32 {
        let z = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1e4, 1e4)
        };
        case(&mut e, z, &format!("fuzz{i}"));
    }
    for &v in &specials(&[2.0]) {
        case(&mut e, v, "special");
    }
}

#[test]
fn f2_sound_attach_z_bias_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x623b_9000u32;
    const TABLE: u64 = 0x0086_2708;

    let mut table = [0.0f32; 0x25];
    for (i, t) in table.iter_mut().enumerate() {
        *t = e.read_f32(TABLE + 4 * i as u64);
        assert_eq!(
            t.to_bits(),
            ATTACH_Z_OFFSET[i].to_bits(),
            "ATTACH_Z_OFFSET[{i}] mirror mismatch: image={:#010x} const={:#010x}",
            t.to_bits(),
            ATTACH_Z_OFFSET[i].to_bits()
        );
    }

    let case = |e: &mut Emu, z: f32, idx: i32, table: &[f32], ctx: &str| {
        e.write_u32(ebp - 4, z.to_bits());
        e.set_reg(RegisterX86::EBX, idx as u32 as u64);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x623bdf, 0x623bec);
        let bin = e.read_f32(ebp - 4);
        assert_f32(bin, sound_attach_z_bias(z, idx, table), ctx);
    };

    // deterministic boundary indices: 0 (first), 0x24 (last valid), 5/18/29 (distinct table values).
    case(&mut e, 10.0, 0, &table, "idx0 (+1.0)");
    case(&mut e, 10.0, 5, &table, "idx5 (+1.8)");
    case(&mut e, 10.0, 18, &table, "idx18 (+2.5)");
    case(&mut e, 10.0, 19, &table, "idx19 (+0.0)");
    case(&mut e, 10.0, 29, &table, "idx29 (+3.0)");
    case(&mut e, 10.0, 0x24, &table, "idx0x24 (last valid, +1.0)");
    case(&mut e, 0.0, 0x24, &table, "z0 idx0x24");
    case(&mut e, -3.5, 11, &table, "neg z idx11 (+2.0)");
    for i in 0..3000u32 {
        let idx = ((next(&mut s) >> 8) % 0x25) as i32; // in-range [0, 0x25) — the slice's domain
        let z = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1e4, 1e4)
        };
        case(&mut e, z, idx, &table, &format!("fuzz{i}"));
    }
    // out-of-range idx is the unchanged path — exercised purely against the Rust fn (the binary's
    // range gate at 0x623bd6 precedes the slice, so these never enter the add).
    for idx in [-1, 0x25, 0x100, i32::MIN, i32::MAX] {
        assert_f32(
            7.5,
            sound_attach_z_bias(7.5, idx, &table),
            &format!("out-of-range idx={idx} → unchanged"),
        );
    }
    // The slice is the add alone, which the range guard before it reaches only for an index in
    // range: the ends of the range, with the live table.
    for idx in [0, 1, 0x24] {
        for &v in &specials(&[]) {
            case(&mut e, v, idx, &table, "special");
        }
    }
}

/// `CGUnit_C::GetAttachmentPoint` (`0x623b90`) whole, on the path without the attachment: the
/// model query `0x712cb0` returns 0, `vtable+0x14` writes the unit's position through its argument,
/// and the z gets the table bias only for an index in `[0, 0x25)`. The out vector's x and y are the
/// position's bits, and z is compared with `sound_attach_z_bias`.
#[test]
fn f2_get_attachment_point_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let vt = e.alloc(0x100);
    let model = e.alloc(0x100);
    let out = e.alloc(0x10);
    let payload = e.alloc(0x10);
    let island = e.alloc(0x10);
    e.write_u32(this, vt as u32);
    e.write_u32(vt + 0x14, island as u32);
    // `vtable+0x14(out)` writes three floats through its stack argument (ret 4).
    e.stub_write_through(island, 0, payload, 12, 0, 1);
    e.stub(0x712cb0, 0, 1); // the model has no such attachment

    const TABLE: u64 = 0x0086_2708;
    let mut table = [0.0f32; 0x25];
    for (i, t) in table.iter_mut().enumerate() {
        *t = e.read_f32(TABLE + 4 * i as u64);
    }
    for with_model in [false, true] {
        e.write_u32(this + 0xd8, if with_model { model as u32 } else { 0 });
        let case = |e: &mut Emu, z: f32, idx: i32, ctx: &str| {
            let pos = [1.5f32, -2.5, z];
            for (k, c) in pos.iter().enumerate() {
                e.write_u32(payload + 4 * k as u64, c.to_bits());
            }
            e.write(out, &[0u8; 12]);
            e.call(0x623b90, this as u32, 0, &[out as u32, idx as u32]);
            assert_eq!(e.read_u32(out), pos[0].to_bits(), "{ctx}: x");
            assert_eq!(e.read_u32(out + 4), pos[1].to_bits(), "{ctx}: y");
            assert_f32(
                e.read_f32(out + 8),
                sound_attach_z_bias(z, idx, &table),
                ctx,
            );
        };
        for idx in [-1, 0, 1, 0x24, 0x25, 0x26, i32::MIN, i32::MAX, 5, 18, 29] {
            for &v in &specials(&[]) {
                case(&mut e, v, idx, "idx");
            }
        }
    }
}
