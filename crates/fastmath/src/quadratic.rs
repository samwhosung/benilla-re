//! The quadratic solver `0x454f40`, the root pair of `a·t² + b·t + c`. The x87 runs at 53-bit
//! precision (`f64`) and each result is stored as `f32`.

/// `[0x80306c]`, 4.
const FOUR: f32 = 4.0;
/// `[0x803068]`, −0.5.
const NEG_HALF: f32 = -0.5;
/// `[0x7ff9d8]`, 1.
const ONE: f32 = 1.0;

/// `0x454f40(&t1, &t2, a, b, c)`: `ecx` = `t1`, `edx` = `t2`, `a`, `b` and `c` on the stack, `ret 0xc`,
/// `al` = whether there are roots. With `g = (a·c)·4` and `h = b·b` at 53 bits, `fcom` and `test
/// ah,0x41; jne` leave when `h <= g` or either is NaN, returning false with both outputs
/// untouched. Otherwise `q = √(h − g)`, `s = (b > 0 ? q + b : b − q)·(−0.5)` (`b` not above 0, NaN
/// included, takes the subtract), and `inv = 1/(a·s)` stays wide while `f32(inv)` is also kept. The
/// two roots are `r_b = (inv·s)·s`, left wide, and `r_a = f32((f32(inv)·a)·c)`. When `r_b` (wide) is
/// below `r_a` (`fcom`, `test ah,5; jp`, falling through only then) `t1 = f32(r_b)` and `t2 = r_a`;
/// otherwise, including equal and NaN, `t2 = f32(r_b)` and `t1 = r_a`. The larger root is in `t2`.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn quadratic_roots(a: f32, b: f32, c: f32) -> Option<(f32, f32)> {
    let f = f64::from;
    let g = f(a) * f(c) * f(FOUR);
    let h = f(b) * f(b);
    // `h > g`, written as the exe's negated compare so that NaN leaves.
    if !(h > g) {
        return None;
    }
    let q = (h - g).sqrt();
    let signed = if b > 0.0 { q + f(b) } else { f(b) - q };
    let s = signed * f(NEG_HALF);
    let inv = f(ONE) / (f(a) * s);
    let inv_f32 = inv as f32;
    let root_b = inv * s * s;
    let root_a = (f(inv_f32) * f(a) * f(c)) as f32;
    if root_b < f(root_a) {
        Some((root_b as f32, root_a))
    } else {
        Some((root_a, root_b as f32))
    }
}
