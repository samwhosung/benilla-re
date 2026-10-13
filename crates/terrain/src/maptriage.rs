//! Small pieces of the frame update: the negated size of `AddRipple` (`0x68f9f0`), the fade gate
//! and count of `CMap::Update` (`0x698390`, `systems/terrain/files.md#map-load`), the buffer size of
//! `CChunkLiquid::Draw` (`0x68de40`, `systems/terrain/liquid.md#liquid-render-passes`) and the
//! negated camera of `DrawQueryCells` (`0x6bd2a0`).

use crate::x87::ftol;

/// `AddRipple` (`0x68f9f0`) at `0x68fa3d`: `fld [ebp+8]; fchs; fstp [ebp+8]`.
pub fn ripple_negate(param1: f32) -> f32 {
    (-f64::from(param1)) as f32
}

/// `CMap::Update` (`0x698390`) from `0x69841f` to the `jne` at `0x698436`: `[0xc7b2a0] − [0xca04f4]`
/// against 2.0 (`[0x801628]`), `test ah,0x41; jne`; it goes on only when the difference is above 2.0
/// (equal, below and NaN skip).
pub fn fade_time_gate(cur_time: f32, last_time: f32) -> bool {
    f64::from(cur_time) - f64::from(last_time) > 2.0
}

/// `0x698390` from `0x69844d` to `0x69845d`: `__ftol` of the 64-bit `fild` of `value` times 0.125
/// (`[0x81061c]`).
pub fn fade_count_index(value: i64) -> i32 {
    ftol(value as f64 * f64::from(f32::from_bits(0x3e00_0000)))
}

/// `CChunkLiquid::Draw` (`0x68de40`) at `0x68dea6` and `0x68def5`, the same bytes: `__ftol` of 0.125
/// (`[0x81023c]`) times 64.0 (`[0x808de4]`), stored to `[ebp-0x4]` for `0x58acd0`.
pub fn liquid_lazy_init_scalar() -> i32 {
    let a = f64::from(f32::from_bits(0x3e00_0000));
    let b = f64::from(f32::from_bits(0x4280_0000));
    ftol(a * b)
}

/// `DrawQueryCells` (`0x6bd2a0`) from `0x6bd2c4`: the camera at `0xc7cf20` negated (`fchs`), stored
/// to `[ebp-0xc]` up.
pub fn dda_debug_negate3(g: [f32; 3]) -> [f32; 3] {
    g.map(|c| (-f64::from(c)) as f32)
}
