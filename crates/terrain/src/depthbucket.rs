//! The depth buckets (`systems/terrain/cull.md#depth-buckets`): `BucketByDepth` (`0x6816f0`),
//! `ComputeDepthBucket` (`0x6817c0`), `CChunkLiquid::BucketCentre` (`0x681970`) and
//! `BucketByDepthTail` (`0x681a40`), beside `CMapChunk::BucketPoint` in `crate::bucketize`.
//!
//! Each key is `fistp(f32(0.03·dist) − 0.5)`: the product stored and popped, then reloaded. The
//! clamp to `[0, 0x20)` and the list insert after it are not modelled.

/// `[0x810174]`: 0.03 (`0x3cf5c28f`).
const BUCKET_FACTOR: f32 = f32::from_bits(0x3cf5_c28f);
/// `[0x86861c]`: 0.5.
const BUCKET_BIAS: f64 = 0.5;

/// A key before the clamp.
pub type BucketKey = i32;

/// `((c1·y + c2·z) + c0·x) + c3` at 53 bits.
#[inline]
fn plane_dot(point: [f32; 3], coef: [f32; 4]) -> f64 {
    let (x, y, z) = (point[0] as f64, point[1] as f64, point[2] as f64);
    let (c0, c1, c2, c3) = (
        coef[0] as f64,
        coef[1] as f64,
        coef[2] as f64,
        coef[3] as f64,
    );
    let s1 = (c1 * y) + (c2 * z);
    let s2 = s1 + (c0 * x);
    s2 + c3
}

/// `fistp(f32(0.03·dist) − 0.5)`.
#[inline]
fn quantize_key(dist: f64) -> BucketKey {
    let m = (BUCKET_FACTOR as f64) * dist;
    let mf = m as f32;
    let v = (mf as f64) - BUCKET_BIAS;
    crate::x87::fistp(v)
}

/// `BucketByDepth` (`0x6816f0`): the key of `dist − radius`, the point and radius from `+0x5c` to
/// `+0x68`, the plane `0xc7cfb8` to `0xc7cfc4`; `fsub [ebp-0xc]` at `0x681738` takes the radius.
pub fn depth_bucket_v1(point: [f32; 3], radius: f32, coef: [f32; 4]) -> BucketKey {
    let dist = plane_dot(point, coef);
    let adj = dist - (radius as f64);
    quantize_key(adj)
}

/// `CChunkLiquid::BucketCentre` (`0x681970`): the key of `dist`, the point a stack argument (`ret
/// 4`), from `0x681977` to the `fistp` at `0x6819b1`.
pub fn depth_bucket_v3(point: [f32; 3], coef: [f32; 4]) -> BucketKey {
    let dist = plane_dot(point, coef);
    quantize_key(dist)
}

/// `BucketByDepthTail` (`0x681a40`): `f32(dot(cam) − radius)` against the plane `0xc7bcb0` to
/// `0xc7bcbc`, stored to `+0x78` at `0x681a6e`, and the key of `dot(coef) − radius` at `0x681aaf`.
pub fn depth_bucket_v4(
    point: [f32; 3],
    radius: f32,
    cam: [f32; 4],
    coef: [f32; 4],
) -> (f32, BucketKey) {
    let cam_dist = plane_dot(point, cam);
    let signed_dist = (cam_dist - (radius as f64)) as f32;
    let dist = plane_dot(point, coef);
    let key = quantize_key(dist - (radius as f64));
    (signed_dist, key)
}

/// `ComputeDepthBucket` (`0x6817c0`): the same two values, stored at `0x6817fa` and `0x68183b`,
/// after an early return when `this` is `[0xc7b748]`.
pub fn view_plane_bucket(
    point: [f32; 3],
    radius: f32,
    cam: [f32; 4],
    coef: [f32; 4],
) -> (f32, BucketKey) {
    let cam_dist = plane_dot(point, cam);
    let signed_dist = (cam_dist - (radius as f64)) as f32;
    let dist = plane_dot(point, coef);
    let key = quantize_key(dist - (radius as f64));
    (signed_dist, key)
}
