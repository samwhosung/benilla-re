//! The floating-point conversions `__output` calls through `_cfltcvt` (`systems/crt/printf.md`):
//! `%e`, `%f` and `%g` of a `double` into a C string, written as the runtime writes it, in place
//! in a byte buffer.

use crate::fltout::{fltout2, StrFlt};
use crate::ld12::TenPowers;

/// The decimal point, `[0x831714]`, which only the locale code writes.
const POINT: u8 = b'.';

fn strlen(buf: &[u8], at: usize) -> usize {
    buf[at..].iter().position(|&b| b == 0).expect("terminated")
}

/// `__shift` (`0x40b84f`): the string at `at`, its NUL included, moved `n` bytes up; nothing for 0.
pub fn shift(buf: &mut [u8], at: usize, n: usize) {
    if n != 0 {
        let len = strlen(buf, at) + 1;
        buf.copy_within(at..at + len, at + n);
    }
}

fn strcpy(buf: &mut [u8], at: usize, s: &[u8]) {
    buf[at..at + s.len()].copy_from_slice(s);
    buf[at + s.len()] = 0;
}

/// `__fptostr` (`0x40fef9`): a `0` and then `digits` digits of the mantissa, `0` once it runs out;
/// for `digits` 0 or more, the next mantissa byte, when `5` or above, adds one with nines carrying.
/// A carry into the leading `0` adds one to `decpt`; otherwise the leading `0` goes.
pub fn fptostr(buf: &mut [u8], at: usize, digits: i32, flt: &mut StrFlt) {
    let man = &flt.mantissa;
    let mut m = 0usize;
    buf[at] = b'0';
    let mut p = at + 1;
    let mut round = digits >= 0;
    if digits > 0 {
        round = true;
        for _ in 0..digits {
            let c = man.get(m).copied().unwrap_or(0);
            buf[p] = if c != 0 {
                m += 1;
                c
            } else {
                b'0'
            };
            p += 1;
        }
    }
    buf[p] = 0;
    if round && (man.get(m).copied().unwrap_or(0) as i8) >= 0x35 {
        p -= 1;
        while buf[p] == b'9' {
            buf[p] = b'0';
            p -= 1;
        }
        buf[p] += 1;
    }
    if buf[at] == b'1' {
        flt.decpt += 1;
    } else {
        let len = strlen(buf, at + 1) + 1;
        buf.copy_within(at + 1..at + 1 + len, at);
    }
}

fn neg(flt: &StrFlt) -> usize {
    usize::from(flt.sign == b'-')
}

/// `__cftoe2` (`0x40b5ad`): the digits at `at` into `d.ddd` with `ndec` after the point (for
/// `%g`, `g_fmt`, the digits are first shifted up one for the point and `ndec - 1` follow it), and
/// the exponent from the template `e+000` (`0x8001e0`): `E` with `caps`, the decimal exponent less
/// one in three digits, `+000` when the first digit is `0`.
pub fn cftoe2(buf: &mut [u8], at: usize, ndec: i32, caps: bool, flt: &StrFlt, g_fmt: bool) {
    if g_fmt {
        shift(buf, at + neg(flt), usize::from(ndec > 0));
    }
    let mut p = at;
    if flt.sign == b'-' {
        buf[p] = b'-';
        p += 1;
    }
    if ndec > 0 {
        buf[p] = buf[p + 1];
        p += 1;
        buf[p] = POINT;
    }
    let mut q = (p as isize + ndec as isize + isize::from(!g_fmt)) as usize;
    strcpy(buf, q, b"e+000");
    if caps {
        buf[q] = b'E';
    }
    q += 1;
    if flt.mantissa.first().copied().unwrap_or(0) != b'0' {
        let mut e = flt.decpt - 1;
        if e < 0 {
            e = -e;
            buf[q] = b'-';
        }
        q += 1;
        if e >= 100 {
            buf[q] = buf[q].wrapping_add((e / 100) as u8);
            e %= 100;
        }
        q += 1;
        if e >= 10 {
            buf[q] = buf[q].wrapping_add((e / 10) as u8);
            e %= 10;
        }
        buf[q + 1] = buf[q + 1].wrapping_add(e as u8);
    }
}

/// `__cftoe` (`0x40b54c`): `_fltout2`, `ndec + 1` digits by `_fptostr` after room for the sign and
/// the point, then `_cftoe2`.
pub fn cftoe(bits: u64, buf: &mut [u8], at: usize, ndec: i32, caps: bool, t: &TenPowers) {
    let mut flt = fltout2(bits, t);
    let start = at + neg(&flt) + usize::from(ndec > 0);
    fptostr(buf, start, ndec.wrapping_add(1), &mut flt);
    cftoe2(buf, at, ndec, caps, &flt, false);
}

/// `__cftof2` (`0x40b6c4`): the digits at `at` with the point after `decpt` of them, a `0` before
/// it when `decpt` is 0 or less, and after it zeros up to the first digit, all of them for `%g`
/// and at most `ndec` for `%f`. For `%g` a `decpt - 1` equal to `ndec` first puts a `0` and the
/// end at that place.
pub fn cftof2(buf: &mut [u8], at: usize, mut ndec: i32, flt: &StrFlt, g_fmt: bool) {
    let e = flt.decpt - 1;
    if g_fmt && e == ndec {
        let q = (at as isize + neg(flt) as isize + e as isize) as usize;
        buf[q] = b'0';
        buf[q + 1] = 0;
    }
    let mut p = at;
    if flt.sign == b'-' {
        buf[p] = b'-';
        p += 1;
    }
    if flt.decpt <= 0 {
        shift(buf, p, 1);
        buf[p] = b'0';
        p += 1;
    } else {
        p += flt.decpt as usize;
    }
    if ndec > 0 {
        shift(buf, p, 1);
        buf[p] = POINT;
        p += 1;
        let decpt = flt.decpt;
        if decpt < 0 {
            if g_fmt || ndec >= -decpt {
                ndec = -decpt;
            }
            shift(buf, p, ndec as usize);
            buf[p..p + ndec as usize].fill(b'0');
        }
    }
}

/// `__cftof` (`0x40b66f`): `_fltout2`, `decpt + ndec` digits by `_fptostr` after room for the sign,
/// then `_cftof2`.
pub fn cftof(bits: u64, buf: &mut [u8], at: usize, ndec: i32, t: &TenPowers) {
    let mut flt = fltout2(bits, t);
    let digits = flt.decpt.wrapping_add(ndec);
    fptostr(buf, at + neg(&flt), digits, &mut flt);
    cftof2(buf, at, ndec, &flt, false);
}

/// `__cftog` (`0x40b76b`): `ndec` significant digits by `_fptostr`, then by the decimal exponent
/// after that rounding the `e` form (below -4, or `ndec` or more) or the `f` form, from which the
/// last digit goes when the rounding carried into a new one.
pub fn cftog(bits: u64, buf: &mut [u8], at: usize, ndec: i32, caps: bool, t: &TenPowers) {
    let mut flt = fltout2(bits, t);
    let before = flt.decpt - 1;
    let start = at + neg(&flt);
    fptostr(buf, start, ndec, &mut flt);
    let after = flt.decpt - 1;
    if after < -4 || after >= ndec {
        cftoe2(buf, at, ndec, caps, &flt, true);
    } else {
        if before < after {
            let end = start + strlen(buf, start);
            buf[end - 1] = 0;
        }
        cftof2(buf, at, ndec, &flt, true);
    }
}

/// `__cfltcvt` (`0x40b7fe`): `e` or `E` to `_cftoe`, `f` to `_cftof`, anything else to `_cftog`.
pub fn cfltcvt(
    bits: u64,
    buf: &mut [u8],
    at: usize,
    format: u8,
    precision: i32,
    caps: bool,
    t: &TenPowers,
) {
    match format {
        b'e' | b'E' => cftoe(bits, buf, at, precision, caps, t),
        b'f' => cftof(bits, buf, at, precision, t),
        _ => cftog(bits, buf, at, precision, caps, t),
    }
}

/// `_cropzeros` (`0x40b4a8`): past the decimal point, the zeros before an `e`, `E` or the end go,
/// and the point with them when no digit is left after it.
pub fn cropzeros(buf: &mut [u8], at: usize) {
    let mut p = at;
    while buf[p] != 0 && buf[p] != POINT {
        p += 1;
    }
    if buf[p] == 0 {
        return;
    }
    p += 1;
    while buf[p] != 0 && buf[p] != b'e' && buf[p] != b'E' {
        p += 1;
    }
    let mut stop = p;
    let mut q = p - 1;
    while buf[q] == b'0' {
        q -= 1;
    }
    if buf[q] == POINT {
        q -= 1;
    }
    loop {
        let c = buf[stop];
        q += 1;
        stop += 1;
        buf[q] = c;
        if c == 0 {
            break;
        }
    }
}

/// `__forcdecpt` (`0x40b44e`): a decimal point put before the first byte past the leading one that
/// is not a digit, or before the string when it starts with `e` or `E`.
pub fn forcdecpt(buf: &mut [u8], at: usize) {
    let mut p = at;
    if !buf[p].eq_ignore_ascii_case(&b'e') {
        loop {
            p += 1;
            if !buf[p].is_ascii_digit() {
                break;
            }
        }
    }
    let mut hold = buf[p];
    buf[p] = POINT;
    loop {
        p += 1;
        std::mem::swap(&mut buf[p], &mut hold);
        if buf[p] == 0 {
            break;
        }
    }
}
