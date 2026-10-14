//! The transport's path point and its speed easing: `0x5f6280` and `0x5f8dc0`
//! (`systems/objectlayer/transports.md`, `#the-transports-path` and `#leg-timing`).
//!
//! `gameobject_path_eval` (`0x5f6280`) places the transport on its looping multi-leg path at a
//! time: it brackets the time among the legs, builds the 3x3 rotation matrix from the packed
//! rotation quaternion, interpolates the two bracketing nodes and rotates the offset by the matrix.
//! The ring-index step `0x5f6260` and the nine-float store `0x5f8d20` are inlined.
//! `transport_speed_easing` (`0x5f8dc0`) is the trapezoid speed curve over a leg's acceleration
//! (`seg+0x18`) and deceleration (`seg+0x20`), which also writes the leg's animation tag.
//!
//! The x87 runs at 53 bits, so an operation is an `f64` operation and each `fstp dword` or `fst
//! dword` rounds to `f32` where the code stores. A compare reads the live register.

use crate::x87::not_above;

/// One path leg as `0x5f6280` reads it, at stride `0x1c`. `time` (`+0x08`) is the leg's cumulative
/// time, the unsigned key of the loop, and `segments[count - 1].time` is the path period. `pos`
/// (`+0x0c` to `+0x14`) is the leg's node position and `anim_id` (`+0x18`) its sound id, which only
/// the sound path at `this+0x10` reads, so this transcription leaves it out.
#[derive(Clone, Copy, Debug)]
pub struct PathSegment {
    pub time: u32,
    pub pos: [f32; 3],
    pub anim_id: i32,
}

/// `0x5f6260`: the next ring index, `cursor + 1` or 0 at `count` (`inc; cmp; sete; dec; and`).
#[inline]
fn ring_next(cursor: u32, count: u32) -> u32 {
    let n = cursor.wrapping_add(1);
    if n == count {
        0
    } else {
        n
    }
}

/// `0x5f6280(this, out, progress)`. It reads the transport's game object (`this+0x04`), the
/// rotation quaternion in its fields (`+0x10` to `+0x1c` of `GO+0x110`), the time base (`GO+0x278`)
/// and the legs (`this+0x20`, count `this+0x24`, ring cursor `this+0x28`), advances the cursor in
/// place and writes the rotated, interpolated offset.
///
/// A sound flag at `this+0x10` makes a leg change call `0x6709f0`; callers leave it clear here.
pub fn gameobject_path_eval(
    quat: [f32; 4],
    time_base: i32,
    progress: i32,
    segments: &[PathSegment],
    cursor: &mut u32,
) -> [f32; 3] {
    let count = segments.len() as u32;
    // The period is `segments[count - 1].time`, the field at `legs + 0x1c * count - 0x14`.
    let period = segments[(count - 1) as usize].time;
    // `(time_base + progress) % period`, unsigned (`add; xor edx,edx; div`).
    let target = (time_base as u32).wrapping_add(progress as u32) % period;

    // Advance the cursor around the ring until `segments[cursor].time <= target < segments[next].time`.
    let mut next = ring_next(*cursor, count);
    loop {
        let sc = &segments[*cursor as usize];
        let sn = &segments[next as usize];
        if sc.time <= target && target < sn.time {
            break;
        }
        *cursor = next;
        next = ring_next(*cursor, count);
    }
    let sc = segments[*cursor as usize];
    let sn = segments[next as usize];

    // `frac = (target - sc.time) / (sn.time - sc.time)`, stored `fstp dword [ebp-4]`. The numerator
    // is `fild qword` of the zero-extended dword; the denominator is `fidiv dword`, which reads it
    // signed.
    let num = target.wrapping_sub(sc.time);
    let den = sn.time.wrapping_sub(sc.time) as i32;
    let frac = (f64::from(num) / f64::from(den)) as f32;

    // The rotation quaternion (x, y, z, w) as a 3x3 matrix, row-major in `m[0..9]`. `2qz`, the six
    // products with `qw` and `qx` and `2qz * qz` are stored as `f32`; `2qx`, `2qy`, `2qy * qy` and
    // `2qz * qy` stay in registers.
    let (qx, qy, qz, qw) = (quat[0], quat[1], quat[2], quat[3]);
    let two_x = f64::from(qx) + f64::from(qx); // register f64
    let two_y = f64::from(qy) + f64::from(qy); // register f64
    let two_z = (f64::from(qz) + f64::from(qz)) as f32; // fstp -> f32 (reloaded below)
    let xw = (two_x * f64::from(qw)) as f32;
    let yw = (two_y * f64::from(qw)) as f32;
    let zw = (f64::from(two_z) * f64::from(qw)) as f32;
    let xx = (two_x * f64::from(qx)) as f32;
    let yx = (two_y * f64::from(qx)) as f32;
    let zx = (f64::from(two_z) * f64::from(qx)) as f32;
    let yy = two_y * f64::from(qy); // register f64
    let zy = f64::from(two_z) * f64::from(qy); // register f64
    let zz = (f64::from(two_z) * f64::from(qz)) as f32;

    let m: [f32; 9] = [
        (1.0 - (f64::from(zz) + yy)) as f32,    // m00 = 1 - (2zz + 2yy)
        (f64::from(yx) + f64::from(zw)) as f32, // m01 = 2xy + 2zw
        (f64::from(zx) - f64::from(yw)) as f32, // m02 = 2xz - 2yw
        (f64::from(yx) - f64::from(zw)) as f32, // m10 = 2xy - 2zw
        (1.0 - (f64::from(zz) + f64::from(xx))) as f32, // m11 = 1 - (2zz + 2xx)
        (zy + f64::from(xw)) as f32,            // m12 = 2yz + 2xw
        (f64::from(zx) + f64::from(yw)) as f32, // m20 = 2xz + 2yw
        (zy - f64::from(xw)) as f32,            // m21 = 2yz - 2xw
        (1.0 - (yy + f64::from(xx))) as f32,    // m22 = 1 - (2yy + 2xx)
    ];

    // The interpolation of the two node positions by `frac`, with the client's stores:
    //   x: both products stay registers and the sum is stored (`fstp [ebp-0x30]`);
    //   y: only `(1 - frac) * cursor` is stored (`fstp [ebp-0x20]`), and the sum stays a register;
    //   z: both products are stored, and the sum stays a register.
    let one_minus = 1.0_f64 - f64::from(frac);
    let d0 = (one_minus * f64::from(sc.pos[0]) + f64::from(frac) * f64::from(sn.pos[0])) as f32;
    let omcy = (one_minus * f64::from(sc.pos[1])) as f32;
    let d1 = f64::from(omcy) + f64::from(frac) * f64::from(sn.pos[1]);
    let omcz = (one_minus * f64::from(sc.pos[2])) as f32;
    let fnz = (f64::from(frac) * f64::from(sn.pos[2])) as f32;
    let d2 = f64::from(omcz) + f64::from(fnz);

    // `out[r] = m[r] * d0 + (m[r+3] * d1 + m[r+6] * d2)`: the pair in parentheses is summed first
    // (`faddp`), then `m[r] * d0`. `d0` reloads as `f32`, `d1` and `d2` are registers, and each
    // `out` is stored as `f32`.
    [
        (f64::from(m[0]) * f64::from(d0) + (f64::from(m[3]) * d1 + f64::from(m[6]) * d2)) as f32,
        (f64::from(m[1]) * f64::from(d0) + (f64::from(m[4]) * d1 + f64::from(m[7]) * d2)) as f32,
        (f64::from(m[2]) * f64::from(d0) + (f64::from(m[5]) * d1 + f64::from(m[8]) * d2)) as f32,
    ]
}

/// `0x5f8dc0(seg, dist, total, out_tag, has_prev, has_next)` returns the `float10` in `st0` and
/// writes the leg animation tag (0xa2, 0xa3 or 0xa4). It reads `accel` at `seg+0x18`, `decel` at
/// `seg+0x20` and `0.5` at `0x7ffa24`.
///
/// The four `(has_prev, has_next)` cases are the four branch groups. Only the case with both stores
/// to `f32` mid-expression: `r = accel / decel` (`fstp [ebp+0x18]`) and `lo = total - fvar4` (`fstp
/// [ebp+0x14]`). Every compare is `fcom`, `fnstsw`, `test ah` with `jp`, `jnp` or `jne`, so an
/// unordered operand (NaN) takes the branch an equal one does where the test is `0x41` and the
/// branch a greater one does where it is `5` with `jp`; each is noted below.
pub fn transport_speed_easing(
    accel: f32,
    decel: f32,
    dist: f32,
    total: f32,
    has_prev: bool,
    has_next: bool,
) -> (f64, u32) {
    let a = f64::from(accel);
    let d = f64::from(decel);
    let p2 = f64::from(dist);
    let p3 = f64::from(total);
    const HALF: f64 = 0.5; // 0x7ffa24

    // `fcomp x; test ah,0x41; jne` jumps for less, equal and unordered, so the fall-through is
    // `greater` and a NaN takes the jump. `fcom x; test ah,5; jp` jumps for greater, equal and
    // unordered, so the fall-through is `less`.
    if !has_prev {
        if !has_next {
            // 0x5f8f7c: accel ramp only.
            return (p2 * a, 0xa3);
        }
        // 0x5f8f0a: the knee `f = accel / decel`, cut to `total` when `total < f`.
        let mut f = a / d;
        if p3 < f {
            f = p3;
        }
        let fvar7 = p3 - f;
        if not_above(p2, fvar7) {
            return (p2 * a, 0xa3); // 0x5f8f63
        }
        // 0x5f8f34: `fvar7 * accel + (2 * (f * decel) - (p2 - fvar7) * decel) * 0.5 * (p2 - fvar7)`
        let g = f * d;
        let e = p2 - fvar7;
        let two_g = g + g;
        return (fvar7 * a + (two_g - e * d) * HALF * e, 0xa4);
    }

    if !has_next {
        // 0x5f8ea3: the knee `f`, cut to `total` when `total < f`; a decel ramp, then the accel tail.
        let f = a / d;
        let fvar7 = if p3 < f { p3 } else { f };
        if not_above(p2, fvar7) {
            return (p2 * d * HALF * p2, 0xa2); // 0x5f8ee8
        }
        return (a * HALF * f + (p2 - f) * a, 0xa3);
    }

    // 0x5f8dd9: both neighbours, the full trapezoid. `r` is stored as `f32`.
    let r = (a / d) as f32;
    let h = p3 * HALF;
    // `fcom r; test ah,5; jnp` keeps `h` only when `h < r` is ordered; equal, greater and NaN take `r`.
    let fvar4 = if h < f64::from(r) { h } else { f64::from(r) };
    let lo = (p3 - fvar4) as f32; // fstp [ebp+0x14]
    if not_above(p2, f64::from(lo)) {
        // 0x5f8e58
        if not_above(p2, fvar4) {
            return (p2 * d * HALF * p2, 0xa2); // 0x5f8e8a
        }
        // `(p2 - r) * accel + accel * 0.5 * r`
        return ((p2 - f64::from(r)) * a + a * HALF * f64::from(r), 0xa3);
    }
    // 0x5f8e0f: the trapezoid summed as `((2j - k * decel) * 0.5 * k + (total - 2 * fvar4) * accel)
    // + j * 0.5 * fvar4`, with `j = fvar4 * decel` and `k = p2 - lo`.
    let j = fvar4 * d;
    let k = p2 - f64::from(lo);
    let two_j = j + j;
    let two_fv4 = fvar4 + fvar4;
    let term_c = (two_j - k * d) * HALF * k;
    let term_b = (p3 - two_fv4) * a;
    let term_a = j * HALF * fvar4;
    ((term_c + term_b) + term_a, 0xa4)
}
