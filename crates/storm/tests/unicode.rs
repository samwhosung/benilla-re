//! Storm's UTF-8 to UTF-16 conversion `SUniConvertUTF8to16` (`0x65b350`) and its length count
//! (`0x65b250`), each run whole against the exe.

mod common;

use benilla_re_difftest::Emu;
use benilla_re_storm::{utf8_to_utf16, utf8_to_utf16_len};
use common::Wow;

/// The sources: ASCII, two- to four-byte forms, the replacement and surrogate edges, overlong and
/// five- and six-byte forms, stray continuation bytes, a NUL inside a sequence, each lead byte
/// with every kind of trailing byte, and a fuzz.
fn sources() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"Thrall".to_vec(),
        "Jaïna Proudmoore".as_bytes().to_vec(),
        "中文名字".as_bytes().to_vec(),
        "a😀b".as_bytes().to_vec(),
        "\u{ffff}\u{10000}\u{10ffff}".as_bytes().to_vec(),
        vec![0xed, 0xa0, 0x80, 0xed, 0xbf, 0xbf],
        vec![0xc0, 0x80, b'x'],
        vec![0xc1, 0xbf],
        vec![0xe0, 0x80, 0x80],
        vec![0xf4, 0x90, 0x80, 0x80],
        vec![0xf7, 0xbf, 0xbf, 0xbf],
        vec![0xf8, 0x88, 0x80, 0x80, 0x80],
        vec![0xfb, 0xbf, 0xbf, 0xbf, 0xbf],
        vec![0xfc, 0x84, 0x80, 0x80, 0x80, 0x80],
        vec![0xfd, 0xbf, 0xbf, 0xbf, 0xbf, 0xbf],
        vec![0xfe, 0x41, 0x42, 0x43, 0x44, 0x45],
        vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        vec![0x80, 0xbf, b'a'],
        vec![0xc3, 0x00, b'a', b'b'],
        vec![0xe4, 0xb8],
        vec![0xc3, 0x41],
    ];
    for lead in [0xc2u8, 0xe1, 0xf1, 0xf9, 0xfc, 0xfd, 0xfe, 0xff] {
        for trail in [0x00u8, 0x01, 0x41, 0x7f, 0x80, 0xa5, 0xbf, 0xc0, 0xff] {
            v.push(vec![lead, trail, trail, trail, trail, trail, b'z']);
        }
    }
    let mut s: u32 = 0x1357_9bdf;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..600 {
        let len = (next() >> 8) as usize % 24;
        v.push((0..len).map(|_| (next() >> 16) as u8).collect());
    }
    v
}

/// Runs `0x65b350(dst, cap, src, len, &written, &read)` (`ret 0x18`) whole for every source,
/// with no length bound (0x7fffffff, NUL-terminated, then zeros) and bounded at every length up to
/// one past the bytes, into destinations of 0 to 4 units and of the bytes' count plus one, over a
/// destination prefilled with 0xaaaa; a null source and a zero length too. It compares the result,
/// both counts and every destination unit.
#[test]
fn utf8_to_utf16_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let src = emu.alloc(0x100);
    let dst = emu.alloc(0x100);
    let counts = emu.alloc(8);
    let mut cases = 0;
    for bytes in sources() {
        let mut lens = vec![0x7fff_ffffu32, 0];
        lens.extend(1..=bytes.len() as u32 + 1);
        let mut caps = vec![0u32, 1, 2, 3, 4];
        caps.push(bytes.len() as u32 + 1);
        for &len in &lens {
            for &cap in &caps {
                let mut z = bytes.clone();
                z.extend_from_slice(&[0; 8]);
                emu.write(src, &z);
                emu.write(dst, &[0xaa; 0x40]);
                emu.write_u32(counts, 0x5555_5555);
                emu.write_u32(counts + 4, 0x5555_5555);
                let got = emu.call(
                    0x65b350,
                    0,
                    0,
                    &[
                        dst as u32,
                        cap,
                        src as u32,
                        len,
                        counts as u32,
                        counts as u32 + 4,
                    ],
                ) as i32;
                let want = utf8_to_utf16(&vec![0xaaaa; cap as usize], Some(&bytes), len);
                let what = format!("{bytes:02x?} len {len:#x} cap {cap}");
                assert_eq!(got, want.result, "result, {what}");
                let written = emu.read_u32(counts);
                let read = emu.read_u32(counts + 4);
                assert_eq!(
                    Some(written),
                    want.written.or(Some(0x5555_5555)),
                    "written, {what}"
                );
                assert_eq!(Some(read), want.read.or(Some(0x5555_5555)), "read, {what}");
                let units: Vec<u16> = emu
                    .read(dst, 2 * cap as usize)
                    .chunks(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                assert_eq!(units, want.dst, "units, {what}");
                cases += 1;
            }
        }
    }
    let got = emu.call(
        0x65b350,
        0,
        0,
        &[dst as u32, 4, 0, 0x7fff_ffff, counts as u32, 0],
    ) as i32;
    assert_eq!(got, utf8_to_utf16(&[0; 4], None, 0x7fff_ffff).result);
    assert!(cases > 10_000);
}

/// Runs `0x65b250(src, len, &read)` (`ret 0xc`) whole for the same sources and lengths, a null
/// source and a zero length, and compares the count and the bytes read.
#[test]
fn utf8_to_utf16_len_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let src = emu.alloc(0x100);
    let read = emu.alloc(4);
    for bytes in sources() {
        let mut lens = vec![0x7fff_ffffu32, 0];
        lens.extend(1..=bytes.len() as u32 + 1);
        for &len in &lens {
            let mut z = bytes.clone();
            z.extend_from_slice(&[0; 8]);
            emu.write(src, &z);
            emu.write_u32(read, 0x5555_5555);
            let got = emu.call(0x65b250, 0, 0, &[src as u32, len, read as u32]) as i32;
            let (count, consumed) = utf8_to_utf16_len(Some(&bytes), len);
            let what = format!("{bytes:02x?} len {len:#x}");
            assert_eq!(got, count, "count, {what}");
            assert_eq!(emu.read_u32(read), consumed, "read, {what}");
        }
    }
    // The count a four-byte form gets is one unit fewer than the conversion stores.
    let emoji = "a\u{1f600}b".as_bytes();
    let wide = utf8_to_utf16(&[0; 8], Some(emoji), 0x7fff_ffff);
    assert_eq!((wide.result, wide.written), (0, Some(5)));
    assert_eq!(utf8_to_utf16_len(Some(emoji), 0x7fff_ffff).0, 4);
    emu.write_u32(read, 0x5555_5555);
    let got = emu.call(0x65b250, 0, 0, &[0, 0x7fff_ffff, read as u32]) as i32;
    assert_eq!(
        (got, emu.read_u32(read)),
        utf8_to_utf16_len(None, 0x7fff_ffff)
    );
}
