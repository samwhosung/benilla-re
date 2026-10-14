//! The unit method kernels of `src/unitm2_fp.rs` against the exe. Slices are read at their
//! `fnstsw ax` or at the store they end in, with the registers and frame set directly:
//! - `0x610300`, `[0x610376, 0x610389)`: the arrival radius `dist * 0.9`;
//! - `0x60a740`, `[0x60a7a3, 0x60a7b4)` and `[0x60a7b9, 0x60a7c7)`: the two occlusion compares;
//! - `0x60d670`, `[0x60d70b, 0x60d721)` and `[0x60d726, 0x60d738)`: the rate gate and the rescale;
//! - `0x611130`, `[0x6111a3, 0x61120b)`: the goal radius `sqrt(radius^2)`;
//! - `0x60cbf0`, `[0x60cc42, 0x60cc65)`: the fade parameter through `__ftol`;
//! - `0x60aee0`, `[0x60af4f, 0x60af57)`, `[0x60af5c, 0x60af64)` and `[0x60af7c, 0x60afac)`: the
//!   two extent compares and the scale.
//!
//! `0x60cb50`, `0x610da0` (with `atan2` `0x7405a0` stubbed to pass `st0` through), `0x6103d0`
//! (with the angle wrap `0x455b30` run for real and the facing a `st0` island) and `0x60a110`
//! (with the render leaves `0x7bf4b0` and `0x7bdd00` stubbed to capture their arguments) run
//! whole. Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step
//! either side, to its random inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::unitm2_fp::*;
use common::*;

/// Plant a vtable code-island returning an f32 in st0: `fld dword[slot]; ret`. Returns
/// `(island_va, slot_va)`; the test writes the desired st0 value into `slot` before each call.
fn st0_island(e: &mut Emu) -> (u64, u64) {
    let island = e.alloc(0x10);
    let slot = e.alloc(0x10);
    let mut code = vec![0xd9u8, 0x05]; // fld dword ptr [slot]
    code.extend_from_slice(&(slot as u32).to_le_bytes());
    code.push(0xc3); // ret
    e.write(island, &code);
    (island, slot)
}

const ARRIVAL_GLOBAL: u64 = 0xc4d998;

#[test]
fn interact_arrival_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x6103_0000u32;

    let case = |e: &mut Emu, dist: f32, ctx: &str| {
        e.write_u32(ebp + 0xc, dist.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x610376, 0x610389);
        let bin = e.read_f32(ARRIVAL_GLOBAL);
        assert_f32(bin, interact_arrival_scale(dist), ctx);
    };
    case(&mut e, 0.0, "zero");
    case(&mut e, 1.0, "one");
    case(&mut e, -2.5, "neg");
    case(&mut e, 1e6, "big");
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
    for i in 0..3000u32 {
        let d = if i % 3 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1e5, 1e5)
        };
        case(&mut e, d, &format!("fuzz{i}"));
    }
}

#[test]
fn occlusion_z_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let cam = e.alloc(0x20);
    let mut s = 0x60a7_4000u32;
    let mut seen1 = std::collections::BTreeSet::new();
    let mut seen2 = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, probe_z: f32, cam_z: f32, ctx: &str| {
        e.write_u32(ebp - 4, probe_z.to_bits());
        e.write_u32(cam + 8, cam_z.to_bits());
        e.write_u32(ebp + 8, cam as u32); // the slice's `mov ecx,[ebp+8]` reloads cam from the arg.
                                          // test1 (slice starts with `fld [ebp-4]; mov ecx,[ebp+8]; fadd; fcomp [ecx+8]`).
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60a7a3, 0x60a7b4);
        let ah1 = ah(e) & 0x41;
        // test2: `fld [ebp-4]; fsub [ecx+8]; fcomp 2/3` — ECX still holds cam from test1's reload,
        // but re-establish it (set [ebp+8] persists; the mov is not in this slice — set ECX).
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ECX, cam);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // fld; fsub; fcomp [0x80abfc] (6 bytes → fnstsw at 0x60a7c5); stop after fnstsw (0x60a7c7).
        e.run(0x60a7b9, 0x60a7c7);
        let ah2 = ah(e) & 0x05;
        seen1.insert(ah1);
        seen2.insert(ah2);
        // Reconstruct the binary's `al`: test1 `jne` (ah1 != 0) → fail; else test2 `jp` (even
        // parity of ah2) → fail; else pass.
        let test1_fail = ah1 != 0;
        let test2_fail =
            (ah2 & 0x01).count_ones() + (ah2 & 0x04).count_ones() == 0 || (ah2 == 0x05); // 0 bits (even) or 2 bits (even) → jp taken → fail
        let bin_al = !test1_fail && !test2_fail;
        let ours = occlusion_z_gate(probe_z, cam_z);
        assert_eq!(
            bin_al, ours,
            "{ctx}: bin_al={bin_al} ours={ours} ah1={ah1:#04x} ah2={ah2:#04x} (pz={probe_z} cz={cam_z})"
        );
    };

    // straddle the two thresholds: cam_z, cam_z - 0.01 (test1 boundary), cam_z + 2/3 (test2).
    case(&mut e, 0.0, 0.0, "equal");
    case(&mut e, 1.0, 0.0, "above-far"); // p+0.01>0 pass1; p-0=1 >= 2/3 → fail2
    case(&mut e, 0.5, 0.0, "above-near"); // 0.5 < 2/3 → pass2
    case(&mut e, -1.0, 0.0, "below"); // p+0.01<0 → fail1
                                      // `probe_z + 0.01 == cam_z` (the probe at cam_z - 0.01 in f64) and `probe_z - cam_z == 2/3`,
                                      // each with a step either side; then NaN, infinities and zeros in both inputs.
    let bias = Z_BIAS_0P01;
    let cz = 3.0f32;
    for pz in [cz - bias, next_up(cz - bias), next_down(cz - bias)] {
        case(&mut e, pz, cz, "test1 edge");
    }
    for pz in [
        cz + TWO_THIRDS,
        next_up(cz + TWO_THIRDS),
        next_down(cz + TWO_THIRDS),
    ] {
        case(&mut e, pz, cz, "test2 edge");
    }
    for &v in &specials(&[0.01, 2.0 / 3.0]) {
        case(&mut e, v, 0.0, "special probe");
        case(&mut e, 0.5, v, "special cam");
        case(&mut e, v, v, "special both");
    }
    for i in 0..5000u32 {
        let cz = frand(&mut s, -10.0, 10.0);
        // probe near cz so both branches fire densely (offsets around the 0.01 and 2/3 edges).
        let off = match i % 4 {
            0 => frand(&mut s, -0.02, 0.02),
            1 => frand(&mut s, 0.6, 0.7),
            2 => frand(&mut s, -2.0, 2.0),
            _ => frand_bits(&mut s),
        };
        case(&mut e, cz + off, cz, &format!("fuzz{i}"));
    }
    assert!(seen1.contains(&0x00), "test1 pass (ah&0x41==0) never seen");
    assert!(
        seen1.contains(&0x01) || seen1.contains(&0x41),
        "test1 fail never seen"
    );
    assert!(seen2.contains(&0x01), "test2 pass (C0 alone) never seen");
    assert!(seen2.contains(&0x00), "test2 fail (>=) never seen");
}

#[test]
fn anim_timer_rate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let fields = e.alloc(0x400); // EBX; rate at +0x22c
    let mut s = 0x60d6_7000u32;
    let mut seen = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, rate: f32, base: i32, ctx: &str| {
        e.write_u32(fields + 0x22c, rate.to_bits());
        // gate AH
        e.set_reg(RegisterX86::EBX, fields);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60d70b, 0x60d721);
        let ah_gate = ah(e) & 0x05;
        seen.insert(ah_gate);
        // The gate: `jnp skip` (PF clear = odd parity) skips rescale → keep base. Reconstruct:
        let bits = (ah_gate & 0x01).count_ones() + (ah_gate & 0x04).count_ones();
        let skip = bits % 2 == 1; // odd parity → jnp taken → skip
        let bin_result = if skip {
            base
        } else {
            // rescale slice
            e.write_u32(ebp + 8, base as u32);
            e.set_reg(RegisterX86::EBX, fields);
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.run(0x60d726, 0x60d738);
            e.read_u32(ebp - 4) as i32
        };
        assert_eq!(
            bin_result,
            anim_timer_rate(rate, base),
            "{ctx}: bin={bin_result} ours={} (rate={rate} base={base})",
            anim_timer_rate(rate, base)
        );
    };

    // rate == 1.0 → skip; rate far from 1 → rescale; rate near 1±eps → boundary.
    case(&mut e, 1.0, 1000, "rate=1 skip");
    case(&mut e, 2.0, 1000, "rate=2 → 2000");
    case(&mut e, 0.5, 1000, "rate=0.5 → 500");
    case(&mut e, 1.0000001, 1000, "rate ~1 within eps?");
    case(&mut e, 1.5, 7, "round-edge"); // 10.5 → 10 (ties-even)
    case(&mut e, 2.5, 5, "ties"); // 12.5 → 12 (ties-even)
                                  // The dead band `|rate - 1| == 2^-22` and a step either side, the integers at their extremes,
                                  // products past `i32` (stored as the indefinite 0x80000000), and NaN, infinities and zeros.
    let eps = RATE_EPS;
    for r in [
        1.0 + eps,
        next_up(1.0 + eps),
        next_down(1.0 + eps),
        1.0 - eps,
        next_down(1.0 - eps),
    ] {
        case(&mut e, r, 1000, "eps edge");
    }
    for &v in &specials(&[1.0]) {
        for base in [0, 1, -1, 1000, i32::MAX, i32::MIN, 0x4000_0000] {
            case(&mut e, v, base, "special rate");
        }
    }
    for r in [1.0e10f32, -1.0e10, 3.0e9, 2147483648.0, 2.0] {
        case(&mut e, r, 1_500_000_000, "overflow");
    }
    for i in 0..4000u32 {
        let rate = match i % 4 {
            0 => frand(&mut s, 0.0, 4.0),
            1 => frand(&mut s, 1.0 - 1e-6, 1.0 + 1e-6), // straddle the eps dead-band
            2 => frand_bits(&mut s),
            _ => frand(&mut s, -2.0, 2.0),
        };
        let base = (next(&mut s) >> 8) as i32 % 100_000 - 50_000;
        case(&mut e, rate, base, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&0x01), "skip (C0, |rate-1|<eps) never seen");
    assert!(seen.contains(&0x00), "rescale (|rate-1|>=eps) never seen");
}

const RADIUS_SQ_GLOBAL: u64 = 0xc4d9c0;
const RADIUS_GLOBAL: u64 = 0xc4d910;

#[test]
fn move_cast_radius_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let ptr = e.alloc(0x20); // EAX/EDI: a readable vec3-ish block
    let mut s = 0x6111_3000u32;

    let case = |e: &mut Emu, rsq: f32, ctx: &str| {
        e.write_u32(RADIUS_SQ_GLOBAL, rsq.to_bits());
        e.set_reg(RegisterX86::EAX, ptr);
        e.set_reg(RegisterX86::EDI, ptr);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 0xc, ptr as u32);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6111a3, 0x61120b);
        let bin = e.read_f32(RADIUS_GLOBAL);
        assert_f32(bin, move_cast_radius(rsq), ctx);
    };
    case(&mut e, 0.0, "zero");
    case(&mut e, 1.0, "one");
    case(&mut e, 4.0, "four→2");
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
    case(&mut e, 2.0, "two");
    case(&mut e, 1e10, "big");
    for i in 0..3000u32 {
        let rsq = if i % 3 == 0 {
            frand_bits(&mut s).abs()
        } else {
            frand(&mut s, 0.0, 1e6)
        };
        case(&mut e, rsq, &format!("fuzz{i}"));
    }
}

#[test]
fn color_fade_t_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x60cb_f000u32;

    // The live path reaches the FP only with `esi >= 0` (0x60cc1a `jns` taken) and forms the
    // dividend `fild qword(esi, ebx=0)` — esi is a NON-NEGATIVE elapsed-into-fade count; the binary
    // zero-extends it (ebx=0). So the diffed domain is esi in [0, dur).
    let case = |e: &mut Emu, esi: u32, dur: i32, ctx: &str| {
        e.set_reg(RegisterX86::ESI, u64::from(esi));
        e.set_reg(RegisterX86::EBX, 0);
        e.set_reg(RegisterX86::EDX, dur as u32 as u64);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60cc42, 0x60cc65);
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        let ours = color_fade_t(i64::from(esi), dur); // esi zero-extended (ebx=0)
        assert_eq!(
            bin, ours,
            "{ctx}: bin={bin} ours={ours} (esi={esi} dur={dur})"
        );
    };

    // esi = now - (start+dur1) in [0, dur); dur = the fade duration (>0). t in [0,255].
    case(&mut e, 0, 1000, "esi=0 → t=255");
    case(&mut e, 1000, 1000, "esi=dur → t=0");
    case(&mut e, 500, 1000, "half → t~127");
    case(&mut e, 1, 256, "small");
    // A zero or negative duration, a zero-extended `esi` past `i32`, and `esi > dur` (negative t).
    for dur in [0, -1, -1000, i32::MIN, i32::MAX, 1] {
        for esi in [0u32, 1, 500, 1000, u32::MAX, 0x8000_0000, 4_000_000_000] {
            case(&mut e, esi, dur, "extremes");
        }
    }
    for i in 0..5000u32 {
        let dur = ((next(&mut s) >> 8) % 100_000) as i32 + 1; // dur != 0
        let esi = (next(&mut s) >> 8) % (dur as u32 + 1); // esi in [0, dur]
        case(&mut e, esi, dur, &format!("fuzz{i}"));
    }
    // also drive esi past dur (t < 0) and large magnitudes (out-of-i32 truncation).
    for i in 0..1000u32 {
        let dur = ((next(&mut s) >> 8) % 1_000_000) as i32 + 1;
        let esi = (next(&mut s) >> 8) % 4_000_000;
        case(&mut e, esi, dur, &format!("wide{i}"));
    }
}

const COLOR_FADE_INTERP: u64 = 0x60cbf0; // stubbed: write-through arg0 = &[ebp-4]

#[test]
fn fade_model_rgb_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let model = e.alloc(0x400); // this+0xd8 = model; +0x190 receives the RGB triple.
    let payload = e.alloc(0x10); // the 32-bit fade color the stub writes into [ebp-4].
    e.write_u32(this + 0xd8, model as u32);
    // this+0xce0 fallback ptr (used only when 0x60cbf0 returns false; we return true so unused).
    e.write_u32(this + 0xce0, 0);
    // 0x60cbf0(this, &[ebp-4]): __thiscall, stack arg0 = &out-color. Write the chosen color through
    // arg0 (4 bytes), return al=1, ret 8 (pop 2 stack args).
    e.stub_write_through(COLOR_FADE_INTERP, 0, payload, 4, 1, 2);
    // 0x710cf0(model, &triple): let it run (copies the float triple to model+0x190). It is a thin
    // memcpy to the light; it can read model fields, and the allocation is zeroed.

    let check = |e: &mut Emu, color: u32, ctx: &str| {
        // Make the change-test pass: this[+0xd04] (the last-applied color) != our color.
        e.write_u32(this + 0xd04, color ^ 0xffff_ffff);
        e.write_u32(payload, color);
        e.write(model + 0x190, &[0u8; 12]);
        let _ = e.call(0x60cb50, this as u32, 0, &[0u32]);
        let bin = [
            e.read_f32(model + 0x190),
            e.read_f32(model + 0x194),
            e.read_f32(model + 0x198),
        ];
        // Determine which 3 bytes of `color` land (in light order) by reading the binary back, then
        // assert our transcription reproduces that exact triple. The byte→channel order is pinned
        // empirically: feed distinct byte values so a wrong pluck is caught.
        let bytes_light = bin_bytes(bin);
        let ours = fade_model_rgb(bytes_light);
        for i in 0..3 {
            assert_f32(bin[i], ours[i], &format!("{ctx} chan[{i}]"));
        }
    };

    check(&mut e, 0x0000_0000, "all 0");
    check(&mut e, 0x00ff_ffff, "rgb 255");
    check(&mut e, 0x0001_0203, "1,2,3");
    check(&mut e, 0xaabb_ccdd, "distinct");
    let mut s = 0x60cb_5000u32;
    for i in 0..4000u32 {
        check(&mut e, next(&mut s), &format!("fuzz{i}"));
    }
}

/// Recover the three light-order bytes from the three light dwords (each = byte * 1/255), so the
/// transcription is checked on exactly the bytes the binary plucked (round-trip via *255).
fn bin_bytes(light: [f32; 3]) -> [u8; 3] {
    light.map(|f| (f64::from(f) * 255.0).round() as i64 as u8)
}

#[test]
fn camera_model_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let this = e.alloc(0x2000); // EBX; compute writes +0xcf0
    let mut s = 0x60ae_e000u32;
    let mut seen_g1 = std::collections::BTreeSet::new();
    let mut seen_g2 = std::collections::BTreeSet::new();

    // The body has TWO `ret 4` exits (default branch 0x60af79, compute 0x60afac), so a single
    // `run` across them is impossible. The tests read the two gate status bytes (slices [0x60af4f,0x60af57) and
    // [0x60af5c,0x60af64), each ending after `fnstsw ax`) AND the compute arithmetic (slice
    // [0x60af7c,0x60afac), which is unconditionally reached from its entry). The composite branch
    // is reconstructed from the two AHs and checked against `camera_model_scale`.
    let setup = |e: &mut Emu, xhi: f32, xlo: f32, yhi: f32, ylo: f32| {
        e.write_u32(ebp - 0x18, xlo.to_bits());
        e.write_u32(ebp - 0x14, ylo.to_bits());
        e.write_u32(ebp - 0xc, xhi.to_bits());
        e.write_u32(ebp - 8, yhi.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
    };

    let mut case = |e: &mut Emu, xhi: f32, xlo: f32, yhi: f32, ylo: f32, ctx: &str| {
        // gate 1 AH: fld [ebp-0xc]=xhi; fcomp [ebp-0x18]=xlo; fnstsw.
        setup(e, xhi, xlo, yhi, ylo);
        e.run(0x60af4f, 0x60af57);
        let g1 = ah(e) & 0x44;
        // gate 2 AH: fld [ebp-8]=yhi; fcomp [ebp-0x14]=ylo; fnstsw.
        setup(e, xhi, xlo, yhi, ylo);
        e.run(0x60af5c, 0x60af64);
        let g2 = ah(e) & 0x44;
        seen_g1.insert(g1);
        seen_g2.insert(g2);
        // `jp compute` taken on EVEN parity of (C3|C2). g1 even → compute; else check g2.
        let even = |m: u8| (m & 0x40).count_ones() + (m & 0x04).count_ones();
        let g1_compute = even(g1) % 2 == 0;
        let g2_compute = even(g2) % 2 == 0;
        let bin_scale = if g1_compute || g2_compute {
            // compute slice (always reaches its own ret at 0x60afac).
            setup(e, xhi, xlo, yhi, ylo);
            e.run(0x60af7c, 0x60afac);
            e.read_f32(this + 0xcf0)
        } else {
            DEFAULT_SCALE
        };
        assert_f32(bin_scale, camera_model_scale(xhi, xlo, yhi, ylo), ctx);
    };

    case(&mut e, 1.0, 1.0, 2.0, 2.0, "both equal → default");
    case(&mut e, 3.0, 0.0, 4.0, 0.0, "compute 3,4");
    case(&mut e, 1.0, 1.0, 5.0, 0.0, "x equal, y differ → compute");
    case(&mut e, 0.0, 0.0, 0.0, 0.0, "all zero → default");
    // Equal bounds including -0 against +0 (equal), a step either side, and NaN, infinities and
    // zeros in each of the four.
    case(&mut e, -0.0, 0.0, 0.0, -0.0, "negative zero equal");
    for &v in &specials(&[1.0]) {
        case(&mut e, v, 1.0, 1.0, 1.0, "special xhi");
        case(&mut e, 1.0, v, 1.0, 1.0, "special xlo");
        case(&mut e, 1.0, 1.0, v, 1.0, "special yhi");
        case(&mut e, 1.0, 1.0, 1.0, v, "special ylo");
        case(&mut e, v, v, v, v, "special all");
    }
    for i in 0..5000u32 {
        let (xhi, xlo, yhi, ylo) = match i % 5 {
            0 => {
                let v = frand(&mut s, -10.0, 10.0);
                let w = frand(&mut s, -10.0, 10.0);
                (v, v, w, w) // both equal → default
            }
            1 => {
                let v = frand(&mut s, -10.0, 10.0);
                (v, v, frand(&mut s, -10.0, 10.0), frand(&mut s, -10.0, 10.0))
            }
            2 => (
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
            ),
            3 => (
                frand_bits(&mut s),
                frand_bits(&mut s),
                frand_bits(&mut s),
                frand_bits(&mut s),
            ),
            _ => (
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
            ),
        };
        case(&mut e, xhi, xlo, yhi, ylo, &format!("fuzz{i}"));
    }
    assert!(
        seen_g1.contains(&0x00),
        "gate1 ordered-unequal (compute) never seen"
    );
    assert!(
        seen_g1.contains(&0x40),
        "gate1 equal (fall-through) never seen"
    );
    assert!(seen_g2.contains(&0x40), "gate2 equal never seen");
}

const ATAN2: u64 = 0x7405a0;

#[test]
fn vec_to_facing_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pa = e.alloc(0x20);
    let pb = e.alloc(0x20);
    e.stub(ATAN2, 0, 0); // passthrough: leaves st0 = the loaded [ebp-8] value.

    let check = |e: &mut Emu, a: [f32; 3], b: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(pa + 4 * k as u64, a[k].to_bits());
            e.write_u32(pb + 4 * k as u64, b[k].to_bits());
        }
        // ecx = &a, edx = &b (the fsub operands: `fld [edx]; fsub [ecx]` = b - a).
        let bin = e.call_f32(0x610da0, pa as u32, pb as u32, &[]);
        assert_f32(bin, vec_to_facing(a, b), ctx);
    };

    check(&mut e, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], "due-x (planar)");
    check(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], "straight up");
    check(&mut e, [0.0, 0.0, 1.0], [0.0, 0.0, 0.0], "straight down");
    check(&mut e, [1.0, 2.0, 3.0], [4.0, 6.0, 3.0], "planar diag");
    // The gate `|n| < eps = 0.001` on the normalized planar part, with the planar offset at the
    // edge and a step either side, and NaN, infinities and zeros in each coordinate.
    let eps = EPS_1EM3;
    for x in [eps, next_up(eps), next_down(eps), -eps] {
        check(&mut e, [0.0; 3], [x, 0.0, 1.0], "gate x");
        check(&mut e, [0.0; 3], [0.0, x, 1.0], "gate y");
        check(&mut e, [0.0; 3], [x, x, -1.0], "gate xy");
    }
    for &v in &specials(&[eps]) {
        for k in 0..3 {
            let mut b = [0.5f32, 0.25, 1.0];
            b[k] = v;
            check(&mut e, [0.0; 3], b, "special b");
            let mut a = [0.0f32; 3];
            a[k] = v;
            check(&mut e, a, [0.5, 0.25, 1.0], "special a");
        }
    }
    check(&mut e, [1.0, 1.0, 1.0], [1.0, 1.0, 1.0], "coincident");
    check(&mut e, [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], "coincident up");
    let mut s = 0x610d_a000u32;
    for i in 0..5000u32 {
        let scale = if (next(&mut s) >> 8).is_multiple_of(2) {
            1.0
        } else {
            100.0
        };
        let a = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        let b = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        check(&mut e, a, b, &format!("fuzz{i}"));
    }
    // vertical-dominant cases (tiny planar, big dz) to exercise both ±π/2 branches.
    for i in 0..1000u32 {
        let z = frand(&mut s, -50.0, 50.0);
        let a = [0.0, 0.0, 0.0];
        let b = [frand(&mut s, -1e-5, 1e-5), frand(&mut s, -1e-5, 1e-5), z];
        check(&mut e, a, b, &format!("vert{i}"));
    }
}

const BEGIN_UPDATE: u64 = 0x60e8f0; // ret 8 (stack args: arg, goal)
const APPLY_FACING: u64 = 0x60e940; // the apply setter — capture arg1 = new facing
const TURN_RATE_GLOBAL: u64 = 0xc4d93c;

#[test]
fn facing_interp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let vt = e.alloc(0x100);
    let (island, facing_slot) = st0_island(&mut e);
    e.write_u32(this, vt as u32);
    e.write_u32(vt + 0x18, island as u32); // current-facing virtual
    e.stub(BEGIN_UPDATE, 0, 2); // 0x60e8f0: ret 8 (2 stack args)
    e.stub(APPLY_FACING, 1, 2); // 0x60e940(this, newFacing, arg): ret 8 (args[0]=arg, args[1]=new)

    let check = |e: &mut Emu, cur: f32, goal: f32, rate: f32, ctx: &str| {
        e.write_u32(facing_slot, cur.to_bits());
        e.write_u32(TURN_RATE_GLOBAL, rate.to_bits()); // pre-seed (skip recompute)
        e.clear_calls();
        let al = e.call(
            0x6103d0,
            this as u32,
            0,
            &[goal.to_bits(), rate.to_bits(), 0],
        ) & 0xff;
        let ours = facing_interp(cur, goal, rate);
        match ours {
            None => {
                assert_eq!(
                    al, 0,
                    "{ctx}: expected early-false but al={al} (cur={cur} goal={goal} rate={rate})"
                );
            }
            Some(new_facing) => {
                assert_eq!(al, 1, "{ctx}: expected apply (al=1) but al={al}");
                let call = e
                    .calls()
                    .iter()
                    .find(|c| c.addr == APPLY_FACING)
                    .expect("apply called");
                let bin_facing = f32::from_bits(call.args[1]); // arg1 = new facing
                assert_f32(bin_facing, new_facing, ctx);
            }
        }
    };

    // rate is the per-frame turn cap (radians). goal/cur are angles. Probe across the wrap and the
    // cap edges.
    check(&mut e, 0.0, 0.0, 0.1, "at goal → false");
    check(&mut e, 0.0, 1.0, 0.1, "small turn capped");
    check(&mut e, 0.0, 1.0, 5.0, "turn uncapped");
    check(&mut e, 0.0, 6.0, 0.1, "wrap short-way");
    check(&mut e, 3.0, 0.5, 0.2, "cross pi");
    // Delta equal to pi and to the dead band `0.001`, a step either side, the rate at the step,
    // and the specials. A NaN or infinite angle, and a rate that makes one, reach the C
    // runtime's `fmod` error path, which needs state the start-up sets and the emulator does not
    // run, and past about 1e15 the emulator's `fprem` rounds in `f64` where the CPU is exact.
    let pi = PI;
    for g in [
        pi,
        next_up(pi),
        next_down(pi),
        EPS_1EM3,
        next_up(EPS_1EM3),
        next_down(EPS_1EM3),
        TAU - EPS_1EM3,
        TAU,
    ] {
        check(&mut e, 0.0, g, 0.1, "delta edge");
        check(&mut e, 0.0, g, 10.0, "delta edge uncapped");
        check(&mut e, 0.0, g, 3.0, "delta edge rate");
    }
    for r in [1.0f32, next_up(1.0), next_down(1.0)] {
        check(&mut e, 0.0, 1.0, r, "rate equal step");
    }
    for &v in specials(&[PI, EPS_1EM3])
        .iter()
        .filter(|v| v.is_finite() && v.abs() < 1.0e9)
    {
        // A rate of exactly pi is the sentinel that makes the function recompute its turn rate
        // from a CVar, which the test does not set up.
        if v != PI {
            check(&mut e, 0.5, 1.0, v, "special rate");
        }
        check(&mut e, v, 1.0, 0.1, "special cur");
        check(&mut e, 0.5, v, 0.1, "special goal");
    }
    let mut s = 0x6103_d000u32;
    for i in 0..5000u32 {
        let cur = frand(&mut s, -7.0, 7.0);
        let goal = frand(&mut s, -7.0, 7.0);
        let rate = match i % 3 {
            0 => frand(&mut s, 0.0, 0.5),
            1 => frand(&mut s, 0.0, 7.0),
            _ => frand(&mut s, 0.0001, 0.01),
        };
        check(&mut e, cur, goal, rate, &format!("fuzz{i}"));
    }
}

const RENDER_ROT: u64 = 0x7bf4b0; // single call: args[0]=arg2, args[1]=arg1 (ret 0xc → pop 3)
const RENDER_SCALE: u64 = 0x7bdd00; // single call: args[0]=arg3 (ret 4 → pop 1)

#[test]
fn swing_transform_args_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let vt = e.alloc(0x100);
    let out_mat = e.alloc(0x100);
    let posvec = e.alloc(0x20); // the GetPosition result vec3 (the tail copies [eax],[eax+4],[eax+8])
    let (vc44_island, vc44_slot) = st0_island(&mut e);
    e.write_u32(this, vt as u32);
    e.write_u32(vt + 0x44, vc44_island as u32); // [vtable+0x44] scalar via st0-island
                                                // [vtable+0x14] GetPosition returns a C3Vector* in EAX (the tail does `mov edx,[eax]` etc.);
                                                // stub it to return `posvec` (a readable vec3); the tail copy is not float code.
    let getpos_island = e.alloc(0x10);
    e.write_u32(vt + 0x14, getpos_island as u32);
    e.write_u32(posvec, 7.0f32.to_bits());
    e.write_u32(posvec + 4, 8.0f32.to_bits());
    e.write_u32(posvec + 8, 9.0f32.to_bits());
    e.stub(getpos_island, posvec as u32, 1);
    // Render leaves: capture the pushed float args. 0x7bf4b0 ret 0xc (3 args), 0x7bdd00 ret 4.
    e.stub(RENDER_ROT, 0, 3);
    e.stub(RENDER_SCALE, 0, 1);

    let check = |e: &mut Emu,
                 facing: f32,
                 vc44: f32,
                 s90: f32,
                 s94: f32,
                 s98: f32,
                 s9c: f32,
                 dc: u32,
                 ctx: &str| {
        e.write_u32(this + 0x9c8, facing.to_bits());
        e.write_u32(vc44_slot, vc44.to_bits());
        e.write_u32(this + 0x90, s90.to_bits());
        e.write_u32(this + 0x94, s94.to_bits());
        e.write_u32(this + 0x98, s98.to_bits());
        e.write_u32(this + 0x9c, s9c.to_bits());
        e.write_u32(this + 0xdc, dc); // selects s_z = (dc!=0)? this[+0x9c] : this[+0x98]
        e.clear_calls();
        let _ = e.call(0x60a110, this as u32, 0, &[out_mat as u32]);
        let rot = e
            .calls()
            .iter()
            .find(|c| c.addr == RENDER_ROT)
            .copied()
            .expect("rotation builder called");
        let scale = e
            .calls()
            .iter()
            .find(|c| c.addr == RENDER_SCALE)
            .copied()
            .expect("scale builder called");
        // 0x7bf4b0 args: [0]=arg2, [1]=arg1 (pushed arg1 first, then arg2 on top).
        let bin_arg2 = f32::from_bits(rot.args[0]);
        let bin_arg1 = f32::from_bits(rot.args[1]);
        let bin_arg3 = f32::from_bits(scale.args[0]);
        let s_z = if dc != 0 { s9c } else { s98 };
        let (arg1, arg2, arg3) = swing_transform_args(facing, vc44, s94, s90, s_z);
        assert_f32(bin_arg1, arg1, &format!("{ctx} arg1"));
        assert_f32(bin_arg2, arg2, &format!("{ctx} arg2"));
        assert_f32(bin_arg3, arg3, &format!("{ctx} arg3"));
    };

    check(&mut e, 1.0, 0.5, 2.0, 3.0, 4.0, 5.0, 1, "dc!=0");
    check(&mut e, 1.0, 0.5, 2.0, 3.0, 4.0, 5.0, 0, "dc==0");
    check(&mut e, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1, "zeros");
    for &v in &specials(&[]) {
        check(&mut e, v, 0.5, 2.0, 3.0, 4.0, 5.0, 1, "special facing");
        check(&mut e, 1.0, v, 2.0, 3.0, 4.0, 5.0, 1, "special vc44");
        check(&mut e, 1.0, 0.5, v, 3.0, 4.0, 5.0, 1, "special 90");
        check(&mut e, 1.0, 0.5, 2.0, v, 4.0, 5.0, 1, "special 94");
        check(&mut e, 1.0, 0.5, 2.0, 3.0, v, 5.0, 0, "special 98");
        check(&mut e, 1.0, 0.5, 2.0, 3.0, 4.0, v, 1, "special 9c");
    }
    let mut s = 0x60a1_1000u32;
    for i in 0..3000u32 {
        check(
            &mut e,
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            if (next(&mut s) >> 8).is_multiple_of(2) {
                0
            } else {
                1
            },
            &format!("fuzz{i}"),
        );
    }
}
