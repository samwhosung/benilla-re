//! The vector, box and plane helpers of the resolution layer
//! (`systems/collision/walking.md#vector-box-and-plane-helpers`), the distance along a ray to a
//! plane (`systems/collision/contact.md#contact-distance`) and the nine planes of the volume
//! (`systems/collision/contact.md#volume-planes`).

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x8029d4]`: 2^-22.
const DEN_GUARD: f64 = 2.384_185_8e-7_f32 as f64;

/// A plane: normal and `d`, 16 bytes.
#[derive(Clone, Copy, Default)]
pub struct Plane {
    pub n: [f32; 3],
    pub d: f32,
}

/// A box: minimum and maximum, 24 bytes.
#[derive(Clone, Copy, Default)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// `AddScalarToVector3` (`0x6372d0`, `systems/collision/walking.md#vector-box-and-plane-helpers`):
/// adds s to each component of a vector.
pub fn vec3_add_scalar(v: &mut [f32; 3], s: f32) {
    for c in v.iter_mut() {
        *c = (f(*c) + f(s)) as f32;
    }
}

/// `SubtractScalarFromVector3` (`0x637300`,
/// `systems/collision/walking.md#vector-box-and-plane-helpers`): subtracts s from each component of
/// a vector.
pub fn vec3_sub_scalar(v: &mut [f32; 3], s: f32) {
    for c in v.iter_mut() {
        *c = (f(*c) - f(s)) as f32;
    }
}

/// `NegateVector3` (`0x637330`, `systems/collision/walking.md#vector-box-and-plane-helpers`):
/// writes the negation of a vector.
pub fn vec3_negate(v: &[f32; 3]) -> [f32; 3] {
    [-v[0], -v[1], -v[2]]
}

/// `IsPointInBox` (`0x637350`, `systems/collision/walking.md#vector-box-and-plane-helpers`):
/// returns 1 when a point lies within a box on all three axes, bounds included.
pub fn point_in_aabb(b: &Aabb, p: &[f32; 3]) -> bool {
    for (i, &c) in p.iter().enumerate() {
        if b.min[i] > c {
            return false;
        }
    }
    for (i, &c) in p.iter().enumerate() {
        if c > b.max[i] {
            return false;
        }
    }
    true
}

/// `UnionBoxes` (`0x6373b0`, `systems/collision/walking.md#vector-box-and-plane-helpers`): writes
/// the union of two boxes, taking b on ties and NaN.
pub fn aabb_union(a: &Aabb, b: &Aabb) -> Aabb {
    Aabb {
        min: core::array::from_fn(|i| {
            if a.min[i] > b.min[i] {
                b.min[i]
            } else {
                a.min[i]
            }
        }),
        max: core::array::from_fn(|i| {
            if b.max[i] > a.max[i] {
                b.max[i]
            } else {
                a.max[i]
            }
        }),
    }
}

/// `PlaneFromTriangle` (`0x637480`, `systems/collision/walking.md#vector-box-and-plane-helpers`):
/// writes the normalized plane through three points, normal (p1 - p0) × (p2 - p0).
pub fn plane_from_points(p0: &[f32; 3], p1: &[f32; 3], p2: &[f32; 3]) -> Plane {
    let a: [f64; 3] = core::array::from_fn(|i| f(p2[i]) - f(p0[i]));
    let bx = (f(p1[0]) - f(p0[0])) as f32;
    let by = (f(p1[1]) - f(p0[1])) as f32;
    let bz = f(p1[2]) - f(p0[2]);
    let n = [
        (f(by) * a[2] - bz * a[1]) as f32,
        (bz * a[0] - a[2] * f(bx)) as f32,
        (a[1] * f(bx) - f(by) * a[0]) as f32,
    ];
    let len = ((f(n[0]) * f(n[0]) + f(n[1]) * f(n[1])) + f(n[2]) * f(n[2])).sqrt();
    let inv = 1.0 / len;

    let nf: [f64; 3] = core::array::from_fn(|i| f(n[i]) * inv);
    let ns: [f32; 3] = core::array::from_fn(|i| nf[i] as f32);
    let d = (-((nf[2] * f(p0[2]) + nf[1] * f(p0[1])) + nf[0] * f(p0[0]))) as f32;
    Plane { n: ns, d }
}

/// `PlaneDistanceAlongRay` (`0x6329e0`, `systems/collision/contact.md#contact-distance`): returns
/// (n·p + d)/(n·dir), or n·p + d when abs(n·dir) is below 2^-22.
pub fn ray_plane_t(p: &[f32; 3], plane: &Plane, dir: &[f32; 3]) -> f64 {
    let n = &plane.n;
    let num = ((f(n[2]) * f(p[2]) + f(n[1]) * f(p[1])) + f(n[0]) * f(p[0])) + f(plane.d);
    let den = (f(n[2]) * f(dir[2]) + f(n[1]) * f(dir[1])) + f(n[0]) * f(dir[0]);
    if den.abs() <= DEN_GUARD {
        num
    } else {
        num / den
    }
}

/// `[0x80dfe4]`: 0.8796419, the bevel normal's horizontal part.
const BEVEL_COS: f32 = f32::from_bits(0x3f61_3036);
/// `[0x80dfe0]`: 0.4756366, its vertical part.
const BEVEL_SIN: f32 = f32::from_bits(0x3ef3_86a4);

/// `BuildVolumePlanes` (`0x631440`, `systems/collision/contact.md#volume-planes`): writes nine
/// planes: a box of half-width radius up to height, over a square pyramid with its apex at pos.
pub fn capsule_kdop_planes(center: &[f32; 3], height: f32, radius: f32) -> [Plane; 9] {
    let (cx, cy, cz) = (f(center[0]), f(center[1]), f(center[2]));
    let (r, h) = (f(radius), f(height));
    let (co, si) = (f(BEVEL_COS), f(BEVEL_SIN));
    let zero = 0.0f64;
    [
        Plane {
            n: [-1.0, 0.0, 0.0],
            d: (cx - r) as f32,
        },
        Plane {
            n: [1.0, 0.0, 0.0],
            d: (-cx - r) as f32,
        },
        Plane {
            n: [0.0, 1.0, 0.0],
            d: (-cy - r) as f32,
        },
        Plane {
            n: [0.0, -1.0, 0.0],
            d: (cy - r) as f32,
        },
        Plane {
            n: [0.0, 0.0, 1.0],
            d: (-cz - h) as f32,
        },
        Plane {
            n: [-BEVEL_COS, 0.0, -BEVEL_SIN],
            d: ((cx * co + cz * si) + cy * zero) as f32,
        },
        Plane {
            n: [BEVEL_COS, 0.0, -BEVEL_SIN],
            d: ((cz * si - cx * co) + cy * zero) as f32,
        },
        Plane {
            n: [0.0, BEVEL_COS, -BEVEL_SIN],
            d: ((cx * zero + cz * si) - cy * co) as f32,
        },
        Plane {
            n: [0.0, -BEVEL_COS, -BEVEL_SIN],
            d: ((cx * zero + cz * si) + cy * co) as f32,
        },
    ]
}
