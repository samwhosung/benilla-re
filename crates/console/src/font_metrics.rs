//! The font metrics: `lineMetric = lineRatio * fontSize`. Three console commands derive the
//! line-height metric `[0xc4ec10]`, which the text layout (`0x63b800`), the mouse-to-line map
//! (`0x63c220`) and the slide (`0x63bd50`) read. Each also rebuilds the font strings through the
//! font layer; only the metric maths is here.

/// The font line-metric factor `[0x801624]` = 10.0 (the lines-per-screen the metric is derived
/// against), and the reset lineRatio `[0x864560]` = 10.0.
pub const LINE_FACTOR: f32 = f32::from_bits(0x4120_0000); // 10.0

const CVAR_SCALE: f32 = f32::from_bits(0x3a83_126f); // [0x801360] = 0.001 (CVar→device)
const CLAMP_LO: f32 = f32::from_bits(0x3c23_d70a); // [0x8029d0] = 0.01
const CLAMP_HI: f32 = f32::from_bits(0x3d4c_cccd); // [0x8029c8] = 0.05

/// `font_size_set`'s clamp (0x639660, slice 0x639666..0x6396a4): the new CVar value is scaled to
/// device units and clamped — `fontSize[0x86455c] = clamp(value · 0.001, 0.01, 0.05)`. The x87
/// `fcom` branch semantics: `< 0.01 → 0.01`, `> 0.05 → 0.05`, else the scaled value (the boundaries
/// `0.01`/`0.05` keep the scaled value, matching the binary's `jp`/`jne` tests).
pub fn font_size_set_clamp(value: f32) -> f32 {
    let x = f64::from(value) * f64::from(CVAR_SCALE);
    if x < f64::from(CLAMP_LO) {
        CLAMP_LO
    } else if x <= f64::from(CLAMP_HI) {
        x as f32
    } else {
        CLAMP_HI
    }
}

/// `font_size_set`'s derived-metric round-trip (0x639660, slice 0x6396c8..0x6396eb): after the new
/// `fontSize`, `lineRatio[0x864560] = lineMetric / fontSize` (stored from the unrounded f64 via
/// `fst`) and `lineMetric[0xc4ec10] = (lineMetric / fontSize) · fontSize` (the same unrounded f64
/// quotient times `fontSize`, then `fstp`). Returns `(lineRatio, lineMetric)`.
pub fn font_size_ratio(line_metric_in: f32, font_size: f32) -> (f32, f32) {
    let q = f64::from(line_metric_in) / f64::from(font_size); // fld lineMetric; fdiv fontSize
    let line_ratio = q as f32; // fst [0x864560]
    let line_metric = (q * f64::from(font_size)) as f32; // fmul fontSize; fstp [0xc4ec10]
    (line_ratio, line_metric)
}

/// `font_metrics_reset`'s arithmetic (0x639100, the metric slice 0x63920e..0x639232): after
/// the config-default reset stores `fontSize`, the derived metric is `lineMetric[0xc4ec10] =
/// fontSize · 10.0` (`lineRatio[0x864560]` is the constant 10.0). Returns `(lineRatio, lineMetric)`.
pub fn font_metrics_reset(font_size: f32) -> (f32, f32) {
    let line_metric = (f64::from(font_size) * f64::from(LINE_FACTOR)) as f32; // fld fs; fmul 10.0; fstp
    (LINE_FACTOR, line_metric)
}

/// `line_spacing_set` (0x6395c0) — the `consolelines`/line-ratio command handler. The argument is
/// the float parsed from the command text (`sscanf "%f"` → `arg`). A zero argument is rejected
/// (no change); otherwise `lineRatio[0x864560] = arg` (raw store) and `lineMetric[0xc4ec10] =
/// fontSize · arg`. Returns `(lineRatio, lineMetric)`, or `None` when `arg == 0` (no write).
pub fn line_spacing_set(arg: f32, font_size: f32) -> Option<(f32, f32)> {
    // fld arg; fcomp 0.0; test ah,0x44; jp → update only when arg != 0 (0x639604..0x63961b)
    if f64::from(arg) == 0.0 {
        return None;
    }
    let line_ratio = arg; // mov [0x864560], arg-bits (raw f32 store)
    let line_metric = (f64::from(font_size) * f64::from(arg)) as f32; // fld fontSize; fmul arg; fstp
    Some((line_ratio, line_metric))
}
