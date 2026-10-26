# lua: block pools

The client's own allocator under its Lua state, `lmemPool.cpp`, which `luaM_irealloc` is the one
caller of. Six pools serve the blocks of up to 256 bytes, sized 16, 32, 64, 96, 128 and 256 (the
table `0x811d34`); larger blocks, and every block of a call with no state, go to Storm's heap. The
global state holds the six pool pointers in an array at `+0`, made on the first call. A pool is
0x18 bytes: a growable array of chunk pointers (its capacity at `+0`, count `+4`, data `+8`), the
bytes of a chunk at `+0xc`, the block size at `+0x10` and the blocks a chunk holds at `+0x14`. A
chunk is 0x14 bytes: the base of its memory at `+0`, the head of its free list at `+4`, its bytes at
`+8`, the block size at `+0xc` and its free count at `+0x10`. A free block holds the next free block
in its first dword.

Roots: `0x6fae90`, `0x6fae00`, `0x6fb2f0`

## Allocating from the pools

`luaM_reallocPool(L, block, size)` with no state is `SMemReAlloc`. Size 0 frees, returning null: a
block of a chunk to the head of its free list, another to `SMemFree`. A null block takes one of the
first pool that holds the size, else `SMemAlloc`; a heap block is `SMemReAlloc`ed. A pool block its pool holds stays; a larger size takes a block of the first
later pool that holds it, else `SMemAlloc`, copies the old pool's block size and frees the old. A pool gives the head of its first chunk with a free block, else grows its
chunk array by one and cuts a chunk of `0x20000 / size` blocks in address order. Only closing the
state frees chunks.

Addresses: `0x6fae90`, `0x6faea3`, `0x6faecd`, `0x6faf1b`, `0x6faf25`, `0x6faf34`, `0x6faf4c`, `0x6faf54`, `0x6faf74`, `0x6faf85`, `0x6fafae`, `0x6fafbe`, `0x6fafc7`, `0x6fafd2`, `0x6fb03b`, `0x6fb052`, `0x6fb070`, `0x6fb079`, `0x6fb094`, `0x6fb0aa`, `0x6fb0fc`, `0x6fb112`, `0x6fb155`, `0x6fb176`, `0x6fb197`, `0x6fb1a3`, `0x6fb1bb`, `0x6fb1cc`, `0x6fb1f0`, `0x6fb20e`, `0x6fb214`, `0x6fb21a`, `0x6fb24e`, `0x6fb277`, `0x6fb28f`, `0x6fb2a7`, `0x6fb2c0`, `0x6fb2e6`, `0x6fb3b0`, `0x6fb1e0`, `0x6fb440`, `0x811d34`
Confidence: verified
Proof: one
Test: `lua::realloc_pool_whole`

`0x6faea3` sends a null state to `SMemReAlloc`. `0x6faecd` builds the pools on a null `[G+0]`: `0x6faf1b` divides
0x20000 by each size and `0x6faf25` to `0x6faf34` store the size, the block count and their product. `0x6faf4c` and
`0x6faf54` split the cases. Each search walks pools 0 to 5 and their chunks, testing `base <= block < base + bytes`
(`0x6faf74`, `0x6faf85`); a free pushes (`0x6fafbe` to `0x6fafd2`) or calls `SMemFree` (`0x6fafae`). A realloc not
found goes to `SMemReAlloc` (`0x6fb03b`); `0x6fb052` keeps a block its pool holds; `0x6fb070` to `0x6fb079` find the
later pool, `0x6fb0fc` takes from it and `0x6fb094` calls `SMemAlloc`; `0x6fb0aa` and `0x6fb112` copy `[pool+0x10]`
bytes, and `0x6fb155` and `0x6fb176` push the old block. A fresh block takes the first pool at `0x6fb1a3`, else
`SMemAlloc` (`0x6fb1bb`). `0x6fb1f0` pops a head (`0x6fb20e` to `0x6fb21a`), else grows the array with `0x6fb3b0`
(`0x6fb24e`), divides the chunk bytes by the size (`0x6fb277`), allocates them (`0x6fb28f`), links each block to the
next (`0x6fb2a7`), ends the list with 0 (`0x6fb2c0`) and pops (`0x6fb2e6`). The test runs the function whole, on
stubbed Storm calls, through 60,000 allocations, frees and reallocations, locating each result in the exe's pools.

## The threshold a collection leaves

The client replaces 5.0's threshold of twice the bytes in use with `0x6fae00(L)`, which `checkSizes`
jumps to at the end of a collection. The headroom is the threshold `[G+0x24]` less the bytes in use
`[G+0x28]` (`luaM_irealloc`'s running sum of size changes), in 32 bits, lowered to the free bytes of each pool: its block size times its chunks'
free counts summed, compared unsigned. The new threshold is the count plus the larger of the
headroom and a quarter of the count, `count >> 2`. A pool with no chunk has no free bytes, so until
every pool has a chunk the threshold is the count and a quarter.

Addresses: `0x6fae00`, `0x6fae1a`, `0x6fae23`, `0x6fae43`, `0x6fae52`, `0x6fae57`, `0x6fae65`, `0x6fae6c`, `0x6fae71`, `0x6fae74`, `0x6fae7f`, `0x6f73d6`
Confidence: verified
Proof: one
Test: `lua::next_threshold_whole`

`0x6fae1a` subtracts the count. The loop `0x6fae23` to `0x6fae65` takes each of the six pools, sums `[chunk+0x10]` over
its chunks (`0x6fae43`), multiplies by `[pool+0x10]` (`0x6fae52`, `imul`, 32 bits) and keeps the smaller (`0x6fae57`,
`jae`). `0x6fae6c` forms `count >> 2`; `0x6fae71` (`jae`) picks the headroom when it is not below that, `0x6fae74` adds
the quarter and `0x6fae7f` the headroom. `0x6f73d6` is `checkSizes`' jump. The test runs the function whole over pools
of zero to three chunks with random free counts and sizes, and counts and thresholds on both sides of each other.

## Closing the pools

`0x6fb2f0(L)`, run by `close_state`, frees every pool's chunks, each chunk's memory and then its
record, from the last chunk to the first, then the chunk array, the pool and the pool array, and
clears `[G+0]`.

Addresses: `0x6fb2f0`, `0x6fb30e`, `0x6fb31d`, `0x6fb335`, `0x6fb341`, `0x6fb350`, `0x6fb363`, `0x6fb378`, `0x6fb387`, `0x6fb3a4`, `0x6f6f54`
Confidence: verified
Proof: one
Test: none

`0x6fb30e` calls `0x6fb350` for each non-null pool and `0x6fb31d` frees the pool; `0x6fb335` frees the array and `0x6fb341`
clears `[G+0]`. `0x6fb350` walks the chunks from the end (`0x6fb363`), freeing the base (`0x6fb378`) and the record
(`0x6fb387`), then the array's data (`0x6fb3a4`). `close_state` calls it at `0x6f6f54`.
