//! Font-metric family difftests. `line_spacing_set` (0x6395c0): the `sscanf "%f"` parse (0x73fb5d,
//! cdecl) is stubbed write-through to inject a chosen `arg` float into its out-pointer and return
//! success, so the `lineRatio`/`lineMetric` stores are diffed against our transcription.

mod common;

use benilla_re_console::font_metrics::{
    font_metrics_reset, font_size_ratio, font_size_set_clamp, line_spacing_set, LINE_FACTOR,
};
use benilla_re_difftest::{Emu, Wow};
use common::{assert_f32, fuzz_unit, g, next};

// font_size_set (0x639660): the value reaches st0 via `push edx; call 0x64aaa0` (str→f32) at
// [0x639660,0x639666). We overwrite those 6 bytes with `fld dword [input]` to inject a chosen st0,
// then run the clamp slice; the derived-metric slice is globals-only.
const FSS_INJECT_AT: u64 = 0x639660;
const FSS_CLAMP_END: u64 = 0x6396a4; // just after `fstp [0x86455c]` (the clamp store)
const FSS_RATIO_BEGIN: u64 = 0x6396c8; // `fld [0xc4ec10]`
const FSS_RATIO_END: u64 = 0x6396eb; // just before `call 0x638ff0`

#[test]
fn font_size_set_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let input = e.alloc(4);
    let mut patch = vec![0xd9, 0x05]; // fld dword [input]
    patch.extend_from_slice(&(input as u32).to_le_bytes());
    e.write(FSS_INJECT_AT, &patch);

    let mut s: u32 = 0x051f_0001;
    // clamp slice: value · 0.001 clamped to [0.01,0.05]; in-range value ∈ [10,50].
    for i in 0..4000u32 {
        let value = match (next(&mut s) >> 8) % 4 {
            0 => fuzz_unit(&mut s, 10.0),         // → x < 0.01 (clamp lo)
            1 => fuzz_unit(&mut s, 40.0) + 10.0,  // → x ∈ [0.01,0.05]
            2 => fuzz_unit(&mut s, 200.0) + 50.0, // → x > 0.05 (clamp hi)
            _ => fuzz_unit(&mut s, 100.0),
        };
        e.write_u32(input, value.to_bits());
        e.run(FSS_INJECT_AT, FSS_CLAMP_END);
        let bin = e.read_f32(g::FONT_SIZE);
        assert_f32(
            bin,
            font_size_set_clamp(value),
            &format!("font_size_set clamp[{i}] value={value}"),
        );
    }
    // derived-metric round-trip: lineRatio = lineMetric/fontSize, lineMetric = ratio·fontSize.
    for i in 0..4000u32 {
        let font_size = fuzz_unit(&mut s, 0.04) + 0.01; // device fontSize ∈ [0.01,0.05]
        let line_metric_in = fuzz_unit(&mut s, 2.0);
        e.write_u32(g::FONT_SIZE, font_size.to_bits());
        e.write_u32(g::LINE_METRIC, line_metric_in.to_bits());
        e.write_u32(g::LINE_RATIO, 0xdead_beef);
        e.run(FSS_RATIO_BEGIN, FSS_RATIO_END);
        let (ratio, metric) = font_size_ratio(line_metric_in, font_size);
        assert_f32(
            e.read_f32(g::LINE_RATIO),
            ratio,
            &format!("font_size_set ratio[{i}] fs={font_size} lm={line_metric_in}"),
        );
        assert_f32(
            e.read_f32(g::LINE_METRIC),
            metric,
            &format!("font_size_set metric[{i}] fs={font_size} lm={line_metric_in}"),
        );
    }
}

const LINE_SPACING_SET: u64 = 0x6395c0;
const SSCANF_F: u64 = 0x73fb5d; // sscanf(str, "%f", &out) — cdecl, out is stack arg 2
                                // font_metrics_reset's metric slice (0x639100): `fld fontSize; fmul 10.0; mov ratio; fstp metric`.
const RESET_METRIC_BEGIN: u64 = 0x63920e;
const RESET_METRIC_END: u64 = 0x63922f; // just after `fstp [0xc4ec10]`, before the frame epilogue

#[test]
fn font_metrics_reset_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s: u32 = 0x4e_7e0001;
    for i in 0..3000u32 {
        let font_size = fuzz_unit(&mut s, 0.06);
        e.write_u32(g::FONT_SIZE, font_size.to_bits());
        e.write_u32(g::LINE_RATIO, 0xdead_beef);
        e.write_u32(g::LINE_METRIC, 0xdead_beef);
        e.run(RESET_METRIC_BEGIN, RESET_METRIC_END);
        let bin_ratio = e.read_f32(g::LINE_RATIO);
        let bin_metric = e.read_f32(g::LINE_METRIC);
        let (ratio, metric) = font_metrics_reset(font_size);
        assert_eq!(
            bin_ratio.to_bits(),
            LINE_FACTOR.to_bits(),
            "reset[{i}] ratio"
        );
        assert_f32(
            bin_metric,
            metric,
            &format!("font_metrics_reset[{i}] fs={font_size}"),
        );
        let _ = ratio;
    }
}

#[test]
fn line_spacing_set_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let arg_src = e.alloc(4); // payload the sscanf stub copies into &local
    let str_ptr = e.alloc(8);
    e.write(str_ptr, b"x\0\0\0\0\0\0\0"); // a non-empty arg string (passes the empty guard)
                                          // sscanf stub: write 4 bytes from arg_src into args[2] (&local_8), return 1, cdecl (caller cleans).
    e.stub_write_through(SSCANF_F, 2, arg_src, 4, 1, 0);

    let mut s: u32 = 0x11ce_0001;
    for i in 0..3000u32 {
        let font_size = fuzz_unit(&mut s, 0.06); // device-space font size ~[0.01,0.05]
        let arg = match (next(&mut s) >> 8) % 3 {
            0 => 0.0,                      // the reject path (no write)
            1 => fuzz_unit(&mut s, 20.0),  // typical line ratios
            _ => fuzz_unit(&mut s, 1.0e3), // large
        };
        e.write_u32(g::FONT_SIZE, font_size.to_bits());
        e.write_u32(arg_src, arg.to_bits());
        // sentinels so a no-write (arg==0) is observable as "unchanged"
        e.write_u32(g::LINE_RATIO, 0xdead_beef);
        e.write_u32(g::LINE_METRIC, 0xdead_beef);

        e.call(LINE_SPACING_SET, 0, str_ptr as u32, &[]);
        let bin_ratio = e.read_f32(g::LINE_RATIO);
        let bin_metric = e.read_f32(g::LINE_METRIC);

        match line_spacing_set(arg, font_size) {
            None => {
                assert_eq!(
                    e.read_u32(g::LINE_RATIO),
                    0xdead_beef,
                    "line_spacing_set[{i}] arg=0 must not write lineRatio"
                );
                assert_eq!(e.read_u32(g::LINE_METRIC), 0xdead_beef);
            }
            Some((ratio, metric)) => {
                assert_f32(
                    bin_ratio,
                    ratio,
                    &format!("line_spacing_set[{i}] ratio fs={font_size} arg={arg}"),
                );
                assert_f32(
                    bin_metric,
                    metric,
                    &format!("line_spacing_set[{i}] metric fs={font_size} arg={arg}"),
                );
            }
        }
    }
}
