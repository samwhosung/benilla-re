//! The realm list's load statistics and bands (`systems/glue/realms.md`).

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_glue::realm::{realm_load_classify, realm_load_stats};
use common::*;

/// `GetRealmLoadBand`, and the realm fields it reads: the flags byte at `+8` and the population at
/// `+0x12c`. The statistics it reads, and `RebuildRealmList` writes: the mean and the deviation.
const REALM_CLASSIFY: u64 = 0x46ec60;
const REALM_FLAGS_OFF: u64 = 0x8;
const REALM_POP_OFF: u64 = 0x12c;
const REALM_MEAN: u64 = 0x00b4_1fb0;
const REALM_STDDEV: u64 = 0x00b4_1fb4;
/// `RebuildRealmList`; `GetRealmInfoByIndex`, which the test patches to index a synthesized array; the
/// realm holder, the category count and the selected category it reads.
const REALM_STATS: u64 = 0x46e510;
const GET_REALM: u64 = 0x5ab930;
const REALM_SEL_CAT: u64 = 0x00c2_8118;
const REALM_CAT_COUNT: u64 = 0x00b4_1f78;
/// The byte at `+0x131` of a realm that makes the statistics loop take it as it is, and the size of
/// a realm record.
const FLAG_OFF: u64 = 0x131;
const STRIDE: u32 = 0x140;
/// The helpers `RebuildRealmList` calls, stubbed: a name hash (pops 8), a name lookup (pops 4), the
/// hook after the statistics, a context getter and a refresh.
const STR_HASH: u64 = 0x5ab7d0;
const STR_FIND: u64 = 0x64a4c0;
const STATS_POST: u64 = 0x46e750;
const GET_CTX: u64 = 0x5ab490;
const REFRESH: u64 = 0x703e50;

/// Runs `GetRealmLoadBand` (`0x46ec60`) whole, with the realm in `ecx` and the statistics in the two
/// globals, and reads `st0`. Half the cases have none of the flag bits `0xe0` and populations around
/// `mean ± deviation`; the others fuzz the flags.
#[test]
fn realm_load_classify_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let realm = e.alloc(0x140);

    let mut s: u32 = 0x2bd1_0001;
    for i in 0..12000u32 {
        let mean = fuzz_unit(&mut s, 4.0);
        let stddev = fuzz_unit(&mut s, 1.5);
        // Half the cases clear the flag bits 0xe0 and put the population around the band; the
        // others fuzz the flags.
        let band = (next(&mut s) >> 8) & 1 == 0;
        let flags: u8 = if band {
            (next(&mut s) >> 8) as u8 & !0xe0
        } else {
            (next(&mut s) >> 8) as u8
        };
        let pop = if band {
            // below, inside and above the band
            mean + (fuzz_unit(&mut s, 4.0) - 2.0)
        } else {
            fuzz_unit(&mut s, 4.0)
        };
        e.write_u32(realm + REALM_FLAGS_OFF, u32::from(flags)); // the low byte
        e.write_u32(realm + REALM_POP_OFF, pop.to_bits());
        e.write_u32(REALM_MEAN, mean.to_bits());
        e.write_u32(REALM_STDDEV, stddev.to_bits());

        let bin = e.call_f32(REALM_CLASSIFY, realm as u32, 0, &[]);
        assert_f32(
            bin,
            realm_load_classify(flags, pop, mean, stddev),
            &format!("realm_load_classify[{i}] flags={flags:#x} pop={pop} mean={mean} sd={stddev}"),
        );
    }
}

/// Runs `RebuildRealmList` (`0x46e510`) whole on a synthesized array of realms, with
/// `GetRealmInfoByIndex` (`0x5ab930`) patched to index it, no categories (so that the sort and selection
/// are skipped) and the other helpers stubbed, and compares the mean and the deviation it writes. The
/// counts include 0 and 1.
#[test]
fn realm_load_stats_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let category = e.alloc(0x40);
    let base = e.alloc(0x4000); // the realm array
    let ctx = e.alloc(0x2000); // returned by the context getter, zeroed

    // `GetRealmInfoByIndex` becomes `imul eax,ecx,STRIDE; add eax,base; ret`.
    let mut patch = vec![0x69, 0xc1];
    patch.extend_from_slice(&STRIDE.to_le_bytes());
    patch.push(0x05);
    patch.extend_from_slice(&(base as u32).to_le_bytes());
    patch.push(0xc3);
    e.write(GET_REALM, &patch);

    e.stub(STR_HASH, 0, 2);
    e.stub(STR_FIND, 1, 1); // nonzero, so the selection is skipped
    e.stub(STATS_POST, 0, 0);
    e.stub(GET_CTX, ctx as u32, 0);
    e.stub(REFRESH, 0, 0);

    e.write_u32(REALM_CAT_COUNT, 0); // no categories
    e.write_u32(REALM_SEL_CAT, category as u32);

    let mut s: u32 = 0x6f3e_0001;
    for i in 0..4000u32 {
        // The count: 0 and 1, then 2 to 14.
        let n = match (next(&mut s) >> 8) % 8 {
            0 => 0usize,
            1 => 1,
            _ => 2 + ((next(&mut s) >> 8) % 13) as usize,
        };
        let mut pops = Vec::with_capacity(n);
        for r in 0..n {
            // Small loads, a wider band, and tiny values.
            let pop = match (next(&mut s) >> 8) % 3 {
                0 => fuzz_unit(&mut s, 3.0),
                1 => fuzz_unit(&mut s, 100.0),
                _ => fuzz_unit(&mut s, 0.5),
            };
            pops.push(pop);
            e.write_u32(base + r as u64 * u64::from(STRIDE) + FLAG_OFF, 1);
            e.write_u32(
                base + r as u64 * u64::from(STRIDE) + REALM_POP_OFF,
                pop.to_bits(),
            );
        }
        e.write_u32(category + 0x8, n as u32); // the category's realm count
        e.write_u32(REALM_MEAN, 0xdead_beef);
        e.write_u32(REALM_STDDEV, 0xdead_beef);

        e.call(REALM_STATS, 0, 0, &[]);
        let (mean, stddev) = realm_load_stats(&pops);
        let ctxs = format!("realm_load_stats[{i}] n={n} pops={pops:?}");
        assert_f32(e.read_f32(REALM_MEAN), mean, &format!("{ctxs} mean"));
        assert_f32(e.read_f32(REALM_STDDEV), stddev, &format!("{ctxs} stddev"));
    }
}
