//! The instance messages' arithmetic (`systems/ui/instances.md`): the duration split of
//! `0x49e9d0` and the lockout warning's pick of `0x49e1c0`, both on signed 32-bit seconds with
//! divisions that truncate toward zero, and the capacity rounding of `0x4a3370`, which grows the
//! saved-instance array.

/// The four fields `0x49e9d0` prints as `%dd %dh %dm %ds`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Duration {
    pub days: i32,
    pub hours: i32,
    pub minutes: i32,
    pub seconds: i32,
}

/// `0x49e9d0`'s split: days `s/86400`, hours `(s - 86400d)/3600`, minutes
/// `(s - 3600(24d + h))/60`, seconds `s % 60`, each a truncating signed division in 32 bits.
pub fn split_duration(s: i32) -> Duration {
    let days = s / 86_400;
    let hours = s.wrapping_sub(days.wrapping_mul(86_400)) / 3_600;
    let minutes = s.wrapping_sub(
        hours
            .wrapping_add(days.wrapping_mul(24))
            .wrapping_mul(3_600),
    ) / 60;
    Duration {
        days,
        hours,
        minutes,
        seconds: s % 60,
    }
}

/// One lockout warning line as `0x49e1c0` builds it before `SStrPrintf`: the template's
/// `GlobalStrings` key, the ordinal `GetText` gets (-1 reads the plain global) and the numbers
/// after the map's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    pub key: &'static str,
    pub ordinal: i32,
    pub numbers: Vec<i32>,
}

/// `0x49e1c0`'s pick by type: 1 hours, 2 and 3 minutes, 4 days, hours and minutes; any other type
/// shows nothing.
pub fn lockout_warning(ty: i32, seconds: i32) -> Option<Warning> {
    match ty {
        1 => {
            let h = seconds / 3_600;
            Some(Warning {
                key: "RAID_INSTANCE_WARNING_HOURS",
                ordinal: h,
                numbers: vec![h],
            })
        }
        2 | 3 => {
            let m = seconds / 60;
            Some(Warning {
                key: if ty == 2 {
                    "RAID_INSTANCE_WARNING_MIN"
                } else {
                    "RAID_INSTANCE_WARNING_MIN_SOON"
                },
                ordinal: m,
                numbers: vec![m],
            })
        }
        4 => {
            let d = split_duration(seconds);
            Some(Warning {
                key: "RAID_INSTANCE_WELCOME",
                ordinal: -1,
                numbers: vec![d.days, d.hours, d.minutes],
            })
        }
        _ => None,
    }
}

/// The saved-instance array header `0x4a3370` reads and writes: `{capacity, count, data,
/// granularity}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrayHeader {
    pub capacity: u32,
    pub count: u32,
    pub granularity: u32,
}

/// What `0x4a3370(extra, round)` does to the header: `None` when the need fits the capacity,
/// else the new header and the byte size it asks of the allocator.
pub fn reserve(h: ArrayHeader, extra: u32, round: bool) -> Option<(ArrayHeader, u32)> {
    let mut need = extra.wrapping_add(h.count);
    if need <= h.capacity {
        return None;
    }
    let mut out = h;
    if round {
        let grain = if h.granularity != 0 {
            h.granularity
        } else if need >= 0x15 {
            out.granularity = 0x15;
            0x15
        } else {
            // The highest set bit of the need, at least 1.
            let mut g = need;
            while g & g.wrapping_sub(1) != 0 {
                g &= g - 1;
            }
            g.max(1)
        };
        let rem = need % grain;
        if rem != 0 {
            need = need.wrapping_add(grain - rem);
        }
    }
    out.capacity = need;
    Some((out, need.wrapping_mul(12)))
}
