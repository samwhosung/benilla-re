//! The swept world query (`systems/collision/resolve.md#swept-query-frame`): the box grown along
//! the move, the transport frame, and the list of triangles with their bases. The world query
//! itself is passed in.

#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::too_many_arguments)]

use crate::resolve_clip::HitRecord;
use crate::resolve_toolkit::{aabb_union, vec3_add_scalar, vec3_negate, vec3_sub_scalar, Aabb};

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x80dfec]`: 1/720.
const SKIN: f32 = f32::from_bits(0x3ab6_0b61);

/// `[0x80e008]`: tan 50°.
const TAN50: f32 = f32::from_bits(0x3f98_8b62);

/// `[0x80e010]`: −tan 50°.
const NEG_TAN50: f32 = f32::from_bits(0xbf98_8b62);

/// `[0x80e00c]`: √2.
const SQRT2: f32 = f32::from_bits(0x3fb5_04f3);

/// `[0x7ffa24]`: 0.5.
const HALF: f32 = f32::from_bits(0x3f00_0000);

/// `[0x7ff9d8]`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);

/// Flags `0x2000`.
const FALLING: u32 = 0x0000_2000;
/// Flags `0x200000`.
const SWIMMING: u32 = 0x0020_0000;
/// Flags `0x40000000`.
const WATER: u32 = 0x4000_0000;

/// The transport's two matrices: `BuildTransportMatrix` (`0x630ac0`)'s, and its inverse from
/// `InvertAffineMatrix` (`0x7bd820`); columns of three, the last the translation.
#[derive(Clone, Copy, Debug)]
pub struct Transport {
    pub m: [[f32; 3]; 4],
    pub trs: [[f32; 3]; 4],
}

/// `TransformPointInPlace` (`0x7bcc60`, `systems/collision/falling.md#frame-transform`): `R·v + T`
/// per component in the order below, each stored as `f32`.
#[inline]
pub(crate) fn affine_xform(c: &[[f32; 3]; 4], v: &[f32; 3]) -> [f32; 3] {
    let (vx, vy, vz) = (f(v[0]), f(v[1]), f(v[2]));
    [
        (((f(c[2][0]) * vz + f(c[1][0]) * vy) + f(c[0][0]) * vx) + f(c[3][0])) as f32,
        (((f(c[2][1]) * vz + f(c[0][1]) * vx) + f(c[1][1]) * vy) + f(c[3][1])) as f32,
        (((f(c[2][2]) * vz + f(c[0][2]) * vx) + f(c[1][2]) * vy) + f(c[3][2])) as f32,
    ]
}

/// `0x633907` in `CMovement::QuerySweptBox` (`0x633840`): `R·v`, each component `(c2·z + c1·y) +
/// c0·x`, stored as `f32`.
#[inline]
pub(crate) fn rot_dir(c: &[[f32; 3]; 4], v: &[f32; 3]) -> [f32; 3] {
    let (vx, vy, vz) = (f(v[0]), f(v[1]), f(v[2]));
    [
        ((f(c[2][0]) * vz + f(c[1][0]) * vy) + f(c[0][0]) * vx) as f32,
        ((f(c[2][1]) * vz + f(c[1][1]) * vy) + f(c[0][1]) * vx) as f32,
        ((f(c[2][2]) * vz + f(c[1][2]) * vy) + f(c[0][2]) * vx) as f32,
    ]
}

/// `0x633f6e` in `CMovement::QuerySweptBox` (`0x633840`): the normal turned by the inverse frame.
#[inline]
fn rot_normal(c: &[[f32; 3]; 4], n: &[f32; 3]) -> [f32; 3] {
    let (nx, ny, nz) = (f(n[0]), f(n[1]), f(n[2]));
    [
        ((f(c[0][0]) * nx + f(c[2][0]) * nz) + f(c[1][0]) * ny) as f32,
        ((f(c[0][1]) * nx + f(c[2][1]) * nz) + f(c[1][1]) * ny) as f32,
        ((f(c[0][2]) * nx + f(c[2][2]) * nz) + f(c[1][2]) * ny) as f32,
    ]
}

/// `AddVector3InPlace` (`0x4841c0`): `a + b` per component, each stored as `f32`.
#[inline]
fn add3(dst: &mut [f32; 3], src: &[f32; 3]) {
    for i in 0..3 {
        dst[i] = (f(dst[i]) + f(src[i])) as f32;
    }
}

/// `CMovement::QuerySweptBox` (`0x633840`, `systems/collision/resolve.md#swept-query-frame`):
/// gathers the world triangles inside the unit's box grown along a move into 0xc4e52c; returns 0 on
/// failure.
pub fn swept_world_query(
    pos_in: [f32; 3],
    radius: f32,
    height: f32,
    flags: u32,
    guid_zero: bool,
    substep_dist: f32,
    fall_dist: f32,
    coll_height: f32,
    dir: &mut [f32; 3],
    trace_hit: bool,
    transport: Option<&Transport>,
    main_recs: &mut [HitRecord],
    liquid_recs: &mut [HitRecord],
) -> (bool, Aabb) {
    let r = radius;
    let h = height;
    let dist = substep_dist;

    let mut pos = pos_in;
    if !guid_zero {
        if let Some(tp) = transport {
            pos = affine_xform(&tp.m, &pos);

            *dir = rot_dir(&tp.m, dir);
        }
    }

    let mut scr = Aabb {
        min: [(f(pos[0]) - f(r)) as f32, (f(pos[1]) - f(r)) as f32, pos[2]],
        max: [
            (f(pos[0]) + f(r)) as f32,
            (f(pos[1]) + f(r)) as f32,
            (f(pos[2]) + f(h)) as f32,
        ],
    };

    let mut sweep = [
        (f(dir[0]) * f(dist)) as f32,
        (f(dir[1]) * f(dist)) as f32,
        (f(dir[2]) * f(dist)) as f32,
    ];

    let falling = flags & FALLING != 0;
    let swimming = flags & SWIMMING != 0;
    let water = flags & WATER != 0;

    if falling {
        sweep[2] = (f(sweep[2]) - f(fall_dist)) as f32;
    }

    if falling {
        let mut box2 = scr;
        add3(&mut box2.min, &sweep);
        add3(&mut box2.max, &sweep);
        scr = aabb_union(&scr, &box2);
        if water {
            scr.min[2] = (f(scr.min[2]) - f(ONE)) as f32;
        }

        let term = f(sweep[2]) * f(NEG_TAN50);
        if term > 0.0 {
            scr.min[0] = (f(scr.min[0]) - term) as f32;
            scr.max[0] = (f(scr.max[0]) + term) as f32;
            scr.min[1] = (f(scr.min[1]) - term) as f32;
            scr.max[1] = (f(scr.max[1]) + term) as f32;
        }
    } else if swimming {
        let half_dist = (f(dist) * f(HALF)) as f32;

        *dir = [
            (f(dir[0]) * f(half_dist)) as f32,
            (f(dir[1]) * f(half_dist)) as f32,
            (f(dir[2]) * f(half_dist)) as f32,
        ];

        let disp = [
            (f(dir[0]) + f(pos[0])) as f32,
            (f(dir[1]) + f(pos[1])) as f32,
            (f(dir[2]) + f(pos[2])) as f32,
        ];
        let rt2 = (f(r) * f(SQRT2)) as f32;
        let mut box_s = Aabb {
            min: disp,
            max: disp,
        };
        vec3_sub_scalar(&mut box_s.min, half_dist);
        vec3_add_scalar(&mut box_s.max, half_dist);
        box_s.min[0] = (f(box_s.min[0]) - f(rt2)) as f32;
        box_s.max[0] = (f(box_s.max[0]) + f(rt2)) as f32;
        box_s.min[1] = (f(box_s.min[1]) - f(rt2)) as f32;
        box_s.max[1] = (f(box_s.max[1]) + f(rt2)) as f32;

        let sel = if !(f(h) < f(rt2)) { h } else { rt2 };
        box_s.max[2] = (f(sel) + f(box_s.max[2])) as f32;
        scr = aabb_union(&scr, &box_s);
    } else {
        let r_plus = (f(r) + f(SKIN)) as f32;

        let h_tan50 = f(coll_height) * f(TAN50);
        let step_rise = if h_tan50 > f(r_plus) {
            h_tan50
        } else {
            f(r_plus)
        };

        let disp_scale = step_rise + f(dist);
        let disp = [
            (f(dir[0]) * disp_scale) as f32,
            (f(dir[1]) * disp_scale) as f32,
            (f(dir[2]) * disp_scale) as f32,
        ];
        let mut box_w = scr;
        add3(&mut box_w.min, &disp);
        add3(&mut box_w.max, &disp);
        scr = aabb_union(&scr, &box_w);

        let half_dist = (f(dist) * f(HALF)) as f32;
        let dmid = [
            (f(dir[0]) * f(half_dist)) as f32,
            (f(dir[1]) * f(half_dist)) as f32,
            (f(dir[2]) * f(half_dist)) as f32,
        ];

        *dir = [
            (f(dmid[0]) + f(pos[0])) as f32,
            (f(dmid[1]) + f(pos[1])) as f32,
            (f(dmid[2]) + f(pos[2])) as f32,
        ];
        let lat = f(r) * f(SQRT2) + f(half_dist);
        let mut box_l = Aabb {
            min: *dir,
            max: *dir,
        };
        box_l.min[0] = (f(box_l.min[0]) - lat) as f32;
        box_l.max[0] = (f(box_l.max[0]) + lat) as f32;
        box_l.min[1] = (f(box_l.min[1]) - lat) as f32;
        box_l.max[1] = (f(box_l.max[1]) + lat) as f32;
        scr = aabb_union(&scr, &box_l);

        let two_h = f(coll_height) + f(coll_height);
        let zext = if two_h > f(dist) { two_h } else { f(dist) };
        scr.max[2] = (zext + f(scr.max[2])) as f32;

        let down = f(coll_height) + f(dist) * f(TAN50);
        scr.min[2] = (f(scr.min[2]) - down) as f32;
        if water {
            scr.min[2] = (f(scr.min[2]) - f(ONE)) as f32;
        }
    }

    vec3_sub_scalar(&mut scr.min, SKIN);
    vec3_add_scalar(&mut scr.max, SKIN);

    if !trace_hit {
        return (false, scr);
    }

    if swimming && !liquid_recs.is_empty() {
        for rec in liquid_recs.iter_mut() {
            let neg = vec3_negate(&rec.normal);
            *dir = neg;
            rec.normal = neg;
            rec.d = -rec.d;
        }
    }

    if !guid_zero {
        if let Some(tp) = transport {
            for rec in main_recs.iter_mut() {
                let row0 = affine_xform(&tp.trs, &rec.basis[0]);
                let row1 = affine_xform(&tp.trs, &rec.basis[1]);
                let row2 = affine_xform(&tp.trs, &rec.basis[2]);
                rec.basis = [row0, row1, row2];
                let wn = rot_normal(&tp.trs, &rec.normal);
                rec.normal = wn;
                *dir = wn;
                let d = -((f(wn[2]) * f(row0[2]) + f(wn[1]) * f(row0[1])) + f(wn[0]) * f(row0[0]));
                rec.d = d as f32;
            }
        }
    }
    (true, scr)
}

/// `IsVector3Finite` (`0x6337d0`, `systems/collision/resolve.md#swept-query-frame`): returns
/// nonzero when all three components of a vector are finite, through __finite.
pub fn pos_finite_validate(v: &[f32; 3]) -> u32 {
    #[inline]
    fn finite_word(x: f32) -> u32 {
        u32::from(x.to_bits() & 0x7f80_0000 != 0x7f80_0000)
    }
    if finite_word(v[0]) == 0 {
        return 0;
    }
    if finite_word(v[1]) == 0 {
        return 0;
    }
    finite_word(v[2])
}
