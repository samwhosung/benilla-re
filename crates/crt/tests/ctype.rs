//! Character classes and case (`systems/crt/ctype.md`): `isalpha` (`0x73fb91`) and `toupper`
//! (`0x41089b`) run whole against `benilla_re_crt::ctype`, `isalpha` for EOF and every byte,
//! `toupper` for every value from -0x200 to 0x2ff and the ends of the range.

use benilla_re_crt::ctype::{isalpha, toupper};
use benilla_re_difftest::{Emu, Wow};

const ISALPHA: u64 = 0x73fb91;
const TOUPPER: u64 = 0x41089b;

#[test]
fn crt_isalpha_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for c in -1..=0xff {
        let got = e.call(ISALPHA, 0, 0, &[c as u32]) as i32;
        assert_eq!(got, isalpha(c), "isalpha({c:#x})");
    }
}

#[test]
fn crt_toupper_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for c in (-0x200..0x300).chain([i32::MIN, i32::MAX]) {
        let got = e.call(TOUPPER, 0, 0, &[c as u32]) as i32;
        assert_eq!(got, toupper(c), "toupper({c:#x})");
    }
}
