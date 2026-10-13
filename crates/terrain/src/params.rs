//! `InitFalloff` (`0x68a1f0`) and `CWorld::SetFarClip` (`0x6725d0`).

use crate::x87::ftol;

/// The values `0x68a1f0` computes.
pub struct FalloffRecip {
    /// `+0x18`: `f32(1/inner)`.
    pub inv_r1: f32,
    /// `+0x1c`: `f32(1/outer)`.
    pub inv_r2: f32,
    /// `+0x10`: `−inner`.
    pub neg_r1: f32,
}

/// `InitFalloff` (`0x68a1f0`, ecx = the record, then a point, the inner and outer radii and three
/// dwords): `1.0` (`[0x7ff9d8]`) divided by each radius (`0x68a1fc`, `0x68a243`) and the inner
/// negated (`0x68a24c`), each stored as `f32`; the rest are copies.
pub fn falloff_recip(r1: f32, r2: f32) -> FalloffRecip {
    FalloffRecip {
        inv_r1: (1.0_f64 / f64::from(r1)) as f32,
        inv_r2: (1.0_f64 / f64::from(r2)) as f32,
        neg_r1: -r1,
    }
}

/// What `0x6725d0` writes.
pub struct ParamSetterResult {
    /// Whether it wrote.
    pub changed: bool,
    /// The bits stored to `0xc7b498`, the argument's.
    pub stored_bits: Option<u32>,
    /// `1 − __ftol(v·−0.03)`, stored to `0xc7b2b8` and `0xc7b2bc`.
    pub index: Option<i32>,
}

/// `CWorld::SetFarClip` (`0x6725d0`): `fcomp` of the stored `[0xc7b498]` with the argument, `test
/// ah,0x44; jnp` returns when they are equal; otherwise the argument is stored and `1 −
/// __ftol(v·[0x80fecc])` (−0.03, `0xbcf5c290`, the product in the register) goes to `0xc7b2b8` and
/// `0xc7b2bc`. NaN on either side writes.
pub fn param_setter(stored: f32, new: f32) -> ParamSetterResult {
    if stored == new {
        return ParamSetterResult {
            changed: false,
            stored_bits: None,
            index: None,
        };
    }
    let scale = f32::from_bits(0xbcf5_c290);
    let idx = ftol(f64::from(new) * f64::from(scale));
    ParamSetterResult {
        changed: true,
        stored_bits: Some(new.to_bits()),
        index: Some(1i32.wrapping_sub(idx)),
    }
}
