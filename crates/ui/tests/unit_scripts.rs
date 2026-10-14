//! The scripts `GetTime`, `UnitLevel`, `UnitRangedDamage`, `UnitAttackSpeed`, `GetPVPRankProgress`
//! and `GetInspectPVPRankProgress` against the exe: each is run as the slice of instructions that
//! scales a count or a byte, with `lua_pushnumber` stubbed. The time and the attack times differ in
//! the constant: `GetTime` multiplies by the f64 0.001 at `0x801608`, the attack times by the `f32`
//! 0.001 at `0x801360`.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::time_values::{attack_time_seconds, ms_to_seconds};
use benilla_re_ui::unit_values::{level_round, rank_progress};

/// `0x801608`: 0.001 as an f64.
const MS_TO_S_F64_BITS: u64 = 0x3f50_624d_d2f1_a9fc;
/// `0x801360`: 0.001 as an `f32`.
const MS_TO_S_F32_BITS: u32 = 0x3a83_126f;
/// `0x8026c8`: 1/255 as an `f32`.
const PVP_INV255_F32_BITS: u32 = 0x3b80_8081;
use fpkit::next;

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

// deterministic ms / count magnitudes: 0, ticks, common cooldown/attack-time spans, 32-bit extremes
// (the binary's domain is a zero-extended u32 field → non-negative i64).
const COUNTS: [i64; 13] = [
    0,
    1,
    100,
    1000,        // 1 s
    1500,        // 1.5 s GCD
    2000,        // a 2.0 s ranged-attack time
    3500,        // a slow weapon
    30_000,      // 30 s
    0x7fff_ffff, // i32::MAX
    0x8000_0000, // first value where the high dword would matter if it weren't forced 0
    0xffff_ffff, // u32::MAX
    86_400_000,  // 24 h in ms
    12_345_678,
];

// (1) Row 0x515ea0 GetTime — fild qword [ebp-8]; fmul qword [0x801608]=0.001; fstp qword [ebp-8].
// fmul @0x515ebb (×1). Slice 0x515eb8..0x515ec4, read f64 at [ebp-8].
const TIME_FMUL: [u8; 6] = [0xdc, 0x0d, 0x08, 0x16, 0x80, 0x00]; // fmul qword [0x801608]=f64 0.001

#[test]
fn luabind_515ea0_get_time() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // byte-verify the constant the fmul reads is the f64 0.001 at 0x801608, and the fmul opcode.
    assert_eq!(
        emu.read(0x515ebb, 6),
        TIME_FMUL.to_vec(),
        "GetTime fmul @0x515ebb changed"
    );
    assert_eq!(
        u64::from_le_bytes(emu.read(0x801608, 8).try_into().unwrap()),
        MS_TO_S_F64_BITS,
        "GetTime ms→s f64 const @0x801608 changed"
    );

    let run = |emu: &mut Emu, ms: i64| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write(f.ebp - 8, &ms.to_le_bytes()); // fild qword [ebp-8]
        emu.run(0x515eb8, 0x515ec4); // fild; fmul [0.001]; fstp qword [ebp-8]
        f64::from_bits(u64::from_le_bytes(
            emu.read(f.ebp - 8, 8).try_into().unwrap(),
        ))
    };

    let mut total = 0u32;
    for &ms in &COUNTS {
        assert_eq!(
            run(&mut emu, ms).to_bits(),
            ms_to_seconds(ms).to_bits(),
            "GetTime ms={ms}"
        );
        total += 1;
    }
    let mut s = 0x515e_a000u32;
    for _ in 0..3000 {
        let ms = i64::from(next(&mut s) >> 8); // 0..=0xFFFFFF non-negative count (high bits)
        assert_eq!(
            run(&mut emu, ms).to_bits(),
            ms_to_seconds(ms).to_bits(),
            "GetTime fuzz ms={ms}"
        );
        total += 1;
    }
    eprintln!("luabind_515ea0_get_time: {total} bit-exact (fild qword; fmul qword [f64 0.001]; fstp qword)");
}

/// (2) Row 0x517fc0 UnitLevel — fild dword [ebp-4]; fmul [5.0]; fmul [0.01]; fadd [0.5]; __ftol.
/// fmul @0x5180cb, @0x5180d1; fadd @0x5180d7 (×3). Slice 0x5180c8..0x5180e2 (after __ftol returns),
/// read i32 in EAX. The clamp min(level,100) is integer (exercised via the transcription, plus the
/// clamp-edge probe). __ftol runs natively (truncate toward zero).
#[test]
fn luabind_517fc0_unit_level() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // byte-verify the three FP ops at their source addresses.
    assert_eq!(
        emu.read(0x5180cb, 6),
        vec![0xd8, 0x0d, 0xf4, 0x8b, 0x80, 0x00],
        "UnitLevel fmul[5.0] @0x5180cb changed"
    );
    assert_eq!(
        emu.read(0x5180d1, 6),
        vec![0xd8, 0x0d, 0xd0, 0x29, 0x80, 0x00],
        "UnitLevel fmul[0.01] @0x5180d1 changed"
    );
    assert_eq!(
        emu.read(0x5180d7, 6),
        vec![0xd8, 0x05, 0x24, 0xfa, 0x7f, 0x00],
        "UnitLevel fadd[0.5] @0x5180d7 changed"
    );

    // diff the FP slice directly: set [ebp-4]=n, run fild;fmul[5];fmul[0.01];fadd[0.5];__ftol, EAX.
    let run = |emu: &mut Emu, n: u32| -> i32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 4, n);
        emu.run(0x5180c8, 0x5180e2); // fild; fmul[5]; fmul[0.01]; fadd[0.5]; call __ftol
        emu.get_reg(RegisterX86::EAX) as i32
    };

    let mut total = 0u32;
    // every clamped n in 0..=100 (the full domain after min(level,100)); covers the round-half pivots.
    for n in 0..=100u32 {
        // mirror level_round but feeding n pre-clamped (n<=100 here, so b.min(100)==n).
        let want = level_round(n as u8);
        assert_eq!(run(&mut emu, n), want, "UnitLevel fp n={n}");
        // and the full transcription on the same n.
        assert_eq!(level_round(n as u8), want, "UnitLevel transcription n={n}");
        total += 1;
    }
    // clamp edge: every raw byte 100..=255 collapses to the n=100 result inside the binary; the
    // transcription's `b.min(100)` must agree. (The slice feeds n directly; n=100 is the cap.)
    let want100 = run(&mut emu, 100);
    for b in [100u8, 101, 127, 150, 200, 255] {
        assert_eq!(level_round(b), want100, "UnitLevel clamp b={b}");
        total += 1;
    }
    eprintln!(
        "luabind_517fc0_unit_level: {total} bit-exact (n·5·0.01+0.5 then __ftol; clamp min(level,100))"
    );
}

// (3) Row 0x518910 UnitRangedDamage — two parts:
//   (A) the ms→s kernel: fild qword [ebp-0x10]; fmul dword [0x801360]=f32 0.001; fstp qword [esp].
//       fmul @0x51898f (×1) — diffed as an instruction slice 0x518987..0x518998 (halt at the
//       call 0x6f3810); the f64 product is read at [esp-8] (the slice's own `sub esp,8`).
//   (B) the resist-scan `fcomp dword [0x7ffd74]=0.0` (vs 0.0) on each school's f32 resist — the
//       only float COMPARE in the family (a compare). The mandated deterministic threshold probe
//       diffs it via a COPIED-BYTE ISLAND: `fld dword [in]` (matching the source's f32 load) seeds
//       st0, then the EXACT source bytes `fcomp dword [0x7ffd74]; fnstsw ax; test ah,0x41`
//       (byte-verified vs 0x518a23) run; we read ZF — exactly the bit the source `je 0x518a3b`
//       tests. ZF=1 ⇒ je taken (the `(ah&0x41)==0` ⇒ resist > 0.0 ⇒ break, picking this school);
//       ZF=0 ⇒ fall through (resist <= 0.0 ⇒ keep scanning).
const F32_001_FMUL: [u8; 6] = [0xd8, 0x0d, 0x60, 0x13, 0x80, 0x00]; // fmul dword [0x801360]=f32 0.001

fn diff_attack_time_slice(emu: &mut Emu, f: &Frame, fild_va: u64, call_va: u64, disp: u64) -> u32 {
    // byte-verify the f32-0.001 fmul lives in this block (the fmul is `mov ecx,edi`(2)+0 after sub).
    let win = emu.read(fild_va, 16);
    let off = win
        .windows(6)
        .position(|w| w == F32_001_FMUL)
        .unwrap_or_else(|| panic!("attack-time f32 0.001 fmul not found after fild {fild_va:#x}"));
    assert_eq!(
        emu.read(fild_va + off as u64, 6),
        F32_001_FMUL.to_vec(),
        "attack-time fmul @ {:#x} != f32 0.001 kernel",
        fild_va + off as u64
    );

    let run = |emu: &mut Emu, ms: i64| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write(f.ebp - disp, &ms.to_le_bytes()); // fild qword [ebp-disp]
        emu.run(fild_va, call_va);
        f64::from_bits(u64::from_le_bytes(
            emu.read(f.esp - 8, 8).try_into().unwrap(),
        )) // fstp qword [esp] after sub esp,8
    };

    let mut total = 0u32;
    for &ms in &COUNTS {
        assert_eq!(
            run(emu, ms).to_bits(),
            attack_time_seconds(ms).to_bits(),
            "attack-time @{fild_va:#x} ms={ms}"
        );
        total += 1;
    }
    let mut s = (fild_va as u32).wrapping_mul(0x9e37_79b1).wrapping_add(1);
    for _ in 0..2000 {
        let ms = i64::from(next(&mut s) >> 8); // 0..=0xFFFFFF non-negative count (high bits)
        assert_eq!(
            run(emu, ms).to_bits(),
            attack_time_seconds(ms).to_bits(),
            "attack-time fuzz @{fild_va:#x} ms={ms}"
        );
        total += 1;
    }
    total
}

#[test]
fn luabind_518910_unit_ranged_damage() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // verify the f32 0.001 constant the fmul reads.
    assert_eq!(
        f32::from_bits(MS_TO_S_F32_BITS).to_bits(),
        f32::from_bits(u32::from_le_bytes(
            emu.read(0x801360, 4).try_into().unwrap()
        ))
        .to_bits(),
        "UnitRangedDamage ms→s f32 const @0x801360 changed"
    );

    // ---- (A) the ms→s kernel slice (fild qword [ebp-0x10]) -------------------------------
    let mut total = diff_attack_time_slice(&mut emu, &f, 0x518987, 0x518998, 0x10);

    // ---- (B) the resist-scan fcomp vs 0.0 threshold probe (copied-byte island) ----------
    const CMP_VA: u64 = 0x518a23; // fcomp dword [0x7ffd74](6); fnstsw ax(2); test ah,0x41(3) = 11
    let src_cmp = emu.read(CMP_VA, 11);
    assert_eq!(
        src_cmp,
        vec![0xd8, 0x1d, 0x74, 0xfd, 0x7f, 0x00, 0xdf, 0xe0, 0xf6, 0xc4, 0x41],
        "UnitRangedDamage resist-scan cmp bytes changed @ {CMP_VA:#x}"
    );

    let in_cell = emu.alloc(4); // f32 resist
    let isl = emu.alloc(0x40);
    let mut code = vec![0xd9u8, 0x05]; // fld dword [in_cell] (matches source's `fld dword [edx]`)
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    code.extend_from_slice(&src_cmp); // byte-verified copy of fcomp;fnstsw;test
    let isl_end = isl + code.len() as u64; // halt EIP, just after `test ah,0x41`
    emu.write(isl, &code);

    const ZF: u64 = 1 << 6; // EFLAGS zero flag; source `je 0x518a3b` taken iff ZF=1
    let run = |emu: &mut Emu, x: f32| -> bool {
        emu.write_u32(in_cell, x.to_bits());
        emu.run(isl, isl_end);
        (emu.get_reg(RegisterX86::EFLAGS) & ZF) != 0
    };
    // transcribed predicate: je taken iff (ah&0x41)==0 iff fcomp set neither C3(equal) nor C0(less)
    // iff st0 > 0.0. NaN ⇒ unordered ⇒ C3=C0=C2=1 ⇒ (ah&0x41)!=0 ⇒ ZF=0 ⇒ not taken (keep scanning).
    let predict = |x: f32| x > 0.0; // NaN > 0.0 is false in Rust too — matches unordered fall-through

    let probes: [f32; 13] = [
        0.0,  // == boundary → not > 0 → ZF=0 (keep scanning)
        -0.0, // negative zero compares equal → ZF=0
        1.0,  // > 0 → ZF=1 (break, pick this school)
        f32::MIN_POSITIVE,
        1e-30,
        -1e-30,
        -1.0,
        1e30,
        f32::from_bits(0x7fc0_0000), // NaN → unordered → ZF=0
        f32::INFINITY,               // > 0 → ZF=1
        f32::NEG_INFINITY,           // < 0 → ZF=0
        100.0,
        0.5,
    ];
    for &x in &probes {
        assert_eq!(
            run(&mut emu, x),
            predict(x),
            "UnitRangedDamage resist>0 probe x={x} (bits {:#010x})",
            x.to_bits()
        );
        total += 1;
    }
    let mut s = 0x5189_1000u32;
    for _ in 0..3000 {
        let r = next(&mut s);
        let x = f32::from(i16::from((r >> 8) as i8)) - 8.0; // straddles negative/zero/positive
        assert_eq!(
            run(&mut emu, x),
            predict(x),
            "UnitRangedDamage resist>0 probe fuzz x={x}"
        );
        total += 1;
    }

    eprintln!("luabind_518910_unit_ranged_damage: {total} bit-exact (fild qword; fmul dword [f32 0.001]; fstp qword + fcomp [0.0] resist-scan ZF probe)");
}

/// (4) Row 0x518e50 UnitAttackSpeed — two fild qword; fmul dword [f32 0.001]; fstp qword blocks.
/// fmul @0x518eb8 (block1, [ebp-8] from [esi+0x110][+0x1e0]) and @0x518f3c (block2, [ebp-8] from
/// [esi+0x110][+0x1e4]). Each diffed as an instruction slice via the shared attack-time helper.
#[test]
fn luabind_518e50_unit_attack_speed() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let mut total = diff_attack_time_slice(&mut emu, &f, 0x518eb0, 0x518ec1, 0x08); // block1
    total += diff_attack_time_slice(&mut emu, &f, 0x518f34, 0x518f45, 0x08); // block2

    eprintln!("luabind_518e50_unit_attack_speed: {total} bit-exact (2× fild qword; fmul dword [f32 0.001]; fstp qword)");
}

// (5) Row 0x51aa90 GetPVPRankProgress — fild dword [ebp-4]; fmul dword [0x8026c8]=f32 1/255;
// fstp qword [esp]. fmul @0x51aace (×1). Slice 0x51aac6..0x51aad7 (the `sub esp,8` is at the slice
// start), read f64 at [esp-8].
// (6) Row 0x51ab00 GetInspectPVPRankProgress — CLONE: fild dword [ebp-4]; fmul dword [0x8026c8];
// fstp qword [esp]. fmul @0x51ab14 (×1). Slice 0x51ab0e..0x51ab1d. Byte-identical fmul kernel.
const PVP_FMUL: [u8; 6] = [0xd8, 0x0d, 0xc8, 0x26, 0x80, 0x00]; // fmul dword [0x8026c8]=f32 1/255

fn diff_pvp_slice(
    emu: &mut Emu,
    f: &Frame,
    name: &str,
    start_va: u64,
    fmul_va: u64,
    end_va: u64,
) -> u32 {
    assert_eq!(
        emu.read(fmul_va, 6),
        PVP_FMUL.to_vec(),
        "{name}: fmul @ {fmul_va:#x} != f32 1/255 kernel (clone check)"
    );

    let run = |emu: &mut Emu, rank: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 4, u32::from(rank)); // fild dword [ebp-4] (zero-extended movzx byte)
        emu.run(start_va, end_va); // sub esp,8; fild; fmul [1/255]; fstp qword [esp]
        f64::from_bits(u64::from_le_bytes(
            emu.read(f.esp - 8, 8).try_into().unwrap(),
        ))
    };

    let mut total = 0u32;
    for rank in 0..=255u8 {
        assert_eq!(
            run(emu, rank).to_bits(),
            rank_progress(rank).to_bits(),
            "{name} rank={rank}"
        );
        total += 1;
    }
    total
}

#[test]
fn luabind_51aa90_get_pvp_rank_progress() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // verify the f32 1/255 constant.
    assert_eq!(
        f32::from_bits(PVP_INV255_F32_BITS).to_bits(),
        f32::from_bits(u32::from_le_bytes(
            emu.read(0x8026c8, 4).try_into().unwrap()
        ))
        .to_bits(),
        "PVP rank f32 1/255 const @0x8026c8 changed"
    );

    let total = diff_pvp_slice(
        &mut emu,
        &f,
        "luabind_51aa90_get_pvp_rank_progress",
        0x51aac6,
        0x51aace,
        0x51aad7,
    );
    eprintln!("luabind_51aa90_get_pvp_rank_progress: {total} bit-exact (fild dword; fmul dword [f32 1/255]; fstp qword)");
}

#[test]
fn luabind_51ab00_get_inspect_pvp_rank_progress() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // CLONE inheritance: the fmul kernel must be byte-identical to GetPVPRankProgress's.
    assert_eq!(
        emu.read(0x51ab14, 6),
        emu.read(0x51aace, 6),
        "GetInspectPVPRankProgress fmul @0x51ab14 != GetPVPRankProgress fmul @0x51aace (not a clone)"
    );

    let total = diff_pvp_slice(
        &mut emu,
        &f,
        "luabind_51ab00_get_inspect_pvp_rank_progress",
        0x51ab0e,
        0x51ab14,
        0x51ab1d,
    );
    eprintln!("luabind_51ab00_get_inspect_pvp_rank_progress: {total} bit-exact (CLONE: fild dword; fmul dword [f32 1/255]; fstp qword)");
}
