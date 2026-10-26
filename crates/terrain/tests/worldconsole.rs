//! The world's console handlers and frame rate, run whole (`systems/terrain/world.md`): the five
//! flag toggles and the simple doodad switch with `ConsoleWrite` (`0x63cb50`) replaced by a stub
//! that records the line it is given, `waterMaxLOD` (`0x6736c0`) with `_sscanf` (`0x73fb5d`)
//! replaced by one that stores a chosen number, and `GetSmoothedFrameRate` (`0x6724c0`) read
//! back at 53 bits.

mod common;

use std::cell::Cell;
use std::rc::Rc;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::worldconsole::{
    clamp_water_max_lod, smoothed_frame_rate, toggle_simple_doodads, toggle_world_flag,
};
use common::next;

const FLAGS_VA: u64 = 0x00c7_b2a4;
const SIMPLE_VA: u64 = 0x00c7_b2c4;
const WATER_MAX_LOD_VA: u64 = 0x0086_a030;
const TIMES_VA: u64 = 0x00c6_31c8;
const RING_INDEX_VA: u64 = 0x00c7_b488;

/// Each toggle with its bit and its "disabled" and "enabled" lines.
const TOGGLES: [(u64, u32, u32, u32); 5] = [
    (0x673050, 0x0010_0000, 0x867ac4, 0x867aac),
    (0x673250, 0x20, 0x867c34, 0x867c18),
    (0x673770, 0x0200_0000, 0x867ea0, 0x867e88),
    (0x673a30, 0x40, 0x867fe8, 0x867fd0),
    (0x673a70, 0x0400_0000, 0x868020, 0x868004),
];

#[test]
fn world_console_toggles_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let line = Rc::new(Cell::new(0u32));
    let seen = line.clone();
    e.stub_fn(0x63cb50, 0, move |io| {
        seen.set(io.reg(RegisterX86::ECX) as u32);
        0
    });
    let mut s = 0x6730_5000u32;
    for i in 0..400 {
        let flags = match i {
            0 => 0,
            1 => u32::MAX,
            _ => next(&mut s),
        };
        for &(va, bit, off, on) in &TOGGLES {
            e.write_u32(FLAGS_VA, flags);
            line.set(0);
            let ret = e.call(va, 0x1111, 0x2222, &[]);
            let (want, now_on) = toggle_world_flag(flags, bit);
            assert_eq!(ret, 1, "{va:#x}: return");
            assert_eq!(e.read_u32(FLAGS_VA), want, "{va:#x} flags {flags:#x}");
            assert_eq!(line.get(), if now_on { on } else { off }, "{va:#x}: line");
        }
        let word = match i % 3 {
            0 => 0,
            1 => 1,
            _ => next(&mut s),
        };
        e.write_u32(SIMPLE_VA, word);
        line.set(0);
        let ret = e.call(0x673ac0, 0, 0, &[]);
        assert_eq!(ret, 1, "simple doodads: return");
        assert_eq!(
            e.read_u32(SIMPLE_VA),
            toggle_simple_doodads(word),
            "simple {word:#x}"
        );
        let want_line = if word == 0 { 0x868040 } else { 0x868058 };
        assert_eq!(line.get(), want_line, "simple doodads: line");
    }
}

#[test]
fn water_max_lod_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let scanned = Rc::new(Cell::new(0i32));
    let value = scanned.clone();
    // `_sscanf(args, "%d", &level)`: store the chosen number through the third argument.
    e.stub_fn(0x73fb5d, 0, move |io| {
        let dst = u64::from(io.arg(2));
        io.write_u32(dst, value.get() as u32);
        1
    });
    let mut s = 0x6736_c000u32;
    for i in 0..2000 {
        let v = match i {
            0..=9 => i - 3,
            10 => i32::MIN,
            11 => i32::MAX,
            _ => next(&mut s) as i32 >> (next(&mut s) % 31),
        };
        scanned.set(v);
        let ret = e.call(0x6736c0, 0, 0x5555, &[]);
        assert_eq!(ret, 1);
        assert_eq!(
            e.read_u32(WATER_MAX_LOD_VA) as i32,
            clamp_water_max_lod(v),
            "waterMaxLOD {v}"
        );
    }
}

#[test]
fn smoothed_frame_rate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s = 0x6724_c000u32;
    let draw = |s: &mut u32| -> f32 {
        let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
        match next(s) >> 29 {
            0 => 0.0,
            1 => (m * 0.002) as f32,
            2 => (m * 0.06) as f32,
            3 => (m * 2.0 - 1.0) as f32,
            4 => f32::from_bits(next(s)),
            _ => (m * 0.1) as f32,
        }
    };
    for i in 0..4000 {
        let times: [f32; 30] = match i {
            0 => [0.0; 30],
            1 => [0.001; 30],
            2 => [1.0 / 60.0; 30],
            // The one sum whose mean rounds to 0.001 exactly, as three floats, then without its
            // last: the equal compare takes the division, the one below answers 1000.
            3 | 4 => {
                let mut t = [0.0; 30];
                t[28] = f32::from_bits(0x3cf5_c28f);
                t[29] = f32::from_bits(0x3011_eb85);
                if i == 3 {
                    t[0] = f32::from_bits(0x2380_0000);
                }
                t
            }
            _ => std::array::from_fn(|_| draw(&mut s)),
        };
        let start = if i == 3 || i == 4 {
            28
        } else {
            (next(&mut s) % 30) as usize
        };
        for (k, &t) in times.iter().enumerate() {
            e.write_u32(TIMES_VA + 4 * k as u64, t.to_bits());
        }
        e.write_u32(RING_INDEX_VA, start as u32);
        let bin = e.call_f64(0x6724c0, 0, 0, &[]);
        let ours = smoothed_frame_rate(&times, start);
        assert!(
            bin.to_bits() == ours.to_bits() || (bin.is_nan() && ours.is_nan()),
            "case {i}: binary={bin:e} ours={ours:e} start {start}"
        );
        assert_eq!(
            e.read_u32(RING_INDEX_VA),
            start as u32,
            "the index is not written"
        );
    }
}
