//! The spell visuals' rates, timings and heights (`systems/spell/visuals.md`).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_spell::visual::{
    spellvisual_attach_height, spellvisual_chain_duration, spellvisual_emitter_active,
    spellvisual_fade_clock, spellvisual_intensity, spellvisual_phase, spellvisual_quality_scale,
    spellvisual_timeline,
};
use common::*;

/// The step of `BlizzardObject::Initialize` from `0x6eb95e` to the `push 0x104` at `0x6eb995`, and the
/// cached `spellEffectLevel` record pointer.
const QUALITY_SCALE_BEGIN: u64 = 0x6eb95e;
const QUALITY_SCALE_END: u64 = 0x6eb995;
const QUALITY_CVAR: u64 = 0x00ce_e9a8;
/// `BlizzardObject::Update`, and the frame time it reads.
const EMITTER_FN: u64 = 0x6ebad0;
const FRAME_DT: u64 = 0x00c6_2510;
/// `GetLightningCasterPoint`, and `CM2Model::HasEvent`, which looks up the `$CSL` event.
const ATTACH_FN: u64 = 0x6ec6f0;
const ATTACH_LOOKUP: u64 = 0x0071_30e0;
/// The level computation of `UpdateSpellScreenTint`, to the `mov eax,[ebp-4]` at `0x6ec8f1` that all
/// four branches reach.
const INTENSITY_BEGIN: u64 = 0x6ec879;
const INTENSITY_END: u64 = 0x6ec8f1;
/// `StartSpellScreenTint`, and its calls of `GetSpellCastTime` (`0x6e3340`) and `OsGetAsyncTimeMs`
/// (`0x42c010`), each 5 bytes, which the test overwrites. The five words it stores.
const TIMELINE_FN: u64 = 0x6ec910;
const TL_GETCASTTIME_CALL: u64 = 0x6ec922;
const TL_GETTICK_CALL: u64 = 0x6ec930;
const TL_G: [u64; 5] = [
    0x00ce_e8d4,
    0x00ce_e8d8,
    0x00ce_e8dc,
    0x00ce_e8e0,
    0x00ce_e8e4,
];

/// `mov eax,[slot]`, 5 bytes.
fn mov_eax_abs(slot: u64) -> [u8; 5] {
    let a = (slot as u32).to_le_bytes();
    [0xa1, a[0], a[1], a[2], a[3]]
}

/// The seconds computation of `SpellVisualsUpdate`, `0x6eca8f` to `0x6ecab3`, and the last time it keeps.
const FADE_BEGIN: u64 = 0x6eca8f;
const FADE_END: u64 = 0x6ecab3;
const LAST_TICK: u64 = 0x0087_0e20;
/// The life computation of `CreateLightningObjects`, to the `call 0x58a980` at `0x6ecd79`.
const CHAIN_BEGIN: u64 = 0x6ecd61;
const CHAIN_END: u64 = 0x6ecd79;

/// Runs the rate scaling of `BlizzardObject::Initialize` (`0x6eb930`), `0x6eb95e` to `0x6eb995`, with no
/// `spellEffectLevel` record and with its value at `+0x28` set to 0 to 3, and reads the rate stored
/// at `this + 0x124`.
#[test]
fn spellvisual_quality_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x200); // esi: the emitter, the rate stored at +0x124
    let frame = e.alloc(0x40) + 0x20; // ebp: the rate argument at [ebp+0x14]
    let cvar = e.alloc(0x40); // the spellEffectLevel CVar record

    let mut s: u32 = 0x6eb9_0001;
    for i in 0..8000u32 {
        let scale = fuzz_unit(&mut s, 8.0);
        // Mode 0 is no record; modes 1 to 4 set the record's +0x28 to 0 to 3.
        let mode = (next(&mut s) >> 8) % 5;
        let quality = if mode == 0 {
            e.write_u32(QUALITY_CVAR, 0);
            None
        } else {
            let q = (mode - 1) as i32;
            e.write_u32(QUALITY_CVAR, cvar as u32);
            e.write_u32(cvar + 0x28, q as u32);
            Some(q)
        };
        e.write_u32(frame + 0x14, scale.to_bits());
        e.write_u32(obj + 0x124, 0xdead_beef);
        e.set_reg(RegisterX86::ESI, obj);
        e.set_reg(RegisterX86::EBP, frame);
        e.run(QUALITY_SCALE_BEGIN, QUALITY_SCALE_END);
        let bin = e.read_f32(obj + 0x124);
        assert_f32(
            bin,
            spellvisual_quality_scale(quality, scale),
            &format!("spellvisual_quality_scale[{i}] q={quality:?} scale={scale}"),
        );
    }
}

/// Runs `BlizzardObject::Update` (`0x6ebad0`) whole on inputs where `dt · rate + phase` stays below
/// 1.0, so that no shard spawns: the phase stored at `+0x120` and the returned flag are compared.
#[test]
fn spellvisual_emitter_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x200); // ecx: the phase at +0x120, the rate at +0x124, the list head at +0x140

    let mut s: u32 = 0x6ead_0001;
    for i in 0..8000u32 {
        let phase = fuzz_unit(&mut s, 0.4);
        let rate = match (next(&mut s) >> 8) % 4 {
            0 => 0.0f32, // a zero rate: the list head decides
            _ => fuzz_unit(&mut s, 2.0),
        };
        let dt = fuzz_unit(&mut s, 0.1); // dt · rate is under 0.2, so the sum is under 0.6: no shard
        let list_head = match (next(&mut s) >> 8) % 4 {
            0 => 0u32,
            1 => 1u32,           // the tag bit: an empty list
            2 => 0x1234_0000u32, // nonzero, tag bit clear
            _ => 0x1234_0001u32, // nonzero, tag bit set
        };
        e.write_u32(obj + 0x120, phase.to_bits());
        e.write_u32(obj + 0x124, rate.to_bits());
        e.write_u32(obj + 0x140, list_head);
        e.write_u32(FRAME_DT, dt.to_bits());
        let eax = e.call(EMITTER_FN, obj as u32, 0, &[]);
        let phase_bin = e.read_f32(obj + 0x120);
        assert_f32(
            phase_bin,
            spellvisual_phase(phase, rate, dt),
            &format!("spellvisual_phase[{i}] phase={phase} rate={rate} dt={dt}"),
        );
        let active_bin = eax != 0;
        let active_ours = spellvisual_emitter_active(rate, list_head);
        assert_eq!(
            active_bin, active_ours,
            "spellvisual_emitter_active[{i}] rate={rate} list={list_head:#x}: bin={active_bin} ours={active_ours}"
        );
    }
}

/// Runs `GetLightningCasterPoint` (`0x6ec6f0`) whole on a model with an attachment manager, with
/// `CM2Model::HasEvent` (`0x7130e0`) stubbed to report no `$CSL` event and the model's `GetPosition`
/// slot stubbed to return a chosen position.
#[test]
fn spellvisual_attach_height_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let model = e.alloc(0x100); // edx: the height at +0x90, the scale at +0xbc, the attachments at +0xd8
    let vtbl = e.alloc(0x40);
    let island = e.alloc(0x10); // the model's GetPosition, stubbed
    let base_pos = e.alloc(12);
    let out = e.alloc(12); // ecx: the output vector
    e.write_u32(model, vtbl as u32);
    e.write_u32(vtbl + 0x14, island as u32);
    e.write_u32(model + 0xd8, 1); // attachments present, so the `$CSL` lookup runs
    e.stub(island, base_pos as u32, 1); // GetPosition(out) returns the position, pops 4
    e.stub(ATTACH_LOOKUP, 0, 1); // the `$CSL` event is absent, so the default height applies; pops 4

    let mut s: u32 = 0x6ec6_0001;
    for i in 0..8000u32 {
        let height = fuzz_unit(&mut s, 6.0);
        let scale = fuzz_unit(&mut s, 3.0);
        let base = [
            fuzz_unit(&mut s, 200.0) - 100.0,
            fuzz_unit(&mut s, 200.0) - 100.0,
            fuzz_unit(&mut s, 100.0) - 50.0,
        ];
        e.write_u32(model + 0x90, height.to_bits());
        e.write_u32(model + 0xbc, scale.to_bits());
        write_vec3(&mut e, base_pos, base);
        e.write(out, &[0u8; 12]);
        e.call(ATTACH_FN, out as u32, model as u32, &[]);
        let bin = [e.read_f32(out), e.read_f32(out + 4), e.read_f32(out + 8)];
        let ours = spellvisual_attach_height(height, scale, base);
        assert_f32(bin[0], ours[0], &format!("attach.x[{i}]"));
        assert_f32(bin[1], ours[1], &format!("attach.y[{i}]"));
        assert_f32(
            bin[2],
            ours[2],
            &format!("attach.z[{i}] h={height} s={scale} base={base:?}"),
        );
    }
}

/// Runs the level computation of `UpdateSpellScreenTint` (`0x6ec870`), `0x6ec879` to `0x6ec8f1`, with
/// the four times at `+0x4` to `+0x10` of the tint and the clock at `[ebp+8]`, on clocks at and
/// around each boundary, and reads the level at `[ebp-4]`. The times are distinct, since the
/// function returns before the slice when start equals end.
#[test]
fn spellvisual_intensity_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let timeline = e.alloc(0x20); // esi: the times at +0x4, +0x8, +0xc, +0x10
    let frame = e.alloc(0x80) + 0x40; // ebp: the clock at [ebp+8], the level at [ebp-4]
    let stack = e.alloc(0x40) + 0x20; // esp, for the slice's `push edi`

    let mut s: u32 = 0x6ec8_0001;
    for i in 0..8000u32 {
        let start = (next(&mut s) >> 4) | 1; // nonzero, well below 2^31
        let mid1 = start.wrapping_add((next(&mut s) >> 16) % 4000 + 1);
        let mid2 = mid1.wrapping_add((next(&mut s) >> 16) % 4000 + 1);
        let end = mid2.wrapping_add((next(&mut s) >> 16) % 4000 + 1);
        let span = end.wrapping_sub(start);
        let now = match (next(&mut s) >> 8) % 8 {
            0 => start, // exactly at a boundary, where the compares decide
            1 => mid1,
            2 => mid2,
            3 => end,
            _ => start.wrapping_add((next(&mut s) >> 8) % (span + span / 4 + 2)),
        };
        e.write_u32(timeline + 0x4, start);
        e.write_u32(timeline + 0x8, mid1);
        e.write_u32(timeline + 0xc, mid2);
        e.write_u32(timeline + 0x10, end);
        e.write_u32(frame + 8, now);
        e.write_u32(frame - 4, 0xdead_beef);
        e.set_reg(RegisterX86::ESI, timeline);
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::ESP, stack);
        e.run(INTENSITY_BEGIN, INTENSITY_END);
        let bin = e.read_f32(frame - 4);
        let ours = spellvisual_intensity(start, mid1, mid2, end, now);
        assert_f32(
            bin,
            ours,
            &format!("spellvisual_intensity[{i}] s={start} m1={mid1} m2={mid2} e={end} now={now}"),
        );
    }
}

/// Runs `StartSpellScreenTint` (`0x6ec910`) whole. Its calls of `GetSpellCastTime` and
/// `OsGetAsyncTimeMs` are overwritten by loads of chosen values, once, since changing code that Unicorn
/// has translated does not take; `__ftol` runs for real. The five words stored at `0xcee8d4` are compared.
#[test]
fn spellvisual_timeline_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ptr = e.alloc(0x10); // ecx: the word at [ptr] is copied to 0xcee8d4
    let ct_slot = e.alloc(4);
    let now_slot = e.alloc(4);
    e.write(TL_GETCASTTIME_CALL, &mov_eax_abs(ct_slot));
    e.write(TL_GETTICK_CALL, &mov_eax_abs(now_slot));

    let mut s: u32 = 0x6ec9_0001;
    // Fractions whose product with the cast time is NaN, infinite, past `i32` or past `i64`:
    // `__ftol` returns the low dword of its qword, 0 for the indefinite.
    const WIDE: [f32; 9] = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0e30,
        -1.0e30,
        1.0e5,
        -1.0e5,
        4.0e4,
        1.0e15,
    ];
    for i in 0..8000u32 + WIDE.len() as u32 {
        let spell_id_field = next(&mut s);
        let mut cast_time = ((next(&mut s) >> 12) % 120_001) as i32; // 0 to 120000 ms
        let now = next(&mut s);
        let mut fraction = match (next(&mut s) >> 8) % 3 {
            0 => fuzz_unit(&mut s, 1.0),
            1 => fuzz_unit(&mut s, 1.5),
            _ => 0.0f32,
        };
        if let Some(&f) = WIDE.get((i as usize).wrapping_sub(8000)) {
            (cast_time, fraction) = (120_000, f);
        }
        e.write_u32(ptr, spell_id_field);
        e.write_u32(ct_slot, cast_time as u32);
        e.write_u32(now_slot, now);
        e.call(TIMELINE_FN, ptr as u32, 0, &[fraction.to_bits()]);
        let bin = [
            e.read_u32(TL_G[0]),
            e.read_u32(TL_G[1]),
            e.read_u32(TL_G[2]),
            e.read_u32(TL_G[3]),
            e.read_u32(TL_G[4]),
        ];
        let ours = spellvisual_timeline(spell_id_field, cast_time, now, fraction);
        assert_eq!(
            bin, ours,
            "spellvisual_timeline[{i}] sid={spell_id_field:#x} ct={cast_time} now={now:#x} frac={fraction}"
        );
    }
}

/// Runs the seconds computation of `SpellVisualsUpdate` (`0x6eca20`), `0x6eca8f` to `0x6ecab3`, with
/// the clock in `edi` and the last time at `0x870e20`, which includes -1, and reads the value at `[ebp-4]`.
#[test]
fn spellvisual_fade_clock_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40) + 0x20; // ebp: the result at [ebp-4]

    let mut s: u32 = 0x6eca_0001;
    for i in 0..8000u32 {
        let last = match (next(&mut s) >> 8) % 4 {
            0 => 0xffff_ffffu32, // the -1 sentinel: 0.0
            _ => next(&mut s),
        };
        let now = match (next(&mut s) >> 8) % 4 {
            0 => last.wrapping_sub((next(&mut s) >> 16) % 100), // a negative difference
            _ => last.wrapping_add((next(&mut s) >> 16) % 100), // a frame difference
        };
        e.write_u32(LAST_TICK, last);
        e.write_u32(frame - 4, 0xdead_beef);
        e.set_reg(RegisterX86::EDI, u64::from(now));
        e.set_reg(RegisterX86::EBP, frame);
        e.run(FADE_BEGIN, FADE_END);
        let bin = e.read_f32(frame - 4);
        let ours = spellvisual_fade_clock(now, last);
        assert_f32(
            bin,
            ours,
            &format!("spellvisual_fade_clock[{i}] now={now:#x} last={last:#x}"),
        );
    }
}

/// Runs the life computation of `CreateLightningObjects` (`0x6ecbd0`), `0x6ecd61` to `0x6ecd79`,
/// with the duration at `+0x14` of the record, including negatives, and reads the float at `+0x3c`
/// of the chain.
#[test]
fn spellvisual_chain_duration_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let record = e.alloc(0x40); // edi: the duration at +0x14
    let seg = e.alloc(0x60); // esi: the chain, the life stored at +0x3c
    let frame = e.alloc(0x40) + 0x20; // ebp, for a `lea [ebp-0x18]`
    let stack = e.alloc(0x40) + 0x20; // esp, for three pushes

    let mut s: u32 = 0x6ecd_0001;
    for i in 0..8000u32 {
        let dur_ms = match (next(&mut s) >> 8) % 3 {
            0 => ((next(&mut s) >> 12) % 60_001) as i32, // 0 to 60000 ms
            1 => -(((next(&mut s) >> 12) % 60_001) as i32), // negative: `fild dword` is signed
            _ => ((next(&mut s) >> 8) % 2000) as i32,    // small
        };
        e.write_u32(record + 0x14, dur_ms as u32);
        e.write_u32(seg + 0x3c, 0xdead_beef);
        e.set_reg(RegisterX86::EDI, record);
        e.set_reg(RegisterX86::ESI, seg);
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::ESP, stack);
        e.set_reg(RegisterX86::EAX, 0);
        e.run(CHAIN_BEGIN, CHAIN_END);
        let bin = e.read_f32(seg + 0x3c);
        let ours = spellvisual_chain_duration(dur_ms);
        assert_f32(
            bin,
            ours,
            &format!("spellvisual_chain_duration[{i}] dur_ms={dur_ms}"),
        );
    }
}
