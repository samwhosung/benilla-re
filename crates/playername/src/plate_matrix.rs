//! `0x6c6e90`'s and `0x6c8d00`'s floating-point arithmetic slices — the world-text plate
//! **depth-fade scalar**, the **atlas-cell + billboard corner-quad** geometry, the **screen-Y
//! accumulate**, and the **ortho-rect** the world-text matrix is built from.
//!
//! Both functions call into the graphics layer (`0x58xxxx` primitives, vertex buffer and device),
//! the maths library (`0x7bxxxx`), the console's text draw, and the gxumath `projection_ortho`
//! (`0x5c3d90`); the billboard draw and the ortho matrix multiply are theirs. What is here is the
//! per-frame arithmetic that feeds those calls.
//!
//! Every op runs at 53-bit precision (`FPCW=0x027F`): an f32 memory operand is widened
//! exactly to f64 on load (`fld dword`/`fsub dword`/`fmul dword`/`fadd dword`), the running st0 is
//! a 53-bit-mantissa f64, and the result is rounded to f32 only at each `fstp dword` store. The
//! two `__ftol` (`0x40a2b0`) calls (truncate-toward-zero, low 32 bits) and the four `0x73ff3f`
//! calls in the matrix slice are modelled exactly; `0x73ff3f` is the C runtime's `floor` (round toward
//! −∞), as a direct call shows.
#![allow(clippy::float_cmp)]

// ─────────────────────────────────────────────────────────────────────────────────────────────
// `0x6c6e90` pn_plate_render — arithmetic slices (module `plate_matrix`).
// ─────────────────────────────────────────────────────────────────────────────────────────────

/// `0x8112a8` (`.rdata`) — the depth-fade pivot, **f32** `4.0` (`0x40800000`).
pub const DEPTH_PIVOT_F32_BITS: u32 = 0x4080_0000;
/// `0x8112ac` (`.rdata`) — the depth-fade gain, **f32** `1.5` (`0x3fc00000`).
pub const DEPTH_GAIN_F32_BITS: u32 = 0x3fc0_0000;
/// `0x80679c` (`.rdata`) — the depth-fade base scale, **f32** `0.2` (`0x3e4ccccd`); also the
/// default scale literal stored before the `> 4.0` test (`mov [ebp-0x4], 0x3e4ccccd`).
pub const SCALE_BASE_F32_BITS: u32 = 0x3e4c_cccd;
/// `0x8029b0` (`.rdata`) — the atlas-cell step, **f32** `0.25` (`0x3e800000`).
pub const ATLAS_STEP_F32_BITS: u32 = 0x3e80_0000;

#[inline]
fn f4(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits)) // an `fld/fsub/fmul/fadd dword` widens the f32 operand to f64
}

/// `0xce875c..0xce8788` (`.data`, written by the const-init `0x6c6a10`) — the anchor unit-quad
/// corner LUT: 4 × `vec3` `[(-0.5,1,0), (0.5,1,0), (0.5,0,0), (-0.5,0,0)]`. The corner-quad slice
/// copies these verbatim into each vertex's `pos` (no arithmetic — a `mov` triple).
pub const ANCHOR_CORNER_LUT: [[f32; 3]; 4] = [
    [-0.5, 1.0, 0.0],
    [0.5, 1.0, 0.0],
    [0.5, 0.0, 0.0],
    [-0.5, 0.0, 0.0],
];

/// `0xce873c..0xce8758` (`.data`, written by the const-init `0x6c6ac0`) — the icon UV-rect LUT:
/// 4 × `vec2` `[(0,0), (0.25,0), (0.25,0.25), (0,0.25)]`. The corner-quad slice adds the atlas
/// cell `(col,row)` to each of these (`fadd dword`).
pub const ICON_UV_LUT: [[f32; 2]; 4] = [[0.0, 0.0], [0.25, 0.0], [0.25, 0.25], [0.0, 0.25]];

/// Slice (1) — the **depth-fade scalar** `[0x6c6f10, 0x6c6f43)`.
///
/// ```text
///   6c6f10: fld   dword [ebp-0x24]   ; camDepth (local_28), widened to f64 on st0
///   6c6f13: fsub  dword [eax+0x8]    ; - obj.bound[+8]      -> camRelDepth on st0
///   6c6f16: mov   [ebp-0x4], 0x3e4ccccd   ; scale := 0.2 (default)
///   6c6f1d: fcom  dword [0x8112a8]   ; compare camRelDepth ? 4.0
///   6c6f23: fnstsw ax; test ah,0x41; jne 6c6f41   ; if camRelDepth <= 4.0 -> keep 0.2
///   6c6f2a: fdiv  dword [0x8112a8]   ; / 4.0
///   6c6f30: fmul  dword [0x8112ac]   ; * 1.5
///   6c6f36: fmul  dword [0x80679c]   ; * 0.2
///   6c6f3c: fstp  dword [ebp-0x4]    ; scale := (camRelDepth/4)*1.5*0.2
/// ```
///
/// `test ah,0x41` masks C3(=0x40, equal) | C0(=0x01, st0<src); `jne` is taken when either is set,
/// i.e. when `camRelDepth <= 4.0`. So the divide branch runs **only** for `camRelDepth > 4.0`; the
/// `== 4.0` boundary keeps the default `0.2`. Returns the scale as the stored f32.
pub fn depth_fade_scale(cam_depth: f32, bound_z: f32) -> f32 {
    let cam_rel = f64::from(cam_depth) - f64::from(bound_z); // fld; fsub dword (both widened)
    if cam_rel > f4(DEPTH_PIVOT_F32_BITS) {
        // fdiv 4.0; fmul 1.5; fmul 0.2 — all in one PC_53 f64 chain, narrowed at the fstp.
        ((cam_rel / f4(DEPTH_PIVOT_F32_BITS)) * f4(DEPTH_GAIN_F32_BITS) * f4(SCALE_BASE_F32_BITS))
            as f32
    } else {
        f32::from_bits(SCALE_BASE_F32_BITS) // the default 0.2 literal kept verbatim
    }
}

/// The cached `desc[+0x30] = textWidth * scale` product `[0x6c6fe4, 0x6c6fed)`.
///
/// ```text
///   6c6fe4: fild  qword [ebp-0xc]    ; (double)textWidth_int  (uVar7 in the low dword, high = 0)
///   6c6fe7: fmul  dword [ebp-0x4]    ; * scale  (the depth-fade scalar, widened to f64)
///   6c6fea: fstp  dword [esi+0x30]   ; desc[+0x30] := narrow(...)
/// ```
///
/// `textWidth` is the unsigned glyph-run pixel width from console GxText (`0x608f50`, delegated).
pub fn text_width_times_scale(text_width: u32, scale: f32) -> f32 {
    (f64::from(text_width) * f64::from(scale)) as f32 // fild qword; fmul dword; fstp dword
}

/// Slice (2) — the **atlas cell** `(col,row) = ((texId & 3)·0.25, (texId >> 2)·0.25)`
/// `[0x6c71c3, 0x6c71f6)`.
///
/// ```text
///   6c71c3: and   esi(edx),0x3 ; [ebp-0xc]=id&3 [ebp-8]=0; 6c71cc fild qword -> (double)(id&3)
///   6c71d5: fmul  dword [0x8029b0]   ; * 0.25 -> col (fVar2)
///   6c71cf: shr   esi,0x2     ; [ebp-0xc]=id>>2; 6c71de fild qword -> (double)(id>>2)
///   6c71f0: fmul  dword [0x8029b0]   ; * 0.25 -> row (fVar3)
/// ```
///
/// `texId` is the raid-target / icon-atlas index. `col`/`row` stay live on the x87 stack (no
/// `fstp` — they are consumed directly by the per-corner UV adds), so the bit-exact observable is
/// the f32 each would narrow to. Returns `(col, row)`.
pub fn atlas_cell(tex_id: u32) -> (f32, f32) {
    let col = f64::from(tex_id & 3) * f4(ATLAS_STEP_F32_BITS); // fild (id&3); fmul 0.25
    let row = f64::from(tex_id >> 2) * f4(ATLAS_STEP_F32_BITS); // fild (id>>2); fmul 0.25
    (col as f32, row as f32)
}

/// Slice (2) — the per-corner billboard quad `[0x6c7200, 0x6c7248)`. For each of the 4 corners:
/// `pos := ANCHOR_CORNER_LUT[i]` (verbatim copy), `u := col + ICON_UV_LUT[i].u`, `v := row +
/// ICON_UV_LUT[i].v`. The UV adds are the arithmetic; the index list emitted alongside is
/// `[0, 1, 3, 3, 1, 2]` (two triangles).
///
/// ```text
///   6c7203: fld   st(1)            ; dup col
///   6c7217: fadd  dword [ecx]      ; col + iconUV[i].u   -> store local_10
///   6c7225: fld   st(0)            ; dup row (st0 carries row across the loop)
///   6c722a: fadd  dword [ecx-0x4]  ; row + iconUV[i].v   -> store local_8
/// ```
///
/// Returns the 4 `[pos(3), u, v]` vertices, the layout fed to the vertex buffer lock and draw.
pub fn billboard_quad(col: f32, row: f32) -> [[f32; 5]; 4] {
    let mut out = [[0.0f32; 5]; 4];
    for i in 0..4 {
        let p = ANCHOR_CORNER_LUT[i];
        let uv = ICON_UV_LUT[i];
        // pos copied verbatim (mov triple); u,v are `fadd dword` (col/row widened, narrowed at fstp).
        let u = (f64::from(col) + f64::from(uv[0])) as f32;
        let v = (f64::from(row) + f64::from(uv[1])) as f32;
        out[i] = [p[0], p[1], p[2], u, v];
    }
    out
}

/// The billboard index list emitted alongside the corner-quad (`0x6c728b..`): `[0, 1, 3, 3, 1, 2]`.
pub const BILLBOARD_INDICES: [u16; 6] = [0, 1, 3, 3, 1, 2];

/// Slice (3) — the **screen-Y accumulate** (the desc-anchor pass).
///
/// Two `fld [ebp-0x24]; fadd dword; fstp [ebp-0x24]` accumulates run on the screen-Y cursor
/// (`local_28`):
/// - `0x6c706e`: `screenY += desc[+0x30]` (the cached text-width × depth-scale), and
/// - `0x6c70d8`: `screenY += scale` (the raw depth-fade scalar, in the icon path).
///
/// Both are the same PC_53 `fld dword; fadd dword; fstp dword` shape; modelled by one helper.
pub fn screen_y_accumulate(screen_y: f32, add: f32) -> f32 {
    (f64::from(screen_y) + f64::from(add)) as f32 // fld dword; fadd dword; fstp dword
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// `0x6c8d00` pn_matrix_scale — arithmetic: the ortho-rect (left,right,bottom,top).
// ─────────────────────────────────────────────────────────────────────────────────────────────

/// `__ftol` (`0x40a2b0`) — the CRT float→int64 conversion, **truncate toward zero** (it forces
/// FPCW rounding-control to `0b11` = chop around an `fistp qword`). The slice keeps only the low
/// 32 bits (`mov eax, [ebp-0xc]`) — a wrapping `u32` of the truncated value (the high dword is
/// then explicitly **zeroed** in `[ebp-0xc]` before the `fild qword`, so a *negative* `__ftol`
/// like `-5 = 0xfffffffb` re-widens as the unsigned `4294967291`, not `-5`).
#[inline]
fn ftol_u32(v: f64) -> u32 {
    benilla_re_cmath::x87::ftol(v) as u32 // fistp qword (chop), keep low dword as raw bits
}

/// `0x73ff3f` — the CRT `floor` (round toward −∞), returning the floored integer as a `float`.
/// A direct call gives `0x73ff3f(2.5) = 2`, `(3.5) = 3`, `(-2.9) = -3`, `(-0.2) = -1` and
/// `(0.2) = 0`: exactly `v.floor()`, not a double to float narrow.
#[inline]
fn floor_f(v: f64) -> f32 {
    v.floor() as f32
}

/// The (left, right, bottom, top) ortho rect that `0x6c8d00` builds and passes to gxumath's
/// `projection_ortho` (`0x5c3d90`) — the arithmetic, **before** the delegated matrix build.
///
/// Inputs (all fetched via gx, fed here):
/// - viewport rect `(left, top, right, bottom)` (from `0x58a240`),
/// - the 4 device-scale floats `(s_left, s_right, s_bottom, s_top)` (from `0x58b060`); in the
///   binary's stack these are `local_28`, `local_38`, `local_2c`, `local_30` respectively,
/// - the half-pixel bias flag `half_pixel` (`true` when `[0x58a230()+4] == 0`): when set,
///   `x_bias = -0.5`, `y_bias = +0.5`; otherwise both `0.0`.
///
/// ```text
///   width_int  = u32(__ftol(right - left))   ; fld;fsub;__ftol   (two ftols, esi=height,eax=width)
///   height_int = u32(__ftol(bottom - top))   ; re-widened UNSIGNED via fild qword over a 0 high-dword
///   left   = floor( (f32)height_int * s_left )   + x_bias    ; fild qword; fst dword(->f32); fmul; 73ff3f
///   right  = floor(      height_int * s_right )  + x_bias    ; fild qword; fmul; 73ff3f
///   bottom = floor(      width_int  * s_bottom ) + y_bias    ; fild qword; fmul; 73ff3f
///   top    = floor( (width_int * s_top) as f32 ) + y_bias    ; fild qword; fmul; fstp dword; 73ff3f
/// ```
///
/// `0x73ff3f` is the CRT `floor` (round toward −∞), NOT a narrow — see [`floor_f`]. Note the
/// `height_int` path: the `fild` result is stored back to a f32 slot (`fst dword [ebp-0x4]`) and
/// that **f32-narrowed** copy is reused for the `* s_left` product (`0x6c8e40 fld [ebp-0x4]`),
/// whereas `* s_right` uses the full st0 `fild` value. The `width_int * s_top` product is stored
/// as f32 (`fstp [ebp-0x3c]`) before the floor.
#[allow(clippy::too_many_arguments)]
pub fn ortho_rect(
    vp_left: f32,
    vp_top: f32,
    vp_right: f32,
    vp_bottom: f32,
    s_left: f32,
    s_right: f32,
    s_bottom: f32,
    s_top: f32,
    half_pixel: bool,
) -> (f32, f32, f32, f32) {
    let (x_bias, y_bias) = if half_pixel {
        (-0.5f32, 0.5f32)
    } else {
        (0.0, 0.0)
    };

    // Two __ftol of (f32 - f32) widened to f64, truncated toward zero, low 32 bits kept as raw
    // u32, then re-widened via `fild qword` over a zeroed high dword → an UNSIGNED widen.
    let width_int = ftol_u32(f64::from(vp_right) - f64::from(vp_left));
    let height_int = ftol_u32(f64::from(vp_bottom) - f64::from(vp_top));

    // height_int as a double (unsigned), plus an f32-narrowed copy (the `fst dword [ebp-0x4]`).
    let h_f64 = f64::from(height_int);
    let h_f32 = h_f64 as f32; // fst dword [ebp-0x4]
    let w_f64 = f64::from(width_int);

    // left: floor((f32)height_int * s_left) + x_bias (fadd dword, narrowed at the final fstp).
    let left =
        (f64::from(floor_f(f64::from(h_f32) * f64::from(s_left))) + f64::from(x_bias)) as f32;
    // right: floor(height_int(f64) * s_right) + x_bias.
    let right = (f64::from(floor_f(h_f64 * f64::from(s_right))) + f64::from(x_bias)) as f32;
    // bottom: floor(width_int(f64) * s_bottom) + y_bias.
    let bottom = (f64::from(floor_f(w_f64 * f64::from(s_bottom))) + f64::from(y_bias)) as f32;
    // top: width_int(f64) * s_top stored as f32 (`fstp [ebp-0x3c]`), then floored via 73ff3f; + y_bias.
    let top_prod = (w_f64 * f64::from(s_top)) as f32; // fstp dword [ebp-0x3c]
    let top = (f64::from(floor_f(f64::from(top_prod))) + f64::from(y_bias)) as f32;

    (left, right, bottom, top)
}
