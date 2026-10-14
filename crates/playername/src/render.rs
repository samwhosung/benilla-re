//! The render-path arithmetic of `PlayerName.cpp`.
//!
//! Two functions do floating-point and integer arithmetic on the world-text render path; the rest
//! of what they call (the vertex-buffer lock, `DrawPrimitive`, the device state, the font measure
//! and the `SArray` reallocation) belongs to other layers and is not transcribed here.
//!
//! # `0x6c7f10` `pn_glyph_quad_build` — push a `SWorldTextIcon` record (stride `0x18`)
//! The FP, in emit order (`eax` = record base, `ebx` = `this`):
//! ```text
//!   6c8009: fld   [ebp-0x4]        ; local_4 = 0.0   (= [ebp-0x20]-[ebp-0x28], both zeroed)
//!   6c800f: fmul  [ebx+0x10]       ; 0.0 · this[0x10]
//!   6c8012: fadd  st(0),st         ; + itself
//!   6c8014: fstp  [eax+0xc]        ; rec[3] = (0.0·this[0x10]) + (0.0·this[0x10])   (·0.0 idiom)
//!   6c8017: mov   [eax+0x14], [0xce88dc + (type-5)*4]   ; rec[5] = tex handle (dword copy)
//!   6c8039: fmul  [eax]            ; local_c(0.0) · rec[0]   (rec[0] just set = this[0x30])
//!   6c803c: fsub  [0x81130c]=40.0  ; - 40.0
//!   6c8042: fstp  [eax]            ; rec[0] = (0.0·this[0x30]) - 40.0  = -40.0
//!   6c8044: fld   [ebp-0x4]        ; local_4 = 0.0
//!   6c8047: fmul  [eax+0x4]        ; 0.0 · rec[1]   (rec[1] just set = this[0x34])
//!   6c804a: fld   [eax+0xc]        ; rec[3]
//!   6c804d: fsub  [0x806534]=32.0  ; rec[3] - 32.0
//!   6c8053: fmul  [0x7ffa24]=0.5   ; (rec[3]-32.0) · 0.5
//!   6c8059: fsubp st(1),st         ; st(1) = (0.0·this[0x34]) - ((rec[3]-32.0)·0.5)
//!   6c805b: fstp  [eax+0x4]        ; rec[1]
//!   6c805e: mov   [eax+0x10], this[0x20]   ; rec[4] = color (dword copy)
//! ```
//! hazards:
//! - **`·0.0` zeroing idiom (×3)** — `local_4`/`local_c` are the *value* `0.0`; the binary emits
//!   `fmul`, NOT a load of `0`, so the sign of the zero result follows the other operand
//!   (`0.0·(-x) = -0.0`). Transcribed as `0.0f64 * x` to keep that signed-zero result exact.
//! - **`fsubp st(1),st`** at `0x6c8059` — `st(1) - st(0)`, i.e. `(0.0·this[0x34]) - ((rec3-32)·0.5)`,
//!   operand order from the bytes (NOT the reverse).
//!
//! # `0x6c8820` `pn_render_driver` — the 1248-byte per-frame world-text driver
//! It calls into the graphics layer (vertex-buffer lock, `DrawPrimitive`) and has three arithmetic
//! slices:
//! 1. the text-quad vertex build loop `[0x6c8ae7, 0x6c8b3b)` — 4 corners, each
//!    `dst.xyz = src_lut.xyz · 32.0` (`[0x806534]`=32.0; `src` from the `0xce87c0` xform LUT),
//!    vertex stride `0x14`;
//! 2. the index pattern `{0,1,3,3,1,2}` (constant `word` writes — integer);
//! 3. the per-icon color-pack `(icon[+0x13] << 24) | 0x00ffffff` (integer).
#![allow(clippy::float_cmp)]

/// `0x806534` (`.rdata`) — the quad-scale / fade-pivot constant **32.0** (`0x42000000`).
pub const SCALE_32_BITS: u32 = 0x4200_0000;
/// `0x81130c` (`.rdata`) — the glyph X bias **40.0** (`0x42200000`).
pub const BIAS_40_BITS: u32 = 0x4220_0000;
/// `0x7ffa24` (`.rdata`) — the **0.5** half-scale (`0x3f000000`).
pub const HALF_BITS: u32 = 0x3f00_0000;

#[inline]
fn scale_32() -> f64 {
    f64::from(f32::from_bits(SCALE_32_BITS))
}
#[inline]
fn bias_40() -> f64 {
    f64::from(f32::from_bits(BIAS_40_BITS))
}
#[inline]
fn half() -> f64 {
    f64::from(f32::from_bits(HALF_BITS))
}

// ===================================================================================
// 0x6c7f10  pn_glyph_quad_build — the record-float arithmetic
// ===================================================================================

/// The 6-dword `SWorldTextIcon` record (stride `0x18`), as the four FP slots the binary writes
/// plus the two dword copies. `pos` is `rec[0..3]`, `rec3` is `[+0x0c]`, `color` is `rec[4]`
/// (`[+0x10]`), `tex` is `rec[5]` (`[+0x14]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldTextIcon {
    /// `rec[0]` (`[+0x00]`) bits — the screen-quad anchor X.
    pub x_bits: u32,
    /// `rec[1]` (`[+0x04]`) bits — the screen-quad anchor Y.
    pub y_bits: u32,
    /// `rec[2]` (`[+0x08]`) bits — `this[0x38]` (dword copy, no arithmetic).
    pub z_bits: u32,
    /// `rec[3]` (`[+0x0c]`) bits — the `·0.0` doubled-zero scratch (`+0x0c`).
    pub rec3_bits: u32,
    /// `rec[4]` (`[+0x10]`) bits — `this[0x20]` color (dword copy).
    pub color_bits: u32,
    /// `rec[5]` (`[+0x14]`) bits — `0xce88dc[type-5]` texture handle (dword copy).
    pub tex_bits: u32,
}

/// `0x6c7f10`'s record build. Given the icon `type` (5..=19) and the `this` fields the FP
/// slice reads — `this[0x10]` (`t10`), `this[0x30]/0x34/0x38` (the screen anchor px), `this[0x20]`
/// (color), and the texture handle resolved from `0xce88dc[type-5]` — produce the `SWorldTextIcon`
/// record exactly as the binary stores its dwords.
///
/// All three `·0.0` products use the binary's signed-zero-preserving `f64 0.0 * x`; the order
/// `0.0 * t10` matches the emitted `fmul [ebx+0x10]` (`st0 = local_4 · this[0x10]`, commutative).
#[allow(clippy::similar_names)]
pub fn glyph_quad_build(
    this_10: f32,
    this_30: f32,
    this_34: f32,
    this_38: f32,
    this_20_color: u32,
    tex_handle: u32,
) -> WorldTextIcon {
    // local_4 and local_c are the f32 value 0.0 (= [ebp-0x20]-[ebp-0x28] and [ebp-0x1c]-[ebp-0x24],
    // every term zeroed); kept f64 through PC_53 like the x87 stack.
    let local_4 = 0.0f64; // +0.0
    let local_c = 0.0f64; // +0.0

    // rec[3] = (local_4 · this[0x10]) + (local_4 · this[0x10])  — the ·0.0 idiom, then `fadd st,st`.
    let prod = local_4 * f64::from(this_10); // fmul [ebx+0x10]
    let rec3 = (prod + prod) as f32; // fadd st(0),st ; fstp [eax+0xc]

    // rec[0] = (local_c · this[0x30]) - 40.0   (= -40.0; rec[0] preloaded with this[0x30]).
    let x = ((local_c * f64::from(this_30)) - bias_40()) as f32; // fmul [eax]; fsub [0x81130c]; fstp [eax]

    // rec[1] = (local_4 · this[0x34]) - ((rec[3] - 32.0) · 0.5)
    //   fld [ebp-0x4]; fmul [eax+0x4]   -> st0 = 0.0·this[0x34]
    //   fld [eax+0xc]; fsub [0x806534]; fmul [0x7ffa24]  -> st0 = (rec3-32)·0.5,  st1 = 0.0·this34
    //   fsubp st(1),st  -> st1 = st1 - st0 = (0.0·this34) - (rec3-32)·0.5
    let zero_t34 = local_4 * f64::from(this_34); // st(1)
    let pivot = (f64::from(rec3) - scale_32()) * half(); // st(0): (rec3-32)·0.5
    let y = (zero_t34 - pivot) as f32; // fsubp st(1),st ; fstp [eax+0x4]

    WorldTextIcon {
        x_bits: x.to_bits(),
        y_bits: y.to_bits(),
        z_bits: this_38.to_bits(), // [eax+0x8] = this[0x38] (dword copy)
        rec3_bits: rec3.to_bits(),
        color_bits: this_20_color, // [eax+0x10] = this[0x20] (dword copy)
        tex_bits: tex_handle,      // [eax+0x14] = 0xce88dc[type-5] (dword copy)
    }
}

// ===================================================================================
// 0x6c8820  pn_render_driver — the arithmetic slices
// ===================================================================================

/// `[0x6c8ae7, 0x6c8b3b)` quad-vertex scale: `dst.xyz = src.xyz · 32.0` (`fmul [0x806534]`,
/// each component, then `fstp`). One corner; the driver runs it 4× over the `0xce87c0` LUT.
pub fn quad_vertex_scale(src: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(src[0]) * scale_32()) as f32,
        (f64::from(src[1]) * scale_32()) as f32,
        (f64::from(src[2]) * scale_32()) as f32,
    ]
}

/// `0xce87c0` (set by the initializer `0x6c86f0`): the quad-position source table of four vec3, the loop's
/// `src.xyz`. The bits are the values the initializer writes.
pub const QUAD_POS_LUT: [[f32; 3]; 4] = [
    [0.0, 1.0, 0.0], // 0xce87c0
    [1.0, 0.0, 1.0], // 0xce87cc
    [1.0, 0.0, 0.0], // 0xce87d8
    [0.0, 0.0, 0.0], // 0xce87e4
];

/// The four scaled quad corners the driver produces from [`QUAD_POS_LUT`] (`src·32`).
pub fn quad_corners() -> [[f32; 3]; 4] {
    [
        quad_vertex_scale(QUAD_POS_LUT[0]),
        quad_vertex_scale(QUAD_POS_LUT[1]),
        quad_vertex_scale(QUAD_POS_LUT[2]),
        quad_vertex_scale(QUAD_POS_LUT[3]),
    ]
}

/// `0x6c8b78..0x6c8b8d` — the 6 `word` index writes `{0,1,3,3,1,2}` (two-triangle quad, integer).
pub const QUAD_INDICES: [u16; 6] = [0, 1, 3, 3, 1, 2];

/// `0x6c8bc7..0x6c8bcf` per-icon color pack: `movzx ecx,[icon+0x13]; shl ecx,0x18; or ecx,0xffffff`
/// → `((alpha as u32) << 24) | 0x00ff_ffff` (the icon alpha byte over an opaque-white RGB).
#[inline]
#[must_use]
pub fn icon_color_pack(alpha: u8) -> u32 {
    (u32::from(alpha) << 24) | 0x00ff_ffff
}
