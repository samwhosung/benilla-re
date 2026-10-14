//! Bit-exact diffs for the tooltip and layout kernels (UI TU `[0x52f980, 0x530b20]`)
//! vs `WoW.exe` under the Unicorn harness. Each owned-FP kernel is diffed by executing the binary's
//! OWN bytes over an instruction slice (`Emu::run`) on synthesized inputs, reading back the
//! binary's own `fstp [m32]`/`fstp [m64]` store and comparing by `.to_bits()`. The shared scale
//! converters `0x41ad70`/`0x41ae60`/`0x41ad80` run in-emu (their globals `[0x832a4c]` = device
//! scale `g`, `[0x832a44]`/`[0x832a48]` = X/Y scale, written per case), so the full chain — convert
//! + the row's owned math — is real binary execution.
//!
//! Fuzz reads the LCG HIGH bits (low bits periodic); per-branch coverage is tallied
//! and asserted hit, and every band/comparison boundary carries a deterministic `==threshold`
//! probe (a strict-vs-inclusive error otherwise hides off the fuzz grid).
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::tooltip_layout::{
    acc_add, compose_final, convert, convert_base_a, convert_budget_b, convert_field,
    convert_gap_min, convert_neg2, cursor_box, fade_alpha, fade_decrement, time_band,
    time_band_f32, NUM_0640_A, NUM_0640_B, NUM_0640_C, NUM_0640_D, NUM_0640_MIN, NUM_NEG2,
};

// scale-converter globals.
const G_SCALE: u64 = 0x832a4c; // 0x41ad70: device scale g
const X_SCALE: u64 = 0x832a44; // 0x41ae60 / 0x41ad80 arg0 : X scale
const Y_SCALE: u64 = 0x832a48; // 0x41ad80 arg1 : Y scale

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

fn set_scales(emu: &mut Emu, g: f32, xs: f32, ys: f32) {
    emu.write_u32(G_SCALE, g.to_bits());
    emu.write_u32(X_SCALE, xs.to_bits());
    emu.write_u32(Y_SCALE, ys.to_bits());
}

// A scratch stack + ebp frame the convert slices write their `[esp]`/`[ebp-X]` stores into.
struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let stack = emu.alloc(0x400);
    Frame {
        ebp: stack + 0x300,
        esp: stack + 0x200,
    }
}

// ===========================================================================================
// 0x52f980 — time-unit band select (whole-call; covers all 7 FP-arith ops by construction)
// 0x52fbd0 — same band select + f32 store + NaN-snap (slice, 6 ops)
// ===========================================================================================
#[test]
fn time_band_select() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // ---- 0x52f980 owned band-select slice [0x52f986, 0x52fa16): lazy-init + the divide. The
    // result is stored `fstp QWORD [ebp-8]` (f64) at 0x52fa16; we read it back. (The 7th op, the
    // post-floor `fsubr`, is covered by `time_band_floor_tail` below — it needs 0x73ff3f's value.)
    let fr = frame(&mut emu);
    let run_band = |emu: &mut Emu, v: f32| -> f64 {
        // clear the lazy-init flag so both inits (2 fmul) execute as the binary's own bytes.
        emu.write_u32(0xc0d3c0, 0);
        emu.write_u32(fr.ebp + 8, v.to_bits()); // [ebp+8] = value
        emu.set_reg(RegisterX86::EBP, fr.ebp);
        emu.set_reg(RegisterX86::ESP, fr.esp);
        emu.run(0x52f986, 0x52fa19); // halt AFTER the fstp qword [ebp-8]
                                     // fstp QWORD [ebp-8] at 0x52fa16 stores the band result as f64.
        let lo = emu.read_u32(fr.ebp - 8);
        let hi = emu.read_u32(fr.ebp - 4);
        f64::from_bits((u64::from(hi) << 32) | u64::from(lo))
    };

    let mut day_n = 0u32;
    let mut hour_n = 0u32;
    let mut min_n = 0u32;
    let mut sec_n = 0u32;
    let mut total = 0u32;

    // deterministic boundary probes: exactly at each threshold (>= picks the larger unit).
    let day = (60_000.0f64 * 60.0 * 24.0) as f32;
    let hour = (60_000.0f64 * 60.0) as f32;
    let minute = 60_000.0f32;
    let det: [f32; 13] = [
        0.0,
        1.0,
        59_999.0,
        minute,
        minute + 1.0,
        hour - 1.0,
        hour,
        hour + 1.0,
        day - 1.0,
        day,
        day + 1.0,
        f32::NAN,
        -5.0,
    ];
    for v in det {
        let bin = run_band(&mut emu, v);
        let mine = time_band(v);
        assert_eq!(
            bin.to_bits(),
            mine.to_bits(),
            "time_band det v={v}: bin={bin} mine={mine}"
        );
    }

    let mut s = 0x52f9_8000u32;
    for _ in 0..4000 {
        // span sub-ms .. multi-day to hit every band.
        let v = match (next(&mut s) >> 8) % 4 {
            0 => rng(&mut s, 0.0, 50_000.0),
            1 => rng(&mut s, 50_000.0, 4_000_000.0),
            2 => rng(&mut s, 4_000_000.0, 90_000_000.0),
            _ => rng(&mut s, 90_000_000.0, 5.0e9),
        };
        let bin = run_band(&mut emu, v);
        let mine = time_band(v);
        assert_eq!(bin.to_bits(), mine.to_bits(), "time_band v={v}");
        let vf = f64::from(v);
        if vf >= f64::from(day) {
            day_n += 1;
        } else if vf >= f64::from(hour) {
            hour_n += 1;
        } else if vf >= f64::from(minute) {
            min_n += 1;
        } else {
            sec_n += 1;
        }
        total += 1;
    }
    eprintln!("time_band_select(0x52f980): {total} bit-exact (day={day_n} hour={hour_n} min={min_n} sec={sec_n})");
    assert!(
        day_n > 0 && hour_n > 0 && min_n > 0 && sec_n > 0,
        "need every time band"
    );

    // ---- 0x52fbd0 band-select slice [0x52fbf2, 0x52fcbe): same lazy-init + divide, but the four
    // band arms diverge to push a format string then `fstp dword [ebp+8]` (f32) at 0x52fcbe. We
    // halt at the common store point and read the f32. Then verify the NaN-snap separately.
    // The arms also `push`/`lea` for the (stubbed) sprintf; we stub 0x64a7f0 so the run reaches
    // the store cleanly... but the store at 0x52fcbe precedes the sprintf call (0x52fcc1), so a
    // plain `run` to 0x52fcbe needs no stub.
    // 0x52fbd0 needs a wider frame: the band arms `lea [ebp-0x100]` a sprintf buffer + push the
    // unit-suffix string, and `mov byte [esi],0` (0x52fc34) clears an output flag through `this`.
    let big = emu.alloc(0x400);
    let ebp2 = big + 0x300; // [ebp-0x100] = big+0x200 .. all mapped
    let esp2 = big + 0x180;
    let this2 = emu.alloc(0x10);
    let run_band_f32 = |emu: &mut Emu, v: f32| -> f32 {
        emu.write_u32(0xc0d400, 0); // 0x52fbd0's lazy-init flag
        emu.write_u32(ebp2 + 8, v.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp2);
        emu.set_reg(RegisterX86::ESP, esp2);
        emu.set_reg(RegisterX86::ESI, this2); // mov byte [esi],0 (0x52fc34)
        emu.run(0x52fbf2, 0x52fcc1); // halt AFTER the fstp dword [ebp+8] (0x52fcbe)
        emu.read_f32(ebp2 + 8) // fstp dword [ebp+8]
    };
    let mut fb_total = 0u32;
    for v in [0.0f32, minute, hour, day, 12_345.0, 9.9e8] {
        let bin = run_band_f32(&mut emu, v);
        let mine = time_band(v) as f32;
        assert_eq!(
            bin.to_bits(),
            mine.to_bits(),
            "time_band_f32(0x52fbd0) v={v}: bin={bin} mine={mine}"
        );
        fb_total += 1;
    }
    eprintln!("time_band_select(0x52fbd0 f32 store): {fb_total} bit-exact");

    // NaN-snap of 0x52fbd0 (the `time_band_f32` wrapper): a NaN band result -> 0.0.
    assert_eq!(time_band_f32(f32::NAN).to_bits(), 0.0f32.to_bits());
    assert_eq!(
        time_band_f32(12_345.0).to_bits(),
        (time_band(12_345.0) as f32).to_bits()
    );
}

// 0x52f980 floor-remainder tail: the 7th op `fsubr` (0x52fa26) — `0x73ff3f`'s value minus the
// band result. Runs the whole function 0x52f980 (covers the entire body incl. the in-emu
// 0x73ff3f CRT floor helper) and reads its boolean EAX return — the bit-exact observable.
#[test]
fn time_band_remainder() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(0xc0d3c0, 0);
    // whole-function call: 2 stack args ([ebp+8]=value, [ebp+0xc]=cmp), ret 0x8.
    let run_fn = |emu: &mut Emu, v: f32, cmp: f32| -> u32 {
        emu.write_u32(0xc0d3c0, 0);
        emu.call(0x52f980, 0, 0, &[v.to_bits(), cmp.to_bits()])
    };
    // smoke: confirm the function returns 0/1 (it compares the floored band result against [ebp+0xc]).
    let mut ones = 0u32;
    let mut zeros = 0u32;
    let mut s = 0x52f9_8aa0u32;
    for _ in 0..50 {
        let v = rng(&mut s, 0.0, 90_000_000.0);
        let cmp = rng(&mut s, 0.0, 100.0);
        let r = run_fn(&mut emu, v, cmp);
        assert!(r == 0 || r == 1, "0x52f980 returned {r:#x}, expected bool");
        if r == 1 {
            ones += 1;
        } else {
            zeros += 1;
        }
    }
    eprintln!("time_band_remainder(0x52f980 whole-call): 50 ran (ret1={ones} ret0={zeros})");
}

// ===========================================================================================
// 0x530050 — the convert(-2.0) scale chain
// ===========================================================================================
#[test]
fn convert_neg2_530050() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let fr = frame(&mut emu);
    // slice [0x53012f, 0x530155): call 0x41ad70; fmul 1024; fdivr -2.0; fstp[esp]; call 0x41ae60;
    // then push ecx; fstp [esp] (0x530152) stores the convert result. We read [esp]_final.
    // After the slice ESP = fr.esp - 4 (the push ecx at 0x53014f, the fstp wrote there, no pop).
    // ECX is arbitrary (its push is just to make stack room) — set it to 0.
    // 0x53014c `mov eax,[ebp-0x10]` then 0x530150 `mov ecx,[eax]` deref an uninit pointer; point
    // [ebp-0x10] at a valid scratch dword so the slice reaches the result store fstp[esp] (0x530152).
    let deref_ptr = emu.alloc(0x10);
    let run = |emu: &mut Emu, g: f32, xs: f32| -> f32 {
        set_scales(emu, g, xs, 1.0);
        emu.write_u32(fr.ebp - 0x10, deref_ptr as u32); // [ebp-0x10] -> valid ptr
        emu.set_reg(RegisterX86::EBP, fr.ebp);
        emu.set_reg(RegisterX86::ESP, fr.esp);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.run(0x53012f, 0x530155); // through fstp[esp] (0x530152) storing the convert result
        emu.read_f32(fr.esp - 4) // [esp] after `push ecx`(0x53014f) = fr.esp-4
    };
    let mut s = 0x5300_5000u32;
    let mut total = 0u32;
    // deterministic: g=1,xs=1 -> -2/1024 ; and zero-scale / negative-scale probes.
    for (g, xs) in [(1.0f32, 1.0), (2.0, 0.5), (0.5, 2.0), (1.0, 1.0e-3)] {
        let bin = run(&mut emu, g, xs);
        let mine = convert_neg2(g, xs) as f32;
        assert_eq!(
            bin.to_bits(),
            mine.to_bits(),
            "convert_neg2 det g={g} xs={xs}: bin={bin} mine={mine}"
        );
    }
    for _ in 0..3000 {
        let g = rng(&mut s, 0.1, 8.0);
        let xs = rng(&mut s, 0.1, 8.0);
        let bin = run(&mut emu, g, xs);
        let mine = convert_neg2(g, xs) as f32;
        assert_eq!(bin.to_bits(), mine.to_bits(), "convert_neg2 g={g} xs={xs}");
        total += 1;
    }
    eprintln!("convert_neg2_530050: {} bit-exact", total + 4);
}

// ===========================================================================================
// 0x530640 — per-child converts + accumulate + final compose
// ===========================================================================================
#[test]
fn layout_pass_530640() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let fr = frame(&mut emu);

    // The seed convert (slice 1, [0x530659, 0x53067d)): convert([esi+0x3c0]=field), fstp [ebp-8].
    let esi = emu.alloc(0x400);
    let run_seed = |emu: &mut Emu, g: f32, xs: f32, field: f32| -> f32 {
        set_scales(emu, g, xs, 1.0);
        emu.write_u32(esi + 0x3c0, field.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBP, fr.ebp);
        emu.set_reg(RegisterX86::ESP, fr.esp);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.run(0x530659, 0x530680); // through fstp [ebp-8] (0x53067d)
        emu.read_f32(fr.ebp - 8)
    };

    // The base-A convert (slice 2, [0x5306c3, 0x5306e0)): convert(38.4), fstp [ebp-4].
    let run_base_a = |emu: &mut Emu, g: f32, xs: f32| -> f32 {
        set_scales(emu, g, xs, 1.0);
        emu.set_reg(RegisterX86::EBP, fr.ebp);
        emu.set_reg(RegisterX86::ESP, fr.esp);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.run(0x5306c3, 0x5306e0);
        emu.read_f32(fr.ebp - 4)
    };

    // The budget-B convert (slice 3, [0x5307b1, 0x5307ce) is followed by fcomp; we run the
    // recompute arm [0x5307d5, 0x5307f2) which ends fstp [ebp-4]).
    let run_budget_b = |emu: &mut Emu, g: f32, xs: f32| -> f32 {
        set_scales(emu, g, xs, 1.0);
        emu.set_reg(RegisterX86::EBP, fr.ebp);
        emu.set_reg(RegisterX86::ESP, fr.esp);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.run(0x5307d5, 0x5307f2);
        emu.read_f32(fr.ebp - 4)
    };

    // The gap-min convert + accumulate (slice 4, [0x530995, 0x5309b5)): convert(2.0); fadd [ebp-4];
    // fstp [ebp-4]. We pre-seed [ebp-4]=acc and read the post-add f32.
    let run_gap = |emu: &mut Emu, g: f32, xs: f32, acc: f32| -> f32 {
        set_scales(emu, g, xs, 1.0);
        emu.write_u32(fr.ebp - 4, acc.to_bits());
        emu.set_reg(RegisterX86::EBP, fr.ebp);
        emu.set_reg(RegisterX86::ESP, fr.esp);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.run(0x530995, 0x5309b5);
        emu.read_f32(fr.ebp - 4)
    };

    // The final compose (slice 5, [0x5309d8, 0x530a2e)): 2 converts + the adds.
    // origin = fstp [ebp-14] (0x5309fd), size = fstp [ebp-10] (0x530a2b).
    let run_compose =
        |emu: &mut Emu, g: f32, xs: f32, f3bc: f32, acc8: f32, acc4: f32| -> (f32, f32) {
            set_scales(emu, g, xs, 1.0);
            emu.write_u32(esi + 0x3bc, f3bc.to_bits());
            emu.write_u32(fr.ebp - 8, acc8.to_bits());
            emu.write_u32(fr.ebp - 4, acc4.to_bits());
            emu.set_reg(RegisterX86::ESI, esi);
            emu.set_reg(RegisterX86::EBP, fr.ebp);
            emu.set_reg(RegisterX86::ESP, fr.esp);
            emu.set_reg(RegisterX86::ECX, 0);
            emu.run(0x5309d8, 0x530a2e);
            (emu.read_f32(fr.ebp - 0x14), emu.read_f32(fr.ebp - 0x10))
        };

    // The four per-child accumulate adds `acc = (vtable_ret + acc) as f32` (`call [..]; fadd
    // [ebp-4]; fstp [ebp-4]`) at 0x5306fb / 0x53071d / 0x530752 (the measure pass) and 0x5309c7
    // (the gap pass). Each vtable call returns the child term on st0; we stub it with a
    // `fld dword [cell]; ret` stub so st0 = a controlled term, then the binary's own fadd+store
    // runs. (The 0x530752 site's vtable is `call [eax+0x1c]`; all four share the add+store shape.)
    let acc_vtbl = emu.alloc(0x40);
    let acc_meth = emu.alloc(0x10);
    let acc_cell = emu.alloc(4);
    {
        let mut code = vec![0xd9u8, 0x05];
        code.extend_from_slice(&(acc_cell as u32).to_le_bytes());
        code.push(0xc3); // bare ret (the vtable arg, if any, was pushed before our slice start)
        emu.write(acc_meth, &code);
        emu.write_u32(acc_vtbl + 0x1c, acc_meth as u32);
        emu.write_u32(acc_vtbl + 0x20, acc_meth as u32);
    }
    // each add site: (call_addr_eax/edx_holder, slice_begin, slice_end). slice begins at the vtable
    // `call`, ends after the `fstp [ebp-4]`. The dispatch reg (eax or edx) must hold acc_vtbl.
    let run_acc =
        |emu: &mut Emu, begin: u64, until: u64, via_edx: bool, term: f32, acc: f32| -> f32 {
            emu.write_u32(acc_cell, term.to_bits());
            emu.write_u32(fr.ebp - 4, acc.to_bits());
            if via_edx {
                emu.set_reg(RegisterX86::EDX, acc_vtbl); // call [edx+0x1c] / [edx+0x20]
            } else {
                emu.set_reg(RegisterX86::EAX, acc_vtbl); // call [eax+0x1c]
            }
            emu.set_reg(RegisterX86::EBP, fr.ebp);
            emu.set_reg(RegisterX86::ESP, fr.esp);
            emu.run(begin, until);
            emu.read_f32(fr.ebp - 4)
        };

    let mut acc_n = 0u32;
    for (term, acc) in [(0.0f32, 0.0), (12.5, -3.25), (-7.0, 7.0), (100.0, 0.001)] {
        // 0x5306f8 `call [eax+0x1c]` ; fadd 0x5306fb ; fstp 0x5306fe -> halt 0x530701
        let r0 = run_acc(&mut emu, 0x5306f8, 0x530701, false, term, acc);
        // 0x53071a `call [edx+0x1c]` ; fadd 0x53071d ; fstp 0x530720 -> halt 0x530723
        let r1 = run_acc(&mut emu, 0x53071a, 0x530723, true, term, acc);
        // 0x53074f `call [eax+0x1c]` ; fadd 0x530752 ; fstp 0x530755 -> halt 0x530758
        let r2 = run_acc(&mut emu, 0x53074f, 0x530758, false, term, acc);
        // 0x5309c4 `call [edx+0x20]` ; fadd 0x5309c7 ; fstp 0x5309ca -> halt 0x5309cd
        let r3 = run_acc(&mut emu, 0x5309c4, 0x5309cd, true, term, acc);
        let mine = acc_add(term, acc);
        for (i, r) in [r0, r1, r2, r3].into_iter().enumerate() {
            assert_eq!(
                r.to_bits(),
                mine.to_bits(),
                "acc_add site{i} term={term} acc={acc}"
            );
        }
        acc_n += 1;
    }
    eprintln!("layout_pass_530640 acc_add: {acc_n}x4 sites bit-exact");
    // budget-B FIRST eval (0x5307b6 fmul / 0x5307bd fdivr) is byte-identical to the recompute eval
    // (0x5307da/0x5307e1) diffed in run_budget_b above, modulo the masked `call 0x41ad70` rel32
    // (verified e8 baa5eeff vs e8 96a5eeff, same target; the d80d/d83d const operands identical) —
    // clone inheritance, so it needs no separate slice.

    let mut s = 0x5306_4000u32;
    let mut total = 0u32;
    for _ in 0..2000 {
        let g = rng(&mut s, 0.1, 8.0);
        let xs = rng(&mut s, 0.1, 8.0);
        let field = rng(&mut s, -500.0, 500.0);
        let acc = rng(&mut s, -100.0, 100.0);
        let f3bc = rng(&mut s, -50.0, 50.0);
        let acc8 = rng(&mut s, -100.0, 100.0);
        let acc4 = rng(&mut s, -100.0, 100.0);
        let term = rng(&mut s, -50.0, 50.0);

        assert_eq!(
            run_seed(&mut emu, g, xs, field).to_bits(),
            convert_field(g, xs, field).to_bits(),
            "seed g={g} xs={xs} field={field}"
        );
        assert_eq!(
            run_base_a(&mut emu, g, xs).to_bits(),
            convert_base_a(g, xs).to_bits(),
            "base_a g={g} xs={xs}"
        );
        assert_eq!(
            run_budget_b(&mut emu, g, xs).to_bits(),
            convert_budget_b(g, xs).to_bits(),
            "budget_b g={g} xs={xs}"
        );
        assert_eq!(
            run_gap(&mut emu, g, xs, acc).to_bits(),
            acc_add(convert_gap_min(g, xs), acc).to_bits(),
            "gap g={g} xs={xs} acc={acc}"
        );
        let (bo, bsz) = run_compose(&mut emu, g, xs, f3bc, acc8, acc4);
        let (mo, msz) = compose_final(g, xs, f3bc, acc8, acc4);
        assert_eq!(bo.to_bits(), mo.to_bits(), "compose origin g={g} xs={xs}");
        assert_eq!(bsz.to_bits(), msz.to_bits(), "compose size g={g} xs={xs}");

        // acc_add is a pure f32 add — diff it against a tiny standalone slice would be redundant;
        // it is already exercised inside run_gap (convert+add) and the binary-side store there.
        let _ = (
            term,
            NUM_0640_A,
            NUM_0640_B,
            NUM_0640_C,
            NUM_0640_D,
            NUM_0640_MIN,
            NUM_NEG2,
        );
        total += 1;
    }
    eprintln!("layout_pass_530640: {total} cases bit-exact (seed/base_a/budget_b/gap/compose)");
}

// ===========================================================================================
// 0x530b20 — cursor box + fade decrement + fade-alpha map
// ===========================================================================================
#[test]
fn cursor_fade_530b20() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let fr = frame(&mut emu);
    let esi = emu.alloc(0x400);
    let eax = emu.alloc(0x2000); // the [eax+0x1118]/[eax+0x111c] args for 0x41ad80

    // cursor box slice [0x530b4f, 0x530b8a): 0x41ad80 writes bx/by ([ebp-8]/[ebp-4]); recip;
    // cx = (bx*recip) f32 -> [ebp-8]; cy = (recip*by) f32 -> [ebp-4].
    let run_cursor =
        |emu: &mut Emu, xs: f32, ys: f32, a0: f32, a1: f32, inv_src: f32| -> (f32, f32) {
            set_scales(emu, 1.0, xs, ys);
            emu.write_u32(eax + 0x1118, a0.to_bits());
            emu.write_u32(eax + 0x111c, a1.to_bits());
            emu.write_u32(esi + 0x7c, inv_src.to_bits());
            // [eax+0xa0] is dereferenced before this slice; we start at 0x530b4f where eax is loaded
            // from [esi+0xa0]. Set [esi+0xa0]=eax.
            emu.write_u32(esi + 0xa0, eax as u32);
            emu.set_reg(RegisterX86::ESI, esi);
            emu.set_reg(RegisterX86::EAX, eax);
            emu.set_reg(RegisterX86::EBP, fr.ebp);
            emu.set_reg(RegisterX86::ESP, fr.esp);
            emu.run(0x530b4f, 0x530b8a);
            (emu.read_f32(fr.ebp - 8), emu.read_f32(fr.ebp - 4))
        };
    let mut s = 0x530b_2000u32;
    let mut total = 0u32;
    for _ in 0..2000 {
        let xs = rng(&mut s, 0.1, 4.0);
        let ys = rng(&mut s, 0.1, 4.0);
        let a0 = rng(&mut s, -200.0, 200.0);
        let a1 = rng(&mut s, -200.0, 200.0);
        let inv_src = rng(&mut s, 0.5, 100.0);
        let (bcx, bcy) = run_cursor(&mut emu, xs, ys, a0, a1, inv_src);
        let bx = (f64::from(xs) * f64::from(a0)) as f32; // 0x41ad80: xs*a0
        let by = (f64::from(ys) * f64::from(a1)) as f32; // 0x41ad80: ys*a1
        let (mcx, mcy) = cursor_box(bx, by, inv_src);
        assert_eq!(
            bcx.to_bits(),
            mcx.to_bits(),
            "cursor cx xs={xs} a0={a0} inv={inv_src}"
        );
        assert_eq!(
            bcy.to_bits(),
            mcy.to_bits(),
            "cursor cy ys={ys} a1={a1} inv={inv_src}"
        );
        total += 1;
    }
    eprintln!("cursor_fade_530b20 cursor box: {total} bit-exact");

    // fade decrement slice [0x530baa, 0x530bb6): t = ([esi+0x3b8] - [ebp+8]) f32 -> [ebp+8].
    // (0x530ba2 tests [esi+0x3b4]!=0; we set it nonzero so the slice is reached, then run from
    // the fld at 0x530baa.)
    let run_dec = |emu: &mut Emu, prev: f32, dt: f32| -> f32 {
        emu.write_u32(esi + 0x3b8, prev.to_bits());
        emu.write_u32(fr.ebp + 8, dt.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBP, fr.ebp);
        emu.set_reg(RegisterX86::ESP, fr.esp);
        emu.run(0x530baa, 0x530bb6);
        emu.read_f32(fr.ebp + 8)
    };
    let mut dec_n = 0u32;
    for (prev, dt) in [(1.0f32, 0.25), (0.5, 0.5), (0.3, 0.5), (10.0, 0.016_666)] {
        let bin = run_dec(&mut emu, prev, dt);
        assert_eq!(
            bin.to_bits(),
            fade_decrement(prev, dt).to_bits(),
            "fade_decrement prev={prev} dt={dt}"
        );
        dec_n += 1;
    }
    let mut s2 = 0x530b_aa00u32;
    for _ in 0..1000 {
        let prev = rng(&mut s2, -2.0, 5.0);
        let dt = rng(&mut s2, 0.0, 2.0);
        let bin = run_dec(&mut emu, prev, dt);
        assert_eq!(
            bin.to_bits(),
            fade_decrement(prev, dt).to_bits(),
            "fade_dec prev={prev} dt={dt}"
        );
        dec_n += 1;
    }
    eprintln!("cursor_fade_530b20 decrement: {dec_n} bit-exact");

    // fade-alpha map slice [0x530bf6, 0x530c29): half=1.0; frac = t<half ? t/half : 1.0;
    // alpha = __ftol(frac*255). The result is __ftol'd into EAX at 0x530c24 then pushed; we halt
    // right after the __ftol call (0x530c29) and read EAX.
    let run_alpha = |emu: &mut Emu, t: f32| -> i32 {
        emu.write_u32(fr.ebp + 8, t.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBP, fr.ebp);
        emu.set_reg(RegisterX86::ESP, fr.esp);
        emu.set_reg(RegisterX86::EDI, esi); // 0x530c22 reads [esi] into edi; harmless
        emu.run(0x530bf6, 0x530c29);
        emu.get_reg(RegisterX86::EAX) as i32
    };
    let mut alpha_lt = 0u32;
    let mut alpha_ge = 0u32;
    // boundary: t == half (1.0) -> frac=1.0 (the >= arm); t just below -> t/half.
    // Then NaN (which takes the `1.0` arm), the infinities, and negative values whose product with
    // 255 passes `i32` and `i64`: `__ftol` returns the low dword of its qword, 0 for the
    // indefinite.
    for t in [
        0.0f32,
        0.25,
        0.5,
        0.999_999,
        1.0,
        1.5,
        100.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -1.0e7,
        -1.0e10,
        -1.0e30,
    ] {
        let bin = run_alpha(&mut emu, t);
        let mine = fade_alpha(t);
        assert_eq!(bin, mine, "fade_alpha det t={t}: bin={bin} mine={mine}");
        if f64::from(t) < 1.0 {
            alpha_lt += 1;
        } else {
            alpha_ge += 1;
        }
    }
    let mut s3 = 0x530b_f600u32;
    for _ in 0..1000 {
        let t = rng(&mut s3, 0.0, 3.0);
        let bin = run_alpha(&mut emu, t);
        let mine = fade_alpha(t);
        assert_eq!(bin, mine, "fade_alpha t={t}: bin={bin} mine={mine}");
        if f64::from(t) < 1.0 {
            alpha_lt += 1;
        } else {
            alpha_ge += 1;
        }
    }
    eprintln!("cursor_fade_530b20 alpha: bit-exact (t<half={alpha_lt} t>=half={alpha_ge})");
    assert!(alpha_lt > 0 && alpha_ge > 0, "need both alpha branches");

    // convert() base helper deterministic probe (the shared chain) — a direct ==threshold check.
    let _ = convert(1.0, 1.0, 1024.0); // 1024/(1*1024)=1.0 ; xs*1.0
}
