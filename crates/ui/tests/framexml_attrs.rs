//! Differential tests of the arithmetic of the attribute readers `XmlReadDimension` (`0x6f1eb0`),
//! `0x6f1d60`, `0x6f20a0` and `0x6f23d0`: the scale division of a metric and the packing of a
//! colour, each run against `WoW.exe` in the emulator.
//!
//! The FrameXML attribute-bag geometry/colour parser TU `[0x6f1d60, 0x6f23d0]`. Each owned FP slice
//! is diffed as an **instruction slice** (`Emu::run`) or a whole-function `Emu::call` — registers/
//! memory/stack synthesized, the binary's own bytes executed, the stored `fstp [m32]` value (or the
//! packed RGBA dword) read back. Every in-body FP-arith opcode (the arithmetic opcode set)
//! lands in a diffed slice. Fuzz reads the LCG's HIGH bits. Every
//! owned-slice test carries a deterministic `==threshold` boundary probe.
//!
//! BOUNDARY: these are ui-OWNED attribute parsers (`0x6f2cf0` = string-keyed `{key,value}` lookup,
//! `0x64aaa0` = CRT `atof`), NOT the Lua 5.0 VM at `0x6f3+` — confirmed at the bytes (no `lua_State`
//! threading, no `TValue` tags). See the module doc.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::framexml_attrs::{
    clamp01_unit, metric_scale_divide, pack_rgba, quantize_channel, SCALE1024_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

const SCALE_GLOBAL_VA: u64 = 0x832a4c; // [.data] UI-scale global loaded by 0x41ad70 (fld;ret)
const SCALE1024_VA: u64 = 0x7ffd68; // [.rdata] = 1024.0 (already mapped; not written)

// ===========================================================================================
// (1) the metric scale-divide kernel — 0x6f1d60 / 0x6f1eb0 / 0x6f20a0 per-attribute slice.
//
// Owned slice per attribute (run from the scale-load `call 0x41ad70` through the `fstp [esp]`):
//   call 0x41ad70           ; st0 = [0x832a4c]  (scale global; a leaf `fld;ret`)
//   fmul dword [0x7ffd68]   ; st0 = scale · 1024.0        <-- OWNED
//   push ecx                ; reserve the arg slot
//   fdivr dword [ebp-8]     ; st0 = atof_value / (scale·1024.0)   <-- OWNED (mem / st0)
//   fstp dword [esp]        ; store the quotient as f32
// The atof value lives in [ebp-8] (we write it; in the binary it was `fstp`'d there by atof's
// result, i.e. an f32). We read the result f32 back from the post-push [esp].
//
// Diffed at EVERY occurrence address across all three rows (each is the same kernel modulo the
// `fdivr` ebp displacement — d99/da0; ef0/ef7; f29/f30; fac/fb3; fe6/fed; and 0x6f20a0's 8 …).
// ===========================================================================================

/// (start_of_call_41ad70, end_after_fstp, ebp_disp_of_atof_value_signed).
/// Each entry's `fdivr` reads `[ebp + disp]`; the slice runs `0x..94`(call) .. `0x..a6`(after fstp).
const METRIC_BLOCKS: &[(u64, u64, i64)] = &[
    // 0x6f1d60 (2×): val / x
    (0x6f1d94, 0x6f1da6, -0x8),
    (0x6f1e13, 0x6f1e25, 0x8),
    // 0x6f1eb0 (4×): x (2 branches) + y (2 branches)
    (0x6f1eeb, 0x6f1efd, -0x8),
    (0x6f1f24, 0x6f1f36, -0x8),
    (0x6f1fa7, 0x6f1fb9, 0xc),
    (0x6f1fe1, 0x6f1ff3, 0xc),
    // 0x6f20a0 (8×): left/right/top/bottom × {present-all branch, individual branch}
    (0x6f20ef, 0x6f2101, -0x8),
    (0x6f2126, 0x6f2138, -0x8),
    (0x6f215d, 0x6f216f, -0x8),
    (0x6f2199, 0x6f21ab, -0x8),
    (0x6f221f, 0x6f2231, 0x14),
    (0x6f2259, 0x6f226b, 0x8),
    (0x6f2293, 0x6f22a5, 0x8),
    (0x6f22cd, 0x6f22df, 0xc),
];

#[test]
fn glue_6f1_metric_scale_divide() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80;
    let stack = emu.alloc(0x1000);
    let esp0 = stack + 0xf00;

    // Confirm the mapped 1024.0 constant matches our transcription's const (independence: read the
    // binary's own .rdata, never re-derive from our bits).
    assert_eq!(emu.read_u32(SCALE1024_VA), SCALE1024_BITS, "1024.0 .rdata");

    let run_block =
        |emu: &mut Emu, start: u64, end: u64, disp: i64, atof: f32, scale: f32| -> f32 {
            emu.write_u32(SCALE_GLOBAL_VA, scale.to_bits()); // [0x832a4c] = scale (0x41ad70 loads it)
            emu.set_reg(RegisterX86::EBP, ebp);
            emu.set_reg(RegisterX86::ESP, esp0);
            emu.set_reg(RegisterX86::ECX, 0); // the `push ecx` placeholder
            let slot = (ebp as i64 + disp) as u64;
            emu.write_u32(slot, atof.to_bits()); // [ebp+disp] = atof(value) as f32
            emu.run(start, end);
            // result was `fstp dword [esp]`, esp = esp0 - 4 (one push ecx happened).
            emu.read_f32(esp0 - 4)
        };

    let mut s = 0x6f1_d60u32;
    let mut cases = 0u32;
    for &(start, end, disp) in METRIC_BLOCKS {
        // deterministic boundary probes: scale==1.0 exact, atof==0.0 exact, and a negative atof.
        let fixed: [(f32, f32); 4] = [
            (1.0, 1.0),    // atof=1.0, scale=1.0 -> 1/1024 exactly
            (0.0, 1.0),    // atof==0.0 boundary -> 0.0
            (-512.0, 1.0), // negative -> negative quotient
            (2048.0, 2.0), // scale==2.0 -> 2048/(2*1024)=1.0 exactly
        ];
        for (atof, scale) in fixed {
            let bin = run_block(&mut emu, start, end, disp, atof, scale);
            let mine = metric_scale_divide(atof, scale);
            assert_eq!(
                mine.to_bits(),
                bin.to_bits(),
                "metric @ {start:#x} atof={atof} scale={scale}"
            );
            cases += 1;
        }
        for _ in 0..32 {
            let atof = rng(&mut s, -4096.0, 4096.0);
            let scale = rng(&mut s, 0.25, 4.0);
            let bin = run_block(&mut emu, start, end, disp, atof, scale);
            let mine = metric_scale_divide(atof, scale);
            assert_eq!(
                mine.to_bits(),
                bin.to_bits(),
                "metric @ {start:#x} atof={atof} scale={scale}"
            );
            cases += 1;
        }
    }
    assert!(cases >= 14 * 36, "expected all blocks fuzzed, got {cases}");
}

// ===========================================================================================
// (2) the colour parser — 0x6f23d0 whole-function call (covers all 8 FP-arith ops by construction:
//     4×(fmul 255.0; fadd 0.5), plus the clamp ladder + __ftol packing).
//
// thiscall: ecx = property bag {.[+0x10]=count, .[+0x14]=ptr to {key,value} entries}, edx = out u32*.
// Returns 1 (attrs present) and writes the packed 0xAARRGGBB to [out]. We build a real attr array
// with keys "r"/"g"/"b"/"a" and numeric value strings; the binary's own atof+clamp+__ftol run.
// ===========================================================================================

struct Bag {
    this: u64,
    out: u64,
}

/// Build a property bag with the given channel value strings (None ⇒ attr absent). Returns the bag.
fn build_bag(emu: &mut Emu, vals: &[(&str, Option<&str>)]) -> Bag {
    // count present entries
    let present: Vec<(&str, &str)> = vals
        .iter()
        .filter_map(|(k, v)| v.map(|v| (*k, v)))
        .collect();
    let n = present.len() as u32;
    let entries = emu.alloc(8 * (present.len().max(1)) as u64);
    for (i, (k, v)) in present.iter().enumerate() {
        let kp = emu.alloc(k.len() as u64 + 1);
        let vp = emu.alloc(v.len() as u64 + 1);
        emu.write(kp, k.as_bytes());
        emu.write(kp + k.len() as u64, &[0]);
        emu.write(vp, v.as_bytes());
        emu.write(vp + v.len() as u64, &[0]);
        emu.write_u32(entries + 8 * i as u64, kp as u32);
        emu.write_u32(entries + 8 * i as u64 + 4, vp as u32);
    }
    let this = emu.alloc(0x20);
    emu.write_u32(this + 0x10, n); // count
    emu.write_u32(this + 0x14, entries as u32); // ptr to entries
    let out = emu.alloc(4);
    Bag { this, out }
}

fn run_color(emu: &mut Emu, vals: &[(&str, Option<&str>)]) -> u32 {
    let bag = build_bag(emu, vals);
    let eax = emu.call(0x6f23d0, bag.this as u32, bag.out as u32, &[]);
    assert_eq!(eax, 1, "0x6f23d0 returns 1 when attrs present");
    emu.read_u32(bag.out)
}

/// Format a channel f32 the way an XML attribute would carry it, then diff the binary's parse+pack
/// against our `pack_rgba(atof(strings))`. We compare on the value strings (so `atof` round-trips
/// identically on both sides — the diff is the OWNED clamp+quantize+pack, not float→string).
fn check_color(emu: &mut Emu, r: &str, g: &str, b: &str, a: Option<&str>) {
    let bin = run_color(
        emu,
        &[("r", Some(r)), ("g", Some(g)), ("b", Some(b)), ("a", a)],
    );
    // our side parses the SAME strings via the same atof semantics: use the binary's atof? No — we
    // diff pack_rgba on the f32 the string denotes. The value strings are exact decimal f32s.
    let pf = |s: &str| -> f32 { s.parse::<f32>().unwrap() };
    let av = a.map(pf).unwrap_or(1.0); // absent ⇒ default 1.0 (the 0x6f23f7 seed)
    let mine = pack_rgba(pf(r), pf(g), pf(b), av);
    assert_eq!(
        mine, bin,
        "color r={r} g={g} b={b} a={a:?} mine={mine:#010x} bin={bin:#010x}"
    );
}

#[test]
fn glue_6f1_color_pack() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // deterministic boundary probes: the exact clamp edges 0.0 and 1.0, out-of-range both sides.
    check_color(&mut emu, "0", "0", "0", Some("0")); // all 0 -> 0x00000000
    check_color(&mut emu, "1", "1", "1", Some("1")); // all 1 (==hi boundary) -> 0xFFFFFFFF
    check_color(&mut emu, "0.5", "0.25", "0.75", Some("0.125")); // mid
    check_color(&mut emu, "-0.5", "2", "1", Some("-1")); // clamp both sides: r->0, g->1, b->1, a->0
    check_color(&mut emu, "0.5", "0.5", "0.5", None); // alpha absent -> default 1.0
                                                      // a value just under 1.0 and one exactly at the 0.5-bias rounding edge of __ftol.
    check_color(&mut emu, "0.999", "0.001", "0.5", Some("0.998"));
    // exact representable decimals that exercise truncate(c*255+0.5).
    check_color(&mut emu, "0.2", "0.4", "0.6", Some("0.8"));

    // fuzz: independent channel values, including the [0,1] band and out-of-range.
    let mut s = 0x6f2_3d0u32;
    for _ in 0..64 {
        let mk = |s: &mut u32| -> String {
            let v = rng(s, -0.25, 1.25);
            format!("{v:.6}")
        };
        let r = mk(&mut s);
        let g = mk(&mut s);
        let b = mk(&mut s);
        let a = mk(&mut s);
        check_color(&mut emu, &r, &g, &b, Some(&a));
    }
}

// ===========================================================================================
// (3) direct unit coverage of the clamp + quantize leaves on the deterministic boundaries (the
// pack test exercises them through the binary; these assert the leaf primitives in isolation at
// the exact float boundaries, so a refactor of the kernels can't silently drift off-boundary).
// The channel is clamped to 0 to 1 before the quantize (NaN becomes 1), so the `__ftol` input is
// at most 255.5 and no NaN or out-of-range value reaches it.
// ===========================================================================================

#[test]
fn glue_6f1_clamp_quantize_boundaries() {
    // clamp boundaries
    assert_eq!(clamp01_unit(0.0).to_bits(), 0.0f32.to_bits());
    assert_eq!(clamp01_unit(1.0).to_bits(), 1.0f32.to_bits());
    assert_eq!(clamp01_unit(-0.0).to_bits(), (-0.0f32).to_bits()); // -0.0 < 0.0 is false -> kept
    assert_eq!(
        clamp01_unit(f32::from_bits(0x3f7f_ffff)).to_bits(),
        0x3f7f_ffff
    ); // just below 1
    assert_eq!(clamp01_unit(-1.0e-30).to_bits(), 0.0f32.to_bits());
    assert_eq!(clamp01_unit(2.0).to_bits(), 1.0f32.to_bits());
    assert_eq!(clamp01_unit(f32::NAN).to_bits(), 1.0f32.to_bits()); // NaN -> hi (even parity)
                                                                    // quantize boundaries: truncate(c*255 + 0.5)
    assert_eq!(quantize_channel(0.0), 0);
    assert_eq!(quantize_channel(1.0), 255); // 255+0.5 trunc -> 255
    assert_eq!(quantize_channel(0.5), 128); // 127.5+0.5=128
}
