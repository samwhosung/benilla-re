//! The rotation builders against the exe, each run whole: the rotation about an axis in a 3×3
//! (`0x7be490`) and a 3×4 (`0x7bb860`), the basis from yaw, pitch and roll (`0x7bf4b0`) and a
//! quaternion applied on the left of a 4×4 (`0x7bddb0`). The axes are the edge fills and random
//! bit patterns, each both flagged as unit and not; the angles reach past a turn in both
//! directions. A NaN result matches any NaN.
//!
//! Host double (`docs/METHOD.md#the-harness`): `euler_to_mat3_f` runs `fsincos` at `0x7bf4d2`,
//! `0x7bf52a` and `0x7bf582`. `axis_angle_mat34` runs `fsincos` at `0x7bb8c7`. `axis_angle_mat3`
//! runs `fsincos` at `0x7be4f7`. The emulator computes each in host `double`, so where its result
//! reaches a compared value the host's math stands in for the x87's, and the test shows the code
//! around it.

mod common;

use benilla_re_cmath::{quaternion as quat, rotation as rot};
use common::*;

#[test]
fn axis_angle_mat34() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let axes = synth_n(3);
    let angles = synth_angles();
    let mut checked = 0u64;
    for (k, axis) in axes.iter().enumerate() {
        let angle = angles[k % angles.len()];
        for norm in [0u32, 1u32] {
            d.put(d.a, axis);
            // ecx=dst, edx=axis ptr, [ebp+8]=angle, [ebp+0xc]=normalized bool
            d.emu
                .call(0x7bb860, d.dst as u32, d.a as u32, &[angle.to_bits(), norm]);
            assert_slice(
                &d.out_n(12),
                &rot::axis_angle_mat34(arr(axis.as_slice()), angle, norm != 0),
                "axis_angle_mat34",
            );
            checked += 1;
        }
    }
    eprintln!("axis_angle_mat34: {checked} cases bit-exact vs WoW.exe @ 0x7bb860");
}

#[test]
fn axis_angle_mat3() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let axes = synth_n(3);
    let angles = synth_angles();
    let mut checked = 0u64;
    for (k, axis) in axes.iter().enumerate() {
        let angle = angles[k % angles.len()];
        for norm in [0u32, 1u32] {
            d.put(d.a, axis);
            // ecx=dst (3×3), edx=axis ptr, [ebp+8]=angle, [ebp+0xc]=normalized bool
            d.emu
                .call(0x7be490, d.dst as u32, d.a as u32, &[angle.to_bits(), norm]);
            assert_slice(
                &d.out_n(9),
                &rot::axis_angle_mat3(arr(axis.as_slice()), angle, norm != 0),
                "axis_angle_mat3",
            );
            checked += 1;
        }
    }
    eprintln!("axis_angle_mat3: {checked} cases bit-exact vs WoW.exe @ 0x7be490");
}

#[test]
fn quat_mul4() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let dsts = synth_n(16);
    let quats = synth_n(4);
    for (k, dst_in) in dsts.iter().enumerate() {
        let q = &quats[(k + 1) % quats.len()];
        d.put(d.dst, dst_in); // dst is in/out: holds the input 4×4
        d.put(d.a, q);
        // ecx=dst (in/out), [ebp+8]=quaternion pointer
        d.emu.call(0x7bddb0, d.dst as u32, 0, &[d.a as u32]);
        assert_slice(
            &d.out_n(16),
            &quat::quat_mul4(arr(dst_in.as_slice()), arr(q.as_slice())),
            "quat_mul4",
        );
    }
    eprintln!(
        "quat_mul4: {} cases bit-exact vs WoW.exe @ 0x7bddb0",
        dsts.len()
    );
}

#[test]
fn euler_to_mat3_f() {
    check_eulerbuild("euler_to_mat3_f", 0x7bf4b0, rot::euler_to_mat3_f);
}
