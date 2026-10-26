//! `0x7a8e50`, which maps a sound file cache handle to its entry and slot, run whole in the
//! emulator against `file_cache_entry`: the entry array pointer `0xcf56e8` and the handle cross
//! zero, small and large values, every slot byte, and a fuzz.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_sound::file_cache_entry;

const ENTRY: u64 = 0x7a8e50;
const ENTRIES: u64 = 0xcf56e8;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

#[test]
fn file_cache_entry_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out = emu.alloc(4);
    let check = |emu: &mut Emu, entries: u32, handle: u32| {
        emu.write_u32(ENTRIES, entries);
        emu.write_u32(out, 0x5a5a_5a5a);
        let got = emu.call(ENTRY, handle, out as u32, &[]);
        let slot = emu.read_u32(out);
        assert_eq!(
            emu.get_reg(RegisterX86::ECX) as u32,
            (handle & 0xff).wrapping_sub(1)
        );
        assert_eq!(
            (got, slot),
            file_cache_entry(entries, handle),
            "entries {entries:#x}, handle {handle:#x}"
        );
    };
    for entries in [0, 0x0123_4560, 0xffff_ff00] {
        for index in [0u32, 1, 2, 0x7f, 0x100, 0xff_ffff] {
            for slot in 0..=0xffu32 {
                check(&mut emu, entries, index << 8 | slot);
            }
        }
    }
    let mut seed = 0x7a8e_5001;
    for _ in 0..20_000 {
        let (e, h) = (next(&mut seed), next(&mut seed));
        check(&mut emu, e, h);
    }
}
