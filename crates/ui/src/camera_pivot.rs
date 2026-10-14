//! Reimplements the pivot-height box accumulator `CCamera::BuildPivotHeights` (`0x50ca90`). The
//! x87 runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores;
//! the tests in `tests/camera_pivot.rs` diff each against the exe.

/// `[0x808ab0]` = 0.0972222238779068 — the accum-branch-A1 additive bias.
pub const ACCUM_BIAS: f32 = f32::from_bits(0x3dc7_1c72);
/// `[0x804588]` = 0.8999999761581421 — the accum-branch-A2 `·0.9` factor.
pub const ACCUM_K09: f32 = f32::from_bits(0x3f66_6666);
/// `[0x8089cc]` = 0.8333333134651184 (5/6) — the box-extent clamp **lower** bound.
pub const CLAMP_LO: f32 = f32::from_bits(0x3f55_5555);
/// `[0x8089c8]` = 15.0 — the box-extent clamp **upper** bound.
pub const CLAMP_HI: f32 = f32::from_bits(0x4170_0000);
/// `[0x808aac]` = 0.9900000095367432 — the parallax-leg `·0.99` factor.
pub const PARALLAX_K099: f32 = f32::from_bits(0x3f7d_70a4);

/// `[0x50cfa7, 0x50d098)` — the box-extent **clamp** applied to each of the three accumulated
/// components `this+0x11c/+0x120/+0x124` in turn.
///
/// The x87 NaN-aware clamp the binary emits (`fld x; fcom [LO]; fnstsw; test ah,5; jp …`): for the
/// masked status bits `{C0,C2}` (`test ah,0x5`), an **ordered** `x < LO` sets C0 (1 bit → odd
/// parity → `jp` not-taken → substitute LO); `x ≥ LO` sets 0 bits (even → `jp` taken → fall to the
/// upper test); a **NaN** sets C0=C2 (2 bits → even → `jp` taken → upper test, where NaN again →
/// substitute HI). The upper test `fcom [HI]; test ah,5; jnp …`: `x < HI` → C0 (odd → `jnp` taken →
/// keep x); `x ≥ HI` or NaN → 0/2 bits (even → `jnp` not-taken → substitute HI). Net:
/// `out = NaN ? HI : x < LO ? LO : x < HI ? x : HI`. The comparison rides f32 operands throughout
/// (`fld dword`/`fcom dword`), so there is no live-vs-spill precision split here.
#[must_use]
pub fn clamp_box_extent(x: f32) -> f32 {
    if x.is_nan() {
        return CLAMP_HI;
    }
    // Ordered from here. `x < LO` → LO; else `x < HI` → x; else (x ≥ HI) → HI.
    if x < CLAMP_LO {
        CLAMP_LO
    } else if x < CLAMP_HI {
        x
    } else {
        CLAMP_HI
    }
}

/// `[0x50cb27, 0x50cb3b)` — the per-call **scale scalar** `local_8` (stored f32 to `[ebp-4]`, reused
/// as the multiplier by every accumulation below). `r` = the virtual-call result
/// `(*(*param_2+0x1c))()` (a float, foreign — orchestration supplies it); `f94` = `[ebx+0x94]`
/// (`[ebp-4]`); `mul` = `local_10` (`[ebx+0x9c]` if `[ebx+0xdc]!=0` else `[ebx+0x98]`).
/// `scalar = (r · f94) · mul` (`fmul dword [ebp-4]; fmul dword [ebp-0xc]; fstp dword [ebp-4]`).
#[must_use]
pub fn scale_scalar(r: f32, f94: f32, mul: f32) -> f32 {
    ((f64::from(r) * f64::from(f94)) * f64::from(mul)) as f32
}

/// `[0x50cbe1, 0x50cbed)` — accumulation-branch **A1** (the `0x712cb0`-hit leg): the per-axis delta
/// `(local_14 + 0.0972…) · scalar`. `v14` = `local_14` (`[ebp-0x10]`, from `0x712cf0`); `scalar` =
/// the `[ebp-4]` scale. `fld dword [ebp-0x10]; fadd dword [0x808ab0]; fmul dword [ebp-4]`. The
/// result is left **LIVE on st0** (NO `fstp [m32]`) — it flows straight into [`accum_fold`], so it
/// stays PC_53 (f64), never narrowed. Returns f64 for exactly that reason.
#[must_use]
pub fn accum_a1(v14: f32, scalar: f32) -> f64 {
    (f64::from(v14) + f64::from(ACCUM_BIAS)) * f64::from(scalar)
}

/// `[0x50cbfa, 0x50cc09)` — accumulation-branch **A2** (the `0x713700`-fill leg): the per-axis delta
/// `((local_20 − local_2c) · scalar) · 0.9`. `v20` = `local_20` (`[ebp-0x20]`); `v2c` = `local_2c`
/// (`[ebp-0x2c]`); `scalar` = the `[ebp-4]` scale.
/// `fld dword [ebp-0x20]; fsub dword [ebp-0x2c]; fmul dword [ebp-4]; fmul dword [0x804588]`. As with
/// A1 the result is left **LIVE on st0** (no store), feeding [`accum_fold`] at PC_53 — returns f64.
#[must_use]
pub fn accum_a2(v20: f32, v2c: f32, scalar: f32) -> f64 {
    ((f64::from(v20) - f64::from(v2c)) * f64::from(scalar)) * f64::from(ACCUM_K09)
}

/// `[0x50cc09, 0x50cc34)` — fold the **live** delta `d` (st0, PC_53 from A1/A2 — NOT a reloaded f32
/// spill, `fst`-no-pop / live-st0 distinction) into the running 3-component box vector
/// `(c0,c1,c2) = this+0x11c/+0x120/+0x124`: `cN = d + cN` for each axis. `fld st(0)` re-duplicates
/// `d` per axis (the original PC_53 value rides through all three `fadd dword [esi+…]; fstp dword
/// [esi+…]`); only the final store narrows to f32. So `d` is f64 throughout — never re-narrowed
/// between the three adds.
#[must_use]
pub fn accum_fold(d: f64, c: [f32; 3]) -> [f32; 3] {
    [
        (d + f64::from(c[0])) as f32,
        (d + f64::from(c[1])) as f32,
        (d + f64::from(c[2])) as f32,
    ]
}

/// `[0x50cf68, 0x50cf74)` — the parallax/ortho-leg combine: the binary's
/// `fld dword [ebp-0x8c]; fsub dword [ebp-0x50]; fsubr dword [ebp-8]` (`fsubr` ⇒ `mem − st`).
/// `a` = `[ebp-0x8c]`, `b` = `[ebp-0x50]`, `inv` = `[ebp-8]`: `out = inv − (a − b)`.
#[must_use]
pub fn parallax_combine(a: f32, b: f32, inv: f32) -> f32 {
    (f64::from(inv) - (f64::from(a) - f64::from(b))) as f32
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The remaining 23 owned FP-arith ops (the z-axis box accum, the
// four per-component scale-transform clusters, the x/y branch accum, the CE44 spill, the 3 parallax
// adds). All sit in the `!=0` jump-table leg of the body, between the key-binding/CString
// orchestration calls — each owns its arithmetic; the calls compose it.
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// `[0x50cce3, 0x50cd04)` — the **z-axis** box-extent accumulation (the `0x711a20`-fill leg's tail):
/// `this+0x124 -= (L8c·scalar − L50·scalar)`. `a` = `local_8c` (`[ebp-0x8c]`); `b` = `local_50`
/// (`[ebp-0x50]`); `scalar` = the `[ebp-4]` scale; `z` = the current `[esi+0x124]` component.
///
/// The binary builds the two products on the x87 stack and subtracts them **st-vs-st** (both LIVE,
/// PC_53 — no f32 spill between):
/// `fld [L8c]; fmul [s]   (st0 = L8c·s)`
/// `fld [L50]; fmul [s]   (st0 = L50·s, st1 = L8c·s)`
/// `fxch st(1)            (st0 = L8c·s, st1 = L50·s)`
/// `fsub st, st(1)        (st0 = L8c·s − L50·s, both PC_53)`
/// `fsubr [esi+0x124]     (st0 = z − (L8c·s − L50·s))`
/// `fstp [esi+0x124]      (narrow to f32 on store)`.
/// So the **only** narrowing point is the final store — the inner difference rides f64. `L8c·s` and
/// `L50·s` are themselves un-spilled products (each just `fld;fmul` left on st0), so they too stay
/// PC_53; modelling either product or the difference as f32 would re-narrow mid-chain. Returns the
/// stored `f32`.
#[must_use]
pub fn accum_z(a: f32, b: f32, scalar: f32, z: f32) -> f32 {
    let pa = f64::from(a) * f64::from(scalar); // L8c·s, live PC_53
    let pb = f64::from(b) * f64::from(scalar); // L50·s, live PC_53
    (f64::from(z) - (pa - pb)) as f32
}

/// `[0x50cd7f, 0x50cda1)` (cluster S1) · `[0x50cda6, 0x50cdda)` (S2) · `[0x50cf0b, 0x50cf3f)` (S3) ·
/// `[0x50cf44, 0x50cf63)` (S4) — a per-component **scale transform**: three independent
/// `loc = (loc · scalar) as f32` (`fld dword [loc]; fmul dword [ebp-4]; fstp dword [loc]`), each
/// with a full f32 store between, feeding a `0x5132f0` (vec3-scale leaf) call. No live-st0 chaining
/// — every product is narrowed to f32 by its own `fstp` before the next op. The four clusters are
/// the SAME arithmetic over different local triples (S1/S4 over `[ebp-0x64/-0x60/-0x5c]`,
/// S2/S3 over `[ebp-0xa0/-0x9c/-0x98]`); one transcription covers all four. `v` = the input
/// component (one of the three `[ebp-…]` locals a CString/resource call filled); `scalar` =
/// `[ebp-4]`.
#[must_use]
pub fn scale_component(v: f32, scalar: f32) -> f32 {
    (f64::from(v) * f64::from(scalar)) as f32
}

/// `0x50ce44` — the single `fmul` spill `[ebp-0x8] = (local_20 · scalar) as f32`
/// (`fld dword [ebp-0x20]; fmul dword [ebp-4]; fstp dword [ebp-0x8]`). Self-contained, narrowed to
/// f32 on the store; `[ebp-0x8]` is then reloaded f32 by the parallax tail. `v20` = `[ebp-0x20]`;
/// `scalar` = `[ebp-4]`. (Same arithmetic shape as [`scale_component`], distinct slot/role — the
/// box-resource magnitude `local_20` scaled into the parallax-tail accumulator `local_8`.)
#[must_use]
pub fn scale_ce44(v20: f32, scalar: f32) -> f32 {
    (f64::from(v20) * f64::from(scalar)) as f32
}

/// `[0x50cde2, 0x50ce0b)` — the **x/y** box-extent accumulation: `d = (L50 − L8c)` (LIVE on st0),
/// `this+0x11c -= d`, and **conditionally** (the `cmp ebx,0x5b; jne` mode test) `this+0x120 -= d`.
/// `a` = `local_50` (`[ebp-0x50]`); `b` = `local_8c` (`[ebp-0x8c]`); `cx`/`cy` = the current
/// `[esi+0x11c]`/`[esi+0x120]` components; `do_y` = whether `ebx == 0x5b` (the `'['` action id), so
/// the `jne 0x50ce09` falls through to the y-leg.
///
/// `fld [L50]; fsub [L8c]    (st0 = d = L50 − L8c, LIVE PC_53 — never spilled to f32)`
/// `fld [esi+0x11c]; fsub st, st(1)   (st0 = cx − d, where d is the LIVE st1)`
/// `fstp [esi+0x11c]         (narrow x on store)`
/// `jne …                    (skip the y-leg if ebx != 0x5b)`
/// `fld [esi+0x120]; fsub st, st(1)   (st0 = cy − d, d still the LIVE st1)`
/// `fstp [esi+0x120]         (narrow y on store)`
/// `fstp st(0)               (drop the leftover d)`.
/// `d` rides st1 across **both** axis subtractions un-spilled — model it f64. Both `cN − d` use the
/// live `d`, not a reloaded f32 spill; only the per-axis stores narrow. Returns `(x', y')` with
/// `y'` = the input `cy` unchanged when `do_y` is false.
#[must_use]
pub fn accum_xy(a: f32, b: f32, cx: f32, cy: f32, do_y: bool) -> (f32, f32) {
    let d = f64::from(a) - f64::from(b); // LIVE PC_53 across both subtractions
    let x = (f64::from(cx) - d) as f32;
    let y = if do_y { (f64::from(cy) - d) as f32 } else { cy };
    (x, y)
}

/// `[0x50cf77, 0x50cfa4)` — the **parallax tail** accumulation: add the parallax magnitude
/// `local_8` (`[ebp-0x8]`) into all three components,
/// `this+0x11c/+0x120/+0x124 += local_8`. Each axis is an independent
/// `fld dword [ebp-8]; fadd dword [esi+…]; fstp dword [esi+…]` — `local_8` is reloaded f32 each
/// time, every product narrowed to f32 by its own store, so there is no live-st0 chaining (unlike
/// [`accum_fold`], which re-dups a single live st0). `inv` = `[ebp-0x8]` (the parallax magnitude);
/// `c` = the three current components.
#[must_use]
pub fn parallax_tail(inv: f32, c: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(inv) + f64::from(c[0])) as f32,
        (f64::from(inv) + f64::from(c[1])) as f32,
        (f64::from(inv) + f64::from(c[2])) as f32,
    ]
}
