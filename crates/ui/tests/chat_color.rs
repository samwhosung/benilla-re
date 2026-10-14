//! The chat colour scripts and two number getters (`ChangeChatColor`, `GetChatWindowInfo`,
//! `GetItemQualityColor`, `GetXPExhaustion`) against the exe: each is run as the slice of
//! instructions that scales a value, with the Lua marshalling stubbed around it.
//!
//! The quality colour multiplies by the f64 1/255 (`0x804578`); the chat colours by the `f32` 1/255
//! (`0x8026c8`); `ChangeChatColor` first multiplies the script number by 255.0 (`0x806498`) and
//! truncates it, with no clamp and no 0.5.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::{channel_over_255, channel_over_255_f32, scaled_byte};
use benilla_re_ui::unit_values::xp_exhaustion;
use fpkit::{next, rngf};

const __FTOL: u64 = 0x40a2b0;

struct Frame {
    ebp: u64,
    esp: u64,
    edx: u64, // a mapped scratch pointer for GetXPExhaustion's [edx+8] factor
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    let edx = emu.alloc(0x40);
    Frame { ebp, esp, edx }
}

/// Row 0x48d3f0 GetXPExhaustion — slice 0x48d47e..0x48d48a:
///   fild qword [ebp-0x10]; sub esp,8; fmul dword [edx+0x8]; fstp qword [esp]  (halt at the call).
/// fmul @0x48d484 (×1). The product is the un-narrowed f64. We feed n (i64, hi-dword 0) at
/// [ebp-0x10] and the runtime factor f32 at [edx+8], read the f64 the `fstp qword` lands at
/// [esp_initial - 8] (the slice's `sub esp,8` runs before the fstp).
#[test]
fn luabind_48d3f0_xp_exhaustion() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, n: i64, factor: f32| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EDX, f.edx);
        // [ebp-0x10] = n (i64, low/high dwords); the binary forces the high dword to 0, but we
        // write the full i64 to be faithful to the qword fild.
        emu.write(f.ebp - 0x10, &n.to_le_bytes());
        emu.write_u32(f.edx + 0x8, factor.to_bits());
        emu.run(0x48d47e, 0x48d48a);
        // fstp qword [esp] landed at (initial esp) - 8 after the slice's `sub esp,8`.
        let bytes = emu.read(f.esp - 0x8, 8);
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    // deterministic: zero, small, a realistic rest-XP count, and a large value; factor spread
    // including the default 0.5 rest-rate and a couple of arbitrary runtime factors.
    // The count is an unsigned dword, so the probes include the values past i32::MAX.
    let ns = [
        0i64,
        1,
        100,
        12345,
        1_000_000,
        2_000_000_000,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
    ];
    let factors = [0.0f32, 0.5, 1.0, 0.125, 2.0, 1.5e3, -0.5];
    for &n in &ns {
        for &factor in &factors {
            assert_eq!(
                run(&mut emu, n, factor).to_bits(),
                xp_exhaustion(n, factor).to_bits(),
                "xp_exhaustion n={n} factor={factor}"
            );
            total += 1;
        }
    }
    let mut s = 0x48d3_f000u32;
    for _ in 0..4000 {
        let n = i64::from(next(&mut s) >> 8); // non-negative rest-XP count
        let factor = rngf(&mut s, -100.0, 100.0) as f32;
        assert_eq!(
            run(&mut emu, n, factor).to_bits(),
            xp_exhaustion(n, factor).to_bits(),
            "xp_exhaustion fuzz n={n} factor={factor}"
        );
        total += 1;
    }
    eprintln!("luabind_48d3f0_xp_exhaustion: {total} bit-exact (fild qword; fmul dword [edx+8]; fstp qword)");
}

/// Row 0x48dfb0 GetItemQualityColor — three identical channel-normalize blocks (B,G,R):
///   fild dword [ebp-4]; fmul qword [0x804578]=1/255(f64); fstp qword [esp].
/// fmul @0x48e007 / 0x48e024 / 0x48e040 (×3). Each diffed at its OWN address: feed the channel byte
/// (zero-extended) at [ebp-4], read the f64 at [esp].
#[test]
fn luabind_48dfb0_quality_color() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (fild_addr, end) for each of the three blocks; each is fild;fmul;fstp [esp].
    let blocks = [
        (0x48e004u64, 0x48e010u64),
        (0x48e021, 0x48e02d),
        (0x48e03d, 0x48e049),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x4, u32::from(byte)); // movzx-style zero-extended channel byte
        emu.run(start, end);
        let bytes = emu.read(f.esp, 8);
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    for (bi, &(start, end)) in blocks.iter().enumerate() {
        // the full channel-byte domain 0..=255
        for byte in 0u32..=255 {
            assert_eq!(
                run(&mut emu, start, end, byte as u8).to_bits(),
                channel_over_255(byte as u8).to_bits(),
                "channel_over_255 block#{bi} @ {start:#x} byte={byte}"
            );
            total += 1;
        }
    }
    eprintln!("luabind_48dfb0_quality_color: {total} bit-exact (3× fild dword; fmul qword [1/255 f64]; fstp qword)");
}

/// Row 0x4a07f0 ChangeChatColor — TWO owned FP regions:
///
/// (A) RGB quantize @0x4a0864 / 0x4a087e / 0x4a0898 (×3): `lua·255.0(f64); __ftol → byte`. The
///     multiplicand is the LIVE f64 `lua_tonumber` (0x6f3620) result on st0 — `run` clears the x87
///     stack, so we diff via a COPIED-BYTE ISLAND: `fld qword [in]` (seeds st0), the EXACT source
///     `fmul qword [0x806498]` bytes (byte-verified vs 0x4a0864), then a real `call __ftol`
///     (recomputed rel32) — the whole quantize-to-byte runs against the binary. The three source
///     fmuls are byte-identical (same `dc 0d 98 64 80 00`), so the island's fmul stands for all
///     three (verified equal to each source address).
///
/// (B) re-normalize @0x4a095a / 0x4a0967 / 0x4a0977 (×3): `fild dword [ebp-0xc]; fmul dword
///     [0x8026c8]=1/255(f32); fstp qword`. Integer byte input — diffed at each own address.
#[test]
fn luabind_4a07f0_change_chat_color() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // ---- (A) the ×255 → __ftol quantize island ------------------------------------------
    let in_cell = emu.alloc(8);
    let isl = emu.alloc(0x40);
    // fld qword [in_cell]  =  DD 05 <disp32>
    let mut code = vec![0xddu8, 0x05];
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    // copied source fmul qword [0x806498]  =  DC 0D 98 64 80 00
    let fmul_off = code.len() as u64;
    code.extend_from_slice(&[0xdc, 0x0d, 0x98, 0x64, 0x80, 0x00]);
    // call __ftol  =  E8 <rel32>, rel32 = target - (next_instr_va)
    let call_va = isl + code.len() as u64;
    let rel = (__FTOL as i64 - (call_va as i64 + 5)) as i32;
    code.push(0xe8);
    code.extend_from_slice(&rel.to_le_bytes());
    let isl_end = isl + code.len() as u64; // __ftol's `ret` lands here
    emu.write(isl, &code);
    // byte-verify the island's fmul == EACH source address (else the island proves nothing).
    let isl_fmul = isl + fmul_off;
    for src in [0x4a0864u64, 0x4a087e, 0x4a0898] {
        assert_eq!(
            emu.read(isl_fmul, 6),
            emu.read(src, 6),
            "ChangeChatColor island fmul != source @ {src:#x}"
        );
    }

    let run_quantize = |emu: &mut Emu, lua: f64| -> u8 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write(in_cell, &lua.to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        (emu.get_reg(RegisterX86::EAX) as i32) as u8 // __ftol result, low byte
    };

    let mut total = 0u32;
    // deterministic: in-range colour fractions plus the truncate pivots (x·255 just below/at int)
    // and out-of-range (no clamp in this path, so >1 / <0 wrap exactly as __ftol does).
    let probes = [
        0.0f64,
        1.0,
        0.5,
        1.0 / 255.0,          // ·255 = 1.0 → 1
        (1.0 / 255.0) - 1e-9, // ·255 just below 1 → 0
        2.0 / 255.0 - 1e-12,  // just below 2 → 1
        2.0 / 255.0,          // = 2 → 2
        0.999,
        1.5,
        -0.25,
        100.0,
        // NaN, the infinities and products past `i32` and `i64`: `__ftol` returns the low dword
        // of its qword, 0 for the indefinite.
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0e8,
        -1.0e8,
        4.0e16,
        1.0e300,
    ];
    for &lua in &probes {
        assert_eq!(
            run_quantize(&mut emu, lua),
            scaled_byte(lua),
            "scaled_byte lua={lua}"
        );
        total += 1;
    }
    let mut s = 0x4a07_f000u32;
    for _ in 0..4000 {
        let lua = rngf(&mut s, -2.0, 2.0);
        assert_eq!(
            run_quantize(&mut emu, lua),
            scaled_byte(lua),
            "scaled_byte fuzz lua={lua}"
        );
        total += 1;
    }

    // ---- (B) the byte · (1/255 f32) re-normalize, three source slices --------------------
    // (fild_addr, end, result_offset_from_esp)
    let renorm_blocks = [
        (0x4a0950u64, 0x4a0964u64, 0x10u64), // fstp qword [esp+0x10]
        (0x4a0964, 0x4a0974, 0x8),           // fstp qword [esp+0x8]
        (0x4a0974, 0x4a0980, 0x0),           // fstp qword [esp]
    ];
    let run_renorm = |emu: &mut Emu, start: u64, end: u64, off: u64, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        // these blocks interleave `mov [ebp-0xc], <reg>` AFTER the fild consumes [ebp-0xc], so the
        // byte fed at [ebp-0xc] is what this block's fild loads; eax/ecx/bl are set to avoid stray
        // writes to mapped memory affecting the read-back slot (they only re-seed [ebp-0xc]).
        emu.set_reg(RegisterX86::EAX, 0);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.set_reg(RegisterX86::EBX, 0);
        emu.write_u32(f.ebp - 0xc, u32::from(byte));
        emu.run(start, end);
        let bytes = emu.read(f.esp + off, 8);
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };
    for (bi, &(start, end, off)) in renorm_blocks.iter().enumerate() {
        for byte in 0u32..=255 {
            assert_eq!(
                run_renorm(&mut emu, start, end, off, byte as u8).to_bits(),
                channel_over_255_f32(byte as u8).to_bits(),
                "channel_over_255_f32 block#{bi} @ {start:#x} byte={byte}"
            );
            total += 1;
        }
    }
    eprintln!("luabind_4a07f0_change_chat_color: {total} bit-exact (3× ·255 island+__ftol; 3× fild;fmul[1/255 f32];fstp qword)");
}

/// Row 0x4a0ba0 GetChatWindowInfo — four colour-channel normalize blocks:
///   fild dword [ebp-4]; fmul dword [0x8026c8]=1/255(f32); fstp qword [esp].
/// fmul @0x4a0c32 / 0x4a0c52 / 0x4a0c72 / 0x4a0c92 (×4). Integer byte input at [ebp-4], f64 read at
/// [esp]. (The two `fild qword; fstp qword` int→number pushes at 0x4a0c13/0x4a0cf4 carry NO fmul —
/// not arithmetic — so they need no slice.)
#[test]
fn luabind_4a0ba0_chat_window_info() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let blocks = [
        (0x4a0c2fu64, 0x4a0c3bu64),
        (0x4a0c4f, 0x4a0c5b),
        (0x4a0c6f, 0x4a0c7b),
        (0x4a0c8f, 0x4a0c9b),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x4, u32::from(byte));
        emu.run(start, end);
        let bytes = emu.read(f.esp, 8);
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    for (bi, &(start, end)) in blocks.iter().enumerate() {
        for byte in 0u32..=255 {
            assert_eq!(
                run(&mut emu, start, end, byte as u8).to_bits(),
                channel_over_255_f32(byte as u8).to_bits(),
                "channel_over_255_f32 block#{bi} @ {start:#x} byte={byte}"
            );
            total += 1;
        }
    }
    eprintln!("luabind_4a0ba0_chat_window_info: {total} bit-exact (4× fild dword; fmul dword [1/255 f32]; fstp qword)");
}
