//! The inline range gates of the Lua checks `CanInspect` (`0x48a1b0`) and `CheckInteractDistance`
//! (`0x48ba00`) (`systems/ui/distancegates.md#caninspect`, `#checkinteractdistance`) run from the exe
//! as instruction slices.
//!
//! The test writes the two positions and the threshold into the memory the slice reads, runs from the
//! first `fld` to the `fnstsw`, and applies the gate's own `test ah, mask` to the status word in
//! `eax`. The inputs straddle the threshold to the last bit and include NaN. Each branch is tallied
//! and required to be hit. Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::distance_gates::{
    can_inspect_out_of_range as caninspect_out_of_range, check_interact_in_range,
    script_distance_sq as dist2, INTERACT_TABLE,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

fn vec3(s: &mut u32, lo: f32, hi: f32) -> [f32; 3] {
    [rng(s, lo, hi), rng(s, lo, hi), rng(s, lo, hi)]
}

fn wvec(emu: &mut Emu, addr: u64, v: [f32; 3]) {
    emu.write_u32(addr, v[0].to_bits());
    emu.write_u32(addr + 4, v[1].to_bits());
    emu.write_u32(addr + 8, v[2].to_bits());
}

fn ah_of(emu: &Emu) -> u8 {
    ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

// ===========================================================================================
// caninspect_dist2 (0x48a1b0) — d² gate slice 0x48a25f..0x48a289
// ===========================================================================================
#[test]
fn caninspect_dist2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let q_buf = emu.alloc(12);
    let p_buf = emu.alloc(12);
    const THR: u64 = 0xb4d918;

    // run one (q,p,thr) → whether the binary takes the out-of-range branch.
    fn bin_out(emu: &mut Emu, q_buf: u64, p_buf: u64, q: [f32; 3], p: [f32; 3], thr: f32) -> bool {
        wvec(emu, q_buf, q);
        wvec(emu, p_buf, p);
        emu.write_u32(THR, thr.to_bits());
        emu.set_reg(RegisterX86::EAX, q_buf);
        emu.set_reg(RegisterX86::EDI, p_buf);
        emu.run(0x48a25f, 0x48a289);
        ah_of(emu) & 0x41 == 0 // jne (C0|C3) skips the out path; out fires iff thr < d²
    }

    let mut out_hits = 0u32;
    let mut in_hits = 0u32;
    let mut eq_hits = 0u32;
    let mut nan_hits = 0u32;
    let mut total = 0u32;

    // deterministic boundary: p=0, q=(3,4,0) ⇒ d²=25.0 exactly (exact f32) — straddle it.
    for (q, p, thr, _lbl) in [
        ([3.0, 4.0, 0.0], [0.0, 0.0, 0.0], 24.0f32, "below"),
        ([3.0, 4.0, 0.0], [0.0, 0.0, 0.0], 25.0, "equal"),
        ([3.0, 4.0, 0.0], [0.0, 0.0, 0.0], 26.0, "above"),
        ([3.0, 4.0, 0.0], [0.0, 0.0, 0.0], 100.0, "canonical"),
        ([f32::NAN, 0.0, 0.0], [0.0, 0.0, 0.0], 100.0, "nan"),
    ] {
        let b = bin_out(&mut emu, q_buf, p_buf, q, p, thr);
        assert_eq!(
            b,
            caninspect_out_of_range(q, p, thr),
            "caninspect det q={q:?} thr={thr}"
        );
        if q[0].is_nan() {
            nan_hits += 1;
        } else if f64::from(thr) == dist2(q, p) {
            eq_hits += 1;
        }
        total += 1;
    }

    let mut s = 0x48a1_b000u32;
    for _ in 0..4000 {
        let q = vec3(&mut s, -400.0, 400.0);
        let p = vec3(&mut s, -400.0, 400.0);
        let d2 = dist2(q, p);
        let d2f = d2 as f32;
        for thr in [0.0f32, d2f * 0.999, d2f * 1.001, 100.0, 1.0e30] {
            let b = bin_out(&mut emu, q_buf, p_buf, q, p, thr);
            let mine = caninspect_out_of_range(q, p, thr);
            assert_eq!(b, mine, "caninspect q={q:?} p={p:?} thr={thr} d2={d2}");
            if mine {
                out_hits += 1;
            } else {
                in_hits += 1;
            }
            total += 1;
        }
    }
    eprintln!("caninspect_dist2: {total} bit-exact (out={out_hits} in={in_hits} eq={eq_hits} nan={nan_hits})");
    assert!(out_hits > 0 && in_hits > 0, "need both in/out coverage");
    assert!(eq_hits > 0, "need an exact-equal-threshold case");
    assert!(nan_hits > 0, "need a NaN/unordered case");
}

// ===========================================================================================
// check_interact_dist2 (0x48ba00) — d² gate slice 0x48badd..0x48bb08, table @0xb4e3cc
// ===========================================================================================
#[test]
fn check_interact_dist2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let q_buf = emu.alloc(12);
    let p_buf = emu.alloc(12);
    const TBL: u64 = 0xb4e3cc;

    fn bin_in(
        emu: &mut Emu,
        q_buf: u64,
        p_buf: u64,
        q: [f32; 3],
        p: [f32; 3],
        ty: u32,
        thr: f32,
    ) -> bool {
        emu.write_u32(TBL + u64::from(ty - 1) * 4, thr.to_bits());
        wvec(emu, q_buf, q);
        wvec(emu, p_buf, p);
        emu.set_reg(RegisterX86::EAX, q_buf);
        emu.set_reg(RegisterX86::EBX, p_buf);
        emu.set_reg(RegisterX86::ESI, u64::from(ty - 1));
        emu.run(0x48badd, 0x48bb08);
        // test ah,0x5; jp -> out when (C0,C2) has EVEN parity; in-range iff odd (ordered below).
        (ah_of(emu) & 0x05).count_ones() % 2 == 1
    }

    let mut in_hits = 0u32;
    let mut out_hits = 0u32;
    let mut ty_tally = [0u32; 4];
    let mut total = 0u32;

    // canonical table {100, 123.45678, 100, 900}: per type, straddle its threshold.
    for ty in 1..=4u32 {
        let thr = INTERACT_TABLE[(ty - 1) as usize];
        let r = thr.sqrt(); // radius so d² ≈ thr
        for scale in [0.9f32, 1.1] {
            let q = [r * scale, 0.0, 0.0];
            let p = [0.0, 0.0, 0.0];
            let b = bin_in(&mut emu, q_buf, p_buf, q, p, ty, thr);
            assert_eq!(
                b,
                check_interact_in_range(q, p, thr),
                "checki canon ty={ty} scale={scale}"
            );
            ty_tally[(ty - 1) as usize] += 1;
            total += 1;
        }
    }

    let mut s = 0x48ba_0000u32;
    for _ in 0..3000 {
        let q = vec3(&mut s, -50.0, 50.0);
        let p = vec3(&mut s, -50.0, 50.0);
        let ty = 1 + (next(&mut s) >> 8) % 4;
        let d2 = dist2(q, p);
        let d2f = d2 as f32;
        for thr in [
            0.0f32,
            d2f * 0.999,
            d2f * 1.001,
            INTERACT_TABLE[(ty - 1) as usize],
        ] {
            let b = bin_in(&mut emu, q_buf, p_buf, q, p, ty, thr);
            let mine = check_interact_in_range(q, p, thr);
            assert_eq!(b, mine, "checki q={q:?} p={p:?} ty={ty} thr={thr} d2={d2}");
            if mine {
                in_hits += 1;
            } else {
                out_hits += 1;
            }
            ty_tally[(ty - 1) as usize] += 1;
            total += 1;
        }
    }
    eprintln!("check_interact_dist2: {total} bit-exact (in={in_hits} out={out_hits} per-type {ty_tally:?})");
    assert!(in_hits > 0 && out_hits > 0, "need both in/out coverage");
    for (i, t) in ty_tally.iter().enumerate() {
        assert!(*t > 0, "interaction type {} never exercised", i + 1);
    }
}
