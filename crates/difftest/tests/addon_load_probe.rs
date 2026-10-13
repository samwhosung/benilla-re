//! What the client does when two `## LoadOnDemand: 1` addons each list the other under
//! `## Dependencies:`, settled by running the loader's own bytes.
//!
//! `AddOn_Load` (`0x51f240`) returns early for a record already loaded (`0x51f2d6` reads
//! `[rec+0x18]`, `0x51f2db` jumps out), and stamps that byte at `0x51f313`, in the dependency
//! loop's preamble, before either dependency loop runs. So a cycle ends.
//!
//! The tests build a registry of `UIADDON` records in the hash table the loader walks
//! (`[0xbe1b7c]`, mask `[0xbe1b84]`), then call `AddOn_Load` and `AddOn_CanLoad` (`0x51e780`).
//! Everything the loader does outside the registry is stubbed, each with the callee's own `ret N`:
//!
//! - `0x51e470`, the per-character enabled query (`ret 4`): 1, enabled.
//! - `0x51f180`, the `checkAddonVersion` CVar read: 1. Either value works, since every record
//!   carries `[rec+0x1c] = 11200`, the value `0x51d7d0` returns.
//! - `0x5abdc0` and `0x5ab7d0`, the current character and realm: scratch strings.
//! - `0x484700`, `0x661da0`, `0x661e90`, `0x419e30`: the log buffer and the MD5 context.
//! - `0x64a7f0`, `SStrPrintf`: its output feeds only stubs.
//! - `0x648a30`, file exists (`ret 8`): 0, so no `.toc`, `Bindings.xml` or SavedVariables file is
//!   found and `0x704bc0` is never reached.
//! - `0x6edb90`, the `.toc` file-list runner (`ret 4`), and `0x6edb40`, its error count.
//! - `0x703f50`, `FrameScript_SignalEvent2`: logged, and its log is the order `ADDON_LOADED`
//!   (`0x1ad`) fires in.
//!
//! The loader's one indirect call, `0x51f475 call [edx+8]` on the log context, lands on a scratch
//! vtable whose slot 2 is a `ret 4` island.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use std::panic::{catch_unwind, AssertUnwindSafe};

const ADDON_LOAD: u64 = 0x51f240;
const ADDON_CANLOAD: u64 = 0x51e780;
/// `AddOn_LoadAllNonDemand`, the boot walk `UI_Init` (`0x48fbf0`) calls at `0x4900a3`:
/// `__fastcall(logContext /*ecx*/)`.
const BOOT_LOAD_ALL: u64 = 0x51f600;
const STR_HASH: u64 = 0x64b3f0;

/// The `UIADDON*` hash table and its bucket mask, read by `0x51f240`, `0x51e780` and `0x51e6f0`.
const G_ADDON_TABLE: u64 = 0xbe1b7c;
const G_ADDON_MASK: u64 = 0xbe1b84;

// The stubbed callees, with the `ret N` each one ends in.
const ENABLED_QUERY: u64 = 0x51e470; // ret 4
const VERSION_CVAR: u64 = 0x51f180; // ret
const CUR_CHARACTER: u64 = 0x5abdc0; // ret
const CUR_REALM: u64 = 0x5ab7d0; // ret (cdecl)
const LOGBUF_INIT: u64 = 0x484700; // ret
const MD5_INIT: u64 = 0x661da0; // ret
const MD5_FINAL: u64 = 0x661e90; // ret
const LOGBUF_FINI: u64 = 0x419e30; // ret
const SSTR_PRINTF: u64 = 0x64a7f0; // cdecl
const FILE_EXISTS: u64 = 0x648a30; // ret 8
const RUN_TOC_FILES: u64 = 0x6edb90; // ret 4
const LOAD_ERR_COUNT: u64 = 0x6edb40; // ret
const SIGNAL_EVENT2: u64 = 0x703f50; // cdecl, variadic

/// `UIADDON`: allocation tag `.?AUUIADDON@@` at `0x853afc`, size `0x1d4`, constructor `0x520550`.
const REC_SIZE: u64 = 0x1d4;
const F_HASH: u64 = 0x00;
const F_NAME: u64 = 0x14;
const F_LOADED: u64 = 0x18;
const F_INTERFACE: u64 = 0x1c;
const F_SECURITY: u64 = 0x24;
const F_CORRUPT: u64 = 0x2a;
const F_LOADONDEMAND: u64 = 0x2c;
const F_VISITING: u64 = 0x2d;
const F_OPTDEP_N: u64 = 0x3c;
const F_REQDEP_N: u64 = 0x4c;
const F_REQDEP_V: u64 = 0x50;
const F_LOADWITH_N: u64 = 0x8c;

/// The `## Interface` value the gate compares with: `0x51d7d0` is `mov eax,0x2bc0; ret`.
const IFACE_11200: u32 = 11200;
/// `ADDON_LOADED`, the event id pushed at `0x51f5a8`.
const EV_ADDON_LOADED: u32 = 0x1ad;

struct Registry {
    name_a: u64,
    name_b: u64,
    rec_a: u64,
    rec_b: u64,
    log_ctx: u64,
}

/// Two records, each naming the other in its `## Dependencies` array, in a registry the loader
/// can walk. `lod_a` and `lod_b` set each record's `[rec+0x2c]`, `## LoadOnDemand: 1`.
fn build(emu: &mut Emu, lod_a: bool, lod_b: bool) -> Registry {
    let name_a = emu.alloc(16);
    emu.write(name_a, b"AddonA\0");
    let name_b = emu.alloc(16);
    emu.write(name_b, b"AddonB\0");

    // The registry's own string hash, run in the emulator (`0x64b3f0`, `ret 4`).
    let hash_a = emu.call(STR_HASH, 0, 0, &[name_a as u32]);
    let hash_b = emu.call(STR_HASH, 0, 0, &[name_b as u32]);
    let mask: u32 = 0xff;
    assert_ne!(
        hash_a & mask,
        hash_b & mask,
        "the two names must land in different buckets so no chain walk is needed"
    );

    let rec_a = emu.alloc(REC_SIZE);
    let rec_b = emu.alloc(REC_SIZE);
    let deps_a = emu.alloc(4);
    emu.write_u32(deps_a, name_b as u32);
    let deps_b = emu.alloc(4);
    emu.write_u32(deps_b, name_a as u32);

    for (rec, name, hash, deps, lod) in [
        (rec_a, name_a, hash_a, deps_a, lod_a),
        (rec_b, name_b, hash_b, deps_b, lod_b),
    ] {
        emu.write_u32(rec + F_HASH, hash);
        emu.write_u32(rec + F_NAME, name as u32);
        emu.write(rec + F_LOADED, &[0]);
        emu.write_u32(rec + F_INTERFACE, IFACE_11200);
        // `+0x24` is 0 secure, 1 insecure, 2 banned; 1 is the constructor's default (`0x5205b6`),
        // an ordinary addon: not banned (`0x51e841`) and not signed, so `0x51f553` skips the MD5
        // compare.
        emu.write_u32(rec + F_SECURITY, 1);
        emu.write(rec + F_CORRUPT, &[0]);
        emu.write(rec + F_LOADONDEMAND, &[u8::from(lod)]);
        emu.write(rec + F_VISITING, &[0]);
        emu.write_u32(rec + F_OPTDEP_N, 0);
        emu.write_u32(rec + F_REQDEP_N, 1);
        emu.write_u32(rec + F_REQDEP_V, deps as u32);
        emu.write_u32(rec + F_LOADWITH_N, 0);
    }

    // 0x100 buckets of three dwords; the loader reads only `[bucket+8]`, the first record.
    let table = emu.alloc(0x100 * 12);
    emu.write_u32(G_ADDON_TABLE, table as u32);
    emu.write_u32(G_ADDON_MASK, mask);
    emu.write_u32(table + u64::from(hash_a & mask) * 12 + 8, rec_a as u32);
    emu.write_u32(table + u64::from(hash_b & mask) * 12 + 8, rec_b as u32);

    // The log context `[ebp+8]`, called through at `0x51f46f mov edx,[ecx]; 0x51f475 call [edx+8]`.
    let island = emu.alloc(16);
    emu.write(island, &[0xc2, 0x04, 0x00]); // ret 4
    let vtbl = emu.alloc(16);
    emu.write_u32(vtbl + 8, island as u32);
    let log_ctx = emu.alloc(16);
    emu.write_u32(log_ctx, vtbl as u32);

    let s_char = emu.alloc(16);
    emu.write(s_char, b"Char\0");
    let s_realm = emu.alloc(16);
    emu.write(s_realm, b"Realm\0");

    emu.stub(ENABLED_QUERY, 1, 1);
    emu.stub(VERSION_CVAR, 1, 0);
    emu.stub(CUR_CHARACTER, s_char as u32, 0);
    emu.stub(CUR_REALM, s_realm as u32, 0);
    emu.stub(LOGBUF_INIT, 0, 0);
    emu.stub(MD5_INIT, 0, 0);
    emu.stub(MD5_FINAL, 0, 0);
    emu.stub(LOGBUF_FINI, 0, 0);
    emu.stub(SSTR_PRINTF, 0, 0);
    emu.stub(FILE_EXISTS, 0, 2);
    emu.stub(RUN_TOC_FILES, 0, 1);
    emu.stub(LOAD_ERR_COUNT, 0, 0);
    emu.stub(SIGNAL_EVENT2, 0, 0);

    Registry {
        name_a,
        name_b,
        rec_a,
        rec_b,
        log_ctx,
    }
}

/// `LoadAddOn("A")` on a mutual `## Dependencies` cycle between two LoadOnDemand addons loads both
/// and returns true, with no refusal and no unbounded recursion.
#[test]
fn mutual_dependency_cycle_loads_both() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let r = build(&mut emu, true, true);
    emu.clear_calls();

    // `Script::LoadAddOn` (`0x48e980`) calls the loader as `0x48ea8c mov dl,1; 0x48ea90 call`.
    let ret = emu.call(ADDON_LOAD, r.name_a as u32, 1, &[r.log_ctx as u32]);

    assert_eq!(
        ret & 0xff,
        1,
        "AddOn_Load(A) must return true (0x51f5e5 mov al,1)"
    );
    assert_eq!(emu.read(r.rec_a + F_LOADED, 1)[0], 1, "A is loaded");
    assert_eq!(emu.read(r.rec_b + F_LOADED, 1)[0], 1, "B is loaded too");
    // The visiting byte belongs to the query, which clears what it sets.
    assert_eq!(emu.read(r.rec_a + F_VISITING, 1)[0], 0);
    assert_eq!(emu.read(r.rec_b + F_VISITING, 1)[0], 0);

    // `ADDON_LOADED` fires once per addon, the dependency first: B, then A.
    let fired: Vec<u32> = emu
        .calls()
        .iter()
        .filter(|c| c.addr == SIGNAL_EVENT2 && c.args[0] == EV_ADDON_LOADED)
        .map(|c| c.args[2])
        .collect();
    assert_eq!(
        fired,
        vec![r.name_b as u32, r.name_a as u32],
        "ADDON_LOADED(B) then ADDON_LOADED(A) — the dependency is loaded from inside A's own \
         required-dep loop at 0x51f350, so its event fires first"
    );
    // Each addon's file list runs once.
    let toc_runs = emu
        .calls()
        .iter()
        .filter(|c| c.addr == RUN_TOC_FILES)
        .count();
    assert_eq!(toc_runs, 2, "each `.toc` file list is run once");
}

/// `AddOn_CanLoad` answers true across a cycle: re-entered on a record whose `[rec+0x2d]` is set
/// (`0x51e8ae`), it jumps at `0x51e812` to `0x51e924 mov al,1`, writing neither reason cell.
#[test]
fn canload_resolves_a_cycle_optimistically() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let r = build(&mut emu, true, true);

    let r1 = emu.alloc(4);
    let r2 = emu.alloc(4);
    emu.write_u32(r1, 0xdead_beef);
    emu.write_u32(r2, 0xdead_beef);
    let ch = emu.alloc(16);
    emu.write(ch, b"Char\0");

    let ok = emu.call(
        ADDON_CANLOAD,
        r.name_a as u32,
        1,
        &[r1 as u32, r2 as u32, ch as u32],
    );
    assert_eq!(ok & 0xff, 1, "CanLoad(A) is TRUE across the cycle");
    assert_eq!(
        emu.read_u32(r1),
        0,
        "reason reset to 0 = LOADABLE at 0x51e89d"
    );
    // The recursion at `0x51e8d5` pushes the caller's fourth argument twice (`0x51e8cd`,
    // `0x51e8ce`), so the nested frame's reason cell is the caller's dependency-reason cell, and
    // `0x51e89d mov dword [esi],0` zeroes it from 0xdeadbeef.
    assert_eq!(
        emu.read_u32(r2),
        0,
        "B's frame zeroes the aliased cell at 0x51e89d"
    );

    // With the visiting byte already set, it returns true and writes nothing.
    emu.write(r.rec_a + F_VISITING, &[1]);
    emu.write_u32(r1, 0xdead_beef);
    let ok = emu.call(
        ADDON_CANLOAD,
        r.name_a as u32,
        1,
        &[r1 as u32, r2 as u32, ch as u32],
    );
    assert_eq!(ok & 0xff, 1);
    assert_eq!(
        emu.read_u32(r1),
        0xdead_beef,
        "check 2 writes no reason code"
    );
}

/// A refusal on a cycle, made by the dependency and not the cycle: with B not
/// `## LoadOnDemand`, `LoadAddOn("A")`'s `demandOnly = 1` fails the check inside the nested query
/// (`0x51e902`, `0x51e915`), which writes 6 into A's dependency-reason cell; `0x51e930`'s
/// `reason == 0` arm formats `"DEP_%s"` over entry 6, `"NOT_DEMAND_LOADED"`.
#[test]
fn cycle_with_a_non_demand_dependency_refuses_with_dep_not_demand_loaded() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let r = build(&mut emu, true, false);

    let r1 = emu.alloc(4);
    let r2 = emu.alloc(4);
    let ch = emu.alloc(16);
    emu.write(ch, b"Char\0");
    let ok = emu.call(
        ADDON_CANLOAD,
        r.name_a as u32,
        1,
        &[r1 as u32, r2 as u32, ch as u32],
    );
    assert_eq!(ok & 0xff, 0, "CanLoad(A) is FALSE");
    assert_eq!(
        emu.read_u32(r1),
        0,
        "A's own reason stays 0, the `DEP_` arm"
    );
    assert_eq!(emu.read_u32(r2), 6, "NOT_DEMAND_LOADED, from B's frame");

    emu.clear_calls();
    let ret = emu.call(ADDON_LOAD, r.name_a as u32, 1, &[r.log_ctx as u32]);
    assert_eq!(ret & 0xff, 0, "AddOn_Load(A) refuses at 0x51f301");
    assert_eq!(
        emu.read(r.rec_a + F_LOADED, 1)[0],
        0,
        "the loaded byte is stamped after the gate, so a refusal leaves it clear"
    );
    assert_eq!(emu.read(r.rec_b + F_LOADED, 1)[0], 0);
    assert!(
        emu.calls().iter().all(|c| c.addr != SIGNAL_EVENT2),
        "no ADDON_LOADED fires"
    );
}

/// The control: the early return at `0x51f2db` is what bounds the recursion. With its six bytes
/// replaced by `nop`s, the same input recurses until the emulator's stack runs out.
#[test]
fn cycle_without_the_early_out_recurses_forever() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let r = build(&mut emu, true, true);
    // 0x51f2db: 0f 85 02 03 00 00   jne 0x51f5e3
    assert_eq!(
        emu.read(0x51f2db, 6),
        vec![0x0f, 0x85, 0x02, 0x03, 0x00, 0x00],
        "the byte we are about to defeat is the already-loaded early-out"
    );
    emu.write(0x51f2db, &[0x90; 6]);

    let blew_up = catch_unwind(AssertUnwindSafe(|| {
        emu.call(ADDON_LOAD, r.name_a as u32, 1, &[r.log_ctx as u32]);
    }))
    .is_err();
    assert!(
        blew_up,
        "without 0x51f2db the mutual dependency recurses without bound"
    );
}

/// The boot walk `0x51f600` over a cycle of two addons that are not `## LoadOnDemand`. It differs
/// from `LoadAddOn` by `0x51f629 xor dl,dl`, which turns the LoadOnDemand check off, and by not
/// testing the result; the same early return bounds it, each `.toc` runs once, and its own later
/// visit to the addon the cycle pulled in does nothing.
#[test]
fn boot_walk_over_a_non_demand_cycle_loads_each_once() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let r = build(&mut emu, false, false);

    // The registry's list: head `[0xbe1b6c]`, link offset `[0xbe1b64]`, next at
    // `[node + linkOffset + 4]`, ended by a value with bit 0 set (`0x51f639 test bl,1`).
    const G_LIST_HEAD: u64 = 0xbe1b6c;
    const G_LIST_LINKOFF: u64 = 0xbe1b64;
    const LINK_OFF: u64 = 0x100;
    emu.write_u32(G_LIST_LINKOFF, LINK_OFF as u32);
    emu.write_u32(G_LIST_HEAD, r.rec_a as u32);
    emu.write_u32(r.rec_a + LINK_OFF + 4, r.rec_b as u32);
    emu.write_u32(r.rec_b + LINK_OFF + 4, 1); // bit 0 set = end of list

    emu.clear_calls();
    // `0x51f600` is `__fastcall(logContext /*ecx*/)`, forwarded as `AddOn_Load`'s `[ebp+8]`
    // (`0x51f60b mov esi,ecx`, `0x51f628 push esi`); 0 there faults at `0x51f46f`.
    emu.call(BOOT_LOAD_ALL, r.log_ctx as u32, 0, &[]);

    assert_eq!(emu.read(r.rec_a + F_LOADED, 1)[0], 1);
    assert_eq!(emu.read(r.rec_b + F_LOADED, 1)[0], 1);
    let fired: Vec<u32> = emu
        .calls()
        .iter()
        .filter(|c| c.addr == SIGNAL_EVENT2 && c.args[0] == EV_ADDON_LOADED)
        .map(|c| c.args[2])
        .collect();
    assert_eq!(
        fired,
        vec![r.name_b as u32, r.name_a as u32],
        "the walk starts at A, A's dependency loop loads B first, and the walk's own later visit \
         to B returns early at 0x51f2db"
    );
    let toc_runs = emu
        .calls()
        .iter()
        .filter(|c| c.addr == RUN_TOC_FILES)
        .count();
    assert_eq!(toc_runs, 2, "each `.toc` file list is run exactly once");
}

/// An addon that lists itself under `## Dependencies`: `0x51f313` stamps the loaded byte, the
/// recursion at `0x51f350` returns early at `0x51f2db`, and it loads with one `ADDON_LOADED`.
#[test]
fn self_dependency_loads_normally() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let r = build(&mut emu, true, true);
    // Repoint A's required-dep array at A's own name.
    let self_dep = emu.alloc(4);
    emu.write_u32(self_dep, r.name_a as u32);
    emu.write_u32(r.rec_a + F_REQDEP_V, self_dep as u32);

    emu.clear_calls();
    let ret = emu.call(ADDON_LOAD, r.name_a as u32, 1, &[r.log_ctx as u32]);

    assert_eq!(ret & 0xff, 1);
    assert_eq!(emu.read(r.rec_a + F_LOADED, 1)[0], 1);
    assert_eq!(emu.read(r.rec_b + F_LOADED, 1)[0], 0, "B is not pulled in");
    let fired: Vec<u32> = emu
        .calls()
        .iter()
        .filter(|c| c.addr == SIGNAL_EVENT2 && c.args[0] == EV_ADDON_LOADED)
        .map(|c| c.args[2])
        .collect();
    assert_eq!(fired, vec![r.name_a as u32], "exactly one ADDON_LOADED");
}

// The order of the registry. The scan for addons enumerates the file system and the archives and
// is not run here; what the order rests on is: the list's initialiser, its link, the boot walk and
// the Lua index rebuild, with the CRT's `qsort` and the client's comparator.

/// The registry's list and hash initialiser: `[0xbe1b64] = 0xc` (the link offset),
/// `[0xbe1b68] = 0xbe1b68` (the sentinel, pointing at itself), `[0xbe1b6c] = 0xbe1b68|1` (the head,
/// the terminator) and `[0xbe1b84] = -1`.
const LIST_INIT: u64 = 0x51c360;
/// The intrusive-list link `(this /*ecx*/, node, mode, ref)`, `ret 0xc`. `0x51c9b0` links every
/// new `UIADDON` with mode 2 (`0x51caea push 2`).
const LIST_LINK: u64 = 0x521ad0;
/// The tail of the `SMSG_ADDON_INFO` reader: rebuilds the Lua name array from the list and sorts
/// it. Entered with `eax` = the list head; ends at the `add esp,0x10` after the sort.
const ARRAY_REBUILD_BEGIN: u64 = 0x51dc30;
const ARRAY_REBUILD_END: u64 = 0x51dcd6;

const G_LIST_BASE: u64 = 0xbe1b64; // +0x0 linkOffset, +0x4 sentinel link, +0x8 head
const G_LIST_LINKOFF: u64 = 0xbe1b64;
const G_LIST_SENTINEL: u64 = 0xbe1b68;
const G_LIST_HEAD: u64 = 0xbe1b6c;
const G_ARRAY_CAP: u64 = 0xbe1b8c;
const G_ARRAY_COUNT: u64 = 0xbe1b90;
const G_ARRAY_DATA: u64 = 0xbe1b94;

/// `[rec+0x29]`, the byte the server sets to hide an addon from the list, which the rebuild
/// filters on (`0x51dc4f`). Its writers are the constructor `0x520550` (0), `0x51db84` and
/// `0x51db8c`.
const F_HIDDEN: u64 = 0x29;
/// The record's own hash of `##` directives: buckets `[rec+0xb4]`, mask `[rec+0xbc]`, read by
/// `AddOn_GetTitle` (`0x51df20`) at `0x51dfbc` and `0x51dfc9`. A mask of -1 means no metadata.
const F_META_BUCKETS: u64 = 0xb4;
const F_META_MASK: u64 = 0xbc;

/// Walks the registry list as `0x51f600` does: head `[0xbe1b6c]`, next at
/// `[node + [0xbe1b64] + 4]`, ending at a null pointer or one with bit 0 set.
fn walk_list(emu: &Emu) -> Vec<u32> {
    let linkoff = u64::from(emu.read_u32(G_LIST_LINKOFF));
    let mut out = Vec::new();
    let mut node = emu.read_u32(G_LIST_HEAD);
    while node != 0 && node & 1 == 0 {
        out.push(node);
        node = emu.read_u32(u64::from(node) + linkoff + 4);
        if out.len() > 64 {
            panic!("list walk did not terminate");
        }
    }
    out
}

/// Three bare records linked by the client's own initialiser and link, at `mode`.
fn link_three(emu: &mut Emu, mode: u32) -> Vec<u32> {
    emu.call(LIST_INIT, 0, 0, &[]);
    assert_eq!(
        emu.read_u32(G_LIST_LINKOFF),
        0xc,
        "0x51c3fe sets linkOffset = 0xC"
    );
    assert_eq!(
        emu.read_u32(G_LIST_SENTINEL) as u64,
        G_LIST_SENTINEL,
        "0x51c408 self-points the sentinel link"
    );
    assert_eq!(
        emu.read_u32(G_LIST_HEAD) as u64,
        G_LIST_SENTINEL | 1,
        "0x51c3f8 makes the head the bit-0-tagged terminator"
    );
    let recs: Vec<u32> = (0..3).map(|_| emu.alloc(REC_SIZE) as u32).collect();
    for &r in &recs {
        emu.call(LIST_LINK, G_LIST_BASE as u32, 0, &[r, mode, 0]);
    }
    recs
}

/// `0x51c9b0` links with mode 2, and the link sends only mode 1 to its head arm (`0x521b2e dec
/// esi; 0x521b2f je 0x521b47`); 2 falls through to `0x521b31`, which links before the sentinel. So
/// the list is in registration order. The mode 1 control comes out reversed.
#[test]
fn registry_link_mode_2_appends_at_the_tail() {
    let Some(wow) = Wow::load() else {
        return;
    };

    let mut emu = Emu::new(&wow);
    let tail = link_three(&mut emu, 2);
    assert_eq!(
        walk_list(&emu),
        tail,
        "mode 2 (the value 0x51caea pushes) appends: head->tail is registration order"
    );

    // Control: the same links at mode 1 take the head arm and come out reversed.
    let mut emu = Emu::new(&wow);
    let head = link_three(&mut emu, 1);
    let mut reversed = head.clone();
    reversed.reverse();
    assert_eq!(
        walk_list(&emu),
        reversed,
        "mode 1 takes the head arm at 0x521b47"
    );
}

/// The boot walk `0x51f600` visits the list from head to tail and loads only a record whose
/// `[rec+0x2c]` is 0 (`0x51f61e`, `0x51f623 jne`). The list is built by the client's own link.
#[test]
fn boot_walk_is_registration_order_and_skips_load_on_demand() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // Three addons without dependencies, not in alphabetical order.
    let names: [&[u8]; 3] = [b"Zeta\0", b"Alpha\0", b"Mid\0"];
    let mut name_ptrs = Vec::new();
    let mut hashes = Vec::new();
    for n in names {
        let p = emu.alloc(16);
        emu.write(p, n);
        name_ptrs.push(p as u32);
        hashes.push(emu.call(STR_HASH, 0, 0, &[p as u32]));
    }
    let mask: u32 = 0xff;
    let table = emu.alloc(0x100 * 12);
    emu.write_u32(G_ADDON_TABLE, table as u32);
    emu.write_u32(G_ADDON_MASK, mask);

    emu.call(LIST_INIT, 0, 0, &[]);
    // `0x51c360` zeroes the bucket pointer (`0x51c391`) and sets the mask to -1 (`0x51c412`); the
    // client sizes the table later, in `0x51c9b0`. Both are restored here, or the loader returns
    // early at `0x51f249`, or with the mask alone reads through a null base at `0x51f276`.
    emu.write_u32(G_ADDON_TABLE, table as u32);
    emu.write_u32(G_ADDON_MASK, mask);

    let mut recs = Vec::new();
    for i in 0..3 {
        let rec = emu.alloc(REC_SIZE);
        emu.write_u32(rec + F_HASH, hashes[i]);
        emu.write_u32(rec + F_NAME, name_ptrs[i]);
        emu.write(rec + F_LOADED, &[0]);
        emu.write_u32(rec + F_INTERFACE, IFACE_11200);
        emu.write_u32(rec + F_SECURITY, 1);
        emu.write(rec + F_CORRUPT, &[0]);
        // The middle addon is `## LoadOnDemand: 1`, which the boot walk skips.
        emu.write(rec + F_LOADONDEMAND, &[u8::from(i == 1)]);
        emu.write(rec + F_VISITING, &[0]);
        emu.write_u32(rec + F_OPTDEP_N, 0);
        emu.write_u32(rec + F_REQDEP_N, 0);
        emu.write_u32(rec + F_LOADWITH_N, 0);
        assert_ne!(hashes[i] & mask, 0xffff_ffff);
        emu.write_u32(table + u64::from(hashes[i] & mask) * 12 + 8, rec as u32);
        emu.call(LIST_LINK, G_LIST_BASE as u32, 0, &[rec as u32, 2, 0]);
        recs.push(rec);
    }

    let island = emu.alloc(16);
    emu.write(island, &[0xc2, 0x04, 0x00]); // ret 4
    let vtbl = emu.alloc(16);
    emu.write_u32(vtbl + 8, island as u32);
    let log_ctx = emu.alloc(16);
    emu.write_u32(log_ctx, vtbl as u32);
    let s_char = emu.alloc(16);
    emu.write(s_char, b"Char\0");
    let s_realm = emu.alloc(16);
    emu.write(s_realm, b"Realm\0");
    emu.stub(ENABLED_QUERY, 1, 1);
    emu.stub(VERSION_CVAR, 1, 0);
    emu.stub(CUR_CHARACTER, s_char as u32, 0);
    emu.stub(CUR_REALM, s_realm as u32, 0);
    emu.stub(LOGBUF_INIT, 0, 0);
    emu.stub(MD5_INIT, 0, 0);
    emu.stub(MD5_FINAL, 0, 0);
    emu.stub(LOGBUF_FINI, 0, 0);
    emu.stub(SSTR_PRINTF, 0, 0);
    emu.stub(FILE_EXISTS, 0, 2);
    emu.stub(RUN_TOC_FILES, 0, 1);
    emu.stub(LOAD_ERR_COUNT, 0, 0);
    emu.stub(SIGNAL_EVENT2, 0, 0);

    emu.clear_calls();
    emu.call(BOOT_LOAD_ALL, log_ctx as u32, 0, &[]);

    let fired: Vec<u32> = emu
        .calls()
        .iter()
        .filter(|c| c.addr == SIGNAL_EVENT2 && c.args[0] == EV_ADDON_LOADED)
        .map(|c| c.args[2])
        .collect();
    assert_eq!(
        fired,
        vec![name_ptrs[0], name_ptrs[2]],
        "registration order (Zeta, Mid), not alphabetical, without the LoadOnDemand record"
    );
    assert_eq!(emu.read(recs[0] + F_LOADED, 1)[0], 1);
    assert_eq!(
        emu.read(recs[1] + F_LOADED, 1)[0],
        0,
        "[rec+0x2c] != 0 is never loaded by the boot walk"
    );
    assert_eq!(emu.read(recs[2] + F_LOADED, 1)[0], 1);
}

/// The Lua index order differs from the list's: the tail of `0x51da70` rebuilds `[0xbe1b94]` from
/// the list, drops each record whose `[rec+0x29]` is set, and sorts with CRT `qsort` (`0x73f727`)
/// and the comparator `0x51deb0`, whose key is `AddOn_GetTitle` (`0x51df20`, the `## Title:`) or
/// else the folder name, compared by `SStrCmpI`.
///
/// The inputs tell the readings apart: the list order is neither the result nor its reverse,
/// `aardvark` against `Aardvark2` separates case-insensitive from byte order, one title sorts far
/// from its folder name, and one record is hidden.
#[test]
fn lua_index_array_is_title_sorted_case_insensitively() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let mask: u32 = 0xff;
    let table = emu.alloc(0x100 * 12);
    emu.call(LIST_INIT, 0, 0, &[]);
    emu.write_u32(G_ADDON_TABLE, table as u32);
    emu.write_u32(G_ADDON_MASK, mask);

    /// One record: its folder name, its `## Title:` if it has one, and whether `[rec+0x29]` hides
    /// it.
    struct Spec {
        name: &'static [u8],
        title: Option<&'static [u8]>,
        hidden: bool,
    }
    const fn spec(name: &'static [u8], title: Option<&'static [u8]>, hidden: bool) -> Spec {
        Spec {
            name,
            title,
            hidden,
        }
    }
    let spec: [Spec; 4] = [
        spec(b"Zeta\0", None, false),
        spec(b"Bagnon\0", Some(b"aardvark\0"), false), // its title sorts first, its name second
        spec(b"Aardvark2\0", None, false),             // case control against "aardvark"
        spec(b"Hidden\0", None, true),                 // [rec+0x29] = 1 -> excluded
    ];

    let mut name_ptrs = Vec::new();
    let mut recs = Vec::new();
    for Spec {
        name,
        title,
        hidden,
    } in spec
    {
        let np = emu.alloc(32);
        emu.write(np, name);
        let h = emu.call(STR_HASH, 0, 0, &[np as u32]);

        let rec = emu.alloc(REC_SIZE);
        emu.write_u32(rec + F_HASH, h);
        emu.write_u32(rec + F_NAME, np as u32);
        emu.write(rec + F_HIDDEN, &[u8::from(hidden)]);

        match title {
            // `0x51dfa5 cmp dword [rec+0xbc],-1`: no metadata, no title, so the folder name.
            None => emu.write_u32(rec + F_META_MASK, 0xffff_ffff),
            Some(t) => {
                // One node of the record's directive hash as `0x51df20` reads it: 12-byte
                // buckets, the node at `[bucket+8]`, key `[node+0x14]`, value `[node+0x18]`.
                let key = emu.alloc(16);
                emu.write(key, b"Title\0");
                let val = emu.alloc(32);
                emu.write(val, t);
                let kh = emu.call(STR_HASH, 0, 0, &[key as u32]);
                let node = emu.alloc(0x20);
                emu.write_u32(node, kh);
                emu.write_u32(node + 0x14, key as u32);
                emu.write_u32(node + 0x18, val as u32);
                let buckets = emu.alloc(0x100 * 12);
                emu.write_u32(buckets + u64::from(kh & mask) * 12 + 8, node as u32);
                emu.write_u32(rec + F_META_BUCKETS, buckets as u32);
                emu.write_u32(rec + F_META_MASK, mask);
            }
        }

        emu.write_u32(table + u64::from(h & mask) * 12 + 8, rec as u32);
        emu.call(LIST_LINK, G_LIST_BASE as u32, 0, &[rec as u32, 2, 0]);
        name_ptrs.push(np as u32);
        recs.push(rec);
    }
    // `0x51c360` cleared the table, which `AddOn_GetTitle` (`0x51df20`) reads to find a record.
    emu.write_u32(G_ADDON_TABLE, table as u32);
    emu.write_u32(G_ADDON_MASK, mask);

    // A destination large enough that the grow branch (`0x51dc65` to `0x51dc97`) is not taken.
    let array = emu.alloc(64);
    emu.write_u32(G_ARRAY_DATA, array as u32);
    emu.write_u32(G_ARRAY_CAP, 16);
    emu.write_u32(G_ARRAY_COUNT, 0xdead);

    let head = emu.read_u32(G_LIST_HEAD);
    assert_eq!(
        walk_list(&emu),
        recs.iter().map(|&r| r as u32).collect::<Vec<_>>(),
        "the input list is in registration order: Zeta, Bagnon, Aardvark2, Hidden"
    );

    let esp = 0x7000_0000u64 + 0x0010_0000 - 0x4000;
    emu.set_reg(RegisterX86::ESP, esp);
    emu.set_reg(RegisterX86::EBP, esp + 0x400);
    emu.set_reg(RegisterX86::EAX, u64::from(head));
    emu.run(ARRAY_REBUILD_BEGIN, ARRAY_REBUILD_END);

    let n = emu.read_u32(G_ARRAY_COUNT);
    assert_eq!(
        n, 3,
        "[rec+0x29] = 1 excludes exactly one record from the Lua index space"
    );
    let out: Vec<u32> = (0..n)
        .map(|i| emu.read_u32(array + u64::from(i) * 4))
        .collect();
    // Keys: "aardvark" (Bagnon's Title), "Aardvark2" (name), "Zeta" (name).
    assert_eq!(
        out,
        vec![name_ptrs[1], name_ptrs[2], name_ptrs[0]],
        "sorted case-insensitively by `## Title:`, else the folder name: Bagnon leads on its title"
    );
    // The array holds name pointers, also for the record sorted on its title.
    assert_eq!(emu.read(u64::from(out[0]), 7), b"Bagnon\0");
}
