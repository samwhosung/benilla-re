//! The video settings scripts' arithmetic (`GetTerrainMip` `0x488fb0`, `GetGamma` `0x4891c0`,
//! `SetGamma` `0x4891f0`, `GetBaseMip` `0x489280`, `SetBaseMip` `0x4892b0`): each stores or reports
//! the complement `1.0 - x` of its value, with the f64 `1.0` at `0x8015b8` as the minuend of an
//! `fsubr`.

/// `1.0 - x` at 53 bits: `fsubr qword [0x8015b8]` on a value widened into `st0`.
pub fn complement(x: f64) -> f64 {
    1.0 - x
}

/// `GetTerrainMip` (`0x488fbd` to `0x488fce`): the setting field `+0x28` of the registry record is
/// an `i32` loaded with `fild`; the complement is pushed to Lua as an f64.
pub fn get_terrain_mip(field_i32: i32) -> f64 {
    complement(f64::from(field_i32))
}

/// `GetGamma` (`0x4891cd` to `0x4891de`): the field `+0x24` is an `f32` loaded with `fld dword`; the
/// complement is pushed as an f64.
pub fn get_gamma(field_f32: f32) -> f64 {
    complement(f64::from(field_f32))
}

/// `GetBaseMip` (`0x48928d` to `0x48929e`): the same bytes as `GetGamma` over the field `+0x24` of a
/// different setting.
pub fn get_base_mip(field_f32: f32) -> f64 {
    complement(f64::from(field_f32))
}

/// `SetGamma` (`0x489230` to `0x489241`): the Lua argument is the f64 that `lua_tonumber` (`0x6f3620`)
/// leaves in `st0`; the complement is stored as an f64 (`fstp qword [ebp-8]`) for the `%f` config
/// writer.
pub fn set_gamma(lua_arg: f64) -> f64 {
    complement(lua_arg)
}

/// `SetBaseMip` (`0x4892f0` to `0x489300`): the complement of the live f64 Lua argument, truncated
/// toward zero to an `i64` by `__ftol` (`0x40a2b0`) for the `%I64d` config writer. `__ftol` returns
/// `0x8000000000000000` for a NaN or a value outside the `i64` range.
pub fn set_base_mip(lua_arg: f64) -> i64 {
    benilla_re_cmath::x87::ftol64(complement(lua_arg))
}
