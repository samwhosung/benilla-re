//! The bone loop body of `CM2Model::Animate` (`0x71492d` to `0x715960`), after the clocks and the
//! blend weight: the flag dispatch, the ignore-parent legs, the pivot, the rotation, scale and
//! translation legs and their composition, and the billboard stage with its common tail
//! (`systems/animation/pose.md`, `systems/animation/billboards.md`). Each sum keeps the
//! association the code uses and rounds to `f32` where it stores.

use super::clock::{bone_clocks, bone_lambda};
use super::{Model, Vec3State, Vec3TrackDef, IDENTITY};
use crate::mat::m2_quat_to_mat4;
use crate::sample_quat::{m2_track_sample_quat, QuatTrack};
use crate::track::{m2_track_search, TrackTimes};
use benilla_re_cmath::multiply::matmul4x4_sse2;
use benilla_re_cmath::transform::scale_matrix_rows;

/// `[0x8029d4]`, 2^-22: the length below which a row is not normalized (the billboard cases and
/// the `& 6 == 2` leg).
const UNIT_EPS: f64 = 2.384_185_791_015_625e-7;
/// `[0x80c5c8]`, 1e-5: the squared world row length at or below which the `& 6 == 4` leg uses a
/// ratio of 1.0.
const RATIO_EPS: f64 = 1.0e-5;

/// The blend gate every blend leg shares (`fld 0.0; fcomp λ; test ah,0x44; jnp`): the leg is
/// skipped only when the weight equals 0, so a negative or NaN weight runs it
/// (`systems/animation/tracks.md#sampling-block`).
pub(super) fn lambda_fires(lambda: f32) -> bool {
    lambda != 0.0
}

/// `0x4549f0`: the squared length `(x² + y²) + z²`, left on the x87 stack at 53 bits.
fn self_dot(v: [f32; 3]) -> f64 {
    f64::from(v[0]) * f64::from(v[0])
        + f64::from(v[1]) * f64::from(v[1])
        + f64::from(v[2]) * f64::from(v[2])
}

/// The row normalize the legs and the billboard cases share: `len = sqrt(0x4549f0(row))`; when
/// `|len| > 2^-22` each component becomes `f32((1.0 / len) · comp)`, else the row is unchanged.
fn normalize_row_eps(row: &mut [f32]) {
    let len = self_dot([row[0], row[1], row[2]]).sqrt();
    if len.abs() > UNIT_EPS {
        let recip = 1.0 / len;
        for c in row.iter_mut().take(3) {
            *c = (recip * f64::from(*c)) as f32;
        }
    }
}

/// Case 3's inline normalize: [`normalize_row_eps`] with the squared length summed
/// `(y² + z²) + x²` (`0x71574b`, `0x7157c7`) instead of `0x4549f0`'s `(x² + y²) + z²`.
fn normalize_row_eps_arm3(row: &mut [f32]) {
    let f = f64::from;
    let len = (f(row[0]) * f(row[0]) + (f(row[1]) * f(row[1]) + f(row[2]) * f(row[2]))).sqrt();
    if len.abs() > UNIT_EPS {
        let recip = 1.0 / len;
        for c in row.iter_mut().take(3) {
            *c = (recip * f64::from(*c)) as f32;
        }
    }
}

/// The three-component interpolation `out[i] = f32((b[i] - a[i]) · t + a[i])`
/// (`fld b; fsub a; fmul t; fadd a; fstp`).
pub(super) fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let tf = f64::from(t);
    let mut out = [0f32; 3];
    for i in 0..3 {
        out[i] = ((f64::from(b[i]) - f64::from(a[i])) * tf + f64::from(a[i])) as f32;
    }
    out
}

/// The vector track block the bone loop and the texture transforms inline
/// (`systems/animation/tracks.md#sampling-block`): the key search, a step copy or an
/// interpolation, and with a nonzero blend weight on a track with no global sequence the second
/// search and interpolation, then `value + (blend - value) · λ` in place. The searches start from
/// the state's previous `k0`; the blend reads the blend value after its store to `f32`.
pub(super) fn sample_vec3_inline(
    gseq_times: &[u32],
    times: &TrackTimes,
    track: &Vec3TrackDef,
    state: &mut Vec3State,
) {
    let view = track.meta.view();
    m2_track_search(
        gseq_times,
        times.time,
        times.gtime,
        &view,
        &mut state.primary,
    );
    if track.meta.interp == 0 {
        // a step track copies the value and skips the blend leg (`0x71500b jmp 0x715107`).
        state.value = track.payload[state.primary.k0 as usize];
        return;
    }
    let a = track.payload[state.primary.k0 as usize];
    let b = track.payload[state.primary.k1 as usize];
    state.value = lerp3(a, b, state.primary.t);
    if lambda_fires(times.lambda) && track.meta.gseq == 0xffff {
        m2_track_search(
            gseq_times,
            times.blend_time,
            times.blend_gtime,
            &view,
            &mut state.blend,
        );
        let a2 = track.payload[state.blend.k0 as usize];
        let b2 = track.payload[state.blend.k1 as usize];
        state.blend_value = lerp3(a2, b2, state.blend.t);
        // value[i] = f32((blend_value[i] - value[i]) · λ + value[i]).
        let lam = f64::from(times.lambda);
        for i in 0..3 {
            let prev = f64::from(state.value[i]);
            state.value[i] = ((f64::from(state.blend_value[i]) - prev) * lam + prev) as f32;
        }
    }
}

/// The bone's `TrackTimes`, with a copy of the global-sequence clocks so the sampler can borrow
/// the model.
fn bone_times(models: &[Model], idx: usize, bone_idx: usize) -> (Vec<u32>, TrackTimes) {
    let m = &models[idx];
    let blk = &m.blocks[bone_idx];
    (
        m.gseq_times.clone(),
        TrackTimes {
            time: blk.time,
            gtime: blk.gtime,
            blend_time: blk.blend_time,
            blend_gtime: blk.blend_gtime,
            lambda: blk.lambda,
        },
    )
}

/// One billboard cross product `rows[dst] = rows[a] × rows[b]`; each component is
/// `f32(p·q - r·s)`.
fn cross_into(m: &mut [f32; 16], dst: usize, a: usize, b: usize) {
    let f = f64::from;
    let x = (f(m[a + 2]) * f(m[b + 1]) - f(m[a + 1]) * f(m[b + 2])) as f32;
    let y = (f(m[a]) * f(m[b + 2]) - f(m[a + 2]) * f(m[b])) as f32;
    let z = (f(m[a + 1]) * f(m[b]) - f(m[a]) * f(m[b + 1])) as f32;
    m[dst] = x;
    m[dst + 1] = y;
    m[dst + 2] = z;
}

/// One bone: its clocks and blend weight, the flag dispatch (for `& 7`: the working copy, the
/// pivot, the `& 6` leg and the `& 1` translation), the `& 0x280` route (the track legs and the
/// product with the parent, or a copy of it), then the `& 0x78` billboard cases and their tail.
pub(super) fn animate_bone(
    models: &mut [Model],
    idx: usize,
    bone_idx: usize,
    working: &mut [f32; 16],
    rot: &mut [f32; 16],
    wrap_delta: u32,
) {
    let (dur, pause_stamp, bone_count, first_update_gate) = {
        let m = &models[idx];
        (
            m.scene_clock,
            m.pause_stamp,
            m.header.bone_count,
            m.first_update_gate,
        )
    };
    let def = models[idx].header.bone_defs[bone_idx].clone();

    // The clocks and the blend weight (`systems/animation/clock.md`).
    {
        let m = &mut models[idx];
        bone_clocks(
            &mut m.blocks,
            &m.header.seqs,
            bone_count,
            def.parent,
            bone_idx,
            dur,
            pause_stamp,
            wrap_delta,
        );
        bone_lambda(
            &mut m.blocks,
            bone_count,
            def.parent,
            bone_idx,
            m.scene_clock,
        );
    }

    // The flags and the parent matrix (`systems/animation/pose.md#parent-matrix`).
    let flags = models[idx].blocks[bone_idx].flags | def.flags;
    let is_root = def.parent == 0xffff;
    let lookup = def.parent as usize;
    // `src` is what `[ebp+8]` points at: the parent matrix, or the working copy once the `& 7`
    // legs have edited it.
    let mut src: [f32; 16] = if is_root {
        models[idx].world
    } else {
        models[idx].palette[lookup]
    };

    if !is_root && flags & 7 != 0 {
        // The 16 dwords go to the working matrix.
        *working = src;

        // The pivot in parent space, from the matrix before the legs edit it
        // (`systems/animation/pose.md#pivot-in-parent-space`): pivot[j] = f32(((M0j·b0 + M1j·b1) +
        // M2j·b2) + M3j).
        let b = def.basis;
        let mut pivot = [0f32; 3];
        for (j, pv) in pivot.iter_mut().enumerate() {
            *pv = ((f64::from(working[j]) * f64::from(b[0])
                + f64::from(working[4 + j]) * f64::from(b[1]))
                + f64::from(working[8 + j]) * f64::from(b[2])
                + f64::from(working[12 + j])) as f32;
        }

        // The ignore-parent legs (`systems/animation/pose.md#ignore-parent-legs`).
        match flags & 6 {
            2 => {
                // Each working row is normalized.
                for r in 0..3 {
                    normalize_row_eps(&mut working[4 * r..4 * r + 3]);
                }
            }
            4 => {
                // Each row is the world row times sqrt(local² / world²): the world direction at
                // the local length. The squared lengths are summed in the groupings below.
                let f = f64::from;
                let world = models[idx].world;
                for r in 0..3 {
                    let rr = [world[4 * r], world[4 * r + 1], world[4 * r + 2]];
                    let s_root = match r {
                        // rows 0/1: (r0²+r1²)+r2²; row 2: (r1²+r2²)+r0².
                        0 | 1 => (f(rr[0]) * f(rr[0]) + f(rr[1]) * f(rr[1])) + f(rr[2]) * f(rr[2]),
                        _ => (f(rr[1]) * f(rr[1]) + f(rr[2]) * f(rr[2])) + f(rr[0]) * f(rr[0]),
                    };
                    let w = &mut working[4 * r..4 * r + 3];
                    let ratio = if s_root > RATIO_EPS {
                        let s_local = match r {
                            // row 0: (m0²+m2²)+m1²; rows 1/2: (m2²+m1²)+m0².
                            0 => (f(w[0]) * f(w[0]) + f(w[2]) * f(w[2])) + f(w[1]) * f(w[1]),
                            _ => (f(w[2]) * f(w[2]) + f(w[1]) * f(w[1])) + f(w[0]) * f(w[0]),
                        };
                        (s_local / s_root).sqrt()
                    } else {
                        1.0
                    };
                    for k in 0..3 {
                        w[k] = (ratio * f(rr[k])) as f32;
                    }
                }
            }
            6 => {
                // The rows are the world matrix rows (dword moves).
                let world = models[idx].world;
                for r in 0..3 {
                    for k in 0..3 {
                        working[4 * r + k] = world[4 * r + k];
                    }
                }
            }
            _ => {} // `& 6 == 0`: the rows are unchanged.
        }

        // The translation row (`systems/animation/pose.md#pivot-in-parent-space`).
        if flags & 1 != 0 {
            // The world translation `+0x12c` to `+0x134`, which is row 3 of the matrix at
            // `+0xfc` (dword moves).
            let w = &models[idx].world;
            working[12] = w[12];
            working[13] = w[13];
            working[14] = w[14];
        } else {
            // translation[j] = f32(pivot[j] - ((M1j·b1 + M0j·b0) + M2j·b2)) with the rows after
            // the legs.
            for j in 0..3 {
                let dot = (f64::from(working[4 + j]) * f64::from(b[1])
                    + f64::from(working[j]) * f64::from(b[0]))
                    + f64::from(working[8 + j]) * f64::from(b[2]);
                working[12 + j] = (f64::from(pivot[j]) - dot) as f32;
            }
        }
        src = *working;
    }

    // The `& 0x280` route (`systems/animation/pose.md#local-matrix-route`).
    let trs_path = flags & 0x280 != 0;
    if trs_path {
        // The rotation leg (`systems/animation/pose.md#rotation-leg`).
        let rot_count = def.rot_track.meta.key_count;
        if rot_count == 0 {
            *rot = IDENTITY;
        } else {
            if rot_count > first_update_gate {
                let (gseq_times, times) = bone_times(models, idx, bone_idx);
                let qt = QuatTrack {
                    interp: def.rot_track.meta.interp,
                    gseq: def.rot_track.meta.gseq,
                    range_count: def.rot_track.meta.range_count,
                    ranges: &def.rot_track.meta.ranges,
                    key_count: def.rot_track.meta.key_count,
                    timestamps: &def.rot_track.meta.timestamps,
                    quats: &def.rot_track.payload,
                };
                let m = &mut models[idx];
                m2_track_sample_quat(&gseq_times, &times, &qt, &mut m.blocks[bone_idx].rot_state);
            }
            // The matrix is built whether or not the sample was taken; it uses the stored
            // quaternion then.
            let q = models[idx].blocks[bone_idx].rot_state.value;
            m2_quat_to_mat4(rot, &q);
        }

        // The scale leg (`systems/animation/pose.md#scale-and-override-legs`).
        let sc_count = def.scale_track.meta.key_count;
        if sc_count != 0 {
            if sc_count > first_update_gate {
                let (gseq_times, times) = bone_times(models, idx, bone_idx);
                let m = &mut models[idx];
                let mut st = m.blocks[bone_idx].scale_state;
                sample_vec3_inline(&gseq_times, &times, &def.scale_track, &mut st);
                m.blocks[bone_idx].scale_state = st;
            }
            let sv = models[idx].blocks[bone_idx].scale_state.value;
            // `0x7bdca0`: rows 0 to 2 times the vector, the fourth column unchanged.
            scale_matrix_rows(rot, sv);
        }

        // The override matrix, with flag `0x80`
        // (`systems/animation/pose.md#scale-and-override-legs`).
        if flags & 0x80 != 0 {
            if let Some(pm) = models[idx].blocks[bone_idx].premult {
                let b_in = *rot;
                matmul4x4_sse2(rot, &b_in, &pm);
            }
        }

        // The translation leg's base (`systems/animation/pose.md#translation-leg`): the pivot plus
        // the sampled vector. The z sum stays on the x87 stack and feeds row 3's z unnarrowed; x
        // and y read their stored floats.
        let mut basis2 = def.basis;
        let mut basis2_z64 = f64::from(def.basis[2]);
        let v_count = def.vec_track.meta.key_count;
        if v_count != 0 {
            if v_count > first_update_gate {
                let (gseq_times, times) = bone_times(models, idx, bone_idx);
                let m = &mut models[idx];
                let mut st = m.blocks[bone_idx].vec_state;
                sample_vec3_inline(&gseq_times, &times, &def.vec_track, &mut st);
                m.blocks[bone_idx].vec_state = st;
            }
            let v = models[idx].blocks[bone_idx].vec_state.value;
            for i in 0..3 {
                let sum = f64::from(basis2[i]) + f64::from(v[i]);
                if i == 2 {
                    basis2_z64 = sum;
                }
                basis2[i] = sum as f32;
            }
        }

        // Row 3 of the local matrix: b[j] - ((L2j·p2 + L1j·p1) + L0j·p0).
        let b = def.basis;
        for j in 0..3 {
            let dot = (f64::from(rot[8 + j]) * f64::from(b[2])
                + f64::from(rot[4 + j]) * f64::from(b[1]))
                + f64::from(rot[j]) * f64::from(b[0]);
            let minuend = if j == 2 {
                basis2_z64
            } else {
                f64::from(basis2[j])
            };
            rot[12 + j] = (minuend - dot) as f32;
        }

        // The palette entry is the local matrix times the parent matrix (`0x74a7c0`).
        let mut out = [0f32; 16];
        matmul4x4_sse2(&mut out, rot, &src);
        models[idx].palette[bone_idx] = out;
    } else {
        // Flags `& 0x280` clear: the entry is a copy of the parent matrix.
        models[idx].palette[bone_idx] = src;
    }

    // The billboard stage (`systems/animation/billboards.md`).
    let sel_bits = flags & 0x78;
    if sel_bits == 0 {
        return; // `0x7151ff je` to the loop tail.
    }
    let f = f64::from;
    let m_pal = models[idx].palette[bone_idx];
    let basis = def.basis;

    // The row lengths, narrowed to floats, and the pivot recomputed from the entry
    // (`systems/animation/billboards.md#row-lengths-and-pivot`).
    let mags: [f32; 3] = core::array::from_fn(|k| {
        ((f(m_pal[4 * k]) * f(m_pal[4 * k]) + f(m_pal[4 * k + 1]) * f(m_pal[4 * k + 1]))
            + f(m_pal[4 * k + 2]) * f(m_pal[4 * k + 2]))
        .sqrt() as f32
    });
    let pivot: [f32; 3] = [
        (((f(m_pal[8]) * f(basis[2]) + f(m_pal[4]) * f(basis[1])) + f(m_pal[0]) * f(basis[0]))
            + f(m_pal[12])) as f32,
        (((f(m_pal[1]) * f(basis[0]) + f(m_pal[9]) * f(basis[2])) + f(m_pal[5]) * f(basis[1]))
            + f(m_pal[13])) as f32,
        (((f(m_pal[2]) * f(basis[0]) + f(m_pal[10]) * f(basis[2])) + f(m_pal[6]) * f(basis[1]))
            + f(m_pal[14])) as f32,
    ];

    // The case index is `(flags & 0x78) - 8`: above 0x38 is the default, and the 57-byte map
    // takes 0 to case 0, 8 to case 1, 0x18 to case 2, 0x38 to case 3 and the rest to the default
    // (`systems/animation/billboards.md#billboard-cases`).
    let sel = sel_bits.wrapping_sub(8);
    let arm = if sel > 0x38 {
        4
    } else {
        match sel {
            0 => 0,
            8 => 1,
            0x18 => 2,
            0x38 => 3,
            _ => 4,
        }
    };

    let mut p = models[idx].palette[bone_idx];
    match arm {
        0 => {
            if trs_path {
                // Row k is the local matrix row (y, z, -x), normalized.
                for k in 0..3 {
                    p[4 * k] = rot[4 * k + 1];
                    p[4 * k + 1] = rot[4 * k + 2];
                    p[4 * k + 2] = -rot[4 * k];
                    normalize_row_eps(&mut p[4 * k..4 * k + 3]);
                }
            } else {
                // Without the local matrix, the fixed rows (0, 0, -1), (1, 0, 0), (0, 1, 0).
                p[0] = 0.0;
                p[1] = 0.0;
                p[2] = -1.0;
                p[4] = 1.0;
                p[5] = 0.0;
                p[6] = 0.0;
                p[8] = 0.0;
                p[9] = 1.0;
                p[10] = 0.0;
            }
        }
        1 => {
            // Row 0 is normalized; row 1 is (m1, -m0, 0) normalized; row 2 is row 0 × row 1.
            normalize_row_eps(&mut p[0..3]);
            p[4] = p[1];
            p[5] = -p[0];
            p[6] = 0.0;
            normalize_row_eps(&mut p[4..7]);
            cross_into(&mut p, 8, 0, 4);
        }
        2 => {
            // Row 1 is normalized; row 0 is (-m5, m4, 0) normalized, negating the first
            // component (`0x715674`: `fld m[5]; fchs`, then `fld m[4]`), the reverse of cases 1
            // and 3; row 2 is row 0 × row 1.
            normalize_row_eps(&mut p[4..7]);
            p[0] = -p[5];
            p[1] = p[4];
            p[2] = 0.0;
            normalize_row_eps(&mut p[0..3]);
            cross_into(&mut p, 8, 0, 4);
        }
        3 => {
            // Row 2 is normalized; row 1 is (m9, -m8, 0) normalized; row 0 is row 1 × row 2. The
            // two squared lengths are summed inline as (y² + z²) + x² (`0x71574b`, `0x7157c7`).
            normalize_row_eps_arm3(&mut p[8..11]);
            p[4] = p[9];
            p[5] = -p[8];
            p[6] = 0.0;
            normalize_row_eps_arm3(&mut p[4..7]);
            cross_into(&mut p, 0, 4, 8);
        }
        _ => {} // The default case goes to the tail.
    }

    // The tail (`systems/animation/billboards.md#row-scale-and-translation`): column 3 is zero and
    // `m15` is 1.0, each row is scaled by its stored length, and the translation sums use the
    // 53-bit products of m0, m1, m4 and m8 and the stored floats of m2, m5, m6, m9 and m10.
    p[3] = 0.0;
    p[7] = 0.0;
    p[11] = 0.0;
    p[15] = 1.0;
    let mut prod64 = [[0f64; 3]; 3]; // the f64 scale products, [row][col]
    for k in 0..3 {
        for i in 0..3 {
            let v = f(mags[k]) * f(p[4 * k + i]);
            prod64[k][i] = v;
            p[4 * k + i] = v as f32;
        }
    }
    let s_m2 = f(prod64[0][2] as f32);
    let s_m5 = f(prod64[1][1] as f32);
    let s_m6 = f(prod64[1][2] as f32);
    let s_m9 = f(prod64[2][1] as f32);
    let s_m10 = f(prod64[2][2] as f32);
    let (b0, b1, b2) = (f(basis[0]), f(basis[1]), f(basis[2]));
    let col0 = (prod64[1][0] * b1 + prod64[2][0] * b2) + prod64[0][0] * b0;
    let col1 = (s_m5 * b1 + s_m9 * b2) + prod64[0][1] * b0;
    let col2 = (s_m6 * b1 + s_m10 * b2) + s_m2 * b0;
    p[12] = (f(pivot[0]) - col0) as f32;
    p[13] = (f(pivot[1]) - col1) as f32;
    p[14] = (f(pivot[2]) - col2) as f32;

    models[idx].palette[bone_idx] = p;
}
