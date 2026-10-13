//! The frame's camera and scene arithmetic (`systems/terrain/cull.md#scene-frame`):
//! `CaptureSceneCamera` (`0x680bc0`), `DrawWorldScene` (`0x681070`) and the texture scroll of
//! `CWorld::BeginFrame` (`0x66fd50`).

/// `[0x7ff9d8]`: 1.0.
pub const SCENE_ONE: f32 = 1.0;
/// `[0x80c4f4]`: 0.0001 (`0x38d1b717`), the least squared length the second normalize scales.
pub const SCENE_EPS2: f32 = f32::from_bits(0x38d1_b717);
/// `[0x7ffd74]`: 0.0.
pub const SCENE_ZERO: f32 = 0.0;
/// `[0x7ffa24]`: 0.5.
pub const FOG_HALF: f32 = 0.5;
/// `[0x80fee4]`: 33.333332 (`0x42055555`), a chunk's width.
pub const FOG_NEAR_BIAS: f32 = f32::from_bits(0x4205_5555);
/// `[0x808de4]`: 64.0, where a scroll offset wraps.
pub const SCROLL_WRAP: f32 = 64.0;

/// `CaptureSceneCamera` (`0x680bc0`) from `0x680c01`: the target `[edx]` less the camera `[esi]`,
/// each stored.
pub fn scene_dir_subtract(target: [f32; 3], cam: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| (f64::from(target[i]) - f64::from(cam[i])) as f32)
}

/// `0x680bc0` from `0x680c27` to `0x680c98`: `1/√((x² + y²) + z²)` kept in the register and each
/// component times it, stored to `0xc7cfec` up.
pub fn scene_dir_normalize(d: [f32; 3]) -> [f32; 3] {
    let [dx, dy, dz] = d.map(f64::from);
    let inv = f64::from(SCENE_ONE) / ((dx * dx + dy * dy) + dz * dz).sqrt();
    [(dx * inv) as f32, (dy * inv) as f32, (dz * inv) as f32]
}

/// `0x680bc0` at `0x680cb6` and `0x680d3d`: the unrounded `DotVector3` (`0x602630`) negated and
/// stored, to `[0xc7bcbc]` and `[0xc7cfc4]`.
pub fn scene_dot_negate(dot_live: f64) -> f32 {
    (-dot_live) as f32
}

/// `0x680bc0` from `0x680cd7` to `0x680ce5`: `y² + x²` of the normalized direction, unrounded; it is
/// stored to `[ebp-0x1c]` with `fst`.
pub fn scene_plane_len2(c: f32, d: f32) -> f64 {
    f64::from(d) * f64::from(d) + f64::from(c) * f64::from(c)
}

/// `0x680bc0` at `0x680ce8`: `fcomp [0x80c4f4]; test ah,0x41; jne` scales only when the unrounded
/// squared length is above 0.0001.
pub fn scene_plane_normalizes(len2: f64) -> bool {
    len2 > f64::from(SCENE_EPS2)
}

/// `0x680bc0` from `0x680cbe` to `0x680d16`: `(x, y, 0)` of the normalized direction; when
/// `scene_plane_normalizes`, x and y times `1/√` of the stored squared length (kept in the
/// register), and the third `that·0.0`.
pub fn scene_plane_normalize(c: f32, d: f32) -> [f32; 3] {
    let len2 = scene_plane_len2(c, d);
    if !scene_plane_normalizes(len2) {
        return [c, d, 0.0];
    }
    let inv = f64::from(SCENE_ONE) / f64::from(len2 as f32).sqrt();
    [
        (f64::from(c) * inv) as f32,
        (f64::from(d) * inv) as f32,
        (inv * f64::from(SCENE_ZERO)) as f32,
    ]
}

/// `0x680bc0` at `0x680d82`, `0x680d94` and `0x680da0`, and at `0x680ee9`, `0x680ef8` and
/// `0x680f06`: a camera component `0xc7cf20` up negated and stored.
pub fn scene_corner_negate(g: f32) -> f32 {
    (-f64::from(g)) as f32
}

/// `DrawWorldScene` (`0x681070`) at `0x6810d8`: `[0xc7b498] − 33.333332` to `[0xc7d278]`.
pub fn render_near_bias(near: f32) -> f32 {
    (f64::from(near) - f64::from(FOG_NEAR_BIAS)) as f32
}

/// `0x681070` from `0x6810f5` to `0x681114`: `fog/cos(fov·0.5) − fog` to `[0xc7d300]`, the field of
/// view from the call before it and the fog `[0xc62964]`; `fcos` is the host's cosine.
pub fn render_fog_dist(fov: f32, fog: f32) -> f32 {
    let cos = (f64::from(fov) * f64::from(FOG_HALF)).cos();
    let fogd = f64::from(fog);
    (fogd / cos - fogd) as f32
}

/// `0x681070` at `0x6811a0`: `[esi+0x10] + 33.333332` to `[0xc7b66c]`.
pub fn render_leg_fog_offset(leg_z: f32) -> f32 {
    (f64::from(leg_z) + f64::from(FOG_NEAR_BIAS)) as f32
}

/// `0x681070` from `0x681310` to `0x681355`: a camera component plus a local, stored back.
pub fn render_pos_accumulate(corner: f32, acc: f32) -> f32 {
    (f64::from(corner) + f64::from(acc)) as f32
}

/// `CWorld::BeginFrame` (`0x66fd50`) from `0x66fe10`: `dt·rate + offset`, `dt` from `[0xc62510]`.
pub fn scroll_accumulate(dt: f32, rate: f32, offset: f32) -> f32 {
    (f64::from(dt) * f64::from(rate) + f64::from(offset)) as f32
}

/// `0x66fd50` from `0x66fe10` to `0x66fe57`, one channel: both offsets accumulated and stored, then
/// each set to 0 at or above 64.0 (`test ah,1; jne`, NaN kept). The u test compares the unrounded
/// sum still in the register, the v test the stored one.
pub fn scroll_update(dt: f32, rate: [f32; 2], offset: [f32; 2]) -> [f32; 2] {
    let u_live = f64::from(dt) * f64::from(rate[0]) + f64::from(offset[0]);
    let v = scroll_accumulate(dt, rate[1], offset[1]);
    let wrap = SCROLL_WRAP;
    [
        if u_live >= f64::from(wrap) {
            0.0
        } else {
            u_live as f32
        },
        if v >= wrap { 0.0 } else { v },
    ]
}
