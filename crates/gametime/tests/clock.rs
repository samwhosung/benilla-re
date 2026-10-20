//! The clock's setters and the time-changed bridge against the exe (`systems/gametime.md`). Each
//! function runs whole. A callee a test replaces is named with the reason: the host-time
//! functions `0x642320` and `0x6423c0` (`mktime`), the minute advance `0x643ad0`, the tick
//! `0x42c010`, the time formatter `0x642c80` and the sinks of text. The sinks and `0x6dd240` only
//! record, and `0x6dd240` is a bare `ret`, so replacing it changes no result.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::RegisterX86::{ECX, EDX};
use benilla_re_difftest::{Emu, StubIo, Wow};
use benilla_re_gametime::{
    day_phase, decode_time, game_time_set, game_time_sync, light_stamp, set_scale, Clock, Step,
    Time, SCALE_MAX_BITS, SCALE_MIN_BITS,
};
use std::cell::RefCell;
use std::rc::Rc;

const SET_SCALE: u64 = 0x643820;
const DECODE: u64 = 0x642c30;
const SET: u64 = 0x6434d0;
const SYNC: u64 = 0x6436c0;
const BRIDGE: u64 = 0x6c5f50;
const ADVANCE: u64 = 0x643ad0;
const ADD_DAYS: u64 = 0x6423c0;
const DAY_NUMBER: u64 = 0x642320;
const FORMAT_TIME: u64 = 0x642c80;
const LOG: u64 = 0x6dd240;
const TICK: u64 = 0x42c010;
const SKY: u64 = 0x66ff60;
const SPRINTF: u64 = 0x64a7f0;
const CONSOLE_WRITE: u64 = 0x63cb50;
/// The clock the bridge reaches by its absolute address, the light state it stamps and the four
/// callback slots.
const CLOCK: u64 = 0xce8538;
const LIGHT: u64 = 0xce9b60;
const CALLBACKS: u64 = 0xce85a8;

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        (self.next() >> 8) % n
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u32) as usize]
    }
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo + 1) as u32) as i32
    }
}

fn read_time(io: &StubIo, va: u64) -> Time {
    let b = io.read(va, 28);
    let mut t = [0i32; 7];
    for (i, slot) in t.iter_mut().enumerate() {
        *slot = i32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
    }
    t
}

fn write_time(emu: &mut Emu, va: u64, t: &Time) {
    for (i, v) in t.iter().enumerate() {
        emu.write_u32(va + 4 * i as u64, *v as u32);
    }
}

fn read_time_emu(emu: &Emu, va: u64) -> Time {
    let mut t = [0i32; 7];
    for (i, slot) in t.iter_mut().enumerate() {
        *slot = emu.read_u32(va + 4 * i as u64) as i32;
    }
    t
}

/// The day number the tests give `0x642320`: 0 for an unset year, month or day, else a function of
/// the date alone (the exe's own is `mktime`, which depends on the host's time zone).
fn day_number(t: &Time) -> i32 {
    if t[5] < 0 || t[4] < 0 || t[3] < 0 {
        0
    } else {
        t[5].wrapping_mul(372)
            .wrapping_add(t[4] * 31)
            .wrapping_add(t[3])
    }
}

/// A time with each field in range, unset (−1), or past its range (the packed fields reach 31
/// hours and 63 minutes).
fn time(g: &mut Lcg) -> Time {
    let mut t = [
        g.range(0, 59),
        g.range(0, 23),
        g.range(0, 6),
        g.range(0, 30),
        g.range(0, 11),
        g.range(0, 31),
        g.range(0, 2),
    ];
    if g.below(5) == 0 {
        t[0] = g.pick(&[-1, 0, 59, 60, 63]);
        t[1] = g.pick(&[-1, 0, 23, 24, 31]);
    }
    if g.below(12) == 0 {
        t[3 + g.below(3) as usize] = -1;
    }
    t
}

/// A float that is not a signalling NaN: the emulator's `fld` keeps one signalling where the
/// processor quiets it (`docs/METHOD.md`, the harness).
fn quiet(bits: u32) -> u32 {
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        bits | 0x0040_0000
    } else {
        bits
    }
}

/// `0x643820` runs whole through `call_f64`, which reads the old scale left in `st0`, and the
/// stored scale is read from `[+0x2c]`. The probes sit at each bound and a step either side, at
/// zero, the infinities, the denormals and the quiet NaNs, then at random bit patterns.
#[test]
fn gametime_643820_set_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x80);
    let mut total = 0u32;
    let mut run = |emu: &mut Emu, old: u32, arg: u32| {
        emu.write_u32(this + 0x2c, old);
        let ret = emu.call_f64(SET_SCALE, this as u32, 0, &[arg]);
        let stored = emu.read_u32(this + 0x2c);
        let want = set_scale(f32::from_bits(arg)).to_bits();
        assert_eq!(stored, want, "stored scale for {arg:#x}");
        if f32::from_bits(old).is_nan() {
            assert!(ret.is_nan(), "the old NaN comes back as a NaN");
        } else {
            assert_eq!(
                ret.to_bits(),
                f64::from(f32::from_bits(old)).to_bits(),
                "old scale {old:#x} returned"
            );
        }
        total += 1;
    };
    let (lo, hi) = (SCALE_MIN_BITS, SCALE_MAX_BITS);
    let args = [
        lo - 1,
        lo,
        lo + 1,
        hi - 1,
        hi,
        hi + 1,
        0,
        0x8000_0000,
        1,
        0x8000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x3f80_0000,
        0xbf80_0000,
        0x4270_0001,
        0x426f_ffff,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0xffc0_0000,
        0x7fc1_2345,
        0xffe0_0001,
        0x3c88_8888,
        0x3c88_888a,
    ];
    let olds = [lo, hi, 0x3f80_0000, 0, 0x7fc0_0000, 0x4048_f5c3];
    for &o in &olds {
        for &a in &args {
            run(&mut emu, o, a);
        }
    }
    let mut g = Lcg(0x6438_2000);
    for _ in 0..20000 {
        let o = quiet(g.next());
        let a = match g.below(3) {
            0 => quiet(g.next()),
            1 => (g.next() >> 7) & 0x7fff_ffff | (g.next() & 0x8000_0000),
            _ => ((g.below(70_000) as f32) / 1000.0 * if g.below(2) == 0 { 1.0 } else { -1.0 })
                .to_bits(),
        };
        run(&mut emu, o, quiet(a));
    }
    eprintln!("gametime_643820_set_scale: {total} bit-exact (clamp and returned old scale)");
}

/// `0x642c30` runs whole, calling `0x642b30` for the fields: each field at its smallest, its
/// largest and the all-ones value alone and with the others, bit 31, then random dwords.
#[test]
fn gametime_642c30_decode_time() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out = emu.alloc(0x40);
    let mut total = 0u32;
    let mut run = |emu: &mut Emu, packed: u32| {
        emu.write(out, &[0xa5; 0x40]);
        emu.call(DECODE, packed, out as u32, &[]);
        let got = read_time_emu(emu, out);
        assert_eq!(got, decode_time(packed), "decode {packed:#x}");
        assert_eq!(
            emu.read(out + 28, 4),
            vec![0xa5; 4],
            "nothing past the record"
        );
        total += 1;
    };
    run(&mut emu, 0);
    run(&mut emu, 0xffff_ffff);
    run(&mut emu, 0x8000_0000);
    run(&mut emu, 0x7fff_ffff);
    let fields: [(u32, u32); 7] = [(0, 6), (6, 5), (11, 3), (14, 6), (20, 4), (24, 5), (29, 2)];
    for &(shift, width) in &fields {
        let mask = (1u32 << width) - 1;
        for v in [0, 1, mask - 1, mask] {
            run(&mut emu, v << shift);
            run(&mut emu, !(mask << shift) | (v << shift));
        }
    }
    let mut g = Lcg(0x642c_3000);
    for _ in 0..20000 {
        run(&mut emu, g.next());
    }
    eprintln!("gametime_642c30_decode_time: {total} bit-exact (seven packed fields)");
}

/// The calls the setters make on stubbed functions, recorded as [`Step`]s.
struct Rig {
    steps: Rc<RefCell<Vec<Step>>>,
    problems: Rc<RefCell<Vec<String>>>,
}

fn rig(emu: &mut Emu, this: u64) -> Rig {
    let steps: Rc<RefCell<Vec<Step>>> = Rc::default();
    let problems: Rc<RefCell<Vec<String>>> = Rc::default();
    let formats: Rc<RefCell<Vec<u32>>> = Rc::default();
    // `0x643ad0`: __thiscall, no stack arguments; the clock is `this`.
    let (s, p) = (steps.clone(), problems.clone());
    emu.stub_fn(ADVANCE, 0, move |io| {
        if io.reg(ECX) != this {
            p.borrow_mut().push("advance on another object".into());
        }
        s.borrow_mut().push(Step::Advance(read_time(io, this)));
        0
    });
    // `0x6423c0(days, 0)`: __thiscall on a record, two stack arguments.
    let s = steps.clone();
    emu.stub_fn(ADD_DAYS, 2, move |io| {
        let rec = io.reg(ECX);
        s.borrow_mut().push(Step::AddDays(
            read_time(io, rec),
            io.arg(0) as i32,
            io.arg(1) as i32,
        ));
        0
    });
    // `0x642c80(time, buffer, size)`: returns the buffer, `ret 4`.
    let (s, f, p) = (steps.clone(), formats.clone(), problems.clone());
    emu.stub_fn(FORMAT_TIME, 1, move |io| {
        let rec = io.reg(ECX);
        let buf = io.reg(EDX) as u32;
        if io.arg(0) != 0x400 {
            p.borrow_mut().push("format size".into());
        }
        s.borrow_mut().push(Step::Format(read_time(io, rec)));
        f.borrow_mut().push(buf);
        buf
    });
    // `0x6dd240(category, level, format, …)`: a bare `ret`, cdecl.
    let (s, f, p) = (steps.clone(), formats, problems.clone());
    emu.stub_fn(LOG, 0, move |io| {
        if (io.arg(0), io.arg(1)) != (0x10, 0xa) {
            p.borrow_mut().push("log category and level".into());
        }
        match io.arg(2) {
            0x8658e8 => {
                // (d, current text, new text): the buffers the two formatter calls returned.
                let f = f.borrow();
                let n = f.len();
                if n < 2 || (io.arg(4), io.arg(5)) != (f[n - 1], f[n - 2]) {
                    p.borrow_mut().push("skip log text arguments".into());
                }
                s.borrow_mut().push(Step::LogSkip(io.arg(3) as i32));
            }
            0x8658ac => s.borrow_mut().push(Step::LogDelta(
                io.arg(3) as i32,
                io.arg(4) as i32,
                io.arg(5) as i32,
            )),
            other => p.borrow_mut().push(format!("log format {other:#x}")),
        }
        0
    });
    Rig { steps, problems }
}

/// Writes the model's clock into the object, around a pattern that shows any stray write.
fn load(emu: &mut Emu, this: u64, c: &Clock) -> Vec<u8> {
    let mut bytes = vec![0xcdu8; 0x80];
    for (i, v) in c.time.iter().enumerate() {
        bytes[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
    }
    bytes[0x20..0x24].copy_from_slice(&c.minute_offset.to_le_bytes());
    bytes[0x24..0x28].copy_from_slice(&c.day_offset.to_le_bytes());
    bytes[0x34..0x38].copy_from_slice(&c.skip.to_le_bytes());
    emu.write(this, &bytes);
    bytes
}

/// The object's bytes as the model leaves them: the pattern with the model's fields stored.
fn expect(mut bytes: Vec<u8>, c: &Clock) -> Vec<u8> {
    for (i, v) in c.time.iter().enumerate() {
        bytes[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
    }
    bytes[0x34..0x38].copy_from_slice(&c.skip.to_le_bytes());
    bytes
}

fn offsets(g: &mut Lcg) -> (i32, i32) {
    let minute = match g.below(5) {
        0 => g.range(-3000, 3000),
        1 => g.pick(&[1, -1, 1439, 1440, -1440, -1441, 2879]),
        _ => 0,
    };
    let day = if g.below(5) == 0 { g.range(-3, 3) } else { 0 };
    (minute, day)
}

/// `0x6434d0` runs whole; the minute advance and the day step are replaced by recorders (their
/// bodies work on the host's calendar), and `0x6424c0` and `0x6424e0` run. The cases cover an unset
/// and an out-of-range field, minute 0 with hour 0 and not, with and without `force`, and the
/// offsets.
#[test]
fn gametime_6434d0_set_time() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x80);
    let rec = emu.alloc(0x40);
    let r = rig(&mut emu, this);
    let mut g = Lcg(0x6434_d000);
    let mut total = 0u32;
    for i in 0..30000u32 {
        let (mo, dof) = offsets(&mut g);
        let mut clock = Clock {
            time: time(&mut g),
            minute_offset: mo,
            day_offset: dof,
            skip: g.next() as i32,
        };
        let mut sent = time(&mut g);
        if i % 7 == 0 {
            // minute 0 and hour 0, the step back into the previous day
            sent[0] = 0;
            sent[1] = if i % 14 == 0 { 0 } else { g.range(1, 23) };
        }
        let force = g.below(3) != 0;
        let bytes = load(&mut emu, this, &clock);
        write_time(&mut emu, rec, &sent);
        r.steps.borrow_mut().clear();
        emu.call(SET, this as u32, 0, &[rec as u32, u32::from(force)]);
        let want = game_time_set(&mut clock, &sent, force);
        assert!(r.problems.borrow().is_empty(), "{:?}", r.problems.borrow());
        assert_eq!(
            emu.read(this, 0x80),
            expect(bytes, &clock),
            "clock after {sent:?} force={force}"
        );
        assert_eq!(
            *r.steps.borrow(),
            want,
            "calls for {sent:?} force={force} {mo} {dof}"
        );
        total += 1;
    }
    eprintln!("gametime_6434d0_set_time: {total} bit-exact (offsets, step back, advance)");
}

/// `0x6436c0` runs whole with `0x642320` replaced by [`day_number`], the other callees as in the
/// set. The cases put the sent time behind, at and ahead of the clock by 0, 1, 2, a few, an
/// hour, half a day, a day less one, a day and more, with and without `force`, from an empty
/// skip count and a full one.
#[test]
fn gametime_6436c0_sync_time() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x80);
    let rec = emu.alloc(0x40);
    let r = rig(&mut emu, this);
    emu.stub_fn(DAY_NUMBER, 0, |io| {
        let t = read_time(io, io.reg(ECX));
        day_number(&t) as u32
    });
    let mut g = Lcg(0x6436_c000);
    let mut total = 0u32;
    let mut signs = [0u32; 3];
    for i in 0..30000u32 {
        let (mo, dof) = if i % 3 == 0 { offsets(&mut g) } else { (0, 0) };
        let mut clock = Clock {
            time: time(&mut g),
            minute_offset: mo,
            day_offset: dof,
            skip: g.pick(&[0, 0, 5, -3, 1000, i32::MAX - 5]),
        };
        // The sent time: the clock's, moved by a number of minutes through the day and hour
        // fields, or an unrelated one.
        let mut sent = clock.time;
        if g.below(8) != 0 && sent.iter().all(|f| *f >= 0) {
            let delta = g.pick(&[
                0, 1, -1, 2, -2, 3, 5, -7, 59, 60, -60, 61, 719, -720, 1000, 1438, -1438, 1439,
                -1439, 1440, -1440, 1441, 2880, -2000, 3000,
            ]);
            let m = (sent[1] * 60 + sent[0] + delta).rem_euclid(1440);
            let carry = (sent[1] * 60 + sent[0] + delta).div_euclid(1440);
            sent[1] = m / 60;
            sent[0] = m % 60;
            sent[3] = (sent[3] + carry).clamp(0, 30);
        } else {
            sent = time(&mut g);
        }
        let force = g.below(4) == 0;
        let bytes = load(&mut emu, this, &clock);
        write_time(&mut emu, rec, &sent);
        r.steps.borrow_mut().clear();
        emu.call(SYNC, this as u32, 0, &[rec as u32, u32::from(force)]);
        let before = clock;
        let want = game_time_sync(&mut clock, &sent, force, &day_number);
        assert!(r.problems.borrow().is_empty(), "{:?}", r.problems.borrow());
        assert_eq!(
            emu.read(this, 0x80),
            expect(bytes, &clock),
            "clock after {sent:?} force={force} from {before:?}"
        );
        assert_eq!(
            *r.steps.borrow(),
            want,
            "calls for {sent:?} force={force} from {before:?}"
        );
        signs[if want.is_empty() {
            0
        } else if clock.skip != before.skip {
            1
        } else {
            2
        }] += 1;
        total += 1;
    }
    assert!(
        signs.iter().all(|&n| n > 500),
        "every path is taken: {signs:?}"
    );
    eprintln!(
        "gametime_6436c0_sync_time: {total} bit-exact (set only {}, stall {}, forward {})",
        signs[0], signs[1], signs[2]
    );
}

/// `0x6c5f50` runs whole against the real clock object and light state: the tick is 0, the day
/// number is [`day_number`], and the sky update, the formatter and the text sinks only record.
/// The cases put the clock at random times, scales and bases, and fill the four callback slots at
/// random.
#[test]
fn gametime_6c5f50_time_changed() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(TICK, 0, 0);
    emu.stub_fn(DAY_NUMBER, 0, |io| {
        let t = read_time(io, io.reg(ECX));
        day_number(&t) as u32
    });
    let log: Rc<RefCell<Vec<String>>> = Rc::default();
    let problems: Rc<RefCell<Vec<String>>> = Rc::default();
    let l = log.clone();
    emu.stub_fn(SKY, 0, move |io| {
        let light: Vec<u32> = (0..3).map(|i| io.read_u32(LIGHT + 4 * i)).collect();
        l.borrow_mut().push(format!(
            "sky {:#x} {:#x} {light:x?}",
            io.reg(ECX),
            io.reg(EDX)
        ));
        0
    });
    let (l, p) = (log.clone(), problems.clone());
    let formats: Rc<RefCell<Vec<u32>>> = Rc::default();
    let f = formats.clone();
    emu.stub_fn(FORMAT_TIME, 1, move |io| {
        if io.reg(ECX) != CLOCK {
            p.borrow_mut().push("format of another time".into());
        }
        l.borrow_mut().push(format!("format {:#x}", io.arg(0)));
        f.borrow_mut().push(io.reg(EDX) as u32);
        io.reg(EDX) as u32
    });
    let (l, p) = (log.clone(), problems.clone());
    let buffers: Rc<RefCell<Vec<u32>>> = Rc::default();
    let (b, f) = (buffers.clone(), formats.clone());
    emu.stub_fn(SPRINTF, 0, move |io| {
        // (buffer, size, format, text): the text is the buffer the formatter returned
        b.borrow_mut().push(io.arg(0));
        if io.arg(1) != 0x100 || f.borrow().last().copied() != Some(io.arg(3)) {
            p.borrow_mut().push("printf size or text".into());
        }
        l.borrow_mut().push(format!("printf {:#x}", io.arg(2)));
        0
    });
    let (l, p, b) = (log.clone(), problems.clone(), buffers.clone());
    emu.stub_fn(CONSOLE_WRITE, 0, move |io| {
        if b.borrow().last().copied() != Some(io.reg(ECX) as u32) {
            p.borrow_mut()
                .push("console text is not the printf buffer".into());
        }
        l.borrow_mut()
            .push(format!("console class {}", io.reg(EDX)));
        0
    });
    let mut islands = [0u32; 4];
    for (i, slot) in islands.iter_mut().enumerate() {
        let a = emu.alloc(0x10);
        emu.write(a, &[0xc3]);
        let l = log.clone();
        emu.stub_fn(a, 0, move |io| {
            l.borrow_mut().push(format!(
                "callback {i} hour={} minute={}",
                io.reg(ECX),
                io.reg(EDX)
            ));
            0
        });
        *slot = a as u32;
    }
    let mut g = Lcg(0x6c5f_5000);
    let mut total = 0u32;
    for _ in 0..4000u32 {
        let t = time(&mut g);
        write_time(&mut emu, CLOCK, &t);
        let scale = f32::from_bits(SCALE_MIN_BITS)
            + (g.below(1 << 20) as f32 / (1 << 20) as f32)
                * (60.0 - f32::from_bits(SCALE_MIN_BITS));
        // at most 5e7 ms of elapsed time, so the day reduction in `0x643a70` loops a few thousand times
        let base = 0u32.wrapping_sub(g.below(50_000_000));
        let offset = (g.below(2_000_000) as f32 - 100_000.0) / 1000.0;
        emu.write_u32(CLOCK + 0x2c, scale.to_bits());
        emu.write_u32(CLOCK + 0x38, base);
        emu.write_u32(CLOCK + 0x3c, offset.to_bits());
        let mut set = [0u32; 4];
        for (i, slot) in set.iter_mut().enumerate() {
            if g.below(2) == 0 {
                *slot = islands[i];
            }
            emu.write_u32(CALLBACKS + 4 * i as u64, *slot);
        }
        emu.write(LIGHT, &[0x5a; 12]);
        log.borrow_mut().clear();
        buffers.borrow_mut().clear();
        formats.borrow_mut().clear();
        emu.call(BRIDGE, 0, 0, &[]);
        assert!(problems.borrow().is_empty(), "{:?}", problems.borrow());
        let phase = day_phase(0, base, scale, offset);
        let light = light_stamp(&t, phase, day_number(&t));
        let mut want = vec![format!("sky 0x1 0x0 {light:x?}")];
        for (i, s) in set.iter().enumerate() {
            if *s != 0 {
                want.push(format!(
                    "callback {i} hour={} minute={}",
                    t[1] as u32, t[0] as u32
                ));
            }
        }
        want.push("format 0x80".into());
        want.push("printf 0x86c50c".into());
        want.push("console class 0".into());
        assert_eq!(*log.borrow(), want, "calls at {t:?}");
        assert_eq!(emu.read(LIGHT, 12), {
            let mut b = Vec::new();
            for w in light {
                b.extend_from_slice(&w.to_le_bytes());
            }
            b
        });
        total += 1;
    }
    eprintln!("gametime_6c5f50_time_changed: {total} bit-exact (light state, sky update, callbacks, line)");
}
