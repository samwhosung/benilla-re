//! The minimap orientation basis renormalize, `0x6da180`.
//!
//! The function renormalizes the 3 by 3 basis (rows at `0xceaa20`, `0xceaa30`, `0xceaa40`) when row 0
//! has drifted from unit length. Two external calls bracket it: `0x670570` derives the basis from the
//! camera and `0x7bdd60` applies the facing rotation; the test stubs both.
//!
//! Row 0's squares are stored and reloaded as `f32` (`fstp dword`) before they are summed; rows 1 and
//! 2 square in `f64` on the x87 stack. The factors are `invlen0 / sqrt(s0)`, which is `1 / s0`, for
//! row 0 and `invlen0 / sqrt(sk)` for rows 1 and 2, where `invlen0 = 1 / sqrt(s0)`: row 0's reciprocal
//! length serves all three rows.

/// 2.384e-7 drift epsilon (`0x8029d4`).
const EPS: f32 = f32::from_bits(0x3480_0000);

/// The renormalize of `0x6da180`.
///
/// `b` is the nine basis floats `[a20, a24, a28, a30, a34, a38, a40, a44, a48]` (rows 0 to 2), returned
/// renormalized, or unchanged when `|s0 - 1| < eps`. The function also zeroes the translation row
/// (`0xceaa50`, `0xceaa54`, `0xceaa58`) and applies the facing rotation; neither is arithmetic here.
pub fn orientation_renormalize(b: [f32; 9]) -> [f32; 9] {
    // row0: squares rounded to f32 (fstp m32) then reloaded; sum order (a24²+a28²)+a20².
    let f4 = (b[0] as f64 * b[0] as f64) as f32; // a20²
    let fc = (b[1] as f64 * b[1] as f64) as f32; // a24²
    let f8 = (b[2] as f64 * b[2] as f64) as f32; // a28²
    let s0 = (fc as f64 + f8 as f64) + f4 as f64;
    let mut out = b;
    if (s0 - 1.0).abs() < EPS as f64 {
        return out; // already unit — skip (jnp)
    }
    let invlen0 = 1.0_f64 / s0.sqrt();
    let factor0 = invlen0 / s0.sqrt(); // = 1/s0, via two separate fsqrt
    out[0] = (b[0] as f64 * factor0) as f32;
    out[1] = (b[1] as f64 * factor0) as f32;
    out[2] = (b[2] as f64 * factor0) as f32;
    // rows 1,2: squares in f64; factor = invlen0/√sk.
    let s1 = (b[3] as f64 * b[3] as f64 + b[4] as f64 * b[4] as f64) + b[5] as f64 * b[5] as f64;
    let factor1 = invlen0 / s1.sqrt();
    out[3] = (b[3] as f64 * factor1) as f32;
    out[4] = (b[4] as f64 * factor1) as f32;
    out[5] = (b[5] as f64 * factor1) as f32;
    let s2 = (b[6] as f64 * b[6] as f64 + b[7] as f64 * b[7] as f64) + b[8] as f64 * b[8] as f64;
    let factor2 = invlen0 / s2.sqrt();
    out[6] = (b[6] as f64 * factor2) as f32;
    out[7] = (b[7] as f64 * factor2) as f32;
    out[8] = (b[8] as f64 * factor2) as f32;
    out
}
