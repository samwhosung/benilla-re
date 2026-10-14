//! Reimplements the arithmetic of the duration text and tooltip layout functions
//! `IsWholeDuration` (`0x52f980`), `FormatDurationFloat` (`0x52fbd0`), `GameTooltipClear`
//! (`0x530050`), `GameTooltipSize` (`0x530640`) and the cursor fade (`0x530b20`). The x87 runs at
//! 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests
//! in `tests/tooltip_layout.rs` diff each against the exe.

/// `convert(num)` — the shared scale chain `call 0x41ad70; fmul 1024; fdivr num; fstp [m32];
/// call 0x41ae60`. `g = [0x832a4c]`, `xs = [0x832a44]`. The `fdivr` result is narrowed to f32 at
/// the `fstp [esp]` store (it is the stack arg to `0x41ae60`), then `0x41ae60` does `xs · (that
/// f32)` at f64. Returns the live st0 (`f64`).
pub fn convert(g: f32, xs: f32, num: f32) -> f64 {
    // call 0x41ad70 -> st0=g ; fmul [0x7ffd68]=1024.0 ; fdivr [num] -> num/(g*1024) ; fstp [esp] (f32)
    let t = (f64::from(num) / (f64::from(g) * 1024.0)) as f32;
    // call 0x41ae60 -> st0 = [0x832a44]=xs * t  (f64 product, live in st0)
    f64::from(xs) * f64::from(t)
}

// ===========================================================================================
// 0x52f980 / 0x52fbd0 — time-unit band select (ms -> day/hour/minute/second divisor)
// ===========================================================================================

/// The lazy-init time-unit thresholds (`0x52f98f`..`0x52f9c5` / `0x52fbfb`..`0x52fc31`), built
/// once into `.bss` from the `.rdata` seconds-base constants. Both 0x52f980 and 0x52fbd0 build the
/// identical pair (distinct `.bss` cells):
/// * `day = MS_MIN(60000) · 60 · 24`  — `[0xc0d38c]` / `[0xc0d378]`
/// * `hour = MS_MIN(60000) · 60`      — `[0xc0d384]` / `[0xc0d3f4]`
/// * `minute = MS_MIN(60000)`         — read straight from `[0x808f1c]` / `[0x808f2c]`.
pub const MS_MIN: f32 = 60_000.0; // 0x808f1c / 0x808f2c
pub const MS_HOUR: f64 = 60_000.0 * 60.0; // [0xc0d384] = fld 60000 ; fmul 60.0
pub const MS_DAY: f64 = MS_HOUR * 24.0; // [0xc0d38c] = fld [0xc0d384] ; fmul 24.0
const MILLI: f32 = f32::from_bits(0x3a83_126f); // 0x801360 ~= 0.001 (sub-minute -> seconds)

/// The band-select divisor of both 0x52f980 (`0x52f9c5`..`0x52fa16`) and 0x52fbd0
/// (`0x52fc31`..`0x52fcbe`): pick the largest time-unit whose threshold `<= value` and divide by
/// it; below a minute the value is scaled by `0.001` (ms -> s) instead. The compares are
/// `fcomp value, thr; test ah,0x1; jne next` — `ah&0x1` is C0 (set iff `value < thr`), so the
/// divide fires iff `value >= thr` (NaN ⇒ C0 set ⇒ falls through to the next/seconds band).
///
/// All math is f64 (PC_53, the live x87 stack); the divide result is the live st0 the caller
/// stores (`fstp QWORD [ebp-8]` in 0x52f980; `fstp dword` (f32) in 0x52fbd0, see [`time_band_f32`]).
pub fn time_band(value: f32) -> f64 {
    let v = f64::from(value);
    if v >= MS_DAY {
        v / MS_DAY // fdiv [0xc0d38c]
    } else if v >= MS_HOUR {
        v / MS_HOUR // fdiv [0xc0d384]
    } else if v >= f64::from(MS_MIN) {
        v / f64::from(MS_MIN) // fdiv [0x808f1c]
    } else {
        v * f64::from(MILLI) // fmul [0x801360] (ms -> seconds)
    }
}

/// 0x52fbd0's variant: the same band select, stored `fstp dword [ebp+8]` (f32, `0x52fcbe`) before
/// the sprintf — so it carries the f32 narrowing, and a subsequent NaN-snap (`fcomp [0x7ffd74]=0.0;
/// test ah,0x41; je`, `0x52fccc`) replaces a NaN result with `0.0` (`ah&0x41` = C0|C3, set only on
/// a NaN/unordered compare; `je` keeps the value when NOT NaN). Result is `time_band as f32`, NaN→0.
pub fn time_band_f32(value: f32) -> f32 {
    let r = time_band(value) as f32; // fstp dword [ebp+8]
    if r.is_nan() {
        0.0 // 0x52fcd9: mov [ebp+8], 0
    } else {
        r
    }
}

// ===========================================================================================
// 0x530050 — scale convert (esi+0x34c child layout)
// ===========================================================================================

pub const NUM_NEG2: f32 = f32::from_bits(0xc000_0000); // 0x8038dc = -2.0

/// 0x530050's lone owned slice (`0x53012f`..`0x530155`): `convert([0x8038dc] = -2.0)`. Fed to
/// `0x767c70` (the child layout setter). Just [`convert`] with `num = -2.0`.
pub fn convert_neg2(g: f32, xs: f32) -> f64 {
    convert(g, xs, NUM_NEG2)
}

// ===========================================================================================
// 0x530640 — the layout pass: per-child convert + list accumulate + final compose
// ===========================================================================================

pub const NUM_0640_A: f32 = f32::from_bits(0x4219_999a); // 0x808e70 = 38.4
pub const NUM_0640_B: f32 = f32::from_bits(0x4366_6666); // 0x808e80 = 230.4
pub const NUM_0640_MIN: f32 = f32::from_bits(0x4000_0000); // 0x808e6c = 2.0
pub const NUM_0640_C: f32 = f32::from_bits(0x4123_d70a); // 0x808e74 = 10.24
pub const NUM_0640_D: f32 = f32::from_bits(0x4123_d70a); // 0x808e78 = 10.24

/// (slice 1, `0x530659`..`0x53067d`) the leading `convert` whose numerator is the runtime field
/// `[esi+0x3c0]`: `xs · field / (g · 1024)`, stored `fstp [ebp-8]` (f32). Seeds the accumulator.
pub fn convert_field(g: f32, xs: f32, field: f32) -> f32 {
    convert(g, xs, field) as f32 // fstp dword [ebp-8]
}

/// (slice 2, `0x5306c3`..`0x5306dd`) `convert([0x808e70]=38.4)`, stored `fstp [ebp-4]` (f32).
pub fn convert_base_a(g: f32, xs: f32) -> f32 {
    convert(g, xs, NUM_0640_A) as f32
}

/// (slices `0x5306fb` / `0x53071d` / `0x530752` / `0x5309af` / `0x5309c7`) the running f32
/// accumulator add `acc = (child_term + acc) as f32` (`fadd [ebp-4]; fstp [ebp-4]`). `child_term`
/// is a vtable return (a measured child extent), `acc` the f32-stored running sum.
pub fn acc_add(child_term: f32, acc: f32) -> f32 {
    (f64::from(child_term) + f64::from(acc)) as f32
}

/// (slice 3, `0x5307b1`..`0x5307ce` + the conditional re-eval `0x5307d5`..`0x5307ef`)
/// `convert([0x808e80]=230.4)` — the per-label width budget, evaluated identically on both arms.
pub fn convert_budget_b(g: f32, xs: f32) -> f32 {
    convert(g, xs, NUM_0640_B) as f32
}

/// (slice 4, `0x53099a`..`0x5309b2`) `convert([0x808e6c]=2.0)` — the per-child gap term added to
/// the accumulator (`fadd [ebp-4]; fstp [ebp-4]`) when the child's fade test passes.
pub fn convert_gap_min(g: f32, xs: f32) -> f32 {
    convert(g, xs, NUM_0640_MIN) as f32
}

/// (slice 5, the final compose `0x5309d8`..`0x530a2b`) two converts folded into the placement:
/// * `origin = (2·convert([0x808e74]=10.24) + [esi+0x3bc] + acc8) as f32` (`fadd st,st; fadd
///   [esi+0x3bc]; fadd [ebp-8]; fstp [ebp-14]`).
/// * `size = (2·convert([0x808e78]=10.24) + acc4) as f32` (`fadd st,st; fadd [ebp-4]; fstp
///   [ebp-10]`).
///
/// `f3bc`=`[esi+0x3bc]`, `acc8`=`[ebp-8]`, `acc4`=`[ebp-4]`. All adds f64 (live st0), narrowed
/// only at the two stores.
pub fn compose_final(g: f32, xs: f32, f3bc: f32, acc8: f32, acc4: f32) -> (f32, f32) {
    let c = convert(g, xs, NUM_0640_C);
    let origin = ((c + c) + f64::from(f3bc) + f64::from(acc8)) as f32; // 0x5309f2..0x5309fd
    let d = convert(g, xs, NUM_0640_D);
    let size = ((d + d) + f64::from(acc4)) as f32; // 0x530a20..0x530a2b
    (origin, size)
}

// ===========================================================================================
// 0x530b20 — caret/cursor placement + fade-alpha map
// ===========================================================================================

/// (slice 1, `0x530b68`..`0x530b87`) the cursor-blink box. `0x41ad80` first writes the X/Y-scaled
/// pair `bx = xs·a0` ([ebp-8]), `by = ys·a1` ([ebp-4]) (both f32). Then `recip = 1.0/[esi+0x7c]`
/// (`fld [esi+0x7c]; fdivr [0x7ff9d8]=1.0`, f64), and the two outputs are:
/// * `cx = (bx · recip) as f32` (`fld [ebp-8]; fmul st,st(1); fstp [ebp-8]`).
/// * `cy = (recip · by) as f32` (`fmul [ebp-4]; fstp [ebp-4]`).
pub fn cursor_box(bx: f32, by: f32, inv_src: f32) -> (f32, f32) {
    let recip = 1.0_f64 / f64::from(inv_src); // fld [esi+0x7c] ; fdivr 1.0
    let cx = (f64::from(bx) * recip) as f32; // fmul st,st(1) ; fstp [ebp-8]
    let cy = (recip * f64::from(by)) as f32; // fmul [ebp-4] ; fstp [ebp-4]
    (cx, cy)
}

/// (slice 2, `0x530baa`..`0x530bb3`) the fade-time decrement: `t = ([esi+0x3b8] - dt) as f32`
/// (`fld [esi+0x3b8]; fsub [ebp+8]; fstp [ebp+8]`), stored as the new `[esi+0x3b8]`.
pub fn fade_decrement(prev: f32, dt: f32) -> f32 {
    (f64::from(prev) - f64::from(dt)) as f32 // fsub [ebp+8] ; fstp [ebp+8]
}

pub const FADE_HALF: f64 = 2.0 * 0.5; // [0x808e7c]=2.0 · [0x7ffa24]=0.5 = 1.0
const ALPHA_MAX: f32 = f32::from_bits(0x437f_0000); // 0x7ffe58 = 255.0

/// (slice 3, `0x530bf6`..`0x530c1b`) the fade-alpha map taken on the `t > 0` arm (`0x530bcd`
/// `test ah,0x41; jp`): `half = [0x808e7c]=2.0 · [0x7ffa24]=0.5` (= 1.0), then `fld t; fcomp st(1)`
/// compares `t` vs `half` (`test ah,0x1; jne` — `ah&0x1`=C0, set iff `t < half`). On `t < half`
/// the `fdivr [ebp+8]` computes `t / half` (m/st0, st0=half); otherwise `frac = 1.0`
/// (`fld [0x7ff9d8]`). Then `alpha = __ftol(frac · [0x7ffe58]=255.0)` (truncate toward zero).
pub fn fade_alpha(t: f32) -> i32 {
    let half = FADE_HALF; // fld 2.0 ; fmul 0.5
                          // An unordered compare sets C0 too, so NaN takes the quotient arm (`t >= half` is false).
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let frac = if !(f64::from(t) >= half) {
        f64::from(t) / half // 0x530c18: fdivr [ebp+8] = t/half
    } else {
        1.0 // 0x530c10: fld [0x7ff9d8]=1.0
    };
    let alpha_f = frac * f64::from(ALPHA_MAX); // fmul [0x7ffe58]=255.0
    ftol(alpha_f) // __ftol (0x40a2b0, RC=truncate)
}

pub use benilla_re_cmath::x87::ftol;
