//! The squared-distance gates (`systems/ui/distancegates.md`) run from the exe.
//!
//! The arithmetic of a gate is a short chain of `fld`, `fsub`, `fmul` and `faddp` ending in an
//! `fcom` and `fnstsw`, or in an `fstp dword` for the producers. A test runs the exact instruction
//! slice from the exe, with the two positions and the threshold written into the memory and
//! registers the slice reads, and observes one of two things:
//!
//! - a gate: execution halts right after `fnstsw ax`, the test reads the status word out of `eax`
//!   and applies the gate's own `test ah, mask` and jump to its C0, C2 and C3 bits to recover the
//!   branch; the Rust function decides the same branch in `f64`. The inputs straddle the threshold
//!   to the last bit, and a NaN operand exercises the unordered leg;
//! - a producer or an `f32` store: execution halts right after the `fstp dword` and the test reads
//!   the stored `f32` back.
//!
//! Every draw comes off the high bits of an LCG.
//!
//! Host double (`docs/METHOD.md#the-harness`): `dist2_5116e0` runs `fyl2x` at `0x73f9af` and
//! `f2xm1` at `0x744b0a`. The emulator computes each in host `double`, so where its result reaches
//! a compared value the host's math stands in for the x87's, and the test shows the code around it.
#![allow(clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::distance_gates::{
    battlefield_leash_distance, battlefield_leash_guard, bubble_sort_key, candidate_distance_sq,
    check_area_spirit_healer, check_battlefield_list_leash, check_interaction_leash,
    cinematic_end_gate, classify_object_dot, drag_commit_gate, fade_gate, inspect_in_range,
    select_area_spirit_healer, shake_falloff, shot_area_gate, unit_distance_sq, FALLOFF_BASE,
};

// ----- fuzz + assert helpers -------------------------------------------------------------- //

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// f32 in `[lo, hi)` from the LCG high bits.
fn rngf(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}
/// The x87 condition bits `(C0, C2, C3)` out of a `fnstsw ax` value (status-word bits 8/10/14).
fn flags(eax: u32) -> (bool, bool, bool) {
    (
        (eax >> 8) & 1 != 0,
        (eax >> 10) & 1 != 0,
        (eax >> 14) & 1 != 0,
    )
}
fn eqf(bin: u32, mine: f32, ctx: &str) {
    assert_eq!(
        bin,
        mine.to_bits(),
        "{ctx}: binary {:#010x} ({}) vs rust {:#010x} ({mine})",
        bin,
        f32::from_bits(bin),
        mine.to_bits()
    );
}

/// Write a `C3Vector` (`[x,y,z]`) at `base + off`.
fn wvec3(emu: &mut Emu, base: u64, off: u64, v: [f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        emu.write_u32(base + off + 4 * i as u64, c.to_bits());
    }
}

/// A `(p, q)` pair fuzz tuned to straddle a threshold of radius `base` (so `dist² ≈ base²`):
/// random general 3-D cases (all axes vary — exercises every squared term + the sum order), plus
/// axis-aligned ulp-straddle triples around `base` on each axis (the `==`/`C3` and `±ulp` legs),
/// plus NaN cases (the `unordered`/C2 leg).
fn gen_pq(s: &mut u32, n: usize, base: f32) -> Vec<([f32; 3], [f32; 3])> {
    let mut v = Vec::new();
    for _ in 0..n {
        let p = [
            rngf(s, -base, base),
            rngf(s, -base, base),
            rngf(s, -base, base),
        ];
        let d = base * 1.3;
        let q = [
            p[0] + rngf(s, -d, d),
            p[1] + rngf(s, -d, d),
            p[2] + rngf(s, -d, d),
        ];
        v.push((p, q));
    }
    // ulp-straddle on each axis: dist² = (base ± k ulp)², crossing base².
    for axis in 0..3 {
        for k in -6i32..=6 {
            let bb = base.to_bits().wrapping_add(k as u32);
            let d = f32::from_bits(bb);
            let mut q = [0.0f32; 3];
            q[axis] = d;
            v.push(([0.0, 0.0, 0.0], q));
        }
    }
    // NaN operand (unordered).
    for axis in 0..3 {
        let mut q = [1.0f32, 2.0, 3.0];
        q[axis] = f32::NAN;
        v.push(([0.0, 0.0, 0.0], q));
    }
    v
}

// A generic stack region for slices that `push`/`pop` or `fstp [esp]`.
fn stack(emu: &mut Emu) -> u64 {
    emu.alloc(0x400) + 0x200
}

// ===================================================================================== //
//  Gate slices — observe the branch via the status word.                                //
// ===================================================================================== //

#[test]
fn dist2_48edf0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let c30 = emu.read_f32(0x804554);
    let frame = emu.alloc(0x40);
    let ebp = frame + 0x20;
    let (mut fire_t, mut fire_f, mut nan) = (0u32, 0u32, 0u32);
    let mut s = 0x48ed_f000u32;
    for (i, (p, q)) in gen_pq(&mut s, 4000, c30).into_iter().enumerate() {
        // p at [ebp-0x1c/-0x18/-0x14], q at [ebp-0x10/-0xc/-0x8].
        for (j, c) in p.iter().enumerate() {
            emu.write_u32(ebp - 0x1c + 4 * j as u64, c.to_bits());
        }
        for (j, c) in q.iter().enumerate() {
            emu.write_u32(ebp - 0x10 + 4 * j as u64, c.to_bits());
        }
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.run(0x48ee93, 0x48eec7);
        let (c0, c2, _) = flags(emu.get_reg(RegisterX86::EAX) as u32);
        let fire_bin = c0 != c2;
        let fire = shot_area_gate(p, q, c30);
        assert_eq!(fire_bin, fire, "48edf0 case {i} p={p:?} q={q:?}");
        if q.iter().any(|x| x.is_nan()) {
            nan += 1;
        } else if fire {
            fire_t += 1;
        } else {
            fire_f += 1;
        }
    }
    eprintln!("dist2_48edf0: bit-exact (fire={fire_t} no-fire={fire_f} nan={nan})");
    assert!(fire_t > 0 && fire_f > 0 && nan > 0, "48edf0 coverage");
}

#[test]
fn dist2_48f080() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let c30 = emu.read_f32(0x804558);
    let p_buf = emu.alloc(0x20); // p read at [esi+8/+c/+10]
    let q_buf = emu.alloc(0x10);
    let (mut ft, mut ff, mut nan) = (0u32, 0u32, 0u32);
    let mut s = 0x48f0_8000u32;
    for (i, (p, q)) in gen_pq(&mut s, 4000, c30).into_iter().enumerate() {
        wvec3(&mut emu, p_buf, 8, p);
        wvec3(&mut emu, q_buf, 0, q);
        emu.set_reg(RegisterX86::ESI, p_buf);
        emu.set_reg(RegisterX86::EAX, q_buf);
        emu.run(0x48f0cf, 0x48f102);
        let (c0, c2, _) = flags(emu.get_reg(RegisterX86::EAX) as u32);
        let fire = cinematic_end_gate(p, q, c30);
        assert_eq!(c0 != c2, fire, "48f080 case {i} p={p:?} q={q:?}");
        if q.iter().any(|x| x.is_nan()) {
            nan += 1;
        } else if fire {
            ft += 1;
        } else {
            ff += 1;
        }
    }
    eprintln!("dist2_48f080: bit-exact (fire={ft} no-fire={ff} nan={nan})");
    assert!(ft > 0 && ff > 0 && nan > 0, "48f080 coverage");
}

#[test]
fn dist2_4923b0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let c20 = emu.read_f32(0x8044d0);
    let c11 = emu.read_f32(0x804580);
    let thr = (f64::from(c20) * f64::from(c11)) as f32; // f32 store at [ebp-4]
    let base = thr; // dist straddles thr²
    let p_buf = emu.alloc(0x10); // esi
    let q_buf = emu.alloc(0x10); // eax
    let frame = emu.alloc(0x20);
    let ebp = frame + 0x10;
    let sp = stack(&mut emu); // for the `pop esi`
    let (mut ft, mut ff, mut nan) = (0u32, 0u32, 0u32);
    let mut s = 0x4923_b000u32;
    for (i, (p, q)) in gen_pq(&mut s, 4000, base).into_iter().enumerate() {
        wvec3(&mut emu, p_buf, 0, p);
        wvec3(&mut emu, q_buf, 0, q);
        emu.write_u32(ebp - 4, thr.to_bits());
        emu.set_reg(RegisterX86::ESI, p_buf);
        emu.set_reg(RegisterX86::EAX, q_buf);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, sp);
        emu.run(0x492448, 0x492475);
        let (c0, c2, _) = flags(emu.get_reg(RegisterX86::EAX) as u32);
        let fire = check_area_spirit_healer(p, q, c20, c11);
        assert_eq!(c0 != c2, fire, "4923b0 case {i} p={p:?} q={q:?}");
        if q.iter().any(|x| x.is_nan()) {
            nan += 1;
        } else if fire {
            ft += 1;
        } else {
            ff += 1;
        }
    }
    eprintln!("dist2_4923b0: bit-exact thr={thr} (fire={ft} no-fire={ff} nan={nan})");
    assert!(ft > 0 && ff > 0 && nan > 0, "4923b0 coverage");
}

#[test]
fn dist2_4924c0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let c20 = emu.read_f32(0x8044d0);
    let p_buf = emu.alloc(0x10); // edi
    let q_buf = emu.alloc(0x10); // eax
    let (mut ft, mut ff, mut nan) = (0u32, 0u32, 0u32);
    let mut s = 0x4924_c000u32;
    for (i, (p, q)) in gen_pq(&mut s, 4000, c20).into_iter().enumerate() {
        wvec3(&mut emu, p_buf, 0, p);
        wvec3(&mut emu, q_buf, 0, q);
        emu.set_reg(RegisterX86::EDI, p_buf);
        emu.set_reg(RegisterX86::EAX, q_buf);
        emu.run(0x492527, 0x492559);
        let (c0, c2, _) = flags(emu.get_reg(RegisterX86::EAX) as u32);
        let fire = select_area_spirit_healer(p, q, c20);
        assert_eq!(c0 == c2, fire, "4924c0 case {i} p={p:?} q={q:?}");
        if q.iter().any(|x| x.is_nan()) {
            nan += 1;
        } else if fire {
            ft += 1;
        } else {
            ff += 1;
        }
    }
    eprintln!("dist2_4924c0: bit-exact (select={ft} no-select={ff} nan={nan})");
    assert!(ft > 0 && ff > 0 && nan > 0, "4924c0 coverage");
}

#[test]
fn dist2_493230() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let thr = 196.0f32; // the pre-squared runtime .bss value (0xb4e2c0 = (10+4)²)
    let base = thr.sqrt();
    let p_buf = emu.alloc(0x10); // esi
    let q_buf = emu.alloc(0x10); // eax
    let (mut ft, mut ff, mut nan) = (0u32, 0u32, 0u32);
    let mut s = 0x4932_3000u32;
    for (i, (p, q)) in gen_pq(&mut s, 4000, base).into_iter().enumerate() {
        wvec3(&mut emu, p_buf, 0, p);
        wvec3(&mut emu, q_buf, 0, q);
        emu.write_u32(0xb4e2c0, thr.to_bits());
        emu.set_reg(RegisterX86::ESI, p_buf);
        emu.set_reg(RegisterX86::EAX, q_buf);
        emu.run(0x4932d2, 0x4932fc);
        let (c0, _, c3) = flags(emu.get_reg(RegisterX86::EAX) as u32);
        let fire = check_interaction_leash(p, q, thr);
        assert_eq!(!c0 && !c3, fire, "493230 case {i} p={p:?} q={q:?}");
        if q.iter().any(|x| x.is_nan()) {
            nan += 1;
        } else if fire {
            ft += 1;
        } else {
            ff += 1;
        }
    }
    eprintln!("dist2_493230: bit-exact (drop={ft} keep={ff} nan={nan})");
    assert!(ft > 0 && ff > 0 && nan > 0, "493230 coverage");
}

#[test]
fn dist2_4944a0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // The threshold `.bss` `0xb4d918` is a CRT static-init const-fold; run the binary's own
    // initializer `0x48cc80` (`fld [0x804498]=10; fmul; fstp [0xb4d918]`) to PRODUCE it and read
    // it back, so the exe's own bytes compute the value and the test does not feed the same
    // assumed number to both sides.
    emu.call(0x48cc80, 0, 0, &[]);
    let thr = emu.read_f32(0xb4d918);
    assert_eq!(thr, 100.0, "0xb4d918 = 10² (src 0x804498); got {thr}");
    let base = thr.sqrt();
    let p_buf = emu.alloc(0x10); // esi (target)
    let q_buf = emu.alloc(0x10); // eax (active player)
    let (mut ft, mut ff, mut nan) = (0u32, 0u32, 0u32);
    let mut s = 0x4944_a000u32;
    for (i, (p, q)) in gen_pq(&mut s, 4000, base).into_iter().enumerate() {
        wvec3(&mut emu, p_buf, 0, p);
        wvec3(&mut emu, q_buf, 0, q);
        emu.set_reg(RegisterX86::ESI, p_buf);
        emu.set_reg(RegisterX86::EAX, q_buf);
        emu.run(0x494505, 0x494531);
        let (c0, _, c3) = flags(emu.get_reg(RegisterX86::EAX) as u32);
        let within = inspect_in_range(p, q, thr);
        assert_eq!(c0 || c3, within, "4944a0 case {i} p={p:?} q={q:?}");
        if q.iter().any(|x| x.is_nan()) {
            nan += 1;
        } else if within {
            ft += 1;
        } else {
            ff += 1;
        }
    }
    eprintln!("dist2_4944a0: bit-exact (within={ft} out={ff} nan={nan})");
    assert!(ft > 0 && ff > 0 && nan > 0, "4944a0 coverage");
}

#[test]
fn dist2_4aa090() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let eps = emu.read_f32(0x8029d4);
    // `0xb6e854` is filled by a static initializer: run `0x4ac540`
    // (`fld [0x806574]=50/9; fmul; fstp [0xb6e854]`) to produce the threshold (50/9)^2 = 30.864195
    // and read it back, so the test does not feed the same assumed value to both sides.
    emu.call(0x4ac540, 0, 0, &[]);
    let thr = emu.read_f32(0xb6e854);
    assert!(
        (f64::from(thr) - 30.864_195).abs() < 1e-3,
        "0xb6e854 should be (50/9)²≈30.864195 got {thr}"
    );
    let base = thr.sqrt();
    let q_buf = emu.alloc(0x10); // eax (active player)
    let (mut gp, mut gf, mut ft, mut ff, mut nan) = (0u32, 0u32, 0u32, 0u32, 0u32);
    let mut s = 0x4aa0_9000u32;
    // dist cases: vec nonzero (so the magnitude guard passes), q straddles it.
    let mut cases: Vec<([f32; 3], [f32; 3])> = Vec::new();
    for _ in 0..2500 {
        let vec = [
            rngf(&mut s, 5.0, 20.0),
            rngf(&mut s, -20.0, 20.0),
            rngf(&mut s, -20.0, 20.0),
        ];
        let d = base * 1.3;
        let q = [
            vec[0] + rngf(&mut s, -d, d),
            vec[1] + rngf(&mut s, -d, d),
            vec[2] + rngf(&mut s, -d, d),
        ];
        cases.push((vec, q));
    }
    // guard-fail cases: tiny vec (mag < eps) -> overall no fire.
    for _ in 0..200 {
        cases.push(([0.0, 0.0, 0.0], [base, 0.0, 0.0]));
    }
    // NaN.
    for axis in 0..3 {
        let mut q = [1.0f32, 2.0, 3.0];
        q[axis] = f32::NAN;
        cases.push(([3.0, 4.0, 0.0], q));
    }
    for (i, (vec, q)) in cases.into_iter().enumerate() {
        // guard slice: reads b6e870/4/8 and eps. (0xb6e854 already holds the const-fold above.)
        wvec3(&mut emu, 0xb6e870, 0, vec);
        emu.run(0x4aa096, 0x4aa0c8);
        let (gc0, gc2, _) = flags(emu.get_reg(RegisterX86::EAX) as u32);
        let guard_bin = gc0 == gc2;
        assert_eq!(
            guard_bin,
            battlefield_leash_guard(vec, eps),
            "4aa090 guard case {i} vec={vec:?}"
        );
        if guard_bin {
            gp += 1;
        } else {
            gf += 1;
        }
        // dist slice: q via eax, vec from .bss.
        wvec3(&mut emu, q_buf, 0, q);
        emu.set_reg(RegisterX86::EAX, q_buf);
        emu.run(0x4aa0fb, 0x4aa12f);
        let (c0, _, c3) = flags(emu.get_reg(RegisterX86::EAX) as u32);
        let dist_bin = !c0 && !c3;
        assert_eq!(
            dist_bin,
            battlefield_leash_distance(vec, q, thr),
            "4aa090 dist case {i} vec={vec:?} q={q:?}"
        );
        let fire = check_battlefield_list_leash(vec, q, eps, thr);
        assert_eq!(fire, guard_bin && dist_bin, "4aa090 overall case {i}");
        if q.iter().any(|x| x.is_nan()) {
            nan += 1;
        } else if fire {
            ft += 1;
        } else {
            ff += 1;
        }
    }
    eprintln!(
        "dist2_4aa090: bit-exact (guard pass={gp} fail={gf}; reset={ft} keep={ff} nan={nan})"
    );
    assert!(gp > 0 && gf > 0, "4aa090 guard coverage");
    assert!(ft > 0 && ff > 0 && nan > 0, "4aa090 dist coverage");
}

#[test]
fn dist2_4eaa90() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let desc_buf = emu.alloc(0x20); // esi: p at +0xc/+0x10/+0x14, R at +0x18
    let q_buf = emu.alloc(0x10); // eax
    let r = 25.0f32;
    let (mut ft, mut ff, mut nan) = (0u32, 0u32, 0u32);
    let mut s = 0x4eaa_9000u32;
    for (i, (p, q)) in gen_pq(&mut s, 4000, r).into_iter().enumerate() {
        wvec3(&mut emu, desc_buf, 0xc, p);
        emu.write_u32(desc_buf + 0x18, r.to_bits());
        wvec3(&mut emu, q_buf, 0, q);
        emu.set_reg(RegisterX86::ESI, desc_buf);
        emu.set_reg(RegisterX86::EAX, q_buf);
        emu.run(0x4eaae5, 0x4eab15);
        let (c0, c2, _) = flags(emu.get_reg(RegisterX86::EAX) as u32);
        let record = classify_object_dot(p, q, r);
        assert_eq!(c0 == c2, record, "4eaa90 case {i} p={p:?} q={q:?}");
        if q.iter().any(|x| x.is_nan()) {
            nan += 1;
        } else if record {
            ft += 1;
        } else {
            ff += 1;
        }
    }
    eprintln!("dist2_4eaa90: bit-exact (record={ft} reject={ff} nan={nan})");
    assert!(ft > 0 && ff > 0 && nan > 0, "4eaa90 coverage");
}

#[test]
fn dist2_76b320() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // `0xcf0c14` is filled by a static initializer: run `0x76b300`
    // (`fld [0x81c468]=0.01; fmul; fstp [0xcf0c14]`) to produce the threshold 0.01^2 = 9.9999997e-05
    // and read it back. It is a click-to-drag hysteresis in normalized screen coordinates (0 to 1),
    // so the positions are drawn around 0.01.
    emu.call(0x76b300, 0, 0, &[]);
    let thr = emu.read_f32(0xcf0c14);
    assert!(
        (f64::from(thr) - 9.9999997e-5).abs() < 1e-7,
        "0xcf0c14 should be (0.01)²≈9.9999997e-05 got {thr}"
    );
    let base = thr.sqrt();
    let frame = emu.alloc(0x20); // ebp: pt at [ebp-8],[ebp-4]
    let ebp = frame + 0x10;
    let this = emu.alloc(0x120); // esi: anchor at +0x10c,+0x110
    let (mut ft, mut ff, mut nan) = (0u32, 0u32, 0u32);
    let mut s = 0x76b3_2000u32;
    let mut cases: Vec<([f32; 2], [f32; 2])> = Vec::new();
    for _ in 0..3000 {
        let a = [rngf(&mut s, -100.0, 100.0), rngf(&mut s, -100.0, 100.0)];
        let d = base * 1.3;
        let pt = [a[0] + rngf(&mut s, -d, d), a[1] + rngf(&mut s, -d, d)];
        cases.push((pt, a));
    }
    for axis in 0..2 {
        for k in -6i32..=6 {
            let d = f32::from_bits(base.to_bits().wrapping_add(k as u32));
            let mut pt = [0.0f32; 2];
            pt[axis] = d;
            cases.push((pt, [0.0, 0.0]));
        }
    }
    for axis in 0..2 {
        let mut pt = [3.0f32, 4.0];
        pt[axis] = f32::NAN;
        cases.push((pt, [0.0, 0.0]));
    }
    for (i, (pt, anchor)) in cases.into_iter().enumerate() {
        emu.write_u32(ebp - 8, pt[0].to_bits());
        emu.write_u32(ebp - 4, pt[1].to_bits());
        emu.write_u32(this + 0x10c, anchor[0].to_bits());
        emu.write_u32(this + 0x110, anchor[1].to_bits());
        // (0xcf0c14 already holds the const-fold from the initializer run above.)
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESI, this);
        emu.run(0x76b34c, 0x76b370);
        let (c0, _, _) = flags(emu.get_reg(RegisterX86::EAX) as u32);
        let fire = drag_commit_gate(pt, anchor, thr);
        assert_eq!(!c0, fire, "76b320 case {i} pt={pt:?} anchor={anchor:?}");
        if pt.iter().any(|x| x.is_nan()) {
            nan += 1;
        } else if fire {
            ft += 1;
        } else {
            ff += 1;
        }
    }
    eprintln!("dist2_76b320: bit-exact (commit={ft} no-commit={ff} nan={nan})");
    assert!(ft > 0 && ff > 0 && nan > 0, "76b320 coverage");
}

// ===================================================================================== //
//  f32-round / delta + gate (4b0c30): two slices — the f32 deltas, then the compare.    //
// ===================================================================================== //

#[test]
fn dist2_4b0c30() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let c400 = emu.read_f32(0x806798);
    let base = c400.sqrt();
    let q_buf = emu.alloc(0x10); // eax = obj pos
    let frame = emu.alloc(0x40); // ebp: owner at [ebp-0x24..], deltas at [ebp-0xc/-8/-4]
    let ebp = frame + 0x30;
    let (mut ft, mut ff, mut nan) = (0u32, 0u32, 0u32);
    let mut s = 0x4b0c_3000u32;
    for (i, (owner, obj)) in gen_pq(&mut s, 4000, base).into_iter().enumerate() {
        let mine = fade_gate(owner, obj, c400);
        // slice 1: the three f32 deltas (obj - owner).
        wvec3(&mut emu, q_buf, 0, obj);
        emu.write_u32(ebp - 0x24, owner[0].to_bits());
        emu.write_u32(ebp - 0x20, owner[1].to_bits());
        emu.write_u32(ebp - 0x1c, owner[2].to_bits());
        emu.set_reg(RegisterX86::EAX, q_buf);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.run(0x4b0c8f, 0x4b0cab);
        let dx = emu.read_u32(ebp - 0xc);
        let dy = emu.read_u32(ebp - 8);
        let dz = emu.read_u32(ebp - 4);
        eqf(dx, mine.dx, &format!("4b0c30 case {i} dx"));
        eqf(dy, mine.dy, &format!("4b0c30 case {i} dy"));
        eqf(dz, mine.dz, &format!("4b0c30 case {i} dz"));
        // slice 2: the compare on those f32 deltas vs 400.
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.run(0x4b0cb4, 0x4b0cd2);
        let (c0, c2, _) = flags(emu.get_reg(RegisterX86::EAX) as u32);
        assert_eq!(
            c0 == c2,
            mine.fade_out,
            "4b0c30 case {i} fade owner={owner:?} obj={obj:?}"
        );
        if obj.iter().any(|x| x.is_nan()) {
            nan += 1;
        } else if mine.fade_out {
            ft += 1;
        } else {
            ff += 1;
        }
    }
    eprintln!("dist2_4b0c30: bit-exact deltas+gate (fade-out={ft} fade-in={ff} nan={nan})");
    assert!(ft > 0 && ff > 0 && nan > 0, "4b0c30 coverage");
}

// ===================================================================================== //
//  Producers / f32-round value sites — read the stored f32 back.                        //
// ===================================================================================== //

#[test]
fn dist2_494200() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let p_buf = emu.alloc(0x10); // ebx
    let q_buf = emu.alloc(0x10); // eax
    let frame = emu.alloc(0x30); // ebp: dist² stored to [ebp-4]
    let ebp = frame + 0x20;
    let sp = stack(&mut emu); // the slice has a `push eax`
    let mut s = 0x4942_0000u32;
    for (i, (p, q)) in gen_pq(&mut s, 4000, 50.0).into_iter().enumerate() {
        if q.iter().any(|x| x.is_nan()) {
            continue; // f32 value diff: skip NaN inputs (the bit pattern of NaN²-sum is not pinned)
        }
        wvec3(&mut emu, p_buf, 0, p);
        wvec3(&mut emu, q_buf, 0, q);
        emu.set_reg(RegisterX86::EBX, p_buf);
        emu.set_reg(RegisterX86::EAX, q_buf);
        emu.set_reg(RegisterX86::EDI, p_buf); // interleaved `mov edx,[edi]` (next getter's vtable)
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, sp);
        emu.run(0x494275, 0x4942a7);
        eqf(
            emu.read_u32(ebp - 4),
            candidate_distance_sq(p, q),
            &format!("494200 case {i} dist²"),
        );
    }
    eprintln!("dist2_494200: f32 dist² bit-exact (cone/LOS = transcendental/stubbed remainder)");
}

#[test]
fn dist2_493d30() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let a_buf = emu.alloc(0x10); // edi
    let b_buf = emu.alloc(0x10); // eax
    let obj = emu.alloc(0x10); // esi; [esi+8] must be a valid ptr (deref'd by the slice)
    let inner = emu.alloc(0x10);
    emu.write_u32(obj + 8, inner as u32);
    let frame = emu.alloc(0x30); // ebp
    let ebp = frame + 0x20;
    let sp = stack(&mut emu); // esp: fstp [esp] result lands here (after a `push`)
    let mut s = 0x493d_3000u32;
    for (i, (a, b)) in gen_pq(&mut s, 4000, 50.0).into_iter().enumerate() {
        if a.iter().chain(b.iter()).any(|x| x.is_nan()) {
            continue;
        }
        wvec3(&mut emu, a_buf, 0, a);
        wvec3(&mut emu, b_buf, 0, b);
        emu.set_reg(RegisterX86::EDI, a_buf);
        emu.set_reg(RegisterX86::EAX, b_buf);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, sp);
        emu.run(0x493dd3, 0x493e09);
        let esp = emu.get_reg(RegisterX86::ESP);
        eqf(
            emu.read_u32(esp),
            unit_distance_sq(a, b),
            &format!("493d30 case {i} dist²"),
        );
    }
    eprintln!("dist2_493d30: f32 dist² producer bit-exact");
}

#[test]
fn dist2_4b1060() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let unit = emu.alloc(0x360); // esi: key stored to [esi+0x350]
    let frame = emu.alloc(0x40); // ebp: transformed coords at [ebp-0x24/-0x20/-0x1c]
    let ebp = frame + 0x30;
    let mut s = 0x4b10_6000u32;
    for i in 0..4000 {
        // synthesized unit-local coords (the FUN_004549a0 transform output — a boundary).
        let x = rngf(&mut s, -500.0, 500.0);
        let y = rngf(&mut s, -500.0, 500.0);
        let z = rngf(&mut s, -500.0, 500.0);
        emu.write_u32(ebp - 0x24, x.to_bits());
        emu.write_u32(ebp - 0x20, y.to_bits());
        emu.write_u32(ebp - 0x1c, z.to_bits());
        emu.set_reg(RegisterX86::ESI, unit);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.run(0x4b12a8, 0x4b12c6);
        eqf(
            emu.read_u32(unit + 0x350),
            bubble_sort_key([x, y, z]),
            &format!("4b1060 case {i} key"),
        );
    }
    eprintln!("dist2_4b1060: f32 (y²+z²)+x² key producer bit-exact (transform = boundary)");
}

#[test]
fn dist2_5116e0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // (a) f32 dist² via slice (bit-exact).
    {
        let p_buf = emu.alloc(0x10); // eax
        let q_buf = emu.alloc(0x10); // ecx
        let frame = emu.alloc(0x30); // ebp: dist² stored to [ebp+8]
        let ebp = frame;
        let mut s = 0x5116_e0a0u32;
        for (i, (p, q)) in gen_pq(&mut s, 3000, 50.0).into_iter().enumerate() {
            if q.iter().any(|x| x.is_nan()) {
                continue;
            }
            wvec3(&mut emu, p_buf, 0, p);
            wvec3(&mut emu, q_buf, 0, q);
            emu.set_reg(RegisterX86::EAX, p_buf);
            emu.set_reg(RegisterX86::ECX, q_buf);
            emu.set_reg(RegisterX86::EBP, ebp);
            emu.run(0x5116e9, 0x51170c);
            let want = shake_falloff(p, q, 1.0, 10.0, 1.0, 1.0).dist2;
            eqf(
                emu.read_u32(ebp + 8),
                want,
                &format!("5116e0 case {i} dist²"),
            );
        }
    }
    // Sanity: the falloff base const at 0x808ad0 is the f64 promotion of the `0.7f` float literal,
    // not 0.7_f64 — assert the kernel's hard-coded const matches the bytes in the binary.
    let c07_bin = (u64::from(emu.read_u32(0x808ad4)) << 32) | u64::from(emu.read_u32(0x808ad0));
    assert_eq!(
        c07_bin,
        FALLOFF_BASE.to_bits(),
        "0x808ad0 falloff base const"
    );

    // (b) the return value (the outer^2 and inner^2 branches) and the falloff's `*scale`, through
    // the whole leaf call. `*scale = (0.7^t * scale_in) as f32` rests on the transcendental step:
    // the x87 `fyl2x` and `f2xm1` pow runs in the emulator's soft-float, not through the host's
    // maths library as `fsincos` and `fpatan` do, against the host's `exp2(t * log2(0.7f))`. The
    // root, the two divides, the multiply by `scale_in` and the final rounding are exact. Over this
    // grid the one-ulp difference of `f64` is absorbed by the rounding to `f32`, so the compare is
    // bit for bit; `max_ulp` is printed. A return of 0 or a skipped falloff leaves `*scale` as it
    // was.
    let p_buf = emu.alloc(0x10);
    let q_buf = emu.alloc(0x10);
    let scale = emu.alloc(0x10);
    let (mut r0, mut r1, mut fall) = (0u32, 0u32, 0u32);
    let mut max_ulp = 0u32;
    let mut s = 0x5116_e0b0u32;
    let base = 40.0f32;
    for (i, (p, q)) in gen_pq(&mut s, 6000, base).into_iter().enumerate() {
        if q.iter().any(|x| x.is_nan()) {
            continue;
        }
        let denom = rngf(&mut s, 8.0, 30.0);
        let inner2 = rngf(&mut s, 400.0, 1200.0);
        let outer2 = inner2 + rngf(&mut s, 0.0, 1200.0);
        let scale_in = rngf(&mut s, 0.25, 4.0);
        let mine = shake_falloff(p, q, scale_in, denom, inner2, outer2);
        wvec3(&mut emu, p_buf, 0, p);
        wvec3(&mut emu, q_buf, 0, q);
        emu.write_u32(scale, scale_in.to_bits());
        let eax = emu.call(
            0x5116e0,
            0,
            0,
            &[
                p_buf as u32,
                q_buf as u32,
                scale as u32,
                denom.to_bits(),
                inner2.to_bits(),
                0,
                outer2.to_bits(),
            ],
        );
        assert_eq!(
            eax & 1 == 1,
            mine.ret,
            "5116e0 case {i}: return {} vs {} (dist²={} inner²={inner2} outer²={outer2})",
            eax & 1,
            mine.ret,
            mine.dist2
        );
        // The binary's post-call *scale — bit-exact whether falloff fired (transcendental, narrowed
        // exact) or not (untouched = scale_in).
        let bin_scale = emu.read_u32(scale);
        max_ulp = max_ulp.max(bin_scale.abs_diff(mine.scale_out.to_bits()));
        eqf(
            bin_scale,
            mine.scale_out,
            &format!("5116e0 case {i} *scale"),
        );
        if mine.falloff {
            fall += 1;
        }
        if mine.ret {
            r1 += 1;
        } else {
            r0 += 1;
        }
    }
    eprintln!(
        "dist2_5116e0: f32 dist² + return + falloff *scale bit-exact \
         (ret1={r1} ret0={r0} falloff={fall}; falloff *scale max f32 ULP={max_ulp})"
    );
    assert!(r0 > 0 && r1 > 0 && fall > 0, "5116e0 branch coverage");
}

// ===================================================================================== //
//  2D nearest-search argmin (4a76e0) — full run to the post-loop winner pointer.        //
// ===================================================================================== //
