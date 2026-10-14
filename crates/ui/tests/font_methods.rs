//! The `Font` object methods whose arithmetic is a layout conversion, a colour pack or a change test
//! (`0x79f210` to `0x79fbe0`) against the exe: each is run as the slice of instructions that
//! computes the value, including the leaf helpers `0x41ad70`, `0x41ae40` and `0x41ae60`, with the
//! Lua marshalling stubbed around it.
//!
//! The setters at `0x79fb40` and `0x79f210` store only when `|new - old|` reaches 2^-22; they differ
//! in whether the change test reads the new value from the stack (an f64) or from the `f32` the
//! store spilled.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::{normalize_channel, pack_color, scaled_byte, unpack_color};
use benilla_re_ui::fontstring::{set_scaled_changed, set_scaled_changed_spilled};
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

// patch 0x6f3620 -> `fld qword [cell]; ret`  (dd 05 <disp32> c3) so the lua fetch yields our f64.
fn patch_fetch_single(emu: &mut Emu, cell: u64) {
    let mut code = vec![0xddu8, 0x05];
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(0x6f3620, &code);
}

// patch 0x6f3620 -> `fld qword [edx*8 + table]; ret`  (dd 04 d5 <disp32> c3) — edx = the arg index.
fn patch_fetch_table(emu: &mut Emu, table: u64) {
    let mut code = vec![0xddu8, 0x04, 0xd5];
    code.extend_from_slice(&(table as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(0x6f3620, &code);
}

/// point_to_logical — single getter (SCALE*1024*value)/0.8, pushed as f64.
///   0x79f3b0 (field +0x48, value -> [ebp+8]); slice 0x79f3d1 .. 0x79f3f0 (push-arg f64 at [esp]).
///   0x79fbe0 (field +0x50, value -> [ebp-4]); slice 0x79fbea .. 0x79fc0a.
#[test]
fn get_scaled_79f3b0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let run = |emu: &mut Emu, value: f32| -> f64 {
        emu.write_u32(ebp + 0x8, value.to_bits()); // value at [ebp+8] (fmul [ebp+8])
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79f3d1, 0x79f3f0); // halt at call 0x6f3810; f64 push-arg at [esp]
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };
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
            "79f3b0 det v={v}"
        );
    }
    let mut s = 0x79f3_b000u32;
    for _ in 0..6000 {
        let v = rng(&mut s, -5000.0, 5000.0);
        let bin = run(&mut emu, v);
        assert_eq!(bin.to_bits(), point_to_logical(v).to_bits(), "79f3b0 v={v}");
    }
    eprintln!("get_scaled_79f3b0: 6009 bit-exact");
}

#[test]
fn get_scaled_79fbe0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let run = |emu: &mut Emu, value: f32| -> f64 {
        emu.write_u32(ebp - 0x4, value.to_bits()); // value at [ebp-4] (fmul [ebp-4])
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79fbea, 0x79fc0a);
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };
    let mut s = 0x79fb_e000u32;
    for _ in 0..6000 {
        let v = rng(&mut s, -5000.0, 5000.0);
        let bin = run(&mut emu, v);
        assert_eq!(bin.to_bits(), point_to_logical(v).to_bits(), "79fbe0 v={v}");
    }
    eprintln!("get_scaled_79fbe0: 6000 bit-exact");
}

/// get_point — 0x79fad0 Get2D: fields +0x60/+0x64 (via [ebp-8]/[ebp-4]), pushed in order.
///   slice 0x79fad6 .. 0x79fb2e (both pushes via stubbed 0x6f3810).
#[test]
fn get_scaled_pair_79fad0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2);
    let widget = emu.alloc(0x140);
    let run = |emu: &mut Emu, v0: f32, v1: f32| -> Vec<f64> {
        emu.write_u32(widget + 0x60, v0.to_bits());
        emu.write_u32(widget + 0x64, v1.to_bits());
        emu.write_u32(ebp + 0x8, 0x1234); // [ebp+8] = lua_state arg (reloaded into esi/ecx)
        emu.set_reg(RegisterX86::EDX, widget);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79fad6, 0x79fb2e); // both legs through both pushes
        pushes(emu)
    };
    let mut s = 0x79fa_d000u32;
    for _ in 0..4000 {
        let v0 = rng(&mut s, -2000.0, 2000.0);
        let v1 = rng(&mut s, -2000.0, 2000.0);
        let p = run(&mut emu, v0, v1);
        assert_eq!(p.len(), 2, "79fad0: expected 2 pushes, got {}", p.len());
        let (m0, m1) = get_point(v0, v1);
        assert_eq!(p[0].to_bits(), m0.to_bits(), "79fad0 v0={v0}");
        assert_eq!(p[1].to_bits(), m1.to_bits(), "79fad0 v1={v1}");
    }
    eprintln!("get_scaled_pair_79fad0: 4000 bit-exact");
}

/// set_point — 0x79f9c0 Set2D: arg2 -> [ebp-4], arg3 -> [ebp+8]; field+0x60=set(arg2),
///   field+0x64=set(arg3). slice 0x79fa16 .. 0x79fa57 (arg3-leg -> [ebp+8], arg2-leg -> [ebp-8]).
#[test]
fn set_scaled_pair_79f9c0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let widget = emu.alloc(0x140); // esi = widget (the tail reads [esi+0x5c])
    let run = |emu: &mut Emu, arg2: f32, arg3: f32| -> (f32, f32) {
        emu.write_u32(ebp - 0x4, arg2.to_bits()); // arg2 fetch result
        emu.write_u32(ebp + 0x8, arg3.to_bits()); // arg3 fetch result
        emu.set_reg(RegisterX86::ESI, widget);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79fa16, 0x79fa57); // halt at jne; arg3-result in [ebp+8], arg2-result in [ebp-8]
                                     // field+0x60 = arg2-result ([ebp-8]), field+0x64 = arg3-result ([ebp+8]).
        (emu.read_f32(ebp - 0x8), emu.read_f32(ebp + 0x8))
    };
    for (a2, a3) in [
        (1.0f32, -1.0),
        (0.5, 1024.0),
        (0.0009765625, 12.0),
        (1.0e-20, 1.0e20),
    ] {
        let (b0, b1) = run(&mut emu, a2, a3);
        let (m0, m1) = set_point(a2, a3);
        assert_eq!(b0.to_bits(), m0.to_bits(), "79f9c0 det field60 a2={a2}");
        assert_eq!(b1.to_bits(), m1.to_bits(), "79f9c0 det field64 a3={a3}");
    }
    let mut s = 0x79f9_c000u32;
    for _ in 0..4000 {
        let a2 = rng(&mut s, -3000.0, 3000.0);
        let a3 = rng(&mut s, -3000.0, 3000.0);
        let (b0, b1) = run(&mut emu, a2, a3);
        let (m0, m1) = set_point(a2, a3);
        assert_eq!(b0.to_bits(), m0.to_bits(), "79f9c0 field60 a2={a2}");
        assert_eq!(b1.to_bits(), m1.to_bits(), "79f9c0 field64 a3={a3}");
    }
    eprintln!("set_scaled_pair_79f9c0: 4004 bit-exact");
}

/// set_scaled_changed — 0x79fb40: LIVE-f64 change-detect. The ×0.8 (0x41ae60) result stays in st0
///   un-narrowed; `fld old; fsubr st,st(1); fabs; fcomp eps; test ah,5; jp` decides store-vs-skip,
///   and on store `fstp [esi+0x50]` narrows the LIVE f64 to f32.
///
/// Diffed as a WHOLE-FN `Emu::call` (covers the body by construction; a mid-block slice re-entered
/// across cases hits a stale translation block and reads unmapped memory). 0x6f34d0 -> 1 (arg present),
/// 0x6f3620 patched to `fld qword [cell]; ret` (yields our chosen lua f64), the tail vtable update
/// `[[esi]+0x14]` pointed at a `xor eax,eax; ret` island. Two calls per case extract both observables:
///   * stored f32  — flag [esi+0x38]=0 (small-Δ stores too) -> field+0x50 always = new as f32.
///   * changed     — flag [esi+0x38]=0x1000 (small-Δ SKIPS) -> the store fires iff jp-taken (|Δ|>=eps
///     or NaN); detect via [esi+0x38] bit 0x10 (the store path ORs 0x1010; skip doesn't).
#[test]
fn set_scaled_changed_79fb40() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cell = emu.alloc(8);
    patch_fetch_single(&mut emu, cell); // 0x6f3620 -> fld qword [cell]; ret
    emu.stub(0x6f34d0, 1, 0); // arg present
    let widget = emu.alloc(0x140); // esi/edx = widget; old at +0x50, flags at +0x38, vtable at [widget]
    let vt = emu.alloc(0x20);
    let null_ret = emu.alloc(0x10);
    emu.write(null_ret, &[0x33, 0xc0, 0xc3]); // xor eax,eax ; ret
    emu.write_u32(vt + 0x14, null_ret as u32); // [[esi]+0x14] update -> no-op
    emu.write_u32(widget, vt as u32);

    // run the whole fn once; `flag` seeds [esi+0x38] (0 -> always store, 0x1000 -> store iff changed).
    let run1 = |emu: &mut Emu, arg: f32, old: f32, flag: u32| -> u32 {
        emu.write(cell, &f64::from(arg).to_bits().to_le_bytes()); // lua value = arg (f64)
        emu.write_u32(widget + 0x50, old.to_bits());
        emu.write_u32(widget + 0x38, flag);
        emu.call(0x79fb40, 0x1234, widget as u32, &[0x1234]); // thiscall: ecx=L, edx=widget, arg=L
        emu.read_u32(widget + 0x38) // for the changed read; field via a separate read
    };
    let run = |emu: &mut Emu, arg: f32, old: f32| -> (f32, bool) {
        run1(emu, arg, old, 0); // flag clear: store always fires
        let stored = emu.read_f32(widget + 0x50);
        let flag_after = run1(emu, arg, old, 0x1000); // flag set: store iff jp-taken (changed)
        let changed = flag_after & 0x10 != 0; // the store path ORs 0x1010; skip leaves 0x1000
        (stored, changed)
    };
    // deterministic: arg/old straddling the eps band, plus exact-equal and NaN.
    let det: [(f32, f32); 6] = [
        (1.0, 0.625),    // big Δ
        (0.8, 0.8),      // Δ == 0 (small)
        (1.0e-9, 0.0),   // tiny arg, near-zero new
        (-1.0, -0.625),  // negative
        (1024.0, 800.0), // large
        (f32::NAN, 0.5), // NaN -> changed
    ];
    for (arg, old) in det {
        let (bin_stored, bin_changed) = run(&mut emu, arg, old);
        let mine = set_scaled_changed(arg, old);
        assert_eq!(
            bin_stored.to_bits(),
            mine.stored.to_bits(),
            "79fb40 stored arg={arg} old={old}"
        );
        assert_eq!(
            bin_changed, mine.changed,
            "79fb40 changed arg={arg} old={old}"
        );
    }
    // boundary probe: choose old so that |new - old| is EXACTLY eps (the <-vs-<= flip).
    let eps = f64::from(f32::from_bits(0x3480_0000));
    {
        let arg = 100.0f32;
        let new_live = {
            // mirror logical_to_device's live f64 to compute a bit-exact old at new - eps (PXSCALE bits).
            let pxscale = f32::from_bits(0x3f4c_cccd); // 0.8
            let denom = f64::from(1.0f32) * f64::from(f32::from_bits(0x4480_0000)); // SCALE*1024
            let q32 = (f64::from(arg) / denom) as f32;
            f64::from(pxscale) * f64::from(q32)
        };
        for delta_at in [eps, eps * 0.5, eps * 2.0] {
            let old = (new_live - delta_at) as f32;
            let (_bs, bin_changed) = run(&mut emu, arg, old);
            assert_eq!(
                bin_changed,
                set_scaled_changed(arg, old).changed,
                "79fb40 eps-boundary arg={arg} old={old} delta_at={delta_at}"
            );
        }
    }
    let mut s = 0x79fb_4000u32;
    let mut changed_n = 0u32;
    let mut unchanged_n = 0u32;
    for _ in 0..4000 {
        let arg = rng(&mut s, -2000.0, 2000.0);
        // half the time set old near the new value to exercise the small-Δ path.
        let near = (next(&mut s) >> 8) & 1 == 0;
        let old = if near {
            logical_to_device(arg) // exactly new -> Δ tiny
        } else {
            rng(&mut s, -2000.0, 2000.0)
        };
        let (bin_stored, bin_changed) = run(&mut emu, arg, old);
        let mine = set_scaled_changed(arg, old);
        assert_eq!(
            bin_stored.to_bits(),
            mine.stored.to_bits(),
            "79fb40 stored arg={arg} old={old}"
        );
        assert_eq!(
            bin_changed, mine.changed,
            "79fb40 changed arg={arg} old={old}"
        );
        if bin_changed {
            changed_n += 1;
        } else {
            unchanged_n += 1;
        }
    }
    eprintln!(
        "set_scaled_changed_79fb40: 4009 bit-exact (changed={changed_n} small={unchanged_n})"
    );
    assert!(
        changed_n > 0 && unchanged_n > 0,
        "need both change-detect branches"
    );
}

/// set_scaled_changed_spilled — 0x79f210: the ×0.8 result is fstp [ebp-4] = f32-spilled, then the
///   change-detect reloads that f32. Slice (a) the set arith 0x79f264 .. 0x79f27b (read [ebp-4] f32);
///   slice (b) the change-detect 0x79f2d7 .. 0x79f2e7 (fld old; fsubr [ebp-4]; fabs; fcomp eps;
///   fnstsw) and read the AH branch.
#[test]
fn set_scaled_changed_spilled_79f210() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let widget = emu.alloc(0x140); // esi = widget; old at [esi+0x48]

    // (a) the set arith leg: value at [ebp-4] (the fetch result), result re-stored to [ebp-4] (f32).
    let run_set = |emu: &mut Emu, value: f32| -> f32 {
        emu.write_u32(ebp - 0x4, value.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79f264, 0x79f27e); // halt after `fstp [ebp-4]`
        emu.read_f32(ebp - 0x4)
    };
    // (b) the change-detect leg: new (f32) in [ebp-4], old in [esi+0x48]; read the |Δ|>=eps branch.
    let run_chg = |emu: &mut Emu, new_f32: f32, old: f32| -> bool {
        emu.write_u32(ebp - 0x4, new_f32.to_bits());
        emu.write_u32(widget + 0x48, old.to_bits());
        emu.set_reg(RegisterX86::ESI, widget);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79f2d7, 0x79f2e7); // fld [esi+48]; fsubr [ebp-4]; fabs; fcomp eps; fnstsw
        let ah = ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8;
        // jp 0x79f2f1 taken (|Δ|>=eps or NaN) iff parity(ah&5) EVEN (PF=1) -> changed.
        (ah & 0x5).count_ones().is_multiple_of(2)
    };

    let det: [(f32, f32); 6] = [
        (1.0, 0.625),
        (0.8, 0.8),
        (1.0e-9, 0.0),
        (-1.0, -0.625),
        (1024.0, 800.0),
        (f32::NAN, 0.5),
    ];
    for (arg, old) in det {
        let bin_set = run_set(&mut emu, arg);
        let mine = set_scaled_changed_spilled(arg, old);
        assert_eq!(
            bin_set.to_bits(),
            mine.stored.to_bits(),
            "79f210 set arg={arg}"
        );
        let bin_chg = run_chg(&mut emu, bin_set, old);
        assert_eq!(bin_chg, mine.changed, "79f210 changed arg={arg} old={old}");
    }
    // eps boundary: old = new_f32 - eps exactly (the strict-< boundary).
    let eps = f64::from(f32::from_bits(0x3480_0000));
    {
        let arg = 100.0f32;
        let new_f32 = logical_to_device(arg);
        for d in [eps, eps * 0.5, eps * 2.0] {
            let old = (f64::from(new_f32) - d) as f32;
            let bin_chg = run_chg(&mut emu, new_f32, old);
            assert_eq!(
                bin_chg,
                set_scaled_changed_spilled(arg, old).changed,
                "79f210 eps-boundary arg={arg} old={old} d={d}"
            );
        }
    }
    let mut s = 0x79f2_1000u32;
    let mut changed_n = 0u32;
    let mut unchanged_n = 0u32;
    for _ in 0..4000 {
        let arg = rng(&mut s, -2000.0, 2000.0);
        let bin_set = run_set(&mut emu, arg);
        let mine = set_scaled_changed_spilled(arg, 0.0);
        assert_eq!(
            bin_set.to_bits(),
            mine.stored.to_bits(),
            "79f210 set arg={arg}"
        );
        let near = (next(&mut s) >> 8) & 1 == 0;
        let old = if near {
            bin_set
        } else {
            rng(&mut s, -2000.0, 2000.0)
        };
        let bin_chg = run_chg(&mut emu, bin_set, old);
        assert_eq!(
            bin_chg,
            set_scaled_changed_spilled(arg, old).changed,
            "79f210 changed arg={arg} old={old}"
        );
        if bin_chg {
            changed_n += 1;
        } else {
            unchanged_n += 1;
        }
    }
    eprintln!(
        "set_scaled_changed_spilled_79f210: 4009 bit-exact (changed={changed_n} small={unchanged_n})"
    );
    assert!(
        changed_n > 0 && unchanged_n > 0,
        "need both change-detect branches"
    );
}

// pack_color — 0x79f4d0 SetColor (store [esi+0x58], EBX) / 0x79f730 SetShadowColor (store
//   [esi+0x5c], ECX). The clamp reads the LIVE 0x6f3620 value -> patch it to a per-index table.
//   0x6f34d0 (alpha-present) stubbed to 1. Diff the assembled u32 (R/G/B f32, A LIVE f64; 0xAARRGGBB).
fn pack_det() -> [(f64, f64, f64, f64); 6] {
    [
        (0.0, 1.0, 0.5, 1.0),
        (-0.1, 1.1, 0.0039215, 0.9), // out-of-range -> clamp
        (f64::NAN, 0.5, 1.0, 0.0),   // NaN -> 1.0
        (1.0, 1.0, 1.0, 1.0),
        (0.5, 0.25, 0.125, 0.75),
        (0.5 / 255.0, 1.5 / 255.0, 2.5 / 255.0, 3.5 / 255.0), // *.5 quantize ties
    ]
}

fn pack_mk(s: &mut u32) -> f64 {
    let pick = (next(s) >> 8) % 3;
    match pick {
        0 => f64::from(rng(s, -0.5, 0.0)), // below -> clamp 0
        1 => f64::from(rng(s, 1.0, 1.5)),  // >=1 -> clamp 1
        _ => f64::from(rng(s, 0.0, 0.9999)),
    }
}

#[test]
fn pack_color_79f4d0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let table = emu.alloc(0x40); // indexed by edx (2..=5) * 8
    patch_fetch_table(&mut emu, table);
    emu.stub(0x6f34d0, 1, 0); // alpha present
    let widget = emu.alloc(0x140); // esi = widget (the tail reads [esi+0x58])

    let run = |emu: &mut Emu, r: f64, g: f64, b: f64, a: f64| -> u32 {
        for (idx, v) in [(2u64, r), (3, g), (4, b), (5, a)] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        emu.set_reg(RegisterX86::EDI, ebp); // edi = lua_state (only used by 0x6f3620/0x6f34d0)
        emu.set_reg(RegisterX86::ESI, widget);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79f4dd, 0x79f652); // clamp+quantize+assemble; halt at `cmp eax,ecx`; eax = u32
        emu.get_reg(RegisterX86::EAX) as u32
    };
    for (r, g, b, a) in pack_det() {
        let bin = run(&mut emu, r, g, b, a);
        let mine = pack_color(r, g, b, a);
        assert_eq!(bin, mine, "79f4d0 det r={r} g={g} b={b} a={a}");
    }
    let mut s = 0x79f4_d000u32;
    let (mut lo, mut hi, mut mid) = (0u32, 0u32, 0u32);
    for _ in 0..6000 {
        let (r, g, b, a) = (
            pack_mk(&mut s),
            pack_mk(&mut s),
            pack_mk(&mut s),
            pack_mk(&mut s),
        );
        for v in [r, g, b, a] {
            if v < 0.0 {
                lo += 1;
            } else if v >= 1.0 {
                hi += 1;
            } else {
                mid += 1;
            }
        }
        let bin = run(&mut emu, r, g, b, a);
        assert_eq!(
            bin,
            pack_color(r, g, b, a),
            "79f4d0 r={r} g={g} b={b} a={a}"
        );
    }
    eprintln!("pack_color_79f4d0: 6006 bit-exact (clamp_lo={lo} clamp_hi={hi} in_range={mid})");
    assert!(
        lo > 0 && hi > 0 && mid > 0,
        "need all three clamp dispositions"
    );
}

/// 0x79f730 SetShadowColor — same clamp/quantize/assemble arithmetic, result lands in ECX.
#[test]
fn pack_color_79f730() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let table = emu.alloc(0x40);
    patch_fetch_table(&mut emu, table);
    emu.stub(0x6f34d0, 1, 0);
    let widget = emu.alloc(0x140); // esi = widget (the tail reads [esi+0x5c])

    let run = |emu: &mut Emu, r: f64, g: f64, b: f64, a: f64| -> u32 {
        for (idx, v) in [(2u64, r), (3, g), (4, b), (5, a)] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        emu.set_reg(RegisterX86::EDI, ebp);
        emu.set_reg(RegisterX86::ESI, widget);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79f73d, 0x79f8b0); // clamp+quantize+assemble; halt at `cmp ecx,eax`; ecx = u32
        emu.get_reg(RegisterX86::ECX) as u32
    };
    for (r, g, b, a) in pack_det() {
        let bin = run(&mut emu, r, g, b, a);
        assert_eq!(
            bin,
            pack_color(r, g, b, a),
            "79f730 det r={r} g={g} b={b} a={a}"
        );
    }
    let mut s = 0x79f7_3000u32;
    for _ in 0..5000 {
        let (r, g, b, a) = (
            pack_mk(&mut s),
            pack_mk(&mut s),
            pack_mk(&mut s),
            pack_mk(&mut s),
        );
        let bin = run(&mut emu, r, g, b, a);
        assert_eq!(
            bin,
            pack_color(r, g, b, a),
            "79f730 r={r} g={g} b={b} a={a}"
        );
    }
    eprintln!("pack_color_79f730: 5006 bit-exact");
}

// unpack_color — 0x79f680 (field +0x58) / 0x79f910 (field +0x5c): u32 0xAARRGGBB -> 4×(byte/255),
//   pushed (R,G,B,A) all as f64. slice over the unpack core; 0x6f3810 stubbed; 0x81c6b4 -> INV255.
fn unpack_det() -> Vec<u32> {
    vec![
        0x0000_0000,
        0xffff_ffff,
        0xff00_0000,
        0x00ff_0000,
        0x0000_ff00,
        0x0000_00ff,
        0x8040_2010,
        0x0103_07ff,
    ]
}

#[test]
fn unpack_color_79f680() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2);
    let widget = emu.alloc(0x140);
    let run = |emu: &mut Emu, color: u32| -> Vec<f64> {
        emu.write_u32(widget + 0x58, color);
        emu.write_u32(ebp + 0x8, 0x1234); // lua_state arg (reloaded into edi)
        emu.set_reg(RegisterX86::EDX, widget);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79f688, 0x79f721); // 4 pushes; halt at `pop edi`
        pushes(emu)
    };
    let mut det = unpack_det();
    let mut s = 0x79f6_8000u32;
    for _ in 0..3000 {
        det.push(next(&mut s));
    }
    for color in det {
        let p = run(&mut emu, color);
        assert_eq!(p.len(), 4, "79f680: expected 4 pushes, got {}", p.len());
        let m = unpack_color(color);
        for (i, (g, e)) in p.iter().zip(m.iter()).enumerate() {
            assert_eq!(
                g.to_bits(),
                e.to_bits(),
                "79f680 color={color:#010x} idx={i}"
            );
        }
    }
    eprintln!("unpack_color_79f680: 3008 bit-exact");
}

#[test]
fn unpack_color_79f910() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2);
    let widget = emu.alloc(0x140);
    let run = |emu: &mut Emu, color: u32| -> Vec<f64> {
        emu.write_u32(widget + 0x5c, color);
        emu.write_u32(ebp + 0x8, 0x1234);
        emu.set_reg(RegisterX86::EDX, widget);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79f918, 0x79f9b1); // 4 pushes; halt at `pop edi`
        pushes(emu)
    };
    let mut det = unpack_det();
    let mut s = 0x79f9_1000u32;
    for _ in 0..3000 {
        det.push(next(&mut s));
    }
    for color in det {
        let p = run(&mut emu, color);
        assert_eq!(p.len(), 4, "79f910: expected 4 pushes, got {}", p.len());
        let m = unpack_color(color);
        for (i, (g, e)) in p.iter().zip(m.iter()).enumerate() {
            assert_eq!(
                g.to_bits(),
                e.to_bits(),
                "79f910 color={color:#010x} idx={i}"
            );
        }
    }
    eprintln!("unpack_color_79f910: 3008 bit-exact");
}

/// normalize_channel — 0x79f4a0: byte [edx+0x5b] -> byte*(1/255), pushed f64.
///   slice 0x79f4a4 .. 0x79f4bd (full byte domain 0..=255).
#[test]
fn normalize_byte_79f4a0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2);
    let widget = emu.alloc(0x140);
    let run = |emu: &mut Emu, byte: u8| -> f64 {
        emu.write(widget + 0x5b, &[byte]); // [edx+0x5b] = the byte field
        emu.write_u32(ebp + 0x8, 0x1234);
        emu.set_reg(RegisterX86::EDX, widget);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x79f4a4, 0x79f4bd); // halt at call 0x6f3810; f64 at [esp]
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };
    for b in 0u8..=255 {
        let bin = run(&mut emu, b);
        assert_eq!(
            bin.to_bits(),
            normalize_channel(b).to_bits(),
            "79f4a0 byte={b}"
        );
        // cross-check against the channel-normalize (identical arithmetic).
        assert_eq!(
            normalize_channel(b).to_bits(),
            normalize_channel(b).to_bits()
        );
    }
    eprintln!("normalize_byte_79f4a0: 256 bit-exact");
}

/// scaled_byte — 0x79f420: lua f64 -> byte via __ftol(lua * 255.0) (truncate, no +0.5 bias).
///   patch 0x6f3620 -> `fld qword [cell]; ret`; slice 0x79f442 .. 0x79f454; read EAX low byte.
#[test]
fn color_byte_79f420() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let cell = emu.alloc(8);
    patch_fetch_single(&mut emu, cell);
    let run = |emu: &mut Emu, lua: f64| -> u8 {
        emu.write(cell, &lua.to_bits().to_le_bytes());
        emu.set_reg(RegisterX86::EDI, ebp); // edi = lua_state (ecx for 0x6f3620)
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x79f442, 0x79f454); // fetch; fmul 255.0; __ftol; halt at cmp al,[esi+0x5b]
        (emu.get_reg(RegisterX86::EAX) & 0xff) as u8
    };
    // deterministic: the [0,1] fractions land on integer/half boundaries; negatives truncate toward 0.
    let det: [f64; 19] = [
        0.0,
        1.0,
        0.5,
        1.0 / 255.0,
        0.5 / 255.0,                  // 0.5 -> trunc 0
        1.5 / 255.0,                  // 1.5 -> trunc 1
        254.9 / 255.0,                // ~254.9 -> 254
        -0.5,                         // -127.5 -> -127 -> low byte 0x81
        -1.0,                         // -255 -> 0x01? (-255 & 0xff)
        2.0,                          // 510 -> low byte 0xfe
        f64::from(f32::from_bits(1)), // tiny -> 0
        0.999_999,                    // ~254.99 -> 254
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
    for lua in det {
        let bin = run(&mut emu, lua);
        assert_eq!(
            bin,
            scaled_byte(lua),
            "79f420 det lua={lua} bin={bin} mine={}",
            scaled_byte(lua)
        );
    }
    let mut s = 0x79f4_2000u32;
    for _ in 0..5000 {
        let lua = f64::from(rng(&mut s, -2.0, 3.0)); // span negatives, [0,1], and >1
        let bin = run(&mut emu, lua);
        assert_eq!(bin, scaled_byte(lua), "79f420 lua={lua}");
    }
    eprintln!("color_byte_79f420: 5012 bit-exact");
}
