//! Differential tests of the arithmetic of the map arrow frames `PositionWorldMapArrowFrame`
//! (`0x4a8d20`) and `PositionMiniWorldMapArrowFrame` (`0x4a8f20`) and of the pending-mail
//! counters `HandleReceivedMailPacket` (`0x4ad620`) and `UpdatePendingMail` (`0x4ade60`), each
//! run against `WoW.exe` in the emulator.
//!
//! * **`luabind_4a8d20_arrow_scale_divide`** — the OWNED metric kernel of
//!   `PositionWorldMapArrowFrame` (`0x4a8d20`), diffed as an **instruction slice** (`Emu::run`) over
//!   each coordinate's `call 0x41ad70; fmul [1024.0]; push ecx; fdivr [ebp-N]; fstp [esp]` block —
//!   the leaf `0x41ad70` (`fld [0x832a4c]; ret`) runs natively; halt before the delegated
//!   `0x41ae60`. Covers `fmul ×2 + fdivr ×2`. The MINI clone `0x4a8f20`
//!   (`PositionMiniWorldMapArrowFrame`) is byte-verified clone-equal here (masked rel32/disp32/imm32
//!   only) and its two blocks are diffed at their own addresses too — so the clone inheritance is
//!   proven, not assumed.
//! * **`luabind_4a8f20_mini_arrow_scale_divide`** — the MINI clone's own two blocks, diffed at
//!   `0x4a90a0`/`0x4a90c9`. Same kernel; redundant proof of the inherited arithmetic.
//! * **`glue_4a8_4ad620_set_value`** — whole-function `Emu::call` of `0x4ad620` with the two
//!   delegated callees (`0x419130` buffer-append `ret 4`, `0x703e50` notify `ret 0`) stubbed,
//!   reading back the value global `[0x845eac]`. Covers the lone `fabs` (`0x4ad64e`) by
//!   construction, with `==eps` / `==0.0` / `arg==cur` threshold probes.
//! * **`glue_4a8_4ade60_step_value`** — whole-function `Emu::call` of `0x4ade60`, same stubs,
//!   reading back `[0x845eac]`. Covers `fsub` (`0x4ade7c`) + `fabs` (`0x4adea6`) by construction,
//!   with the `cur<=0`, `(cur-arg)==0`, and `|new|==eps` boundary probes.
//!
//! Fuzz reads the LCG's HIGH bits. Bit-exact = `.to_bits()` equality.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::worldmap_arrow::{
    arrow_scale_divide, mini_arrow_scale_divide, set_value, step_value, EPS_BITS, SCALE1024_BITS,
    ZERO_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

// Owned/delegated VAs.
const SCALE_GLOBAL_VA: u64 = 0x832a4c; // [.data] UI-scale global, loaded by leaf 0x41ad70
const SCALE1024_VA: u64 = 0x7ffd68; // [.rdata] = 1024.0 (mapped; never written)
const EPS_VA: u64 = 0x8029d4; // [.rdata] = 2^-22 epsilon threshold
const ZERO_VA: u64 = 0x7ffd74; // [.rdata] = 0.0
const VALUE_GLOBAL_VA: u64 = 0x845eac; // [.data] the cur value 0x4ad620/0x4ade60 read+write
const FLAG_A_VA: u64 = 0xb6ef88; // [.data] busy flag #1 (0x4ad620)
const FLAG_B_VA: u64 = 0xb6ef8c; // [.data] busy flag #2 (0x4ad620)

// The metric-kernel slice starts at the leaf scale-load 0x41ad70 (runs natively) and halts before
// logical_to_device 0x41ae60; the Lua getter 0x6f3620 is never reached by the slice.
// The threshold-fn callees, stubbed for the whole-fn calls.
const BUF_APPEND: u64 = 0x419130; // ret 4 (thiscall, 1 pushed dword arg)
const NOTIFY: u64 = 0x703e50; // ret 0 (fastcall ecx, no stack args)

// ===========================================================================================
// 0x4a8d20 PositionWorldMapArrowFrame — the OWNED metric scale-divide kernel, twice (X then Y).
//
// Each block: call 0x41ad70 (st0=[0x832a4c]); fmul [0x7ffd68]=1024.0; push ecx; fdivr [ebp-N];
// fstp [esp]. We seed [0x832a4c]=scale and [ebp-N]=value, run from the scale-load call to just
// after `fstp [esp]`, and read the f32 result from [esp] (= esp0-4 after the one `push ecx`).
//
//   X coord: 0x4a8ea0 (call) .. 0x4a8eb2 (call 0x41ae60, not executed). fdivr [ebp-8].   fmul,fdivr
//   Y coord: 0x4a8ec9 (call) .. 0x4a8edb (call 0x41ae60, not executed). fdivr [ebp-4].   fmul,fdivr
// ===========================================================================================
const REP_BLOCKS: [(u64, u64, i64); 2] = [
    (0x4a8ea0, 0x4a8eb2, -0x8), // X: fdivr [ebp-8]
    (0x4a8ec9, 0x4a8edb, -0x4), // Y: fdivr [ebp-4]
];
// The MINI clone 0x4a8f20's equivalent two blocks (byte-identical kernel).
const MINI_BLOCKS: [(u64, u64, i64); 2] = [
    (0x4a90a0, 0x4a90b2, -0x8), // X: fdivr [ebp-8]
    (0x4a90c9, 0x4a90db, -0x4), // Y: fdivr [ebp-4]
];

fn run_metric(
    emu: &mut Emu,
    ebp: u64,
    esp0: u64,
    blk: (u64, u64, i64),
    value: f32,
    scale: f32,
) -> f32 {
    let (start, end, disp) = blk;
    emu.write_u32(SCALE_GLOBAL_VA, scale.to_bits()); // [0x832a4c] = scale (0x41ad70 loads it)
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::ESP, esp0);
    emu.set_reg(RegisterX86::ECX, 0); // the `push ecx` placeholder
    let slot = (ebp as i64 + disp) as u64;
    emu.write_u32(slot, value.to_bits()); // [ebp+disp] = coord value as f32
    emu.run(start, end);
    // result `fstp dword [esp]`; esp = esp0 - 4 (one push ecx happened in the slice).
    emu.read_f32(esp0 - 4)
}

fn exercise_metric(
    emu: &mut Emu,
    ebp: u64,
    esp0: u64,
    blocks: &[(u64, u64, i64); 2],
    tag: &str,
) -> u32 {
    let mut total = 0u32;
    // deterministic boundary probes: scale==1.0 exact, value==0.0 exact, negative, scale==2.0.
    let fixed: [(f32, f32); 6] = [
        (1.0, 1.0),     // 1 / 1024 exactly
        (0.0, 1.0),     // value==0.0 boundary -> 0.0
        (-512.0, 1.0),  // negative -> negative quotient
        (2048.0, 2.0),  // scale==2.0 -> 2048/(2*1024) = 1.0 exactly
        (1234.5, 0.75), // arbitrary
        (-1.0e6, 1.5),  // large negative
    ];
    let mut s = (blocks[0].0 as u32).wrapping_mul(0x1001);
    for &blk in blocks {
        for (value, scale) in fixed {
            let bin = run_metric(emu, ebp, esp0, blk, value, scale);
            let mine = arrow_scale_divide(value, scale);
            assert_eq!(
                mine.to_bits(),
                bin.to_bits(),
                "{tag} metric @ {:#x} value={value} scale={scale}",
                blk.0
            );
            total += 1;
        }
        for _ in 0..2000 {
            let value = rng(&mut s, -1.0e5, 1.0e5);
            let scale = rng(&mut s, 0.1, 4.0);
            let bin = run_metric(emu, ebp, esp0, blk, value, scale);
            let mine = arrow_scale_divide(value, scale);
            assert_eq!(
                mine.to_bits(),
                bin.to_bits(),
                "{tag} metric @ {:#x} fuzz value={value} scale={scale}",
                blk.0
            );
            total += 1;
        }
    }
    total
}

#[test]
fn luabind_4a8d20_arrow_scale_divide() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80;
    let stack = emu.alloc(0x1000);
    let esp0 = stack + 0xf00;

    // Independence: read the binary's own mapped constant, never re-derive from our bits.
    assert_eq!(emu.read_u32(SCALE1024_VA), SCALE1024_BITS, "1024.0 .rdata");

    // ---- CLONE PROOF: 0x4a8d20 vs 0x4a8f20 byte-identical modulo masked rel32/disp32/imm32. ----
    // Both are 512 bytes / 179 instructions. We assert byte-equality after masking each differing
    // 4-byte field. The only diffs (verified at the bytes): 24 direct `call` rel32 (each resolves
    // to the SAME absolute target), the frame-object global disp32 (0xb6e6b8 vs 0xb6e6bc), and one
    // error-string imm32 (0x8456b8 vs 0x845768). We mask by zeroing each masked-field span in both
    // copies, then compare. (The lone indirect `ff 52 28` call carries no rel32 and is identical.)
    {
        let a = emu.read(0x4a8d20, 512);
        let b = emu.read(0x4a8f20, 512);
        // Offsets (within the 512-byte fn) of the 4-byte masked fields, with their kind.
        // 24 direct `call` rel32 (opcode E8, dword at +1) — the lone indirect `ff 52 28` at
        // 0x4a8dbe carries no rel32 and is byte-identical, so it is NOT listed — plus the mov-edi
        // disp32 and one push imm32. We mask the disp32/imm32/rel32 dwords by zeroing.
        let mut am = a.clone();
        let mut bm = b.clone();
        // Helper: zero a 4-byte little-endian field at offset `o`.
        let mask = |v: &mut [u8], o: usize| {
            for k in 0..4 {
                v[o + k] = 0;
            }
        };
        // The masked-field offsets (VA - 0x4a8d20) for the rel32/disp32/imm32 dwords, taken from the
        // normalized-byte compare (every differing byte sat at the dword after the opcode):
        //   mov edi,[disp32]   @VA 0x4a8d2c, opcode `8b 3d` => dword at +2  (frame-object global)
        //   24 call rel32      @each `e8 ..`,            dword at +1
        //   push imm32         @VA 0x4a8f09, opcode `68` => dword at +1  (error string literal)
        let fields: &[(u64, usize)] = &[
            (0x4a8d2c, 2), // mov edi, [global]  (disp32)
            (0x4a8d41, 1),
            (0x4a8d55, 1),
            (0x4a8d77, 1),
            (0x4a8d81, 1),
            (0x4a8d90, 1),
            (0x4a8dae, 1),
            (0x4a8dda, 1),
            (0x4a8de7, 1),
            (0x4a8e0e, 1),
            (0x4a8e1b, 1),
            (0x4a8e33, 1),
            (0x4a8e47, 1),
            (0x4a8e51, 1),
            (0x4a8e60, 1),
            (0x4a8e78, 1),
            (0x4a8e88, 1),
            (0x4a8e98, 1),
            (0x4a8ea0, 1),
            (0x4a8eb2, 1),
            (0x4a8ec1, 1),
            (0x4a8ec9, 1),
            (0x4a8edb, 1),
            (0x4a8efb, 1),
            (0x4a8f0f, 1), // 24 direct calls
            (0x4a8f09, 1), // push imm32 (error string)
        ];
        for &(va, byteoff) in fields {
            let o = (va - 0x4a8d20) as usize + byteoff;
            mask(&mut am, o);
            mask(&mut bm, o);
        }
        assert_eq!(
            am, bm,
            "0x4a8d20 vs 0x4a8f20 not clone-equal after masking rel32/disp32/imm32"
        );
        // And confirm every listed `call` target resolves identically in both functions.
        for &(va, byteoff) in fields {
            if byteoff != 1 {
                continue;
            }
            // skip the push-imm32 at 0x4a8f09 (not a call); only E8 calls have matching targets.
            let opa = emu.read(va, 1)[0];
            if opa != 0xe8 {
                continue;
            }
            let rel_a = i32::from_le_bytes(emu.read(va + 1, 4).try_into().unwrap());
            let mini_va = va + 0x200; // +512: the corresponding insn in 0x4a8f20
            let rel_b = i32::from_le_bytes(emu.read(mini_va + 1, 4).try_into().unwrap());
            let tgt_a = (va as i64 + 5 + rel_a as i64) as u64;
            let tgt_b = (mini_va as i64 + 5 + rel_b as i64) as u64;
            assert_eq!(
                tgt_a, tgt_b,
                "call @ {va:#x} target differs from MINI clone"
            );
        }
    }

    let total = exercise_metric(&mut emu, ebp, esp0, &REP_BLOCKS, "WMAP");
    eprintln!(
        "luabind_4a8d20_arrow_scale_divide: {total} bit-exact (fmul[1024.0]×2 + fdivr×2); \
         0x4a8f20 MINI proven clone-equal (masked rel32/disp32/imm32 + matching call targets)"
    );
}

#[test]
fn luabind_4a8f20_mini_arrow_scale_divide() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80;
    let stack = emu.alloc(0x1000);
    let esp0 = stack + 0xf00;
    assert_eq!(emu.read_u32(SCALE1024_VA), SCALE1024_BITS, "1024.0 .rdata");

    // Diff the MINI clone's own two blocks (proves the inherited kernel at the clone's bytes).
    let total = exercise_metric(&mut emu, ebp, esp0, &MINI_BLOCKS, "MINI");
    // And the transcription alias equals the representative for the full domain it was diffed on.
    let mut s = 0x4a8f_2000u32;
    let mut alias = 0u32;
    for _ in 0..2000 {
        let value = rng(&mut s, -1.0e5, 1.0e5);
        let scale = rng(&mut s, 0.1, 4.0);
        assert_eq!(
            mini_arrow_scale_divide(value, scale).to_bits(),
            arrow_scale_divide(value, scale).to_bits(),
            "mini alias value={value} scale={scale}"
        );
        alias += 1;
    }
    eprintln!(
        "luabind_4a8f20_mini_arrow_scale_divide: {total} bit-exact at clone bytes + {alias} alias checks"
    );
}

// ===========================================================================================
// 0x4ad620 — clamp/threshold "set value" (`ret 8`). Whole-fn call; callees stubbed; read [0x845eac].
//   args: [ebp+8]=arg1 (unused by FP), [ebp+0xc]=arg2 (the FP input). We pass (arg, arg).
// ===========================================================================================
#[test]
fn glue_4a8_4ad620_set_value() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // Independence: read the binary's own threshold/zero constants.
    assert_eq!(emu.read_u32(EPS_VA), EPS_BITS, "2^-22 eps .rdata");
    assert_eq!(emu.read_u32(ZERO_VA), ZERO_BITS, "0.0 .rdata");

    // Stub the two delegated callees ONCE before any case.
    emu.stub(BUF_APPEND, 0, 1); // 0x419130, ret 4 => pop_args=1
    emu.stub(NOTIFY, 0, 0); // 0x703e50, ret 0 (fastcall) => pop_args=0

    // run the whole fn with chosen (arg, cur, busy); read back the new [0x845eac].
    let run = |emu: &mut Emu, arg: f32, cur: f32, busy: bool| -> f32 {
        emu.write_u32(VALUE_GLOBAL_VA, cur.to_bits());
        emu.write_u32(FLAG_A_VA, if busy { 1 } else { 0 });
        emu.write_u32(FLAG_B_VA, 0);
        emu.clear_calls();
        emu.call(0x4ad620, 0, 0, &[arg.to_bits(), arg.to_bits()]);
        emu.read_f32(VALUE_GLOBAL_VA)
    };

    let eps = f32::from_bits(EPS_BITS);
    let mut total = 0u32;
    // deterministic threshold probes: the eps boundary, the 0.0 boundary, arg==cur.
    let probes: [(f32, f32, bool); 16] = [
        (0.0, 5.0, false),                          // |arg|==0 < eps -> arg
        (eps, 5.0, false),                          // |arg|==eps (>=) -> threshold path
        (f32::from_bits(EPS_BITS - 1), 5.0, false), // just under eps -> arg
        (-eps, 5.0, false),                         // |−eps|==eps -> threshold
        (3.0, -2.0, false),                         // cur<0 -> arg
        (3.0, 0.0, false),                          // cur==0 (>=0), arg>=cur -> leave cur
        (3.0, 3.0, false),                          // arg==cur -> leave cur
        (2.0, 3.0, false),                          // arg<cur -> arg
        (4.0, 3.0, false),                          // arg>cur -> leave cur
        (-4.0, 3.0, false),                         // arg<cur (negative) -> arg
        (7.0, 7.0, true),                           // busy -> unchanged
        (0.0, 9.0, true),                           // busy with small arg -> unchanged
        (100.0, -100.0, false),
        (-0.0, 1.0, false), // -0.0: |−0|==0 < eps -> arg(-0.0)
        (1.0e9, 5.0, false),
        (-1.0e9, 5.0, false),
    ];
    for (arg, cur, busy) in probes {
        let bin = run(&mut emu, arg, cur, busy);
        let mine = set_value(arg, cur, busy);
        assert_eq!(
            mine.to_bits(),
            bin.to_bits(),
            "set_value arg={arg} cur={cur} busy={busy}"
        );
        total += 1;
    }
    // fuzz: wide arg/cur, never busy (busy path has no FP and is covered above).
    let mut s = 0x4ad6_2000u32;
    for _ in 0..6000 {
        let arg = rng(&mut s, -1.0e4, 1.0e4);
        let cur = rng(&mut s, -1.0e4, 1.0e4);
        let bin = run(&mut emu, arg, cur, false);
        let mine = set_value(arg, cur, false);
        assert_eq!(
            mine.to_bits(),
            bin.to_bits(),
            "set_value fuzz arg={arg} cur={cur}"
        );
        total += 1;
    }
    // tight band around eps to hammer the |arg|<eps boundary.
    for _ in 0..2000 {
        let arg = rng(&mut s, -1.0e-6, 1.0e-6);
        let cur = rng(&mut s, -2.0, 2.0);
        let bin = run(&mut emu, arg, cur, false);
        let mine = set_value(arg, cur, false);
        assert_eq!(
            mine.to_bits(),
            bin.to_bits(),
            "set_value eps-band arg={arg} cur={cur}"
        );
        total += 1;
    }
    eprintln!("glue_4a8_4ad620_set_value: {total} bit-exact (fabs + 3 fcomp threshold ladder)");
}

// ===========================================================================================
// 0x4ade60 — paired "step toward value" (`ret 4`). Whole-fn call; callees stubbed; read [0x845eac].
//   arg: [ebp+0x8]=step amount. cur seeded into [0x845eac].
// ===========================================================================================
#[test]
fn glue_4a8_4ade60_step_value() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(emu.read_u32(EPS_VA), EPS_BITS, "2^-22 eps .rdata");
    assert_eq!(emu.read_u32(ZERO_VA), ZERO_BITS, "0.0 .rdata");

    emu.stub(NOTIFY, 0, 0); // 0x703e50, ret 0 (fastcall) => pop_args=0

    let run = |emu: &mut Emu, cur: f32, arg: f32| -> f32 {
        emu.write_u32(VALUE_GLOBAL_VA, cur.to_bits());
        emu.clear_calls();
        emu.call(0x4ade60, 0, 0, &[arg.to_bits()]);
        emu.read_f32(VALUE_GLOBAL_VA)
    };

    let eps = f32::from_bits(EPS_BITS);
    let mut total = 0u32;
    // deterministic probes: cur<=0 (no step), (cur-arg) boundary at/above/below 0, |new|==eps notify.
    let probes: [(f32, f32); 14] = [
        (0.0, 1.0),          // cur==0 -> unchanged (0.0)
        (-3.0, 1.0),         // cur<0 -> unchanged
        (5.0, 2.0),          // cur>0, diff=3 -> 3.0
        (5.0, 5.0),          // diff==0 -> 0.0
        (5.0, 6.0),          // diff<0 -> 0.0
        (5.0, -2.0),         // diff=7 -> 7.0 (negative step grows)
        (3.0, 3.0 - eps),    // diff = eps -> stored; |new|>=eps so no notify
        (eps, eps),          // cur==eps>0; diff=0 -> 0.0
        (1.0e6, 1.0),        // diff huge
        (1.0e-6, 1.0e-7),    // small positive cur, diff tiny -> may be < eps (notify), value = diff
        (2.0, 2.0 - 1.0e-8), // diff ~ tiny positive -> stored
        (10.0, 9.0),         // diff=1 -> 1.0
        (0.5, 0.25),         // diff=0.25
        (1.0, 1.0),          // diff==0 -> 0.0
    ];
    for (cur, arg) in probes {
        let bin = run(&mut emu, cur, arg);
        let mine = step_value(cur, arg);
        assert_eq!(
            mine.to_bits(),
            bin.to_bits(),
            "step_value cur={cur} arg={arg}"
        );
        total += 1;
    }
    // fuzz: cur in a positive-and-negative band, arg wide.
    let mut s = 0x4ade_6000u32;
    for _ in 0..6000 {
        let cur = rng(&mut s, -10.0, 10.0);
        let arg = rng(&mut s, -10.0, 10.0);
        let bin = run(&mut emu, cur, arg);
        let mine = step_value(cur, arg);
        assert_eq!(
            mine.to_bits(),
            bin.to_bits(),
            "step_value fuzz cur={cur} arg={arg}"
        );
        total += 1;
    }
    // band where (cur-arg) crosses 0 tightly (cur>0).
    for _ in 0..2000 {
        let cur = rng(&mut s, 0.001, 5.0);
        let arg = cur + rng(&mut s, -1.0e-5, 1.0e-5);
        let bin = run(&mut emu, cur, arg);
        let mine = step_value(cur, arg);
        assert_eq!(
            mine.to_bits(),
            bin.to_bits(),
            "step_value cross-zero cur={cur} arg={arg}"
        );
        total += 1;
    }
    eprintln!("glue_4a8_4ade60_step_value: {total} bit-exact (fsub + fabs threshold ladder)");
}
