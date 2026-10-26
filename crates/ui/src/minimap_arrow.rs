//! The minimap's player arrow placement (`0x4ee100`, `systems/ui/minimap.md`): the position
//! `(w·0.5, h·0.5, 0)` from the arrow's width and height slots, each product rounded once to a
//! float by `fstp dword`.

/// The three floats `0x4ee100` writes at the arrow's `+0x390`, from the values its slots 7
/// (width) and 8 (height) leave in `st0`.
pub fn arrow_position(width: f64, height: f64) -> [f32; 3] {
    [(width * 0.5) as f32, (height * 0.5) as f32, 0.0]
}
