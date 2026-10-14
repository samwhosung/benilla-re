//! `GetGlareCloudCover` (`0x6cf7b0`) and `GetGlareCoverEdge` (`0x6cf7d0`) against
//! `skyblend_complement` and `skyblend_triangle`. Each calls `0x6cfa90` and leaves its result in
//! `st0`, so a test runs the instructions after the call as an island: `fld qword [in]`, the
//! function's own bytes (checked identical to the exe's), `fstp qword [out]`. The island is the
//! `fsubr` at `0x6cf7c3` and the `fsub`, `fadd`, `fabs` and `fsubr` from `0x6cf7e3` to `0x6cf7f2`.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::sky_fade::{skyblend_complement, skyblend_triangle};

const G_ONE: u64 = 0x007f_f9d8; // 1.0  (f32 0x3f800000)
const G_HALF: u64 = 0x007f_fa24; // 0.5 (f32 0x3f000000)

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}
/// An `f64` in `[lo, hi)` from the LCG high bits; the low bits repeat quickly.
fn frand(s: &mut u32, lo: f64, hi: f64) -> f64 {
    let m = f64::from(lcg(s) >> 8) / f64::from(1u32 << 24);
    lo + m * (hi - lo)
}
fn assert_f64(binary: f64, ours: f64, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#018x} ours={:#018x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// Builds the island `fld qword [in]; <function bytes>; fstp qword [out]; ret`, after checking the
/// bytes are the exe's own at `src`, and returns a closure that runs it on one input.
fn run_island(emu: &mut Emu, src: u64, owned: &[u8]) -> impl Fn(&mut Emu, f64) -> f64 {
    assert_eq!(
        emu.read(src, owned.len()),
        owned,
        "island bytes differ from WoW.exe @ {src:#x}"
    );
    // The constants the instructions read, as the image holds them.
    emu.write_u32(G_ONE, 0x3f80_0000);
    emu.write_u32(G_HALF, 0x3f00_0000);
    let in_slot = emu.alloc(0x10);
    let out_slot = emu.alloc(0x10);
    let isle = emu.alloc(0x40);
    let mut code = vec![0xdd, 0x05]; // fld qword [in_slot]
    code.extend_from_slice(&(in_slot as u32).to_le_bytes());
    code.extend_from_slice(owned);
    code.push(0xdd); // fstp qword [out_slot]
    code.push(0x1d);
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    code.push(0xc3); // ret
    let ret_at = isle + code.len() as u64 - 1;
    emu.write(isle, &code);
    move |emu: &mut Emu, x: f64| {
        emu.write(in_slot, &x.to_le_bytes());
        emu.write(out_slot, &[0u8; 8]);
        emu.run(isle, ret_at);
        f64::from_le_bytes(emu.read(out_slot, 8).try_into().unwrap())
    }
}

#[test]
fn sf_skyblend_complement_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // `fsubr [0x7ff9d8]` at 0x6cf7c3.
    let run = run_island(&mut emu, 0x6cf7c3, &[0xd8, 0x2d, 0xd8, 0xf9, 0x7f, 0x00]);
    let mut s = 0x6cf7_b001u32;
    // The cover's edges, a signed zero, an exact half, NaN and the infinities.
    let mut probes = vec![
        0.0,
        -0.0,
        0.5,
        1.0,
        -1.0,
        2.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
        1.0 - f64::EPSILON,
    ];
    for _ in 0..4000 {
        probes.push(frand(&mut s, -2.0, 3.0));
    }
    for x in probes {
        assert_f64(run(&mut emu, x), skyblend_complement(x), &format!("1-{x}"));
    }
}

#[test]
fn sf_skyblend_triangle_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // `fsub [0.5]`, `fadd st,st`, `fabs`, `fsubr [1.0]` from 0x6cf7e3 (16 bytes).
    let owned = [
        0xd8, 0x25, 0x24, 0xfa, 0x7f, 0x00, 0xdc, 0xc0, 0xd9, 0xe1, 0xd8, 0x2d, 0xd8, 0xf9, 0x7f,
        0x00,
    ];
    let run = run_island(&mut emu, 0x6cf7e3, &owned);
    let mut s = 0x6cf7_d001u32;
    // The peak at 0.5 and a step either side, the edges at 0 and 1, the midpoints, a signed zero,
    // NaN and the infinities.
    let mut probes = vec![
        0.5,
        0.0,
        -0.0,
        1.0,
        0.25,
        0.75,
        -1.0,
        2.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.5 + f64::EPSILON,
        0.5 - f64::EPSILON / 2.0,
    ];
    for _ in 0..4000 {
        probes.push(frand(&mut s, -2.0, 3.0));
    }
    for x in probes {
        assert_f64(run(&mut emu, x), skyblend_triangle(x), &format!("tri({x})"));
    }
}
