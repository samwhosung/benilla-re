//! `__output` (`0x742ba5`, `systems/crt/printf.md`), the C runtime's `printf` engine, for its narrow
//! conversions, and the two string front ends `sprintf` and `_vsnprintf`.
//!
//! The arguments are the dwords the caller pushed, read in order as each conversion asks; a string
//! argument is an address read through [`Memory`]. The wide conversions (`%C`, `%S`, `%Z` and `l`
//! or `w` before `c` or `s`) go through `_wctomb` and are not modelled: they panic.

use crate::cvt::{cfltcvt, cropzeros, forcdecpt};
use crate::ld12::TenPowers;

/// Bytes at addresses, for `%s`.
pub trait Memory {
    fn byte(&self, address: u32) -> u8;
}

/// A store `%n` makes: the address, the count, and whether `h` made it two bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Store {
    pub address: u32,
    pub value: u32,
    pub short: bool,
}

/// What `__output` did: the bytes it wrote, its return (the count, or -1 once a write failed) and
/// the `%n` stores in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub text: Vec<u8>,
    pub ret: i32,
    pub stores: Vec<Store>,
}

/// The character classes of the table at `0x8161e0` (the low nibble of the byte at `c - 0x20`, for
/// `c` from `0x20` to `0x78`; everything else is class 0).
fn class(c: u8) -> usize {
    match c {
        b'%' => 1,
        b'.' => 2,
        b'*' => 3,
        b'0' => 4,
        b'1'..=b'9' => 5,
        b' ' | b'#' | b'+' | b'-' => 6,
        b'F' | b'I' | b'L' | b'N' | b'h' | b'l' | b'w' => 7,
        b'B' | b'C' | b'E' | b'G' | b'S' | b'X' | b'Z' | b'c' | b'd' | b'e' | b'f' | b'g'
        | b'i' | b'n' | b'o' | b'p' | b's' | b'u' | b'x' => 8,
        _ => 0,
    }
}

/// The next state by class and state (the high nibbles of the same table at `class * 8 + state`).
/// States: 0 normal, 1 percent, 2 flag, 3 width, 4 dot, 5 precision, 6 size, 7 type.
const NEXT: [[u8; 8]; 9] = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [0, 4, 4, 4, 0, 0, 0, 0],
    [0, 3, 3, 0, 5, 0, 0, 0],
    [0, 2, 2, 3, 5, 5, 0, 0],
    [0, 3, 3, 3, 5, 5, 0, 0],
    [0, 2, 2, 0, 0, 0, 0, 0],
    [0, 6, 6, 6, 6, 6, 6, 0],
    [0, 7, 7, 7, 7, 7, 7, 0],
];

const PLUS: u32 = 0x1;
const SPACE: u32 = 0x2;
const LEFT: u32 = 0x4;
const ZERO: u32 = 0x8;
const LONG: u32 = 0x10;
const SHORT: u32 = 0x20;
const SIGNED: u32 = 0x40;
const ALT: u32 = 0x80;
const NEG: u32 = 0x100;
const FORCE_OCTAL: u32 = 0x200;
const WIDE: u32 = 0x800;
const INT64: u32 = 0x8000;

struct Sink {
    text: Vec<u8>,
    /// The string `FILE`'s count, decremented by every write: a write that takes it below zero
    /// fails, as 32-bit arithmetic decides.
    room: i32,
    count: i32,
}

impl Sink {
    fn put(&mut self, c: u8) {
        self.room = self.room.wrapping_sub(1);
        if self.room < 0 {
            self.count = -1;
        } else {
            self.text.push(c);
            self.count += 1;
        }
    }

    fn multi(&mut self, c: u8, n: i32) {
        for _ in 0..n.max(0) {
            self.put(c);
            if self.count == -1 {
                break;
            }
        }
    }

    fn string(&mut self, s: &[u8], n: i32) {
        for &c in s.iter().take(n.max(0) as usize) {
            self.put(c);
            if self.count == -1 {
                break;
            }
        }
    }
}

/// `__output` writing into a string `FILE` whose count is `room` (no NUL): the text, the return
/// and the `%n` stores, and the count it leaves.
pub fn output(
    fmt: &[u8],
    args: &[u32],
    mem: &dyn Memory,
    room: i32,
    t: &TenPowers,
) -> (Output, i32) {
    let mut sink = Sink {
        text: Vec::new(),
        room,
        count: 0,
    };
    let mut stores = Vec::new();
    let mut next_arg = 0usize;
    let mut arg = || {
        let v = args[next_arg];
        next_arg += 1;
        v
    };
    let mut state = 0usize;
    let (mut flags, mut width, mut precision) = (0u32, 0i32, -1i32);
    let (mut capexp, mut no_output, mut prefix_len) = (false, false, 0i32);
    let mut prefix = [0u8; 2];
    // The text a conversion leaves, kept across conversions: a type with no case writes it again.
    let mut text: Vec<u8> = Vec::new();
    let mut text_len = 0i32;
    let mut i = 0usize;
    while i < fmt.len() && fmt[i] != 0 {
        if sink.count < 0 {
            break;
        }
        let c = fmt[i];
        i += 1;
        let cls = if (0x20..=0x78).contains(&c) {
            class(c)
        } else {
            0
        };
        state = NEXT[cls][state] as usize;
        match state {
            1 => {
                precision = -1;
                capexp = false;
                no_output = false;
                width = 0;
                prefix_len = 0;
                flags = 0;
            }
            2 => match c {
                b' ' => flags |= SPACE,
                b'#' => flags |= ALT,
                b'+' => flags |= PLUS,
                b'-' => flags |= LEFT,
                b'0' => flags |= ZERO,
                _ => {}
            },
            3 => {
                if c == b'*' {
                    width = arg() as i32;
                    if width < 0 {
                        flags |= LEFT;
                        width = width.wrapping_neg();
                    }
                } else {
                    width = width.wrapping_mul(10).wrapping_add(i32::from(c) - 0x30);
                }
            }
            4 => precision = 0,
            5 => {
                if c == b'*' {
                    precision = arg() as i32;
                    if precision < 0 {
                        precision = -1;
                    }
                } else {
                    precision = precision.wrapping_mul(10).wrapping_add(i32::from(c) - 0x30);
                }
            }
            6 => match c {
                b'I' => {
                    if fmt.get(i) == Some(&b'6') && fmt.get(i + 1) == Some(&b'4') {
                        i += 2;
                        flags |= INT64;
                    } else {
                        state = 0;
                        sink.put(c);
                    }
                }
                b'h' => flags |= SHORT,
                b'l' => flags |= LONG,
                b'w' => flags |= WIDE,
                _ => {}
            },
            7 => {
                let mut ch = c;
                let mut radix = 0u64;
                let mut hexadd = 0u8;
                let mut integer = false;
                if (ch == b'C' || ch == b'S') && flags & (LONG | SHORT | WIDE) == 0 {
                    flags |= WIDE;
                }
                match ch {
                    b'Z' => panic!("counted strings are not modelled"),
                    b'c' | b'C' => {
                        assert!(
                            flags & (LONG | WIDE) == 0,
                            "wide characters are not modelled"
                        );
                        text = vec![arg() as u8];
                        text_len = 1;
                    }
                    b's' | b'S' => {
                        assert!(flags & (LONG | WIDE) == 0, "wide strings are not modelled");
                        let mut limit = if precision == -1 {
                            0x7fff_ffff
                        } else {
                            precision as u32
                        };
                        let p = arg();
                        let mut s = Vec::new();
                        if p == 0 {
                            s.extend_from_slice(b"(null)");
                            s.truncate(limit.min(6) as usize);
                        } else {
                            let mut a = p;
                            while limit != 0 {
                                limit -= 1;
                                let b = mem.byte(a);
                                if b == 0 {
                                    break;
                                }
                                s.push(b);
                                a = a.wrapping_add(1);
                            }
                        }
                        text_len = s.len() as i32;
                        text = s;
                    }
                    b'e' | b'f' | b'g' | b'E' | b'G' => {
                        if ch == b'E' || ch == b'G' {
                            capexp = true;
                            ch += 0x20;
                        }
                        flags |= SIGNED;
                        if precision < 0 {
                            precision = 6;
                        } else if precision == 0 && ch == b'g' {
                            precision = 1;
                        }
                        let lo = u64::from(arg());
                        let hi = u64::from(arg());
                        let mut buf = vec![0u8; 0x400];
                        cfltcvt(hi << 32 | lo, &mut buf, 0, ch, precision, capexp, t);
                        if flags & ALT != 0 && precision == 0 {
                            forcdecpt(&mut buf, 0);
                        }
                        if ch == b'g' && flags & ALT == 0 {
                            cropzeros(&mut buf, 0);
                        }
                        let mut s = &buf[..];
                        if s[0] == b'-' {
                            flags |= NEG;
                            s = &s[1..];
                        }
                        let n = s.iter().position(|&b| b == 0).expect("terminated");
                        text = s[..n].to_vec();
                        text_len = n as i32;
                    }
                    b'd' | b'i' => {
                        flags |= SIGNED;
                        radix = 10;
                        integer = true;
                    }
                    b'u' => {
                        radix = 10;
                        integer = true;
                    }
                    b'o' => {
                        radix = 8;
                        if flags & ALT != 0 {
                            flags |= FORCE_OCTAL;
                        }
                        integer = true;
                    }
                    b'x' | b'X' | b'p' => {
                        if ch == b'p' {
                            precision = 8;
                        }
                        hexadd = if ch == b'x' { 0x27 } else { 7 };
                        radix = 16;
                        if flags & ALT != 0 {
                            prefix = [b'0', b'Q' + hexadd];
                            prefix_len = 2;
                        }
                        integer = true;
                    }
                    b'n' => {
                        let address = arg();
                        stores.push(Store {
                            address,
                            value: sink.count as u32,
                            short: flags & SHORT != 0,
                        });
                        no_output = true;
                    }
                    // A type with no case of its own (`B`): the last conversion's text again.
                    _ => {}
                }
                if integer {
                    let mut v: u64 = if flags & INT64 != 0 {
                        let lo = u64::from(arg());
                        lo | u64::from(arg()) << 32
                    } else if flags & SHORT != 0 {
                        let w = arg() as u16;
                        if flags & SIGNED != 0 {
                            i64::from(w as i16) as u64
                        } else {
                            u64::from(w)
                        }
                    } else {
                        let d = arg();
                        if flags & SIGNED != 0 {
                            i64::from(d as i32) as u64
                        } else {
                            u64::from(d)
                        }
                    };
                    if flags & SIGNED != 0 && (v as i64) < 0 {
                        v = (v as i64).wrapping_neg() as u64;
                        flags |= NEG;
                    }
                    if flags & INT64 == 0 {
                        v &= 0xffff_ffff;
                    }
                    if precision < 0 {
                        precision = 1;
                    } else {
                        flags &= !ZERO;
                    }
                    if v == 0 {
                        prefix_len = 0;
                    }
                    let mut digits = Vec::new();
                    loop {
                        let more = precision > 0;
                        precision -= 1;
                        if !more && v == 0 {
                            break;
                        }
                        let mut d = (v % radix) as u8 + b'0';
                        v /= radix;
                        if d > b'9' {
                            d += hexadd;
                        }
                        digits.push(d);
                    }
                    digits.reverse();
                    if flags & FORCE_OCTAL != 0
                        && (digits.first() != Some(&b'0') || digits.is_empty())
                    {
                        digits.insert(0, b'0');
                    }
                    text_len = digits.len() as i32;
                    text = digits;
                }
                if !no_output {
                    if flags & SIGNED != 0 {
                        if flags & NEG != 0 {
                            prefix[0] = b'-';
                            prefix_len = 1;
                        } else if flags & PLUS != 0 {
                            prefix[0] = b'+';
                            prefix_len = 1;
                        } else if flags & SPACE != 0 {
                            prefix[0] = b' ';
                            prefix_len = 1;
                        }
                    }
                    let pad = width.wrapping_sub(prefix_len).wrapping_sub(text_len);
                    if flags & (LEFT | ZERO) == 0 {
                        sink.multi(b' ', pad);
                    }
                    sink.string(&prefix, prefix_len);
                    if flags & ZERO != 0 && flags & LEFT == 0 {
                        sink.multi(b'0', pad);
                    }
                    sink.string(&text, text_len);
                    if flags & LEFT != 0 {
                        sink.multi(b' ', pad);
                    }
                }
            }
            _ => sink.put(c),
        }
    }
    let room = sink.room;
    (
        Output {
            text: sink.text,
            ret: sink.count,
            stores,
        },
        room,
    )
}

/// `sprintf` (`0x73fec4`): `__output` with room 0x7fffffff, then a NUL.
pub fn sprintf(fmt: &[u8], args: &[u32], mem: &dyn Memory, t: &TenPowers) -> Output {
    let (mut o, _) = output(fmt, args, mem, 0x7fff_ffff, t);
    o.text.push(0);
    o
}

/// `_vsnprintf` (`0x73f6ad`): `__output` with the count `count`, then a NUL when one more
/// decrement leaves it at zero or above: the text and a NUL when shorter, the text alone when it
/// is exactly `count`, `count` bytes and -1 when longer.
pub fn vsnprintf(count: u32, fmt: &[u8], args: &[u32], mem: &dyn Memory, t: &TenPowers) -> Output {
    let (mut o, room) = output(fmt, args, mem, count as i32, t);
    if room.wrapping_sub(1) >= 0 {
        o.text.push(0);
    }
    o
}
