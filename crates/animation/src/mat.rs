//! The 4×4 products the math slot `0x876504` can hold and the rotation matrix from a quaternion
//! `0x74b6bb` that the slot `0x876594` holds
//! (`systems/models/animation.md#dispatched-matrix-math`, `systems/animation/pose.md#rotation-leg`).
//! The first call selects the product by processor: a processor with SSE2 runs `0x754a66`
//! ([`matmul4x4_sse2`], in `benilla_re_cmath::multiply`), the product the animate pass uses; one without runs `0x7507bb`
//! ([`m2_matmul4x4_base`]), which `0x750930` stores. The table's entry `0x74a7c6`
//! ([`m2_matmul4x4`]) is replaced by `0x750930` before any call, so no client reaches it. The
//! rotation routine is the default that every set but 3DNow keeps.
//!
//! The x87 products run at 53 bits, so each output is one `f64` chain rounded once with `as f32`
//! at its `fstp dword`. The SSE2 product rounds to single precision after every multiply and add.

/// `0x7507bb` (`cdecl`, `ret 0xc`; `[ebp+8]` is the output, `[ebp+0xc]` is `b` and `[ebp+0x10]`
/// is `c`): the row-major product `a = b · c` that `0x750930` stores in the slot, run by a
/// processor without SSE2 (or with the registry override). Each output is
/// `(b[i][0]·c[0][j] + b[i][2]·c[2][j]) + (b[i][1]·c[1][j] + b[i][3]·c[3][j])`, the four products
/// exact at 53 bits, the sums rounded to 53 bits, narrowed once at its `fstp dword`
/// (`0x750810` to `0x75081a`, `0x750877` to `0x750881`): after the four products the `fxch`
/// and `faddp` pairs add the first and third, then the second and fourth, then the two sums.
///
/// The routine has three paths. With `c` distinct from the output it walks `b` by rows
/// (`0x750841`); with `c` the output and `b` distinct it walks `c` by columns (`0x7507d5`),
/// which forms the same sums in the same order; with `b` and `c` both the output it copies `c`
/// to a local and takes the first path (`0x750833`). Each path reads what it needs before
/// storing over it, so every case equals the disjoint result.
pub fn m2_matmul4x4_base(a: &mut [f32; 16], b: &[f32; 16], c: &[f32; 16]) {
    let f = f64::from;
    let mut out = [0f32; 16];
    for i in 0..4 {
        for j in 0..4 {
            let p = |k: usize| f(b[4 * i + k]) * f(c[4 * k + j]);
            out[4 * i + j] = ((p(0) + p(2)) + (p(1) + p(3))) as f32;
        }
    }
    *a = out;
}

/// `0x74a7c6` (`cdecl`, `ret 0xc`; `[ebp+8]` is `a`, the output, `[ebp+0xc]` is `b` and
/// `[ebp+0x10]` is `c`): the table's original entry, which no client reaches. The row-major product `a = b · c`, `a[4i+j] = Σ_k b[4i+k] · c[4k+j]`. Each
/// output is a four-term `fld`, `fmul`, `faddp` chain summed from `k = 3` down to 0 and narrowed
/// once at its `fstp dword [esi+k]`.
///
/// When the output aliases an input (`0x74a7d4`), the exe writes a 0x40-byte local copy and moves
/// it back with `rep movsd`, so the result equals the disjoint one. The borrow rules rule out
/// aliasing here, and writing each `a[k]` as it is produced cannot change a later read of `b` or
/// `c`, so one path covers every case.
pub fn m2_matmul4x4(a: &mut [f32; 16], b: &[f32; 16], c: &[f32; 16]) {
    let f = f64::from;
    for i in 0..4 {
        for j in 0..4 {
            // The four terms are added from `k = 3` down to 0, narrowed once at the store.
            let s = f(b[4 * i + 3]) * f(c[12 + j])
                + f(b[4 * i + 2]) * f(c[8 + j])
                + f(b[4 * i + 1]) * f(c[4 + j])
                + f(b[4 * i]) * f(c[j]);
            a[4 * i + j] = s as f32;
        }
    }
}

/// `0x74b6bb` (`cdecl`, `ret 8`; `[ebp+8]` is the output, `[ebp+0xc]` the quaternion `x, y, z, w`):
/// the row-major rotation matrix (`systems/animation/pose.md#rotation-leg`), built from the doubled
/// components (the `2.0` at `0x801628`). Column 3 and the translation row are 0 and `m15` is 1.0
/// (`fld1; fstp`). The narrowing of each operand, at `0x74b6c6` to `0x74b79c`:
/// - `2x` and `2y` are stored as floats, `2z` stays at 53 bits.
/// - `2xw`, `2yw`, `2zw` and `2zz` stay at 53 bits; `2xx`, `2xy`, `2xz`, `2yy` and `2yz` are stored
///   as floats.
/// - Each off-diagonal element, and `m0` and `m5` (`1 - A - B`), take the stored float as the
///   memory operand and the 53-bit register as the other term, narrowed once at the store.
/// - `m10` reloads the float stored by the `fst dword` at `0x74b74a` and subtracts the stored
///   `2yy`: `f32(f32(1 - f32(2xx)) - f32(2yy))`.
pub fn m2_quat_to_mat4(out: &mut [f32; 16], q: &[f32; 4]) {
    let f = f64::from;
    let (x, y, z, w) = (f(q[0]), f(q[1]), f(q[2]), f(q[3]));
    // `2x` and `2y` are stored as floats; `2z` stays a register (`fmul [eax+8]`).
    let (x2, y2) = (f((2.0 * x) as f32), f((2.0 * y) as f32));
    let z2 = 2.0 * z;

    // The products stored as floats, each a memory operand.
    let (fxx, fyy) = ((x * x2) as f32, (y * y2) as f32);
    let (fxy, fxz, fyz) = ((x * y2) as f32, (x * z2) as f32, (y * z2) as f32);
    // The products that stay in registers at 53 bits.
    let pzz = z * z2;
    let (pwx, pwy, pwz) = (w * x2, w * y2, w * z2);

    out[0] = ((1.0 - f(fyy)) - pzz) as f32; // 1 − 2yy − 2zz
    out[1] = (f(fxy) + pwz) as f32; // 2xy + 2wz
    out[2] = (f(fxz) - pwy) as f32; // 2xz − 2wy
    out[3] = 0.0;

    out[4] = (f(fxy) - pwz) as f32; // 2xy − 2wz
    out[5] = ((1.0 - f(fxx)) - pzz) as f32; // 1 − 2xx − 2zz
    out[6] = (f(fyz) + pwx) as f32; // 2yz + 2wx
    out[7] = 0.0;

    out[8] = (f(fxz) + pwy) as f32; // 2xz + 2wy
    out[9] = (f(fyz) - pwx) as f32; // 2yz − 2wx
    out[10] = (f((1.0 - f(fxx)) as f32) - f(fyy)) as f32; // 1 − 2xx − 2yy (floats throughout)
    out[11] = 0.0;

    out[12] = 0.0;
    out[13] = 0.0;
    out[14] = 0.0;
    out[15] = 1.0;
}
