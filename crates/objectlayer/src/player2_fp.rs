//! Float kernels of the player's interaction code: `LootCorpse` (`0x5df130`) and `LootUnit`
//! (`0x5df2a0`) (`systems/objectlayer/interaction.md#looting`), the three range-checked dialogs
//! `SpiritHealerDialog` (`0x5df730`), `TalentWipeDialog` (`0x5df980`) and `PetUnlearnDialog`
//! (`0x5dfba0`) (`#confirmation-dialogs`), `IsInsideAreaTrigger` (`0x5e22d0`)
//! (`systems/objectlayer/players.md#area-triggers`) and `OnDrunkChanged` (`0x5e2a90`)
//! (`systems/objectlayer/playerfields.md#drunk-messages`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fstp dword`. The positions come from
//! the virtual `GetPosition` (slot 5); their results are inputs.

use crate::x87::{fcom_ah, ftol};

/// A position as `GetPosition` writes it.
pub type Vec3 = [f32; 3];

/// `_DAT_0080b054` (`0x41c80000`): 25.0, the squared distance within which `0x5df130` loots.
pub const DIST2_5DF130_THRESHOLD: f32 = f32::from_bits(0x41c8_0000);
/// `_DAT_0080b058` (`0x3faaaaab`): the slop added to the radii in `0x5df2a0`.
pub const REACH_5DF2A0_SLOP: f32 = f32::from_bits(0x3faa_aaab);
/// `_DAT_0080a1e8` (`0x40a00000`): 5.0, the least reach of `0x5df2a0`.
pub const REACH_5DF2A0_FLOOR: f32 = f32::from_bits(0x40a0_0000);
/// `_DAT_0080ae90` (`0x3d4ccccd`): 0.05, the skill rank scale of `0x5df730`.
pub const SKILL_SCALE_5DF730: f32 = f32::from_bits(0x3d4c_cccd);
/// `_DAT_008029d0` (`0x3c23d70a`): 0.01, the drunk level scale of `0x5e2a90`.
pub const DRUNK_SCALE_5E2A90: f32 = f32::from_bits(0x3c23_d70a);

/// The squared distance the four range gates share: `dx`, `dy` and `dz` are `base` less
/// `subtracted`, left on the stack at 53 bits, and the sum is `(dz² + dy²) + dx²`.
fn dist2_yx(base: Vec3, subtracted: Vec3) -> f64 {
    let dx = f64::from(base[0]) - f64::from(subtracted[0]);
    let dy = f64::from(base[1]) - f64::from(subtracted[1]);
    let dz = f64::from(base[2]) - f64::from(subtracted[2]);
    (dz * dz + dy * dy) + dx * dx
}

/// The squared distance of `0x5df130` (`0x5df1a0` to `0x5df1c1`), `0x5df2a0` (`0x5df355` to
/// `0x5df376`), `0x5df730` (`0x5df7a1` to `0x5df7bf`), `0x5df980` (`0x5df9dc` to `0x5df9fa`) and
/// `0x5dfba0` (`0x5dfbfc` to `0x5dfc1a`).
pub fn range_dist2(base: Vec3, subtracted: Vec3) -> f64 {
    dist2_yx(base, subtracted)
}

/// `fcomp` of the squared distance against an `f32` threshold and `test ah,0x41`: the result is
/// non-zero for `d² <= thr` and for NaN, which `jne` (`0x5df1d4`) and `je` (`0x5df7d2`, `0x5dfa0d`,
/// `0x5dfc2d`) read.
fn in_range_le(d2: f64, threshold: f32) -> bool {
    fcom_ah(d2, f64::from(threshold)) & 0x41 != 0
}

/// `LootCorpse` (`0x5df130`), `0x5df1a0` to `0x5df1d4`: `this` position less the target's, `fcomp
/// 25.0`; `jne` takes the near arm for `d² <= 25` and NaN.
pub fn loot_range_5df130(this_pos: Vec3, target_pos: Vec3) -> bool {
    in_range_le(dist2_yx(this_pos, target_pos), DIST2_5DF130_THRESHOLD)
}

/// The reach of `LootUnit` (`0x5df2a0`), `0x5df301` to `0x5df337`: `(target radius + this radius) +
/// 1.3333334` on the stack; `fld 5.0; fcomp st(1)` and `test ah,0x41; jne` keep it when `5.0 <=
/// reach` and for NaN, else store 5.0. `fstp dword [ebp-4]` rounds the kept value to `f32`.
pub fn interact_reach_value(this_radius: f32, target_radius: f32) -> f32 {
    let reach = (f64::from(target_radius) + f64::from(this_radius)) + f64::from(REACH_5DF2A0_SLOP);
    if fcom_ah(f64::from(REACH_5DF2A0_FLOOR), reach) & 0x41 == 0 {
        REACH_5DF2A0_FLOOR
    } else {
        reach as f32
    }
}

/// `LootUnit` (`0x5df2a0`), `0x5df33c` to `0x5df38b`: the stored reach is reloaded and squared, and
/// `fcompp` of the square against the squared distance with `test ah,5; jp` takes the near arm for
/// `r² >= d²` and NaN.
pub fn interact_reach_5df2a0(
    this_radius: f32,
    target_radius: f32,
    this_pos: Vec3,
    target_pos: Vec3,
) -> bool {
    let reach = interact_reach_value(this_radius, target_radius);
    let r2 = f64::from(reach) * f64::from(reach);
    let d2 = dist2_yx(this_pos, target_pos);
    (fcom_ah(r2, d2) & 0x05).count_ones().is_multiple_of(2)
}

/// `SpiritHealerDialog` (`0x5df730`), `0x5df7a1` to `0x5df7d2`: the target's position less this
/// player's, `fcomp` against the runtime threshold `[0xc4c28c]`; `je` leaves for `d² > thr`.
pub fn range_gate_5df730(this_pos: Vec3, target_pos: Vec3, threshold: f32) -> bool {
    in_range_le(dist2_yx(target_pos, this_pos), threshold)
}

/// The skill rank argument of `0x5df730`, `0x5df81e` to `0x5df827`: `fild dword` of the skill,
/// `fmul 0.05`, and `__ftol`.
pub fn skill_rank_scale_5df730(skill: i32) -> i32 {
    ftol(f64::from(skill) * f64::from(SKILL_SCALE_5DF730))
}

/// `TalentWipeDialog` (`0x5df980`), `0x5df9dc` to `0x5dfa0d`: as [`range_gate_5df730`].
pub fn range_gate_5df980(this_pos: Vec3, vendor_pos: Vec3, threshold: f32) -> bool {
    in_range_le(dist2_yx(vendor_pos, this_pos), threshold)
}

/// `PetUnlearnDialog` (`0x5dfba0`), `0x5dfbfc` to `0x5dfc2d`: as [`range_gate_5df730`].
pub fn range_gate_5dfba0(this_pos: Vec3, npc_pos: Vec3, threshold: f32) -> bool {
    in_range_le(dist2_yx(npc_pos, this_pos), threshold)
}

/// Whether `IsInsideAreaTrigger` (`0x5e22d0`) tests a box (`0x5e22ea` to `0x5e22f8`): `fcomp` of
/// the radius against 0.0 and `test ah,0x44; jnp` take the box path on an odd number of C2 and C3,
/// which is a radius of exactly 0 either sign; negative, positive and NaN radii test the sphere.
pub fn area_trigger_is_box(radius: f32) -> bool {
    !(fcom_ah(f64::from(radius), 0.0) & 0x44)
        .count_ones()
        .is_multiple_of(2)
}

/// The sphere test of `IsInsideAreaTrigger` (`0x5e22d0`), `0x5e22fa` to `0x5e2331`: `dx`, `dy` and
/// `dz` are the centre less the point, kept at 53 bits, the sum is `(dz² + dx²) + dy²`, and `r²` is
/// the radius times itself. `fcompp; test ah,1; je` is inside when `r² >= d²`; NaN is outside.
pub fn area_trigger_sphere_5e22d0(center: Vec3, point: Vec3, radius: f32) -> bool {
    let dx = f64::from(center[0]) - f64::from(point[0]);
    let dy = f64::from(center[1]) - f64::from(point[1]);
    let dz = f64::from(center[2]) - f64::from(point[2]);
    let d2 = (dz * dz + dx * dx) + dy * dy;
    let r2 = f64::from(radius) * f64::from(radius);
    fcom_ah(r2, d2) & 0x01 == 0
}

/// The drunk fraction of `OnDrunkChanged` (`0x5e2a90`), `0x5e2ac9` to `0x5e2aeb`: the level byte is
/// kept below `0x64` by `cmp al,0x64; jb`, `movzx` stores it, and `fild dword; fmul 0.01; fstp
/// dword` rounds the product to `f32`.
pub fn drunk_fraction_5e2a90(level: u8) -> f32 {
    let clamped = i32::from(level.min(0x64));
    (f64::from(clamped) * f64::from(DRUNK_SCALE_5E2A90)) as f32
}
