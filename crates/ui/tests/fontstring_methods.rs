//! The `FontString` methods whose arithmetic is a layout conversion or a colour pack (`GetFont`,
//! `SetFont`, `GetTextColor`, `SetTextColor`, `GetShadowColor`, `SetShadowColor`, `GetShadowOffset`,
//! `SetShadowOffset`, `GetSpacing`, `SetSpacing`, `SetTextHeight`, `GetStringWidth`) against the
//! exe: each is run as the slice of instructions that computes the value, including the leaf
//! helpers `0x41ad70`, `0x41ae40` and `0x41ae60`, with the Lua marshalling stubbed around it.
//!
//! A conversion narrows its product to `f32` as the argument of the leaf and again at the store.
//! The colour setters clamp and quantize as `SetVertexColor` does; `GetTextColor` pushes red as an
//! f64 and the rest through an `f32` store, `GetShadowColor` all four as f64.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::{
    clamp01, clamp01_alpha, normalize_channel, pack_color, quantize_channel, unpack_color,
    unpack_vertex_color,
};
use benilla_re_ui::fontstring::set_text_height_guarded;
use benilla_re_ui::ui_scale::{get_point, logical_to_device, point_to_logical, set_point};
use fpkit::{next, read_f64, rng};

fn dbits(c: &benilla_re_difftest::StubCall) -> f64 {
    f64::from_bits((u64::from(c.args[1]) << 32) | u64::from(c.args[0]))
}
fn pushes(emu: &Emu) -> Vec<f64> {
    emu.calls()
        .iter()
        .filter(|c| c.addr == 0x6f3810)
        .map(dbits)
        .collect()
}

/// A synthesized stack frame: `(ebp, esp)` with room below `ebp` for locals and above for pushes.
fn frame(emu: &mut Emu) -> (u64, u64) {
    let base = emu.alloc(0x200);
    let ebp = base + 0x100;
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000;
    (ebp, esp)
}

/// point_to_logical — the 1-value getter scale (SCALE*1024*value)/0.8, pushed as f64.
///   slice: call 0x41ad70 .. call 0x6f3810 ; the f64 push-arg sits at [esp].
/// Representatives: 0x79d4a5 (0x79d400) · 0x79e27c (0x79e1d0) · 0x79e5bb (0x79e510).
#[test]
fn get_scaled_slice() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);

    // run one value through the 0x79e1d0 getter slice (0x79e27c arith .. 0x79e29b push).
    let run = |emu: &mut Emu, value: f32| -> f64 {
        emu.write_u32(ebp - 0x4, value.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79e27c, 0x79e29b); // halt at `call 0x6f3810`; arg f64 at [esp]
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };

    // deterministic boundaries: 0, ±tiny, exact powers, a value whose product rounds at the f32 store.
    for v in [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        12.0,
        0.0009765625,
        1.0e-30,
        1.0e30,
    ] {
        let bin = run(&mut emu, v);
        assert_eq!(
            bin.to_bits(),
            point_to_logical(v).to_bits(),
            "point_to_logical det v={v} bin={bin} mine={}",
            point_to_logical(v)
        );
    }
    let mut s = 0x79e1_d000u32;
    for _ in 0..6000 {
        let v = rng(&mut s, -5000.0, 5000.0);
        let bin = run(&mut emu, v);
        assert_eq!(
            bin.to_bits(),
            point_to_logical(v).to_bits(),
            "point_to_logical v={v} bin={bin} mine={}",
            point_to_logical(v)
        );
    }
    eprintln!("get_scaled_slice: 6009 bit-exact");
}

/// 0x79d400 getter: same slice shape (0x79d4a5 arith .. 0x79d4c4 push) — verify the representative
/// covers it (value in [ebp-4], identical leaf chain). A separate slice run confirms byte-identity.
#[test]
fn get_scaled_79d400() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let run = |emu: &mut Emu, value: f32| -> f64 {
        emu.write_u32(ebp - 0x4, value.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79d4a5, 0x79d4c4);
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };
    let mut s = 0x79d4_0000u32;
    for _ in 0..4000 {
        let v = rng(&mut s, -3000.0, 3000.0);
        let bin = run(&mut emu, v);
        assert_eq!(
            bin.to_bits(),
            point_to_logical(v).to_bits(),
            "0x79d400 v={v}"
        );
    }
    eprintln!("get_scaled_79d400: 4000 bit-exact");
}

/// 0x79e510 GetStringWidth: width is gx-delegated (0x772890), but the scale leg is owned and
/// byte-identical to point_to_logical (0x79e5bb arith .. 0x79e5da push).
#[test]
fn get_scaled_79e510() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let run = |emu: &mut Emu, value: f32| -> f64 {
        emu.write_u32(ebp - 0x4, value.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79e5bb, 0x79e5da);
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };
    let mut s = 0x79e5_1000u32;
    for _ in 0..4000 {
        let v = rng(&mut s, 0.0, 4000.0); // a width is >= 0
        let bin = run(&mut emu, v);
        assert_eq!(
            bin.to_bits(),
            point_to_logical(v).to_bits(),
            "0x79e510 v={v}"
        );
    }
    eprintln!("get_scaled_79e510: 4000 bit-exact");
}

/// get_point — 0x79df60 GetShadowOffset, two point_to_logical legs over [ebp-8] / [ebp-4],
/// pushed in order (v0, v1). slice 0x79e017 .. 0x79e05a (both pushes via stubbed 0x6f3810).
#[test]
fn get_scaled_pair_79df60() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2); // capture lua_pushnumber

    let run = |emu: &mut Emu, v0: f32, v1: f32| -> Vec<f64> {
        emu.write_u32(ebp - 0x8, v0.to_bits());
        emu.write_u32(ebp - 0x4, v1.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79e017, 0x79e05f); // both legs, through both pushes
        pushes(emu)
    };

    let mut s = 0x79df_6000u32;
    for _ in 0..4000 {
        let v0 = rng(&mut s, -2000.0, 2000.0);
        let v1 = rng(&mut s, -2000.0, 2000.0);
        let p = run(&mut emu, v0, v1);
        assert_eq!(p.len(), 2, "79df60: expected 2 pushes, got {}", p.len());
        let (m0, m1) = get_point(v0, v1);
        assert_eq!(p[0].to_bits(), m0.to_bits(), "79df60 v0={v0}");
        assert_eq!(p[1].to_bits(), m1.to_bits(), "79df60 v1={v1}");
    }
    eprintln!("get_scaled_pair_79df60: 4000 bit-exact");
}

/// logical_to_device — the 1-value setter scale 0.8*(value/(SCALE*1024)), stored back as f32.
///   slice 0x79d5c1 .. 0x79d5d8 (the field store re-stores to [ebp-4]).
/// Representatives: 0x79d4f0 · 0x79e2b0 (push to [esp] for gx store).
#[test]
fn set_scaled_slice() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);

    // 0x79d4f0 slice: value at [ebp-4], result re-stored to [ebp-4] by 0x79d5d8.
    let run = |emu: &mut Emu, value: f32| -> f32 {
        emu.write_u32(ebp - 0x4, value.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79d5c1, 0x79d5db); // halt after `fstp [ebp-4]`
        emu.read_f32(ebp - 0x4)
    };

    for v in [
        1.0f32,
        -1.0,
        0.5,
        12.0,
        1024.0,
        0.0009765625,
        1.0e-20,
        1.0e20,
    ] {
        let bin = run(&mut emu, v);
        assert_eq!(
            bin.to_bits(),
            logical_to_device(v).to_bits(),
            "logical_to_device det v={v} bin={bin} mine={}",
            logical_to_device(v)
        );
    }
    let mut s = 0x79d4_f000u32;
    for _ in 0..6000 {
        let v = rng(&mut s, -5000.0, 5000.0);
        let bin = run(&mut emu, v);
        assert_eq!(
            bin.to_bits(),
            logical_to_device(v).to_bits(),
            "logical_to_device v={v}"
        );
    }
    eprintln!("set_scaled_slice: 6008 bit-exact");
}

/// 0x79e2b0 SetSpacing: same arith, result pushed to [esp] for the gx store 0x772240.
/// slice 0x79e359 .. 0x79e376 (halt at `call 0x772240`; the f32 arg at [esp]).
#[test]
fn set_scaled_79e2b0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let run = |emu: &mut Emu, value: f32| -> f32 {
        emu.write_u32(ebp - 0x4, value.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79e359, 0x79e376); // halt at `call 0x772240`; f32 arg at [esp]
        emu.read_f32(emu.get_reg(RegisterX86::ESP))
    };
    let mut s = 0x79e2_b000u32;
    for _ in 0..5000 {
        let v = rng(&mut s, -4000.0, 4000.0);
        let bin = run(&mut emu, v);
        assert_eq!(
            bin.to_bits(),
            logical_to_device(v).to_bits(),
            "0x79e2b0 v={v}"
        );
    }
    eprintln!("set_scaled_79e2b0: 5000 bit-exact");
}

/// set_point — 0x79e070 SetShadowOffset, two logical_to_device legs over [ebp-4] / [ebp-8].
///   leg-0 result -> [ebp-0xc] (via [ebp-4]); leg-1 result -> [ebp-0x10].
///   slice 0x79e13e .. 0x79e17b (halt at `push edx` before the gx store 0x7723f0).
#[test]
fn set_scaled_pair_79e070() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let run = |emu: &mut Emu, a0: f32, a1: f32| -> (f32, f32) {
        emu.write_u32(ebp - 0x4, a0.to_bits());
        emu.write_u32(ebp - 0x8, a1.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79e13e, 0x79e17b);
        (emu.read_f32(ebp - 0xc), emu.read_f32(ebp - 0x10))
    };
    let mut s = 0x79e0_7000u32;
    for _ in 0..4000 {
        let a0 = rng(&mut s, -3000.0, 3000.0);
        let a1 = rng(&mut s, -3000.0, 3000.0);
        let (b0, b1) = run(&mut emu, a0, a1);
        let (m0, m1) = set_point(a0, a1);
        assert_eq!(b0.to_bits(), m0.to_bits(), "0x79e070 a0={a0}");
        assert_eq!(b1.to_bits(), m1.to_bits(), "0x79e070 a1={a1}");
    }
    eprintln!("set_scaled_pair_79e070: 4000 bit-exact");
}

/// set_text_height_guarded — 0x79e3c0, the non-positive guard + logical_to_device.
///   The fcomp reads the LIVE f64 from 0x6f3620; we patch 0x6f3620 -> `fld qword [cell]; ret`
///   and write the chosen f64 into the cell, then run the WHOLE body 0x79e461 .. exit and observe
///   which path ran (the gx store 0x771600 on compute, the format push 0x6f4940 on error).
#[test]
fn set_spacing_guarded_79e3c0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    // patch 0x6f3620 -> `fld qword [cell]; ret` so the fetch yields our chosen f64 LIVE in st0.
    let cell = emu.alloc(8);
    let mut code = vec![0xddu8, 0x05]; // fld qword [disp32]
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(0x6f3620, &code);
    // stub the two terminal sinks so we can detect which path ran; both return to the body.
    emu.stub(0x771600, 0, 1); // gx SetSpacing store (compute path) — __thiscall, `ret 4` (1 arg)
    emu.stub(0x6f4940, 0, 0); // format/error push (error path) — __cdecl variadic (caller cleans)
    let edi = emu.alloc(0x140); // self widget (only [edi+0xd4] flag is touched on compute)
    let vt = emu.alloc(0x20);
    emu.write_u32(edi, vt as u32);
    // the error path does a virtual call `[[edi]+4]` (GetObjectName -> a string ptr); point it at a
    // `xor eax,eax; ret` island so it returns null (the format then uses the 0x84c7f0 fallback).
    let null_ret = emu.alloc(0x10);
    emu.write(null_ret, &[0x33, 0xc0, 0xc3]); // xor eax,eax ; ret
    emu.write_u32(vt + 4, null_ret as u32);

    const SENT: u32 = 0x6f6f_6f6f; // both paths' `mov esp,ebp; pop ebp; ret` land on this.
    let mut compute_n = 0u32;
    let mut error_n = 0u32;

    let run = |emu: &mut Emu, lua: f64| -> (Option<f32>, &'static str) {
        emu.write(cell, &lua.to_bits().to_le_bytes());
        emu.write_u32(ebp, ebp as u32); // [ebp] = saved ebp (popped by `pop ebp`)
        emu.write_u32(ebp + 4, SENT); // [ebp+4] = return addr -> sentinel halt
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.clear_calls();
        // run the body (from the value fetch) through whichever exit; the gx store 0x771600 and the
        // format push 0x6f4940 are stubbed, so both paths reach the epilogue and `ret` to SENT.
        emu.run(0x79e461, u64::from(SENT));
        // which path ran? the compute path calls 0x771600 (the f32 spacing pushed at [esp]); the
        // error path calls 0x6f4940. Read the compute result from the 0x771600 stub's [esp] arg.
        let store = emu.calls().iter().find(|c| c.addr == 0x771600).copied();
        match store {
            Some(c) => (Some(f32::from_bits(c.args[0])), "compute"),
            None => (None, "error"),
        }
    };

    // deterministic boundary straddle around 0.0 (the guard edge), plus a sub-f32 denormal.
    for lua in [
        1.0f64,
        -1.0,
        0.0,
        -0.0,
        1.0e-30,
        -1.0e-30,
        f64::from(f32::from_bits(1)), // smallest positive f32
        1.0e-50,                      // positive f64 that narrows to +0 f32
        f64::NAN,
    ] {
        let (bin, path) = run(&mut emu, lua);
        let mine = set_text_height_guarded(lua);
        match (bin, mine) {
            (Some(b), Some(m)) => assert_eq!(b.to_bits(), m.to_bits(), "guard compute lua={lua}"),
            (None, None) => {}
            _ => panic!("guard path mismatch lua={lua}: bin={path}/{bin:?} mine={mine:?}"),
        }
    }

    let mut s = 0x79e3_c000u32;
    for _ in 0..4000 {
        // span negatives, zero-ish and positives so both paths fire.
        let lua = f64::from(rng(&mut s, -100.0, 100.0));
        let (bin, path) = run(&mut emu, lua);
        let mine = set_text_height_guarded(lua);
        match (bin, mine) {
            (Some(b), Some(m)) => assert_eq!(b.to_bits(), m.to_bits(), "guard compute lua={lua}"),
            (None, None) => {}
            _ => panic!("guard path mismatch lua={lua}: bin={path} mine={mine:?}"),
        }
        if bin.is_some() {
            compute_n += 1;
        } else {
            error_n += 1;
        }
    }
    eprintln!("set_spacing_guarded_79e3c0: 4009 bit-exact (compute={compute_n} error={error_n})");
    assert!(compute_n > 0 && error_n > 0, "need both guard paths");
}

/// unpack_color — 0x79d840 / 0x79dbe0 GetColor: u32 0xAARRGGBB -> 4× (byte/255), pushed (R,G,B,A).
///   slice over the unpack core, 0x6f3810 stubbed; 0x81c6b4 written to INV255.
#[test]
fn unpack_color_79dbe0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2);
    // 0x79dbe0 reads the colour from field [eax+0x104]; slice 0x79dc88 .. 0x79dd2e (4 pushes).
    let widget = emu.alloc(0x140);
    let run = |emu: &mut Emu, color: u32| -> Vec<f64> {
        emu.write_u32(widget + 0x104, color);
        emu.set_reg(RegisterX86::EAX, widget);
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79dc88, 0x79dd2e);
        pushes(emu)
    };
    let mut det: Vec<u32> = vec![
        0x0000_0000,
        0xffff_ffff,
        0xff00_0000,
        0x00ff_0000,
        0x0000_ff00,
        0x0000_00ff,
        0x8040_2010,
        0x0103_07ff,
    ];
    let mut s = 0x79db_e000u32;
    for _ in 0..3000 {
        det.push(next(&mut s));
    }
    for color in det {
        let p = run(&mut emu, color);
        assert_eq!(p.len(), 4, "79dbe0: expected 4 pushes, got {}", p.len());
        let m = unpack_color(color);
        for (i, (g, e)) in p.iter().zip(m.iter()).enumerate() {
            assert_eq!(
                g.to_bits(),
                e.to_bits(),
                "79dbe0 color={color:#010x} idx={i}"
            );
        }
    }
    eprintln!("unpack_color_79dbe0: 3008 bit-exact");
}

/// 0x79d840 reads the colour via a temp [ebp-4] (filled by 0x77f8c0); slice 0x79d8f8 .. 0x79d9ad.
#[test]
fn unpack_color_79d840() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2);
    let run = |emu: &mut Emu, color: u32| -> Vec<f64> {
        emu.write_u32(ebp - 0x4, color);
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79d8f8, 0x79d9ad);
        pushes(emu)
    };
    let mut det: Vec<u32> = vec![0, 0xffff_ffff, 0x8040_2010, 0x0103_07ff];
    let mut s = 0x79d8_4000u32;
    for _ in 0..3000 {
        det.push(next(&mut s));
    }
    for color in det {
        let p = run(&mut emu, color);
        assert_eq!(p.len(), 4, "79d840: expected 4 pushes, got {}", p.len());
        let m = unpack_vertex_color(color); // R is f64; G/B/A carry the f32 round.
        for (i, (g, e)) in p.iter().zip(m.iter()).enumerate() {
            assert_eq!(
                g.to_bits(),
                e.to_bits(),
                "79d840 color={color:#010x} idx={i}"
            );
        }
    }
    eprintln!("unpack_color_79d840: 3004 bit-exact");
}

/// Exercise the per-channel normalize directly across the full byte domain (covers every fild/fmul).
#[test]
fn normalize_channel_full_domain() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2);
    let widget = emu.alloc(0x140);
    // every byte appears in the B (low) channel for color = 0x000000BB -> push idx 2.
    let run = |emu: &mut Emu, byte: u8| -> f64 {
        emu.write_u32(widget + 0x104, u32::from(byte));
        emu.set_reg(RegisterX86::EAX, widget);
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79dc88, 0x79dd2e);
        pushes(emu)[2] // B channel = color & 0xff
    };
    for b in 0u8..=255 {
        let bin = run(&mut emu, b);
        assert_eq!(
            bin.to_bits(),
            normalize_channel(b).to_bits(),
            "normalize byte={b} bin={bin} mine={}",
            normalize_channel(b)
        );
    }
    eprintln!("normalize_channel_full_domain: 256 bit-exact");
}

/// pack_color — 0x79d9c0 SetColor / 0x79dd40 SetShadowColor (clone family; arith byte-identical,
/// differing only by store-callee + flag mask). Clamp reads the LIVE 0x6f3620 value -> patch it to
/// `fld qword [TABLE + edx*8]; ret`, write per-arg-index f64s, run the clamp+quantize+assembly, read
/// the assembled u32 the binary writes (ebx -> [ebp-0x10]). Diffs the OWNED arithmetic of both rows.
#[test]
fn pack_color_79d9c0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);

    // patch 0x6f3620 -> `fld qword [edx*8 + TABLE]; ret`  (dd 04 d5 <disp32> c3).
    let table = emu.alloc(0x40); // indexed by edx (2..=5) * 8
    let mut code = vec![0xddu8, 0x04, 0xd5];
    code.extend_from_slice(&(table as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(0x6f3620, &code);
    // 0x6f34d0 (is-arg-present, for the alpha arg5): stub -> present(1) so we always supply alpha.
    emu.stub(0x6f34d0, 1, 0);

    // run the clamp+quantize+pack slice (0x79da4b .. 0x79dbc8, halt before the gx store 0x77f750).
    let run = |emu: &mut Emu, r: f64, g: f64, b: f64, a: f64| -> u32 {
        for (idx, v) in [(2u64, r), (3, g), (4, b), (5, a)] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EDI, ebp); // edi = self; the assembly writes ebx to [ebp-0x10]
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79da4b, 0x79dbcb); // halt at `call 0x77f750`; ebx assembled, stored to [ebp-0x10]
        emu.get_reg(RegisterX86::EBX) as u32
    };

    // deterministic: the clamp edges (exactly 0.0 / 1.0), NaN, out-of-range, and the *.5 quantize tie.
    for (r, g, b, a, _l) in [
        (0.0f64, 1.0, 0.5, 1.0, "edges"),
        (-0.1, 1.1, 0.0039215, 0.9, "out-of-range -> clamp"),
        (f64::NAN, 0.5, 1.0, 0.0, "nan -> 1.0"),
        (1.0, 1.0, 1.0, 1.0, "all 1"),
        (0.5, 0.25, 0.125, 0.75, "fracs"),
        // c*255 lands on *.5: e.g. 0.5/255 boundaries — exercise quantize rounding.
        (
            0.5 / 255.0,
            1.5 / 255.0,
            2.5 / 255.0,
            3.5 / 255.0,
            "half-ties",
        ),
    ] {
        let bin = run(&mut emu, r, g, b, a);
        let mine = pack_color(r, g, b, a);
        assert_eq!(
            bin, mine,
            "pack det r={r} g={g} b={b} a={a}: bin={bin:#010x} mine={mine:#010x}"
        );
    }

    let mut s = 0x79d9_c000u32;
    let mut clamp_lo = 0u32;
    let mut clamp_hi = 0u32;
    let mut in_range = 0u32;
    for _ in 0..6000 {
        // mix in-range [0,1] with out-of-range to exercise both clamp branches.
        let mk = |s: &mut u32| -> f64 {
            let pick = (next(s) >> 8) % 3;
            match pick {
                0 => f64::from(rng(s, -0.5, 0.0)), // below -> clamp 0
                1 => f64::from(rng(s, 1.0, 1.5)),  // >=1 -> clamp 1
                _ => f64::from(rng(s, 0.0, 0.9999)),
            }
        };
        let (r, g, b, a) = (mk(&mut s), mk(&mut s), mk(&mut s), mk(&mut s));
        for v in [r, g, b, a] {
            if v < 0.0 {
                clamp_lo += 1;
            } else if v >= 1.0 {
                clamp_hi += 1;
            } else {
                in_range += 1;
            }
        }
        let bin = run(&mut emu, r, g, b, a);
        let mine = pack_color(r, g, b, a);
        assert_eq!(
            bin, mine,
            "pack r={r} g={g} b={b} a={a}: bin={bin:#010x} mine={mine:#010x}"
        );
    }
    eprintln!(
        "pack_color_79d9c0: 6006 bit-exact (clamp_lo={clamp_lo} clamp_hi={clamp_hi} in_range={in_range})"
    );
    assert!(
        clamp_lo > 0 && clamp_hi > 0 && in_range > 0,
        "need all three clamp dispositions"
    );
}

/// 0x79dd40 SetShadowColor — clone of 0x79d9c0 (masked-byte-identical arith; differs only by the
/// store callee 0x7723c0 and the flag mask). Diff its own bytes over the same slice to confirm the
/// inherited arithmetic holds at the bytes (not by positional assumption).
#[test]
fn pack_color_79dd40() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let table = emu.alloc(0x40);
    let mut code = vec![0xddu8, 0x04, 0xd5];
    code.extend_from_slice(&(table as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(0x6f3620, &code);
    emu.stub(0x6f34d0, 1, 0);

    let run = |emu: &mut Emu, r: f64, g: f64, b: f64, a: f64| -> u32 {
        for (idx, v) in [(2u64, r), (3, g), (4, b), (5, a)] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EDI, ebp);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79ddcb, 0x79df4b); // 0x79dd40 clamp+quantize+assembly; halt at `call 0x7723c0`
        emu.get_reg(RegisterX86::EBX) as u32
    };

    let mut s = 0x79dd_4000u32;
    for _ in 0..5000 {
        let mk = |s: &mut u32| -> f64 {
            let pick = (next(s) >> 8) % 3;
            match pick {
                0 => f64::from(rng(s, -0.5, 0.0)),
                1 => f64::from(rng(s, 1.0, 1.5)),
                _ => f64::from(rng(s, 0.0, 0.9999)),
            }
        };
        let (r, g, b, a) = (mk(&mut s), mk(&mut s), mk(&mut s), mk(&mut s));
        let bin = run(&mut emu, r, g, b, a);
        let mine = pack_color(r, g, b, a);
        assert_eq!(bin, mine, "79dd40 r={r} g={g} b={b} a={a}");
    }
    eprintln!("pack_color_79dd40: 5000 bit-exact");
}

/// Exercise clamp01 + quantize_channel directly across edges (deterministic boundary probes).
#[test]
fn clamp_quantize_boundaries() {
    // pure-Rust boundary checks (no emu): the values where strict-vs-inclusive would flip.
    assert_eq!(clamp01(0.0), 0.0);
    assert_eq!(clamp01(-0.0), -0.0); // -0.0 is not < 0.0 -> kept (then quantizes to 0)
    assert_eq!(clamp01(1.0), 1.0);
    assert_eq!(
        clamp01(f32::from_bits(0x3f7f_ffff)),
        f32::from_bits(0x3f7f_ffff)
    ); // 1.0 - ulp kept
    assert_eq!(clamp01(f32::NAN).to_bits(), 1.0f32.to_bits());
    assert_eq!(clamp01_alpha(0.0), 0.0);
    assert_eq!(clamp01_alpha(1.0), 1.0);
    assert_eq!(clamp01_alpha(f64::NAN).to_bits(), 1.0f64.to_bits());
    assert_eq!(quantize_channel(0.0), 0);
    assert_eq!(quantize_channel(1.0), 255); // 255.0 + 0.5 = 255.5 -> trunc 255
    assert_eq!(quantize_channel(0.5), 128); // 127.5 + 0.5 = 128.0 -> trunc 128
}
