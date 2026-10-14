//! The screen size `CGlueMgr::Resume` stores.

/// A screen dimension of `CGlueMgr::Resume` (`0x46a7b0`, `0x46a7f7` to `0x46a817`): `__ftol(hi - lo)`
/// of the screen rectangle's edges, stored at `0xb41e1c` (width) and `0xb41e20` (height)
/// (`systems/glue/manager.md#screen-size-and-aspect`). The difference is at 53 bits; `__ftol`
/// truncates toward zero to 64 bits and returns the low 32, and gives 0 for NaN and values past
/// 64 bits.
pub fn viewport_extent(hi: f32, lo: f32) -> i32 {
    let t = (f64::from(hi) - f64::from(lo)).trunc();
    let i = if (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&t) {
        t as i64
    } else {
        i64::MIN
    };
    i as i32
}
