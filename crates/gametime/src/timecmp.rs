//! The comparisons of two time records and the daytime test built on them: the field compares
//! `0x6425f0` to `0x6426e0`, the orderings `0x642770`, `0x6428c0`, `0x642a10`, `0x642880` and
//! `0x6429d0`, the range test `0x642710`, the packing `0x642ae0`, and `IsDaytime` (`0x60a050`,
//! `systems/gametime/overview.md#time-range-test`).

use crate::Time;

/// The fields an ordering walks, as indices into a [`Time`]: year, month, day, weekday, hour and
/// minute, in the order the code reads them (`+0x14` down to `+0`). The two-bit field is not read.
const ORDER: [usize; 6] = [5, 4, 3, 2, 1, 0];

/// `0x6425f0` to `0x6426e0` on one field: 1 when `this` is above `other`, 0 when equal, −1 when
/// below, signed.
#[must_use]
pub fn compare_field(this: i32, other: i32) -> i32 {
    match this.cmp(&other) {
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Less => -1,
    }
}

/// The first field, in [`ORDER`], that both records set (non-negative) and that differs, compared
/// as `this` against `other`.
fn first_difference(this: &Time, other: &Time) -> Option<i32> {
    ORDER.iter().find_map(|&f| {
        if this[f] < 0 || other[f] < 0 {
            return None;
        }
        match compare_field(this[f], other[f]) {
            0 => None,
            c => Some(c),
        }
    })
}

/// `0x642770`: `this` is before `other` at the first field both set and unequal. The same record,
/// by address, and records with no such field answer false.
#[must_use]
pub fn is_less(this: &Time, other: &Time) -> bool {
    !std::ptr::eq(this, other) && first_difference(this, other) == Some(-1)
}

/// `0x6428c0`: `this` is after `other` at the first field both set and unequal; false for the same
/// record, by address, and for records with no such field.
#[must_use]
pub fn is_greater(this: &Time, other: &Time) -> bool {
    !std::ptr::eq(this, other) && first_difference(this, other) == Some(1)
}

/// `0x642a10`: every field both records set is equal; a field either leaves unset (negative)
/// matches anything.
#[must_use]
pub fn is_equal(this: &Time, other: &Time) -> bool {
    ORDER
        .iter()
        .all(|&f| this[f] < 0 || other[f] < 0 || this[f] == other[f])
}

/// `0x642880`: equal (`0x642a10`) or less (`0x642770`).
#[must_use]
pub fn is_less_or_equal(this: &Time, other: &Time) -> bool {
    is_equal(this, other) || is_less(this, other)
}

/// `0x6429d0`: equal (`0x642a10`) or greater (`0x6428c0`).
#[must_use]
pub fn is_greater_or_equal(this: &Time, other: &Time) -> bool {
    is_equal(this, other) || is_greater(this, other)
}

/// `0x642710`: `this` lies from `from` up to, not including, `to`. A `from` at or before `to`
/// (`0x642880`) gives `this >= from` and `this < to`; a `from` after `to` wraps and gives
/// `this >= from` or `this < to`.
#[must_use]
pub fn in_range(this: &Time, from: &Time, to: &Time) -> bool {
    if is_less_or_equal(from, to) {
        is_greater_or_equal(this, from) && is_less(this, to)
    } else {
        is_greater_or_equal(this, from) || is_less(this, to)
    }
}

/// `0x642ae0`: the packed date of its arguments, each masked to its field (minute 6 bits, hour 5,
/// weekday 3, day 6, month 4, year 5, two-bit field 2) and placed from bit 0 up in that order.
#[must_use]
pub fn encode_time(
    minute: i32,
    hour: i32,
    weekday: i32,
    day: i32,
    month: i32,
    year: i32,
    extra: i32,
) -> u32 {
    let mut packed = (extra as u32) & 0x3;
    packed = (packed << 5) | ((year as u32) & 0x1f);
    packed = (packed << 4) | ((month as u32) & 0xf);
    packed = (packed << 6) | ((day as u32) & 0x3f);
    packed = (packed << 3) | ((weekday as u32) & 0x7);
    packed = (packed << 5) | ((hour as u32) & 0x1f);
    (packed << 6) | ((minute as u32) & 0x3f)
}

/// `IsDaytime` (`0x60a050`) on the game clock: its hour and minute lie from 5:30 up to, not
/// including, 21:00. The two bounds are packed with the weekday, day, month and year unset and
/// decoded again, so only the hour and the minute are compared.
#[must_use]
pub fn is_daytime(clock: &Time) -> bool {
    let from = crate::decode_time(encode_time(30, 5, -1, -1, -1, -1, 0));
    let to = crate::decode_time(encode_time(0, 21, -1, -1, -1, -1, 0));
    in_range(clock, &from, &to)
}
