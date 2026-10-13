//! The values the view, orthographic and unprojection setups compute before a matrix builder
//! (`gxumath`) and the device take over. Each is one `fadd`, `fsub`, `fmul` or `fdiv` of `f32`
//! operands at 53 bits, stored with `fstp dword`.

#![allow(clippy::float_cmp)]

/// The `CCamera` fields [`view_lookat_owned`] reads (`systems/camera.md#properties`): the eye
/// `+0x3c` and target `+0x64`, and the cosine and sine an angle property keeps at its `+0x20` and
/// `+0x24` (`systems/camera.md#angle-properties`), of yaw (`+0x148`, so `+0x168` and `+0x16c`) and of roll (`+0x120`, so `+0x140` and
/// `+0x144`).
#[derive(Clone, Copy, Debug)]
pub struct LookParams {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    /// `+0x16c`
    pub sin_yaw: f32,
    /// `+0x168`
    pub cos_yaw: f32,
    /// `+0x144`
    pub sin_roll: f32,
    /// `+0x140`
    pub cos_roll: f32,
}

/// What `CCamera::SetDeviceTransforms` (`0x7ac640`, `systems/camera.md#view-and-projection`)
/// computes for the matrix builders: the aspect `(vp[3]−vp[1])/(vp[2]−vp[0])` for the projection,
/// and for the view the direction `target − eye` and the up
/// `(sin yaw·sin roll, −(cos yaw·sin roll), cos roll)`, read as `+0x16c·+0x144`, `−(+0x168·+0x144)`
/// and `+0x140` (`0x7ac778` to `0x7ac7dc`).
#[must_use]
pub fn view_lookat_owned(vp: &[f32; 4], p: &LookParams) -> (f32, [f32; 3], [f32; 3]) {
    let aspect =
        ((f64::from(vp[3]) - f64::from(vp[1])) / (f64::from(vp[2]) - f64::from(vp[0]))) as f32;
    let dir = [
        (f64::from(p.target[0]) - f64::from(p.eye[0])) as f32,
        (f64::from(p.target[1]) - f64::from(p.eye[1])) as f32,
        (f64::from(p.target[2]) - f64::from(p.eye[2])) as f32,
    ];
    let up = [
        (f64::from(p.sin_yaw) * f64::from(p.sin_roll)) as f32,
        (-(f64::from(p.cos_yaw) * f64::from(p.sin_roll))) as f32,
        p.cos_roll,
    ];
    (aspect, dir, up)
}

/// The centre of `SetupOrthoTransforms` (`0x7ad7f0`, `systems/camera.md#orthographic-setup`):
/// `((rect[0]+rect[2])·0.5, (rect[1]+rect[3])·0.5)`, the entry's `cy` and `cx`.
#[must_use]
pub fn ortho_midpoints(rect: &[f32; 4]) -> (f32, f32) {
    let mid_x = ((f64::from(rect[0]) + f64::from(rect[2])) * 0.5) as f32;
    let mid_y = ((f64::from(rect[1]) + f64::from(rect[3])) * 0.5) as f32;
    (mid_x, mid_y)
}

/// The rectangle `0x7ad7f0` passes to `BuildOrthoMatrix` (`0x5c3d90`), moved to the centre:
/// `(rect[1]−cx, rect[3]−cx, rect[0]−cy, rect[2]−cy)`, each an `fadd` of the negated centre.
#[must_use]
pub fn ortho_corner_args(rect: &[f32; 4]) -> [f32; 4] {
    let (mx, my) = ortho_midpoints(rect);
    let (nmx, nmy) = (-mx, -my);
    [
        (f64::from(rect[1]) + f64::from(nmy)) as f32,
        (f64::from(nmy) + f64::from(rect[3])) as f32,
        (f64::from(nmx) + f64::from(rect[0])) as f32,
        (f64::from(nmx) + f64::from(rect[2])) as f32,
    ]
}

/// The view translation `0x7ad7f0` passes to `0x7bdc40`: `(p[0]−cx, p[1]−cy)`.
#[must_use]
pub fn ortho_translate(out2: &[f32; 2], mid_x: f32, mid_y: f32) -> [f32; 2] {
    [
        (f64::from(out2[0]) - f64::from(mid_y)) as f32,
        (f64::from(out2[1]) - f64::from(mid_x)) as f32,
    ]
}

/// The shift `0x7ad7f0` applies when `0x589ca0` returns 1 (`0x7ad946` to `0x7ad976`):
/// `mat12 − (r[3]−r[1])/(s[3]−s[1])·0.5` (`fsubr`) and `(r[2]−r[0])/(s[2]−s[0])·0.5 + mat13`.
#[must_use]
pub fn ortho_recenter(rect: &[f32; 4], screen: &[f32; 4], mat12: f32, mat13: f32) -> (f32, f32) {
    let y = (f64::from(mat12)
        - ((f64::from(rect[3]) - f64::from(rect[1]))
            / (f64::from(screen[3]) - f64::from(screen[1])))
            * 0.5) as f32;
    let x = (((f64::from(rect[2]) - f64::from(rect[0]))
        / (f64::from(screen[2]) - f64::from(screen[0])))
        * 0.5
        + f64::from(mat13)) as f32;
    (y, x)
}

// `UnprojectScreenPoint` (`0x7ad2c0`): each step is an `fsub`, `fmul` or `fadd` of `f32` operands,
// stored with `fstp dword` and passed to `SetVector3` (`0x4549a0`), which copies three dwords.

/// `a − b` per component, each rounded to `f32`.
#[inline]
fn vsub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(a[0]) - f64::from(b[0])) as f32,
        (f64::from(a[1]) - f64::from(b[1])) as f32,
        (f64::from(a[2]) - f64::from(b[2])) as f32,
    ]
}
/// `a + b` per component, each rounded to `f32`.
#[inline]
fn vadd(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(a[0]) + f64::from(b[0])) as f32,
        (f64::from(a[1]) + f64::from(b[1])) as f32,
        (f64::from(a[2]) + f64::from(b[2])) as f32,
    ]
}
/// `v·s` per component, each rounded to `f32`.
#[inline]
fn vscale(v: [f32; 3], s: f32) -> [f32; 3] {
    [
        (f64::from(v[0]) * f64::from(s)) as f32,
        (f64::from(v[1]) * f64::from(s)) as f32,
        (f64::from(v[2]) * f64::from(s)) as f32,
    ]
}

/// The near and far points of `UnprojectScreenPoint` (`0x7ad2c0`,
/// `systems/camera.md#unprojection`), from the eight corners `BuildFrustumCorners` (`0x5c43b0`)
/// writes and the fractions `p3` (`a`) and `p4` (`b`).
///
/// Near, through `ecx`: `E = C0 + (C1−C0)·p4`, `F = C3 + (C2−C3)·p4`, `E + (F−E)·p3`, computed
/// inline. Far, through `edx`: `E2 = C4 + (C5−C4)·p4` and `(C6−C7)·p4` inline, then the add
/// `0x621160`, subtract `0x4841f0` and scale `0x5f8cf0` give `F2 = C7 + (C6−C7)·p4` and
/// `E2 + (F2−E2)·p3`, with the same `f32` rounding per component. Returns `(near, far)`.
#[must_use]
pub fn frustum_split_owned(corners: &[[f32; 3]; 8], p3: f32, p4: f32) -> ([f32; 3], [f32; 3]) {
    let c = corners;
    // Near.
    let e = vadd(c[0], vscale(vsub(c[1], c[0]), p4));
    let f = vadd(c[3], vscale(vsub(c[2], c[3]), p4));
    let out8 = vadd(e, vscale(vsub(f, e), p3));

    // Far: E2 and (C6−C7)·p4 inline, the rest through 0x621160, 0x4841f0 and 0x5f8cf0.
    let e2 = vadd(c[4], vscale(vsub(c[5], c[4]), p4));
    let b = vscale(vsub(c[6], c[7]), p4);
    let f2 = vadd(c[7], b);
    let out2 = vadd(e2, vscale(vsub(f2, e2), p3));

    (out8, out2)
}
