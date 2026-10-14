//! CharSections selection: the validity predicate, the face and hair region-select guard, the
//! geoset-variation count, the region-record lookup and the geoset base selection.
//!
//! The CharSections table is addressed by `idx = sex + 2*race` (sex at `[this+0x1c]`, race at
//! `[this+0x18]`) from the array base `[0xb4231c]`. `0x477c20` indexes a flat 8-byte cell array
//! at `base + 8*(5*idx + raceBlock)`; the region-select handlers index `base + 8*(5*idx) + 4`. The
//! geoset and region table scans (`0x478540`, `0x478740`) walk the global tables `[0xc0df78]` and
//! `[0xc0df28]`. Counts compare signed (`jl`, `jge`, `jle`, `jg`), so every guard is `i32`.
#![allow(clippy::float_cmp)]

/// `0x477c20`: CharSections validity predicate (`ret 0xc`; `ecx` is this, three stack args
/// `raceBlock`, `a1`, `a2`):
/// - `raceBlock` (arg0): `test;jl` then `cmp 5;jge` → must be `0 <= raceBlock < 5` (signed).
/// - `a1` (arg1): `test;jl` → `a1 >= 0`.
/// - `idx = sex + 2*race`; `cell = [0xb4231c] + 8*(5*idx + raceBlock)`.
/// - `count0 = [cell]`; gate `a1 < count0` (signed `jge`) AND `count0 != 0`.
/// - `a2` (arg2): `test;jl` → `a2 >= 0`.
/// - `ptr = [cell+4]`; `count1 = [ptr + 12*a1]` (`lea edx,[edx+2*edx]` = 3*a1, scaled ×4);
///   gate `a2 < count1` (signed `jge`) AND `count1 != 0`.
/// - all gates pass → `al = 1`, else `al = 0`.
///
/// The transcription takes the fields the exe reads (`count0`, `count1`), so the index and the guards
/// are tested; the test places the cell at `base + 8*(5*idx+raceBlock)` to exercise the cell index.
pub fn charsec_predicate(race_block: u32, a1: u32, a2: u32, count0: u32, count1: u32) -> bool {
    if (race_block as i32) < 0 || (race_block as i32) >= 5 {
        return false;
    }
    if (a1 as i32) < 0 {
        return false;
    }
    // count0 = [cell]; a1 < count0 (signed) and count0 != 0 are needed
    if (a1 as i32) >= (count0 as i32) || count0 == 0 {
        return false;
    }
    if (a2 as i32) < 0 {
        return false;
    }
    // count1 = [[cell+4] + 12*a1]; a2 < count1 (signed) and count1 != 0 are needed
    if (a2 as i32) >= (count1 as i32) || count1 == 0 {
        return false;
    }
    true
}

/// The shared CharSections region-select index and guard (`0x477210` face, `0x477280` hair):
/// `idx = sex + 2*race`; `edx = 5*idx`; the DBC cell read is `[ [0xb4231c] + 8*(5*idx) + 4 ]`, a pointer
/// `rec` (since `8*(5*idx)+4 = 0x28*idx + 4`, this is field `+4` of the CharSections row); the guard is
/// `[this+0x24] < [rec+0x4]` (signed `jl`); when true the default `0x477070` dispatch is skipped.
///
/// `rec` is the value at the DBC cell; `field4` is `[rec+0x4]`; `this_24` is `[this+0x24]`. Returns
/// `([this+0x24] as i32) < ([rec+0x4] as i32)`, the `jl` outcome. Both handlers (`0x477210`,
/// `0x477280`) share this arithmetic; only the entry-list offsets after it differ.
pub fn region_select_guard(this_24: u32, field4: u32) -> bool {
    (this_24 as i32) < (field4 as i32)
}

/// `0x478540`: geoset-variation count lookup. Scans the global table `[0xc0df78]`
/// (count `[0xc0df7c]`, stride 0x18) with index `esi` from 0 while `0 <= esi < count`. The compared
/// record dwords are at `edx-0x4`, `edx`, `edx+0x4` where `edx = table + 0x18*esi + 0x8` — i.e. the
/// record fields at table offsets `+0x4`, `+0x8`, `+0xc`. A record matches when:
/// `[rec+0x4]==[ecx]` && `[rec+0x8]==[ecx+0x4]` && `[rec+0xc]==[ecx+0x1c]`. On match, `r = [rec+0x10]`
/// (`[edx+0x8]`); returns `r` if `r > 0` (signed `jle` → keep default), else 1. No match (or empty
/// table) → 1.
///
/// Args: the key `(k0,k4,k1c) = ([ecx],[ecx+4],[ecx+0x1c])`, the table as `(f4,f8,fc,f10)` records.
pub fn geoset_variation_count(key: (u32, u32, u32), records: &[(u32, u32, u32, u32)]) -> u32 {
    for &(f4, f8, fc, f10) in records {
        if f4 == key.0 && f8 == key.1 && fc == key.2 {
            // match: [rec+0x10] if > 0 (signed), else the default 1
            return if (f10 as i32) > 0 { f10 } else { 1 };
        }
    }
    1
}

/// `0x478740`: region-record lookup. Scans the global table `[0xc0df28]` (count
/// `[0xc0df2c]`, stride 0x28) with index `edx` from 0 while `0 <= edx < count` AND table ptr != 0.
/// A record at `rec = table + 0x28*edx` matches when: `[rec]==[ecx]` && `[rec+0x4]==[ecx+0x4]` &&
/// `[rec+0x8]==[ecx+0x18]`. Returns the matched record POINTER (`eax`), else 0.
///
/// Args: key `(k0,k4,k18) = ([ecx],[ecx+4],[ecx+0x18])`, the table records as `(f0,f4,f8)`. Returns
/// `Some(matched_index)` (the test maps it to `table + 0x28*idx`) or `None` (→ 0).
pub fn region_record_lookup(key: (u32, u32, u32), records: &[(u32, u32, u32)]) -> Option<usize> {
    records
        .iter()
        .position(|&(f0, f4, f8)| f0 == key.0 && f4 == key.1 && f8 == key.2)
}

/// `0x478660`: geoset and region base selection (`ret 0x8`; `ecx` is this, held in `esi`; two stack
/// args `arg0`, `flag`). The part before the calls to `0x478220` and `0x4782e0`:
/// - `arg0` (`[ebp+8]`): `test;jl 0x47872f` (early-out) — if `arg0 < 0` (signed), nothing stores.
/// - `count = 0x47a4c0(this, edx=sex=[esi+0x1c], ecx=race=[esi+0x18])` (`= [0xb4244c][sex + 2*race]`);
///   guard `cmp arg0,count; jg 0x47872f` → if `arg0 > count` (signed) early-out, nothing stores.
/// - else `[esi+0x30] = arg0` is stored, then `rec = 0x478740(&[esi+0x18] key)`; if `rec == 0`, no
///   region-offset stores. Else: `[esi+0x148] = [rec+0x18] + 0x64`; `[esi+0x150] = [rec+0x1c] + 0x12c`;
///   `[esi+0x14c] = [rec+0x20] + 0xc8` (note the +0x150 then +0x14c dest order); `[esi+0xa] = 1`.
///
/// This transcribes the bounds-clamp outcome (whether `[esi+0x30]` is stored and the three region
/// stores are reached) and the three region-offset adds with their destinations. `rec_18/1c/20` are
/// the looked-up region record fields. The test stubs the later calls to `0x478220` and `0x4782e0`
/// and lets `0x47a4c0` and `0x478740` run, then reads the stores back; they all precede those
/// calls, so the run reaches `ret` with the stores intact.
pub struct GeosetBase {
    /// `[esi+0x30] = arg0` was stored (the bounds gate passed).
    pub stored_arg: bool,
    /// The three region-offset adds, present only when the bounds gate passed and `rec != 0`.
    pub dest_148: Option<u32>,
    pub dest_150: Option<u32>,
    pub dest_14c: Option<u32>,
}

/// `count` is `[0xb4244c][sex+2*race]` (from `0x47a4c0`); `rec` is the region record from `0x478740`
/// (`None` if not found), as `(rec_18, rec_1c, rec_20)`.
pub fn geoset_base_select(arg0: u32, count: u32, rec: Option<(u32, u32, u32)>) -> GeosetBase {
    // arg0 < 0 or arg0 > count: early out, no stores
    if (arg0 as i32) < 0 || (arg0 as i32) > (count as i32) {
        return GeosetBase {
            stored_arg: false,
            dest_148: None,
            dest_150: None,
            dest_14c: None,
        };
    }
    // [esi+0x30] = arg0 is stored unconditionally past the gate
    match rec {
        Some((r18, r1c, r20)) => GeosetBase {
            stored_arg: true,
            dest_148: Some(r18.wrapping_add(0x64)),
            dest_150: Some(r1c.wrapping_add(0x12c)),
            dest_14c: Some(r20.wrapping_add(0xc8)),
        },
        None => GeosetBase {
            stored_arg: true,
            dest_148: None,
            dest_150: None,
            dest_14c: None,
        },
    }
}
