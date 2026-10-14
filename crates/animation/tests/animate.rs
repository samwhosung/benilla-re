//! `CM2Model::Animate` (`0x714260`) run whole (`systems/animation/kernel.md`). A model instance
//! and its shared header are laid into emulator memory at the offsets of `systems/animation/`
//! (the Rust `Model` describes the fixture, and one builder feeds both sides), the exe's 17.7 KB
//! pass runs on it, and every region it writes is read back and compared bit for bit with
//! `m2_animate`: the frame, `+0x4c`, `+0x50` and `+0x17c`; `+0x84`; the alpha, diffuse and
//! emissive colours; the global sequence clocks; the world matrix; every bone state (clocks,
//! blend weight, the three track states); the bone matrices; and every element state.
//!
//! The cases cover the gates; the parent-driven state; the pause slide; the bone clocks (clamp,
//! loop, inherited) and the blend weight (inherited, zero, the smoothstep, saturated); the
//! flag dispatch, the ignore-parent legs and the translation, rotation, scale and override legs;
//! the five billboard cases; the colour, transparency, texture frame, texture transform, light,
//! camera and ribbon loops, with the blend legs armed through bone 0 and through the bones the
//! records name; the particle loop; the attachment walk with the children; and both exits.
//!
//! The math slots are set to the routines a processor with no registry override reaches except
//! the product: `[0x876594] = 0x74b6bb` (rotation matrix) and `[0x876588] = 0x74d11a` (blend)
//! hold what the first call stores, but `[0x876504]` is set to `0x74a7c6`, the table's original
//! entry that `0x750930` replaces with `0x7507bb` at the first call
//! (`systems/models/animation.md#dispatched-matrix-math`), so every product here is the x87
//! routine that `m2_matmul4x4` transcribes. `HasLiveParticles` (`0x7b5f60`) is a stub that
//! returns a fixed value, 0 or 1 per emulator.

mod common;

use benilla_re_animation::animate::{
    m2_animate, AttachmentDef, BoneDef, BoneState, ByteTrackDef, Md20, Model, ParticleDef,
    ParticleRec, QuatTrackDef, ScalarTrackDef, SeqRecord, TrackMeta, Vec3State, Vec3TrackDef,
};
use benilla_re_animation::animate::{
    CameraElemDef, CameraElemRec, ColorElemDef, ColorElemRec, CubicScalarTrackDef,
    CubicVec3TrackDef, I16TrackDef, LightElemDef, LightElemRec, RibbonElemDef, RibbonElemRec,
    TexXformDef, TexXformRec, WordState, WordTrackDef,
};
use benilla_re_animation::sample_quat::QuatSampleState;
use benilla_re_animation::sample_scalar::{ByteSampleState, FloatSampleState};
use benilla_re_animation::track::TrackSearchResult;
use benilla_re_difftest::{Emu, Wow};
use common::{assert_f32, next};

const FN_VA: u64 = 0x714260;

// arena sub-offsets (per model; every region disjoint).
const SHARED: u64 = 0x400; // +0xc time, +0x10 limit
const MODELDATA: u64 = 0x500; // +0x130 -> MD20
const MD20: u64 = 0x600;
const GSEQ: u64 = 0x800; // the duration table
const GTIMES: u64 = 0x900; // the +0x64 dest array
const SEQS: u64 = 0xa00; // stride 0x44
const ARGS: u64 = 0xe00; // the parent matrix at +0, the diffuse at +0x40, the emissive at +0x50
const DEFS: u64 = 0x1000; // stride 0x6c
const BLOCKS: u64 = 0x2000; // stride 0x118
const PALETTE: u64 = 0x4000; // stride 0x40
const ATTRECS: u64 = 0x4800; // stride 0x16c (the +0x3d0 dest array)
const PAYLOAD: u64 = 0x4e00; // stride 0x20 (the +0x1c8 payload)
const ATTSEC: u64 = 0x4f80; // dwords (the +0x3d4 emitters)
const BYTEDEFS: u64 = 0x5000; // stride 0x30 (the attachments, header +0x108)
const ATTDEFS: u64 = 0x5200; // stride 0x1f8 (the particle records, header +0x140)
const COLORDEFS: u64 = 0x6000; // colour records s0x38
const COLORRECS: u64 = 0x6200; // colour states s0x50
const FIX16DEFS: u64 = 0x6400; // transparency records s0x1c
const FIX16RECS: u64 = 0x6600; // transparency states s0x20
const WORDDEFS: u64 = 0x6800; // texture frame records s0x1c
const WORDRECS: u64 = 0x6a00; // texture frame states s0x20
const TEXDEFS: u64 = 0x6c00; // texture transform records s0x54
const TEXSTATES: u64 = 0x7000; // texture transform states s0x98
const TEXMATS: u64 = 0x7400; // texture transform matrices s0x40
const LIGHTDEFS: u64 = 0x7800; // light records s0xd4
const LIGHTRECS: u64 = 0x8000; // light states s0x170
const CAMERADEFS: u64 = 0x8800; // camera records s0x7c
const CAMERARECS: u64 = 0x8c00; // camera states s0x84
const RIBBONDEFS: u64 = 0x9000; // ribbon records s0xdc
const RIBBONRECS: u64 = 0x9800; // ribbon states s0xd0
const TRACKDATA: u64 = 0x10000; // bump region for ranges/timestamps/payloads/premults

/// A bump allocator over the per-model track-data region.
struct Bump(u64);
impl Bump {
    fn take(&mut self, bytes: u64) -> u64 {
        let at = self.0;
        self.0 = at + ((bytes + 0xf) & !0xf);
        at
    }
}

fn write_mat(emu: &mut Emu, va: u64, m: &[f32; 16]) {
    for (i, &v) in m.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, v.to_bits());
    }
}

fn read_mat(emu: &Emu, va: u64) -> [f32; 16] {
    core::array::from_fn(|i| emu.read_f32(va + (i as u64) * 4))
}

fn write_vec3(emu: &mut Emu, va: u64, v: &[f32; 3]) {
    for (i, &c) in v.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, c.to_bits());
    }
}

fn read_vec3(emu: &Emu, va: u64) -> [f32; 3] {
    core::array::from_fn(|i| emu.read_f32(va + (i as u64) * 4))
}

/// Serialize one track meta+payload; returns the def-relative writes applied at `def_va`
/// (the embedded M2Track header: +0 interp|gseq, +4 range_count, +8 ranges, +0xc key_count,
/// +0x10 timestamps, +0x18 payload).
#[allow(clippy::too_many_arguments)] // mirrors the embedded M2Track header field-for-field
fn write_track(
    emu: &mut Emu,
    bump: &mut Bump,
    arena: u64,
    def_va: u64,
    meta: &TrackMeta,
    payload_stride: u64,
    payload_len: usize,
    mut write_payload: impl FnMut(&mut Emu, u64),
) {
    emu.write_u32(
        def_va,
        (u32::from(meta.gseq) << 16) | u32::from(meta.interp),
    );
    emu.write_u32(def_va + 4, meta.range_count);
    let ranges = bump.take((meta.ranges.len() as u64) * 8 + 8);
    for (i, r) in meta.ranges.iter().enumerate() {
        emu.write_u32(arena + ranges + (i as u64) * 8, r[0]);
        emu.write_u32(arena + ranges + (i as u64) * 8 + 4, r[1]);
    }
    emu.write_u32(def_va + 8, (arena + ranges) as u32);
    emu.write_u32(def_va + 0xc, meta.key_count);
    let times = bump.take((meta.timestamps.len() as u64) * 4 + 4);
    for (i, &t) in meta.timestamps.iter().enumerate() {
        emu.write_u32(arena + times + (i as u64) * 4, t);
    }
    emu.write_u32(def_va + 0x10, (arena + times) as u32);
    let pl = bump.take((payload_len as u64) * payload_stride + 0x10);
    write_payload(emu, arena + pl);
    emu.write_u32(def_va + 0x18, (arena + pl) as u32);
}

fn write_vec3_track(emu: &mut Emu, bump: &mut Bump, arena: u64, def_va: u64, t: &Vec3TrackDef) {
    let payload = t.payload.clone();
    write_track(
        emu,
        bump,
        arena,
        def_va,
        &t.meta,
        12,
        payload.len(),
        move |emu, base| {
            for (i, v) in payload.iter().enumerate() {
                write_vec3(emu, base + (i as u64) * 12, v);
            }
        },
    );
}

fn write_quat_track(emu: &mut Emu, bump: &mut Bump, arena: u64, def_va: u64, t: &QuatTrackDef) {
    let payload = t.payload.clone();
    write_track(
        emu,
        bump,
        arena,
        def_va,
        &t.meta,
        16,
        payload.len(),
        move |emu, base| {
            for (i, q) in payload.iter().enumerate() {
                for (k, &c) in q.iter().enumerate() {
                    emu.write_u32(base + (i as u64) * 16 + (k as u64) * 4, c.to_bits());
                }
            }
        },
    );
}

fn write_byte_track(emu: &mut Emu, bump: &mut Bump, arena: u64, def_va: u64, t: &ByteTrackDef) {
    let payload = t.payload.clone();
    write_track(
        emu,
        bump,
        arena,
        def_va,
        &t.meta,
        1,
        payload.len(),
        move |emu, base| {
            for (i, &v) in payload.iter().enumerate() {
                emu.write(base + i as u64, &[v]);
            }
        },
    );
}

fn write_scalar_track(emu: &mut Emu, bump: &mut Bump, arena: u64, def_va: u64, t: &ScalarTrackDef) {
    let payload = t.payload.clone();
    write_track(
        emu,
        bump,
        arena,
        def_va,
        &t.meta,
        4,
        payload.len(),
        move |emu, base| {
            for (i, &v) in payload.iter().enumerate() {
                emu.write_u32(base + (i as u64) * 4, v.to_bits());
            }
        },
    );
}

fn write_i16_track(emu: &mut Emu, bump: &mut Bump, arena: u64, def_va: u64, t: &I16TrackDef) {
    let payload = t.payload.clone();
    write_track(
        emu,
        bump,
        arena,
        def_va,
        &t.meta,
        2,
        payload.len(),
        move |emu, base| {
            for (i, &v) in payload.iter().enumerate() {
                emu.write(base + (i as u64) * 2, &(v as u16).to_le_bytes());
            }
        },
    );
}

fn write_word_track(emu: &mut Emu, bump: &mut Bump, arena: u64, def_va: u64, t: &WordTrackDef) {
    let payload = t.payload.clone();
    write_track(
        emu,
        bump,
        arena,
        def_va,
        &t.meta,
        2,
        payload.len(),
        move |emu, base| {
            for (i, &v) in payload.iter().enumerate() {
                emu.write(base + (i as u64) * 2, &v.to_le_bytes());
            }
        },
    );
}

fn write_cubic_v3_track(
    emu: &mut Emu,
    bump: &mut Bump,
    arena: u64,
    def_va: u64,
    t: &CubicVec3TrackDef,
) {
    let payload = t.payload.clone();
    write_track(
        emu,
        bump,
        arena,
        def_va,
        &t.meta,
        0x24,
        payload.len(),
        move |emu, base| {
            for (i, key) in payload.iter().enumerate() {
                for (j, tri) in key.iter().enumerate() {
                    write_vec3(emu, base + (i as u64) * 0x24 + (j as u64) * 12, tri);
                }
            }
        },
    );
}

fn write_cubic_sc_track(
    emu: &mut Emu,
    bump: &mut Bump,
    arena: u64,
    def_va: u64,
    t: &CubicScalarTrackDef,
) {
    let payload = t.payload.clone();
    write_track(
        emu,
        bump,
        arena,
        def_va,
        &t.meta,
        0xc,
        payload.len(),
        move |emu, base| {
            for (i, key) in payload.iter().enumerate() {
                write_vec3(emu, base + (i as u64) * 0xc, key);
            }
        },
    );
}

/// Write a search-result triple {k0,k1,t}.
fn write_triple(emu: &mut Emu, va: u64, r: &TrackSearchResult) {
    emu.write_u32(va, r.k0);
    emu.write_u32(va + 4, r.k1);
    emu.write_u32(va + 8, r.t.to_bits());
}

fn read_triple(emu: &Emu, va: u64) -> TrackSearchResult {
    TrackSearchResult {
        k0: emu.read_u32(va),
        k1: emu.read_u32(va + 4),
        t: emu.read_f32(va + 8),
    }
}

fn write_fstate(emu: &mut Emu, va: u64, st: &FloatSampleState) {
    write_triple(emu, va, &st.primary);
    emu.write_u32(va + 0xc, st.value.to_bits());
    write_triple(emu, va + 0x10, &st.secondary);
    emu.write_u32(va + 0x1c, st.secondary_value.to_bits());
}

fn read_fstate(emu: &Emu, va: u64) -> FloatSampleState {
    FloatSampleState {
        primary: read_triple(emu, va),
        value: emu.read_f32(va + 0xc),
        secondary: read_triple(emu, va + 0x10),
        secondary_value: emu.read_f32(va + 0x1c),
    }
}

fn write_bstate(emu: &mut Emu, va: u64, st: &ByteSampleState) {
    write_triple(emu, va, &st.primary);
    emu.write_u32(va + 0xc, u32::from(st.value));
    write_triple(emu, va + 0x10, &st.secondary);
    emu.write_u32(va + 0x1c, u32::from(st.secondary_value));
}

fn read_bstate(emu: &Emu, va: u64) -> ByteSampleState {
    ByteSampleState {
        primary: read_triple(emu, va),
        value: emu.read_u32(va + 0xc) as u8,
        secondary: read_triple(emu, va + 0x10),
        secondary_value: emu.read_u32(va + 0x1c) as u8,
    }
}

fn write_v3state(emu: &mut Emu, va: u64, st: &Vec3State) {
    write_triple(emu, va, &st.primary);
    write_vec3(emu, va + 0xc, &st.value);
    write_triple(emu, va + 0x18, &st.blend);
    write_vec3(emu, va + 0x24, &st.blend_value);
}

fn read_v3state(emu: &Emu, va: u64) -> Vec3State {
    Vec3State {
        primary: read_triple(emu, va),
        value: read_vec3(emu, va + 0xc),
        blend: read_triple(emu, va + 0x18),
        blend_value: read_vec3(emu, va + 0x24),
    }
}

fn write_wstate(emu: &mut Emu, va: u64, st: &WordState) {
    write_triple(emu, va, &st.primary);
    emu.write_u32(va + 0xc, u32::from(st.value));
    write_triple(emu, va + 0x10, &st.secondary);
    emu.write_u32(va + 0x1c, u32::from(st.secondary_value));
}

fn read_wstate(emu: &Emu, va: u64) -> WordState {
    WordState {
        primary: read_triple(emu, va),
        value: emu.read_u32(va + 0xc) as u16,
        secondary: read_triple(emu, va + 0x10),
        secondary_value: emu.read_u32(va + 0x1c) as u16,
    }
}

fn write_qstate(emu: &mut Emu, va: u64, st: &QuatSampleState) {
    write_triple(emu, va, &st.primary);
    for (k, &c) in st.value.iter().enumerate() {
        emu.write_u32(va + 0xc + (k as u64) * 4, c.to_bits());
    }
    write_triple(emu, va + 0x1c, &st.blend);
    for (k, &c) in st.blend_value.iter().enumerate() {
        emu.write_u32(va + 0x28 + (k as u64) * 4, c.to_bits());
    }
}

fn read_qstate(emu: &Emu, va: u64) -> QuatSampleState {
    QuatSampleState {
        primary: read_triple(emu, va),
        value: core::array::from_fn(|k| emu.read_f32(va + 0xc + (k as u64) * 4)),
        blend: read_triple(emu, va + 0x1c),
        blend_value: core::array::from_fn(|k| emu.read_f32(va + 0x28 + (k as u64) * 4)),
    }
}

/// Writes one `Model` into its arena at the offsets of `systems/animation/kernel.md`.
fn write_model(emu: &mut Emu, arenas: &[u64], mi: usize, models: &[Model]) {
    let m = &models[mi];
    let arena = arenas[mi];
    let this = arena;
    emu.write_u32(this + 0x4, m.flags4);
    emu.write_u32(this + 0x10, m.loaded);
    emu.write_u32(this + 0x2c, (arena + SHARED) as u32);
    emu.write_u32(this + 0x30, (arena + MODELDATA) as u32);
    emu.write_u32(this + 0x40, m.frame);
    emu.write_u32(this + 0x4c, m.pause_stamp);
    emu.write_u32(this + 0x50, m.visible);
    emu.write_u32(this + 0x64, (arena + GTIMES) as u32);
    emu.write_u32(this + 0x68, m.gseq_base);
    emu.write_u32(this + 0x84, m.depth_key.to_bits());
    emu.write_u32(this + 0x8c, m.first_update_gate);
    emu.write_u32(this + 0x90, (arena + BLOCKS) as u32);
    emu.write_u32(this + 0x94, (arena + PALETTE) as u32);
    write_mat(emu, this + 0xbc, &m.base);
    write_mat(emu, this + 0xfc, &m.world);
    emu.write_u32(this + 0x17c, m.transport);
    emu.write_u32(this + 0x180, m.alpha_scale.to_bits());
    write_vec3(emu, this + 0x184, &m.diffuse_scale);
    write_vec3(emu, this + 0x190, &m.emissive_bias);
    emu.write_u32(this + 0x19c, m.alpha.to_bits());
    write_vec3(emu, this + 0x1a0, &m.diffuse);
    write_vec3(emu, this + 0x1ac, &m.emissive);
    match &m.attach_payload {
        None => emu.write_u32(this + 0x1c8, 0),
        Some(pl) => {
            emu.write_u32(this + 0x1c8, (arena + PAYLOAD) as u32);
            for (j, st) in pl.iter().enumerate() {
                write_bstate(emu, arena + PAYLOAD + (j as u64) * 0x20, st);
            }
        }
    }
    emu.write_u32(this + 0x1cc, m.parent.map_or(0, |p| arenas[p] as u32));
    emu.write_u32(this + 0x1d4, m.attachment_index);
    emu.write_u32(this + 0x1d8, m.child_visible);
    emu.write_u32(this + 0x1dc, m.first_child.map_or(0, |c| arenas[c] as u32));
    emu.write_u32(this + 0x1e4, m.next_sibling.map_or(0, |c| arenas[c] as u32));
    emu.write_u32(this + 0x3d0, (arena + ATTRECS) as u32);
    emu.write_u32(this + 0x3d4, (arena + ATTSEC) as u32);
    for (i, &v) in m.particle_emitters.iter().enumerate() {
        emu.write_u32(arena + ATTSEC + (i as u64) * 4, v);
    }
    for (i, r) in m.particle_recs.iter().enumerate() {
        let rv = arena + ATTRECS + (i as u64) * 0x16c;
        for (k, st) in r.scalars.iter().enumerate() {
            write_fstate(emu, rv + (k as u64) * 0x20, st);
        }
        for (k, st) in r.named.iter().enumerate() {
            write_fstate(emu, rv + 0xc0 + (k as u64) * 0x20, st);
        }
        write_bstate(emu, rv + 0x140, &r.byte);
        emu.write_u32(rv + 0x160, r.enabled_blend);
        emu.write_u32(rv + 0x164, r.visible);
    }
    emu.write_u32(this + 0x3d8, m.any_visible);

    // shared + modeldata→MD20.
    emu.write_u32(arena + SHARED + 0xc, m.scene_clock);
    emu.write_u32(arena + SHARED + 0x10, m.scene_frame);
    emu.write_u32(arena + MODELDATA + 0x130, (arena + MD20) as u32);

    // MD20: the gseq table, the sequence records, the bone defs; every E/F/G count zero.
    let md = arena + MD20;
    for (cnt_off, ptr_off, n, region) in [
        (0x54u64, 0x58u64, m.header.color_elems.len(), COLORDEFS),
        (0x64, 0x68, m.header.fix16_elems.len(), FIX16DEFS),
        (0x6c, 0x70, m.header.word_elems.len(), WORDDEFS),
        (0x74, 0x78, m.header.tex_xforms.len(), TEXDEFS),
        (0x11c, 0x120, m.header.light_elems.len(), LIGHTDEFS),
        (0x124, 0x128, m.header.camera_elems.len(), CAMERADEFS),
        (0x134, 0x138, m.header.ribbon_elems.len(), RIBBONDEFS),
    ] {
        emu.write_u32(md + cnt_off, n as u32);
        emu.write_u32(md + ptr_off, (arena + region) as u32);
    }
    for (off, region) in [
        (0xa0u64, COLORRECS),
        (0xa8, FIX16RECS),
        (0xac, WORDRECS),
        (0xb0, TEXSTATES),
        (0xb4, TEXMATS),
        (0x200, LIGHTRECS),
        (0x3c4, CAMERARECS),
        (0x3c8, RIBBONRECS),
    ] {
        emu.write_u32(arena + off, (arena + region) as u32);
    }
    emu.write_u32(md + 0x104, m.header.attachments.len() as u32);
    emu.write_u32(md + 0x108, (arena + BYTEDEFS) as u32);
    emu.write_u32(md + 0x13c, m.header.particles.len() as u32);
    emu.write_u32(md + 0x140, (arena + ATTDEFS) as u32);
    emu.write_u32(md + 0x14, m.header.gseq.len() as u32);
    emu.write_u32(md + 0x18, (arena + GSEQ) as u32);
    for (i, &d) in m.header.gseq.iter().enumerate() {
        emu.write_u32(arena + GSEQ + (i as u64) * 4, d);
    }
    for (i, &t) in m.gseq_times.iter().enumerate() {
        emu.write_u32(arena + GTIMES + (i as u64) * 4, t);
    }
    emu.write_u32(md + 0x20, (arena + SEQS) as u32);
    for (i, s) in m.header.seqs.iter().enumerate() {
        let r = arena + SEQS + (i as u64) * 0x44;
        emu.write_u32(r + 4, s.start);
        emu.write_u32(r + 8, s.end);
        emu.write_u32(r + 0x10, s.flags);
    }
    emu.write_u32(md + 0x34, m.header.bone_count);
    emu.write_u32(md + 0x38, (arena + DEFS) as u32);

    let mut bump = Bump(TRACKDATA);
    for (i, d) in m.header.color_elems.iter().enumerate() {
        let dv = arena + COLORDEFS + (i as u64) * 0x38;
        write_vec3_track(emu, &mut bump, arena, dv, &d.vec_track);
        write_i16_track(emu, &mut bump, arena, dv + 0x1c, &d.fix_track);
        let rv = arena + COLORRECS + (i as u64) * 0x50;
        write_v3state(emu, rv, &m.color_recs[i].vec);
        write_fstate(emu, rv + 0x30, &m.color_recs[i].fix);
    }
    for (i, d) in m.header.fix16_elems.iter().enumerate() {
        write_i16_track(
            emu,
            &mut bump,
            arena,
            arena + FIX16DEFS + (i as u64) * 0x1c,
            d,
        );
        write_fstate(emu, arena + FIX16RECS + (i as u64) * 0x20, &m.fix16_recs[i]);
    }
    for (i, d) in m.header.word_elems.iter().enumerate() {
        write_word_track(
            emu,
            &mut bump,
            arena,
            arena + WORDDEFS + (i as u64) * 0x1c,
            d,
        );
        write_wstate(emu, arena + WORDRECS + (i as u64) * 0x20, &m.word_recs[i]);
    }
    for (i, d) in m.header.tex_xforms.iter().enumerate() {
        let dv = arena + TEXDEFS + (i as u64) * 0x54;
        write_vec3_track(emu, &mut bump, arena, dv, &d.trans_track);
        write_quat_track(emu, &mut bump, arena, dv + 0x1c, &d.quat_track);
        write_vec3_track(emu, &mut bump, arena, dv + 0x38, &d.scale_track);
        let rv = arena + TEXSTATES + (i as u64) * 0x98;
        write_v3state(emu, rv, &m.tex_states[i].trans);
        write_qstate(emu, rv + 0x30, &m.tex_states[i].quat);
        write_v3state(emu, rv + 0x68, &m.tex_states[i].scale);
        write_mat(emu, arena + TEXMATS + (i as u64) * 0x40, &m.tex_matrices[i]);
    }
    for (i, d) in m.header.light_elems.iter().enumerate() {
        let dv = arena + LIGHTDEFS + (i as u64) * 0xd4;
        emu.write_u32(dv, u32::from(d.bone) << 16); // word at +2
        write_vec3_track(emu, &mut bump, arena, dv + 0x10, &d.vec1);
        write_scalar_track(emu, &mut bump, arena, dv + 0x2c, &d.s1);
        write_vec3_track(emu, &mut bump, arena, dv + 0x48, &d.vec2);
        write_scalar_track(emu, &mut bump, arena, dv + 0x64, &d.s2);
        write_byte_track(emu, &mut bump, arena, dv + 0xb8, &d.byte_track);
        let rv = arena + LIGHTRECS + (i as u64) * 0x170;
        let r = &m.light_recs[i];
        write_v3state(emu, rv, &r.vec1);
        write_fstate(emu, rv + 0x30, &r.s1);
        write_v3state(emu, rv + 0x50, &r.vec2);
        write_fstate(emu, rv + 0x80, &r.s2);
        write_bstate(emu, rv + 0xe0, &r.byte);
        emu.write_u32(rv + 0x100, r.vis_enable);
        write_vec3(emu, rv + 0x134, &r.ambient);
        write_vec3(emu, rv + 0x140, &r.diffuse_light);
    }
    for (i, d) in m.header.camera_elems.iter().enumerate() {
        let dv = arena + CAMERADEFS + (i as u64) * 0x7c;
        write_cubic_v3_track(emu, &mut bump, arena, dv + 0x10, &d.c1);
        write_cubic_v3_track(emu, &mut bump, arena, dv + 0x38, &d.c2);
        write_cubic_sc_track(emu, &mut bump, arena, dv + 0x60, &d.c3);
        let rv = arena + CAMERARECS + (i as u64) * 0x84;
        write_v3state(emu, rv, &m.camera_recs[i].c1);
        write_v3state(emu, rv + 0x30, &m.camera_recs[i].c2);
        write_fstate(emu, rv + 0x60, &m.camera_recs[i].c3);
    }
    for (i, d) in m.header.ribbon_elems.iter().enumerate() {
        let dv = arena + RIBBONDEFS + (i as u64) * 0xdc;
        emu.write_u32(dv + 4, u32::from(d.bone));
        write_vec3_track(emu, &mut bump, arena, dv + 0x24, &d.vec);
        write_i16_track(emu, &mut bump, arena, dv + 0x40, &d.fix);
        write_scalar_track(emu, &mut bump, arena, dv + 0x5c, &d.s1);
        write_scalar_track(emu, &mut bump, arena, dv + 0x78, &d.s2);
        write_word_track(emu, &mut bump, arena, dv + 0xa4, &d.word);
        write_byte_track(emu, &mut bump, arena, dv + 0xc0, &d.byte_track);
        let rv = arena + RIBBONRECS + (i as u64) * 0xd0;
        let r = &m.ribbon_recs[i];
        write_v3state(emu, rv, &r.vec);
        write_fstate(emu, rv + 0x30, &r.fix);
        write_fstate(emu, rv + 0x50, &r.s1);
        write_fstate(emu, rv + 0x70, &r.s2);
        write_wstate(emu, rv + 0x90, &r.word);
        write_bstate(emu, rv + 0xb0, &r.byte);
    }
    for (j, d) in m.header.attachments.iter().enumerate() {
        let dv = arena + BYTEDEFS + (j as u64) * 0x30;
        emu.write_u32(dv + 4, u32::from(d.bone));
        write_vec3(emu, dv + 8, &d.local);
        write_byte_track(emu, &mut bump, arena, dv + 0x14, &d.byte_track);
    }
    for (i, d) in m.header.particles.iter().enumerate() {
        let dv = arena + ATTDEFS + (i as u64) * 0x1f8;
        emu.write_u32(dv + 0x14, u32::from(d.bone));
        for (k, tr) in d.scalars.iter().enumerate() {
            write_scalar_track(emu, &mut bump, arena, dv + 0x34 + (k as u64) * 0x1c, tr);
        }
        for (k, tr) in d.named.iter().enumerate() {
            write_scalar_track(emu, &mut bump, arena, dv + 0xdc + (k as u64) * 0x1c, tr);
        }
        write_byte_track(emu, &mut bump, arena, dv + 0x1dc, &d.byte_track);
    }
    for (i, d) in m.header.bone_defs.iter().enumerate() {
        let dv = arena + DEFS + (i as u64) * 0x6c;
        emu.write_u32(dv + 4, d.flags);
        emu.write_u32(dv + 8, u32::from(d.parent)); // word + the next word (unread) zeroed
        write_vec3_track(emu, &mut bump, arena, dv + 0xc, &d.vec_track);
        write_quat_track(emu, &mut bump, arena, dv + 0x28, &d.rot_track);
        write_vec3_track(emu, &mut bump, arena, dv + 0x44, &d.scale_track);
        write_vec3(emu, dv + 0x60, &d.basis);
    }

    for (i, b) in m.blocks.iter().enumerate() {
        let bv = arena + BLOCKS + (i as u64) * 0x118;
        // the three sampler-state sub-blocks.
        write_triple(emu, bv, &b.vec_state.primary);
        write_vec3(emu, bv + 0xc, &b.vec_state.value);
        write_triple(emu, bv + 0x18, &b.vec_state.blend);
        write_vec3(emu, bv + 0x24, &b.vec_state.blend_value);
        write_triple(emu, bv + 0x30, &b.rot_state.primary);
        for (k, &c) in b.rot_state.value.iter().enumerate() {
            emu.write_u32(bv + 0x3c + (k as u64) * 4, c.to_bits());
        }
        write_triple(emu, bv + 0x4c, &b.rot_state.blend);
        for (k, &c) in b.rot_state.blend_value.iter().enumerate() {
            emu.write_u32(bv + 0x58 + (k as u64) * 4, c.to_bits());
        }
        write_triple(emu, bv + 0x68, &b.scale_state.primary);
        write_vec3(emu, bv + 0x74, &b.scale_state.value);
        write_triple(emu, bv + 0x80, &b.scale_state.blend);
        write_vec3(emu, bv + 0x8c, &b.scale_state.blend_value);
        // The clocks and the blend weight.
        emu.write_u32(bv + 0x98, b.time);
        emu.write_u32(bv + 0x9c, b.gtime);
        emu.write_u32(bv + 0xa0, b.stamp);
        emu.write_u32(bv + 0xa4, b.track_idx);
        emu.write_u32(bv + 0xa8, b.cur_lo);
        emu.write_u32(bv + 0xac, b.cur_hi);
        emu.write_u32(bv + 0xb0, b.rate.to_bits());
        emu.write_u32(bv + 0xb8, b.bias as u32);
        emu.write_u32(bv + 0xc4, b.blend_time);
        emu.write_u32(bv + 0xc8, b.blend_gtime);
        emu.write_u32(bv + 0xd0, b.blend_track_idx);
        emu.write_u32(bv + 0xd4, b.blend_lo);
        emu.write_u32(bv + 0xd8, b.blend_hi);
        emu.write_u32(bv + 0xdc, b.blend_rate.to_bits());
        emu.write_u32(bv + 0xe4, b.blend_bias as u32);
        match &b.premult {
            None => emu.write_u32(bv + 0xf0, 0),
            Some(pm) => {
                let at = bump.take(0x40);
                write_mat(emu, arena + at, pm);
                emu.write_u32(bv + 0xf0, (arena + at) as u32);
            }
        }
        emu.write_u32(bv + 0xf4, b.flags);
        emu.write_u32(bv + 0x100, b.trans_end);
        emu.write_u32(bv + 0x104, b.inv_duration.to_bits());
        emu.write_u32(bv + 0x108, b.blend_amp.to_bits());
        emu.write_u32(bv + 0x10c, b.lambda.to_bits());
    }
    for (i, p) in m.palette.iter().enumerate() {
        write_mat(emu, arena + PALETTE + (i as u64) * 0x40, p);
    }
}

/// Compares every region `Animate` writes in the emulator with the Rust model.
fn diff_model(emu: &Emu, arena: u64, ours: &Model, ctx: &str) {
    let this = arena;
    assert_eq!(emu.read_u32(this + 0x40), ours.frame, "{ctx}: +0x40 frame");
    assert_eq!(emu.read_u32(this + 0x4c), ours.pause_stamp, "{ctx}: +0x4c");
    assert_eq!(
        emu.read_u32(this + 0x50),
        ours.visible,
        "{ctx}: +0x50 visible"
    );
    assert_eq!(emu.read_u32(this + 0x17c), ours.transport, "{ctx}: +0x17c");
    assert_f32(
        emu.read_f32(this + 0x84),
        ours.depth_key,
        &format!("{ctx}: +0x84 depth_key"),
    );
    assert_f32(
        emu.read_f32(this + 0x19c),
        ours.alpha,
        &format!("{ctx}: +0x19c alpha"),
    );
    for i in 0..3 {
        assert_f32(
            emu.read_f32(this + 0x1a0 + (i as u64) * 4),
            ours.diffuse[i],
            &format!("{ctx}: +0x1a0[{i}]"),
        );
        assert_f32(
            emu.read_f32(this + 0x1ac + (i as u64) * 4),
            ours.emissive[i],
            &format!("{ctx}: +0x1ac[{i}]"),
        );
    }
    for (i, &g) in ours.gseq_times.iter().enumerate() {
        assert_eq!(
            emu.read_u32(arena + GTIMES + (i as u64) * 4),
            g,
            "{ctx}: gseq_times[{i}]"
        );
    }
    let bw = read_mat(emu, this + 0xfc);
    for (i, (&b, &o)) in bw.iter().zip(ours.world.iter()).enumerate() {
        assert_f32(b, o, &format!("{ctx}: world[{i}]"));
    }
    assert_eq!(
        emu.read_u32(this + 0x3d8),
        ours.any_visible,
        "{ctx}: +0x3d8"
    );

    for (bi, ob) in ours.blocks.iter().enumerate() {
        let bv = arena + BLOCKS + (bi as u64) * 0x118;
        let c = |what: &str| format!("{ctx}: bone {bi} {what}");
        let diff_state = |off: u64, s: &Vec3State, tag: &str| {
            assert_eq!(
                read_triple(emu, bv + off),
                s.primary,
                "{}",
                c(&format!("{tag}.primary"))
            );
            let v = read_vec3(emu, bv + off + 0xc);
            for (k, &bv32) in v.iter().enumerate() {
                assert_f32(bv32, s.value[k], &c(&format!("{tag}.value[{k}]")));
            }
            assert_eq!(
                read_triple(emu, bv + off + 0x18),
                s.blend,
                "{}",
                c(&format!("{tag}.blend"))
            );
            let v2 = read_vec3(emu, bv + off + 0x24);
            for (k, &bv32) in v2.iter().enumerate() {
                assert_f32(
                    bv32,
                    s.blend_value[k],
                    &c(&format!("{tag}.blend_value[{k}]")),
                );
            }
        };
        // clocks first — sampler states consume them, so a clock divergence localizes here.
        assert_eq!(emu.read_u32(bv + 0x98), ob.time, "{}", c("time"));
        assert_eq!(emu.read_u32(bv + 0x9c), ob.gtime, "{}", c("gtime"));
        assert_eq!(
            emu.read_u32(bv + 0xc4),
            ob.blend_time,
            "{}",
            c("blend_time")
        );
        assert_eq!(
            emu.read_u32(bv + 0xc8),
            ob.blend_gtime,
            "{}",
            c("blend_gtime")
        );
        assert_f32(emu.read_f32(bv + 0x10c), ob.lambda, &c("lambda"));
        diff_state(0, &ob.vec_state, "vec");
        diff_state(0x68, &ob.scale_state, "scale");
        let rs = &ob.rot_state;
        assert_eq!(
            read_triple(emu, bv + 0x30),
            rs.primary,
            "{}",
            c("rot.primary")
        );
        for k in 0..4u64 {
            assert_f32(
                emu.read_f32(bv + 0x3c + k * 4),
                rs.value[k as usize],
                &c(&format!("rot.value[{k}]")),
            );
        }
        assert_eq!(read_triple(emu, bv + 0x4c), rs.blend, "{}", c("rot.blend"));
        for k in 0..4u64 {
            assert_f32(
                emu.read_f32(bv + 0x58 + k * 4),
                rs.blend_value[k as usize],
                &c(&format!("rot.blend_value[{k}]")),
            );
        }
        assert_eq!(emu.read_u32(bv + 0x98), ob.time, "{}", c("time"));
        assert_eq!(emu.read_u32(bv + 0x9c), ob.gtime, "{}", c("gtime"));
        assert_eq!(emu.read_u32(bv + 0xa0), ob.stamp, "{}", c("stamp"));
        assert_eq!(emu.read_u32(bv + 0xa4), ob.track_idx, "{}", c("track_idx"));
        assert_eq!(emu.read_u32(bv + 0xa8), ob.cur_lo, "{}", c("cur_lo"));
        assert_eq!(emu.read_u32(bv + 0xac), ob.cur_hi, "{}", c("cur_hi"));
        assert_eq!(
            emu.read_u32(bv + 0xc4),
            ob.blend_time,
            "{}",
            c("blend_time")
        );
        assert_eq!(
            emu.read_u32(bv + 0xc8),
            ob.blend_gtime,
            "{}",
            c("blend_gtime")
        );
        assert_eq!(
            emu.read_u32(bv + 0xd0),
            ob.blend_track_idx,
            "{}",
            c("blend_track_idx")
        );
        assert_eq!(emu.read_u32(bv + 0xd4), ob.blend_lo, "{}", c("blend_lo"));
        assert_eq!(emu.read_u32(bv + 0xd8), ob.blend_hi, "{}", c("blend_hi"));
        assert_f32(emu.read_f32(bv + 0x10c), ob.lambda, &c("lambda"));
    }

    let d_v3 = |emu: &Emu, va: u64, st: &Vec3State, what: &str| {
        let b = read_v3state(emu, va);
        assert_eq!(b.primary, st.primary, "{ctx}: {what}.primary");
        assert_eq!(b.blend, st.blend, "{ctx}: {what}.blend");
        for k in 0..3 {
            assert_f32(
                b.value[k],
                st.value[k],
                &format!("{ctx}: {what}.value[{k}]"),
            );
            assert_f32(
                b.blend_value[k],
                st.blend_value[k],
                &format!("{ctx}: {what}.blend_value[{k}]"),
            );
        }
    };
    let d_fs = |emu: &Emu, va: u64, st: &FloatSampleState, what: &str| {
        let b = read_fstate(emu, va);
        assert_eq!(b.primary, st.primary, "{ctx}: {what}.primary");
        assert_eq!(b.secondary, st.secondary, "{ctx}: {what}.secondary");
        assert_f32(b.value, st.value, &format!("{ctx}: {what}.value"));
        assert_f32(
            b.secondary_value,
            st.secondary_value,
            &format!("{ctx}: {what}.sec_value"),
        );
    };
    for (i, r) in ours.color_recs.iter().enumerate() {
        let rv = arena + COLORRECS + (i as u64) * 0x50;
        d_v3(emu, rv, &r.vec, &format!("colour[{i}].vec"));
        d_fs(emu, rv + 0x30, &r.fix, &format!("colour[{i}].fix"));
    }
    for (i, r) in ours.fix16_recs.iter().enumerate() {
        d_fs(
            emu,
            arena + FIX16RECS + (i as u64) * 0x20,
            r,
            &format!("transparency[{i}]"),
        );
    }
    for (i, r) in ours.word_recs.iter().enumerate() {
        let b = read_wstate(emu, arena + WORDRECS + (i as u64) * 0x20);
        assert_eq!(b, *r, "{ctx}: frame[{i}]");
    }
    for (i, r) in ours.tex_states.iter().enumerate() {
        let rv = arena + TEXSTATES + (i as u64) * 0x98;
        d_v3(emu, rv, &r.trans, &format!("texture[{i}].trans"));
        let bq = read_qstate(emu, rv + 0x30);
        assert_eq!(
            bq.primary, r.quat.primary,
            "{ctx}: texture[{i}].quat.primary"
        );
        assert_eq!(bq.blend, r.quat.blend, "{ctx}: texture[{i}].quat.blend");
        for k in 0..4 {
            assert_f32(
                bq.value[k],
                r.quat.value[k],
                &format!("{ctx}: texture[{i}].quat.v[{k}]"),
            );
            assert_f32(
                bq.blend_value[k],
                r.quat.blend_value[k],
                &format!("{ctx}: texture[{i}].quat.bv[{k}]"),
            );
        }
        d_v3(emu, rv + 0x68, &r.scale, &format!("texture[{i}].scale"));
        let bm = read_mat(emu, arena + TEXMATS + (i as u64) * 0x40);
        for (k, (&b, &o)) in bm.iter().zip(ours.tex_matrices[i].iter()).enumerate() {
            assert_f32(b, o, &format!("{ctx}: texture[{i}].mat[{k}]"));
        }
    }
    for (i, r) in ours.light_recs.iter().enumerate() {
        let rv = arena + LIGHTRECS + (i as u64) * 0x170;
        d_v3(emu, rv, &r.vec1, &format!("light[{i}].vec1"));
        d_fs(emu, rv + 0x30, &r.s1, &format!("light[{i}].s1"));
        d_v3(emu, rv + 0x50, &r.vec2, &format!("light[{i}].vec2"));
        d_fs(emu, rv + 0x80, &r.s2, &format!("light[{i}].s2"));
        let bb = read_bstate(emu, rv + 0xe0);
        assert_eq!(bb, r.byte, "{ctx}: light[{i}].byte");
        for k in 0..3u64 {
            assert_f32(
                emu.read_f32(rv + 0x134 + k * 4),
                r.ambient[k as usize],
                &format!("{ctx}: light[{i}].ambient[{k}]"),
            );
            assert_f32(
                emu.read_f32(rv + 0x140 + k * 4),
                r.diffuse_light[k as usize],
                &format!("{ctx}: light[{i}].diffuse_light[{k}]"),
            );
        }
    }
    for (i, r) in ours.camera_recs.iter().enumerate() {
        let rv = arena + CAMERARECS + (i as u64) * 0x84;
        d_v3(emu, rv, &r.c1, &format!("camera[{i}].c1"));
        d_v3(emu, rv + 0x30, &r.c2, &format!("camera[{i}].c2"));
        d_fs(emu, rv + 0x60, &r.c3, &format!("camera[{i}].c3"));
    }
    for (i, r) in ours.ribbon_recs.iter().enumerate() {
        let rv = arena + RIBBONRECS + (i as u64) * 0xd0;
        d_v3(emu, rv, &r.vec, &format!("ribbon[{i}].vec"));
        d_fs(emu, rv + 0x30, &r.fix, &format!("ribbon[{i}].fix"));
        d_fs(emu, rv + 0x50, &r.s1, &format!("ribbon[{i}].s1"));
        d_fs(emu, rv + 0x70, &r.s2, &format!("ribbon[{i}].s2"));
        let bw = read_wstate(emu, rv + 0x90);
        assert_eq!(bw, r.word, "{ctx}: ribbon[{i}].word");
        let bb = read_bstate(emu, rv + 0xb0);
        assert_eq!(bb, r.byte, "{ctx}: ribbon[{i}].byte");
    }
    for (i, orr) in ours.particle_recs.iter().enumerate() {
        let rv = arena + ATTRECS + (i as u64) * 0x16c;
        for (k, st) in orr.scalars.iter().enumerate() {
            let b = read_fstate(emu, rv + (k as u64) * 0x20);
            assert_eq!(b.primary, st.primary, "{ctx}: att {i} sc{k}.primary");
            assert_f32(b.value, st.value, &format!("{ctx}: att {i} sc{k}.value"));
            assert_eq!(b.secondary, st.secondary, "{ctx}: att {i} sc{k}.secondary");
            assert_f32(
                b.secondary_value,
                st.secondary_value,
                &format!("{ctx}: att {i} sc{k}.sec_value"),
            );
        }
        for (k, st) in orr.named.iter().enumerate() {
            let b = read_fstate(emu, rv + 0xc0 + (k as u64) * 0x20);
            assert_eq!(b.primary, st.primary, "{ctx}: att {i} nm{k}.primary");
            assert_f32(b.value, st.value, &format!("{ctx}: att {i} nm{k}.value"));
            assert_eq!(b.secondary, st.secondary, "{ctx}: att {i} nm{k}.secondary");
            assert_f32(
                b.secondary_value,
                st.secondary_value,
                &format!("{ctx}: att {i} nm{k}.sec_value"),
            );
        }
        let bb = read_bstate(emu, rv + 0x140);
        assert_eq!(bb, orr.byte, "{ctx}: att {i} byte state");
        assert_eq!(
            emu.read_u32(rv + 0x160),
            orr.enabled_blend,
            "{ctx}: att {i} +0x160"
        );
        assert_eq!(
            emu.read_u32(rv + 0x164),
            orr.visible,
            "{ctx}: att {i} +0x164"
        );
    }
    if let Some(pl) = &ours.attach_payload {
        for (j, st) in pl.iter().enumerate() {
            let b = read_bstate(emu, arena + PAYLOAD + (j as u64) * 0x20);
            assert_eq!(b, *st, "{ctx}: payload {j}");
        }
    }

    for (bi, op) in ours.palette.iter().enumerate() {
        let pm = read_mat(emu, arena + PALETTE + (bi as u64) * 0x40);
        for (k, (&b, &o)) in pm.iter().zip(op.iter()).enumerate() {
            assert_f32(b, o, &format!("{ctx}: palette[{bi}][{k}]"));
        }
    }
}

/// Run one fixture through both sides. `models` = [parent?, subject]; the subject is last.
fn run_case(emu: &mut Emu, arenas: &[u64], models: &mut [Model], args: &KernelArgs, ctx: &str) {
    let subject = models.len() - 1;
    for i in 0..models.len() {
        // the 0x7b5f60 predicate is a fixture-wide constant (a stub returns one eax); every
        // model in a forest must agree.
        assert_eq!(
            models[i].has_live_particles,
            models[subject].has_live_particles
        );
        write_model(emu, arenas, i, models);
    }
    // (the 0x7b5f60 predicate stub is registered ONCE per Emu by the caller — Emu::stub adds
    // a hook each call, so per-case registration would stack them.)
    let a = arenas[subject] + ARGS;
    write_mat(emu, a, &args.parent_mtx);
    write_vec3(emu, a + 0x40, &args.diffuse);
    write_vec3(emu, a + 0x50, &args.emissive);

    emu.clear_calls();
    emu.call(
        FN_VA,
        arenas[subject] as u32,
        0,
        &[
            a as u32,
            (a + 0x40) as u32,
            (a + 0x50) as u32,
            args.alpha.to_bits(),
        ],
    );

    m2_animate(
        models,
        subject,
        &args.parent_mtx,
        &args.diffuse,
        &args.emissive,
        args.alpha,
    );
    // the recursion writes children too — diff EVERY model in the forest.
    for (i, m) in models.iter().enumerate() {
        diff_model(emu, arenas[i], m, &format!("{ctx} (model {i})"));
    }
}

struct KernelArgs {
    parent_mtx: [f32; 16],
    diffuse: [f32; 3],
    emissive: [f32; 3],
    alpha: f32,
}

// ===== fixture generators (deterministic branch driving; LCG high bits for values) =====

fn fz(s: &mut u32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (m * 8.0 - 4.0) as f32
}

fn unit_quat(s: &mut u32) -> [f32; 4] {
    let mut q = [0f64; 4];
    let mut n = 0f64;
    for c in q.iter_mut() {
        *c = f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0;
        n += *c * *c;
    }
    if n <= 0.0 {
        return [1.0, 0.0, 0.0, 0.0];
    }
    let inv = 1.0 / n.sqrt();
    core::array::from_fn(|i| (q[i] * inv) as f32)
}

/// A track with `n` keys spanning [0, span] (strictly increasing), the given interp/gseq.
fn meta(s: &mut u32, n: u32, span: u32, interp: u16, gseq: u16) -> TrackMeta {
    let mut ts = Vec::with_capacity(n as usize);
    for i in 0..n {
        let base = if n > 1 { span * i / (n - 1) } else { 0 };
        ts.push(base + (next(s) >> 8) % 3);
    }
    for i in 1..ts.len() {
        if ts[i] <= ts[i - 1] {
            ts[i] = ts[i - 1] + 1;
        }
    }
    TrackMeta {
        interp,
        gseq,
        range_count: 0,
        ranges: vec![[0, 0]],
        key_count: n,
        timestamps: ts,
    }
}

fn vec3_track(s: &mut u32, n: u32, span: u32, interp: u16, gseq: u16) -> Vec3TrackDef {
    let m = meta(s, n, span, interp, gseq);
    let payload = (0..n).map(|_| [fz(s), fz(s), fz(s)]).collect();
    Vec3TrackDef { meta: m, payload }
}

fn quat_track(s: &mut u32, n: u32, span: u32, interp: u16, gseq: u16) -> QuatTrackDef {
    let m = meta(s, n, span, interp, gseq);
    let payload = (0..n).map(|_| unit_quat(s)).collect();
    QuatTrackDef { meta: m, payload }
}

/// A baseline model: `n_bones` bones, sane clocks, identity-ish matrices, no element/attach
/// work. Case generators specialize from here.
fn base_model(s: &mut u32, n_bones: u32) -> Model {
    let span = 2000u32;
    let seqs = vec![
        SeqRecord {
            start: 0,
            end: span,
            flags: 0,
        },
        SeqRecord {
            start: 100,
            end: 900,
            flags: 1,
        },
        SeqRecord {
            start: 50,
            end: 50,
            flags: 1,
        }, // degenerate span
    ];
    let mut bone_defs = Vec::new();
    for i in 0..n_bones {
        bone_defs.push(BoneDef {
            flags: 0x280, // TRS path by default
            parent: if i == 0 {
                0xffff
            } else {
                (next(s) >> 8) as u16 % i as u16
            },
            vec_track: vec3_track(s, 4, span, 1, 0xffff),
            rot_track: quat_track(s, 4, span, 1, 0xffff),
            scale_track: vec3_track(s, 4, span, 1, 0xffff),
            basis: [fz(s), fz(s), fz(s)],
        });
    }
    let blocks = (0..n_bones)
        .map(|_| BoneState {
            track_idx: 0,
            cur_lo: 0,
            cur_hi: 500,
            rate: 1.0,
            bias: 0,
            blend_track_idx: 0xffff_ffff,
            blend_lo: 0,
            blend_hi: 500,
            blend_rate: 1.0,
            blend_bias: 0,
            trans_end: 0,
            inv_duration: 0.001,
            blend_amp: 1.0,
            scale_state: Vec3State {
                value: [1.0, 1.0, 1.0],
                ..Default::default()
            },
            rot_state: QuatSampleState {
                value: [0.0, 0.0, 0.0, 1.0],
                ..Default::default()
            },
            ..Default::default()
        })
        .collect::<Vec<_>>();
    Model {
        loaded: 1,
        frame: 7,
        scene_frame: 9, // not the frame, so the pass runs
        scene_clock: 700,
        pause_stamp: 0,
        gseq_times: vec![0; 4],
        gseq_base: 13,
        first_update_gate: 1, // counts of 4 > 1 ⇒ samples run
        blocks,
        palette: (0..n_bones)
            .map(|_| {
                let mut m = [0f32; 16];
                for (k, e) in m.iter_mut().enumerate() {
                    *e = if k % 5 == 0 { 1.0 } else { fz(s) * 0.1 };
                }
                m
            })
            .collect(),
        base: core::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { fz(s) * 0.05 }),
        world: core::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { 0.0 }),
        alpha_scale: 1.0,
        diffuse_scale: [1.0, 1.0, 1.0],
        emissive_bias: [0.0, 0.0, 0.0],
        header: Md20 {
            gseq: vec![0, 350, 1, 1000],
            seqs,
            bone_count: n_bones,
            bone_defs,
            attachments: Vec::new(),
            particles: Vec::new(),
            color_elems: Vec::new(),
            fix16_elems: Vec::new(),
            word_elems: Vec::new(),
            tex_xforms: Vec::new(),
            light_elems: Vec::new(),
            camera_elems: Vec::new(),
            ribbon_elems: Vec::new(),
        },
        ..Default::default()
    }
}

/// A scalar track with `n` keys over the span.
fn scalar_track(s: &mut u32, n: u32, span: u32, interp: u16, gseq: u16) -> ScalarTrackDef {
    let m = meta(s, n, span, interp, gseq);
    let payload = (0..n).map(|_| fz(s)).collect();
    ScalarTrackDef { meta: m, payload }
}

/// A byte track with `n` keys over the span.
fn byte_track(s: &mut u32, n: u32, span: u32, interp: u16, gseq: u16) -> ByteTrackDef {
    let m = meta(s, n, span, interp, gseq);
    let payload = (0..n).map(|_| (next(s) >> 24) as u8).collect();
    ByteTrackDef { meta: m, payload }
}

fn i16_track(s: &mut u32, n: u32, span: u32, interp: u16, gseq: u16) -> I16TrackDef {
    let m = meta(s, n, span, interp, gseq);
    let payload = (0..n).map(|_| (next(s) >> 16) as i16).collect();
    I16TrackDef { meta: m, payload }
}

fn word_track_def(s: &mut u32, n: u32, span: u32, interp: u16, gseq: u16) -> WordTrackDef {
    let m = meta(s, n, span, interp, gseq);
    let payload = (0..n).map(|_| (next(s) >> 16) as u16).collect();
    WordTrackDef { meta: m, payload }
}

fn cubic_v3_track(s: &mut u32, n: u32, span: u32, interp: u16) -> CubicVec3TrackDef {
    let m = meta(s, n, span, interp, 0xffff);
    let payload = (0..n)
        .map(|_| {
            [
                [fz(s), fz(s), fz(s)],
                [fz(s), fz(s), fz(s)],
                [fz(s), fz(s), fz(s)],
            ]
        })
        .collect();
    CubicVec3TrackDef { meta: m, payload }
}

fn cubic_sc_track(s: &mut u32, n: u32, span: u32, interp: u16) -> CubicScalarTrackDef {
    let m = meta(s, n, span, interp, 0xffff);
    let payload = (0..n).map(|_| [fz(s), fz(s), fz(s)]).collect();
    CubicScalarTrackDef { meta: m, payload }
}

/// A particle record whose ten tracks cycle between skipped and sampled, and step and linear.
fn particle_def(s: &mut u32, n_bones: u32, span: u32, kase: u32) -> ParticleDef {
    let mk = |s: &mut u32, k: u32| {
        // One key does not exceed the gate, so that track keeps its stored state.
        let n = if (kase + k).is_multiple_of(4) { 1 } else { 4 };
        scalar_track(s, n, span, ((kase + k) % 2) as u16, 0xffff)
    };
    ParticleDef {
        bone: ((next(s) >> 8) % n_bones) as u16,
        scalars: core::array::from_fn(|k| mk(s, k as u32)),
        named: core::array::from_fn(|k| mk(s, 6 + k as u32)),
        byte_track: byte_track(
            s,
            if kase.is_multiple_of(3) { 1 } else { 4 },
            span,
            (kase % 2) as u16,
            0xffff,
        ),
    }
}

#[test]
fn m2_animate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // pre-resolve the delay-link slots (the resolved-program state).
    emu.write_u32(0x876504, 0x74a7c6);
    emu.write_u32(0x876594, 0x74b6bb);
    emu.write_u32(0x876588, 0x74d11a);
    // plant the UV-pivot globals' STEADY state: the texture-transform loop's lazy init
    // (0x715f45–0x715f81) is gated on the flag byte [0xcf04c4]&1 and on first run also
    // registers an atexit (`push 0x7187e0; call 0x409aef`) that walks into the CRT heap —
    // setting the flag + the 0.5/0.5/0 pivots mirrors the live program after frame one.
    emu.write_u32(0x00cf_04c4, 1);
    emu.write_u32(0x00cf_043c, 0x3f00_0000);
    emu.write_u32(0x00cf_0440, 0x3f00_0000);
    emu.write_u32(0x00cf_0444, 0);
    let arena_parent = emu.alloc(0x20000);
    let arena_subject = emu.alloc(0x20000);

    let mut s = 0x7142_6000u32;
    let mut checked = 0u32;

    // The gates: not loaded, and the frame already the scene's; nothing changes.
    for gate in 0..2u32 {
        let mut m = base_model(&mut s, 2);
        if gate == 0 {
            m.loaded = 0;
        } else {
            m.frame = m.scene_frame;
        }
        let args = KernelArgs {
            parent_mtx: core::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { 0.0 }),
            diffuse: [1.0, 1.0, 1.0],
            emissive: [0.0, 0.0, 0.0],
            alpha: 33.0,
        };
        run_case(
            &mut emu,
            &[arena_subject],
            &mut [m],
            &args,
            &format!("gate {gate}"),
        );
        checked += 1;
    }

    // Bones only, driving the bone loop's paths.
    for case in 0..160u32 {
        let n_bones = 1 + (next(&mut s) >> 8) % 5;
        let with_parent = case % 4 == 1;
        let mut subject = base_model(&mut s, n_bones);

        // The colour and alpha scales, and the global sequence clocks.
        subject.alpha_scale = fz(&mut s).abs() + 0.25;
        subject.diffuse_scale = [fz(&mut s), fz(&mut s), fz(&mut s)];
        subject.emissive_bias = [fz(&mut s), fz(&mut s), fz(&mut s)];
        subject.scene_clock = 200 + (next(&mut s) >> 8) % 3000;
        // The pause slide: the stamp 0 and nonzero.
        subject.pause_stamp = if case % 3 == 0 {
            0
        } else {
            400 + (next(&mut s) >> 8) % 300
        };

        // per-bone leg driving, cycled deterministically.
        for (bi, def) in subject.header.bone_defs.iter_mut().enumerate() {
            let mode = (case as usize + bi) % 8;
            let blk = &mut subject.blocks[bi];
            match mode {
                0 => {
                    // primary inherit + both-inherit λ ladder.
                    blk.track_idx = 0xffff_ffff;
                }
                1 => {
                    // the clamp leg (seq 1 flags&1, hi <= dur).
                    blk.track_idx = 1;
                    blk.cur_lo = 30;
                    blk.cur_hi = 400.min(subject.scene_clock);
                }
                2 => {
                    // the wrap leg via flags&1 + hi > dur.
                    blk.track_idx = 1;
                    blk.cur_lo = 10;
                    blk.cur_hi = subject.scene_clock + 500;
                }
                3 => {
                    // the modulo leg (seq 0, flags clear) + a live blend with λ smoothstep.
                    blk.track_idx = 0;
                    blk.blend_track_idx = 1;
                    blk.blend_lo = 20;
                    blk.blend_hi = 300;
                    blk.trans_end = subject.scene_clock + 400; // elapsed>0 ⇒ smoothstep
                    blk.inv_duration = 0.002;
                    blk.blend_amp = 0.75;
                }
                4 => {
                    // billboard path (arm by sel bits), &0x280 clear.
                    def.flags = 0x8 << ((next(&mut s) >> 9) % 4); // 0x8/0x10/0x20/0x40
                }
                5 => {
                    // the &7/&6 block: normalize variants, &1 pivot or fold.
                    def.flags = 0x280 | (1 + ((next(&mut s) >> 9) % 7)); // bits 0..2
                }
                6 => {
                    // TRS + billboard overlay (arm 0's TRS-side leg + the tail).
                    def.flags = 0x280 | 0x8;
                }
                _ => {
                    // step-interp tracks + the premult leg.
                    def.vec_track.meta.interp = 0;
                    def.scale_track.meta.interp = 0;
                    def.flags = 0x280 | 0x80;
                    blk.premult = Some(core::array::from_fn(|k| {
                        if k % 5 == 0 {
                            1.0
                        } else {
                            fz(&mut s) * 0.1
                        }
                    }));
                }
            }
            // degenerate-span + transition-expiry sprinkles.
            if case % 5 == 0 && mode == 3 {
                blk.blend_track_idx = 2; // the degenerate seq (end==start)
                blk.trans_end = subject.scene_clock.wrapping_sub(1); // expiry fires
            }
            // gseq-tagged rotation tracks on some bones (the dual time base inside 0x713ea0).
            if mode == 6 && case % 2 == 0 {
                def.rot_track.meta.gseq = 1; // gseq[1] = 350 ticks
            }
        }

        let args = KernelArgs {
            parent_mtx: core::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { fz(&mut s) * 0.1 }),
            diffuse: [fz(&mut s), fz(&mut s), fz(&mut s)],
            emissive: [fz(&mut s), fz(&mut s), fz(&mut s)],
            alpha: fz(&mut s).abs() * 50.0,
        };

        if with_parent {
            let mut parent = base_model(&mut s, 1);
            parent.visible = u32::from(case % 8 >= 4);
            parent.transport = next(&mut s);
            parent.depth_key = fz(&mut s);
            parent.flags4 = u32::from(case % 2 == 0);
            subject.parent = Some(0);
            subject.child_visible = u32::from(case % 3 != 0);
            let mut models = [parent, subject];
            run_case(
                &mut emu,
                &[arena_parent, arena_subject],
                &mut models,
                &args,
                &format!("case {case} (parented)"),
            );
        } else {
            run_case(
                &mut emu,
                &[arena_subject],
                &mut [subject],
                &args,
                &format!("case {case}"),
            );
        }
        checked += 1;
    }

    // The particle loop and the attachment walk call `HasLiveParticles` (`0x7b5f60`), stubbed
    // to a fixed value per emulator.
    let mut emu_pred: [Emu; 2] = [Emu::new(&wow), Emu::new(&wow)];
    let mut pred_arenas = [[0u64; 3]; 2];
    for (pv, e) in emu_pred.iter_mut().enumerate() {
        e.write_u32(0x876504, 0x74a7c6);
        e.write_u32(0x876594, 0x74b6bb);
        e.write_u32(0x876588, 0x74d11a);
        e.stub(0x7b5f60, pv as u32, 0);
        for a in pred_arenas[pv].iter_mut() {
            *a = e.alloc(0x20000);
        }
    }

    // The particle loop: the flags and the ten tracks.
    for case in 0..40u32 {
        let mut m = base_model(&mut s, 3);
        let n_att = 1 + (next(&mut s) >> 8) % 3;
        m.header.particles = (0..n_att)
            .map(|k| particle_def(&mut s, 3, 2000, case + k))
            .collect();
        m.particle_recs = vec![ParticleRec::default(); n_att as usize];
        m.particle_emitters = (0..n_att).map(|_| next(&mut s)).collect();
        m.has_live_particles = u32::from(case % 2 == 0);
        m.visible = u32::from(case % 4 < 2);
        // pre-seed byte states so GATED (stale) enable tracks exercise the nonzero leg too.
        if case % 3 == 0 {
            for r in m.particle_recs.iter_mut() {
                r.byte.value = 1;
            }
        }
        let args = KernelArgs {
            parent_mtx: core::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { 0.0 }),
            diffuse: [1.0, 1.0, 1.0],
            emissive: [fz(&mut s), fz(&mut s), fz(&mut s)],
            alpha: fz(&mut s).abs() * 40.0,
        };
        let pv = m.has_live_particles as usize;
        run_case(
            &mut emu_pred[pv],
            &pred_arenas[pv][2..3],
            &mut [m],
            &args,
            &format!("attach {case}"),
        );
        checked += 1;
    }

    // The attachment walk, with a forest of children.
    for case in 0..30u32 {
        let mut subject = base_model(&mut s, 3);
        subject.header.attachments = (0..2u32)
            .map(|j| AttachmentDef {
                bone: ((next(&mut s) >> 8) % 3) as u16,
                local: [fz(&mut s), fz(&mut s), fz(&mut s)],
                byte_track: byte_track(
                    &mut s,
                    if (case + j) % 4 == 0 { 1 } else { 4 },
                    2000,
                    ((case + j) % 2) as u16,
                    0xffff,
                ),
            })
            .collect();
        let mut payload = vec![ByteSampleState::default(); 2];
        for (j, st) in payload.iter_mut().enumerate() {
            // the child gate reads this byte (a gated/stale track leaves it as planted).
            st.value = u8::from(!(case as usize + j).is_multiple_of(3));
        }
        subject.attach_payload = Some(payload);
        // F + G together on half the cases.
        if case % 2 == 0 {
            subject.header.particles = vec![particle_def(&mut s, 3, 2000, case)];
            subject.particle_recs = vec![ParticleRec::default()];
            subject.particle_emitters = vec![next(&mut s)];
        }
        subject.first_child = Some(0);
        let mut c0 = base_model(&mut s, 2);
        c0.parent = Some(2);
        c0.attachment_index = 0;
        c0.next_sibling = Some(1);
        if case % 5 == 0 {
            c0.frame = c0.scene_frame; // a child the entry gate skips
        }
        let mut c1 = base_model(&mut s, 2);
        c1.parent = Some(2);
        c1.attachment_index = if case % 4 == 3 { 0xffff } else { 1 };
        let pred = u32::from(case % 2 == 0);
        subject.has_live_particles = pred;
        c0.has_live_particles = pred;
        c1.has_live_particles = pred;
        let args = KernelArgs {
            parent_mtx: core::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { fz(&mut s) * 0.1 }),
            diffuse: [fz(&mut s), fz(&mut s), fz(&mut s)],
            emissive: [fz(&mut s), fz(&mut s), fz(&mut s)],
            alpha: fz(&mut s).abs() * 40.0,
        };
        let mut models = [c0, c1, subject];
        let pv = pred as usize;
        run_case(
            &mut emu_pred[pv],
            &pred_arenas[pv],
            &mut models,
            &args,
            &format!("recursion {case}"),
        );
        checked += 1;
    }

    // The element loops, with the bones that the light and ribbon records name armed.
    for case in 0..45u32 {
        let mut m = base_model(&mut s, 3);
        if case % 6 == 5 {
            m.first_update_gate = 0; // every track samples; lights with the gate at 0
        }
        // A live cross-fade on bones 1 and 2, so the blend legs run for records that name them.
        for bi in 1..3usize {
            let blk = &mut m.blocks[bi];
            blk.blend_track_idx = 1;
            blk.blend_lo = 20;
            blk.blend_hi = 300;
            blk.trans_end = m.scene_clock + 300 + 100 * bi as u32;
            blk.inv_duration = 0.002;
            blk.blend_amp = 0.8;
        }
        let span = 2000u32;
        let n_of = |k: u32| if (case + k).is_multiple_of(4) { 1 } else { 4 }; // 1 ⇒ gated/stale
        let iv = |k: u32| ((case + k) % 2) as u16; // step vs linear
        let gq = |k: u32| {
            if (case + k).is_multiple_of(5) {
                1u16
            } else {
                0xffff
            }
        }; // the global sequence clock
        m.header.color_elems = (0..2)
            .map(|k| ColorElemDef {
                vec_track: vec3_track(&mut s, n_of(k), span, iv(k), gq(k)),
                fix_track: i16_track(&mut s, n_of(k + 1), span, iv(k + 1), gq(k + 3)),
            })
            .collect();
        m.color_recs = vec![ColorElemRec::default(); 2];
        m.header.fix16_elems = vec![i16_track(&mut s, n_of(case % 3), span, iv(case), 0xffff)];
        m.fix16_recs = vec![FloatSampleState::default(); 1];
        m.header.word_elems = vec![word_track_def(
            &mut s,
            n_of(case % 2),
            span,
            iv(case + 1),
            0xffff,
        )];
        m.word_recs = vec![WordState::default(); 1];
        // Texture transforms: key counts 0 and n (each track runs when its count is not 0).
        let l4n = |k: u32| if (case + k).is_multiple_of(3) { 0 } else { 4 };
        m.header.tex_xforms = vec![TexXformDef {
            trans_track: vec3_track(&mut s, l4n(0).max(1), span, iv(case), 0xffff),
            quat_track: quat_track(&mut s, l4n(1).max(1), span, 1, 0xffff),
            scale_track: vec3_track(&mut s, l4n(2).max(1), span, iv(case + 1), 0xffff),
        }];
        if l4n(0) == 0 {
            m.header.tex_xforms[0].trans_track.meta.key_count = 0;
        }
        if l4n(1) == 0 {
            m.header.tex_xforms[0].quat_track.meta.key_count = 0;
        }
        if l4n(2) == 0 {
            m.header.tex_xforms[0].scale_track.meta.key_count = 0;
        }
        m.tex_states = vec![TexXformRec::default(); 1];
        m.tex_matrices = vec![[0f32; 16]; 1];
        // Lights: the enabled word and the byte, with the bones the records name.
        m.header.light_elems = (0..2)
            .map(|k| LightElemDef {
                bone: (1 + (k as u16 + case as u16) % 2),
                vec1: vec3_track(&mut s, n_of(k), span, iv(k), 0xffff),
                s1: scalar_track(&mut s, n_of(k + 2), span, iv(k + 1), 0xffff),
                vec2: vec3_track(&mut s, n_of(k + 1), span, iv(k), 0xffff),
                s2: scalar_track(&mut s, n_of(k + 3), span, iv(k), 0xffff),
                byte_track: byte_track(
                    &mut s,
                    if (case + k) % 3 == 0 { 0 } else { 4 },
                    span,
                    1,
                    0xffff,
                ),
            })
            .collect();
        m.light_recs = (0..2)
            .map(|k| {
                let mut r = LightElemRec {
                    vis_enable: u32::from((case as usize + k) % 3 != 2),
                    ..Default::default()
                };
                // The stored byte, which gates the light's tracks when the byte track is skipped.
                r.byte.value = u8::from((case as usize + k).is_multiple_of(2));
                r
            })
            .collect();
        // Cameras: the interpolation words 0 to 3, cycled.
        m.header.camera_elems = vec![CameraElemDef {
            c1: cubic_v3_track(&mut s, 4, span, (case % 4) as u16),
            c2: cubic_v3_track(&mut s, 4, span, ((case + 1) % 4) as u16),
            c3: cubic_sc_track(&mut s, 4, span, ((case + 2) % 4) as u16),
        }];
        m.camera_recs = vec![CameraElemRec::default(); 1];
        // Ribbons.
        m.header.ribbon_elems = vec![RibbonElemDef {
            bone: (case % 3) as u16,
            vec: vec3_track(&mut s, n_of(case), span, iv(case), gq(case)),
            fix: i16_track(&mut s, n_of(case + 1), span, iv(case + 1), 0xffff),
            s1: scalar_track(&mut s, n_of(case + 2), span, iv(case), 0xffff),
            s2: scalar_track(&mut s, 4, span, iv(case + 1), 0xffff),
            word: word_track_def(&mut s, 4, span, iv(case), 0xffff),
            byte_track: byte_track(&mut s, 4, span, iv(case + 1), 0xffff),
        }];
        m.ribbon_recs = vec![RibbonElemRec::default(); 1];

        let args = KernelArgs {
            parent_mtx: core::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { fz(&mut s) * 0.1 }),
            diffuse: [1.0, 1.0, 1.0],
            emissive: [fz(&mut s), fz(&mut s), fz(&mut s)],
            alpha: fz(&mut s).abs() * 40.0,
        };
        run_case(
            &mut emu,
            &[arena_subject],
            &mut [m],
            &args,
            &format!("elements {case}"),
        );
        checked += 1;
    }

    // The blend legs of the element loops (`systems/animation/tracks.md#sampling-block`). A
    // track blends when its interpolation word is not 0, it has no global sequence and its bone's
    // weight is not 0. The colour, transparency, texture frame, texture transform and camera
    // loops read the clocks of bone 0, so the cross-fade is armed on every bone, and every
    // track gets the interpolation word 1 (the cameras cycle 1 to 3), no global sequence and
    // four keys, more than the gate.
    //
    // `arm_blend` arms one bone: a blend record other than the primary one (so the two slots
    // differ and the weight is not 0), a fade end after the clock, a finite fade rate and a
    // nonzero scale.
    let arm_blend = |blk: &mut BoneState, scene_clock: u32, bseq: u32, amp: f32, invd: f32| {
        blk.track_idx = 0; // primary on seq 0
        blk.cur_lo = 10;
        blk.cur_hi = 400;
        blk.rate = 1.0;
        blk.bias = 0;
        blk.blend_track_idx = bseq; // another record, so the slots differ and the weight is not 0
        blk.blend_lo = 20;
        blk.blend_hi = 300;
        blk.blend_rate = 1.0;
        blk.blend_bias = 0;
        blk.trans_end = scene_clock + 600; // the fade end is after the clock
        blk.inv_duration = invd;
        blk.blend_amp = amp;
    };
    for case in 0..36u32 {
        let mut m = base_model(&mut s, 3);
        // Every track samples: four keys exceed the gate.
        m.first_update_gate = 1;
        // A live cross-fade on all three bones (the light and ribbon records name bones 0 to 2).
        // The cases cycle the three weight stores of `systems/animation/clock.md#weight-selection`:
        // the smoothstep, the saturated weight (a large fade rate), and `t < 0` (a negative fade
        // rate).
        let st = m.scene_clock;
        let (amp, invd) = match case % 6 {
            0 | 3 => (0.8f32, 0.0005f32), // `0 < t < 1`: the smoothstep
            1 | 4 => (0.7f32, 0.05f32),   // `t` far above 1: the weight is the scale
            _ => (0.6f32, -0.002f32), // a negative fade rate: `t < 0`, the weight is 0 times the scale
        };
        for b in 0..3usize {
            arm_blend(&mut m.blocks[b], st, 0, amp, invd);
        }
        let span = 2000u32;
        // Every track has four keys, which exceed the gate, and no global sequence. The
        // interpolation word is 1 except for the camera tracks, which cycle 1 to 3 so that the
        // dispatch inside the blend (`0x716d75`: `dec; je; dec; je; dec; jne`) reaches each curve.
        let lin_v3 = |s: &mut u32| vec3_track(s, 4, span, 1, 0xffff);
        let lin_i16 = |s: &mut u32| i16_track(s, 4, span, 1, 0xffff);
        let lin_word = |s: &mut u32| word_track_def(s, 4, span, 1, 0xffff);
        let lin_sc = |s: &mut u32| scalar_track(s, 4, span, 1, 0xffff);
        // The camera words 1 to 3, cycled (0 is a step and skips the blend).
        let civ = |k: u32| (1 + (case + k) % 3) as u16;

        m.header.color_elems = (0..2)
            .map(|_| ColorElemDef {
                vec_track: lin_v3(&mut s),
                fix_track: lin_i16(&mut s),
            })
            .collect();
        m.color_recs = vec![ColorElemRec::default(); 2];
        m.header.fix16_elems = vec![lin_i16(&mut s)];
        m.fix16_recs = vec![FloatSampleState::default(); 1];
        m.header.word_elems = vec![lin_word(&mut s)];
        m.word_recs = vec![WordState::default(); 1];
        // Texture transforms: the translation and scale tracks blend (the rotation track has its
        // own quaternion blend, reached already); four keys each, linear, no global sequence.
        m.header.tex_xforms = vec![TexXformDef {
            trans_track: lin_v3(&mut s),
            quat_track: quat_track(&mut s, 4, span, 1, 0xffff),
            scale_track: lin_v3(&mut s),
        }];
        m.tex_states = vec![TexXformRec::default(); 1];
        m.tex_matrices = vec![[0f32; 16]; 1];
        // Lights: a record names an armed bone; the vector and scalar tracks are linear with no
        // global sequence.
        m.header.light_elems = (0..2)
            .map(|k| LightElemDef {
                bone: ((k as u16 + case as u16) % 3),
                vec1: lin_v3(&mut s),
                s1: lin_sc(&mut s),
                vec2: lin_v3(&mut s),
                s2: lin_sc(&mut s),
                byte_track: byte_track(&mut s, 4, span, 1, 0xffff),
            })
            .collect();
        m.light_recs = (0..2)
            .map(|k| {
                let mut r = LightElemRec {
                    vis_enable: 1, // enabled, so the light's tracks run
                    ..Default::default()
                };
                r.byte.value = u8::from(k % 2 == 0);
                r
            })
            .collect();
        // Cameras: the words 1 to 3 cycled, so the blend leg covers each curve.
        m.header.camera_elems = vec![CameraElemDef {
            c1: cubic_v3_track(&mut s, 4, span, civ(0)),
            c2: cubic_v3_track(&mut s, 4, span, civ(1)),
            c3: cubic_sc_track(&mut s, 4, span, civ(2)),
        }];
        m.camera_recs = vec![CameraElemRec::default(); 1];
        // Ribbons: a record names an armed bone; every track is linear with no global sequence,
        // so the vector, 16-bit, scalar and word blend legs all run.
        m.header.ribbon_elems = vec![RibbonElemDef {
            bone: (case % 3) as u16,
            vec: lin_v3(&mut s),
            fix: lin_i16(&mut s),
            s1: lin_sc(&mut s),
            s2: lin_sc(&mut s),
            word: lin_word(&mut s),
            byte_track: byte_track(&mut s, 4, span, 1, 0xffff),
        }];
        m.ribbon_recs = vec![RibbonElemRec::default(); 1];

        let args = KernelArgs {
            parent_mtx: core::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { fz(&mut s) * 0.1 }),
            diffuse: [1.0, 1.0, 1.0],
            emissive: [fz(&mut s), fz(&mut s), fz(&mut s)],
            alpha: fz(&mut s).abs() * 40.0,
        };
        run_case(
            &mut emu,
            &[arena_subject],
            &mut [m],
            &args,
            &format!("blend {case}"),
        );
        checked += 1;
    }

    // The blend slot's loop leg (`0x714773`): `fild(base - blend_lo) · blend_rate`, `__ftol`, and
    // the division by the span. It runs only when the blend record has flag bit 0 clear (record 0,
    // `{0, 2000, flags 0}`) with a span above 0. The earlier cases put the blend on a clamped
    // record, so here the primary is on the clamped record 1 and the blend on record 0, with a
    // wide window and varied speeds.
    for case in 0..4u32 {
        let mut m = base_model(&mut s, 3);
        m.first_update_gate = 1;
        let stime = m.scene_clock;
        for b in 0..3usize {
            let blk = &mut m.blocks[b];
            blk.track_idx = 1; // primary on seq 1 (flags&1 clamp)
            blk.cur_lo = 30;
            blk.cur_hi = 400;
            blk.rate = 1.0;
            blk.blend_track_idx = 0; // blend on seq 0 (flags&1==0 ⇒ the WRAP leg, 0x714773)
            blk.blend_lo = 5 + 50 * case;
            blk.blend_hi = 1900;
            blk.blend_rate = 1.0 + 0.25 * case as f32;
            blk.blend_bias = 0;
            blk.trans_end = stime + 500;
            blk.inv_duration = 0.0008;
            blk.blend_amp = 0.9;
        }
        // a linear color/light track so the blend uses these clocks (keeps E2b exercising the
        // element blend too, not just the bone clock).
        let span = 2000u32;
        m.header.color_elems = vec![ColorElemDef {
            vec_track: vec3_track(&mut s, 4, span, 1, 0xffff),
            fix_track: i16_track(&mut s, 4, span, 1, 0xffff),
        }];
        m.color_recs = vec![ColorElemRec::default(); 1];
        m.header.ribbon_elems = vec![RibbonElemDef {
            bone: (case % 3) as u16,
            vec: vec3_track(&mut s, 4, span, 1, 0xffff),
            fix: i16_track(&mut s, 4, span, 1, 0xffff),
            s1: scalar_track(&mut s, 4, span, 1, 0xffff),
            s2: scalar_track(&mut s, 4, span, 1, 0xffff),
            word: word_track_def(&mut s, 4, span, 1, 0xffff),
            byte_track: byte_track(&mut s, 4, span, 1, 0xffff),
        }];
        m.ribbon_recs = vec![RibbonElemRec::default(); 1];
        let args = KernelArgs {
            parent_mtx: core::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { fz(&mut s) * 0.1 }),
            diffuse: [1.0, 1.0, 1.0],
            emissive: [fz(&mut s), fz(&mut s), fz(&mut s)],
            alpha: fz(&mut s).abs() * 40.0,
        };
        run_case(
            &mut emu,
            &[arena_subject],
            &mut [m],
            &args,
            &format!("blendwrap {case}"),
        );
        checked += 1;
    }

    eprintln!(
        "m2_animate: {checked} fixtures bit-exact vs WoW.exe @ 0x714260 \
         (the gates, the bone loop, the element loops and their blend legs, the particle loop, \
         the attachment walk and the exits)"
    );
}
