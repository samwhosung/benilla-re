//! The 4-vector constructor `0x5c6540`.

/// `0x5c6540` (`ecx` = the vector, `x`, `y`, `z` and `w` on the stack; `ret 0x10`): the four words
/// stored, with no arithmetic.
#[must_use]
pub fn c4_set(x: f32, y: f32, z: f32, w: f32) -> [f32; 4] {
    [x, y, z, w]
}
