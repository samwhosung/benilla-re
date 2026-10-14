//! The cooldown scripts (`GetSpellCooldown`, `GetShapeshiftFormCooldown`, `GetPetActionCooldown`,
//! `GetInventoryItemCooldown`, `GetActionCooldown`) against the exe: each pushes a start and a
//! duration, and each push is run as the `fild`, `fmul` by the f64 0.001 (`0x801608`) and `fstp`
//! that turn a millisecond count into seconds. The test first checks that every block's `fmul` has
//! the bytes of the shared multiply.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::time_values::ms_to_seconds;
use fpkit::next;

const MS_FMUL_BYTES: [u8; 6] = [0xdc, 0x0d, 0x08, 0x16, 0x80, 0x00]; // fmul qword [0x801608]=0.001

// deterministic ms counts: 0, the GCD tick, common cooldown magnitudes, and the 32-bit extremes
// (the binary's domain is the zero-extended u32 count → non-negative i64).
const COUNTS: [i64; 12] = [
    0,
    1,
    100,
    1000,        // 1 s
    1500,        // 1.5 s GCD
    30_000,      // 30 s
    1_800_000,   // 30 min
    0x7fff_ffff, // i32::MAX
    0x8000_0000, // first value where the high dword would matter if it weren't forced 0
    0xffff_ffff, // u32::MAX
    86_400_000,  // 24 h in ms
    12_345_678,
];

struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { ebp, esp }
}

/// Diff one cooldown row: its two `fmul`-bearing blocks `(fild_va, call_va)`, the `fild` reading
/// `qword [ebp-disp]`. Byte-verifies each block's `fmul` is the shared ×0.001 kernel, then diffs
/// the f64 product (`fstp qword [esp]` → read at `esp-8`) over `COUNTS` + 2000 fuzz cases each.
fn diff_row(name: &str, blocks: &[(u64, u64)], disp: u64) -> u32 {
    let Some(wow) = Wow::load() else {
        return 0;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // byte-verify each block's fmul == shared kernel before trusting the diff (else circular).
    for &(fild_va, _call) in blocks {
        let win = emu.read(fild_va, 16);
        let off = win
            .windows(6)
            .position(|w| w == MS_FMUL_BYTES)
            .unwrap_or_else(|| panic!("{name}: ms→s fmul bytes not found after fild {fild_va:#x}"));
        assert_eq!(
            emu.read(fild_va + off as u64, 6),
            MS_FMUL_BYTES.to_vec(),
            "{name}: fmul @ {:#x} != shared ×0.001 kernel",
            fild_va + off as u64
        );
    }

    let run = |emu: &mut Emu, fild_va: u64, call_va: u64, ms: i64| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write(f.ebp - disp, &ms.to_le_bytes()); // fild qword [ebp-disp]
        emu.run(fild_va, call_va);
        let bytes = emu.read(f.esp - 8, 8); // fstp qword [esp] after `sub esp,8`
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    for (bi, &(fild_va, call_va)) in blocks.iter().enumerate() {
        for &ms in &COUNTS {
            assert_eq!(
                run(&mut emu, fild_va, call_va, ms).to_bits(),
                ms_to_seconds(ms).to_bits(),
                "{name} block#{bi} @ {fild_va:#x} ms={ms}"
            );
            total += 1;
        }
        let mut s = (fild_va as u32).wrapping_mul(0x9e37_79b1).wrapping_add(1);
        for _ in 0..2000 {
            let ms = i64::from(next(&mut s) >> 8); // 0..=0xFFFFFF non-negative count (high bits)
            assert_eq!(
                run(&mut emu, fild_va, call_va, ms).to_bits(),
                ms_to_seconds(ms).to_bits(),
                "{name} block#{bi} fuzz @ {fild_va:#x} ms={ms}"
            );
            total += 1;
        }
    }
    total
}

/// Row 0x4b40a0 GetSpellCooldown — fild qword [ebp-0x18]; fmul [0.001]; fstp qword [esp].
/// fmul @0x4b4125 / 0x4b4144 (×2).
#[test]
fn luabind_4b40a0_get_spell_cooldown() {
    let n = diff_row(
        "luabind_4b40a0_get_spell_cooldown",
        &[(0x4b411d, 0x4b412e), (0x4b413c, 0x4b414d)],
        0x18,
    );
    if n > 0 {
        eprintln!("luabind_4b40a0_get_spell_cooldown: {n} bit-exact (2× fild qword; fmul [0.001]; fstp qword)");
    }
}

/// Row 0x4bdfa0 GetPetActionCooldown — fild qword [ebp-0x14]; fmul [0.001]; fstp qword [esp].
/// fmul @0x4be052 / 0x4be071 (×2).
#[test]
fn luabind_4bdfa0_get_pet_action_cooldown() {
    let n = diff_row(
        "luabind_4bdfa0_get_pet_action_cooldown",
        &[(0x4be04a, 0x4be05b), (0x4be069, 0x4be07a)],
        0x14,
    );
    if n > 0 {
        eprintln!("luabind_4bdfa0_get_pet_action_cooldown: {n} bit-exact (2× fild qword; fmul [0.001]; fstp qword)");
    }
}

/// Row 0x4c8a60 GetInventoryItemCooldown — fild qword [ebp-0x18]; fmul [0.001]; fstp qword [esp].
/// fmul @0x4c8b9e / 0x4c8bbd (×2).
#[test]
fn luabind_4c8a60_get_inventory_item_cooldown() {
    let n = diff_row(
        "luabind_4c8a60_get_inventory_item_cooldown",
        &[(0x4c8b96, 0x4c8ba7), (0x4c8bb5, 0x4c8bc6)],
        0x18,
    );
    if n > 0 {
        eprintln!("luabind_4c8a60_get_inventory_item_cooldown: {n} bit-exact (2× fild qword; fmul [0.001]; fstp qword)");
    }
}

/// Row 0x4e6ed0 GetActionCooldown — fild qword [ebp-0x14]; fmul [0.001]; fstp qword [esp].
/// fmul @0x4e6f26 / 0x4e6f45 (×2).
#[test]
fn luabind_4e6ed0_get_action_cooldown() {
    let n = diff_row(
        "luabind_4e6ed0_get_action_cooldown",
        &[(0x4e6f1e, 0x4e6f2f), (0x4e6f3d, 0x4e6f4e)],
        0x14,
    );
    if n > 0 {
        eprintln!("luabind_4e6ed0_get_action_cooldown: {n} bit-exact (2× fild qword; fmul [0.001]; fstp qword)");
    }
}

/// Row 0x4b49a0 GetShapeshiftFormCooldown — two parts:
///   (A) the ms→s kernel: fild qword [ebp-0x14]; fmul [0.001]; fstp qword [esp]; fmul @0x4b4a2f /
///       0x4b4a4e (×2) — diffed as instruction slices like the other rows.
///   (B) the arg-validation `fcomp qword [0x802970]` (vs 0.0) on the `lua_tonumber` result — the
///       only float COMPARE in the family (a compare). The mandated deterministic `==0.0`
///       threshold probe diffs it via a COPIED-BYTE ISLAND: `fld qword [in]` seeds st0, then the
///       EXACT source bytes `fcomp qword [0x802970]; fnstsw ax; test ah,0x44` (byte-verified vs
///       0x4b49b4) run; we read PF from EFLAGS — exactly the bit the source `jp 0x4b49d7` tests.
///       PF=1 ⇒ jp taken (arg valid, proceed); PF=0 ⇒ fall through to the `arg == 0` error.
#[test]
fn luabind_4b49a0_get_shapeshift_form_cooldown() {
    let Some(wow) = Wow::load() else {
        return;
    };

    // ---- (A) the ms→s kernel slices ------------------------------------------------------
    let mut total = diff_row(
        "luabind_4b49a0_get_shapeshift_form_cooldown",
        &[(0x4b4a27, 0x4b4a38), (0x4b4a46, 0x4b4a57)],
        0x14,
    );

    // ---- (B) the fcomp vs 0.0 threshold probe (copied-byte island) -----------------------
    let mut emu = Emu::new(&wow);

    const CMP_VA: u64 = 0x4b49b4; // fcomp qword [0x802970](6); fnstsw ax(2); test ah,0x44(3) = 11
    let src_cmp = emu.read(CMP_VA, 11);
    assert_eq!(
        src_cmp,
        vec![0xdc, 0x1d, 0x70, 0x29, 0x80, 0x00, 0xdf, 0xe0, 0xf6, 0xc4, 0x44],
        "GetShapeshiftFormCooldown cmp bytes changed @ {CMP_VA:#x}"
    );

    let in_cell = emu.alloc(8);
    let isl = emu.alloc(0x40);
    let mut code = vec![0xddu8, 0x05]; // fld qword [in_cell]
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    code.extend_from_slice(&src_cmp); // byte-verified copy of fcomp;fnstsw;test
    let isl_end = isl + code.len() as u64; // halt EIP, just after `test ah,0x44`
    emu.write(isl, &code);

    const PF: u64 = 1 << 2; // EFLAGS parity flag; source `jp` taken iff PF=1
    let run = |emu: &mut Emu, x: f64| -> bool {
        emu.write(in_cell, &x.to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        (emu.get_reg(RegisterX86::EFLAGS) & PF) != 0
    };
    // transcribed predicate: x==0.0 ⇒ PF=0 (error path); else (incl. NaN/Inf) ⇒ PF=1 (proceed).
    let predict = |x: f64| x != 0.0;

    let probes: [f64; 13] = [
        0.0,  // == boundary → PF=0 (error)
        -0.0, // negative zero compares equal to 0 → PF=0
        1.0,
        2.0,
        1e-300,  // tiny positive → proceed
        -1e-300, // tiny negative → proceed
        -1.0,
        1e300,
        f64::MIN_POSITIVE,
        f64::from_bits(0x7ff8_0000_0000_0000), // NaN → unordered → proceed
        f64::INFINITY,
        f64::NEG_INFINITY,
        4503599627370496.0, // 2^52
    ];
    for &x in &probes {
        assert_eq!(
            run(&mut emu, x),
            predict(x),
            "shapeshift arg-zero probe x={x} (bits {:#018x})",
            x.to_bits()
        );
        total += 1;
    }
    let mut s = 0x4b49_a000u32;
    for _ in 0..3000 {
        let r = next(&mut s);
        let x = f64::from(i32::from((r >> 8) as i16)) - 128.0; // straddles negative/zero/positive
        assert_eq!(
            run(&mut emu, x),
            predict(x),
            "shapeshift arg-zero probe fuzz x={x}"
        );
        total += 1;
    }

    if total > 0 {
        eprintln!("luabind_4b49a0_get_shapeshift_form_cooldown: {total} bit-exact (2× fild qword; fmul [0.001]; fstp qword + fcomp [0.0] PF probe)");
    }
}
