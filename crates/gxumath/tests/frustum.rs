//! `BuildFrustumCorners` (`0x5c43b0`) against the exe, run whole: the eight corners it writes through
//! its output pointer. The view matrices are look-ats with a target at least 20 away on each axis, so
//! they invert well; the projections alternate between perspectives (`m15 = 0`) and orthographics
//! (`m15 = 1`), so both branches of the corner set run. The matrices come from the builders in
//! `matrix`, which `matrix.rs` runs against the exe.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_gxumath::frustum::frustum_corners;
use benilla_re_gxumath::matrix::{lookat_v1, projection_ortho, projection_perspective};
use common::{assert_arr, fuzz, fuzz_pos};

const FRUSTUM_CORNERS: u64 = 0x5c43b0; // __fastcall: ecx=view, edx=proj, stack out[8×C3Vector]

fn wr_mat(e: &mut Emu, va: u64, m: &[f32; 16]) {
    for (k, &x) in m.iter().enumerate() {
        e.write_u32(va + 4 * k as u64, x.to_bits());
    }
}

fn rd3(e: &Emu, va: u64) -> [f32; 3] {
    [e.read_f32(va), e.read_f32(va + 4), e.read_f32(va + 8)]
}

fn rd_corners(e: &Emu, va: u64) -> [[f32; 3]; 8] {
    let mut c = [[0f32; 3]; 8];
    for (i, ci) in c.iter_mut().enumerate() {
        *ci = rd3(e, va + 0xc * i as u64); // C3Vector stride 0xc
    }
    c
}

/// An invertible view matrix: the target is offset from the eye by at least 20 on each axis and
/// `up` is near-vertical, so `lookat_v1` never reaches its degenerate gate.
fn synth_view(s: &mut u32) -> [f32; 16] {
    let eye = [fuzz(s, 80.0), fuzz(s, 80.0), fuzz(s, 80.0)];
    let target = [
        eye[0] + 60.0 + fuzz(s, 40.0), // each ≥ 20 in magnitude → len²(dir) ≥ 400
        eye[1] + 60.0 + fuzz(s, 40.0),
        eye[2] + 60.0 + fuzz(s, 40.0),
    ];
    let up = [fuzz(s, 2.0), 5.0 + fuzz(s, 2.0), fuzz(s, 2.0)];
    lookat_v1(eye, target, up)
}

/// A perspective projection (`m15 = 0`) with valid arguments.
fn synth_perspective(s: &mut u32) -> [f32; 16] {
    let fov = 0.1 + fuzz_pos(s, 2.8); // (0.1, 2.9] ⊂ (0, π)
    let aspect = 0.2 + fuzz_pos(s, 3.0);
    let near = 0.5 + fuzz_pos(s, 40.0);
    let far = near + 5.0 + fuzz_pos(s, 400.0); // strictly > near
    projection_perspective(fov, aspect, near, far)
}

/// An orthographic projection (`m15 = 1`) with valid arguments.
fn synth_ortho(s: &mut u32) -> [f32; 16] {
    let left = fuzz(s, 100.0);
    let right = left + 1.0 + fuzz_pos(s, 200.0);
    let bottom = fuzz(s, 100.0);
    let top = bottom + 1.0 + fuzz_pos(s, 200.0);
    let near = fuzz(s, 50.0);
    let far = near + 1.0 + fuzz_pos(s, 300.0);
    projection_ortho(left, right, bottom, top, near, far)
}

#[test]
fn frustum_corners_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let view_p = e.alloc(0x40);
    let proj_p = e.alloc(0x40);
    let out_p = e.alloc(0x60); // 8 × C3Vector
    let mut s: u32 = 0x5c43_0001;
    for i in 0..3000u32 {
        let view = synth_view(&mut s);
        let proj = if i % 2 == 0 {
            synth_perspective(&mut s)
        } else {
            synth_ortho(&mut s)
        };

        wr_mat(&mut e, view_p, &view);
        wr_mat(&mut e, proj_p, &proj);
        e.write(out_p, &[0u8; 0x60]);
        e.call(
            FRUSTUM_CORNERS,
            view_p as u32,
            proj_p as u32,
            &[out_p as u32],
        );

        let bin = rd_corners(&e, out_p);
        let ours = frustum_corners(&view, &proj);
        for (k, (b, o)) in bin.iter().zip(ours.iter()).enumerate() {
            assert_arr(b, o, &format!("frustum_corners[{i}].corner{k}"));
        }
    }
}
