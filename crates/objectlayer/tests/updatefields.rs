//! The field geometry of an object against the exe, each function run whole: `UpdateMaskTestBit`
//! (`0x465550`) over every bit of a 41-dword mask, `GetDataBaseOffset` (`0x465690`) over its
//! block ids, `NextFieldSection` (`0x4656e0`) over the type masks and sub-blocks,
//! `ObjectWatcherList` (`0x465850`) over the sections and indices around `UNIT_END`,
//! `ObjectFieldCount` (`0x465a80`) for the active player and another, and
//! `CGPlayer_C::SetActivePlayerExtra` (`0x466de0`).

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::updatefields::{
    descriptor_section, player_dynfield_base, updateblock_stride, updatefield_addr,
    updatefield_count, updatemask_test_bit,
};
use common::next;

const TEST_BIT: u64 = 0x465550;
const STRIDE: u64 = 0x465690;
const SECTION: u64 = 0x4656e0;
const ADDR: u64 = 0x465850;
const COUNT: u64 = 0x465a80;
const DYNBASE: u64 = 0x466de0;
/// The count `SetActivePlayerExtra` fetches, run for real.
const DYNFIELD_GETTER: u64 = 0x47d540;
/// The object manager pointer that `ClntObjMgrGetActivePlayer` (`0x468550`) reads.
const MGR_PTR_GLOBAL: u64 = 0x00b4_1414;

/// The eight type masks (object 1, item 3, container 7, unit 9, player 0x19, game object 0x21,
/// dynamic object 0x41, corpse 0x81) and some unhandled values for the default branches.
const TYPEMASKS: [u32; 12] = [0, 1, 3, 5, 7, 9, 0x19, 0x21, 0x41, 0x81, 0xa5, 0xff];

/// Installs an object manager whose active-player guid (`+0xc0`, `+0xc4`) is `self_guid`, so
/// `0x468550` returns it. Returns the manager address.
fn install_mgr(e: &mut Emu, self_guid: u64) -> u64 {
    let mgr = e.alloc(0xe0);
    e.write_u32(mgr + 0xc0, self_guid as u32);
    e.write_u32(mgr + 0xc4, (self_guid >> 32) as u32);
    e.write_u32(MGR_PTR_GLOBAL, mgr as u32);
    mgr
}

#[test]
fn updatemask_test_bit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut seed = 0x1234_5678u32;
    const NW: usize = 41; // the client's mask buffer: 41 dwords, 1312 bits
    let mut mask = [0u32; NW];
    for m in mask.iter_mut() {
        *m = next(&mut seed);
    }
    let mask_va = e.alloc((NW * 4) as u64);
    for (i, m) in mask.iter().enumerate() {
        e.write_u32(mask_va + 4 * i as u64, *m);
    }
    for idx in 0..(NW as u32 * 32) {
        let bin = e.call(TEST_BIT, mask_va as u32, idx, &[]);
        assert_eq!(bin, updatemask_test_bit(&mask, idx), "test_bit idx {idx}");
    }
}

#[test]
fn updateblock_stride_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // Every block id the switch handles and values past its range.
    for block_id in (0u32..12).chain([0x20, 0x100, 0xffff_ffff]) {
        let bin = e.call(STRIDE, block_id, 0, &[]);
        assert_eq!(bin, updateblock_stride(block_id), "stride {block_id:#x}");
    }
}

#[test]
fn descriptor_section_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x10);
    let rec = e.alloc(0x10);
    e.write_u32(obj + 8, rec as u32);
    for &tm in &TYPEMASKS {
        e.write_u32(rec + 8, tm);
        for sub in 0u32..6 {
            let bin = e.call(SECTION, obj as u32, sub, &[]);
            assert_eq!(
                bin,
                descriptor_section(tm, sub),
                "section tm {tm:#x} sub {sub}"
            );
        }
    }
}

#[test]
fn updatefield_addr_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let self_guid = 0xAABB_CCDD_1122_3344u64;
    install_mgr(&mut e, self_guid);
    let obj = e.alloc(0x2000); // covers the spill pointer at +0x1c68
    let rec = e.alloc(0x10);
    let spill = e.alloc(0x1000);
    e.write_u32(obj + 8, rec as u32);
    e.write_u32(obj + 0x1c68, spill as u32);
    // Indices across `UNIT_END` (0x12a), where a player's fields go from inline to the spill.
    let indices = [0u32, 1, 2, 5, 0x11, 0x80, 0x129, 0x12a, 0x12b, 0x200];
    for &guid in &[self_guid, 0x1111_2222_3333_4444u64] {
        e.write_u32(rec, guid as u32);
        e.write_u32(rec + 4, (guid >> 32) as u32);
        for section in 0u32..9 {
            for &index in &indices {
                let bin = e.call(ADDR, obj as u32, section, &[index]);
                let ours =
                    updatefield_addr(obj as u32, section, index, guid, self_guid, spill as u32);
                assert_eq!(bin, ours, "addr s{section} i{index:#x} guid {guid:#x}");
            }
        }
    }
}

#[test]
fn updatefield_count_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let self_guid = 0x0102_0304_0506_0708u64;
    install_mgr(&mut e, self_guid);
    for &tm in &TYPEMASKS {
        for &guid in &[self_guid, 0x9999_8888_7777_6666u64] {
            let bin = e.call(COUNT, tm, 0, &[guid as u32, (guid >> 32) as u32]);
            let ours = updatefield_count(tm, guid, self_guid);
            assert_eq!(bin, ours, "count tm {tm:#x} guid {guid:#x}");
        }
    }
}

#[test]
fn player_dynfield_base_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // The count from the getter chain `0x47d540` to `0x47db10`, run for real.
    let k = e.call(DYNFIELD_GETTER, 0, 0, &[]);
    let obj = e.alloc(0x4000);
    e.write_u32(obj + 0x1c68, 0xDEAD_BEEF); // a poison value, so the store shows
    e.call(DYNBASE, obj as u32, 0, &[]);
    let bin = e.read_u32(obj + 0x1c68);
    assert_eq!(
        bin,
        player_dynfield_base(obj as u32, k),
        "dynfield base k={k:#x}"
    );
}
