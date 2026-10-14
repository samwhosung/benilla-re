//! Bit-exact tests of the attach-point and geoset-enable selection of the character component
//! against `WoW.exe`. Each test isolates the slot-and-case-to-id selection by a slice (run to the point
//! the selected id or index is in a register, then read it) or by the recorded dispatch; the calls
//! out to the model and scene code are not modelled.

use benilla_re_charactermodel::attach::{
    attach_install_select, attach_slot, boots_region_decision, race_attach_pair,
    shoulder_attach_pair, weapon_attach_table, AttachPair,
};
use benilla_re_difftest::{Emu, RegisterX86, Wow};

fn esi_i32(e: &Emu) -> i32 {
    e.get_reg(RegisterX86::ESI) as u32 as i32
}
fn edi_i32(e: &Emu) -> i32 {
    e.get_reg(RegisterX86::EDI) as u32 as i32
}
fn edx_u32(e: &Emu) -> u32 {
    e.get_reg(RegisterX86::EDX) as u32
}

// ---------------------------------------------------------------------------------------------
// 0x47a070 — attach-slot jump table (the shared selection primitive; pure __fastcall → eax).
// ---------------------------------------------------------------------------------------------
#[test]
fn cm_fanout_c_attach_slot() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut n = 0u32;
    // case 0..=8 (covers the in-table 0..4, the `ja` >4 default, and the unsigned-negative path) ×
    // flag {0,1} (dl carries the flag; only its zero/non-zero matters — sweep a few non-zero values).
    for case in 0u32..=8 {
        for &dl in &[0u32, 1, 0xff, 0x100, 0x80] {
            let flag = (dl & 0xff) != 0; // dl is `mov al, dl` = the low byte
            let bin = emu.call(0x47a070, case, dl, &[]) as i32;
            assert_eq!(
                bin,
                attach_slot(case, flag),
                "attach_slot case={case:#x} dl={dl:#x}"
            );
            n += 1;
        }
    }
    // a negative `ecx` (the `cmp ecx,4; ja` is UNSIGNED → default -1).
    for &case in &[0xffff_ffffu32, 0x8000_0000, 0x7fff_ffff] {
        let bin = emu.call(0x47a070, case, 1, &[]) as i32;
        assert_eq!(bin, -1, "attach_slot negative case={case:#x}");
        n += 1;
    }
    eprintln!("cm_fanout_c_attach_slot: {n} bit-exact (0x47a070 jump table → eax)");
}

// ---------------------------------------------------------------------------------------------
// 0x47a380 — single attach-point install/clear. id → whether `call 0x4796b0` happens, and its edx arg.
//   id∈{0,1,2}: run to the `call 0x4796b0` (0x47a39f) and read EDX = the pushed arg.
//   other ids : full `call` with 0x4796b0 + 0x7103d0 stubbed → assert 0x4796b0 was NOT invoked.
// ---------------------------------------------------------------------------------------------
#[test]
fn cm_fanout_c_attach_install_select() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // 0x4796b0 (geoset-range enable B): record + return. fastcall, no stack args cleaned → pop 0.
    emu.stub(0x4796b0, 0, 0);
    // 0x7103d0 ("child exists?"): thiscall, 2 stack args (push 0;push 0) → return 0 to stop early.
    emu.stub(0x7103d0, 0, 2);
    let this = emu.alloc(0x10) as u32; // ecx=model, non-null so the branch after the id dispatch is reached

    let mut n = 0u32;
    for id in [-1i32, 0, 1, 2, 3, 4, 7, 0x40, -2] {
        let expect = attach_install_select(id);
        match expect {
            Some(edx_arg) => {
                // The id reaches the single `call 0x4796b0` site; EDX there is the chosen arg.
                emu.set_reg(RegisterX86::ECX, u64::from(this));
                emu.set_reg(RegisterX86::EDX, id as u32 as u64);
                // push two scratch dwords so the fn's `push esi/edi` prologue + the body have stack.
                let esp = 0x7000_0000u64 + 0x0010_0000 - 0x2000;
                emu.set_reg(RegisterX86::ESP, esp);
                emu.run(0x47a380, 0x47a39f);
                assert_eq!(
                    edx_u32(&emu),
                    edx_arg,
                    "attach_install id={id}: 0x4796b0 edx-arg"
                );
            }
            None => {
                emu.clear_calls();
                emu.call(0x47a380, this, id as u32, &[]);
                assert!(
                    !emu.calls().iter().any(|c| c.addr == 0x4796b0),
                    "attach_install id={id}: 0x4796b0 should NOT be called"
                );
            }
        }
        n += 1;
    }
    eprintln!(
        "cm_fanout_c_attach_install_select: {n} ids bit-exact (0x47a380 id→0x4796b0 dispatch)"
    );
}

// ---------------------------------------------------------------------------------------------
// 0x479f40 — weapon attach by inv-slot. slot → table base (0x18→0x838d70, 0x19→0x838d50,
// else 0) behind a 3-pointer non-null gate. Valid slots: run to 0x479f90 (`push eax` before the
// first 0x64a7f0 call), read ESI = the selected base. Invalid: the gate's 0-return is the verdict.
// ---------------------------------------------------------------------------------------------
#[test]
fn cm_fanout_c_weapon_attach_table() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x10) as u32;
    // item struct (edx=ebx): [ebx+8] and [ebx+0x10] must point to non-empty strings (byte != 0).
    let item = emu.alloc(0x80);
    let s8 = emu.alloc(0x10);
    let s10 = emu.alloc(0x10);
    emu.write(s8, b"A\0");
    emu.write(s10, b"B\0");
    emu.write_u32(item + 8, s8 as u32);
    emu.write_u32(item + 0x10, s10 as u32);
    let stack = emu.alloc(0x1000);

    let mut n = 0u32;
    for slot in [0x18u32, 0x19, 0, 1, 0x17, 0x1a, 0xff] {
        let expect = weapon_attach_table(slot);
        match expect {
            Some(base) => {
                let esp = stack + 0xf00;
                emu.set_reg(RegisterX86::ESP, esp);
                // run from the fn ENTRY: arg0 (`[ebp+8]`) sits at `[esp_entry+4]` after the prologue.
                emu.write_u32(esp + 4, slot); // [ebp+8] = inv-slot
                emu.set_reg(RegisterX86::ECX, u64::from(this));
                emu.set_reg(RegisterX86::EDX, item as u32 as u64); // edx → ebx = item
                emu.run(0x479f40, 0x479f90); // gate passes → esi = selected base, HALT at `push eax`
                assert_eq!(
                    esi_i32(&emu) as u32,
                    base,
                    "weapon_attach_table slot={slot:#x} base"
                );
            }
            None => {
                // The gate's slot check (`jne 0x47a037`) returns 0 with no base selected and no
                // child call (the item-pointer gate still passes). The fn sets up its own EBP.
                let eax = emu.call(0x479f40, this, item as u32, &[slot]);
                assert_eq!(eax, 0, "weapon_attach_table slot={slot:#x} → 0-return");
            }
        }
        n += 1;
    }
    eprintln!("cm_fanout_c_weapon_attach_table: {n} slots bit-exact (0x479f40 slot→base)");
}

// ---------------------------------------------------------------------------------------------
// 0x47a0c0 — per-race/slot attach resolver. slot → (esi, edi) pair fed to the two 0x47a380
// installs. Run to the FIRST `call 0x47a380` (0x47a14e): at that point ESI=esi, EDI=edi (both set;
// `attach_slot` 0x47a070 ran natively in between). None-slots take the -1 early-out → 0x47a380 never
// fires (verified via a stubbed full call).
// ---------------------------------------------------------------------------------------------
#[test]
fn cm_fanout_c_race_attach_pair() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x47a380, 0, 0); // the install is not modelled: record only (for the None early-out check)
    let this = emu.alloc(0x20) as u32; // ecx, non-null
    let display = emu.alloc(0x80) as u32; // edx, non-null
    let stack = emu.alloc(0x1000);

    let mut n = 0u32;
    // slot × case × the two flags that steer the pair (flag14 for 0x10, flag18 for 0x11).
    for &slot in &[0x0fi32, 0x10, 0x11, 0x12, 0x0e, 0x13, -1] {
        for &case in &[0u32, 1, 2, 3, 4, 5] {
            for &f14 in &[0u32, 1] {
                for &f18 in &[0u32, 1] {
                    let expect = race_attach_pair(slot, case, false, f14 != 0, f18 != 0);
                    match expect {
                        Some(AttachPair { esi, edi }) => {
                            let esp = stack + 0xf00;
                            emu.set_reg(RegisterX86::ESP, esp);
                            // run from the fn ENTRY: args `[ebp+8..0x18]` map to `[esp_entry+4..0x14]`.
                            emu.write_u32(esp + 0x04, slot as u32); // [ebp+8]  eq-slot
                            emu.write_u32(esp + 0x08, case); // [ebp+0xc] case index → 0x47a070 ecx
                            emu.write_u32(esp + 0x0c, 0); // [ebp+0x10] flag10 (not in the pair)
                            emu.write_u32(esp + 0x10, f14); // [ebp+0x14] flag14
                            emu.write_u32(esp + 0x14, f18); // [ebp+0x18] flag18
                            emu.set_reg(RegisterX86::ECX, u64::from(this));
                            emu.set_reg(RegisterX86::EDX, u64::from(display));
                            emu.run(0x47a0c0, 0x47a14e); // → ESI=esi, EDI=edi at the first 0x47a380 call
                            assert_eq!(
                                esi_i32(&emu),
                                esi,
                                "race_attach_pair slot={slot:#x} case={case} f14={f14} f18={f18}: esi"
                            );
                            assert_eq!(
                                edi_i32(&emu),
                                edi,
                                "race_attach_pair slot={slot:#x} case={case} f14={f14} f18={f18}: edi"
                            );
                        }
                        None => {
                            emu.clear_calls();
                            emu.call(0x47a0c0, this, display, &[slot as u32, case, 0, f14, f18]);
                            assert!(
                                !emu.calls().iter().any(|c| c.addr == 0x47a380),
                                "race_attach_pair slot={slot:#x}: 0x47a380 should NOT fire"
                            );
                        }
                    }
                    n += 1;
                }
            }
        }
    }
    eprintln!("cm_fanout_c_race_attach_pair: {n} cases bit-exact (0x47a0c0 slot→(esi,edi))");
}

// ---------------------------------------------------------------------------------------------
// 0x47a310 — shoulder/special attach helper. slot → (esi, edi). Run to the FIRST 0x47a380
// call (0x47a369): ESI=esi, EDI=edi. None-slots (out of [0xe,0x12], or null this) → no install.
// ---------------------------------------------------------------------------------------------
#[test]
fn cm_fanout_c_shoulder_attach_pair() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x47a380, 0, 0);
    let this = emu.alloc(0x20) as u32; // ecx=ebx, non-null
    let stack = emu.alloc(0x1000);

    let mut n = 0u32;
    for &slot in &[0x0ei32, 0x0f, 0x10, 0x11, 0x12, 0x0d, 0x13, -1] {
        for &case in &[0u32, 1, 2, 3, 4, 5] {
            for &flag in &[0u32, 1] {
                let expect = shoulder_attach_pair(slot, case, flag != 0);
                match expect {
                    Some(AttachPair { esi, edi }) => {
                        let esp = stack + 0xf00;
                        emu.set_reg(RegisterX86::ESP, esp);
                        // run from the fn ENTRY: args `[ebp+8]`/`[ebp+0xc]` map to `[esp_entry+4]`/`+8`.
                        emu.write_u32(esp + 0x04, case); // [ebp+8] case index → 0x47a070 ecx
                        emu.write_u32(esp + 0x08, flag); // [ebp+0xc] flag
                        emu.set_reg(RegisterX86::ECX, u64::from(this));
                        emu.set_reg(RegisterX86::EDX, slot as u32 as u64); // edx = eq-slot
                        emu.run(0x47a310, 0x47a369); // → ESI=esi, EDI=edi at the first 0x47a380 call
                        assert_eq!(
                            esi_i32(&emu),
                            esi,
                            "shoulder_attach_pair slot={slot:#x} case={case} flag={flag}: esi"
                        );
                        assert_eq!(
                            edi_i32(&emu),
                            edi,
                            "shoulder_attach_pair slot={slot:#x} case={case} flag={flag}: edi"
                        );
                    }
                    None => {
                        emu.clear_calls();
                        emu.call(0x47a310, this, slot as u32, &[case, flag]);
                        assert!(
                            !emu.calls().iter().any(|c| c.addr == 0x47a380),
                            "shoulder_attach_pair slot={slot:#x}: 0x47a380 should NOT fire"
                        );
                    }
                }
                n += 1;
            }
        }
    }
    eprintln!("cm_fanout_c_shoulder_attach_pair: {n} cases bit-exact (0x47a310 slot→(esi,edi))");
}

// ---------------------------------------------------------------------------------------------
// 0x4793f0 — boots/leg-layer apply. `[0x803c10 + 0x28*arg]` table index + the
// arg/occupancy → region (3 or 4) choice. Run to 0x479489 (`push eax`); EAX = the chosen value
// (region code 3/4, or the table value). The occupancy outcomes are driven via the ptrC / this
// fields the `0x4774f0` predicate reads; the table value is read from the binary's own .rdata.
// ---------------------------------------------------------------------------------------------
#[test]
fn cm_fanout_c_boots_region_decision() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x600); // esi = this (needs +0x4b4 and the +0xa write target)
    let ptr_c = emu.alloc(0x40); // [ebp+0xc] — arg0 of 0x4774f0 (occ probes idx 0 and 2)
    let field_4b4_obj = emu.alloc(0x40); // [this+0x4b4] target object for the secondary occ probe
    let stack = emu.alloc(0x1000);

    // 0x4774f0(arg0, idx) reads *(arg0 + 4*idx + 0x18): idx0 → +0x18, idx2 → +0x20.
    // All four occupancy combinations the branches read are swept.
    let mut n = 0u32;
    for arg in [6u32, 3, 5, 0, 1, 2, 4, 7] {
        // table value the binary loads at 0x803c10 + 0x28*arg (read from its own mapped .rdata).
        let table_val = emu.read_u32(0x803c10 + 0x28 * u64::from(arg));
        for &occ0 in &[0u32, 1] {
            // *(ptrC+0x18)  (idx 0)
            for &occ2 in &[0u32, 1] {
                // *(ptrC+0x20)  (idx 2)
                for &has_4b4 in &[0u32, 1] {
                    for &occ4b4 in &[0u32, 1] {
                        // *(field_4b4_obj+0x20)  (idx 2)
                        emu.write_u32(ptr_c + 0x18, occ0);
                        emu.write_u32(ptr_c + 0x20, occ2);
                        emu.write_u32(
                            this + 0x4b4,
                            if has_4b4 != 0 {
                                field_4b4_obj as u32
                            } else {
                                0
                            },
                        );
                        emu.write_u32(field_4b4_obj + 0x20, occ4b4);

                        let esp = stack + 0xf00;
                        emu.set_reg(RegisterX86::ESP, esp);
                        // run from the fn ENTRY: args `[ebp+8/0xc/0x10]` map to `[esp_entry+4/8/0xc]`.
                        emu.write_u32(esp + 0x04, arg); // [ebp+8]  arg
                        emu.write_u32(esp + 0x08, ptr_c as u32); // [ebp+0xc] ptrC
                        emu.write_u32(esp + 0x0c, 0); // [ebp+0x10] flag10 (read only after the halt point)
                        emu.set_reg(RegisterX86::ECX, this); // ecx → esi = this
                        emu.run(0x4793f0, 0x479489); // through the branches, halting at `push eax`

                        let bin = emu.get_reg(RegisterX86::EAX) as u32;
                        let occ_c = occupancy_field(occ0);
                        let occ_c2 = occupancy_field(occ2);
                        let occ_4b4 = occupancy_field(occ4b4);
                        let field_4b4 = if has_4b4 != 0 {
                            field_4b4_obj as u32
                        } else {
                            0
                        };
                        let ours = boots_region_decision(
                            arg, table_val, occ_c, occ_c2, field_4b4, occ_4b4,
                        );
                        assert_eq!(
                            bin, ours,
                            "boots_region_decision arg={arg} occ0={occ0} occ2={occ2} has_4b4={has_4b4} occ4b4={occ4b4} table_val={table_val:#x}"
                        );
                        n += 1;
                    }
                }
            }
        }
    }
    eprintln!(
        "cm_fanout_c_boots_region_decision: {n} cases bit-exact (0x4793f0 region 3/4 / table)"
    );
}

/// The occupancy predicate's verdict given the dword `*(arg0+4*idx+0x18)`: our arg0 ptrs are always
/// non-null in this test, so the predicate reduces to `field != 0` (mirrors `fanout_c::occupancy`).
fn occupancy_field(field: u32) -> bool {
    field != 0
}
