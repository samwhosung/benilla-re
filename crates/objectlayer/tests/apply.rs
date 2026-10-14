//! The object-update value writers against the exe. `ObjectClearFields` (`0x466c70`) runs whole
//! with its eight cleanup constructors stubbed, and `ObjectApplyValues` (`0x466590`) runs whole
//! with the skip-mirror argument set, so the change-record path through `*(obj+0xc)` (inert
//! without watchers) is left out. The live field array is compared byte for byte, for a unit, an
//! item and a game object, and a player as the active player and as another, with
//! `clear_unset` both ways.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::apply::{apply_update_fields, zero_descriptor_block};
use common::make_read_store;

const ZERO_DESC: u64 = 0x466c70;
const APPLY_UF: u64 = 0x466590;
/// The field count of each type id (`0xb4143c`, zero-filled at load) that the apply loop reads.
const FIELD_COUNTS_GLOBAL: u64 = 0x00b4_143c;
/// The count `CGPlayer_C::SetActivePlayerExtra` (`0x466de0`) fetches, run for real.
const DYNFIELD_GETTER: u64 = 0x47d540;
/// The object manager pointer that `ClntObjMgrGetActivePlayer` (`0x468550`) reads.
const MGR_PTR_GLOBAL: u64 = 0x00b4_1414;
/// The eight per-class cleanup constructors, stubbed to return (`ret 8`). They preserve `edi` and
/// `ebx`, so the fill pointer and size survive the stub.
const CLEANUP_CTORS: [u64; 8] = [
    0x613980, 0x5d81b0, 0x5d76f0, 0x5fad10, 0x5dd2a0, 0x5f6ef0, 0x5d5100, 0x5d6220,
];

fn install_mgr(e: &mut Emu, self_guid: u64) {
    let mgr = e.alloc(0xe0);
    e.write_u32(mgr + 0xc0, self_guid as u32);
    e.write_u32(mgr + 0xc4, (self_guid >> 32) as u32);
    e.write_u32(MGR_PTR_GLOBAL, mgr as u32);
}

#[test]
fn zero_descriptor_block_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for &c in &CLEANUP_CTORS {
        e.stub(c, 0, 2);
    }
    let self_guid = 0x1122_3344_5566_7788u64;
    install_mgr(&mut e, self_guid);
    let count = e.call(DYNFIELD_GETTER, 0, 0, &[]);

    const OBJSZ: usize = 0x3200; // past the player block, 0x1d70 + 0x1408 = 0x3178
    let obj_va = e.alloc(OBJSZ as u64);
    // A mostly nonzero pattern, so the zero fill shows wherever it lands.
    let pattern: Vec<u8> = (0..OBJSZ)
        .map(|i| (i as u8).wrapping_mul(31) ^ 0x5a)
        .collect();

    // (type id, guid): a player as the active player and as another, and 8, which does nothing.
    let cases: &[(u32, u64)] = &[
        (0, 0x0123_4567_89ab_cdef),
        (1, 0x0123_4567_89ab_cdef),
        (2, 0x0123_4567_89ab_cdef),
        (3, 0x0123_4567_89ab_cdef),
        (4, self_guid),             // player, self
        (4, 0x9999_8888_7777_6666), // player, other
        (5, 0x0123_4567_89ab_cdef),
        (6, 0x0123_4567_89ab_cdef),
        (7, 0x0123_4567_89ab_cdef),
        (8, 0x0123_4567_89ab_cdef), // > 7 → no-op
    ];
    for &(type_id, guid) in cases {
        e.write(obj_va, &pattern);
        let self_player = guid == self_guid;
        e.call(
            ZERO_DESC,
            type_id,
            obj_va as u32,
            &[guid as u32, (guid >> 32) as u32],
        );
        let bin_img = e.read(obj_va, OBJSZ);

        let mut our_img = pattern.clone();
        zero_descriptor_block(&mut our_img, obj_va as u32, type_id, self_player, count);

        assert_eq!(
            bin_img, our_img,
            "zero_descriptor_block typeId {type_id} self {self_player}"
        );
    }
}

/// Writes the field counts the manager constructor stores at load.
fn write_field_counts(e: &mut Emu) {
    for (i, c) in [6u32, 0x30, 0x7a, 0xbc, 0x502, 0x1a, 0x10, 0x26]
        .iter()
        .enumerate()
    {
        e.write_u32(FIELD_COUNTS_GLOBAL + 4 * i as u64, *c);
    }
}

#[test]
fn apply_update_fields_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let self_guid = 0x1122_3344_5566_7788u64;
    install_mgr(&mut e, self_guid);
    write_field_counts(&mut e);

    const LIVE: usize = 0x1410; // past the active player's 1282 fields
    let live_va = e.alloc(LIVE as u64);
    let mirror_va = e.alloc(LIVE as u64);
    let obj_va = e.alloc(0x20);
    e.write_u32(obj_va + 8, live_va as u32);
    e.write_u32(obj_va + 0xc, mirror_va as u32);
    e.write_u32(obj_va + 0x14, 3); // type id, read only by the change-record path

    let other = 0x9999_8888_7777_6666u64;
    // (TYPEMASK, object GUID, field count)
    let cases: &[(u32, u64, u32)] = &[
        (9, other, 0xbc),         // UNIT, 188
        (0x19, self_guid, 0x502), // PLAYER self, 1282
        (0x19, other, 0x1e6),     // PLAYER other, 486
        (3, other, 0x30),         // ITEM, 48
        (0x21, other, 0x1a),      // GAMEOBJECT, 26
    ];

    let test_bit = |mask: &[u32], idx: u32| (1u32 << (idx & 0x1f)) & mask[(idx >> 5) as usize];

    for &(typemask, guid, n) in cases {
        let mask_dwords = n.div_ceil(32) as usize;
        let mask: Vec<u32> = (0..mask_dwords)
            .map(|i| 0x9E37_79B9u32.wrapping_mul(i as u32 + 1)) // about half the bits set
            .collect();

        // A nonzero pattern, so a field left alone shows, with the guid at +0 and the type mask
        // at +8 that the kernel reads.
        let mut init = vec![0u8; LIVE];
        for (i, b) in init.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(37) ^ 0x3c;
        }
        init[0..8].copy_from_slice(&guid.to_le_bytes());
        init[8..12].copy_from_slice(&typemask.to_le_bytes());

        // The wire: a byte count, the mask, then a dword per set field in index order.
        let mut wire = vec![mask_dwords as u8];
        for m in &mask {
            wire.extend_from_slice(&m.to_le_bytes());
        }
        let mut ctr = 0u32;
        for idx in 0..n {
            if test_bit(&mask, idx) != 0 {
                wire.extend_from_slice(&(0xB000_0000 | ctr).to_le_bytes());
                ctr += 1;
            }
        }

        // The skip-mirror argument is 1, which bypasses the change-record path; both clear states run.
        for &clear_unset in &[0u32, 1] {
            e.write(live_va, &init);
            let (store_va, _) = make_read_store(&mut e, &wire);
            e.call(
                APPLY_UF,
                obj_va as u32,
                store_va as u32,
                &[guid as u32, (guid >> 32) as u32, 1, clear_unset],
            );
            let bin_live = e.read(live_va, n as usize * 4);

            let mut our_live = init.clone();
            apply_update_fields(
                &mut our_live,
                &wire,
                typemask,
                guid,
                self_guid,
                clear_unset != 0,
            );

            assert_eq!(
                &bin_live[..],
                &our_live[..n as usize * 4],
                "apply_update_fields tm {typemask:#x} clearUnset {clear_unset}"
            );
        }
    }
}
