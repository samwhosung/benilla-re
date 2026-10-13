//! A model's animation (`systems/models/animation.md`): stretches of `0x7121a0`, `0x712910`,
//! `0x719370` and `0x718960` run as slices or islands of the exe's own bytes, checked, and
//! `0x7127f0` called whole.

#![allow(clippy::too_many_arguments)] // a case takes every field it writes
#![allow(clippy::needless_range_loop)] // index loops follow the `+off + 4·k` layout

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::animation::{
    abs_int_scale, add2, anim_fade, anim_fade_resets, bone_emit_frame, bone_timed_value,
    camera_xform, count_scale, field_scale, half_pi, keyframe_ratio, keyframe_timing,
    m2_set_bone_timed, particle_bone_xform, rate_reciprocal, ribbon_bone_xform, BoneTiming,
    COUNT_SCALE, KEYFRAME_EPS, PI,
};
use common::{
    assert_f32, assert_f64, frand, fuzz_i32, fuzz_scales, island_bytes_match, next, read_f64, Lcg,
    SLICE_ESP,
};

/// A finite `f32` over six ranges, one of them `±1e-5` round the rate gate.
fn fuzz_gate(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,
        1 => (m * 20.0 - 10.0) as f32,
        2 => (m * 2000.0 - 1000.0) as f32,
        3 => (m * 1.0e6) as f32,
        4 => (m * 2.0e-5 - 1.0e-5) as f32, // straddles the gate band ±KEYFRAME_EPS
        _ => (m * 1.0e-6) as f32,
    }
}

/// The bone at `esi`, the instance at `[ebp-0x4]`, its record at `+0x2c`.
struct KfScene {
    obj: u64,   // esi
    this: u64,  // [ebp-0x4]
    model: u64, // [this+0x2c]
    frame: u64, // ebp base
}

/// `0x7121a0` from `0x7126d2` to `0x71279c`, `__ftol` run, with `edx`, `eax`, `edi` set so the
/// length is `q`; the bone's `+0x10` to `+0x1c` read back.
fn kf_run(
    e: &mut Emu,
    sc: &KfScene,
    param6: f32,
    type16: i16,
    i14: i32,
    q: u32,
    frametime: i32,
    ctx: &str,
) {
    let ebp = sc.frame + 0x100;
    // [this+0x2c] -> model ; [model+0xc] = frametime.
    e.write_u32(sc.this + 0x2c, sc.model as u32);
    e.write_u32(sc.model + 0xc, frametime as u32);
    // stack locals the slice reads.
    e.write_u32(ebp.wrapping_sub(0x4), sc.this as u32); // [ebp-0x4] = this
    e.write_u32(ebp + 0x18, param6.to_bits()); // [ebp+0x18] = param6
    e.write_u32(ebp + 0x14, i14 as u32); // [ebp+0x14] = i14 (fild dword, signed)
    e.write_u32(ebp + 0x10, 0); // [ebp+0x10] cleared @7126c6 (before slice start)
    e.write_u32(ebp.wrapping_sub(0x8), 0); // [ebp-0x8] = eax-reload src ([esi+0xc]); diff-irrelevant
    e.write_u32(ebp.wrapping_sub(0x14), (type16 as u16 as u32) << 16); // [ebp-0x12] = local_16 (high word)

    // Registers at 0x7126d2: edx/eax/edi feed `edx_after = (edx − eax)·edi → [ebp+0x8] → qword Q`.
    // Force edi=1, eax=0, edx=q so the in-slice integer prep yields Q == q exactly.
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.set_reg(RegisterX86::ESI, sc.obj);
    e.set_reg(RegisterX86::EDX, u64::from(q));
    e.set_reg(RegisterX86::EDI, 1);
    e.set_reg(RegisterX86::EAX, 0);

    // run the keyframe-timing region; the two `call 0x40a2b0` (__ftol) execute inline. Halt at the
    // `je 0x7127de` BB boundary (0x71279c), PAST both f32 stores and both integer-delta stores.
    e.run(0x7126d2, 0x71279c);

    let bin_value = e.read_f32(sc.obj + 0x18);
    let bin_ratio = e.read_f32(sc.obj + 0x1c);
    let bin_start = e.read_u32(sc.obj + 0x10) as i32;
    let bin_end = e.read_u32(sc.obj + 0x14) as i32;

    let ours = keyframe_timing(param6, type16, i14, q, frametime);
    assert_f32(bin_value, ours.value, &format!("{ctx} value"));
    assert_f32(bin_ratio, ours.ratio, &format!("{ctx} ratio"));
    assert_eq!(
        bin_start, ours.start,
        "{ctx} start: bin={bin_start} ours={}",
        ours.start
    );
    assert_eq!(
        bin_end, ours.end,
        "{ctx} end: bin={bin_end} ours={}",
        ours.end
    );
}

/// 9.9999997e-6 (`0x80c5c8`).
const BONE_EPS: f32 = f32::from_bits(0x3727_c5ac);

/// A finite `f32` over six ranges from 1e-4 to 1e6, both signs.
fn fuzz_spine(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    let v = match pick {
        0 => m * 2.0 - 1.0,            // [-1, 1)
        1 => m * 200.0 - 100.0,        // moderate, signed
        2 => m * 2.0e4 - 1.0e4,        // large
        3 => (m * 2.0 - 1.0) * 1.0e-4, // tiny
        4 => (m * 2.0 - 1.0) * 1.0e6,  // huge
        _ => m * 8.0 - 4.0,            // ~[-4,4) signed mid-range
    };
    v as f32
}

/// Sixteen draws of [`fuzz_spine`].
fn fuzz_mat(s: &mut u32) -> [f32; 16] {
    let mut m = [0.0f32; 16];
    for slot in &mut m {
        *slot = fuzz_spine(s);
    }
    m
}

/// One of `0x718960`'s bone transforms from `begin` to `until`: the matrix at `[ebp-0x78]`, the
/// offset at `eax+0x8`, the translation at `[ebp-0x48]`, which is read back.
fn run_bone_xform(
    e: &mut Emu,
    rec: u64,
    this: u64,
    ebp: u64,
    begin: u64,
    until: u64,
    m: &[f32; 16],
    v: [f32; 3],
    t: [f32; 3],
    expect: [f32; 3],
    ctx: &str,
) {
    for (i, mi) in m.iter().enumerate() {
        e.write_u32(ebp - 0x78 + 4 * i as u64, mi.to_bits()); // matrix scratch (movsd already done)
    }
    for k in 0..3u64 {
        e.write_u32(rec + 0x8 + 4 * k, v[k as usize].to_bits()); // [eax+0x8/0xc/0x10] = v
    }
    e.write_u32(ebp - 0x48, t[0].to_bits()); // translation accumulator (row 0)
    e.write_u32(ebp - 0x44, t[1].to_bits());
    e.write_u32(ebp - 0x40, t[2].to_bits());
    e.set_reg(RegisterX86::EAX, rec);
    e.set_reg(RegisterX86::EBX, this); // [ebx+0x2c] read for in-slice arg setup
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.set_reg(RegisterX86::EBP, ebp);
    e.run(begin, until);
    let bin = [
        e.read_f32(ebp - 0x48),
        e.read_f32(ebp - 0x44),
        e.read_f32(ebp - 0x40),
    ];
    for k in 0..3 {
        assert_f32(bin[k], expect[k], &format!("{ctx} row[{k}]"));
    }
}

/// `0x7121a0` from `0x712352` to `0x7123a2` in an island with `fstp qword` appended, and on to the
/// `test` at `0x7123ad` with `mov [out],eax`: the count at `[ebp+0x10]`, the rate and weight at
/// `ebx+0x104`. Exactly 0, 0.5 and 1, a subnormal of either sign, and NaN probe the compares.
#[test]
fn anim_fade_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const SRC: u64 = 0x712352;
    const LEN: usize = 0x7123a2 - 0x712352;
    let out = e.alloc(0x10);
    let isl = e.alloc(0x100);
    let mut code = e.read(SRC, LEN);
    code.extend_from_slice(&[0xdd, 0x1d]);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl, SRC, LEN);
    let end = isl + code.len() as u64;
    // The same stretch on to the `test ah,0x41` of the compare with 0.5, with `mov [out],eax`.
    const GATE_LEN: usize = 0x7123ad - 0x712352;
    let gate_out = e.alloc(0x10);
    let gate_isl = e.alloc(0x100);
    let mut gate = e.read(SRC, GATE_LEN);
    gate.push(0xa3);
    gate.extend_from_slice(&(gate_out as u32).to_le_bytes());
    e.write(gate_isl, &gate);
    island_bytes_match(&e, gate_isl, SRC, GATE_LEN);
    let gate_end = gate_isl + gate.len() as u64;

    let buf = e.alloc(0x200);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, n: i32, rate: f32, weight: f32, ctx: &str| {
        let ours = anim_fade(n, rate, weight);
        for (isl, end) in [(isl, end), (gate_isl, gate_end)] {
            e.write_u32(ebp + 0x10, n as u32);
            e.write_u32(buf + 0x104, rate.to_bits());
            e.write_u32(buf + 0x108, weight.to_bits());
            e.set_reg(RegisterX86::EBX, buf);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(isl, end);
        }
        assert_f64(read_f64(e, out), ours, ctx);
        // `jp` skips the copy on even parity of `ah & 0x41`.
        let ah = (e.read_u32(gate_out) >> 8) & 0x41;
        let resets = !ah.count_ones().is_multiple_of(2);
        assert_eq!(resets, anim_fade_resets(ours), "{ctx} resets");
    };

    // ==threshold / clamp-boundary probes. t = n·rate.
    run_case(&mut e, 0, 0.1, 1.0, "fade t=0 (smoothstep 0)"); // t=0 → 0
    run_case(
        &mut e,
        10,
        0.1,
        1.0,
        "fade t=1 exact (clamp/smoothstep == 1)",
    ); // t=1.0
    run_case(&mut e, 5, 0.1, 1.0, "fade t=0.5 interior"); // t=0.5 → 0.5
    run_case(&mut e, -3, 0.1, 1.0, "fade t<0 (clamp to 0)"); // t<0
    run_case(&mut e, 20, 0.1, 1.0, "fade t=2 (clamp to 1)"); // t>1
    run_case(&mut e, 5, 0.1, 0.0, "fade weight=0"); // fade*0 = 0
    run_case(&mut e, 5, -0.1, 1.0, "fade t<0 via neg rate"); // t=-0.5
    run_case(&mut e, 7, 0.0, 2.5, "fade rate=0 (t=0)"); // t=0
    run_case(&mut e, 5, 0.1, 1.0, "fade 0.5 at the compare");
    run_case(&mut e, 1, 0.5, 1.0, "t exactly 0.5");
    run_case(&mut e, 1, f32::NAN, 1.0, "NaN rate");
    run_case(&mut e, 1, 1.0, f32::NAN, "NaN weight");
    run_case(&mut e, 1, f32::from_bits(1), 1.0, "t the least subnormal");
    run_case(
        &mut e,
        -1,
        f32::from_bits(1),
        1.0,
        "t the least negative subnormal",
    );

    let mut s = 0x7121_5252u32;
    for i in 0..4000u32 {
        // n across a span that straddles the [0,1] clamp band for the chosen rate.
        let n = (next(&mut s) as i32) % 4000 - 2000;
        let rate = frand(&mut s, -0.01, 0.01);
        let weight = fuzz_scales(&mut s);
        run_case(&mut e, n, rate, weight, &format!("fade fuzz[{i}]"));
    }
}

/// `0x7121a0` from `0x712718` to `0x712739` after `fld dword`, `fstp dword` appended. Exactly
/// 9.9999997e-6, a step either side, zero and NaN probe the gate.
#[test]
fn keyframe_ratio_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // Island: `fld dword [p_slot]`, then the exe's gate
    // 0x712718..0x712739 = `fld st(0); fabs; fcomp [eps]; fnstsw; test ah,0x41; jne 0x712733;
    // (fld [1.0]; fdiv st,st(1); jmp 0x712739); 0x712733: fld [0.0]` — leaving the ratio LIVE in
    // st0 (p stays at st1). We append `fstp dword [out]` to capture the ratio. Only the MIDDLE
    // (binary) bytes are byte-verified == WoW.exe; the prefix/suffix are capture scaffolding.
    const SRC: u64 = 0x712718;
    const LEN: usize = 0x712739 - 0x712718; // 0x21 = 33
    let p_slot = e.alloc(0x10);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x100);

    let mut code: Vec<u8> = Vec::new();
    code.extend_from_slice(&[0xd9, 0x05]); // fld dword [p_slot]
    code.extend_from_slice(&(p_slot as u32).to_le_bytes());
    let mid = e.read(SRC, LEN);
    let mid_off = code.len() as u64;
    code.extend_from_slice(&mid);
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    // verify ONLY the copied binary span (offset by the 6-byte fld prefix).
    island_bytes_match(&e, isl + mid_off, SRC, LEN);
    let end = isl + code.len() as u64;

    let run_case = |e: &mut Emu, p: f32, ctx: &str| {
        e.write_u32(p_slot, p.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(isl, end);
        let bin = e.read_f32(out);
        let ours = keyframe_ratio(p);
        assert_f32(bin, ours, ctx);
    };

    // ==threshold probes around the STRICT `|p| > eps` gate.
    let eps = KEYFRAME_EPS;
    run_case(&mut e, eps, "p == eps (must be 0.0, NOT divide)"); // |p| == eps → 0.0
    run_case(&mut e, -eps, "p == -eps (0.0)");
    run_case(
        &mut e,
        f32::from_bits(eps.to_bits() + 1),
        "p just-above eps (divides)",
    ); // |p| > eps
    run_case(
        &mut e,
        f32::from_bits(eps.to_bits() - 1),
        "p just-below eps (0.0)",
    ); // |p| < eps
    run_case(&mut e, 0.0, "p == 0 (0.0)");
    run_case(&mut e, -0.0, "p == -0 (0.0)");
    run_case(&mut e, 1.0, "p == 1 (1.0)");
    run_case(&mut e, -2.0, "p == -2 (-0.5, signed)");
    run_case(&mut e, f32::NAN, "p NaN (must be 0.0)"); // NaN → C0=C3=1 → 0.0
    run_case(&mut e, f32::INFINITY, "p +inf (1/inf = 0.0)");
    run_case(&mut e, 1.0e-4, "p 1e-4 (> eps, divides)");
    run_case(&mut e, 1.0e-6, "p 1e-6 (< eps, 0.0)");

    let mut s = 0x7126_1818u32;
    for i in 0..4000u32 {
        // bias toward the gate band so both legs are well-exercised.
        let p = match (next(&mut s) >> 8) % 4 {
            0 => frand(&mut s, -2.0e-5, 2.0e-5), // straddles eps
            1 => frand(&mut s, -1.0, 1.0),
            2 => fuzz_scales(&mut s),
            _ => frand(&mut s, -1000.0, 1000.0),
        };
        run_case(&mut e, p, &format!("keyframe fuzz[{i}] p={p}"));
    }
}

/// `0x7121a0` from `0x71260c` to `0x712623`: the count at `[ebp-0x10]`, the rate read from
/// `ebx+0x104`.
#[test]
fn rate_reciprocal_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let buf = e.alloc(0x200); // EBX: 1/count stored at +0x104
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    // 0x71260c `fild qword [ebp-0x10]`; 0x71260f `fdivr [1.0]`; 0x712615 `jmp 0x71261d`;
    // 0x71261d `fstp [ebx+0x104]` — halt at 0x712623 (PAST the store; the `mov [ebx+0x108],imm`
    // BB boundary). count is the i64 at [ebp-0x10] (high dword 0).
    let run_case = |e: &mut Emu, count: u32, ctx: &str| {
        e.write_u32(ebp - 0x10, count); // [ebp-0x10] = count (low dword)
        e.write_u32(ebp - 0xc, 0); // [ebp-0xc] = 0 (high dword)
        e.set_reg(RegisterX86::EBX, buf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x71260c, 0x712623);
        let bin = e.read_f32(buf + 0x104);
        let ours = rate_reciprocal(count);
        assert_f32(bin, ours, ctx);
    };

    // ==threshold / exact probes: powers of two (exact reciprocals), 1, large.
    run_case(&mut e, 1, "1/1 = 1");
    run_case(&mut e, 2, "1/2 = 0.5");
    run_case(&mut e, 4, "1/4");
    run_case(&mut e, 3, "1/3 (rounded)");
    run_case(&mut e, 1000, "1/1000");
    run_case(&mut e, 0x7fff_ffff, "1/INT_MAX");

    let mut s = 0x7126_0c0cu32;
    for i in 0..4000u32 {
        let count = (next(&mut s) & 0x000f_ffff).max(1); // non-zero span (the int guard precedes)
        run_case(&mut e, count, &format!("rate fuzz[{i}] count={count}"));
    }
}

/// [`kf_run`] for the four types, the rate gate at, above and under 9.9999997e-6, NaN, and fuzz.
#[test]
fn keyframe_timing_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x200);
    let this = e.alloc(0x100);
    let model = e.alloc(0x100);
    let frame = e.alloc(0x200);
    let sc = KfScene {
        obj,
        this,
        model,
        frame,
    };

    // ---- the four type-dispatch legs (local_16 ∈ {0,1,2,3}) — fchs + 0.0-replacement. ----
    kf_run(&mut e, &sc, 0.5, 0, 100, 50, 1000, "type0 (V=P)");
    kf_run(&mut e, &sc, 0.5, 1, 100, 50, 1000, "type1 (V=-P, fchs)");
    kf_run(&mut e, &sc, 0.5, 2, 100, 50, 1000, "type2 (G=0)");
    kf_run(&mut e, &sc, 0.5, 3, 100, 50, 1000, "type3 (fchs then G=0)");
    kf_run(&mut e, &sc, -0.5, 0, 100, 50, 1000, "type0 neg-P");
    kf_run(&mut e, &sc, -0.5, 1, 100, 50, 1000, "type1 neg-P (fchs→+)");

    // ---- the STRICT |G| > eps gate: ==eps boundary (must NOT divide → ratio 0.0) +
    // just-above/below. ----
    let eps = KEYFRAME_EPS;
    kf_run(
        &mut e,
        &sc,
        eps,
        0,
        30,
        7,
        500,
        "G == eps (ratio MUST be 0.0)",
    );
    kf_run(
        &mut e,
        &sc,
        -eps,
        1,
        30,
        7,
        500,
        "G == eps via fchs(-eps) (0.0)",
    );
    kf_run(
        &mut e,
        &sc,
        f32::from_bits(eps.to_bits() + 1),
        0,
        30,
        7,
        500,
        "G just-above eps (divides)",
    );
    kf_run(
        &mut e,
        &sc,
        f32::from_bits(eps.to_bits() - 1),
        0,
        30,
        7,
        500,
        "G just-below eps (0.0)",
    );
    kf_run(&mut e, &sc, 0.0, 0, 30, 7, 500, "G == 0 (0.0)");
    kf_run(&mut e, &sc, -0.0, 0, 30, 7, 500, "G == -0 (0.0)");
    kf_run(
        &mut e,
        &sc,
        f32::NAN,
        0,
        30,
        7,
        500,
        "G NaN (ratio 0.0, value NaN)",
    );
    kf_run(
        &mut e,
        &sc,
        f32::INFINITY,
        0,
        30,
        7,
        500,
        "G +inf (1/inf = 0.0)",
    );

    // ---- divide cases: signed ratio, big/overflowing __ftol products, negative i14, unsigned q.
    // ----
    kf_run(&mut e, &sc, 2.0, 0, 30, 7, 500, "G=2 → ratio 0.5");
    kf_run(
        &mut e,
        &sc,
        -2.0,
        0,
        30,
        7,
        500,
        "G=-2 → ratio -0.5 (signed)",
    );
    kf_run(&mut e, &sc, 0.3, 0, -77, 999, 12000, "neg i14");
    kf_run(
        &mut e,
        &sc,
        0.123,
        0,
        100000,
        2_000_000_000,
        0,
        "big q (i32 overflow path)",
    );
    kf_run(
        &mut e,
        &sc,
        0.001,
        0,
        1,
        0xC000_0000,
        0,
        "q high-bit set (UNSIGNED qword)",
    );
    kf_run(
        &mut e,
        &sc,
        0.001,
        0,
        1,
        0xFFFF_FFFF,
        0,
        "q = 0xffffffff (unsigned 4.29e9)",
    );
    kf_run(
        &mut e,
        &sc,
        1.0e-4,
        0,
        30,
        7,
        500,
        "G 1e-4 (> eps, divides)",
    );
    kf_run(&mut e, &sc, 1.0e-6, 0, 30, 7, 500, "G 1e-6 (< eps, 0.0)");

    // ---- fuzz: random param6 across the gate band + all type legs + random i14/q/frametime. ----
    let mut s = 0x626f_6e65u32; // "bone"
    for i in 0..6000u32 {
        let param6 = fuzz_gate(&mut s);
        let type16 = ((next(&mut s) >> 8) % 5) as i16; // 0..4 incl. an out-of-range 4 (V=P, no zero)
        let i14 = next(&mut s) as i32 % 4_000_000 - 2_000_000;
        let q = next(&mut s);
        let frametime = next(&mut s) as i32 % 100_000;
        kf_run(
            &mut e,
            &sc,
            param6,
            type16,
            i14,
            q,
            frametime,
            &format!("kf fuzz[{i}] p6={param6} t={type16}"),
        );
    }
}

/// `0x7127f0` whole on an instance whose key bone plays: the stamps read from the bone's `+0xa8`.
/// A product past 2^63 and NaN probe `__ftol`.
#[test]
fn m2_set_bone_timed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x100);
    let skel = e.alloc(0x200); // S = [skel+0x130]
    let s_obj = e.alloc(0x100); // S: needs +0x20 (bone array), +0x34 (count gate)
    let rec = e.alloc(0x200); // D = [this+0x90], the 0x118-stride record (index 0)
    let bone = e.alloc(0x100); // B = [S+0x20], the 0x44-stride bone array (index r=0)
    let model = e.alloc(0x40); // [this+0x2c]; base = [model+0xc]

    // fixed wiring (index r = remap[0] = 0).
    e.write_u32(this + 0x10, 1); // take the interesting leg
    e.write_u32(this + 0x30, skel as u32);
    e.write_u32(skel + 0x130, s_obj as u32);
    e.write_u32(s_obj + 0x34, 1); // count > index(0) → don't bail
    e.write_u32(s_obj + 0x20, bone as u32);
    e.write_u32(this + 0x90, rec as u32);
    e.write_u32(rec + 0xa4, 0); // remap[0] = 0 (must != -1)
    e.write_u32(this + 0x2c, model as u32);

    // Keep the products inside the i64-representable band so __ftol (x87 round-toward-zero) and our
    // truncation agree: |scale|·|frame| and |span0·stride|·|scale| stay well under 2^31.
    let run_case = |e: &mut Emu,
                    scale: f32,
                    frame: i32,
                    base: i32,
                    span0: i32,
                    stride: i32,
                    ctx: &str| {
        e.write_u32(rec + 0xb4, scale.to_bits());
        e.write_u32(rec + 0xbc, stride as u32);
        // span0 = [B+0x44*0+8] - [B+0x44*0+4] = src8 - src4. Choose src4=0, src8=span0.
        e.write_u32(bone + 4, 0);
        e.write_u32(bone + 8, span0 as u32);
        e.write_u32(model + 0xc, base as u32);
        // junk-seed the outputs so a non-write diverges.
        e.write_u32(rec + 0xa8, 0xdead_beef);
        e.write_u32(rec + 0xac, 0x1234_5678);

        e.call(0x7127f0, this as u32, 0, &[(-1i32) as u32, frame as u32]);
        let bin = BoneTiming {
            a8: e.read_u32(rec + 0xa8) as i32,
            ac: e.read_u32(rec + 0xac) as i32,
        };
        let ours = m2_set_bone_timed(scale, frame, base, span0, stride);
        assert_eq!(
            bin, ours,
            "{ctx}: binary={:?} ours={:?} (scale={scale} frame={frame} base={base} span0={span0} stride={stride})",
            bin, ours
        );
    };

    // structured: zero scale, negative scale (fabs), zero span, sign mixes, exact products.
    run_case(&mut e, 0.0, 100, 1000, 50, 3, "zero-scale");
    run_case(&mut e, -2.5, 10, 500, 7, 4, "neg-scale (fabs)");
    run_case(&mut e, 1.0, 0, 0, 0, 0, "all-zero");
    run_case(&mut e, 1.5, -20, -100, -8, -5, "negatives");
    run_case(&mut e, 2.0, 17, 333, 9, 11, "exact-ish");
    run_case(&mut e, -0.0, 5, 42, 3, 2, "neg-zero scale");
    run_case(&mut e, 1.0e30, 5, 42, 3, 2, "past 2^63");
    run_case(&mut e, f32::NAN, 5, 42, 3, 2, "NaN scale");

    let mut s = 0x7127_f0f0u32;
    for _ in 0..3000u32 {
        // scale a modest float (incl. negative for fabs); ints kept small so products stay in band.
        let scale = {
            let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
            (m * 8.0 - 4.0) as f32 // [-4, 4)
        };
        let frame = (next(&mut s) >> 16) as i32 % 4096 - 2048;
        let base = (next(&mut s) >> 8) as i32 % 1_000_000 - 500_000;
        let span0 = (next(&mut s) >> 16) as i32 % 2048 - 1024;
        let stride = (next(&mut s) >> 18) as i32 % 512 - 256;
        run_case(&mut e, scale, frame, base, span0, stride, "fuzz");
    }
}

/// `0x712910` from `0x7129db` to `0x712a7c`: the bone at `esi`, the track at `[edi+0x20]`; the
/// bone's `+0xa8` to `+0xb4` read back. The rate gate at, above and under 9.9999997e-6, and NaN.
#[test]
fn bone_timed_value_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let entry = e.alloc(0x200); // ESI → the bone-anim entry; reads +0xa8/0xb0/0xb8/0xbc, writes +0xa8..+0xb4
    let track = e.alloc(0x40); // EDI → the track entry; reads +0x4/+0x8
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    // run_case takes the eight slice inputs (binary terms) and the float `p` (param_3).
    let run_case = |e: &mut Emu,
                    time: i32,
                    a0: i32,
                    scale: f32,
                    b8: i32,
                    bc: i32,
                    t4: i32,
                    t8: i32,
                    p: f32,
                    ctx: &str| {
        e.write_u32(entry + 0xa8, a0 as u32);
        e.write_u32(entry + 0xb0, scale.to_bits());
        e.write_u32(entry + 0xb8, b8 as u32);
        e.write_u32(entry + 0xbc, bc as u32);
        // junk-seed the outputs that are not pre-read (ac, b4) so a non-write would diverge.
        e.write_u32(entry + 0xac, 0x1111_2222);
        e.write_u32(entry + 0xb4, 0x3333_4444);
        e.write_u32(track + 0x4, t4 as u32);
        e.write_u32(track + 0x8, t8 as u32);
        e.set_reg(RegisterX86::ESI, entry);
        e.set_reg(RegisterX86::EDI, track);
        e.set_reg(RegisterX86::EBX, time as u32 as u64); // ebx = current time (int)
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.write_u32(ebp + 0xc, p.to_bits()); // [ebp+0xc] = param_3 (p)
        e.run(0x7129db, 0x712a7c); // delta·scale + gate + 2×|ratio| scale; halt at `pop edi`

        let ours = bone_timed_value(time, a0, scale, b8, bc, t4, t8, p);
        assert_eq!(
            e.read_u32(entry + 0xa8) as i32,
            ours.start,
            "{ctx} start (+0xa8): binary={:#x} ours={:#x}",
            e.read_u32(entry + 0xa8),
            ours.start as u32
        );
        assert_eq!(
            e.read_u32(entry + 0xac) as i32,
            ours.end,
            "{ctx} end (+0xac): binary={:#x} ours={:#x}",
            e.read_u32(entry + 0xac),
            ours.end as u32
        );
        assert_eq!(
            e.read_u32(entry + 0xb0),
            ours.param_bits,
            "{ctx} param (+0xb0): binary={:#x} ours={:#x}",
            e.read_u32(entry + 0xb0),
            ours.param_bits
        );
        assert_f32(
            e.read_f32(entry + 0xb4),
            ours.ratio,
            &format!("{ctx} ratio (+0xb4)"),
        );
    };

    // ---- gate ==threshold / boundary probes (the STRICT `>` on |p|, NaN→default) ----
    // p == +ε exactly  → |p|==ε → C3 set → default 0.0 (NOT divided).
    run_case(&mut e, 100, 0, 1.0, 0, 1, 0, 10, BONE_EPS, "gate p==+eps");
    run_case(&mut e, 100, 0, 1.0, 0, 1, 0, 10, -BONE_EPS, "gate p==-eps");
    // p just below ε  → |p|<ε → C0 set → default 0.0.
    run_case(
        &mut e,
        100,
        0,
        1.0,
        0,
        1,
        0,
        10,
        f32::from_bits(BONE_EPS.to_bits() - 1),
        "gate p<eps",
    );
    // p just above ε  → |p|>ε → divide path (1.0/p).
    run_case(
        &mut e,
        100,
        0,
        1.0,
        0,
        1,
        0,
        10,
        f32::from_bits(BONE_EPS.to_bits() + 1),
        "gate p>eps",
    );
    run_case(
        &mut e,
        100,
        0,
        1.0,
        0,
        1,
        0,
        10,
        2.0,
        "gate p=2 (ratio 0.5)",
    );
    run_case(&mut e, 100, 0, 1.0, 0, 1, 0, 10, -4.0, "gate p=-4");
    // p == NaN  → unordered → C3,C2,C0 all set → default 0.0 (NOT divided, NOT inf).
    run_case(&mut e, 100, 0, 1.0, 0, 1, 0, 10, f32::NAN, "gate p=NaN");
    run_case(&mut e, 100, 0, 1.0, 0, 1, 0, 10, 0.0, "gate p=0 (default)");
    // p == ±Inf → |p|>ε true → 1.0/±Inf = ±0.0 (ordered).
    run_case(
        &mut e,
        100,
        0,
        1.0,
        0,
        1,
        0,
        10,
        f32::INFINITY,
        "gate p=+Inf",
    );
    run_case(
        &mut e,
        100,
        0,
        1.0,
        0,
        1,
        0,
        10,
        f32::NEG_INFINITY,
        "gate p=-Inf",
    );

    // ---- __ftol edge probes: products near/over i32 range, exact, negative ----
    run_case(&mut e, 0, 0, 0.0, 0, 0, 0, 0, 3.0, "ftol all-zero");
    run_case(&mut e, 1_000_000, 0, 1.5, 7, 3, 2, 9, 4.0, "ftol mid");
    run_case(&mut e, 5, 0, 1.0e12, 0, 1, 0, 1, 2.0, "ftol huge-prod"); // delta·scale overflows i32

    let mut rng = Lcg(0x7129_1000);
    for i in 0..6000u32 {
        let time = rng.i32_mixed();
        let a0 = rng.i32_mixed();
        let scale = rng.f32_mixed();
        let b8 = rng.i32_mixed();
        let bc = rng.i32_mixed();
        let t4 = rng.i32_mixed();
        let t8 = rng.i32_mixed();
        // bias p across the gate band: half the cases land |p| near ε to stress the boundary.
        let p = if rng.next() & 1 == 0 {
            rng.f32_mixed()
        } else {
            let s = if rng.next() & 1 == 0 { 1.0 } else { -1.0 };
            s * BONE_EPS * (0.5 + (rng.next() >> 8) as f32 / f32::from(1u16 << 8))
        };
        run_case(
            &mut e,
            time,
            a0,
            scale,
            b8,
            bc,
            t4,
            t8,
            p,
            &format!("btv fuzz[{i}]"),
        );
    }
}

/// `0x719370` from `0x71952c` to `0x719543` and from `0x719554` to `0x71956c`, `__ftol` run; the
/// result read from `eax`. NaN and a product past 2^63 give 0.
#[test]
fn bone_emit_frame_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let edi = e.alloc(0x200); // EDI → the bone entry; reads +0xa8 (a8), +0xb0 (scale f32), +0xb8 (b8)
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    // Site 1 (fmul@719538): clean ftol slice [0x71952c,0x719543) — sub eax,[edi+0xa8]; fild; fmul
    // [edi+0xb0]; call __ftol. EAX = ftol((bound−a8)·scale). Then +b8 (int) → bone_emit_frame.
    let run_site1 = |e: &mut Emu, bound: i32, a8: i32, scale: f32, b8: i32, ctx: &str| {
        e.write_u32(edi + 0xa8, a8 as u32);
        e.write_u32(edi + 0xb0, scale.to_bits());
        e.write_u32(edi + 0xb8, b8 as u32);
        e.set_reg(RegisterX86::EAX, bound as u32 as u64); // eax = bound (local_c)
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x71952c, 0x719543); // sub; fild; fmul; __ftol(inline); halt PAST the call
        let bin_full = (e.get_reg(RegisterX86::EAX) as u32 as i32).wrapping_add(b8);
        let ours = bone_emit_frame(bound, a8, scale, b8);
        assert_eq!(
            bin_full, ours,
            "{ctx} site1: binary={bin_full:#x} ours={:#x}",
            ours as u32
        );
    };

    // Site 2 (fmul@719561): clean ftol slice [0x719554,0x71956c) — sub eax,ecx(=a8); fild; fmul
    // [edi+0xb0]; call __ftol. eax=bound, ecx=a8. The interleaved `add ebx,edx` reads ebx/edx
    // (harmless to the ftol/eax result); set them 0.
    let run_site2 = |e: &mut Emu, bound: i32, a8: i32, scale: f32, b8: i32, ctx: &str| {
        e.write_u32(edi + 0xb0, scale.to_bits());
        e.set_reg(RegisterX86::EAX, bound as u32 as u64);
        e.set_reg(RegisterX86::ECX, a8 as u32 as u64); // ecx = a8 (the sub's rhs)
        e.set_reg(RegisterX86::EBX, 0);
        e.set_reg(RegisterX86::EDX, 0);
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x719554, 0x71956c); // sub eax,ecx; mov [ebp-0x34]; fild; (add ebx,edx); fmul; __ftol
        let bin_full = (e.get_reg(RegisterX86::EAX) as u32 as i32).wrapping_add(b8);
        let ours = bone_emit_frame(bound, a8, scale, b8);
        assert_eq!(
            bin_full, ours,
            "{ctx} site2: binary={bin_full:#x} ours={:#x}",
            ours as u32
        );
    };

    // Exact / edge probes.
    for &(bound, a8, sc, b8, name) in &[
        (0i32, 0i32, 1.0f32, 0i32, "all-zero"),
        (100, 10, 1.0, 0, "delta 90 ×1.0"),
        (100, 10, 0.5, 0, "delta 90 ×0.5 (trunc 45)"),
        (10, 100, 1.0, 0, "negative delta"),
        (7, 0, 2.5, 1000, "×2.5 +b8"),
        (5, 0, 1.0e12, 0, "huge prod (i32 wrap)"),
        (100, 0, -1.5, -7, "neg scale + neg b8"),
        (3, 5, 0.0, 42, "scale 0 (prod 0)"),
        (3, 5, f32::NAN, 42, "NaN scale"),
        (1, 0, 1.0e30, 42, "past 2^63"),
    ] {
        run_site1(&mut e, bound, a8, sc, b8, name);
        run_site2(&mut e, bound, a8, sc, b8, name);
    }

    let mut s = 0x7195_3800u32;
    for i in 0..6000u32 {
        let bound = fuzz_i32(&mut s);
        let a8 = fuzz_i32(&mut s);
        let scale = fuzz_scales(&mut s);
        let b8 = fuzz_i32(&mut s);
        run_site1(&mut e, bound, a8, scale, b8, &format!("bef fuzz[{i}]"));
        run_site2(&mut e, bound, a8, scale, b8, &format!("bef fuzz[{i}]"));
    }
}

/// `0x719370` from `0x719724` to `0x719746` and from `0x719844` to `0x719866`, `__ftol` run. NaN
/// and a product past 2^63 give 0.
#[test]
fn abs_int_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let eax_struct = e.alloc(0x200); // EAX → the bone struct; f32 `f` at +0xb4 (the fld source)
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    // Site 1 (fabs@719735+fimul@71973e): [0x719724,0x719746) — fld [eax+0xb4]; ...; sub
    // ecx,[ebp-0x2c]; mov [ebp-0x44],ecx; fabs; fimul dword [ebp-0x44]; call __ftol. n = [ebp-4] −
    // [ebp-0x2c].
    let run_site1 = |e: &mut Emu, f: f32, n: i32, ctx: &str| {
        // pick lhs/rhs so lhs − rhs == n (control the int operand exactly).
        let lhs = n; // [ebp-4]
        let rhs = 0i32; // [ebp-0x2c]
        e.write_u32(eax_struct + 0xb4, f.to_bits());
        e.write_u32(ebp.wrapping_sub(4), lhs as u32); // [ebp-4]
        e.write_u32(ebp.wrapping_sub(0x2c), rhs as u32); // [ebp-0x2c]
        e.set_reg(RegisterX86::EAX, eax_struct);
        e.set_reg(RegisterX86::ECX, lhs as u32 as u64); // mov ecx,[ebp-4] reloads this; preset matches
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x719724, 0x719746); // fld; fabs; fimul [n]; __ftol(inline); halt PAST the call
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        let ours = abs_int_scale(f, n);
        assert_eq!(
            bin, ours,
            "{ctx} site1: binary={bin:#x} ours={:#x}",
            ours as u32
        );
    };

    // Site 2 (fabs@719855+fimul@71985e): start at the `fld` (0x719844, skipping the
    // `mov eax,[ebp-0x30]` struct-base reload that site 1 also has — we preset EAX directly, as in
    // site 1). fld [eax+0xb4]; mov ecx,[ebp-0x2c]; sub ecx,[ebp-4]; mov [ebp-0x4c],ecx; fabs;
    // fimul dword [ebp-0x4c]; call __ftol. n = [ebp-0x2c] − [ebp-4] (reverse subtraction).
    let run_site2 = |e: &mut Emu, f: f32, n: i32, ctx: &str| {
        let lhs = n; // [ebp-0x2c]
        let rhs = 0i32; // [ebp-4]
        e.write_u32(eax_struct + 0xb4, f.to_bits());
        e.write_u32(ebp.wrapping_sub(0x2c), lhs as u32); // [ebp-0x2c]
        e.write_u32(ebp.wrapping_sub(4), rhs as u32); // [ebp-4]
        e.set_reg(RegisterX86::EAX, eax_struct);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x719844, 0x719866); // fld; fabs; fimul [n]; __ftol(inline); halt PAST the call
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        let ours = abs_int_scale(f, n);
        assert_eq!(
            bin, ours,
            "{ctx} site2: binary={bin:#x} ours={:#x}",
            ours as u32
        );
    };

    // Exact / edge probes: |f|·n with sign flips (fimul reads n as SIGNED i32), zero, huge,
    // denormal.
    for &(f, n, name) in &[
        (0.0f32, 0i32, "0·0"),
        (1.0, 1, "1·1"),
        (2.5, 4, "2.5·4 = 10"),
        (-2.5, 4, "|-2.5|·4 = 10 (fabs)"),
        (2.5, -4, "2.5·(-4) = -10 (signed n)"),
        (-3.0, -3, "|-3|·(-3) = -9"),
        (1.5, 1_000_000, "1.5·1e6"),
        (1.0e12, 5, "huge f (i32 wrap)"),
        (1.0e30, 5, "past 2^63"),
        (f32::NAN, 5, "NaN"),
        (0.9999999, 7, "near-1 ·7"),
        (f32::from_bits(1), 1000, "denormal f ·1000"),
    ] {
        run_site1(&mut e, f, n, name);
        run_site2(&mut e, f, n, name);
    }

    let mut s = 0x7197_3500u32;
    for i in 0..6000u32 {
        let f = fuzz_scales(&mut s);
        let n = fuzz_i32(&mut s);
        run_site1(&mut e, f, n, &format!("ais fuzz[{i}]"));
        run_site2(&mut e, f, n, &format!("ais fuzz[{i}]"));
    }
}

/// `0x718960` from `0x718a7e` to `0x718aed`: the camera at `eax`, the scene matrix through
/// `[ebx+0x2c]`; read from `[ebp-0x34]`.
#[test]
fn camera_xform_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let cam = e.alloc(0x40); // eax: camera record, src vector at [eax]/[eax+4]/[eax+8]
    let this = e.alloc(0x100); // ebx
    let matbuf = e.alloc(0x80); // the matrix; [ebx+0x2c] := matbuf-0xdc so [ebx+0x2c]+0xdc==matbuf
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    // [ebx+0x2c] must point so that ([ebx+0x2c]+0xdc) == matbuf (the binary does mov
    // eax,[ebx+0x2c]; add eax,0xdc, then reads m[i] at [eax+4i]).
    e.write_u32(this + 0x2c, (matbuf - 0xdc) as u32);

    let run_case = |e: &mut Emu, src: [f32; 3], m: &[f32; 16], ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(cam + 4 * k, src[k as usize].to_bits());
        }
        for (i, mi) in m.iter().enumerate() {
            e.write_u32(matbuf + 4 * i as u64, mi.to_bits());
        }
        e.set_reg(RegisterX86::EAX, cam);
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x718a7e, 0x718aed); // fld;fchs;fstp ×3; (fld;fmul;...;faddp)×3 → [ebp-0x34/-0x30/-0x2c]
        let bin = [
            e.read_f32(ebp - 0x34),
            e.read_f32(ebp - 0x30),
            e.read_f32(ebp - 0x2c),
        ];
        let ours = camera_xform(src, m);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} out[{k}]"));
        }
    };

    // probes: identity 3×3 (out = -src), zero src (out = 0 / -0 via fchs), exacts.
    let ident = {
        let mut m = [0.0f32; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m
    };
    run_case(&mut e, [1.0, 2.0, 3.0], &ident, "identity (out=-src)");
    run_case(&mut e, [0.0, 0.0, 0.0], &ident, "zero src (fchs -0.0)");
    run_case(&mut e, [-0.0, 1.0, -2.0], &ident, "signed-zero src");

    let mut s = 0x7189_6060u32;
    for i in 0..6000u32 {
        let src = [fuzz_spine(&mut s), fuzz_spine(&mut s), fuzz_spine(&mut s)];
        let m = fuzz_mat(&mut s);
        run_case(&mut e, src, &m, &format!("cam fuzz[{i}]"));
    }
}

/// `0x718960` from `0x718b66` to `0x718ba9` and from `0x718bcc` to `0x718c03`: the three sums of
/// each.
#[test]
fn light_translate_adds_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x400); // ebx ([ebx+0x3c4] = the base record ptr; [ebx+0x2c] read for arg setup)
    let anim = e.alloc(0x80); // edi (anim record: +0x2c/0x30/0x34 pos, +0x54/0x58/0x5c color)
    let base = e.alloc(0x80); // [ebx+0x3c4]: base record (+0xc/0x10/0x14 pos, +0x3c/0x40/0x44 color)
    let scratch = e.alloc(0x80); // [ebx+0x2c] target for the in-slice arg-setup adds (harmless)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    e.write_u32(this + 0x3c4, base as u32);
    e.write_u32(this + 0x2c, scratch as u32);

    // Part 1 (0x718b66..0x718ba9): pos[i] = anim[0x2c+4i] + base[0xc+4i] → [ebp-0x34/-0x30/-0x2c].
    let run_pos = |e: &mut Emu, ap: [f32; 3], bp: [f32; 3], ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(anim + 0x2c + 4 * k, ap[k as usize].to_bits());
            e.write_u32(base + 0xc + 4 * k, bp[k as usize].to_bits());
        }
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::EDI, anim);
        e.set_reg(RegisterX86::ESI, 0); // esi=[ebp-0x8] preset 0 at slice entry
        e.set_reg(RegisterX86::EDX, 0); // edx=[ebp-0x4] preset 0
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x718b66, 0x718ba9); // (fld anim; fadd base; fstp)×3, halt before call 0x7bca80
        let bin = [
            e.read_f32(ebp - 0x34),
            e.read_f32(ebp - 0x30),
            e.read_f32(ebp - 0x2c),
        ];
        for k in 0..3 {
            let ours = add2(ap[k], bp[k]);
            assert_f32(bin[k], ours, &format!("{ctx} pos[{k}]"));
        }
    };

    // Part 2 (0x718bcc..0x718c03): col[i] = base[0x3c+4i] + anim[0x54+4i] → [ebp-0x24/-0x20/-0x1c].
    let run_col = |e: &mut Emu, bc: [f32; 3], ac: [f32; 3], ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(base + 0x3c + 4 * k, bc[k as usize].to_bits());
            e.write_u32(anim + 0x54 + 4 * k, ac[k as usize].to_bits());
        }
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::EDI, anim);
        e.set_reg(RegisterX86::ESI, base); // esi = base record ([ebx+0x3c4]+edx, edx=0)
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x718bcc, 0x718c03); // (fld base; fadd anim; fstp)×3, halt before call 0x7bca80
        let bin = [
            e.read_f32(ebp - 0x24),
            e.read_f32(ebp - 0x20),
            e.read_f32(ebp - 0x1c),
        ];
        for k in 0..3 {
            let ours = add2(bc[k], ac[k]);
            assert_f32(bin[k], ours, &format!("{ctx} col[{k}]"));
        }
    };

    // probes: exact cancellation (a + -a = 0), zeros, signed zero.
    run_pos(&mut e, [1.0, 2.0, 3.0], [-1.0, -2.0, -3.0], "pos cancel");
    run_pos(&mut e, [0.0, -0.0, 5.0], [0.0, 0.0, 0.0], "pos zeros");
    run_col(&mut e, [1.5, 2.5, 3.5], [-1.5, 0.0, 0.5], "col mix");

    let mut s = 0x718b_6666u32;
    for i in 0..3000u32 {
        run_pos(
            &mut e,
            [fuzz_spine(&mut s), fuzz_spine(&mut s), fuzz_spine(&mut s)],
            [fuzz_spine(&mut s), fuzz_spine(&mut s), fuzz_spine(&mut s)],
            &format!("pos fuzz[{i}]"),
        );
        run_col(
            &mut e,
            [fuzz_spine(&mut s), fuzz_spine(&mut s), fuzz_spine(&mut s)],
            [fuzz_spine(&mut s), fuzz_spine(&mut s), fuzz_spine(&mut s)],
            &format!("col fuzz[{i}]"),
        );
    }
}

/// `0x718960` from `0x718c78` to `0x718c95`: the count at `[ebp-0x20]`, read from `[ebp-0x38]`.
#[test]
fn count_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200); // ebx (in-slice `mov [ebx+0x88],eax` clears the accumulator)
    let rec = e.alloc(0x200); // ecx=[ebp-0x14] (in-slice `mov edx,[ecx+0x134]` read)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    e.write_u32(ebp - 0x14, rec as u32); // [ebp-0x14] = the MD20 record ptr (ecx loaded just before)

    let run_case = |e: &mut Emu, count: i32, ctx: &str| {
        // [ebp-0x20] lo = count, [ebp-0x1c] hi = 0 (the binary zero-extends [ebx+0x88] to i64).
        e.write_u32(ebp - 0x20, count as u32);
        e.write_u32(ebp - 0x1c, 0);
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::ECX, rec);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x718c78, 0x718c95); // fild [ebp-0x20]; fmul [0x801360]; fstp [ebp-0x38]
        let bin = e.read_f32(ebp - 0x38);
        let ours = count_scale(i64::from(count));
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 0, "0");
    run_case(&mut e, 1, "1");
    run_case(&mut e, 1000, "1000");
    run_case(&mut e, 0x7fff_ffff, "i32::MAX");
    let mut s = 0x718c_7878u32;
    for i in 0..3000u32 {
        let count = (next(&mut s) >> 8) as i32 & 0x00ff_ffff; // non-negative emit counts
        run_case(&mut e, count, &format!("count fuzz[{i}]"));
    }
}

/// `0x718960` from `0x718cf0` to `0x718cfd`: the product pushed for `0x7b7b10`.
#[test]
fn field_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emitter = e.alloc(0x80); // esi (field at +0x3c)
    let this = e.alloc(0x200); // ebx (gscale at +0x19c)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, field: f32, gscale: f32, ctx: &str| {
        e.write_u32(emitter + 0x3c, field.to_bits());
        e.write_u32(this + 0x19c, gscale.to_bits());
        e.set_reg(RegisterX86::ESI, emitter);
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // fld [esi+0x3c]; push ecx (esp-=4); fmul [ebx+0x19c]; fstp [esp] — halt before call
        // 0x7b7b10.
        e.run(0x718cf0, 0x718cfd);
        let bin = e.read_f32(SLICE_ESP - 4); // fstp dword [esp], esp moved by the in-slice push ecx
        let ours = field_scale(field, gscale);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 0.0, 5.0, "0*5");
    run_case(&mut e, 1.0, 1.0, "1*1");
    run_case(&mut e, 2.5, -4.0, "signed");
    let mut s = 0x718c_f0f0u32;
    for i in 0..3000u32 {
        run_case(
            &mut e,
            fuzz_spine(&mut s),
            fuzz_spine(&mut s),
            &format!("field fuzz[{i}]"),
        );
    }
}

/// [`run_bone_xform`] from `0x718d6d` to `0x718dd0`.
#[test]
fn ribbon_bone_xform_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x40); // eax (v at +0x8/0xc/0x10)
    let this = e.alloc(0x80); // ebx
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    e.write_u32(this + 0x2c, this as u32); // [ebx+0x2c] benign (in-slice push-arg setup)

    let run = |e: &mut Emu, m: &[f32; 16], v: [f32; 3], t: [f32; 3], ctx: &str| {
        let expect = ribbon_bone_xform(m, v, t);
        run_bone_xform(e, rec, this, ebp, 0x718d6d, 0x718dd3, m, v, t, expect, ctx);
    };

    let ident = {
        let mut m = [0.0f32; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m
    };
    run(
        &mut e,
        &ident,
        [1.0, 2.0, 3.0],
        [10.0, 20.0, 30.0],
        "ident (out=v+t)",
    );
    run(&mut e, &ident, [0.0; 3], [0.0; 3], "zeros");
    let mut s = 0x718d_6d6du32;
    for i in 0..6000u32 {
        let m = fuzz_mat(&mut s);
        let v = [fuzz_spine(&mut s), fuzz_spine(&mut s), fuzz_spine(&mut s)];
        let t = [fuzz_spine(&mut s), fuzz_spine(&mut s), fuzz_spine(&mut s)];
        run(&mut e, &m, v, t, &format!("ribbon fuzz[{i}]"));
    }
}

/// [`run_bone_xform`] from `0x7190a9` to `0x71910c`, with an input on which the two first-row
/// orders round apart.
#[test]
fn particle_bone_xform_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x40); // eax (v at +0x8/0xc/0x10)
    let this = e.alloc(0x80); // ebx
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    e.write_u32(this + 0x2c, this as u32);

    let run = |e: &mut Emu, m: &[f32; 16], v: [f32; 3], t: [f32; 3], ctx: &str| {
        let expect = particle_bone_xform(m, v, t);
        run_bone_xform(e, rec, this, ebp, 0x7190a9, 0x71910f, m, v, t, expect, ctx);
    };

    let ident = {
        let mut m = [0.0f32; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m
    };
    run(
        &mut e,
        &ident,
        [1.0, 2.0, 3.0],
        [10.0, 20.0, 30.0],
        "ident (out=v+t)",
    );
    run(&mut e, &ident, [0.0; 3], [0.0; 3], "zeros");

    // An input where the ribbon's first-row order ((m4·vy+m0·vx)+m8·vz) and the emitter's
    // ((m0·vx+m8·vz)+m4·vy) round differently. Catastrophic-cancel set: m0·vx and m8·vz nearly
    // cancel, m4·vy is the small term whose add-position flips the rounding.
    let mut m = [0.0f32; 16];
    m[0] = 1.0;
    m[8] = -1.0;
    m[4] = f32::from_bits(0x3380_0000); // ~5.96e-8 (sub-ULP relative to ~1.0)
    m[1] = 1.0;
    m[5] = 1.0;
    m[9] = 1.0;
    m[2] = 1.0;
    m[6] = 1.0;
    m[10] = 1.0;
    let v = [
        f32::from_bits(0x3f80_0001), // 1+ULP
        1.0,
        f32::from_bits(0x3f80_0001),
    ];
    run(&mut e, &m, v, [0.0; 3], "row0-order cancel probe");

    let mut s = 0x7190_a9a9u32;
    for i in 0..6000u32 {
        let mm = fuzz_mat(&mut s);
        let vv = [fuzz_spine(&mut s), fuzz_spine(&mut s), fuzz_spine(&mut s)];
        let tt = [fuzz_spine(&mut s), fuzz_spine(&mut s), fuzz_spine(&mut s)];
        run(&mut e, &mm, vv, tt, &format!("particle fuzz[{i}]"));
    }
}

/// `0x718960` from `0x719114` to `0x71912d`: the product pushed for `0x7bdd60`.
#[test]
fn half_pi_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.set_reg(RegisterX86::EBP, ebp);
    // fld [0x81203c](π); fmul [0x7ffa24](0.5); fstp [esp] — halt at the int-stores @0x71912d. Three
    // in-slice pushes precede the store (push 0x1 @71911a, push ecx @719125, push ecx @719126), so
    // at `fstp [esp]`@71912a ESP == SLICE_ESP-12 and the f32 lands there.
    e.run(0x719114, 0x71912d);
    let bin = e.read_f32(SLICE_ESP - 12);
    let ours = half_pi();
    assert_f32(bin, ours, "pi*0.5");
}

/// The constants the models use, read from the exe.
#[test]
fn update_after_animate_constants_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let e = Emu::new(&wow);
    assert_eq!(e.read_u32(0x801360), COUNT_SCALE.to_bits());
    assert_eq!(e.read_u32(0x7ffa24), 0.5f32.to_bits());
    assert_eq!(e.read_u32(0x81203c), PI.to_bits());
    assert_eq!(e.read_u32(0x80c5c8), KEYFRAME_EPS.to_bits());
}
