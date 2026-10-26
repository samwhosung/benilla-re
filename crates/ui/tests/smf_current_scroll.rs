//! `0x788ac0`, the scrolling message frame's current scroll, against the exe, run whole
//! (`systems/ui/messageframe.md#the-current-scroll`), over the count `+0x33c`, the write
//! position `+0x344` and the scroll position `+0x348`.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::message_scroll::current_scroll;

const CURRENT_SCROLL: u64 = 0x788ac0;

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

#[test]
fn smf_current_scroll_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x3c4);
    let mut cases: Vec<(i32, i32, i32)> = Vec::new();
    let edges = [0, 1, 2, 23, 127, 128, 0x7fff_ffff, -2, -128, i32::MIN + 1];
    for &count in &[0, 1, 2, 3, 24, 128, 500, 0x7fff_ffff, -2, -5] {
        for &position in &[-1, 0, 1, 23, 127, 499, 0x7fff_fffe] {
            for &scroll in &edges {
                cases.push((count, position, scroll));
            }
        }
    }
    let mut seed = 0x1234_u32;
    for _ in 0..2000 {
        let count = (lcg(&mut seed) >> 8) as i32 % 600;
        let position = (lcg(&mut seed) >> 8) as i32 % 600 - 1;
        let scroll = (lcg(&mut seed) >> 8) as i32 % 600;
        cases.push((count, position, scroll));
    }
    for (count, position, scroll) in cases {
        // `idiv` faults for the dividend i32::MIN over −1, which no count of −1 reaches here.
        e.write_u32(frame + 0x33c, count as u32);
        e.write_u32(frame + 0x344, position as u32);
        e.write_u32(frame + 0x348, scroll as u32);
        let got = e.call(CURRENT_SCROLL, frame as u32, 0, &[]) as i32;
        assert_eq!(
            got,
            current_scroll(count, position, scroll),
            "count {count} position {position} scroll {scroll}"
        );
    }
}
