//! Reimplements the arithmetic of `CLayoutFrame::SetPoint` (`0x767c70`), `SetLayoutScale`
//! (`0x768360`), `SetFrameScale` (`0x76ac10`), `PackColorFromFloats` (`0x76f900`) and `0x783380`.
//! The x87 runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword`
//! stores; the tests in `tests/layout_scale.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// norm-ε = 2⁻²² (`DAT_008029d4` = `0x34800000` = 2.384185791015625e-07), the change-detect
/// epsilon shared by 0x767c70 / 0x768360 / 0x76ac10.
pub const EPS_BITS: u32 = 0x3480_0000;
/// 0.0 (`DAT_007ffd74` = `0x00000000`), the 0x768360 reject-zero guard / the 0x783380 numerator
/// sign gate + zero-fallback result.
pub const ZERO_BITS: u32 = 0x0000_0000;
/// 255.0 (`DAT_007ffe58` = `0x437f0000`), the 0x76f900 [0,1]→byte colour scale.
pub const C255_BITS: u32 = 0x437f_0000;
/// 0.5 (`DAT_007ffa24` = `0x3f000000`), the 0x76f900 colour-pack round bias.
pub const HALF_BITS: u32 = 0x3f00_0000;

#[inline]
fn f(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// (1) 0x767c70 — 2× epsilon change-detect `|field − arg| ?< eps` (X and Y).
// ===========================================================================================

/// One change-detect difference of `0x767c70` (`0x767cb6`..`0x767cbc` for X, `0x767ccb`..`0x767cd1`
/// for Y): `fld [field]; fsub [arg]; fabs`. So `diff = |field − arg|`, left LIVE in st0 (no slot
/// store) and compared `fcomp [eps]` un-narrowed → the genuine value is the PC_53 f64. `field` is
/// `[edi+4]`/`[edi+8]` (the existing tooltip-anchor coord), `arg` is `[ebp+0x14]`/`[ebp+0x18]`
/// (the requested coord). Returns the un-rounded f64 `|field − arg|`.
pub fn anchor_diff(field: f32, arg: f32) -> f64 {
    (f64::from(field) - f64::from(arg)).abs()
}

/// `0x767c70` change-detect branch outcome (`fcomp [eps=2⁻²²]; fnstsw; test ah,5; jp/jnp`). For
/// the X test (`0x767cc9 jp 0x767ce0`): `jp` (PF=1) is taken iff EVEN set-bits in `AH & 0x5`
/// (mask {C0,C2}); for the Y test (`0x767cde jnp 0x767d5e`): `jnp` (PF=0) is taken iff ODD set-bits.
/// After `fcomp diff, eps`: `diff < eps` → C0=1 (1 bit, odd); `diff > eps` → 0 bits (even);
/// `diff == eps` → C3=1 (masked 0, even); NaN → C0|C2 (2 bits, even). So `unchanged` (this
/// coordinate already matches within eps) ⟺ `diff < eps` ⟺ ODD parity. Returns `true` ⇒ the
/// coordinate is within eps (the binary treats it as unchanged). NaN ⇒ `false`.
pub fn anchor_unchanged(field: f32, arg: f32) -> bool {
    let d = anchor_diff(field, arg);
    !d.is_nan() && d < f(EPS_BITS)
}

// ===========================================================================================
// (2) 0x768360 — reject-zero guard, then 1× epsilon change-detect (reverse-subtract).
// ===========================================================================================

/// `0x768360`'s reject-zero guard (`0x768363`..`0x768371`): `fld [ebp+8]=arg; fcomp [0.0];
/// fnstsw; test ah,0x44; jp 0x768380`. Mask 0x44 = {C2,C3}; `jp` (PF=1) is taken (→ continue) iff
/// EVEN set-bits. After `fcomp arg, 0.0`: `arg > 0` → 0 bits (even, continue); `arg < 0` → C0
/// (unmasked, 0 masked bits, even, continue); `arg == 0` → C3 (1 masked bit, odd, NOT taken →
/// reject); NaN → C2|C3 (2 bits, even, continue). So the guard rejects EXACTLY `arg == 0.0`.
/// Returns `true` ⇒ accepted (proceed); `false` ⇒ rejected (`arg == 0.0`).
pub fn accept_nonzero(arg: f32) -> bool {
    f64::from(arg) != 0.0
}

/// `0x768360`'s change-detect difference (`0x768387`..`0x76838d`): `fld [ecx+0x58]=stored;
/// fsubr [ebp+8]=arg; fabs`. `fsubr` reverses operands → `st0 = arg − stored`; `fabs` →
/// `|arg − stored|`, left LIVE in st0, compared `fcomp [eps]` un-narrowed. `stored` is `[ecx+0x58]`
/// (the current field), `arg` the new value. Returns the un-rounded f64 `|arg − stored|`.
pub fn rsub_diff(stored: f32, arg: f32) -> f64 {
    (f64::from(arg) - f64::from(stored)).abs()
}

/// `0x768360` change-detect branch (`0x76838f fcomp [eps]; fnstsw; test ah,5; jp 0x7683a2`).
/// Identical mask/parity to `0x767c70`'s X test: `jp` (PF=1) taken (→ the slot already matches,
/// skip the write & return false) iff EVEN parity; falls through (do the write) iff `diff < eps`
/// (C0, odd). So `unchanged` ⟺ `diff < eps` ⟺ ODD parity. Returns `true` ⇒ within eps. NaN ⇒
/// `false`. (`stored`/`arg` as in [`rsub_diff`].)
pub fn rsub_unchanged(stored: f32, arg: f32) -> bool {
    let d = rsub_diff(stored, arg);
    !d.is_nan() && d < f(EPS_BITS)
}

// ===========================================================================================
// (3) 0x76ac10 — 1× epsilon change-detect `|arg − [ecx+0xb8]| ?< eps` (reverse-subtract).
// ===========================================================================================

/// `0x76ac10`'s change-detect difference (`0x76ac1a`..`0x76ac23`): `fld [ecx+0xb8]=stored;
/// fsubr [ebp+8]=arg; fabs` → `|arg − stored|`, LIVE in st0, compared `fcomp [eps]` un-narrowed.
/// Byte-identical kernel to `0x768360`'s second chain modulo the field disp (`[ecx+0xb8]` here vs
/// `[ecx+0x58]`). Returns the un-rounded f64 `|arg − stored|`.
pub fn ac10_diff(stored: f32, arg: f32) -> f64 {
    (f64::from(arg) - f64::from(stored)).abs()
}

/// `0x76ac10` change-detect branch (`0x76ac25 fcomp [eps]; fnstsw; test ah,5; jp 0x76ac38`):
/// same mask/parity as [`rsub_unchanged`]. `unchanged` ⟺ `diff < eps` ⟺ ODD parity. NaN ⇒
/// `false`. Returns `true` ⇒ within eps (the setter skips the write & dispatch).
pub fn ac10_unchanged(stored: f32, arg: f32) -> bool {
    let d = ac10_diff(stored, arg);
    !d.is_nan() && d < f(EPS_BITS)
}

// ===========================================================================================
// (4) 0x76f900 — 4-channel colour pack: per channel `__ftol(arg·255 + 0.5)`, low byte packed.
// ===========================================================================================

/// One colour channel of `0x76f900`'s pack (the shared `fld [arg]; fmul [255.0]; fadd [0.5];
/// call __ftol` kernel — `0x76f907`/`91e`/`936`/`94f` for the four args). NO clamp gates (unlike
/// the gated packer `0x76d680`): the product `arg·255 + 0.5` (a PC_53 f64, kept in st0 straight
/// into `__ftol`) is truncated toward zero by the CRT helper, and only the LOW byte of the i32
/// result is taken (`mov bh,al` / `mov bl,al` / `movzx eax,al` / `mov cl,al`). For NaN, `__ftol`
/// yields the integer indefinite, whose low byte is `0x00`. Returns the packed byte.
pub fn channel(arg: f32) -> u8 {
    let v = f64::from(arg) * f(C255_BITS) + f(HALF_BITS);
    // __ftol truncates toward zero; only the low byte is kept (matches al/bl/bh/cl truncation).
    (benilla_re_cmath::x87::ftol(v) as u32 & 0xff) as u8
}

/// `0x76f900`'s full 4-channel pack (`0x76f903`..`0x76f96e`). The four args (`[ebp+8]`/`[ebp+0xc]`/
/// `[ebp+0x10]`/`[ebp+0x14]`) are each `__ftol(arg·255 + 0.5)`'d in order, the low bytes packed
/// MSB→LSB into EBX as the calls return:
/// * arg0 → `mov bh,al`        ⇒ ebx = c0<<8
/// * arg1 → `mov bl,al; shl 8` ⇒ ebx = (c0<<16)|(c1<<8)
/// * arg2 → `or bl,al;  shl 8` ⇒ ebx = (c0<<24)|(c1<<16)|(c2<<8)
/// * arg3 → `or bl,al`         ⇒ ebx = (c0<<24)|(c1<<16)|(c2<<8)|c3
///
/// then `mov [esi], ebx`. So the dword = `(c0<<24)|(c1<<16)|(c2<<8)|c3`. Returns that u32.
pub fn pack4(a0: f32, a1: f32, a2: f32, a3: f32) -> u32 {
    let c0 = u32::from(channel(a0));
    let c1 = u32::from(channel(a1));
    let c2 = u32::from(channel(a2));
    let c3 = u32::from(channel(a3));
    (c0 << 24) | (c1 << 16) | (c2 << 8) | c3
}

// ===========================================================================================
// (5) 0x783380 — guarded fraction `(c[0x320] − c[0x318]) / (c[0x31c] − c[0x318])` (gate on c31c).
// ===========================================================================================

/// `0x783380`'s guarded fraction (a no-prologue `__thiscall` leaf returning a `float` in st0).
/// `0x783380 fld [ecx+0x31c]; fsub [ecx+0x318]` → `gate = c31c − c318` (the divisor, left in st1);
/// `0x78338c fcom [0.0]; fnstsw; test ah,0x41; jne 0x7833a8`. Mask 0x41 = {C0,C3}; `jne` (ZF=0 ⇔
/// masked AH ≠ 0) taken (→ fallback 0.0) iff `gate <= 0` (C0 set for `gate<0`, C3 for `gate==0`)
/// OR NaN (C0|C2|C3). Falls through (compute the ratio) iff `gate > 0`. The fall-through
/// `0x783399 fld [ecx+0x320]; fsub [ecx+0x318]` → `top = c320 − c318` (st0); `0x7833a5
/// fdivrp st(1),st` is the REVERSE divide-and-pop: `st(1) = st(0) / st(1) = top / gate` (NOT
/// `gate/top` — the binary returns `(c320−c318)/(c31c−c318)`, e.g. (2−0)/(1−0) = 2.0, RED-localized
/// vs the initial `gate/top` reading). The taken branch `0x7833a8 fstp st(0); fld [0.0]` returns
/// `0.0`. So:
/// * `gate > 0` → return `top / gate` (un-narrowed PC_53 f64 in st0),
/// * else (`gate <= 0` or NaN) → return `0.0`.
///
/// `c318`/`c31c`/`c320` are the three `f32` fields. Returns the f64 left in st0.
pub fn fraction(c318: f32, c31c: f32, c320: f32) -> f64 {
    let gate = f64::from(c31c) - f64::from(c318); // divisor (st1), the sign-gated difference
                                                  // jne (→ 0.0) iff gate<=0 or NaN; fall-through (ratio) iff gate>0.
    if gate > 0.0 {
        let top = f64::from(c320) - f64::from(c318); // dividend (st0)
        top / gate // fdivrp st(1),st  ⇒  st0/st1  ⇒  top/gate
    } else {
        0.0
    }
}
