//! The time record comparisons and the daytime test against the exe, each run whole on records
//! built in the scratch arena: the field compares `0x6425f0` to `0x6426e0`, the orderings
//! `0x642770`, `0x6428c0`, `0x642a10`, `0x642880` and `0x6429d0`, the range test `0x642710`, the
//! packing `0x642ae0`, and `IsDaytime` (`0x60a050`) with the clock at `0xce8538`. Nothing is
//! stubbed. The record fields draw from a set with negatives (unset), the smallest and largest
//! `i32` and values that meet, so every skip and every tie is reached.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_gametime::timecmp::{
    compare_field, encode_time, in_range, is_daytime, is_equal, is_greater, is_greater_or_equal,
    is_less, is_less_or_equal,
};
use benilla_re_gametime::Time;

const COMPARES: [u64; 6] = [0x6425f0, 0x642620, 0x642650, 0x642680, 0x6426b0, 0x6426e0];
/// The field each compare reads, as an index into a record: year, month, day, weekday, hour,
/// minute.
const COMPARE_FIELD: [usize; 6] = [5, 4, 3, 2, 1, 0];
const LESS: u64 = 0x642770;
const GREATER: u64 = 0x6428c0;
const EQUAL: u64 = 0x642a10;
const LESS_OR_EQUAL: u64 = 0x642880;
const GREATER_OR_EQUAL: u64 = 0x6429d0;
const IN_RANGE: u64 = 0x642710;
const ENCODE: u64 = 0x642ae0;
const IS_DAYTIME: u64 = 0x60a050;
const CLOCK: u64 = 0xce8538;

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[((self.next() >> 8) as usize) % xs.len()]
    }
}

/// Values a field takes: unset and other negatives, the extremes, and a few that collide.
const VALUES: [i32; 12] = [-1, -2, i32::MIN, 0, 1, 2, 5, 21, 30, 59, 0x7fff_ffff, 1999];

fn random_time(g: &mut Lcg) -> Time {
    let mut t = [0i32; 7];
    for v in t.iter_mut() {
        *v = g.pick(&VALUES);
    }
    t
}

/// A record that differs from `base` in at most one field, to reach every tie before a decision.
fn near(g: &mut Lcg, base: &Time) -> Time {
    let mut t = *base;
    if g.next() & 3 != 0 {
        let f = (g.next() >> 8) as usize % 7;
        t[f] = g.pick(&VALUES);
    }
    t
}

fn put(e: &mut Emu, va: u64, t: &Time) {
    for (i, v) in t.iter().enumerate() {
        e.write_u32(va + 4 * i as u64, *v as u32);
    }
}

#[test]
fn time_order_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let a = e.alloc(0x20);
    let b = e.alloc(0x20);
    let mut g = Lcg(0x6425_f000);
    let mut cases = 0u32;
    for n in 0..6000 {
        let ta = random_time(&mut g);
        let tb = if n % 2 == 0 {
            random_time(&mut g)
        } else {
            near(&mut g, &ta)
        };
        put(&mut e, a, &ta);
        put(&mut e, b, &tb);
        for (call, &field) in COMPARES.iter().zip(&COMPARE_FIELD) {
            let bin = e.call(*call, a as u32, 0, &[b as u32]) as i32;
            assert_eq!(
                bin,
                compare_field(ta[field], tb[field]),
                "{call:#x} {ta:?} {tb:?}"
            );
        }
        // The orderings return in `al` (`0x642770`, `0x6428c0`, `0x642a10`) or in all of `eax`.
        let al = |v: u32| v & 0xff;
        assert_eq!(
            al(e.call(LESS, a as u32, 0, &[b as u32])),
            u32::from(is_less(&ta, &tb)),
            "less {ta:?} {tb:?}"
        );
        assert_eq!(
            al(e.call(GREATER, a as u32, 0, &[b as u32])),
            u32::from(is_greater(&ta, &tb)),
            "greater {ta:?} {tb:?}"
        );
        assert_eq!(
            al(e.call(EQUAL, a as u32, 0, &[b as u32])),
            u32::from(is_equal(&ta, &tb)),
            "equal {ta:?} {tb:?}"
        );
        assert_eq!(
            e.call(LESS_OR_EQUAL, a as u32, 0, &[b as u32]),
            u32::from(is_less_or_equal(&ta, &tb)),
            "less or equal {ta:?} {tb:?}"
        );
        assert_eq!(
            e.call(GREATER_OR_EQUAL, a as u32, 0, &[b as u32]),
            u32::from(is_greater_or_equal(&ta, &tb)),
            "greater or equal {ta:?} {tb:?}"
        );
        cases += 1;
    }
    // The same record by address: neither less nor greater, equal to itself.
    for _ in 0..200 {
        let ta = random_time(&mut g);
        put(&mut e, a, &ta);
        assert_eq!(e.call(LESS, a as u32, 0, &[a as u32]) & 0xff, 0);
        assert_eq!(e.call(GREATER, a as u32, 0, &[a as u32]) & 0xff, 0);
        assert_eq!(e.call(EQUAL, a as u32, 0, &[a as u32]) & 0xff, 1);
        assert!(!is_less(&ta, &ta) && !is_greater(&ta, &ta) && is_equal(&ta, &ta));
        cases += 1;
    }
    eprintln!("time_order: {cases} cases bit-exact vs WoW.exe @ 0x6425f0..0x6429d0");
}

#[test]
fn time_in_range_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x20);
    let from = e.alloc(0x20);
    let to = e.alloc(0x20);
    let mut g = Lcg(0x6427_1000);
    let mut cases = 0u32;
    for n in 0..30000 {
        let tf = random_time(&mut g);
        let tt = match n % 3 {
            0 => random_time(&mut g),
            _ => near(&mut g, &tf),
        };
        let tx = match n % 4 {
            0 => random_time(&mut g),
            1 => near(&mut g, &tf),
            2 => near(&mut g, &tt),
            _ => near(&mut g, &tf),
        };
        put(&mut e, this, &tx);
        put(&mut e, from, &tf);
        put(&mut e, to, &tt);
        let bin = e.call(IN_RANGE, this as u32, 0, &[from as u32, to as u32]);
        assert_eq!(
            bin,
            u32::from(in_range(&tx, &tf, &tt)),
            "{tx:?} in {tf:?}..{tt:?}"
        );
        cases += 1;
    }
    // Hour and minute only, as the daytime test sets them: every minute of the day against a range
    // that does not wrap and one that does, the bounds and one either side exactly.
    for (f, t) in [
        ((5, 30), (21, 0)),
        ((21, 0), (5, 30)),
        ((12, 0), (12, 0)),
        ((0, 0), (23, 59)),
    ] {
        put(&mut e, from, &[f.1, f.0, -1, -1, -1, -1, 0]);
        put(&mut e, to, &[t.1, t.0, -1, -1, -1, -1, 0]);
        for hour in 0..24 {
            for minute in 0..60 {
                let tx = [minute, hour, 3, 7, 2, 4, 0];
                put(&mut e, this, &tx);
                let bin = e.call(IN_RANGE, this as u32, 0, &[from as u32, to as u32]);
                let tf = [f.1, f.0, -1, -1, -1, -1, 0];
                let tt = [t.1, t.0, -1, -1, -1, -1, 0];
                assert_eq!(
                    bin,
                    u32::from(in_range(&tx, &tf, &tt)),
                    "{hour}:{minute} in {f:?}..{t:?}"
                );
                cases += 1;
            }
        }
    }
    eprintln!("time_in_range: {cases} cases bit-exact vs WoW.exe @ {IN_RANGE:#x}");
}

#[test]
fn time_encode_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out = e.alloc(0x10);
    let mut g = Lcg(0x642a_e000);
    let mut cases = 0u32;
    let mut run = |e: &mut Emu, a: [i32; 7]| {
        e.write(out, &[0xa5; 8]);
        // minute in edx, hour then weekday, day, month, year and the two-bit field on the stack.
        e.call(
            ENCODE,
            out as u32,
            a[0] as u32,
            &[
                a[1] as u32,
                a[2] as u32,
                a[3] as u32,
                a[4] as u32,
                a[5] as u32,
                a[6] as u32,
            ],
        );
        assert_eq!(
            e.read_u32(out),
            encode_time(a[0], a[1], a[2], a[3], a[4], a[5], a[6]),
            "{a:?}"
        );
        assert_eq!(e.read(out + 4, 4), vec![0xa5; 4], "stores one dword");
        cases += 1;
    };
    run(&mut e, [0; 7]);
    run(&mut e, [-1; 7]);
    run(&mut e, [30, 5, -1, -1, -1, -1, 0]);
    run(&mut e, [0, 21, -1, -1, -1, -1, 0]);
    // Each field alone at its mask, one past it and all ones.
    let widths = [6, 5, 3, 6, 4, 5, 2];
    for (i, &w) in widths.iter().enumerate() {
        for v in [
            1i32,
            (1 << w) - 1,
            1 << w,
            (1 << w) + 1,
            -1,
            i32::MIN,
            i32::MAX,
        ] {
            let mut a = [0i32; 7];
            a[i] = v;
            run(&mut e, a);
            let mut a = [-1i32; 7];
            a[i] = v;
            run(&mut e, a);
        }
    }
    for _ in 0..5000 {
        let mut a = [0i32; 7];
        for v in a.iter_mut() {
            *v = g.next() as i32;
        }
        run(&mut e, a);
    }
    eprintln!("time_encode: {cases} cases bit-exact vs WoW.exe @ {ENCODE:#x}");
}

#[test]
fn is_daytime_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut g = Lcg(0x60a0_5000);
    let mut cases = 0u32;
    let mut run = |e: &mut Emu, clock: Time| {
        put(e, CLOCK, &clock);
        let bin = e.call(IS_DAYTIME, 0, 0, &[]);
        assert_eq!(bin & 0xff, u32::from(is_daytime(&clock)), "{clock:?}");
        cases += 1;
    };
    // Every minute of the day with the date set, the date unset and the date random.
    for hour in 0..24 {
        for minute in 0..60 {
            run(&mut e, [minute, hour, 3, 7, 2, 4, 0]);
            run(&mut e, [minute, hour, -1, -1, -1, -1, 0]);
            let date = [
                g.next() as i32,
                g.next() as i32,
                g.next() as i32,
                g.next() as i32,
            ];
            run(
                &mut e,
                [minute, hour, date[0], date[1], date[2], date[3], 1],
            );
        }
    }
    // The bounds and the unset clock the constructor leaves.
    for (minute, hour, expect) in [
        (29, 5, false),
        (30, 5, true),
        (59, 20, true),
        (0, 21, false),
        (59, 4, false),
        (-1, -1, false),
    ] {
        let clock = [minute, hour, -1, -1, -1, -1, -1];
        run(&mut e, clock);
        assert_eq!(is_daytime(&clock), expect, "{hour}:{minute}");
    }
    for _ in 0..3000 {
        run(&mut e, random_time(&mut g));
    }
    eprintln!("is_daytime: {cases} cases bit-exact vs WoW.exe @ {IS_DAYTIME:#x}");
}
