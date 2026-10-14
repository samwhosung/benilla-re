//! The client's angle wrap `0x455b30` (`angle_wrap`), its quadratic solver `0x454f40`
//! (`quadratic`) and its software `exp2`: the exponent
//! field, a 64-entry table and a polynomial, with no `f2xm1`, so the result is exact on any CPU,
//! unlike CRT `pow` (`0x73f940`). The x87 runs at 53-bit precision (`f64`). Its `log2`,
//! `FastLog2` (`0x454c80`), is unreached and owes no port.
#![allow(clippy::float_cmp)]

pub mod angle_wrap;
pub mod quadratic;

/// The data `exp2` reads, from the reader's exe. The table is 64 qwords, `LUT[i] = 2^(i/64)`, indexed
/// by the top six mantissa bits of the fraction.
pub struct Exp2Tables<'a> {
    /// `0x8029e0`: `LUT[i] = 2^(i/64)`.
    pub lut: &'a [u8],
    /// `c0` to `c5`, the polynomial for `2^rf`, at `0x803028` to `0x803050` in address order.
    pub coeffs: [f64; 6],
    /// `0x803058`: 1023.0, the exponent bias.
    pub bias: f64,
    /// `0x875e90`: `+inf`, the result on overflow (`n` above 0x7ff unsigned, and positive).
    pub inf: f64,
    /// `0x7ffd74`: `0.0` as `f32`, the result when `n <= 0`.
    pub underflow: f64,
}

/// `0x454d40`: `x` at `[ebp+8]`, the result in `st0`, `ret 8`. `S = x + 1023`, `n = trunc(S) − 1`,
/// `frac = S − n` in `[1,2)`; the top six mantissa bits of `frac` pick `LUT[i]` and are cleared to
/// give `rf`; a degree-5 polynomial gives `2^rf`, and `exp2(x) = poly·LUT[i]·2^n`. `n` above 0x7ff
/// unsigned gives `+inf` when positive and `0.0` otherwise. The operations are in the client's order.
pub fn exp2(x: f64, t: &Exp2Tables) -> f64 {
    let [c0, c1, c2, c3, c4, c5] = t.coeffs;

    // fld x; fadd [0x803058]; fld st(0); call __ftol (truncates); dec.
    let s = x + t.bias;
    let n = (s.trunc() as i64 as i32).wrapping_sub(1); // __ftol's int64, low dword in eax

    // [ebp+8] = (int64)n; fild; fsubr st,st(1); fstp [ebp+8].
    let frac = s - f64::from(n);

    // `fstp qword [ebp+8]` writes frac's high dword over [ebp+0xc], so `edx = [ebp+0xc]` reads it,
    // not the 0 stored there before.
    let frac_hi = (frac.to_bits() >> 32) as u32;
    let frac_lo = frac.to_bits() as u32;

    // ecx = frac_hi & 0xfc000, the top six mantissa bits; edx ^= ecx clears them.
    let bits6 = frac_hi & 0x000f_c000;
    let rf_hi = frac_hi ^ bits6;
    let rf = f64::from_bits(((rf_hi as u64) << 32) | frac_lo as u64); // [ebp+8]; fld

    // ecx >>= 0xb, the table's byte offset; fmul qword [ecx+0x8029e0].
    let byte_off = (bits6 >> 11) as usize; // 0 to 0x1f8
    let lut = f64::from_le_bytes(t.lut[byte_off..byte_off + 8].try_into().unwrap());

    // edx = n << 0x14, the exponent field of 2^n; [ebp-8] = 0; fld qword [ebp-8].
    let scale = f64::from_bits(((n as u32).wrapping_shl(20) as u64) << 32);

    // cmp eax,0x7ff; jbe: in range. Otherwise test eax; jle: 0.0 when n <= 0, else +inf.
    if (n as u32) > 0x7ff {
        return if n <= 0 { t.underflow } else { t.inf };
    }

    // 2^n·LUT[i]; at n == 0x7ff the scale is +inf, and so is the result.
    let scaled = scale * lut;

    // The polynomial in the client's order, from st0 = scaled, st1 = rf², st2 = rf:
    let rf2 = rf * rf; // fld st(0); fmul st,st(1)
    let a = (c4 + rf * c5) * rf2; // fld st(2); fmul c5; fadd c4; fmul st,st(2)
    let b = (c2 + rf * c3) + a; // fld st(3); fmul c3; fadd c2; faddp st(1),st
    let bp = b * rf2; // fmul st,st(2)
    let poly = (c0 + rf * c1) + bp; // fxch st(3); fmul c1; fadd c0; faddp st(3),st
    poly * scaled // fxch st(2); fmulp st(2),st
}
