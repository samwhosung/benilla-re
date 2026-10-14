//! Reimplements the look-at frame (`0x50a9f0`), the orthonormal basis and the projection aspect
//! (`0x50ab70`) that feed the device view and projection matrices. The x87 runs at 53-bit
//! precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests in
//! `tests/view_matrices.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

#[inline]
fn g(x: f32) -> f64 {
    f64::from(x)
}

/// `0x8029d4` = 2^-22 (`0x34800000`), the squared-parallel / squared-length epsilon.
pub const EPS_BITS: u32 = 0x3480_0000;
/// `0x7ff9d8` = 1.0 (`0x3f800000`), the `fsub 1.0` parallel pivot and the `fdivr 1.0` reciprocal.
pub const ONE_BITS: u32 = 0x3f80_0000;

/// vec3 cross `a × b`, each component computed at f64 then narrowed to f32 (one `fstp dword` per
/// component, exactly as `0x50a9f0` stores `[ebp-0xc/-0x8/-0x4]`). The component order matches the
/// binary's `fld;fmul;fld;fmul;fsubp;fstp` triples.
#[inline]
fn cross_f32(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        (g(a[1]) * g(b[2]) - g(a[2]) * g(b[1])) as f32,
        (g(a[2]) * g(b[0]) - g(a[0]) * g(b[2])) as f32,
        (g(a[0]) * g(b[1]) - g(a[1]) * g(b[0])) as f32,
    ]
}

/// `0x50a9f0`'s owned look-at frame. Returns `(out0, out1, out2)` as f32 triples for a
/// non-degenerate forward `f` (precondition `|f|² ≥ eps` — the binary's `0x50a9f6` guard, whose
/// degenerate branch writes no output and is never fed). `up` is the up-hint.
///
/// * `out0 = f` (raw copy).
/// * `s = |abs(dot(up,f)) − 1|`; if `s < eps` → `out1 = (1,0,0)` (up ∥ f), else
///   `out1 = normalize(up × f)`.
/// * `out2 = f × out1`.
pub fn look_at_frame(f: [f32; 3], up: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let out0 = f; // exact dword copy, F not normalized

    // parallel guard. dot accumulated in the binary's order: (Uz·Fz + Ux·Fx) + Uy·Fy.
    let dot = (g(up[2]) * g(f[2]) + g(up[0]) * g(f[0])) + g(up[1]) * g(f[1]);
    let eps = g(f32::from_bits(EPS_BITS));
    let parallel = (dot.abs() - g(f32::from_bits(ONE_BITS))).abs() < eps; // s < eps (==eps ⇒ compute)

    let out1 = if parallel {
        [1.0f32, 0.0, 0.0]
    } else {
        let r = cross_f32(up, out0); // U × F, each component f32
                                     // normalize: |r|² = (rx²+rz²)+ry²  (binary's accumulation order @0x50aacf..),
                                     //            inv = 1.0 / fsqrt(|r|²).
        let m2 = (g(r[0]) * g(r[0]) + g(r[2]) * g(r[2])) + g(r[1]) * g(r[1]);
        let inv = g(f32::from_bits(ONE_BITS)) / m2.sqrt();
        [
            (g(r[0]) * inv) as f32,
            (g(r[1]) * inv) as f32,
            (g(r[2]) * inv) as f32,
        ]
    };

    let out2 = cross_f32(out0, out1); // F × out1
    (out0, out1, out2)
}

// ===========================================================================================
// 0x50a840 — orthonormal basis from a single forward vector `in` (`__thiscall`, `ret 8`).
//   ecx = `in` vec3 ptr; edx = out0 ptr; [ebp+8] = out1 ptr; [ebp+0xc] = out2 ptr.
//   * |in|² guard (`0x50a869`, compare-only) — degenerate `in` early-outs via SErrSetLastError,
//     never fed (precondition |in|² ≥ eps).
//   * out0 = in (raw copy).
//   * |in.xy|² guard: if `< eps` → out1 = (1,0,0); else out1 = normalize2d(−in.y, in.x) with z=0,
//     the normalize DELEGATED to `0x456280` (a vec2-normalize cmath leaf — runs native under the
//     whole-fn `call`; reproduced in `normalize2d` here only as diff-reference, NOT a ui ownership
//     claim — `0x456280` stays `BOUNDARY: -> cmath`).
//   * out2 = cross(out0, out1) (uses only out1.x/out1.y; out1.z≡0): (−out0.z·out1.y, out0.z·out1.x,
//     out1.y·out0.x − out0.y·out1.x).
// ===========================================================================================

/// `0x456280` — in-place vec2 normalize (the cmath leaf `0x50a840` delegates to). `m2 = a²+b²`
/// (a squared first), `inv = 1.0 / fsqrt(m2)`, each component `·inv` narrowed to f32. Reproduced
/// here as the whole-fn diff reference; ownership of `0x456280` is cmath's.
#[inline]
fn normalize2d(a: f32, b: f32) -> (f32, f32) {
    let m2 = g(a) * g(a) + g(b) * g(b);
    let inv = g(f32::from_bits(ONE_BITS)) / m2.sqrt();
    ((g(a) * inv) as f32, (g(b) * inv) as f32)
}

/// `0x50a840`'s owned orthonormal basis. Returns `(out0, out1, out2)` for a non-degenerate `inp`
/// (precondition `|inp|² ≥ eps`). `out0 = inp`; `out1 = (1,0,0)` if `|inp.xy|² < eps` else
/// `normalize2d(−inp.y, inp.x)` with z=0; `out2 = cross(out0, out1)` (out1.z≡0, never read).
pub fn orthonormal_basis(inp: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let out0 = inp;
    let eps = g(f32::from_bits(EPS_BITS));
    let xy2 = g(out0[0]) * g(out0[0]) + g(out0[1]) * g(out0[1]); // |in.xy|² (x² first)
    let out1 = if xy2 < eps {
        [1.0f32, 0.0, 0.0]
    } else {
        let (nx, ny) = normalize2d(-out0[1], out0[0]); // perp = (−y, x), then normalize
        [nx, ny, 0.0]
    };
    // out2 = cross(out0, out1), reading only out1.x/out1.y (binary @0x50a8f0 never loads out1.z):
    let ux = (-(g(out1[1]) * g(out0[2]))) as f32; // −out1.y·out0.z  (fmul;fchs)
    let uy = (g(out0[2]) * g(out1[0])) as f32; // out0.z·out1.x
    let uz = (g(out1[1]) * g(out0[0]) - g(out0[1]) * g(out1[0])) as f32; // out1.y·out0.x − out0.y·out1.x
    (out0, out1, [ux, uy, uz])
}

// ===========================================================================================
// 0x50ab70 — projection+view setup (SmartScreenRect).
//   Owned arith: the aspect ratio `(rect.b − rect.t) / (rect.r − rect.l)`, stored to [esi+0x44].
//   Everything else is integer identity-matrix fill + delegated math (gxumath perspective
//   0x5c3cc0 / lookat 0x5c3e70) + gx device push (0x58b040/50) + vtable dispatch — composition.
//   The quotient stays live on st0 through the integer fill until `fst[esi+0x44]`/`fstp[ebp-0x8]`.
// ===========================================================================================

/// `0x50ab70`'s owned aspect ratio (slice `0x50ab7c`..`0x50ac25`): `(b − t) / (r − l)` for the
/// rect `(l,t,r,b) = ([eax], [eax+4], [eax+8], [eax+0xc])`. `fld[eax+0xc];fsub[eax+4]` = b−t;
/// `fld[eax+8];fsub[eax]` = r−l; `fdivp` = (b−t)/(r−l) (f64 at PC_53), narrowed to f32 on store.
pub fn proj_aspect(l: f32, t: f32, r: f32, b: f32) -> f32 {
    ((g(b) - g(t)) / (g(r) - g(l))) as f32
}
