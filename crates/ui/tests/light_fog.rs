//! The model light and fog scripts (`SetLight`, `SetFogColor`, `GetFogColor`) and `Frame::GetAlpha`
//! against the exe: each is run as the slice of instructions that computes its colour bytes or
//! channels, with the Lua marshalling stubbed around it.
//!
//! The setters clamp, multiply by 255.0 and add 0.5 before `__ftol`; the fog getter multiplies by
//! the float at `0x81c6b4` and `GetAlpha` by the f64 1/255.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::{
    channel_over_255, intensity_abs, light_color_block, normalize_channel, quantize_byte,
};
use fpkit::{next, rngf};

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

/// Row 0x774dc0 GetAlpha — slice 0x774e72..0x774e7e:
///   fild dword [ebp-4]; fmul qword [0x804578]=1/255(f64); fstp qword [esp]  (halt at the call).
/// fmul @0x774e75 (×1). Feed the alpha byte (zero-extended) at [ebp-4], read the f64 at [esp].
#[test]
fn luabind_774dc0_get_alpha() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x4, u32::from(byte)); // movzx-style zero-extended alpha byte
        emu.run(0x774e72, 0x774e7e);
        let bytes = emu.read(f.esp, 8); // fstp qword [esp]
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    for byte in 0u32..=255 {
        assert_eq!(
            run(&mut emu, byte as u8).to_bits(),
            channel_over_255(byte as u8).to_bits(),
            "channel_over_255 byte={byte}"
        );
        total += 1;
    }
    eprintln!(
        "luabind_774dc0_get_alpha: {total} bit-exact (fild dword; fmul qword [1/255 f64]; fstp qword)"
    );
}

/// Row 0x76f080 GetFogColor — four identical channel-normalize blocks, each:
///   fild qword [ebp-8]; sub esp,8; fmul dword [0x81c6b4]=k_fog(f32); fstp qword [esp].
/// fmul @0x76f14b / 0x76f175 / 0x76f19d / 0x76f1c0 (×4). Feed the channel byte (qword, hi-dword 0)
/// at [ebp-8]; the slice's `sub esp,8` runs before the fstp, so the f64 lands at [esp_initial - 8].
#[test]
fn luabind_76f080_get_fog_color() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (fild_addr, end) for each block; each is fild qword; sub esp,8; mov ecx,esi; fmul; fstp [esp].
    let blocks = [
        (0x76f143u64, 0x76f154u64),
        (0x76f16d, 0x76f17e),
        (0x76f195, 0x76f1a6),
        (0x76f1b8, 0x76f1c9),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, 0); // ecx<-esi is set inside; value irrelevant (no deref pre-fstp)
                                          // [ebp-8] = byte (qword: low dword = byte, high dword forced 0 as the source does).
        emu.write_u32(f.ebp - 0x8, u32::from(byte));
        emu.write_u32(f.ebp - 0x4, 0);
        emu.run(start, end);
        let bytes = emu.read(f.esp - 0x8, 8); // fstp qword [esp] after the `sub esp,8`
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    for (bi, &(start, end)) in blocks.iter().enumerate() {
        for byte in 0u32..=255 {
            assert_eq!(
                run(&mut emu, start, end, byte as u8).to_bits(),
                normalize_channel(byte as u8).to_bits(),
                "normalize_channel block#{bi} @ {start:#x} byte={byte}"
            );
            total += 1;
        }
    }
    eprintln!(
        "luabind_76f080_get_fog_color: {total} bit-exact (4x fild qword; fmul dword [k_fog f32]; fstp qword)"
    );
}

/// The clamp->`·255 + 0.5`->`__ftol` color-byte quantize, shared by SetFogColor's 4 legs and
/// SetLight's 3 legs (7 fmul[255] @0x76eff6/0x76f00a/0x76f022/0x76f03b + 0x76e6ba/0x76e6ce/0x76e6ec,
/// 7 fadd[0.5] @0x76effc/0x76f014/0x76f02a/0x76f045 + 0x76e6c0/0x76e6d8/0x76e6f6). The multiplicand
/// is the LIVE clamped f64 on st0 (`run` clears the x87 stack) so we diff via a COPIED-BYTE ISLAND:
/// `fld qword [in]` seeds st0, the EXACT source `fmul dword [255.0]` + `fadd dword [0.5]` bytes
/// follow (byte-verified == ALL 14 source sites), then a real `call __ftol` runs the truncate.
#[test]
fn luabind_76ee60_setfog_76e1e0_setlight_quantize() {
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
        0x76eff6u64,
        0x76f00a,
        0x76f022,
        0x76f03b,
        0x76e6ba,
        0x76e6ce,
        0x76e6ec,
    ] {
        assert_eq!(
            emu.read(isl_fmul, 6),
            emu.read(src, 6),
            "quantize island fmul != source @ {src:#x}"
        );
    }
    for src in [
        0x76effcu64,
        0x76f014,
        0x76f02a,
        0x76f045,
        0x76e6c0,
        0x76e6d8,
        0x76e6f6,
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
    let mut s = 0x76ee_6000u32;
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
        "luabind_76ee60_setfog_76e1e0_setlight_quantize: {total} bit-exact (island: fmul[255];fadd[0.5];__ftol == all 14 sites)"
    );
}

/// Row 0x76e1e0 SetLight — two FP regions:
///
/// (A) intensity `fabs` ×2 (0x76e41f, 0x76e5bf): `fld [ebp-0xc]; fabs; fcomp [eps]`. The fcomp is a
///     compare (excluded); the fabs is the arithmetic. Diffed via a copied-byte island
///     `fld qword [in]; <source d9 e1>; fstp qword [out]`, the `d9 e1` byte-verified == both sites.
///
/// (B) color-normalize blocks ×2 (0x76e53d.. / 0x76e70f..): each `3x (fild;fmul[1/255 f32])` then
///     `fxch` + `3x fmul[ebp-0xc]=intensity` with one product reloaded-as-f32 (the store-width
///     asymmetry between the live and the reloaded product). 6 fmul[1/255] + 6 fmul[intensity]. Own-address slices, feeding the channel bytes
///     (block1 via the [ebp-0x24] dword + [ebp-0x22]; block2 via ebx + [ebp-0x2]) and intensity at
///     [ebp-0xc], reading the three f32 products at [ebp-0x1c]/[ebp-0x18]/[ebp-0x14].
#[test]
fn luabind_76e1e0_set_light() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // ---- (A) the fabs island -------------------------------------------------------------
    let in_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut code = vec![0xddu8, 0x05]; // fld qword [in_cell]
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    let fabs_off = code.len() as u64;
    code.extend_from_slice(&[0xd9, 0xe1]); // copied source fabs
    code.push(0xdd); // fstp qword [out_cell]  = DD 1D <disp32>
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    for src in [0x76e41fu64, 0x76e5bf] {
        assert_eq!(
            emu.read(isl + fabs_off, 2),
            emu.read(src, 2),
            "fabs island byte != source @ {src:#x}"
        );
    }
    let run_fabs = |emu: &mut Emu, v: f64| -> f64 {
        emu.write(in_cell, &v.to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        let b = emu.read(out_cell, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    // the fabs operand is f32(intensity) widened to f64; sweep representative f32 intensities.
    let ivals = [
        0.0f32, -0.0, 1.0, -1.0, 0.5, -0.5, 2.5, -2.5, 1e-7, -1e-7, 1e9, -1e9,
    ];
    for &iv in &ivals {
        let v = f64::from(iv);
        assert_eq!(
            run_fabs(&mut emu, v).to_bits(),
            intensity_abs(iv).to_bits(),
            "intensity_abs iv={iv}"
        );
        total += 1;
    }
    let mut s = 0x76e1_e000u32;
    for _ in 0..2000 {
        let iv = rngf(&mut s, -1000.0, 1000.0) as f32;
        assert_eq!(
            run_fabs(&mut emu, f64::from(iv)).to_bits(),
            intensity_abs(iv).to_bits(),
            "intensity_abs fuzz iv={iv}"
        );
        total += 1;
    }

    // ---- (B) the two color-normalize blocks ----------------------------------------------
    // Block 1: start 0x76e53d (reg-setup reads [ebp-0x24] for c1=byte1,c2=byte0 and [ebp-0x22] for
    // c0), end 0x76e599. Feed packed dword at [ebp-0x24] = c2 | (c1<<8) | (c0<<16); [ebp-0x22] then
    // reads c0 automatically (it overlaps byte2 of [ebp-0x24]). intensity at [ebp-0xc].
    let run_block1 = |emu: &mut Emu, c0: u8, c1: u8, c2: u8, intensity: f32| -> (f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        let packed = u32::from(c2) | (u32::from(c1) << 8) | (u32::from(c0) << 16);
        emu.write_u32(f.ebp - 0x24, packed); // [ebp-0x24]; byte2 (=[ebp-0x22]) = c0
        emu.write_u32(f.ebp - 0xc, intensity.to_bits());
        emu.run(0x76e53d, 0x76e599);
        (
            emu.read_f32(f.ebp - 0x1c),
            emu.read_f32(f.ebp - 0x18),
            emu.read_f32(f.ebp - 0x14),
        )
    };
    // Block 2: start 0x76e70f (reads byte [ebp-0x2] = c0, bh = c1, bl = c2 from ebx), end 0x76e768.
    let run_block2 = |emu: &mut Emu, c0: u8, c1: u8, c2: u8, intensity: f32| -> (f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EBX, u64::from(c2) | (u64::from(c1) << 8)); // bl=c2, bh=c1
        emu.write_u32(f.ebp - 0x2, u32::from(c0)); // byte [ebp-0x2] = c0 (low byte of the dword)
        emu.write_u32(f.ebp - 0xc, intensity.to_bits());
        emu.run(0x76e70f, 0x76e768);
        (
            emu.read_f32(f.ebp - 0x1c),
            emu.read_f32(f.ebp - 0x18),
            emu.read_f32(f.ebp - 0x14),
        )
    };

    let check = |emu: &mut Emu, total: &mut u32, c0: u8, c1: u8, c2: u8, iv: f32| {
        let want = light_color_block(c0, c1, c2, iv);
        let g1 = run_block1(emu, c0, c1, c2, iv);
        assert_eq!(
            (g1.0.to_bits(), g1.1.to_bits(), g1.2.to_bits()),
            (want.0.to_bits(), want.1.to_bits(), want.2.to_bits()),
            "light_color_block#1 c=({c0},{c1},{c2}) i={iv}"
        );
        let g2 = run_block2(emu, c0, c1, c2, iv);
        assert_eq!(
            (g2.0.to_bits(), g2.1.to_bits(), g2.2.to_bits()),
            (want.0.to_bits(), want.1.to_bits(), want.2.to_bits()),
            "light_color_block#2 c=({c0},{c1},{c2}) i={iv}"
        );
        *total += 2;
    };

    // deterministic: corner bytes x representative intensities (the runtime intensity is the
    // clamped-or-raw Lua scalar; the block sees it as the [ebp-0xc] f32).
    let bytes = [0u8, 1, 64, 127, 128, 200, 254, 255];
    let ivs = [0.0f32, 1.0, 0.5, 0.25, 2.0, -1.0, 0.123_456, 1.0 / 3.0];
    for &iv in &ivs {
        for &c0 in &bytes {
            for &c1 in &bytes {
                check(&mut emu, &mut total, c0, c1, c0 ^ c1, iv);
            }
        }
    }
    let mut s = 0x76e5_0000u32;
    for _ in 0..3000 {
        let c0 = (next(&mut s) >> 8) as u8;
        let c1 = (next(&mut s) >> 8) as u8;
        let c2 = (next(&mut s) >> 8) as u8;
        let iv = rngf(&mut s, -4.0, 4.0) as f32;
        check(&mut emu, &mut total, c0, c1, c2, iv);
    }
    eprintln!(
        "luabind_76e1e0_set_light: {total} bit-exact (2x fabs island; 2x block: 6 fmul[1/255] + 6 fmul[intensity])"
    );
}
