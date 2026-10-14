//! Two leaves of object files not yet identified: the integer square root `0x454a10` and the
//! normalization of a vector of two `0x456280`.

/// `0x454a10` (`ecx` = `n`, the root in `eax`): the integer square root by Newton's iteration.
/// The first estimate is `n·0xaaaaaaab >> 35` plus 1 for `n <= 0xff`, `n·0x51eb851f >> 38` plus 21
/// for `n <= 0xffff`, and otherwise `((n − t)/2 + t) >> 14` plus `0x1bc` with `t = n·0x39acc69d
/// >> 32`. Each step is `q = n / e`, `d = e − q` and `e = (q + e) / 2`, both halves truncating
/// toward zero, and it stops when `d / 2` is 0.
#[must_use]
pub fn isqrt(n: u32) -> u32 {
    let mut esi: u32 = if n <= 0xff {
        let hi = (u64::from(n) * 0xaaaa_aaab) >> 32;
        ((hi >> 3) as u32).wrapping_add(1)
    } else if n <= 0xffff {
        let hi = (u64::from(n) * 0x51eb_851f) >> 32;
        ((hi >> 6) as u32).wrapping_add(0x15)
    } else {
        let t = ((u64::from(n) * 0x39ac_c69d) >> 32) as u32;
        let mut s = n.wrapping_sub(t);
        s >>= 1;
        s = s.wrapping_add(t);
        s >>= 0xe;
        s.wrapping_add(0x1bc)
    };
    loop {
        let q = n / esi;
        let diff = (esi as i32).wrapping_sub(q as i32);
        let edi = diff.wrapping_sub(diff >> 31) >> 1;
        let sum = (q as i32).wrapping_add(esi as i32);
        esi = (sum.wrapping_sub(sum >> 31) >> 1) as u32;
        if edi == 0 {
            break;
        }
    }
    esi
}

/// `0x456280` (`ecx` = `&x`, `edx` = `&y`, both changed in place): `r = 1/√(x·x + y·y)` at 53
/// bits (`1.0` is the `f32` at `0x7ff9d8`), then `x·r` and `y·r` stored as `f32`. There is no zero
/// test.
#[must_use]
pub fn normalize2_f32(x: f32, y: f32) -> [f32; 2] {
    let (xf, yf) = (f64::from(x), f64::from(y));
    let r = 1.0_f64 / (xf * xf + yf * yf).sqrt();
    [(xf * r) as f32, (yf * r) as f32]
}
