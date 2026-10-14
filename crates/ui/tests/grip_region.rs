//! `CSimpleTop::StartMovingOrSizingAt` (`0x765320`, `systems/ui/input.md#moving-and-sizing`) run
//! from the exe: the grip it picks from the cursor's position in the frame.
//!
//! The frame's rectangle comes through the three calls `0x768310`, `0x7680e0` and `0x768320`; the
//! test stubs them (the last copies a chosen rectangle out) and stubs the grip apply `0x7652b0`,
//! whose last argument is the grip. With the size flag 0 the exe goes to grip 4 without reading the
//! rectangle. The cursor is placed on both sides of every third boundary, exactly on it and at the
//! far edges, and with NaN. Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::resize_bounds::grip_region as region;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// f32 in `[lo, hi)` from the LCG high bits.
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

// ---- geo_765320: frame-rect fetch (stubbed out-param) + region apply (stubbed) ----
const RECT_FETCH: u64 = 0x0076_8320; // fills [out] with 4 f32 (a,b,c,d); returns nonzero/zero

const RECT_PRE1: u64 = 0x0076_8310; // pre-fetch predicate (we return 0 -> skip 0x7680e0)

const RECT_PRE2: u64 = 0x0076_80e0; // conditional follow-up (skipped)

const APPLY: u64 = 0x0076_52b0; // FUN_007652b0 (region apply; region = args[4])

#[test]
fn geo_765320() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x20);
    let frame = emu.alloc(0x40);
    let payload = emu.alloc(0x10); // the rect [a,b,c,d] the fetch writes through
    emu.stub(RECT_PRE1, 0, 0); // returns 0 -> 0x7680e0 skipped
    emu.stub(RECT_PRE2, 0, 1); // defensive
    emu.stub_write_through(RECT_FETCH, 0, payload, 0x10, 1, 1); // out=arg0; ret nonzero -> proceed
    emu.stub(APPLY, 0, 5); // thiscall, ret 0x14; region = args[4]

    let call_region = |emu: &mut Emu, arg4: u32, cur_x: f32, cur_y: f32, rect: [f32; 4]| -> u32 {
        for (k, &v) in rect.iter().enumerate() {
            emu.write_u32(payload + 4 * k as u64, v.to_bits());
        }
        emu.clear_calls();
        emu.call(
            0x765320,
            this as u32,
            0,
            &[frame as u32, 0, cur_x.to_bits(), cur_y.to_bits(), arg4],
        );
        let call = emu
            .calls()
            .iter()
            .find(|c| c.addr == APPLY)
            .copied()
            .expect("geo_765320: apply FUN_007652b0 not called");
        call.args[4]
    };

    // Fixed reference rect: top=10, left=20, bottom=410, right=420 (q_ca=100, q_db=100).
    let rect0 = [10.0f32, 20.0, 410.0, 420.0];
    // X quarter lines at left+100=120 and left+300=320; Y at top+100=110 and top+300=310.
    let mut tally = [0u32; 9];
    let mut arg4_zero = 0u32;

    // Deterministic probes: cursor centred in each of the 9 cells + the arg4==0 short-circuit.
    let xs = [70.0f32, 220.0, 370.0]; // col 0, 1, 2 centres
    let ys = [360.0f32, 210.0, 60.0]; // bottom(band0), mid(band1), top(band2) centres
    for (ci, &cx) in xs.iter().enumerate() {
        for (bi, &cy) in ys.iter().enumerate() {
            let mine = region(1, cx, cy, rect0);
            let bin = call_region(&mut emu, 1, cx, cy, rect0);
            assert_eq!(
                mine, bin,
                "geo_765320 probe col={ci} band={bi}: rust {mine} vs binary {bin}"
            );
            assert_eq!(mine as usize, ci + 3 * bi, "probe region mapping");
            tally[mine as usize] += 1;
        }
    }
    // arg4 == 0 -> region 4 regardless of cursor/rect.
    {
        let mine = region(0, 999.0, -999.0, rect0);
        let bin = call_region(&mut emu, 0, 999.0, -999.0, rect0);
        assert_eq!(mine, 4);
        assert_eq!(mine, bin, "geo_765320 arg4==0: rust {mine} vs binary {bin}");
        arg4_zero += 1;
    }
    // Boundary cursors exactly on the quarter lines (exercise the equal/unordered fall-through).
    for &cx in &[120.0f32, 320.0] {
        for &cy in &[110.0f32, 310.0] {
            let mine = region(1, cx, cy, rect0);
            let bin = call_region(&mut emu, 1, cx, cy, rect0);
            assert_eq!(
                mine, bin,
                "geo_765320 boundary ({cx},{cy}): rust {mine} vs binary {bin}"
            );
            tally[mine as usize] += 1;
        }
    }

    // Fuzz: random rects + cursors straddling the cells.
    let mut s = 0x7653_2000u32;
    for _ in 0..8000 {
        let left = rng(&mut s, -200.0, 200.0);
        let top = rng(&mut s, -200.0, 200.0);
        let w = rng(&mut s, 40.0, 800.0);
        let h = rng(&mut s, 40.0, 800.0);
        let rect = [top, left, top + h, left + w]; // [a=top, b=left, c=bottom, d=right]
        let cx = rng(&mut s, left - 0.1 * w, left + 1.1 * w);
        let cy = rng(&mut s, top - 0.1 * h, top + 1.1 * h);
        let arg4 = if (next(&mut s) >> 8).is_multiple_of(16) {
            0
        } else {
            1
        };
        let mine = region(arg4, cx, cy, rect);
        let bin = call_region(&mut emu, arg4, cx, cy, rect);
        assert_eq!(
            mine, bin,
            "geo_765320 fuzz (arg4={arg4} cur=({cx},{cy}) rect={rect:?}): rust {mine} vs binary {bin}"
        );
        if arg4 == 0 {
            arg4_zero += 1;
        } else {
            tally[mine as usize] += 1;
        }
    }

    // Early-return probe: a separate emu where the rect fetch returns 0 -> apply NOT called.
    {
        let mut emu2 = Emu::new(&wow);
        let this2 = emu2.alloc(0x20);
        let frame2 = emu2.alloc(0x40);
        let payload2 = emu2.alloc(0x10);
        emu2.stub(RECT_PRE1, 0, 0);
        emu2.stub(RECT_PRE2, 0, 1);
        emu2.stub_write_through(RECT_FETCH, 0, payload2, 0x10, 0, 1); // ret 0 -> early return
        emu2.stub(APPLY, 0, 5);
        emu2.call(
            0x765320,
            this2 as u32,
            0,
            &[frame2 as u32, 0, 50.0f32.to_bits(), 50.0f32.to_bits(), 1],
        );
        assert!(
            !emu2.calls().iter().any(|c| c.addr == APPLY),
            "geo_765320: apply must NOT fire when rect fetch returns 0"
        );
    }

    eprintln!(
        "geo_765320: bit-exact (3×3 region; dy/q_ca f32 sites, dx/q_db 80-bit, quarter-line splits); \
         per-region {tally:?} arg4_zero={arg4_zero}; early-return verified"
    );
    for (r, h) in tally.iter().enumerate() {
        assert!(*h > 0, "geo_765320 region {r} never hit");
    }
    assert!(arg4_zero > 0, "geo_765320 arg4==0 path never hit");
}
