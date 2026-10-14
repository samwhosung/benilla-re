//! The damage range rounding of `GetTradeSkillItemStats` (`0x4fdec0`) against the exe: the slice
//! `0x4fe48d` to `0x4fe4cb` biases the maximum up and the minimum down by 0.99999 (`0x808120`)
//! before `__ftol` truncates it.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::tooltip::{bias_round_neg, bias_round_pos};
use fpkit::rngf;

// The damage-range quantize block: fld [esi+0xcc]; fcom 0.0; jne; fadd [0x808120]; call __ftol;
// fld [esi+0xb8]; fcom 0.0; mov edi,eax; je; fsub [0x808120]; call __ftol. Halt at 0x4fe4cb.
// NB [esi+0xcc] is DamageMAX (ceil leg) and [esi+0xb8] is DamageMIN (floor leg) — the two were
// named the wrong way round until 2026-09-09; see the src module's correction note.
const SLICE_START: u64 = 0x4fe48d;
const SLICE_END: u64 = 0x4fe4cb;
const FIELD_DMGMAX_OFF: u64 = 0xcc; // [esi+0xcc] = DamageMax -> the ceil leg
const FIELD_DMGMIN_OFF: u64 = 0xb8; // [esi+0xb8] = DamageMin -> the floor leg

/// Both arithmetic ops in one slice at their own addresses. Feed the two field f32s at [esi+0xcc]/[esi+0xb8],
/// read back edi (min __ftol result) and eax (max __ftol result).
#[test]
fn luabind_4fde_get_trade_skill_item_stats_quantize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // Pin the end of the function: the byte at 0x4ff37d is `c3` (ret) and the next function's
    // hot-patch prologue `8b ff` (mov edi,edi) sits at 0x4ff37e, so the function ends where its row
    // says (size 0x14be).
    assert_eq!(emu.read(0x4ff37d, 1), vec![0xc3], "not a ret @ 0x4ff37d");
    assert_eq!(
        emu.read(0x4ff37e, 2),
        vec![0x8b, 0xff],
        "next fn hot-patch prologue (mov edi,edi) not at 0x4ff37e — bound wrong"
    );
    // Pin the two arithmetic op bytes too (fadd/fsub dword [0x808120]) so the slice diffs the ops the
    // row claims, not a relocated copy.
    assert_eq!(
        emu.read(0x4fe4a0, 6),
        vec![0xd8, 0x05, 0x20, 0x81, 0x80, 0x00],
        "fadd dword [0x808120] bytes @ 0x4fe4a0"
    );
    assert_eq!(
        emu.read(0x4fe4c0, 6),
        vec![0xd8, 0x25, 0x20, 0x81, 0x80, 0x00],
        "fsub dword [0x808120] bytes @ 0x4fe4c0"
    );

    // A scratch "object" for esi (room for [esi+0xcc]), and a valid stack for the __ftol calls.
    let obj = emu.alloc(0x200);
    let stack = emu.alloc(0x1000);
    let esp = stack + 0x800;
    let ebp = obj + 0x190; // gives a valid [ebp-0x10] read inside the slice (value irrelevant)

    let run = |emu: &mut Emu, v: f32, w: f32| -> (i32, i32) {
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.write_u32(obj + FIELD_DMGMAX_OFF, v.to_bits()); // [esi+0xcc] = DamageMax field
        emu.write_u32(obj + FIELD_DMGMIN_OFF, w.to_bits()); // [esi+0xb8] = DamageMin field
        emu.run(SLICE_START, SLICE_END);
        let ceil_i = emu.get_reg(RegisterX86::EDI) as u32 as i32; // mov edi,eax saved the ceil leg's __ftol
        let floor_i = emu.get_reg(RegisterX86::EAX) as u32 as i32; // the floor leg's __ftol
        (ceil_i, floor_i)
    };

    let mut total = 0u32;
    let check = |emu: &mut Emu, total: &mut u32, v: f32, w: f32| {
        let (gm, gx) = run(emu, v, w);
        assert_eq!(
            gm,
            bias_round_pos(v),
            "bias_round_pos v={v} (bits={:#010x})",
            v.to_bits()
        );
        assert_eq!(
            gx,
            bias_round_neg(w),
            "bias_round_neg w={w} (bits={:#010x})",
            w.to_bits()
        );
        *total += 2;
    };

    // ---- deterministic == boundary probes ------------------------------------------------
    // fcom sign pivot: exactly 0.0 / -0.0 / smallest positive / smallest negative; the +0.99999
    // fadd happens iff v>0, the -0.99999 fsub happens iff w<=0. And the __ftol truncate pivots:
    // a positive value whose +0.99999 lands just below / at an integer.
    let pivots = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        f32::from_bits(0x0000_0001), // smallest positive subnormal
        -f32::from_bits(0x0000_0001),
        1e-7,
        -1e-7,
        0.000_009_999, // +0.99999 -> ~0.99999... just below 1.0 -> trunc 0
        0.000_010_1,   // +0.99999 -> just above 1.0 -> trunc 1
        9.000_01,      // +0.99999 -> ~10.0 -> 9 or 10 boundary
        10.0,
        99.999_99,
        100.0,
        1234.5,
        -1234.5,
        2_000_000.0,
        -2_000_000.0,
        // NaN, the infinities and values past `i32` and `i64`: `__ftol` returns the low dword of
        // its qword, 0 for the indefinite.
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        3.0e9,
        -3.0e9,
        1.0e10,
        1.0e30,
        -1.0e30,
    ];
    for &v in &pivots {
        for &w in &pivots {
            check(&mut emu, &mut total, v, w);
        }
    }

    // ---- dense integer-target sweep: v near each small integer (the display range) ----------
    for k in 0i32..=512 {
        let base = k as f32;
        for d in [-1e-3f32, 0.0, 1e-3, 0.4999, 0.5] {
            let v = base + d;
            check(&mut emu, &mut total, v, -v);
            check(&mut emu, &mut total, -v, v);
        }
    }

    // ---- fuzz: HIGH LCG bits, both signs, a wide magnitude range ----------------------------
    let mut s = 0x4fde_c000u32;
    for _ in 0..6000 {
        let v = rngf(&mut s, -50_000.0, 50_000.0) as f32;
        let w = rngf(&mut s, -50_000.0, 50_000.0) as f32;
        check(&mut emu, &mut total, v, w);
    }
    // a tight band around the sign boundary, where the branch decision is most sensitive.
    for _ in 0..3000 {
        let v = rngf(&mut s, -2.0, 2.0) as f32;
        let w = rngf(&mut s, -2.0, 2.0) as f32;
        check(&mut emu, &mut total, v, w);
    }

    eprintln!(
        "luabind_4fde_get_trade_skill_item_stats_quantize: {total} bit-exact \
         (slice 0x4fe48d..0x4fe4cb: fadd[0.99999]@0x4fe4a0 + fsub[0.99999]@0x4fe4c0, both __ftol native)"
    );
}
