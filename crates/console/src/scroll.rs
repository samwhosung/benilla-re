//! The scrollbar drag and the mouse to line maths.

const BASELINE_FACTOR: f32 = f32::from_bits(0x3f40_0000); // [0x8012cc] = 0.75

/// The outcome of `mouse_down_drag` (0x63c0b0) — which state it writes.
#[derive(Debug, PartialEq, Eq)]
pub enum MouseDown {
    /// `(1.0 − lineMetric) > mouseY`: the click is above the console; returns early, no write.
    OffRegion,
    /// The click landed on the resize thumb: starts a scroll-drag (`[0xc4ec38] = 1`).
    ThumbDrag,
    /// The click selected a line: sets the selection-band corners `[0x864570]` / `[0x864568]`.
    Select { band_top: u32, band_bottom: u32 },
}

/// `mouse_down_drag`'s decision and geometry (0x63c0b0). Above the console → `OffRegion`. On the
/// resize thumb (`fVar2 ∈ [min(lineMetric,1.0) − fontSize·0.75, lineMetric]`, `fVar2 = 1.0 − mouseY`
/// stored f32) → `ThumbDrag`. Otherwise a line is selected and the highlight band is laid out from
/// the truncated line offset: `bandTop = 1.0 − (A − idx·fontSize)`, `bandBottom = bandTop − fontSize`
/// where `A = lineMetric − fontSize·0.75 − fontSize` and `idx = ftol((lineMetric − fVar2)/fontSize −
/// 1.0)`. Band corners returned as raw f32 bits (the binary stores them to `[0x864570]`/`[0x864568]`).
pub fn mouse_down_drag(line_metric: f32, font_size: f32, mouse_y: f32) -> MouseDown {
    let lm = f64::from(line_metric);
    let fs = f64::from(font_size);
    let my = f64::from(mouse_y);
    // off-region gate: fld 1.0; fsub lineMetric; fcomp mouseY; je return  (1.0−lineMetric > mouseY)
    if (1.0 - lm) > my {
        return MouseDown::OffRegion;
    }
    let fvar1 = if lm >= 1.0 { 1.0 } else { lm }; // min(lineMetric, 1.0) via fcomp 1.0
    let fvar2 = f64::from((1.0 - my) as f32); // fVar2 = (1.0 − mouseY) stored as f32, re-loaded
    let thumb_lo = fvar1 - fs * f64::from(BASELINE_FACTOR); // fVar1 − fontSize·0.75
    if fvar2 >= thumb_lo && fvar2 <= lm {
        return MouseDown::ThumbDrag;
    }
    let a = (lm - fs * f64::from(BASELINE_FACTOR)) - fs; // lineMetric − fontSize·0.75 − fontSize
    let b = (lm - fvar2) / fs - 1.0;
    let idx = b as i32; // _ftol truncate toward zero
    let c = a - f64::from(idx) * fs;
    let band_top = 1.0 - c; // kept unrounded in st0 across the fst
    MouseDown::Select {
        band_top: (band_top as f32).to_bits(),
        band_bottom: ((band_top - fs) as f32).to_bits(),
    }
}

/// `mouse_move_drag`'s `lineMetric` update (0x63c400, the snap/drag-resize path). While the
/// console is being drag-resized (`[0xc4ec38]==1`), the line metric tracks the cursor:
/// `lineMetric[0xc4ec10] = 1.0 − mouseY`, clamped UP to `fontSize`. The `1.0 − mouseY` is computed
/// once (f64); the **unrounded** value drives the `< fontSize` clamp test while the **f32-rounded**
/// value is what's stored when not clamped. Returns the new `lineMetric` (snap path); the non-snap
/// path leaves `lineMetric` unchanged (an off-region gate + a `dragX` copy, no arithmetic).
pub fn mouse_move_drag_line_metric(
    snap: u32,
    line_metric: f32,
    font_size: f32,
    mouse_y: f32,
) -> f32 {
    if snap == 1 {
        let raw = 1.0_f64 - f64::from(mouse_y); // fld mouseY; fsubr 1.0
        let stored = raw as f32; // fst [0xc4ec10]
        if raw < f64::from(font_size) {
            font_size // fcomp fontSize; jnp → clamp up
        } else {
            stored
        }
    } else {
        line_metric
    }
}

/// `line_at_mouse`'s index (0x63c220, slice 0x63c223..0x63c23d): the console line index under
/// the mouse is `idx = ftol((lineMetric − (1.0 − mouseY)) / fontSize)`. The `1.0 − mouseY` and the
/// `lineMetric − …` stay in the x87 register (f64) before the divide; `_ftol` (0x40a2b0) truncates
/// toward zero. The subsequent line-list walk that maps `idx` to a `CONSOLELINE*` is index/pointer
/// bookkeeping over the TSList (no arithmetic). Returns the truncated index.
pub fn line_at_mouse_index(mouse_y: f32, line_metric: f32, font_size: f32) -> i32 {
    // fld 1.0; fsub mouseY → 1.0 − mouseY ; fsubr lineMetric → lineMetric − (1.0 − mouseY)
    let inner = f64::from(line_metric) - (1.0_f64 - f64::from(mouse_y));
    let q = inner / f64::from(font_size); // fdiv fontSize
    q as i32 // _ftol: truncate toward zero
}
