//! Float kernels of the combat log's range tests and the blood front/back choice: `SpillBlood`
//! (`0x625010`), `ResolveCombatLogPair` (`0x626630`), the one-sided range tests of `0x627de0`,
//! `0x628ff0`, `0x62aac0` and `0x62b000`, and `CombatLogSpellCast` (`0x62b1b0`)
//! (`systems/objectlayer/unitcombat.md#blood`, `systems/objectlayer/combatlog.md#range` and
//! `#the-one-sided-range-test`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. A
//! comparison reads the stack at 53 bits. The positions, the ranges and the sine and cosine of the
//! facing come from other code; their results are inputs.

use crate::x87::fcom_ah;

/// `_DAT_007ffd74` (`0x00000000`): 0.0, the facing dot's comparand.
pub const FACING_ZERO: f32 = 0.0;

/// Whether `test ah,mask; jnp` jumps after `fcom` of `a` against `b`: the parity flag is clear when
/// the masked status byte holds an odd number of bits.
fn jnp_taken(a: f64, b: f64, mask: u8) -> bool {
    !(fcom_ah(a, b) & mask).count_ones().is_multiple_of(2)
}

/// The facing dot of `SpillBlood` (`0x625010`), `0x625080` to `0x6250cb`. `dx` and `dy` are the
/// attacker's position less the victim's, each stored to a dword by `fstp`; `cos` and `sin` are the
/// `fsincos` of the victim's facing, stored as dwords. The dot is `dy·sin + cos·dx`.
pub fn blood_spurt_dot(attacker_xy: [f32; 2], victim_xy: [f32; 2], cos: f32, sin: f32) -> f64 {
    let dx = (f64::from(attacker_xy[0]) - f64::from(victim_xy[0])) as f32;
    let dy = (f64::from(attacker_xy[1]) - f64::from(victim_xy[1])) as f32;
    f64::from(dy) * f64::from(sin) + f64::from(cos) * f64::from(dx)
}

/// The attachment choice of `SpillBlood` (`0x625010`), `0x625010` to `0x6250de`: `fcomp 0.0; test
/// ah,5; jnp` keeps `edi = 0x10`, the back effect, when the dot is below 0 and sets `0xf`, the
/// front effect, otherwise, 0 and NaN included. Returns whether the front effect is chosen.
pub fn blood_spurt_front_back(
    attacker_xy: [f32; 2],
    victim_xy: [f32; 2],
    cos: f32,
    sin: f32,
) -> bool {
    let dot = blood_spurt_dot(attacker_xy, victim_xy, cos, sin);
    !jnp_taken(dot, f64::from(FACING_ZERO), 0x05)
}

/// The squared range `0x626630` stores: `fld r; fmul r; fstp dword` at `0x62672a` for the source's
/// and `fld st0; fmul st,st1; fstp dword` at `0x626737` for the target's, both rounded to `f32`.
pub fn range_squared_stored(range: f32) -> f32 {
    (f64::from(range) * f64::from(range)) as f32
}

/// The source's squared distance in `ResolveCombatLogPair` (`0x626630`), `0x626768` to `0x6267c0`:
/// the differences are stored to dwords at `0x626773`, `0x62677c` and `0x626789`, and the sum is
/// `(dz² + dy²) + dx²`.
pub fn source_dist2(base: [f32; 3], sub: [f32; 3]) -> f64 {
    let dx = (f64::from(base[0]) - f64::from(sub[0])) as f32;
    let dy = (f64::from(base[1]) - f64::from(sub[1])) as f32;
    let dz = (f64::from(base[2]) - f64::from(sub[2])) as f32;
    (f64::from(dz) * f64::from(dz) + f64::from(dy) * f64::from(dy)) + f64::from(dx) * f64::from(dx)
}

/// The target's squared distance in `ResolveCombatLogPair` (`0x626630`), `0x62679c` to `0x6267da`:
/// the differences stay on the stack at 53 bits, and the sum is `(dz² + dy²) + dx²`.
pub fn target_dist2(base: [f32; 3], sub: [f32; 3]) -> f64 {
    let dx = f64::from(base[0]) - f64::from(sub[0]);
    let dy = f64::from(base[1]) - f64::from(sub[1]);
    let dz = f64::from(base[2]) - f64::from(sub[2]);
    (dz * dz + dy * dy) + dx * dx
}

/// The source's range test of `ResolveCombatLogPair` (`0x626630`), `0x6267c2` to `0x6267ca`: `fcomp
/// [ebp+0x24]` against the stored squared range and `test ah,5; jnp` pass when `d² < r²`; equal,
/// greater and NaN fail.
pub fn source_in_range(base: [f32; 3], sub: [f32; 3], range2: f32) -> bool {
    jnp_taken(source_dist2(base, sub), f64::from(range2), 0x05)
}

/// The target's range test of `ResolveCombatLogPair` (`0x626630`), `0x6267dc` to `0x6267ea`: `fcomp
/// [ebp+0x1c]` against the stored squared range and `test ah,5; jnp` pass when `d² < r²`.
pub fn target_in_range(base: [f32; 3], sub: [f32; 3], range2: f32) -> bool {
    jnp_taken(target_dist2(base, sub), f64::from(range2), 0x05)
}

/// `ResolveCombatLogPair` (`0x626630`) as `al` at `0x6267f7` and `0x6267fd`: the player is within
/// range of the source, or else within range of the target. The squared ranges are stored as dwords
/// ([`range_squared_stored`]).
pub fn two_endpoint_visible(
    base_a: [f32; 3],
    sub_a: [f32; 3],
    range_a: f32,
    base_b: [f32; 3],
    sub_b: [f32; 3],
    range_b: f32,
) -> bool {
    source_in_range(base_a, sub_a, range_squared_stored(range_a))
        || target_in_range(base_b, sub_b, range_squared_stored(range_b))
}

/// The squared distance of the one-sided range test of `0x627de0` (`0x627f0c` to `0x627f2c`),
/// `0x628ff0` (`0x629083` to `0x6290a3`), `0x62aac0` (`0x62ab71` to `0x62ab91`) and `0x62b000`
/// (`0x62b090` to `0x62b0b0`): the differences stay on the stack and the sum is `(dx² + dz²) +
/// dy²`.
pub fn one_sided_dist2(base: [f32; 3], sub: [f32; 3]) -> f64 {
    let dx = f64::from(base[0]) - f64::from(sub[0]);
    let dy = f64::from(base[1]) - f64::from(sub[1]);
    let dz = f64::from(base[2]) - f64::from(sub[2]);
    (dx * dx + dz * dz) + dy * dy
}

/// The one-sided range test: `range·range` is formed on the stack without a store, `fcompp`
/// compares it with `d²`, and `test ah,0x41; jne` skips the line unless `r² > d²`; equal, less and
/// NaN skip. Returns whether the line is written.
pub fn range_gate_visible(base: [f32; 3], sub: [f32; 3], range: f32) -> bool {
    let r2 = f64::from(range) * f64::from(range);
    fcom_ah(r2, one_sided_dist2(base, sub)) & 0x41 == 0
}

/// The squared distance of `CombatLogSpellCast` (`0x62b1b0`), `0x62b243` to `0x62b27d`: each
/// difference is stored to a dword as an argument of the vector constructor `0x4549a0`, which
/// copies the three into `[ebp-0x18]`; the sum is `(dx² + dy²) + dz²` of the copies.
pub fn quest_dist2(base: [f32; 3], sub: [f32; 3]) -> f64 {
    let dx = (f64::from(base[0]) - f64::from(sub[0])) as f32;
    let dy = (f64::from(base[1]) - f64::from(sub[1])) as f32;
    let dz = (f64::from(base[2]) - f64::from(sub[2])) as f32;
    copied_dist2(dx, dy, dz)
}

/// The sum over differences already stored as dwords and reloaded, `(dx² + dy²) + dz²`: the part of
/// `quest_dist2` from `0x62b269`, and of `0x62b800` and `0x62c360` from their reload.
pub fn copied_dist2(dx: f32, dy: f32, dz: f32) -> f64 {
    (f64::from(dx) * f64::from(dx) + f64::from(dy) * f64::from(dy)) + f64::from(dz) * f64::from(dz)
}

/// The range test of `CombatLogSpellCast` (`0x62b1b0`), `0x62b27f` to `0x62b28c`: `range·range` is
/// formed on the stack, `fcompp; test ah,0x41; jne` skips unless `r² > d²`.
pub fn quest_range_gate_visible(base: [f32; 3], sub: [f32; 3], range: f32) -> bool {
    let r2 = f64::from(range) * f64::from(range);
    fcom_ah(r2, quest_dist2(base, sub)) & 0x41 == 0
}
