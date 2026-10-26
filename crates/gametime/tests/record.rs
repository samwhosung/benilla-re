//! The time record helpers against the exe (`systems/gametime/overview.md`, Time record helpers).
//! `WowTime::GetMinuteOfDay` (`0x6424c0`) and `WowTime::SetMinuteOfDay` (`0x6424e0`) run whole on
//! fields from a set with negatives, the extremes and the values each divides at.
//! `WowTime::AddDays` (`0x6423c0`) runs whole with `_mktime` (`0x74066b`) and `_localtime`
//! (`0x73f125`) replaced by UTC versions, the same ones the model is handed: the exe's read the
//! host's time zone through calls the emulator cannot make, so the test shows the arithmetic around
//! the calendar. `CopyLocalTime` (`0x642490`) runs as the exe has it, a refused time included.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_gametime::{add_days, minute_of_day, set_minute_of_day, Time, Tm};
use std::cell::RefCell;
use std::rc::Rc;

const GET_MINUTE_OF_DAY: u64 = 0x6424c0;
const SET_MINUTE_OF_DAY: u64 = 0x6424e0;
const ADD_DAYS: u64 = 0x6423c0;
const MKTIME: u64 = 0x74066b;
const LOCALTIME: u64 = 0x73f125;

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.next() as usize % xs.len()]
    }
}

const VALUES: [i32; 16] = [
    -1,
    -2,
    i32::MIN,
    i32::MAX,
    0,
    1,
    23,
    30,
    59,
    60,
    61,
    1439,
    1440,
    -60,
    -61,
    35_791_394,
];

fn put(e: &mut Emu, va: u64, t: &Time) {
    for (i, v) in t.iter().enumerate() {
        e.write_u32(va + 4 * i as u64, *v as u32);
    }
}

fn get(e: &Emu, va: u64) -> Time {
    let mut t = [0i32; 7];
    for (i, v) in t.iter_mut().enumerate() {
        *v = e.read_u32(va + 4 * i as u64) as i32;
    }
    t
}

#[test]
fn minute_of_day_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x20);
    let mut g = Lcg(0x6424_c000);
    let mut n = 0u32;
    for i in 0..20_000u32 {
        let mut t = [0i32; 7];
        for v in t.iter_mut() {
            *v = if i % 3 == 0 {
                g.next() as i32 - 0x40_0000
            } else {
                g.pick(&VALUES)
            };
        }
        put(&mut e, rec, &t);
        let bin = e.call(GET_MINUTE_OF_DAY, rec as u32, 0, &[]) as i32;
        assert_eq!(bin, minute_of_day(&t), "{t:?}");
        n += 1;
    }
    eprintln!("minute_of_day: {n} cases bit-exact vs WoW.exe @ 0x6424c0");
}

#[test]
fn set_minute_of_day_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x20);
    let mut g = Lcg(0x6424_e000);
    let mut n = 0u32;
    for i in 0..20_000u32 {
        let mut t = [0x5a5a_5a5au32 as i32; 7];
        let minutes = if i % 2 == 0 {
            g.pick(&VALUES)
        } else {
            (g.next() as i32).wrapping_mul(if i % 4 == 1 { -1 } else { 1 })
        };
        // `i32::MIN / 60` divides; only `idiv` of `i32::MIN` by -1 would fault.
        put(&mut e, rec, &t);
        e.call(SET_MINUTE_OF_DAY, rec as u32, 0, &[minutes as u32]);
        set_minute_of_day(&mut t, minutes);
        assert_eq!(get(&e, rec), t, "{minutes}");
        n += 1;
    }
    eprintln!("set_minute_of_day: {n} cases bit-exact vs WoW.exe @ 0x6424e0");
}

/// Days from 1970-01-01 of a proleptic Gregorian date, its month counted from 0 and allowed past
/// 11 or below 0, its day of the month from 1 and allowed past the month's end.
fn days_from_civil(year: i64, month: i64, mday: i64) -> i64 {
    let year = year + month.div_euclid(12);
    let m = month.rem_euclid(12) + 1;
    let y = year - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468 + mday - 1
}

/// `mktime` in UTC: the `tm`'s fields normalized together, the result cut to 32 bits.
fn utc_mktime(tm: &Tm) -> i32 {
    let days = days_from_civil(1900 + i64::from(tm[5]), i64::from(tm[4]), i64::from(tm[3]));
    let s = days * 86_400 + i64::from(tm[2]) * 3600 + i64::from(tm[1]) * 60 + i64::from(tm[0]);
    s as i32
}

/// `localtime` in UTC; null for a negative time, as the C library of the client gives.
fn utc_localtime(t: i32) -> Option<Tm> {
    if t < 0 {
        return None;
    }
    let t = i64::from(t);
    let days = t.div_euclid(86_400);
    let rem = t.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let mday = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 2 } else { mp - 10 };
    let year = yoe + era * 400 + i64::from(month <= 1);
    let jan1 = days_from_civil(year, 0, 1);
    Some([
        (rem % 60) as i32,
        (rem / 60 % 60) as i32,
        (rem / 3600) as i32,
        mday as i32,
        month as i32,
        (year - 1900) as i32,
        (days + 4).rem_euclid(7) as i32,
        (days - jan1) as i32,
        0,
    ])
}

fn read_tm(io: &benilla_re_difftest::StubIo, va: u64) -> Tm {
    let mut tm = [0i32; 9];
    for (i, v) in tm.iter_mut().enumerate() {
        *v = io.read_u32(va + 4 * i as u64) as i32;
    }
    tm
}

#[test]
fn add_days_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x20);
    let tm_out = e.alloc(0x40);
    let seen: Rc<RefCell<Vec<(Tm, i32)>>> = Rc::default();
    // `_mktime(tm)`, cdecl.
    let s = seen.clone();
    e.stub_fn(MKTIME, 0, move |io| {
        let tm = read_tm(io, u64::from(io.arg(0)));
        s.borrow_mut().push((tm, 0));
        utc_mktime(&tm) as u32
    });
    // `_localtime(&t)`, cdecl: a pointer to a static `tm`, or null.
    let s = seen.clone();
    e.stub_fn(LOCALTIME, 0, move |io| {
        let t = io.read_u32(u64::from(io.arg(0))) as i32;
        s.borrow_mut().push(([0; 9], t));
        match utc_localtime(t) {
            Some(tm) => {
                for (i, v) in tm.iter().enumerate() {
                    io.write_u32(tm_out + 4 * i as u64, *v as u32);
                }
                tm_out as u32
            }
            None => 0,
        }
    });
    let mut g = Lcg(0x6423_c000);
    let mut n = 0u32;
    for i in 0..20_000u32 {
        let mut t: Time = [
            (g.next() % 70) as i32 - 5,
            (g.next() % 30) as i32 - 3,
            (g.next() % 9) as i32 - 1,
            (g.next() % 35) as i32 - 2,
            (g.next() % 15) as i32 - 2,
            (g.next() % 40) as i32 - 2,
            (g.next() % 4) as i32,
        ];
        if i % 50 == 0 {
            t[5] = g.pick(&[0, 37, 38, 39, 100, 500]);
        }
        let days = match i % 5 {
            0 => 1,
            1 => -1,
            2 => (g.next() % 800) as i32 - 400,
            3 => g.pick(&[0, 365, -365, 24_855, -24_855, 30_000]),
            _ => (g.next() % 3) as i32 - 1,
        };
        let keep = i % 3 == 0;
        put(&mut e, rec, &t);
        seen.borrow_mut().clear();
        e.call(ADD_DAYS, rec as u32, 0, &[days as u32, u32::from(keep)]);
        let calls: RefCell<Vec<(Tm, i32)>> = RefCell::default();
        let mut want = t;
        add_days(
            &mut want,
            days,
            keep,
            &mut |tm| {
                calls.borrow_mut().push((*tm, 0));
                utc_mktime(tm)
            },
            &mut |s| {
                calls.borrow_mut().push(([0; 9], s));
                utc_localtime(s)
            },
        );
        assert_eq!(
            *seen.borrow(),
            calls.into_inner(),
            "calls for {t:?} {days} {keep}"
        );
        assert_eq!(get(&e, rec), want, "record for {t:?} {days} {keep}");
        n += 1;
    }
    eprintln!("add_days: {n} cases bit-exact vs WoW.exe @ 0x6423c0");
}
