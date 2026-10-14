//! Bit-exact tests of the four live InventoryType-to-body-slot dispatchers (`0x478bd0`, `0x478cb0`,
//! `0x478f20`, `0x479010`) against `WoW.exe`.
//!
//! The mapping tested is `arg -> slot -> handler target`: the per-argument byte LUT and the dword
//! jump table. The exe's own code runs from the function entry up to the `jmp dword ptr [4*slot +
//! table]` (not executed), so the handlers never run. The test then reads the slot register the exe
//! computed (`movzx <reg>, byte[arg + lut]`) and requires it to equal `lut[arg]`, and reads the
//! resolved target `[table + 4*slot]` out of the image and requires it to equal `table[slot]`. It
//! also compares the LUT and the jump table with the mapped image. Out-of-range arguments run from
//! the entry to the default and require the exe to halt there (the `jg` or `ja` bound check is
//! taken). Each test runs the function from its entry to the `jmp`, a slice.
use benilla_re_charactermodel::dispatch as cm;
use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// A scratch stack for a slice `run`: lays the frame out as `Emu::call` would (arguments pushed
/// right to left under a dummy return address) and sets ESP, ECX and EDX. The prologue's `push ebp`
/// then sees `[ebp+8]` == `[esp_entry+4]` == arg0.
fn setup_frame(emu: &mut Emu, ecx: u32, edx: u32, stack_args: &[u32]) {
    let stack_top = emu.alloc(0x1000) + 0x800; // mid-region: room to push ebp below, args above
    let mut esp = stack_top;
    for &a in stack_args.iter().rev() {
        esp -= 4;
        emu.write_u32(esp, a);
    }
    esp -= 4;
    emu.write_u32(esp, 0xdead_beef); // dummy return address (never used: the run halts at the jmp)
    emu.set_reg(RegisterX86::ESP, esp);
    emu.set_reg(RegisterX86::ECX, u64::from(ecx));
    emu.set_reg(RegisterX86::EDX, u64::from(edx));
}

/// Tests one `Dispatch` whose dispatch reads arg0 from `[ebp+8]` (the three `eax`-slot functions). For
/// each in-range arg, runs entry to jmp, reads the slot register (`eax`) and requires `lut[arg]`, and
/// requires the resolved target read from the image to equal `table[slot]`. For a few out-of-range
/// args, runs entry to default and requires the bound check to halt there.
fn diff_eax_dispatch(emu: &mut Emu, d: &cm::Dispatch) -> u32 {
    // The LUT and the jump table equal the mapped image.
    let lut_img = emu.read(u64::from(d.lut_va), d.lut.len());
    assert_eq!(lut_img, d.lut, "LUT bytes @ {:#x} diverge", d.lut_va);
    for (slot, &t) in d.table.iter().enumerate() {
        let img = emu.read_u32(u64::from(d.table_va) + 4 * slot as u64);
        assert_eq!(img, t, "jump table[{slot}] @ {:#x} diverges", d.table_va);
    }

    let mut total = 0u32;
    // In range: the slot the exe computed and the target it resolves, read at the jmp.
    for arg in 0..=d.max {
        setup_frame(emu, 0, 0, &[arg]);
        emu.run(u64::from(d.fn_va), u64::from(d.jmp_va));
        let bin_slot = emu.get_reg(RegisterX86::EAX) as u32;
        let exp_slot = u32::from(d.slot(arg));
        assert_eq!(
            bin_slot, exp_slot,
            "{:#x} arg={arg:#x}: slot reg={bin_slot:#x} expected {exp_slot:#x}",
            d.fn_va
        );
        // The resolved jump target equals table[slot] (the full arg-to-handler mapping).
        let bin_target = emu.read_u32(u64::from(d.table_va) + 4 * u64::from(bin_slot));
        assert_eq!(
            bin_target,
            d.target(arg),
            "{:#x} arg={arg:#x}: target={bin_target:#x} expected {:#x}",
            d.fn_va,
            d.target(arg)
        );
        total += 1;
    }
    // Out of range: the bound check (`jg` or `ja`) is taken and the run halts at the default target.
    // Only args the exe catches are probed: a signed `jg` bound lets a high-bit arg through to index
    // the LUT out of bounds, which is no default to emulate. `in_range` encodes each function's
    // signedness.
    for arg in [d.max + 1, d.max + 2, 0x100, 0xffff_ffff] {
        if d.in_range(arg) {
            continue; // a signed bound does not take the default here
        }
        setup_frame(emu, 0, 0, &[arg]);
        emu.run(u64::from(d.fn_va), u64::from(d.default_va));
        total += 1;
    }
    total
}

/// `0x478bd0`: handlers `push N; call 0x478960`; arg <= 0x12, slot register `eax`.
#[test]
fn dispatch_478bd0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let n = diff_eax_dispatch(&mut emu, &cm::D_478BD0);
    eprintln!("dispatch_478bd0: {n} bit-exact");
}

/// `0x478cb0`: handlers `push displayId; push N; call 0x478aa0`; arg <= 0x12, slot register `eax`.
#[test]
fn dispatch_478cb0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let n = diff_eax_dispatch(&mut emu, &cm::D_478CB0);
    eprintln!("dispatch_478cb0: {n} bit-exact");
}

/// `0x478f20`: handlers `push N; call 0x478960`; arg <= 0x18, slot register `eax`.
#[test]
fn dispatch_478f20() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let n = diff_eax_dispatch(&mut emu, &cm::D_478F20);
    eprintln!("dispatch_478f20: {n} bit-exact");
}

/// `0x479010`: clears via `0x478f20`, then dispatches under two guards (`arg1 > 0` signed and
/// `arg0 <= 0x1a`); the slot register is `ecx`, and arg0 is read from `edi`. The slot and target
/// mapping runs the dispatch tail alone (from `0x47902b`, after the inner call and the arg1 guard,
/// with `edi = arg0`) up to the jmp, so neither the inner `call 0x478f20` nor a handler runs. The
/// slice is `0x47902b` to the jmp at `0x47903b`.
#[test]
fn dispatch_479010() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let d = &cm::D_479010;

    // The LUT and the jump table equal the image.
    let lut_img = emu.read(u64::from(d.lut_va), d.lut.len());
    assert_eq!(lut_img, d.lut, "479010 LUT bytes diverge");
    for (slot, &t) in d.table.iter().enumerate() {
        let img = emu.read_u32(u64::from(d.table_va) + 4 * slot as u64);
        assert_eq!(img, t, "479010 jump table[{slot}] diverges");
    }

    let mut total = 0u32;
    // The dispatch tail: sets edi = arg0, runs 0x47902b to the jmp (0x47903b) and reads ecx.
    const TAIL_START: u64 = 0x47902b; // the `cmp edi,0x1a`, after the arg1 > 0 guard and the inner call
    for arg in 0..=d.max {
        emu.set_reg(RegisterX86::EDI, u64::from(arg));
        emu.run(TAIL_START, u64::from(d.jmp_va));
        let bin_slot = emu.get_reg(RegisterX86::ECX) as u32;
        let exp_slot = u32::from(d.slot(arg));
        assert_eq!(
            bin_slot, exp_slot,
            "0x479010 arg={arg:#x}: slot reg(ecx)={bin_slot:#x} expected {exp_slot:#x}"
        );
        let bin_target = emu.read_u32(u64::from(d.table_va) + 4 * u64::from(bin_slot));
        assert_eq!(
            bin_target,
            d.target(arg),
            "0x479010 arg={arg:#x}: target={bin_target:#x} expected {:#x}",
            d.target(arg)
        );
        total += 1;
    }
    // Out of range arg0: the unsigned `cmp edi,0x1a; ja` is taken and the run halts at the default 0x479116.
    for arg in [d.max + 1, d.max + 2, 0x100, 0xffff_ffff] {
        assert!(
            !d.in_range(arg),
            "0x479010 arg {arg:#x} should be out of range"
        );
        emu.set_reg(RegisterX86::EDI, u64::from(arg));
        emu.run(TAIL_START, u64::from(d.default_va));
        total += 1;
    }
    eprintln!("dispatch_479010: {total} bit-exact");
}
