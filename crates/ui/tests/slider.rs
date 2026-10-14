//! Differential tests of the arithmetic of the slider functions `0x7898f0`, `0x789930`,
//! `0x789a60`, `0x789aa0` and `CSimpleSlider::OnMouseMove` (`0x789ba0`), each run against
//! `WoW.exe` in the emulator.
//!
//! Each row is diffed by executing the binary's OWN bytes and reading back the value it stores /
//! passes downstream — covering every in-body FP-arith op by construction:
//! - `0x7898f0` (span): whole-fn `call`, its conditional call to `0x789930` stubbed; read `[this+0x31c]`.
//! - `0x789930` (set_value): whole-fn `call`, the value-changed signal `0x7026f0` skipped (the
//!   `[+0x330]==0` guard); the `__ftol` `0x40a2b0` runs natively (pure x87). Flags forced so the
//!   clamp/quantize result stores to `[this+0x320]`, read back.
//! - `0x789a60` (range+min): whole-fn `call`, its conditional call to `0x7898f0` stubbed; read its arg.
//! - `0x789aa0` (value→thumb-offset, the OnUpdate reposition): whole-fn `call`; the thumb's vtable position getter resolved to a
//!   real `fld [cell]; ret` thunk that pushes a chosen float (the getter pushes a float in st0);
//!   the bounds-prep `0x76b2c0` and layout setter `0x767c70` stubbed; the two computed floats read
//!   from the `0x767c70` stub's recorded args.
//! - `0x789ba0` (value→pos): whole-fn `call`; the same getter-thunk (called twice → same value);
//!   `0x76b320` stubbed; `0x789930` stubbed and the computed value read from its arg.
//!
//! Every test carries deterministic boundary probes: the clamp-low/high `==`-boundary
//! (`v == min`, `v == bound`), the quantize `diff == 0` pivot and step-half boundary, and the
//! degenerate-range `|range| == eps` guard of 0x789aa0. Fuzz reads the LCG's HIGH bits.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::slider::{
    pos_horiz, pos_to_value, pos_vert, range_plus_min, set_value, span, thumb_frac,
    thumb_offset_horiz, thumb_offset_vert,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

// Struct field offsets (the slider/scrollbar control).
const F_TRACK_LO_V: u64 = 0x64; // [+0x64] vertical track lo
const F_TRACK_HI_V: u64 = 0x6c; // [+0x6c] vertical track hi
const F_TRACK_LO_H: u64 = 0x68; // [+0x68] horizontal track lo
const F_TRACK_HI_H: u64 = 0x70; // [+0x70] horizontal track hi
const F_SCALE: u64 = 0x7c; // [+0x7c] per-node scale
const F_FLAGS: u64 = 0x314;
const F_MIN: u64 = 0x318;
const F_SPAN: u64 = 0x31c;
const F_VALUE: u64 = 0x320;
const F_STEP: u64 = 0x324;
const F_THUMB: u64 = 0x328; // [+0x328] thumb sub-widget
const F_ORIENT: u64 = 0x32c; // [+0x32c] orientation (0 = horizontal)
const F_CALLBACK: u64 = 0x330; // [+0x330] value-changed callback ptr

struct W {
    this: u64,
}
fn world(emu: &mut Emu) -> W {
    let this = emu.alloc(0x400);
    W { this }
}

/// Install a `fld dword [cell]; ret` thunk at `tramp`, returning `(tramp, cell)`. Calling the
/// thunk (a thiscall getter, no stack args) pushes `*cell` (an f32) onto st0 and returns — exactly
/// the contract of the thumb's position getter the slider delegates to.
fn float_getter(emu: &mut Emu) -> (u64, u64) {
    let cell = emu.alloc(0x10);
    let tramp = emu.alloc(0x10);
    // fld dword ptr [disp32]  =  D9 /0, modrm 0x05 (disp32 absolute) + disp32; then ret (C3).
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(tramp, &code);
    (tramp, cell)
}

// ===========================================================================================
// 0x7898f0 — span = end − start (fsub ×1). Whole-fn; the conditional 0x789930 call stubbed; read [+0x31c].
// ===========================================================================================
#[test]
fn glue_7898f0_span() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let w = world(&mut emu);
    emu.stub(0x789930, 0, 1); // the bit2-gated call at 0x789927 (ret 4)

    let run = |emu: &mut Emu, start: f32, end: f32| -> f32 {
        // [+0x314] flag bit 4 clear → the tail-call is skipped anyway, but stub it for safety.
        emu.write_u32(w.this + F_FLAGS, 0);
        // thiscall: ecx=this; stack args (start, end) = [ebp+8],[ebp+0xc].
        emu.call(
            0x7898f0,
            w.this as u32,
            0,
            &[start.to_bits(), end.to_bits()],
        );
        emu.read_f32(w.this + F_SPAN)
    };

    let mut total = 0u32;
    for (start, end) in [
        (0.0f32, 1.0f32),
        (1.0, 1.0),    // span 0
        (5.0, -5.0),   // negative span
        (-1.0, 100.0), // probe: end exactly representable
        (16_777_216.0, 16_777_217.0),
    ] {
        let got = run(&mut emu, start, end);
        assert_eq!(
            got.to_bits(),
            span(start, end).to_bits(),
            "span start={start} end={end}"
        );
        total += 1;
    }
    let mut s = 0x7898_f000u32;
    for _ in 0..4000 {
        let start = rng(&mut s, -10000.0, 10000.0);
        let end = rng(&mut s, -10000.0, 10000.0);
        assert_eq!(
            run(&mut emu, start, end).to_bits(),
            span(start, end).to_bits(),
            "span fuzz"
        );
        total += 1;
    }
    eprintln!("glue_7898f0_span: {total} bit-exact (end − start)");
}

// ===========================================================================================
// 0x789930 — clamp-low + clamp-high + round-to-step quantize (9 FP ops). Whole-fn; formatter
// skipped via [+0x330]==0; __ftol runs native; result stored to [+0x320] and read back.
// ===========================================================================================
#[test]
fn glue_789930_set_value() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let w = world(&mut emu);

    let run = |emu: &mut Emu, v: f32, min: f32, sp: f32, step: f32| -> f32 {
        // flags: bit1 set (the `test bl,2` gate → do work), bit2 clear (the `bl&4` store-gate →
        // `je` taken → unconditional store to [+0x320]). [+0x330]=0 → no formatter call.
        emu.write_u32(w.this + F_FLAGS, 0x2);
        emu.write_u32(w.this + F_MIN, min.to_bits());
        emu.write_u32(w.this + F_SPAN, sp.to_bits());
        emu.write_u32(w.this + F_STEP, step.to_bits());
        emu.write_u32(w.this + F_VALUE, 0); // current value (overwritten by the store)
        emu.write_u32(w.this + F_CALLBACK, 0); // skip 0x7026f0
        emu.call(0x789930, w.this as u32, 0, &[v.to_bits()]);
        emu.read_f32(w.this + F_VALUE)
    };

    let mut total = 0u32;
    // Deterministic boundary probes:
    //   v==min (clamp-low ==), v==bound (clamp-high ==), diff==0 (quantize pivot), step-half edge.
    for (v, min, sp, step) in [
        (5.0f32, 0.0f32, 10.0f32, 0.0f32), // in range, no step
        (-3.0, 0.0, 10.0, 0.0),            // below min → clamp to min
        (15.0, 0.0, 10.0, 0.0),            // above bound → clamp to bound
        (0.0, 0.0, 10.0, 0.0),             // v == min exactly
        (10.0, 0.0, 10.0, 0.0),            // v == bound exactly (lo==bound boundary)
        (5.0, 0.0, 10.0, 2.0),             // quantize, diff=5, half-step=1 → round
        (5.0, 0.0, 10.0, 5.0),             // diff=5, half-step=2.5 → (5+2.5)/5=1.5 trunc=1 → 5
        (1.0, 0.0, 10.0, 2.0),             // diff=1, (2/2+1)... round to 2
        (0.0, 0.0, 10.0, 3.0),             // diff==0 exactly → round-down path
        (7.5, 0.0, 10.0, 2.5),             // diff=7.5, exact multiples
        (-100.0, -50.0, 0.0, 0.0),         // span 0: bound==min, clamp to min
        (3.3, 1.0, 8.0, 0.5),              // fractional step
        // NaN, the infinities and quotients past `i32` and `i64`: `__ftol` returns the low dword
        // of its qword, 0 for the indefinite.
        (f32::NAN, 0.0, 10.0, 2.0),
        (5.0, 0.0, 10.0, f32::NAN),
        (5.0, f32::NAN, 10.0, 2.0),
        (5.0, 0.0, 10.0, 1.0e-9),
        (5.0, 0.0, 10.0, 1.0e-30),
        (5.0, 0.0, 10.0, -1.0e-9),
        (5.0, 0.0, f32::INFINITY, 2.0),
        (f32::INFINITY, 0.0, 10.0, 2.0),
        (5.0, 0.0, 1.0e30, 1.0e-3),
    ] {
        let got = run(&mut emu, v, min, sp, step);
        let want = set_value(v, min, sp, step);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "set_value v={v} min={min} span={sp} step={step}"
        );
        total += 1;
    }
    let mut s = 0x7899_3000u32;
    for _ in 0..6000 {
        let min = rng(&mut s, -100.0, 100.0);
        let sp = rng(&mut s, -50.0, 200.0);
        let v = rng(&mut s, -200.0, 300.0);
        // step: 0 sometimes (no quantize), else a positive-ish value; high bit picks.
        let step = if (next(&mut s) >> 8).is_multiple_of(4) {
            0.0
        } else {
            rng(&mut s, -10.0, 30.0)
        };
        let got = run(&mut emu, v, min, sp, step);
        let want = set_value(v, min, sp, step);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "set_value fuzz v={v} min={min} span={sp} step={step}"
        );
        total += 1;
    }
    eprintln!("glue_789930_set_value: {total} bit-exact (clamp-lo/hi + round-to-step)");
}

// ===========================================================================================
// 0x789a60 — range+min (fadd ×1). Whole-fn; the bit1-gated 0x7898f0 call stubbed; read its first stack arg.
// ===========================================================================================
#[test]
fn glue_789a60_range_plus_min() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let w = world(&mut emu);
    // 0x7898f0 is __thiscall(ecx, start, end) with `ret 8`. Stub captures (start,end) in args[0..2].
    emu.stub(0x7898f0, 0, 2);

    let run = |emu: &mut Emu, sp: f32, min: f32| -> f32 {
        emu.write_u32(w.this + F_FLAGS, 0x2); // bit1 set → fires the tail-call
        emu.write_u32(w.this + F_MIN, min.to_bits());
        emu.write_u32(w.this + F_SPAN, sp.to_bits());
        emu.clear_calls();
        emu.call(0x789a60, w.this as u32, 0, &[0u32]); // param2 (step) — integer-stored, irrelevant
                                                       // 0x7898f0 is called as FUN_007898f0([+0x318]=min(thiscall ecx), span+min). The second stack
                                                       // arg [esp+4] = (span+min) is args[1] of the stub's [esp+4..]. Stub records [esp+4],[esp+8].
        let c = emu.calls();
        assert_eq!(c.len(), 1, "expected one call to 0x7898f0");
        f32::from_bits(c[0].args[1])
    };

    let mut total = 0u32;
    for (sp, min) in [
        (10.0f32, 0.0f32),
        (0.0, 5.0),
        (-3.0, 2.0),
        (100.0, -100.0),
        (1.0, 16_777_216.0), // rounding boundary
    ] {
        let got = run(&mut emu, sp, min);
        assert_eq!(
            got.to_bits(),
            range_plus_min(sp, min).to_bits(),
            "range+min span={sp} min={min}"
        );
        total += 1;
    }
    let mut s = 0x789a_6000u32;
    for _ in 0..4000 {
        let sp = rng(&mut s, -10000.0, 10000.0);
        let min = rng(&mut s, -10000.0, 10000.0);
        assert_eq!(
            run(&mut emu, sp, min).to_bits(),
            range_plus_min(sp, min).to_bits(),
            "range+min fuzz"
        );
        total += 1;
    }
    eprintln!("glue_789a60_range_plus_min: {total} bit-exact (span + min)");
}

// ===========================================================================================
// 0x789aa0 — value → thumb-pixel offset (OnUpdate reposition; 14 FP ops). Whole-fn; thumb getter → float thunk; bounds
// prep 0x76b2c0 + layout setter 0x767c70 stubbed; computed floats read from 0x767c70's args.
// horiz [+0x32c]==0: args[3]=delta, args[4]=0; vert: args[3]=0, args[4]=delta.
// ===========================================================================================
#[test]
fn glue_789aa0_thumb_offset() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let w = world(&mut emu);
    let (tramp, cell) = float_getter(&mut emu);
    // thumb object + its vtable. [+0x328]=thumb; [thumb+0x24]=vtable; vtable[0x1c]/[0x20]=getter.
    let thumb = emu.alloc(0x100);
    let vtable = emu.alloc(0x100);
    emu.write_u32(thumb + 0x24, vtable as u32);
    emu.write_u32(vtable + 0x1c, tramp as u32);
    emu.write_u32(vtable + 0x20, tramp as u32);
    emu.stub(0x76b2c0, 0, 1); // bounds prep on param2 (__thiscall, ret 4 → 1 stack arg)
    emu.stub(0x767c70, 0, 6); // the layout setter; ret 0x18 → 6 stack args

    let run = |emu: &mut Emu,
               orient: u32,
               s_scale: f32,
               t_hi: f32,
               t_lo: f32,
               g: f32,
               value: f32,
               min: f32,
               sp: f32|
     -> f32 {
        // flags: need bits 0,1,2 all set (the `(uVar3&1)&&...&&(uVar3&2)&&(uVar3&4)` guard).
        emu.write_u32(w.this + F_FLAGS, 0x7);
        emu.write_u32(w.this + F_THUMB, thumb as u32);
        emu.write_u32(w.this + F_ORIENT, orient);
        emu.write_u32(w.this + F_SCALE, s_scale.to_bits());
        emu.write_u32(w.this + F_MIN, min.to_bits());
        emu.write_u32(w.this + F_SPAN, sp.to_bits());
        emu.write_u32(w.this + F_VALUE, value.to_bits());
        // track endpoints (both orientations written; only the matching pair is read).
        emu.write_u32(w.this + F_TRACK_HI_H, t_hi.to_bits());
        emu.write_u32(w.this + F_TRACK_LO_H, t_lo.to_bits());
        emu.write_u32(w.this + F_TRACK_HI_V, t_hi.to_bits());
        emu.write_u32(w.this + F_TRACK_LO_V, t_lo.to_bits());
        emu.write_u32(cell, g.to_bits()); // the getter returns g
        emu.clear_calls();
        emu.call(0x789aa0, w.this as u32, 0, &[0.0f32.to_bits()]);
        // 0x767c70 recorded: args = (uVar6, this+0x24, uVar7, fVar8, fVar9, 1).
        let c = emu.calls();
        let setter = c
            .iter()
            .find(|sc| sc.addr == 0x767c70)
            .expect("0x767c70 called");
        if orient == 0 {
            f32::from_bits(setter.args[3]) // horiz: fVar8 = delta, fVar9 = 0
        } else {
            f32::from_bits(setter.args[4]) // vert: fVar8 = 0, fVar9 = delta
        }
    };

    let mut total = 0u32;
    let probe = |orient: u32, s: f32, hi: f32, lo: f32, g: f32, v: f32, min: f32, sp: f32| {
        let frac = thumb_frac(v, min, sp);
        if orient == 0 {
            thumb_offset_horiz(s, hi, lo, g, frac)
        } else {
            thumb_offset_vert(s, hi, lo, g, frac)
        }
    };
    // Deterministic probes incl. the |range|==eps guard (span==0 → range 0 → frac 0).
    for (orient, s, hi, lo, g, v, min, sp) in [
        (
            0u32, 2.0f32, 100.0f32, 0.0f32, 10.0f32, 5.0f32, 0.0f32, 10.0f32,
        ),
        (1, 2.0, 100.0, 0.0, 10.0, 5.0, 0.0, 10.0),
        (0, 1.0, 200.0, 0.0, 50.0, 0.0, 0.0, 0.0), // span 0 → range 0 → frac 0
        (0, 4.0, 256.0, 16.0, -3.0, -50.0, -100.0, 200.0),
        (1, 4.0, 256.0, 16.0, -3.0, -50.0, -100.0, 200.0),
        (0, 2.0, 50.0, 50.0, 25.0, 5.0, 0.0, 10.0), // hi==lo → track 0
    ] {
        let got = run(&mut emu, orient, s, hi, lo, g, v, min, sp);
        let want = probe(orient, s, hi, lo, g, v, min, sp);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "thumb-offset o={orient} s={s} hi={hi} lo={lo} g={g} v={v} min={min} span={sp}"
        );
        total += 1;
    }
    let mut st = 0x789a_a000u32;
    for _ in 0..5000 {
        let orient = (next(&mut st) >> 8) % 2;
        let s = {
            let x = rng(&mut st, -100.0, 100.0);
            if x.abs() < 0.25 {
                x + x.signum() * 0.25
            } else {
                x
            }
        };
        let hi = rng(&mut st, -500.0, 500.0);
        let lo = rng(&mut st, -500.0, 500.0);
        let g = rng(&mut st, -500.0, 500.0);
        let min = rng(&mut st, -100.0, 100.0);
        let sp = rng(&mut st, -50.0, 200.0);
        let v = rng(&mut st, -200.0, 300.0);
        let got = run(&mut emu, orient, s, hi, lo, g, v, min, sp);
        let want = probe(orient, s, hi, lo, g, v, min, sp);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "thumb-offset fuzz o={orient}"
        );
        total += 1;
    }
    eprintln!("glue_789aa0_thumb_offset: {total} bit-exact (((hi-lo)/s − g)·frac, vert negated)");
}

// ===========================================================================================
// 0x789ba0 — value → thumb-pixel position (15 FP ops). Whole-fn; getter thunk (called twice →
// same g); 0x76b320 stubbed; 0x789930 stubbed and the computed value read from its single arg.
// ===========================================================================================
#[test]
fn glue_789ba0_cursor_value() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let w = world(&mut emu);
    let (tramp, cell) = float_getter(&mut emu);
    let thumb = emu.alloc(0x100);
    let vtable = emu.alloc(0x100);
    emu.write_u32(thumb + 0x24, vtable as u32);
    emu.write_u32(vtable + 0x1c, tramp as u32);
    emu.write_u32(vtable + 0x20, tramp as u32);
    emu.stub(0x789930, 0, 1); // SetValue — capture its float arg (ret 4 → 1 stack arg)
    emu.stub(0x76b320, 0, 1); // bounds finalize (ret 4 → 1 stack arg)

    // param2 is a pointer-to-struct whose fields [+0x24]/[+0x28] are the target pixel (read by
    // `fsubr [edi+0x24]` / `fsub [edi+0x28]`). edi = param2.
    let pstruct = emu.alloc(0x40);

    let run = |emu: &mut Emu, orient: u32, hi: f32, lo: f32, g: f32, p: f32| -> f32 {
        emu.write_u32(w.this + F_FLAGS, 0x8); // bit3 set (the `test [+0x314],8` gate)
        emu.write_u32(w.this + F_THUMB, thumb as u32);
        emu.write_u32(w.this + F_ORIENT, orient);
        emu.write_u32(w.this + F_MIN, 0.0f32.to_bits());
        emu.write_u32(w.this + F_SPAN, 10.0f32.to_bits());
        emu.write_u32(w.this + F_TRACK_HI_H, hi.to_bits());
        emu.write_u32(w.this + F_TRACK_LO_H, lo.to_bits());
        emu.write_u32(w.this + F_TRACK_HI_V, hi.to_bits());
        emu.write_u32(w.this + F_TRACK_LO_V, lo.to_bits());
        // param2 struct: [+0x24]=p (horiz pixel), [+0x28]=p (vert pixel).
        emu.write_u32(pstruct + 0x24, p.to_bits());
        emu.write_u32(pstruct + 0x28, p.to_bits());
        emu.write_u32(cell, g.to_bits());
        emu.clear_calls();
        emu.call(0x789ba0, w.this as u32, 0, &[pstruct as u32]);
        let c = emu.calls();
        let sv = c
            .iter()
            .find(|sc| sc.addr == 0x789930)
            .expect("0x789930 called");
        f32::from_bits(sv.args[0])
    };

    // predict: g1==g2==g (getter called twice on same state), min=0, span=10.
    let predict = |orient: u32, hi: f32, lo: f32, g: f32, p: f32| -> f32 {
        let (numer, den) = if orient == 0 {
            pos_horiz(hi, lo, g, g, p)
        } else {
            pos_vert(hi, lo, g, g, p)
        };
        pos_to_value(numer, den, 0.0, 10.0)
    };

    let mut total = 0u32;
    for (orient, hi, lo, g, p) in [
        (0u32, 100.0f32, 0.0f32, 10.0f32, 50.0f32),
        (1, 100.0, 0.0, 10.0, 50.0),
        (0, 256.0, 16.0, -3.0, 128.0),
        (1, 256.0, 16.0, -3.0, 128.0),
        (0, 50.0, 0.0, 25.0, 0.0),    // p==0
        (0, 100.0, 100.0, 5.0, 50.0), // hi==lo → trk 0 → den = -g
    ] {
        let got = run(&mut emu, orient, hi, lo, g, p);
        let want = predict(orient, hi, lo, g, p);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "pos o={orient} hi={hi} lo={lo} g={g} p={p}"
        );
        total += 1;
    }
    let mut st = 0x789b_a000u32;
    for _ in 0..5000 {
        let orient = (next(&mut st) >> 8) % 2;
        let hi = rng(&mut st, -500.0, 500.0);
        let lo = rng(&mut st, -500.0, 500.0);
        let g = rng(&mut st, -500.0, 500.0);
        let p = rng(&mut st, -1000.0, 1000.0);
        let got = run(&mut emu, orient, hi, lo, g, p);
        let want = predict(orient, hi, lo, g, p);
        assert_eq!(got.to_bits(), want.to_bits(), "pos fuzz o={orient}");
        total += 1;
    }
    eprintln!("glue_789ba0_cursor_value: {total} bit-exact (value = range·(numer/den) + min)");
}

// ===========================================================================================
// 0x789a60 — what the STEP WRITE does downstream, executed end-to-end on the binary's own
// bytes. NOT a bit-exact diff (there is no Rust counterpart to compare): a behaviour pin: what the
// primitive's signature cannot express is ungated prose and must be pinned by a test that
// observes the CONSEQUENCE. `0x7898f0` and `0x789930` run for real; only the script invoke
// `0x7026f0` is stubbed, so the OnValueChanged fire is observable. Every case is paired with a
// control configuration in which it must NOT fire.
// ===========================================================================================
#[test]
fn glue_789a60_step_write_requantizes_and_fires() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let w = world(&mut emu);
    // 0x7026f0 is cdecl — the caller pops (`add esp,0x14` at 0x789a49), so pop_args = 0.
    emu.stub(0x7026f0, 0, 0);

    struct Out {
        value: f32,
        span: f32,
        step: f32,
        flags: u32,
        fires: usize,
    }
    let mut run = |flags: u32, min: f32, span: f32, value: f32, handler: u32, step: f32| -> Out {
        emu.write_u32(w.this + F_FLAGS, flags);
        emu.write_u32(w.this + F_MIN, min.to_bits());
        emu.write_u32(w.this + F_SPAN, span.to_bits());
        emu.write_u32(w.this + F_VALUE, value.to_bits());
        emu.write_u32(w.this + F_STEP, 0.0f32.to_bits());
        emu.write_u32(w.this + F_CALLBACK, handler);
        emu.clear_calls();
        emu.call(0x789a60, w.this as u32, 0, &[step.to_bits()]);
        Out {
            value: f32::from_bits(emu.read_u32(w.this + F_VALUE)),
            span: f32::from_bits(emu.read_u32(w.this + F_SPAN)),
            step: f32::from_bits(emu.read_u32(w.this + F_STEP)),
            flags: emu.read_u32(w.this + F_FLAGS),
            fires: emu.calls().iter().filter(|c| c.addr == 0x7026f0).count(),
        }
    };

    // (1) bit1 (range-valid) + bit2 (has-a-value), a value OFF the new step's lattice: the step
    //     write re-quantizes it (3 -> 4 on a step-4 lattice anchored at min=0) and FIRES.
    let o = run(0x6, 0.0, 10.0, 3.0, 1, 4.0);
    assert_eq!(o.step.to_bits(), 4.0f32.to_bits(), "step field written");
    assert_eq!(
        o.value.to_bits(),
        4.0f32.to_bits(),
        "value re-quantized by the step write"
    );
    assert_eq!(o.fires, 1, "OnValueChanged invoke fired");
    assert_eq!(o.flags, 0x7, "bits 0|1|2 set");

    // (2) control — same, but the value is already ON the lattice: no move, no fire.
    let o = run(0x6, 0.0, 10.0, 4.0, 1, 4.0);
    assert_eq!(o.value.to_bits(), 4.0f32.to_bits());
    assert_eq!(o.fires, 0, "an on-lattice value must not re-fire");

    // (3) control — bit2 clear (no value yet), which is the state every shipped caller is in at
    //     its first SetValueStep: the nested SetValue is skipped entirely, value untouched.
    let o = run(0x2, 0.0, 10.0, 3.0, 1, 4.0);
    assert_eq!(
        o.value.to_bits(),
        3.0f32.to_bits(),
        "bit2 clear ⇒ value untouched"
    );
    assert_eq!(o.fires, 0);
    assert_eq!(o.flags, 0x3, "bit0 set via 0x7898f0's `or edx,3`");

    // (4) control — bit1 clear: the step field is written and NOTHING else, bit0 included.
    let o = run(0x0, 0.0, 10.0, 3.0, 1, 4.0);
    assert_eq!(o.step.to_bits(), 4.0f32.to_bits());
    assert_eq!(o.value.to_bits(), 3.0f32.to_bits());
    assert_eq!(o.span.to_bits(), 10.0f32.to_bits(), "range untouched");
    assert_eq!(o.flags, 0x0, "bit0 NOT set when bit1 is clear");
    assert_eq!(o.fires, 0);

    // (5) the range round trip `span' = f32(f32(span+min) - min)` is not the identity: with a
    //     large min the step write COLLAPSES the range to zero.
    let o = run(0x2, 1.0, 1e-8, 1.0, 1, 4.0);
    assert_eq!(
        o.span.to_bits(),
        0.0f32.to_bits(),
        "range collapsed by the re-push"
    );

    // (6) control for (5) — min = 0 (every stock scroll bar) round-trips exactly.
    let o = run(0x2, 0.0, 1e-8, 0.0, 1, 4.0);
    assert_eq!(
        o.span.to_bits(),
        1e-8f32.to_bits(),
        "min=0 ⇒ exact round trip"
    );

    eprintln!("glue_789a60_step_write: 6 cases (3 fire/no-fire pairs) observed on WoW.exe bytes");
}
