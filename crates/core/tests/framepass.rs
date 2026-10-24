//! The frame pass against the exe (`systems/core/framepass.md`): the frame-time handler
//! `0x442330` whole, the region calls `0x65c610`, `0x65bd90` (modes 2 and 4), `0x65ca80` and
//! `0x65c710` whole, and `RunFramePass` (`0x442350`) whole over layers that `ScrnLayerCreate`
//! (`0x442800`) links.
//!
//! The region calls run with their callees; Storm's allocator (`SMemAlloc` `0x6462e0`,
//! `SMemReAlloc` `0x646320`, `SMemFree` `0x646430`) is a bump arena, `SErrSetLastError`
//! (`0x64e850`) is stubbed, and the critical-section imports (`0x7ff2d4`, `0x7ff2d8`, `0x7ff2dc`)
//! point at a `ret 4`. The handle table's static initializer `0x65b970` runs first, as the C
//! runtime's table runs it. In `0x442350` the device calls are stubbed and logged: the viewport
//! read and set (`0x58b060`, `0x58af60`), the projection (`0x5c3d90`, `0x58b050`, `0x58b040`), the
//! font frame (`0x5c2ce0`), the present (`0x58a960`), the device's active flag (`0x58a260`) and
//! `OsSleep` (`0x429d50`); the screenshot slot `0x88560c` is empty. Each layer's callback is a
//! stub that logs its `ecx`, `edx`, the visible rectangle its first argument points at and the
//! elapsed time of its second.
//!
//! Inputs: for the region calls, the frame pass's use of them, the unit square with mode 2 and then
//! mode 4 rectangles; rectangles on a grid of quarters in [−0.25, 1.25], degenerate and inverted
//! ones included; one to six layers with every combination of flag bits 0, 1 and 2, and priorities
//! with ties; the frame time and the sum as finite floats of both signs, zero, the infinities and
//! both sides of the 53-bit rounding. No NaN is among them: the frame time is `dt_seconds` of an
//! `i32`.

use std::cell::RefCell;
use std::rc::Rc;

use benilla_re_core::framepass::{
    frame_pass_layers, frame_time_accumulate, layer_insert_index, Layer, RectF, Region,
};
use benilla_re_difftest::{Emu, RegisterX86, Wow};

const ACC: u64 = 0x885608;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s >> 8
}

fn write_rect(emu: &mut Emu, p: u64, r: RectF) {
    for (i, v) in r.iter().enumerate() {
        emu.write_u32(p + 4 * i as u64, v.to_bits());
    }
}

fn read_rect(emu: &Emu, p: u64) -> RectF {
    [0, 1, 2, 3].map(|i| emu.read_f32(p + 4 * i))
}

fn bits(r: RectF) -> [u32; 4] {
    r.map(f32::to_bits)
}

/// An emulator with the region code and the layer list ready.
fn storm_emu(wow: &Wow) -> Emu<'_> {
    let mut emu = Emu::new(wow);
    let arena = emu.alloc(0xc0_0000);
    emu.stub_alloc_sized(0x6462e0, arena, arena + 0xc0_0000, 0, 4);
    emu.stub_realloc_sized(0x646320, 5);
    emu.stub(0x646430, 1, 4);
    emu.stub(0x64e850, 0, 1);
    let ret4 = emu.alloc(0x10);
    emu.write(ret4, &[0xc2, 0x04, 0x00]);
    for slot in [0x7ff2d4, 0x7ff2d8, 0x7ff2dc] {
        emu.write_u32(slot, ret4 as u32);
    }
    emu.call(0x65b970, 0, 0, &[]);
    emu
}

/// `0x442330` whole: `ecx` points at the frame time, the sum lands in `0x885608`, `eax` is 1.
#[test]
fn frame_time_accumulate_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let dt = emu.alloc(8);
    let mut checked = 0u64;
    let mut case = |emu: &mut Emu, acc: f32, d: f32| {
        emu.write_u32(ACC, acc.to_bits());
        emu.write_u32(dt, d.to_bits());
        emu.write_u32(dt + 4, 0xdead_beef);
        let eax = emu.call(0x442330, dt as u32, 0, &[]);
        assert_eq!(eax, 1, "acc={acc} dt={d}");
        let got = emu.read_u32(ACC);
        let ours = frame_time_accumulate(acc, d).to_bits();
        assert_eq!(
            got,
            ours,
            "acc={acc:e} ({:#x}) dt={d:e} ({:#x})",
            acc.to_bits(),
            d.to_bits()
        );
        checked += 1;
    };
    let edges = [
        0.0f32,
        -0.0,
        0.001,
        0.016,
        0.25,
        1.0,
        -1.0,
        2_147_483.6,
        -2_147_483.6,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        16_777_216.0,
        1.0 + f32::EPSILON,
    ];
    for &a in &edges {
        for &d in &edges {
            if a.is_infinite() && d.is_infinite() && a != d {
                continue; // the indefinite: outside the client's inputs
            }
            case(&mut emu, a, d);
        }
    }
    // The 53-bit rounding boundary: dt 2^-k ulps of acc, k around 29 and 30, both signs.
    for e in [-20i32, 0, 10, 20] {
        let a = 2f32.powi(e);
        for k in 20..=40 {
            for s in [1.0f32, -1.0] {
                let d = s * a * 2f32.powi(-k) * 1.5;
                case(&mut emu, a, d);
                case(&mut emu, a + a * f32::EPSILON, d);
            }
        }
    }
    let mut seed = 0x4423_3001u32;
    for _ in 0..20_000 {
        let a = f32::from_bits(next(&mut seed) << 8 | next(&mut seed) & 0xff);
        let d = f32::from_bits(next(&mut seed) << 8 | next(&mut seed) & 0xff);
        if !a.is_finite() || !d.is_finite() {
            continue;
        }
        case(&mut emu, a, d);
    }
    eprintln!("frame_time_accumulate: {checked} cases bit-exact vs WoW.exe @ 0x442330");
}

fn grid(s: &mut u32) -> f32 {
    (next(s) % 7) as f32 * 0.25 - 0.25
}

/// `0x65c610`, `0x65bd90` and `0x65ca80` whole: a region made, given the unit square with mode 2,
/// then a run of mode 4 rectangles, its bounding rectangle read after each: the frame pass's use.
/// [`Region`] holds for that use only; a mode 2 rectangle that overlaps a piece and is then cut
/// leaves a bounding rectangle it does not model.
#[test]
fn region_bounding_rect_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = storm_emu(&wow);
    let h = emu.alloc(4);
    let r = emu.alloc(16);
    let out = emu.alloc(16);
    let mut seed = 0x65bd_9001u32;
    let mut checked = 0u64;
    for case in 0..3000 {
        emu.write_u32(h, 0);
        emu.call(0x65c610, 0, 0, &[h as u32, 0]);
        let hd = emu.read_u32(h);
        assert_ne!(hd, 0, "a region handle");
        let mut ours = Region::default();
        write_rect(&mut emu, r, [0.0, 0.0, 1.0, 1.0]);
        emu.call(0x65bd90, 0, 0, &[hd, r as u32, 0, 2]);
        ours.add([0.0, 0.0, 1.0, 1.0]);
        let steps = 1 + next(&mut seed) % 6;
        let mut log = Vec::new();
        for _ in 0..steps {
            let rect = [
                grid(&mut seed),
                grid(&mut seed),
                grid(&mut seed),
                grid(&mut seed),
            ];
            write_rect(&mut emu, r, rect);
            emu.call(0x65bd90, 0, 0, &[hd, r as u32, 0, 4]);
            ours.subtract(rect);
            log.push(rect);
            write_rect(&mut emu, out, [9.0; 4]);
            emu.call(0x65ca80, 0, 0, &[hd, out as u32]);
            let got = read_rect(&emu, out);
            assert_eq!(
                bits(got),
                bits(ours.bounding_rect()),
                "case {case}: after {log:?}: binary {got:?} ours {:?}",
                ours.bounding_rect()
            );
            checked += 1;
        }
        emu.call(0x65c710, 0, 0, &[hd]);
    }
    eprintln!("region_bounding_rect: {checked} bounding rectangles bit-exact vs WoW.exe");
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Called {
    callback: u64,
    ecx: u32,
    edx: u32,
    rect_at: u32,
    rect: [u32; 4],
    elapsed: u32,
}

/// `0x442350` whole over layers linked by `0x442800`: which callbacks run, in what order, with
/// what visible rectangle and elapsed time; the accumulator cleared; `OsSleep(250)` exactly when
/// the device is inactive.
#[test]
fn frame_pass_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = storm_emu(&wow);
    emu.stub(0x41af10, 0x77, 0); // CreateHandle
    for (addr, pop) in [
        (0x58b060u64, 4u32),
        (0x58af60, 6),
        (0x5c3d90, 6),
        (0x58b050, 0),
        (0x58b040, 0),
        (0x5c2ce0, 0),
        (0x58a960, 0),
    ] {
        emu.stub(addr, 0, pop);
    }
    let active = Rc::new(RefCell::new(1u32));
    {
        let active = active.clone();
        emu.stub_fn(0x58a260, 0, move |_| *active.borrow());
    }
    let sleeps: Rc<RefCell<Vec<u32>>> = Rc::default();
    {
        let sleeps = sleeps.clone();
        emu.stub_fn(0x429d50, 0, move |io| {
            sleeps.borrow_mut().push(io.reg(RegisterX86::ECX) as u32);
            0
        });
    }
    let called: Rc<RefCell<Vec<Called>>> = Rc::default();
    let callbacks: Vec<u64> = (0..6).map(|_| emu.alloc(0x10)).collect();
    for &cb in &callbacks {
        let called = called.clone();
        emu.stub_fn(cb, 2, move |io| {
            let p = u64::from(io.arg(0));
            let rect = [0, 1, 2, 3].map(|i| io.read_u32(p + 4 * i));
            called.borrow_mut().push(Called {
                callback: cb,
                ecx: io.reg(RegisterX86::ECX) as u32,
                edx: io.reg(RegisterX86::EDX) as u32,
                rect_at: p as u32,
                rect,
                elapsed: io.arg(1),
            });
            1
        });
    }
    emu.write_u32(0x88560c, 0);
    emu.write_u32(0x885604, 0);
    let rect_in = emu.alloc(16);
    let handle = emu.alloc(4);
    let prios = [1.0f32, 6.0, 7.0, 9.0, 10.0, 10.0, 0.5];
    let mut seed = 0x4423_5001u32;
    let mut checked = 0u64;
    for case in 0..1500 {
        emu.call(0x442220, 0, 0, &[]); // the list's initializer: empty
        let n = 1 + (next(&mut seed) % 6) as usize;
        // Head to tail, as ours links them; each entry: (callback index, priority, layer).
        let mut list: Vec<(usize, f32, Layer)> = Vec::new();
        for (i, &callback) in callbacks.iter().enumerate().take(n) {
            let rect = if next(&mut seed).is_multiple_of(3) {
                None // the default unit square, `ecx` 0
            } else {
                Some([
                    grid(&mut seed),
                    grid(&mut seed),
                    grid(&mut seed),
                    grid(&mut seed),
                ])
            };
            let flags = next(&mut seed) % 8;
            let prio = prios[(next(&mut seed) % prios.len() as u32) as usize];
            let ecx = match rect {
                Some(r) => {
                    write_rect(&mut emu, rect_in, r);
                    rect_in as u32
                }
                None => 0,
            };
            emu.clear_calls();
            emu.call(
                0x442800,
                ecx,
                flags,
                &[
                    prio.to_bits(),
                    0x100 + i as u32,
                    callback as u32,
                    handle as u32,
                ],
            );
            let at = layer_insert_index(&list.iter().map(|e| e.1).collect::<Vec<_>>(), prio);
            let layer = Layer {
                rect: rect.unwrap_or([0.0, 0.0, 1.0, 1.0]),
                flags,
            };
            list.insert(at, (i, prio, layer));
        }
        let elapsed = f32::from_bits(next(&mut seed) << 8);
        let elapsed = if elapsed.is_finite() { elapsed } else { 0.016 };
        emu.write_u32(ACC, elapsed.to_bits());
        let inactive = next(&mut seed).is_multiple_of(5);
        *active.borrow_mut() = u32::from(!inactive);
        called.borrow_mut().clear();
        sleeps.borrow_mut().clear();
        emu.clear_calls();
        let eax = emu.call(0x442350, 0, 0, &[]);
        assert_eq!(eax, 1);
        let ours = frame_pass_layers(&list.iter().map(|e| e.2).collect::<Vec<_>>());
        let expect: Vec<(u64, u32, [u32; 4])> = list
            .iter()
            .zip(&ours)
            .filter(|(_, o)| o.1)
            .map(|(e, o)| (callbacks[e.0], 0x100 + e.0 as u32, bits(o.0)))
            .collect();
        let got: Vec<(u64, u32, [u32; 4])> = called
            .borrow()
            .iter()
            .map(|c| (c.callback, c.ecx, c.rect))
            .collect();
        assert_eq!(got, expect, "case {case}: layers {list:?}");
        for c in called.borrow().iter() {
            assert_eq!(c.elapsed, elapsed.to_bits(), "case {case}: elapsed");
            assert_eq!(
                c.edx + 0x10,
                c.rect_at,
                "case {case}: edx is the layer's rectangle"
            );
        }
        assert_eq!(
            emu.read_u32(ACC),
            0,
            "case {case}: the accumulator is cleared"
        );
        let want: Vec<u32> = if inactive { vec![250] } else { vec![] };
        assert_eq!(*sleeps.borrow(), want, "case {case}: sleep");
        let presents = emu.calls().iter().filter(|c| c.addr == 0x58a960).count();
        assert_eq!(presents, 1, "case {case}: one present");
        checked += 1;
    }
    eprintln!("frame_pass: {checked} passes match WoW.exe @ 0x442350");
}
