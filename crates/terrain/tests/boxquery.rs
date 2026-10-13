//! The box queries' arithmetic, as slices: `GatherVolumeFacets` (`0x6ac060`) from `0x6ac0b2` to
//! `0x6ac148`, `TestMapObjectBox` (`0x6ad330`) from `0x6ad381` to `0x6ad3a8`, and `TestTerrainBox`
//! (`0x6ad4e0`) from its entry to `0x6ad586`, after the last `fistp`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::boxquery::{aabb8_cell_indices, box_cell_indices, negate3};
use common::{assert_f32, next};

/// An EBP and ESP in the middle of the harness's stack, so every frame access and push of a slice
/// lands in it.
const SLICE_EBP: u64 = 0x7008_0000;

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// The `i32` a `fistp dword` stored.
fn read_i32(e: &Emu, va: u64) -> i32 {
    e.read_u32(va) as i32
}

/// `[0x7ffab4]`, the map's half extent, for inputs on a tie.
fn super_center() -> f32 {
    f32::from_bits(0x4685_5555)
}

/// `0x6ac0b2` to `0x6ac148`, after `call 0x7c1450`; no calls inside.
#[test]
fn aabb8_cell_indices_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s = 0x6ac0_0000u32;
    let mut checked = 0u32;

    // in at [ebp-0x44], [ebp-0x40], [ebp-0x50], [ebp-0x4c]; out at [ebp-0x18], [ebp-0x14],
    // [ebp-0x10], [ebp-0xc]
    let case = |e: &mut Emu, ext: [f32; 4], ctx: &str| {
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.write_u32(SLICE_EBP - 0x44, ext[0].to_bits()); // c0
        e.write_u32(SLICE_EBP - 0x40, ext[1].to_bits()); // c1
        e.write_u32(SLICE_EBP - 0x50, ext[2].to_bits()); // c2
        e.write_u32(SLICE_EBP - 0x4c, ext[3].to_bits()); // c3
        e.run(0x6ac0b2, 0x6ac148);
        let bin = [
            read_i32(e, SLICE_EBP - 0x18),
            read_i32(e, SLICE_EBP - 0x14),
            read_i32(e, SLICE_EBP - 0x10),
            read_i32(e, SLICE_EBP - 0xc),
        ];
        let ours = aabb8_cell_indices(ext);
        assert_eq!(bin, ours, "{ctx}: binary={bin:?} ours={ours:?}");
    };

    // A tie: an extreme at the centre gives `fistp(−0.5)`, which rounds to 0; rounding half up
    // would give −1.
    case(
        &mut e,
        [
            super_center(),
            super_center(),
            super_center(),
            super_center(),
        ],
        "tie at -0.5",
    );
    case(&mut e, [0.0, 0.0, 0.0, 0.0], "all-zero (neg = -(-C) = +C)");
    // an overflow, an infinity or NaN stores 0x80000000
    case(
        &mut e,
        [1e30, -1e30, f32::INFINITY, f32::NAN],
        "overflow/inf/nan",
    );

    for i in 0..6000u32 {
        let ext: [f32; 4] = if i % 4 == 0 {
            std::array::from_fn(|_| frand_bits(&mut s))
        } else {
            // coordinates either side of the centre, over many cells
            std::array::from_fn(|_| frand(&mut s, -2000.0, 36000.0))
        };
        case(&mut e, ext, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "aabb8_cell_indices: {checked} cases + probe bit-exact vs WoW.exe @ 0x6ac060 slice [0x6ac0b2,0x6ac148)"
    );
}

/// `0x6ad381` to `0x6ad3a8`, with the `sub esp,0xc` and a `rep movsd` of six dwords from `[esi]` to
/// `[ebp-0x54]` among the negations; the results are read at `[esp]`, `[esp+4]`, `[esp+8]`.
#[test]
fn negate3_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s = 0x6ad3_0000u32;
    let mut checked = 0u32;
    let movsd_src = e.alloc(0x40); // six readable dwords for the `rep movsd`

    // x is [ebp-0x24], [ebp-0x20], [ebp-0x1c]; after the stretch's `sub esp,0xc` its negations are
    // at entry ESP − 0xc, − 8 and − 4.
    let case = |e: &mut Emu, x: [f32; 3], ctx: &str| {
        let entry_esp = SLICE_EBP - 0x100;
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, entry_esp);
        e.set_reg(RegisterX86::ESI, movsd_src);
        e.write_u32(SLICE_EBP - 0x24, x[0].to_bits());
        e.write_u32(SLICE_EBP - 0x20, x[1].to_bits());
        e.write_u32(SLICE_EBP - 0x1c, x[2].to_bits());
        e.run(0x6ad381, 0x6ad3a8);
        let bin = [
            e.read_f32(entry_esp - 0xc),
            e.read_f32(entry_esp - 0x8),
            e.read_f32(entry_esp - 0x4),
        ];
        let ours = negate3(x);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} neg[{k}]"));
        }
    };

    // `fchs` flips the sign of zero, which `0.0 − x` would not.
    case(&mut e, [0.0, -0.0, 0.0], "signed zero");
    case(&mut e, [1.0, -2.0, 3.0], "mixed sign");
    case(
        &mut e,
        [f32::INFINITY, f32::NEG_INFINITY, f32::NAN],
        "inf/-inf/nan",
    );

    for i in 0..6000u32 {
        let x: [f32; 3] = if i % 3 == 0 {
            std::array::from_fn(|_| frand_bits(&mut s))
        } else {
            std::array::from_fn(|_| frand(&mut s, -1e6, 1e6))
        };
        case(&mut e, x, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "negate3: {checked} cases + probe bit-exact vs WoW.exe @ 0x6ad330 slice [0x6ad381,0x6ad3a8)"
    );
}

/// `0x6ad4e0` to `0x6ad586`, from the prologue, so the frame is its own; no calls inside. The
/// extremes are at the box's `+0xc`, `+0x10`, `+0x0` and `+0x4`; edx is stored and not read.
#[test]
fn box_cell_indices_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let bx = e.alloc(0x20);
    let mut s = 0x6ad4_0000u32;
    let mut checked = 0u32;

    // the box: +0xc, +0x10, +0x0, +0x4 in load order
    let case = |e: &mut Emu, ext: [f32; 4], ctx: &str| {
        e.write_u32(bx + 0xc, ext[0].to_bits()); // c0
        e.write_u32(bx + 0x10, ext[1].to_bits()); // c1
        e.write_u32(bx, ext[2].to_bits()); // c2
        e.write_u32(bx + 4, ext[3].to_bits()); // c3
        e.set_reg(RegisterX86::ECX, bx);
        e.set_reg(RegisterX86::EDX, 0xdead_0000);
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.run(0x6ad4e0, 0x6ad586);
        // the outputs are below the function's own EBP, entry ESP − 4
        let fbp = e.get_reg(RegisterX86::EBP);
        let bin = [
            read_i32(e, fbp - 0x18),
            read_i32(e, fbp - 0x14),
            read_i32(e, fbp - 0x10),
            read_i32(e, fbp - 0xc),
        ];
        let ours = box_cell_indices(ext);
        assert_eq!(bin, ours, "{ctx}: binary={bin:?} ours={ours:?}");
    };

    // the tie at the centre, as above
    case(
        &mut e,
        [
            super_center(),
            super_center(),
            super_center(),
            super_center(),
        ],
        "tie at -0.5",
    );
    case(&mut e, [0.0, 0.0, 0.0, 0.0], "all-zero");
    case(
        &mut e,
        [1e30, -1e30, f32::INFINITY, f32::NAN],
        "overflow/inf/nan",
    );

    for i in 0..6000u32 {
        let ext: [f32; 4] = if i % 4 == 0 {
            std::array::from_fn(|_| frand_bits(&mut s))
        } else {
            std::array::from_fn(|_| frand(&mut s, -2000.0, 36000.0))
        };
        case(&mut e, ext, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "box_cell_indices: {checked} cases + probe bit-exact vs WoW.exe @ 0x6ad4e0 slice [0x6ad4e0,0x6ad586)"
    );
}
