//! The message frame colour scripts (`AddMessage` of the scrolling and the plain frame,
//! `UpdateColorByID`) and the edit box inset scripts (`SetTextInsets`, `GetTextInsets`) against the
//! exe: each is run as the slice of instructions that quantizes a colour channel or converts an inset,
//! with the Lua marshalling stubbed around it.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::quantize_byte;
use benilla_re_ui::ui_scale::{gettextinsets_block, settextinsets_block};
use fpkit::rngf;

const __FTOL: u64 = 0x40a2b0;
/// `0x832a4c` (.data) = the f32 `0x41ad70` pushes (= the UI-scale leaf). Read live from the image.
const SCALE_LEAF: u64 = 0x832a4c;

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

/// The clamp->`·255 + 0.5`->`__ftol` color-byte quantize, shared by AddMessage(3ch)'s 3 legs,
/// UpdateColorByID's 3 legs, and AddMessage(4ch)'s 4 legs (10 fmul[255] + 10 fadd[0.5]). The
/// multiplicand is the LIVE clamped f64 on st0 (the island seeds it via `fld qword [in]`); the
/// EXACT source `fmul dword [255.0]` + `fadd dword [0.5]` bytes follow (byte-verified == all 20
/// source sites), then a real `call __ftol` runs the truncate.
#[test]
fn luabind_792_color_quantize() {
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
        // 0x792900 AddMessage(3ch)
        0x792ac0u64,
        0x792ad4,
        0x792af1,
        // 0x7932b0 UpdateColorByID
        0x793464,
        0x793478,
        0x793495,
        // 0x795590 AddMessage(4ch)
        0x79579f,
        0x7957b3,
        0x7957d1,
        0x7957ed,
    ] {
        assert_eq!(
            emu.read(isl_fmul, 6),
            emu.read(src, 6),
            "quantize island fmul != source @ {src:#x}"
        );
    }
    for src in [
        0x792ac6u64,
        0x792ae6,
        0x792afb,
        0x79346a,
        0x79348a,
        0x79349f,
        0x7957a5,
        0x7957bd,
        0x7957d9,
        0x7957fa,
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
    // below/at an integer). The runtime only ever feeds clamped [0,1] (or the 0.0/1.0 constants).
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
    let mut s = 0x7929_0000u32;
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
        "luabind_792_color_quantize: {total} bit-exact (island: fmul[255];fadd[0.5];__ftol == all 20 sites)"
    );
}

/// 0x798a60 SetTextInsets — four byte-identical convert blocks. Own-address slices over
///   call 0x41ad70; fmul dword [0x7ffd68]=1024; push ecx; fdivr dword [ebp-4]; fstp dword [esp]
/// halting at the `call 0x41ae60`. The real 0x41ad70 pushes the f32 at [0x832a4c]; feed the Rust
/// the SAME k_scale = read_f32(0x832a4c). Seed [ebp-4] = inset (f32). Read the f32 at [esp].
/// fmul[1024] @0x798b4e/0x798b78/0x798ba2/0x798bcc ; fdivr @0x798b55/0x798b7f/0x798ba9/0x798bd3.
#[test]
fn luabind_798a60_set_text_insets() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let k_scale = emu.read_f32(SCALE_LEAF); // the f32 0x41ad70 pushes (live from image)

    // (call_0x41ad70_addr, end_at_0x41ae60_call) per block. Each slice: call; fmul[1024];
    // push ecx; fdivr [ebp-4]; fstp [esp].  The end is the `call 0x41ae60` site.
    let blocks = [
        (0x798b49u64, 0x798b5bu64),
        (0x798b73, 0x798b85),
        (0x798b9d, 0x798baf),
        (0x798bc7, 0x798bd9),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, inset: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x4, inset.to_bits()); // the lua value, already f32-narrowed
        emu.run(start, end);
        // `push ecx` moved esp down 4 before `fstp dword [esp]`; result lands at f.esp - 4.
        emu.read_f32(f.esp - 0x4)
    };

    let mut total = 0u32;
    let check = |emu: &mut Emu, total: &mut u32, inset: f32| {
        let want = settextinsets_block(k_scale, inset);
        for (bi, &(start, end)) in blocks.iter().enumerate() {
            let got = run(emu, start, end, inset);
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "settextinsets_block#{bi} @ {start:#x} inset={inset} k_scale={k_scale}"
            );
            *total += 1;
        }
    };

    // deterministic: 0 (pixel inset 0 -> 0), small/large insets, a fractional pivot, negatives.
    let insets = [
        0.0f32,
        1.0,
        2.0,
        8.0,
        16.0,
        32.0,
        64.0,
        128.0,
        1024.0,
        -8.0,
        0.5,
        1.0 / 3.0,
        12345.0,
    ];
    for &iv in &insets {
        check(&mut emu, &mut total, iv);
    }
    let mut s = 0x798a_6000u32;
    for _ in 0..3000 {
        let iv = rngf(&mut s, -2048.0, 2048.0) as f32;
        check(&mut emu, &mut total, iv);
    }
    eprintln!(
        "luabind_798a60_set_text_insets: {total} bit-exact (4 slices: fmul[1024] + fdivr[ebp-4], k_scale={k_scale})"
    );
}

/// 0x798c30 GetTextInsets — four byte-identical convert blocks (the inverse: multiply). Own-address
/// slices over  call 0x41ad70; fmul dword [0x7ffd68]=1024; push ecx; fmul dword [ebp-X]; fstp [esp]
/// halting at the `call 0x41ae40`. Seed [ebp-X] = inset (f32); read f32 at [esp].
/// fmul[1024] @0x798cef/0x798d13/0x798d37/0x798d5b ;
/// fmul[ebp-X] @0x798cf6([ebp-4])/0x798d1a([ebp-8])/0x798d3e([ebp-c])/0x798d62([ebp-10]).
#[test]
fn luabind_798c30_get_text_insets() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let k_scale = emu.read_f32(SCALE_LEAF);

    // (call_0x41ad70_addr, end_at_0x41ae40_call, ebp_disp) per block.
    let blocks = [
        (0x798ceau64, 0x798cfcu64, 0x4u64),
        (0x798d0e, 0x798d20, 0x8),
        (0x798d32, 0x798d44, 0xc),
        (0x798d56, 0x798d68, 0x10),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, disp: u64, inset: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - disp, inset.to_bits()); // the stored inset (f32 out-param)
        emu.run(start, end);
        emu.read_f32(f.esp - 0x4) // after `push ecx`, `fstp dword [esp]` -> f.esp - 4
    };

    let mut total = 0u32;
    let check = |emu: &mut Emu, total: &mut u32, inset: f32| {
        let want = gettextinsets_block(k_scale, inset);
        for (bi, &(start, end, disp)) in blocks.iter().enumerate() {
            let got = run(emu, start, end, disp, inset);
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "gettextinsets_block#{bi} @ {start:#x} inset={inset} k_scale={k_scale}"
            );
            *total += 1;
        }
    };

    // deterministic: 0, unit insets, a fractional pivot (the unit->pixel domain), negatives.
    let insets = [
        0.0f32,
        1.0,
        0.5,
        0.25,
        0.001_953_125, // 2/1024 -> exact pixel back if k_scale=1
        -0.5,
        1.0 / 3.0,
        0.123_456,
        2.0,
    ];
    for &iv in &insets {
        check(&mut emu, &mut total, iv);
    }
    let mut s = 0x798c_3000u32;
    for _ in 0..3000 {
        let iv = rngf(&mut s, -4.0, 4.0) as f32;
        check(&mut emu, &mut total, iv);
    }
    eprintln!(
        "luabind_798c30_get_text_insets: {total} bit-exact (4 slices: fmul[1024] + fmul[ebp-X], k_scale={k_scale})"
    );
}
