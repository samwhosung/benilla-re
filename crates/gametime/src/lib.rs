//! The game clock (`GAMETIMECBSTRUCT`): the server-synced time of day that drives the day and
//! night cycle. All x87 arithmetic is at 53-bit precision (`f64`); `f32` operands widen exactly.
#![allow(clippy::float_cmp)]

#[inline]
fn f(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

/// `[0x801360]`, 0.001 as `f32` (`0x3a83126f`): milliseconds to seconds.
pub const MS_TO_S_BITS: u32 = 0x3a83_126f;
/// `[0x80e460]`, 1440.0: minutes per day.
pub const MIN_PER_DAY_BITS: u32 = 0x44b4_0000;
/// `[0x804480]`, 1/1440 as `f32`.
pub const INV_MIN_PER_DAY_BITS: u32 = 0x3a36_0b61;

/// `0x643a70`: the time of day as a fraction of a day. For the clock at `ecx`,
/// `elapsed = tick() − [+0x38]` as `u32` (the `fild` reads a 64-bit integer with a zero high
/// dword), `phase = elapsed·0.001·[+0x2c] + [+0x3c]`, less 1440 while above 1440, times 1/1440.
/// The result is the unrounded `st0`, which the caller uses as it is.
pub fn day_phase(tick: u32, base: u32, scale: f32, offset: f32) -> f64 {
    let elapsed = tick.wrapping_sub(base); // fild of [elapsed:0], zero-extended
    let mut phase = (elapsed as f64) * f(MS_TO_S_BITS) * f64::from(scale) + f64::from(offset);
    let day = f(MIN_PER_DAY_BITS);
    while phase > day {
        phase -= day;
    }
    phase * f(INV_MIN_PER_DAY_BITS)
}

/// `0x6435a0(dt)`: the clock's per-frame minute accumulator, returning the new `[+0x30]`.
/// `result = dt·[+0x2c] + [+0x30]`, stored as `f32`. If `pending = [+0x34]` is not 0 and
/// `result >= 1.0`, `clamped = min(trunc(result), pending)` (`__ftol`, then `cmp; jae`, unsigned)
/// is taken from both (`fsubr`). Then `while result >= 1.0`, `result -= 1.0` and `0x643ad0`
/// advances the clock one minute without touching `result`.
pub fn minute_accum(dt: f32, scale: f32, accum: f32, pending: i32) -> f32 {
    let mut result = (f64::from(dt) * f64::from(scale) + f64::from(accum)) as f32;
    let mut e = pending;
    if e != 0 && f64::from(result) >= 1.0 {
        let t = benilla_re_cmath::x87::ftol(f64::from(result)); // __ftol truncates; result >= 1, so t >= 1
        let clamped = if (e as u32) >= (t as u32) { t } else { e }; // cmp edi,t; jae
        e -= clamped;
        let _ = e; // the new [+0x34], not returned
        result = (f64::from(result) - f64::from(clamped)) as f32; // fsubr
    }
    while f64::from(result) >= 1.0 {
        result = (f64::from(result) - 1.0) as f32; // fld; fsub 1.0; fstp
    }
    result
}

/// The float arithmetic of `0x643ad0`, `0x643b11` to `0x643b1e`: the new minute offset `[+0x3c]`
/// is `fild new_minute; fadd [+0x30]; fstp`, `(double)new_minute + accum` stored as `f32`. The
/// rest of the function, the next minute mod 1440, the day change, the observers and the tick
/// base, is integer work.
pub fn advance_offset(new_minute: i32, accum: f32) -> f32 {
    (f64::from(new_minute) + f64::from(accum)) as f32
}

/// `[0x80e45c]`, 1/60 as `f32` (`0x3c888889`): the lowest time scale and the clock's default.
pub const SCALE_MIN_BITS: u32 = 0x3c88_8889;
/// `[0x80e448]`, 60.0 as `f32` (`0x42700000`): the highest time scale.
pub const SCALE_MAX_BITS: u32 = 0x4270_0000;

/// `0x643820`: the value `[+0x2c]` takes for the argument `scale`. An argument below 1/60
/// (`-0.0` and the negatives included) gives 1/60, one above 60.0 gives 60.0, and a NaN goes
/// through `fld` and `fstp`: a quiet one keeps its bits and a signalling one is stored with its
/// quiet bit set. The test leaves signalling NaNs out, since the emulator's `fld` does not quiet them.
pub fn set_scale(scale: f32) -> f32 {
    let (min, max) = (
        f32::from_bits(SCALE_MIN_BITS),
        f32::from_bits(SCALE_MAX_BITS),
    );
    if scale < min {
        min
    } else if scale > max {
        max
    } else if scale.is_nan() {
        f32::from_bits(scale.to_bits() | 0x0040_0000)
    } else {
        scale
    }
}

/// A time record (`0x1c` bytes, the first `0x1c` bytes of the clock): minute, hour, weekday, day,
/// month, year and a two-bit field, each an `i32`, −1 for an unset field.
pub type Time = [i32; 7];

/// `0x642b30` through `0x642c30`: the fields of a packed date, each `(shift, width)`; a field
/// whose bits are all ones is −1.
pub fn decode_time(packed: u32) -> Time {
    const FIELDS: [(u32, u32); 7] = [(0, 6), (6, 5), (11, 3), (14, 6), (20, 4), (24, 5), (29, 2)];
    let mut t = [0i32; 7];
    for (slot, &(shift, width)) in t.iter_mut().zip(FIELDS.iter()) {
        let mask = (1u32 << width) - 1;
        let v = (packed >> shift) & mask;
        *slot = if v == mask { -1 } else { v as i32 };
    }
    t
}

/// `0x6424c0`: `hour·60 + minute`, 0 when either is negative.
pub fn minute_of_day(t: &Time) -> i32 {
    if t[1] < 0 || t[0] < 0 {
        0
    } else {
        t[1].wrapping_mul(60).wrapping_add(t[0])
    }
}

/// `0x6424e0`: hour and minute from a number of minutes, `idiv` by 60 (the quotient truncates
/// toward zero and the remainder takes the dividend's sign).
pub fn set_minute_of_day(t: &mut Time, minutes: i32) {
    t[1] = minutes / 60;
    t[0] = minutes % 60;
}

/// The clock's fields the set and the sync read and write: its time, `+0x20` (minute offset),
/// `+0x24` (day offset) and `+0x34` (minutes of frame time still to skip).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    pub time: Time,
    pub minute_offset: i32,
    pub day_offset: i32,
    pub skip: i32,
}

/// A call the set or the sync makes on a function the test stubs, in call order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// `0x6423c0(days, 0)` on a record: the record as it was at the call.
    AddDays(Time, i32, i32),
    /// `0x643ad0()` on the clock, as it was at the call.
    Advance(Time),
    /// `0x642c80(time, buffer, 0x400)`: the time it formats.
    Format(Time),
    /// `0x6dd240(0x10, 0xa, "GameTimeSync: skipping forwards …", d, current, new)`.
    LogSkip(i32),
    /// `0x6dd240(0x10, 0xa, "GameTimeSync: delta=…", 0, skip, minute of day)`.
    LogDelta(i32, i32, i32),
}

/// `0x6434d0` and `0x6436c0`, the head they share: the sent time with the clock's minute offset
/// added to its minute of day (the sum wraps by `+1440` when negative, else `% 1440`), then its
/// day offset added through `0x6423c0`.
fn offset_time(c: &Clock, t: &mut Time, steps: &mut Vec<Step>) {
    if c.minute_offset != 0 {
        let m = minute_of_day(t).wrapping_add(c.minute_offset);
        let m = if m < 0 {
            m.wrapping_add(1440)
        } else {
            m % 1440
        };
        set_minute_of_day(t, m);
    }
    if c.day_offset != 0 {
        steps.push(Step::AddDays(*t, c.day_offset, 0));
    }
}

/// `0x6434d0(time, force)`: the clock takes the offset time; with `force` it then steps back one
/// minute (minute 0 becomes 59 of the previous hour, hour 0 of the previous day, through
/// `0x6423c0(-1, 0)`) and advances one minute.
pub fn game_time_set(c: &mut Clock, time: &Time, force: bool) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut t = *time;
    offset_time(c, &mut t, &mut steps);
    c.time = t;
    if force {
        if c.time[0] == 0 {
            c.time[0] = 59;
            if c.time[1] == 0 {
                c.time[1] = 23;
                steps.push(Step::AddDays(c.time, -1, 0));
            } else {
                c.time[1] -= 1;
            }
        } else {
            c.time[0] -= 1;
        }
        steps.push(Step::Advance(c.time));
    }
    steps
}

/// `0x6436c0(time, force)`, with `day_number` standing for `0x642320`. `d` is
/// `((day(sent) − day(clock))·1440 − minuteOfDay(clock) + minuteOfDay(sent)) % 1440`, signed as
/// its dividend. Without `force` and `d <= 0` the clock takes the sent time, `+0x34` grows by `−d`
/// and the clock advances `−d` minutes; otherwise it advances `d` minutes if positive (logging a
/// `d` above 1 first) and then takes the sent time.
pub fn game_time_sync(
    c: &mut Clock,
    time: &Time,
    force: bool,
    day_number: &dyn Fn(&Time) -> i32,
) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut t = *time;
    offset_time(c, &mut t, &mut steps);
    let mut dividend = day_number(&t).wrapping_sub(day_number(&c.time));
    dividend = dividend.wrapping_mul(1440);
    dividend = dividend.wrapping_sub(minute_of_day(&c.time));
    dividend = dividend.wrapping_add(minute_of_day(&t));
    let d = dividend % 1440;
    let mut stalled = 0;
    if !force && d <= 0 {
        stalled = d.wrapping_neg();
    } else {
        if d > 1 {
            steps.push(Step::Format(t));
            steps.push(Step::Format(c.time));
            steps.push(Step::LogSkip(d));
        }
        for _ in 0..d.max(0) {
            steps.push(Step::Advance(c.time));
        }
    }
    c.time = t;
    if stalled != 0 {
        c.skip = c.skip.wrapping_add(stalled);
        for _ in 0..stalled.max(0) {
            steps.push(Step::Advance(c.time));
        }
        steps.push(Step::LogDelta(0, c.skip, minute_of_day(&c.time)));
    }
    steps
}

/// The three words `0x6c5f50` stores in the light state at `+0`, `+4` and `+8`: the minute of day,
/// the day phase `st0` of `0x643a70` rounded once to a float, and the day number as a float.
pub fn light_stamp(clock: &Time, phase: f64, day_number: i32) -> [u32; 3] {
    [
        minute_of_day(clock) as u32,
        (phase as f32).to_bits(),
        (f64::from(day_number) as f32).to_bits(),
    ]
}

/// A message handler's reads from the data store the dispatcher hands it: the store holds the
/// two opcode bytes and then the body, and the cursor starts after the opcode.
#[derive(Clone, Copy, Debug)]
pub struct Store {
    pub size: u32,
    pub read: u32,
}

impl Store {
    pub fn new(body_len: usize) -> Store {
        Store {
            size: 2 + body_len as u32,
            read: 2,
        }
    }

    /// A dword read (`0x418eb0`, `0x419130`): past the end it writes nothing and moves the cursor
    /// to `size + 1`.
    pub fn get_u32(&mut self, body: &[u8]) -> Option<u32> {
        if self.read.wrapping_add(4) > self.size {
            self.read = self.size.wrapping_add(1);
            return None;
        }
        let at = (self.read - 2) as usize;
        self.read += 4;
        Some(u32::from_le_bytes([
            body[at],
            body[at + 1],
            body[at + 2],
            body[at + 3],
        ]))
    }

    /// `0x401d10`: the cursor is at the size.
    pub fn is_read(&self) -> bool {
        self.read == self.size
    }
}

/// A call a game-time handler makes outside the data store, in call order.
#[derive(Clone, Debug)]
pub enum Call {
    /// `0x63cd00(format, 0, opcode, size, read)`, the format `Malformed message recieved: …`.
    Malformed { opcode: u32, size: u32, read: u32 },
    /// `0x64a7f0(buffer, 0x100, "Gamespeed set from %.03f to %.03f", old, new)`.
    SpeedLine { old: f64, new: f64 },
    /// `0x63cb50(buffer, 0)` on the buffer the last `0x64a7f0` filled.
    ConsoleWrite { class: u32 },
    /// `0x6434d0(time, force)` on the clock.
    SetTime { time: Time, force: bool },
    /// `0x6436c0(time, force)` on the clock.
    SyncTime { time: Time, force: bool },
    /// `0x6c5f50()`.
    Bridge,
    /// `0x642c80(time, buffer, 0x80)` on the decoded date, returning the buffer.
    FormatTime { time: Time },
    /// `0x64a7f0(buffer, 0x100, "Server game time is %s", text)`, the text being the formatter's buffer.
    TimeLine,
}

impl PartialEq for Call {
    fn eq(&self, other: &Call) -> bool {
        // A NaN rate compares by being a NaN, since a host widening may differ in its payload.
        let same = |a: f64, b: f64| a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan());
        match (self, other) {
            (
                Call::Malformed {
                    opcode: a,
                    size: b,
                    read: c,
                },
                Call::Malformed {
                    opcode: d,
                    size: e,
                    read: f,
                },
            ) => (a, b, c) == (d, e, f),
            (Call::SpeedLine { old: a, new: b }, Call::SpeedLine { old: c, new: d }) => {
                same(*a, *c) && same(*b, *d)
            }
            (Call::ConsoleWrite { class: a }, Call::ConsoleWrite { class: b }) => a == b,
            (Call::SetTime { time: a, force: b }, Call::SetTime { time: c, force: d }) => {
                (a, b) == (c, d)
            }
            (Call::SyncTime { time: a, force: b }, Call::SyncTime { time: c, force: d }) => {
                (a, b) == (c, d)
            }
            (Call::Bridge, Call::Bridge) => true,
            (Call::FormatTime { time: a }, Call::FormatTime { time: b }) => a == b,
            (Call::TimeLine, Call::TimeLine) => true,
            _ => false,
        }
    }
}

/// What a handler returns in `eax`, the calls it makes, and the scale the clock holds after it.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub eax: u32,
    pub calls: Vec<Call>,
    pub scale: f32,
}

/// The malformed-message line, shared by every handler: the packet was not read to its end.
fn malformed(opcode: u32, s: Store, scale: f32) -> Outcome {
    Outcome {
        eax: 0,
        calls: vec![Call::Malformed {
            opcode,
            size: s.size,
            read: s.read,
        }],
        scale,
    }
}

/// `0x6c5de0`, opcode `0x47`: a float rate. Read to its end, it is passed to `0x643820`, and the
/// line prints the clock's old scale and the rate as received.
pub fn game_speed_set(opcode: u32, body: &[u8], old_scale: f32) -> Outcome {
    let mut s = Store::new(body.len());
    let rate = s.get_u32(body).map(f32::from_bits);
    let Some(rate) = rate.filter(|_| s.is_read()) else {
        return malformed(opcode, s, old_scale);
    };
    Outcome {
        eax: 1,
        calls: vec![
            Call::SpeedLine {
                old: f64::from(old_scale),
                new: f64::from(rate),
            },
            Call::ConsoleWrite { class: 0 },
        ],
        scale: set_scale(rate),
    }
}

/// `0x6c6010`, opcode `0x43`: a packed date, decoded and synced without `force`.
pub fn game_time_update(opcode: u32, body: &[u8], scale: f32) -> Outcome {
    let mut s = Store::new(body.len());
    let packed = s.get_u32(body);
    let Some(packed) = packed.filter(|_| s.is_read()) else {
        return malformed(opcode, s, scale);
    };
    Outcome {
        eax: 1,
        calls: vec![Call::SyncTime {
            time: decode_time(packed),
            force: false,
        }],
        scale,
    }
}

/// `0x6c6120`, opcode `0x45`: a packed date, decoded and set with `force`, then `0x6c5f50`.
pub fn game_time_set_packet(opcode: u32, body: &[u8], scale: f32) -> Outcome {
    let mut s = Store::new(body.len());
    let packed = s.get_u32(body);
    let Some(packed) = packed.filter(|_| s.is_read()) else {
        return malformed(opcode, s, scale);
    };
    Outcome {
        eax: 1,
        calls: vec![
            Call::SetTime {
                time: decode_time(packed),
                force: true,
            },
            Call::Bridge,
        ],
        scale,
    }
}

/// `0x6c6080`, opcode `0x49`: a packed date, decoded, formatted and written to the console; the
/// clock is untouched.
pub fn server_time(opcode: u32, body: &[u8], scale: f32) -> Outcome {
    let mut s = Store::new(body.len());
    let packed = s.get_u32(body);
    let Some(packed) = packed.filter(|_| s.is_read()) else {
        return malformed(opcode, s, scale);
    };
    Outcome {
        eax: 1,
        calls: vec![
            Call::FormatTime {
                time: decode_time(packed),
            },
            Call::TimeLine,
            Call::ConsoleWrite { class: 0 },
        ],
        scale,
    }
}
