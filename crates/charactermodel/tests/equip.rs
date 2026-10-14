//! Bit-exact tests against `WoW.exe` of `0x478160` (base-skin section apply), `0x478220` (face and
//! facial-hair section), `0x4782e0` (region-layer compositor: slot index and dirty mask), `0x478900`
//! (equip attach-slot cache store), `0x478960` (equip slot dispatch by item class) and `0x478ad0`
//! (equip item into slot).
//!
//! The arithmetic tested is the DBC record index, the texture-id clamp, the flat slot and array
//! index, and the dirty bit. The calls out (texture creation at `0x449d90`, `0x58a980`, `0x710ec0`,
//! the free at `0x47b010`, the dispatch table at `[0xb42424]`) are not modelled: a test either
//! runs a slice to the point the index or clamped value is in a register, or runs the function
//! whole with the calls stubbed and reads the struct field the exe wrote.
use benilla_re_charactermodel::equip as cm;
use benilla_re_difftest::{Emu, RegisterX86, Wow};

const DBC: u64 = 0xb4231c; // CharSections DBC record array base ptr
const TEX_MAX: u64 = 0xc0df70; // texid clamp upper bound (signed)
const TEX_TBL: u64 = 0xc0df6c; // name-table base ptr ([tbl + 4*texid])
                               // ESP for `run` slices (no auto-ESP): point into the harness stack region well below the top.
const SLICE_ESP: u64 = 0x7000_0000 + 0x0010_0000 - 0x2000;

fn idx(race: u32, sex: u32) -> u64 {
    u64::from(sex + 2 * race)
}

/// Set up the shared texid clamp globals: `[0xc0df70]` = max (signed bound), `[0xc0df6c]` = a
/// name-table array of `n` distinct dwords. Returns the table base.
fn setup_texid_globals(emu: &mut Emu, max: i32, n: u64) -> u64 {
    let tbl = emu.alloc(n * 4);
    emu.write_u32(TEX_MAX, max as u32);
    emu.write_u32(TEX_TBL, tbl as u32);
    for i in 0..n {
        emu.write_u32(tbl + 4 * i, 0x4e00_0000 + i as u32); // distinct, recognizable name entries
    }
    tbl
}

/// Read back a snapshot of the texid name-table for the Rust mirror.
fn read_name_tbl(emu: &Emu, tbl: u64, n: u64) -> Vec<u32> {
    (0..n).map(|i| emu.read_u32(tbl + 4 * i)).collect()
}

// ── 0x478160 — base-skin section apply: idx + nested deref + texid clamp (partial slice) ─────────
//
// idx = [this+0x1c] + 2*[this+0x18]; rec4 = [ [0xb4231c] + 0x28*idx + 4 ]; texid = [ [rec4+8] + 4*esi ]
// with esi=[this+0x24]; then clamp to [0, [0xc0df70]] (signed) and index [0xc0df6c][texid] (else 0).
// We stub the 0x477c20 gate → 1 so the chain runs, set up the nested derefs so `texid` is a value we
// control, and run to the clamp convergence (0x4781b1) reading EAX = the clamped name-table dword.
#[test]
fn cm_fanout_g_base_skin_texid() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x477c20, 1, 3); // the validity gate is not modelled (ret 0xc): it passes

    let max = 0x20i32;
    let tbl_n = 0x40u64;
    let _tbl = setup_texid_globals(&mut emu, max, tbl_n);
    let tbl = TEX_TBL; // base ptr held at [0xc0df6c]

    let this = emu.alloc(0x100);
    // CharSections rows (stride 0x28) for idx 0..8; row field +4 holds a "rec4" ptr.
    let rows = emu.alloc(8 * 0x28);
    let rec4s = emu.alloc(0x800); // rec4 structs ([rec4+8] -> tex array)
    let texarrs = emu.alloc(0x2000); // per-(idx) texid arrays indexed by esi
    emu.write_u32(DBC, rows as u32);
    for i in 0u64..8 {
        let rec4 = rec4s + 0x40 * i;
        emu.write_u32(rows + 0x28 * i + 4, rec4 as u32);
        let texarr = texarrs + 0x100 * i;
        emu.write_u32(rec4 + 0x8, texarr as u32);
        // fill the texid array (indexed by esi=[this+0x24]) with a spread of signed values that
        // exercise the clamp: negative, 0, in-range, == max, > max.
        for k in 0u64..8 {
            let texid: i32 = match k {
                0 => -1,
                1 => 0,
                2 => 5,
                3 => max,     // == max → in-range (jg is strict)
                4 => max + 1, // > max → clamp to 0
                5 => 0x10,
                6 => -0x100,
                _ => 0x1f,
            };
            emu.write_u32(texarr + 4 * k, texid as u32);
        }
    }

    let mut total = 0u32;
    for race in 0u32..4 {
        for sex in 0u32..2 {
            emu.write_u32(this + 0x18, race);
            emu.write_u32(this + 0x1c, sex);
            for esi in 0u32..8 {
                emu.write_u32(this + 0x24, esi);
                let i = idx(race, sex);
                let rec4 = emu.read_u32(rows + 0x28 * i + 4);
                let texarr = emu.read_u32(u64::from(rec4) + 0x8);
                let texid = emu.read_u32(u64::from(texarr) + 4 * u64::from(esi));
                let max_v = emu.read_u32(TEX_MAX);
                let name_tbl = read_name_tbl(&emu, u64::from(emu.read_u32(tbl)), tbl_n);

                emu.set_reg(RegisterX86::ESP, SLICE_ESP);
                emu.set_reg(RegisterX86::ECX, this);
                emu.run(0x478160, 0x4781b1); // → EAX = clamped name-table dword
                let bin = emu.get_reg(RegisterX86::EAX) as u32;
                let exp = cm::base_skin_texid_name(texid, max_v, &name_tbl);
                assert_eq!(
                    bin, exp,
                    "base_skin race={race} sex={sex} esi={esi} texid={texid:#x} max={max_v:#x}"
                );
                total += 1;
            }
        }
    }
    eprintln!("cm_fanout_g_base_skin_texid: {total} bit-exact (0x478160 idx+deref+clamp → EAX)");
}

// ── 0x478220 — face/facial-hair section: nested DBC index (idx*… +0x1c, 3*arg inner) + clamp ─────
//
// idx = [this+0x1c]+2*[this+0x18]; rec1c = [ [0xb4231c] + 0x28*idx + 0x1c ];
// sub = [ rec1c + 4*(3*arg0) + 8 ]; texid = [ sub + 4*[this+0x20] ]; then the same texid clamp.
// We stub 0x477c20 → 1 and run to the clamp convergence (0x47827a) reading EAX. arg0 = [ebp+8].
#[test]
fn cm_fanout_g_face_texid() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x477c20, 1, 3);

    let max = 0x18i32;
    let tbl_n = 0x40u64;
    setup_texid_globals(&mut emu, max, tbl_n);

    let this = emu.alloc(0x100);
    let rows = emu.alloc(8 * 0x28);
    let rec1cs = emu.alloc(0x1000); // rec1c structs ([rec1c + 12*arg0 + 8] -> sub)
    let subs = emu.alloc(0x4000); // per-(idx,arg0) sub arrays indexed by [this+0x20]
    emu.write_u32(DBC, rows as u32);
    let n_arg = 4u64; // exercise arg0 in 0..4 (inner stride 3*arg0)
    let n_f20 = 6u64; // exercise [this+0x20] in 0..6
    for i in 0u64..8 {
        let rec1c = rec1cs + 0x200 * i;
        emu.write_u32(rows + 0x28 * i + 0x1c, rec1c as u32);
        for arg0 in 0..n_arg {
            // sub ptr at rec1c + 4*(3*arg0) + 8
            let sub = subs + 0x100 * (i * n_arg + arg0);
            emu.write_u32(rec1c + 4 * (3 * arg0) + 8, sub as u32);
            for f20 in 0..n_f20 {
                let texid: i32 = match (arg0 + f20) % 6 {
                    0 => -1,
                    1 => 0,
                    2 => 4,
                    3 => max,
                    4 => max + 2,
                    _ => 0x12,
                };
                emu.write_u32(sub + 4 * f20, texid as u32);
            }
        }
    }

    let mut total = 0u32;
    for race in 0u32..4 {
        for sex in 0u32..2 {
            emu.write_u32(this + 0x18, race);
            emu.write_u32(this + 0x1c, sex);
            for arg0 in 0..n_arg as u32 {
                for f20 in 0..n_f20 as u32 {
                    emu.write_u32(this + 0x20, f20);
                    let i = idx(race, sex);
                    let rec1c = emu.read_u32(rows + 0x28 * i + 0x1c);
                    let sub = emu.read_u32(u64::from(rec1c) + 4 * (3 * u64::from(arg0)) + 8);
                    let texid = emu.read_u32(u64::from(sub) + 4 * u64::from(f20));
                    let max_v = emu.read_u32(TEX_MAX);
                    let name_tbl = read_name_tbl(&emu, u64::from(emu.read_u32(TEX_TBL)), tbl_n);

                    emu.set_reg(RegisterX86::ESP, SLICE_ESP);
                    emu.write_u32(SLICE_ESP + 4, arg0); // [ebp+8] after the prologue push ebp
                    emu.set_reg(RegisterX86::ECX, this);
                    emu.run(0x478220, 0x47827a); // → EAX = clamped name-table dword
                    let bin = emu.get_reg(RegisterX86::EAX) as u32;
                    let exp = cm::face_texid_name(texid, max_v, &name_tbl);
                    assert_eq!(
                        bin, exp,
                        "face race={race} sex={sex} arg0={arg0} f20={f20} texid={texid:#x} max={max_v:#x}"
                    );
                    total += 1;
                }
            }
        }
    }
    eprintln!("cm_fanout_g_face_texid: {total} bit-exact (0x478220 nested idx+clamp → EAX)");
}

// ── 0x4782e0 — region-layer compositor: slot index (partial slice) + dirty mask (micro-slice) ────
//
// Slot index: edi = this + 4*(sub_idx + 8*slot) + 0x188, computed at entry before any out-call. Run
// to 0x4782f9 (just after `lea edi`) and read EDI - this = the byte offset.
#[test]
fn cm_fanout_g_region_layer_slot_off() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x4000); // large enough that this + off is mapped (off up to ~0x1000)

    let mut total = 0u32;
    for slot in 0u32..8 {
        for sub_idx in 0u32..8 {
            emu.set_reg(RegisterX86::ESP, SLICE_ESP);
            emu.write_u32(SLICE_ESP + 4, slot); // [ebp+8] = slot (ebx)
            emu.write_u32(SLICE_ESP + 8, sub_idx); // [ebp+0xc] = sub_idx (eax)
            emu.set_reg(RegisterX86::ECX, this);
            emu.run(0x4782e0, 0x4782f9); // → EDI = this + 4*(sub_idx+8*slot) + 0x188
            let edi = emu.get_reg(RegisterX86::EDI) as u32;
            let off = edi.wrapping_sub(this as u32);
            let exp = cm::region_layer_slot_off(slot, sub_idx);
            assert_eq!(off, exp, "region_layer slot={slot} sub_idx={sub_idx}");
            total += 1;
        }
    }
    eprintln!(
        "cm_fanout_g_region_layer_slot_off: {total} bit-exact (0x4782e0 4*(sub+8*slot)+0x188)"
    );
}

// 0x4782e0 dirty-bit mask: [this+0x10] |= (1 << (bit_no & 0x1f)) at the fn tail (0x4783c7..0x4783d6).
// We micro-slice that block: set esi=this, ebp so [ebp+0x18]=bit_no, [this+0x10]=initial, run
// 0x4783c7 → 0x4783d9 (through the store), read [this+0x10].
#[test]
fn cm_fanout_g_region_layer_dirty_mask() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40);
    let frame = emu.alloc(0x40); // synthetic ebp frame ([ebp+0x18] = bit_no)

    let mut total = 0u32;
    for bit_no in 0u32..40 {
        // sweep past 31 to exercise the x86 `shl` 5-bit count mask
        for &init in &[0u32, 0x1, 0xffff_ffff, 0x8000_0000, 0x5a5a_5a5a] {
            emu.write_u32(this + 0x10, init);
            emu.write_u32(frame + 0x18, bit_no);
            emu.set_reg(RegisterX86::ESP, SLICE_ESP);
            emu.set_reg(RegisterX86::ESI, this);
            emu.set_reg(RegisterX86::EBP, frame);
            emu.run(0x4783c7, 0x4783d9); // mov ecx,[ebp+0x18]; ... or [esi+0x10]
            let bin = emu.read_u32(this + 0x10);
            let exp = init | cm::region_layer_dirty_mask(bit_no);
            assert_eq!(bin, exp, "dirty_mask bit_no={bit_no} init={init:#x}");
            total += 1;
        }
    }
    eprintln!(
        "cm_fanout_g_region_layer_dirty_mask: {total} bit-exact (0x4782e0 1<<(bit&0x1f) | …)"
    );
}

// ── 0x478900 — equip attach-slot cache store: (col<<4)+row flat index + store outcome ───────────
//
// esi=(col<<4)+row; cur=[this+4*esi+0x248]; if cur!=0 && [cur+4*col+0x38]==[new_ptr+4*col+0x38] →
// cache hit (no store), else free +0x228 if set and store [this+4*esi+0x248]=new_ptr. We stub the
// free 0x47b010, run the WHOLE fn, and read back [this+4*esi+0x248] (struct output) to verify the flat
// index + the store outcome. col=[ebp+8], new_ptr=[ebp+0xc], row=[ebp+0x10].
#[test]
fn cm_fanout_g_attach_cache_store() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x47b010, 0, 0); // the free is not modelled (thiscall, ecx; no stack args): record only

    let this = emu.alloc(0x4000); // holds the +0x228 / +0x248 parallel arrays
    let new_ptr = emu.alloc(0x100); // edx-side struct: [new_ptr+4*col+0x38]
    let cur_obj = emu.alloc(0x100); // a non-null "cur" object: [cur+4*col+0x38]

    let mut total = 0u32;
    for col in 0u32..4 {
        for row in 0u32..4 {
            let esi = cm::attach_cache_index(col, row);
            let src_off = this + 4 * u64::from(esi) + 0x248;
            let tex_off = this + 4 * u64::from(esi) + 0x228;
            for &flag_3c in &[0u32, 1] {
                for &cur_kind in &[0u32, 1, 2] {
                    // 0 = cur null; 1 = cur non-null, eq-field equal (cache hit); 2 = non-null, differ
                    for &tex_set in &[0u32, 1] {
                        emu.write_u32(this + 0x3c, flag_3c);
                        // new_ptr eq field at [new_ptr+4*col+0x38]
                        emu.write_u32(new_ptr + 4 * u64::from(col) + 0x38, 0xabcd_0000 + col);
                        let new_eq = emu.read_u32(new_ptr + 4 * u64::from(col) + 0x38);
                        // set up cur (the +0x248 SRC entry)
                        let cur = match cur_kind {
                            0 => 0u32,
                            _ => cur_obj as u32,
                        };
                        emu.write_u32(src_off, cur);
                        if cur != 0 {
                            let cur_eq = if cur_kind == 1 { new_eq } else { new_eq ^ 0x1 };
                            emu.write_u32(cur_obj + 4 * u64::from(col) + 0x38, cur_eq);
                        }
                        // the +0x228 TEX entry (freed before store when set)
                        emu.write_u32(tex_off, if tex_set != 0 { 0xdead_2228 } else { 0 });

                        emu.clear_calls();
                        emu.call(0x478900, this as u32, 0, &[col, new_ptr as u32, row]);

                        let stored = emu.read_u32(src_off);
                        let cur_eq_field = if cur != 0 {
                            emu.read_u32(cur_obj + 4 * u64::from(col) + 0x38)
                        } else {
                            0
                        };
                        let stores = cm::attach_cache_stores(flag_3c, cur, cur_eq_field, new_eq);
                        let freed = emu.calls().iter().any(|c| c.addr == 0x47b010);
                        if stores {
                            assert_eq!(
                                stored, new_ptr as u32,
                                "attach store col={col} row={row} f3c={flag_3c} cur={cur_kind} tex={tex_set}"
                            );
                            // free fires iff the +0x228 entry was set
                            assert_eq!(
                                freed,
                                tex_set != 0,
                                "attach free col={col} row={row} cur={cur_kind} tex={tex_set}"
                            );
                        } else {
                            // no-op: the SRC slot is unchanged (== cur) and no free fires
                            assert_eq!(
                                stored, cur,
                                "attach no-op col={col} row={row} f3c={flag_3c} cur={cur_kind}"
                            );
                            assert!(
                                !freed,
                                "attach no-op no-free col={col} row={row} cur={cur_kind}"
                            );
                        }
                        total += 1;
                    }
                }
            }
        }
    }
    eprintln!("cm_fanout_g_attach_cache_store: {total} bit-exact (0x478900 (col<<4)+row + store)");
}

// ── 0x478960 — equip slot dispatch: slot index 4*slot+0x4a8 + slot-0 region adds + constants ─────
//
// Two diffs: (a) the slot index 4*slot+0x4a8 by slice (run to 0x478971, just after
// `mov eax,[esi+4*ebx+0x4a8]`, read EAX = the loaded slot entry — proving the address by placing a
// recognizable value at this+4*slot+0x4a8); (b) the slot-0 region-offset adds + the 0x2be constant by
// struct output (stub the out-calls, run the whole fn for slot 0, read [this+0x148/0x150/0x14c/0x160]).
#[test]
fn cm_fanout_g_equip_slot_index() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x1000); // holds the +0x4a8.. slot table

    let mut total = 0u32;
    for slot in 0u32..16 {
        let off = cm::equip_slot_index(slot);
        let marker = 0x5100_0000 + slot;
        emu.write_u32(this + u64::from(off), marker);
        emu.set_reg(RegisterX86::ESP, SLICE_ESP);
        emu.write_u32(SLICE_ESP + 4, slot); // [ebp+8] = slot (ebx)
        emu.set_reg(RegisterX86::ECX, this);
        emu.run(0x478960, 0x478971); // → EAX = [this + 4*slot + 0x4a8]
        let bin = emu.get_reg(RegisterX86::EAX) as u32;
        assert_eq!(bin, marker, "equip_slot_index slot={slot} off={off:#x}");
        total += 1;
    }
    eprintln!("cm_fanout_g_equip_slot_index: {total} bit-exact (0x478960 4*slot+0x4a8 → EAX)");
}

// (b) slot-0 region adds + the 0x2be constant — struct output, the 0x478660-style mapping.
const RR_TBL: u64 = 0xc0df28; // 0x478740 region-record table ptr
const RR_CNT: u64 = 0xc0df2c; //  ...its count
const GV_TBL: u64 = 0xc0df78; // 0x478540 geoset-variation table ptr
const GV_CNT: u64 = 0xc0df7c; //  ...its count

#[test]
fn cm_fanout_g_equip_slot0_region() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // The calls out in the slot-0 path, not modelled: 0x47a380 (attach install, fastcall ecx/edx, no stack),
    // and the 0x478540 / 0x478740 table scans run for real. Empty the geoset-variation table so
    // 0x478540 returns its default (1) deterministically.
    emu.stub(0x47a380, 0, 0);
    emu.write_u32(GV_CNT, 0);
    emu.write_u32(GV_TBL, 0);

    let this = emu.alloc(0x1000);
    emu.write_u32(this + 0x38, 0); // [this+0x38] loaded into ecx for the 0x47a380 call

    // 0x478740 region-record table keyed (race, sex, slot0-key). The slot-0 path calls 0x478740 with
    // ecx = &[this+0x18], i.e. key (race, sex, [this+0x20]). [this+0x20] is whatever we leave it; set 0.
    let n = 8u32;
    let rr = emu.alloc(u64::from(n) * 0x28);
    emu.write_u32(RR_TBL, rr as u32);
    emu.write_u32(RR_CNT, n);
    let mut ri = 0u64;
    let mut records: Vec<(u32, u32, u32, u32, u32, u32)> = Vec::new(); // (race,sex,k18, f18,f1c,f20)
    for race in 0u32..4 {
        for sex in 0u32..2 {
            let r = rr + ri * 0x28;
            emu.write_u32(r, race);
            emu.write_u32(r + 0x4, sex);
            emu.write_u32(r + 0x8, 0); // key [this+0x20] = 0
            let f18 = 0x1000 + ri as u32;
            let f1c = 0x2000 + ri as u32;
            let f20 = 0x3000 + ri as u32;
            emu.write_u32(r + 0x18, f18);
            emu.write_u32(r + 0x1c, f1c);
            emu.write_u32(r + 0x20, f20);
            records.push((race, sex, 0, f18, f1c, f20));
            ri += 1;
        }
    }

    let mut total = 0u32;
    for race in 0u32..4 {
        for sex in 0u32..2 {
            emu.write_u32(this + 0x18, race);
            emu.write_u32(this + 0x1c, sex);
            emu.write_u32(this + 0x20, 0);
            // [this+4*0+0x4a8] != 0 so the fn does NOT early-return (the eax==0 guard).
            emu.write_u32(this + 0x4a8, 0x1234_5678);
            // clear the slot-0 store targets so "not written" is detectable.
            for &o in &[0x148u64, 0x14c, 0x150, 0x160] {
                emu.write_u32(this + o, 0xdead_0000 + o as u32);
            }
            emu.clear_calls();
            let _ = emu.call(0x478960, this as u32, 0, &[0u32]); // slot 0

            let rec = records
                .iter()
                .find(|&&(k0, k4, k8, ..)| k0 == race && k4 == sex && k8 == 0)
                .map(|&(_, _, _, f18, f1c, f20)| (f18, f1c, f20));
            let exp = cm::equip_slot0_region(rec);
            let d148 = emu.read_u32(this + 0x148);
            let d14c = emu.read_u32(this + 0x14c);
            let d150 = emu.read_u32(this + 0x150);
            let d160 = emu.read_u32(this + 0x160);
            match exp.dest_148 {
                Some(v) => assert_eq!(d148, v, "slot0 d148 race={race} sex={sex}"),
                None => assert_eq!(d148, 0xdead_0148, "slot0 d148 unchanged r={race} s={sex}"),
            }
            match exp.dest_14c {
                Some(v) => assert_eq!(d14c, v, "slot0 d14c race={race} sex={sex}"),
                None => assert_eq!(d14c, 0xdead_014c, "slot0 d14c unchanged r={race} s={sex}"),
            }
            match exp.dest_150 {
                Some(v) => assert_eq!(d150, v, "slot0 d150 race={race} sex={sex}"),
                None => assert_eq!(d150, 0xdead_0150, "slot0 d150 unchanged r={race} s={sex}"),
            }
            match exp.dest_160 {
                Some(v) => assert_eq!(d160, v, "slot0 d160 (0x2be) race={race} sex={sex}"),
                None => assert_eq!(d160, 0xdead_0160, "slot0 d160 unchanged r={race} s={sex}"),
            }
            total += 1;
        }
    }
    eprintln!(
        "cm_fanout_g_equip_slot0_region: {total} bit-exact (0x478960 slot-0 region adds + 0x2be)"
    );
}

// ── 0x478ad0 — equip item into slot: slot index 4*slot+0x4a8 + 5*slot validity-table index ───────
//
// (a) slot index: [this+4*slot+0x4a8] = new_ptr stored at entry (0x478ae1). slice: run to
// 0x478ae8 (just after the store), read [this+4*slot+0x4a8] == new_ptr.
// (b) validity index: in the default branch the per-(slot,i) probe is [0x803bf8 + 4*(i+10*slot)] != -1.
// slice: drive the default branch to the first cmp (0x478ba0, i=0) and read ECX = i+10*slot.
#[test]
fn cm_fanout_g_equip_item_slot_index() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x1000);

    let mut total = 0u32;
    for slot in 0u32..16 {
        let off = cm::equip_item_slot_index(slot);
        let new_ptr = 0x6100_0000 + slot;
        // clear the target so the store is observable
        emu.write_u32(this + u64::from(off), 0xdead_4a80);
        emu.set_reg(RegisterX86::ESP, SLICE_ESP);
        emu.write_u32(SLICE_ESP + 4, slot); // [ebp+8] = slot (edi)
        emu.write_u32(SLICE_ESP + 8, new_ptr); // [ebp+0xc] = new_ptr (eax)
        emu.set_reg(RegisterX86::ECX, this);
        emu.run(0x478ad0, 0x478ae8); // mov [ebx+0xa],1; mov [ebx+4*edi+0x4a8],eax
        let stored = emu.read_u32(this + u64::from(off));
        assert_eq!(
            stored, new_ptr,
            "equip_item slot index slot={slot} off={off:#x}"
        );
        total += 1;
    }
    eprintln!("cm_fanout_g_equip_item_slot_index: {total} bit-exact (0x478ad0 4*slot+0x4a8 store)");
}

#[test]
fn cm_fanout_g_equip_validity_index() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x1000);
    let new_obj = emu.alloc(0x80); // eax = new_ptr; [new_ptr+0x38] -> a non-empty string ptr
    let strbuf = emu.alloc(0x20);
    emu.write(strbuf, b"X\0");
    emu.write_u32(new_obj + 0x38, strbuf as u32); // [eax+0x38] = char* (non-empty → loop body runs)

    // Set up the validity table so the first probe is != -1 (so the cmp is the only gate we exercise).
    // It is read at [0x803bf8 + 4*(i+10*slot)]; we leave it whatever the binary mapped (non -1 in
    // general). We only READ ECX at the cmp, so the table contents don't affect the diffed index.
    let mut total = 0u32;
    // default branch requires [this+0x3c] == 0 and slot not in {0,1,0xa,0xb}.
    emu.write_u32(this + 0x3c, 0);
    for slot in [2u32, 3, 4, 5, 6, 7, 8, 0xc, 0xd, 0x20] {
        let off = cm::equip_item_slot_index(slot);
        // [this+4*slot+0x4a8] is overwritten by the store at entry; not relevant to the default branch.
        emu.write_u32(this + u64::from(off), 0);
        emu.set_reg(RegisterX86::ESP, SLICE_ESP);
        emu.write_u32(SLICE_ESP + 4, slot); // [ebp+8] = slot (edi)
        emu.write_u32(SLICE_ESP + 8, new_obj as u32); // [ebp+0xc] = new_ptr (eax)
        emu.set_reg(RegisterX86::ECX, this);
        emu.run(0x478ad0, 0x478ba0); // default branch, first iteration → ECX = i + 10*slot (i=0)
        let ecx = emu.get_reg(RegisterX86::ECX) as u32;
        let exp = cm::equip_validity_index(slot, 0);
        assert_eq!(ecx, exp, "equip_validity_index slot={slot} i=0");
        total += 1;
    }
    eprintln!("cm_fanout_g_equip_validity_index: {total} bit-exact (0x478ad0 i+10*slot @ cmp)");
}
