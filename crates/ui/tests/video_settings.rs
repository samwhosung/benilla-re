//! The video settings scripts (`GetTerrainMip`, `GetGamma`, `SetGamma`, `GetBaseMip`,
//! `SetBaseMip`) against the exe: the complement `1.0 - x` each applies, run as the slice of
//! instructions that computes it.
//!
//! The registry lookup, the Lua marshalling and the config writer around the slice are not run.
//! Probes: `x == 1.0` (the complement lands on 0.0), `x == 0.0`, negatives, and for `SetBaseMip`
//! the truncation ties (`+-0.5`, `+-1.5`, the integer boundaries) where rounding and truncating
//! differ.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::video_settings::{
    get_base_mip, get_gamma, get_terrain_mip, set_base_mip, set_gamma,
};
use fpkit::{next, read_f64, rng};

/// Synthesized scratch: a settings-record block (`rec`), an ebp window with locals below, and an
/// esp stack with headroom both ways (the getter slices do `sub esp,8; fstp qword [esp]`).
struct Frame {
    rec: u64,
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let rec = emu.alloc(0x100);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40; // room below for [ebp-8]/[ebp-0x18]
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000; // room below for the x87 qword pushes
    Frame { rec, ebp, esp }
}

/// GetTerrainMip 0x488fb0 — fild dword [rec+0x28] (i32); fsubr [1.0]; fstp qword [esp] (f64).
///   slice [0x488fbd, 0x488fce): `sub esp,8` lowers esp first, so the f64 lands at the final esp.
#[test]
fn luabind_488_get_terrain_mip() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, field: i32| -> f64 {
        emu.set_reg(RegisterX86::EAX, f.rec); // fild [eax+0x28]
        emu.set_reg(RegisterX86::ESI, f.rec); // mov ecx,esi (irrelevant to the FP, just defined)
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.rec + 0x28, field as u32);
        emu.run(0x488fbd, 0x488fce);
        // `sub esp,8` ran inside the slice; read the f64 at the post-sub esp.
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };

    let mut total = 0u32;
    // deterministic: field==1 => complement is exactly 0.0 (the ==threshold probe); 0 => 1.0.
    for field in [
        0i32,
        1,
        -1,
        2,
        -2,
        16,
        -16,
        255,
        -255,
        65535,
        -65536,
        i32::MAX,
        i32::MIN,
    ] {
        let bin = run(&mut emu, field);
        assert_eq!(
            get_terrain_mip(field).to_bits(),
            bin.to_bits(),
            "GetTerrainMip field={field}"
        );
        total += 1;
    }
    let mut s = 0x488f_b000u32;
    for _ in 0..3000 {
        let field = (next(&mut s) as i32) >> (next(&mut s) % 24); // spread magnitudes
        let bin = run(&mut emu, field);
        assert_eq!(
            get_terrain_mip(field).to_bits(),
            bin.to_bits(),
            "GetTerrainMip fuzz field={field}"
        );
        total += 1;
    }
    eprintln!("luabind_488_get_terrain_mip: {total} bit-exact (1.0 - i32, pushed f64)");
}

/// GetGamma 0x4891c0 / GetBaseMip 0x489280 — fld dword [rec+0x24] (f32); fsubr [1.0]; fstp qword
///   [esp] (f64). The two slices are byte-identical (a clone family); both are diffed at their OWN
///   bytes here ([0x4891cd, 0x4891de) and [0x48928d, 0x48929e)).
#[test]
fn luabind_488_get_gamma_basemip() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (start, end, is_basemip, label): the two byte-identical FP slices, each diffed at its OWN
    // bytes against its OWN transcription (the clone is byte-verified, not verdict-inherited).
    let rows: [(u64, u64, bool, &str); 2] = [
        (0x4891cd, 0x4891de, false, "GetGamma"),
        (0x48928d, 0x48929e, true, "GetBaseMip"),
    ];
    let mine = |is_basemip: bool, field: f32| -> f64 {
        if is_basemip {
            get_base_mip(field)
        } else {
            get_gamma(field)
        }
    };
    let run = |emu: &mut Emu, start: u64, end: u64, field: f32| -> f64 {
        emu.set_reg(RegisterX86::EAX, f.rec); // fld dword [eax+0x24]
        emu.set_reg(RegisterX86::ESI, f.rec);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.rec + 0x24, field.to_bits());
        emu.run(start, end);
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };

    let mut total = 0u32;
    for &(start, end, is_basemip, label) in &rows {
        // field==1.0 => complement exactly 0.0 (==threshold); 0.0 => 1.0; fractions/negatives/NaN.
        for field in [
            0.0f32,
            1.0,
            -1.0,
            0.5,
            -0.5,
            0.8,
            1.0 / 3.0,
            2.5,
            -2.5,
            255.0,
            f32::from_bits(0x3f80_0001), // 1.0 + 1 ulp => complement just below 0.0
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            let bin = run(&mut emu, start, end, field);
            assert_eq!(
                mine(is_basemip, field).to_bits(),
                bin.to_bits(),
                "{label} field={field}"
            );
            total += 1;
        }
        let mut s = 0x4891_0000u32 ^ (start as u32);
        for _ in 0..2000 {
            let field = rng(&mut s, -64.0, 64.0);
            let bin = run(&mut emu, start, end, field);
            assert_eq!(
                mine(is_basemip, field).to_bits(),
                bin.to_bits(),
                "{label} fuzz field={field}"
            );
            total += 1;
        }
    }
    eprintln!(
        "luabind_488_get_gamma_basemip: {total} bit-exact (1.0 - f32, pushed f64, clone pair)"
    );
}

/// SetGamma 0x4891f0 — call 0x6f3620 (lua arg, LIVE f64 in st0); fsubr [1.0]; fstp qword [ebp-8]
///   (f64). slice [0x489230, 0x489241): 0x6f3620 patched to `fld qword [TABLE]; ret` so we deliver
///   the chosen arg on st0; the complement f64 is read at [ebp-8].
#[test]
fn luabind_488_set_gamma() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    // patch the lua-arg glue 0x6f3620 to `fld qword [TABLE]; ret` (delivers the chosen f64 in st0).
    let table = emu.alloc(0x10);
    let mut code = vec![0xddu8, 0x05]; // fld qword ptr [disp32]
    code.extend_from_slice(&(table as u32).to_le_bytes());
    code.push(0xc3); // ret
    emu.write(0x6f3620, &code);

    let run = |emu: &mut Emu, arg: f64| -> f64 {
        emu.write(table, &arg.to_bits().to_le_bytes());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, f.rec);
        emu.run(0x489230, 0x489241);
        read_f64(emu, f.ebp - 8)
    };

    let mut total = 0u32;
    // arg==1.0 => complement exactly 0.0 (==threshold); 0.0 => 1.0; fractions/negatives/NaN/inf.
    for arg in [
        0.0f64,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.8,
        1.0 / 3.0,
        f64::from_bits(0x3ff0_0000_0000_0001), // 1.0 + 1 ulp => complement just below 0.0
        2.5,
        -2.5,
        255.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        let bin = run(&mut emu, arg);
        assert_eq!(
            set_gamma(arg).to_bits(),
            bin.to_bits(),
            "SetGamma arg={arg}"
        );
        total += 1;
    }
    let mut s = 0x4891_f000u32;
    for _ in 0..3000 {
        let arg = f64::from(rng(&mut s, -8.0, 8.0));
        let bin = run(&mut emu, arg);
        assert_eq!(
            set_gamma(arg).to_bits(),
            bin.to_bits(),
            "SetGamma fuzz arg={arg}"
        );
        total += 1;
    }
    eprintln!("luabind_488_set_gamma: {total} bit-exact (1.0 - lua_arg, stored f64)");
}

/// SetBaseMip 0x4892b0 — call 0x6f3620 (lua arg, LIVE f64); fsubr [1.0]; call __ftol 0x40a2b0
///   (truncate toward zero -> i64 in edx:eax). slice [0x4892f0, 0x489300): 0x6f3620 patched to
///   deliver the arg in st0; the i64 result is read from edx:eax. Fuzzed over the realizable
///   small-integer / fractional domain (where __ftol's truncate == Rust's `as i64`).
#[test]
fn luabind_488_set_base_mip() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let table = emu.alloc(0x10);
    let mut code = vec![0xddu8, 0x05]; // fld qword ptr [disp32]
    code.extend_from_slice(&(table as u32).to_le_bytes());
    code.push(0xc3); // ret
    emu.write(0x6f3620, &code);

    let run = |emu: &mut Emu, arg: f64| -> i64 {
        emu.write(table, &arg.to_bits().to_le_bytes());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, f.rec);
        emu.run(0x4892f0, 0x489300);
        let lo = u64::from(emu.get_reg(RegisterX86::EAX) as u32);
        let hi = u64::from(emu.get_reg(RegisterX86::EDX) as u32);
        ((hi << 32) | lo) as i64
    };

    let mut total = 0u32;
    // Truncate-toward-zero boundaries: 1.0 -> 0; the complement of 0.5 is 0.5 -> 0; of 1.5 is
    // -0.5 -> 0 (toward zero, not floor=-1); of -0.5 is 1.5 -> 1; of -1.0 is 2.0 -> 2.  Each
    // probes round-vs-truncate, which would diverge on a tie.
    for arg in [
        0.0f64,  // 1-0 => 1
        1.0,     // 0   => 0
        0.5,     // 0.5 => 0 (trunc)
        1.5,     // -0.5 => 0 (toward zero, not -1)
        -0.5,    // 1.5 => 1 (trunc, not 2)
        -1.0,    // 2.0 => 2
        2.0,     // -1.0 => -1
        2.5,     // -1.5 => -1 (toward zero)
        2.9999,  // ~-2.0 => -1 (trunc)
        3.0,     // -2.0 => -2
        -10.0,   // 11.0 => 11
        16.0,    // -15.0 => -15
        100.25,  // -99.25 => -99
        -100.75, // 101.75 => 101
        // NaN, the infinities and values past `i32` and `i64`: `__ftol` returns the whole qword,
        // `0x8000000000000000` for the indefinite.
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        3.0e9,
        -3.0e9,
        1.0e10,
        9.3e18,
        -9.3e18,
        1.0e19,
        1.0e300,
    ] {
        let bin = run(&mut emu, arg);
        assert_eq!(
            set_base_mip(arg),
            bin,
            "SetBaseMip arg={arg}: mine={} bin={bin}",
            set_base_mip(arg)
        );
        total += 1;
    }
    // fuzz the realizable domain: small magnitudes, both signs, fractional — where __ftol's
    // truncate-toward-zero is exactly Rust's `as i64`.
    let mut s = 0x4892_b000u32;
    let (mut neg, mut zero, mut pos) = (0u32, 0u32, 0u32);
    for _ in 0..4000 {
        let arg = f64::from(rng(&mut s, -2048.0, 2048.0));
        let bin = run(&mut emu, arg);
        let mine = set_base_mip(arg);
        assert_eq!(
            mine, bin,
            "SetBaseMip fuzz arg={arg}: mine={mine} bin={bin}"
        );
        match mine.cmp(&0) {
            std::cmp::Ordering::Less => neg += 1,
            std::cmp::Ordering::Equal => zero += 1,
            std::cmp::Ordering::Greater => pos += 1,
        }
        total += 1;
    }
    assert!(
        neg > 0 && zero > 0 && pos > 0,
        "SetBaseMip: need all sign dispositions (neg={neg} zero={zero} pos={pos})"
    );
    eprintln!(
        "luabind_488_set_base_mip: {total} bit-exact (1.0 - lua_arg, __ftol trunc i64; \
         neg={neg} zero={zero} pos={pos})"
    );
}
