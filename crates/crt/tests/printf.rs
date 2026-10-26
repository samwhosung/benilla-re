//! The C runtime's `printf` (`systems/crt/printf.md`): `sprintf` (`0x73fec4`) and `_vsnprintf`
//! (`0x73f6ad`) run whole with `__output` (`0x742ba5`), its writers and argument readers, the
//! floating-point conversions through `_cfltcvt` (`0x40b7fe`) and the digits through `_fltout2`
//! (`0x40ff70`), against `benilla_re_crt::output`; `_snprintf` (`0x740107`) and `_vsprintf`
//! (`0x7412c1`) against the same models. `0x409a29` runs first, as the start-up does, to fill
//! the conversion slots from `0x82f3f8`.
//!
//! The double is loaded and stored by `fld`/`fstp` at `0x40b560`, `0x40b683` and `0x40b781`; the
//! emulator keeps a signalling NaN signalling there where the x87 quiets it
//! (`docs/METHOD.md#the-harness`), so no case here passes one.

mod common;

use benilla_re_crt::ld12::TenPowers;
use benilla_re_crt::output::{sprintf, vsnprintf, Memory, Output, Store};
use benilla_re_difftest::{Emu, Wow};
use common::{doubles, ten_powers, Lcg};

const SPRINTF: u64 = 0x73fec4;
const VSNPRINTF: u64 = 0x73f6ad;
const SNPRINTF: u64 = 0x740107;
const VSPRINTF: u64 = 0x7412c1;
const CFLTCVT_INIT: u64 = 0x409a29;
const BUF: usize = 0x1000;
const SENTINEL: u8 = 0xa5;

/// The strings a case places in the emulator, by address.
struct Strings(Vec<(u32, Vec<u8>)>);

impl Memory for Strings {
    fn byte(&self, address: u32) -> u8 {
        for (at, s) in &self.0 {
            if (*at..*at + s.len() as u32 + 1).contains(&address) {
                return s.get((address - at) as usize).copied().unwrap_or(0);
            }
        }
        panic!("read of {address:#x} outside the strings")
    }
}

struct Rig<'a> {
    e: Emu<'a>,
    t: TenPowers,
    buf: u64,
    fmt: u64,
    strings: u64,
    stores: u64,
}

impl Rig<'_> {
    fn new(wow: &Wow) -> Rig<'_> {
        let mut e = Emu::new(wow);
        e.call(CFLTCVT_INIT, 0, 0, &[]);
        let t = ten_powers(&e);
        let buf = e.alloc(BUF as u64);
        let fmt = e.alloc(0x200);
        let strings = e.alloc(0x800);
        let stores = e.alloc(0x40);
        Rig {
            e,
            t,
            buf,
            fmt,
            strings,
            stores,
        }
    }

    /// Writes the strings and rewrites the argument markers: `STR + i` is the address of string
    /// `i`, `STORE + i` a slot for `%n`.
    fn place(&mut self, args: &[u32], strings: &[&[u8]]) -> (Vec<u32>, Strings) {
        let mut placed = Vec::new();
        let mut at = self.strings as u32;
        for s in strings {
            let mut z = s.to_vec();
            z.push(0);
            self.e.write(u64::from(at), &z);
            placed.push((at, s.to_vec()));
            at += z.len() as u32;
        }
        self.e.write(self.stores, &[0; 0x40]);
        let args = args
            .iter()
            .map(|&a| match a {
                STR..=STR_END => placed[(a - STR) as usize].0,
                STORE..=STORE_END => self.stores as u32 + 4 * (a - STORE),
                _ => a,
            })
            .collect();
        (args, Strings(placed))
    }

    /// Runs the exe's `sprintf` or `_vsnprintf` and reads back what it wrote: the buffer, its
    /// return, and the `%n` slots.
    fn run(&mut self, fmt: &[u8], args: &[u32], count: Option<u32>) -> (Vec<u8>, i32, Vec<u8>) {
        let mut f = fmt.to_vec();
        f.push(0);
        self.e.write(self.fmt, &f);
        self.e.write(self.buf, &[SENTINEL; BUF]);
        let ret = match count {
            None => {
                let mut a = vec![self.buf as u32, self.fmt as u32];
                a.extend_from_slice(args);
                self.e.call(SPRINTF, 0, 0, &a)
            }
            Some(n) => {
                let va = self.strings as u32 + 0x700;
                for (i, &v) in args.iter().enumerate() {
                    self.e.write_u32(u64::from(va) + 4 * i as u64, v);
                }
                self.e
                    .call(VSNPRINTF, 0, 0, &[self.buf as u32, n, self.fmt as u32, va])
            }
        };
        let b = self.e.read(self.buf, BUF);
        (b, ret as i32, self.e.read(self.stores, 0x40))
    }

    fn check(&mut self, fmt: &[u8], args: &[u32], strings: &[&[u8]], count: Option<u32>) {
        let (args, mem) = self.place(args, strings);
        let (buf, ret, slots) = self.run(fmt, &args, count);
        let want: Output = match count {
            None => sprintf(fmt, &args, &mem, &self.t),
            Some(n) => vsnprintf(n, fmt, &args, &mem, &self.t),
        };
        // What the model wrote, and every byte after it untouched.
        let n = want.text.len();
        let got = &buf[..n];
        assert!(
            buf[n..].iter().all(|&c| c == SENTINEL),
            "{:?}: a write past the text",
            String::from_utf8_lossy(fmt)
        );
        let mut want_slots = vec![0u8; 0x40];
        for Store {
            address,
            value,
            short,
        } in &want.stores
        {
            let at = (address - self.stores as u32) as usize;
            let n = if *short { 2 } else { 4 };
            want_slots[at..at + n].copy_from_slice(&value.to_le_bytes()[..n]);
        }
        assert_eq!(
            (String::from_utf8_lossy(got), ret, slots),
            (String::from_utf8_lossy(&want.text), want.ret, want_slots),
            "{:?} {args:x?} count {count:?}",
            String::from_utf8_lossy(fmt)
        );
    }
}

const STR: u32 = 0x5354_0000;
const STR_END: u32 = STR + 0xff;
const STORE: u32 = 0x5354_0100;
const STORE_END: u32 = STORE + 0xf;

/// Every float conversion with the flags, widths and precisions the code tells apart, over the
/// doubles of every digit regime; `%.14g`, the client's own number text, over all of them.
#[test]
fn crt_sprintf_float_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x5f10_0005);
    let mut specs: Vec<String> = Vec::new();
    for conv in ["e", "E", "f", "g", "G"] {
        for flags in ["", "-", "+", " ", "#", "0", "+0", "-0", "# ", "#0"] {
            for width in ["", "1", "12", "30"] {
                for prec in [
                    "", ".", ".0", ".1", ".2", ".5", ".14", ".16", ".17", ".20", ".40",
                ] {
                    specs.push(format!("%{flags}{width}{prec}{conv}"));
                }
            }
        }
    }
    specs.push("%Lf".into());
    specs.push("%lf".into());
    specs.push("%hg".into());
    specs.push("%*.*g".into());
    let quiet = |b: u64| {
        b & 0x7ff0_0000_0000_0000 != 0x7ff0_0000_0000_0000
            || b & 0x000f_ffff_ffff_ffff == 0
            || b & (1 << 51) != 0
    };
    let values: Vec<u64> = doubles(&mut g, 4000)
        .into_iter()
        .filter(|&b| quiet(b))
        .collect();
    // `%.14g`, the client's number text, and `%.16g`, the saved variables' (`0x70452c`).
    for &v in &values {
        r.check(b"%.14g", &[v as u32, (v >> 32) as u32], &[], None);
        r.check(b"%.16g", &[v as u32, (v >> 32) as u32], &[], None);
    }
    for (k, spec) in specs.iter().enumerate() {
        for _ in 0..40 {
            let v = values[g.below(values.len() as u64) as usize];
            let mut args = vec![];
            if spec.contains('*') {
                args.extend([
                    (g.below(40) as u32).wrapping_sub(10),
                    (g.below(30) as u32).wrapping_sub(5),
                ]);
            }
            args.extend([v as u32, (v >> 32) as u32]);
            r.check(spec.as_bytes(), &args, &[], None);
        }
        // Fixed with a large magnitude, from the integer digits to the 512-byte buffer's edge.
        if k % 7 == 0 {
            for b in [1e300f64, -1.7976931348623157e308, 1e22, 123456789e200] {
                let b = b.to_bits();
                r.check(
                    spec.replace(".40", ".2").as_bytes(),
                    &[b as u32, (b >> 32) as u32],
                    &[],
                    None,
                );
            }
        }
    }
}

/// The integer, character and string conversions, `%%`, `%n`, the size prefixes and the state
/// machine's other paths: an `I` not followed by `64`, a type with no case (`B`), unknown and
/// trailing characters.
#[test]
fn crt_sprintf_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x5f10_0006);
    let ints: Vec<u32> = vec![
        0,
        1,
        7,
        8,
        9,
        10,
        15,
        16,
        255,
        256,
        0x7fff,
        0x8000,
        0xffff,
        0x1_0000,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        0xdead_beef,
        12345,
    ];
    let mut specs: Vec<String> = Vec::new();
    for conv in ["d", "i", "u", "o", "x", "X", "p", "c"] {
        for flags in ["", "-", "+", " ", "#", "0", "+0", "-0", "#0", "- "] {
            for width in ["", "1", "5", "12", "*"] {
                for prec in ["", ".", ".0", ".1", ".3", ".10", ".*"] {
                    for size in ["", "h", "l", "I64", "L", "F", "N"] {
                        if conv == "c" && (size == "l" || size == "I64") {
                            continue;
                        }
                        specs.push(format!("%{flags}{width}{prec}{size}{conv}"));
                    }
                }
            }
        }
    }
    for spec in &specs {
        for _ in 0..3 {
            let mut args = vec![];
            for _ in 0..spec.matches('*').count() {
                args.push((g.below(30) as u32).wrapping_sub(8));
            }
            args.push(
                ints[g.below(ints.len() as u64) as usize] ^ (g.below(2) as u32 * g.next() as u32),
            );
            args.push(g.next() as u32);
            r.check(spec.as_bytes(), &args, &[], None);
        }
    }
    let strings: &[&[u8]] = &[
        b"",
        b"a",
        b"hello",
        b"a longer string here",
        b"\x80\xff\x01x",
    ];
    for flags in ["", "-", "0", "-0", "#", "+ "] {
        for width in ["", "3", "8", "*"] {
            for prec in ["", ".", ".0", ".2", ".7", ".*"] {
                for size in ["", "h", "hS"] {
                    let conv = if size == "hS" { "" } else { "s" };
                    let spec = format!("%{flags}{width}{prec}{size}{conv}");
                    for i in 0..=strings.len() {
                        let mut args = vec![];
                        for _ in 0..spec.matches('*').count() {
                            args.push((g.below(16) as u32).wrapping_sub(4));
                        }
                        args.push(if i == strings.len() {
                            0
                        } else {
                            STR + i as u32
                        });
                        r.check(spec.as_bytes(), &args, strings, None);
                    }
                }
            }
        }
    }
    let cases: &[(&[u8], &[u32])] = &[
        (b"", &[]),
        (b"plain text", &[]),
        (b"100%% sure %", &[]),
        (b"%%%d%%", &[5]),
        (b"%I32d %I6d %I64d", &[1, 2, 3, 0]),
        (b"%d%B", &[42]),
        (b"%B%5B", &[]),
        (b"%s %B %-6B|", &[STR]),
        (b"%y %5y %#y %.3q", &[]),
        (b"%5.2f%B", &[0, 0x4000_0000]),
        (b"abc%n def%hn %d", &[STORE, STORE + 1, 9]),
        (b"%c%c%c", &[0x41, 0x142, 0xff]),
        (b"%hC %hc", &[0x41, 0x42]),
        (b"%-*d|%*d|", &[0xffff_fffb, 3, 0xffff_fffd, 4]),
        (b"%.*s|%.*d", &[0xffff_ffff, STR, 0xffff_fff0, 7]),
        (
            b"%#o %#o %#.0o %#.3o %#x %#X %#x %#p %p",
            &[0, 8, 0, 8, 0, 0xab, 0xab, 0x1234, 0xab],
        ),
        (
            b"%I64d %I64u %I64x %I64o",
            &[
                0,
                0x8000_0000,
                1,
                0,
                0xffff_ffff,
                0xffff_ffff,
                0x89ab,
                0x1234_5678,
            ],
        ),
        (
            b"%lld %ld %hd %hu %hx",
            &[5, 6, 7, 0xffff_8001, 0x1_fffe, 0x1_2345],
        ),
        (
            b"%4294967295d|%.4294967295d|%.4294967296d|%4294967297d",
            &[1, 2, 0, 3],
        ),
        (b"%\x01d %\x7fd %\xffd", &[1, 2, 3]),
    ];
    for (fmt, args) in cases {
        r.check(fmt, args, &[b"text"], None);
    }
}

/// `_vsnprintf` for counts from 0 past the text's length and a negative count, on texts that end
/// in a conversion, in literal text and in padding.
#[test]
fn crt_vsnprintf_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let pi = std::f64::consts::PI.to_bits();
    let cases: &[(&[u8], Vec<u32>)] = &[
        (b"hello world", vec![]),
        (b"%d items", vec![12345]),
        (b"%-10s|", vec![STR]),
        (b"%10s", vec![STR]),
        (b"x%.3fy", vec![pi as u32, (pi >> 32) as u32]),
        (b"%5%%c", vec![0x5a]),
    ];
    for (fmt, args) in cases {
        for count in (0..20).chain([0x7fff_ffff, 0x8000_0000, 0xffff_ffff]) {
            r.check(fmt, args, &[b"abc"], Some(count));
        }
    }
}

/// Every byte class in every state: a prefix that leaves `__output` in each of the eight states,
/// then bytes of each class, then a conversion to close what is left open. The size state is
/// entered by `h`, which keeps a following `c` narrow.
#[test]
fn crt_sprintf_states_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    // Normal, percent, flag, width, dot, precision, size, type.
    let prefixes: [&[u8]; 8] = [b"a", b"%", b"%-", b"%5", b"%.", b"%.3", b"%h", b"%d"];
    let classes: [&[u8]; 9] = [
        b"y\x01~",
        b"%",
        b".",
        b"*",
        b"0",
        b"19",
        b" #+-",
        b"hlwFLN",
        b"dixXuocBp",
    ];
    let mut n = 0;
    for prefix in prefixes {
        for class in classes {
            for &c in class {
                let mut fmt = prefix.to_vec();
                fmt.push(c);
                fmt.extend_from_slice(b"d|%d");
                let args: Vec<u32> = (1..=8).collect();
                r.check(&fmt, &args, &[], None);
                n += 1;
            }
        }
    }
    assert_eq!(n, 8 * 28);
}

/// `_snprintf` (`0x740107`) and `_vsprintf` (`0x7412c1`), the other two front ends, against the
/// `_vsnprintf` and `sprintf` models: the same `FILE`, the arguments pushed or by `va_list`.
#[test]
fn crt_snprintf_vsprintf_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let pi = std::f64::consts::PI.to_bits();
    let cases: &[(&[u8], Vec<u32>)] = &[
        (b"", vec![]),
        (b"hello world", vec![]),
        (b"%d items", vec![12345]),
        (b"%-10s|", vec![STR]),
        (b"x%.3fy", vec![pi as u32, (pi >> 32) as u32]),
    ];
    for (fmt, args) in cases {
        let (args, mem) = r.place(args, &[b"abc"]);
        let mut f = fmt.to_vec();
        f.push(0);
        r.e.write(r.fmt, &f);
        for count in (0..16).chain([0x7fff_ffff, 0x8000_0000, 0xffff_ffff]) {
            r.e.write(r.buf, &[SENTINEL; BUF]);
            let mut a = vec![r.buf as u32, count, r.fmt as u32];
            a.extend_from_slice(&args);
            let ret = r.e.call(SNPRINTF, 0, 0, &a) as i32;
            let want = vsnprintf(count, fmt, &args, &mem, &r.t);
            let b = r.e.read(r.buf, BUF);
            let n = want.text.len();
            assert!(b[n..].iter().all(|&c| c == SENTINEL), "{fmt:?} {count}");
            assert_eq!(
                (&b[..n], ret),
                (&want.text[..], want.ret),
                "{fmt:?} {count}"
            );
        }
        r.e.write(r.buf, &[SENTINEL; BUF]);
        let va = r.strings + 0x700;
        for (i, &v) in args.iter().enumerate() {
            r.e.write_u32(va + 4 * i as u64, v);
        }
        let ret =
            r.e.call(VSPRINTF, 0, 0, &[r.buf as u32, r.fmt as u32, va as u32]) as i32;
        let want = sprintf(fmt, &args, &mem, &r.t);
        let b = r.e.read(r.buf, BUF);
        let n = want.text.len();
        assert!(b[n..].iter().all(|&c| c == SENTINEL), "{fmt:?}");
        assert_eq!((&b[..n], ret), (&want.text[..], want.ret), "{fmt:?}");
    }
}
