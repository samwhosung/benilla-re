//! The telemetry record append (`systems/net/connection.md#telemetry-record`), run whole beside
//! the model.

mod common;

use benilla_re_difftest::Emu;
use benilla_re_net::{append_telemetry, ArrayCall, RecordArray};
use common::{Lcg, Wow};

const APPEND: u64 = 0x5ab9b0;
const LOADED: u64 = 0xc28135;
const RECORD: u64 = 0xc27e90;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const SMEM_FREE: u64 = 0x646430;
const ARENA: u64 = 0x10_0000;

/// `AppendTelemetryCdKeyInfo` whole: the loaded byte clear and set; counts from 0 to 40 against
/// capacities below, at and above them; growth steps 0, 1, 3, 8 and 16, across the power-of-two
/// rounding below 8; an old block or none; and the in-place reallocation succeeding or failing,
/// which takes the copy into a new block: the return, every field, the records and the allocator
/// calls.
#[test]
fn telemetry_append() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let arena = e.alloc(ARENA);
    let obj = e.alloc(0x20);
    let flag = e.alloc(0x10);
    e.stub_alloc_sized(SMEM_ALLOC, arena, arena + ARENA, 0, 4);
    let real = e.alloc(0x10);
    e.stub_realloc_sized(real, 5);
    e.stub(SMEM_FREE, 1, 4);
    let fail = e.alloc(0x10);
    e.stub(fail, 0, 5);
    // `SMemReAlloc`: goes on to a stub that returns 0 while the byte at `flag` is 0, else to the
    // sized stub.
    let mut c = vec![0x80, 0x3d];
    c.extend((flag as u32).to_le_bytes());
    c.extend([0x00, 0x75, 0x05, 0xe9]);
    let island = e.alloc(0x40);
    let after = island + c.len() as u64 + 4;
    c.extend((fail as u32).wrapping_sub(after as u32).to_le_bytes());
    c.push(0xe9);
    let after = island + c.len() as u64 + 4;
    c.extend((real as u32).wrapping_sub(after as u32).to_le_bytes());
    e.write(island, &c);
    let mut j = vec![0xe9];
    j.extend(
        (island as u32)
            .wrapping_sub(SMEM_REALLOC as u32 + 5)
            .to_le_bytes(),
    );
    e.write(SMEM_REALLOC, &j);
    let mut g = Lcg(0x7e1e_0001);
    for case in 0..6000 {
        e.reset_bump(arena);
        e.clear_calls();
        let count = [0u32, 1, 2, 3, 6, 7, 8, 9, 15, 16, 40][(g.next() >> 16) as usize % 11];
        let capacity = match g.next() >> 30 {
            0 => count,
            1 => count + (g.next() >> 29),
            2 => count.saturating_sub(1 + (g.next() >> 30)),
            _ => count + 1,
        };
        let step = [0u32, 0, 1, 3, 8, 16][(g.next() >> 16) as usize % 6];
        let records: Vec<[u8; 32]> = (0..count)
            .map(|_| std::array::from_fn(|_| (g.next() >> 24) as u8))
            .collect();
        // An array without a block has no capacity.
        let had_block = count != 0 || g.next() >> 31 == 0;
        let capacity = if had_block { capacity } else { 0 };
        let block = if had_block {
            let b = e.call(0x6462e0, 0, 0, &[0x20 * count.max(1), 0, 0, 0]);
            for (i, r) in records.iter().enumerate() {
                e.write(u64::from(b) + 0x20 * i as u64, r);
            }
            b
        } else {
            0
        };
        e.clear_calls();
        let loaded = g.next() >> 29 != 0;
        let record: [u8; 32] = std::array::from_fn(|_| (g.next() >> 24) as u8);
        e.write(LOADED, &[u8::from(loaded)]);
        e.write(RECORD, &record);
        let realloc_ok = g.next() >> 31 == 0;
        e.write(flag, &[u8::from(realloc_ok)]);
        let first = g.next();
        for (i, v) in [first, capacity, count, block, step]
            .into_iter()
            .enumerate()
        {
            e.write_u32(obj + 4 * i as u64, v);
        }
        let got = e.call(APPEND, 0, 0, &[obj as u32]) as u8;
        let mut m = RecordArray {
            flag: first,
            capacity,
            step,
            records,
        };
        let (want, calls) =
            append_telemetry(&mut m, loaded.then_some(record), had_block, realloc_ok);
        let ctx = format!("case {case} count {count} capacity {capacity} step {step} block {had_block} realloc {realloc_ok}");
        assert_eq!(got, want, "{ctx}: return");
        let n = e.read_u32(obj + 8);
        let data = u64::from(e.read_u32(obj + 0xc));
        let back = RecordArray {
            flag: e.read_u32(obj),
            capacity: e.read_u32(obj + 4),
            step: e.read_u32(obj + 0x10),
            records: (0..u64::from(n))
                .map(|i| e.read(data + 0x20 * i, 0x20).try_into().unwrap())
                .collect(),
        };
        assert_eq!(back, m, "{ctx}: array");
        let got_calls: Vec<ArrayCall> = e
            .calls()
            .iter()
            .map(|c| match c.addr {
                SMEM_ALLOC => ArrayCall::Alloc(c.args[0]),
                SMEM_FREE => {
                    assert_eq!(c.args[0], block, "{ctx}: the old block freed");
                    ArrayCall::Free
                }
                a if a == real || a == fail => {
                    assert_eq!(c.args[0], block, "{ctx}: the old block reallocated");
                    ArrayCall::Realloc(c.args[1])
                }
                a => panic!("{ctx}: call {a:#x}"),
            })
            .collect();
        assert_eq!(got_calls, calls, "{ctx}: calls");
    }
}
