//! The curve object's evaluators: its control points are `C3Vector`s (`this+0x10`, stride 12), its
//! segment lengths `f32`s (`this+0x20`), and `this+0x4` holds the scale `seg_locate` multiplies the
//! parameter by. The basis matrices are data the callers pass in, so each function here is the
//! algorithm and its x87 order.

/// `0x453620` (`ecx` = the degree, `edx` = a row of coefficients, `t` on the stack; the result in
/// `st0`): the polynomial in Horner form at 53 bits, `w = c[0]; for i in 1..=degree { w = w·t + c[i] }`.
/// A degree of 0 runs no step.
#[must_use]
pub fn basis_weight(degree: u32, coeff: &[f32], t: f32) -> f64 {
    let t = f64::from(t);
    let mut w = f64::from(coeff[0]);
    for c in &coeff[1..=degree as usize] {
        w = w * t + f64::from(*c);
    }
    w
}

/// The body `0x453580` and `0x453640` share: for each of four control points `P[seg + i]` the
/// weight `basis_weight(degree, row i, t)` times the point is added to the output. The x component
/// of a product stays in the register before the add; the y and z products are stored as `f32`
/// first. Every sum is stored as `f32`.
fn eval_point_body(
    degree: u32,
    coeff: &[f32],
    row_stride: usize,
    cps: &[[f32; 3]; 4],
    t: f32,
) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for (i, p) in cps.iter().enumerate() {
        let w = basis_weight(degree, &coeff[i * row_stride..], t);
        out[0] = (w * f64::from(p[0]) + f64::from(out[0])) as f32;
        let wy = (w * f64::from(p[1])) as f32;
        let wz = (w * f64::from(p[2])) as f32;
        out[1] = (f64::from(wy) + f64::from(out[1])) as f32;
        out[2] = (f64::from(wz) + f64::from(out[2])) as f32;
    }
    out
}

/// `0x453580` (`ecx` = the curve, the stack arguments the segment, `t`, the coefficients and the
/// output): the point of degree 3, rows of four coefficients (16 bytes) apart.
#[must_use]
pub fn eval_point_cubic(coeff: &[f32], cps: &[[f32; 3]; 4], t: f32) -> [f32; 3] {
    eval_point_body(3, coeff, 4, cps, t)
}

/// `0x453640`: the point of degree 2, with the same arguments and rows of four coefficients.
#[must_use]
pub fn eval_point_quad(coeff: &[f32], cps: &[[f32; 3]; 4], t: f32) -> [f32; 3] {
    eval_point_body(2, coeff, 4, cps, t)
}

/// `0x453760` (`ecx` = the curve, the stack arguments the segment and the coefficients; the result
/// in `st0`): the length of the cubic curve as the sum of 20 chords, the points at `t = 0.05·k`.
/// `sample` is `0x453580` at the fixed segment and coefficients, and `step_const` is the `f32` at
/// `0x8029c8` (0.05). Each chord is `√((dz² + dy²) + dx²)` of the differences of the `f32` points,
/// the running sum is stored as `f32` after each, and `t` is an `f32` stepped by `step_const`
/// from `0x3d4ccccd` (0.05).
#[must_use]
pub fn arc_length(step_const: f32, mut sample: impl FnMut(f32) -> [f32; 3]) -> f64 {
    let mut prev = sample(0.0);
    let mut acc = 0.0f32;
    let mut t = f32::from_bits(0x3d4c_cccd);
    for _ in 0..20 {
        let cur = sample(t);
        let dx = f64::from(cur[0]) - f64::from(prev[0]);
        let dy = f64::from(cur[1]) - f64::from(prev[1]);
        let dz = f64::from(cur[2]) - f64::from(prev[2]);
        let len = ((dz * dz + dy * dy) + dx * dx).sqrt();
        acc = (len + f64::from(acc)) as f32;
        t = (f64::from(t) + f64::from(step_const)) as f32;
        prev = cur;
    }
    f64::from(acc)
}

/// `0x453840` (`ecx` = the curve, the stack arguments `t`, an unused word, the segment count and
/// the two output pointers): the segment index and the local parameter, `(idx, frac)`. A count of
/// 1 gives `(0, t)`. Otherwise `s = scale·t` at 53 bits, and the segment lengths are walked
/// with a running sum stored as `f32` after each, stopping at the first `idx` where `s <
/// running + len[idx]` (strictly; an equal or unordered `s` goes on) or at the last segment; `frac
/// = (s − running) / len[idx]` stored as `f32`.
#[must_use]
pub fn seg_locate(scale: f32, seg_len: &[f32], t: f32, n_seg: u32) -> (u32, f32) {
    if n_seg == 1 {
        return (0, t);
    }
    let s = f64::from(scale) * f64::from(t);
    let mut running = 0.0f32;
    let mut idx = 0u32;
    let last = n_seg.wrapping_sub(1);
    if last != 0 {
        loop {
            let next = (f64::from(seg_len[idx as usize]) + f64::from(running)) as f32;
            if s < f64::from(next) {
                break;
            }
            idx += 1;
            running = next;
            if idx >= last {
                break;
            }
        }
    }
    let frac = ((s - f64::from(running)) / f64::from(seg_len[idx as usize])) as f32;
    (idx, frac)
}

/// `0x4538c0` (`cdecl`: `t`, a count `n`, two output pointers): the index and remainder when `[0,
/// 1]` is cut into `n` equal parts, `(idx, frac)`. `q = t·n + 512` is stored as `f32` (`bias` is
/// the `f32` at `0x8029cc`), and `idx` is byte 1 of its bits, `(bits >> 14) & 0xff`. `frac = (t −
/// (1/n)·idx)·n` with `1/n` at 53 bits (`one` is the `f32` at `0x7ff9d8`), stored as `f32`.
#[must_use]
pub fn fixed_quantize(bias: f32, one: f32, t: f32, n: u32) -> (u32, f32) {
    let f_n = f64::from(n);
    let q = (f64::from(t) * f_n + f64::from(bias)) as f32;
    let idx = (q.to_bits() >> 14) & 0xff;
    let recip = f64::from(one) / f_n;
    let frac = ((f64::from(t) - recip * f64::from(idx)) * f_n) as f32;
    (idx, frac)
}

/// `0x453300` (`ecx` = the curve, `n` on the stack; the result in `st0`): the sum of the first
/// `n − 1` segment lengths at 53 bits, starting from the `f32` at `0x7ffd74` (`zero`); a count of 0
/// returns `zero`.
#[must_use]
pub fn knot_sum_nm1(zero: f32, seg_len: &[f32], n: u32) -> f64 {
    if n == 0 {
        return f64::from(zero);
    }
    let mut acc = f64::from(zero);
    for &k in seg_len.iter().take((n - 1) as usize) {
        acc += f64::from(k);
    }
    acc
}

/// `0x453920` (`ecx` = the curve, `n` on the stack; the result in `st0`): the sum of the first `n`
/// segment lengths at 53 bits, starting from `zero`.
#[must_use]
pub fn knot_sum_n(zero: f32, seg_len: &[f32], n: u32) -> f64 {
    let mut acc = f64::from(zero);
    for &k in seg_len.iter().take(n as usize) {
        acc += f64::from(k);
    }
    acc
}
