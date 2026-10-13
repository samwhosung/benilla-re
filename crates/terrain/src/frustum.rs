//! The plane tests of a convex volume: `GetPlaneOutcode` (`0x686c20`), `TestSpherePlanes`
//! (`0x686b80`), `TestBoxPlanes` (`0x686940`, `systems/terrain/cull.md#frustum-box-test`) and
//! `TranslateConvexVolume` (`0x686820`).
//!
//! A volume holds six planes `[n, d]` 16 bytes apart at `+0x0`, eight corners 12 bytes apart at
//! `+0x60` and two points at `+0xc0` and `+0xcc`. The distances are one 53-bit expression each,
//! compared without a store.

/// `[0x8101b4]`: −0.019444443 (`0xbc9f49f4`), how far behind a plane a point may be.
const EPS_NEG: f32 = f32::from_bits(0xbc9f_49f4);

/// A plane, `[n.x, n.y, n.z, d]`.
pub type Plane = [f32; 4];

/// `((n.x·p.x + n.z·p.z) + n.y·p.y) + d` at 53 bits.
#[inline]
fn dot4(plane: &Plane, p: &[f32; 3]) -> f64 {
    f64::from(plane[0]) * f64::from(p[0])
        + f64::from(plane[2]) * f64::from(p[2])
        + f64::from(plane[1]) * f64::from(p[1])
        + f64::from(plane[3])
}

/// `GetPlaneOutcode` (`0x686c20`): bit `i` set when the point is behind plane `i`, distance below
/// −0.019444443 (`0x686c57`); NaN sets no bit.
pub fn outcode6(planes: &[Plane; 6], p: &[f32; 3]) -> u32 {
    let mut code = 0u32;
    for (i, plane) in planes.iter().enumerate() {
        if dot4(plane, p) < f64::from(EPS_NEG) {
            code |= 1 << (i as u32 & 0x1f);
        }
    }
    code
}

/// `TestSpherePlanes` (`0x686b80`): 0 when the centre is further behind a plane than the radius
/// `sphere[3]` (`fchs` once, then `fcomp [ebp+8]; jnp` at `0x686bab`), else 3; NaN passes.
pub fn sphere_frustum(planes: &[Plane; 6], sphere: &[f32; 4]) -> u32 {
    let thr = f64::from(-sphere[3]);
    let c = [sphere[0], sphere[1], sphere[2]];
    for plane in planes.iter() {
        if dot4(plane, &c) < thr {
            return 0;
        }
    }
    3
}

/// `TestBoxPlanes` (`0x686940`): 0 when the box's corner farthest along a plane's normal is behind
/// it, else 3; the corner per axis is the maximum where the component's sign bit is clear and the
/// minimum where it is set (`shr eax,0x1f` at `0x68695a`), so −0.0 takes the minimum. The box is
/// its minimum then its maximum.
pub fn aabb_frustum_pvertex(planes: &[Plane; 6], aabb_max: &[f32; 3], aabb_min: &[f32; 3]) -> u32 {
    let pick = |comp: f32, axis: usize| -> f32 {
        if (comp.to_bits() >> 31) == 1 {
            aabb_min[axis]
        } else {
            aabb_max[axis]
        }
    };
    for plane in planes.iter() {
        let v_far = [pick(plane[0], 0), pick(plane[1], 1), pick(plane[2], 2)];
        if dot4(plane, &v_far) < f64::from(EPS_NEG) {
            return 0;
        }
    }
    3
}

/// A convex volume: the planes, the corners and the two points.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryVolume {
    pub planes: [Plane; 6],
    pub corners: [[f32; 3]; 8],
    pub anchor_a: [f32; 3],
    pub anchor_b: [f32; 3],
}

/// `TranslateConvexVolume` (`0x686820`): the corners moved by `delta` (`0x686830`), each plane's
/// `d` less `(n.z·dz + n.x·dx) + n.y·dy` (`0x686858`), then the two points moved, each value stored
/// as `f32`.
#[allow(clippy::needless_range_loop)] // the two points walked together, as the client does
pub fn volume_translate(v: &mut QueryVolume, delta: &[f32; 3]) {
    for c in v.corners.iter_mut() {
        c[0] = (f64::from(c[0]) + f64::from(delta[0])) as f32;
        c[1] = (f64::from(delta[1]) + f64::from(c[1])) as f32;
        c[2] = (f64::from(delta[2]) + f64::from(c[2])) as f32;
    }
    for plane in v.planes.iter_mut() {
        let inner = f64::from(plane[2]) * f64::from(delta[2])
            + f64::from(plane[0]) * f64::from(delta[0])
            + f64::from(plane[1]) * f64::from(delta[1]);
        plane[3] = (f64::from(plane[3]) - inner) as f32;
    }
    for i in 0..3 {
        v.anchor_a[i] = (f64::from(v.anchor_a[i]) + f64::from(delta[i])) as f32;
        v.anchor_b[i] = (f64::from(v.anchor_b[i]) + f64::from(delta[i])) as f32;
    }
}
