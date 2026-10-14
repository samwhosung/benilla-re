//! The element loops of `CM2Model::Animate` (`0x715966` to `0x717d6a`): colours, transparency,
//! texture frames, texture transforms, lights, cameras and ribbons, in the order of
//! `systems/animation/elements.md#element-families`. The cubic bases mix floats stored by the
//! code and 53-bit registers, differently for the vector and scalar forms and for the Hermite and
//! Bézier curves (`systems/animation/tracks.md#vector-hermite`).

use super::bone::{lambda_fires, lerp3, sample_vec3_inline};
use super::{Model, TrackMeta, Vec3State, Vec3TrackDef, IDENTITY};
use crate::sample_quat::{m2_track_sample_quat, QuatSampleState, QuatTrack};
use crate::sample_scalar::{m2_track_sample_byte, ByteSampleState, FloatSampleState};
use crate::track::{m2_track_search, TrackSearchResult, TrackTimes};
use benilla_re_cmath::quaternion::quat_mul4;
use benilla_re_cmath::transform::{scale_matrix_rows, translate_matrix};

/// A 16-bit fixed track (stride-2 keys, decoded as the word times 1/32767,
/// `systems/animation/tracks.md#value-codecs`).
#[derive(Clone, Default)]
pub struct I16TrackDef {
    pub meta: TrackMeta,
    pub payload: Vec<i16>,
}

/// A word track (stride-2 keys, no decode, never blended).
#[derive(Clone, Default)]
pub struct WordTrackDef {
    pub meta: TrackMeta,
    pub payload: Vec<u16>,
}

/// A camera vector track (stride-0x24 keys: value, in tangent and out tangent, three floats each).
#[derive(Clone, Default)]
pub struct CubicVec3TrackDef {
    pub meta: TrackMeta,
    pub payload: Vec<[[f32; 3]; 3]>,
}

/// A camera scalar track (stride-0xc keys: value, in tangent and out tangent).
#[derive(Clone, Default)]
pub struct CubicScalarTrackDef {
    pub meta: TrackMeta,
    pub payload: Vec<[f32; 3]>,
}

/// The state of a word track: the scalar state with 16-bit values.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct WordState {
    pub primary: TrackSearchResult,
    pub value: u16,
    pub secondary: TrackSearchResult,
    pub secondary_value: u16,
}

// The record types of each loop: the header record, and the instance state it writes.

/// Colours (header `+0x58`, stride 0x38; state `+0xa0`, stride 0x50): a vector track at `+0` and
/// a 16-bit track at `+0x1c`.
#[derive(Clone, Default)]
pub struct ColorElemDef {
    pub vec_track: Vec3TrackDef,
    pub fix_track: I16TrackDef,
}
#[derive(Clone, Copy, Default)]
pub struct ColorElemRec {
    pub vec: Vec3State,        // +0x00
    pub fix: FloatSampleState, // +0x30
}

/// Texture transforms (header `+0x78`, stride 0x54; state `+0xb0`, stride 0x98; matrix `+0xb4`,
/// stride 0x40): translation at `+0`, rotation at `+0x1c`, scale at `+0x38`.
#[derive(Clone, Default)]
pub struct TexXformDef {
    pub trans_track: Vec3TrackDef,
    pub quat_track: super::QuatTrackDef,
    pub scale_track: Vec3TrackDef,
}
#[derive(Clone, Copy, Default)]
pub struct TexXformRec {
    pub trans: Vec3State,      // +0x00
    pub quat: QuatSampleState, // +0x30
    pub scale: Vec3State,      // +0x68
}

/// Lights (header `+0x120`, stride 0xd4; state `+0x200`, stride 0x170), clocked by the bone in
/// the word at `+2`: vector at `+0x10`, scalar at `+0x2c`, vector at `+0x48`, scalar at `+0x64`,
/// byte at `+0xb8`.
#[derive(Clone, Default)]
pub struct LightElemDef {
    pub bone: u16,
    pub vec1: Vec3TrackDef,
    pub s1: super::ScalarTrackDef,
    pub vec2: Vec3TrackDef,
    pub s2: super::ScalarTrackDef,
    pub byte_track: super::ByteTrackDef,
}
#[derive(Clone, Copy, Default)]
pub struct LightElemRec {
    pub vec1: Vec3State,         // +0x00
    pub s1: FloatSampleState,    // +0x30
    pub vec2: Vec3State,         // +0x50
    pub s2: FloatSampleState,    // +0x80
    pub byte: ByteSampleState,   // +0xe0
    pub vis_enable: u32,         // +0x100 (read, not written: gates the byte track and the record)
    pub ambient: [f32; 3],       // +0x134 (s1.value · alpha · vec1.value)
    pub diffuse_light: [f32; 3], // +0x140 (s2.value · alpha · vec2.value)
}

/// Cameras (header `+0x128`, stride 0x7c; state `+0x3c4`, stride 0x84): vector tracks at `+0x10`
/// and `+0x38` and a scalar track at `+0x60`, each with the four forms of
/// `systems/animation/tracks.md#vector-hermite`.
#[derive(Clone, Default)]
pub struct CameraElemDef {
    pub c1: CubicVec3TrackDef,
    pub c2: CubicVec3TrackDef,
    pub c3: CubicScalarTrackDef,
}
#[derive(Clone, Copy, Default)]
pub struct CameraElemRec {
    pub c1: Vec3State,        // +0x00
    pub c2: Vec3State,        // +0x30
    pub c3: FloatSampleState, // +0x60
}

/// Ribbons (header `+0x138`, stride 0xdc; state `+0x3c8`, stride 0xd0), clocked by the bone in
/// the word at `+4`: vector at `+0x24`, 16-bit at `+0x40`, scalars at `+0x5c` and `+0x78`, word at
/// `+0xa4`, byte at `+0xc0`.
#[derive(Clone, Default)]
pub struct RibbonElemDef {
    pub bone: u16,
    pub vec: Vec3TrackDef,
    pub fix: I16TrackDef,
    pub s1: super::ScalarTrackDef,
    pub s2: super::ScalarTrackDef,
    pub word: WordTrackDef,
    pub byte_track: super::ByteTrackDef,
}
#[derive(Clone, Copy, Default)]
pub struct RibbonElemRec {
    pub vec: Vec3State,        // +0x00
    pub fix: FloatSampleState, // +0x30
    pub s1: FloatSampleState,  // +0x50
    pub s2: FloatSampleState,  // +0x70
    pub word: WordState,       // +0x90
    pub byte: ByteSampleState, // +0xb0
}

/// `[0x811610]`, the float nearest 1/32767, widened to 53 bits at each load.
const I16_SCALE: f64 = (1.0_f32 / 32767.0_f32) as f64;

/// The point the texture transform rotates and scales about, `(0.5, 0.5, 0)` (`[0xcf043c]`).
const PIVOT: [f32; 3] = [0.5, 0.5, 0.0];
/// The negated pivot, built with `fchs`: `(-0.5, -0.5, -0.0)`.
const NEG_PIVOT: [f32; 3] = [-0.5, -0.5, -0.0];

fn bone_times_of(m: &Model, bone: usize) -> TrackTimes {
    let blk = &m.blocks[bone];
    TrackTimes {
        time: blk.time,
        gtime: blk.gtime,
        blend_time: blk.blend_time,
        blend_gtime: blk.blend_gtime,
        lambda: blk.lambda,
    }
}

// The 16-bit fixed block (`systems/animation/tracks.md#value-codecs`).

/// `value = f32((b·S - a·S) · t + a·S)` with `S = [0x811610]`; a step track stores
/// `f32(p[k0] · S)` and skips the blend. The blend stores the second value as a float, but
/// blends from the 53-bit value (`fst` without a pop, `0x715c49`).
fn sample_i16_inline(
    gseq_times: &[u32],
    times: &TrackTimes,
    meta: &TrackMeta,
    payload: &[i16],
    st: &mut FloatSampleState,
) {
    let view = meta.view();
    m2_track_search(gseq_times, times.time, times.gtime, &view, &mut st.primary);
    if meta.interp == 0 {
        st.value = (f64::from(payload[st.primary.k0 as usize]) * I16_SCALE) as f32;
        return;
    }
    let a = f64::from(payload[st.primary.k0 as usize]) * I16_SCALE;
    let b = f64::from(payload[st.primary.k1 as usize]) * I16_SCALE;
    st.value = ((b - a) * f64::from(st.primary.t) + a) as f32;
    if lambda_fires(times.lambda) && meta.gseq == 0xffff {
        m2_track_search(
            gseq_times,
            times.blend_time,
            times.blend_gtime,
            &view,
            &mut st.secondary,
        );
        let a2 = f64::from(payload[st.secondary.k0 as usize]) * I16_SCALE;
        let b2 = f64::from(payload[st.secondary.k1 as usize]) * I16_SCALE;
        let w2 = (b2 - a2) * f64::from(st.secondary.t) + a2;
        st.secondary_value = w2 as f32;
        let prev = f64::from(st.value);
        st.value = ((w2 - prev) * f64::from(times.lambda) + prev) as f32;
    }
}

// The word block: the value at `k`, and with the blend leg the second value, never blended
// (`systems/animation/tracks.md#sampling-block`).

fn sample_word_inline(
    gseq_times: &[u32],
    times: &TrackTimes,
    meta: &TrackMeta,
    payload: &[u16],
    st: &mut WordState,
) {
    let view = meta.view();
    m2_track_search(gseq_times, times.time, times.gtime, &view, &mut st.primary);
    st.value = payload[st.primary.k0 as usize];
    if meta.interp != 0 && lambda_fires(times.lambda) && meta.gseq == 0xffff {
        m2_track_search(
            gseq_times,
            times.blend_time,
            times.blend_gtime,
            &view,
            &mut st.secondary,
        );
        st.secondary_value = payload[st.secondary.k0 as usize];
    }
}

// The camera loop's curves (`systems/animation/tracks.md#vector-hermite`, `#vector-bézier`,
// `#scalar-cubics`).

/// The vector value by interpolation word: 0 step, 1 linear, 2 Bézier, 3 Hermite; any other
/// word leaves the value.
fn cubic_vec3_value(out: &mut [f32; 3], interp: u16, p: &[[[f32; 3]; 3]], r: &TrackSearchResult) {
    let (k0, k1) = (r.k0 as usize, r.k1 as usize);
    match interp {
        0 => *out = p[k0][0],
        1 => *out = lerp3(p[k0][0], p[k1][0], r.t),
        2 => {
            // Bézier (`0x716c41`): `t³`, `3t²` and `3·f32(t³)` are stored as floats, `3t` stays at
            // 53 bits, and `B1` and `B2` are 53-bit.
            let f = f64::from;
            let t = f(r.t);
            let t2 = t * t;
            let t3s = (t2 * t) as f32;
            let s3t2 = (3.0 * t2) as f32;
            let threet = t * 3.0;
            let b0 = (((f(s3t2) - f(t3s)) - threet) + 1.0) as f32;
            let t3x3 = (f(t3s) * 3.0) as f32;
            let b1 = (f(t3x3) - t2 * 6.0) + threet;
            let b2 = f(s3t2) - f(t3x3);
            let b3 = f(t3s);
            let (a, b) = (&p[k0], &p[k1]);
            for i in 0..3 {
                out[i] = (((f(b0) * f(a[0][i]) + b3 * f(b[0][i])) + b1 * f(a[2][i]))
                    + b2 * f(b[1][i])) as f32;
            }
        }
        3 => {
            // Hermite (`0x716b9e`): `2t³` and `3t²` are stored as floats, `h00` and `h10` are
            // narrowed to floats, and `h11` and `h01` stay at 53 bits.
            let f = f64::from;
            let t = f(r.t);
            let t2 = t * t;
            let t3 = t2 * t;
            let s2t3 = (t3 + t3) as f32;
            let s3t2 = (t2 * 3.0) as f32;
            let h00 = ((f(s2t3) - f(s3t2)) + 1.0) as f32;
            let h10 = ((t3 - (t2 + t2)) + t) as f32;
            let h11 = t3 - t2;
            let h01 = f(s3t2) - f(s2t3);
            let (a, b) = (&p[k0], &p[k1]);
            for i in 0..3 {
                out[i] = (((h11 * f(b[1][i]) + f(h00) * f(a[0][i])) + h01 * f(b[0][i]))
                    + f(h10) * f(a[2][i])) as f32;
            }
        }
        _ => {}
    }
}

fn sample_cubic_vec3(
    gseq_times: &[u32],
    times: &TrackTimes,
    track: &CubicVec3TrackDef,
    st: &mut Vec3State,
) {
    let view = track.meta.view();
    m2_track_search(gseq_times, times.time, times.gtime, &view, &mut st.primary);
    if track.meta.interp == 0 {
        st.value = track.payload[st.primary.k0 as usize][0];
        return; // a step track skips the blend leg
    }
    cubic_vec3_value(
        &mut st.value,
        track.meta.interp,
        &track.payload,
        &st.primary,
    );
    if lambda_fires(times.lambda) && track.meta.gseq == 0xffff {
        m2_track_search(
            gseq_times,
            times.blend_time,
            times.blend_gtime,
            &view,
            &mut st.blend,
        );
        let blend = st.blend;
        cubic_vec3_value(
            &mut st.blend_value,
            track.meta.interp,
            &track.payload,
            &blend,
        );
        let lam = f64::from(times.lambda);
        for i in 0..3 {
            let prev = f64::from(st.value[i]);
            st.value[i] = ((f64::from(st.blend_value[i]) - prev) * lam + prev) as f32;
        }
    }
}

/// The scalar value by interpolation word (`0x717419`, `0x717466`): the bases stay at 53 bits
/// except the Bézier's `3t²`, which is stored as a float. The accumulate orders differ from the
/// vector form. `None` for a word above 3.
fn cubic_scalar_value(interp: u16, p: &[[f32; 3]], r: &TrackSearchResult) -> Option<f32> {
    let f = f64::from;
    let (k0, k1) = (r.k0 as usize, r.k1 as usize);
    let (a, b) = (&p[k0], &p[k1]);
    let (av, ao) = (f(a[0]), f(a[2]));
    let (bv, bi) = (f(b[0]), f(b[1]));
    match interp {
        0 => Some(p[k0][0]),
        1 => {
            let t = f(r.t);
            Some(((bv - av) * t + av) as f32)
        }
        2 => {
            let t = f(r.t);
            let t2 = t * t;
            let t3 = t2 * t;
            let threet3 = 3.0 * t3;
            let threet = t * 3.0;
            let s3t2 = (3.0 * t2) as f32; // the only stored float (`0x71748c`)
            let b1 = (threet3 - t2 * 6.0) + threet;
            let b2 = f(s3t2) - threet3;
            let b0 = ((f(s3t2) - t3) - threet) + 1.0;
            Some((((b1 * ao + b2 * bi) + b0 * av) + t3 * bv) as f32)
        }
        3 => {
            let t = f(r.t);
            let t2 = t * t;
            let t3 = t2 * t;
            let h10 = (t3 - (t2 + t2)) + t;
            let h11 = t3 - t2;
            let h00 = ((t3 + t3) - t2 * 3.0) + 1.0;
            let h01 = t2 * 3.0 - (t3 + t3);
            Some((((h10 * ao + h11 * bi) + h00 * av) + h01 * bv) as f32)
        }
        _ => None,
    }
}

fn sample_cubic_scalar(
    gseq_times: &[u32],
    times: &TrackTimes,
    track: &CubicScalarTrackDef,
    st: &mut FloatSampleState,
) {
    let view = track.meta.view();
    m2_track_search(gseq_times, times.time, times.gtime, &view, &mut st.primary);
    if track.meta.interp == 0 {
        st.value = track.payload[st.primary.k0 as usize][0];
        return;
    }
    if let Some(v) = cubic_scalar_value(track.meta.interp, &track.payload, &st.primary) {
        st.value = v;
    }
    if lambda_fires(times.lambda) && track.meta.gseq == 0xffff {
        m2_track_search(
            gseq_times,
            times.blend_time,
            times.blend_gtime,
            &view,
            &mut st.secondary,
        );
        if let Some(w2) = cubic_scalar_value(track.meta.interp, &track.payload, &st.secondary) {
            st.secondary_value = w2;
            let prev = f64::from(st.value);
            st.value = ((f64::from(w2) - prev) * f64::from(times.lambda) + prev) as f32;
        }
    }
}

/// The element loops, between the bone loop and the particle loop. Each walks its header records
/// and its state records in parallel, trusting the header count.
pub(super) fn element_loops(models: &mut [Model], idx: usize) {
    let gseq_times = models[idx].gseq_times.clone();
    let first_update_gate = models[idx].first_update_gate;
    let alpha = models[idx].alpha;

    // Colours (bone 0's clocks).
    for i in 0..models[idx].header.color_elems.len() {
        let def = models[idx].header.color_elems[i].clone();
        let times = bone_times_of(&models[idx], 0);
        if def.vec_track.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.color_recs[i].vec;
            sample_vec3_inline(&gseq_times, &times, &def.vec_track, &mut st);
            m.color_recs[i].vec = st;
        }
        if def.fix_track.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.color_recs[i].fix;
            sample_i16_inline(
                &gseq_times,
                &times,
                &def.fix_track.meta,
                &def.fix_track.payload,
                &mut st,
            );
            m.color_recs[i].fix = st;
        }
    }

    // Transparency.
    for i in 0..models[idx].header.fix16_elems.len() {
        let def = models[idx].header.fix16_elems[i].clone();
        if def.meta.key_count > first_update_gate {
            let times = bone_times_of(&models[idx], 0);
            let m = &mut models[idx];
            let mut st = m.fix16_recs[i];
            sample_i16_inline(&gseq_times, &times, &def.meta, &def.payload, &mut st);
            m.fix16_recs[i] = st;
        }
    }

    // Texture frames.
    for i in 0..models[idx].header.word_elems.len() {
        let def = models[idx].header.word_elems[i].clone();
        if def.meta.key_count > first_update_gate {
            let times = bone_times_of(&models[idx], 0);
            let m = &mut models[idx];
            let mut st = m.word_recs[i];
            sample_word_inline(&gseq_times, &times, &def.meta, &def.payload, &mut st);
            m.word_recs[i] = st;
        }
    }

    // Texture transforms: each matrix starts as the identity, and a track runs whenever its key
    // count is not 0.
    for i in 0..models[idx].header.tex_xforms.len() {
        let def = models[idx].header.tex_xforms[i].clone();
        let times = bone_times_of(&models[idx], 0);
        let mut m4: [f32; 16] = IDENTITY;
        // Rotation about the pivot.
        if def.quat_track.meta.key_count != 0 {
            let qt = QuatTrack {
                interp: def.quat_track.meta.interp,
                gseq: def.quat_track.meta.gseq,
                range_count: def.quat_track.meta.range_count,
                ranges: &def.quat_track.meta.ranges,
                key_count: def.quat_track.meta.key_count,
                timestamps: &def.quat_track.meta.timestamps,
                quats: &def.quat_track.payload,
            };
            let m = &mut models[idx];
            let mut st = m.tex_states[i].quat;
            m2_track_sample_quat(&gseq_times, &times, &qt, &mut st);
            m.tex_states[i].quat = st;
            translate_matrix(&mut m4, &PIVOT);
            m4 = quat_mul4(&m4, &st.value);
            translate_matrix(&mut m4, &NEG_PIVOT);
        }
        // Scale about the pivot.
        if def.scale_track.meta.key_count != 0 {
            let m = &mut models[idx];
            let mut st = m.tex_states[i].scale;
            sample_vec3_inline(&gseq_times, &times, &def.scale_track, &mut st);
            m.tex_states[i].scale = st;
            translate_matrix(&mut m4, &PIVOT);
            scale_matrix_rows(&mut m4, st.value);
            translate_matrix(&mut m4, &NEG_PIVOT);
        }
        // Translation.
        if def.trans_track.meta.key_count != 0 {
            let m = &mut models[idx];
            let mut st = m.tex_states[i].trans;
            sample_vec3_inline(&gseq_times, &times, &def.trans_track, &mut st);
            m.tex_states[i].trans = st;
            translate_matrix(&mut m4, &st.value);
        }
        models[idx].tex_matrices[i] = m4;
    }

    // Lights (`systems/animation/elements.md#light-loop`), clocked by the record's bone.
    for i in 0..models[idx].header.light_elems.len() {
        let def = models[idx].header.light_elems[i].clone();
        let times = bone_times_of(&models[idx], def.bone as usize);
        let vis = models[idx].light_recs[i].vis_enable;
        // The visibility byte runs when the enabled word is nonzero and the track has keys; its
        // count is tested against 0, not the gate (`0x716404`, `0x716415`).
        if vis != 0 && def.byte_track.meta.key_count != 0 {
            let m = &mut models[idx];
            let mut st = m.light_recs[i].byte;
            m2_track_sample_byte(
                &gseq_times,
                &times,
                &def.byte_track.meta.view(),
                &def.byte_track.payload,
                &mut st,
            );
            m.light_recs[i].byte = st;
        }
        // The other tracks run when the enabled word and the byte are both nonzero, or the gate
        // is 0 (`0x7164f2`, `0x716506`).
        let byte_on = models[idx].light_recs[i].byte.value != 0;
        if !((vis != 0 && byte_on) || first_update_gate == 0) {
            continue;
        }
        // The scalar at `+0x2c` into the state at `+0x30`.
        if def.s1.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.light_recs[i].s1;
            crate::sample_scalar::m2_track_sample_float(
                &gseq_times,
                &times,
                &def.s1.meta.view(),
                &def.s1.payload,
                &mut st,
            );
            m.light_recs[i].s1 = st;
        }
        // The vector at `+0x10` into the state at `+0x00`; the product stored at `+0x134` is
        // inside this gate, so a skipped track leaves it as it was.
        if def.vec1.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.light_recs[i].vec1;
            sample_vec3_inline(&gseq_times, &times, &def.vec1, &mut st);
            m.light_recs[i].vec1 = st;
            let r = &mut m.light_recs[i];
            // One 53-bit product serves all three components.
            let v = f64::from(r.s1.value) * f64::from(alpha);
            for k in 0..3 {
                r.ambient[k] = (v * f64::from(r.vec1.value[k])) as f32;
            }
        }
        // The scalar at `+0x64` into the state at `+0x80`.
        if def.s2.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.light_recs[i].s2;
            crate::sample_scalar::m2_track_sample_float(
                &gseq_times,
                &times,
                &def.s2.meta.view(),
                &def.s2.payload,
                &mut st,
            );
            m.light_recs[i].s2 = st;
        }
        // The vector at `+0x48` into the state at `+0x50`; the product at `+0x140` is inside the
        // gate likewise.
        if def.vec2.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.light_recs[i].vec2;
            sample_vec3_inline(&gseq_times, &times, &def.vec2, &mut st);
            m.light_recs[i].vec2 = st;
            let r = &mut m.light_recs[i];
            let v = f64::from(r.s2.value) * f64::from(alpha);
            for k in 0..3 {
                r.diffuse_light[k] = (v * f64::from(r.vec2.value[k])) as f32;
            }
        }
    }

    // Cameras (bone 0's clocks).
    for i in 0..models[idx].header.camera_elems.len() {
        let def = models[idx].header.camera_elems[i].clone();
        let times = bone_times_of(&models[idx], 0);
        if def.c1.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.camera_recs[i].c1;
            sample_cubic_vec3(&gseq_times, &times, &def.c1, &mut st);
            m.camera_recs[i].c1 = st;
        }
        if def.c2.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.camera_recs[i].c2;
            sample_cubic_vec3(&gseq_times, &times, &def.c2, &mut st);
            m.camera_recs[i].c2 = st;
        }
        if def.c3.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.camera_recs[i].c3;
            sample_cubic_scalar(&gseq_times, &times, &def.c3, &mut st);
            m.camera_recs[i].c3 = st;
        }
    }

    // Ribbons, clocked by the record's bone.
    for i in 0..models[idx].header.ribbon_elems.len() {
        let def = models[idx].header.ribbon_elems[i].clone();
        let times = bone_times_of(&models[idx], def.bone as usize);
        if def.byte_track.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.ribbon_recs[i].byte;
            m2_track_sample_byte(
                &gseq_times,
                &times,
                &def.byte_track.meta.view(),
                &def.byte_track.payload,
                &mut st,
            );
            m.ribbon_recs[i].byte = st;
        }
        if def.vec.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.ribbon_recs[i].vec;
            sample_vec3_inline(&gseq_times, &times, &def.vec, &mut st);
            m.ribbon_recs[i].vec = st;
        }
        if def.fix.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.ribbon_recs[i].fix;
            sample_i16_inline(
                &gseq_times,
                &times,
                &def.fix.meta,
                &def.fix.payload,
                &mut st,
            );
            m.ribbon_recs[i].fix = st;
        }
        if def.s1.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.ribbon_recs[i].s1;
            crate::sample_scalar::m2_track_sample_float(
                &gseq_times,
                &times,
                &def.s1.meta.view(),
                &def.s1.payload,
                &mut st,
            );
            m.ribbon_recs[i].s1 = st;
        }
        if def.s2.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.ribbon_recs[i].s2;
            crate::sample_scalar::m2_track_sample_float(
                &gseq_times,
                &times,
                &def.s2.meta.view(),
                &def.s2.payload,
                &mut st,
            );
            m.ribbon_recs[i].s2 = st;
        }
        if def.word.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.ribbon_recs[i].word;
            sample_word_inline(
                &gseq_times,
                &times,
                &def.word.meta,
                &def.word.payload,
                &mut st,
            );
            m.ribbon_recs[i].word = st;
        }
    }
}
