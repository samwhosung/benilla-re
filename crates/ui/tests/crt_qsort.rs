//! The C runtime's `qsort` (`0x73f727`), run whole with its helpers `shortsort` (`0x73f87b`) and
//! `swap` (`0x73f8c9`) over records the test builds in scratch memory, against the model in
//! `benilla_re_ui::crt_qsort`.
//!
//! The comparator is a stub at a padding address, so the test reads every call the exe makes: the
//! two records it passes, in order. The trace and the final bytes of every record must match the
//! model for each case. A record of 4 bytes or more carries a 16-bit key and a 16-bit id in its
//! first dword, so equal keys are told apart and the order that ties end in is compared; a record
//! of 1 to 3 bytes is its key. The widths 1 to 3 are where the partition test
//! `higuy - 1 - lo >= hi - loguy` can be equal. The comparator returns the sign three ways (-1/0/1,
//! the extremes of `int`, a key difference) and in reverse, to show that only the sign is read.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::crt_qsort::qsort;

const QSORT: u64 = 0x0073_f727;
/// Padding after `swap`: nothing executes it, so the stub stands in for the comparator.
const COMPARATOR: u64 = 0x0073_f8f5;
/// Records in the largest case.
const MAX_RECORDS: usize = 1200;
const MAX_WIDTH: usize = 32;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s >> 8
}

/// The comparator's `int` for keys `a` and `b` (both below 65536) in the given mode.
fn answer(mode: u32, a: u32, b: u32) -> i32 {
    let (a, b) = (a as i32, b as i32);
    match mode {
        0 => (a - b).signum(),
        1 => match a.cmp(&b) {
            std::cmp::Ordering::Less => i32::MIN,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => i32::MAX,
        },
        2 => a - b,
        _ => (b - a).signum(),
    }
}

/// A record of `width` bytes. From 4 bytes its first dword is `key << 16 | id` and the rest is a
/// pattern of the id, so a swap that moves too few bytes shows; below 4 it is the key alone.
fn record(width: usize, key: u32, id: u32) -> Vec<u8> {
    if width < 4 {
        return key.to_le_bytes()[..width].to_vec();
    }
    let mut r = ((key << 16) | id).to_le_bytes().to_vec();
    for j in 4..width {
        r.push((id.wrapping_mul(7) as u8).wrapping_add(j as u8));
    }
    r
}

/// The comparator's view of a record: its first dword, or the whole record below 4 bytes.
fn header(r: &[u8]) -> u32 {
    r.iter()
        .take(4)
        .enumerate()
        .fold(0, |h, (i, &b)| h | u32::from(b) << (8 * i))
}

/// The key a comparator reads from a header, given the record width.
fn key_of(width: usize, h: u32) -> u32 {
    if width < 4 {
        h
    } else {
        h >> 16
    }
}

/// The keys of one case: `n` of them, drawn by `pattern`.
fn keys(pattern: u32, n: usize, seed: &mut u32) -> Vec<u32> {
    if pattern >= 12 {
        // A run of equal keys with a few smaller (12) or larger (13) ones at random places: the
        // scans pass the ties from both sides, so the parts come out lopsided.
        let mut ks = vec![100u32; n];
        for _ in 0..1 + next(seed) % 3 {
            if n > 0 {
                let at = next(seed) as usize % n;
                ks[at] = if pattern == 12 {
                    next(seed) % 100
                } else {
                    101 + next(seed) % 50
                };
            }
        }
        return ks;
    }
    (0..n as u32)
        .map(|i| match pattern {
            0 => next(seed) % 3,
            1 => next(seed) % 6,
            2 => next(seed) % 50,
            3 => next(seed) & 0xffff,
            4 => i,
            5 => n as u32 - i,
            6 => 7,
            7 => {
                // Organ pipe: up, then down.
                let h = n as u32 / 2;
                if i < h {
                    i
                } else {
                    n as u32 - i
                }
            }
            8 => i % 5,
            9 => (i % 5) * 1000 + next(seed) % 3,
            // Mostly sorted with a few displaced.
            10 => {
                if next(seed).is_multiple_of(10) {
                    next(seed) % 100
                } else {
                    i
                }
            }
            _ => 1 - (i & 1),
        })
        .collect()
}

#[test]
fn crt_qsort_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let base = emu.alloc((MAX_RECORDS * MAX_WIDTH) as u64);

    let mode = Rc::new(Cell::new(0u32));
    let width_cell = Rc::new(Cell::new(0usize));
    let trace = Rc::new(RefCell::new(Vec::<(u32, u32)>::new()));
    {
        let mode = Rc::clone(&mode);
        let width_cell = Rc::clone(&width_cell);
        let trace = Rc::clone(&trace);
        emu.stub_fn(COMPARATOR, 0, move |io| {
            let w = width_cell.get();
            let mut h = [0u32; 2];
            for (i, h) in h.iter_mut().enumerate() {
                *h = header(&io.read(u64::from(io.arg(i as u64)), w.min(4)));
            }
            trace.borrow_mut().push((h[0], h[1]));
            answer(mode.get(), key_of(w, h[0]), key_of(w, h[1])) as u32
        });
    }

    let mut cases = 0u32;
    let mut calls = 0usize;
    // Runs one case in the exe and in the model and compares the comparator calls, the bytes and
    // the order of the result.
    let mut check = |emu: &mut Emu, width: usize, ks: &[u32], m: u32, what: &dyn Fn() -> String| {
        let n = ks.len();
        let recs: Vec<Vec<u8>> = ks
            .iter()
            .enumerate()
            .map(|(i, &k)| record(width, k, i as u32))
            .collect();
        for (i, r) in recs.iter().enumerate() {
            emu.write(base + (i * width) as u64, r);
        }
        mode.set(m);
        width_cell.set(width);
        trace.borrow_mut().clear();
        emu.call(
            QSORT,
            0,
            0,
            &[base as u32, n as u32, width as u32, COMPARATOR as u32],
        );
        let exe_trace = trace.borrow().clone();
        let exe_bytes = emu.read(base, n * width);

        let mut model = recs.clone();
        let mut model_trace = Vec::new();
        qsort(&mut model, width, |a, b| {
            let (ha, hb) = (header(a), header(b));
            model_trace.push((ha, hb));
            answer(m, key_of(width, ha), key_of(width, hb))
        });
        let model_bytes: Vec<u8> = model.concat();

        assert_eq!(exe_trace, model_trace, "comparator calls, {}", what());
        assert_eq!(exe_bytes, model_bytes, "sorted records, {}", what());
        // The comparator's own order holds in the result.
        for w in model.windows(2) {
            assert!(
                answer(
                    m,
                    key_of(width, header(&w[0])),
                    key_of(width, header(&w[1]))
                ) <= 0,
                "order, {}",
                what()
            );
        }
        cases += 1;
        calls += exe_trace.len();
    };

    let mut seed = 0x07a3_f727_u32;
    let sizes = [
        0usize, 1, 2, 3, 7, 8, 9, 10, 11, 12, 15, 16, 17, 18, 24, 25, 26, 31, 32, 33, 40, 64, 65,
        100, 129, 257, 1000, 1200,
    ];
    for &width in &[1usize, 2, 3, 4, 8, 12, 16, 32] {
        for &n in &sizes {
            for pattern in 0..14u32 {
                for m in 0..4u32 {
                    // Large cases only for some patterns and modes: the cost is n log n.
                    if n > 300 && (pattern % 3 != 0 || m > 1) {
                        continue;
                    }
                    let ks: Vec<u32> = keys(pattern, n, &mut seed)
                        .into_iter()
                        .map(|k| if width == 1 { k & 0xff } else { k })
                        .collect();
                    check(&mut emu, width, &ks, m, &|| {
                        format!("width={width} n={n} pattern={pattern} mode={m}")
                    });
                }
            }
        }
    }

    // Every array of 9 and 10 keys from three values, at the widths where the partition test is
    // equal (1) and not (4). The ranges above 8 elements split in every way ties allow: a part of
    // 0, 1, 2 or 3 elements, a part that is not kept, and both sides of `higuy - 1 - lo >= hi - loguy`.
    for &width in &[1usize, 4] {
        for n in [9usize, 10] {
            for code in 0..3u32.pow(n as u32) {
                let ks: Vec<u32> = (0..n as u32).map(|i| code / 3u32.pow(i) % 3).collect();
                check(&mut emu, width, &ks, 0, &|| {
                    format!("width={width} n={n} keys={ks:?}")
                });
            }
        }
    }

    // Fewer than two elements, or a zero width: no comparator call and no change.
    for &(n, width) in &[(0usize, 8usize), (1, 8), (5, 0), (0, 0), (1, 0)] {
        let before: Vec<u8> = (0..64u8).collect();
        emu.write(base, &before);
        trace.borrow_mut().clear();
        emu.call(
            QSORT,
            0,
            0,
            &[base as u32, n as u32, width as u32, COMPARATOR as u32],
        );
        assert!(trace.borrow().is_empty(), "n={n} width={width}");
        assert_eq!(emu.read(base, 64), before, "n={n} width={width}");
        cases += 1;
    }
    eprintln!("crt_qsort_difftest: {cases} cases, {calls} comparator calls bit-exact");
}
