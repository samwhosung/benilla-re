//! A circle against a box in x and y, in four modes (`0x7c2040`).
//!
//! `__fastcall`: `ecx` = the box `[min.x, min.y, min.z, max.x, max.y, max.z]`, `edx` = the
//! circle `[c.x, c.y, c.z, r]`, `[esp+4]` = the mode `n`; `ret 4`; the answer in `al`. Only the
//! first two axes are read. `r²` is rounded to `f32` once. Its one caller, `0x68af10`, passes
//! `n = 3`.

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `0x7c2040`: 1 or 0 for mode `n`; `n > 3` returns 0. Per axis, `near` adds `(c − min)²` below
/// the slab and `(c − max)²` above it, and `far` adds the larger of the two.
///
/// - 0: true when some axis is outside the box or has the nearer face within `r²`, and
///   `near ≤ r² ≤ far`; `(c − max)²` is rounded to `f32`.
/// - 1: true when some axis is outside or within `r` of a face, and `near ≤ r²`.
/// - 2: true when `near ≤ r² ≤ far`.
/// - 3: true when `near ≤ r²`.
///
/// Every `≤` is false for NaN.
pub fn sphere_aabb_classify(boxf: &[f32; 6], sph: &[f32; 4], n: u32) -> u32 {
    let r2 = f((f(sph[3]) * f(sph[3])) as f32);

    let cc = |i: usize| f(sph[i]);
    let mn = |i: usize| f(boxf[i]);
    let mx = |i: usize| f(boxf[i + 3]);

    match n {
        0 => {
            let mut far = 0.0f64;
            let mut near = 0.0f64;
            let mut hit = false;
            for i in 0..2 {
                let d_min = {
                    let t = cc(i) - mn(i);
                    t * t
                };
                // (c − max)² is stored to [ebp+8] as f32 and reloaded.
                let d_max = {
                    let t = cc(i) - mx(i);
                    f((t * t) as f32)
                };
                let farther = if d_max < d_min { d_min } else { d_max };
                far += farther;

                if cc(i) < mn(i) {
                    near += d_min;
                    hit = true;
                } else if cc(i) >= mx(i) && cc(i) != mx(i) {
                    near += d_max;
                    hit = true;
                } else {
                    let nearer = if d_min < d_max { d_min } else { d_max };
                    if !nearer.is_nan() && !r2.is_nan() && nearer <= r2 {
                        hit = true;
                    }
                }
            }
            if !hit || (near < r2) == (near == r2) {
                return 0;
            }
            le_ordered(r2, far) as u32
        }
        1 => {
            // The slab test compares with r itself, not r².
            let r = f(sph[3]);
            let mut acc = 0.0f64;
            let mut hit = false;
            for i in 0..2 {
                if cc(i) < mn(i) {
                    let t = cc(i) - mn(i);
                    acc += t * t;
                    hit = true;
                } else if cc(i) > mx(i) {
                    let t = cc(i) - mx(i);
                    acc += t * t;
                    hit = true;
                } else {
                    let a = cc(i) - mn(i);
                    let b = mx(i) - cc(i);
                    let a_hit = !a.is_nan() && !r.is_nan() && a <= r;
                    let b_hit = !b.is_nan() && !r.is_nan() && b <= r;
                    if a_hit || b_hit {
                        hit = true;
                    }
                }
            }
            if !hit {
                return 0;
            }
            le_ordered(acc, r2) as u32
        }
        2 => {
            let mut far = 0.0f64;
            let mut near = 0.0f64;
            for i in 0..2 {
                let d_min = {
                    let t = cc(i) - mn(i);
                    t * t
                };
                let d_max = {
                    let t = cc(i) - mx(i);
                    t * t
                };
                let farther = if d_max < d_min { d_min } else { d_max };
                far += farther;
                if cc(i) < mn(i) {
                    near += d_min;
                } else if cc(i) > mx(i) {
                    near += d_max;
                }
            }
            (le_ordered(near, r2) && le_ordered(r2, far)) as u32
        }
        3 => {
            let mut near = 0.0f64;
            for i in 0..2 {
                if cc(i) < mn(i) {
                    let t = cc(i) - mn(i);
                    near += t * t;
                } else if cc(i) > mx(i) {
                    let t = cc(i) - mx(i);
                    near += t * t;
                }
            }
            le_ordered(near, r2) as u32
        }
        _ => 0,
    }
}

/// `fcomp; test ah,0x41; jp` to false: true when `a ≤ b` and neither is NaN.
#[inline]
fn le_ordered(a: f64, b: f64) -> bool {
    !a.is_nan() && !b.is_nan() && a <= b
}
