//! The squared distance between two segments (`0x6dc900`), returned in `st0`.
//!
//! `__fastcall`: `ecx` = `p1`, `edx` = `p2`, then the directions `d1` and `d2`, the segments
//! being `p1 + s·d1` and `p2 + t·d2`. It has no epsilon: the parameter clamps compare
//! cross-multiplied numerators with the denominator, and every exit takes `|…|` of its sum. `r`
//! is `p2 − p1`, so the linear terms enter as `+2c` and `+2f`.

// A negated compare is how a NaN takes the branch the x87 test gives it.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `0x6dc900`: with `a = d1·d1`, `e = d2·d2`, `b = d1·d2`, `c = r·d1`, `f = r·d2`, the
/// denominator `|ea − b²|` and the numerators `sN = fb − ce` and `cb − fa`, each rounded where the
/// code below stores it, a ladder of compares picks the interior or one of the clamped exits.
pub fn seg_seg_dist_sq(p1: &[f32; 3], p2: &[f32; 3], d1: &[f32; 3], d2: &[f32; 3]) -> f32 {
    let r: [f32; 3] = core::array::from_fn(|i| (f(p2[i]) - f(p1[i])) as f32);
    let a = ((f(d1[0]) * f(d1[0]) + f(d1[1]) * f(d1[1])) + f(d1[2]) * f(d1[2])) as f32;
    let e = ((f(d2[0]) * f(d2[0]) + f(d2[1]) * f(d2[1])) + f(d2[2]) * f(d2[2])) as f32;
    // r·r sums x, z, y and stays in the register.
    let rr = (f(r[0]) * f(r[0]) + f(r[2]) * f(r[2])) + f(r[1]) * f(r[1]);
    let b = ((f(d2[2]) * f(d1[2]) + f(d2[1]) * f(d1[1])) + f(d1[0]) * f(d2[0])) as f32;
    let c = ((f(r[2]) * f(d1[2]) + f(r[1]) * f(d1[1])) + f(r[0]) * f(d1[0])) as f32;
    let fv = ((f(r[2]) * f(d2[2]) + f(r[1]) * f(d2[1])) + f(r[0]) * f(d2[0])) as f32;
    let denom = (f(e) * f(a) - f(b) * f(b)).abs() as f32;
    let sn = (f(fv) * f(b) - f(c) * f(e)) as f32;
    let cbfa = f(c) * f(b) - f(fv) * f(a);

    let leaf_rr = || rr.abs() as f32;
    let leaf_s1 = || (((f(c) + f(c)) + rr) + f(a)).abs() as f32;
    let leaf_t1 = || (((f(fv) + f(fv)) + rr) + f(e)).abs() as f32;
    let leaf_s0_div = || (rr + ((-1.0 / f(e)) * f(fv)) * f(fv)).abs() as f32;
    let leaf_t0_div = || (rr + ((-1.0 / f(a)) * f(c)) * f(c)).abs() as f32;
    // s = 0: three exits on f.
    let s0_t = || {
        if f(fv) >= 0.0 {
            return leaf_rr();
        }
        if -f(fv) < f(e) {
            return leaf_s0_div();
        }
        leaf_t1()
    };
    // t = 0: two exits on c.
    let t0_s = || {
        if -f(c) < f(a) {
            return leaf_t0_div();
        }
        leaf_s1()
    };
    // The interior reloads s from its f32 store and keeps t in the register.
    let interior = || {
        let inv = 1.0 / f(denom);
        let s32 = (f(sn) * inv) as f32;
        let tt = cbfa * inv;
        let k = ((f(s32) * f(b) + tt * f(e)) + (f(fv) + f(fv))) * tt;
        let m = ((tt * f(b) + f(s32) * f(a)) + (f(c) + f(c))) * f(s32);
        ((k + m) + rr).abs() as f32
    };
    // From 0x6dcc57, s in the register.
    let t_clamp_tail = |s: f64| {
        let oms = 1.0 - s;
        let x = ((f(b) * oms + s * f(e)) + (f(fv) + f(fv))) * s;
        let y = ((s * f(b) + oms * f(a)) + (f(c) + f(c))) * oms;
        ((x + y) + rr).abs() as f32
    };
    // From 0x6dcd3b, s in the register.
    let t1_tail = |s: f64| {
        let oms = 1.0 - s;
        let w = ((s * f(b) + oms * f(e)) + (f(fv) + f(fv))) * oms;
        let v = ((oms * f(b) + s * f(a)) + (f(c) + f(c))) * s;
        ((w + v) + rr).abs() as f32
    };
    let diag = || (f(a) - (f(b) + f(b))) + f(e);

    // sN + (cb − fa) against the denominator; NaN takes the s ≥ 1 side.
    let sum = f(sn) + cbfa;
    if !(sum <= f(denom)) {
        if !(f(sn) >= 0.0) {
            let cb = (f(c) + f(b)) as f32;
            let fe = f(fv) + f(e);
            if fe > f(cb) {
                let num = fe - f(cb);
                let den = diag();
                if num < den {
                    return t1_tail(num / den);
                }
                return leaf_s1();
            }
            return s0_t();
        }
        if cbfa >= 0.0 {
            // 0x6dcceb `fst` stores q as f32 and keeps it in the register: the > 0 test reads the
            // register, the compare and the divide after 0x6dcd18 the stored value.
            let q_live = (f(fv) + f(e)) - f(b) - f(c);
            let q = q_live as f32;
            if !(q_live > 0.0) {
                return leaf_t1();
            }
            let den = diag();
            if f(q) < den {
                return t1_tail(f(q) / den);
            }
            return leaf_s1();
        }
        let fb = (f(fv) + f(b)) as f32;
        let ca = f(c) + f(a);
        if ca > f(fb) {
            let num = ca - f(fb);
            let den = diag();
            if num < den {
                return t_clamp_tail(num / den);
            }
            return leaf_t1();
        }
        if f(c) >= 0.0 {
            return leaf_rr();
        }
        return t0_s();
    }
    if f(sn) >= 0.0 {
        if cbfa >= 0.0 {
            return interior();
        }
        if f(c) >= 0.0 {
            return leaf_rr();
        }
        return t0_s();
    }
    if cbfa >= 0.0 {
        return s0_t();
    }
    if f(c) < 0.0 {
        return t0_s();
    }
    s0_t()
}
