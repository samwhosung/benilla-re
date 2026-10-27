//! `sscanf` and its engine `__input` in the C locale (`systems/crt/scanf.md`): `_sscanf`
//! (`0x73fb5d`) makes a string `FILE` and runs `__input` (`0x744e21`) over it.
//!
//! The port follows the exe's order of reads and pushbacks, so the stream is modelled as the `FILE`
//! is: a pointer, a count that falls below 0 at the end, and a pushback that succeeds only when the
//! byte is the one before the pointer. A conversion writes through the next argument; the caller
//! gives each argument as a block of bytes. The decimal point is `.`, no byte is a lead byte, and
//! `__mb_cur_max` is 1.

use crate::ctype::ctype;
use crate::ld12::TenPowers;
use crate::strtod::{ld12cvt, strgtold12, DOUBLE, FLOAT};

const EOF: i32 = -1;

/// `_fassign(is_double, out, text)` (`0x40b50e`): `___strgtold12` over the text with no scale, then
/// `__ld12cvt` into a double's eight bytes, or a float's four.
pub fn fassign(is_double: bool, text: &[u8], t: &TenPowers) -> Vec<u8> {
    let scan = strgtold12(text, false, 0, 0, false, t);
    if is_double {
        ld12cvt(&scan.ld, &DOUBLE).0.to_le_bytes().to_vec()
    } else {
        (ld12cvt(&scan.ld, &FLOAT).0 as u32).to_le_bytes().to_vec()
    }
}

/// The string `FILE` `_sscanf` builds (`0x73fb66` to `0x73fb79`): the pointer and base at the
/// string, the count its length, the flags 0x49.
struct Stream<'a> {
    s: &'a [u8],
    ptr: usize,
    cnt: i32,
}

impl Stream<'_> {
    fn byte(&self, i: usize) -> u8 {
        self.s.get(i).copied().unwrap_or(0)
    }

    /// `_inc` (`0x74587d`): `--cnt >= 0` gives the next byte; past the end `__filbuf` (`0x745946`)
    /// answers EOF for a string file (flag 0x40) and leaves the count where `_inc` put it.
    fn inc(&mut self) -> i32 {
        self.cnt -= 1;
        if self.cnt >= 0 {
            let c = i32::from(self.byte(self.ptr));
            self.ptr += 1;
            c
        } else {
            EOF
        }
    }

    /// `__un_inc` (`0x745897`) over `_ungetc_lk` (`0x741c1b`): nothing for EOF; at the base a count
    /// other than 0 refuses; otherwise the pointer steps back and keeps the step only when that
    /// byte is the one pushed, raising the count.
    fn un_inc(&mut self, c: i32) {
        if c == EOF {
            return;
        }
        if self.ptr == 0 {
            if self.cnt != 0 {
                return;
            }
            self.ptr += 1;
        }
        self.ptr -= 1;
        if self.byte(self.ptr) == c as u8 {
            self.cnt += 1;
        } else {
            self.ptr += 1;
        }
    }
}

fn isspace(c: i32) -> bool {
    ctype(c) & 8 != 0
}

fn isdigit(c: i32) -> bool {
    ctype(c) & 4 != 0
}

fn isxdigit(c: i32) -> bool {
    ctype(c) & 0x80 != 0
}

/// `__hextodec` (`0x745846`): a digit as it is, a letter as its value plus 0x30.
fn hextodec(c: i32) -> i32 {
    if isdigit(c) {
        c
    } else {
        (c & !0x20) - 7
    }
}

/// `__input` over `input` (read as a C string) and `format`. Each argument is a block of bytes;
/// `slots` are taken in order by the conversions that are not suppressed.
///
/// A format of no bytes reads a local the exe never sets; this returns as if it held a byte. A set
/// the format leaves open, as in `%[a`, makes the exe read on past the format: this panics.
pub fn sscanf(input: &[u8], format: &[u8], slots: &mut [Vec<u8>], t: &TenPowers) -> i32 {
    let len = input.iter().position(|&c| c == 0).unwrap_or(input.len());
    let mut st = Stream {
        s: &input[..len],
        ptr: 0,
        cnt: len as i32,
    };
    Input {
        st: &mut st,
        f: format,
        slots,
        t,
        cc: 0,
    }
    .run()
}

struct Input<'a, 'b> {
    st: &'a mut Stream<'b>,
    f: &'a [u8],
    slots: &'a mut [Vec<u8>],
    t: &'a TenPowers,
    /// `charcount`, the bytes read so far that `%n` stores.
    cc: i32,
}

impl Input<'_, '_> {
    fn fmt(&self, i: usize) -> u8 {
        self.f.get(i).copied().unwrap_or(0)
    }

    /// `__whiteout` (`0x7458ae`): reads while the byte is a space, counting each read.
    fn whiteout(&mut self) -> i32 {
        loop {
            self.cc += 1;
            let c = self.st.inc();
            if !isspace(c) {
                return c;
            }
        }
    }

    fn write(&mut self, slot: usize, at: usize, bytes: &[u8]) {
        if let Some(block) = self.slots.get_mut(slot) {
            if block.len() < at + bytes.len() {
                block.resize(at + bytes.len(), 0);
            }
            block[at..at + bytes.len()].copy_from_slice(bytes);
        }
    }

    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn run(&mut self) -> i32 {
        let mut count: i32 = 0;
        let mut matched: u8 = 0;
        // `[ebp-0x14]` is unset until the first read; a format of no bytes ends before one.
        let mut ch: i32 = 0;
        let mut fi: usize = 0;
        let mut argn: usize = 0;
        if self.fmt(0) == 0 {
            return self.finish(ch, count, matched);
        }
        'main: loop {
            // 0x744e50: blanks in the format.
            if isspace(i32::from(self.fmt(fi))) {
                self.cc -= 1;
                let w = self.whiteout();
                self.st.un_inc(w);
                fi += 1;
                while isspace(i32::from(self.fmt(fi))) {
                    fi += 1;
                }
            }
            if self.fmt(fi) != b'%' {
                // 0x745793: a byte of the format must be the next input byte.
                self.cc += 1;
                ch = self.st.inc();
                let want = i32::from(self.fmt(fi));
                fi += 1;
                if want != ch {
                    self.cc -= 1;
                    self.st.un_inc(ch);
                    return self.finish(ch, count, matched);
                }
                if !self.goes_on(ch, fi) {
                    return self.finish(ch, count, matched);
                }
                continue 'main;
            }
            // 0x744eba: a conversion.
            let mut suppress = false;
            let mut done = false;
            let mut longone: i8 = 1;
            let mut wprefix: i8 = 0;
            let mut neg = false;
            let mut started: i32 = 0;
            let mut widthset: i32 = 0;
            let mut width: i32 = 0;
            let mut int64 = false;
            let mut num64: u64 = 0;
            let mut reject: i8 = 0;
            loop {
                fi += 1;
                let c = i32::from(self.fmt(fi));
                if isdigit(c) {
                    widthset += 1;
                    width = width.wrapping_mul(10).wrapping_add(c - 0x30);
                } else {
                    match c as u8 {
                        b'N' | b'F' => {}
                        b'*' => suppress = true,
                        b'I' => {
                            if self.fmt(fi + 1) == b'6' && self.fmt(fi + 2) == b'4' {
                                int64 = true;
                                num64 = 0;
                                fi += 2;
                            } else {
                                done = true;
                            }
                        }
                        b'L' => longone = longone.wrapping_add(1),
                        b'h' => {
                            longone = longone.wrapping_sub(1);
                            wprefix = wprefix.wrapping_sub(1);
                        }
                        b'l' => {
                            longone = longone.wrapping_add(1);
                            wprefix = wprefix.wrapping_add(1);
                        }
                        b'w' => wprefix = wprefix.wrapping_add(1),
                        _ => done = true,
                    }
                }
                if done {
                    break;
                }
            }
            done = false;
            // 0x744f99: the next argument, and a wide or narrow output for strings.
            let arg = argn;
            let argsave = argn;
            if !suppress {
                argn += 1;
            }
            if wprefix == 0 {
                wprefix = if matches!(self.fmt(fi), b'S' | b'C') {
                    1
                } else {
                    -1
                };
            }
            let ty = self.fmt(fi);
            let lc = i32::from(ty | 0x20);
            // 0x744fd2: the first byte.
            if lc != 0x6e {
                if lc == 0x63 || lc == 0x7b {
                    self.cc += 1;
                    ch = self.st.inc();
                } else {
                    ch = self.whiteout();
                }
            }
            if widthset != 0 && width == 0 {
                self.cc -= 1;
                self.st.un_inc(ch);
                return self.finish(ch, count, matched);
            }
            // The type, as `esi` holds it through the number loops.
            let mut esi = lc;
            // What is left after the conversion that stored something: `Stored(count bump)`.
            enum Next {
                Done,
                Fail,
            }
            let next: Next = 'conv: {
                match lc as u8 {
                    b'n' => {
                        // 0x745055: the characters read so far, through the integer store.
                        if !suppress {
                            self.store_int(arg, int64, longone, self.cc as u32, num64);
                        }
                        break 'conv Next::Done;
                    }
                    b'c' | b's' | b'{' => {
                        let wide = wprefix > 0;
                        let set: Vec<u8>;
                        let mut ei: usize;
                        let mut bits = [0u8; 32];
                        match lc as u8 {
                            b'c' => {
                                if widthset == 0 {
                                    width += 1;
                                    widthset = 1;
                                }
                                set = vec![b']'];
                                ei = 0;
                                reject = -1;
                            }
                            b's' => {
                                set = vec![0x20, 0x09, b'-', 0x0d, b']'];
                                ei = 0;
                                reject = -1;
                            }
                            _ => {
                                fi += 1;
                                let mut s2 = self.f[fi.min(self.f.len())..].to_vec();
                                s2.push(0);
                                ei = 0;
                                if s2[0] == b'^' {
                                    ei = 1;
                                    reject = -1;
                                }
                                set = s2;
                            }
                        }
                        let at = |i: usize, set: &Vec<u8>| set.get(i).copied().unwrap_or(0);
                        let mut last: u8 = 0;
                        if lc == 0x7b && at(ei, &set) == b']' {
                            last = b']';
                            ei += 1;
                            bits[11] = 0x20;
                        }
                        loop {
                            let a = at(ei, &set);
                            if a == b']' {
                                break;
                            }
                            // The exe reads on past the format's NUL until a `]`.
                            assert!(ei <= set.len() + 1, "a set the format leaves open");
                            ei += 1;
                            if a == b'-' && last != 0 && at(ei, &set) != b']' {
                                let cl = at(ei, &set);
                                ei += 1;
                                let (lo, hi) = if last < cl { (last, cl) } else { (cl, last) };
                                for v in lo..=hi {
                                    bits[usize::from(v >> 3)] |= 1 << (v & 7);
                                }
                                last = 0;
                            } else {
                                bits[usize::from(a >> 3)] |= 1 << (a & 7);
                                last = a;
                            }
                        }
                        if at(ei, &set) == 0 {
                            return self.finish(ch, count, matched);
                        }
                        if lc == 0x7b {
                            // The format stops at the closing bracket.
                            fi += ei;
                        }
                        self.cc -= 1;
                        self.st.un_inc(ch);
                        let mut p: usize = 0;
                        let mut taken = 0usize;
                        loop {
                            if widthset != 0 {
                                let e = width;
                                width -= 1;
                                if e == 0 {
                                    break;
                                }
                            }
                            self.cc += 1;
                            ch = self.st.inc();
                            if ch == EOF {
                                self.cc -= 1;
                                break;
                            }
                            let m =
                                (bits[(ch >> 3) as usize] as i8 ^ reject) as i32 & (1 << (ch & 7));
                            if m == 0 {
                                self.cc -= 1;
                                self.st.un_inc(ch);
                                break;
                            }
                            if suppress {
                                taken += 1;
                            } else if wide {
                                let w = (ch as u8 as u16).to_le_bytes();
                                self.write(arg, p, &w);
                                p += 2;
                                taken += 1;
                            } else {
                                self.write(arg, p, &[ch as u8]);
                                p += 1;
                                taken += 1;
                            }
                        }
                        if taken == 0 {
                            break 'conv Next::Fail;
                        }
                        if !suppress {
                            count += 1;
                            if lc != 0x63 {
                                if wide {
                                    self.write(arg, p, &[0, 0]);
                                } else {
                                    self.write(arg, p, &[0]);
                                }
                            }
                        }
                        break 'conv Next::Done;
                    }
                    b'e' | b'f' | b'g' => {
                        let mut fs: Vec<u8> = Vec::new();
                        if ch == 0x2d {
                            fs.push(b'-');
                            width = width.wrapping_sub(1);
                            self.cc += 1;
                            ch = self.st.inc();
                        } else if ch == 0x2b {
                            width = width.wrapping_sub(1);
                            self.cc += 1;
                            ch = self.st.inc();
                        }
                        if widthset == 0 || width > 0x15d {
                            width = 0x15d;
                        }
                        let take = |w: &mut i32| {
                            let e = *w;
                            *w = w.wrapping_sub(1);
                            e != 0
                        };
                        while isdigit(ch) {
                            if !take(&mut width) {
                                break;
                            }
                            started += 1;
                            fs.push(ch as u8);
                            self.cc += 1;
                            ch = self.st.inc();
                        }
                        if (ch as u8) == b'.' && take(&mut width) {
                            self.cc += 1;
                            ch = self.st.inc();
                            fs.push(b'.');
                            while isdigit(ch) {
                                if !take(&mut width) {
                                    break;
                                }
                                started += 1;
                                fs.push(ch as u8);
                                self.cc += 1;
                                ch = self.st.inc();
                            }
                        }
                        if started != 0 && (ch == 0x65 || ch == 0x45) && take(&mut width) {
                            fs.push(b'e');
                            self.cc += 1;
                            ch = self.st.inc();
                            let mut read_digits = true;
                            if ch == 0x2d || ch == 0x2b {
                                if ch == 0x2d {
                                    fs.push(b'-');
                                }
                                if take(&mut width) {
                                    self.cc += 1;
                                    ch = self.st.inc();
                                } else {
                                    width = 0;
                                    read_digits = true;
                                }
                            }
                            if read_digits {
                                while isdigit(ch) {
                                    if !take(&mut width) {
                                        break;
                                    }
                                    started += 1;
                                    fs.push(ch as u8);
                                    self.cc += 1;
                                    ch = self.st.inc();
                                }
                            }
                        }
                        self.cc -= 1;
                        self.st.un_inc(ch);
                        if started == 0 {
                            break 'conv Next::Fail;
                        }
                        if !suppress {
                            count += 1;
                            let bytes = fassign(longone.wrapping_sub(1) != 0, &fs, self.t);
                            self.write(arg, 0, &bytes);
                        }
                        break 'conv Next::Done;
                    }
                    b'd' | b'u' | b'o' | b'x' | b'i' | b'p' => {
                        // 0x745534 and 0x74506a: sign, the 0x prefix of x and i, then digits.
                        if lc == 0x69 {
                            esi = 0x64;
                        }
                        if lc == 0x70 {
                            longone = 1;
                        }
                        let prefix = lc == 0x69 || lc == 0x78;
                        if ch == 0x2d {
                            neg = true;
                        }
                        if ch == 0x2d || ch == 0x2b {
                            let w = width.wrapping_sub(1);
                            width = w;
                            if w == 0 && widthset != 0 {
                                done = true;
                            } else {
                                self.cc += 1;
                                ch = self.st.inc();
                            }
                        }
                        if prefix && ch == 0x30 {
                            self.cc += 1;
                            ch = self.st.inc();
                            if ch == 0x78 || ch == 0x58 {
                                self.cc += 1;
                                ch = self.st.inc();
                                esi = 0x78;
                            } else {
                                started = 1;
                                if esi == 0x78 {
                                    self.cc -= 1;
                                    self.st.un_inc(ch);
                                    ch = 0x30;
                                } else {
                                    esi = 0x6f;
                                }
                            }
                        }
                        let mut number: u32 = 0;
                        if int64 {
                            if !done {
                                loop {
                                    if esi == 0x78 {
                                        if !isxdigit(ch) {
                                            self.cc -= 1;
                                            self.st.un_inc(ch);
                                            break;
                                        }
                                        num64 <<= 4;
                                        ch = hextodec(ch);
                                    } else {
                                        if !isdigit(ch) {
                                            self.cc -= 1;
                                            self.st.un_inc(ch);
                                            break;
                                        }
                                        if esi == 0x6f {
                                            if ch >= 0x38 {
                                                self.cc -= 1;
                                                self.st.un_inc(ch);
                                                break;
                                            }
                                            num64 <<= 3;
                                        } else {
                                            num64 = num64.wrapping_mul(10);
                                        }
                                    }
                                    started += 1;
                                    num64 = num64.wrapping_add((ch - 0x30) as i64 as u64);
                                    if widthset != 0 {
                                        width = width.wrapping_sub(1);
                                        if width == 0 {
                                            break;
                                        }
                                    }
                                    self.cc += 1;
                                    ch = self.st.inc();
                                }
                            }
                            if neg {
                                num64 = num64.wrapping_neg();
                            }
                        } else {
                            if !done {
                                loop {
                                    if esi == 0x78 || esi == 0x70 {
                                        if !isxdigit(ch) {
                                            self.cc -= 1;
                                            self.st.un_inc(ch);
                                            break;
                                        }
                                        number <<= 4;
                                        ch = hextodec(ch);
                                    } else {
                                        if !isdigit(ch) {
                                            self.cc -= 1;
                                            self.st.un_inc(ch);
                                            break;
                                        }
                                        if esi == 0x6f {
                                            if ch >= 0x38 {
                                                self.cc -= 1;
                                                self.st.un_inc(ch);
                                                break;
                                            }
                                            number <<= 3;
                                        } else {
                                            number = number.wrapping_mul(10);
                                        }
                                    }
                                    started += 1;
                                    number = number.wrapping_add((ch - 0x30) as u32);
                                    if widthset != 0 {
                                        width = width.wrapping_sub(1);
                                        if width == 0 {
                                            break;
                                        }
                                    }
                                    self.cc += 1;
                                    ch = self.st.inc();
                                }
                            }
                            if neg {
                                number = number.wrapping_neg();
                            }
                        }
                        if started == 0 {
                            break 'conv Next::Fail;
                        }
                        if !suppress {
                            count += 1;
                            self.store_num(arg, int64, longone, number, num64);
                        }
                        break 'conv Next::Done;
                    }
                    _ => {
                        // 0x7452ac: any other byte must be the next input byte (`%%`).
                        if i32::from(ty) != ch {
                            self.cc -= 1;
                            self.st.un_inc(ch);
                            return self.finish(ch, count, matched);
                        }
                        matched = matched.wrapping_sub(1);
                        if !suppress {
                            argn = argsave;
                        }
                        break 'conv Next::Done;
                    }
                }
            };
            if let Next::Fail = next {
                return self.finish(ch, count, matched);
            }
            // 0x745788: one conversion more.
            matched = matched.wrapping_add(1);
            fi += 1;
            if !self.goes_on(ch, fi) {
                return self.finish(ch, count, matched);
            }
        }
    }

    /// 0x7457d5: after a format byte or a conversion, an input at its end lets only `%n` go on, and
    /// a format at its end stops.
    fn goes_on(&self, ch: i32, fi: usize) -> bool {
        if ch == EOF && !(self.fmt(fi) == b'%' && self.fmt(fi + 1) == b'n') {
            return false;
        }
        self.fmt(fi) != 0
    }

    /// 0x745827: the count, or -1 for an input that ended before anything matched.
    fn finish(&self, ch: i32, count: i32, matched: u8) -> i32 {
        if ch != EOF || count != 0 || matched != 0 {
            count
        } else {
            -1
        }
    }

    /// 0x745762: the integer store of `%n`, which keeps the zero 64-bit value a `I64` prefix left.
    fn store_int(&mut self, arg: usize, int64: bool, longone: i8, v: u32, num64: u64) {
        self.store_num(arg, int64, longone, v, num64);
    }

    /// 0x745762 to 0x745785: eight bytes for `I64`, else four when `longone` is not 0, else two.
    fn store_num(&mut self, arg: usize, int64: bool, longone: i8, v: u32, num64: u64) {
        if int64 {
            self.write(arg, 0, &num64.to_le_bytes());
        } else if longone != 0 {
            self.write(arg, 0, &v.to_le_bytes());
        } else {
            self.write(arg, 0, &(v as u16).to_le_bytes());
        }
    }
}
