//! A ray against a plane (`0x7c22b0`) and a point in a polygon (`0x7c23e0`), the two halves of a
//! ray and triangle test.

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x811658]`: 1e-4 as an `f64`, the parallel test on `|dir·n|`.
const PARALLEL_EPS: f64 = f64::from_bits(0x3f1a_36e2_eb1c_432d);

/// A ray: origin at `+0`, direction at `+0xc`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ray {
    pub origin: [f32; 3],
    pub dir: [f32; 3],
}

/// A plane: normal at `+0`, constant `d` at `+0xc`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Plane {
    pub normal: [f32; 3],
    pub d: f32,
}

/// `0x7c22b0`: `ecx` = the ray, `edx` = the plane, then `out_t`, `out_point` (either may be null)
/// and the distance tolerance `eps`; `ret 0xc`. Returns the hit and the two values it writes.
///
/// `D = dir·n` is rounded to `f32`. With `|D| ≤ 1e-4` the ray is parallel: it hits at `t = 0`,
/// the point its origin, when `|origin·n + d| ≤ eps`, and misses otherwise (NaN misses).
/// Otherwise `N = origin·n + d` and `t = −N/D` when `|N| > eps` or NaN, else 0; `out_t` is `t`
/// rounded, and the point is `origin + t·dir` with `t·dir.x` and `t·dir.y` rounded to `f32` first
/// and `t·dir.z` not.
pub fn ray_plane(ray: &Ray, plane: &Plane, dist_eps: f32) -> (bool, f32, [f32; 3]) {
    let n = plane.normal;
    let o = ray.origin;
    let dir = ray.dir;

    let d_den_f32 = ((f(dir[1]) * f(n[1]) + f(dir[2]) * f(n[2])) + f(n[0]) * f(dir[0])) as f32;
    let d_abs = f(d_den_f32).abs();

    // fcomp qword [0x811658]; jp: |D| > 1e-4 or NaN takes the non-parallel path.
    let non_parallel = d_abs > PARALLEL_EPS || d_abs.is_nan();

    if !non_parallel {
        let n_num = (f(o[1]) * f(n[1]) + f(o[2]) * f(n[2])) + f(o[0]) * f(n[0]) + f(plane.d);
        if n_num.abs() > f(dist_eps) || n_num.is_nan() {
            return (false, 0.0, [0.0; 3]);
        }
        return (true, 0.0, o);
    }

    let n_num = ((f(n[0]) * f(o[0]) + f(n[2]) * f(o[2])) + f(n[1]) * f(o[1])) + f(plane.d);

    let t80: f64 = if n_num.abs() > f(dist_eps) || n_num.is_nan() {
        -(n_num / f(d_den_f32))
    } else {
        0.0
    };

    let out_t = t80 as f32;

    // t·dir.x and t·dir.y are stored and reloaded; t·dir.z stays in the register.
    let tdx = (t80 * f(dir[0])) as f32;
    let tdy = (t80 * f(dir[1])) as f32;
    let point = [
        ((f(tdx) + f(o[0])) as f32),
        ((f(tdy) + f(o[1])) as f32),
        ((t80 * f(dir[2]) + f(o[2])) as f32),
    ];

    (true, out_t, point)
}

/// The two in-plane axes for each dominant axis: `A` from `0xcf5c80` and `B` from `0xcf5c7c`, at
/// a stride of 8 bytes. Both are zero in the image; the static initializer `0x7c18d0` writes them.
const PROJ_A: [usize; 3] = [2, 0, 1];
const PROJ_B: [usize; 3] = [1, 2, 0];

/// `0x7c23e0`: `ecx` = the query point, `edx` = the vertices (vertex `i`'s component `c` at
/// `3*i + c`), then the vertex count `n` and the dominant axis; `ret 8`. Returns `al`.
///
/// A crossing count in the plane of the two other axes `A` and `B`. Walking each edge from
/// `prev` to `cur`, starting with the closing edge, a vertex's side is `v.A ≥ q.A` (false for
/// NaN). Where the sides differ, `(cur.B − q.B)·(prev.A − cur.A) ≤ (prev.B − cur.B)·(cur.A − q.A)`
/// (false for NaN) equal to `cur`'s side toggles the result, which starts false. `n = 0` returns
/// false.
pub fn point_in_poly(query: &[f32; 3], verts: &[f32], n: usize, axis_idx: usize) -> bool {
    let a = PROJ_A[axis_idx];
    let b = PROJ_B[axis_idx];

    let v = |i: usize, c: usize| -> f32 { verts[3 * i + c] };

    let q_a = f(query[a]);
    let q_b = f(query[b]);

    let mut inside = false;

    if n == 0 {
        return inside;
    }

    let mut prev = n - 1;
    let mut prev_side = f(v(prev, a)) >= q_a;

    for cur in 0..n {
        let cur_side = f(v(cur, a)) >= q_a;

        if prev_side != cur_side {
            let term1 = (f(v(prev, b)) - f(v(cur, b))) * (f(v(cur, a)) - q_a);
            let term2 = (f(v(cur, b)) - q_b) * (f(v(prev, a)) - f(v(cur, a)));
            let orient = term2 <= term1;
            if orient == cur_side {
                inside = !inside;
            }
        }

        prev_side = cur_side;
        prev = cur;
    }

    inside
}

/// `0x7bf680`: the dominant axis, the index of the largest `|n_i|`, the earlier index on a tie or
/// NaN. Not run against the exe.
pub fn dominant_axis(n: &[f32; 3]) -> usize {
    let ax = f(n[0]).abs();
    let ay = f(n[1]).abs();
    let az = f(n[2]).abs();
    let (mut idx, max) = if ay > ax { (1usize, ay) } else { (0usize, ax) };
    if az > max {
        idx = 2;
    }
    idx
}
