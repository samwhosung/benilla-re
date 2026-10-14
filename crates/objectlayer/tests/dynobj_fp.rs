//! The dynamic object and corpse kernels of `src/dynobj_fp.rs` against the exe. Each runs a slice
//! of its function, with the frame, registers and fields it reads set directly, reading the store
//! it ends in or the status byte `fnstsw ax` leaves:
//! - the dynamic object's animate slot `0x5d52f0`: the divide of either arm `[0x5d539b, 0x5d53d8)`
//!   and `[0x5d53c6, 0x5d53d8)`, and the two gates `[0x5d538b, 0x5d5396)` and `[0x5d53b6,
//!   0x5d53c1)`;
//! - its ground light `0x5d55c0`: the biased light value, `[0x5d5650, 0x5d5673)`;
//! - the corpse's submerged test `0x5d6540`, `[0x5d6584, 0x5d6592)`, its model-ready slot `0x5d6850`,
//!   `[0x5d689b, 0x5d68aa)`, and its attach centre `0x5d68c0`: the reciprocal `[0x5d6b27,
//!   0x5d6b45)`, the y component `[0x5d6b56, 0x5d6b5f)` and the loop with all three components
//!   `[0x5d6aca, 0x5d6b74)`;
//! - its decal box `0x5d6fe0`, `[0x5d6ff4, 0x5d70be)`.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either side,
//! to its random inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::dynobj_fp::*;
use common::*;

#[test]
fn dy_dynobj_visual_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x300);
    let fields = e.alloc(0x40);
    let rec = e.alloc(0x40);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    e.write_u32(obj + 0x110, fields as u32);
    let mut s = 0x5d52_f000u32;

    // Arm A divides by the model bound at `[ebp-4]`; arm B by the record's radius at `[ecx+0x10]`.
    let case_a = |e: &mut Emu, num: f32, den: f32, ctx: &str| {
        e.write_u32(fields + 0x10, num.to_bits());
        e.write_u32(ebp - 4, den.to_bits());
        e.set_reg(RegisterX86::ESI, obj);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5d539b, 0x5d53d8);
        let bin = e.read_f32(obj + 0x194);
        assert_f32(
            bin,
            dynobj_visual_scale(num, den),
            &format!("{ctx} (arm A) {num} / {den}"),
        );
    };
    let case_b = |e: &mut Emu, num: f32, den: f32, ctx: &str| {
        e.write_u32(fields + 0x10, num.to_bits());
        e.write_u32(rec + 0x10, den.to_bits());
        e.set_reg(RegisterX86::ESI, obj);
        e.set_reg(RegisterX86::ECX, rec);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5d53c6, 0x5d53d8);
        let bin = e.read_f32(obj + 0x194);
        assert_f32(
            bin,
            dynobj_visual_scale(num, den),
            &format!("{ctx} (arm B) {num} / {den}"),
        );
    };

    case_a(&mut e, 10.0, 4.0, "10/4");
    case_a(&mut e, 1.0, 3.0, "1/3");
    case_a(&mut e, 5.0, 0.0, "5/0");
    case_a(&mut e, 0.0, 0.0, "0/0");
    case_b(&mut e, 10.0, 4.0, "10/4");
    case_b(&mut e, 7.0, 0.0, "7/0");
    let sp = specials(&[1.0]);
    for &num in &sp {
        for &den in &sp {
            case_a(&mut e, num, den, "special");
            case_b(&mut e, num, den, "special");
        }
    }
    for i in 0..4000u32 {
        let (num, den) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -200.0, 200.0), frand(&mut s, -50.0, 50.0))
        };
        if i % 2 == 0 {
            case_a(&mut e, num, den, &format!("fuzz{i}"));
        } else {
            case_b(&mut e, num, den, &format!("fuzz{i}"));
        }
    }
}

#[test]
fn dy_dynobj_visual_scale_gates_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x40);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5d52_f001u32;
    let mut seen = std::collections::BTreeSet::new();

    // The constant the bound is compared with is a double in the image.
    let image = f64::from_le_bytes(e.read(0x801608, 8).try_into().unwrap());
    assert_eq!(image.to_bits(), BOUND_MIN.to_bits());

    let mut bound_case = |e: &mut Emu, bound: f32| {
        e.write_u32(ebp - 4, bound.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5d538b, 0x5d5396);
        let bin = ah(e) & 0x41 == 0;
        assert_eq!(
            bin,
            dynobj_uses_model_bound(bound),
            "bound {bound}: ah={:#04x}",
            ah(e)
        );
        seen.insert(bin);
    };
    // No f32 equals 0.001 as a double: the nearest above and below bracket it.
    let near = 0.001f32;
    bound_case(&mut e, near);
    bound_case(&mut e, next_down(near));
    bound_case(&mut e, next_up(near));
    for &b in &specials(&[0.001]) {
        bound_case(&mut e, b);
    }
    for _ in 0..1500 {
        let b = if next(&mut s).is_multiple_of(2) {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -0.002, 0.002)
        };
        bound_case(&mut e, b);
    }
    assert!(seen.contains(&true) && seen.contains(&false));

    let mut seen = std::collections::BTreeSet::new();
    let mut radius_case = |e: &mut Emu, radius: f32| {
        e.write_u32(rec + 0x10, radius.to_bits());
        e.set_reg(RegisterX86::ECX, rec);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5d53b6, 0x5d53c1);
        let bin = ah(e) & 0x41 == 0;
        assert_eq!(
            bin,
            dynobj_effect_radius_usable(radius),
            "radius {radius}: ah={:#04x}",
            ah(e)
        );
        seen.insert(bin);
    };
    for &r in &specials(&[]) {
        radius_case(&mut e, r);
    }
    for _ in 0..1500 {
        radius_case(&mut e, frand_bits(&mut s));
    }
    assert!(seen.contains(&true) && seen.contains(&false));
}

#[test]
fn dy_dynobj_light_bias_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let effect = e.alloc(0x80);
    let this = e.alloc(0x200);
    let fields = e.alloc(0x40);
    e.write_u32(this + 0x110, fields as u32);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5d55_c000u32;

    // `eax = 0` selects `[esi+0x4c]`; the interleaved loads read `edi` and the stack.
    let case = |e: &mut Emu, ef: f32, ctx: &str| {
        e.write_u32(effect + 0x4c, ef.to_bits());
        e.set_reg(RegisterX86::EAX, 0);
        e.set_reg(RegisterX86::ESI, effect);
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5d5650, 0x5d5673);
        let bias = e.read_f32(0x8029cc);
        assert_eq!(bias.to_bits(), LIGHT_BIAS.to_bits());
        let bin = e.read_f32(ebp - 4);
        assert_f32(bin, dynobj_light_bias(ef, bias), &format!("{ctx}: {ef}"));
    };

    case(&mut e, 1.0, "1.0");
    case(&mut e, 0.0, "0.0");
    case(&mut e, 123.456, "123.456");
    case(&mut e, -7.5, "negative");
    case(&mut e, -512.0, "cancels the bias to +0");
    // Sums that fall exactly between two f32 and round to even at the store.
    case(&mut e, 2.0f32.powi(-15), "half an ulp of 512 above");
    case(&mut e, 3.0 * 2.0f32.powi(-15), "three halves of an ulp");
    for &v in &specials(&[512.0, 2.0f32.powi(-14)]) {
        case(&mut e, v, "special");
    }
    for i in 0..4000u32 {
        let ef = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1e4, 1e4)
        };
        case(&mut e, ef, &format!("fuzz{i}"));
    }
}

#[test]
fn dy_corpse_submerged_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x40);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5d65_4000u32;
    let mut seen_sub = false;
    let mut seen_dry = false;

    let mut case = |e: &mut Emu, liquid_surface_z: f32, pos_z: f32, ctx: &str| {
        e.write_u32(ebp - 4, liquid_surface_z.to_bits());
        e.write_u32(this + 8, pos_z.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5d6584, 0x5d6592);
        let m = ah(e) & 0x41;
        let bin_sub = m == 0;
        if bin_sub {
            seen_sub = true;
        } else {
            seen_dry = true;
        }
        let thr = e.read_f32(0x80abfc);
        assert_eq!(thr.to_bits(), SUBMERGE_DEPTH.to_bits());
        assert_eq!(
            bin_sub,
            corpse_submerged(liquid_surface_z, pos_z, thr),
            "{ctx}: ah&0x41={m:#04x} surface={liquid_surface_z} z={pos_z}"
        );
    };

    case(&mut e, 1e9, 0.0, "deep");
    case(&mut e, -1e9, 0.0, "far below");
    case(&mut e, 0.0, 0.0, "zero depth");
    case(&mut e, f32::NAN, 0.0, "nan");
    case(&mut e, SUBMERGE_DEPTH, 0.0, "exactly the threshold");
    case(&mut e, next_up(SUBMERGE_DEPTH), 0.0, "a step above");
    case(&mut e, next_down(SUBMERGE_DEPTH), 0.0, "a step below");
    case(&mut e, 1.0 + SUBMERGE_DEPTH, 1.0, "the threshold, offset");
    let thr = SUBMERGE_DEPTH;
    for &z in &specials(&[thr]) {
        for &p in &[0.0, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY] {
            case(&mut e, z, p, "special");
        }
    }
    for i in 0..5000u32 {
        let scale = match i % 3 {
            0 => 1.0,
            1 => 1e4,
            _ => 1e6,
        };
        let liquid_surface_z = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -scale, scale)
        };
        let pos_z = frand(&mut s, -scale, scale);
        case(&mut e, liquid_surface_z, pos_z, &format!("fuzz{i}"));
    }
    assert!(seen_sub, "submerged never seen");
    assert!(seen_dry, "dry never seen");
}

#[test]
fn dy_corpse_half_render_radius_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let bounds = e.alloc(0x20);
    let this = e.alloc(0x300);
    let mut s = 0x5d68_5000u32;

    let case = |e: &mut Emu, mb: f32, ctx: &str| {
        e.write_u32(bounds + 0xc, mb.to_bits());
        e.set_reg(RegisterX86::EAX, bounds);
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5d689b, 0x5d68aa);
        let bin = e.read_f32(this + 0x2b0);
        assert_f32(bin, corpse_half_render_radius(mb), &format!("{ctx}: {mb}"));
    };

    case(&mut e, 4.0, "4 * 0.5");
    case(&mut e, 1.0, "1 * 0.5");
    case(&mut e, -3.0, "negative");
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
    for i in 0..4000u32 {
        let mb = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1e5, 1e5)
        };
        case(&mut e, mb, &format!("fuzz{i}"));
    }
}

#[test]
fn dy_corpse_center_inv_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5d68_c000u32;

    // `[ebp-8]` is set to 0 by the slice, so `fild qword` reads the count as unsigned.
    let case = |e: &mut Emu, count: u32, ctx: &str| {
        e.set_reg(RegisterX86::EDI, u64::from(count));
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5d6b27, 0x5d6b45);
        let bin = e.read_f32(ebp - 4);
        assert_f32(bin, corpse_center_inv(count), &format!("{ctx}: {count}"));
    };

    // The code reaches this block only with a count of at least 1.
    for c in [
        1,
        2,
        3,
        7,
        100,
        1 << 24,
        (1 << 24) + 1,
        0x7fff_ffff,
        0x8000_0000,
        u32::MAX,
    ] {
        case(&mut e, c, "edge");
    }
    for i in 0..4000u32 {
        let count = 1 + (next(&mut s) >> 8) % 4096;
        case(&mut e, count, &format!("fuzz{i}"));
    }
}

#[test]
fn dy_corpse_center_component_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5d68_c100u32;

    let case = |e: &mut Emu, inv: f32, sum: f32, ctx: &str| {
        e.write_u32(ebp - 4, inv.to_bits());
        e.write_u32(ebp - 0x18, sum.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5d6b56, 0x5d6b5f);
        let bin = e.read_f32(ebp - 0xc);
        assert_f32(
            bin,
            corpse_center_component(inv, sum),
            &format!("{ctx}: {inv} * {sum}"),
        );
    };

    case(&mut e, 0.5, 10.0, "0.5 * 10");
    case(&mut e, 0.333_333_34, 7.0, "1/3 * 7");
    let sp = specials(&[]);
    for &inv in &sp {
        for &sum in &sp {
            case(&mut e, inv, sum, "special");
        }
    }
    for i in 0..4000u32 {
        let inv = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            let c = 1 + (next(&mut s) >> 8) % 4096;
            (1.0f64 / f64::from(c)) as f32
        };
        let sum = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1e6, 1e6)
        };
        case(&mut e, inv, sum, &format!("fuzz{i}"));
    }
}

#[test]
fn dy_corpse_center_x_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x300);
    let mut s = 0x5d68_c200u32;

    const BONE_PTR_GLOBAL: u64 = 0x00c4_bfac;
    const STRIDE: u64 = 0x34;
    let bones = e.alloc(STRIDE * 64);

    // The loop reads each bone's `[x, y, z]` at `STRIDE`, from the array at `[0xc4bfac]`, for the
    // count in `eax`; the centre is stored to `[esi+0x2a0]`. At least one bone must be added, or
    // the store is skipped and the run leaves the slice.
    let case = |e: &mut Emu, b: &[[f32; 3]], ctx: &str| {
        for (i, v) in b.iter().enumerate() {
            let base = bones + STRIDE * i as u64;
            e.write_u32(base, v[0].to_bits());
            e.write_u32(base + 4, v[1].to_bits());
            e.write_u32(base + 8, v[2].to_bits());
        }
        e.write_u32(BONE_PTR_GLOBAL, bones as u32);
        e.set_reg(RegisterX86::EAX, b.len() as u64);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5d6aca, 0x5d6b74);
        let bin = [
            e.read_f32(this + 0x2a0),
            e.read_f32(this + 0x2a4),
            e.read_f32(this + 0x2a8),
        ];
        let ours = corpse_attach_center(b).expect("a bone is added");
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} [{k}] of {b:?}"));
        }
    };

    // The sum 0.2 + 0.2 + 0.2 is not exact in f64, and rounding it to f32 first moves the result.
    case(
        &mut e,
        &[[0.2, 0.2, 1.0], [0.2, 0.2, 1.0], [0.2, 0.2, 1.0]],
        "thirds",
    );
    case(&mut e, &[[1.0, 2.0, 3.0]], "one bone");
    case(&mut e, &[[1.0, 2.0, 3.0], [2.0, 3.0, 4.0]], "two bones");
    case(
        &mut e,
        &[[1e7, 1e7, 1.0], [1.0, 1.0, 1.0], [-1e7, -1e7, 1.0]],
        "cancelling sums",
    );
    case(
        &mut e,
        &[[100.0, 100.0, -1.0], [1.0, 1.0, 1.0], [100.0, 100.0, -1.0]],
        "bones below z = 0 skipped",
    );
    // The gate on z: above 0 adds, below 0 and exactly 0 do not, and NaN does.
    for z in specials(&[]) {
        case(
            &mut e,
            &[[5.0, 6.0, 1.0], [7.0, 8.0, z]],
            &format!("second z {z}"),
        );
        case(
            &mut e,
            &[[9.0, 10.0, z], [7.0, 8.0, 1.0]],
            &format!("first z {z}"),
        );
    }
    for i in 0..2000u32 {
        let total = 1 + (next(&mut s) >> 8) % 24;
        let mut list: Vec<[f32; 3]> = (0..total)
            .map(|_| {
                let mut v = [0.0f32; 3];
                for c in v.iter_mut().take(2) {
                    *c = if next(&mut s) >> 31 == 1 {
                        f32::from_bits(next(&mut s))
                    } else {
                        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
                        ((m - 0.5) * 2.0e3) as f32
                    };
                }
                v[2] = if (next(&mut s) >> 8).is_multiple_of(4) {
                    -1.0
                } else {
                    frand(&mut s, 0.0, 1e3)
                };
                v
            })
            .collect();
        if !list.iter().any(|b| corpse_center_accepts(b[2])) {
            list[0][2] = 1.0;
        }
        case(&mut e, &list, &format!("fuzz{i}"));
    }
}

#[test]
fn dy_corpse_decal_box_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x300);
    let fields = e.alloc(0x40);
    e.write_u32(this + 8, fields as u32);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5d6f_e000u32;

    let case = |e: &mut Emu, pos: [f32; 3], radius: f32, scale: f32, ctx: &str| {
        e.write_u32(ebp - 0xc, pos[0].to_bits());
        e.write_u32(ebp - 8, pos[1].to_bits());
        e.write_u32(ebp - 4, pos[2].to_bits());
        e.write_u32(fields + 0x10, radius.to_bits());
        e.write_u32(this + 0x2b0, scale.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5d6ff4, 0x5d70be);
        let bin_hi = [
            e.read_f32(ebp - 0x24),
            e.read_f32(ebp - 0x20),
            e.read_f32(ebp - 0x1c),
        ];
        let bin_lo = [
            e.read_f32(ebp - 0x18),
            e.read_f32(ebp - 0x14),
            e.read_f32(ebp - 0x10),
        ];
        let s_ext = corpse_decal_half_extent(radius, scale);
        let (our_hi, our_lo) = corpse_decal_box(pos, s_ext);
        for k in 0..3 {
            assert_f32(bin_hi[k], our_hi[k], &format!("{ctx} hi[{k}]"));
            assert_f32(bin_lo[k], our_lo[k], &format!("{ctx} lo[{k}]"));
        }
    };

    case(&mut e, [10.0, 20.0, 30.0], 2.0, 1.5, "simple");
    case(&mut e, [0.0; 3], 0.0, 0.0, "zeros");
    case(&mut e, [-100.0, 5.0, 0.0], 1.0, 1.0, "mixed");
    // A half extent that is not exact in f32, so the 53-bit value and its f32 rounding differ.
    case(&mut e, [1.0, 2.0, 3.0], 0.1, 0.3, "extent not exact in f32");
    let sp = specials(&[]);
    for &r in &sp {
        for &sc in &sp {
            case(&mut e, [1.0, -1.0, 0.5], r, sc, "special extent");
        }
    }
    for &x in &sp {
        for &y in &sp {
            case(&mut e, [x, y, 2.0], 1.5, 0.5, "special position");
            case(
                &mut e,
                [y, 0.0, x],
                1.5,
                0.5,
                "special position, other axes",
            );
        }
    }
    for i in 0..4000u32 {
        let pos = if i % 4 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
            ]
        };
        let radius = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 50.0)
        };
        let scale = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.1, 5.0)
        };
        case(&mut e, pos, radius, scale, &format!("fuzz{i}"));
    }
}
