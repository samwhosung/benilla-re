//! Small resolution helpers: the far-fall latch, the push back from a triangle, two tests of the
//! volume against a plane or triangle, and the collision class mask.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x80dff8]`: 1/9.
const NINTH: f32 = f32::from_bits(0x3de3_8e39);

/// The mover fields `0x633240` reads and writes.
#[derive(Clone, Copy, Default)]
pub struct LandSnapState {
    pub flags: u32,
    pub cur_z: f32,
    pub fall_accum: u32,
    pub fall_start_z: f32,
    pub fall_vel: f32,
}

/// `CMovement::LatchFallingFar` (`0x633240`, `systems/collision/airborne.md#fall-latches`): sets
/// flag 0x4000 unless 0x20000000 is set: a fall with speed at 1/9 below its start z, else after 500
/// ms.
pub fn ground_land_snap(st: &mut LandSnapState) {
    if st.flags & 0x2000_4000 != 0 {
        return;
    }
    if st.flags & 0x2000 != 0 {
        if f(st.fall_vel) != 0.0 {
            if !((f(st.fall_start_z) - f(NINTH)) <= f(st.cur_z)) {
                st.flags |= 0x4000;
            }
            return;
        }
        if f(st.fall_vel).is_nan() {
            if !((f(st.fall_start_z) - f(NINTH)) <= f(st.cur_z)) {
                st.flags |= 0x4000;
            }
            return;
        }
    }

    if st.fall_accum >= 0x1f4 {
        st.flags |= 0x4000;
    }
}

/// `ComputeTrianglePushback` (`0x634300`, `systems/collision/resolve.md#free-move-slide`): writes
/// -(n.dir)·dist·n for the normal n of a list's triangle.
pub fn reflect_velocity(normal: &[f32; 3], vel: &[f32; 3], scale: f32) -> [f32; 3] {
    let nn = (f(-normal[2]) * f(vel[2]) + f(-normal[1]) * f(vel[1])) + f(-normal[0]) * f(vel[0]);
    let k64 = nn * f(scale);
    let k = k64 as f32;
    [
        (f(k) * f(normal[0])) as f32,
        (f(k) * f(normal[1])) as f32,
        (k64 * f(normal[2])) as f32,
    ]
}

/// `DotVector3` (`0x602630`): `(a.z·b.z + a.y·b.y) + a.x·b.x`, unrounded.
fn dot3(a: &[f32; 3], b: &[f32; 3]) -> f64 {
    (f(a[2]) * f(b[2]) + f(a[1]) * f(b[1])) + f(a[0]) * f(b[0])
}

/// `[0x80dfec]`: 1/720.
const SKIN: f32 = f32::from_bits(0x3ab6_0b61);

/// `CMovement::BoxTopTouchesTriangle` (`0x6333d0`,
/// `systems/collision/airborne.md#triangle-class-tests`): returns 1 when a top corner of the unit's
/// box lies within 1/720 of a listed triangle's plane.
pub fn aabb_top_flush_test(
    pos: &[f32; 3],
    radius: f32,
    height: f32,
    normal: &[f32; 3],
    plane_d: f32,
) -> bool {
    let (x, y, z) = (f(pos[0]), f(pos[1]), f(pos[2]));
    let (r, h) = (f(radius), f(height));
    let corners: [[f32; 3]; 4] = [
        [(x - r) as f32, (y - r) as f32, (z + h) as f32],
        [(x + r) as f32, (y - r) as f32, (z + h) as f32],
        [(x - r) as f32, (y + r) as f32, (z + h) as f32],
        [(x + r) as f32, (y + r) as f32, (z + h) as f32],
    ];
    for c in &corners {
        let sd = (dot3(normal, c) + f(plane_d)).abs();
        if sd < f(SKIN) {
            return true;
        }
    }
    false
}

/// `CMovement::PlaneNearFootPoints` (`0x634960`,
/// `systems/collision/airborne.md#wall-normal-from-contacts`): returns 1 when a plane passes within
/// 1/720 of the position or of four raised corners of the unit.
pub fn prism_lower_flush_test(
    pos: &[f32; 3],
    radius: f32,
    normal: &[f32; 3],
    plane_d: f32,
) -> bool {
    const R_SCALE: f32 = f32::from_bits(0x3fec_b91b);
    let (x, y, z) = (f(pos[0]), f(pos[1]), f(pos[2]));
    let r = f(radius);
    let rp = (r * f(R_SCALE)) as f32;
    let verts: [[f32; 3]; 5] = [
        *pos,
        [(x - r) as f32, (y - r) as f32, (z + f(rp)) as f32],
        [(x + r) as f32, (y - r) as f32, (z + f(rp)) as f32],
        [(x - r) as f32, (y + r) as f32, (z + f(rp)) as f32],
        [(x + r) as f32, (y + r) as f32, (z + f(rp)) as f32],
    ];
    for v in &verts {
        let sd = (dot3(normal, v) + f(plane_d)).abs();
        if sd < f(SKIN) {
            return true;
        }
    }
    false
}

/// `[0x80dfe8]`: −0.6457718.
const PITCH_CUT: f32 = f32::from_bits(0xbf25_514d);

/// What `0x6315f0` reads: the flags, the pitch, what `0x5fa550` returns and two of the unit's
/// flags.
#[derive(Clone, Copy, Default)]
pub struct MaskSelectInput {
    pub flags: u32,
    pub pitch: f32,
    pub pred: bool,
    pub unit_flag1: bool,
    pub unit_flag2: bool,
}

/// `CMovement::GetCollisionMask` (`0x6315f0`, `systems/collision/contact.md#collision-class-mask`):
/// returns the class mask: a base by 0x5fa550, or 0x30000 for a non-swimming water walker above
/// -37°, or 0x8000 for a ghost.
pub fn query_mask_select(inp: &MaskSelectInput) -> u32 {
    let mut m = if inp.pred { 0x0010_0111 } else { 0x0010_2111 };
    if inp.flags & 0x1000_0000 != 0 && inp.flags & 0x0020_0000 == 0 && f(inp.pitch) > f(PITCH_CUT) {
        m |= 0x0003_0000;
    }
    if inp.unit_flag1 && inp.unit_flag2 {
        m |= 0x8000;
    }
    m
}
