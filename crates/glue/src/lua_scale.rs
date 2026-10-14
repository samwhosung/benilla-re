//! The conversions the script functions apply to a value they push to Lua or take from it
//! (`systems/glue/select.md#facing-in-degrees`, `systems/glue/create.md#facing-in-degrees`,
//! `systems/glue/script.md#cursor-and-screen-size`).

/// 180/π, the float `0x42652ee1` at `0x803a78`.
pub const RAD2DEG: f32 = f32::from_bits(0x4265_2ee1);
/// π/180, the float `0x3c8efa35` at `0x7ffaac`.
pub const DEG2RAD: f32 = f32::from_bits(0x3c8e_fa35);
/// 1024.0, the float at `0x7ffd68`.
pub const MODEL_SCALE: f32 = f32::from_bits(0x4480_0000);

/// The facing getters `GetCharacterCreateFacing` (`0x471910`) and `GetCharacterSelectFacing`
/// (`0x473650`): the stored radians times 57.29578, kept as the `f64` that `fstp qword` hands
/// `lua_pushnumber`. The product of two floats is exact at 53 bits.
pub fn lua_rad2deg(facing_radians: f32) -> f64 {
    f64::from(facing_radians) * f64::from(RAD2DEG)
}

/// The facing setters `SetCharacterCreateFacing` (`0x471930`) and `SetCharacterSelectFacing`
/// (`0x473670`): the degrees from `lua_tonumber`, a `double`, times 0.017453292 at 53 bits, stored
/// as an `f32` (`fstp dword`).
pub fn lua_deg2rad(degrees: f64) -> f32 {
    (degrees * f64::from(DEG2RAD)) as f32
}

/// The value the cursor and size getters (`0x46dad0`, `0x46dd30`, `0x46dd80`) pass to `0x41ae40`:
/// `(scale · 1024.0) · coord` at 53 bits rounded to `f32`, with `scale` the screen scale `A` that
/// `0x41ad70` returns and `coord` a scaled cursor fraction or `K1 · 1` or `K2 · 1`. `0x41ae40` then
/// divides by `K1` (`0x832a44`), which is not here.
pub fn lua_model_scale(scale: f32, coord: f32) -> f32 {
    ((f64::from(scale) * f64::from(MODEL_SCALE)) * f64::from(coord)) as f32
}
