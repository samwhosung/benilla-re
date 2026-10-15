//! Storm's hash-table string hash, `SStrHashHT` (`0x64b3f0`), with `bjhash` (`0x64b1d0`) inside it.

mod common;

use benilla_re_difftest::Emu;
use common::Wow;

/// Runs `0x64b3f0(name)` (`ret 4`) whole over names of every length from 0 to 40 (each tail length
/// of `bjhash`'s 12-byte blocks, three times), names with lower case, `/`, `\` and bytes above
/// 0x7f, names at and past the 1023-byte cap, and a fuzz, and compares with `str_hash_ht`.
#[test]
fn str_hash_ht() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let buf = emu.alloc(0x1000);
    let mut names: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"Interface\\AddOns\\Foo\\Foo.toc".to_vec(),
        b"interface/addons/foo/foo.toc".to_vec(),
        b"FRIZQT__.TTF".to_vec(),
        b"Fonts\\frizqt__.ttf".to_vec(),
        b"`{@[az AZ/\\\x7f\x80\xc3\xa9\xff".to_vec(),
    ];
    for len in 0..=40usize {
        names.push((0..len).map(|i| b"aZ/\\.0~"[i % 7]).collect());
    }
    for len in [1021usize, 1022, 1023, 1024, 1025, 1100, 2000] {
        names.push((0..len).map(|i| b'a' + (i % 26) as u8).collect());
    }
    let mut s: u32 = 0x2545_f491;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..500 {
        let len = (next() >> 8) as usize % 120;
        names.push((0..len).map(|_| ((next() >> 16) as u8).max(1)).collect());
    }
    for name in &names {
        let mut z = name.clone();
        z.push(0);
        emu.write(buf, &z);
        let got = emu.call(0x64b3f0, 0, 0, &[buf as u32]);
        assert_eq!(
            got,
            benilla_re_storm::str_hash_ht(name),
            "SStrHashHT mismatch on {} bytes: {:?}",
            name.len(),
            String::from_utf8_lossy(&name[..name.len().min(40)])
        );
    }
}
