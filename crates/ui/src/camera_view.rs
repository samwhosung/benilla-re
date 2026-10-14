//! The camera's view basis: applying a shot's eye, look-at and roll (`systems/uicamera/cinematic.md
//! #applying-the-view`) and the three basis rows the camera returns
//! (`systems/uicamera/driver.md#riding-a-transport`).
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and each `fstp dword` is an `as f32`.

/// The `f32` at `0x8029d4`, 2^-22: a vector shorter than this is left as it is.
const NORM_EPS: f64 = f32::from_bits(0x3480_0000) as f64;

/// What `CCamera::SetLookAt` (`0x50d5d0`) hands the basis builder `0x50a930`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookRoll {
    /// The unit vector from the eye to the target.
    pub dir: [f32; 3],
    /// The sine of the roll, from `fsincos`.
    pub sin: f32,
    /// The cosine of the roll.
    pub cos: f32,
}

/// `CCamera::SetLookAt` (`0x50d5d0`): forms `d = target - eye` and normalizes it when
/// `|sqrt(dz^2 + dx^2 + dy^2)| >= 2^-22`, then takes the sine and cosine of `roll`.
///
/// The `dx` and `dy` differences are stored as `f32` and reloaded, so they square as `f32`; `dz` is
/// stored with `fst` and not popped, so its square multiplies the register-precision difference by
/// its `f32` copy. The sum adds `dz^2`, then `dx^2`, then `dy^2`. Each normalized component and
/// the sine and cosine are stored as `f32`. The copy of the eye into `this+8` is not returned.
pub fn set_look_at(eye: [f32; 3], tgt: [f32; 3], roll: f32) -> LookRoll {
    let dxf = (f64::from(tgt[0]) - f64::from(eye[0])) as f32;
    let dyf = (f64::from(tgt[1]) - f64::from(eye[1])) as f32;
    let dz_full = f64::from(tgt[2]) - f64::from(eye[2]);
    let dzf = dz_full as f32;
    let s = dz_full * f64::from(dzf)
        + f64::from(dxf) * f64::from(dxf)
        + f64::from(dyf) * f64::from(dyf);
    let len = s.sqrt();
    let (mut x, mut y, mut z) = (dxf, dyf, dzf);
    if NORM_EPS <= len.abs() {
        let inv = 1.0 / len;
        x = (f64::from(dxf) * inv) as f32;
        y = (f64::from(dyf) * inv) as f32;
        z = (f64::from(dzf) * inv) as f32;
    }
    let r = f64::from(roll);
    LookRoll {
        dir: [x, y, z],
        sin: r.sin() as f32,
        cos: r.cos() as f32,
    }
}

/// The basis rows `CCamera::GetForward` (`0x511f40`), `CCamera::GetLeft` (`0x511fe0`) and
/// `CCamera::GetUp` (`0x512080`): one row `v` of the basis, rotated by the transport's facing.
/// They differ only in which row they read (`+0x14`, `+0x20`, `+0x2c`).
///
/// With no transport (`identity`: the GUID words `+0x98` and `+0x9c` are both 0) the row is returned
/// as it is. Otherwise `out = Mᵀ v` for the 3x3 rotation `m` that `0x511e90` builds, row-major, with
/// the sums formed in the order the binary builds them on the x87 stack and stored as `f32`:
/// ```text
/// out[0] = (v0*M0) + ((v1*M3) + (v2*M6))
/// out[1] = ((v0*M1) + (v2*M7)) + (v1*M4)
/// out[2] = ((v0*M2) + (v2*M8)) + (v1*M5)
/// ```
pub fn basis_row(v: [f32; 3], identity: bool, m: &[f32; 9]) -> [f32; 3] {
    if identity {
        return v;
    }
    let (v0, v1, v2) = (f64::from(v[0]), f64::from(v[1]), f64::from(v[2]));
    let md = |i: usize| f64::from(m[i]);
    let out0 = (v0 * md(0) + (v1 * md(3) + v2 * md(6))) as f32;
    let out1 = ((v0 * md(1) + v2 * md(7)) + v1 * md(4)) as f32;
    let out2 = ((v0 * md(2) + v2 * md(8)) + v1 * md(5)) as f32;
    [out0, out1, out2]
}
