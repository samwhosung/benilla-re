//! The player's proximity and ratio gates: the confirmation-dialog range checks `0x5df8c0`,
//! `0x5dfb10`, `0x5dfd30` and `0x5dff50`, `CGPlayer_C::OnAttackStart` (`0x5ea800`) and the two
//! ratio legs of `CGPlayer_C::StopMeleeAttack` (`0x5ecac0`)
//! (`systems/objectlayer/interaction.md#confirmation-dialogs` and
//! `playercombat.md#the-melee-attack`).
//!
//! The object positions come from the virtual `GetPosition(out)` (vtable `+0x14`), which is an
//! input here. Each distance test keeps its differences on the x87 stack at 53 bits and sums the
//! squares in the order [`dist2`] documents; the compares read `fcomp` and `test ah` as noted on
//! each function. The x87 runs at 53 bits: an operation is an `f64` operation.

/// A position as `GetPosition` writes it, three packed `f32`.
pub type Vec3 = [f32; 3];

/// The squared distance of the range checks `0x5df8c0`, `0x5dfb10`, `0x5dfd30` and `0x5dff50` and of
/// `0x5ea800`: `eax` holds the object whose position is the base and `esi` the one subtracted. The
/// three differences stay on the stack at 53 bits and the squares sum as `(dz^2 + dx^2) + dy^2`.
#[inline]
fn dist2(base: Vec3, subtracted: Vec3) -> f64 {
    let dx = f64::from(base[0]) - f64::from(subtracted[0]);
    let dy = f64::from(base[1]) - f64::from(subtracted[1]);
    let dz = f64::from(base[2]) - f64::from(subtracted[2]);
    (dz * dz + dx * dx) + dy * dy
}

/// The range check of the four confirmation dialogs, `0x5df8c0` and its clones `0x5dfb10`,
/// `0x5dfd30` and `0x5dff50`, which differ only in the guid globals, the event id pushed and the
/// call to `0x468460`. Each resolves the remembered NPC with `ClntObjMgrObjectPtr`; if found it
/// reads `this.GetPosition` into the subtracted operand and `target.GetPosition` into the base, and
/// compares the squared distance with `_DAT_00c4c28c`.
///
/// `fcomp thr; test ah,0x41; jp out`: greater is 00 and unordered is 11, both even, so they leave;
/// less is C0 and equal is C3, both odd, so they stay. It returns 1 when the NPC resolves and
/// `dist^2 <= thr` ordered, and 0 without resolving before any float runs.
pub fn range_gate_5df8c0(resolved: bool, this_pos: Vec3, target_pos: Vec3, threshold: f32) -> bool {
    if !resolved {
        return false;
    }
    let d2 = dist2(target_pos, this_pos);
    d2 <= f64::from(threshold)
}

/// `_DAT_0080b058` (`0x3faaaaab`): 1.3333334, added to the radii in `OnAttackStart`.
pub const REACH_5EA800_SLOP: f32 = f32::from_bits(0x3faa_aaab);
/// `_DAT_0080a1e8` (`0x40a00000`): 5.0, the least reach.
pub const REACH_5EA800_FLOOR: f32 = f32::from_bits(0x40a0_0000);
/// 4.0, the factor on the reach.
pub const REACH_5EA800_SCALE: f32 = f32::from_bits(0x4080_0000);
/// The leash check of `CGPlayer_C::OnAttackStart` (`0x5ea800`, `ret 4`), reached from
/// `SMSG_ATTACKERSTATEUPDATE` (`0x6259b6`) for the local player's attack on a resolved unit. With a
/// target it builds the reach and calls `SendAttackStop` (`0x624370`, `CMSG_ATTACKSTOP`) when the
/// target is at or beyond it:
/// - `reach = (target_radius + this_radius) + 1.3333334`, the radii being the floats at `+0x1f0` of
///   each `[obj+0x110]`;
/// - `fld 5.0; fcomp st(1); test ah,0x41; jne` keeps `reach` unless `5.0 > reach` ordered, so a NaN
///   reach stays and a smaller one becomes 5.0;
/// - `reach * 4.0`, stored as `f32` at `[ebp+8]`;
/// - `fld reach; fmul reach` against `dist^2` by `fcompp; test ah,0x41; jp skip`: less and equal
///   are odd and send, greater and unordered are even and skip. It sends when `reach^2 <= dist^2`
///   ordered.
pub fn reach_gate_5ea800(
    this_radius: f32,
    target_radius: f32,
    this_pos: Vec3,
    target_pos: Vec3,
) -> bool {
    let mut reach =
        (f64::from(target_radius) + f64::from(this_radius)) + f64::from(REACH_5EA800_SLOP);
    if f64::from(REACH_5EA800_FLOOR) > reach {
        reach = f64::from(REACH_5EA800_FLOOR);
    }
    let reach_scaled: f32 = (reach * f64::from(REACH_5EA800_SCALE)) as f32;
    let d2 = dist2(this_pos, target_pos);
    let r2 = f64::from(reach_scaled) * f64::from(reach_scaled);
    r2 <= d2
}

/// 0.5 (`0x3f000000`): the ratio below which `StopMeleeAttack` raises the low-health or low-mana tutorial.
pub const WARN_5ECAC0_RATIO: f32 = f32::from_bits(0x3f00_0000);
/// The two ratio legs of `CGPlayer_C::StopMeleeAttack` (`0x5ecac0`), which after its self and
/// attack-lock gates sends `CMSG_ATTACKSTOP` (`0x142`) and ends in `CancelNextSwingSpells`
/// (`0x6e6f30`). Each leg calls `TriggerTutorial` (`0x4b5390`) with id `0xa` (low health) or `0xb`
/// (low mana) on the unit's fields `[this+0x110]`:
/// - health: `fild +0x40; fidiv +0x58; fcomp 0.5; test ah,5; jp`: it fires when `health /
///   max_health < 0.5` ordered (less is C0 alone, odd; greater, equal and NaN are even);
/// - mana: only when the power type byte `+0x7b` is 0. `fild +0x5c; fcom 0.0; test ah,0x44; jnp`
///   fires without dividing when `max_power == 0`; otherwise `fidivr +0x44; fcomp 0.5` fires when
///   `power / max_power < 0.5` ordered.
///
/// Returns whether each tutorial fires. The self gate and `0x60ecb0` are held true by the test.
pub fn lowhp_warn_5ecac0(
    health: i32,
    max_health: i32,
    power: i32,
    max_power: i32,
    power_type_zero: bool,
) -> (bool, bool) {
    let hr = f64::from(health) / f64::from(max_health);
    let health_warn = hr < f64::from(WARN_5ECAC0_RATIO);

    let mana_warn = if !power_type_zero {
        false
    } else if max_power == 0 {
        true
    } else {
        let pr = f64::from(power) / f64::from(max_power);
        pr < f64::from(WARN_5ECAC0_RATIO)
    };
    (health_warn, mana_warn)
}
