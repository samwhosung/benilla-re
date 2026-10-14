//! Reimplements the arithmetic of the map arrow frames `PositionWorldMapArrowFrame` (`0x4a8d20`)
//! and `PositionMiniWorldMapArrowFrame` (`0x4a8f20`) and of the pending-mail counters
//! `HandleReceivedMailPacket` (`0x4ad620`) and `UpdatePendingMail` (`0x4ade60`). The x87 runs at
//! 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests
//! in `tests/worldmap_arrow.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact; cited by VA).
// ===========================================================================================

/// `0x7ffd68` (`.rdata`) = `1024.0` (`0x44800000`), the metric scale-divide denominator factor
/// (`fmul [0x7ffd68]`) in `0x4a8d20`/`0x4a8f20`.
pub const SCALE1024_BITS: u32 = 0x4480_0000; // 1024.0
/// `0x832a4c` (`.data`, runtime-init UI scale; default `1.0`) = the scale global loaded by the
/// helper `0x41ad70` (`fld [0x832a4c]; ret`) at the head of each metric block. The difftest feeds
/// chosen values directly into the diffed slice's operand.
pub const SCALE_GLOBAL_DEFAULT_BITS: u32 = 0x3f80_0000; // 1.0 (default)
/// `0x8029d4` (`.rdata`) = `2^-22` = `2.384185791015625e-07` (`0x34800000`), the `fabs(x)`-vs-this
/// epsilon threshold of `0x4ad620`/`0x4ade60` (`fcomp [0x8029d4]`).
pub const EPS_BITS: u32 = 0x3480_0000; // 2^-22
/// `0x7ffd74` (`.rdata`) = `0.0` (`0x00000000`), the zero bound in `0x4ad620`/`0x4ade60`
/// (`fcomp/fcom [0x7ffd74]`).
pub const ZERO_BITS: u32 = 0x0000_0000; // 0.0

#[inline]
fn scale1024() -> f64 {
    f64::from(f32::from_bits(SCALE1024_BITS))
}
#[inline]
fn eps() -> f64 {
    f64::from(f32::from_bits(EPS_BITS))
}

// ===========================================================================================
// (1) 0x4a8d20 / 0x4a8f20 — the world-map arrow metric scale-divide kernel.
// ===========================================================================================

/// The per-coordinate owned slice of `PositionWorldMapArrowFrame` (`0x4a8ea0`..`0x4a8eb2` for X,
/// `0x4a8ec9`..`0x4a8edb` for Y; the MINI clone `0x4a8f20` is byte-identical here):
/// `q = value / (scale · 1024.0)`.
///
/// Disasm per coordinate (e.g. X, `0x4a8e98`..`0x4a8eb7`):
/// ```text
/// call 0x6f3620          ; st0 = Lua number arg   (DELEGATED: Lua-stack getter, fld qword)
/// fstp dword [ebp-8]     ; save the coord value as f32
/// call 0x41ad70          ; st0 = [0x832a4c]       (DELEGATED: scale-global load, fld;ret)
/// fmul dword [0x7ffd68]  ; st0 = scale · 1024.0          <-- OWNED
/// push ecx               ; reserve the logical_to_device arg slot
/// fdivr dword [ebp-8]    ; st0 = value / (scale·1024.0)  <-- OWNED (mem / st0)
/// fstp dword [esp]       ; push the quotient as f32
/// call 0x41ae60          ; · screen-scale  (DELEGATED: logical_to_device, · [0x832a44])
/// fstp dword [ebp-8]     ; store the device-space coord
/// ```
/// The owned ops are `fmul [1024.0]` then `fdivr` — `fdivr [m]` computes `[m] / st0`, i.e.
/// `value / (scale·1024.0)`. The value enters as an `f32` (`fstp` then reload), and the quotient is
/// `fstp`'d to `f32` before the delegated `logical_to_device`. Both the divide's memory operand and
/// its result narrow at `f32`; the PC_53 divide itself is `f64`.
///
/// `value` = the parsed Lua coordinate (the `f32` in `[ebp-8]`/`[ebp-4]`); `scale` = the runtime UI
/// scale global `0x832a4c` (default `1.0`).
pub fn arrow_scale_divide(value: f32, scale: f32) -> f32 {
    let denom = f64::from(scale) * scale1024(); // fmul [0x7ffd68]
    (f64::from(value) / denom) as f32 // fdivr [ebp-N] then fstp dword
}

/// `0x4a8f20` `PositionMiniWorldMapArrowFrame` — a clean clone of `0x4a8d20` (byte-identical modulo
/// masked rel32/disp32/imm32), so its owned arithmetic is identical. Provided as a named alias so
/// the MINI row maps to its own primary test; the body is `arrow_scale_divide`.
#[inline]
pub fn mini_arrow_scale_divide(value: f32, scale: f32) -> f32 {
    arrow_scale_divide(value, scale)
}

// ===========================================================================================
// (2) 0x4ad620 — clamp/threshold "set value" glue (`ret 8`).
// ===========================================================================================

/// `0x4ad620`'s owned arithmetic + branch ladder, modelling the value it leaves in the global
/// `[0x845eac]` (`cur`) for a given input `arg = [ebp+0xc]`, given the prior global value `cur`
/// and the busy-flag pair `[0xb6ef88]|[0xb6ef8c]`.
///
/// Control flow (`0x4ad62f`..`0x4ad6aa`; the leading `call 0x419130` is a delegated buffer-append
/// with no effect on this state):
/// ```text
/// if ([0xb6ef88] | [0xb6ef8c]) != 0:  [0xb6efcc] = 1;  return cur unchanged   ; busy path
/// fld [ebp+0xc]; fabs; fcomp [0x8029d4]=eps; test ah,5; jp 0x4ad679           ; |arg| vs eps
///   ; jp NOT taken  ⇔ |arg| < eps  → store arg into [0x845eac], call 0x703e50(0x1f4)
///   ; jp taken      ⇔ |arg| >= eps → 0x4ad679:
/// fld [0x845eac]=cur; fcomp [0x7ffd74]=0.0; test ah,5; jnp 0x4ad69c          ; cur vs 0.0
///   ; jnp taken ⇔ cur < 0  → store arg
///   ; jnp NOT taken (cur >= 0) → 0x4ad68c:
/// fld [ebp+0xc]=arg; fcomp [0x845eac]=cur; test ah,5; jp 0x4ad6a4            ; arg vs cur
///   ; jp taken ⇔ arg >= cur → leave cur unchanged
///   ; jp NOT taken (arg < cur) → 0x4ad69c: store arg
/// ```
/// Net new `[0x845eac]`:
/// * busy (`flags != 0`) → `cur` unchanged (and `[0xb6efcc]` set to 1).
/// * `|arg| < eps` → `arg`.
/// * `|arg| >= eps` and `cur < 0` → `arg`.
/// * `|arg| >= eps` and `cur >= 0` and `arg < cur` → `arg`.
/// * `|arg| >= eps` and `cur >= 0` and `arg >= cur` → `cur` unchanged.
///
/// The ONLY owned FP-arith op is `fabs` (`0x4ad64e`); the three `fcomp` are compares (not arith).
/// `arg` enters as an `f32`; `cur` is the stored `f32`. Returns the new value stored in `[0x845eac]`
/// (bit-exact `f32`). `busy` = `(flags != 0)`.
pub fn set_value(arg: f32, cur: f32, busy: bool) -> f32 {
    if busy {
        return cur; // [0xb6efcc]=1; ret with cur unchanged
    }
    // fld arg; fabs; fcomp eps. jp NOT taken ⇔ |arg| < eps.
    let abs_arg = f64::from(arg).abs(); // fabs (OWNED), kept f64 for the PC_53 compare
    if abs_arg < eps() {
        return arg; // |arg| < eps → store arg
    }
    // |arg| >= eps. cur < 0 ⇒ store arg.
    if f64::from(cur) < 0.0 {
        return arg;
    }
    // cur >= 0: arg < cur ⇒ store arg, else leave cur.
    if f64::from(arg) < f64::from(cur) {
        arg
    } else {
        cur
    }
}

// ===========================================================================================
// (3) 0x4ade60 — paired "step toward value" glue (`ret 4`).
// ===========================================================================================

/// `0x4ade60`'s owned arithmetic + branch ladder, modelling the value it leaves in the global
/// `[0x845eac]` (`cur`) for a step amount `arg = [ebp+0x8]`.
///
/// Control flow (`0x4ade63`..`0x4adec0`):
/// ```text
/// fld [0x845eac]=cur; fcomp [0x7ffd74]=0.0; test ah,0x41; jnp 0x4adebf       ; cur vs 0.0
///   ; jnp taken ⇔ cur <= 0 → return cur unchanged (no FP step)
///   ; jnp NOT taken (cur > 0) → 0x4ade76:
/// fld [0x845eac]=cur; fsub [ebp+0x8]=arg; fcom [0x7ffd74]=0.0; test ah,0x41; jne 0x4ade94
///   ; st0 = cur - arg ; jne taken ⇔ (cur-arg) <= 0 → discard, [0x845eac]=0
///   ; jne NOT taken ((cur-arg) > 0) → 0x4ade8c: fstp [0x845eac] = (cur-arg)
/// 0x4adea0: fld [0x845eac]; fabs; fcomp [0x8029d4]=eps; test ah,5; jp 0x4adebf ; |new| vs eps
///   ; jp NOT taken ⇔ |new| < eps → call 0x703e50(0x1f4) (notify); value unchanged
/// ```
/// Net new `[0x845eac]`:
/// * `cur <= 0` → `cur` unchanged.
/// * `cur > 0` and `(cur - arg) > 0` → `cur - arg`.
/// * `cur > 0` and `(cur - arg) <= 0` → `0.0`.
///
/// The owned FP-arith ops are `fsub` (`0x4ade7c`, `cur - arg`) and `fabs` (`0x4adea6`, over the
/// updated global — feeds only the notify outcome, not the stored value). The `fcomp`/`fcom` are
/// compares. The subtraction's operand and result narrow at the `fstp dword [0x845eac]` to `f32`.
/// `cur`/`arg` enter as `f32`. Returns the new value stored in `[0x845eac]`.
pub fn step_value(cur: f32, arg: f32) -> f32 {
    // cur vs 0.0: cur <= 0 ⇒ unchanged.
    if f64::from(cur) <= 0.0 {
        return cur;
    }
    // cur > 0: st0 = cur - arg, then compare vs 0.0.
    let diff = f64::from(cur) - f64::from(arg); // fsub (OWNED)
    if diff > 0.0 {
        diff as f32 // fstp dword [0x845eac]
    } else {
        0.0 // fstp st(0); mov [0x845eac], 0
    }
    // the trailing fld;fabs;fcomp(eps) gates ONLY the notify call 0x703e50 — no effect on the value.
}
