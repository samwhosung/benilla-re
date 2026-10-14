//! Bit-exact diff for `playername::timing` — the per-category timing/scale math — vs `WoW.exe`.
//!
//! Each primitive reads the per-category config table at `.bss` `0xce8828` (stride `0x1c`,
//! `row = this[8]*0x1c`). The table is uninitialized in the file image (init `0x6c79a0` fills it at
//! runtime), so every case writes the chosen row's fields at their absolute VAs via `emu.write_u32`
//! and runs the real slice, which feeds the `.bss` directly rather than synthesizing a relocated
//! island.
//!
//! Observables:
//! * `interp_param` (`0x6c8070`) leaves its result live on st0 → `call_f64` reads the full PC_53 f64.
//! * `keyframe_interp` (`0x6c80b0`) tail-relays the clamped scale to `0x6c82c0` → STUB `0x6c82c0` and
//!   read the pushed f32 arg (`calls()[0].args[0]`).
//! * `time_alpha_fade` (`0x6c82e0`) builds `bl`=alpha / `al`=color before `0x6c843d` → halt there and
//!   read `EBX`/`EAX` (no stubbing; `__ftol` `0x40a2b0` runs natively).
//! * `width_dirty_guard` (`0x6c82c0`) stores + calls `0x6c81a0` iff `param != cached` → STUB `0x6c81a0`
//!   and observe whether it was called (and that `this[0x2c]` was updated).

#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_playername::timing::{
    interp_param, keyframe_interp, time_alpha_fade, width_dirty_guard, Category,
};

// shared LCG fuzz (high bits only).
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

// --- config-table layout (per-category, stride 0x1c, base 0xce8828) ---
const TABLE_BASE: u64 = 0x00ce_8828;
const STRIDE: u64 = 0x1c;
// The cat==2 keyframe multiplier `[edi+0xce883c]` with edi=2*0x1c resolves to 0xce8874 = row2's own
// value_hi slot, so it is just `Category.value_hi` of cat 2. The real init value there is 0.0275.
const CAT2_MUL_BITS: u32 = 0x3ce1_47ad; // 0.0275 (= row2.value_hi)

/// Write a category row at `0xce8828 + cat*0x1c`.
fn write_cat(emu: &mut Emu, cat: u32, c: &Category) {
    let row = TABLE_BASE + u64::from(cat) * STRIDE;
    emu.write_u32(row, c.rise_span.to_bits()); // +0x00 f32 rise-span
    emu.write_u32(row + 0x04, c.fade_in_end_ms as u32); // +0x04 i32 fade-in-end
    emu.write_u32(row + 0x08, c.fade_out_start_ms as u32); // +0x08 i32 fade-out-start
    emu.write_u32(row + 0x0c, c.duration_ms as u32); // +0x0c i32 duration
    emu.write_u32(row + 0x10, c.value_lo.to_bits()); // +0x10 f32 value-lo
    emu.write_u32(row + 0x14, c.value_hi.to_bits()); // +0x14 f32 value-hi
}

/// A small bank of category rows exercising each shape. cat 0/1: affine ("rising"/"crit") with
/// distinct value_lo/value_hi and fade windows; cat 2: the keyframe-curve category (the piecewise
/// branch); cat 3: a degenerate fade window (fade_in_end == fade_out_start == duration).
fn cat_bank() -> [(u32, Category); 4] {
    [
        (
            0,
            Category {
                rise_span: 2.0,                        // 0x40000000 (real init row0 rise-span)
                fade_in_end_ms: 0x96,                  // 150
                fade_out_start_ms: 0x2f8,              // 760
                duration_ms: 0x5dc,                    // 1500
                value_lo: f32::from_bits(0x3c96_2fc9), // ~0.01835 (real init value_lo/hi)
                value_hi: f32::from_bits(0x3c96_2fc9),
            },
        ),
        (
            1,
            Category {
                rise_span: 1.5,
                fade_in_end_ms: 0x5a,     // 90
                fade_out_start_ms: 0x320, // 800
                duration_ms: 0x5dc,       // 1500
                value_lo: 0.25,
                value_hi: 1.75,
            },
        ),
        (
            2,
            Category {
                rise_span: 1.0,
                fade_in_end_ms: 0x96,    // 150
                fade_out_start_ms: 1000, // 0x3e8
                duration_ms: 0x5dc,      // 1500 (cat==2 piecewise branch)
                value_lo: 0.0,
                // value_hi (slot 0xce8874) IS the cat==2 piecewise multiplier — the real init 0.0275.
                value_hi: f32::from_bits(CAT2_MUL_BITS),
            },
        ),
        (
            3,
            Category {
                rise_span: 0.5,
                fade_in_end_ms: 0x4b0,    // 1200
                fade_out_start_ms: 0x4b0, // 1200 == in_end (degenerate fade-out window)
                duration_ms: 0x4b0,       // 1200 == out_start (span dur-out_start == 0)
                value_lo: 0.5,
                value_hi: 2.0,
            },
        ),
    ]
}

// ============================================================================================
// 0x6c8070 pn_interp_param — (unsigned elapsed)/dur_i32 * span_f32, result live on st0.
// ============================================================================================
#[test]
fn pn_interp_param() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40); // a WORLDTEXTSTRING; only this[8]=category is read.

    // call_f64 reads the FULL st0 (PC_53 f64) via the `fstp qword` trampoline — the caller `fadd`s the
    // live st0 unrounded, so the f64 is the bit-exact observable. ecx=this, [esp+4]=elapsed.
    let run = |emu: &mut Emu, cat: u32, elapsed: u32| -> f64 {
        emu.write_u32(this + 8, cat);
        emu.call_f64(0x6c8070, this as u32, 0, &[elapsed])
    };

    let mut total = 0u32;
    for (cat, c) in cat_bank() {
        write_cat(&mut emu, cat, &c);
        // boundary probes: elapsed at 0, 1, dur-1, dur, dur+1, dur*2, and the i32-as-u32 boundary
        // 0x80000000 (the fild-qword unsigned read pivot).
        let dur = c.duration_ms as u32;
        let probes = [
            0u32,
            1,
            dur - 1,
            dur,
            dur + 1,
            dur * 2,
            0x8000_0000,
            0xffff_ffff,
        ];
        for &e in &probes {
            let got = run(&mut emu, cat, e);
            let want = interp_param(e, &c);
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "interp_param cat={cat} elapsed={e}: got {got} want {want}"
            );
            total += 1;
        }
        let mut s = 0x6c80_7000u32 ^ cat;
        for _ in 0..1500 {
            let e = next(&mut s) >> 8; // high LCG bits, 24-bit elapsed
            let got = run(&mut emu, cat, e);
            let want = interp_param(e, &c);
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "interp_param fuzz cat={cat} e={e}"
            );
            total += 1;
        }
    }
    eprintln!("pn_interp_param: {total} bit-exact (fild qword; fidiv dword dur; fmul dword span)");
}

// ============================================================================================
// 0x6c80b0 pn_keyframe_interp — scale curve; tail-relays clamped scale (f32) to 0x6c82c0.
// ============================================================================================
#[test]
fn pn_keyframe_interp() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40);
    // Stub the relay target 0x6c82c0 so we capture the pushed scale f32 (and avoid 0x6c81a0/font).
    // (The cat==2 multiplier 0xce8874 = row2.value_hi is written by write_cat for cat 2.)
    emu.stub(0x6c82c0, 0, 1); // __thiscall ret 4 → pop 1 stack arg

    // Run the real slice; ecx=this, [esp+4]=elapsed; sentinel return; read the captured arg.
    let run = |emu: &mut Emu, cat: u32, elapsed: u32| -> f32 {
        emu.write_u32(this + 8, cat);
        emu.clear_calls();
        // emu.call sets up ecx + stack-arg + sentinel; 0x6c80b0 calls (stubbed) 0x6c82c0 then ret 4.
        emu.call(0x6c80b0, this as u32, 0, &[elapsed]);
        let calls = emu.calls();
        assert_eq!(
            calls.len(),
            1,
            "keyframe must relay exactly once (cat={cat} e={elapsed})"
        );
        f32::from_bits(calls[0].args[0])
    };

    let mut total = 0u32;
    for (cat, c) in cat_bank() {
        write_cat(&mut emu, cat, &c);
        let dur = c.duration_ms as u32;
        // boundary probes: t at 0, the segment boundaries (.1, .2 → elapsed = 0.1*dur, 0.2*dur),
        // t==1 (elapsed==dur), the clamp-to-0.001 edge, and just past each boundary.
        let seg10 = (dur as f64 * 0.1) as u32; // t = 0.1 (segment boundary)
        let seg20 = (dur as f64 * 0.2) as u32; // t = 0.2
        let probes = [
            0u32,
            1,
            seg10.saturating_sub(1),
            seg10,
            seg10 + 1,
            seg20.saturating_sub(1),
            seg20,
            seg20 + 1,
            dur - 1,
            dur,
            dur + 1,
            dur * 2,
        ];
        for &e in &probes {
            let got = run(&mut emu, cat, e);
            let want = keyframe_interp(e, cat as i32, &c);
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "keyframe cat={cat} elapsed={e}: got {got} want {want}"
            );
            total += 1;
        }
        let mut s = 0x6c80_b000u32 ^ cat;
        for _ in 0..1500 {
            let e = (next(&mut s) >> 8) % (dur * 3 + 8); // span 0..3*dur to exercise all segments + clamp
            let got = run(&mut emu, cat, e);
            let want = keyframe_interp(e, cat as i32, &c);
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "keyframe fuzz cat={cat} e={e}"
            );
            total += 1;
        }
    }
    eprintln!(
        "pn_keyframe_interp: {total} bit-exact (t spilled→f32; affine | piecewise; 0.001 clamp)"
    );
}

// ============================================================================================
// 0x6c82e0 pn_time_alpha_fade — (alpha,color) bytes; halt at 0x6c843d, read EBX(bl)/EAX(al).
// ============================================================================================
#[test]
fn pn_time_alpha_fade() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40);
    let stack = emu.alloc(0x1000);

    // Run from 0x6c82e0 to 0x6c843d (after both byte computes, before the 0x5c2750/0x5c27a0 relays).
    // ecx=this, [esp+4]=elapsed, [esp]=return slot. EBX low byte = alpha, EAX low byte = color.
    // EBX must start zeroed in its upper bytes so the bl read is clean (the slice writes only bl).
    let run = |emu: &mut Emu, cat: u32, elapsed: u32| -> (u8, u8) {
        emu.write_u32(this + 8, cat);
        let esp = stack + 0xf00;
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write_u32(esp, 0); // return addr slot (unused — we halt before the ret)
        emu.write_u32(esp + 4, elapsed); // [esp+4] = param
        emu.set_reg(RegisterX86::ECX, this);
        emu.set_reg(RegisterX86::EBX, 0);
        emu.set_reg(RegisterX86::EAX, 0);
        emu.run(0x6c82e0, 0x6c843d);
        let ebx = emu.get_reg(RegisterX86::EBX) as u32;
        let eax = emu.get_reg(RegisterX86::EAX) as u32;
        ((ebx & 0xff) as u8, (eax & 0xff) as u8)
    };

    let mut total = 0u32;
    for (cat, c) in cat_bank() {
        write_cat(&mut emu, cat, &c);
        let in_end = c.fade_in_end_ms as u32;
        let out_start = c.fade_out_start_ms as u32;
        let dur = c.duration_ms as u32;
        // boundary probes: the regime edges (in_end-1/in_end = fade-in→mid, out_start-1/out_start =
        // mid→fade-out), 0, dur, dur+1, and the __ftol truncation pivots near each clamp edge.
        let probes = [
            0u32,
            1,
            in_end.saturating_sub(1),
            in_end,
            in_end + 1,
            out_start.saturating_sub(1),
            out_start,
            out_start + 1,
            (in_end + out_start) / 2,
            dur.saturating_sub(1),
            dur,
            dur + 1,
            dur * 2,
        ];
        for &e in &probes {
            let (ga, gc) = run(&mut emu, cat, e);
            let want = time_alpha_fade(e, &c);
            assert_eq!(
                (ga, gc),
                (want.alpha, want.color),
                "alpha_fade cat={cat} elapsed={e}: got ({ga},{gc}) want ({},{})",
                want.alpha,
                want.color
            );
            total += 1;
        }
        let mut s = 0x6c82_e000u32 ^ cat;
        for _ in 0..1500 {
            let e = (next(&mut s) >> 8) % (dur * 2 + 8);
            let (ga, gc) = run(&mut emu, cat, e);
            let want = time_alpha_fade(e, &c);
            assert_eq!(
                (ga, gc),
                (want.alpha, want.color),
                "alpha_fade fuzz cat={cat} e={e}"
            );
            total += 1;
        }
    }
    eprintln!(
        "pn_time_alpha_fade: {total} bit-exact (fade-in/mid/out; 2x __ftol; clamps 0/127/255)"
    );
}

// ============================================================================================
// 0x6c82c0 pn_width_dirty_guard — store+relay iff param != cached (FP !=); stub 0x6c81a0.
// ============================================================================================
#[test]
fn pn_width_dirty_guard() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40);
    emu.stub(0x6c81a0, 0, 0); // relayout; __fastcall(this in ecx), no stack args → ret 0

    // ecx=this, this[0x2c]=cached, [esp+4]=param. Returns whether the body ran (0x6c81a0 called) AND
    // checks this[0x2c] was updated iff it relayed.
    let run = |emu: &mut Emu, param: f32, cached: f32| -> bool {
        emu.write_u32(this + 0x2c, cached.to_bits());
        emu.clear_calls();
        emu.call(0x6c82c0, this as u32, 0, &[param.to_bits()]);
        let relayed = !emu.calls().is_empty();
        // cross-check: if it relayed, this[0x2c] now holds param; else it's unchanged (cached).
        let stored = f32::from_bits(emu.read_u32(this + 0x2c));
        if relayed {
            assert_eq!(
                stored.to_bits(),
                param.to_bits(),
                "relayed but did not store param"
            );
        } else {
            assert_eq!(
                stored.to_bits(),
                cached.to_bits(),
                "not relayed but width changed"
            );
        }
        relayed
    };

    let mut total = 0u32;
    // boundary probes: the == predicate (equal → no relay), tiny deltas (the != boundary), signed
    // zero (-0.0 == 0.0 in FP → no relay), and NaN (unordered → relay).
    let pairs: [(f32, f32); 10] = [
        (1.0, 1.0),                         // equal → no relay
        (1.0, 1.000_001),                   // differ → relay
        (0.0, -0.0),                        // +0 == -0 in FP → no relay
        (-0.0, 0.0),                        // mirror → no relay
        (f32::NAN, 1.0),                    // NaN unordered → relay
        (1.0, f32::NAN),                    // mirror → relay
        (100.0, 100.0),                     // equal → no relay
        (-1.0, 1.0),                        // differ (sign) → relay
        (f32::from_bits(0x3f80_0001), 1.0), // 1 ULP above 1.0 → relay
        (1e30, 1e30),                       // equal large → no relay
    ];
    for &(p, ca) in &pairs {
        let got = run(&mut emu, p, ca);
        let want = width_dirty_guard(p, ca);
        assert_eq!(
            got, want,
            "width_guard param={p} cached={ca}: got {got} want {want}"
        );
        total += 1;
    }
    let mut s = 0x6c82_c000u32;
    for _ in 0..3000 {
        // mix equal and unequal cases: half the time, reuse cached as param (force the == branch).
        let bits_a = next(&mut s);
        let ca = f32::from_bits(bits_a);
        let p = if next(&mut s) & 1 == 0 {
            ca // exact equal
        } else {
            f32::from_bits(next(&mut s))
        };
        // skip NaN-vs-itself ambiguity isn't an issue: NaN != NaN in both binary and Rust.
        let got = run(&mut emu, p, ca);
        let want = width_dirty_guard(p, ca);
        assert_eq!(
            got,
            want,
            "width_guard fuzz param_bits={:08x} cached_bits={:08x}",
            p.to_bits(),
            ca.to_bits()
        );
        total += 1;
    }
    eprintln!(
        "pn_width_dirty_guard: {total} bit-exact (fcomp; test ah,0x44; jnp — FP != predicate)"
    );
}
