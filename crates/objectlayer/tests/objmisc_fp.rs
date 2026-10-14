//! The kernels of eight object functions in `src/objmisc_fp.rs` against the exe. Each runs the
//! slice it covers, with the frame, registers and fields it reads set directly, reading the store
//! it ends in, `eax` after `__ftol`, or the status byte `fnstsw ax` leaves:
//! - the game object placement `0x5f7910`: the extent `[0x5f79ad, 0x5f79c4)` and the corner sums
//!   `[0x5f79df, 0x5f7a01)`;
//! - the game object use test `0x5f3130`: the distance and range, `[0x5f3346, 0x5f3377)`;
//! - `CGUnit_C::OnHealthFieldChanged` (`0x6046f0`): the health ratio, `[0x604726, 0x604737)`;
//! - `CGUnit_C::Mount` (`0x607a00`): the display scales `[0x607ae0, 0x607b0e)` and the ratio
//!   `[0x607b49, 0x607b8c)`;
//! - the object update `0x614a90`: the cubic ease and lerp `[0x614b3b, 0x614b9c)` and the phase
//!   angle `[0x614bdb, 0x614bf1)`;
//! - `PlayBowPull` (`0x624cc0`): the frame index `[0x624d75, 0x624d80)` and the speed
//!   `[0x624e12, 0x624e26)`;
//! - the item repair cost `0x5da330`: `[0x5da4ab, 0x5da4d4)`;
//! - `LogExhaustionXpGain` (`0x637ac0`): the two rest gates `[0x637b3e, 0x637b4b)` and
//!   `[0x637b54, 0x637b67)` and the count ratio `[0x637b70, 0x637b7b)`.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either
//! side, to its random inputs; the `__ftol` legs add the integers' extremes and quotients past
//! `i32` and `i64`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::objmisc_fp::*;
use common::*;

/// A fuzzed i32 in `[lo, hi)` from the LCG high bits.
fn irand(s: &mut u32, lo: i32, hi: i32) -> i32 {
    let span = (hi - lo) as i64;
    lo + ((next(s) >> 8) as i64 % span) as i32
}

/// The x86 parity flag the CPU computes after `test ah,IMM` (`PF = 1` ⟺ even number of set bits in
/// the low byte of the result). The `jp`/`jnp` after our sliced `test ah,IMM` branches on exactly
/// this: `jp` taken ⟺ `parity_even == true`; `jnp` taken ⟺ `parity_even == false`.
fn parity_even(masked: u8) -> bool {
    masked.count_ones().is_multiple_of(2)
}

const ANIMATE: u64 = 0x5f7910;
const CANUSE: u64 = 0x5f3130;
const LOW_HP: u64 = 0x6046f0;
const MOUNT: u64 = 0x607a00;
const CASTBAR: u64 = 0x624cc0;
const RANDPROP: u64 = 0x5da330;

#[test]
fn go_animate_bounds_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x400);
    let blk = e.alloc(0x40); // edx: the copied 6-float block ([+0x8]=blk2, [+0x14]=blk5)
    let bbox = e.alloc(0x40); // eax: the 6-float bbox vec (e[0..6])
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x5f79_1000u32;

    // ----- slice 1: the bound-extent subtract -----
    let extent_case = |e: &mut Emu, blk5: f32, blk2: f32, ctx: &str| {
        e.write_u32(blk + 0x14, blk5.to_bits());
        e.write_u32(blk + 0x8, blk2.to_bits());
        e.write_u32(ebp - 4, this as u32); // [ebp-4] = this (reloaded into esi at 0x5f79b3)
        e.set_reg(RegisterX86::EDX, blk);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x5f79ad, 0x5f79c4); // fld;fsub; mov esi; mov eax; test; fstp[esi+0xbc]
        let bin = e.read_f32(this + 0xbc);
        assert_f32(bin, animate_extent(blk5, blk2), ctx);
    };
    extent_case(&mut e, 3.0, 1.0, "extent simple");
    extent_case(&mut e, 0.0, 0.0, "extent zero");
    extent_case(&mut e, -2.5, 4.0, "extent neg");
    extent_case(&mut e, f32::INFINITY, f32::INFINITY, "extent inf-inf");
    for i in 0..3000 {
        let (a, b) = if i % 2 == 0 {
            (frand(&mut s, -1e5, 1e5), frand(&mut s, -1e5, 1e5))
        } else {
            (frand_bits(&mut s), frand_bits(&mut s))
        };
        extent_case(&mut e, a, b, &format!("extent fuzz{i}"));
    }

    // ----- slice 2: the three corner-sums -----
    let sums_case = |e: &mut Emu, ev: [f32; 6], ctx: &str| {
        for (k, v) in ev.iter().enumerate() {
            e.write_u32(bbox + 4 * k as u64, v.to_bits());
        }
        e.set_reg(RegisterX86::EAX, bbox);
        e.set_reg(RegisterX86::EBP, ebp);
        // 5f79e5 does `sub esp,0xc`; the slice starts BEFORE it, so leave headroom below ESP.
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        let esp_at_call = ebp - 0x40 - 0xc; // ESP after the `sub esp,0xc`
        e.run(0x5f79df, 0x5f7a01); // the three fld/fadd/fstp into [esp+8],[esp+4],[esp]
        let bin = [
            e.read_f32(esp_at_call),     // arg0 = e[0]+e[3]
            e.read_f32(esp_at_call + 4), // arg1 = e[4]+e[1]
            e.read_f32(esp_at_call + 8), // arg2 = e[5]+e[2]
        ];
        let ours = animate_corner_sums(ev);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} [{k}]"));
        }
    };
    sums_case(&mut e, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], "sums simple");
    sums_case(&mut e, [0.0; 6], "sums zero");
    for i in 0..3000 {
        let f = |s: &mut u32| {
            if next(s) >> 8 & 3 == 0 {
                frand_bits(s)
            } else {
                frand(s, -300.0, 300.0)
            }
        };
        let ev = [
            f(&mut s),
            f(&mut s),
            f(&mut s),
            f(&mut s),
            f(&mut s),
            f(&mut s),
        ];
        sums_case(&mut e, ev, &format!("sums fuzz{i}"));
    }
    let _ = ANIMATE;
    for &v in &specials(&[]) {
        extent_case(&mut e, v, 1.0, "special blk5");
        extent_case(&mut e, 1.0, v, "special blk2");
        for k in 0..6 {
            let mut ev = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
            ev[k] = v;
            sums_case(&mut e, ev, "special sum");
        }
    }
}

const CANUSE_MASK: u8 = 0x5;

#[test]
fn go_canuse_dist2_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let avec = e.alloc(0x20);
    let bvec = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5f31_3000u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], radius: f32, ctx: &str| {
        for k in 0..3 {
            e.write_u32(avec + 4 * k as u64, a[k].to_bits());
            e.write_u32(bvec + 4 * k as u64, b[k].to_bits());
        }
        e.write_u32(ebp - 4, radius.to_bits());
        e.set_reg(RegisterX86::EAX, avec);
        e.set_reg(RegisterX86::ESI, bvec);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x5f3346, 0x5f3377); // dist² + r² compute + fcompp + fnstsw + test ah,5
        let bin_ah = ah(e) & CANUSE_MASK;
        let bin_in_range = parity_even(bin_ah); // jp taken (→ success/in range) ⟺ parity even
        assert_eq!(
            bin_in_range,
            canuse_in_range(a, b, radius),
            "{ctx}: ah&5={bin_ah:#04x}"
        );
        seen.insert(bin_ah);
    };

    // dist² < r² (in), > r² (out), == r² (in, C0=0), NaN (in).
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        5.0,
        "coincident → in",
    );
    case(&mut e, [100.0, 0.0, 0.0], [0.0, 0.0, 0.0], 5.0, "far → out");
    case(
        &mut e,
        [5.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        5.0,
        "exactly r → in (==)",
    );
    case(
        &mut e,
        [f32::NAN, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        5.0,
        "nan → in",
    );
    for i in 0..6000 {
        // Mix near-boundary radii so both arms fire densely.
        let radius = if (next(&mut s) >> 8).is_multiple_of(2) {
            frand(&mut s, 0.0, 10.0)
        } else {
            frand(&mut s, 0.0, 50.0)
        };
        let scale = 12.0;
        let a = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        let b = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        case(&mut e, a, b, radius, &format!("fuzz{i}"));
    }
    // `dist^2 == radius^2` (3-4-5 against 5) and a step either side; NaN in either operand is in
    // range; then the specials.
    for x in [3.0f32, next_up(3.0), next_down(3.0)] {
        case(&mut e, [x, 4.0, 0.0], [0.0; 3], 5.0, "equal");
    }
    for r in [next_up(5.0), next_down(5.0)] {
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], r, "radius step");
    }
    for &v in &specials(&[5.0]) {
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], v, "special radius");
        for k in 0..3 {
            let mut a = [3.0f32, 4.0, 0.0];
            a[k] = v;
            case(&mut e, a, [0.0; 3], 5.0, "special a");
            let mut b = [0.0f32; 3];
            b[k] = v;
            case(&mut e, [3.0, 4.0, 0.0], b, 5.0, "special b");
        }
    }
    assert!(seen.contains(&0x00), "in-range (ah&5==0) never seen");
    assert!(seen.contains(&0x01), "out-of-range (C0) never seen");
    let _ = CANUSE;
}

const LOW_HP_MASK: u8 = 0x5;

#[test]
fn unit_low_health_ratio_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let block = e.alloc(0x80); // eax: the UF block
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x6046_f000u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, health: i32, maxhealth: i32, ctx: &str| {
        e.write_u32(block + 0x40, health as u32);
        e.write_u32(block + 0x58, maxhealth as u32);
        e.set_reg(RegisterX86::EAX, block);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x604726, 0x604737); // fild;fidiv;fcomp 0.2;fnstsw;test ah,5
        let bin_ah = ah(e) & LOW_HP_MASK;
        let bin_low = !parity_even(bin_ah); // jp not taken (→ LOW-hp) ⟺ parity odd ⟺ ratio<0.2
        assert_eq!(
            bin_low,
            low_health(health, maxhealth),
            "{ctx}: ah&5={bin_ah:#04x}"
        );
        seen.insert(bin_ah);
    };

    case(&mut e, 100, 100, "full → not low");
    case(&mut e, 10, 100, "10% → low");
    case(&mut e, 20, 100, "exactly 20% → not low (>=)");
    case(&mut e, 19, 100, "19% → low");
    case(&mut e, 50, 0, "maxhp 0 → div→inf → not low");
    for i in 0..3000 {
        let maxhealth = irand(&mut s, 1, 50000);
        // half the time below 20%, half above, so both arms fire densely.
        let health = if (next(&mut s) >> 8).is_multiple_of(2) {
            irand(&mut s, 0, maxhealth / 5 + 1) // ~< 20%
        } else {
            irand(&mut s, 0, maxhealth + 1) // full range
        };
        case(&mut e, health, maxhealth, &format!("fuzz{i}"));
    }
    // The integers' extremes, zero maxima, and a ratio of one fifth, which as f64 is under the
    // f32 0.2.
    let ints = [0, 1, -1, 2, 5, 10, 100, i32::MAX, i32::MIN, 0x4000_0000];
    for &h in &ints {
        for &m in &ints {
            case(&mut e, h, m, "ints");
        }
    }
    assert!(seen.contains(&0x00), "not-low (ah&5==0) never seen");
    assert!(seen.contains(&0x01), "low-hp (C0) never seen");
    let _ = LOW_HP;
}

#[test]
fn unit_mount_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x1000);
    let rec = e.alloc(0x40); // ebx: CreatureDisplayInfo record ([+0x1c],[+0x20])
    let frame = e.alloc(0x80);
    let ebp = frame + 0x60;
    let mut s = 0x607a_0000u32;

    // ----- slice 1: the two ×(1/36) scale products -----
    let scale_case = |e: &mut Emu, r20: f32, r1c: f32, ctx: &str| {
        e.write_u32(rec + 0x20, r20.to_bits());
        e.write_u32(rec + 0x1c, r1c.to_bits());
        e.write_u32(this + 0xcc0, 0); // 607ada int copy target (harmless)
        e.set_reg(RegisterX86::EBX, rec);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        // Run through the second store: [esi+0xcc4] (from edx) lands at 0x607afc, but [esi+0xcc8]
        // (from eax = [ebp-4]) lands at 0x607b08, so stop just before the call at 0x607b0e.
        e.run(0x607ae0, 0x607b0e);
        let bin = [e.read_f32(this + 0xcc4), e.read_f32(this + 0xcc8)];
        let ours = mount_display_scale(r20, r1c);
        for k in 0..2 {
            assert_f32(bin[k], ours[k], &format!("{ctx} scale[{k}]"));
        }
    };
    scale_case(&mut e, 36.0, 72.0, "simple");
    scale_case(&mut e, 0.0, 0.0, "zero");
    scale_case(&mut e, -1.0, 3.5, "neg");
    for i in 0..3000 {
        let f = |s: &mut u32| {
            if next(s) >> 8 & 3 == 0 {
                frand_bits(s)
            } else {
                frand(s, -500.0, 500.0)
            }
        };
        scale_case(&mut e, f(&mut s), f(&mut s), &format!("scale fuzz{i}"));
    }

    // ----- slice 2: the scale ratio this+0x98 / this+0x9c -----
    let ratio_case = |e: &mut Emu, a: f32, b: f32, ctx: &str| {
        e.write_u32(this + 0x98, a.to_bits());
        e.write_u32(this + 0x9c, b.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x607b49, 0x607b8c); // fld[esi+0x98]; fdiv[esi+0x9c]; ...; fst[ebp-0x4c]
        let bin = e.read_f32(ebp - 0x4c);
        assert_f32(bin, mount_scale_ratio(a, b), ctx);
    };
    ratio_case(&mut e, 6.0, 2.0, "ratio simple");
    ratio_case(&mut e, 1.0, 0.0, "ratio div0 → inf");
    ratio_case(&mut e, 0.0, 0.0, "ratio 0/0 → nan");
    ratio_case(&mut e, -7.0, 3.0, "ratio neg");
    for i in 0..3000 {
        let f = |s: &mut u32| {
            if next(s) >> 8 & 3 == 0 {
                frand_bits(s)
            } else {
                frand(s, -200.0, 200.0)
            }
        };
        ratio_case(&mut e, f(&mut s), f(&mut s), &format!("ratio fuzz{i}"));
    }
    let _ = MOUNT;
    for &v in &specials(&[]) {
        scale_case(&mut e, v, 2.0, "special r20");
        scale_case(&mut e, 2.0, v, "special r1c");
        ratio_case(&mut e, v, 2.0, "special a");
        ratio_case(&mut e, 2.0, v, "special b");
    }
}

#[test]
fn obj_anim_interp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x400);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x614a_9000u32;

    // The fraction `elapsed / duration`, the ease and the interpolation of `0x614a90`.
    let interp_case =
        |e: &mut Emu, elapsed: i32, duration: i32, target: f32, prev: f32, ctx: &str| {
            // The slice reads elapsed from [ebp-8] at the `fild`, then OVERWRITES [ebp-0xc] with EAX
            // (0x614b40 `mov [ebp-0xc],eax`) and divides by it — so the divisor (duration) comes
            // from the EAX register (= [esi+0xf0], loaded outside the slice), not a memory write.
            e.write_u32(ebp - 8, elapsed as u32);
            e.write_u32(this + 0xfc, target.to_bits());
            e.write_u32(this + 0xf8, prev.to_bits());
            e.set_reg(RegisterX86::EAX, u64::from(duration as u32)); // fidiv divisor at [ebp-0xc]
            e.set_reg(RegisterX86::ESI, this);
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, ebp - 0x20);
            e.run(0x614b3b, 0x614b9c); // fild;fidiv;clamp;cube;lerp;fstp[esi+0xf4]
            let bin_out = e.read_f32(this + 0xf4);
            let bin_t = e.read_f32(ebp - 4);
            let t = (f64::from(elapsed) / f64::from(duration)) as f32;
            assert_f32(bin_t, t, &format!("{ctx} t"));
            // The cube stays on the stack unrounded; `anim_interp` rounds once, at the store.
            let ours = anim_interp(t, target, prev);
            assert_f32(bin_out, ours, &format!("{ctx} out"));
        };

    interp_case(&mut e, 0, 100, 10.0, 0.0, "t=0 → prev");
    interp_case(&mut e, 100, 100, 10.0, 0.0, "t=1 → target");
    interp_case(&mut e, 50, 100, 10.0, 0.0, "t=0.5 cube");
    interp_case(&mut e, -5, 100, 10.0, 0.0, "t<0 → prev (eased 0)");
    interp_case(&mut e, 200, 100, 10.0, 0.0, "t>1 → target (eased 1)");
    let mut seen_t0 = false;
    let mut seen_t1 = false;
    let mut seen_cube = false;
    for i in 0..3000 {
        let duration = irand(&mut s, 1, 5000);
        // mix t<0, t in (0,1), t>1 so all three arms fire.
        let kind = (next(&mut s) >> 8) % 4;
        let elapsed = match kind {
            0 => irand(&mut s, -duration, 0),           // t<=0
            1 => irand(&mut s, duration, duration * 3), // t>=1
            _ => irand(&mut s, 0, duration),            // t in [0,1)
        };
        let target = frand(&mut s, -100.0, 100.0);
        let prev = frand(&mut s, -100.0, 100.0);
        let t = (f64::from(elapsed) / f64::from(duration)) as f32;
        if t <= 0.0 {
            seen_t0 = true;
        } else if t >= 1.0 {
            seen_t1 = true;
        } else {
            seen_cube = true;
        }
        interp_case(&mut e, elapsed, duration, target, prev, &format!("fuzz{i}"));
    }
    // The fraction at 0 and 1 exactly and the divisor 0 (0/0 is NaN, which takes the cube; the
    // others are infinities that clamp), negative divisors, the extremes, and the specials in the
    // lerp ends.
    for &(el, du) in &[
        (0, 0),
        (5, 0),
        (-5, 0),
        (1, 1),
        (-1, 1),
        (0, -4),
        (3, -4),
        (-3, -4),
        (i32::MAX, 1),
        (i32::MIN, 1),
        (i32::MAX, i32::MAX),
        (1, i32::MAX),
        (i32::MIN, i32::MIN),
        (100, 100),
        (101, 100),
        (99, 100),
    ] {
        interp_case(&mut e, el, du, 10.0, 2.0, "edge");
    }
    for &v in &specials(&[]) {
        interp_case(&mut e, 50, 100, v, 2.0, "special target");
        interp_case(&mut e, 50, 100, 10.0, v, "special prev");
    }
    assert!(seen_t0, "t<=0 (eased 0) arm never fired");
    assert!(seen_t1, "t>=1 (eased 1) arm never fired");
    assert!(seen_cube, "cube arm never fired");

    // ----- the osc phase angle slice (0x614bdb..0x614bf1): angle = (int)elapsed2 * 0.0005 * π -----
    // The slice reads `[esi]` (0x614bde `mov edx,[esi]`) for the vtable ptr (consumed only by the
    // call we stop before); ESI must point to readable memory.
    let angle_case = |e: &mut Emu, elapsed2: i32, ctx: &str| {
        e.write_u32(ebp - 8, elapsed2 as u32);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x614bdb, 0x614bf1); // fild;fmul 0.0005;fmul π;fstp[ebp-8]  (stops before the call)
        let bin = e.read_f32(ebp - 8);
        assert_f32(bin, osc_phase_angle(elapsed2), ctx);
    };
    angle_case(&mut e, 0, "angle 0");
    angle_case(&mut e, 1000, "angle 1000");
    angle_case(&mut e, 1999, "angle 1999");
    for i in 0..2000 {
        let el = irand(&mut s, 0, 2000); // the < 0x7d0 path
        angle_case(&mut e, el, &format!("angle fuzz{i}"));
    }
    for el in [i32::MAX, i32::MIN, -1, 0x7d0, 0x7cf, 0x7ffe_ffff] {
        angle_case(&mut e, el, "angle extremes");
    }
}

#[test]
fn castbar_anim_speed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000); // esi: the speed slice reads [esi+0xd24] (a model handle, unused here)
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x624c_c000u32;

    // ----- slice 1: the frame index `_ftol(num/den)` (run through the real _ftol call) -----
    let idx_case = |e: &mut Emu, num: i64, den: f32, ctx: &str| {
        e.write_u32(ebp - 8, (num as u64 & 0xffff_ffff) as u32); // lo
        e.write_u32(ebp - 4, ((num as u64 >> 32) & 0xffff_ffff) as u32); // hi
        e.write_u32(ebp - 0x24, den.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x624d75, 0x624d80); // fild qword; fdiv dword; call 0x40a2b0 (_ftol)
        let bin = e.get_reg(RegisterX86::EAX) as i32 as i64; // _ftol low dword
        let ours = castbar_frame_index(num, den) as i32 as i64; // compare low 32 bits
        assert_eq!(bin, ours, "{ctx}: bin={bin} ours={ours}");
    };
    idx_case(&mut e, 100, 4.0, "100/4 = 25");
    idx_case(&mut e, 7, 2.0, "7/2 → trunc 3");
    idx_case(&mut e, -7, 2.0, "-7/2 → trunc -3");
    idx_case(&mut e, 5, 0.0, "5/0 → inf → indefinite");
    for i in 0..3000 {
        let num = i64::from(irand(&mut s, 0, 1_000_000));
        let den = if (next(&mut s) >> 8).is_multiple_of(7) {
            0.0
        } else {
            frand(&mut s, 0.01, 240.0)
        };
        idx_case(&mut e, num, den, &format!("idx fuzz{i}"));
    }

    // ----- slice 2: the anim speed (i64 frames)/(int total) → f32 (fidiv, fstp dword) -----
    // [ebp-0x10] = (i64) frames (lo at [ebp-0x10], hi at [ebp-0xc]); [ebp-4] = (int) total.
    // 624e12 fild qword; 624e19 fidiv dword[ebp-4]; 624e23 fstp dword[esp]. Run to read [esp].
    let speed_case = |e: &mut Emu, frames: i64, total: i32, ctx: &str| {
        e.write_u32(ebp - 0x10, (frames as u64 & 0xffff_ffff) as u32);
        e.write_u32(ebp - 0xc, ((frames as u64 >> 32) & 0xffff_ffff) as u32);
        e.write_u32(ebp - 4, total as u32);
        e.set_reg(RegisterX86::ESI, this); // 0x624e1d reads [esi+0xd24]
        e.set_reg(RegisterX86::EBP, ebp);
        let esp = ebp - 0x40;
        e.set_reg(RegisterX86::ESP, esp);
        // The slice pushes args (624e15 push 1; 624e17 push 1; 624e1c push ecx) before the fstp[esp].
        // Run 0x624e12..0x624e26 covers fild;push;push;fidiv;push;fstp[esp];push. Read [esp].
        e.run(0x624e12, 0x624e26);
        let cur_esp = e.get_reg(RegisterX86::ESP);
        let bin = e.read_f32(cur_esp); // the fstp dword[esp] target (esp after the 3 pushes)
        assert_f32(bin, castbar_anim_speed(frames, total), ctx);
    };
    speed_case(&mut e, 60, 2, "60/2 = 30.0");
    speed_case(&mut e, 7, 2, "7/2 = 3.5");
    speed_case(&mut e, -9, 4, "-9/4 = -2.25");
    speed_case(&mut e, 5, 1, "5/1 = 5.0");
    for i in 0..3000 {
        let frames = i64::from(irand(&mut s, -100000, 100000));
        let total = {
            let t = irand(&mut s, -5000, 5000);
            if t == 0 {
                1
            } else {
                t
            }
        };
        speed_case(&mut e, frames, total, &format!("speed fuzz{i}"));
    }
    let _ = CASTBAR;
    // Counts past `i32` keep the low dword, a zero or negative denominator, past `i64` gives 0.
    for &(n, d) in &[
        (1i64 << 40, 1.0f32),
        (-(1i64 << 40), 1.0),
        (i64::MAX, 1.0),
        (i64::MIN, 1.0),
        (i64::MAX, 1.0e-30),
        (1, 0.0),
        (0, 0.0),
        (-1, 0.0),
        (7, -2.0),
        (1 << 33, 3.0),
    ] {
        idx_case(&mut e, n, d, "extremes");
    }
    for &v in &specials(&[]) {
        idx_case(&mut e, 100, v, "special den");
    }
    for &(f, t) in &[
        (0i64, 0i32),
        (1, 0),
        (-1, 0),
        (1 << 40, 3),
        (i64::MAX, 1),
        (i64::MIN, 1),
        (5, i32::MIN),
        (5, i32::MAX),
    ] {
        speed_case(&mut e, f, t, "speed extremes");
    }
}

// The repair cost of `0x5da330`, `[0x5da4ab, 0x5da4d4)` through the real `__ftol`: `n` is the
// durability lost at `[ebp-8]`, `factor` the quality multiplier at `[ecx+4]` and `classval` the
// cost column at `[ebp-4]`; `eax` is the truncated result.

#[test]
fn item_randprop_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x40); // ecx: DBC record, [+4]=factor
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5da3_3000u32;

    let case = |e: &mut Emu, n: i32, factor: f32, classval: i32, ctx: &str| {
        e.write_u32(rec + 4, factor.to_bits());
        e.write_u32(ebp - 8, n as u32);
        e.write_u32(ebp - 4, classval as u32);
        e.set_reg(RegisterX86::ECX, rec);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x5da4ab, 0x5da4d4); // fild;fmul;fimul;fcom;branch;±0.5;call _ftol
        let bin = e.get_reg(RegisterX86::EAX) as i32 as i64;
        let ours = random_prop_delta(n, factor, classval) as i32 as i64;
        assert_eq!(bin, ours, "{ctx}: bin={bin} ours={ours}");
    };

    case(&mut e, 10, 1.5, 2, "pos round");
    case(&mut e, -10, 1.5, 2, "neg round");
    case(&mut e, 0, 1.5, 2, "zero n");
    case(&mut e, 3, 0.5, 1, "1.5 → +0.5 → 2");
    case(&mut e, -3, 0.5, 1, "-1.5 → -0.5 → -2");
    case(&mut e, 7, 0.0, 5, "factor 0 → val 0 → 0");
    let mut seen_pos = false;
    let mut seen_neg = false;
    let mut seen_zero = false;
    for i in 0..3000 {
        let n = irand(&mut s, -500, 500);
        let factor = if (next(&mut s) >> 8).is_multiple_of(5) {
            0.0
        } else {
            frand(&mut s, -10.0, 10.0)
        };
        let classval = irand(&mut s, -20, 20);
        let val = f64::from(n) * f64::from(factor) * f64::from(classval);
        if val > 0.0 {
            seen_pos = true;
        } else if val < 0.0 {
            seen_neg = true;
        } else {
            seen_zero = true;
        }
        case(&mut e, n, factor, classval, &format!("fuzz{i}"));
    }
    // A product of exactly 0 from a negative factor (-0), results past `i32`, the integers'
    // extremes, and the specials as the factor.
    for &(n, f, c) in &[
        (0, -1.0f32, 5),
        (3, -0.0, 4),
        (-3, 0.0, 4),
        (0, 0.0, 0),
        (i32::MAX, 2.0, 2),
        (i32::MIN, 2.0, 2),
        (i32::MAX, 1.0e10, i32::MAX),
        (1, 0.5, 1),
        (1, 1.5, 1),
        (-1, 1.5, 1),
        (1, 0.5, -1),
    ] {
        case(&mut e, n, f, c, "edges");
    }
    for &v in &specials(&[0.5]) {
        case(&mut e, 7, v, 3, "special factor");
    }
    assert!(seen_pos, "+0.5 (positive) arm never fired");
    assert!(seen_neg, "-0.5 (negative) arm never fired");
    assert!(seen_zero, "zero-product case never fired");
    let _ = RANDPROP;
}

const REST_MASK: u8 = 0x5;

#[test]
fn xpgain_rest_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x40); // ecx: rest record, [+8]=mult
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x637a_c000u32;

    // gate 1: |mult| < eps?  (ah&5 odd → early-out)
    let g1 = |e: &mut Emu, mult: f32| -> bool {
        e.write_u32(rec + 8, mult.to_bits());
        e.set_reg(RegisterX86::ECX, rec);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x637b3e, 0x637b4b); // fld;fabs;fcomp eps;fnstsw  (stops at the test ah,5)
        let a = ah(e) & REST_MASK;
        !parity_even(a) // jnp taken (→ early-out) ⟺ parity odd ⟺ |mult|<eps (near-zero)
    };
    // gate 2: |mult-1| < eps?
    let g2 = |e: &mut Emu, mult: f32| -> bool {
        e.write_u32(rec + 8, mult.to_bits());
        e.set_reg(RegisterX86::ECX, rec);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x637b54, 0x637b67); // fld;fsub 1.0;fabs;fcomp eps;fnstsw
        let a = ah(e) & REST_MASK;
        !parity_even(a) // jnp taken (→ early-out) ⟺ parity odd ⟺ |mult-1|<eps (near-one)
    };

    let mut seen_near0 = false;
    let mut seen_near1 = false;
    let mut seen_mid = false;
    let mut check = |e: &mut Emu, mult: f32, ctx: &str| {
        let near0 = g1(e, mult);
        // gate 2 only matters if gate 1 didn't early-out, but the binary always evaluates the bytes
        // we sliced; we cross-check the COMBINED predicate against our fn.
        let near1 = g2(e, mult);
        let bin_meaningful = !near0 && !near1;
        assert_eq!(
            bin_meaningful,
            xpgain_rest_meaningful(mult),
            "{ctx}: near0={near0} near1={near1}"
        );
        if near0 {
            seen_near0 = true;
        } else if near1 {
            seen_near1 = true;
        } else {
            seen_mid = true;
        }
    };

    check(&mut e, 0.0, "mult 0 → near0");
    check(&mut e, 1.0, "mult 1 → near1");
    check(&mut e, 0.5, "mult 0.5 → mid");
    check(&mut e, 1.5, "mult 1.5 → mid");
    check(&mut e, f32::NAN, "nan → meaningful (passes both)");
    for i in 0..3000 {
        let kind = (next(&mut s) >> 8) % 4;
        let mult = match kind {
            0 => frand(&mut s, -1e-8, 1e-8),       // near 0
            1 => 1.0 + frand(&mut s, -1e-8, 1e-8), // near 1
            _ => frand(&mut s, -5.0, 5.0),         // mid
        };
        check(&mut e, mult, &format!("fuzz{i}"));
    }
    // `|mult| == 2^-22` and `|mult - 1| == 2^-22` and a step either side, then the specials.
    let eps = REST_EPS;
    for m in [
        eps,
        next_up(eps),
        next_down(eps),
        -eps,
        1.0 + eps,
        next_up(1.0 + eps),
        next_down(1.0 + eps),
        1.0 - eps,
        next_down(1.0 - eps),
    ] {
        check(&mut e, m, "eps edge");
    }
    for &v in &specials(&[eps, 1.0]) {
        check(&mut e, v, "special");
    }
    assert!(seen_near0, "near-0 early-out never fired");
    assert!(seen_near1, "near-1 early-out never fired");
    assert!(seen_mid, "meaningful (mid) arm never fired");

    // ----- the count ratio _ftol(p2/p3) (run through the real _ftol) -----
    let ratio_case = |e: &mut Emu, p2: i32, p3: f32, ctx: &str| {
        e.write_u32(ebp + 0xc, p2 as u32);
        e.write_u32(ebp + 0x10, p3.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x637b70, 0x637b7b); // fild[ebp+0xc]; fdiv[ebp+0x10]; call _ftol
        let bin = e.get_reg(RegisterX86::EAX) as i32 as i64;
        let ours = xpgain_count_ratio(p2, p3) as i32 as i64;
        assert_eq!(bin, ours, "{ctx}: bin={bin} ours={ours}");
    };
    ratio_case(&mut e, 10, 2.0, "10/2 = 5");
    ratio_case(&mut e, 7, 2.0, "7/2 → 3");
    ratio_case(&mut e, -7, 2.0, "-7/2 → -3");
    ratio_case(&mut e, 5, 0.0, "5/0 → inf → indefinite");
    for i in 0..3000 {
        let p2 = irand(&mut s, -1000, 1000);
        let p3 = if (next(&mut s) >> 8).is_multiple_of(7) {
            0.0
        } else {
            frand(&mut s, 0.5, 40.0)
        };
        ratio_case(&mut e, p2, p3, &format!("ratio fuzz{i}"));
    }
}
