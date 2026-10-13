//! The setups that end in a matrix builder and the device, run with those stubbed: the values they
//! compute are read from the stack, from the stubs' logged arguments, or from their outputs.

mod common;

use benilla_re_camera::transform::{
    frustum_split_owned, ortho_corner_args, ortho_midpoints, ortho_recenter, ortho_translate,
    view_lookat_owned, LookParams,
};
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use common::{assert_f32, fuzz, next};

const VIEW_LOOKAT: u64 = 0x7ac640;
const PERSPECTIVE: u64 = 0x5c3cc0; // BuildPerspectiveMatrix(fov, aspect, near, far), ret 0x10
const DEV_PUSH: u64 = 0x58b040; // the device's projection
const OWNED_END: u64 = 0x7ac7df; // after the last up store, before the view matrix is built

/// `CCamera::SetDeviceTransforms` (`0x7ac640`) run from its entry to `0x7ac7df`: the aspect from
/// the stubbed `BuildPerspectiveMatrix`'s arguments, the direction and up from its frame.
#[test]
fn camera_view_lookat() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x180);
    let vp = e.alloc(0x10);
    let stack = e.alloc(0x800);
    e.stub(PERSPECTIVE, 0, 4);
    e.stub(DEV_PUSH, 0, 0);
    let mut s: u32 = 0x7ac6_0001;
    for case in 0..4000u32 {
        let vpr = [
            fuzz(&mut s, 1.0),
            fuzz(&mut s, 1000.0),
            fuzz(&mut s, 1000.0),
            fuzz(&mut s, 1.0),
        ];
        let p = LookParams {
            eye: [
                fuzz(&mut s, 500.0),
                fuzz(&mut s, 500.0),
                fuzz(&mut s, 500.0),
            ],
            target: [
                fuzz(&mut s, 500.0),
                fuzz(&mut s, 500.0),
                fuzz(&mut s, 500.0),
            ],
            sin_yaw: fuzz(&mut s, 1.0),
            cos_yaw: fuzz(&mut s, 1.0),
            sin_roll: fuzz(&mut s, 4.0),
            cos_roll: fuzz(&mut s, 4.0),
        };
        // The projection's other inputs, which the test does not check.
        e.write_u32(this + 0xcc, fuzz(&mut s, 100.0).to_bits()); // near
        e.write_u32(this + 0xac, fuzz(&mut s, 5000.0).to_bits()); // far
        e.write_u32(this + 0x114, fuzz(&mut s, 1.5).to_bits()); // fov
                                                                // The inputs checked.
        e.write_u32(this + 0x3c, p.eye[0].to_bits());
        e.write_u32(this + 0x40, p.eye[1].to_bits());
        e.write_u32(this + 0x44, p.eye[2].to_bits());
        e.write_u32(this + 0x64, p.target[0].to_bits());
        e.write_u32(this + 0x68, p.target[1].to_bits());
        e.write_u32(this + 0x6c, p.target[2].to_bits());
        e.write_u32(this + 0x16c, p.sin_yaw.to_bits());
        e.write_u32(this + 0x168, p.cos_yaw.to_bits());
        e.write_u32(this + 0x144, p.sin_roll.to_bits());
        e.write_u32(this + 0x140, p.cos_roll.to_bits());
        for (i, &x) in vpr.iter().enumerate() {
            e.write_u32(vp + 4 * i as u64, x.to_bits());
        }

        let esp = stack + 0x400;
        e.write_u32(esp, 0); // a return address the run never reaches
        e.write_u32(esp + 4, vp as u32); // [ebp+8], the viewport
        e.set_reg(RegisterX86::ESP, esp);
        e.set_reg(RegisterX86::ECX, this);
        e.clear_calls();
        e.run(VIEW_LOOKAT, OWNED_END);

        let ebp = esp - 4;
        let dir = [
            e.read_f32(ebp - 0x24),
            e.read_f32(ebp - 0x20),
            e.read_f32(ebp - 0x1c),
        ];
        let up = [
            e.read_f32(ebp - 0x18),
            e.read_f32(ebp - 0x14),
            e.read_f32(ebp - 0x10),
        ];
        let persp = e
            .calls()
            .iter()
            .find(|c| c.addr == PERSPECTIVE)
            .expect("perspective stub not hit");
        let aspect = f32::from_bits(persp.args[1]);

        let (oa, od, ou) = view_lookat_owned(&vpr, &p);
        assert_f32(aspect, oa, &format!("view_lookat aspect case={case}"));
        for i in 0..3 {
            assert_f32(dir[i], od[i], &format!("view_lookat dir case={case}[{i}]"));
            assert_f32(up[i], ou[i], &format!("view_lookat up case={case}[{i}]"));
        }
    }
}

const ORTHO: u64 = 0x7ad7f0;
const GX_ORTHO: u64 = 0x5c3d90; // BuildOrthoMatrix: four edges and two depths, ret 0x18
const ORTHO_RECENTER_BEGIN: u64 = 0x7ad946; // after 0x58a250 has written the screen
const ORTHO_RECENTER_END: u64 = 0x7ad979;

/// `SetupOrthoTransforms` (`0x7ad7f0`) run whole with its callees stubbed: the rectangle from
/// `BuildOrthoMatrix`'s arguments and the translation through `0x7bdc40`'s. Then the shift, the
/// slice `0x7ad946` to `0x7ad979`, on its own inputs.
#[test]
fn camera_ortho() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rectbuf = e.alloc(0x10);
    let out2buf = e.alloc(0x10);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    // The whole function.
    e.stub(GX_ORTHO, 0, 6);
    e.stub(0x7bdca0, 0, 1); // the Z flip, ret 4
    e.stub(0x589ca0, 0, 1); // 0, so no shift
    e.stub(0x7bdc40, 0, 1); // the translation, ret 4
    e.stub(0x58b040, 0, 0);
    e.stub(0x58b050, 0, 0);

    let mut s: u32 = 0x7ad7_0001;
    for case in 0..4000u32 {
        let rect = [
            fuzz(&mut s, 1000.0),
            fuzz(&mut s, 1000.0),
            fuzz(&mut s, 1000.0),
            fuzz(&mut s, 1000.0),
        ];
        for (i, &x) in rect.iter().enumerate() {
            e.write_u32(rectbuf + 4 * i as u64, x.to_bits());
        }
        let out2 = [fuzz(&mut s, 2000.0), fuzz(&mut s, 2000.0)];
        e.write_u32(out2buf, out2[0].to_bits());
        e.write_u32(out2buf + 4, out2[1].to_bits());

        e.clear_calls();
        e.call(ORTHO, rectbuf as u32, out2buf as u32, &[]);
        let gxo = e
            .calls()
            .iter()
            .find(|c| c.addr == GX_ORTHO)
            .expect("BuildOrthoMatrix stub not hit");
        let corners = [
            f32::from_bits(gxo.args[0]),
            f32::from_bits(gxo.args[1]),
            f32::from_bits(gxo.args[2]),
            f32::from_bits(gxo.args[3]),
        ];
        let oc = ortho_corner_args(&rect);
        for i in 0..4 {
            assert_f32(corners[i], oc[i], &format!("ortho corner case={case}[{i}]"));
        }

        // The translation, through the pointer 0x7bdc40 is passed.
        let tcall = e
            .calls()
            .iter()
            .find(|c| c.addr == 0x7bdc40)
            .expect("translate stub not hit");
        let tptr = u64::from(tcall.args[0]);
        let bt = [e.read_f32(tptr), e.read_f32(tptr + 4)];
        let (mid_x, mid_y) = ortho_midpoints(&rect);
        let ot = ortho_translate(&out2, mid_x, mid_y);
        assert_f32(bt[0], ot[0], &format!("ortho translate x case={case}"));
        assert_f32(bt[1], ot[1], &format!("ortho translate y case={case}"));

        // The shift, on random screen and matrix values.
        let screen = [
            fuzz(&mut s, 1.0),
            fuzz(&mut s, 1.0),
            fuzz(&mut s, 2000.0),
            fuzz(&mut s, 2000.0),
        ];
        let mat12 = fuzz(&mut s, 2.0);
        let mat13 = fuzz(&mut s, 2.0);
        e.set_reg(RegisterX86::ESI, rectbuf);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0xc0);
        e.write_u32(ebp - 0x10, screen[0].to_bits());
        e.write_u32(ebp - 0xc, screen[1].to_bits());
        e.write_u32(ebp - 0x8, screen[2].to_bits());
        e.write_u32(ebp - 0x4, screen[3].to_bits());
        e.write_u32(ebp - 0x28, mat12.to_bits());
        e.write_u32(ebp - 0x24, mat13.to_bits());
        e.run(ORTHO_RECENTER_BEGIN, ORTHO_RECENTER_END);
        let (oy, ox) = ortho_recenter(&rect, &screen, mat12, mat13);
        assert_f32(
            e.read_f32(ebp - 0x28),
            oy,
            &format!("ortho recenter y case={case}"),
        );
        assert_f32(
            e.read_f32(ebp - 0x24),
            ox,
            &format!("ortho recenter x case={case}"),
        );
    }
}

const FRUSTUM_SPLIT: u64 = 0x7ad2c0;
const FRUSTUM_CORNERS: u64 = 0x5c43b0; // BuildFrustumCorners: 8 corners, 0x60 bytes, ret 4
const DEV_Q0: u64 = 0x58b0b0; // the device's matrix stack top, ret
const DEV_Q1: u64 = 0x58b090; // the device's matrix at +0xf50, ret
const MEMSET3: u64 = 0x4078e0; // a vector constructed from one value, __thiscall, ret 4

/// `UnprojectScreenPoint` (`0x7ad2c0`) run whole: the stubbed `BuildFrustumCorners` writes random
/// corners into its frame, the device reads and the vector constructor are stubbed, and
/// `SetVector3` (`0x4549a0`), the vector add, subtract and scale (`0x621160`, `0x4841f0`,
/// `0x5f8cf0`) and the array's destructor (`0x4044b0`) run. Both points are compared.
#[test]
fn camera_frustum_split() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out8 = e.alloc(0x20);
    let out2 = e.alloc(0x20);
    let cornerbuf = e.alloc(0x60); // 8 corners of 3 floats

    // The corners go to the builder's one stack argument.
    e.stub_write_through(FRUSTUM_CORNERS, 0, cornerbuf, 0x60, 0, 1);
    e.stub(DEV_Q0, 0, 0);
    e.stub(DEV_Q1, 0, 0);
    e.stub(MEMSET3, 0, 1); // overwritten by the corners

    let mut s: u32 = 0x7ad2_0001;
    for case in 0..4000u32 {
        // Eight random corners.
        let mut corners = [[0.0f32; 3]; 8];
        for c in &mut corners {
            for x in c.iter_mut() {
                *x = fuzz(&mut s, 1000.0);
            }
        }
        for (i, c) in corners.iter().enumerate() {
            for (j, &x) in c.iter().enumerate() {
                e.write_u32(cornerbuf + (12 * i + 4 * j) as u64, x.to_bits());
            }
        }
        // p3 and p4 in (0, 1), inside the range test.
        let p3 = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32 * 0.999_999 + 1e-6;
        let p4 = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32 * 0.999_999 + 1e-6;

        e.clear_calls();
        e.call(
            FRUSTUM_SPLIT,
            out8 as u32,
            out2 as u32,
            &[p3.to_bits(), p4.to_bits()],
        );

        let bin8 = [e.read_f32(out8), e.read_f32(out8 + 4), e.read_f32(out8 + 8)];
        let bin2 = [e.read_f32(out2), e.read_f32(out2 + 4), e.read_f32(out2 + 8)];
        let (o8, o2) = frustum_split_owned(&corners, p3, p4);
        for i in 0..3 {
            assert_f32(
                bin8[i],
                o8[i],
                &format!("frustum_split out8 case={case}[{i}]"),
            );
            assert_f32(
                bin2[i],
                o2[i],
                &format!("frustum_split out2 case={case}[{i}]"),
            );
        }
    }
}
