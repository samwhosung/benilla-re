//! The client's block pools for its Lua state (`lmemPool.cpp`): six pools of fixed-size blocks
//! that `luaM_reallocPool` (`0x6fae90`) serves every block of up to 256 bytes from, and the
//! threshold a collection leaves, capped by the least free space among them (`0x6fae00`).

/// The pools' block sizes, the table at `0x811d34`.
pub const BLOCK_SIZES: [u32; 6] = [16, 32, 64, 96, 128, 256];

/// The bytes a chunk is cut from, before rounding down to whole blocks (`0x6faf1b`).
pub const CHUNK_BYTES: u32 = 0x2_0000;

/// The threshold `0x6fae00` stores at `+0x24` of the global state, from the old threshold, the
/// block count `+0x28` and each pool's block size and the free counts of its chunks in order.
///
/// The headroom is `threshold - count` modulo 2^32, lowered (unsigned) to each pool's
/// `size * free` (the free counts summed and the product taken modulo 2^32); the result is the
/// count plus the larger of the headroom and `count >> 2`, modulo 2^32.
#[must_use]
pub fn next_threshold(threshold: u32, count: u32, pools: &[(u32, Vec<u32>)]) -> u32 {
    let mut head = threshold.wrapping_sub(count);
    for (size, frees) in pools {
        let sum = frees.iter().fold(0u32, |a, &f| a.wrapping_add(f));
        let bytes = size.wrapping_mul(sum);
        if bytes < head {
            head = bytes;
        }
    }
    let quarter = count >> 2;
    if head < quarter {
        count.wrapping_add(quarter)
    } else {
        count.wrapping_add(head)
    }
}

/// A block as the pools hand it out: a slot of a chunk of a pool, or a block of Storm's heap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    /// No block (a null pointer).
    Null,
    /// Slot `slot` of chunk `chunk` of pool `pool`, the slot counted from the chunk's base.
    Pool {
        pool: usize,
        chunk: usize,
        slot: u32,
    },
    /// A block of `SMemAlloc` or `SMemReAlloc`.
    System,
}

/// The six pools as `luaM_reallocPool` keeps them once a state has called it: each chunk's free
/// slots as a stack, the head last.
#[derive(Clone, Debug, Default)]
pub struct Pools {
    chunks: [Vec<Vec<u32>>; 6],
}

impl Pools {
    /// Blocks a chunk of pool `i` holds: `0x20000 / size`.
    #[must_use]
    pub fn per_chunk(i: usize) -> u32 {
        CHUNK_BYTES / BLOCK_SIZES[i]
    }

    /// `0x6fb1f0`: the head of the first chunk, in order, that has a free slot; else a new chunk
    /// appended, its slots linked in address order, and its first slot.
    fn take(&mut self, pool: usize) -> Block {
        for (chunk, free) in self.chunks[pool].iter_mut().enumerate() {
            if let Some(slot) = free.pop() {
                return Block::Pool { pool, chunk, slot };
            }
        }
        let n = Self::per_chunk(pool);
        let mut free: Vec<u32> = (0..n).rev().collect();
        let slot = free.pop().expect("a chunk holds a block");
        self.chunks[pool].push(free);
        Block::Pool {
            pool,
            chunk: self.chunks[pool].len() - 1,
            slot,
        }
    }

    /// A pool block given back: pushed at its chunk's head.
    fn give(&mut self, b: Block) {
        if let Block::Pool { pool, chunk, slot } = b {
            self.chunks[pool][chunk].push(slot);
        }
    }

    /// `luaM_reallocPool(L, block, size)` with a state: what it returns.
    ///
    /// Size 0 frees: a pool block goes back on its chunk's free list, a heap block to `SMemFree`,
    /// and it returns null. A null block of a nonzero size comes from the first pool whose blocks
    /// hold it, else from `SMemAlloc`. A heap block reallocates through `SMemReAlloc` at any size.
    /// A pool block whose pool holds the new size stays; a larger size takes a block from the
    /// first later pool that holds it, else from `SMemAlloc`, copies the old pool's block size
    /// into it and gives the old block back.
    pub fn realloc(&mut self, block: Block, size: u32) -> Block {
        if size == 0 {
            self.give(block);
            return Block::Null;
        }
        match block {
            Block::Null => match BLOCK_SIZES.iter().position(|&b| size <= b) {
                Some(i) => self.take(i),
                None => Block::System,
            },
            Block::System => Block::System,
            Block::Pool { pool, .. } if size <= BLOCK_SIZES[pool] => block,
            Block::Pool { pool, .. } => {
                let fresh = match (pool + 1..6).find(|&k| size <= BLOCK_SIZES[k]) {
                    Some(k) => self.take(k),
                    None => Block::System,
                };
                self.give(block);
                fresh
            }
        }
    }

    /// The chunks each pool has taken.
    #[must_use]
    pub fn chunk_counts(&self) -> [usize; 6] {
        std::array::from_fn(|i| self.chunks[i].len())
    }

    /// The free slots of each chunk of each pool.
    #[must_use]
    pub fn free_counts(&self) -> [Vec<u32>; 6] {
        std::array::from_fn(|i| self.chunks[i].iter().map(|c| c.len() as u32).collect())
    }
}
