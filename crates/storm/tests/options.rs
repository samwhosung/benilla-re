//! Storm's option setter `0x64ec80`, `__stdcall(id, data, size)`, run whole. `SErrSetLastError`
//! (`0x64e850`, `ret 4`), which stores the code and calls `SetLastError`, is a stub that records
//! the code; the codes are compared in order.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_storm::options::{set_option, OPTION_CELLS};

const SET_OPTION: u64 = 0x64ec80;
const SERR_SET_LAST_ERROR: u64 = 0x64e850;

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

#[test]
fn set_option_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(SERR_SET_LAST_ERROR, 0, 1);
    let data = emu.alloc(8);
    let cells: Vec<u32> = OPTION_CELLS
        .iter()
        .flat_map(|(_, c)| c.iter().copied())
        .collect();
    let mut s = 0x64ec_8000u32;
    let mut values = vec![
        0u32,
        1,
        2,
        3,
        4,
        6,
        0x8000_0000,
        0x8000_0001,
        0xffff_ffff,
        0x4000_0000,
    ];
    for _ in 0..40 {
        values.push(lcg(&mut s));
    }
    // Option 2's 32-bit test: every power of two and every 2^31 + 2^k pass, a step off fails.
    for k in 0..32 {
        values.push(1 << k);
        values.push((1u32 << 31) | (1 << k));
        values.push((1u32 << k).wrapping_add(1) | 2);
    }
    let mut cases = 0u32;
    for id in 0..=15u32 {
        for size in [0u32, 3, 4, 5, 8, 12] {
            for (k, &v) in values.iter().enumerate() {
                for null in [false, true] {
                    if null && k > 0 {
                        continue;
                    }
                    let hi = v.rotate_left(7) ^ 0x5a5a_5a5a;
                    for &c in &cells {
                        emu.write_u32(u64::from(c), 0xdead_0000 | c & 0xffff);
                    }
                    emu.write_u32(data, v);
                    emu.write_u32(data + 4, hi);
                    emu.clear_calls();
                    let ptr = if null { 0 } else { data as u32 };
                    let got = emu.call(SET_OPTION, 0, 0, &[id, ptr, size]);
                    let codes: Vec<u32> = emu.calls().iter().map(|c| c.args[0]).collect();
                    let got_cells: Vec<u32> =
                        cells.iter().map(|&c| emu.read_u32(u64::from(c))).collect();

                    let mut want_cells: Vec<u32> =
                        cells.iter().map(|&c| 0xdead_0000 | c & 0xffff).collect();
                    let input = (!null).then_some([v, hi]);
                    let (ok, want_codes) = set_option(id, input, size, &mut |cell, value| {
                        let at = cells.iter().position(|&c| c == cell).expect("a cell");
                        want_cells[at] = value;
                    });
                    let ctx = format!("id={id} size={size} v={v:#x} null={null}");
                    assert_eq!(got, u32::from(ok), "{ctx}: return");
                    assert_eq!(codes, want_codes, "{ctx}: error codes");
                    assert_eq!(got_cells, want_cells, "{ctx}: cells");
                    cases += 1;
                }
            }
        }
    }
    eprintln!("set_option: {cases} cases bit-exact vs WoW.exe @ 0x64ec80");
}
