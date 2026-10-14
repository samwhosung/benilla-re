//! Float kernels of the player's range checks: `CanLootNow` (`0x5ec110`)
//! (`systems/objectlayer/interaction.md#looting`), `InteractWithNpc` (`0x5f0130`)
//! (`#talking-to-an-npc`) and `InteractWithSpell` (`0x5f05e0`) (`#interacting-through-a-spell`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fstp dword`. The positions come from
//! the virtual `GetPosition` (slot 5), and `0x4549a0` copies three dwords to its vector; their
//! results are inputs.

use crate::x87::fcom_ah;

/// A position as `GetPosition` writes it.
pub type Vec3 = [f32; 3];

/// `_DAT_0080b058` (`0x3faaaaab`): 1.3333334, added to the radii in `0x5ec110`.
pub const REACH_SLOP: f32 = f32::from_bits(0x3faa_aaab);
/// `_DAT_0080a1e8` (`0x40a00000`): 5.0, the least reach of `0x5ec110`.
pub const REACH_FLOOR: f32 = f32::from_bits(0x40a0_0000);

/// The squared reach of `CanLootNow` (`0x5ec110`), `0x5ec142` to `0x5ec17b`: `(target radius + this
/// radius) + 1.3333334` stays on the stack; `fld 5.0; fcomp st(1)` and `test ah,0x41; jne` keep it
/// when `5.0 <= reach` and for NaN, and otherwise load 5.0. `fld st0; fmul st,st1; fstp dword
/// [ebp-4]` stores the square, rounded to `f32` once.
pub fn interact_reach_sq_5ec110(this_radius: f32, target_radius: f32) -> f32 {
    let reach = (f64::from(target_radius) + f64::from(this_radius)) + f64::from(REACH_SLOP);
    let reach = if fcom_ah(f64::from(REACH_FLOOR), reach) & 0x41 == 0 {
        f64::from(REACH_FLOOR)
    } else {
        reach
    };
    (reach * reach) as f32
}

/// The squared distance of `CanLootNow` (`0x5ec110`), `0x5ec196` to `0x5ec1b8`: this position less
/// the target's, left on the stack, with the sum `(dz² + dy²) + dx²`.
pub fn dist2_5ec110(this_pos: Vec3, target_pos: Vec3) -> f64 {
    let dx = f64::from(this_pos[0]) - f64::from(target_pos[0]);
    let dy = f64::from(this_pos[1]) - f64::from(target_pos[1]);
    let dz = f64::from(this_pos[2]) - f64::from(target_pos[2]);
    (dz * dz + dy * dy) + dx * dx
}

/// The range test of `CanLootNow` (`0x5ec110`), `0x5ec1ba` to `0x5ec1c8`: `fcomp [ebp-4]` of the
/// squared distance against the stored squared reach, and `test ah,0x41; jnp` is taken on an odd
/// number of C0 and C3, which is `d² < r²` and `d² == r²`; greater and NaN return 0.
pub fn interact_in_range_5ec110(dist2: f64, reach_sq: f32) -> bool {
    !(fcom_ah(dist2, f64::from(reach_sq)) & 0x41)
        .count_ones()
        .is_multiple_of(2)
}

/// The squared distance of `InteractWithNpc` (`0x5f0130`), `0x5f01b9` to `0x5f01fd`: each
/// difference of this position less the target's is stored to a dword as an argument of `0x4549a0`,
/// which copies the three; the sum of the copies is `(dy² + dz²) + dx²`, and `fstp dword [ebp-4]`
/// rounds it to `f32`.
pub fn dist2_f32deltas_5f0130(this_pos: Vec3, target_pos: Vec3) -> f32 {
    let dx = (f64::from(this_pos[0]) - f64::from(target_pos[0])) as f32;
    let dy = (f64::from(this_pos[1]) - f64::from(target_pos[1])) as f32;
    let dz = (f64::from(this_pos[2]) - f64::from(target_pos[2])) as f32;
    ((f64::from(dy) * f64::from(dy) + f64::from(dz) * f64::from(dz))
        + f64::from(dx) * f64::from(dx)) as f32
}

/// The range test of `InteractWithNpc` (`0x5f0130`), `0x5f0202` to `0x5f0210` and again `0x5f0238`
/// to `0x5f0246`: `fld [ebp-4]; fcomp [0xc4c28c]` and `test ah,0x41` is non-zero for `d² <= thr`
/// and for NaN. The first `jne` takes it for that; the second `je` leaves for `d² > thr`.
pub fn in_range_5f0130(dist2: f32, threshold: f32) -> bool {
    fcom_ah(f64::from(dist2), f64::from(threshold)) & 0x41 != 0
}

/// The squared distance of `InteractWithSpell` (`0x5f05e0`), `0x5f0656` to `0x5f0690`: as
/// [`dist2_f32deltas_5f0130`] but the copies are squared in the order `(dz² + dy²) + dx²` and the
/// sum stays on the stack at 53 bits.
pub fn dist2_f32deltas_5f05e0(this_pos: Vec3, target_pos: Vec3) -> f64 {
    let dx = (f64::from(this_pos[0]) - f64::from(target_pos[0])) as f32;
    let dy = (f64::from(this_pos[1]) - f64::from(target_pos[1])) as f32;
    let dz = (f64::from(this_pos[2]) - f64::from(target_pos[2])) as f32;
    (f64::from(dz) * f64::from(dz) + f64::from(dy) * f64::from(dy)) + f64::from(dx) * f64::from(dx)
}

/// The range test of `InteractWithSpell` (`0x5f05e0`), `0x5f0692` to `0x5f069f`: `range·range` is
/// formed on the stack, `fcompp` compares it with the squared distance, and `test ah,0x41; je` is
/// taken, to `0x5f06c6`, for `r² > d²`; equal, less and NaN fall through. Returns whether `je` is
/// taken.
pub fn attack_range_gate_5f05e0(range: f32, dist2: f64) -> bool {
    fcom_ah(f64::from(range) * f64::from(range), dist2) & 0x41 == 0
}
