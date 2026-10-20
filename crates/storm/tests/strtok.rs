//! Storm's `SStrTokenize` (`0x64ae50`) run whole in the emulator against `str_tokenize`.
//!
//! It runs with its own callee `SStrChr` (`0x64a3c0`); `SErrSetLastError` (`0x64e850`) is stubbed,
//! since it calls `SetLastError`. The output buffer is filled with a sentinel before each call, so
//! the comparison covers every byte the call writes and every byte it leaves alone, the cursor
//! cell and the quote cell. The inputs are every string of five symbols to length five against
//! seven delimiter strings and five output sizes, the console's delimiter strings among them, then
//! longer strings over a wider alphabet, and whole lines tokenized call after call.

mod common;

use benilla_re_difftest::Emu;
use benilla_re_storm::str_tokenize;
use common::Wow;

const SENTINEL: u8 = 0xaa;
const QUOTE_SENTINEL: u32 = 0x5555_5555;

/// The delimiter strings: the console's `0x82ecc8` and `0x82ef90`, the CVar commands' set, and
/// shapes that move the quote: one space, a comma, the quote alone, no delimiter at all, a set
/// without the quote.
const DELIMS: [&[u8]; 7] = [
    b"\t\r\n\" ",
    b"\t\r\n\" ,",
    b" ,;\t\"\r\n",
    b" ",
    b",",
    b"\"",
    b"",
];

/// The buffers one call uses.
struct Rig {
    text: u64,
    delims: u64,
    out: u64,
    cursor: u64,
    quoted: u64,
}

impl Rig {
    fn new(emu: &mut Emu) -> Rig {
        Rig {
            text: emu.alloc(0x400),
            delims: emu.alloc(0x40),
            out: emu.alloc(0x100),
            cursor: emu.alloc(4),
            quoted: emu.alloc(4),
        }
    }

    /// One call on `input` from offset `at`, compared with `str_tokenize`; returns the offset the
    /// cursor ends at.
    fn check(&self, emu: &mut Emu, input: &[u8], at: usize, delims: &[u8], size: usize) -> usize {
        let mut z = input.to_vec();
        z.push(0);
        emu.write(self.text, &z);
        let mut d = delims.to_vec();
        d.push(0);
        emu.write(self.delims, &d);
        emu.write(self.out, &[SENTINEL; 0x100]);
        emu.write_u32(self.cursor, (self.text + at as u64) as u32);
        emu.write_u32(self.quoted, QUOTE_SENTINEL);
        let (out_arg, quoted_arg) = (if size == 0 { 0 } else { self.out }, self.quoted);
        emu.call(
            0x64ae50,
            0,
            0,
            &[
                self.cursor as u32,
                out_arg as u32,
                size as u32,
                self.delims as u32,
                quoted_arg as u32,
            ],
        );
        let want = str_tokenize(&input[at..], delims, size);
        let what = format!(
            "{:?} from {at} delims {:?} size {size}",
            String::from_utf8_lossy(input),
            String::from_utf8_lossy(delims)
        );
        let mut expect = vec![SENTINEL; 0x100];
        if size > 0 {
            expect[..want.text.len()].copy_from_slice(&want.text);
            expect[want.text.len()] = 0;
        }
        assert_eq!(emu.read(self.out, 0x100), expect, "output: {what}");
        let cursor = emu.read_u32(self.cursor) as u64 - self.text;
        assert_eq!(cursor as usize, at + want.next, "cursor: {what}");
        assert_eq!(
            emu.read_u32(self.quoted),
            u32::from(want.quoted),
            "quote flag: {what}"
        );
        assert!(emu.calls().is_empty(), "no error raised: {what}");
        at + want.next
    }
}

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s >> 8
}

/// Every string of `alphabet` up to `max` bytes, shortest first.
fn strings(alphabet: &[u8], max: usize) -> Vec<Vec<u8>> {
    let mut all = vec![Vec::new()];
    let mut from = 0;
    for _ in 0..max {
        let to = all.len();
        for i in from..to {
            for &c in alphabet {
                let mut v = all[i].clone();
                v.push(c);
                all.push(v);
            }
        }
        from = to;
    }
    all
}

/// `0x64ae50` whole: the exhaustive grid, the lines the commands see, a fuzz, whole lines call by
/// call, the optional arguments and the argument errors.
#[test]
fn sstrtokenize_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x64e850, 0, 1);
    let rig = Rig::new(&mut emu);

    // Every string of five symbols to length five against every delimiter string and size.
    let grid = strings(b"a \"\t,", 5);
    let sizes = [0usize, 1, 2, 3, 8];
    let mut calls = 0;
    for text in &grid {
        for delims in DELIMS {
            for size in sizes {
                rig.check(&mut emu, text, 0, delims, size);
                calls += 1;
            }
        }
    }

    // The lines the commands see.
    let lines: &[&[u8]] = &[
        b"",
        b" ",
        b"\"",
        b"\"\"",
        b"\"\"\"",
        b"1",
        b"1 2 3",
        b"  1   2",
        b"1,2,3",
        b"\"a b\" c",
        b"a\"b c\"",
        b"\"abc",
        b"\"abc\"def\"ghi",
        b"abc\"",
        b"a\tb\r\nc",
        b"a  \" b \"  c",
        b"\x80 \xff \x80\xff",
        b"0123456789012345678901234567890123456789012345678901234567890123456789 x",
        b"\"0123456789012345678901234567890123456789012345678901234567890123456789\" x",
    ];
    for line in lines {
        for delims in DELIMS {
            for size in [0usize, 1, 5, 64, 0x40, 0x80] {
                rig.check(&mut emu, line, 0, delims, size);
                calls += 1;
            }
        }
    }

    // A fuzz over a wider alphabet and longer strings, from a random offset.
    let alphabet = b"ab01 \t\r\n\",;\x80";
    let mut s = 0x64ae_5011u32;
    for _ in 0..6000 {
        let len = next(&mut s) % 40;
        let text: Vec<u8> = (0..len)
            .map(|_| alphabet[(next(&mut s) as usize) % alphabet.len()])
            .collect();
        let at = if text.is_empty() {
            0
        } else {
            (next(&mut s) as usize) % (text.len() + 1)
        };
        let delims = DELIMS[(next(&mut s) as usize) % DELIMS.len()];
        let size = [0usize, 1, 2, 3, 4, 8, 64][(next(&mut s) as usize) % 7];
        rig.check(&mut emu, &text, at, delims, size);
        calls += 1;
    }

    // Whole lines, call after call from the cursor each call leaves, as a command loop reads them.
    let mut chained = 0;
    for _ in 0..1500 {
        let len = next(&mut s) % 30;
        let text: Vec<u8> = (0..len)
            .map(|_| alphabet[(next(&mut s) as usize) % alphabet.len()])
            .collect();
        for delims in [DELIMS[0], DELIMS[1]] {
            let mut at = 0;
            for _ in 0..40 {
                let end = rig.check(&mut emu, &text, at, delims, 0x40);
                chained += 1;
                if end == at || end >= text.len() {
                    break;
                }
                at = end;
            }
        }
    }

    // The quote cell is optional: a null fifth argument writes nothing and the rest is the same.
    let line = b"\"a b\" c";
    emu.write(rig.text, b"\"a b\" c\0");
    emu.write(rig.delims, b"\t\r\n\" \0");
    emu.write(rig.out, &[SENTINEL; 0x100]);
    emu.write_u32(rig.cursor, rig.text as u32);
    emu.write_u32(rig.quoted, QUOTE_SENTINEL);
    emu.call(
        0x64ae50,
        0,
        0,
        &[
            rig.cursor as u32,
            rig.out as u32,
            0x40,
            rig.delims as u32,
            0,
        ],
    );
    assert_eq!(emu.read(rig.out, 4), b"a b\0");
    assert_eq!(
        emu.read_u32(rig.cursor) as u64 - rig.text,
        str_tokenize(line, b"\t\r\n\" ", 0x40).next as u64
    );
    assert_eq!(emu.read_u32(rig.quoted), QUOTE_SENTINEL);

    // A non-null output of size 0 is never written either.
    emu.write(rig.out, &[SENTINEL; 0x100]);
    emu.write(rig.text, b"abc def\0");
    emu.write_u32(rig.cursor, rig.text as u32);
    emu.call(
        0x64ae50,
        0,
        0,
        &[
            rig.cursor as u32,
            rig.out as u32,
            0,
            rig.delims as u32,
            rig.quoted as u32,
        ],
    );
    assert_eq!(emu.read(rig.out, 0x100), vec![SENTINEL; 0x100]);
    assert_eq!(emu.read_u32(rig.cursor) as u64 - rig.text, 4);

    // Null arguments: error 0x57 and nothing written.
    let errors: [(&str, [u64; 4]); 4] = [
        ("null cursor", [0, rig.out, 8, rig.delims]),
        ("null output with a size", [rig.cursor, 0, 8, rig.delims]),
        ("null delimiters", [rig.cursor, rig.out, 8, 0]),
        ("cursor at null", [rig.cursor, rig.out, 8, rig.delims]),
    ];
    for (what, [cursor, out, size, delims]) in errors {
        emu.write(rig.out, &[SENTINEL; 0x100]);
        emu.write(rig.delims, b" \0");
        let held = if what == "cursor at null" {
            0
        } else {
            rig.text
        };
        emu.write_u32(rig.cursor, held as u32);
        emu.write_u32(rig.quoted, QUOTE_SENTINEL);
        emu.clear_calls();
        emu.call(
            0x64ae50,
            0,
            0,
            &[
                cursor as u32,
                out as u32,
                size as u32,
                delims as u32,
                rig.quoted as u32,
            ],
        );
        let c = emu.calls();
        assert_eq!(c.len(), 1, "{what}: one error");
        assert_eq!((c[0].addr, c[0].args[0]), (0x64e850, 0x57), "{what}");
        assert_eq!(emu.read(rig.out, 0x100), vec![SENTINEL; 0x100], "{what}");
        assert_eq!(emu.read_u32(rig.cursor), held as u32, "{what}: cursor kept");
        assert_eq!(emu.read_u32(rig.quoted), QUOTE_SENTINEL, "{what}");
    }
    eprintln!("sstrtokenize_whole: {calls} calls bit-exact, {chained} chained");
}
