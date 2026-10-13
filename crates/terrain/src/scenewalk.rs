//! The arithmetic of `CullGroupDoodads` (`0x6834e0`), `CullEntityList` (`0x683700`),
//! `CullLowDetailTiles` (`0x683040`, `systems/terrain/lod.md#low-detail-pass`) and the fade ageing
//! of `BeginMapObjectFrame` (`0x6b3890`). The sort key and the audibility test of the first two are
//! `crate::scenevis`'s.

/// `CullGroupDoodads` (`0x6834e0`) from `0x683681`: the centre `[ebx]` less the camera `0xc7cf20`,
/// each stored as an argument to `0x4549a0`; `0x4549f0` squares and sums them.
pub fn near_audibility_diffs(v: [f32; 3], cam: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|k| (f64::from(v[k]) - f64::from(cam[k])) as f32)
}

/// `CullLowDetailTiles` (`0x683040`) at `0x6830c7`: `[ebp-0x4] − [0x8101b0]`, stored back.
pub fn arealow_far_project(field_3c: f32, far_const: f32) -> f32 {
    (f64::from(field_3c) - f64::from(far_const)) as f32
}

/// `0x683040` from `0x6831fc`: the camera `0xc7cf20` negated (`fchs`), stored to `[ebp-0x2c]` up.
pub fn camera_negate(cam: [f32; 3]) -> [f32; 3] {
    cam.map(|c| (-f64::from(c)) as f32)
}

/// `BeginMapObjectFrame` (`0x6b3890`) from `0x6b398a` and the same at the second list: `[+0x8] −
/// dt` (`[0xc62510]`) stored, then the stored value reloaded against `[0x7ffd74]` (0.0), `test ah,5;
/// jp`: it expires only below the threshold; equal, above and NaN do not. The stored value and
/// whether it expired.
pub fn portal_age_fstp(timer: f32, dt: f32, threshold: f32) -> (f32, bool) {
    let stored = (f64::from(timer) - f64::from(dt)) as f32;
    (stored, stored < threshold)
}

/// `0x6b3890` from `0x6b39eb` (`+0x15c`) and from `0x6b3a72` (`+0x1cc`): `timer − dt` stored with
/// `fst` and the unrounded value compared, `test ah,0x41; jp`: it expires at or below the threshold;
/// above and NaN do not.
pub fn portal_age_fst_live(timer: f32, dt: f32, threshold: f32) -> (f32, bool) {
    let live = f64::from(timer) - f64::from(dt);
    (live as f32, live <= f64::from(threshold))
}
