//! Calendar time (`systems/crt/time.md`): `gmtime` and `localtime` break a 32-bit `time_t` down,
//! `mktime` builds one from fields, and `strftime` formats the fields with the C locale's names
//! and patterns.

/// `struct tm`, nine `int`s in the runtime's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tm {
    pub sec: i32,
    pub min: i32,
    pub hour: i32,
    pub mday: i32,
    pub mon: i32,
    pub year: i32,
    pub wday: i32,
    pub yday: i32,
    pub isdst: i32,
}

impl Tm {
    pub fn to_words(self) -> [i32; 9] {
        [
            self.sec, self.min, self.hour, self.mday, self.mon, self.year, self.wday, self.yday,
            self.isdst,
        ]
    }

    pub fn from_words(w: [i32; 9]) -> Tm {
        Tm {
            sec: w[0],
            min: w[1],
            hour: w[2],
            mday: w[3],
            mon: w[4],
            year: w[5],
            wday: w[6],
            yday: w[7],
            isdst: w[8],
        }
    }
}

/// `_lpdays` (`0x832800`) and `_days` (`0x832834`): the days before each month less one, for a
/// leap year and another, with the year's last day less one at the end.
pub const LPDAYS: [i32; 13] = [-1, 30, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365];
pub const DAYS: [i32; 13] = [-1, 30, 58, 89, 119, 150, 180, 211, 242, 272, 303, 333, 364];

const DAY: i32 = 86_400;
const YEAR: i32 = 365 * DAY;
const LEAP_YEAR: i32 = 366 * DAY;
const FOUR_YEARS: i32 = 1461 * DAY;

/// `gmtime` (`0x7421da`): `None` for a negative time. The year from four-year blocks of 1461 days
/// from 1970, whose third year is the leap one; the month from the tables; the weekday from day 0,
/// a Thursday; `isdst` 0.
pub fn gmtime(t: i32) -> Option<Tm> {
    if t < 0 {
        return None;
    }
    let quads = t / FOUR_YEARS;
    let mut rest = t - quads * FOUR_YEARS;
    let mut year = quads * 4 + 70;
    let mut leap = false;
    if rest >= YEAR {
        rest -= YEAR;
        year += 1;
        if rest >= YEAR {
            rest -= YEAR;
            year += 1;
            if rest >= LEAP_YEAR {
                year += 1;
                rest -= LEAP_YEAR;
            } else {
                leap = true;
            }
        }
    }
    let yday = rest / DAY;
    rest -= yday * DAY;
    let table = if leap { &LPDAYS } else { &DAYS };
    let mut mon = 1;
    while table[mon] < yday {
        mon += 1;
    }
    mon -= 1;
    let hour = rest / 3600;
    rest -= hour * 3600;
    let min = rest / 60;
    Some(Tm {
        sec: rest - min * 60,
        min,
        hour,
        mday: yday - table[mon],
        mon: mon as i32,
        year,
        wday: (t / DAY + 4) % 7,
        yday,
        isdst: 0,
    })
}

/// The zone `___tzset` leaves: `_timezone` (`0x83274c`), seconds west of UTC; `_daylight`
/// (`0x832750`); `_dstbias` (`0x832754`), what daylight time adds to it.
#[derive(Clone, Copy, Debug)]
pub struct Zone {
    pub timezone: i32,
    pub daylight: bool,
    pub dstbias: i32,
}

/// `localtime` (`0x73f125`), with `isindst` standing for `__isindst` (`0x41482c`) where the zone
/// has daylight time. `None` for a negative time. Between 3 days and 3 days short of the end of
/// the 32-bit range, `gmtime` of the time less `_timezone`, and with daylight time in force
/// `gmtime` of that less `_dstbias` with `isdst` 1. Outside it, `gmtime` of the time, the offset
/// carried through the seconds, minutes, hours and days by hand, `isdst` left 0.
pub fn localtime(t: i32, z: &Zone, isindst: &dyn Fn(&Tm) -> bool) -> Option<Tm> {
    if t < 0 {
        return None;
    }
    if t > 3 * DAY && t < i32::MAX - 3 * DAY {
        let lt = t.wrapping_sub(z.timezone);
        let tm = gmtime(lt)?;
        if z.daylight && isindst(&tm) {
            let mut tm = gmtime(lt.wrapping_sub(z.dstbias))?;
            tm.isdst = 1;
            return Some(tm);
        }
        return Some(tm);
    }
    let mut tm = gmtime(t)?;
    let mut lt = tm.sec;
    if z.daylight && isindst(&tm) {
        lt = lt.wrapping_sub(z.dstbias);
    }
    lt = lt.wrapping_sub(z.timezone);
    tm.sec = lt % 60;
    if tm.sec < 0 {
        tm.sec += 60;
        lt -= 60;
    }
    lt = lt / 60 + tm.min;
    tm.min = lt % 60;
    if tm.min < 0 {
        tm.min += 60;
        lt -= 60;
    }
    lt = lt / 60 + tm.hour;
    tm.hour = lt % 24;
    if tm.hour < 0 {
        tm.hour += 24;
        lt -= 24;
    }
    let days = lt / 24;
    if days > 0 {
        tm.wday = (tm.wday + days) % 7;
        tm.mday += days;
        tm.yday += days;
    } else if days < 0 {
        tm.wday = (tm.wday + days + 7) % 7;
        tm.mday += days;
        if tm.mday <= 0 {
            tm.mday += 31;
            tm.year -= 1;
            tm.yday = 364;
            tm.mon = 11;
        } else {
            tm.yday += days;
        }
    }
    Some(tm)
}

/// `__make_time_t`'s answer for a local time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MakeTime {
    /// The time, the fields rewritten with the broken-down local time.
    Time(i32),
    /// -1, with `tm_mon` folded into 0 to 11 when the year was in range.
    Refused,
    /// The second `localtime` refused the time and the copy reads address 0.
    Fault,
}

fn add(a: i32, b: i32) -> Option<i32> {
    a.checked_add(b)
}

fn mul(a: i32, k: i32) -> Option<i32> {
    a.checked_mul(k)
}

/// `__make_time_t` (`0x740687`) with its local flag set, as `_mktime` (`0x74066b`) calls it.
/// `tm_year` from 69 to 139, the month folded into the year, then the days, hours, minutes and
/// seconds, each step refused on a 32-bit overflow; `_timezone` added without a test, then
/// `localtime`; `_dstbias` added and `localtime` again when `tm_isdst` is positive, or negative
/// with the first answer in daylight time; the last answer copied over the fields.
pub fn mktime(tm: &mut Tm, z: &Zone, isindst: &dyn Fn(&Tm) -> bool) -> MakeTime {
    let mut year = tm.year;
    if !(69..=139).contains(&year) {
        return MakeTime::Refused;
    }
    if !(0..=11).contains(&tm.mon) {
        year += tm.mon / 12;
        tm.mon %= 12;
        if tm.mon < 0 {
            tm.mon += 12;
            year -= 1;
        }
        if !(69..=139).contains(&year) {
            return MakeTime::Refused;
        }
    }
    let mut days = DAYS[tm.mon as usize];
    if year & 3 == 0 && tm.mon > 1 {
        days += 1;
    }
    let base = year * 365 + ((year - 1) >> 2) + days - 25_567;
    let Some(secs) = add(base, tm.mday)
        .and_then(|v| mul(v, 24))
        .and_then(|v| add(v, tm.hour))
        .and_then(|v| mul(v, 60))
        .and_then(|v| add(v, tm.min))
        .and_then(|v| mul(v, 60))
        .and_then(|v| add(v, tm.sec))
    else {
        return MakeTime::Refused;
    };
    let mut t = secs.wrapping_add(z.timezone);
    let Some(first) = localtime(t, z, isindst) else {
        return MakeTime::Refused;
    };
    let mut last = Some(first);
    if tm.isdst > 0 || (tm.isdst < 0 && first.isdst > 0) {
        t = t.wrapping_add(z.dstbias);
        last = localtime(t, z, isindst);
    }
    match last {
        Some(l) => {
            *tm = l;
            MakeTime::Time(t)
        }
        None => MakeTime::Fault,
    }
}

/// The C locale's time record (`0x8323a0`): the day and month names, the two designators and the
/// three patterns `%x`, `%#x` and `%X` expand.
pub struct TimeNames<'a> {
    pub short_days: [&'a [u8]; 7],
    pub days: [&'a [u8]; 7],
    pub short_months: [&'a [u8]; 12],
    pub months: [&'a [u8]; 12],
    pub am: &'a [u8],
    pub pm: &'a [u8],
    pub short_date: &'a [u8],
    pub long_date: &'a [u8],
    pub time: &'a [u8],
}

pub const C_NAMES: TimeNames<'static> = TimeNames {
    short_days: [b"Sun", b"Mon", b"Tue", b"Wed", b"Thu", b"Fri", b"Sat"],
    days: [
        b"Sunday",
        b"Monday",
        b"Tuesday",
        b"Wednesday",
        b"Thursday",
        b"Friday",
        b"Saturday",
    ],
    short_months: [
        b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov",
        b"Dec",
    ],
    months: [
        b"January",
        b"February",
        b"March",
        b"April",
        b"May",
        b"June",
        b"July",
        b"August",
        b"September",
        b"October",
        b"November",
        b"December",
    ],
    am: b"AM",
    pm: b"PM",
    short_date: b"M/d/yy",
    long_date: b"dddd, MMMM dd, yyyy",
    time: b"H:mm:ss",
};

/// A string as the runtime reads it: its bytes up to a NUL, NULs after.
fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

/// `strftime`'s state: the buffer, the next byte and the room left, `__alternate_form`
/// (`0xcf7284`) and `__no_lead_zeros` (`0xcf7288`).
struct Writer<'b, 'n> {
    buf: &'b mut [u8],
    pos: usize,
    left: usize,
    alternate: bool,
    no_lead_zeros: bool,
    tm: Tm,
    names: &'n TimeNames<'n>,
    tzname: [&'n [u8]; 2],
}

impl Writer<'_, '_> {
    fn put(&mut self, b: u8) {
        self.buf[self.pos] = b;
        self.pos += 1;
        self.left -= 1;
    }

    /// `__store_str` (`0x4139e7`): the string's bytes while room is left.
    fn store_str(&mut self, s: &[u8]) {
        if self.left == 0 {
            return;
        }
        let mut i = 0;
        while at(s, i) != 0 {
            self.put(at(s, i));
            i += 1;
            if self.left == 0 {
                break;
            }
        }
    }

    /// `__store_num` (`0x413a0e`): with `__no_lead_zeros` the number's own digits, else `digits`
    /// digits zero-padded when fewer than the room left, else the room set to 0. A digit is
    /// `'0'` plus the signed remainder.
    fn store_num(&mut self, mut n: i32, digits: usize) {
        if self.no_lead_zeros {
            return self.store_number(n);
        }
        if digits >= self.left {
            self.left = 0;
            return;
        }
        for i in (0..digits).rev() {
            self.buf[self.pos + i] = (0x30 + n % 10) as u8;
            n /= 10;
        }
        self.pos += digits;
        self.left -= digits;
    }

    /// `__store_number` (`0x413a7d`): the low digits while more than one byte of room is left, up
    /// to the last nonzero quotient, then the bytes from the start through the end reversed by a
    /// loop that swaps before it tests, so no digit at all swaps the byte before the start with
    /// the byte at it.
    fn store_number(&mut self, mut n: i32) {
        let start = self.pos;
        let mut end = start;
        while self.left > 1 {
            self.buf[end] = (0x30 + n % 10) as u8;
            end += 1;
            n /= 10;
            self.left -= 1;
            if n <= 0 {
                break;
            }
        }
        self.pos = end;
        let (mut a, mut b) = (start as isize, end as isize - 1);
        loop {
            self.buf.swap(a as usize, b as usize);
            a += 1;
            b -= 1;
            if a >= b {
                break;
            }
        }
    }

    /// `__store_winword` (`0x413ac6`): each run of one letter in the pattern is a conversion by
    /// its letter and length, a quoted run is copied, anything else is one literal byte.
    fn store_winword(&mut self, p: &[u8]) {
        let mut i = 0;
        if at(p, 0) == 0 {
            return;
        }
        loop {
            if self.left == 0 {
                return;
            }
            self.no_lead_zeros = false;
            let c = at(p, i);
            let mut run = 1;
            while at(p, i + run) == c {
                run += 1;
            }
            let mut next = i + run;
            let single = |w: &mut Self, spec| {
                w.no_lead_zeros = true;
                Some(spec)
            };
            let spec = match (c, run) {
                (b'M', 1) => single(self, b'm'),
                (b'M', 2) => Some(b'm'),
                (b'M', 3) => Some(b'b'),
                (b'M', 4) => Some(b'B'),
                (b'd', 1) => single(self, b'd'),
                (b'd', 2) => Some(b'd'),
                (b'd', 3) => Some(b'a'),
                (b'd', 4) => Some(b'A'),
                (b'H', 1) => single(self, b'H'),
                (b'H', 2) => Some(b'H'),
                (b'h', 1) => single(self, b'I'),
                (b'h', 2) => Some(b'I'),
                (b'm', 1) => single(self, b'M'),
                (b'm', 2) => Some(b'M'),
                (b's', 1) => single(self, b'S'),
                (b's', 2) => Some(b'S'),
                (b'y', 2) => Some(b'y'),
                (b'y', 4) => Some(b'Y'),
                (b'A' | b'a', _) => {
                    let rest: Vec<u8> = p[i.min(p.len())..]
                        .iter()
                        .take_while(|&&b| b != 0)
                        .map(u8::to_ascii_lowercase)
                        .collect();
                    if rest == b"am/pm" {
                        next = i + 5;
                    } else if rest == b"a/p" {
                        next = i + 3;
                    }
                    Some(b'p')
                }
                (b't', _) => {
                    let d = if self.tm.hour > 11 {
                        self.names.pm
                    } else {
                        self.names.am
                    };
                    let mut k = 0;
                    while k < run && self.left > 0 {
                        self.put(at(d, k));
                        k += 1;
                    }
                    i = next;
                    if at(p, i) == 0 {
                        return;
                    }
                    continue;
                }
                (b'\'', _) => {
                    i += run;
                    if run % 2 == 1 {
                        while at(p, i) != 0 && self.left > 0 {
                            if at(p, i) == b'\'' {
                                i += 1;
                                break;
                            }
                            self.put(at(p, i));
                            i += 1;
                        }
                    }
                    if at(p, i) == 0 {
                        return;
                    }
                    continue;
                }
                _ => None,
            };
            match spec {
                Some(s) => {
                    self.expand(s);
                    i = next;
                }
                None => {
                    self.put(c);
                    i += 1;
                }
            }
            if at(p, i) == 0 {
                return;
            }
        }
    }

    /// The numeric conversions copy `__alternate_form` into `__no_lead_zeros` first.
    fn num(&mut self, n: i32, digits: usize) {
        self.no_lead_zeros = self.alternate;
        self.store_num(n, digits);
    }

    /// `__expandtime` (`0x4135f8`): one conversion; an unknown one writes nothing.
    fn expand(&mut self, spec: u8) {
        let tm = self.tm;
        let names = self.names;
        match spec {
            b'a' => self.store_str(names.short_days[tm.wday as usize]),
            b'A' => self.store_str(names.days[tm.wday as usize]),
            b'b' => self.store_str(names.short_months[tm.mon as usize]),
            b'B' => self.store_str(names.months[tm.mon as usize]),
            b'c' => {
                let date = if self.alternate {
                    self.alternate = false;
                    names.long_date
                } else {
                    names.short_date
                };
                self.store_winword(date);
                if self.left == 0 {
                    return;
                }
                self.put(b' ');
                self.store_winword(names.time);
            }
            b'd' => self.num(tm.mday, 2),
            b'H' => self.num(tm.hour, 2),
            b'I' => {
                let h = tm.hour % 12;
                self.num(if h == 0 { 12 } else { h }, 2);
            }
            b'j' => self.num(tm.yday + 1, 3),
            b'm' => self.num(tm.mon + 1, 2),
            b'M' => self.num(tm.min, 2),
            b'p' => self.store_str(if tm.hour > 11 { names.pm } else { names.am }),
            b'S' => self.num(tm.sec, 2),
            b'U' | b'W' => {
                let first = match (spec, tm.wday) {
                    (b'U', d) => d,
                    (_, 0) => 6,
                    (_, d) => d - 1,
                };
                let week = if tm.yday < first {
                    0
                } else {
                    tm.yday / 7 + i32::from(tm.yday % 7 >= first)
                };
                self.num(week, 2);
            }
            b'w' => self.num(tm.wday, 1),
            b'x' => {
                let date = if self.alternate {
                    self.alternate = false;
                    names.long_date
                } else {
                    names.short_date
                };
                self.store_winword(date);
            }
            b'X' => {
                self.alternate = false;
                self.store_winword(names.time);
            }
            b'y' => self.num(tm.year % 100, 2),
            b'Y' => self.num((tm.year / 100 + 19) * 100 + tm.year % 100, 4),
            b'Z' | b'z' => self.store_str(self.tzname[usize::from(tm.isdst != 0)]),
            b'%' => self.put(b'%'),
            _ => {}
        }
    }
}

/// `_Strftime` (`0x4134fe`) as `strftime` (`0x4134e3`) calls it, into `buf` (its bytes before the
/// call matter only to the fault of `__store_number`) with room `max`, over `fmt`, the bytes from
/// the format on: a `%` that ends the format reads on past its NUL. The `%Z` names are
/// `_tzname`'s (`0x8327d8`). The count written, or 0 when no room was left for the NUL.
pub fn strftime(
    buf: &mut [u8],
    max: usize,
    fmt: &[u8],
    tm: &Tm,
    names: &TimeNames,
    tzname: [&[u8]; 2],
) -> usize {
    let mut w = Writer {
        buf,
        pos: 0,
        left: max,
        alternate: false,
        no_lead_zeros: false,
        tm: *tm,
        names,
        tzname,
    };
    let mut i = 0;
    while w.left > 0 && at(fmt, i) != 0 {
        if at(fmt, i) != b'%' {
            w.put(at(fmt, i));
            i += 1;
        } else {
            w.alternate = false;
            i += 1;
            if at(fmt, i) == b'#' {
                w.alternate = true;
                i += 1;
            }
            w.expand(at(fmt, i));
            i += 1;
        }
    }
    if w.left > 0 {
        w.buf[w.pos] = 0;
        max - w.left
    } else {
        0
    }
}
