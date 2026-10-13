//! The start of `CMovement::SweepCarrierMove` (`0x6343b0`,
//! `systems/collision/resolve.md#carrier-push`), from `0x6343b0` to `0x634481`: the carried point,
//! its box and the push.

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// What the stretch stores: the box at `0xc4e5a0`, minimum then maximum, and the push at
/// `[ebp-0x24]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StartFalling {
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    pub swept_disp: [f32; 3],
}

/// `TransformPoint` (`0x7bca80`, `systems/collision/falling.md#frame-transform`), which the stretch
/// calls: `M·v + T`, each component stored as `f32`.
#[inline]
pub fn affine_xform(m: &[f32; 16], v: &[f32; 3]) -> [f32; 3] {
    benilla_re_cmath::transform::transform_point(v, m)
}

/// `p = M·base + T`; the box from `p − (r, r, 0)` to `p + (r, r, h)` with `r` = `+0xb0` and `h` =
/// `+0xb4`, `p.z` stored as it is; and the push `scale·dir`, each component stored as `f32`.
pub fn start_falling(
    base: &[f32; 3],
    mat: &[f32; 16],
    ext_xy: f32,
    ext_z: f32,
    scale: f32,
    dir: &[f32; 3],
) -> StartFalling {
    let b = affine_xform(mat, base);
    let (bx, by, bz) = (f(b[0]), f(b[1]), f(b[2]));
    let exy = f(ext_xy);
    let ez = f(ext_z);

    let aabb_min = [(bx - exy) as f32, (by - exy) as f32, b[2]];
    let aabb_max = [(bx + exy) as f32, (by + exy) as f32, (bz + ez) as f32];

    let s = f(scale);
    let swept_disp = [
        (s * f(dir[0])) as f32,
        (s * f(dir[1])) as f32,
        (s * f(dir[2])) as f32,
    ];

    StartFalling {
        aabb_min,
        aabb_max,
        swept_disp,
    }
}
