//! The particle loop and the attachment walk of `CM2Model::Animate` (`0x717d6a` to `0x71876f`):
//! each particle record's enabled byte, its visibility flags and its scalars
//! (`systems/animation/elements.md#particle-states`), then each attachment's byte and the
//! animation of the attached children (`systems/animation/kernel.md#attached-children`). The
//! inline blocks at `0x717dfb` and `0x717f26` give the results of the samplers `0x71ae90` and
//! `0x71af20`, which the transcription calls. A record's clocks are those of the bone it names
//! (the word at `+0x14` of a particle record, at `+4` of an attachment).

use super::{Model, TrackMeta};

/// The signature of `CM2Model::Animate`, which the child walk calls on each attached child.
pub(super) type AnimateFn = fn(&mut [Model], usize, &[f32; 16], &[f32; 3], &[f32; 3], f32);
use crate::sample_scalar::{
    m2_track_sample_byte, m2_track_sample_float, ByteSampleState, FloatSampleState,
};
use crate::track::TrackTimes;

/// A byte track (stride-1 keys).
#[derive(Clone, Default)]
pub struct ByteTrackDef {
    pub meta: TrackMeta,
    pub payload: Vec<u8>,
}

/// A float track (stride-4 keys).
#[derive(Clone, Default)]
pub struct ScalarTrackDef {
    pub meta: TrackMeta,
    pub payload: Vec<f32>,
}

/// An attachment (header array `+0x108`, stride 0x30): the bone word at `+4`, the offset
/// `(x, y, z)` at `+8` to `+0x10` that the child walk adds to the bone matrix, and the byte track
/// at `+0x14`, whose key count at `+0x20` gates the walk.
#[derive(Clone, Default)]
pub struct AttachmentDef {
    pub bone: u16,
    pub local: [f32; 3],
    pub byte_track: ByteTrackDef,
}

/// A particle record (header array `+0x140`, stride 0x1f8): the bone word at `+0x14`, ten
/// scalar tracks from `+0x34` in steps of 0x1c (six inline blocks, then four sampled by
/// `0x71af20`), and the enabled byte track at `+0x1dc`.
#[derive(Clone, Default)]
pub struct ParticleDef {
    pub bone: u16,
    pub scalars: [ScalarTrackDef; 6],
    pub named: [ScalarTrackDef; 4],
    pub byte_track: ByteTrackDef,
}

/// A particle state (instance `+0x3d0`, stride 0x16c): the six inline scalar states from `+0x00`
/// (0x20 apart), the four others from `+0xc0`, the byte state at `+0x140`, and the flags at
/// `+0x160` and `+0x164`.
#[derive(Clone, Default)]
pub struct ParticleRec {
    pub scalars: [FloatSampleState; 6],
    pub named: [FloatSampleState; 4],
    pub byte: ByteSampleState,
    pub enabled_blend: u32,
    pub visible: u32,
}

/// The `TrackTimes` of the bone a record names.
fn ref_times(m: &Model, bone: u16) -> TrackTimes {
    let blk = &m.blocks[bone as usize];
    TrackTimes {
        time: blk.time,
        gtime: blk.gtime,
        blend_time: blk.blend_time,
        blend_gtime: blk.blend_gtime,
        lambda: blk.lambda,
    }
}

/// The particle loop (`0x717d6a` to `0x7185e1`). The caller has cleared `+0x3d8`
/// (`0x717d6f`, before the count test). Each record samples its enabled byte, sets
/// `+0x160 = enabled && +0x50` and `+0x164 = +0x160 || HasLiveParticles(emitter)` (the OR of the
/// latter into `+0x3d8`), and samples its ten scalars unless the enabled byte is 0 and the gate
/// `+0x8c` is nonzero.
pub(super) fn particle_loop(models: &mut [Model], idx: usize) {
    let n = models[idx].header.particles.len();
    for i in 0..n {
        let def = models[idx].header.particles[i].clone();
        let times = ref_times(&models[idx], def.bone);
        let (gseq_times, first_update_gate, visible, pred) = {
            let m = &models[idx];
            (
                m.gseq_times.clone(),
                m.first_update_gate,
                m.visible,
                m.has_live_particles,
            )
        };

        // The enabled byte (`0x717dfb`), when its key count exceeds the gate.
        if def.byte_track.meta.key_count > first_update_gate {
            let m = &mut models[idx];
            let mut st = m.particle_recs[i].byte;
            m2_track_sample_byte(
                &gseq_times,
                &times,
                &def.byte_track.meta.view(),
                &def.byte_track.payload,
                &mut st,
            );
            m.particle_recs[i].byte = st;
        }

        // The flags.
        let enable = models[idx].particle_recs[i].byte.value;
        let a = u32::from(enable != 0 && visible != 0);
        let visible = if a != 0 { 1 } else { u32::from(pred != 0) };
        {
            let m = &mut models[idx];
            m.particle_recs[i].enabled_blend = a;
            m.particle_recs[i].visible = visible;
            m.any_visible |= visible;
        }

        // A disabled record is skipped once the gate is 1 (`0x717ee7`).
        if enable == 0 && first_update_gate != 0 {
            continue;
        }
        for (k, tr) in def.scalars.iter().enumerate() {
            if tr.meta.key_count > first_update_gate {
                let m = &mut models[idx];
                let mut st = m.particle_recs[i].scalars[k];
                m2_track_sample_float(&gseq_times, &times, &tr.meta.view(), &tr.payload, &mut st);
                m.particle_recs[i].scalars[k] = st;
            }
        }
        for (k, tr) in def.named.iter().enumerate() {
            if tr.meta.key_count > first_update_gate {
                let m = &mut models[idx];
                let mut st = m.particle_recs[i].named[k];
                m2_track_sample_float(&gseq_times, &times, &tr.meta.view(), &tr.payload, &mut st);
                m.particle_recs[i].named[k] = st;
            }
        }
    }
}

/// The attachment walk (`0x7185e1` to `0x71876f`). Without the byte array `+0x1c8` it ends. Each
/// attachment samples its byte track into that array; then each child with an attachment index
/// `+0x1d4` other than 0xffff and a nonzero byte at that index animates itself on a copy of the
/// attachment bone's matrix, with the offset added to the translation.
pub(super) fn attachment_walk(models: &mut [Model], idx: usize, animate: AnimateFn) {
    if models[idx].attach_payload.is_none() {
        return;
    }

    // The attachment bytes (header `+0x104`, `+0x108`; stride 0x30; array `+0x1c8`, stride 0x20).
    let n_elems = models[idx].header.attachments.len();
    for j in 0..n_elems {
        let def = models[idx].header.attachments[j].clone();
        if def.byte_track.meta.key_count > models[idx].first_update_gate {
            let times = ref_times(&models[idx], def.bone);
            let gseq_times = models[idx].gseq_times.clone();
            let m = &mut models[idx];
            let pl = m.attach_payload.as_mut().expect("gated above");
            m2_track_sample_byte(
                &gseq_times,
                &times,
                &def.byte_track.meta.view(),
                &def.byte_track.payload,
                &mut pl[j],
            );
        }
    }

    // The children, by `+0x1dc` and each `+0x1e4`.
    let (diffuse, emissive, alpha) = {
        let m = &models[idx];
        (m.diffuse, m.emissive, m.alpha)
    };
    let mut child = models[idx].first_child;
    while let Some(ch) = child {
        let next = models[ch].next_sibling;
        let lookup = models[ch].attachment_index;
        if lookup != 0xffff {
            let cidx = lookup as usize;
            let flag = models[idx].attach_payload.as_ref().expect("gated above")[cidx].value;
            if flag != 0 {
                let def = models[idx].header.attachments[cidx].clone();
                // The attachment bone's matrix with the offset added to the translation:
                // t[c] = f32(((M2c·vz + M1c·vy) + M0c·vx) + t[c]), the 3×3 unchanged.
                let mut matrix = models[idx].palette[def.bone as usize];
                let f = f64::from;
                let v = def.local;
                for c in 0..3 {
                    matrix[12 + c] = (((f(matrix[8 + c]) * f(v[2]) + f(matrix[4 + c]) * f(v[1]))
                        + f(matrix[c]) * f(v[0]))
                        + f(matrix[12 + c])) as f32;
                }
                animate(models, ch, &matrix, &diffuse, &emissive, alpha);
            }
        }
        child = next;
    }
}
