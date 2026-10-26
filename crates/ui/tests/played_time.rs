//! `0x49e9d0` against [`duration_parts`]: a count of seconds at every unit's boundary, a step either
//! side, both signs, the 32-bit extremes and a fuzz, with `SStrPrintf` stubbed to record the
//! buffer, the size, the format and the four numbers it is passed.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::time_values::duration_parts;

const FORMAT_DURATION: u64 = 0x49e9d0;
const SSTR_PRINTF: u64 = 0x64a7f0;
/// `%dd %dh %dm %ds`.
const FORMAT: u32 = 0x0084_49f8;

#[test]
fn format_duration_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(SSTR_PRINTF, 0, 0);
    let buf = e.alloc(0x100);
    let mut counts: Vec<i32> = vec![i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1];
    for unit in [1i32, 60, 3_600, 86_400] {
        for k in [0i32, 1, 2, 23, 24, 59, 60, 1_000] {
            let base = unit.saturating_mul(k);
            for d in [-1i32, 0, 1] {
                counts.push(base.saturating_add(d));
                counts.push(base.saturating_add(d).saturating_neg());
            }
        }
    }
    let mut s: u32 = 0x49e9_0001;
    for _ in 0..20_000 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        counts.push(match s >> 30 {
            0 => s as i32,
            1 => (s >> 8) as i32,
            2 => -((s >> 12) as i32),
            _ => (s >> 16) as i32,
        });
    }
    for &n in &counts {
        e.clear_calls();
        e.call(FORMAT_DURATION, buf as u32, 0x100, &[n as u32]);
        let calls = e.calls();
        assert_eq!(calls.len(), 1, "one call for {n}");
        let a = calls[0].args;
        let parts = duration_parts(n);
        assert_eq!(
            a[..7],
            [
                buf as u32,
                0x100,
                FORMAT,
                parts[0] as u32,
                parts[1] as u32,
                parts[2] as u32,
                parts[3] as u32
            ],
            "{n}"
        );
    }
}
