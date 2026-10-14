//! Float kernels of unit methods in `[0x604140, 0x608e00]`: `HandlePetDismissSoundPacket`
//! (`0x604140`), `CGUnit_C::ScaleQuestMarker` (`0x607570`), `CGUnit_C::UpdateBreathState`
//! (`0x607710`), `CGUnit_C::UpdateNamePlate` (`0x6086e0`), `CGUnit_C::ShowChatBubble` (`0x608ac0`)
//! and `CGUnit_C::DrawSelectionCircle` (`0x608e00`) (`systems/objectlayer/` `unitnames.md`,
//! `unitbody.md#breath`, `unitdisplay.md#selection-circle-and-colour`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`; a value an
//! `fst` stores while the register stays live is read from the register by the next compare.

use crate::x87::not_above;

/// 1.0 (`0x7ff9d8`).
pub const ONE: f32 = 1.0;
/// 5.0 (`0x80c44c`), added to the unit's collision height in the breath test.
pub const MELEE_SLACK: f32 = 5.0;
/// 400.0 (`0x806798`), the chat-bubble distance squared (20 yards).
pub const BUBBLE_DIST2: f32 = 400.0;

/// `HandlePetDismissSoundPacket` (`0x604140`): the sound is played one yard above the point three
/// `CDataStore::GetFloat` reads fill, `fld [ebp-4]; fadd 1.0; fstp [ebp-4]` at `0x6041cb`. `vz` is
/// the third read.
pub fn pet_dismiss_pos_lift(vz: f32) -> f32 {
    (f64::from(vz) + f64::from(ONE)) as f32
}

/// The depth test of `UpdateBreathState` (`0x607710`, `ret 4`), from `0x60775b`. After the liquid
/// query `0x670630` finds a surface at `dist` (`[ebp-4]`), the unit's height `+0x90` times its
/// scale `+0xbc` is stored as `reach` (`[ebp-8]`), `vtable+0x14` gives the unit's position and the
/// compare is `fcompp` of `5.0 + reach` against `dist - vz`: `test ah,5; jp 0x6077b2` skips setting
/// bit 0x20 of `+0xc58` for an even count of C0 and C2, so the bit is set only when `5.0 + reach <
/// dist - vz` is ordered. This returns whether it is set.
pub fn attack_range_in(dist: f32, vec_z: f32, reach_base: f32, reach_scale: f32) -> bool {
    let reach = f64::from(reach_base) * f64::from(reach_scale);
    let lhs = f64::from(MELEE_SLACK) + reach;
    let rhs = f64::from(dist) - f64::from(vec_z);
    lhs < rhs
}

/// Whether `UpdateNamePlate` (`0x6086e0`) moves an existing plate: from `0x60876e`, `[0xc4da34] &
/// 2` always moves it; otherwise `dy^2 + dx^2` of the new position against the old (`[node+0x508]`,
/// `[node+0x50c]`) is compared with 0.0 by `fcomp; test ah,0x44; jnp`, which skips the move for a
/// distance of exactly zero (C3 alone, odd) and moves it for a positive one and for NaN (C2 and C3,
/// even).
pub fn nameplate_pos_changed(old: [f32; 2], new: [f32; 2], flag2: bool) -> bool {
    if flag2 {
        return true;
    }
    let dx = f64::from(new[0]) - f64::from(old[0]);
    let dy = f64::from(new[1]) - f64::from(old[1]);
    let dist2 = dy * dy + dx * dx;
    dist2 != 0.0 || dist2.is_nan()
}

/// The distance gate of `ShowChatBubble` (`0x608ac0`) at `0x608b74`: with `a` the unit's position
/// and `b` the active player's, `dist^2 = (dz^2 + dy^2) + dx^2` is compared with 400.0 by `fcomp;
/// test ah,0x41; je 0x608beb`, which skips the bubble when `dist^2 > 400` is ordered. This returns
/// whether the bubble goes on: `dist^2 <= 400` or NaN.
pub fn bubble_in_range(a: [f32; 3], b: [f32; 3]) -> bool {
    let dx = f64::from(a[0]) - f64::from(b[0]);
    let dy = f64::from(a[1]) - f64::from(b[1]);
    let dz = f64::from(a[2]) - f64::from(b[2]);
    let dist2 = (dz * dz + dy * dy) + dx * dx;
    not_above(dist2, f64::from(BUBBLE_DIST2))
}

/// `DrawSelectionCircle` (`0x608e00`, `ret`): the square the circle is drawn as. With `s = scale *
/// m` (`+0xcf0` times `[+8]+0x10`) stored twice as `f32`, `2s` stored as `f32`, and the position
/// `(px, py, pz)` from `vtable+0x14`, the corner is `(px - s, py - s, pz - 2s)` and the size is
/// `s`; `px - s` and `py - s` subtract the register `s`, and `pz - 2s` the stored `2s`. This
/// returns `[px - s, py - s, pz - 2s, s]`.
pub fn shadow_quad(scale: f32, m: f32, px: f32, py: f32, pz: f32) -> [f32; 4] {
    let s = f64::from(scale) * f64::from(m);
    let s_f32 = s as f32;
    let two_s = (s + s) as f32;
    let qx = (f64::from(px) - s) as f32;
    let qy = (f64::from(py) - s) as f32;
    let qz = (f64::from(pz) - f64::from(two_s)) as f32;
    [qx, qy, qz, s_f32]
}

/// `ScaleQuestMarker` (`0x607570`): the marker model gets a uniform scale `1 / len`, `len` being
/// the length of the attachment matrix vector `0x712de0` fills, `sqrt((v1^2 + v2^2) + v0^2)`. `fst
/// [ebp-4]` stores `len` as `f32` and leaves the register live:
/// - `fcomp 1.0; test ah,0x44; jnp` reads the live register and leaves when `len == 1.0` (C3 alone,
///   odd); NaN is C2 and C3, even, so it goes on;
/// - `fld [ebp-4]; fcomp 0.0; test ah,0x41; jne` reads the stored float and leaves unless it is
///   greater than 0 (zero is C3, NaN is all three);
/// - the scale is `1.0 / len_f32` stored as `f32`.
///
/// It returns `None` for a call that leaves before `0x710620` and `Some(scale)` otherwise.
pub fn glow_inv_len(v0: f32, v1: f32, v2: f32) -> Option<f32> {
    let len2 = (f64::from(v1) * f64::from(v1) + f64::from(v2) * f64::from(v2))
        + f64::from(v0) * f64::from(v0);
    let len_live = len2.sqrt();
    let len_f32 = len_live as f32;
    if len_live == 1.0 {
        return None;
    }
    if not_above(f64::from(len_f32), 0.0) {
        return None;
    }
    Some((f64::from(ONE) / f64::from(len_f32)) as f32)
}
