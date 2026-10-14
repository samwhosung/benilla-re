//! The player's range and ratio gates against the exe. The four confirmation-dialog checks
//! (`0x5df8c0`, `0x5dfb10`, `0x5dfd30`, `0x5dff50`), `CGPlayer_C::OnAttackStart` (`0x5ea800`) and
//! `CGPlayer_C::StopMeleeAttack` (`0x5ecac0`) run whole in the emulator, with the virtual
//! `GetPosition` (vtable `+0x14`), the object lookups, the self gate and the tutorial and
//! attack-stop calls stubbed to chosen behaviour. The verdict is the observable: a boolean in
//! `eax`, or whether a stubbed call fired.
//!
//! Each distance gate runs across its threshold with the squared distance equal to it and a step
//! either side, then with NaN, infinities and zeros in the threshold, the radii and each position
//! component. The four dialog checks are byte-identical except for their operands, so each runs.
//! The ratio gates run over the signs, zeros and extremes of the integers, 0.5 exactly, and a
//! zero maximum.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::player_fp::*;
use common::*;

/// The squared range of the first dialog check in the tests; the exe reads it from `0xc4c28c`.
const THRESHOLD_5DF8C0: f32 = 109.2025;
const CLONE_5DF8C0: u64 = 0x5df8c0;
const CLONE_5DFB10: u64 = 0x5dfb10;
const CLONE_5DFD30: u64 = 0x5dfd30;
const CLONE_5DFF50: u64 = 0x5dff50;
const REACH_5EA800: u64 = 0x5ea800;
const STOP_ATTACK_5ECAC0: u64 = 0x5ecac0;

// The callees the tests stub.
const CLNTOBJMGR_OBJ: u64 = 0x468460; // ClntObjMgrObjectPtr(guidLo, guidHi, TYPEMASK, kind)
const GET_ACTIVE_GUID: u64 = 0x468550; // ClntObjMgrGetActivePlayerGuid -> EAX:EDX
const ENTRY_5ECDB0: u64 = 0x5ecdb0; // 0x5ea800 entry call
const SEND_ATTACK_STOP_624370: u64 = 0x624370; // CGUnit_C::SendAttackStop — CMSG_ATTACKSTOP 0x142 + [this+0xc54]=1
const IS_ATTACKING_60ECB0: u64 = 0x60ecb0; // StopAttack precondition: u64[this+0xc48] != 0
const TRIGGER_TUTORIAL_4B5390: u64 = 0x4b5390; // TriggerTutorial(id, delayMs) — TUTORIAL_TRIGGER popup flagger
const CANCEL_QUEUED_CAST_6E6F30: u64 = 0x6e6f30; // StopAttack tail-jmp: CancelQueuedCast (spell node)

/// Build an object whose `[obj]` vtable's slot +0x14 (the virtual `GetPosition`) returns, in EAX, a
/// pointer to a caller-controlled C3Vector (`pos_va`). The binary's dist² kernels read the position
/// through the EAX pointer the vcall returns (`mov esi,eax; fsub [esi]` / `fld [eax]`), ignoring the
/// stack out-param — so the stub just returns `pos_va` and cleans its 1 stack arg (`ret 4`).
/// Returns the object VA. `pos` initialises the payload (rewritten per case via `pos_va`).
fn make_obj_with_position(e: &mut Emu, slot14_target: u32, pos: [f32; 3], pos_va: u64) -> u64 {
    let obj = e.alloc(0x400);
    let vtbl = e.alloc(0x80);
    // slot +0x14 = the GetPosition stub island.
    e.write_u32(vtbl + 0x14, slot14_target);
    e.write_u32(obj, vtbl as u32);
    for (i, c) in pos.iter().enumerate() {
        e.write_u32(pos_va + 4 * i as u64, c.to_bits());
    }
    obj
}

fn run_clone(e: &mut Emu, addr: u64, threshold: f32) {
    // ClntObjMgrObjectPtr is stubbed: ret_eax = the resolved object VA (or 0). The stub cleans
    // its args: `mov edx,0x86018c; mov ecx,8; push id; push guidLo; push guidHi; call` — the call
    // is __fastcall(ecx,edx) with TWO stack args (guidHi-ish + id) per the prologue; objdump shows
    // 3 pushes (eax, ecx, imm) → __fastcall takes ecx/edx in regs + 3 stack args. We let the stub
    // pop all stack args via pop_args; 0x468460's own `ret` cleans them, but our stub must mimic.
    // The function reads only the EAX result (the object ptr), so pop_args must match how the call
    // site expects the stack: it is a __fastcall `call` with the 3 pushed dwords as stack args, so
    // pop_args = 3.
    let this_payload = e.alloc(0x10);
    let target_payload = e.alloc(0x10);
    let this_island = e.alloc(1);
    let target_island = e.alloc(1);
    e.stub(this_island, this_payload as u32, 1);
    e.stub(target_island, target_payload as u32, 1);

    let this_obj = make_obj_with_position(e, this_island as u32, [0.0; 3], this_payload);
    let target_obj = make_obj_with_position(e, target_island as u32, [0.0; 3], target_payload);

    // resolve to target_obj (non-null) so the FP path runs.
    e.stub(CLNTOBJMGR_OBJ, target_obj as u32, 3);
    e.write_u32(0xc4c28c, threshold.to_bits()); // the runtime threshold global

    let probe = |e: &mut Emu, this_p: [f32; 3], tgt_p: [f32; 3], ctx: &str| -> bool {
        for (i, c) in this_p.iter().enumerate() {
            e.write_u32(this_payload + 4 * i as u64, c.to_bits());
        }
        for (i, c) in tgt_p.iter().enumerate() {
            e.write_u32(target_payload + 4 * i as u64, c.to_bits());
        }
        let eax = e.call(addr, this_obj as u32, 0, &[]);
        let bin = eax & 1 == 1;
        let ours = range_gate_5df8c0(true, this_p, tgt_p, threshold);
        assert_eq!(bin, ours, "{ctx}: binary={bin} ours={ours}");
        bin
    };

    let r = (threshold as f64).sqrt() as f32;
    probe(e, [0.0; 3], [r, 0.0, 0.0], "at-radius");
    probe(e, [0.0; 3], [r * 0.99, 0.0, 0.0], "just-in");
    probe(e, [0.0; 3], [r * 1.01, 0.0, 0.0], "just-out");
    probe(e, [0.0; 3], [0.0; 3], "coincident");

    let mut s = 0xd_f8c0u32 ^ (addr as u32);
    let (mut seen_in, mut seen_out) = (false, false);
    // Fuzz positions scaled to the radius so both the in-range and out-of-range arms fire reliably.
    let span = r * 1.5;
    for it in 0..2000u32 {
        let this_p = [
            frand(&mut s, -span, span),
            frand(&mut s, -span, span),
            frand(&mut s, -span, span),
        ];
        let tgt_p = [
            frand(&mut s, -span, span),
            frand(&mut s, -span, span),
            frand(&mut s, -span, span),
        ];
        let res = probe(e, this_p, tgt_p, &format!("fuzz {addr:#x} {it}"));
        seen_in |= res;
        seen_out |= !res;
    }
    assert!(seen_in && seen_out, "{addr:#x}: both arms must fire");

    // Thresholds and positions at the edges: the squared distance equal to the threshold (3-4-5
    // gives 25 exactly) and a step either side, then NaN, infinities and zeros for the threshold
    // and for each position component.
    let probe_thr = |e: &mut Emu, this_p: [f32; 3], tgt_p: [f32; 3], thr: f32, ctx: &str| {
        e.write_u32(0xc4c28c, thr.to_bits());
        for (i, c) in this_p.iter().enumerate() {
            e.write_u32(this_payload + 4 * i as u64, c.to_bits());
        }
        for (i, c) in tgt_p.iter().enumerate() {
            e.write_u32(target_payload + 4 * i as u64, c.to_bits());
        }
        let eax = e.call(addr, this_obj as u32, 0, &[]);
        let bin = eax & 1 == 1;
        let ours = range_gate_5df8c0(true, this_p, tgt_p, thr);
        assert_eq!(bin, ours, "{ctx}: thr={thr:e} binary={bin} ours={ours}");
    };
    for x in [3.0f32, next_up(3.0), next_down(3.0)] {
        probe_thr(e, [0.0; 3], [x, 4.0, 0.0], 25.0, "equal");
    }
    for thr in [next_up(25.0), next_down(25.0)] {
        probe_thr(e, [0.0; 3], [3.0, 4.0, 0.0], thr, "threshold step");
    }
    for &v in &specials(&[25.0]) {
        probe_thr(e, [0.0; 3], [3.0, 4.0, 0.0], v, "special threshold");
        for k in 0..3 {
            let mut t = [3.0f32, 4.0, 0.0];
            t[k] = v;
            probe_thr(e, [0.0; 3], t, 25.0, "special target");
            let mut o = [0.0f32; 3];
            o[k] = v;
            probe_thr(e, o, [3.0, 4.0, 0.0], 25.0, "special self");
        }
    }
}

#[test]
fn range_gate_5df8c0_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    run_clone(&mut e, CLONE_5DF8C0, THRESHOLD_5DF8C0);
}

#[test]
fn range_gate_clone_family_difftest() {
    // The three clones of 0x5df8c0 run the same code at other addresses; each gets a fresh Emu with
    // its own stubs.
    for &addr in &[CLONE_5DFB10, CLONE_5DFD30, CLONE_5DFF50] {
        let Some(wow) = Wow::load() else {
            return;
        };
        let mut e = Emu::new(&wow);
        run_clone(&mut e, addr, 25.0f32);
    }
}

#[test]
fn reach_gate_5ea800_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(ENTRY_5ECDB0, 0, 0); // entry call FUN_005ecdb0(ecx=0); __fastcall no stack args
    e.stub(SEND_ATTACK_STOP_624370, 0, 0); // 0x624370 SendAttackStop (observable); __fastcall no stack args

    let this_payload = e.alloc(0x10);
    let target_payload = e.alloc(0x10);
    let this_island = e.alloc(1);
    let target_island = e.alloc(1);
    e.stub(this_island, this_payload as u32, 1);
    e.stub(target_island, target_payload as u32, 1);

    // this/target objects each carry a unit sub-object at +0x110 with a bounding radius at +0x1f0.
    let this_obj = make_obj_with_position(&mut e, this_island as u32, [0.0; 3], this_payload);
    let target_obj = make_obj_with_position(&mut e, target_island as u32, [0.0; 3], target_payload);
    let this_unit = e.alloc(0x400);
    let target_unit = e.alloc(0x400);
    e.write_u32(this_obj + 0x110, this_unit as u32);
    e.write_u32(target_obj + 0x110, target_unit as u32);

    let probe = |e: &mut Emu,
                 this_r: f32,
                 tgt_r: f32,
                 this_p: [f32; 3],
                 tgt_p: [f32; 3],
                 ctx: &str|
     -> bool {
        e.write_u32(this_unit + 0x1f0, this_r.to_bits());
        e.write_u32(target_unit + 0x1f0, tgt_r.to_bits());
        for (i, c) in this_p.iter().enumerate() {
            e.write_u32(this_payload + 4 * i as u64, c.to_bits());
        }
        for (i, c) in tgt_p.iter().enumerate() {
            e.write_u32(target_payload + 4 * i as u64, c.to_bits());
        }
        e.clear_calls();
        e.call(REACH_5EA800, this_obj as u32, 0, &[target_obj as u32]);
        let bin = e.calls().iter().any(|c| c.addr == SEND_ATTACK_STOP_624370);
        let ours = reach_gate_5ea800(this_r, tgt_r, this_p, tgt_p);
        assert_eq!(
            bin, ours,
            "{ctx}: binary_sends_attackstop={bin} ours={ours}"
        );
        bin
    };

    // reach = (r1 + r2 + 1.3333334), floored to 5.0, ×4. With r1=r2=0 → reach = max(1.3333,5)=5,
    // ×4 = 20, reach² = 400 → dist of 20 is the knee.
    probe(&mut e, 0.0, 0.0, [0.0; 3], [20.0, 0.0, 0.0], "knee-at");
    probe(&mut e, 0.0, 0.0, [0.0; 3], [19.0, 0.0, 0.0], "inside-knee");
    probe(&mut e, 0.0, 0.0, [0.0; 3], [21.0, 0.0, 0.0], "outside-knee");
    probe(&mut e, 5.0, 5.0, [0.0; 3], [0.0; 3], "coincident-big-radii");
    // `reach^2 == dist^2` exactly at the knee (reach 20) and a step either side, the 5.0 floor with
    // a radius sum under it and equal to it, and NaN, infinities and zeros in each input.
    for x in [20.0f32, next_up(20.0), next_down(20.0)] {
        probe(&mut e, 0.0, 0.0, [0.0; 3], [x, 0.0, 0.0], "knee step");
    }
    let floor_sum = 5.0f32 - REACH_5EA800_SLOP;
    for r in [
        floor_sum,
        next_up(floor_sum),
        next_down(floor_sum),
        0.0,
        -0.0,
        3.0,
    ] {
        probe(&mut e, r, 0.0, [0.0; 3], [20.0, 0.0, 0.0], "floor");
        probe(&mut e, 0.0, r, [0.0; 3], [20.0, 0.0, 0.0], "floor");
    }
    for &v in &specials(&[20.0, 5.0]) {
        probe(
            &mut e,
            v,
            1.0,
            [0.0; 3],
            [20.0, 0.0, 0.0],
            "special this radius",
        );
        probe(
            &mut e,
            1.0,
            v,
            [0.0; 3],
            [20.0, 0.0, 0.0],
            "special target radius",
        );
        for k in 0..3 {
            let mut t = [20.0f32, 0.0, 0.0];
            t[k] = v;
            probe(&mut e, 0.0, 0.0, [0.0; 3], t, "special target");
            let mut o = [0.0f32; 3];
            o[k] = v;
            probe(&mut e, 0.0, 0.0, o, [20.0, 0.0, 0.0], "special self");
        }
    }

    let mut s = 0x5ea8_0001u32;
    let (mut seen_in, mut seen_out) = (false, false);
    for it in 0..3000u32 {
        let this_r = frand(&mut s, 0.0, 10.0);
        let tgt_r = frand(&mut s, 0.0, 10.0);
        let this_p = [
            frand(&mut s, -40.0, 40.0),
            frand(&mut s, -40.0, 40.0),
            frand(&mut s, -40.0, 40.0),
        ];
        let tgt_p = [
            frand(&mut s, -40.0, 40.0),
            frand(&mut s, -40.0, 40.0),
            frand(&mut s, -40.0, 40.0),
        ];
        let r = probe(&mut e, this_r, tgt_r, this_p, tgt_p, &format!("fuzz {it}"));
        seen_in |= r;
        seen_out |= !r;
    }
    assert!(
        seen_in && seen_out,
        "both arms must fire (in={seen_in} out={seen_out})"
    );
}

// `0x5ecac0` sends CMSG_ATTACKSTOP (`0x624370`) and tail-jumps into `CancelQueuedCast` (`0x6e6f30`).
// Its float code is the low-health and low-mana ratio pair, whose result is a `TriggerTutorial`
// (`0x4b5390`) call with `0xa` or `0xb`.

#[test]
fn lowhp_warn_5ecac0_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // self-gate: own GUID (`[[this+8]][0/1]`) == GetActivePlayerGuid. The function does
    // `mov edi,[[this+8]]; mov ebx,[[this+8]+4]; call 0x468550; cmp edi,eax; cmp ebx,edx`. The
    // (stubbed) 0x468550 returns the GUID lo in EAX, hi in EDX. The harness `stub` sets only EAX;
    // EDX keeps its entry value. For a `__fastcall(this)` invoked via `e.call(.., this, 0, &[])`
    // the entry EDX = 0, and 0x468550 is not reached before the cmp clobbers EDX-via-callee — so
    // at `cmp ebx,edx` EDX is whatever the stub left, i.e. the entry EDX = 0. So pin GUID hi = 0
    // (matches EDX=0) and GUID lo = a chosen value the stub returns.
    let guid_lo = 0xdead_beefu32;
    e.stub(GET_ACTIVE_GUID, guid_lo, 0); // EAX = guid_lo; EDX stays 0 (the edx passed to call)
    e.stub(IS_ATTACKING_60ECB0, 1, 0); // precondition true (al != 0)
    e.stub(TRIGGER_TUTORIAL_4B5390, 0, 0); // 0x4b5390 TriggerTutorial (the observable); __fastcall, count it
    e.stub(SEND_ATTACK_STOP_624370, 0, 0); // SendAttackStop (always called once the guards pass)
    e.stub(CANCEL_QUEUED_CAST_6E6F30, 0, 0); // CancelQueuedCast tail-jmp — stub so emulation halts cleanly

    let this_obj = e.alloc(0x400);
    e.write_u32(this_obj + 0x8, /* descriptor ptr */ 0);
    // `[this+8]` is a pointer to the descriptor; `[desc][0]/[1]` = GUID lo/hi. Allocate a desc.
    let desc = e.alloc(0x20);
    e.write_u32(this_obj + 0x8, desc as u32);
    e.write_u32(desc, guid_lo); // GUID lo (edi)
    e.write_u32(desc + 0x4, 0); // GUID hi (ebx) = 0 to match EDX=0

    let unit = e.alloc(0x400);
    e.write_u32(this_obj + 0x110, unit as u32);
    e.write_u32(this_obj + 0xc50, 1); // precondition fallback flag (also true)

    let probe = |e: &mut Emu,
                 health: i32,
                 max_health: i32,
                 power: i32,
                 max_power: i32,
                 ptype_zero: bool,
                 ctx: &str| {
        e.write_u32(unit + 0x40, health as u32);
        e.write_u32(unit + 0x58, max_health as u32);
        e.write_u32(unit + 0x44, power as u32);
        e.write_u32(unit + 0x5c, max_power as u32);
        e.write(unit + 0x7b, &[if ptype_zero { 0 } else { 1 }]);
        e.clear_calls();
        e.call(STOP_ATTACK_5ECAC0, this_obj as u32, 0, &[]);
        let warn_count = e
            .calls()
            .iter()
            .filter(|c| c.addr == TRIGGER_TUTORIAL_4B5390)
            .count() as u32;
        let (hw, mw) = lowhp_warn_5ecac0(health, max_health, power, max_power, ptype_zero);
        let ours = (hw as u32) + (mw as u32);
        assert_eq!(
            warn_count, ours,
            "{ctx}: binary warns={warn_count} ours={ours} (h={hw} m={mw})"
        );
    };

    // Targeted: pin each gate independently so the total count diffs each boolean. Health gate
    // fires iff health/maxHealth < 0.5; mana gate iff power/maxPower < 0.5 (and powerType==0).
    // -- only health gate live (powerType != 0 disables mana):
    probe(&mut e, 40, 100, 50, 100, false, "hp-low only");
    probe(&mut e, 60, 100, 50, 100, false, "hp-ok only");
    // -- only mana gate live (health full so no hp warn; powerType==0):
    probe(&mut e, 100, 100, 40, 100, true, "mana-low only");
    probe(&mut e, 100, 100, 60, 100, true, "mana-ok only");
    // -- both:
    probe(&mut e, 40, 100, 40, 100, true, "both-low");
    probe(&mut e, 49, 100, 49, 100, true, "both-just-under-half");
    probe(&mut e, 50, 100, 50, 100, true, "both-exactly-half");
    // `max_power == 0`: the `fcom 0.0; jnp` guard fires the mana tutorial without dividing.
    probe(
        &mut e,
        100,
        100,
        0,
        0,
        true,
        "maxpower-0 power-0 → mana-warn",
    );
    probe(
        &mut e,
        100,
        100,
        50,
        0,
        true,
        "maxpower-0 power-50 → mana-warn",
    );
    probe(
        &mut e,
        40,
        100,
        7,
        0,
        true,
        "maxpower-0 + hp-low → both warn",
    );
    // maxHealth == 0 (no guard in the health path — `fidiv` by 0 → +inf > 0.5 → no hp warn).
    probe(
        &mut e,
        50,
        0,
        50,
        100,
        false,
        "maxhealth-0 → +inf, no hp-warn",
    );

    // The signs, zeros and extremes of the integers: `fidiv` of 0 by 0 is NaN, a negative health
    // over a zero maximum is -inf, and a ratio of exactly 0.5 is not below it.
    let ints = [
        0,
        1,
        -1,
        2,
        -2,
        50,
        100,
        i32::MAX,
        i32::MIN,
        0x4000_0000,
        -0x4000_0000,
    ];
    for &h in &ints {
        for &mh in &ints {
            probe(&mut e, h, mh, h, mh, true, "ints");
            probe(&mut e, h, 100, 50, mh, true, "ints power");
        }
    }
    probe(&mut e, 50, 100, 50, 100, true, "exactly half");
    probe(&mut e, 1, 2, 1, 2, true, "exactly half small");
    probe(
        &mut e,
        i32::MAX,
        i32::MAX,
        i32::MIN,
        i32::MIN,
        true,
        "extremes",
    );

    let mut s = 0x5eca_c001u32;
    let mut seen = [false; 3]; // the warn counts 0, 1 and 2 seen
    for it in 0..3000u32 {
        let max_health = 1 + (next(&mut s) >> 8) % 5000;
        let health = (next(&mut s) >> 8) % (max_health + max_health / 2);
        let ptype_zero = next(&mut s) >> 31 == 1;
        let max_power = 1 + (next(&mut s) >> 8) % 5000;
        let power = (next(&mut s) >> 8) % (max_power + max_power / 2);
        let (hw, mw) = lowhp_warn_5ecac0(
            health as i32,
            max_health as i32,
            power as i32,
            max_power as i32,
            ptype_zero,
        );
        seen[(hw as usize) + (mw as usize)] = true;
        probe(
            &mut e,
            health as i32,
            max_health as i32,
            power as i32,
            max_power as i32,
            ptype_zero,
            &format!("fuzz {it}"),
        );
    }
    assert!(
        seen[0] && seen[1] && seen[2],
        "all warn counts must fire: {seen:?}"
    );
}
