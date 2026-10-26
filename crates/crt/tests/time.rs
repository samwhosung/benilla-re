//! Calendar time (`systems/crt/time.md`): `gmtime` (`0x7421da`), `localtime` (`0x73f125`),
//! `_mktime` (`0x74066b`) with `__make_time_t` (`0x740687`), and `strftime` (`0x4134e3`) with
//! `_Strftime` (`0x4134fe`), `__expandtime` (`0x4135f8`), `__store_str` (`0x4139e7`),
//! `__store_num` (`0x413a0e`), `__store_number` (`0x413a7d`), `__store_winword` (`0x413ac6`) and
//! `__strcmpi` (`0x414240`), each run whole against `benilla_re_crt::time`.
//!
//! What the runs need from outside is supplied: `__getptd` (`0x40acbb`) answers a block whose
//! `+0x40` points at the `tm` buffer, so `_malloc` is not reached; `__lock` and `__unlock`
//! (`0x40be60`, `0x40bec1`) return at once; `InterlockedIncrement` and `InterlockedDecrement`
//! (the imports at `0x7ff2ac` and `0x7ff2b0`) return through `ret 4`; and the flag `0x883c60`
//! is set, so `___tzset` (`0x414561`) keeps the zone the test writes. `_daylight` is 0 in every
//! case, so `__isindst` (`0x41482c`) answers 0 at `0x41485a` and its rules are not run.

mod common;

use benilla_re_crt::time::{
    gmtime, localtime, mktime, strftime, MakeTime, TimeNames, Tm, Zone, C_NAMES,
};
use benilla_re_difftest::{Emu, Wow};
use common::Lcg;

const GMTIME: u64 = 0x7421da;
const LOCALTIME: u64 = 0x73f125;
const MKTIME: u64 = 0x74066b;
const STRFTIME: u64 = 0x4134e3;
const STRFTIME_LC: u64 = 0x4134fe;
const GETPTD: u64 = 0x40acbb;
const LOCK: u64 = 0x40be60;
const UNLOCK: u64 = 0x40bec1;
const TZSET_DONE: u64 = 0x883c60;
const TIMEZONE: u64 = 0x83274c;
const DAYLIGHT: u64 = 0x832750;
const DSTBIAS: u64 = 0x832754;

const NO_DST: &dyn Fn(&Tm) -> bool = &|_| false;

struct Rig<'a> {
    e: Emu<'a>,
    tm: u64,
    arg: u64,
    buf: u64,
    fmt: u64,
}

impl Rig<'_> {
    fn new(wow: &Wow) -> Rig<'_> {
        let mut e = Emu::new(wow);
        let ptd = e.alloc(0x100);
        let tm = e.alloc(0x40);
        e.write_u32(ptd + 0x40, tm as u32);
        e.stub(GETPTD, ptd as u32, 0);
        e.stub(LOCK, 0, 0);
        e.stub(UNLOCK, 0, 0);
        let ret4 = e.alloc(0x10);
        e.write(ret4, &[0xc2, 0x04, 0x00]);
        for slot in [0x7ff2ac, 0x7ff2b0] {
            e.write_u32(slot, ret4 as u32);
        }
        e.write_u32(TZSET_DONE, 1);
        let arg = e.alloc(0x40);
        let buf = e.alloc(0x400);
        let fmt = e.alloc(0x400);
        Rig {
            e,
            tm,
            arg,
            buf,
            fmt,
        }
    }

    fn zone(&mut self, z: &Zone) {
        self.e.write_u32(TIMEZONE, z.timezone as u32);
        self.e.write_u32(DAYLIGHT, u32::from(z.daylight));
        self.e.write_u32(DSTBIAS, z.dstbias as u32);
    }

    fn read_tm(&self, at: u64) -> Tm {
        Tm::from_words(std::array::from_fn(|i| {
            self.e.read_u32(at + 4 * i as u64) as i32
        }))
    }

    fn write_tm(&mut self, at: u64, tm: &Tm) {
        for (i, w) in tm.to_words().iter().enumerate() {
            self.e.write_u32(at + 4 * i as u64, *w as u32);
        }
    }

    /// `gmtime` or `localtime` of `t`: the fields, or `None` for a null answer.
    fn breakdown(&mut self, f: u64, t: i32) -> Option<Tm> {
        self.e.write_u32(self.arg, t as u32);
        self.e.write(self.tm, &[0x5a; 0x24]);
        let p = self.e.call(f, 0, 0, &[self.arg as u32]);
        (p != 0).then(|| {
            assert_eq!(u64::from(p), self.tm, "answered from the thread's buffer");
            self.read_tm(self.tm)
        })
    }
}

/// Times at every edge `gmtime` and `localtime` distinguish: the start, the first years and
/// their ends, the leap days, `localtime`'s three-day margins and the end of the range.
fn times(g: &mut Lcg) -> Vec<i32> {
    let mut v = vec![
        0,
        1,
        59,
        60,
        3599,
        3600,
        86_399,
        86_400,
        i32::MAX,
        -1,
        i32::MIN,
    ];
    for y in 0..70i64 {
        let start = y * 365 * 86_400 + ((y + 1) / 4) * 86_400;
        for d in [
            -1,
            0,
            1,
            58 * 86_400,
            59 * 86_400,
            60 * 86_400,
            364 * 86_400,
            365 * 86_400,
        ] {
            let t = start + d;
            if (0..=i64::from(i32::MAX)).contains(&t) {
                v.push(t as i32);
            }
        }
    }
    for edge in [259_200, i32::MAX - 259_200] {
        for d in [-90_000, -3_601, -1, 0, 1, 3_601, 90_000] {
            v.push(edge.wrapping_add(d));
        }
    }
    for _ in 0..4000 {
        v.push(g.next() as i32 & i32::MAX);
    }
    v
}

#[test]
fn crt_gmtime_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x71ae_0001);
    for t in times(&mut g) {
        assert_eq!(r.breakdown(GMTIME, t), gmtime(t), "gmtime({t})");
    }
}

/// Zones from fourteen hours east to twelve west, whole hours and not.
fn zones(g: &mut Lcg) -> Vec<Zone> {
    let mut v: Vec<Zone> = [0, 28_800, -3_600, 43_200, -50_400, 19_800, -34_200]
        .iter()
        .map(|&timezone| Zone {
            timezone,
            daylight: false,
            dstbias: -3_600,
        })
        .collect();
    for _ in 0..6 {
        v.push(Zone {
            timezone: g.below(26 * 3_600) as i32 - 14 * 3_600,
            daylight: false,
            dstbias: -(g.below(2 * 3_600) as i32),
        });
    }
    v
}

#[test]
fn crt_localtime_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x71ae_0002);
    let ts = times(&mut g);
    for z in zones(&mut g) {
        r.zone(&z);
        for &t in &ts {
            let want = localtime(t, &z, NO_DST);
            assert_eq!(r.breakdown(LOCALTIME, t), want, "localtime({t}) in {z:?}");
        }
    }
}

/// Fields from every regime `__make_time_t` distinguishes: years at and past 69 and 139, months
/// out of range either way, day counts and times that overflow a step, and each `isdst`.
fn fields(g: &mut Lcg) -> Tm {
    let pick = |g: &mut Lcg, v: &[i32]| v[g.below(v.len() as u64) as usize];
    let wide = |g: &mut Lcg| match g.below(5) {
        0 => g.next() as i32,
        1 => g.below(200) as i32 - 100,
        _ => g.below(70) as i32,
    };
    Tm {
        sec: wide(g),
        min: wide(g),
        hour: wide(g),
        mday: match g.below(4) {
            0 => g.next() as i32 >> g.below(31),
            _ => g.below(40) as i32 - 5,
        },
        mon: match g.below(4) {
            0 => g.below(400) as i32 - 200,
            _ => g.below(12) as i32,
        },
        year: match g.below(4) {
            0 => pick(g, &[68, 69, 70, 137, 138, 139, 140, -1, 0, 1900]),
            _ => 69 + g.below(71) as i32,
        },
        wday: g.next() as i32,
        yday: g.next() as i32,
        isdst: pick(g, &[-1, 0, 1, 7, -7]),
    }
}

#[test]
fn crt_mktime_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x71ae_0003);
    let at = r.e.alloc(0x40);
    let mut faults = 0;
    for z in zones(&mut g) {
        r.zone(&z);
        for _ in 0..3000 {
            let tm = fields(&mut g);
            let mut want_tm = tm;
            let want = mktime(&mut want_tm, &z, NO_DST);
            if want == MakeTime::Fault {
                faults += 1;
                continue;
            }
            r.write_tm(at, &tm);
            let got = r.e.call(MKTIME, 0, 0, &[at as u32]) as i32;
            let want_t = match want {
                MakeTime::Time(t) => t,
                _ => -1,
            };
            assert_eq!(got, want_t, "mktime({tm:?}) in {z:?}");
            assert_eq!(r.read_tm(at), want_tm, "mktime({tm:?}) fields in {z:?}");
        }
    }
    assert!(faults < 100, "{faults} cases fault");
}

/// A broken-down time with names in range and numbers anywhere.
fn any_tm(g: &mut Lcg) -> Tm {
    let num = |g: &mut Lcg| match g.below(6) {
        0 => g.next() as i32,
        1 => -(g.below(100) as i32),
        _ => g.below(400) as i32,
    };
    Tm {
        sec: num(g),
        min: num(g),
        hour: match g.below(3) {
            0 => num(g),
            _ => g.below(24) as i32,
        },
        mday: num(g),
        mon: g.below(12) as i32,
        year: match g.below(3) {
            0 => num(g),
            _ => 70 + g.below(70) as i32,
        },
        wday: g.below(7) as i32,
        yday: match g.below(3) {
            0 => num(g),
            _ => g.below(366) as i32,
        },
        isdst: g.below(3) as i32 - 1,
    }
}

/// A format of conversions, `#` marks, literals and runs of `%`, sometimes ending in a `%`.
fn format(g: &mut Lcg) -> Vec<u8> {
    let specs = b"aAbBcdHIjmMpSUwWxXyYzZ%qE ";
    let mut f = Vec::new();
    for _ in 0..g.below(12) {
        match g.below(4) {
            0 => f.push(b"-/: x."[g.below(6) as usize]),
            _ => {
                f.push(b'%');
                if g.below(3) == 0 {
                    f.push(b'#');
                }
                f.push(specs[g.below(specs.len() as u64) as usize]);
            }
        }
    }
    if g.below(10) == 0 {
        f.push(b'%');
    }
    f
}

/// Runs `strftime` or `_Strftime` with `lc` and compares the buffer and the count.
fn check(r: &mut Rig, g: &mut Lcg, lc: Option<(u64, &TimeNames)>, tzname: [&[u8]; 2]) {
    let tm = any_tm(g);
    let fmt = format(g);
    let max = match g.below(4) {
        0 => 2 + g.below(40) as usize,
        _ => 256,
    };
    // The format, its NUL, and a tail a final `%` reads on into.
    let mut mem = fmt.clone();
    mem.push(0);
    let tail: Vec<u8> = (0..g.below(6))
        .map(|_| b"%Yd-"[g.below(4) as usize])
        .collect();
    mem.extend(&tail);
    mem.extend([0; 16]);
    r.e.write(r.fmt, &mem);
    let prior: Vec<u8> = (0..max + 8).map(|_| g.next() as u8 | 1).collect();
    r.e.write(r.buf, &prior);
    let tm_at = r.arg;
    r.write_tm(tm_at, &tm);
    let got = match lc {
        None => r.e.call(
            STRFTIME,
            0,
            0,
            &[r.buf as u32, max as u32, r.fmt as u32, tm_at as u32],
        ),
        Some((at, _)) => r.e.call(
            STRFTIME_LC,
            0,
            0,
            &[
                r.buf as u32,
                max as u32,
                r.fmt as u32,
                tm_at as u32,
                at as u32,
            ],
        ),
    };
    let names = lc.map_or(&C_NAMES, |(_, n)| n);
    let mut want_buf = prior.clone();
    let want = strftime(&mut want_buf, max, &mem, &tm, names, tzname);
    let what = format!(
        "strftime({:?}, {max}, {tm:?})",
        String::from_utf8_lossy(&fmt)
    );
    assert_eq!(got as usize, want, "{what} count");
    assert_eq!(r.e.read(r.buf, max + 8), want_buf, "{what} buffer");
}

#[test]
fn crt_strftime_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x71ae_0004);
    for _ in 0..20_000 {
        check(&mut r, &mut g, None, [b"PST", b"PDT"]);
    }
}

/// Patterns for `%x`, `%#x` and `%X` from a record of the test's own: every letter run
/// `__store_winword` reads, quotes, designators, `am/pm` and `a/p` at the end, and literals.
fn pattern(g: &mut Lcg) -> Vec<u8> {
    let pieces: &[&[u8]] = &[
        b"M", b"MM", b"MMM", b"MMMM", b"MMMMM", b"d", b"dd", b"ddd", b"dddd", b"ddddd", b"y",
        b"yy", b"yyy", b"yyyy", b"h", b"hh", b"hhh", b"H", b"HH", b"HHH", b"m", b"mm", b"mmm",
        b"s", b"ss", b"sss", b"t", b"tt", b"ttt", b"a", b"A", b"aa", b"'", b"''", b"'lit'",
        b"'open", b"'''", b"/", b":", b", ", b" ", b"x", b"-", b"am/pm", b"A/P", b"AM/PM",
    ];
    let mut p = Vec::new();
    for _ in 0..1 + g.below(7) {
        p.extend_from_slice(pieces[g.below(pieces.len() as u64) as usize]);
    }
    match g.below(8) {
        0 => p.extend_from_slice(b" am/pm"),
        1 => p.extend_from_slice(b" A/P"),
        2 => p.extend_from_slice(b" AM/PM"),
        _ => {}
    }
    p
}

#[test]
fn crt_strftime_patterns_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x71ae_0005);
    let c = &C_NAMES;
    let count = 7 + 7 + 12 + 12 + 5;
    let record = r.e.alloc(4 * count as u64);
    let strings = r.e.alloc(0x30 * count as u64);
    for _ in 0..3000 {
        let pats = [pattern(&mut g), pattern(&mut g), pattern(&mut g)];
        // The record's strings in its order, each followed by zeros as far as a designator run
        // reads.
        let names: Vec<&[u8]> = c
            .short_days
            .iter()
            .chain(&c.days)
            .chain(&c.short_months)
            .chain(&c.months)
            .copied()
            .chain([c.am, c.pm, &pats[0][..], &pats[1][..], &pats[2][..]])
            .collect();
        for (i, s) in names.iter().enumerate() {
            let at = strings + 0x30 * i as u64;
            let mut z = s.to_vec();
            z.resize(0x30, 0);
            r.e.write(at, &z);
            r.e.write_u32(record + 4 * i as u64, at as u32);
        }
        let lc = TimeNames {
            short_date: &pats[0],
            long_date: &pats[1],
            time: &pats[2],
            ..C_NAMES
        };
        for _ in 0..4 {
            check(&mut r, &mut g, Some((record, &lc)), [b"PST", b"PDT"]);
        }
    }
}
