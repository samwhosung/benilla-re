//! Storm's string-to-number conversions, `SStrToFloat` (`0x64aaa0`), `SStrToInt` (`0x64ac60`) and
//! `SStrToUnsigned` (`0x64ade0`), and the power table `0x64a680` fills for the float's fraction
//! digits (`systems/storm/strings.md`).
//!
//! The float conversion runs at the client's 53-bit x87 precision, so every multiply and add here
//! is one `f64` operation. Its powers of ten come from the C runtime's `pow` (`0x73f940`), whose
//! `fyl2x` and `f2xm1` differ by CPU: the caller supplies `pow10(n)` for `pow(10.0, n)`, and the
//! table is built from the same function. The transcription holds while every step stays a normal
//! double; past that the x87's wider exponent keeps values a double cannot.

/// The table `0x64a680` writes at `0xc52560`: row `r` (0 to 19), column `k` (0 to 9), holding
/// `k · pow(10, −(r + 1))` rounded to a double (`fild k; fmul st,st(1); fstp qword`).
pub fn pow10_table(pow10: impl Fn(i32) -> f64) -> [[f64; 10]; 20] {
    let mut table = [[0.0; 10]; 20];
    for (r, row) in table.iter_mut().enumerate() {
        let p = pow10(-(r as i32) - 1);
        for (k, cell) in row.iter_mut().enumerate() {
            *cell = k as f64 * p;
        }
    }
    table
}

/// The byte at `i` of a C string: 0 at and past its end or its first NUL.
fn byte(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

/// The digit at `i`: the byte sign-extended minus `'0'`, kept when it is below 10 unsigned.
fn digit(s: &[u8], i: usize) -> Option<u32> {
    let d = (i32::from(byte(s, i) as i8) - 0x30) as u32;
    (d < 10).then_some(d)
}

/// `SStrToInt` (`0x64ac60`) of a non-null string: one optional `-`, then decimal digits in
/// wrapping 32-bit arithmetic, negated after a `-`; 0 when no digit follows.
pub fn str_to_int(s: &[u8]) -> i32 {
    let s = &s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())];
    let neg = byte(s, 0) == b'-';
    let mut i = usize::from(neg);
    let mut v: u32 = 0;
    while let Some(d) = digit(s, i) {
        v = v.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    let v = v as i32;
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

/// `SStrToUnsigned` (`0x64ade0`) of a non-null string: decimal digits only, in wrapping 32-bit
/// arithmetic; 0 when the first byte is not a digit. No sign is read, so `-5` gives 0.
pub fn str_to_unsigned(s: &[u8]) -> u32 {
    let s = &s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())];
    let mut i = 0;
    let mut v: u32 = 0;
    while let Some(d) = digit(s, i) {
        v = v.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    v
}

/// `SStrToFloat` (`0x64aaa0`) of a non-null string, as the double the x87 holds in `st0`.
///
/// One optional `-`, no space and no `+`. The integer digits gather in a `u32` chunk; when a chunk
/// reaches `0x19999999` it is folded into the double `acc = pow10(n)·acc + chunk`, `n` the chunk's
/// digit count, and the last chunk is folded the same way unless `acc` is 0. A `.` then adds each
/// fraction digit `d` at place `k` (−1 down) as `table[−k − 1][d]` while `k > −21`, else as
/// `pow10(k)·d`. A `.` without a digit after it is still passed. An `e` or `E`, an optional `+`, and
/// `SStrToInt` of the rest give `e`, and the value becomes `pow10(e)·value`. A `-` negates last.
pub fn str_to_float(s: &[u8], table: &[[f64; 10]; 20], pow10: impl Fn(i32) -> f64) -> f64 {
    let s = &s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())];
    let neg = byte(s, 0) == b'-';
    let mut i = usize::from(neg);
    let mut v = match digit(s, i) {
        None => 0.0,
        Some(first) => {
            let mut chunk = first;
            let mut start = i;
            i += 1;
            match digit(s, i) {
                None => f64::from(chunk),
                Some(mut d) => {
                    let mut acc = 0.0f64;
                    loop {
                        chunk = chunk * 10 + d;
                        i += 1;
                        if chunk >= 0x1999_9999 {
                            acc = pow10((i - start) as i32) * acc + f64::from(chunk);
                            chunk = 0;
                            start = i;
                        }
                        match digit(s, i) {
                            Some(next) => d = next,
                            None => break,
                        }
                    }
                    if acc == 0.0 {
                        f64::from(chunk)
                    } else {
                        pow10((i - start) as i32) * acc + f64::from(chunk)
                    }
                }
            }
        }
    };
    if byte(s, i) == b'.' {
        i += 1;
        if let Some(mut d) = digit(s, i) {
            let mut k: i32 = -1;
            loop {
                i += 1;
                v = if k > -21 {
                    v + table[(-k - 1) as usize][d as usize]
                } else {
                    pow10(k) * f64::from(d) + v
                };
                k -= 1;
                match digit(s, i) {
                    Some(next) => d = next,
                    None => break,
                }
            }
        }
    }
    if matches!(byte(s, i), b'e' | b'E') {
        i += 1;
        if byte(s, i) == b'+' {
            i += 1;
        }
        let e = str_to_int(&s[i.min(s.len())..]);
        v *= pow10(e);
    }
    if neg {
        -v
    } else {
        v
    }
}
