//! The `FontString` colour and alpha scripts (`SetVertexColor`, `GetAlpha`, `SetAlpha`), the
//! hyperlink button's `Configure` scale arithmetic and the static initializer `0x7cca80` against the
//! exe: each is run as the slice of instructions that computes the value.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::{channel_over_255, quantize_byte, scaled_byte};
use benilla_re_ui::hyperlink::{axis_offset, recip_scale_block};
use benilla_re_ui::ui_scale::static_scale;
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

/// Row 0x79c830 SetVertexColor — the clamp->`·255 + 0.5`->`__ftol` quantize, 4 legs (the clamp is
/// compares-only). The 4 fmul[255] (@0x79c9f0/0x79ca04/0x79ca1c/0x79ca35) + 4 fadd[0.5]
/// (@0x79c9f6/0x79ca0e/0x79ca24/0x79ca43) are byte-identical, so a single COPIED-BYTE ISLAND
/// `fld qword [in]; <fmul dword [255.0]>; <fadd dword [0.5]>; call __ftol` stands for all 8 ops
/// (byte-verified == all 8 source sites). The multiplicand is the LIVE clamped f64 on st0 (`run`
/// clears the x87 stack), so the island seeds it from a slot.
#[test]
fn luabind_79c830_set_vertex_color_quantize() {
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
    for src in [0x79c9f0u64, 0x79ca04, 0x79ca1c, 0x79ca35] {
        assert_eq!(
            emu.read(isl_fmul, 6),
            emu.read(src, 6),
            "quantize island fmul != source @ {src:#x}"
        );
    }
    for src in [0x79c9f6u64, 0x79ca0e, 0x79ca24, 0x79ca43] {
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
    let mut s = 0x79c8_3000u32;
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
        "luabind_79c830_set_vertex_color_quantize: {total} bit-exact (island: fmul[255];fadd[0.5];__ftol == all 8 sites)"
    );
}

/// Row 0x79ca90 GetVertexColor — slice 0x79cb51..0x79cb5d:
///   fild dword [ebp-8]; fmul qword [0x804578]=1/255(f64); fstp qword [esp]  (halt at the call).
/// fmul @0x79cb54 (×1). Feed the channel byte (zero-extended) at [ebp-8], read the f64 at [esp].
#[test]
fn luabind_79ca90_get_vertex_color() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x8, u32::from(byte)); // movzx-style zero-extended channel byte
        emu.run(0x79cb51, 0x79cb5d);
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
        "luabind_79ca90_get_vertex_color: {total} bit-exact (fild dword; fmul qword [1/255 f64]; fstp qword)"
    );
}

/// Row 0x79cb70 SetAlpha — `fmul qword [0x806498]=255.0(f64); call __ftol`. fmul @0x79cc28 (×1).
/// The value is the LIVE lua_tonumber result on st0 (no slot store before the __ftol), so diff via
/// a COPIED-BYTE ISLAND: `fld qword [in]` seeds st0, the EXACT source `fmul qword [255.0]` bytes
/// (byte-verified == 0x79cc28), then a real `call __ftol` runs the truncate.
#[test]
fn luabind_79cb70_set_alpha() {
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
    // copied source fmul qword [0x806498]=255.0  =  DC 0D 98 64 80 00
    let fmul_off = code.len() as u64;
    code.extend_from_slice(&[0xdc, 0x0d, 0x98, 0x64, 0x80, 0x00]);
    // call __ftol
    let call_va = isl + code.len() as u64;
    let rel = (__FTOL as i64 - (call_va as i64 + 5)) as i32;
    code.push(0xe8);
    code.extend_from_slice(&rel.to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    assert_eq!(
        emu.read(isl + fmul_off, 6),
        emu.read(0x79cc28, 6),
        "set_alpha island fmul != source @ 0x79cc28"
    );

    let run = |emu: &mut Emu, a: f64| -> u8 {
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write(in_cell, &a.to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        (emu.get_reg(RegisterX86::EAX) as i32) as u8 // __ftol result, low byte
    };

    let mut total = 0u32;
    // deterministic: the [0,1] alpha domain plus the __ftol truncate pivots (a·255 at/just below an
    // integer). NB no +0.5 bias and no clamp here — bare truncate, so a=k/255 sits exactly on a
    // pivot.
    let probes = [
        0.0f64,
        1.0,
        0.5,
        1.0 / 255.0,         // ·255 = 1.0 -> 1
        1.0 / 255.0 - 1e-12, // just below 1.0 -> 0
        2.0 / 255.0,         // -> 2.0 -> 2
        2.0 / 255.0 - 1e-12, // -> just below 2.0 -> 1
        128.0 / 255.0,
        254.0 / 255.0,
        0.999,
        0.001,
    ];
    for &a in &probes {
        assert_eq!(run(&mut emu, a), scaled_byte(a), "scaled_byte a={a}");
        total += 1;
    }
    // dense sweep of the [0,1] domain: every byte target a = k/255 (exact pivot) and a touch
    // above/below.
    for k in 0u32..=255 {
        for d in [-1e-9, 0.0, 1e-9] {
            let a = f64::from(k) / 255.0 + d;
            assert_eq!(
                run(&mut emu, a),
                scaled_byte(a),
                "scaled_byte a={a} (k={k} d={d})"
            );
            total += 1;
        }
    }
    let mut s = 0x79cb_7000u32;
    for _ in 0..4000 {
        let a = rngf(&mut s, 0.0, 1.0);
        assert_eq!(run(&mut emu, a), scaled_byte(a), "scaled_byte fuzz a={a}");
        total += 1;
    }
    eprintln!(
        "luabind_79cb70_set_alpha: {total} bit-exact (island: fmul qword [255.0]; __ftol == source 0x79cc28)"
    );
}

/// Row 0x7a3350 geometry — three own-address FP slices (`run` clears the x87 stack each entry):
///
///   R1 0x7a341a..0x7a343c: fld 1.0; fdiv [esi+0x7c]=scale; (push;sub esp,8;...) fld [ebp-0x8]=A;
///     fmul st,st(1) (R LIVE); fstp [esp+4]; fmul [ebp-0xc]=B; fstp [esp].
///     After `push 0x1`+`sub esp,8` (esp-=0xc) the two f32 products land at [esp_init-8] (A·R) and
///     [esp_init-0xc] (R·B). Covers fdiv@0x7a3420, fmul@0x7a3430 (LIVE st,st(1)), fmul@0x7a3436.
///   R2 0x7a3446..0x7a3457: fld [ebp-0x4]=D; fsub [ebp-0xc]=B; (mov edx,[edi]; push ecx; ...)
///     fdiv [esi+0x7c]; fstp [esp]. Product (D-B)/scale at [esp_init-4]. Covers fsub@0x7a3449,
///     fdiv@0x7a3451. (`mov edx,[edi]` needs EDI readable.)
///   R3 0x7a345a..0x7a346b: fld [ebp-0x8]=A; (mov eax,[edi]) fsub [ebp-0x10]=C; fdiv [esi+0x7c];
///     (push ecx; ...) fstp [esp]. Product (A-C)/scale at [esp_init-4]. Covers fsub@0x7a345f,
///     fdiv@0x7a3462. (`mov eax,[edi]` needs EDI readable.)
/// All 7 arithmetic operations covered.
#[test]
fn luabind_7a3350_geometry() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let obj = emu.alloc(0x100); // ESI base; [esi+0x7c] = scale
    let edi_buf = emu.alloc(0x10); // EDI must be readable for the mov edx/eax,[edi]

    // Region 1 — feed A=[ebp-0x8], B=[ebp-0xc], scale=[esi+0x7c]; read products at [esp-8]/[esp-0xc].
    let run_r1 = |emu: &mut Emu, a: f32, b: f32, scale: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.write_u32(f.ebp - 0x8, a.to_bits());
        emu.write_u32(f.ebp - 0xc, b.to_bits());
        emu.write_u32(obj + 0x7c, scale.to_bits());
        emu.run(0x7a341a, 0x7a343c);
        (emu.read_f32(f.esp - 0x8), emu.read_f32(f.esp - 0xc)) // (A·R), (R·B)
    };
    // Region 2 — feed D=[ebp-0x4], B=[ebp-0xc], scale; read (D-B)/scale at [esp-4].
    let run_r2 = |emu: &mut Emu, d: f32, b: f32, scale: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EDI, edi_buf);
        emu.write_u32(f.ebp - 0x4, d.to_bits());
        emu.write_u32(f.ebp - 0xc, b.to_bits());
        emu.write_u32(obj + 0x7c, scale.to_bits());
        emu.run(0x7a3446, 0x7a3457);
        emu.read_f32(f.esp - 0x4)
    };
    // Region 3 — feed A=[ebp-0x8], C=[ebp-0x10], scale; read (A-C)/scale at [esp-4].
    let run_r3 = |emu: &mut Emu, a: f32, c: f32, scale: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EDI, edi_buf);
        emu.write_u32(f.ebp - 0x8, a.to_bits());
        emu.write_u32(f.ebp - 0x10, c.to_bits());
        emu.write_u32(obj + 0x7c, scale.to_bits());
        emu.run(0x7a345a, 0x7a346b);
        emu.read_f32(f.esp - 0x4)
    };

    let mut total = 0u32;
    let check = |emu: &mut Emu, total: &mut u32, a: f32, b: f32, c: f32, d: f32, scale: f32| {
        // R1
        let g1 = run_r1(emu, a, b, scale);
        let w1 = recip_scale_block(a, b, scale);
        assert_eq!(
            (g1.0.to_bits(), g1.1.to_bits()),
            (w1.0.to_bits(), w1.1.to_bits()),
            "recip_scale_block a={a} b={b} scale={scale}"
        );
        // R2
        let g2 = run_r2(emu, d, b, scale);
        assert_eq!(
            g2.to_bits(),
            axis_offset(d, b, scale).to_bits(),
            "axis_offset(R2) d={d} b={b} scale={scale}"
        );
        // R3
        let g3 = run_r3(emu, a, c, scale);
        assert_eq!(
            g3.to_bits(),
            axis_offset(a, c, scale).to_bits(),
            "axis_offset(R3) a={a} c={c} scale={scale}"
        );
        *total += 3;
    };

    // deterministic boundary probes: scale=1.0 (R=1.0 exact), scale a power-of-two (exact recip),
    // equal extents (d-s == 0.0), and a few corner extents.
    let exts = [0.0f32, 1.0, -1.0, 0.5, 256.0, -256.0, 1234.5, 0.001];
    let scales = [1.0f32, 2.0, 4.0, 0.5, 36.0, 1024.0, 3.0, 1280.0];
    for &scale in &scales {
        for &a in &exts {
            for &b in &exts {
                check(&mut emu, &mut total, a, b, b, a, scale);
            }
        }
    }
    // explicit equal-extent (d==s) zero-difference probe across scales.
    for &scale in &scales {
        check(&mut emu, &mut total, 7.0, 7.0, 7.0, 7.0, scale);
    }
    let mut s = 0x7a33_5000u32;
    for _ in 0..3000 {
        let a = rngf(&mut s, -2048.0, 2048.0) as f32;
        let b = rngf(&mut s, -2048.0, 2048.0) as f32;
        let c = rngf(&mut s, -2048.0, 2048.0) as f32;
        let d = rngf(&mut s, -2048.0, 2048.0) as f32;
        let scale = rngf(&mut s, 0.0625, 4096.0) as f32; // positive non-zero scales
        check(&mut emu, &mut total, a, b, c, d, scale);
    }
    eprintln!(
        "luabind_7a3350_geometry: {total} bit-exact (R1 fdiv+fmul(st,st1 LIVE)+fmul[mem]; R2/R3 fsub+fdiv)"
    );
}

/// Row 0x7cca80 static-init — slice 0x7cca80..0x7cca92:
///   fld dword [0xcf6164]; fmul dword [0x7ff9cc]=(-255.0 f32); fstp dword [0xcf61a0].
/// fmul @0x7cca86 (×1). The operands are absolute .data/.rdata addresses (mapped by the harness):
/// feed [0xcf6164], read [0xcf61a0]. (`0xcf6164`/`0xcf61a0` sit in .data's BSS tail — virtual size
/// 0x4d3154 spans them, so the harness maps & zero-inits them.)
#[test]
fn luabind_7cca80_static_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let run = |emu: &mut Emu, input: f32| -> f32 {
        emu.write_u32(0xcf6164, input.to_bits()); // [0xcf6164] = the runtime global
        emu.run(0x7cca80, 0x7cca92);
        emu.read_f32(0xcf61a0) // fstp dword [0xcf61a0]
    };

    let mut total = 0u32;
    // deterministic: representative magnitudes incl. signed-zero (the scale is negative, so the
    // sign flips; -0.0 vs +0.0 is a bit-distinct boundary), and exact small values.
    let probes = [
        0.0f32, -0.0, 1.0, -1.0, 36.0, -36.0, 0.5, 2.5, 1e-7, -1e-7, 1e9, -1e9, 0.123_456,
    ];
    for &input in &probes {
        assert_eq!(
            run(&mut emu, input).to_bits(),
            static_scale(input).to_bits(),
            "static_scale input={input}"
        );
        total += 1;
    }
    let mut s = 0x7cca_8000u32;
    for _ in 0..3000 {
        let input = rngf(&mut s, -10000.0, 10000.0) as f32;
        assert_eq!(
            run(&mut emu, input).to_bits(),
            static_scale(input).to_bits(),
            "static_scale fuzz input={input}"
        );
        total += 1;
    }
    eprintln!("luabind_7cca80_static_scale: {total} bit-exact (fld [g]; fmul dword [-255.0 f32]; fstp [g2])");
}
