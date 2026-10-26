//! `AddFontStrings`' core `0x529940` against the exe, run whole
//! (`systems/ui/tooltip.md#adding-font-strings`). The two array resizers it calls,
//! `SetArrayCapacity536c80` (`0x536c80`) for the string arrays and its copy `0x4368c0` for the
//! wrap flags, are stubbed to log their array and capacity and to store the capacity, as their
//! first store does; the arrays' data is allocated large enough that no move is needed, so the
//! stubs leave the data where it is. Limits stay below 0xffffffff, whose wrap to 0 frees the
//! arrays and then stores below their data.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::tooltip_font_strings::{add_font_strings, LineArray, Lines};
use std::cell::RefCell;
use std::rc::Rc;

const ADD: u64 = 0x529940;
const RESIZE: u64 = 0x536c80;
const RESIZE_COPY: u64 = 0x4368c0;
const ARRAYS: [u64; 3] = [0x324, 0x330, 0x33c];
const CAP: usize = 0x80;

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s >> 8
}

#[test]
fn tooltip_add_font_strings_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let log: Rc<RefCell<Vec<(u64, u32, u32)>>> = Rc::default();
    for addr in [RESIZE, RESIZE_COPY] {
        let log = log.clone();
        e.stub_fn(addr, 1, move |io| {
            let this = io.reg(RegisterX86::ECX);
            let cap = io.arg(0);
            log.borrow_mut().push((addr, this as u32, cap));
            io.write_u32(this, cap);
            0
        });
    }
    let tip = e.alloc(0x460);
    let data: Vec<u64> = (0..3).map(|_| e.alloc(4 * CAP as u64)).collect();
    let mut seed = 0x5eed_u32;
    let mut cases: Vec<(u32, [u32; 3])> = Vec::new();
    for limit in [0u32, 1, 2, 7, 29, 30, 31, 0x7e] {
        // Counts equal to the new limit, to the old one, and mixed.
        cases.push((limit, [limit + 1; 3]));
        cases.push((limit, [limit; 3]));
        cases.push((limit, [limit + 1, limit, limit + 1]));
        cases.push((limit, [limit, limit + 1, 0]));
    }
    for _ in 0..64 {
        let limit = lcg(&mut seed) % 0x7f;
        let c = |s: &mut u32| match lcg(s) % 3 {
            0 => limit,
            1 => limit + 1,
            _ => lcg(s) % 0x80,
        };
        cases.push((limit, [c(&mut seed), c(&mut seed), c(&mut seed)]));
    }
    for (limit, counts) in cases {
        let left = 0x0100_0000 | lcg(&mut seed);
        let right = 0x0200_0000 | lcg(&mut seed);
        let mut fill = Vec::new();
        for (k, &off) in ARRAYS.iter().enumerate() {
            let words: Vec<u32> = (0..CAP).map(|_| lcg(&mut seed) | 1).collect();
            for (i, w) in words.iter().enumerate() {
                e.write_u32(data[k] + 4 * i as u64, *w);
            }
            let cap = lcg(&mut seed) % 0x100;
            e.write_u32(tip + off, cap);
            e.write_u32(tip + off + 4, counts[k]);
            e.write_u32(tip + off + 8, data[k] as u32);
            fill.push(LineArray {
                capacity: cap,
                count: counts[k],
                data: words,
            });
        }
        e.write_u32(tip + 0x320, limit);
        let mut want = Lines {
            limit,
            left: fill[0].clone(),
            right: fill[1].clone(),
            wrap: fill[2].clone(),
        };
        log.borrow_mut().clear();
        e.call(ADD, tip as u32, 0, &[left, right]);
        let resizes = add_font_strings(&mut want, left, right);
        let what = format!("limit {limit} counts {counts:?}");
        assert_eq!(e.read_u32(tip + 0x320), want.limit, "{what}: limit");
        for (k, (&off, a)) in ARRAYS
            .iter()
            .zip([&want.left, &want.right, &want.wrap])
            .enumerate()
        {
            assert_eq!(
                e.read_u32(tip + off),
                a.capacity,
                "{what}: array {k} capacity"
            );
            assert_eq!(
                e.read_u32(tip + off + 4),
                a.count,
                "{what}: array {k} count"
            );
            assert_eq!(
                e.read_u32(tip + off + 8),
                data[k] as u32,
                "{what}: array {k} data"
            );
            for (i, w) in a.data.iter().enumerate() {
                assert_eq!(
                    e.read_u32(data[k] + 4 * i as u64),
                    *w,
                    "{what}: array {k}[{i}]"
                );
            }
        }
        let calls: Vec<(u64, u32, u32)> = resizes
            .iter()
            .map(|&(k, cap)| {
                let addr = if k == 2 { RESIZE_COPY } else { RESIZE };
                (addr, (tip + ARRAYS[k as usize]) as u32, cap)
            })
            .collect();
        assert_eq!(log.borrow().as_slice(), calls.as_slice(), "{what}: resizes");
    }
}
