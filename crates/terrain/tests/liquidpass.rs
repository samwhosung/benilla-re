//! The liquid passes against `0x685010`, `0x6855a0`, `0x68aac0` and `0x68dca0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::liquidpass::{
    chunk_liquid_cam_offset, liquid_anim_frame, liquid_pass_anim_frame,
};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes).
const SLICE_EBP: u64 = 0x7008_0000;

/// The camera position.
const CAM_X: u64 = 0xc7cf20;

const CAM_Y: u64 = 0xc7cf24;

const CAM_Z: u64 = 0xc7cf28;

/// The animation periods in seconds, one per liquid type.
const PERIOD_TABLE: u64 = 0x8102b8;

/// The lava phase `0x68dca0` scales.
const ANIM_PHASE: u64 = 0xc7b2a0;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any float, NaN and the infinities included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// The camera-relative offset from `begin` to `end`, eax the liquid's header: the three differences
/// at `[ebp-0xc]` up.
fn run_cam_offset(
    e: &mut Emu,
    begin: u64,
    end: u64,
    hdr: u64,
    field: [f32; 3],
    cam: [f32; 3],
) -> [f32; 3] {
    e.write_u32(hdr + 0x6c, field[0].to_bits());
    e.write_u32(hdr + 0x70, field[1].to_bits());
    e.write_u32(hdr + 0x74, field[2].to_bits());
    e.write_u32(CAM_X, cam[0].to_bits());
    e.write_u32(CAM_Y, cam[1].to_bits());
    e.write_u32(CAM_Z, cam[2].to_bits());
    e.set_reg(RegisterX86::EAX, hdr);
    e.set_reg(RegisterX86::EBP, SLICE_EBP);
    e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x40);
    e.run(begin, end);
    [
        e.read_f32(SLICE_EBP - 0xc),
        e.read_f32(SLICE_EBP - 0x8),
        e.read_f32(SLICE_EBP - 0x4),
    ]
}

/// Fuzzes the offset between `begin` and `end`.
fn cam_offset_family(test: &str, begin: u64, end: u64, seed: u32) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let hdr = e.alloc(0x100);
    let mut s = seed;
    let mut checked = 0u32;

    let case = |e: &mut Emu, field: [f32; 3], cam: [f32; 3], label: &str| {
        let bin = run_cam_offset(e, begin, end, hdr, field, cam);
        let ours = chunk_liquid_cam_offset(field, cam);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{label} offset[{k}]"));
        }
    };

    // equal: +0.0
    case(&mut e, [1.0, -2.5, 1e7], [1.0, -2.5, 1e7], "equal");
    case(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "0−0");
    // differences that round when stored
    case(
        &mut e,
        [1.677_722e7, -1.677_722e7, 3.355_444e7],
        [0.3, 0.7, -0.9],
        "narrow-on-store",
    );
    case(&mut e, [100.0, 200.0, 300.0], [10.0, 20.0, 30.0], "spread");
    for i in 0..3000u32 {
        let (field, cam) = if i % 5 == 0 {
            (
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
            )
        } else {
            (
                [
                    frand(&mut s, -2.0e7, 2.0e7),
                    frand(&mut s, -2.0e7, 2.0e7),
                    frand(&mut s, -2.0e7, 2.0e7),
                ],
                [
                    frand(&mut s, -2.0e7, 2.0e7),
                    frand(&mut s, -2.0e7, 2.0e7),
                    frand(&mut s, -2.0e7, 2.0e7),
                ],
            )
        };
        case(&mut e, field, cam, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!("{test}: {checked} cases + probes bit-exact vs WoW.exe slice [{begin:#x},{end:#x})");
}

/// `DrawOceanPass` (`0x685010`) from `0x685147` to the call at `0x685175`.
#[test]
fn chunk_liquid_pass1_offset_difftest() {
    cam_offset_family(
        "chunk_liquid_pass1_offset_difftest",
        0x685147,
        0x685175,
        0x6850_1000,
    );
}

/// `DrawMagmaPass` (`0x6855a0`) from `0x68566a` to the call at `0x685698`.
#[test]
fn chunk_liquid_pass2_offset_difftest() {
    cam_offset_family(
        "chunk_liquid_pass2_offset_difftest",
        0x68566a,
        0x685698,
        0x6855_a000,
    );
}

/// `GetLiquidFrame` (`0x68aac0`) from `0x68aacc` to `0x68ab14`, ebx = the type, run in two parts
/// around the clock call at `0x68aae2` with the tick put in eax between them: the stored product at
/// `[ebp-0x10]` and the frame at `[ebp-0x8]`.
#[test]
fn liquid_anim_frame_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s = 0x68aa_c000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, ty: u32, tick: u32, label: &str| {
        let period_secs = e.read_f32(PERIOD_TABLE + u64::from(ty) * 4);
        e.set_reg(RegisterX86::EBX, u64::from(ty));
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x80);
        e.run(0x68aacc, 0x68aae2); // the period, to the clock call
                                   // the clock's value in eax, then the rest
        e.set_reg(RegisterX86::EAX, u64::from(tick));
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.run(0x68aae7, 0x68ab14);
        let bin_frac30 = e.read_f32(SLICE_EBP - 0x10);
        let bin_frame = e.read_u32(SLICE_EBP - 0x8) as i32;
        let (frac30, frame) = liquid_anim_frame(period_secs, tick);
        assert_f32(bin_frac30, frac30, &format!("{label} frac30"));
        assert_eq!(
            bin_frame, frame,
            "{label}: frame binary={bin_frame} ours={frame} (frac30={bin_frac30})"
        );
    };

    // with a period of 1250, 625 gives 14.5, which rounds to 14
    case(&mut e, 0, 625, "14.5");
    // 250 gives 5.5, which rounds to 6
    case(&mut e, 0, 250, "5.5");
    // 0 gives −0.5, which rounds to 0
    case(&mut e, 0, 0, "-0.5");
    case(&mut e, 0, 1250, "tick=period → wraps to 0");
    case(&mut e, 1, 9_999_999, "large tick type1");
    case(&mut e, 3, 0x7fff_ffff, "max signed tick");
    case(&mut e, 6, 0xffff_ffff, "max unsigned tick");
    for i in 0..3000u32 {
        let ty = (next(&mut s) >> 8) % 8;
        let tick = next(&mut s);
        case(&mut e, ty, tick, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "liquid_anim_frame: {checked} cases + probes bit-exact vs WoW.exe @ 0x68aac0 \
         slices [0x68aacc,0x68aae2)+[0x68aae7,0x68ab14)"
    );
}

/// `CChunkLiquid::DrawMagma` (`0x68dca0`) from `0x68dd26` to `0x68dd41`: the stored product at
/// `[ebp-0x4]` and the frame at `[ebp-0x8]`.
#[test]
fn liquid_pass_anim_frame_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s = 0x68dc_a000u32;
    let mut checked = 0u32;

    let case = |e: &mut Emu, phase: f32, label: &str| {
        e.write_u32(ANIM_PHASE, phase.to_bits());
        e.set_reg(RegisterX86::EBP, SLICE_EBP);
        e.set_reg(RegisterX86::ESP, SLICE_EBP - 0x40);
        e.run(0x68dd26, 0x68dd41);
        let bin_scaled = e.read_f32(SLICE_EBP - 0x4);
        let bin_frame = e.read_u32(SLICE_EBP - 0x8) as i32;
        let (scaled, frame) = liquid_pass_anim_frame(phase);
        assert_f32(bin_scaled, scaled, &format!("{label} scaled"));
        assert_eq!(
            bin_frame, frame,
            "{label}: frame binary={bin_frame} ours={frame} (scaled={bin_scaled})"
        );
    };

    // 600 gives 14.5, which rounds to 14
    case(&mut e, 600.0, "14.5");
    // 560 gives 13.5, which rounds to 14
    case(&mut e, 560.0, "13.5");
    case(&mut e, 20.0, "0.0");
    case(&mut e, 0.0, "phase 0");
    case(&mut e, -40.0, "negative phase");
    case(&mut e, 12345.678, "spread");
    case(&mut e, f32::NAN, "NaN phase");
    for i in 0..3000u32 {
        let phase = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1.0e5, 1.0e5)
        };
        case(&mut e, phase, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "liquid_pass_anim_frame: {checked} cases + probes bit-exact vs WoW.exe @ 0x68dca0 slice [0x68dd26,0x68dd41)"
    );
}
