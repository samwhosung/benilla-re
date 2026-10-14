//! The camera's pivot bias is a pitch about the camera's own left axis, not a rotation about the
//! world's vertical (`systems/uicamera/driver.md#where-each-channel-enters`).
//!
//! The basis at `+0x14` is built each frame by `0x50f710` through `0x50ab50` and
//! `BuildYawPitchRollBasis` (`0x7bf4b0`) from `(yaw, pitch, roll)`, and `0x50ee58` then calls
//! `0x7be730`, which builds `BuildAxisRotation` (`0x7be490`) about `(0, 1, 0)` (the vector at
//! `0xbe0f20`) and multiplies it with `MultiplyMatrix3` (`0x7bdfc0`) with the basis as the right
//! factor. Each of the three is run against the exe in the tests beside this one, so the
//! composition needs no emulation of its own. With `roll = 0` the identity is
//! `multiply_matrix3(axis_angle_mat3((0, 1, 0), θ), euler_to_mat3_f(yaw, pitch, 0)) ==
//! euler_to_mat3_f(yaw, pitch + θ, 0)`.
//!
//! Two controls let it fail: the world-axis reading, the factors the other way round, must
//! disagree where the basis does not commute with a rotation about y; and with `roll != 0` the
//! identity must break, which is why it holds for a roll of 0 only (`+0xf8` is written only by the
//! constructor's zero).

use benilla_re_cmath::multiply::multiply_matrix3 as mat3_mul;
use benilla_re_cmath::rotation::{axis_angle_mat3, euler_to_mat3_f};

#[test]
fn camera_pivot_bias_is_a_local_pitch() {
    const AXIS: [f32; 3] = [0.0, 1.0, 0.0];
    // The +-89.00deg pitch clamp, `.rdata` 0x8089d8 / 0x8089dc, written from its bits so the
    // literal is the binary's own value rather than a decimal that rounds to it.
    const PITCH_CLAMP: f32 = f32::from_bits(0x3fc6_d3f2);
    let maxdiff = |a: &[f32; 9], b: &[f32; 9]| -> f32 {
        (0..9).fold(0.0f32, |m, i| m.max((a[i] - b[i]).abs()))
    };

    // (yaw, pitch, theta) in radians. Includes theta = 0, the 0.001 deadband edge 0x50ee3a
    // refuses below, a mid-range drag, and pitches at both ends of the +-89deg clamp.
    let probes: [(f32, f32, f32); 8] = [
        (0.0, 0.0, 0.0),
        (0.0, 0.0, 0.001),
        (0.7, 0.2, 0.1),
        (-2.3, -0.9, 0.35),
        (3.0, PITCH_CLAMP, -0.4),
        (-1.1, -PITCH_CLAMP, 0.4),
        (1.9, 0.75, 1.0),
        (0.35, -0.05, -1.2),
    ];

    let mut local_worst = 0.0f32;
    let mut world_worst = 0.0f32;
    for (yaw, pitch, theta) in probes {
        let basis = euler_to_mat3_f(&[yaw, pitch, 0.0]);
        let rot = axis_angle_mat3(&AXIS, theta, true);
        let expect = euler_to_mat3_f(&[yaw, pitch + theta, 0.0]);

        // The exe: R * M -- pre-multiply, which mixes the target's ROWS, i.e. a LOCAL axis.
        let got = mat3_mul(&rot, &basis);
        let d = maxdiff(&got, &expect);
        assert!(
            d < 2.0e-6,
            "R*M != basis(yaw, pitch+theta) at yaw={yaw} pitch={pitch} theta={theta}: maxdiff {d:e}"
        );
        local_worst = local_worst.max(d);

        // The refuted alternative: M * R -- post-multiply, mixing COLUMNS, i.e. a WORLD axis.
        world_worst = world_worst.max(maxdiff(&mat3_mul(&basis, &rot), &expect));
    }

    // The control, on one named probe: the two readings coincide wherever the basis commutes with a
    // rotation about y (a basis of yaw alone, and every probe with theta == 0), so the control is
    // taken where they differ, with yaw and pitch both nonzero.
    let (yaw, pitch, theta) = (0.7f32, 0.2f32, 0.1f32);
    let basis = euler_to_mat3_f(&[yaw, pitch, 0.0]);
    let rot = axis_angle_mat3(&AXIS, theta, true);
    let expect = euler_to_mat3_f(&[yaw, pitch + theta, 0.0]);
    let world = maxdiff(&mat3_mul(&basis, &rot), &expect);
    assert!(
        world > 1.0e-2,
        "non-vacuity: at yaw={yaw} pitch={pitch} theta={theta} the world-axis reading M*R was \
         indistinguishable from the exe's R*M (maxdiff {world:e})"
    );
    assert!(
        maxdiff(&mat3_mul(&rot, &basis), &expect) < 2.0e-6,
        "the discriminating probe must still satisfy the LOCAL reading"
    );

    // Scoping control: with roll != 0 the composition is still a rotation about the basis's own
    // row 1, but it is NO LONGER the scalar substitution pitch -> pitch + theta.
    let (yaw, pitch, roll, theta) = (0.6f32, 0.3f32, 0.5f32, 0.4f32);
    let rolled = mat3_mul(
        &axis_angle_mat3(&AXIS, theta, true),
        &euler_to_mat3_f(&[yaw, pitch, roll]),
    );
    let naive = euler_to_mat3_f(&[yaw, pitch + theta, roll]);
    let d = maxdiff(&rolled, &naive);
    assert!(
        d > 1.0e-2,
        "scoping control: 'pitch + theta' unexpectedly held at roll != 0 (maxdiff {d:e})"
    );

    eprintln!(
        "camera_pivot_bias_is_a_local_pitch: 8 probes, worst local error {local_worst:e}, \
         world-axis reading differs by up to {world_worst:e}"
    );
}
