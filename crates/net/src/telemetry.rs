//! The telemetry record the logon proof carries (`systems/net/connection.md#telemetry-record`).

/// The logon object's record array: `+0` a flag, `+4` the capacity, `+8` the count, `+0xc` the
/// records of 32 bytes, `+0x10` the growth step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordArray {
    pub flag: u32,
    pub capacity: u32,
    pub step: u32,
    /// The first `count` records.
    pub records: Vec<[u8; 32]>,
}

/// An allocator call of the append, with its byte count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayCall {
    /// `SMemReAlloc` (`0x646320`) of the old block with flag `0x10`, which may fail.
    Realloc(u32),
    /// `SMemAlloc` (`0x6462e0`), after a failed reallocation.
    Alloc(u32),
    /// `SMemFree` of the old block, after the copy.
    Free,
}

/// The capacity a growth to `needed` records sets, and the step it stores: the stored step, or,
/// when it is 0, 8 for 8 records or more (stored) and otherwise the largest power of two not
/// above `needed`; `needed` rounded up to a multiple of the step. The step of 1 for a `needed`
/// of 0 (`0x5ab9fe`) is never reached from the append, which grows only when `needed` is above
/// the capacity.
pub fn grow_capacity(needed: u32, step: u32) -> (u32, u32) {
    let (by, stored) = match step {
        0 if needed >= 8 => (8, 8),
        0 => {
            let mut p = needed;
            while p & p.wrapping_sub(1) != 0 {
                p &= p - 1;
            }
            (p.max(1), 0)
        }
        s => (s, s),
    };
    let rem = needed % by;
    let cap = if rem == 0 {
        needed
    } else {
        needed + (by - rem)
    };
    (cap, stored)
}

/// `AppendTelemetryCdKeyInfo` (`0x5ab9b0`): `None` while the byte `0xc28135` is clear (it
/// returns 0); otherwise grows the array when the count reaches the capacity, appends the
/// record from `0xc27e90`, sets the flag to 1 and returns 1. `realloc_ok` is whether the
/// in-place reallocation succeeds; on failure a new block is allocated and, when there was an
/// old one, the records are copied and the old block freed.
pub fn append_telemetry(
    a: &mut RecordArray,
    record: Option<[u8; 32]>,
    had_block: bool,
    realloc_ok: bool,
) -> (u8, Vec<ArrayCall>) {
    let Some(record) = record else {
        return (0, Vec::new());
    };
    let mut calls = Vec::new();
    let needed = (a.records.len() as u32).wrapping_add(1);
    if needed > a.capacity {
        let (cap, step) = grow_capacity(needed, a.step);
        a.capacity = cap;
        a.step = step;
        calls.push(ArrayCall::Realloc(cap << 5));
        if !realloc_ok {
            calls.push(ArrayCall::Alloc(cap << 5));
            if had_block {
                calls.push(ArrayCall::Free);
            }
        }
    }
    a.records.push(record);
    a.flag = 1;
    (1, calls)
}
