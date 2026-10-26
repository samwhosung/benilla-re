//! The client's Lua block pools (`lmemPool.cpp`), run against the exe: `luaM_reallocPool`
//! (`0x6fae90`) whole, with the pool block allocator `0x6fb1f0` and the chunk array's growth
//! `0x6fb3b0` inside it and Storm's `SMemAlloc`, `SMemReAlloc` and `SMemFree` stubbed on an
//! arena; and the threshold a collection leaves (`0x6fae00`) whole.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lua::pools::{next_threshold, Block, Pools, BLOCK_SIZES};

const REALLOC_POOL: u64 = 0x6fae90;
const NEXT_THRESHOLD: u64 = 0x6fae00;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const SMEM_FREE: u64 = 0x646430;
const ARENA: u64 = 0x3000_0000;
const ARENA_SIZE: u64 = 0x0400_0000;

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// Where an address the exe returned sits: a slot of a chunk of a pool, read from the pools the
/// global state points at (`[G+0]`, each pool's chunk count `+4` and array `+8`, each chunk's base
/// `+0` and bytes `+8`), else a heap block.
fn locate(emu: &Emu, g: u64, va: u32) -> Block {
    if va == 0 {
        return Block::Null;
    }
    let pools = u64::from(emu.read_u32(g));
    for (pool, &size) in BLOCK_SIZES.iter().enumerate() {
        let p = u64::from(emu.read_u32(pools + 4 * pool as u64));
        let (n, chunks) = (emu.read_u32(p + 4), u64::from(emu.read_u32(p + 8)));
        for chunk in 0..n as usize {
            let c = u64::from(emu.read_u32(chunks + 4 * chunk as u64));
            let (base, bytes) = (emu.read_u32(c), emu.read_u32(c + 8));
            if va >= base && va < base + bytes {
                let slot = (va - base) / size;
                assert_eq!((va - base) % size, 0, "a block off its slot");
                return Block::Pool { pool, chunk, slot };
            }
        }
    }
    Block::System
}

/// A sequence of 60,000 allocations, frees and reallocations of 0 to 300 bytes, weighted so the
/// 96- to 256-byte pools take several chunks, over live blocks each tagged with an id in its
/// first dword. Each result is located in the exe's own pools and compared with the model's, a
/// moved block's id checked in its new place, and every pool's chunk and free counts compared at
/// the end. A null state's call goes straight to `SMemReAlloc`.
#[test]
fn realloc_pool_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.map(ARENA, ARENA_SIZE);
    emu.stub_alloc_sized(SMEM_ALLOC, ARENA, ARENA + ARENA_SIZE, 0, 4);
    emu.stub_realloc_sized(SMEM_REALLOC, 5);
    emu.stub(SMEM_FREE, 0, 4);
    let l = emu.alloc(0x20);
    let g = emu.alloc(0x40);
    emu.write_u32(l + 0x10, g as u32);
    let mut model = Pools::default();
    let mut live: Vec<(u32, Block, u32)> = Vec::new();
    let mut next_id = 1u32;
    let mut s = 0x6fae_9000u32;
    let size_of = |s: &mut u32| -> u32 {
        match lcg(s) >> 28 {
            0..=5 => 161 + (lcg(s) >> 8) % 96,
            6..=9 => 65 + (lcg(s) >> 8) % 64,
            10..=12 => 1 + (lcg(s) >> 8) % 64,
            13 => 257 + (lcg(s) >> 8) % 44,
            _ => (lcg(s) >> 8) % 301,
        }
    };
    for op in 0..60_000 {
        let pick = lcg(&mut s) >> 24;
        let (old_va, old_block, id, size) = if live.is_empty() || pick < 140 {
            (0, Block::Null, 0, size_of(&mut s))
        } else {
            let i = (lcg(&mut s) >> 8) as usize % live.len();
            let (va, b, id) = live.swap_remove(i);
            let size = if pick < 200 { 0 } else { size_of(&mut s) };
            (va, b, id, size)
        };
        let got = emu.call(REALLOC_POOL, l as u32, old_va, &[size]);
        let want = model.realloc(old_block, size);
        let block = locate(&emu, g, got);
        assert_eq!(block, want, "op {op}: realloc of {old_block:?} to {size}");
        if size == 0 {
            assert_eq!(got, 0, "op {op}: a free returns null");
            continue;
        }
        if old_va != 0 {
            // A heap block shrunk below 4 bytes keeps only that much of its id (the stub copies
            // the new size).
            let keep = size.min(4) as usize;
            assert_eq!(
                emu.read(u64::from(got), keep),
                id.to_le_bytes()[..keep].to_vec(),
                "op {op}: the moved block's id, {old_block:?} to {size}"
            );
            live.push((got, block, id));
        } else {
            emu.write_u32(u64::from(got), next_id);
            live.push((got, block, next_id));
            next_id += 1;
        }
    }
    let pools = u64::from(emu.read_u32(g));
    let free = model.free_counts();
    for pool in 0..6 {
        let p = u64::from(emu.read_u32(pools + 4 * pool as u64));
        assert_eq!(
            emu.read_u32(p + 0x10),
            BLOCK_SIZES[pool],
            "pool {pool}'s block size"
        );
        assert_eq!(
            emu.read_u32(p + 0x14),
            0x2_0000 / BLOCK_SIZES[pool],
            "pool {pool}'s blocks"
        );
        let (n, chunks) = (emu.read_u32(p + 4), u64::from(emu.read_u32(p + 8)));
        assert_eq!(
            n as usize,
            model.chunk_counts()[pool],
            "pool {pool}'s chunks"
        );
        let got: Vec<u32> = (0..n as u64)
            .map(|c| emu.read_u32(u64::from(emu.read_u32(chunks + 4 * c)) + 0x10))
            .collect();
        assert_eq!(got, free[pool], "pool {pool}'s free counts");
    }
    assert!(
        model.chunk_counts()[3..].iter().all(|&n| n >= 2),
        "the large pools take chunks: {:?}",
        model.chunk_counts()
    );
    emu.clear_calls();
    let got = emu.call(REALLOC_POOL, 0, 0x1234_5670, &[0x40]);
    assert_eq!(emu.calls().len(), 1, "a null state calls SMemReAlloc alone");
    assert_eq!(
        (
            emu.calls()[0].addr,
            emu.calls()[0].args[0],
            emu.calls()[0].args[1]
        ),
        (SMEM_REALLOC, 0x1234_5670, 0x40)
    );
    assert_ne!(got, 0);
}

/// The threshold over pools of zero to three chunks with random free counts (and one pool with
/// none), thresholds below, at and above the count, counts near 2^32, and products that wrap.
#[test]
fn next_threshold_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let l = emu.alloc(0x20);
    let g = emu.alloc(0x40);
    let array = emu.alloc(0x20);
    let pool_area = emu.alloc(0x20 * 6);
    let chunk_ptrs = emu.alloc(4 * 6 * 4);
    let chunk_area = emu.alloc(0x20 * 6 * 4);
    emu.write_u32(l + 0x10, g as u32);
    emu.write_u32(g, array as u32);
    let mut s = 0x6fae_0000u32;
    for round in 0..20_000 {
        let mut pools: Vec<(u32, Vec<u32>)> = Vec::new();
        for i in 0..6u64 {
            let size = match round % 4 {
                0 => BLOCK_SIZES[i as usize],
                _ => lcg(&mut s),
            };
            let n = if round % 7 == 0 && i == 0 {
                0
            } else {
                (lcg(&mut s) >> 8) % 4
            };
            let frees: Vec<u32> = (0..n)
                .map(|_| match round % 3 {
                    0 => (lcg(&mut s) >> 8) % 8193,
                    _ => lcg(&mut s),
                })
                .collect();
            let p = pool_area + 0x20 * i;
            emu.write_u32(array + 4 * i, p as u32);
            emu.write_u32(p + 4, n);
            emu.write_u32(p + 8, (chunk_ptrs + 0x10 * i) as u32);
            emu.write_u32(p + 0x10, size);
            for (c, &f) in frees.iter().enumerate() {
                let chunk = chunk_area + 0x80 * i + 0x20 * c as u64;
                emu.write_u32(chunk_ptrs + 0x10 * i + 4 * c as u64, chunk as u32);
                emu.write_u32(chunk + 0x10, f);
            }
            pools.push((size, frees));
        }
        let count = match round % 5 {
            0 => lcg(&mut s) >> 12,
            1 => 0xffff_ff00 | (lcg(&mut s) & 0xff),
            _ => lcg(&mut s),
        };
        let threshold = match round % 6 {
            0 => count,
            1 => count.wrapping_sub(1),
            2 => count.wrapping_add(lcg(&mut s) >> 16),
            _ => lcg(&mut s),
        };
        emu.write_u32(g + 0x24, threshold);
        emu.write_u32(g + 0x28, count);
        emu.call(NEXT_THRESHOLD, l as u32, 0, &[]);
        assert_eq!(
            emu.read_u32(g + 0x24),
            next_threshold(threshold, count, &pools),
            "round {round}: threshold {threshold:#x}, count {count:#x}, pools {pools:?}"
        );
    }
}
