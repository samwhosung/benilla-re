//! Bit-exact tests of the character component's leaf primitives against `WoW.exe`: pure-register,
//! global-table, this-memory and stack-argument-plus-struct inputs.
use benilla_re_charactermodel as cm;
use benilla_re_difftest::{Emu, Wow};

/// `0x47a070` attach-slot id — pure register in (`ecx`=case, `dl`) → `eax`.
#[test]
fn attach_slot() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut total = 0u32;
    for case in 0u32..8 {
        for dl in [0u32, 1, 0xff, 0x100, 0x42, 0x80] {
            let bin = emu.call(0x47a070, case, dl, &[]);
            assert_eq!(
                bin,
                cm::attach_slot(case, dl),
                "attach_slot case={case} dl={dl:#x}"
            );
            total += 1;
        }
    }
    eprintln!("attach_slot: {total} bit-exact");
}

/// `0x47a4c0` race-id table index — reads the `[0xb4244c]` pointer + `[table + 4*(edx+2*ecx)]`.
#[test]
fn raceid_index() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let tbl = emu.alloc(0x200);
    let vals: Vec<u32> = (0..0x40u32)
        .map(|i| 0x1000u32.wrapping_add(i.wrapping_mul(0x97)))
        .collect();
    for (i, v) in vals.iter().enumerate() {
        emu.write_u32(tbl + 4 * i as u64, *v);
    }
    emu.write_u32(0xb4244c, tbl as u32);
    let mut total = 0u32;
    for ecx in 0u32..8 {
        for edx in 0u32..4 {
            if (edx + 2 * ecx) as usize >= vals.len() {
                continue;
            }
            let bin = emu.call(0x47a4c0, ecx, edx, &[]);
            assert_eq!(
                bin,
                cm::raceid_index(&vals, ecx, edx),
                "raceid ecx={ecx} edx={edx}"
            );
            total += 1;
        }
    }
    eprintln!("raceid_index: {total} bit-exact");
}

/// `0x47b6f0` TSLink offset resolver — `__thiscall` (`ecx`=this) + 1 stack arg; reads `[this]`.
#[test]
fn tslink_offset() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40) as u32;
    let mut total = 0u32;
    for link0 in [0u32, 0x1000, 0xdead_beef, 4] {
        emu.write_u32(this as u64, link0);
        for arg in [0u32, 4, 0x20, 0x100, 0xfffc] {
            let bin = emu.call(0x47b6f0, this, 0, &[arg]);
            assert_eq!(
                bin,
                cm::tslink_offset(this, link0, arg),
                "tslink this={this:#x} link0={link0:#x} arg={arg}"
            );
            total += 1;
        }
    }
    eprintln!("tslink_offset: {total} bit-exact");
}

/// `0x4774f0` occupancy predicate — 2 stack args (`ret 8`); reads `[arg0 + 4*idx + 0x18]`.
#[test]
fn occupancy() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let buf = emu.alloc(0x100) as u32;
    let mut total = 0u32;
    for idx in 0u32..6 {
        for slot_val in [0u32, 1, 0xff, 0x8000_0000] {
            emu.write_u32(buf as u64 + (4 * idx + 0x18) as u64, slot_val);
            for arg0 in [buf, 0] {
                let bin = emu.call(0x4774f0, 0, 0, &[arg0, idx]) & 0xff;
                let exp = u32::from(cm::occupancy(arg0, if arg0 != 0 { slot_val } else { 0 }));
                assert_eq!(
                    bin, exp,
                    "occupancy arg0={arg0:#x} idx={idx} val={slot_val:#x}"
                );
                total += 1;
            }
        }
    }
    eprintln!("occupancy: {total} bit-exact");
}
