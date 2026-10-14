//! The patter geometry of `0x675ac0` against the exe: the half-edge basis from the view rows
//! (`0x675bfc` to `0x675c9b`) and the per-patter corners, centre and texture words (`0x675db3` to
//! `0x6760c3`), each run as the exe's own instructions on a frame laid out in the emulator, with
//! the vector leaves `0x4549a0` and `0x4549f0` run for real. Every stored value is compared bit for
//! bit.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::wx_rainrender::{wx_quad_basis, wx_quad_geom, QuadGeom};
use common::{assert_f32, fuzz_f32, next};

/// The basis: the identity, signed and exact rows, zero rows and a fuzz.
#[test]
fn wx_rainrender_basis_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400) + 0x200;
    let mut s = 0x7261_696e_u32;

    let run = |emu: &mut Emu, r0: [f32; 3], r1: [f32; 3], tag: &str| {
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x180);
        emu.write_u32(frame - 0x11c, r0[0].to_bits());
        emu.write_u32(frame - 0x10c, r0[1].to_bits());
        emu.write_u32(frame - 0xfc, r0[2].to_bits());
        emu.write_u32(frame - 0x118, r1[0].to_bits());
        emu.write_u32(frame - 0x108, r1[1].to_bits());
        emu.write_u32(frame - 0xf8, r1[2].to_bits());
        emu.run(0x675bfc, 0x675c9b);
        let right = [
            emu.read_f32(frame - 0x44),
            emu.read_f32(frame - 0x40),
            emu.read_f32(frame - 0x3c),
        ];
        let up = [
            emu.read_f32(frame - 0x128),
            emu.read_f32(frame - 0x124),
            emu.read_f32(frame - 0x120),
        ];
        let back = [
            emu.read_f32(frame - 0x158),
            emu.read_f32(frame - 0x154),
            emu.read_f32(frame - 0x150),
        ];
        let (our_right, our_up, our_back) = wx_quad_basis(r0, r1);
        for i in 0..3 {
            assert_f32(
                right[i],
                our_right[i],
                &format!("{tag}: right[{i}] r0={r0:?}"),
            );
            assert_f32(up[i], our_up[i], &format!("{tag}: up[{i}] r1={r1:?}"));
            assert_f32(back[i], our_back[i], &format!("{tag}: back[{i}] r0={r0:?}"));
        }
    };

    run(&mut emu, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], "identity");
    run(&mut emu, [-1.0, 2.0, -3.0], [4.0, -5.0, 6.0], "signed");
    run(&mut emu, [12.0, 6.0, 0.0], [6.0, 12.0, 0.0], "exact");
    run(&mut emu, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "zero");

    for _ in 0..500 {
        let r0 = [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)];
        let r1 = [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)];
        run(&mut emu, r0, r1, "fuzz");
    }
}

/// The patter quad: patters ahead, behind, above and below the camera (the visibility gate at
/// `-dir.z` exactly 0, below 0 and above 0), a zero delta, NaN and infinite positions, -0, and a
/// fuzz.
#[test]
fn wx_rainrender_quad_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x600) + 0x300;
    let ebx = emu.alloc(0x40) + 0x20;
    let outbuf = emu.alloc(0x400) + 0x100;
    let mut s = 0x7061_7474_u32;

    let run = |emu: &mut Emu,
               packet_pos: [f32; 3],
               cam_pos: [f32; 3],
               cam_dist: f32,
               packet_far: f32,
               right: [f32; 3],
               up: [f32; 3],
               back: [f32; 3],
               tag: &str| {
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x200);
        emu.set_reg(RegisterX86::EBX, ebx);
        emu.set_reg(RegisterX86::EDI, outbuf);
        emu.set_reg(RegisterX86::ESI, outbuf + 0xc);
        emu.set_reg(RegisterX86::EAX, outbuf + 0x200);
        emu.set_reg(RegisterX86::ECX, outbuf + 0x200);
        emu.set_reg(RegisterX86::EDX, outbuf + 0x200);

        emu.write_u32(ebx.wrapping_sub(0x14), packet_pos[0].to_bits());
        emu.write_u32(ebx.wrapping_sub(0x10), packet_pos[1].to_bits());
        emu.write_u32(ebx.wrapping_sub(0xc), packet_pos[2].to_bits());
        emu.write_u32(frame - 0x50, cam_pos[0].to_bits());
        emu.write_u32(frame - 0x4c, cam_pos[1].to_bits());
        emu.write_u32(frame - 0x48, cam_pos[2].to_bits());
        emu.write_u32(frame - 0x24, cam_dist.to_bits());
        emu.write_u32(frame - 0xcc, packet_far.to_bits());
        emu.write_u32(frame - 0xb8, up[0].to_bits());
        emu.write_u32(frame - 0xb4, up[1].to_bits());
        emu.write_u32(frame - 0xb0, up[2].to_bits());
        emu.write_u32(frame - 0xac, right[0].to_bits());
        emu.write_u32(frame - 0xa8, right[1].to_bits());
        emu.write_u32(frame - 0xa4, right[2].to_bits());
        emu.write_u32(frame - 0x158, back[0].to_bits());
        emu.write_u32(frame - 0x154, back[1].to_bits());
        emu.write_u32(frame - 0x150, back[2].to_bits());
        emu.write_u32(frame - 0x4, 0);
        emu.write_u32(frame - 0x20, 0);

        for &off in &[
            0x30i64, 0x2c, 0x28, 0x134, 0x130, 0x12c, 0x18, 0x14, 0x10, 0x13c, 0x138, 0x1c, 0x8,
            0x70, 0x7c, 0x78, 0x74, 0x5c, 0x58, 0x54, 0x68, 0x64, 0x60, 0x98, 0x94, 0xa0, 0x9c,
            0x8c, 0x88,
        ] {
            emu.write_u32((frame as i64 - off) as u64, 0xdead_beef);
        }

        emu.run(0x675db3, 0x6760c3);

        let rd = |emu: &Emu, off: i64| emu.read_f32((frame as i64 - off) as u64);
        let rdu = |emu: &Emu, off: i64| emu.read_u32((frame as i64 - off) as u64);
        let binary = QuadGeom {
            delta: [rd(emu, 0x30), rd(emu, 0x2c), rd(emu, 0x28)],
            dir: [rd(emu, 0x134), rd(emu, 0x130), rd(emu, 0x12c)],
            center: [rd(emu, 0x18), rd(emu, 0x14), rd(emu, 0x10)],
            up_scaled: [rd(emu, 0x13c), rd(emu, 0x138)],
            color: [rdu(emu, 0x1c) >> 14, 0],
            tex: [rd(emu, 0x70), 0.0],
            corner0: [rd(emu, 0x7c), rd(emu, 0x78), rd(emu, 0x74)],
            corner1: [rd(emu, 0x5c), rd(emu, 0x58), rd(emu, 0x54)],
            corner2: [rd(emu, 0x68), rd(emu, 0x64), rd(emu, 0x60)],
            texword: [
                rd(emu, 0x98),
                rd(emu, 0x94),
                rd(emu, 0xa0),
                rd(emu, 0x9c),
                rd(emu, 0x8c),
                rd(emu, 0x88),
            ],
        };
        let ours = wx_quad_geom(packet_pos, cam_pos, cam_dist, packet_far, right, up, back);

        for i in 0..3 {
            assert_f32(
                binary.delta[i],
                ours.delta[i],
                &format!("{tag}: delta[{i}]"),
            );
            assert_f32(binary.dir[i], ours.dir[i], &format!("{tag}: dir[{i}]"));
            assert_f32(
                binary.center[i],
                ours.center[i],
                &format!("{tag}: center[{i}]"),
            );
            assert_f32(
                binary.corner0[i],
                ours.corner0[i],
                &format!("{tag}: corner0[{i}]"),
            );
            assert_f32(
                binary.corner1[i],
                ours.corner1[i],
                &format!("{tag}: corner1[{i}]"),
            );
            assert_f32(
                binary.corner2[i],
                ours.corner2[i],
                &format!("{tag}: corner2[{i}]"),
            );
        }
        for i in 0..2 {
            assert_f32(
                binary.up_scaled[i],
                ours.up_scaled[i],
                &format!("{tag}: up_scaled[{i}]"),
            );
        }
        assert_eq!(binary.color[0], ours.color[0], "{tag}: color[0]");
        assert_f32(binary.tex[0], ours.tex[0], &format!("{tag}: tex[0]=TH"));
        for i in 0..6 {
            assert_f32(
                binary.texword[i],
                ours.texword[i],
                &format!("{tag}: texword[{i}]"),
            );
        }
    };

    let right = [0.0833_f32, 0.0, 0.0];
    let up = [0.0, 0.1666, 0.0];
    let back = [-0.0833_f32, 0.0, 0.0];

    run(
        &mut emu,
        [10.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        5.0,
        20.0,
        right,
        up,
        back,
        "ahead-x",
    );
    run(
        &mut emu,
        [-10.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        5.0,
        20.0,
        right,
        up,
        back,
        "behind-x",
    );
    run(
        &mut emu,
        [0.0, 10.0, 0.0],
        [0.0, 0.0, 0.0],
        5.0,
        20.0,
        right,
        up,
        back,
        "above-y",
    );
    run(
        &mut emu,
        [3.0, 4.0, 12.0],
        [1.0, 1.0, 1.0],
        8.0,
        8.0,
        right,
        up,
        back,
        "dist-eq",
    );
    run(
        &mut emu,
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        0.0,
        0.0,
        right,
        up,
        back,
        "near-zero",
    );
    run(
        &mut emu,
        [0.0, 5.0, 0.0],
        [0.0, 0.0, 0.0],
        5.0,
        10.0,
        right,
        up,
        back,
        "ndirx-zero",
    );

    let nan = f32::NAN;
    for (pp, cp, tag) in [
        ([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], "zero delta"),
        ([nan, 2.0, 3.0], [0.0, 0.0, 0.0], "NaN position"),
        (
            [f32::INFINITY, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            "infinite position",
        ),
        ([0.0, 0.0, -10.0], [0.0, 0.0, 0.0], "straight below"),
        ([0.0, 0.0, 10.0], [0.0, 0.0, 0.0], "straight above"),
        ([-0.0, -0.0, -0.0], [0.0, 0.0, 0.0], "negative zero"),
    ] {
        run(&mut emu, pp, cp, 5.0, 20.0, right, up, back, tag);
    }
    for _ in 0..600 {
        let packet_pos = [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)];
        let cam_pos = [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)];
        let cam_dist = fuzz_f32(&mut s);
        let packet_far = if (next(&mut s) >> 8).is_multiple_of(4) {
            cam_dist
        } else {
            fuzz_f32(&mut s)
        };
        let rb = [
            fuzz_f32(&mut s) * 0.1,
            fuzz_f32(&mut s) * 0.1,
            fuzz_f32(&mut s) * 0.1,
        ];
        let ub = [
            fuzz_f32(&mut s) * 0.1,
            fuzz_f32(&mut s) * 0.1,
            fuzz_f32(&mut s) * 0.1,
        ];
        let bb = [-rb[0], -rb[1], -rb[2]];
        run(
            &mut emu, packet_pos, cam_pos, cam_dist, packet_far, rb, ub, bb, "fuzz",
        );
    }
}
