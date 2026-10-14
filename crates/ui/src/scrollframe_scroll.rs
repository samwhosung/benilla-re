//! Reimplements the arithmetic of `CSimpleScrollFrame::SetHorizontalScroll` (`0x786d30`),
//! `SetVerticalScroll` (`0x786db0`) and `0x786e30`, `0x787140`, `0x787220`. The x87 runs at
//! 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests
//! in `tests/scrollframe_scroll.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// norm-ε = 2⁻²² (`DAT_008029d4`), the change-detect epsilon. Same constant as `camera_view_input`'s
/// `EPS_BITS` — the camera/UI change-detect epsilon.
pub const EPS_BITS: u32 = 0x3480_0000; // 2.384185791015625e-07
/// 1024.0 (`DAT_007ffd68`), the display-product scale.
pub const SCALE1024_BITS: u32 = 0x4480_0000;
/// 0.0 (`DAT_007ffd74`), the clamp-≥0 / sign-test constant.
pub const ZERO_BITS: u32 = 0x0000_0000;
/// 1.0 (`DAT_007ff9d8`), the reciprocal numerator / the clamp-to-1 ceiling.
pub const ONE_BITS: u32 = 0x3f80_0000;

#[inline]
fn eps() -> f64 {
    f64::from(f32::from_bits(EPS_BITS))
}
#[inline]
fn scale1024() -> f64 {
    f64::from(f32::from_bits(SCALE1024_BITS))
}
#[inline]
fn one() -> f64 {
    f64::from(f32::from_bits(ONE_BITS))
}

// ===========================================================================================
// (1) 0x786d30 / 0x786db0 — per-channel eps-gated display update (clone pair)
// ===========================================================================================

/// The eps change-detect gate of `0x786d30`/`0x786db0` (`0x786d36`..`0x786d4d`):
/// `fld [field]; fsubr [ebp+8]=arg; fabs; fcomp eps(2⁻²²); fnstsw; test ah,5; jnp 0x786da2(=skip)`.
/// `fsubr` = `arg − field`. `test ah,5` masks {C0,C2}; **PF=1 ⟺ EVEN set-bits**, so `jnp` (PF=0) is
/// taken iff ODD. After `fcomp(|d|, eps)`: C0=1 iff `|d|<eps`. ordered `|d|<eps`: C0=1,C2=0 → 1 bit
/// (odd) → jnp TAKEN → SKIP (settled, no display update). ordered `|d|>=eps`/`==eps`: 0 bits (even)
/// → not taken → does the update. NaN: C0=C2=1 → 2 bits (even) → update. So the channel UPDATES iff
/// `|arg−field| >= eps` (ordered) or unordered. This predicate is `|d| < eps` (false on NaN) — the
/// binary SKIPS the update iff this is true.
pub fn eps_settled(arg: f32, field: f32) -> bool {
    let d = (f64::from(arg) - f64::from(field)).abs();
    !d.is_nan() && d < eps()
}

/// The display product of `0x786d30`/`0x786db0` (`0x786d76`..`0x786d85`): a getter (`call 0x41ad70`
/// = `fld [0x832a4c]; ret`) returns a scale global `g` in st0, then `fmul [0x7ffd68]=1024.0; fmul
/// [ebp+8]=v; fstp dword [esp]` → the f32 value pushed to the `%g` formatter. `v` is the channel's
/// new value (`[esi+0x324]` copied to `[ebp+8]` at 0x786d6d). All three operands multiply at PC_53;
/// the result narrows to f32 at the `fstp dword`. `g` is the runtime scale (read from `[0x832a4c]`,
/// init 1.0). Returns `(g·1024.0·v)` as f32.
pub fn display_product(g: f32, v: f32) -> f32 {
    ((f64::from(g) * scale1024()) * f64::from(v)) as f32
}

// ===========================================================================================
// (2) 0x786e30 — axis scale-normalize (X live recip / Y reloaded f32 recip) + dual eps + product
// ===========================================================================================

/// `0x786e30`'s X-axis scale-normalize (`0x786e7e`..`0x786eaa`): given a box-extent pair
/// `(max_x, min_x)` (from `[ebp-0xc]`/`[ebp-0x14]`, populated by `call 0x786f80`), the per-axis
/// argument `arg0` (`[ebp+8]`), and the per-channel scale `s = [esi+0x7c]`:
/// * `width = max_x − min_x` (`fsub [ebp-0x14]`),
/// * `c = width − arg0` (`fsub [ebp+8]`),
/// * `clamped = max(c, 0)` — `fcom 0.0; test ah,0x41; je` keeps `c` iff `c > 0` ordered, else 0.0,
/// * `recip = 1.0 / s` (`fld [esi+0x7c]; fdivr 1.0`), stored f32 to `[ebp+8]` (`fst`, NO pop —
///   st0 stays the LIVE PC_53 reciprocal),
/// * `rx = clamped · recip_LIVE` (`fmulp`) — the X result uses the un-rounded reciprocal.
///
/// Returns `(rx_f32, recip_f32)` — `rx` is `fst`'d → f32 to `[esi+0x31c]`, and `recip_f32` is the
/// f32 the `[ebp+8]` slot holds (the value [`axis_scale_y`] reloads). Both at PC_53.
pub fn axis_scale_x(max_x: f32, min_x: f32, arg0: f32, s: f32) -> (f32, f32) {
    let c = (f64::from(max_x) - f64::from(min_x)) - f64::from(arg0);
    let clamped = clamp_ge0(c);
    let recip = one() / f64::from(s); // LIVE PC_53 reciprocal
    let recip_f32 = recip as f32; // fst [ebp+8] → f32 (st0 stays live)
    let rx = (clamped * recip) as f32; // fmulp on the LIVE recip
    (rx, recip_f32)
}

/// `0x786e30`'s Y-axis scale-normalize (`0x786eb0`..`0x786ed1`): the mirror of [`axis_scale_x`], but
/// the reciprocal is RELOADED as f32 (`fmul [ebp+8]`, the f32 slot [`axis_scale_x`] stored), NOT the
/// live st0 — live-vs-reload distinction. Given `(max_y, min_y)`, `arg1` (`[ebp+0xc]`), and
/// the f32 reciprocal `recip_f32` from [`axis_scale_x`]:
/// * `width = max_y − min_y`, `c = width − arg1`, `clamped = max(c, 0)`,
/// * `ry = clamped · recip_F32` (`fmul [ebp+8]`).
///
/// Returns `ry` as f32 (`fst`'d → f32 to `[esi+0x320]`).
pub fn axis_scale_y(max_y: f32, min_y: f32, arg1: f32, recip_f32: f32) -> f32 {
    let c = (f64::from(max_y) - f64::from(min_y)) - f64::from(arg1);
    let clamped = clamp_ge0(c);
    (clamped * f64::from(recip_f32)) as f32
}

/// The `max(c, 0)` clamp shared by the X/Y scale blocks (`0x786e87`..`0x786e9c` /
/// `0x786eb9`..`0x786ece`): `fcom 0.0; fnstsw; test ah,0x41; je <keep>` — `test ah,0x41` masks
/// {C0,C3}, `je` taken iff masked-AH==0 iff `c > 0.0` ordered (C0=0 AND C3=0). `je` taken ⇒ keep
/// `c`; not-taken (`c <= 0` ordered, or unordered/NaN) ⇒ replace with `0.0`. So result = `c` iff
/// `c > 0` (ordered, strictly), else `0.0` (NaN → 0.0, matching the C0-set unordered route).
fn clamp_ge0(c: f64) -> f64 {
    if !c.is_nan() && c > 0.0 {
        c
    } else {
        f64::from(f32::from_bits(ZERO_BITS))
    }
}

/// `0x786e30`'s dual eps change-detect (`0x786ed7`..`0x786ef2`): with the X result `rx` and Y result
/// `ry` live on the stack and the prior values `old0`/`old1` (`[ebp-0x8]`/`[ebp-0x4]`, copied from
/// `[esi+0x31c]`/`[esi+0x320]` at 0x786e4e):
/// * `|rx − old0| < eps` (`fld old0; fsubr st,st(2)=rx; fabs; fcomp eps; test ah,5; jp 0x786eff`):
///   `jp` (PF=1, EVEN) taken iff `|d| >= eps`/NaN → does the work (NOT settled on X). C0=1 odd
///   (`|d|<eps`) → not taken → check Y.
/// * `|ry − old1| < eps` (`fld old1; fsubr st,st(1)=ry; fabs; fcomp eps; test ah,5; jnp 0x786f66`):
///   `jnp` (PF=0) taken iff ODD = `|d|<eps` → 0x786f66 = SKIP. else → work.
///
/// So the binary SKIPS the display work iff BOTH `|rx−old0| < eps` AND `|ry−old1| < eps` (ordered).
/// `fsubr` = `rx − old0` / `ry − old1`. Returns `true` ⇒ both within eps ⇒ binary SKIPS.
pub fn axis_settled(rx: f32, ry: f32, old0: f32, old1: f32) -> bool {
    let dx = (f64::from(rx) - f64::from(old0)).abs();
    if dx.is_nan() || dx >= eps() {
        return false; // X not settled (or unordered) → work
    }
    let dy = (f64::from(ry) - f64::from(old1)).abs();
    !dy.is_nan() && dy < eps()
}

// ===========================================================================================
// (3) 0x787140 — rect delta (two subtractions feeding 0x786e30)
// ===========================================================================================

/// `0x787140`'s rect delta (`0x787188`..`0x78719d`): a 4-float rect buffer
/// `(buf0, buf1, buf2, buf3) = [ebp-0x10]/[ebp-0xc]/[ebp-0x8]/[ebp-0x4]` (filled by
/// `call 0x768320`) is reduced to a `(width, height)` pair pushed to `0x786e30`:
/// * `dy = buf2 − buf0` (`fld [ebp-0x8]; fsub [ebp-0x10]; fstp [esp+4]`) → second arg,
/// * `dx = buf3 − buf1` (`fld [ebp-0x4]; fsub [ebp-0xc]; fstp [esp]`) → first arg.
///
/// Each difference is `fstp`'d → f32. Returns `(dy, dx)` as f32 in stored order (`[esp+4]`, `[esp]`).
pub fn rect_delta(buf0: f32, buf1: f32, buf2: f32, buf3: f32) -> (f32, f32) {
    let dy = (f64::from(buf2) - f64::from(buf0)) as f32;
    let dx = (f64::from(buf3) - f64::from(buf1)) as f32;
    (dy, dx)
}

// ===========================================================================================
// (4) 0x787220 — UV clamp-to-[0,1] with carry (per-axis)
// ===========================================================================================

/// `0x787220`'s per-axis UV clamp-to-[0,1] with carry (axis A: `0x787482`..`0x7874c8`; axis B:
/// `0x7874c8`..`0x78750e` is the instruction-mirror). Given a coordinate pair `(lo, hi)` — A is
/// `(lo=[ebp-0xc], hi=[ebp-0x4])`, B is `(lo=[ebp-0x10], hi=[ebp-0x8])` — clamp the interval into
/// `[0, 1]`, carrying the clipped amount onto the opposite endpoint:
/// * **lower clip** (`0x787482 fld lo; fcomp 0.0; test ah,5; jp <skip>`): `jp` (PF=1, EVEN) taken
///   iff `lo >= 0`/NaN → skip; ordered `lo < 0` (C0=1, odd) → `hi = hi − lo; lo = 0`
///   (`fld hi; fsub lo; fstp hi`). So when `lo < 0`: shift `hi` down by `lo` (which is negative ⇒
///   `hi` grows) and snap `lo` to 0.
/// * **upper clip** (`0x7874a2 fld hi; fcomp 1.0; test ah,0x41; jne <skip>`): `test ah,0x41` masks
///   {C0,C3}, `jne` taken iff masked-AH != 0 iff `hi < 1.0`(C0) OR `hi == 1.0`(C3) OR unordered →
///   skip; falls through (does work) iff `hi > 1.0` ordered strictly → `lo = lo − (hi − 1.0); hi =
///   1.0` (`fld hi; fsub 1.0; fsubr lo; fstp lo`). So when `hi > 1.0`: shift `lo` up by `hi−1` and
///   snap `hi` to 1.
///
/// `fsub` / `fsubr` operands as emitted: lower-clip `hi − lo`; upper-clip `hi − 1.0` then
/// `lo − (hi−1.0)` (`fsubr [lo]` = `lo − st0`). Both endpoints `fstp`'d → f32. NaN routes per the
/// emitted jcc (lower: `jp` skips on unordered → no lower change; upper: `jne` skips on unordered →
/// no upper change). Returns the clamped `(lo, hi)` as f32. Order matters: the lower clip runs
/// FIRST and its updated `hi` feeds the upper-clip test.
pub fn clamp01_carry(lo: f32, hi: f32) -> (f32, f32) {
    let mut lo64 = f64::from(lo);
    let mut hi64 = f64::from(hi);
    // lower clip: lo < 0 (ordered) ⇒ hi -= lo (f32 store), lo = 0.
    if !lo64.is_nan() && lo64 < 0.0 {
        hi64 = f64::from((hi64 - lo64) as f32); // fstp hi → f32 (then reloaded for the upper test)
        lo64 = 0.0;
    }
    // upper clip: hi > 1.0 (ordered, strict) ⇒ lo -= (hi - 1.0), hi = 1.0.
    if !hi64.is_nan() && hi64 > one() {
        let excess = hi64 - one(); // fsub 1.0
        lo64 = f64::from((lo64 - excess) as f32); // fsubr [lo] = lo - excess; fstp lo → f32
        hi64 = one();
    }
    (lo64 as f32, hi64 as f32)
}
