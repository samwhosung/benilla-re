//! The lighting colours and gates against `0x672a20`, `0x6a8050`, `0x6c31e0` and `0x6952a0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::lightblend::{
    chunk_fade_scale, ctx_fog_gate_dispatch_second, daynight_ambient_color,
    daynight_directional_color, fog_indoor_color, fog_outdoor_color,
    lightstate_exterior_scaled_directional, lightstate_interior_ambient,
    lightstate_interior_directional,
};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes) for the slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

/// The object `0x6d48b0` returns.
const DAYNIGHT: u64 = 0xce9b60;

/// The value `0x672a20` compares the context's difference with.
const FOG_END_GLOBAL: u64 = 0xc62964;

/// The pointer to the light block `0x6a8050` reads outdoors.
const BLEND_BLOCK_PTR: u64 = 0xc89ef0;

const TEST_AH_5: u8 = 0x5;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any float, NaN and the infinities included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// A byte from the high bits.
fn brand(s: &mut u32) -> u8 {
    ((next(s) >> 8) & 0xff) as u8
}

/// AH after `fnstsw ax`.
fn ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// `GetMapObjectFog` (`0x6c31e0`) from `begin` to the `call 0x71c110` at `0x6c3300`, eax the object
/// `0x6d48b0` returns: the colour at `[ebp-0x10]` up.
fn run_fog_slice(e: &mut Emu, begin: u64, ebp: u64) -> [f32; 3] {
    e.set_reg(RegisterX86::EAX, DAYNIGHT);
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ESP, ebp - 0x40);
    e.run(begin, 0x6c3300);
    [
        e.read_f32(ebp - 0x10),
        e.read_f32(ebp - 0xc),
        e.read_f32(ebp - 0x8),
    ]
}

/// `ApplyEntityLighting` (`0x672a20`) with no context: the ambient from `0x672a7f` to its call at
/// `0x672ac7` and the directional from `0x672acc` to its call at `0x672b1b`, each read before the
/// next overwrites `[ebp-0xc]` up; esi the object `0x6d48b0` returns.
#[test]
fn apply_cb_null_color_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let target = e.alloc(0x200);
    let ebp = SLICE_ESP;
    let mut s = 0x672a_0000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, amb: [u8; 3], dir: [u8; 3], ctx: &str| {
        e.write(DAYNIGHT + 0x178, &amb);
        e.write(DAYNIGHT + 0x174, &dir);
        e.set_reg(RegisterX86::ESI, DAYNIGHT);
        e.set_reg(RegisterX86::EDI, target);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x672a7f, 0x672ac7);
        let bin_amb = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let ours_amb = daynight_ambient_color(amb[2], amb[1], amb[0]);
        e.set_reg(RegisterX86::ESI, DAYNIGHT);
        e.set_reg(RegisterX86::EDI, target);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x672acc, 0x672b1b);
        let bin_dir = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let ours_dir = daynight_directional_color(dir[2], dir[1], dir[0]);
        for k in 0..3 {
            assert_f32(bin_amb[k], ours_amb[k], &format!("{ctx} ambient[{k}]"));
            assert_f32(bin_dir[k], ours_dir[k], &format!("{ctx} directional[{k}]"));
        }
    };

    case(&mut e, [0, 0, 0], [0, 0, 0], "all zero");
    case(&mut e, [255, 255, 255], [255, 255, 255], "all max");
    case(&mut e, [1, 128, 254], [3, 127, 200], "spread");
    for i in 0..2000u32 {
        let amb = [brand(&mut s), brand(&mut s), brand(&mut s)];
        let dir = [brand(&mut s), brand(&mut s), brand(&mut s)];
        case(&mut e, amb, dir, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "apply_cb_null_color: {checked} cases bit-exact vs WoW.exe @ 0x672a20 \
         slices [0x672a7f,0x672ac7) + [0x672acc,0x672b1b) (DayNight @ 0xce9b60)"
    );
}

/// `0x672a20` with a context, `0x672a39` to the `jp` at `0x672a4a`: esi = the context, the call to
/// `[vtbl+8]` made when PF is clear.
#[test]
fn apply_cb_fog_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ctx = e.alloc(0x100);
    let mut s = 0x672a_3900u32;
    let mut seen = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, f78: f32, f68: f32, thr: f32, label: &str| {
        e.write_u32(ctx + 0x78, f78.to_bits());
        e.write_u32(ctx + 0x68, f68.to_bits());
        e.write_u32(FOG_END_GLOBAL, thr.to_bits());
        e.set_reg(RegisterX86::ESI, ctx);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x672a39, 0x672a4a); // to the `jp`, after `test ah,5`
        let bin = ah(e) & TEST_AH_5;
        let pf = (e.get_reg(RegisterX86::EFLAGS) >> 2) & 1;
        let ours = ctx_fog_gate_dispatch_second(f78, f68, thr);
        // `jp` jumps over the call when PF is set
        let dispatch = pf == 0;
        assert_eq!(
            dispatch, ours,
            "{label}: binary AH&5={bin:#04x} PF={pf} ours_dispatch={ours}"
        );
        seen.insert(bin);
    };

    // equal: C3 alone, an even count, so the call is skipped
    case(&mut e, 3.0, 1.0, 2.0, "equal (skip)");
    case(&mut e, 0.0, 0.0, 0.0, "0 − 0 = 0 (skip)");
    case(&mut e, 1.0, 0.0, 5.0, "below (call)");
    case(&mut e, 10.0, 0.0, 1.0, "above (skip)");
    case(&mut e, f32::NAN, 0.0, 1.0, "NaN field (skip)");
    case(&mut e, 0.0, 0.0, f32::NAN, "NaN threshold (skip)");

    let mut checked = 0u32;
    for i in 0..3000u32 {
        let (f78, f68, thr) = if i % 5 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -50.0, 50.0),
            )
        };
        case(&mut e, f78, f68, thr, &format!("fuzz{i}"));
        checked += 1;
    }
    assert!(seen.contains(&0x00), "no compare above or equal");
    assert!(seen.contains(&0x01), "no compare below");
    assert!(seen.contains(&0x05), "no unordered compare");
    eprintln!(
        "apply_cb_fog_gate: {checked} cases + probes bit-exact vs WoW.exe @ 0x672a20 from 0x672a39 to the jp"
    );
}

/// `CMapDoodadDef::AddLights` (`0x6a8050`) with `[this+0xc] & 2` set: the ambient from `0x6a8073` to
/// its call at `0x6a80bb` and the directional from `0x6a80c0` to `0x6a810a`, esi = this.
#[test]
fn add_lights_interior_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let ebp = SLICE_ESP;
    let mut s = 0x6a80_5000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, amb: [u8; 3], dir: [u8; 3], ctx: &str| {
        e.write(this + 0x9c, &amb);
        e.write(this + 0xa0, &dir);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x6a8073, 0x6a80bb);
        let bin_amb = [
            e.read_f32(ebp - 0x10),
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
        ];
        let ours_amb = lightstate_interior_ambient(amb[2], amb[1], amb[0]);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x6a80c0, 0x6a810a);
        let bin_dir = [
            e.read_f32(ebp - 0x10),
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
        ];
        let ours_dir = lightstate_interior_directional(dir[2], dir[1], dir[0]);
        for k in 0..3 {
            assert_f32(bin_amb[k], ours_amb[k], &format!("{ctx} ambient[{k}]"));
            assert_f32(bin_dir[k], ours_dir[k], &format!("{ctx} directional[{k}]"));
        }
    };

    case(&mut e, [0, 0, 0], [0, 0, 0], "all zero");
    case(&mut e, [255, 255, 255], [255, 255, 255], "all max");
    case(&mut e, [1, 128, 254], [3, 127, 200], "spread");
    for i in 0..2000u32 {
        let amb = [brand(&mut s), brand(&mut s), brand(&mut s)];
        let dir = [brand(&mut s), brand(&mut s), brand(&mut s)];
        case(&mut e, amb, dir, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "add_lights_interior: {checked} cases bit-exact vs WoW.exe @ 0x6a8050 \
         slices [0x6a8073,0x6a80bb) + [0x6a80c0,0x6a810a)"
    );
}

/// `0x6a8050` called whole with the flag clear, both colour calls stubbed: the scaled directional
/// colour passed to `0x71bce0`.
#[test]
fn add_lights_exterior_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let target = e.alloc(0x200);
    let block = e.alloc(0x200);
    e.write_u32(BLEND_BLOCK_PTR, block as u32);
    e.stub(0x71bc70, 0, 1);
    e.stub(0x71bce0, 0, 2);
    let mut s = 0x6a80_e000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, intensity: f32, comps: [f32; 3], label: &str| {
        e.write_u32(this + 0xc, 0x0); // outdoors, and no call to 0x6a79c0
        e.write_u32(this + 0x84, 0); // no point lights
        e.write_u32(this + 0xa4, intensity.to_bits());
        e.write_u32(block + 0xb8, comps[0].to_bits());
        e.write_u32(block + 0xbc, comps[1].to_bits());
        e.write_u32(block + 0xc0, comps[2].to_bits());
        e.clear_calls();
        e.call(0x6a8050, this as u32, 0, &[target as u32]);
        let calls: Vec<_> = e.calls().to_vec();
        assert!(
            calls.len() >= 2,
            "{label}: expected >=2 color-setter calls, got {}",
            calls.len()
        );
        assert_eq!(
            calls[1].addr, 0x71bce0,
            "{label}: 2nd call should be 0x71bce0"
        );
        let scaled_va = u64::from(calls[1].args[0]); // [ebp-0x28]
        let bin: [f32; 3] = std::array::from_fn(|i| e.read_f32(scaled_va + i as u64 * 4));
        let ours = lightstate_exterior_scaled_directional(intensity, comps);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} scaled[{k}]"));
        }
    };

    // 1.375 · 2.6624860763549805 is halfway between two floats, and the store takes the even one
    case(
        &mut e,
        1.375_f32,
        [f32::from_bits(0x402a_662c), 0.0, 0.0],
        "halfway",
    );
    case(&mut e, 1.0, [1.0, 2.0, 3.0], "identity ×1");
    case(&mut e, 0.0, [5.0, 6.0, 7.0], "×0");
    case(&mut e, -2.5, [4.0, -8.0, 0.5], "negative intensity");
    for i in 0..2000u32 {
        let intensity = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -4.0, 4.0)
        };
        let comps = if i % 7 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -10.0, 10.0),
                frand(&mut s, -10.0, 10.0),
                frand(&mut s, -10.0, 10.0),
            ]
        };
        case(&mut e, intensity, comps, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("add_lights_exterior: {checked} cases bit-exact vs WoW.exe @ 0x6a8050");
}

/// `0x6c31e0` outdoors, from `0x6c32b8`.
#[test]
fn fog_select_outdoor_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp = SLICE_ESP;
    let mut s = 0x6c32_b800u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, c: [u8; 3], label: &str| {
        e.write(DAYNIGHT + 0x72, &[c[0]]);
        e.write(DAYNIGHT + 0x71, &[c[1]]);
        e.write(DAYNIGHT + 0x70, &[c[2]]);
        let bin = run_fog_slice(e, 0x6c32b8, ebp);
        let ours = fog_outdoor_color(c[0], c[1], c[2]);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} fog[{k}]"));
        }
    };

    case(&mut e, [0, 0, 0], "zero");
    case(&mut e, [255, 255, 255], "max");
    case(&mut e, [1, 128, 254], "spread");
    for i in 0..2000u32 {
        case(
            &mut e,
            [brand(&mut s), brand(&mut s), brand(&mut s)],
            &format!("fuzz{i}"),
        );
        checked += 1;
    }
    eprintln!(
        "fog_select_outdoor: {checked} cases bit-exact vs WoW.exe @ 0x6c31e0 slice [0x6c32b8,0x6c3300)"
    );
}

/// `0x6c31e0` inside a qualifying group, from `0x6c3233`.
#[test]
fn fog_select_indoor_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp = SLICE_ESP;
    let mut s = 0x6c32_3300u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, c: [u8; 3], label: &str| {
        e.write(DAYNIGHT + 0x82, &[c[0]]);
        e.write(DAYNIGHT + 0x81, &[c[1]]);
        e.write(DAYNIGHT + 0x80, &[c[2]]);
        let bin = run_fog_slice(e, 0x6c3233, ebp);
        let ours = fog_indoor_color(c[0], c[1], c[2]);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} fog[{k}]"));
        }
    };

    case(&mut e, [0, 0, 0], "zero");
    case(&mut e, [255, 255, 255], "max");
    case(&mut e, [7, 200, 64], "spread");
    for i in 0..2000u32 {
        case(
            &mut e,
            [brand(&mut s), brand(&mut s), brand(&mut s)],
            &format!("fuzz{i}"),
        );
        checked += 1;
    }
    eprintln!(
        "fog_select_indoor: {checked} cases bit-exact vs WoW.exe @ 0x6c31e0 slice [0x6c3233,0x6c3300)"
    );
}

/// `0x6c31e0` when no group qualifies, from `0x6c327a`.
#[test]
fn fog_select_fallthrough_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp = SLICE_ESP;
    let mut s = 0x6c32_7a00u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, c: [u8; 3], label: &str| {
        e.write(DAYNIGHT + 0x72, &[c[0]]);
        e.write(DAYNIGHT + 0x71, &[c[1]]);
        e.write(DAYNIGHT + 0x70, &[c[2]]);
        let bin = run_fog_slice(e, 0x6c327a, ebp);
        let ours = fog_outdoor_color(c[0], c[1], c[2]);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} fog[{k}]"));
        }
    };

    case(&mut e, [0, 0, 0], "zero");
    case(&mut e, [255, 255, 255], "max");
    case(&mut e, [9, 17, 240], "spread");
    for i in 0..2000u32 {
        case(
            &mut e,
            [brand(&mut s), brand(&mut s), brand(&mut s)],
            &format!("fuzz{i}"),
        );
        checked += 1;
    }
    eprintln!(
        "fog_select_fallthrough: {checked} cases bit-exact vs WoW.exe @ 0x6c31e0 slice [0x6c327a,0x6c3300)"
    );
}

/// `UpdateDoodadDef` (`0x6952a0`) from `0x69538a` to `0x6953a3`: ebx = this, the fade at
/// `[ebp-0x1c]`, eax a buffer the slice copies from; the product at `[ebx+0x68]`.
#[test]
fn chunk_fade_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x100);
    let xform = e.alloc(0x20);
    let ebp = SLICE_ESP;
    let mut s = 0x6952_a000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, fade: f32, scale: f32, label: &str| {
        e.write_u32(this + 0x30, scale.to_bits());
        e.write_u32(this + 0x68, 0xdead_beef);
        e.write_u32(ebp - 0x1c, fade.to_bits());
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::EAX, xform);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x69538a, 0x6953a3);
        let bin = e.read_f32(this + 0x68);
        let ours = chunk_fade_scale(fade, scale);
        assert_f32(bin, ours, label);
    };

    // 1.375 · 2.6624860763549805 is halfway between two floats, and the store takes the even one
    case(&mut e, 1.375_f32, f32::from_bits(0x402a_662c), "halfway");
    case(&mut e, 1.0, 1.0, "1×1");
    case(&mut e, 0.0, 5.0, "0×5");
    case(&mut e, -2.0, 3.5, "neg×pos");
    case(&mut e, f32::NAN, 2.0, "NaN fade");
    for i in 0..3000u32 {
        let (fade, scale) = if i % 5 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -100.0, 100.0), frand(&mut s, -10.0, 10.0))
        };
        case(&mut e, fade, scale, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("chunk_fade_scale: {checked} cases bit-exact vs WoW.exe @ 0x6952a0");
}
