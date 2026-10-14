//! The five track builders `0x6ce120`, `0x6ce390`, `0x6ce4a0`, `0x6ce560` and `0x6ce670` whole,
//! against the `(address, bits)` lists of `dn_init`. Each global is poisoned first, so a store the
//! function misses fails; the result is read back and compared bit for bit.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::dn_init::{
    dn_init_6ce120, dn_init_6ce390, dn_init_6ce4a0, dn_init_6ce560, dn_init_6ce670,
};

/// Runs the builder at `addr` and compares every global it writes.
fn check(addr: u64, expected: &[(u64, u32)]) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    for &(va, _) in expected {
        emu.write_u32(va, 0xdead_beef);
    }
    // `__cdecl`, no arguments.
    emu.call(addr, 0, 0, &[]);
    for &(va, bits) in expected {
        let got = emu.read_u32(va);
        assert_eq!(got, bits, "dn_init {addr:#08x} @ {va:#08x}");
    }
}

#[test]
fn dn_init_6ce120_difftest() {
    check(0x6ce120, &dn_init_6ce120());
}

#[test]
fn dn_init_6ce390_difftest() {
    check(0x6ce390, &dn_init_6ce390());
}

#[test]
fn dn_init_6ce4a0_difftest() {
    check(0x6ce4a0, &dn_init_6ce4a0());
}

#[test]
fn dn_init_6ce560_difftest() {
    check(0x6ce560, &dn_init_6ce560());
}

#[test]
fn dn_init_6ce670_difftest() {
    check(0x6ce670, &dn_init_6ce670());
}
