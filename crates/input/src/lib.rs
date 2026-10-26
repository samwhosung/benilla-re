//! `InputControl.cpp`'s movement emitters (`systems/input.md`).

#![forbid(unsafe_code)]

/// StrafeLeft in the command word `+0x4`.
pub const STRAFE_LEFT: u32 = 0x40;
/// StrafeRight.
pub const STRAFE_RIGHT: u32 = 0x80;
/// TurnLeft.
pub const TURN_LEFT: u32 = 0x100;
/// TurnRight.
pub const TURN_RIGHT: u32 = 0x200;
/// The mouse turn command, the right button's.
pub const MOUSE_TURN: u32 = 0x1;
/// The latch that records a strafe start was sent.
pub const STRAFE_LATCH: u32 = 0x20000;

/// The sideways total of `0x514e80`: StrafeLeft +1, TurnLeft with the mouse turn command +1,
/// StrafeRight −1, TurnRight with the mouse turn command −1.
pub fn strafe_total(word: u32) -> i32 {
    let mouse = word & MOUSE_TURN != 0;
    let mut total = i32::from(word & STRAFE_LEFT != 0);
    total += i32::from(mouse && word & TURN_LEFT != 0);
    total -= i32::from(word & STRAFE_RIGHT != 0);
    total -= i32::from(mouse && word & TURN_RIGHT != 0);
    total
}

/// A call `0x514e80` makes, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrafeCall {
    /// `0x60dce0(time)`.
    Stop { time: u32 },
    /// The unit's stand state, its slot `+0xa4`.
    ReadStandState,
    /// `0x60be30(0)`.
    StandUp,
    /// `0x5eb830(0)`.
    ClearAfk,
    /// `0x60dcb0(time, left)`.
    Start { time: u32, left: bool },
}

/// `0x514e80(time, unit)` on the command word: the word it leaves and the calls it makes.
/// `stand_state` is what the unit's slot `+0xa4` returns and `is_player` bit 4 of its type word.
pub fn emit_strafe_axis(
    word: u32,
    time: u32,
    stand_state: u32,
    is_player: bool,
) -> (u32, Vec<StrafeCall>) {
    let total = strafe_total(word);
    let mut calls = Vec::new();
    if total == 0 {
        if word & STRAFE_LATCH != 0 {
            calls.push(StrafeCall::Stop { time });
            return (word & !STRAFE_LATCH, calls);
        }
        return (word, calls);
    }
    calls.push(StrafeCall::ReadStandState);
    if stand_state != 0 {
        calls.push(StrafeCall::StandUp);
    }
    if is_player {
        calls.push(StrafeCall::ClearAfk);
    }
    if word & STRAFE_LATCH != 0 {
        return (word, calls);
    }
    calls.push(StrafeCall::Start {
        time,
        left: total > 0,
    });
    (word | STRAFE_LATCH, calls)
}
