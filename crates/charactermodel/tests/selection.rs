//! Bit-exact tests against `WoW.exe` of the CharSections validity predicate (`0x477c20`), the face
//! and hair region-select guard (`0x477210`, `0x477280`), the geoset-variation count (`0x478540`), the
//! region-record lookup (`0x478740`) and the geoset and region base selection (`0x478660`). They use
//! the same shapes throughout: the DBC record array at `[0xb4231c]`, global-table scans and struct
//! output, with the fields read back from emulator memory so the index, the guard and the select
//! are all compared.
use benilla_re_charactermodel::selection as cm;
use benilla_re_difftest::{Emu, RegisterX86, Wow};

const DBC: u64 = 0xb4231c; // CharSections DBC record array base ptr
const RACEID_TBL: u64 = 0xb4244c; // 0x47a4c0's flat table ptr
const GV_TBL: u64 = 0xc0df78; // geoset-variation table ptr (0x478540)
const GV_CNT: u64 = 0xc0df7c; // ...its count
const RR_TBL: u64 = 0xc0df28; // region-record table ptr (0x478740)
const RR_CNT: u64 = 0xc0df2c; // ...its count

// ── 0x477c20 — CharSections validity predicate (ret 0xc, ecx=this + 3 stack args) ────────────────
//
// raceBlock=arg0 in [0,5); a1=arg1>=0; idx=sex+2*race; cell=[0xb4231c]+8*(5*idx+rb);
// count0=[cell] gate a1<count0 && count0!=0; a2=arg2>=0; count1=[ [cell+4] + 12*a1 ] gate a2<count1 &&
// count1!=0. We place an 8-byte cell array so the flat 5*idx+rb index is exercised, plus a sub-array
// for the count1 deref.
#[test]
fn charsec_predicate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // this struct: race=[+0x18], sex=[+0x1c]
    let this = emu.alloc(0x40);

    // The flat cell array: indexed by n = 5*idx + raceBlock, idx in [0,8) (race<4,sex<2), rb in [0,5)
    // → n in [0, 5*7+4 = 39]; 8 bytes per cell → 40*8.
    let n_cells = 5 * 8; // 40 cells
    let cells = emu.alloc((n_cells as u64) * 8);
    let subs = emu.alloc(0x4000);
    emu.write_u32(DBC, cells as u32);

    // Populate each cell: [cell] = count0 (signed mix), [cell+4] = ptr into subs (or null).
    // sub array at subs + 0x100*n holds count1 values at offset 12*a1.
    for n in 0..n_cells {
        let cell = cells + (n as u64) * 8;
        // count0: mix of -1/0/1/2/3 by n%5, scaled
        let count0 = match n % 5 {
            0 => 0u32,        // count0 == 0 → fail
            1 => 0xffff_ffff, // negative → a1<count0 false for a1>=0
            2 => 1,           // count0=1
            3 => 3,           // count0=3
            _ => 5,           // count0=5
        };
        emu.write_u32(cell, count0);
        // [cell+4] must ALWAYS be a valid pointer: the binary derefs `[ptr + 12*a1]` whenever the
        // count0 + a1 gates pass, and a null ptr there is an unmapped read (not a path the binary
        // takes with real DBC data — the row's sub-array is present whenever count0 > 0). We give a
        // valid sub-array for every cell; the count1 values still cover the 0/negative/positive gates.
        let sub = subs + 0x100 * (n as u64);
        emu.write_u32(cell + 4, sub as u32);
        // count1 values at offset 12*a1 for a1 in 0..8
        for a1 in 0u64..8 {
            let v = match (n + a1 as usize) % 5 {
                0 => 0u32,        // count1==0 → fail
                1 => 0xffff_ffff, // negative
                2 => 2,
                3 => 4,
                _ => 6,
            };
            emu.write_u32(sub + 12 * a1, v);
        }
    }

    let mut total = 0u32;
    for race in 0u32..4 {
        for sex in 0u32..2 {
            emu.write_u32(this + 0x18, race);
            emu.write_u32(this + 0x1c, sex);
            let idx = sex + 2 * race;
            for rb in [0u32, 1, 2, 4, 5, 0xffff_ffff] {
                for a1 in [0u32, 1, 2, 3, 7, 0xffff_ffff] {
                    for a2 in [0u32, 1, 5, 0xffff_ffff] {
                        // compute the fields the binary reads (guard against OOB cell index when rb>=5
                        // — the binary early-outs on rb before touching the cell, so the field reads
                        // are only meaningful when 0<=rb<5; we mirror by reading from a valid cell only
                        // when in range, else pass 0 placeholders, which the rust predicate also rejects
                        // on the rb gate first).
                        let (count0, count1) = if (rb as i32) >= 0 && rb < 5 {
                            let n = (5 * idx + rb) as u64;
                            let cell = cells + n * 8;
                            let c0 = emu.read_u32(cell);
                            let ptr = emu.read_u32(cell + 4);
                            let c1 = if ptr != 0 && (a1 as i32) >= 0 {
                                // 12*a1 read; only meaningful past the a1>=0 & a1<count0 gates but the
                                // rust fn applies those gates itself, so reading here is harmless when
                                // a1 is in [0,8) sub range; for a1 out of range we still index but the
                                // earlier gate rejects.
                                let off = 12u64.wrapping_mul(a1 as u64);
                                if off < 0x100 {
                                    emu.read_u32(u64::from(ptr) + off)
                                } else {
                                    0
                                }
                            } else {
                                0
                            };
                            (c0, c1)
                        } else {
                            (0, 0)
                        };
                        let bin = emu.call(0x477c20, this as u32, 0, &[rb, a1, a2]) & 0xff;
                        let exp = u32::from(cm::charsec_predicate(rb, a1, a2, count0, count1));
                        assert_eq!(
                            bin, exp,
                            "charsec_predicate race={race} sex={sex} rb={rb} a1={a1} a2={a2} \
                             count0={count0:#x} count1={count1:#x}"
                        );
                        total += 1;
                    }
                }
            }
        }
    }
    eprintln!("charsec_predicate: {total} bit-exact");
}

// ── 0x477210 / 0x477280 — face/hair region-select guard (partial slice) ──────────────────────────
//
// idx=sex+2*race; rec=[ [0xb4231c] + 8*(5*idx) + 4 ]; guard `[this+0x24] < [rec+0x4]`
// (signed jl) decides whether the default 0x477070 dispatch is SKIPPED. We run a slice from
// the fn entry to the `jl` (0x47722e / 0x47729e), reading back `ecx`=rec and the flags-deciding
// fields, then diff the computed rec pointer + the guard outcome. The cell at base + 0x28*idx + 4 is
// the CharSections row field +4 (since 8*(5*idx)+4 = 0x28*idx + 4).
fn region_select_one(addr: u64, jl_addr: u64, name: &str) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x80);
    // CharSections rows: stride 0x28, idx in [0,8). field +4 holds a pointer to a "rec" struct.
    let rows = emu.alloc(8 * 0x28);
    let recs = emu.alloc(0x400);
    emu.write_u32(DBC, rows as u32);
    for i in 0u64..8 {
        // rec pointer at row+4, distinct per idx; rec+4 holds a count to compare against [this+0x24]
        let rec = recs + 0x40 * i;
        emu.write_u32(rows + 0x28 * i + 4, rec as u32);
        // rec+0x4 = count: mix of values incl. negative
        let cnt = match i % 4 {
            0 => 0u32,
            1 => 5,
            2 => 0xffff_ffff,
            _ => 0x10,
        };
        emu.write_u32(rec + 0x4, cnt);
    }
    let mut total = 0u32;
    for race in 0u32..4 {
        for sex in 0u32..2 {
            emu.write_u32(this + 0x18, race);
            emu.write_u32(this + 0x1c, sex);
            let idx = (sex + 2 * race) as u64;
            for this24 in [0u32, 1, 4, 5, 0x20, 0xffff_ffff] {
                emu.write_u32(this + 0x24, this24);
                // partial slice: entry → the jl (not executed). After it, ecx=rec, the cmp set flags.
                // `run` does not set ESP (unlike `call`); the fn's `push esi` needs a valid stack, so
                // point ESP into the harness stack region well below the top.
                emu.set_reg(RegisterX86::ESP, 0x7000_0000 + 0x0010_0000 - 0x2000);
                emu.set_reg(RegisterX86::ECX, this);
                emu.run(addr, jl_addr);
                let rec_bin = emu.get_reg(RegisterX86::ECX) as u32;
                let edx_bin = emu.get_reg(RegisterX86::EDX) as u32; // = [this+0x24]
                let rec_exp = emu.read_u32(rows + 0x28 * idx + 4); // [ [0xb4231c]+8*(5*idx)+4 ]
                assert_eq!(rec_bin, rec_exp, "{name} rec race={race} sex={sex}");
                assert_eq!(edx_bin, this24, "{name} edx race={race} sex={sex}");
                // guard outcome: compare via a single-step over the jl to see which path was taken.
                let field4 = emu.read_u32(u64::from(rec_bin) + 4);
                let guard_exp = cm::region_select_guard(this24, field4);
                // confirm the binary's jl: step one instruction; if guard true (jl taken) EIP = the
                // skip target (jl_addr + 2 + rel8); else fall-through (jl_addr + 2).
                let skip_target = skip_branch_target(jl_addr);
                emu.run(jl_addr, if guard_exp { skip_target } else { jl_addr + 2 });
                // (run() asserts EIP halted exactly there; reaching it confirms the branch taken)
                total += 1;
            }
        }
    }
    eprintln!("{name}: {total} bit-exact");
}

/// `jl rel8` at `jl_addr`: target = jl_addr + 2 + (i8)rel8. For 0x47722e the byte is `7c 09`
/// (target 0x477239); for 0x47729e it is `7c 09` (target 0x4772a9). Computed from the rel8 we read.
fn skip_branch_target(jl_addr: u64) -> u64 {
    // we can't read .text here without an Emu; hardcode the two verified rel8 displacements.
    // 0x47722e: 7c 09 → +0x0b from 0x47722e = 0x477239
    // 0x47729e: 7c 09 → +0x0b from 0x47729e = 0x4772a9
    jl_addr + 2 + 0x09
}

#[test]
fn region_select_face() {
    // 0x477210, jl at 0x47722e
    region_select_one(0x477210, 0x47722e, "region_select_face");
}

#[test]
fn region_select_hair() {
    // 0x477280, jl at 0x47729e
    region_select_one(0x477280, 0x47729e, "region_select_hair");
}

// ── 0x478540 — geoset-variation count (global-table scan) ────────────────────────────────────────
//
// scan [0xc0df78] (count [0xc0df7c], stride 0x18); record matches when rec[+4]==[ecx],
// rec[+8]==[ecx+4], rec[+0xc]==[ecx+0x1c]; on match return rec[+0x10] if >0 (signed) else 1; no match
// → 1.
#[test]
fn geoset_variation_count() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let key = emu.alloc(0x40); // ecx = key ptr: [+0]=k0, [+4]=k4, [+0x1c]=k1c
    let n = 6u32;
    let tbl = emu.alloc(u64::from(n) * 0x18);
    emu.write_u32(GV_TBL, tbl as u32);
    emu.write_u32(GV_CNT, n);
    // records: (f4,f8,fc,f10) at rec+0x4/+0x8/+0xc/+0x10
    let recs: [(u32, u32, u32, u32); 6] = [
        (1, 2, 3, 7),           // match returns 7
        (4, 5, 6, 0),           // f10==0 → would return 1
        (7, 8, 9, 0xffff_ffff), // f10 negative → return 1
        (0xa, 0xb, 0xc, 0x20),  // match returns 0x20
        (0, 0, 0, 0x40),        // key (0,0,0)
        (0x10, 0x20, 0x30, 5),  // match returns 5
    ];
    for (i, &(f4, f8, fc, f10)) in recs.iter().enumerate() {
        let r = tbl + (i as u64) * 0x18;
        emu.write_u32(r + 0x4, f4);
        emu.write_u32(r + 0x8, f8);
        emu.write_u32(r + 0xc, fc);
        emu.write_u32(r + 0x10, f10);
    }
    let rec_vec: Vec<(u32, u32, u32, u32)> = recs.to_vec();
    // keys: exact matches, near-misses, no-match
    let keys: [(u32, u32, u32); 8] = [
        (1, 2, 3),
        (4, 5, 6),
        (7, 8, 9),
        (0xa, 0xb, 0xc),
        (0, 0, 0),
        (0x10, 0x20, 0x30),
        (1, 2, 4),    // near-miss (k1c differs)
        (99, 99, 99), // no match
    ];
    let mut total = 0u32;
    for &(k0, k4, k1c) in &keys {
        emu.write_u32(key, k0);
        emu.write_u32(key + 4, k4);
        emu.write_u32(key + 0x1c, k1c);
        let bin = emu.call(0x478540, key as u32, 0, &[]);
        let exp = cm::geoset_variation_count((k0, k4, k1c), &rec_vec);
        assert_eq!(bin, exp, "geoset_variation_count key={:#x?}", (k0, k4, k1c));
        total += 1;
    }
    // also test the empty-table default (count<=0 → 1)
    emu.write_u32(GV_CNT, 0);
    let bin = emu.call(0x478540, key as u32, 0, &[]);
    assert_eq!(bin, 1, "geoset_variation_count empty table → 1");
    total += 1;
    eprintln!("geoset_variation_count: {total} bit-exact");
}

// ── 0x478740 — region-record lookup (global-table scan, returns rec ptr) ──────────────────────────
//
// scan [0xc0df28] (count [0xc0df2c], stride 0x28); record matches when rec[+0]==[ecx],
// rec[+4]==[ecx+4], rec[+8]==[ecx+0x18]; returns the matched record POINTER else 0.
#[test]
fn region_record_lookup() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let key = emu.alloc(0x40); // ecx = key ptr: [+0]=k0, [+4]=k4, [+0x18]=k18
    let n = 6u32;
    let tbl = emu.alloc(u64::from(n) * 0x28);
    emu.write_u32(RR_TBL, tbl as u32);
    emu.write_u32(RR_CNT, n);
    let recs: [(u32, u32, u32); 6] = [
        (1, 2, 3),
        (4, 5, 6),
        (7, 8, 9),
        (0xa, 0xb, 0xc),
        (0, 0, 0),
        (0x10, 0x20, 0x30),
    ];
    for (i, &(f0, f4, f8)) in recs.iter().enumerate() {
        let r = tbl + (i as u64) * 0x28;
        emu.write_u32(r, f0);
        emu.write_u32(r + 0x4, f4);
        emu.write_u32(r + 0x8, f8);
    }
    let rec_vec: Vec<(u32, u32, u32)> = recs.to_vec();
    let keys: [(u32, u32, u32); 8] = [
        (1, 2, 3),
        (4, 5, 6),
        (7, 8, 9),
        (0xa, 0xb, 0xc),
        (0, 0, 0),
        (0x10, 0x20, 0x30),
        (1, 2, 99),   // near-miss
        (99, 99, 99), // no match
    ];
    let mut total = 0u32;
    for &(k0, k4, k18) in &keys {
        emu.write_u32(key, k0);
        emu.write_u32(key + 4, k4);
        emu.write_u32(key + 0x18, k18);
        let bin = emu.call(0x478740, key as u32, 0, &[]);
        let exp_ptr = match cm::region_record_lookup((k0, k4, k18), &rec_vec) {
            Some(i) => tbl as u32 + (i as u32) * 0x28,
            None => 0,
        };
        assert_eq!(
            bin,
            exp_ptr,
            "region_record_lookup key={:#x?}",
            (k0, k4, k18)
        );
        total += 1;
    }
    // empty-table default → 0
    emu.write_u32(RR_CNT, 0);
    let bin = emu.call(0x478740, key as u32, 0, &[]);
    assert_eq!(bin, 0, "region_record_lookup empty table → 0");
    total += 1;
    eprintln!("region_record_lookup: {total} bit-exact");
}

// ── 0x478660 — geoset/region base selection (struct output) ──────────────────────────────────────
//
// arg0>=0 && arg0<=count (count=0x47a4c0(this)=[0xb4244c][sex+2*race], signed)
// → store [esi+0x30]=arg0; rec=0x478740(&[esi+0x18]); if rec!=0 → [esi+0x148]=[rec+0x18]+0x64,
// [esi+0x150]=[rec+0x1c]+0x12c, [esi+0x14c]=[rec+0x20]+0xc8, [esi+0xa]=1. We let 0x47a4c0 + 0x478740
// run for real (set up their tables) and stub the downstream out-calls 0x478220 (pop 1) / 0x4782e0
// (pop 5) so the run reaches `ret`; the stores precede the out-calls, so we read them back after.
#[test]
fn geoset_base_select() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // stub the two downstream compositor out-calls (out of scope): 0x478220 is __thiscall w/ 1 stack
    // arg (push 1); 0x4782e0 takes 5 stack args. Their return value is unused by 0x478660.
    emu.stub(0x478220, 0, 1);
    emu.stub(0x4782e0, 0, 5);

    let this = emu.alloc(0x200); // esi: race=[+0x18], sex=[+0x1c], [+0x30], [+0x3c], [+0x148/14c/150]

    // 0x47a4c0 flat table [0xb4244c][sex+2*race] = the per-(race,sex) bound `count`.
    let raceid = emu.alloc(0x100);
    emu.write_u32(RACEID_TBL, raceid as u32);
    for race in 0u32..4 {
        for sex in 0u32..2 {
            let i = (sex + 2 * race) as u64;
            // bound values: small so we can test arg0 above/below
            emu.write_u32(raceid + 4 * i, 3 + i as u32);
        }
    }

    // 0x478740 region-record table [0xc0df28]: key (race, sex, arg0). We populate matches for the
    // (race,sex,arg0) combos we test so rec!=0 sometimes and rec==0 (no match) other times. There are
    // 4 races * 2 sexes * 2 matched-arg0 = 16 records — the count MUST cover them all (the scan stops
    // at [0xc0df2c]) or late records (race 3) fall outside the scanned range.
    let n = 16u32;
    let rr = emu.alloc(u64::from(n) * 0x28);
    emu.write_u32(RR_TBL, rr as u32);
    emu.write_u32(RR_CNT, n);
    // records keyed (race,sex,arg0) at rec[+0/+4/+8], with region fields at rec[+0x18/+0x1c/+0x20].
    let mut ri = 0u64;
    let mut rr_records: Vec<(u32, u32, u32, u32, u32, u32)> = Vec::new(); // (k0,k4,k8, f18,f1c,f20)
    for race in 0u32..4 {
        for sex in 0u32..2 {
            // give a matching record for arg0 == 1 and arg0 == 2 for each (race,sex)
            for arg0 in [1u32, 2] {
                let r = rr + ri * 0x28;
                emu.write_u32(r, race);
                emu.write_u32(r + 0x4, sex);
                emu.write_u32(r + 0x8, arg0);
                let f18 = 0x100 + ri as u32;
                let f1c = 0x200 + ri as u32;
                let f20 = 0x300 + ri as u32;
                emu.write_u32(r + 0x18, f18);
                emu.write_u32(r + 0x1c, f1c);
                emu.write_u32(r + 0x20, f20);
                rr_records.push((race, sex, arg0, f18, f1c, f20));
                ri += 1;
            }
        }
    }

    let mut total = 0u32;
    for race in 0u32..4 {
        for sex in 0u32..2 {
            emu.write_u32(this + 0x18, race);
            emu.write_u32(this + 0x1c, sex);
            // [esi+0x3c] != 0 so the 0x4782e0 calls are never reached (jne ret); [esi+0x30] cleared.
            emu.write_u32(this + 0x3c, 1);
            let count = emu.read_u32(raceid + 4 * (sex + 2 * race) as u64);
            for arg0 in [0u32, 1, 2, 3, 4, 0xffff_ffff] {
                // clear the store targets so we can detect "not written" deterministically.
                emu.write_u32(this + 0x30, 0xdead_0030);
                emu.write_u32(this + 0x148, 0xdead_0148);
                emu.write_u32(this + 0x14c, 0xdead_014c);
                emu.write_u32(this + 0x150, 0xdead_0150);
                emu.clear_calls();

                // run the whole fn (out-calls stubbed) → ret
                let _ = emu.call(0x478660, this as u32, 0, &[arg0, 0u32]); // flag=0 anyway

                // expected: count from 0x47a4c0; rec from 0x478740 key (race,sex,arg0) IF gate passes
                // (the store [esi+0x30]=arg0 happens before the 0x478740 call, so the key's [+0x18] =
                // arg0). Gate: 0<=arg0 && arg0<=count.
                let gate = (arg0 as i32) >= 0 && (arg0 as i32) <= (count as i32);
                let rec = if gate {
                    rr_records
                        .iter()
                        .find(|&&(k0, k4, k8, ..)| k0 == race && k4 == sex && k8 == arg0)
                        .map(|&(_, _, _, f18, f1c, f20)| (f18, f1c, f20))
                } else {
                    None
                };
                let exp = cm::geoset_base_select(arg0, count, rec);

                let stored30 = emu.read_u32(this + 0x30);
                let d148 = emu.read_u32(this + 0x148);
                let d14c = emu.read_u32(this + 0x14c);
                let d150 = emu.read_u32(this + 0x150);

                if exp.stored_arg {
                    assert_eq!(
                        stored30, arg0,
                        "geoset_base_select [esi+0x30] race={race} sex={sex} arg0={arg0} cnt={count}"
                    );
                } else {
                    assert_eq!(
                        stored30, 0xdead_0030,
                        "geoset_base_select [esi+0x30] unchanged race={race} sex={sex} arg0={arg0} cnt={count}"
                    );
                }
                match exp.dest_148 {
                    Some(v) => assert_eq!(d148, v, "dest_148 race={race} sex={sex} arg0={arg0}"),
                    None => assert_eq!(
                        d148, 0xdead_0148,
                        "dest_148 unchanged r={race} s={sex} a={arg0}"
                    ),
                }
                match exp.dest_14c {
                    Some(v) => assert_eq!(d14c, v, "dest_14c race={race} sex={sex} arg0={arg0}"),
                    None => assert_eq!(
                        d14c, 0xdead_014c,
                        "dest_14c unchanged r={race} s={sex} a={arg0}"
                    ),
                }
                match exp.dest_150 {
                    Some(v) => assert_eq!(d150, v, "dest_150 race={race} sex={sex} arg0={arg0}"),
                    None => assert_eq!(
                        d150, 0xdead_0150,
                        "dest_150 unchanged r={race} s={sex} a={arg0}"
                    ),
                }
                total += 1;
            }
        }
    }
    eprintln!("geoset_base_select: {total} bit-exact");
}
