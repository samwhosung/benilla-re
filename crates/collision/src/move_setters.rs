//! The speed setters and the current speed
//! (`systems/collision/commands.md#speed-and-run-mode-setters`,
//! `systems/collision/kinematics.md#current-speed-selector`).

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x8029d4]`: 2^-22.
const SPEED_EPS: f64 = 2.384_185_791_015_625e-7;

/// `[0x8022cc]`: 1000.0.
const TRANSPORT_FACTOR: f32 = f32::from_bits(0x447a_0000);

/// The setters' test: store unless `|arg − field| < 2^-22`; NaN stores.
#[inline]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn eps_should_store(field: f32, arg: f32) -> bool {
    let diff = f(arg) - f(field);
    !(diff.abs() < SPEED_EPS)
}

/// One setter: the stored field and `eax`. `CMovement::SetTurnRate` (`0x7c6ff0`) writes `+0x9c`,
/// `SetRunSpeed` (`0x7c7030`) `+0x8c`, `SetRunBackSpeed` (`0x7c7080`) `+0x90`, `SetWalkSpeed`
/// (`0x7c70d0`) `+0x88`, `SetSwimSpeed` (`0x7c7120`) `+0x94` and `SetSwimBackSpeed` (`0x7c7170`)
/// `+0x98`.
#[inline]
pub fn set_speed_field(field: &mut f32, arg: f32) -> u32 {
    if eps_should_store(*field, arg) {
        *field = arg;
        1
    } else {
        0
    }
}

/// What `0x7c4c90` reads: flags `+0x40`, the speeds `+0x88` to `+0x98`, and the spline at `+0xa4`
/// (its `+0x18` flags, `+0x24` count and `+0x30` length).
pub struct GetSpeedState {
    pub flags: u32,
    pub walk: f32,
    pub run: f32,
    pub runback: f32,
    pub swim: f32,
    pub swimback: f32,
    pub transport: Option<(u8, i32, f32)>,
}

/// `CMovement::GetCurrentSpeed` (`0x7c4c90`,
/// `systems/collision/kinematics.md#current-speed-selector`): 0 without flags & 0xf; a live
/// spline's `+0x30 / +0x24 · 1000`; swimming, the smaller of swim back and swim when moving back,
/// else swim; with neither 0x100 nor `param`, the same of run back and run; otherwise the smaller
/// of walk and run.
pub fn get_speed(st: &GetSpeedState, param: u32) -> f32 {
    let flags = st.flags;
    if flags & 0xf == 0 {
        return 0.0;
    }
    if let Some((tflags, i24, f30)) = st.transport {
        if tflags & 0x4 == 0 {
            return ((f(f30) / f64::from(i24)) * f(TRANSPORT_FACTOR)) as f32;
        }
    }
    if flags & 0x0020_0000 != 0 {
        if flags & 0x2 != 0 {
            if st.swimback <= st.swim {
                st.swimback
            } else {
                st.swim
            }
        } else {
            st.swim
        }
    } else if flags & 0x100 == 0 && param == 0 {
        if flags & 0x2 != 0 {
            if st.runback <= st.run {
                st.runback
            } else {
                st.run
            }
        } else {
            st.run
        }
    } else if st.run > st.walk {
        st.walk
    } else {
        st.run
    }
}
