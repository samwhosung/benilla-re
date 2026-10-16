//! The Direct3D device's texture coordinate modes against the exe: `CGxDeviceD3d::ApplyTexCoordMode`
//! (`0x5a2b40`) for modes 0 and 3 to 6, and for a mode above 6, with the shadow-cached state call
//! `0x5a2570` stubbed to a log and the stack code `0x591370` and `0x591440` run for real. Modes 1
//! and 2 reach the view matrix code (`0x5a2c6e`) and are not run.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_gx::texmode::{
    mode_matrix, store_mode_matrix, texture_coordinate_index, StageMatrix,
};

/// The device's stage count, at `+0x23c`.
const UNIT_COUNT: u64 = 0x23c;
/// The first stage's mode matrix stack; a stack is 0x118 bytes: the top index at `+0`, the dirty
/// byte at `+4`, the matrices from `+8` (64 bytes each) and the level flags from `+0x108`.
const STACKS: u64 = 0x1bd8;
const STACK_SIZE: u64 = 0x118;

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

#[test]
fn tex_coord_mode_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(0x5a2570, 0, 2);
    let dev = e.alloc(0x4000);
    let mut s = 0x5a2b_4000u32;
    let mut reflected = 0u32;
    for case in 0..4000u32 {
        let units = 1 + lcg(&mut s) % 8;
        let stage = lcg(&mut s) % 10;
        let mode = match lcg(&mut s) % 9 {
            0 => 0,
            1 => 3,
            2 => 4,
            3 => 5,
            4 | 5 => 6,
            6 => 7,
            7 => 0xffff_ffff,
            _ => lcg(&mut s) % 100 + 7,
        };
        let base = dev + STACKS + STACK_SIZE * u64::from(stage);
        let depth = u64::from(lcg(&mut s) % 4);
        e.write_u32(dev + UNIT_COUNT, units);
        e.write_u32(base, depth as u32);
        e.write(base + 4, &[(lcg(&mut s) & 1) as u8]);
        let mut matrix = [0f32; 16];
        for (i, m) in matrix.iter_mut().enumerate() {
            *m = f32::from_bits(lcg(&mut s));
            e.write_u32(base + 8 + 64 * depth + 4 * i as u64, m.to_bits());
        }
        let flags = lcg(&mut s) % 4;
        e.write_u32(base + 0x108 + 4 * depth, flags);
        let mut ours = StageMatrix {
            matrix,
            flags,
            dirty: e.read(base + 4, 1)[0] != 0,
        };

        e.clear_calls();
        e.call(0x5a2b40, dev as u32, 0, &[stage, mode]);
        let calls: Vec<[u32; 2]> = e.calls().iter().map(|c| [c.args[0], c.args[1]]).collect();

        let mut expect_calls = Vec::new();
        if stage < units {
            if let Some(v) = texture_coordinate_index(stage, mode) {
                expect_calls.push([0x3a + stage, v]);
            }
            let which = mode_matrix(mode).expect("modes 1 and 2 are not run");
            store_mode_matrix(&mut ours, which);
            if which == benilla_re_gx::texmode::ModeMatrix::Reflection {
                reflected += 1;
            }
        }
        assert_eq!(
            calls, expect_calls,
            "case {case}: stage {stage} mode {mode}"
        );
        let mut got = [0u32; 16];
        for (i, g) in got.iter_mut().enumerate() {
            *g = e.read_u32(base + 8 + 64 * depth + 4 * i as u64);
        }
        let want = ours.matrix.map(f32::to_bits);
        assert_eq!(got, want, "case {case}: stage {stage} mode {mode} matrix");
        assert_eq!(
            e.read_u32(base + 0x108 + 4 * depth),
            ours.flags,
            "case {case}: stage {stage} mode {mode} flags"
        );
        assert_eq!(
            e.read(base + 4, 1)[0] != 0,
            ours.dirty,
            "case {case}: stage {stage} mode {mode} dirty"
        );
        assert_eq!(e.read_u32(base), depth as u32, "case {case}: depth");
    }
    assert!(reflected > 100, "{reflected} reflection stores");
}
