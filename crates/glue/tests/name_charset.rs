//! The character name charsets against the exe (`systems/glue/enterworld.md`, Name charsets).
//! `GetNameCharsets` (`0x6c9a70`) runs whole for every region and locale index around the bounds,
//! with the region written into `0x86c9e0`. The charset tests run whole on all 65,536 units, the
//! upper half of `ecx` set on odd units to show it is not read: `IsInNameCharset` (`0x6c9dd0`) for
//! sets 0 to 6, and `IsLatin1Letter` (`0x6c9c60`), `IsHangulNameSyllable` (`0x6c9d50`),
//! `IsCjkIdeograph` (`0x6c9d80`), `IsCjkNameUnit` (`0x6c9dc0`) and `IsAsciiLetter` (`0x6c9ce0`)
//! called directly. Those that return in `al` are compared in `al`; the others in all of `eax`.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_glue::name_charset::{
    in_name_charset, is_ascii_letter, is_cjk_ideograph, is_hangul_name_syllable, is_latin1_letter,
    name_charsets,
};

const GET_NAME_CHARSETS: u64 = 0x6c9a70;
const REGION: u64 = 0x86c9e0;
const IN_NAME_CHARSET: u64 = 0x6c9dd0;
const LATIN1: u64 = 0x6c9c60;
const HANGUL: u64 = 0x6c9d50;
const CJK: u64 = 0x6c9d80;
const CJK_THUNK: u64 = 0x6c9dc0;
const ASCII: u64 = 0x6c9ce0;
const SYLLABLES: u64 = 0x86ca30;

#[test]
fn name_charset_pair_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out = e.alloc(0x10);
    let mut n = 0u32;
    let regions = [i32::MIN, -1, 0, 1, 2, 3, 4, 5, 6, 7, i32::MAX];
    for &region in &regions {
        for locale in (-3..=10).chain([i32::MIN, i32::MAX]) {
            e.write_u32(REGION, region as u32);
            e.write_u32(out, 0xdead_beef);
            e.write_u32(out + 4, 0xdead_beef);
            e.call(
                GET_NAME_CHARSETS,
                locale as u32,
                out as u32,
                &[(out + 4) as u32],
            );
            let bin = (e.read_u32(out), e.read_u32(out + 4));
            assert_eq!(
                bin,
                name_charsets(region, locale),
                "region {region} locale {locale}"
            );
            n += 1;
        }
    }
    eprintln!("name_charset_pair: {n} cases bit-exact vs WoW.exe @ 0x6c9a70");
}

#[test]
fn name_charset_units_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let syllables: Vec<u16> = e
        .read(SYLLABLES, 0x125c)
        .chunks(2)
        .map(|w| u16::from_le_bytes([w[0], w[1]]))
        .collect();
    let al = |v: u32| v & 0xff != 0;
    let mut n = 0u32;
    for unit in 0..=0xffffu32 {
        let u = unit as u16;
        let ecx = if unit % 2 == 1 {
            unit | 0xabcd_0000
        } else {
            unit
        };
        for set in 0..=6u32 {
            let bin = al(e.call(IN_NAME_CHARSET, ecx, set, &[]));
            assert_eq!(
                bin,
                in_name_charset(u, set, &syllables),
                "set {set} unit {u:#x}"
            );
            n += 1;
        }
        assert_eq!(
            al(e.call(LATIN1, ecx, 0, &[])),
            is_latin1_letter(u),
            "{u:#x}"
        );
        assert_eq!(
            al(e.call(HANGUL, ecx, 0, &[])),
            is_hangul_name_syllable(u, &syllables),
            "{u:#x}"
        );
        let cjk = u32::from(is_cjk_ideograph(u));
        assert_eq!(e.call(CJK, ecx, 0, &[]), cjk, "{u:#x}");
        assert_eq!(e.call(CJK_THUNK, ecx, 0, &[]), cjk, "{u:#x}");
        assert_eq!(
            e.call(ASCII, ecx, 0, &[]),
            u32::from(is_ascii_letter(u)),
            "{u:#x}"
        );
        n += 5;
    }
    eprintln!("name_charset_units: {n} cases bit-exact vs WoW.exe @ 0x6c9dd0..0x6c9d50");
}
