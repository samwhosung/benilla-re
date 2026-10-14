//! Bit-exact tests of the CharSections-DBC fetch family (`0x47a3e0`, `0x47a400`, `0x47a440`,
//! `0x47a4a0`) against `WoW.exe`. Each test sets `[0xb4231c]` to a scratch record array, places a
//! record at `base + 0x28*idx` (idx = sex + 2*race) per race and sex, runs the function, and compares
//! with the transcription fed the same fields read back from emulator memory, so the index, the
//! guard and the select are all compared.
use benilla_re_charactermodel as cm;
use benilla_re_difftest::{Emu, Wow};

const DBC: u64 = 0xb4231c;
const STRIDE: u64 = 0x28;

fn idx(race: u32, sex: u32) -> u64 {
    u64::from(sex + 2 * race)
}

/// Build a CharSections record array of 8 records (idx 0..7) + sub-arrays, with a mix of
/// zero/non-zero guard fields and signed counts. Returns `base`.
fn setup(emu: &mut Emu) -> u64 {
    let base = emu.alloc(8 * STRIDE);
    let subs = emu.alloc(0x800);
    emu.write_u32(DBC, base as u32);
    for i in 0..8u64 {
        let r = base + STRIDE * i;
        // sub-ptr present for some idx, null for others (guard variety)
        let sub_present = i % 3 != 0;
        let sub_1c = if sub_present { subs + 0x40 * i } else { 0 };
        let sub_4 = if i % 2 == 0 {
            subs + 0x40 * i + 0x20
        } else {
            0
        };
        let sub_c = if i % 4 != 0 {
            subs + 0x40 * i + 0x10
        } else {
            0
        };
        // counts: a mix of negative / zero / positive (signed compares)
        let cnt = (i as i32) - 2; // -2,-1,0,1,2,3,4,5
        emu.write_u32(r, cnt as u32);
        emu.write_u32(r + 0x04, sub_4 as u32);
        emu.write_u32(r + 0x08, (cnt + 1) as u32);
        emu.write_u32(r + 0x0c, sub_c as u32);
        emu.write_u32(r + 0x18, (cnt + 3) as u32);
        emu.write_u32(r + 0x1c, sub_1c as u32);
        // fill the sub-arrays with distinct values
        for k in 0..16u64 {
            emu.write_u32(
                subs + 0x40 * i + 4 * k,
                0x9000 + (i as u32) * 0x100 + k as u32,
            );
        }
    }
    base
}

fn rec(base: u64, race: u32, sex: u32) -> u64 {
    base + STRIDE * idx(race, sex)
}

#[test]
fn charsec_a() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let base = setup(&mut emu);
    let mut total = 0u32;
    for race in 0..4 {
        for sex in 0..2 {
            let r = rec(base, race, sex);
            let bin = emu.call(0x47a3e0, race, sex, &[]);
            let exp = cm::charsec_a(emu.read_u32(r + 0x1c), emu.read_u32(r + 0x18));
            assert_eq!(bin, exp, "charsec_a race={race} sex={sex}");
            total += 1;
        }
    }
    eprintln!("charsec_a: {total} bit-exact");
}

#[test]
fn charsec_b() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let base = setup(&mut emu);
    let mut total = 0u32;
    for race in 0..4 {
        for sex in 0..2 {
            for arg in 0u32..6 {
                let r = rec(base, race, sex);
                let rec_1c = emu.read_u32(r + 0x1c);
                let rec_18 = emu.read_u32(r + 0x18);
                let sub = if rec_1c != 0 {
                    emu.read_u32(u64::from(rec_1c) + u64::from(12 * arg))
                } else {
                    0
                };
                let bin = emu.call(0x47a400, race, sex, &[arg]);
                assert_eq!(
                    bin,
                    cm::charsec_b(rec_1c, rec_18, arg, sub),
                    "charsec_b race={race} sex={sex} arg={arg}"
                );
                total += 1;
            }
        }
    }
    eprintln!("charsec_b: {total} bit-exact");
}

#[test]
fn charsec_c() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let base = setup(&mut emu);
    let mut total = 0u32;
    for race in 0..4 {
        for sex in 0..2 {
            let r = rec(base, race, sex);
            let rec_0 = emu.read_u32(r);
            let rec_4 = emu.read_u32(r + 0x4);
            let sub4 = if rec_4 != 0 {
                emu.read_u32(u64::from(rec_4) + 4)
            } else {
                0
            };
            let bin = emu.call(0x47a440, race, sex, &[]);
            assert_eq!(
                bin,
                cm::charsec_c(rec_0, rec_4, sub4),
                "charsec_c race={race} sex={sex}"
            );
            total += 1;
        }
    }
    eprintln!("charsec_c: {total} bit-exact");
}

#[test]
fn charsec_e() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let base = setup(&mut emu);
    let mut total = 0u32;
    for race in 0..4 {
        for sex in 0..2 {
            let r = rec(base, race, sex);
            let bin = emu.call(0x47a4a0, race, sex, &[]);
            let exp = cm::charsec_e(emu.read_u32(r + 0xc), emu.read_u32(r + 0x8));
            assert_eq!(bin, exp, "charsec_e race={race} sex={sex}");
            total += 1;
        }
    }
    eprintln!("charsec_e: {total} bit-exact");
}
