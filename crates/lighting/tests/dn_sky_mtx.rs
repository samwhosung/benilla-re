//! `BuildDecalTransforms` (`0x6d6fa0`) whole, against `sky_billboard_build`; and the stretch of
//! `DrawDecalBatches` (`0x6d7480`) from `0x6d7506` to `0x6d7532` that tests `[0xcea620]` and calls
//! `0x589e30` with its scaled value, against `sky_element_draw_scale`. The matrix functions
//! `0x7bc6a0`, `0x7bdb00` and `0x7bdc40` run in the emulator inside the first.
//!
//! Host double (`docs/METHOD.md#the-harness`): `sky_billboard_build_difftest` runs `fsincos` at
//! `0x7bdb6a`. The emulator computes each in host `double`, so where its result reaches a compared
//! value the host's math stands in for the x87's, and the test shows the code around it.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::dn_sky_mtx::{
    sky_billboard_build, sky_element_draw_scale, SkyBillboard, SkyCamera,
};
use common::{assert_f32, fuzz_f32, next};

/// The scalar `0x6d7506` tests.
const G_CEA620: u64 = 0x00ce_a620;
/// The root of the camera chain `0x481900` follows: `[0xb4b2bc]`, then `+0x65b8`, then `+8`.
const G_B4B2BC: u64 = 0x00b4_b2bc;

/// `0x6d7506` to `0x6d7532`, with `0x589e30` stubbed to record its argument: no call for 0 and -0,
/// one call carrying the scaled value for anything else, NaN included.
#[test]
fn sky_element_draw_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x589e30, 0, 1);
    let stack = emu.alloc(0x100) + 0x80;
    let mut s = 0x736b_796c_u32;

    let run = |emu: &mut Emu, g: f32| {
        emu.write_u32(G_CEA620, g.to_bits());
        emu.set_reg(RegisterX86::ESP, stack);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.clear_calls();
        emu.run(0x6d7506, 0x6d7532);
        let ours = sky_element_draw_scale(g);
        let calls = emu.calls();
        match ours {
            None => assert!(calls.is_empty(), "g={g}: a call for zero"),
            Some(o) => {
                assert_eq!(calls.len(), 1, "g={g}: calls");
                assert_f32(
                    f32::from_bits(calls[0].args[0]),
                    o,
                    &format!("g={g} ({:#010x})", g.to_bits()),
                );
            }
        }
    };

    let edges = [
        0.0f32,
        -0.0,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        1.0,
        -1.0,
        16384.0,
        -16384.0,
        32768.0,
        0.5,
        -0.5,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN_POSITIVE,
    ];
    for &g in &edges {
        run(&mut emu, g);
    }
    for _ in 0..500 {
        run(&mut emu, fuzz_f32(&mut s));
    }
}

/// `0x6d6fa0` with the camera chain, the box, the optional matrix, the fade value and the flag
/// written, and both outputs poisoned so a bail shows as unchanged. The cases cover the flag both
/// ways, the optional matrix present and absent, extents of 0, just under, exactly at and above
/// 0.001 on each axis, a NaN extent (which goes on), and a fuzz.
#[test]
fn sky_billboard_build_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // The camera chain ends at `holder + 8`.
    let world = emu.alloc(0x7000);
    let holder = emu.alloc(0x40);
    emu.write_u32(G_B4B2BC, world as u32);
    emu.write_u32(world + 0x65b8, holder as u32);

    let out1 = emu.alloc(0x40);
    let out2 = emu.alloc(0x40);
    let p3 = emu.alloc(0x18);
    let p4 = emu.alloc(0x40);

    let mut s = 0x6d6f_a000_u32;

    const POISON: [u32; 16] = [
        0x1111_1111,
        0x2222_2222,
        0x3333_3333,
        0x4444_4444,
        0x5555_5555,
        0x6666_6666,
        0x7777_7777,
        0x0888_8888,
        0x0999_9999,
        0x0aaa_aaaa,
        0x0bbb_bbbb,
        0x0ccc_cccc,
        0x0ddd_dddd,
        0x0eee_eeee,
        0x0fff_ffff,
        0x1234_5678,
    ];

    let run = |emu: &mut Emu,
               cam: [f32; 3],
               p3v: [f32; 6],
               p4v: Option<[f32; 16]>,
               p5: f32,
               p6: bool,
               tag: &str| {
        emu.write_u32(holder + 8, cam[0].to_bits());
        emu.write_u32(holder + 0xc, cam[1].to_bits());
        emu.write_u32(holder + 0x10, cam[2].to_bits());
        for (i, v) in p3v.iter().enumerate() {
            emu.write_u32(p3 + 4 * i as u64, v.to_bits());
        }
        let p4_arg = match p4v {
            Some(m) => {
                for (i, v) in m.iter().enumerate() {
                    emu.write_u32(p4 + 4 * i as u64, v.to_bits());
                }
                p4 as u32
            }
            None => 0,
        };
        for i in 0..16u64 {
            emu.write_u32(out1 + 4 * i, POISON[i as usize]);
            emu.write_u32(out2 + 4 * i, POISON[i as usize]);
        }
        emu.call(
            0x6d6fa0,
            out1 as u32,
            out2 as u32,
            &[p3 as u32, p4_arg, p5.to_bits(), u32::from(p6)],
        );
        let mut bin1 = [0f32; 16];
        let mut bin2 = [0f32; 16];
        for i in 0..16u64 {
            bin1[i as usize] = emu.read_f32(out1 + 4 * i);
            bin2[i as usize] = emu.read_f32(out2 + 4 * i);
        }
        let poison_f: [f32; 16] = POISON.map(f32::from_bits);
        let ours: SkyBillboard = sky_billboard_build(
            SkyCamera {
                x: cam[0],
                y: cam[1],
                z: cam[2],
            },
            &p3v,
            p4v.as_ref(),
            p5,
            p6,
            &poison_f,
            &poison_f,
        );
        for i in 0..16 {
            assert_f32(bin1[i], ours.out1[i], &format!("{tag}: out1[{i}]"));
            assert_f32(bin2[i], ours.out2[i], &format!("{tag}: out2[{i}]"));
        }
    };

    let p4_mat: [f32; 16] = [
        2.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 1.5, 0.0, 3.0, -1.0, 4.0, 1.0,
    ];

    let box_ok: [f32; 6] = [-10.0, -20.0, -5.0, 10.0, 20.0, 5.0];
    let cam_a = [100.0f32, 50.0, 30.0];
    run(&mut emu, cam_a, box_ok, None, 0.0, false, "ok/cam/no-p4");
    run(&mut emu, cam_a, box_ok, None, 0.0, true, "ok/nocam/no-p4");
    run(
        &mut emu,
        cam_a,
        box_ok,
        Some(p4_mat),
        0.0,
        false,
        "ok/cam/p4",
    );
    run(
        &mut emu,
        cam_a,
        box_ok,
        Some(p4_mat),
        0.0,
        true,
        "ok/nocam/p4",
    );
    run(&mut emu, cam_a, box_ok, None, 0.25, false, "ok/p5=.25");
    run(&mut emu, cam_a, box_ok, None, -2.0, false, "ok/p5=-2");

    let box_flat_x: [f32; 6] = [1.0, -20.0, -5.0, 1.0, 20.0, 5.0];
    run(&mut emu, cam_a, box_flat_x, None, 0.0, false, "bail/flatx");
    let box_flat_y: [f32; 6] = [-10.0, 2.0, -5.0, 10.0, 2.0005, 5.0];
    run(&mut emu, cam_a, box_flat_y, None, 0.0, false, "bail/flaty");
    let eps = f32::from_bits(0x3a83_126f);
    let box_eq_x: [f32; 6] = [0.0, -20.0, -5.0, eps, 20.0, 5.0];
    run(&mut emu, cam_a, box_eq_x, None, 0.0, false, "ok/eps-x");
    let box_below_x: [f32; 6] = [0.0, -20.0, -5.0, f32::from_bits(0x3a83_126e), 20.0, 5.0];
    run(
        &mut emu,
        cam_a,
        box_below_x,
        None,
        0.0,
        false,
        "bail/below-x",
    );
    let box_eq_y: [f32; 6] = [-10.0, 0.0, -5.0, 10.0, eps, 5.0];
    run(&mut emu, cam_a, box_eq_y, None, 0.0, false, "ok/eps-y");

    let box_nan: [f32; 6] = [0.0, 0.0, 0.0, f32::NAN, 10.0, 10.0];
    run(&mut emu, cam_a, box_nan, None, 0.0, false, "nan-x");

    for _ in 0..400 {
        let cam = [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)];
        let mut p3v = [0f32; 6];
        for k in 0..3 {
            let lo = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 40.0 - 20.0) as f32;
            let span = match (next(&mut s) >> 8) % 4 {
                0 => 0.0,
                1 => 0.001,
                2 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 50.0) as f32,
                _ => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 0.002) as f32,
            };
            p3v[k] = lo;
            p3v[k + 3] = lo + span;
        }
        if (next(&mut s) >> 8).is_multiple_of(8) {
            p3v[(next(&mut s) >> 8) as usize % 6] = fuzz_f32(&mut s);
        }
        let p4v = if (next(&mut s) >> 8) & 1 == 0 {
            Some(p4_mat)
        } else {
            None
        };
        let p5 = fuzz_f32(&mut s);
        let p6 = (next(&mut s) >> 8) & 1 == 0;
        run(&mut emu, cam, p3v, p4v, p5, p6, "fuzz");
    }
}
