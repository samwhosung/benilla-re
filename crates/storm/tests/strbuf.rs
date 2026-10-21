//! Storm's `SStrLen` (`0x64a6f0`), `SStrPack` (`0x64a760`) and `SStrStr` (`0x64b4f0`, with its
//! second copy `0x64b4a0`) run whole in the emulator against `str_len`, `str_pack` and
//! `str_str`.
//!
//! Each function runs with its own callees (`SStrCmp` `0x64a480` and the C runtime's `strncmp`
//! `0x40de80` under `SStrStr`); `SErrSetLastError` (`0x64e850`) is stubbed, since it calls
//! `SetLastError`, and its log shows the argument errors. Every destination is a sentinel-filled
//! region compared whole after the call, so a write past the end of the string, or past the limit,
//! shows. The inputs are every string of a small alphabet to a length, against every size the
//! limit test distinguishes (0 to 7, 9, 0x40, 0x7fffffff, 0x80000000 and the two sizes that wrap the
//! limit address), then a fuzz.

mod common;

use benilla_re_difftest::Emu;
use benilla_re_storm::{str_len, str_pack, str_str};
use common::Wow;

const SENTINEL: u8 = 0xaa;
const SERR: u64 = 0x64e850;

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

/// The log of `SErrSetLastError` calls since the last clear, as their arguments.
fn errors(emu: &Emu) -> Vec<u32> {
    emu.calls()
        .iter()
        .inspect(|c| assert_eq!(c.addr, SERR))
        .map(|c| c.args[0])
        .collect()
}

/// `0x64a6f0` whole: lengths, bytes after the NUL, high bytes, and a null string.
#[test]
fn sstrlen_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(SERR, 0, 1);
    let buf = emu.alloc(0x100);
    let mut calls = 0;
    let mut check = |emu: &mut Emu, bytes: &[u8]| {
        let mut z = bytes.to_vec();
        z.push(0);
        z.extend_from_slice(b"tail");
        emu.write(buf, &z);
        emu.clear_calls();
        let got = emu.call(0x64a6f0, 0, 0, &[buf as u32]);
        assert_eq!(got, str_len(bytes), "{bytes:?}");
        assert_eq!(got, str_len(&z), "{bytes:?} then a tail");
        assert!(emu.calls().is_empty(), "{bytes:?}: no error");
        calls += 1;
    };
    for s in strings(&[b'a', 0x80, 0xff, b'%'], 6) {
        check(&mut emu, &s);
    }
    let mut seed = 0x64a6_f001u32;
    for _ in 0..2000 {
        let len = next(&mut seed) as usize % 0x90;
        let s: Vec<u8> = (0..len)
            .map(|_| 1 + (next(&mut seed) % 255) as u8)
            .collect();
        check(&mut emu, &s);
    }
    // A null string sets error 0x57 and gives 0.
    emu.clear_calls();
    assert_eq!(emu.call(0x64a6f0, 0, 0, &[0]), 0);
    assert_eq!(errors(&emu), vec![0x57]);
    eprintln!("sstrlen_whole: {calls} calls bit-exact");
}

/// The buffers one `SStrPack` call uses.
struct PackRig {
    region: u64,
    src: u64,
}

const REGION: usize = 0x200;
const DEST_AT: usize = 0x20;

impl PackRig {
    /// One call: the destination string `dest` at offset `DEST_AT` of a sentinel region, `src`
    /// and `size`; compares the result and the whole region with `str_pack`.
    fn check(&self, emu: &mut Emu, dest: &[u8], src: &[u8], size: u32) {
        let mut mem = vec![SENTINEL; REGION];
        mem[DEST_AT..DEST_AT + dest.len()].copy_from_slice(dest);
        mem[DEST_AT + dest.len()] = 0;
        emu.write(self.region, &mem);
        let mut z = src.to_vec();
        z.push(0);
        emu.write(self.src, &z);
        emu.clear_calls();
        let got = emu.call(
            0x64a760,
            0,
            0,
            &[(self.region + DEST_AT as u64) as u32, self.src as u32, size],
        );
        let want = str_pack(&mut mem, DEST_AT, self.region as u32, src, size);
        let what = format!("dest {dest:?} src {src:?} size {size:#x}");
        assert_eq!(got, want, "result: {what}");
        assert_eq!(emu.read(self.region, REGION), mem, "memory: {what}");
        assert!(emu.calls().is_empty(), "no error: {what}");
    }
}

/// `0x64a760` whole: every destination and source of two letters to a length against the sizes
/// around each length, the unlimited size, the sizes that wrap the limit, a fuzz, and the
/// argument errors.
#[test]
fn sstrpack_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(SERR, 0, 1);
    let rig = PackRig {
        region: emu.alloc(REGION as u64),
        src: emu.alloc(0x100),
    };
    let sizes: [u32; 14] = [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        9,
        0x40,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_fffe,
        0xffff_ffff,
    ];
    let mut calls = 0;
    for dest in strings(b"ab", 5) {
        for src in strings(b"cd", 5) {
            for size in sizes {
                rig.check(&mut emu, &dest, &src, size);
                calls += 1;
            }
        }
    }
    let mut seed = 0x64a7_6001u32;
    for _ in 0..8000 {
        let dl = next(&mut seed) as usize % 0x30;
        let sl = next(&mut seed) as usize % 0x60;
        let dest: Vec<u8> = (0..dl).map(|_| 1 + (next(&mut seed) % 255) as u8).collect();
        let src: Vec<u8> = (0..sl).map(|_| 1 + (next(&mut seed) % 255) as u8).collect();
        let size = match next(&mut seed) % 4 {
            0 => (next(&mut seed) as usize % 0x80) as u32,
            1 => (dl + 1 + (next(&mut seed) as usize % 3)) as u32,
            _ => sizes[next(&mut seed) as usize % sizes.len()],
        };
        rig.check(&mut emu, &dest, &src, size);
        calls += 1;
    }
    // A null destination or source sets error 0x57, gives 0 and writes nothing, even for size 0.
    for (what, dest, src, size) in [
        ("null destination", 0, rig.src, 8),
        ("null source", rig.region + DEST_AT as u64, 0, 8),
        ("both null", 0, 0, 8),
        ("null destination, size 0", 0, rig.src, 0),
    ] {
        emu.write(rig.region, &[SENTINEL; REGION]);
        emu.write(rig.src, b"xy\0");
        emu.clear_calls();
        let got = emu.call(0x64a760, 0, 0, &[dest as u32, src as u32, size]);
        assert_eq!(got, 0, "{what}");
        assert_eq!(errors(&emu), vec![0x57], "{what}");
        assert_eq!(emu.read(rig.region, REGION), [SENTINEL; REGION], "{what}");
    }
    eprintln!("sstrpack_whole: {calls} calls bit-exact");
}

/// `0x64b4f0` and `0x64b4a0` whole on haystacks and needles of mixed case and high bytes,
/// empty ones included, and their null arguments.
#[test]
fn sstrstr_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(SERR, 0, 1);
    let hay_at = emu.alloc(0x100);
    let needle_at = emu.alloc(0x100);
    let mut calls = 0;
    let mut check = |emu: &mut Emu, hay: &[u8], needle: &[u8]| {
        let mut h = hay.to_vec();
        h.extend_from_slice(b"\0tail");
        let mut n = needle.to_vec();
        n.extend_from_slice(b"\0tail");
        emu.write(hay_at, &h);
        emu.write(needle_at, &n);
        let want = str_str(hay, needle).map_or(0, |i| hay_at as u32 + i as u32);
        for function in [0x64b4f0, 0x64b4a0] {
            emu.clear_calls();
            let got = emu.call(function, hay_at as u32, needle_at as u32, &[]);
            assert_eq!(got, want, "{function:#x}: {hay:?} {needle:?}");
            assert!(emu.calls().is_empty(), "{function:#x}: no error");
            calls += 1;
        }
        assert_eq!(
            str_str(&h, &n),
            str_str(hay, needle),
            "the bytes after the NUL are not read"
        );
    };
    let words = strings(b"aAb", 5);
    let needles = strings(b"aAb", 3);
    for hay in &words {
        for needle in &needles {
            check(&mut emu, hay, needle);
        }
    }
    let mut seed = 0x64b4_f001u32;
    let alphabet = [b'a', b'A', b'|', b'r', b'R', 0x80, 0xff];
    for _ in 0..6000 {
        let hl = next(&mut seed) as usize % 24;
        let nl = next(&mut seed) as usize % 4;
        let hay: Vec<u8> = (0..hl)
            .map(|_| alphabet[next(&mut seed) as usize % alphabet.len()])
            .collect();
        let needle: Vec<u8> = (0..nl)
            .map(|_| alphabet[next(&mut seed) as usize % alphabet.len()])
            .collect();
        check(&mut emu, &hay, &needle);
    }
    // The escape check's own case: `|r` in a `|c` run.
    check(&mut emu, b"|cffffffffText|r after", b"|r");
    check(&mut emu, b"|cffffffffText|R after", b"|r");
    // A null haystack or needle sets error 0x57 and gives null.
    emu.write(hay_at, b"abc\0");
    emu.write(needle_at, b"b\0");
    for function in [0x64b4f0, 0x64b4a0] {
        for (hay, needle) in [(0, needle_at), (hay_at, 0), (0, 0)] {
            emu.clear_calls();
            let got = emu.call(function, hay as u32, needle as u32, &[]);
            assert_eq!(got, 0, "{function:#x}: null argument");
            assert_eq!(errors(&emu), vec![0x57], "{function:#x}: null argument");
        }
    }
    eprintln!("sstrstr_whole: {calls} calls bit-exact");
}
