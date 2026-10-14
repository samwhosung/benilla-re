//! The movement block codec against the exe. `ReadMovementBlock` (`0x47f050`),
//! `SkipMovementBlock` (`0x47ede0`), `SkipMovementStatus` (`0x47eb10`) and
//! `CMovementStatus::Write` (`0x47ecb0`) run whole on synthesized wire streams, and the output
//! struct image, the stream cursor, the returned flags, the written bytes and the spline handoff
//! arguments are compared.
//!
//! Every flag gate runs: all 16 subsets of the four status gates, alone and with the spline
//! enabled over all 8 sub-flag subsets and point counts 0, 1 and 3. A truncation sweep then cuts a
//! full stream at every 4-byte boundary to check the error latch.
//!
//! One word is masked: on a stream cut before the spline time-passed dword, the reader subtracts
//! an uninitialized stack local from the tick into `+0x84`, so that dword is left out of the
//! compare in exactly those cases.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::datastore::{DataStore, DataStoreW};
use benilla_re_objectlayer::movement::{
    movement_block_decode, movement_block_encode, movement_block_skip, movement_info_size,
    MovementInfo, FLAG_FALLING, FLAG_ONTRANSPORT, FLAG_SPLINE_ELEVATION, FLAG_SPLINE_ENABLED,
    FLAG_SWIMMING, MOVEMENT_INFO_SIZE, SPLINE_FINAL_ANGLE, SPLINE_FINAL_POINT, SPLINE_FINAL_TARGET,
};
use common::{make_read_store, make_write_store, next};

const DECODE: u64 = 0x47f050;
const SKIP: u64 = 0x47ede0;
const ENCODE: u64 = 0x47ecb0;
const INFO_SIZE: u64 = 0x47eb10;
/// The clock `OsGetAsyncTimeMsBody` the reader subtracts from, stubbed to a fixed value.
const TICK_FN: u64 = 0x42b790;
const TICK: u32 = 0x1234_5678;
/// `SetSplinePath`, which the reader hands the points to, stubbed with its arguments recorded.
const SPLINE_HANDOFF_FN: u64 = 0x453360;

const BASE_GATES: [u32; 4] = [
    FLAG_ONTRANSPORT,
    FLAG_SWIMMING,
    FLAG_FALLING,
    FLAG_SPLINE_ELEVATION,
];
const SUB_GATES: [u32; 3] = [SPLINE_FINAL_POINT, SPLINE_FINAL_TARGET, SPLINE_FINAL_ANGLE];

/// One case: the wire bytes and the stream offset of the spline time-passed dword, for the
/// truncation mask, when the stream has a spline block.
struct Case {
    wire: Vec<u8>,
    time_passed_off: Option<usize>,
}

/// Assembles a wire stream for the given gates in the layout `movement.rs` documents.
fn build_case(flags: u32, sub: u32, count: u32, seed: &mut u32) -> Case {
    let mut w: Vec<u8> = Vec::new();
    let pu32 = |w: &mut Vec<u8>, v: u32| w.extend_from_slice(&v.to_le_bytes());
    let f = |w: &mut Vec<u8>, s: &mut u32| {
        let v = next(s);
        w.extend_from_slice(&v.to_le_bytes());
    };
    pu32(&mut w, flags);
    f(&mut w, seed); // time
    for _ in 0..4 {
        f(&mut w, seed); // pos x,y,z,o
    }
    if flags & FLAG_ONTRANSPORT != 0 {
        f(&mut w, seed);
        f(&mut w, seed); // guid lo/hi
        for _ in 0..4 {
            f(&mut w, seed); // transport off
        }
    }
    if flags & FLAG_SWIMMING != 0 {
        f(&mut w, seed); // pitch
    }
    f(&mut w, seed); // fall_time
    if flags & FLAG_FALLING != 0 {
        for _ in 0..4 {
            f(&mut w, seed);
        }
    }
    if flags & FLAG_SPLINE_ELEVATION != 0 {
        f(&mut w, seed);
    }
    for _ in 0..6 {
        f(&mut w, seed); // speeds
    }
    let mut time_passed_off = None;
    if flags & FLAG_SPLINE_ENABLED != 0 {
        pu32(&mut w, sub);
        if sub & SPLINE_FINAL_POINT != 0 {
            for _ in 0..3 {
                f(&mut w, seed);
            }
        }
        if sub & SPLINE_FINAL_TARGET != 0 {
            f(&mut w, seed);
            f(&mut w, seed);
        }
        if sub & SPLINE_FINAL_ANGLE != 0 {
            f(&mut w, seed);
        }
        time_passed_off = Some(w.len());
        f(&mut w, seed); // time passed
        f(&mut w, seed); // duration
        f(&mut w, seed); // id
        pu32(&mut w, count);
        for _ in 0..count * 3 {
            f(&mut w, seed); // points
        }
        for _ in 0..3 {
            f(&mut w, seed); // dest
        }
    }
    Case {
        wire: w,
        time_passed_off,
    }
}

/// Every flag-gate combination: 16 status subsets, each without a spline and with one over the 8
/// sub-flag subsets and 3 counts.
fn all_cases(seed: &mut u32) -> Vec<Case> {
    let mut out = Vec::new();
    for base_bits in 0u32..16 {
        let mut flags = 0u32;
        for (i, g) in BASE_GATES.iter().enumerate() {
            if base_bits & (1 << i) != 0 {
                flags |= g;
            }
        }
        // unrelated noise bits must not gate anything
        let noise = (next(seed) & 0x1) | ((next(seed) & 0x1) << 8);
        out.push(build_case(flags | noise, 0, 0, seed));
        for sub_bits in 0u32..8 {
            let mut sub = 0u32;
            for (i, g) in SUB_GATES.iter().enumerate() {
                if sub_bits & (1 << i) != 0 {
                    sub |= g;
                }
            }
            for count in [0u32, 1, 3] {
                out.push(build_case(flags | FLAG_SPLINE_ENABLED, sub, count, seed));
            }
        }
    }
    out
}

fn mv_fill() -> MovementInfo {
    unsafe { std::mem::transmute([0xCCu8; MOVEMENT_INFO_SIZE]) }
}
fn mv_bytes(mv: &MovementInfo) -> Vec<u8> {
    unsafe {
        std::slice::from_raw_parts((mv as *const MovementInfo).cast::<u8>(), MOVEMENT_INFO_SIZE)
    }
    .to_vec()
}

/// Runs one read in the emulator and in the transcription and compares them.
fn check_decode(e: &mut Emu, wire: &[u8], time_passed_off: Option<usize>, ctx: &str) {
    let (store_va, buf_va) = make_read_store(e, wire);
    let mv_va = e.alloc(MOVEMENT_INFO_SIZE as u64);
    e.write(mv_va, &[0xCCu8; MOVEMENT_INFO_SIZE]);
    e.clear_calls();
    e.call(DECODE, store_va as u32, mv_va as u32, &[]);
    let bin_img = e.read(mv_va, MOVEMENT_INFO_SIZE);
    let bin_cursor = e.read_u32(store_va + 0x14);
    let handoff_calls: Vec<[u32; 8]> = e
        .calls()
        .iter()
        .filter(|c| c.addr == SPLINE_HANDOFF_FN)
        .map(|c| c.args)
        .collect();

    let mut ds = DataStore::new(wire);
    let mut mv = mv_fill();
    let pre_points_pos = {
        // run ours
        let handoff = movement_block_decode(&mut ds, &mut mv, TICK);
        handoff.map(|h| (h.count, h.raw_points.len()))
    };
    let mut ours_img = mv_bytes(&mv);
    let mut bin_img = bin_img;

    // The one documented mask: truncated before time-passed ⇒ the binary's +0x84 is
    // tick − <uninit stack local> (unspecified) — exclude that dword from the compare.
    if let Some(off) = time_passed_off {
        if wire.len() < off + 4 {
            for img in [&mut ours_img, &mut bin_img] {
                img[0x84..0x88].fill(0);
            }
        }
    }
    assert_eq!(bin_img, ours_img, "{ctx}: struct image");
    assert_eq!(bin_cursor, ds.pos, "{ctx}: stream cursor");

    // The spline handoff: the binary calls 0x453360(ptr, count) whenever count != 0 (ptr = 0
    // when the point-window skip failed). Mirror from our side's observations.
    match pre_points_pos {
        Some((count, raw_len)) => {
            assert_eq!(handoff_calls.len(), 1, "{ctx}: one handoff");
            assert_eq!(handoff_calls[0][1], count, "{ctx}: handoff count");
            assert_ne!(handoff_calls[0][0], 0, "{ctx}: handoff ptr non-null");
            // the window the binary handed off is the same bytes our skip yielded
            let bin_pts = e.read(u64::from(handoff_calls[0][0]), raw_len);
            let our_start = (handoff_calls[0][0] as u64 - buf_va) as usize;
            assert_eq!(
                bin_pts,
                wire[our_start..our_start + raw_len].to_vec(),
                "{ctx}: handoff window"
            );
        }
        None => {
            // ours saw count==0 or a failed skip; the binary only skips the call for count==0
            for c in &handoff_calls {
                assert_eq!(c[0], 0, "{ctx}: failed-skip handoff carries ptr 0");
            }
        }
    }
}

#[test]
fn movement_block_decode_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    e.stub(TICK_FN, TICK, 0);
    e.stub(SPLINE_HANDOFF_FN, 0, 2);
    let mut seed = 0xd1ff_7e57u32;
    for (i, case) in all_cases(&mut seed).iter().enumerate() {
        check_decode(
            &mut e,
            &case.wire,
            case.time_passed_off,
            &format!("case {i}"),
        );
    }
    // truncation sweep over one full-featured stream, cut at every 4-byte boundary
    let full = build_case(
        FLAG_ONTRANSPORT
            | FLAG_SWIMMING
            | FLAG_FALLING
            | FLAG_SPLINE_ELEVATION
            | FLAG_SPLINE_ENABLED,
        SPLINE_FINAL_POINT | SPLINE_FINAL_TARGET | SPLINE_FINAL_ANGLE,
        2,
        &mut seed,
    );
    for cut in (0..full.wire.len()).step_by(4) {
        check_decode(
            &mut e,
            &full.wire[..cut],
            full.time_passed_off,
            &format!("trunc {cut}"),
        );
    }
}

#[test]
fn movement_block_skip_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let mut seed = 0x51c0_ffeeu32;
    for (i, case) in all_cases(&mut seed).iter().enumerate() {
        let (store_va, _) = make_read_store(&mut e, &case.wire);
        e.call(SKIP, store_va as u32, 0, &[]);
        let bin_cursor = e.read_u32(store_va + 0x14);
        let mut ds = DataStore::new(&case.wire);
        movement_block_skip(&mut ds);
        assert_eq!(bin_cursor, ds.pos, "skip case {i}: cursor");
    }
}

#[test]
fn movement_info_size_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let mut seed = 0x14f0_517eu32;
    for (i, case) in all_cases(&mut seed).iter().enumerate() {
        let (store_va, _) = make_read_store(&mut e, &case.wire);
        let bin_flags = e.call(INFO_SIZE, store_va as u32, 0, &[]);
        let bin_cursor = e.read_u32(store_va + 0x14);
        let mut ds = DataStore::new(&case.wire);
        let our_flags = movement_info_size(&mut ds);
        assert_eq!(bin_flags, our_flags, "size case {i}: flags");
        assert_eq!(bin_cursor, ds.pos, "size case {i}: cursor");
    }
}

#[test]
fn movement_block_encode_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let mut seed = 0xe2c0_de00u32;
    for base_bits in 0u32..16 {
        let mut flags = 0u32;
        for (i, g) in BASE_GATES.iter().enumerate() {
            if base_bits & (1 << i) != 0 {
                flags |= g;
            }
        }
        let mut mv = mv_fill();
        mv.flags = flags;
        mv.time = next(&mut seed);
        for i in 0..4 {
            mv.pos[i] = f32::from_bits(next(&mut seed));
            mv.transport_off[i] = f32::from_bits(next(&mut seed));
            mv.fall[i] = f32::from_bits(next(&mut seed));
        }
        mv.transport_guid = [next(&mut seed), next(&mut seed)];
        mv.pitch = f32::from_bits(next(&mut seed));
        mv.fall_time = next(&mut seed);
        mv.spline_elevation = f32::from_bits(next(&mut seed));

        let (store_va, buf_va) = make_write_store(&mut e, 0x200);
        let mv_va = e.alloc(MOVEMENT_INFO_SIZE as u64);
        e.write(mv_va, &mv_bytes(&mv));
        e.call(ENCODE, store_va as u32, mv_va as u32, &[]);
        let bin_len = e.read_u32(store_va + 0x10);
        let bin_out = e.read(buf_va, bin_len as usize);

        let mut w = DataStoreW::default();
        movement_block_encode(&mut w, &mv);
        assert_eq!(bin_out, w.buf, "encode {base_bits:#x}: bytes");
        assert_eq!(bin_len as usize, w.buf.len(), "encode {base_bits:#x}: len");
    }
}
