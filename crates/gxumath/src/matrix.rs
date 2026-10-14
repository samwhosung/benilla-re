//! The projection and view builders. Each checks its arguments, and a bad one ends in a call to
//! `SErrSetLastError` (`0x64e850`) with no matrix written, so these model the valid arguments only:
//! `0 < fov < π`, `aspect > 0` and `near < far` for the perspective; `left ≠ right`, `bottom ≠ top`
//! and `near < far` for the orthographic; and squared lengths of the direction and of `up` of at
//! least 0.01 for the look-at.

use benilla_re_cmath::transform::translate_matrix;

/// `BuildPerspectiveMatrix` (`0x5c3cc0`, `ecx` = the output, `fov`, `aspect`, `near` and `far` on
/// the stack; `systems/camera.md#view-and-projection`). The half-angle is `t = tan(h)` with `h =
/// (fov / √(aspect² + 1))·0.5`, so `fov` is the diagonal angle. `t·near` is computed once and
/// reused, so `m0` is `near / (aspect·(t·near))` and `m5` is `near / (t·near)`, not the cancelled
/// `1/(aspect·t)` and `1/t`. `far − near` is likewise reused: `m10` is `(near + far) / (far −
/// near)` and `m14` is `(near·far·−2) / (far − near)`. `m11 = 1`; every other entry is 0. Each is
/// stored as `f32`.
#[must_use]
pub fn projection_perspective(fov: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    let asp = f64::from(aspect);
    let denom = (asp * asp + 1.0).sqrt();
    let angle = (f64::from(fov) / denom) * 0.5;
    let t = angle.tan();
    let n = f64::from(near);
    let f = f64::from(far);
    let tn = t * n;
    let fmn = f - n;
    let mut m = [0.0f32; 16];
    m[0] = (n / (asp * tn)) as f32;
    m[5] = (n / tn) as f32;
    m[10] = ((n + f) / fmn) as f32;
    m[14] = ((n * f * -2.0) / fmn) as f32;
    m[11] = 1.0;
    m
}

/// `BuildOrthoMatrix` (`0x5c3d90`, `ecx` = the output, `left`, `right`, `bottom`, `top`, `near` and
/// `far` on the stack; `systems/camera.md#orthographic-setup`): `m0 = 2/(r−l)`, `m12 = −((l+r)/(r−l))`,
/// `m5 = 2/(t−b)`, `m13 = −((b+t)/(t−b))`, `m10 = 2/(f−n)`, `m14 = −((n+f)/(f−n))` and `m15 = 1`,
/// every other entry 0, each stored as `f32`.
#[must_use]
pub fn projection_ortho(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
) -> [f32; 16] {
    let l = f64::from(left);
    let r = f64::from(right);
    let b = f64::from(bottom);
    let t = f64::from(top);
    let n = f64::from(near);
    let f = f64::from(far);
    let mut m = [0.0f32; 16];
    m[0] = (2.0 / (r - l)) as f32;
    m[12] = (-((l + r) / (r - l))) as f32;
    m[5] = (2.0 / (t - b)) as f32;
    m[13] = (-((b + t) / (t - b))) as f32;
    m[10] = (2.0 / (f - n)) as f32;
    m[14] = (-((n + f) / (f - n))) as f32;
    m[15] = 1.0;
    m
}

/// `BuildLookAtMatrix` (`0x5c3e70`, `ecx` = the eye, `edx` = the target, `up` and the output on the
/// stack; `systems/camera.md#view-and-projection`). The forward vector `f = target − eye`
/// (differences at 53 bits, stored as `f32`) is divided by `√((fy² + fz²) + fx²)`, where each square
/// is first rounded to `f32`. The side `s = f × up` and the up `u = s × f` (each component a
/// difference of two products at 53 bits, stored as `f32`) are each divided by `√((z² + y²) + x²)`
/// with the squares at 53 bits. The upper 3×3 holds `s` in `m[0]`, `m[4]` and `m[8]`, `u` in `m[1]`, `m[5]` and
/// `m[9]` and `f` in `m[2]`, `m[6]` and `m[10]`, with `m15 = 1`; `TranslateMatrix` (`0x7bdc40`)
/// then adds `−eye` through that 3×3 into `m[12..15]`.
#[must_use]
pub fn lookat_v1(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
    let f = [
        (f64::from(target[0]) - f64::from(eye[0])) as f32,
        (f64::from(target[1]) - f64::from(eye[1])) as f32,
        (f64::from(target[2]) - f64::from(eye[2])) as f32,
    ];
    let fx2 = (f64::from(f[0]) * f64::from(f[0])) as f32;
    let fy2 = (f64::from(f[1]) * f64::from(f[1])) as f32;
    let fz2 = (f64::from(f[2]) * f64::from(f[2])) as f32;
    let len2 = f64::from(fy2) + f64::from(fz2) + f64::from(fx2);
    let f = scale3(f, 1.0 / len2.sqrt());

    let s = cross(f, up);
    let s = scale3(s, norm_inv(s));
    let u = cross(s, f);
    let u = scale3(u, norm_inv(u));

    let mut m = [0.0f32; 16];
    m[0] = s[0];
    m[4] = s[1];
    m[8] = s[2];
    m[1] = u[0];
    m[5] = u[1];
    m[9] = u[2];
    m[2] = f[0];
    m[6] = f[1];
    m[10] = f[2];
    m[15] = 1.0;
    translate_matrix(&mut m, &[-eye[0], -eye[1], -eye[2]]);
    m
}

/// `a × b` with each component `(a.y·b.z − a.z·b.y)` and so on at 53 bits, stored as `f32`.
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    let af = a.map(f64::from);
    let bf = b.map(f64::from);
    [
        (af[1] * bf[2] - af[2] * bf[1]) as f32,
        (af[2] * bf[0] - af[0] * bf[2]) as f32,
        (af[0] * bf[1] - af[1] * bf[0]) as f32,
    ]
}

/// `1 / √((z² + y²) + x²)` at 53 bits.
fn norm_inv(v: [f32; 3]) -> f64 {
    let x = f64::from(v[0]);
    let y = f64::from(v[1]);
    let z = f64::from(v[2]);
    1.0 / (z * z + y * y + x * x).sqrt()
}

/// Each component times `inv` at 53 bits, stored as `f32`.
fn scale3(v: [f32; 3], inv: f64) -> [f32; 3] {
    v.map(|c| (f64::from(c) * inv) as f32)
}
