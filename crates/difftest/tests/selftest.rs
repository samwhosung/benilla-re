//! The harness's own checks, each shown to fire: a check that cannot fail proves nothing. Each test
//! runs a small island of code in scratch; no client function is involved, but `Emu::new` maps the
//! exe, so these skip without it.

use benilla_re_difftest::{Emu, MemType, Wow};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// An `ldmxcsr` that sets a control bit (FTZ, `0x9F80`) must fail the run.
#[test]
fn mxcsr_control_change_is_caught() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let bad = emu.alloc(0x10);
    emu.write_u32(bad, 0x9F80);

    // ldmxcsr [bad]: 0F AE /2 with a disp32 operand (modrm 0x15), 7 bytes, then a nop to halt on.
    let island = emu.alloc(0x20);
    let mut code = vec![0x0Fu8, 0xAE, 0x15];
    code.extend_from_slice(&(bad as u32).to_le_bytes());
    code.push(0x90);
    emu.write(island, &code);

    let caught = catch_unwind(AssertUnwindSafe(|| {
        emu.run(island, island + 7);
    }));
    assert!(
        caught.is_err(),
        "the MXCSR check let an ldmxcsr that changed the control word pass"
    );
}

/// The control for the test above: the same island loading the default `0x1F80`, or the default
/// with a sticky status flag (PE, `0x20`), must pass.
#[test]
fn mxcsr_unchanged_control_passes() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let ok = emu.alloc(0x10);
    emu.write_u32(ok, 0x1F80);
    let island = emu.alloc(0x20);
    let mut code = vec![0x0Fu8, 0xAE, 0x15];
    code.extend_from_slice(&(ok as u32).to_le_bytes());
    code.push(0x90);
    emu.write(island, &code);

    let res = catch_unwind(AssertUnwindSafe(|| {
        emu.run(island, island + 7);
    }));
    assert!(
        res.is_ok(),
        "the MXCSR check fired on an unchanged control word (0x1F80)"
    );
    emu.write_u32(ok, 0x1F80 | 0x20);
    let res2 = catch_unwind(AssertUnwindSafe(|| {
        emu.run(island, island + 7);
    }));
    assert!(
        res2.is_ok(),
        "the MXCSR check rejected a sticky exception flag, which it must ignore"
    );
}

/// An unmapped read must fail the run, and the failure must name the address, the pc and the
/// kind of access, in the panic and in `last_fault`; the next clean run clears it.
#[test]
fn unmapped_fault_is_captured() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // mov eax, ds:[0xDEAD0000]: A1 + disp32, 5 bytes. Nothing maps that address.
    const BAD: u32 = 0xDEAD_0000;
    let island = emu.alloc(0x20);
    let mut code = vec![0xA1u8];
    code.extend_from_slice(&BAD.to_le_bytes());
    emu.write(island, &code);

    let caught = catch_unwind(AssertUnwindSafe(|| {
        emu.run(island, island + 5);
    }));
    assert!(
        caught.is_err(),
        "an unmapped read must abort emu_start (and panic)"
    );
    let err = caught.unwrap_err();
    let msg = err
        .downcast_ref::<String>()
        .map(String::as_str)
        .unwrap_or("");
    assert!(
        msg.contains("[fault") && msg.contains("READ_UNMAPPED"),
        "panic must carry the fault suffix (addr/pc/kind); got: {msg:?}"
    );

    let (addr, pc, kind) = emu
        .last_fault()
        .expect("last_fault must hold the captured unmapped access");
    assert_eq!(addr, u64::from(BAD), "faulting VA");
    assert_eq!(pc, island, "PC = the faulting instruction's VA");
    assert_eq!(kind, MemType::READ_UNMAPPED, "access kind");

    // A lone nop must clear the capture.
    let ok = emu.alloc(0x10);
    emu.write(ok, &[0x90u8]);
    emu.run(ok, ok + 1);
    assert!(
        emu.last_fault().is_none(),
        "clear_fault must reset the capture at the start of each run"
    );
}
