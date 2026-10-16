//! The play-time warning (`systems/ui/billing.md`): the arm in `CGGameUI::InitializeGame`
//! (`0x48f480`, from `0x48f636`) and the poll in `CGGameUI::Update` (`0x495590`, from `0x495679`),
//! over the three words `0xb4e360`, `0xb4e364` and `0xb4e368`.
//!
//! The x87 runs at 53-bit precision, so the minutes product is an `f64` one.

/// `[0x804590]` (`.rdata`), the `f32` that scales milliseconds to minutes: `1.6666666851961054e-05`,
/// a hair over 1/60000.
pub const MINUTES_PER_MS_BITS: u32 = 0x378b_cf65;
/// The longest remaining time that arms the warning, in minutes (`cmp esi,0x10e0` at `0x48f647`).
pub const ARM_MAX_MINUTES: u32 = 0x10e0;
/// A minute in milliseconds (`imul esi,esi,0xea60` at `0x48f666`).
pub const MINUTE_MS: u32 = 60_000;
/// The line cadence above five minutes left (`mov ecx,0x493e0` at `0x4957f6`).
pub const LINE_STEP_MS: u32 = 300_000;
/// Bit 1 of the plan flags (`test byte [eax+0x1afc],2` at `0x48f654`): the plan arms no warning.
pub const FLAG_NO_WARNING: u8 = 0x02;
/// Bit 3 of the plan flags (`shr ecx,3; and ecx,1` at `0x4956cb`): the internet-cafe plan.
pub const FLAG_INTERNET_CAFE: u8 = 0x08;

/// The three words: the deadline (`0xb4e360`, 0 when unarmed), the next line's time (`0xb4e364`,
/// 0 before the first poll) and the dialog latch (`0xb4e368`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Warning {
    pub deadline: u32,
    pub next_line: u32,
    pub dialog_shown: u32,
}

/// What a poll shows, in the order it shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// `BILLING_NAG_DIALOG` (event 0x205) with the minutes left (`0x495700`, `0x49581e`).
    Dialog(u32),
    /// `IGR_BILLING_NAG_DIALOG` (event 0x206, `0x4956e9`, `0x495741`).
    InternetCafeDialog,
    /// A `CHAT_MSG_SYSTEM` line of `BILLING_NAG_WARNING` for the minutes left (`0x4957d1`), or
    /// `held` in the pending queue while the display latch is set (`0x4957ad`).
    Line { minutes: u32, held: bool },
}

/// The minutes the poll reads (`0x49569b` to `0x4956b7`): `left` is `deadline - now` as a dword,
/// zero-extended into a 64-bit `fild`, multiplied by the `f32` at `0x804590` at 53 bits, rounded up
/// by `ceil` (`0x73fdf5`) and truncated by `__ftol` (`0x40a2b0`).
pub fn minutes_left(left: u32) -> u32 {
    let k = f64::from(f32::from_bits(MINUTES_PER_MS_BITS));
    (f64::from(left) * k).ceil() as u32
}

/// The arm (`0x48f636` to `0x48f679`): the deadline goes to 0, then to `now + remaining · 60000`
/// with the other two words cleared when `0 < remaining <= 4320` and the flags have bit 1 clear.
pub fn arm(prev: Warning, remaining: u32, flags: u8, now: u32) -> Warning {
    let mut w = prev;
    w.deadline = 0;
    if remaining <= ARM_MAX_MINUTES && flags & FLAG_NO_WARNING == 0 && remaining > 0 {
        w.deadline = now.wrapping_add(remaining.wrapping_mul(MINUTE_MS));
        w.next_line = 0;
        w.dialog_shown = 0;
    }
    w
}

/// One poll (`0x495679` to `0x495826`) at clock `now` with the session's plan `flags` and whether
/// the chat display latch (`0x8435fc`) holds lines.
pub fn poll(w: &mut Warning, now: u32, flags: u8, held: bool) -> Vec<Shown> {
    let mut out = Vec::new();
    if w.deadline == 0 {
        return out;
    }
    let left = w.deadline.wrapping_sub(now);
    let minutes = minutes_left(left);
    let cafe = flags & FLAG_INTERNET_CAFE != 0;
    if w.next_line == 0 {
        // The first poll (`0x4956d8` to `0x49570b`): the dialog when due, with no latch test, and
        // the line clock starts now.
        if cafe {
            if minutes <= 600 {
                out.push(Shown::InternetCafeDialog);
                w.dialog_shown = 1;
            }
        } else if minutes <= 30 {
            out.push(Shown::Dialog(minutes));
            w.dialog_shown = 1;
        }
        w.next_line = now;
    } else if (now.wrapping_sub(w.next_line) as i32) < 0 {
        // Before the line clock (`0x49574f`): straight to the dialogs.
    } else if !cafe {
        // A line is due (`0x495755` to `0x4957ff`).
        if minutes <= 30 {
            out.push(Shown::Line { minutes, held });
        }
        if (now.wrapping_sub(w.deadline) as i32) >= 0 {
            w.deadline = 0;
        } else {
            let step = if minutes <= 5 {
                MINUTE_MS
            } else {
                LINE_STEP_MS
            };
            w.next_line = now.wrapping_add(left % step);
        }
    }
    // The dialogs, once (`0x495723`, `0x495805`).
    if cafe {
        if minutes <= 600 && w.dialog_shown == 0 {
            out.push(Shown::InternetCafeDialog);
            w.dialog_shown = 1;
        }
    } else if minutes <= 30 && w.dialog_shown == 0 {
        out.push(Shown::Dialog(minutes));
        w.dialog_shown = 1;
    }
    out
}
