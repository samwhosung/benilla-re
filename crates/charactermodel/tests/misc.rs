//! Bit-exact tests against `WoW.exe` of the TSHashTable bucket and rehash index arithmetic, the
//! region-layer-clear flat index, the two hand-grip argument sets, the helmet and region `1<<race`
//! bit test, the equip-filename gender patch and the texture-composite layer case-1 doubling and
//! handle. The hash key, create and insert, realloc, sprintf, path canonicalization and model calls
//! are not modelled; a test either runs a slice that halts where the index or value is in a register
//! or written to the struct, or runs the function whole with the calls stubbed and reads the cleared
//! or written slots.
use benilla_re_charactermodel::misc as h;
use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// A stack/ebp base well inside the mapped harness stack (`0x7000_0000..+0x10_0000`), with room below
/// for the `[ebp-N]` locals a sliced fn reads and the pushes it makes before the capture point.
const SLICE_ESP: u64 = 0x7000_0000 + 0x8_0000;

// ── 0x47ae30 — TSHashTable get-or-create: bucket index `key & mask`, cell `base+12*idx+4` ──────
// The hash key is the return of the call 0x64af90 (not modelled); rather than stub it, the slice starts past it,
// starting at 0x47ae48 with EAX = the chosen key (the fn does `mov ebx,eax` there, so EAX is the key).
// Set the global mask `[0xb4274c]` (!= -1 so the create path is skipped) and bucket base `[0xb42744]`,
// set EBP (the fn writes `[ebp-0x4]=ebx`), then run 0x47ae48 .. 0x47ae69 (after `lea eax,[esi+4*eax+4]`)
// and read EAX = base + 12*(key&mask) + 4.
#[test]
fn cm_fanout_h_hashtable_bucket() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let base = emu.alloc(0x4000) as u32; // bucket array (12*idx+4 stays well inside)
    let mut n = 0u32;
    for &mask in &[0x3u32, 0x7, 0xf, 0x1f, 0xff, 0x1ff] {
        for &key in &[0u32, 1, 2, 3, 7, 0x10, 0x11, 0xabcd, 0x12345, 0xffff_ffff] {
            emu.write_u32(0xb4274c, mask); // mask = capacity-1
            emu.write_u32(0xb42744, base); // bucket base ([0xb42744] → esi)
            emu.set_reg(RegisterX86::EAX, u64::from(key)); // eax = hash key (→ ebx at 0x47ae51)
            emu.set_reg(RegisterX86::EBP, SLICE_ESP); // ebp for the `[ebp-0x4]=ebx` local write
            emu.set_reg(RegisterX86::ESP, SLICE_ESP - 0x100);
            emu.run(0x47ae48, 0x47ae69);
            let bin_addr = emu.get_reg(RegisterX86::EAX) as u32;
            let (_idx, exp_cell4) = h::hashtable_bucket(key, mask, base);
            assert_eq!(
                bin_addr, exp_cell4,
                "hashtable_bucket key={key:#x} mask={mask:#x}"
            );
            n += 1;
        }
    }
    eprintln!("cm_fanout_h_hashtable_bucket: {n} bit-exact (0x47ae30 base + 12*(key&mask) + 4)");
}

// ── 0x47b7d0 — TSHashTable rehash: new-mask `arg-1` + re-bucket `base+12*(key&mask)` ─────────────────
#[test]
fn cm_fanout_h_hashtable_rehash() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40) as u32;
    let base = emu.alloc(0x4000) as u32; // re-bucket bucket array [this+0x1c]
    let node = emu.alloc(0x10) as u32; // a link node; [node] is the key
    let ebp = SLICE_ESP; // ebp for the [ebp-0x4]=this local

    let mut n = 0u32;
    // ---- new-mask: 0x47b955 .. 0x47b95c reads [ebp-0x4]=this, ebx=arg, stores [this+0x24]=arg-1 ----
    for &arg in &[1u32, 2, 3, 4, 8, 0x10, 0x100, 0x1000, 0xffff_ffff] {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.write_u32(ebp - 0x4, this); // [ebp-0x4] = this
        emu.set_reg(RegisterX86::EBX, u64::from(arg)); // ebx = requested new capacity
        emu.write_u32(u64::from(this) + 0x24, 0xdead_beef); // poison so a missed store diverges
        emu.set_reg(RegisterX86::ESP, ebp - 0x100);
        emu.run(0x47b955, 0x47b95c);
        let bin_mask = emu.read_u32(u64::from(this) + 0x24);
        assert_eq!(
            bin_mask,
            h::hashtable_new_mask(arg),
            "hashtable_new_mask arg={arg:#x}"
        );
        n += 1;
    }

    // ---- re-bucket: 0x47b96c .. 0x47b97f reads [ebp-0x4]=this, [ecx]=key, [this+0x24]=mask,
    //      [this+0x1c]=base; leaves ebx = base + 12*(key&mask). ----
    for &mask in &[0x3u32, 0x7, 0xf, 0x1f, 0xff] {
        for &key in &[0u32, 1, 3, 7, 0x10, 0x11, 0xabcd, 0x123456, 0xffff_ffff] {
            emu.set_reg(RegisterX86::EBP, ebp);
            emu.write_u32(ebp - 0x4, this); // [ebp-0x4] = this
            emu.write_u32(u64::from(this) + 0x24, mask); // [this+0x24] = new mask
            emu.write_u32(u64::from(this) + 0x1c, base); // [this+0x1c] = bucket base
            emu.write_u32(u64::from(node), key); // [node] = key
            emu.set_reg(RegisterX86::ECX, u64::from(node)); // ecx = link node
            emu.set_reg(RegisterX86::ESP, ebp - 0x100);
            emu.run(0x47b96c, 0x47b97f);
            let bin_cell = emu.get_reg(RegisterX86::EBX) as u32;
            let (_idx, exp_cell) = h::hashtable_rebucket(key, mask, base);
            assert_eq!(
                bin_cell, exp_cell,
                "hashtable_rebucket key={key:#x} mask={mask:#x}"
            );
            n += 1;
        }
    }
    eprintln!("cm_fanout_h_hashtable_rehash: {n} bit-exact (0x47b7d0 new-mask arg-1 + base+12*(key&mask))");
}

// ── 0x479180 — region-layer clear: flat index (col<<4)+row + the +0x228/+0x248 parallel arrays ──────
// struct output: stub the free 0x47b010, full-call, assert both slots for the computed index were
// cleared and no neighbour slot moved (so the index is exercised).
#[test]
fn cm_fanout_h_region_layer_clear() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x47b010, 0, 0); // the free (thiscall, no stack args) is not modelled: record only
                              // this needs to span +0x228 + 4*edi for the largest edi we test ((col<<4)+row).
    let this = emu.alloc(0x4000) as u32;

    // The two written slots are `[this + 0x228 + 4*edi]` and `[this + 0x248 + 4*edi]`. Since
    // `0x248 = 0x228 + 0x20` (8 dwords), the second is index `edi+8` of one array based at +0x228 — i.e.
    // the fn clears indices `edi` and `edi+8` of the +0x228 array. Poison the whole array, run, assert
    // those two indices are zero and a sample of untouched indices keeps its poison (so the flat index
    // `(col<<4)+row` is exercised, not a coincidental zero).
    let mut n = 0u32;
    for &(col, row) in &[
        (0u32, 0u32),
        (0, 1),
        (1, 0),
        (1, 5),
        (2, 3),
        (3, 0xf),
        (5, 7),
        (0xf, 0xf),
    ] {
        let edi = h::region_layer_clear_index(col, row);
        let cap = edi + 0x20; // poison comfortably past both touched indices
        for i in 0..=cap {
            emu.write_u32(u64::from(this) + 0x228 + 4 * u64::from(i), 0xa000_0000 + i);
        }
        emu.call(0x479180, this, 0, &[col, row]);
        // indices edi and edi+8 (= +0x228+4*edi and +0x248+4*edi) must be cleared.
        assert_eq!(
            emu.read_u32(u64::from(this) + 0x228 + 4 * u64::from(edi)),
            0,
            "region_layer_clear +0x228 (idx {edi}) col={col} row={row}"
        );
        assert_eq!(
            emu.read_u32(u64::from(this) + 0x228 + 4 * u64::from(edi + 8)),
            0,
            "region_layer_clear +0x248 (idx {}) col={col} row={row}",
            edi + 8
        );
        // a sample of indices the fn must NOT touch keep their poison.
        for &i in &[edi.wrapping_sub(1), edi + 1, edi + 7, edi + 9, edi + 0x10] {
            if i <= cap && i != edi && i != edi + 8 {
                assert_eq!(
                    emu.read_u32(u64::from(this) + 0x228 + 4 * u64::from(i)),
                    0xa000_0000 + i,
                    "region_layer_clear stray idx={i} col={col} row={row}"
                );
            }
        }
        n += 1;
    }
    eprintln!(
        "cm_fanout_h_region_layer_clear: {n} bit-exact (0x479180 (col<<4)+row → +0x228/+0x248)"
    );
}

// ── 0x479660 / 0x4796b0 — hand-grip arm/release: capture the args pushed to op4 0x7121a0 ─────────────
// The per-hand weapon grip (not a geoset): `0x7121a0` is the per-bone animation call, and
// {8..=12} and {13..=17} are finger key-bone ids. Arm pushes animId 0xf (15, "HandsClosed"); release
// pushes -1 (disarm). `0x7121a0` is stubbed (fastcall, ecx = this, 7 stack dwords cleaned: ret 0x1c,
// pop 7). The function runs whole; the recorded args are the literal arg vector per key-bone over
// the selected range.
#[test]
fn cm_fanout_h_hand_grip_arm() {
    hand_grip_check("0x479660 arm", 0x479660, &h::hand_grip_arm_args);
}
#[test]
fn cm_fanout_h_hand_grip_release() {
    hand_grip_check("0x4796b0 release", 0x4796b0, &h::hand_grip_release_args);
}

fn hand_grip_check(tag: &str, addr: u64, expect_args: &dyn Fn(u32) -> [u32; 7]) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // op4 0x7121a0(keyBoneId, animId, -1, ?, 0x3f800000, ?, 1): __fastcall ecx=this, 7 stack dwords → ret 0x1c.
    emu.stub(0x7121a0, 0, 7);
    let this = emu.alloc(0x10) as u32;

    let mut n = 0u32;
    for &hand in &[0u32, 1, 2, 0xff] {
        let (lo, hi) = h::finger_keybone_range(hand);
        emu.clear_calls();
        emu.call(addr, this, hand, &[]);
        let calls: Vec<_> = emu
            .calls()
            .iter()
            .filter(|c| c.addr == 0x7121a0)
            .copied()
            .collect();
        // one op4 call per key-bone id in [lo..=hi] inclusive, in order.
        let ids: Vec<u32> = (lo..=hi).collect();
        assert_eq!(calls.len(), ids.len(), "{tag} hand={hand}: call count");
        for (c, &id) in calls.iter().zip(ids.iter()) {
            let exp = expect_args(id);
            // args[0..7] are [esp+4..esp+0x1c] = the 7 pushed dwords (keyBoneId, animId, -1, ?, 0x3f800000, ?, 1).
            for (k, &want) in exp.iter().enumerate() {
                assert_eq!(c.args[k], want, "{tag} hand={hand} keyBone={id} arg[{k}]");
            }
            n += 1;
        }
    }
    eprintln!(
        "cm_fanout_h_hand_grip ({tag}): {n} key-bones bit-exact (range + literal op4 args to 0x7121a0)"
    );
}

// ── 0x4799a0 — helmet/region geoset-flag resolver: 1<<race bit-test + 5 region constants ─────────────
// SLICE 0x479ade .. 0x479b33 (the bit-test tail, after the string/sprintf out-calls). Set esi=this,
// edx=rec, [this+0x18]=race; pre-poison [this+0x144..]; read back the written geoset fields.
#[test]
fn cm_fanout_h_helmet_geoset_flags() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x400) as u32;
    let rec = emu.alloc(0x40) as u32;
    let offsets = [0x144u64, 0x148, 0x14c, 0x150, 0x160];

    let mut n = 0u32;
    for &race in &[0u32, 1, 2, 3, 6, 7, 0x1f] {
        // sweep the 5 region-flag words across set/clear at the race bit (and noise in other bits).
        for &pattern in &[0u32, 0x1f, 0xffff_ffff, 0x5555_5555, 0xaaaa_aaaa, 0x80] {
            let mask = 1u32 << (race & 0x1f);
            // build the 5 fields: some have the race bit, some not, plus the pattern.
            let f4 = pattern;
            let f8 = pattern ^ mask;
            let fc = mask | (pattern & !mask);
            let f10 = pattern & !mask;
            let f14 = mask;
            emu.write_u32(u64::from(rec) + 0x4, f4);
            emu.write_u32(u64::from(rec) + 0x8, f8);
            emu.write_u32(u64::from(rec) + 0xc, fc);
            emu.write_u32(u64::from(rec) + 0x10, f10);
            emu.write_u32(u64::from(rec) + 0x14, f14);
            emu.write_u32(u64::from(this) + 0x18, race);
            // poison the 5 geoset fields so an un-written one stays != its constant.
            for (i, &off) in offsets.iter().enumerate() {
                emu.write_u32(u64::from(this) + off, 0xc000_0000 + i as u32);
            }
            emu.set_reg(RegisterX86::ESI, u64::from(this));
            emu.set_reg(RegisterX86::EDX, u64::from(rec));
            emu.set_reg(RegisterX86::ESP, SLICE_ESP);
            emu.run(0x479ade, 0x479b33);

            let exp = h::helmet_geoset_flags(race, (f4, f8, fc, f10, f14));
            for (i, &off) in offsets.iter().enumerate() {
                let got = emu.read_u32(u64::from(this) + off);
                match exp[i] {
                    Some((eoff, val)) => {
                        assert_eq!(eoff as u64, off, "helmet offset table mismatch i={i}");
                        assert_eq!(
                            got, val,
                            "helmet_geoset_flags race={race} pat={pattern:#x} field +{off:#x} written"
                        );
                    }
                    None => assert_eq!(
                        got,
                        0xc000_0000 + i as u32,
                        "helmet_geoset_flags race={race} pat={pattern:#x} field +{off:#x} untouched"
                    ),
                }
            }
            n += 1;
        }
    }
    eprintln!(
        "cm_fanout_h_helmet_geoset_flags: {n} bit-exact (0x4799a0 1<<race → 5 region consts)"
    );
}

// ── 0x476e20 — equip filename: table addrs (sprintf args) + gender-letter patch ──────────────────────
#[test]
fn cm_fanout_h_equip_filename() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // ---- table addrs: slice 0x476e2a .. 0x476e4a leaves EAX=[0x838a94+4*disp loaded value],
    //      EDX=[arg0+4*disp+0x38 loaded value]. We instead verify the ADDRESS math by planting unique
    //      markers at those table cells and asserting the loaded values match. ----
    let arg0 = emu.alloc(0x4000) as u32; // [ebp+8] per-record base
    let ebp = SLICE_ESP;
    let mut n = 0u32;
    for &disp in &[0u32, 1, 2, 5, 0x10, 0x40] {
        let (addr0, addr1) = h::equip_filename_table_addrs(arg0, disp);
        // plant unique sentinels at the two computed table cells, read them back via the binary.
        let v0 = 0x1111_0000 + disp;
        let v1 = 0x2222_0000 + disp;
        emu.write_u32(u64::from(addr0), v0); // [0x838a94 + 4*disp]
        emu.write_u32(u64::from(addr1), v1); // [arg0 + 4*disp + 0x38]
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, ebp - 0x100);
        emu.set_reg(RegisterX86::ECX, u64::from(arg0)); // [ebp+8] read into ecx then indexed
        emu.write_u32(ebp + 0x8, arg0); // [ebp+8]  = arg0 (per-record base)
        emu.write_u32(ebp + 0xc, disp); // [ebp+0xc] = displayId
        emu.run(0x476e2a, 0x476e4a); // through both table loads; EAX=tbl0, EDX=tbl1
        assert_eq!(
            emu.get_reg(RegisterX86::EAX) as u32,
            v0,
            "equip tbl0 disp={disp}"
        );
        assert_eq!(
            emu.get_reg(RegisterX86::EDX) as u32,
            v1,
            "equip tbl1 disp={disp}"
        );
        n += 1;
    }

    // ---- gender-letter patch: slice 0x476e6d .. 0x476e92. [0xb42510] = canon path, [this+0x1c]=idx,
    //      [0x838ab4+4*idx] → string whose first byte is the suffix letter; patched byte at
    //      [0xb42510 + strlen - 5] = that letter. eax(al) must be 0 (scasb terminator). ----
    let this = emu.alloc(0x40) as u32;
    let suffix_strs = emu.alloc(0x200) as u32; // pool of one-char suffix strings
    for &(idx, path, letter) in &[
        (0u32, &b"Human_Male.blp\0"[..], b'M'),
        (1u32, &b"Human_Female.blp\0"[..], b'F'),
        (2u32, &b"Orc.blp\0"[..], b'X'), // len 7: 7-5=2 (patch at index 2)
        (3u32, &b"abcdef\0"[..], b'Z'),  // len 6: patch at index 1
        (5u32, &b"NightElf_Hd.tga\0"[..], b'Q'),
    ] {
        // suffix string for this idx: a 2-byte "<letter>\0" planted at suffix_strs + 8*idx.
        let sptr = suffix_strs + 8 * idx;
        emu.write(u64::from(sptr), &[letter, 0]);
        emu.write_u32(0x838ab4 + 4 * u64::from(idx), sptr); // [0x838ab4 + 4*idx] = &suffix
        emu.write_u32(u64::from(this) + 0x1c, idx); // [this+0x1c] = sex/race index
        emu.write(0xb42510, path); // the canon buffer
        emu.set_reg(RegisterX86::ESI, u64::from(this)); // esi = this
        emu.set_reg(RegisterX86::EAX, 0); // al = 0 for the scasb null match
        emu.set_reg(RegisterX86::ESP, SLICE_ESP); // valid stack for the push edi
        emu.run(0x476e6d, 0x476e92);

        let strlen = (path.len() - 1) as u32; // bytes before the NUL
        let patch_at = h::equip_gender_patch_offset(strlen);
        let patched = emu.read(0xb42510 + u64::from(patch_at), 1)[0];
        assert_eq!(
            patched, letter,
            "equip gender patch idx={idx} path={path:?}"
        );
        n += 1;
    }
    eprintln!(
        "cm_fanout_h_equip_filename: {n} bit-exact (0x476e20 table addrs + gender patch byte)"
    );
}

// ── 0x477a70 — texture-composite layer callback case 1: 2*edx + handle-table lookup ─────────────────
// struct output: ecx=1 (case 1), edx=region index, stack args [.., idx@+0x10, .., outA@+0x18, outB@+0x1c].
// [0xb422d8] = scratch handle table. Full call (ret 0x18, no out-calls in case 1); read *outA, *outB.
#[test]
fn cm_fanout_h_texture_layer_case1() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let handle_table = emu.alloc(0x400) as u32; // [0xb422d8] base
    let out_a = emu.alloc(0x10) as u32; // [ebp+0x18] out-ptr
    let out_b = emu.alloc(0x10) as u32; // [ebp+0x1c] out-ptr
    emu.write_u32(0xb422d8, handle_table);

    // seed the handle table with distinct values.
    let count = 16usize;
    let mut table = vec![0u32; count];
    for (i, v) in table.iter_mut().enumerate() {
        *v = 0x4848_0000 + i as u32;
        emu.write_u32(u64::from(handle_table) + 4 * i as u64, *v);
    }

    let mut n = 0u32;
    for &edx in &[0u32, 1, 2, 5, 0x100, 0x8000_0001, 0xffff_ffff] {
        for index in 0u32..count as u32 {
            // poison the out slots so a missed write would diverge.
            emu.write_u32(u64::from(out_a), 0xdead_0001);
            emu.write_u32(u64::from(out_b), 0xdead_0002);
            // stack args: [0]=+8, [1]=+0xc, [2]=+0x10 (table idx), [3]=+0x14, [4]=+0x18 (outA), [5]=+0x1c (outB)
            emu.call(
                0x477a70,
                1, // ecx = case 1
                edx,
                &[0, 0, index, 0, out_a, out_b],
            );
            let (exp_double, exp_handle) = h::texture_layer_case1(edx, &table, index);
            assert_eq!(
                emu.read_u32(u64::from(out_a)),
                exp_double,
                "texture_layer_case1 *outA edx={edx:#x} idx={index}"
            );
            assert_eq!(
                emu.read_u32(u64::from(out_b)),
                exp_handle,
                "texture_layer_case1 *outB edx={edx:#x} idx={index}"
            );
            n += 1;
        }
    }
    eprintln!(
        "cm_fanout_h_texture_layer_case1: {n} bit-exact (0x477a70 case1 2*edx + handle[idx])"
    );
}

// ── 0x4799a0 head — the helm geoset-vis PRECONDITION (five early-outs → the epilogue) ────────────────
// struct output plus the halt address, so the branch taken is tested, not just the tail arithmetic. Slice `[0x4799ac, ...)`
// (skipping only the prologue, which the slice has no frame for): with the 5 region-base slots poisoned
// and a HelmetGeosetVisData record whose 5 masks are all-ones (so the tail WOULD write every slot for
// any race), each early-out case must halt at the epilogue 0x479b33 with all 5 slots UNTOUCHED, and the
// pass case must halt at 0x4799ef (the filename-build block). Non-vacuity is two-sided: (a) the pass
// case then runs the tail on the SAME state and every slot IS written, so "untouched" is a real skip,
// and (b) the harness asserts the halt EIP, so a case taking the other path faults instead of passing.
#[test]
fn cm_fanout_h_helmet_head_gate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x600) as u32; // cc
    let rec = emu.alloc(0x60) as u32; // ItemDisplayInfo in-mem record (stride 0x5c)
    let vis = emu.alloc(0x18) as u32; // HelmetGeosetVisData record (stride 0x18)
    let vis_idx = emu.alloc(0x40) as u32; // its idIndex table
    let race_idx = emu.alloc(0x40) as u32; // the race-DBC idIndex table [0xc0dee0]
    let race_row = emu.alloc(0x40) as u32;
    let s_empty = emu.alloc(0x10) as u32; // ""
    let s_named = emu.alloc(0x10) as u32; // "H.mdx"
    emu.write(u64::from(s_empty), b"\0");
    emu.write(u64::from(s_named), b"H.mdx\0");

    let offsets = [0x144u64, 0x148, 0x14c, 0x150, 0x160];
    let consts = [0x1u32, 0x65, 0xc9, 0x12d, 0x2bd];
    const ROW: u32 = 3; // the HelmetGeosetVisData row id the head record names
    const RACE: i32 = 7; // gnome — a race whose bit is set in the all-ones masks

    // the vis record: all five race masks all-ones ⇒ the tail writes all five slots for ANY race.
    for i in 0..5u64 {
        emu.write_u32(u64::from(vis) + 4 + i * 4, 0xffff_ffff);
    }
    emu.write_u32(u64::from(vis_idx) + u64::from(ROW) * 4, vis);
    emu.write_u32(0xc0dc4c, vis_idx); // HelmetGeosetVisData idIndexPtr
    emu.write_u32(0xc0dc50, 0x20); // ... and its maxId

    // (case, head-row ptr, ModelName[0] ptr, race, race-DBC maxId, race row ptr, reaches-tail)
    let cases: &[(&str, u32, u32, i32, i32, u32, bool)] = &[
        ("rec-null", 0, 0, RACE, 0x20, race_row, false),
        ("model-empty", rec, s_empty, RACE, 0x20, race_row, false),
        ("race-negative", rec, s_named, -1, 0x20, race_row, false),
        ("race-over-max", rec, s_named, RACE, 2, race_row, false),
        ("race-row-null", rec, s_named, RACE, 0x20, 0, false),
        ("model-named", rec, s_named, RACE, 0x20, race_row, true),
    ];

    let mut n = 0u32;
    for &(name, head_ptr, model_ptr, race, race_max, row_ptr, exp_tail) in cases {
        emu.write_u32(u64::from(this) + 0x4a8, head_ptr); // cc+0x4a8 = head ItemDisplayInfo row
        emu.write_u32(u64::from(this) + 0x18, race as u32); // cc+0x18 = race
        emu.write_u32(u64::from(this) + 0x1c, 0); // cc+0x1c = sex (male → rec+0x30)
        emu.write_u32(u64::from(rec) + 0x4, model_ptr); // rec+0x04 = ModelName[0] (col 1)
        emu.write_u32(u64::from(rec) + 0x30, ROW); // rec+0x30 = HelmVis[male] (col 12)
        emu.write_u32(0xc0dee4, race_max as u32); // race-DBC maxId
        emu.write_u32(0xc0dee0, race_idx); // race-DBC idIndexPtr
        emu.write_u32(u64::from(race_idx) + (race.max(0) as u64) * 4, row_ptr);
        for (i, &off) in offsets.iter().enumerate() {
            emu.write_u32(u64::from(this) + off, 0xc000_0000 + i as u32);
        }
        emu.set_reg(RegisterX86::ESI, u64::from(this));
        emu.set_reg(RegisterX86::ESP, SLICE_ESP);
        emu.set_reg(RegisterX86::EBP, SLICE_ESP);

        let head = if head_ptr == 0 {
            h::HeadDisplay::Absent
        } else {
            h::HeadDisplay::Present(emu.read(u64::from(model_ptr), 1)[0])
        };
        let exp = h::helmet_vis_reaches_tail(head, race, race_max, row_ptr != 0);
        assert_eq!(exp, exp_tail, "{name}: case table disagrees with the model");

        if exp_tail {
            // reaches the filename-build block: the gate passed.
            emu.run(0x4799ac, 0x4799ef);
            for (i, &off) in offsets.iter().enumerate() {
                assert_eq!(
                    emu.read_u32(u64::from(this) + off),
                    0xc000_0000 + i as u32,
                    "{name}: slot +{off:#x} written before the tail"
                );
            }
            // non-vacuity: the tail, run on this exact state, DOES write every slot.
            emu.set_reg(RegisterX86::ESI, u64::from(this));
            emu.set_reg(RegisterX86::ESP, SLICE_ESP);
            emu.run(0x479ab8, 0x479b33);
            for (i, &off) in offsets.iter().enumerate() {
                assert_eq!(
                    emu.read_u32(u64::from(this) + off),
                    consts[i],
                    "{name}: tail did not write slot +{off:#x}"
                );
            }
        } else {
            // every early-out jumps to the shared epilogue, skipping the whole tail.
            emu.run(0x4799ac, 0x479b33);
            for (i, &off) in offsets.iter().enumerate() {
                assert_eq!(
                    emu.read_u32(u64::from(this) + off),
                    0xc000_0000 + i as u32,
                    "{name}: vis tail RAN — slot +{off:#x} was written"
                );
            }
        }
        n += 1;
    }
    eprintln!(
        "cm_fanout_h_helmet_head_gate: {n} bit-exact (0x4799a0 head early-outs → 0x479b33, tail skipped)"
    );
}
