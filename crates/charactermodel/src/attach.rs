//! The attach-point and geoset-enable selection: slot and case to id, with no floating point.
//!
//! The calls out to the model and scene code (`0x7110d0`, `0x7121a0`, `0x47a380`, `0x4796b0`) are
//! not modelled. Each test reads the selected id from a register at a slice end, or from the
//! recorded dispatch.
//!
//! The `neg al; sbb eax,eax; add eax,K` idiom is `K - (al != 0)`; the jump-table functions
//! below build on it.

/// `0x47a070`: the equipment attach-slot jump table (`__fastcall`, ecx = case, dl = flag), the
/// selection every resolver below calls. Jump table at `0x47a0a8`: case 0 gives -1, case 1
/// `0x1b-(flag!=0)`, case 2 `0x1f-(flag!=0)`, case 3 `0x21-(flag!=0)`, case 4 `0x1c`, and case > 4
/// gives -1. (`cmp ecx,4; ja` is unsigned, so a negative case is also `> 4` and gives -1.)
pub fn attach_slot(case: u32, flag: bool) -> i32 {
    let f = i32::from(flag); // (flag != 0) ? 1 : 0
    match case {
        0 => -1,
        1 => 0x1b - f,
        2 => 0x1f - f,
        3 => 0x21 - f,
        4 => 0x1c,
        _ => -1,
    }
}

/// `0x47a380`: single attach-point install or clear (`__fastcall`, ecx = model, edx = id; `ret`). The
/// id-to-call mapping that runs before the child attach: `edi = id; if id == -1 nothing; eax = id; if
/// id == 0 call with edx = 1; dec; if id == 1 call with edx = 0; dec; if id != 2 skip; else (id == 2)
/// call with edx = 1`. So the call to `0x4796b0` fires for id in {0, 1, 2} with the edx
/// argument `{0: 1, 1: 0, 2: 1}`, and is skipped for every other id, including -1. Returns
/// `Some(edx_arg)` if `0x4796b0` is invoked, else `None`.
pub fn attach_install_select(id: i32) -> Option<u32> {
    match id {
        0 => Some(1),
        1 => Some(0),
        2 => Some(1),
        _ => None, // -1 (early skip) and everything above 2 reach the join with no 0x4796b0 call
    }
}

/// `0x479f40`: weapon attach by inventory slot (ecx = this, edx = item, arg = `[ebp+8]`; `ret 4`).
/// The mapping from slot to attach-name-table base, and its gate: after a three-pointer non-null
/// gate on the item struct, `if slot != 0x18 && slot != 0x19` it returns 0; else `esi = (slot ==
/// 0x19) ? 0x838d50 : 0x838d70`. Returns `Some(base)` for the two valid slots, `None` (the exe's
/// `xor eax,eax` return of 0) otherwise.
pub fn weapon_attach_table(slot: u32) -> Option<u32> {
    match slot {
        0x18 => Some(0x838d70),
        0x19 => Some(0x838d50),
        _ => None,
    }
}

/// The (esi, edi) pair of attach ids a per-slot resolver feeds to the two `0x47a380` calls. `esi` is
/// the first call's edx and `edi` the second's (an `attach_slot` result, or -1 when unset).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachPair {
    pub esi: i32,
    pub edi: i32,
}

/// `0x47a0c0`: per-race and slot attach resolver (ecx = this, edx = display, eq-slot = `[ebp+8]`,
/// case = `[ebp+0xc]`, flag10 = `[ebp+0x10]`, flag14 = `[ebp+0x14]`, flag18 = `[ebp+0x18]`). The
/// slot-to-(esi, edi) attach-id computation fed to the two `0x47a380` installs (`eax = slot-0xf`):
/// - slot 0xf: esi = 1, edi = `attach_slot(case, true)`
/// - slot 0x10: esi = 2 (0 if flag14), edi = `attach_slot(case, false)`
/// - slot 0x11: if flag18, esi = 1 and edi = `attach_slot(case, true)`; else esi = 2 and
///   edi = `attach_slot(case, false)`
/// - slot 0x12, and any slot outside 0xf..0x12: the function returns -1 early (no installs)
///
/// Returns the pair when the slot resolves, `None` when the function takes its `-1` early-out (slot
/// 0x12 or out of range, or this or display null). The flag14 and name-table (`ebx`) writes are
/// texture-path state, not part of the (esi, edi) attach selection.
pub fn race_attach_pair(
    slot: i32,
    case: u32,
    flag10: bool,
    flag14: bool,
    flag18: bool,
) -> Option<AttachPair> {
    let _ = flag10; // flag10 only swaps which of (esi, edi) the child attach uses downstream; it is not in the pair
    match slot {
        0xf => Some(AttachPair {
            esi: 1,
            edi: attach_slot(case, true),
        }),
        0x10 => Some(AttachPair {
            esi: if flag14 { 0 } else { 2 },
            edi: attach_slot(case, false),
        }),
        0x11 => {
            if flag18 {
                Some(AttachPair {
                    esi: 1,
                    edi: attach_slot(case, true),
                })
            } else {
                Some(AttachPair {
                    esi: 2,
                    edi: attach_slot(case, false),
                })
            }
        }
        _ => None,
    }
}

/// `0x47a310`: shoulder and special attach helper (ecx = this, edx = eq-slot in [0xe, 0x12], case =
/// `[ebp+8]`, flag = `[ebp+0xc]`; `ret 8`). The slot-to-(esi, edi) attach-id selection fed to the two
/// `0x47a380` installs (esi and edi start at -1):
/// - slot 0xe: both -1 (no `attach_slot`)
/// - slot 0xf: esi = 1, edi = `attach_slot(case, true)`
/// - slots 0x10 and 0x11: esi = 2 (0 if flag), edi = `attach_slot(case, false)`
/// - slot 0x12: both -1
/// - outside [0xe, 0x12], or this null: no installs (early return)
///
/// Returns the pair, or `None` for the early-out (out of range or this null).
pub fn shoulder_attach_pair(slot: i32, case: u32, flag: bool) -> Option<AttachPair> {
    match slot {
        0xe | 0x12 => Some(AttachPair { esi: -1, edi: -1 }),
        0xf => Some(AttachPair {
            esi: 1,
            edi: attach_slot(case, true),
        }),
        0x10 | 0x11 => Some(AttachPair {
            esi: if flag { 0 } else { 2 },
            edi: attach_slot(case, false),
        }),
        _ => None,
    }
}

/// `0x4793f0`: boots and leg layer apply (ecx = this, arg = `[ebp+8]`, ptrC = `[ebp+0xc]`, flag10 =
/// `[ebp+0x10]`). The `0x803c10 + 0x28*arg` table index and the choice of region (3 or 4) from the
/// arg and the occupancy results, which picks the first value the function forwards to its layer
/// apply (`push eax` at `0x479489`). The inputs are the exe-supplied `table_val = [0x803c10 +
/// 0x28*arg]` and the three occupancy-predicate (`0x4774f0`) outcomes the branches read:
/// - arg 6: `occ(this, ptrC, 0)` gives region 3, else `table_val`
/// - arg 3: `occ(this, ptrC, 2)` gives region 4, else `table_val`
/// - arg 5: if `occ(this, ptrC, 2)`, then 4 when `[this+0x4b4] == 0`, else 3 if
///   `occ(this, [this+0x4b4], 2)` and 4 if not; otherwise `table_val`
/// - any other arg: `table_val` (the branches are skipped)
///
/// `occ_c` is `0x4774f0(this, ptrC, 0)`, `occ_c2` is `0x4774f0(this, ptrC, 2)`, `field_4b4` is
/// `[this+0x4b4]` (0 means no second probe), `occ_4b4` is `0x4774f0(this, [this+0x4b4], 2)`.
#[allow(clippy::too_many_arguments)]
pub fn boots_region_decision(
    arg: u32,
    table_val: u32,
    occ_c: bool,
    occ_c2: bool,
    field_4b4: u32,
    occ_4b4: bool,
) -> u32 {
    match arg {
        6 => {
            if occ_c {
                3
            } else {
                table_val
            }
        }
        3 => {
            if occ_c2 {
                4
            } else {
                table_val
            }
        }
        5 => {
            if occ_c2 {
                if field_4b4 == 0 {
                    4
                } else if occ_4b4 {
                    3
                } else {
                    4
                }
            } else {
                table_val
            }
        }
        _ => table_val,
    }
}

/// `0x4774f0`: the occupancy predicate the boots layer and many siblings branch on (`__thiscall`,
/// ecx = arg0, args `[ebp+8]` and `[ebp+0xc]`; `ret 8`): `al = (arg0 != 0) && (*(arg0 + 4*idx +
/// 0x18) != 0)`. `field` is the loaded `*(arg0 + 4*idx + 0x18)` dword; only whether it is zero
/// matters.
pub fn occupancy(arg0_nonnull: bool, field: u32) -> bool {
    arg0_nonnull && field != 0
}
