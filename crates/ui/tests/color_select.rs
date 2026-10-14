//! The colour select and status bar colour scripts (`SetColorRGB`, `GetColorRGB`,
//! `GetStatusBarColor`, and the status bar setter that shares the quantize) against the exe: each is
//! run as the slice of instructions that computes its bytes or channels, with the Lua marshalling
//! stubbed around it.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::{
    channel_over_255, normalize_channel, normalize_channel_f32, quantize_byte,
};
use fpkit::rngf;

const __FTOL: u64 = 0x40a2b0;

struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0xc0; // room for [ebp-0xc0 .. ebp]
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { ebp, esp }
}

/// The clamp->`·255 + 0.5`->`__ftol` color-byte quantize, shared by SetColorRGB's 4 legs
/// (fmul @0x78ec7d/0x78ec91/0x78eca9/0x78ecc2, fadd @0x78ec83/0x78ec9b/0x78ecb1/0x78eccc) and
/// SetStatusBarColor's 4 legs (fmul @0x78fdbd/0x78fdd1/0x78fde9/0x78fe02, fadd @0x78fdc3/0x78fddb/
/// 0x78fdf1/0x78fe0c). The multiplicand is the LIVE clamped f64 on st0 (`run` clears the x87 stack)
/// so we diff via a COPIED-BYTE ISLAND: `fld qword [in]` seeds st0, the EXACT source
/// `fmul dword [255.0]` + `fadd dword [0.5]` bytes follow (byte-verified == ALL 16 source sites),
/// then a real `call __ftol` runs the truncate.
#[test]
fn luabind_78eae0_setcolor_78fc20_setstatus_quantize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let in_cell = emu.alloc(8);
    let isl = emu.alloc(0x40);
    // fld qword [in_cell]  =  DD 05 <disp32>
    let mut code = vec![0xddu8, 0x05];
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    // copied source fmul dword [0x7ffe58]=255.0  =  D8 0D 58 FE 7F 00
    let fmul_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x0d, 0x58, 0xfe, 0x7f, 0x00]);
    // copied source fadd dword [0x7ffa24]=0.5     =  D8 05 24 FA 7F 00
    let fadd_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x05, 0x24, 0xfa, 0x7f, 0x00]);
    // call __ftol  =  E8 <rel32>, rel32 = target - (next_instr_va)
    let call_va = isl + code.len() as u64;
    let rel = (__FTOL as i64 - (call_va as i64 + 5)) as i32;
    code.push(0xe8);
    code.extend_from_slice(&rel.to_le_bytes());
    let isl_end = isl + code.len() as u64; // __ftol's `ret` lands here
    emu.write(isl, &code);
    // byte-verify the island's fmul/fadd == EVERY source site (else the island proves nothing).
    let isl_fmul = isl + fmul_off;
    let isl_fadd = isl + fadd_off;
    for src in [
        0x78ec7du64,
        0x78ec91,
        0x78eca9,
        0x78ecc2, // SetColorRGB fmul[255]
        0x78fdbd,
        0x78fdd1,
        0x78fde9,
        0x78fe02, // SetStatusBarColor fmul[255]
    ] {
        assert_eq!(
            emu.read(isl_fmul, 6),
            emu.read(src, 6),
            "quantize island fmul != source @ {src:#x}"
        );
    }
    for src in [
        0x78ec83u64,
        0x78ec9b,
        0x78ecb1,
        0x78eccc, // SetColorRGB fadd[0.5]
        0x78fdc3,
        0x78fddb,
        0x78fdf1,
        0x78fe0c, // SetStatusBarColor fadd[0.5]
    ] {
        assert_eq!(
            emu.read(isl_fadd, 6),
            emu.read(src, 6),
            "quantize island fadd != source @ {src:#x}"
        );
    }

    let run = |emu: &mut Emu, v: f64| -> u8 {
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write(in_cell, &v.to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        (emu.get_reg(RegisterX86::EAX) as i32) as u8 // __ftol result, low byte
    };

    let mut total = 0u32;
    // deterministic: the clamped [0,1] domain plus the __ftol truncate pivots (v·255+0.5 just
    // below/at an integer). The runtime only ever feeds clamped [0,1] (or the 0.0/1.0 constants),
    // but the arithmetic is domain-agnostic — probe the pivots exactly.
    let probes = [
        0.0f64,
        1.0,
        0.5,
        0.5 / 255.0,         // ·255 = 0.5, +0.5 = 1.0 -> 1
        0.5 / 255.0 - 1e-12, // just below -> +0.5 just below 1.0 -> 0
        1.5 / 255.0,         // -> 2.0 -> 2
        1.5 / 255.0 - 1e-12, // -> just below 2.0 -> 1
        254.5 / 255.0,       // -> 255.0 -> 255
        1.0 / 255.0,
        128.0 / 255.0,
        0.999,
        0.001,
    ];
    for &v in &probes {
        assert_eq!(run(&mut emu, v), quantize_byte(v), "quantize_byte v={v}");
        total += 1;
    }
    // dense sweep of the clamped domain: every byte target v = k/255 and a touch above/below.
    for k in 0u32..=255 {
        for d in [-1e-9, 0.0, 1e-9] {
            let v = f64::from(k) / 255.0 + d;
            assert_eq!(
                run(&mut emu, v),
                quantize_byte(v),
                "quantize_byte v={v} (k={k} d={d})"
            );
            total += 1;
        }
    }
    let mut s = 0x78ee_6000u32;
    for _ in 0..4000 {
        let v = rngf(&mut s, 0.0, 1.0); // the clamped runtime domain
        assert_eq!(
            run(&mut emu, v),
            quantize_byte(v),
            "quantize_byte fuzz v={v}"
        );
        total += 1;
    }
    eprintln!(
        "luabind_78eae0_setcolor_78fc20_setstatus_quantize: {total} bit-exact (island: fmul[255];fadd[0.5];__ftol == all 16 sites)"
    );
}

/// Row 0x78ed10 GetColorRGB — three identical channel-normalize blocks, each:
///   sub esp,8; mov ecx,esi; fild dword [ebp-8]; fmul qword [0x804578]=1/255(f64); fstp qword [esp].
/// fmul @0x78edfa / 0x78ee17 / 0x78ee34 (×3). Slice each fild..fstp (halt at the lua_pushnumber call):
///   start 0x78edf7 / 0x78ee14 / 0x78ee31, end 0x78ee03 / 0x78ee20 / 0x78ee3d (the call).
/// Feed the channel byte (zero-extended into the [ebp-8] dword); read the f64 at [esp].
#[test]
fn luabind_78ed10_get_color_rgb() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (fild_addr, call_addr) for each block; the slice runs fild; fmul qword; fstp qword [esp].
    let blocks = [
        (0x78edf7u64, 0x78ee03u64),
        (0x78ee14, 0x78ee20),
        (0x78ee31, 0x78ee3d),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        // [ebp-8] = byte (dword, zero-extended movzx of the channel byte).
        emu.write_u32(f.ebp - 0x8, u32::from(byte));
        emu.run(start, end);
        let bytes = emu.read(f.esp, 8); // fstp qword [esp]
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    for (bi, &(start, end)) in blocks.iter().enumerate() {
        for byte in 0u32..=255 {
            assert_eq!(
                run(&mut emu, start, end, byte as u8).to_bits(),
                channel_over_255(byte as u8).to_bits(),
                "channel_over_255 block#{bi} @ {start:#x} byte={byte}"
            );
            total += 1;
        }
    }
    eprintln!(
        "luabind_78ed10_get_color_rgb: {total} bit-exact (3x fild dword; fmul qword [1/255 f64]; fstp qword)"
    );
}

/// Row 0x78fab0 GetStatusBarColor — four channel-normalize blocks `fild qword; fmul dword [k_fog]`,
/// with the store-width split between the f32 stores and the f64 push:
///   * channel A24 (>>0x18):       fild qword [ebp-0x18] @0x78fb74; fmul @0x78fb82; fstp DWORD [ebp-0x14] @0x78fb90 (f32)
///   * channel B8  (>>8 & 0xff):   fild qword [ebp-0x10] @0x78fb99; fmul @0x78fba5; fstp DWORD [ebp-0x8]  @0x78fbb6 (f32)
///   * channel C0  (& 0xff):       fild qword [ebp-0x10] @0x78fbbc; fmul @0x78fbc1; fstp DWORD [ebp-0xc]  @0x78fbc7 (f32)
///   * channel D16 (>>0x10 & 0xff):fild qword [ebp-0x20] @0x78fbca; fmul @0x78fbcd; fstp QWORD [esp]      @0x78fbd3 (f64)
///
/// The four multiplies are at 0x78fb82, 0x78fba5, 0x78fbc1 and 0x78fbcd. Each slice feeds its qword
/// slot (low dword the byte, high dword 0) and reads the store (f32 for A, B and C; f64 at [esp] for
/// D). The shifts, masks and moves interleaved in the slices touch only other registers and slots and
/// do not change the diffed value.
#[test]
fn luabind_78fab0_get_status_bar_color() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // --- channel A24: slice 0x78fb74..0x78fb93 (fild qword [ebp-0x18] .. just after fstp dword [ebp-0x14]).
    //     Reads f32 at [ebp-0x14]. (f32-roundtrip channel.)
    let run_a24 = |emu: &mut Emu, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x18, u32::from(byte)); // qword lo
        emu.write_u32(f.ebp - 0x14, 0); // qword hi
        emu.run(0x78fb74, 0x78fb93);
        f64::from(emu.read_f32(f.ebp - 0x14)) // fstp dword (f32), reloaded+pushed widened to f64
    };
    // --- channel B8: slice 0x78fb99..0x78fbb9 (fild qword [ebp-0x10] .. just after fstp dword [ebp-0x8]).
    let run_b8 = |emu: &mut Emu, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x10, u32::from(byte)); // qword lo
        emu.write_u32(f.ebp - 0xc, 0); // qword hi
        emu.run(0x78fb99, 0x78fbb9);
        f64::from(emu.read_f32(f.ebp - 0x8))
    };
    // --- channel C0: slice 0x78fbbc..0x78fbca (fild qword [ebp-0x10] .. just after fstp dword [ebp-0xc]).
    let run_c0 = |emu: &mut Emu, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x10, u32::from(byte)); // qword lo
        emu.write_u32(f.ebp - 0xc, 0); // qword hi
        emu.run(0x78fbbc, 0x78fbca);
        f64::from(emu.read_f32(f.ebp - 0xc))
    };
    // --- channel D16: slice 0x78fbca..0x78fbd6 (fild qword [ebp-0x20] .. fstp qword [esp]; halt at the call).
    //     Reads the un-narrowed f64 at [esp]. (f64 channel.)
    let run_d16 = |emu: &mut Emu, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x20, u32::from(byte)); // qword lo
        emu.write_u32(f.ebp - 0x1c, 0); // qword hi
        emu.run(0x78fbca, 0x78fbd6);
        let bytes = emu.read(f.esp, 8); // fstp qword [esp]
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    for byte in 0u32..=255 {
        let b = byte as u8;
        // A/B/C are the f32-roundtrip channels; D16 is the un-narrowed f64 channel.
        assert_eq!(
            run_a24(&mut emu, b).to_bits(),
            normalize_channel_f32(b).to_bits(),
            "normalize_channel_f32 (A24 @0x78fb82) byte={byte}"
        );
        assert_eq!(
            run_b8(&mut emu, b).to_bits(),
            normalize_channel_f32(b).to_bits(),
            "normalize_channel_f32 (B8 @0x78fba5) byte={byte}"
        );
        assert_eq!(
            run_c0(&mut emu, b).to_bits(),
            normalize_channel_f32(b).to_bits(),
            "normalize_channel_f32 (C0 @0x78fbc1) byte={byte}"
        );
        assert_eq!(
            run_d16(&mut emu, b).to_bits(),
            normalize_channel(b).to_bits(),
            "normalize_channel (D16 @0x78fbcd) byte={byte}"
        );
        total += 4;
    }
    eprintln!(
        "luabind_78fab0_get_status_bar_color: {total} bit-exact (4x fild qword; fmul dword [k_fog f32]; 3x fstp dword f32 + 1x fstp qword f64)"
    );
}
