//! Facing arithmetic in the object layer: the 2π wrap of `CGUnit_C::RotateFacings` (`0x601050`,
//! `systems/objectlayer/unitmovement.md#facing`) and `CGUnit_C::OnAutoMoveTransportChange`
//! (`0x6115e0`, `systems/objectlayer/unitmovement.md#steering-an-auto-move`), the facing negation
//! in `0x615070`, and the height of a box in `0x615150`.

/// `[0x7ffd74]`: 0.0, the wrap's lower bound.
pub const ZERO: f32 = 0.0;

/// `[0x80295c]`: 2π (`0x40c90fdb`), the wrap's period.
pub const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);

/// `0x601050`, once for `+0xc98` and once for `+0xc94`: `delta + acc` (`fld delta; fadd acc`),
/// plus 2π below 0 (`test ah,5; jp`), minus 2π at 2π or above (`test ah,1; jne`), stored as `f32`.
/// A NaN sum is neither and passes through.
pub fn wrap_angle(acc: f32, delta: f32) -> f32 {
    let sum = f64::from(delta) + f64::from(acc);
    let wrapped = if sum < f64::from(ZERO) {
        sum + f64::from(TWO_PI)
    } else if sum >= f64::from(TWO_PI) {
        sum - f64::from(TWO_PI)
    } else {
        sum
    };
    wrapped as f32
}

/// `0x611619`, modes 2 and 8: the same wrap on the global `[0xc4d9a8]`, summed `fld acc; fadd
/// delta`.
pub fn broadcast_wrap_angle(acc: f32, delta: f32) -> f32 {
    let sum = f64::from(acc) + f64::from(delta);
    let wrapped = if sum < f64::from(ZERO) {
        sum + f64::from(TWO_PI)
    } else if sum >= f64::from(TWO_PI) {
        sum - f64::from(TWO_PI)
    } else {
        sum
    };
    wrapped as f32
}

/// `0x6150cc`: `fchs` on the heading `HeadingBetweenPoints` (`0x47f220`) returns, stored as `f32`
/// for `RotateMatrix` (`0x7bdd60`).
pub fn negate_facing(facing: f32) -> f32 {
    (-f64::from(facing)) as f32
}

/// `0x615185`: the box's `max.z − min.z` (`+0x14` and `+8` of the box at `+0xc0`), stored at
/// `+0xbc` as `f32`.
pub fn bounds_z_extent(max_z: f32, min_z: f32) -> f32 {
    (f64::from(max_z) - f64::from(min_z)) as f32
}
