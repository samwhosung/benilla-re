//! Storm's UTF-8 to UTF-16 conversion (`SUniConvertUTF8to16`, `0x65b350`) and the length count
//! that sizes its buffers (`0x65b250`). Neither validates: a lead byte's table entry says how many
//! bytes follow, and those are folded in whatever they hold.

/// The bytes that follow a lead byte, the table at `0x80e780`: 0 for `0x00` to `0xbf`, then 1 to
/// 5 for the runs from `0xc0`, `0xe0`, `0xf0`, `0xf8` and `0xfc`.
fn trailing(lead: u8) -> usize {
    match lead {
        0x00..=0xbf => 0,
        0xc0..=0xdf => 1,
        0xe0..=0xef => 2,
        0xf0..=0xf7 => 3,
        0xf8..=0xfb => 4,
        0xfc..=0xff => 5,
    }
}

/// What is subtracted after folding a sequence of `n + 1` bytes, the table at `0x80e764`.
const OFFSETS: [u32; 6] = [0, 0x3080, 0xe2080, 0x3c8_2080, 0xfa08_2080, 0x8208_2080];

/// One sequence folded as `0x65b350` folds it: each byte zero-extended, `v = (v << 6) + byte`
/// modulo 2^32, then the table's offset subtracted. `signed` folds the bytes sign-extended, as
/// `0x65b250` does.
fn fold(bytes: &[u8], signed: bool) -> u32 {
    let mut v: u32 = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 {
            v = v.wrapping_shl(6);
        }
        let b = if signed {
            b as i8 as i32 as u32
        } else {
            u32::from(b)
        };
        v = v.wrapping_add(b);
    }
    v.wrapping_sub(OFFSETS[bytes.len() - 1])
}

/// The result of [`utf8_to_utf16`]: the return value, the units written and the bytes read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Utf16Conversion {
    /// 0 when a NUL was written, 1 when the destination filled, -1 when the source ran out (or
    /// was null or empty), and `-(n + 1)` when a sequence of `n + 1` bytes would run past it.
    pub result: i32,
    /// The units stored, the NUL included; `None` for a null or empty source, which stores none.
    pub written: Option<u32>,
    /// The bytes consumed, the NUL's sequence not included.
    pub read: Option<u32>,
    /// The destination as left, `cap` units.
    pub dst: Vec<u16>,
}

/// `SUniConvertUTF8to16(dst, cap, src, len, &written, &read)` as `0x65b350` runs it, over a
/// destination of `cap` units that starts as `dst`, `src` standing for a non-null source whose
/// bytes past its slice read as 0. A folded value up to 0xffff is stored as one unit, surrogates
/// and overlong forms included, and a stored 0 ends the conversion; above 0x10ffff it stores
/// 0xfffd; otherwise a surrogate pair, `0xd800 + (v - 0x10000 >> 10)` and `0xdc00 + (v & 0x3ff)`
/// after the subtraction, when both units fit. A unit that does not fit stores nothing.
pub fn utf8_to_utf16(dst: &[u16], src: Option<&[u8]>, len: u32) -> Utf16Conversion {
    let mut out = dst.to_vec();
    let Some(src) = src.filter(|_| len != 0) else {
        return Utf16Conversion {
            result: -1,
            written: None,
            read: None,
            dst: out,
        };
    };
    let at = |i: usize| src.get(i).copied().unwrap_or(0);
    let end = if len == 0x7fff_ffff {
        usize::MAX
    } else {
        len as usize
    };
    let cap = dst.len();
    let (mut i, mut d) = (0usize, 0usize);
    let result = loop {
        if i >= end {
            break -1;
        }
        let n = trailing(at(i));
        if i + n >= end {
            break -1 - n as i32;
        }
        let bytes: Vec<u8> = (0..=n).map(|k| at(i + k)).collect();
        let v = fold(&bytes, false);
        if d >= cap {
            break 1;
        }
        if v <= 0xffff {
            out[d] = v as u16;
            d += 1;
            if v == 0 {
                break 0;
            }
        } else if v > 0x10_ffff {
            out[d] = 0xfffd;
            d += 1;
        } else {
            if d + 1 >= cap {
                break 1;
            }
            let w = v - 0x1_0000;
            out[d] = ((w >> 10) as u16).wrapping_add(0xd800);
            out[d + 1] = ((w & 0x3ff) as u16).wrapping_add(0xdc00);
            d += 2;
        }
        i += n + 1;
    };
    Utf16Conversion {
        result,
        written: Some(d as u32),
        read: Some(i as u32),
        dst: out,
    }
}

/// `0x65b250(src, len, &read)`: the units [`utf8_to_utf16`] would need, the NUL included, as
/// counted with each byte sign-extended. So every sequence whose lead byte is below 0xfc folds to
/// a value above 0x10ffff and counts 1, a four-byte form included, where the conversion stores two;
/// the count is the NUL-included count of code points for valid UTF-8. A count stops at the NUL; a
/// source that runs out gives -1, a sequence past its end `-(n + 1)`, and a null or empty source
/// -1 with `read` 0.
pub fn utf8_to_utf16_len(src: Option<&[u8]>, len: u32) -> (i32, u32) {
    let Some(src) = src.filter(|_| len != 0) else {
        return (-1, 0);
    };
    let at = |i: usize| src.get(i).copied().unwrap_or(0);
    let end = if len == 0x7fff_ffff {
        usize::MAX
    } else {
        len as usize
    };
    let (mut i, mut count) = (0usize, 0i32);
    loop {
        if i >= end {
            return (-1, i as u32);
        }
        let n = trailing(at(i));
        if i + n >= end {
            return (-1 - n as i32, i as u32);
        }
        let bytes: Vec<u8> = (0..=n).map(|k| at(i + k)).collect();
        let v = fold(&bytes, true);
        i += n + 1;
        if v <= 0xffff {
            count += 1;
            if v == 0 {
                return (count, i as u32);
            }
        } else if v > 0x10_ffff {
            count += 1;
        } else {
            count += 2;
        }
    }
}
