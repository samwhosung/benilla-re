//! `CM2Model::Animate` (`0x714260`), the 17.7 KB pass that poses a model instance
//! (`systems/animation/kernel.md`): the gates, the parent-driven state, the world matrix, the pause
//! slide, the bone loop (`clock`, `pose`, `billboards`), the element loops (`elements`), the
//! particle loop, and the attachment walk that animates each attached child.
//!
//! The x87 runs at 53 bits, so a sum is an `f64` chain with `as f32` at each `fstp dword`, and the
//! association and the floats the code stores differ from site to site. `clock.rs` holds the
//! bone clocks and the blend weight, `bone.rs` the bone loop body, `elements.rs` the element
//! loops, `attach.rs` the particle loop and the attachment walk; this file holds the model's
//! records and the pass.

mod attach;
mod bone;
mod clock;
mod elements;

pub use attach::{AttachmentDef, ByteTrackDef, ParticleDef, ParticleRec, ScalarTrackDef};
pub use elements::{
    CameraElemDef, CameraElemRec, ColorElemDef, ColorElemRec, CubicScalarTrackDef,
    CubicVec3TrackDef, I16TrackDef, LightElemDef, LightElemRec, RibbonElemDef, RibbonElemRec,
    TexXformDef, TexXformRec, WordState, WordTrackDef,
};

use crate::mat::m2_matmul4x4;
use crate::sample_quat::QuatSampleState;
use crate::sample_scalar::ByteSampleState;
use crate::track::{M2Track, TrackSearchResult};

/// `0xffffffff` — the "no track" sentinel in the per-bone clock slots.
pub(crate) const NONE32: u32 = 0xffff_ffff;

/// A sequence record (header `+0x20`, stride 0x44): `+4` start, `+8` end, `+0x10` flags
/// (bit 0 selects the clamp leg over the loop leg, `systems/animation/clock.md#clock-legs`).
#[derive(Clone, Copy, Default)]
pub struct SeqRecord {
    pub start: u32,
    pub end: u32,
    pub flags: u32,
}

/// An owned track definition: the header every track carries (`track.rs`, `M2Track`).
#[derive(Clone, Default)]
pub struct TrackMeta {
    pub interp: u16,
    pub gseq: u16,
    pub range_count: u32,
    pub ranges: Vec<[u32; 2]>,
    pub key_count: u32,
    pub timestamps: Vec<u32>,
}

impl TrackMeta {
    pub(crate) fn view(&self) -> M2Track<'_> {
        M2Track {
            interp: self.interp,
            gseq: self.gseq,
            range_count: self.range_count,
            ranges: &self.ranges,
            key_count: self.key_count,
            timestamps: &self.timestamps,
        }
    }
}

/// A vec3-payload track (stride-12 keys).
#[derive(Clone, Default)]
pub struct Vec3TrackDef {
    pub meta: TrackMeta,
    pub payload: Vec<[f32; 3]>,
}

/// A quaternion-payload track (stride-16 keys).
#[derive(Clone, Default)]
pub struct QuatTrackDef {
    pub meta: TrackMeta,
    pub payload: Vec<[f32; 4]>,
}

/// A bone record (header `+0x38`, stride 0x6c): `+4` flags, `+8` the parent word, the translation
/// track at `+0xc`, the rotation track at `+0x28`, the scale track at `+0x44`, and the pivot at
/// `+0x60` (`systems/animation/pose.md`).
#[derive(Clone, Default)]
pub struct BoneDef {
    pub flags: u32,
    pub parent: u16,
    pub vec_track: Vec3TrackDef,
    pub rot_track: QuatTrackDef,
    pub scale_track: Vec3TrackDef,
    pub basis: [f32; 3],
}

/// The state of a vector track (`systems/animation/tracks.md#sampling-block`): the search result at
/// `+0`, the value at `+0xc`, the blend search result at `+0x18` and the blend value at `+0x24`.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Vec3State {
    pub primary: TrackSearchResult,
    pub value: [f32; 3],
    pub blend: TrackSearchResult,
    pub blend_value: [f32; 3],
}

/// A bone state (instance `+0x90`, stride 0x118, `systems/animation/clock.md#bone-state`). The
/// track states are at `+0x00` (translation), `+0x30` (rotation, the quaternion at `+0x3c`) and
/// `+0x68` (scale, the vector at `+0x74`).
#[derive(Clone, Default)]
pub struct BoneState {
    pub vec_state: Vec3State,       // +0x00
    pub rot_state: QuatSampleState, // +0x30
    pub scale_state: Vec3State,     // +0x68
    pub time: u32,                  // +0x98
    pub gtime: u32,                 // +0x9c (the range slot)
    pub stamp: u32,                 // +0xa0 (the bone stamp)
    pub track_idx: u32,             // +0xa4 (the record index; NONE32: none)
    pub cur_lo: u32,                // +0xa8 (the start cursor)
    pub cur_hi: u32,                // +0xac (the end cursor)
    pub rate: f32,                  // +0xb0 (the speed)
    pub bias: i32,                  // +0xb8 (the offset)
    pub blend_time: u32,            // +0xc4
    pub blend_gtime: u32,           // +0xc8
    pub blend_track_idx: u32,       // +0xd0 (the blend record index)
    pub blend_lo: u32,              // +0xd4
    pub blend_hi: u32,              // +0xd8
    pub blend_rate: f32,            // +0xdc
    pub blend_bias: i32,            // +0xe4
    pub premult: Option<[f32; 16]>, // +0xf0 (the override matrix; None: null)
    pub flags: u32,                 // +0xf4 (the flags)
    pub trans_end: u32,             // +0x100 (the fade end)
    pub inv_duration: f32,          // +0x104 (the fade rate)
    pub blend_amp: f32,             // +0x108 (the fade scale)
    pub lambda: f32,                // +0x10c (the blend weight)
}

/// The arrays of the shared model's header that the pass walks
/// (`systems/animation/kernel.md#arrays-the-pass-walks`). An empty array takes the pass's count-0
/// skip.
#[derive(Clone, Default)]
pub struct Md20 {
    /// `+0x14/+0x18`: the global-sequence duration table.
    pub gseq: Vec<u32>,
    /// `+0x20`: the sequence records (stride 0x44).
    pub seqs: Vec<SeqRecord>,
    /// `+0x34`: the bone count, which the pass trusts over the array.
    pub bone_count: u32,
    /// `+0x38`: the bone definitions (stride 0x6c).
    pub bone_defs: Vec<BoneDef>,
    /// `+0x54`, `+0x58`: colours (stride 0x38).
    pub color_elems: Vec<ColorElemDef>,
    /// `+0x64`, `+0x68`: transparency (stride 0x1c, one track at `+0`).
    pub fix16_elems: Vec<I16TrackDef>,
    /// `+0x6c`, `+0x70`: texture frames (stride 0x1c).
    pub word_elems: Vec<WordTrackDef>,
    /// `+0x74`, `+0x78`: texture transforms (stride 0x54).
    pub tex_xforms: Vec<TexXformDef>,
    /// `+0x11c`, `+0x120`: lights (stride 0xd4).
    pub light_elems: Vec<LightElemDef>,
    /// `+0x124`, `+0x128`: cameras (stride 0x7c).
    pub camera_elems: Vec<CameraElemDef>,
    /// `+0x134`, `+0x138`: ribbons (stride 0xdc).
    pub ribbon_elems: Vec<RibbonElemDef>,
    /// `+0x104`, `+0x108`: attachments (stride 0x30).
    pub attachments: Vec<AttachmentDef>,
    /// `+0x13c`, `+0x140`: particles (stride 0x1f8).
    pub particles: Vec<ParticleDef>,
}

/// The fields of a `CM2Model` instance that the pass reads and writes. Parent, children and
/// siblings are indices, so the attachment walk can animate a child.
#[derive(Clone, Default)]
pub struct Model {
    /// `+0x4`: with bit 0 clear, a child takes its parent's `depth_key`.
    pub flags4: u32,
    /// `+0x10`: the instance is loaded (the first gate).
    pub loaded: u32,
    /// `+0x40`: the frame the instance was last animated in (the second gate); the normal exit
    /// stores the scene's frame here.
    pub frame: u32,
    /// `+0x4c`: the pause stamp (`systems/animation/kernel.md#pause-slide`).
    pub pause_stamp: u32,
    /// `+0x50`: the effective visible flag.
    pub visible: u32,
    /// `[[+0x2c]+0xc]`: the scene clock.
    pub scene_clock: u32,
    /// `[[+0x2c]+0x10]`: the scene's frame.
    pub scene_frame: u32,
    /// `+0x64`: the global sequence clocks, one per header duration.
    pub gseq_times: Vec<u32>,
    /// `+0x68`: the clock when the instance was initialized.
    pub gseq_base: u32,
    /// `+0x84`: the squared length of the world translation, the scene's depth key.
    pub depth_key: f32,
    /// `+0x8c`: the first-update gate; a track is sampled when its key count exceeds it.
    pub first_update_gate: u32,
    /// `+0x90`: the bone states.
    pub blocks: Vec<BoneState>,
    /// `+0x94`: the bone matrices (stride 0x40).
    pub palette: Vec<[f32; 16]>,
    /// `+0xbc`: the base matrix.
    pub base: [f32; 16],
    /// `+0xfc`: the world matrix. Row 3 (`+0x12c` to `+0x134`) is the world translation that the
    /// `depth_key` sum and the `& 1` translation read; rows 0 to 2 are the rows the ignore-parent
    /// legs copy.
    pub world: [f32; 16],
    /// `+0x17c`: the transport matrix pointer, copied from the parent.
    pub transport: u32,
    /// `+0x180`: multiplies the alpha argument.
    pub alpha_scale: f32,
    /// `+0x184`: multiplies the diffuse argument.
    pub diffuse_scale: [f32; 3],
    /// `+0x190`: added to the emissive argument.
    pub emissive_bias: [f32; 3],
    /// `+0x19c`: the instance's alpha.
    pub alpha: f32,
    /// `+0x1a0`: the instance's diffuse colour.
    pub diffuse: [f32; 3],
    /// `+0x1ac`: the instance's emissive colour.
    pub emissive: [f32; 3],
    /// `+0x1c8`: the attachment byte states (stride 0x20); `None` is a null pointer.
    pub attach_payload: Option<Vec<ByteSampleState>>,
    /// `+0x1cc`: the parent instance.
    pub parent: Option<usize>,
    /// `+0x1d4`: the attachment index, read when this instance is a child.
    pub attachment_index: u32,
    /// `+0x1d8`: the visible flag of an attached instance.
    pub child_visible: u32,
    /// `+0x1dc`: the first child.
    pub first_child: Option<usize>,
    /// `+0x1e4`: the next sibling.
    pub next_sibling: Option<usize>,
    /// `+0xa0`: colour states (stride 0x50).
    pub color_recs: Vec<ColorElemRec>,
    /// `+0xa8`: transparency states (stride 0x20).
    pub fix16_recs: Vec<crate::sample_scalar::FloatSampleState>,
    /// `+0xac`: texture frame states (stride 0x20).
    pub word_recs: Vec<WordState>,
    /// `+0xb0`: texture transform states (stride 0x98).
    pub tex_states: Vec<TexXformRec>,
    /// `+0xb4`: the texture transform matrices (stride 0x40).
    pub tex_matrices: Vec<[f32; 16]>,
    /// `+0x200`: light states (stride 0x170).
    pub light_recs: Vec<LightElemRec>,
    /// `+0x3c4`: camera states (stride 0x84).
    pub camera_recs: Vec<CameraElemRec>,
    /// `+0x3c8`: ribbon states (stride 0xd0).
    pub ribbon_recs: Vec<RibbonElemRec>,
    /// `+0x3d8`: the OR of the particle records' `+0x164` flags.
    pub any_visible: u32,
    /// `+0x3d0`: particle states (stride 0x16c).
    pub particle_recs: Vec<ParticleRec>,
    /// `+0x3d4`: the emitter of each particle record, the argument of `HasLiveParticles`
    /// (`0x7b5f60`).
    pub particle_emitters: Vec<u32>,
    /// What `CParticleEmitter2::HasLiveParticles` (`0x7b5f60`) returns for every record.
    pub has_live_particles: u32,
    pub header: Md20,
}

pub(crate) const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// `0x714260`, `CM2Model::Animate`: a `thiscall` on `models[idx]` with the four stack arguments
/// `parent_mtx`, the colours `diffuse_in` and `emissive_in`, and `alpha_in`
/// (`systems/animation/kernel.md#animate-gates`, `systems/models/animation.md#instance-colours`).
/// It calls itself on each attached child.
pub fn m2_animate(
    models: &mut [Model],
    idx: usize,
    parent_mtx: &[f32; 16],
    diffuse_in: &[f32; 3],
    emissive_in: &[f32; 3],
    alpha_in: f32,
) {
    // The gates (`0x714260` to `0x714293`); neither exit stores the frame.
    if models[idx].loaded == 0 {
        return;
    }
    if models[idx].frame == models[idx].scene_frame {
        return;
    }

    // The parent-driven state (`0x714293` to `0x71434c`,
    // `systems/animation/kernel.md#parent-driven-state`), and the instance's colours and alpha
    // (`systems/models/animation.md#instance-colours`).
    if let Some(p) = models[idx].parent {
        let (p_blend_active, p_f17c) = (models[p].visible, models[p].transport);
        let m = &mut models[idx];
        m.visible = u32::from(p_blend_active != 0 && m.child_visible != 0);
        m.transport = p_f17c;
    }
    {
        let m = &mut models[idx];
        for i in 0..3 {
            m.diffuse[i] = (f64::from(diffuse_in[i]) * f64::from(m.diffuse_scale[i])) as f32;
            m.emissive[i] = (f64::from(emissive_in[i]) + f64::from(m.emissive_bias[i])) as f32;
        }
        m.alpha = (f64::from(alpha_in) * f64::from(m.alpha_scale)) as f32;
    }

    // The global sequence clocks (`0x714352` to `0x714389`): the scene clock less the base, modulo
    // the duration, or 0 for a duration of 0.
    {
        let m = &mut models[idx];
        for i in 0..m.header.gseq.len() {
            let dur = m.header.gseq[i];
            m.gseq_times[i] = if dur != 0 {
                m.scene_clock.wrapping_sub(m.gseq_base) % dur
            } else {
                0
            };
        }
    }

    // The world matrix, `+0x84` and the pause slide (`0x714389` to `0x714530`).
    let wrap_delta;
    {
        // `+0xfc = +0xbc · parent_mtx`.
        let base = models[idx].base;
        let mut w = [0f32; 16];
        m2_matmul4x4(&mut w, &base, parent_mtx);
        models[idx].world = w;

        // `+0x84`.
        let parent_copy = models[idx]
            .parent
            .map(|p| (models[p].flags4, models[p].depth_key));
        let m = &mut models[idx];
        match parent_copy {
            Some((pf, po)) if pf & 1 == 0 => m.depth_key = po,
            _ => {
                // `(a² + b²) + c²` of the new world translation `+0x12c` to `+0x134`, narrowed
                // once.
                let (a, b, c) = (m.world[12], m.world[13], m.world[14]);
                m.depth_key = (f64::from(a) * f64::from(a)
                    + f64::from(b) * f64::from(b)
                    + f64::from(c) * f64::from(c)) as f32;
            }
        }

        // The slide: the stamp moves to the clock only when both are nonzero
        // (`systems/animation/kernel.md#pause-slide`).
        let old = m.pause_stamp;
        let dur = m.scene_clock;
        if old != 0 && dur != 0 {
            wrap_delta = dur.wrapping_sub(old);
            m.pause_stamp = dur;
        } else {
            wrap_delta = 0;
        }
    }

    // The bone loop (`0x714530` to `0x715960`). The working and rotation matrices are set to the
    // identity once, before the loop, and carry from bone to bone on the paths that do not
    // rewrite them.
    let mut working: [f32; 16] = IDENTITY;
    let mut rot: [f32; 16] = IDENTITY;
    let bone_count = models[idx].header.bone_count;
    for bone_idx in 0..bone_count as usize {
        bone::animate_bone(models, idx, bone_idx, &mut working, &mut rot, wrap_delta);
    }

    // The element loops (`0x715966` to `0x717d6a`).
    elements::element_loops(models, idx);

    // The particle loop (`0x717d6a` to `0x7185e1`). `+0x3d8` is cleared before the count test
    // (`0x717d6f`).
    models[idx].any_visible = 0;
    attach::particle_loop(models, idx);

    // The attachment walk (`0x7185e1` to `0x71876f`); without `+0x1c8` the pass still ends at the
    // normal exit, which stores the frame.
    attach::attachment_walk(models, idx, m2_animate);

    // The normal exit (`0x718775`) stores the scene's frame.
    models[idx].frame = models[idx].scene_frame;
}
