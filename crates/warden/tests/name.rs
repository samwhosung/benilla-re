//! The loop `0x6ca3e7` to `0x6ca401` that builds the name the module signature covers: a slice
//! of `0x6ca360`, run on tables the test writes at `0x811321`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_warden::{signature_name, NAME_TABLE_VA};
use common::Rng;

const SLICE_START: u64 = 0x6ca3e7;
const SLICE_END: u64 = 0x6ca403;

fn run(emu: &mut Emu, frame: u64, table: &[u8]) -> Vec<u8> {
    emu.write(u64::from(NAME_TABLE_VA), table);
    emu.write(frame - 0x40, &[0xee; 0x80]);
    emu.set_reg(RegisterX86::EBP, frame);
    emu.run(SLICE_START, SLICE_END);
    let mut out = Vec::new();
    let mut p = frame - 0x1c;
    loop {
        let b = emu.read(p, 1)[0];
        out.push(b);
        if b == 0 {
            return out;
        }
        p += 1;
    }
}

#[test]
fn signature_name_loop() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100;
    let original = emu.read(u64::from(NAME_TABLE_VA), 0x20);

    // The exe's own table.
    let got = run(&mut emu, frame, &original);
    let mut want = signature_name(&original);
    want.push(0);
    assert_eq!(got, want);
    assert_eq!(&got[..9], b"Maiev.mod");

    // Every first byte, alone and followed by the terminating zero product.
    for b in 0..=255u8 {
        let mut table = vec![b, 0, 0x55, 0x55];
        table.resize(0x20, 0x77);
        let got = run(&mut emu, frame, &table);
        let mut want = signature_name(&table);
        want.push(0);
        assert_eq!(got, want, "first byte {b:#x}");
    }

    // Random tables; each ends where a byte is zero.
    let mut rng = Rng(0x6ca3_e7e7);
    for case in 0..2000 {
        let n = 1 + rng.below(30) as usize;
        let mut table: Vec<u8> = (0..n).map(|_| 1 + rng.below(255) as u8).collect();
        table.push(0);
        table.resize(0x20, 0x33);
        let got = run(&mut emu, frame, &table);
        let mut want = signature_name(&table);
        want.push(0);
        assert_eq!(got, want, "case {case}: {table:02x?}");
    }

    emu.write(u64::from(NAME_TABLE_VA), &original);
}
