//! `sscanf` (`0x73fb5d`) with its engine `__input` (`0x744e21`) and the helpers it calls run whole
//! against `benilla_re_crt::scanf` (`systems/crt/scanf.md`): `_inc` (`0x74587d`), `__whiteout`
//! (`0x7458ae`), `__un_inc` (`0x745897`) over `_ungetc_lk` (`0x741c1b`), `__filbuf` (`0x745946`),
//! `__hextodec` (`0x745846`), `__allshl`, `__allmul`, `_memset`, `strlen` and the float store
//! `_fassign` (`0x40b50e`) through its slot, which `0x409a29` fills as the start-up does. The
//! lock wrapper `_mbtowc` (`0x747b78`) is a stub that does what `__mbtowc_lk` (`0x747bd5`) does
//! with the locale handle 0: the byte as a 16-bit unit, 1 for a byte, 0 for a NUL.
//!
//! Each case runs a format over an input with ten output blocks of 256 bytes filled with 0xa5 and
//! compares the return value and every block. A format of no bytes is not run: its result reads a
//! local the exe never sets. A set that the format leaves open (`%[a`) is not run either: the exe
//! reads on past the format's NUL until a `]`.

mod common;

use benilla_re_crt::scanf::sscanf;
use benilla_re_difftest::{Emu, Wow};
use common::{ten_powers, Lcg};

const SSCANF: u64 = 0x73fb5d;
const MBTOWC: u64 = 0x747b78;
const CFLTCVT_INIT: u64 = 0x409a29;
const SLOTS: usize = 10;
const SLOT: usize = 256;

struct Rig<'a> {
    e: Emu<'a>,
    input: u64,
    format: u64,
    out: Vec<u64>,
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut e = Emu::new(wow);
        e.call(CFLTCVT_INIT, 0, 0, &[]);
        e.stub_fn(MBTOWC, 0, |io| {
            let (dest, src, n) = (u64::from(io.arg(0)), u64::from(io.arg(1)), io.arg(2));
            if src == 0 || n == 0 {
                return 0;
            }
            let b = io.read(src, 1)[0];
            if dest != 0 {
                io.write(dest, &u16::from(b).to_le_bytes());
            }
            u32::from(b != 0)
        });
        let input = e.alloc(0x400);
        let format = e.alloc(0x200);
        let out = (0..SLOTS).map(|_| e.alloc(SLOT as u64)).collect();
        Rig {
            e,
            input,
            format,
            out,
        }
    }

    /// The exe's answer: the return value and the ten blocks.
    fn exe(&mut self, input: &[u8], format: &[u8]) -> (i32, Vec<Vec<u8>>) {
        let mut s = input.to_vec();
        s.push(0);
        self.e.write(self.input, &s);
        let mut f = format.to_vec();
        f.push(0);
        self.e.write(self.format, &f);
        for &o in &self.out {
            self.e.write(o, &[0xa5; SLOT]);
        }
        let mut args = vec![self.input as u32, self.format as u32];
        args.extend(self.out.iter().map(|&o| o as u32));
        let ret = self.e.call(SSCANF, 0, 0, &args) as i32;
        let blocks = self.out.iter().map(|&o| self.e.read(o, SLOT)).collect();
        (ret, blocks)
    }
}

/// Formats that cross every conversion, flag and edge.
const FORMATS: &[&str] = &[
    "%d",
    "%i",
    "%u",
    "%o",
    "%x",
    "%X",
    "%p",
    "%hd",
    "%ld",
    "%hhd",
    "%lld",
    "%I64d",
    "%I64x",
    "%I64p",
    "%I64X",
    "%I64o",
    "%I64i",
    "%I64u",
    "%I64",
    "%I6d",
    "%I",
    "%3d",
    "%1d",
    "%0d",
    "%2x",
    "%1x",
    "%2i",
    "%5o",
    "%2I64d",
    "%16I64X",
    "%16I64X:%d:%d:%d:%d",
    "%16I64X:%d:%d",
    "%*d",
    "%*d%d",
    "%d%d",
    "%d,%d",
    "%d %d",
    "%d  %d",
    "%d\n%d",
    " %d",
    "  %d  ",
    "%d ",
    "%f",
    "%e",
    "%g",
    "%E",
    "%G",
    "%lf",
    "%Lf",
    "%hf",
    "%3f",
    "%1f",
    "%0f",
    "%5e",
    "%*f",
    "%f%f",
    "%f %f",
    "%f,%f",
    "%c",
    "%C",
    "%3c",
    "%0c",
    "%*c",
    "%lc",
    "%hc",
    "%wc",
    "%s",
    "%S",
    "%ls",
    "%hs",
    "%ws",
    "%3s",
    "%0s",
    "%*s",
    "%s%s",
    "%s %s",
    "%[abc]",
    "%[^abc]",
    "%[a-c]",
    "%[c-a]",
    "%[-a]",
    "%[a-]",
    "%[]a]",
    "%[^]a]",
    "%[a-c-e]",
    "%[0-9]",
    "%[0-9a-f]",
    "%[^0-9]",
    "%[^ ]",
    "%3[a-z]",
    "%*[a-z]",
    "%l[a-z]",
    "%{ab]",
    "%n",
    "%d%n",
    "%hn",
    "%ln",
    "%I64n",
    "%*n",
    "%s%n",
    "%c%n",
    "%%",
    "%d%%",
    "%%%d",
    "%",
    "%d%",
    "%z",
    "%q%d",
    "%5",
    "%-d",
    "%+d",
    "%.2f",
    "%D",
    "%O",
    "%U",
    "%P",
    "%N",
    "%Fd",
    "%Nd",
    "%Ld",
    "%wd",
    "a",
    "ab",
    "a%d",
    "%da",
    "%d:%d",
    "x%dy",
    "%d%c",
    "%c%d",
    "%s:%d",
    "%[a-z]%d",
    "%d%s%f%x",
    "%f%e%g",
    "%hd%ld%I64d%d",
    " ",
    "  \t\n",
    "% d",
    "%d %",
    "%d  \n",
];

/// Inputs that cross the same edges.
const INPUTS: &[&str] = &[
    "",
    " ",
    "\t\n\x0b\x0c\r ",
    "0",
    "1",
    "-1",
    "+1",
    "-",
    "+",
    "--1",
    "+-1",
    "12",
    "123",
    "  42",
    "\n\t 42xyz",
    "007",
    "08",
    "0x1f",
    "0X1F",
    "0x",
    "0xg",
    "0x1g",
    "-0x10",
    "+0x10",
    "1f",
    "ff",
    "FF",
    "dead",
    "DEADBEEF",
    "0xdeadbeef",
    "0xFFFFFFFF",
    "4294967295",
    "4294967296",
    "-4294967295",
    "2147483647",
    "2147483648",
    "-2147483648",
    "-2147483649",
    "99999999999",
    "18446744073709551615",
    "18446744073709551616",
    "9223372036854775808",
    "0123456701234567",
    "1234567890123456",
    "123456789ABCDEF0",
    "00000000DEADBEEF:1:2:3:4",
    "123456789abcdef0:100:200:300:400",
    "    DEADBEEF:5:6",
    "0x0000000DEADBEEF:5:6",
    "0000000000000000:0:0",
    "FFFFFFFFFFFFFFFF:4294967295:4294967295:1:2",
    "1.5",
    "-1.5",
    "+1.5",
    ".5",
    "5.",
    ".",
    "-.",
    "1e5",
    "1E5",
    "1e+5",
    "1e-5",
    "1e",
    "1e+",
    "1e-",
    "1ex",
    "e5",
    ".e5",
    "1d5",
    "1.5.5",
    "1e5e5",
    "460e8",
    "1e39",
    "1e-40",
    "1e400",
    "3.4028235e38",
    "3.4028236e38",
    "1.7976931348623157e308",
    "5e-324",
    "123456789012345678901234567890",
    "0.1",
    "0.30000000000000004",
    "1.5,2.5",
    "1.5 2.5",
    "1,2",
    "1 2",
    "1  2",
    "1\n2",
    "1\t\t2",
    "12abc",
    "abc",
    "a",
    "b",
    "abcabc",
    "aaa",
    "cba",
    "a-c",
    "a]b",
    "]]a",
    "hello world",
    "hello",
    "   hello   ",
    "x1y",
    "x12y",
    "xy",
    "1%",
    "%",
    "%1",
    "1:2",
    "1:",
    ":2",
    "Z",
    "zzzz:9",
    "a1.5",
    "a1",
    "1a",
    "5 apples 2.5 0x1F",
    "\u{80}",
    "\u{ff}1",
    "1\u{e9}",
    "12\u{0}34",
    "\u{0}",
];

fn check(rig: &mut Rig, t: &benilla_re_crt::ld12::TenPowers, input: &[u8], format: &[u8]) {
    let (want_ret, want_blocks) = rig.exe(input, format);
    let mut slots = vec![vec![0xa5u8; SLOT]; SLOTS];
    let got_ret = sscanf(input, format, &mut slots, t);
    assert_eq!(
        got_ret,
        want_ret,
        "return of {:?} over {:?}",
        String::from_utf8_lossy(format),
        String::from_utf8_lossy(input)
    );
    for (i, (a, b)) in slots.iter().zip(&want_blocks).enumerate() {
        assert_eq!(
            a,
            b,
            "block {i} of {:?} over {:?}",
            String::from_utf8_lossy(format),
            String::from_utf8_lossy(input)
        );
    }
}

/// Numerals around the float text's 349-byte limit, with the sign, the point and the exponent
/// placed at it.
fn long_inputs() -> Vec<String> {
    let mut v = Vec::new();
    for n in [346, 347, 348, 349, 350, 351, 400] {
        let d = "1".repeat(n);
        v.push(d.clone());
        v.push(format!("-{d}"));
        v.push(format!("+{d}e5"));
        v.push(format!("0.{d}"));
        v.push(format!("-0.{d}e-3"));
        v.push(format!("{d}.5"));
        v.push(format!("{d}e"));
        v.push(format!("{d}e-"));
        v.push(format!("{d}e-7"));
        v.push(format!("1.{d}e+2"));
    }
    for n in [345, 346, 347, 348, 349, 350] {
        v.push(format!("1{}e5", "0".repeat(n)));
        v.push(format!("1.{}e-5x", "0".repeat(n)));
    }
    v
}

#[test]
fn crt_sscanf_long_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let t = ten_powers(&rig.e);
    for f in [
        "%f", "%lf", "%e", "%g %d", "%f%n", "%lf%n", "%e%n", "%349f%n", "%d%n", "%3f", "%349f",
        "%350f", "%348f", "%d", "%I64d", "%x",
    ] {
        for i in long_inputs() {
            check(&mut rig, &t, i.as_bytes(), f.as_bytes());
        }
    }
}

#[test]
fn crt_sscanf_fixed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let t = ten_powers(&rig.e);
    for f in FORMATS {
        for i in INPUTS {
            check(&mut rig, &t, i.as_bytes(), f.as_bytes());
        }
    }
}

/// A random format of up to six items, each a literal, a space or a conversion.
fn random_format(g: &mut Lcg) -> Vec<u8> {
    let mut f = Vec::new();
    let mut args = 0;
    // `%n` reads nothing, so one before any read leaves the exe's last-byte local unset.
    let mut read = false;
    for _ in 0..1 + g.below(6) {
        match g.below(10) {
            0 => {
                f.push(b"ab:,xy"[g.below(6) as usize]);
                read = true;
            }
            1 => f.push(b"  \t\n"[g.below(4) as usize]),
            _ => {
                if args >= SLOTS - 1 {
                    continue;
                }
                f.push(b'%');
                if g.below(6) == 0 {
                    f.push(b'*');
                } else {
                    args += 1;
                }
                if g.below(3) == 0 {
                    f.extend(match g.below(9) {
                        0 => b"0".as_slice(),
                        1 => b"1",
                        2 => b"2",
                        3 => b"3",
                        4 => b"5",
                        5 => b"8",
                        6 => b"16",
                        7 => b"349",
                        _ => b"400",
                    });
                }
                match g.below(14) {
                    0 => f.extend(b"h"),
                    1 => f.extend(b"l"),
                    2 => f.extend(b"L"),
                    3 => f.extend(b"w"),
                    4 => f.extend(b"I64"),
                    5 => f.extend(b"hh"),
                    _ => {}
                }
                let ty: &[u8] = match g.below(24) {
                    0 | 1 => b"d",
                    2 => b"i",
                    3 => b"u",
                    4 => b"o",
                    5 | 6 => b"x",
                    7 => b"X",
                    8 => b"p",
                    9 | 10 => b"f",
                    11 => b"e",
                    12 => b"g",
                    13 => b"c",
                    14 => b"s",
                    15 => b"S",
                    16 => b"C",
                    17 => b"[a-f]",
                    18 => b"[^ab ]",
                    19 => b"[]x1-3]",
                    20 => b"n",
                    21 => b"%",
                    22 => b"E",
                    _ => b"G",
                };
                let ty = if ty == b"n" && !read { b"d" } else { ty };
                read |= ty != b"n";
                f.extend(ty);
            }
        }
    }
    f
}

/// A random input from the bytes numbers and separators are made of.
fn random_input(g: &mut Lcg) -> Vec<u8> {
    const ALPHABET: &[u8] = b"0123456789  \t\n+-..eExXabcdefABCDEFgz:,%\x80\xff";
    let n = g.below(26) as usize;
    (0..n)
        .map(|_| ALPHABET[g.below(ALPHABET.len() as u64) as usize])
        .collect()
}

#[test]
fn crt_sscanf_random_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let t = ten_powers(&rig.e);
    let mut g = Lcg(0x0005_ca2f_0000);
    for _ in 0..40000 {
        let f = random_format(&mut g);
        for _ in 0..8 {
            let i = random_input(&mut g);
            check(&mut rig, &t, &i, &f);
        }
    }
}
