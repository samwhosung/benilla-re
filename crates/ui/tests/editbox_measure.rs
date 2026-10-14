//! Differential tests of the arithmetic of the edit-box measuring functions
//! `CSimpleEditBox::GetIndexAtPoint` (`0x77d0d0`), `0x77d4d0`, `0x77da80` and `0x77de70`, each
//! run against `WoW.exe` in the emulator.
//!
//! These four functions interleave their owned FP arithmetic with heavy foreign-state delegation
//! (thumb position getters, `__ftol`-style logical/device converters, per-piece pixel→logical
//! converters, layout setters) — so each owned op-group is diffed by the **owned-arithmetic-slice
//! / copied-byte-island** method: the op's VERBATIM binary bytes are assembled in a
//! scratch island behind a synthesized `fld`-prologue that establishes the x87 operands, followed
//! by a synthesized `fstp`/`fnstsw` capture. Every island BYTE-VERIFIES its copied op bytes against
//! the source address before running (an `e.read(VA,len)` + `assert_eq!` in-test) — else the island
//! is circular self-verification (blind spot). Self-contained `fld`-from-memory slices (the
//! clamp loop, the case reciprocals, the piece loops) run their own `.text` bytes verbatim via
//! `Emu::run` over a copied island with a rebased `ebp`/`esi`/`ecx` frame.
//!
//! Coverage  (every FP-arith op in
//! `[addr, true_ret)`):
//!   - 0x77d0d0 (17): head 3 · clamp 4 · case2 5 · tail 1 · caseA 3 · caseB 1
//!   - 0x77d4d0 (4):  scale 2 · tail 2
//!   - 0x77da80 (16): head 4 · loop 1 · neg 1 · half 2 · signal 8
//!   - 0x77de70 (9):  head 4 · loop 1 · neg 1 · half 2 · settle(fabs) 1
//!
//! Every test carries deterministic boundary probes: the clamp `acc == step` pivot, the
//! eps `|d| == eps` settle boundary, and exact-multiple scales. Fuzz reads the LCG HIGH bits.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::editbox_measure::{
    case2_recenter, case_a_recip, case_b_recip, clamp_loop, half_subr, half_subr_2, head_add,
    head_combine, multiline_height, neg, neg_2, piece_loop_sub, piece_loop_sub_2, rewindow_width,
    scale_leg, settled_lt_eps, signal_scale, tail_offset,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

fn load() -> Option<Wow> {
    Wow::load()
}

// ===========================================================================================
// Island infrastructure. An island executes a VERBATIM copy of an owned op's bytes behind a
// synthesized x87-stack prologue. The caller byte-verifies the copied op bytes vs their source VA.
// ===========================================================================================

/// One operand cell + scratch struct + out slot. `fld dword [cell_0]` (st0) … then `op_bytes`,
/// then `fstp dword [out]`. For multi-operand ops, additional operands live in the `esi` struct
/// (the op's modrm references `[esi+disp]`); `cell` is the single st0 input.
struct Island {
    entry: u64,
    end: u64,
    cell: u64,
    out: u64,
    esi: u64,
}

/// Assemble `fld dword [cell]; <op_bytes>; fstp dword [out]`. The op may also read `[esi+disp]`
/// operands the caller writes into the returned `esi` struct.
fn build_island(emu: &mut Emu, op_bytes: &[u8]) -> Island {
    let esi = emu.alloc(0x500);
    let out = emu.alloc(0x10);
    let cell = emu.alloc(0x10);
    let entry = emu.alloc(0x100);
    let mut code = vec![0xd9u8, 0x05]; // fld dword [cell]
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.extend_from_slice(op_bytes);
    code.push(0xd9); // fstp dword [out]
    code.push(0x1d);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = entry + code.len() as u64;
    emu.write(entry, &code);
    Island {
        entry,
        end,
        cell,
        out,
        esi,
    }
}

/// Byte-verify the island's copied op bytes against the source VA (else circular).
fn verify_bytes(emu: &Emu, src_va: u64, op_bytes: &[u8]) {
    let got = emu.read(src_va, op_bytes.len());
    assert_eq!(
        got, op_bytes,
        "island op bytes diverge from source @ {src_va:#x}"
    );
}

/// Run a single-st0-operand island: write `g` to the cell, set ESI, execute, return the f32.
fn run_island(emu: &mut Emu, isl: &Island, g: f32) -> f32 {
    emu.write_u32(isl.cell, g.to_bits());
    emu.set_reg(RegisterX86::ESI, isl.esi);
    emu.run(isl.entry, isl.end);
    emu.read_f32(isl.out)
}

// ===========================================================================================
// 0x77d0d0 head — fmul[esi+0x7c] (scale_leg), fadd (head_add). Copied-byte islands.
// ===========================================================================================
#[test]
fn glue_77d0d0_head() {
    let Some(wow) = load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // scale_leg: fmul dword [esi+0x7c] = D8 4E 7C @ 0x77d143.
    let mul_scale = [0xd8u8, 0x4e, 0x7c];
    verify_bytes(&emu, 0x77d143, &mul_scale);
    let isl_scale = build_island(&mut emu, &mul_scale);

    // head_add: the binary's `fadd dword [ebp-0x8]` (D8 45 F8 @ 0x77d161) — a D8 /0 mem-fadd. The
    // island uses the identical opcode with an [esi+0x10] operand (the modrm only picks the address;
    // the arithmetic — f64 add of two f32 reloads — is the same). Byte-verify the source is D8 /0.
    let src_fadd = emu.read(0x77d161, 3);
    assert_eq!(src_fadd[0], 0xd8, "0x77d161 head fadd is a D8-group op");
    assert_eq!(src_fadd[1] & 0x38, 0x00, "/0 = fadd reg field");
    let isl_add = build_island(&mut emu, &[0xd8, 0x46, 0x10]); // fadd dword [esi+0x10]

    let mut s = 0x77d0_d000u32;
    let mut total = 0u32;
    for (g, scale) in [
        (1.0f32, 1.0f32),
        (0.0, 5.0),
        (3.0, 0.5),
        (16_777_217.0, 1.0),
        (-2.5, 0.75),
    ] {
        emu.write_u32(isl_scale.esi + 0x7c, scale.to_bits());
        assert_eq!(
            run_island(&mut emu, &isl_scale, g).to_bits(),
            scale_leg(g, scale).to_bits(),
            "scale_leg g={g} scale={scale}"
        );
        total += 1;
    }
    for (c0, c1) in [(1.0f32, 2.0f32), (0.0, 0.0), (-3.5, 3.5), (1e7, 1.0)] {
        emu.write_u32(isl_add.esi + 0x10, c0.to_bits());
        assert_eq!(
            run_island(&mut emu, &isl_add, c1).to_bits(),
            head_add(c0, c1).to_bits(),
            "head_add c0={c0} c1={c1}"
        );
        total += 1;
    }
    for _ in 0..3000 {
        let g = rng(&mut s, -1000.0, 1000.0);
        let scale = rng(&mut s, 0.01, 4.0);
        emu.write_u32(isl_scale.esi + 0x7c, scale.to_bits());
        assert_eq!(
            run_island(&mut emu, &isl_scale, g).to_bits(),
            scale_leg(g, scale).to_bits(),
            "scale_leg fuzz"
        );
        let c0 = rng(&mut s, -1000.0, 1000.0);
        let c1 = rng(&mut s, -1000.0, 1000.0);
        emu.write_u32(isl_add.esi + 0x10, c0.to_bits());
        assert_eq!(
            run_island(&mut emu, &isl_add, c1).to_bits(),
            head_add(c0, c1).to_bits(),
            "head_add fuzz"
        );
        total += 2;
    }
    eprintln!("glue_77d0d0_head: {total} bit-exact (g·scale, c1+c0)");
}

// ===========================================================================================
// 0x77d0d0 clamp loop — copied .text slice 0x77d17c..0x77d1a6, rebased ebp frame, edi/ecx loop.
// ===========================================================================================
#[test]
fn glue_77d0d0_clamp() {
    let Some(wow) = load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100; // ebp; negative offsets valid

    let slice = emu.read(0x77d17c, 0x77d1a6 - 0x77d17c);
    assert_eq!(
        &slice[0..3],
        &[0xd9, 0x45, 0xf0],
        "clamp slice head fld[-0x10]"
    );
    let out = emu.alloc(0x10);
    let entry = emu.alloc(0x200);
    let mut code = slice.clone();
    code.push(0xd9); // fstp dword [out]
    code.push(0x1d);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = entry + code.len() as u64;
    emu.write(entry, &code);

    let run = |emu: &mut Emu, a10: f32, a18: f32, c0c: f32, step: f32, limit: i32| -> f32 {
        emu.write_u32(frame.wrapping_sub(0x10), a10.to_bits());
        emu.write_u32(frame.wrapping_sub(0x18), a18.to_bits());
        emu.write_u32(frame.wrapping_add(0x0c), c0c.to_bits());
        emu.write_u32(frame.wrapping_sub(0x04), step.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.set_reg(RegisterX86::ECX, limit as u64 & 0xffff_ffff);
        emu.run(entry, end);
        emu.read_f32(out)
    };

    let mut total = 0u32;
    for (a10, a18, c0c, step, limit) in [
        (100.0f32, 0.0f32, 0.0f32, 10.0f32, 100i32),
        (10.0, 0.0, 0.0, 10.0, 100), // acc==step exactly → exit, residual 10
        (25.0, 0.0, 0.0, 10.0, 1),   // limit caps at 1: 25 → 15
        (5.0, 0.0, 0.0, 10.0, 100),  // acc<step → no subtract
        (-30.0, 0.0, 0.0, 10.0, 100),
        (50.0, 5.0, 5.0, 7.0, 100),
    ] {
        let got = run(&mut emu, a10, a18, c0c, step, limit);
        let want = clamp_loop(a10, a18, c0c, step, limit) as f32;
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "clamp a10={a10} a18={a18} c0c={c0c} step={step} limit={limit}"
        );
        total += 1;
    }
    let mut s = 0x77d1_7c00u32;
    for _ in 0..3000 {
        let a10 = rng(&mut s, -200.0, 200.0);
        let a18 = rng(&mut s, -50.0, 50.0);
        let c0c = rng(&mut s, -200.0, 200.0);
        let step = {
            let x = rng(&mut s, 0.1, 30.0);
            if x < 0.1 {
                0.1
            } else {
                x
            }
        };
        let limit = ((next(&mut s) >> 8) % 8) as i32;
        let got = run(&mut emu, a10, a18, c0c, step, limit);
        let want = clamp_loop(a10, a18, c0c, step, limit) as f32;
        assert_eq!(got.to_bits(), want.to_bits(), "clamp fuzz");
        total += 1;
    }
    eprintln!("glue_77d0d0_clamp: {total} bit-exact (acc subtract-while-gt loop)");
}

// ===========================================================================================
// 0x77d0d0 case2 — copied slice 0x77d1d3..0x77d1f1 behind fld[g], with capture of [-0xc]/[-0x14].
// ===========================================================================================
#[test]
fn glue_77d0d0_case2() {
    let Some(wow) = load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let slice = emu.read(0x77d1d3, 0x77d1f1 - 0x77d1d3);
    assert_eq!(
        &slice[0..3],
        &[0xd9, 0x45, 0xf4],
        "case2 slice head fld[-0xc]"
    );
    let frame = emu.alloc(0x200) + 0x100;
    let g_cell = emu.alloc(0x10);
    let out_e = emu.alloc(0x10);
    let out_f = emu.alloc(0x10);
    let entry = emu.alloc(0x200);
    let mut code = vec![0xd9u8, 0x05]; // fld dword [g_cell]
    code.extend_from_slice(&(g_cell as u32).to_le_bytes());
    code.extend_from_slice(&slice); // leaves new_e in [ebp-0xc], new_f in [ebp-0x14]
    code.extend_from_slice(&[0xd9, 0x45, 0xf4, 0xd9, 0x1d]); // fld [ebp-0xc]; fstp [out_e]
    code.extend_from_slice(&(out_e as u32).to_le_bytes());
    code.extend_from_slice(&[0xd9, 0x45, 0xec, 0xd9, 0x1d]); // fld [ebp-0x14]; fstp [out_f]
    code.extend_from_slice(&(out_f as u32).to_le_bytes());
    let end = entry + code.len() as u64;
    emu.write(entry, &code);

    let run = |emu: &mut Emu, e: f32, f: f32, g: f32| -> (f32, f32) {
        emu.write_u32(frame.wrapping_sub(0x0c), e.to_bits());
        emu.write_u32(frame.wrapping_sub(0x14), f.to_bits());
        emu.write_u32(g_cell, g.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(entry, end);
        (emu.read_f32(out_e), emu.read_f32(out_f))
    };

    let mut total = 0u32;
    for (e, f, g) in [
        (10.0f32, 0.0f32, 0.0f32),
        (10.0, 2.0, 4.0),
        (0.0, 0.0, 0.0),
        (-5.0, 5.0, 3.0),
        (100.0, -100.0, 50.0),
    ] {
        let got = run(&mut emu, e, f, g);
        let want = case2_recenter(e, f, g);
        assert_eq!(
            (got.0.to_bits(), got.1.to_bits()),
            (want.0.to_bits(), want.1.to_bits()),
            "case2 e={e} f={f} g={g}"
        );
        total += 1;
    }
    let mut s = 0x77d1_d300u32;
    for _ in 0..3000 {
        let e = rng(&mut s, -500.0, 500.0);
        let f = rng(&mut s, -500.0, 500.0);
        let g = rng(&mut s, -500.0, 500.0);
        let got = run(&mut emu, e, f, g);
        let want = case2_recenter(e, f, g);
        assert_eq!(
            (got.0.to_bits(), got.1.to_bits()),
            (want.0.to_bits(), want.1.to_bits()),
            "case2 fuzz"
        );
        total += 1;
    }
    eprintln!("glue_77d0d0_case2: {total} bit-exact (recenter half = ((e-f)-g)·0.5)");
}

// ===========================================================================================
// 0x77d0d0 tail/caseA/caseB — copied .text slices, rebased ebp/ecx/esi frame.
// ===========================================================================================
#[test]
fn glue_77d0d0_tail_cases() {
    let Some(wow) = load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100;

    // tail_offset: fld[+8]; fsub[-0x14] (0x77d1f3..0x77d1fc), then fstp [out].
    let tail = emu.read(0x77d1f3, 0x77d1fc - 0x77d1f3);
    assert_eq!(&tail[0..3], &[0xd9, 0x45, 0x08], "tail head fld[+8]");
    assert_eq!(&tail[3..6], &[0xd8, 0x65, 0xec], "tail fsub[-0x14]");
    let out_t = emu.alloc(0x10);
    let entry_t = emu.alloc(0x100);
    let mut ct = tail.clone();
    ct.extend_from_slice(&[0xd9, 0x1d]);
    ct.extend_from_slice(&(out_t as u32).to_le_bytes());
    let end_t = entry_t + ct.len() as u64;
    emu.write(entry_t, &ct);

    // caseA: 0x77d256..0x77d278 (fld[-0xc]; fsub[-0x14]; fsub[+8]; fld[ecx+0x7c]; fxch; fdiv).
    let casea = emu.read(0x77d256, 0x77d278 - 0x77d256);
    assert_eq!(&casea[0..3], &[0xd9, 0x45, 0xf4], "caseA head fld[-0xc]");
    let out_a = emu.alloc(0x10);
    let entry_a = emu.alloc(0x100);
    let mut ca = casea.clone();
    ca.extend_from_slice(&[0xd9, 0x1d]);
    ca.extend_from_slice(&(out_a as u32).to_le_bytes());
    let end_a = entry_a + ca.len() as u64;
    emu.write(entry_a, &ca);

    // caseB: 0x77d2bf..0x77d2d0 (fld[ecx+0x7c]; fld[+8]; fdiv).
    let caseb = emu.read(0x77d2bf, 0x77d2d0 - 0x77d2bf);
    assert_eq!(
        &caseb[0..3],
        &[0xd9, 0x41, 0x7c],
        "caseB head fld[ecx+0x7c]"
    );
    let out_b = emu.alloc(0x10);
    let entry_b = emu.alloc(0x100);
    let mut cb = caseb.clone();
    cb.extend_from_slice(&[0xd9, 0x1d]);
    cb.extend_from_slice(&(out_b as u32).to_le_bytes());
    let end_b = entry_b + cb.len() as u64;
    emu.write(entry_b, &cb);

    let sc = emu.alloc(0x600); // [ecx+0x7c] scale; ecx/esi point here; [esi+0x380] read by int movs
    let stack = emu.alloc(0x400) + 0x200; // caseA/B slices `push` — give them a scratch ESP
    let mut total = 0u32;

    let run_tail = |emu: &mut Emu, a8: f32, f14: f32| -> f32 {
        emu.write_u32(frame.wrapping_add(8), a8.to_bits());
        emu.write_u32(frame.wrapping_sub(0x14), f14.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(entry_t, end_t);
        emu.read_f32(out_t)
    };
    for (a8, f14) in [
        (10.0f32, 0.0f32),
        (0.0, 0.0),
        (5.0, 5.0),
        (-3.0, 2.0),
        (1e7, 1.0),
    ] {
        assert_eq!(
            run_tail(&mut emu, a8, f14).to_bits(),
            tail_offset(a8, f14).to_bits(),
            "tail a8={a8} f14={f14}"
        );
        total += 1;
    }

    let run_a = |emu: &mut Emu, e: f32, f: f32, a8: f32, scale: f32| -> f32 {
        emu.write_u32(frame.wrapping_sub(0x0c), e.to_bits());
        emu.write_u32(frame.wrapping_sub(0x14), f.to_bits());
        emu.write_u32(frame.wrapping_add(8), a8.to_bits());
        emu.write_u32(sc + 0x7c, scale.to_bits());
        emu.write_u32(sc + 0x380, (sc + 0x400) as u32); // [esi+0x380] base ptr → safe buffer
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ECX, sc);
        emu.set_reg(RegisterX86::ESI, sc);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.set_reg(RegisterX86::ESP, stack);
        emu.run(entry_a, end_a);
        emu.read_f32(out_a)
    };
    for (e, f, a8, scale) in [
        (10.0f32, 0.0f32, 2.0f32, 4.0f32),
        (100.0, 50.0, 10.0, 2.0),
        (0.0, 0.0, 0.0, 1.0),
        (-5.0, 5.0, -3.0, 0.5),
    ] {
        assert_eq!(
            run_a(&mut emu, e, f, a8, scale).to_bits(),
            case_a_recip(e, f, a8, scale).to_bits(),
            "caseA e={e} f={f} a8={a8} sc={scale}"
        );
        total += 1;
    }

    let run_b = |emu: &mut Emu, a8: f32, scale: f32| -> f32 {
        emu.write_u32(frame.wrapping_add(8), a8.to_bits());
        emu.write_u32(sc + 0x7c, scale.to_bits());
        emu.write_u32(sc + 0x380, (sc + 0x400) as u32);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ECX, sc);
        emu.set_reg(RegisterX86::ESI, sc);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.set_reg(RegisterX86::ESP, stack);
        emu.run(entry_b, end_b);
        emu.read_f32(out_b)
    };
    for (a8, scale) in [
        (10.0f32, 4.0f32),
        (0.0, 1.0),
        (100.0, 2.0),
        (-3.0, 0.5),
        (1.0, 16_777_216.0),
    ] {
        assert_eq!(
            run_b(&mut emu, a8, scale).to_bits(),
            case_b_recip(a8, scale).to_bits(),
            "caseB a8={a8} sc={scale}"
        );
        total += 1;
    }

    let mut s = 0x77d1_f300u32;
    for _ in 0..2500 {
        let a8 = rng(&mut s, -1000.0, 1000.0);
        let f14 = rng(&mut s, -1000.0, 1000.0);
        let e = rng(&mut s, -500.0, 500.0);
        let f = rng(&mut s, -500.0, 500.0);
        let scale = {
            let x = rng(&mut s, 0.05, 8.0);
            if x < 0.05 {
                0.05
            } else {
                x
            }
        };
        assert_eq!(
            run_tail(&mut emu, a8, f14).to_bits(),
            tail_offset(a8, f14).to_bits(),
            "tail fuzz"
        );
        assert_eq!(
            run_a(&mut emu, e, f, a8, scale).to_bits(),
            case_a_recip(e, f, a8, scale).to_bits(),
            "caseA fuzz"
        );
        assert_eq!(
            run_b(&mut emu, a8, scale).to_bits(),
            case_b_recip(a8, scale).to_bits(),
            "caseB fuzz"
        );
        total += 3;
    }
    eprintln!("glue_77d0d0_tail_cases: {total} bit-exact (tail offset + caseA/B reciprocals)");
}

// ===========================================================================================
// 0x77d4d0 — re-window width (·0.25 / ·0.75) and the multi-line box height ((top+bottom)+g).
// ===========================================================================================
#[test]
fn glue_77d4d0() {
    let Some(wow) = load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let mul25 = [0xd8u8, 0x0d, 0xb0, 0x29, 0x80, 0x00]; // fmul [0x8029b0]=0.25 @ 0x77d7d7
    let mul75 = [0xd8u8, 0x0d, 0xcc, 0x12, 0x80, 0x00]; // fmul [0x8012cc]=0.75 @ 0x77d7e2
    verify_bytes(&emu, 0x77d7d7, &mul25);
    verify_bytes(&emu, 0x77d7e2, &mul75);
    let isl25 = build_island(&mut emu, &mul25);
    let isl75 = build_island(&mut emu, &mul75);

    // box height (0x77d893..0x77d8a8): fld[+0x3f0]; fadd[+0x3e8]; fadd st,st(1). Byte-verify.
    let src_e8 = emu.read(0x77d89c, 6); // fadd [ebx+0x3e8] = D8 83 E8 03 00 00
    assert_eq!(src_e8[0], 0xd8, "height fadd[+0x3e8] D8 group");
    assert_eq!(src_e8[1] & 0x38, 0x00, "/0 = fadd");
    let src_p = emu.read(0x77d8a6, 2); // fadd st,st(1) = D8 C1
    assert_eq!(&src_p[..], &[0xd8, 0xc1], "height fadd st,st(1)");

    let esi = emu.alloc(0x500);
    let g_cell = emu.alloc(0x10);
    let out = emu.alloc(0x10);
    let entry = emu.alloc(0x100);
    let mut code = vec![0xd9u8, 0x05]; // fld [g]
    code.extend_from_slice(&(g_cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd9, 0x86, 0xf0, 0x03, 0x00, 0x00]); // fld [esi+0x3f0]
    code.extend_from_slice(&[0xd8, 0x86, 0xe8, 0x03, 0x00, 0x00]); // fadd [esi+0x3e8]
    code.extend_from_slice(&[0xd8, 0xc1]); // fadd st,st(1)
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = entry + code.len() as u64;
    emu.write(entry, &code);
    let run_height = |emu: &mut Emu, a: f32, b: f32, g: f32| -> f32 {
        emu.write_u32(esi + 0x3f0, a.to_bits());
        emu.write_u32(esi + 0x3e8, b.to_bits());
        emu.write_u32(g_cell, g.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.run(entry, end);
        emu.read_f32(out)
    };

    let mut total = 0u32;
    for (v,) in [(4.0f32,), (0.0,), (-2.0,), (1e7,), (0.333_333_34,)] {
        assert_eq!(
            run_island(&mut emu, &isl25, v).to_bits(),
            rewindow_width(v, true).to_bits(),
            "·0.25 v={v}"
        );
        assert_eq!(
            run_island(&mut emu, &isl75, v).to_bits(),
            rewindow_width(v, false).to_bits(),
            "·0.75 v={v}"
        );
        total += 2;
    }
    for (a, b, g) in [
        (1.0f32, 2.0f32, 3.0f32),
        (0.0, 0.0, 0.0),
        (-5.0, 5.0, 10.0),
        (1e6, 1.0, 0.5),
    ] {
        assert_eq!(
            run_height(&mut emu, a, b, g).to_bits(),
            multiline_height(a, b, g).to_bits(),
            "height a={a} b={b} g={g}"
        );
        total += 1;
    }
    let mut s = 0x77d4_d000u32;
    for _ in 0..3000 {
        let v = rng(&mut s, -2000.0, 2000.0);
        assert_eq!(
            run_island(&mut emu, &isl25, v).to_bits(),
            rewindow_width(v, true).to_bits(),
            "·0.25 fuzz"
        );
        assert_eq!(
            run_island(&mut emu, &isl75, v).to_bits(),
            rewindow_width(v, false).to_bits(),
            "·0.75 fuzz"
        );
        let a = rng(&mut s, -1000.0, 1000.0);
        let b = rng(&mut s, -1000.0, 1000.0);
        let g = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run_height(&mut emu, a, b, g).to_bits(),
            multiline_height(a, b, g).to_bits(),
            "height fuzz"
        );
        total += 3;
    }
    eprintln!("glue_77d4d0: {total} bit-exact (·0.25/·0.75 re-window, (top+bottom)+g height)");
}

// ===========================================================================================
// 0x77da80 — head combine, piece loop, neg, half_subr, signal·1024.
// ===========================================================================================
#[test]
fn glue_77da80() {
    let Some(wow) = load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let mul_scale = [0xd8u8, 0x4e, 0x7c]; // fmul [esi+0x7c] @ 0x77daf0
    verify_bytes(&emu, 0x77daf0, &mul_scale);
    let isl_scale = build_island(&mut emu, &mul_scale);

    // head_combine: fadd[-0x14] (0x77db0e, D8 /0) + fdiv[esi+0x7c] (0x77db23, D8 76 7C).
    let src_add = emu.read(0x77db0e, 3);
    assert_eq!(src_add[0], 0xd8, "head fadd D8 group");
    assert_eq!(src_add[1] & 0x38, 0x00, "/0 = fadd");
    verify_bytes(&emu, 0x77db23, &[0xd8, 0x76, 0x7c]);
    let esi = emu.alloc(0x500);
    let c1_cell = emu.alloc(0x10);
    let out_hc = emu.alloc(0x10);
    let entry_hc = emu.alloc(0x100);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(c1_cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd8, 0x46, 0x10]); // fadd [esi+0x10] = c0
    code.extend_from_slice(&[0xd8, 0x76, 0x7c]); // fdiv [esi+0x7c] (verbatim from source)
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(out_hc as u32).to_le_bytes());
    let end_hc = entry_hc + code.len() as u64;
    emu.write(entry_hc, &code);

    // half_subr: fmul[0.5] (0x77dc65 verbatim) + fsubr[-0x4] (0x77dc78, D8 /5).
    let mul_half = [0xd8u8, 0x0d, 0x24, 0xfa, 0x7f, 0x00];
    verify_bytes(&emu, 0x77dc65, &mul_half);
    let src_fsubr = emu.read(0x77dc78, 3);
    assert_eq!(src_fsubr[0], 0xd8, "half fsubr D8 group");
    assert_eq!(src_fsubr[1] & 0x38, 0x28, "/5 = fsubr");
    let g_cell = emu.alloc(0x10);
    let out_hs = emu.alloc(0x10);
    let entry_hs = emu.alloc(0x100);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(g_cell as u32).to_le_bytes());
    code.extend_from_slice(&mul_half);
    code.extend_from_slice(&[0xd8, 0x6e, 0x10]); // fsubr [esi+0x10] = stored − g·0.5
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(out_hs as u32).to_le_bytes());
    let end_hs = entry_hs + code.len() as u64;
    emu.write(entry_hs, &code);

    // neg: fchs (0x77dbae, D9 E0).
    let fchs = [0xd9u8, 0xe0];
    verify_bytes(&emu, 0x77dbae, &fchs);
    let isl_neg = build_island(&mut emu, &fchs);

    // signal_scale: fmul[1024] (0x77dda5 verbatim) + fmul[-0x14] (0x77ddac, D8 /1).
    let mul1024 = [0xd8u8, 0x0d, 0x68, 0xfd, 0x7f, 0x00];
    verify_bytes(&emu, 0x77dda5, &mul1024);
    let src_mulv = emu.read(0x77ddac, 3);
    assert_eq!(src_mulv[0], 0xd8, "signal fmul[v] D8 group");
    assert_eq!(src_mulv[1] & 0x38, 0x08, "/1 = fmul");
    let sc_cell = emu.alloc(0x10);
    let out_sig = emu.alloc(0x10);
    let entry_sig = emu.alloc(0x100);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(sc_cell as u32).to_le_bytes());
    code.extend_from_slice(&mul1024);
    code.extend_from_slice(&[0xd8, 0x4e, 0x10]); // fmul [esi+0x10] = v
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(out_sig as u32).to_le_bytes());
    let end_sig = entry_sig + code.len() as u64;
    emu.write(entry_sig, &code);

    // piece loop — self-contained fld[-0x8] slice 0x77db38..0x77db52.
    let frame = emu.alloc(0x200) + 0x100;
    let loop_slice = emu.read(0x77db38, 0x77db52 - 0x77db38);
    assert_eq!(
        &loop_slice[0..3],
        &[0xd9, 0x45, 0xf8],
        "piece loop head fld[-0x8]"
    );
    let out_loop = emu.alloc(0x10);
    let entry_loop = emu.alloc(0x200);
    let mut cl = loop_slice.clone();
    cl.extend_from_slice(&[0xd9, 0x1d]);
    cl.extend_from_slice(&(out_loop as u32).to_le_bytes());
    let end_loop = entry_loop + cl.len() as u64;
    emu.write(entry_loop, &cl);
    let pbuf = emu.alloc(0x400);

    let mut total = 0u32;

    let run_loop = |emu: &mut Emu, start: f32, step: f32, n: i32| -> f32 {
        emu.write_u32(frame.wrapping_sub(0x08), start.to_bits());
        emu.write_u32(frame.wrapping_sub(0x14), step.to_bits());
        for k in 0..0x80u64 {
            emu.write_u32(pbuf + 4 * k, 0); // piece values 0 → edi>=0 continues to the eax bound
        }
        // counter is ebx (xor ebx,ebx before the slice); bound `cmp ebx,eax;jae exit`. The inner
        // `cmp edi,[ecx+4];jae continue` must hold each iter → set edi=max, [ecx+4]=0 → n iters.
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::EBX, 0);
        emu.set_reg(RegisterX86::EDI, 0xffff_ffff);
        emu.set_reg(RegisterX86::EAX, n as u64 & 0xffff_ffff);
        emu.set_reg(RegisterX86::ECX, pbuf);
        emu.run(entry_loop, end_loop);
        emu.read_f32(out_loop)
    };
    for (start, step, n) in [
        (100.0f32, 10.0f32, 5i32),
        (0.0, 1.0, 0),
        (50.0, 7.0, 3),
        (-20.0, 5.0, 4),
    ] {
        assert_eq!(
            run_loop(&mut emu, start, step, n).to_bits(),
            piece_loop_sub(start, step, n).to_bits(),
            "piece loop start={start} step={step} n={n}"
        );
        total += 1;
    }

    let run_hc = |emu: &mut Emu, c0: f32, c1: f32, scale: f32| -> f32 {
        emu.write_u32(esi + 0x10, c0.to_bits());
        emu.write_u32(esi + 0x7c, scale.to_bits());
        emu.write_u32(c1_cell, c1.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.run(entry_hc, end_hc);
        emu.read_f32(out_hc)
    };
    let run_hs = |emu: &mut Emu, stored: f32, g: f32| -> f32 {
        emu.write_u32(esi + 0x10, stored.to_bits());
        emu.write_u32(g_cell, g.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.run(entry_hs, end_hs);
        emu.read_f32(out_hs)
    };
    let run_sig = |emu: &mut Emu, sc: f32, v: f32| -> f32 {
        emu.write_u32(esi + 0x10, v.to_bits());
        emu.write_u32(sc_cell, sc.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.run(entry_sig, end_sig);
        emu.read_f32(out_sig)
    };

    for g in [1.0f32, 0.0, -2.0, 100.0, 0.5] {
        emu.write_u32(isl_scale.esi + 0x7c, 2.0f32.to_bits());
        assert_eq!(
            run_island(&mut emu, &isl_scale, g).to_bits(),
            scale_leg(g, 2.0).to_bits(),
            "scale_leg g={g}"
        );
        assert_eq!(
            run_island(&mut emu, &isl_neg, g).to_bits(),
            neg(g).to_bits(),
            "neg g={g}"
        );
        total += 2;
    }
    for (c0, c1, scale) in [
        (10.0f32, 20.0f32, 2.0f32),
        (0.0, 0.0, 1.0),
        (-5.0, 5.0, 0.5),
        (1e6, 1.0, 4.0),
    ] {
        assert_eq!(
            run_hc(&mut emu, c0, c1, scale).to_bits(),
            head_combine(c0, c1, scale).to_bits(),
            "head_combine c0={c0} c1={c1} scale={scale}"
        );
        total += 1;
    }
    for (stored, g) in [(10.0f32, 4.0f32), (0.0, 0.0), (-3.0, 6.0), (100.0, -50.0)] {
        assert_eq!(
            run_hs(&mut emu, stored, g).to_bits(),
            half_subr(stored, g).to_bits(),
            "half_subr stored={stored} g={g}"
        );
        total += 1;
    }
    for (sc, v) in [(1.0f32, 2.0f32), (0.0, 5.0), (0.001, 1000.0), (-2.0, 3.0)] {
        assert_eq!(
            run_sig(&mut emu, sc, v).to_bits(),
            signal_scale(sc, v).to_bits(),
            "signal_scale sc={sc} v={v}"
        );
        total += 1;
    }

    let mut s = 0x77da_8000u32;
    for _ in 0..3000 {
        let c0 = rng(&mut s, -1000.0, 1000.0);
        let c1 = rng(&mut s, -1000.0, 1000.0);
        let scale = {
            let x = rng(&mut s, 0.05, 8.0);
            if x < 0.05 {
                0.05
            } else {
                x
            }
        };
        let g = rng(&mut s, -1000.0, 1000.0);
        let stored = rng(&mut s, -1000.0, 1000.0);
        let sc = rng(&mut s, -4.0, 4.0);
        let v = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run_hc(&mut emu, c0, c1, scale).to_bits(),
            head_combine(c0, c1, scale).to_bits(),
            "head_combine fuzz"
        );
        assert_eq!(
            run_hs(&mut emu, stored, g).to_bits(),
            half_subr(stored, g).to_bits(),
            "half_subr fuzz"
        );
        assert_eq!(
            run_sig(&mut emu, sc, v).to_bits(),
            signal_scale(sc, v).to_bits(),
            "signal_scale fuzz"
        );
        emu.write_u32(isl_scale.esi + 0x7c, scale.to_bits());
        assert_eq!(
            run_island(&mut emu, &isl_scale, g).to_bits(),
            scale_leg(g, scale).to_bits(),
            "scale_leg fuzz"
        );
        assert_eq!(
            run_island(&mut emu, &isl_neg, g).to_bits(),
            neg(g).to_bits(),
            "neg fuzz"
        );
        total += 5;
    }
    eprintln!("glue_77da80: {total} bit-exact (head combine, loop, neg, half_subr, signal·1024)");
}

// ===========================================================================================
// 0x77de70 — head combine, piece loop, neg, half_subr, eps settle (fabs).
// ===========================================================================================
#[test]
fn glue_77de70() {
    let Some(wow) = load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let mul_scale = [0xd8u8, 0x4e, 0x7c]; // fmul [esi+0x7c] @ 0x77deb5
    verify_bytes(&emu, 0x77deb5, &mul_scale);
    let isl_scale = build_island(&mut emu, &mul_scale);

    // head_combine: fadd[-0x8] (0x77ded3, D8 /0) + fdiv[esi+0x7c] (0x77dee5, D8 76 7C).
    let src_add = emu.read(0x77ded3, 3);
    assert_eq!(src_add[0], 0xd8, "head fadd D8 group");
    assert_eq!(src_add[1] & 0x38, 0x00, "/0 = fadd");
    verify_bytes(&emu, 0x77dee5, &[0xd8, 0x76, 0x7c]);
    let esi = emu.alloc(0x500);
    let c1_cell = emu.alloc(0x10);
    let out_hc = emu.alloc(0x10);
    let entry_hc = emu.alloc(0x100);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(c1_cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd8, 0x46, 0x10]); // fadd [esi+0x10] = c0
    code.extend_from_slice(&[0xd8, 0x76, 0x7c]); // fdiv [esi+0x7c]
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(out_hc as u32).to_le_bytes());
    let end_hc = entry_hc + code.len() as u64;
    emu.write(entry_hc, &code);

    // half_subr_2: fmul[0.5] (0x77e06f verbatim) + fsubr[+0xc] (0x77e082, D8 /5).
    let mul_half = [0xd8u8, 0x0d, 0x24, 0xfa, 0x7f, 0x00];
    verify_bytes(&emu, 0x77e06f, &mul_half);
    let src_fsubr = emu.read(0x77e082, 3);
    assert_eq!(src_fsubr[0], 0xd8, "half fsubr D8 group");
    assert_eq!(src_fsubr[1] & 0x38, 0x28, "/5 = fsubr");
    let g_cell = emu.alloc(0x10);
    let out_hs = emu.alloc(0x10);
    let entry_hs = emu.alloc(0x100);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(g_cell as u32).to_le_bytes());
    code.extend_from_slice(&mul_half);
    code.extend_from_slice(&[0xd8, 0x6e, 0x10]); // fsubr [esi+0x10] = stored − g·0.5
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(out_hs as u32).to_le_bytes());
    let end_hs = entry_hs + code.len() as u64;
    emu.write(entry_hs, &code);

    // neg_2: fchs (0x77dfbd, D9 E0).
    let fchs = [0xd9u8, 0xe0];
    verify_bytes(&emu, 0x77dfbd, &fchs);
    let isl_neg = build_island(&mut emu, &fchs);

    // settle predicate: fld[d]; fabs (0x77e18c verbatim); fcomp eps (0x77e18e verbatim);
    // fnstsw ax; mov [out],eax. Read AH, reconstruct the `test ah,5; jnp` outcome.
    let fabs = [0xd9u8, 0xe1];
    verify_bytes(&emu, 0x77e18c, &fabs);
    let fcomp_eps = emu.read(0x77e18e, 6); // D8 1D D4 29 80 00 = fcomp [0x8029d4]=eps
    assert_eq!(fcomp_eps[0], 0xd8, "settle fcomp D8 group");
    let d_cell = emu.alloc(0x10);
    let out_ah = emu.alloc(0x10);
    let entry_ah = emu.alloc(0x100);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(d_cell as u32).to_le_bytes());
    code.extend_from_slice(&fabs); // fabs (verbatim)
    code.extend_from_slice(&fcomp_eps); // fcomp [0x8029d4] (verbatim)
    code.extend_from_slice(&[0xdf, 0xe0]); // fnstsw ax
    code.push(0xa3); // mov [out_ah], eax
    code.extend_from_slice(&(out_ah as u32).to_le_bytes());
    let end_ah = entry_ah + code.len() as u64;
    emu.write(entry_ah, &code);

    // piece loop (0x77def7..0x77df0f, fld[-0x4] led). slots [-0x4]/[-0x10]; bound edi<eax.
    let frame = emu.alloc(0x200) + 0x100;
    let loop_slice = emu.read(0x77def7, 0x77df0f - 0x77def7);
    assert_eq!(
        &loop_slice[0..3],
        &[0xd9, 0x45, 0xfc],
        "piece loop2 head fld[-0x4]"
    );
    let out_loop = emu.alloc(0x10);
    let entry_loop = emu.alloc(0x200);
    let mut cl = loop_slice.clone();
    cl.extend_from_slice(&[0xd9, 0x1d]);
    cl.extend_from_slice(&(out_loop as u32).to_le_bytes());
    let end_loop = entry_loop + cl.len() as u64;
    emu.write(entry_loop, &cl);
    let pbuf = emu.alloc(0x400);

    let mut total = 0u32;

    let run_hc = |emu: &mut Emu, c0: f32, c1: f32, scale: f32| -> f32 {
        emu.write_u32(esi + 0x10, c0.to_bits());
        emu.write_u32(esi + 0x7c, scale.to_bits());
        emu.write_u32(c1_cell, c1.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.run(entry_hc, end_hc);
        emu.read_f32(out_hc)
    };
    let run_hs = |emu: &mut Emu, stored: f32, g: f32| -> f32 {
        emu.write_u32(esi + 0x10, stored.to_bits());
        emu.write_u32(g_cell, g.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.run(entry_hs, end_hs);
        emu.read_f32(out_hs)
    };
    let run_settle = |emu: &mut Emu, d: f32| -> bool {
        emu.write_u32(d_cell, d.to_bits());
        emu.run(entry_ah, end_ah);
        let ah = (emu.read_u32(out_ah) >> 8) as u8;
        // test ah,5 → masked = ah & 5; jnp taken (settled) iff PF=0 iff odd popcount of masked.
        (ah & 5).count_ones() % 2 == 1
    };
    let run_loop = |emu: &mut Emu, start: f32, step: f32, n: i32| -> f32 {
        emu.write_u32(frame.wrapping_sub(0x04), start.to_bits());
        emu.write_u32(frame.wrapping_sub(0x10), step.to_bits());
        for k in 0..0x80u64 {
            emu.write_u32(pbuf + 4 * k, 0);
        }
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.set_reg(RegisterX86::EAX, n as u64 & 0xffff_ffff);
        emu.set_reg(RegisterX86::EBX, pbuf);
        emu.set_reg(RegisterX86::ECX, pbuf + 4); // the slice reads [ecx]; ecx = ebx+4 (set pre-slice)
        emu.set_reg(RegisterX86::EDX, 0xffff_ffff); // cmp edx,[ecx]: edx large → continue to eax bound
        emu.run(entry_loop, end_loop);
        emu.read_f32(out_loop)
    };

    for (c0, c1, scale) in [
        (10.0f32, 20.0f32, 2.0f32),
        (0.0, 0.0, 1.0),
        (-5.0, 5.0, 0.5),
        (1e6, 1.0, 4.0),
    ] {
        assert_eq!(
            run_hc(&mut emu, c0, c1, scale).to_bits(),
            head_combine(c0, c1, scale).to_bits(),
            "head_combine c0={c0}"
        );
        total += 1;
    }
    for (stored, g) in [(10.0f32, 4.0f32), (0.0, 0.0), (-3.0, 6.0), (100.0, -50.0)] {
        assert_eq!(
            run_hs(&mut emu, stored, g).to_bits(),
            half_subr_2(stored, g).to_bits(),
            "half_subr_2 stored={stored}"
        );
        total += 1;
    }
    let eps = f32::from_bits(0x3480_0000);
    for d in [
        0.0f32,
        eps,       // |d| == eps exactly — boundary probe
        eps * 0.5, // below
        eps * 2.0, // above
        -eps,      // negative, |d|==eps
        1.0,
        -1.0,
        f32::from_bits(0x3480_0001), // just above eps
        f32::from_bits(0x347f_ffff), // just below eps
    ] {
        assert_eq!(
            run_settle(&mut emu, d),
            settled_lt_eps(d),
            "settle d={d} (bits {:#x})",
            d.to_bits()
        );
        total += 1;
    }
    for (start, step, n) in [
        (100.0f32, 10.0f32, 5i32),
        (0.0, 1.0, 0),
        (50.0, 7.0, 3),
        (-20.0, 5.0, 4),
    ] {
        assert_eq!(
            run_loop(&mut emu, start, step, n).to_bits(),
            piece_loop_sub_2(start, step, n).to_bits(),
            "piece loop2 start={start} step={step} n={n}"
        );
        total += 1;
    }
    for g in [1.0f32, 0.0, -2.0, 100.0, 0.5] {
        emu.write_u32(isl_scale.esi + 0x7c, 2.0f32.to_bits());
        assert_eq!(
            run_island(&mut emu, &isl_scale, g).to_bits(),
            scale_leg(g, 2.0).to_bits(),
            "scale_leg g={g}"
        );
        assert_eq!(
            run_island(&mut emu, &isl_neg, g).to_bits(),
            neg_2(g).to_bits(),
            "neg_2 g={g}"
        );
        total += 2;
    }

    let mut s = 0x77de_7000u32;
    for _ in 0..3000 {
        let c0 = rng(&mut s, -1000.0, 1000.0);
        let c1 = rng(&mut s, -1000.0, 1000.0);
        let scale = {
            let x = rng(&mut s, 0.05, 8.0);
            if x < 0.05 {
                0.05
            } else {
                x
            }
        };
        let g = rng(&mut s, -1000.0, 1000.0);
        let stored = rng(&mut s, -1000.0, 1000.0);
        let d = rng(&mut s, -1e-5, 1e-5);
        assert_eq!(
            run_hc(&mut emu, c0, c1, scale).to_bits(),
            head_combine(c0, c1, scale).to_bits(),
            "head_combine fuzz"
        );
        assert_eq!(
            run_hs(&mut emu, stored, g).to_bits(),
            half_subr_2(stored, g).to_bits(),
            "half_subr_2 fuzz"
        );
        assert_eq!(
            run_settle(&mut emu, d),
            settled_lt_eps(d),
            "settle fuzz d={d}"
        );
        emu.write_u32(isl_scale.esi + 0x7c, scale.to_bits());
        assert_eq!(
            run_island(&mut emu, &isl_scale, g).to_bits(),
            scale_leg(g, scale).to_bits(),
            "scale_leg fuzz"
        );
        assert_eq!(
            run_island(&mut emu, &isl_neg, g).to_bits(),
            neg_2(g).to_bits(),
            "neg_2 fuzz"
        );
        total += 5;
    }
    eprintln!("glue_77de70: {total} bit-exact (head combine, loop, neg, half_subr, eps settle)");
}
