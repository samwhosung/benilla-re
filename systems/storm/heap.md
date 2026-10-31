# storm: heap

Storm's allocator, `SMem3.cpp`: one heap object behind `SMemAlloc` and `SMemFree`. It hands out power-of-two
blocks from 16 MiB segments that it reserves and commits by page. Every block starts with an 8-byte header and the
user pointer is the header plus 8. A free block waits in an array of its size class, ordered by address; a freed
block joins its free buddy, and the joined block that buddy again, until a buddy is in use or of another class. The heap is the
library's own code, so its functions are class `boundary`. This doc follows a block from the choice of its class to
what a free writes.

Roots: `0x645640`, `0x645790`

## Block header

A block is 2^class bytes, the header its first 8. The class is the smallest from 3 up whose block holds the request
plus 8, so a 12-byte request is class 5, a 32-byte block. The first header dword holds the in-use bit 0, the
segment index in bits 1 to 7 and the requested size in bits 8 to 31; the second holds the class in bits 0 to 4 and,
while the block is free, its index in its class's array in bits 5 to 31. A block above class 24 has no segment: bits
1 to 7 of the first dword hold bits 24 to 30 of its size.

Addresses: `0x645640`, `0x645653`, `0x645664`, `0x645910`, `0x645947`, `0x645950`, `0x645b30`, `0x645b9d`, `0x6461c0`, `0x646209`
Confidence: verified
Proof: one
Test: none

`0x645653` to `0x645664` search the class from 3 for `1 << class >= size + 8`; none below 32 raises the memory
error. `0x645910` stores the size (`shl eax,8`, `0x645947`), sets the in-use bit (`or ecx,1`, `0x645950`) and keeps the
segment bits. A block above class 24 comes straight from `VirtualAlloc` (`0x645b30`), with the size's top bits stored at
`0x645b9d`. `0x6461c0` rebuilds the size of such a block as `(dword0 & 0xfe) << 23 | dword0 >> 8` (`0x646209` to
`0x646217`).

## Taking a block

For a class of 24 or less `0x645910` pops the lowest-addressed free block of the class, the root of its array. With
none free it asks for the next class and splits the result: the lower half is the block, and the upper half, whose two
header dwords are zeroed, gets its class and is queued as free. A class of 24 with none free reserves a new 16 MiB
segment. The arrays are binary heaps of header pointers ordered by address; an entry's index lives in its header.

Addresses: `0x645910`, `0x645931`, `0x64593d`, `0x645976`, `0x6459b7`, `0x6459c5`, `0x6459db`, `0x6459dd`, `0x645a14`, `0x645160`, `0x6453b0`, `0x645170`, `0x645450`, `0x6458e0`, `0x645050`, `0x646070`
Confidence: verified
Proof: one
Test: none

`0x645931` calls `0x645160`, which returns `[array][0]` or 0 (`0x645163`, `0x645169`); a block found is removed by index
with `0x6453b0` (`0x64593d`). A zero recurses with class + 1 and commit 0 (`0x645976`). `0x6459b7` calls `0x6458e0` for the
upper half, `0x6459c5` commits its page with `0x646070`, `0x6459db` and `0x6459dd` zero its two dwords and `0x645a14`
queues it. `0x645170` stores the entry at the count, writes its index into the header's second dword (`0x645256` to
`0x645267`) and sifts it up while it is below its parent at `(i − 1) / 2`, unsigned (`0x64527a`, `jae` at `0x64527d`).
`0x645450` swaps two entries and rewrites both indexes.

## Checking a freed block

`SMemFree` (`0x646430`) calls `0x645790` and returns 1 for any block. A null pointer does nothing. Under the heap's critical
section `0x6461c0` checks the header and, when it passes, `0x645eb0` frees the block. A header fails when it is not
readable for 8 bytes, has the in-use bit clear, lies outside its segment or off its class grid (class
24 or less), or, above class 24, is not page-aligned or has a class size below the stored size. A failure reports error
`0x8510007c` through `0x6452a0` with the caller's name and line and frees nothing.

Addresses: `0x646430`, `0x645790`, `0x6457a9`, `0x6457af`, `0x6457c4`, `0x6457d0`, `0x6461c0`, `0x6461cd`, `0x6461d9`, `0x6461e3`, `0x6461f5`, `0x646230`, `0x6450c0`, `0x6452a0`
Confidence: verified
Proof: one
Test: none

`0x6457a9` enters the heap's lock, `0x6457af` tests the initialised word `+0x424`, `0x6457c4` calls the check and
`0x6457d0` the free. The check calls `IsBadReadPtr(header, 8)` (`0x6461cd`), tests `dword0 & 1` (`0x6461d9`) and branches
on the class (`0x6461e3`, `cmp ecx,0x18`). Class 24 or less calls `0x6450c0` (`0x6461f5`): the header lies below the
segment base plus 16 MiB and its offset has no bit under the class size. Above 24 the class size must reach the stored
size and `test [heap+4],header` (`0xfff`) must be zero. `0x646230` loads the error code. A block below class 12 that joined its lower buddy never has its own header
rewritten, so its in-use bit stays set and a second free of it passes the check and queues it inside its free parent.

## Merging a freed block

`0x645eb0` subtracts the block from the heap's counters. A block above class 24 is released whole. Otherwise, from
its class up to 23, it finds the buddy header (the segment offset exclusive-ored with the class size). A buddy of the
same class and not in use leaves its array and joins: the parent header is the offset rounded down to twice the class size, and when the joined halves are of class 12
or more the first page of the upper half is decommitted (a join of two class-11 halves decommits nothing). The first
buddy that is in use or of another class ends the loop, and the block is queued.

Addresses: `0x645eb0`, `0x645ec1`, `0x645ec4`, `0x645ee6`, `0x645eec`, `0x645f0d`, `0x645f25`, `0x645f2a`, `0x645f37`, `0x645f3d`, `0x645f46`, `0x645f5a`, `0x645f66`, `0x645f9f`, `0x645fad`, `0x645ff7`, `0x64603f`, `0x6458b0`, `0x6458e0`, `0x6460c0`, `0x646170`, `0x645090`
Confidence: verified
Proof: one
Test: none

`0x645ec1` and `0x645ec4` read the class (`[hdr+4] & 0x1f`) and the size (`dword0 >> 8`); above class 24 `0x645ff7` ends in
`VirtualFree` with release (`0x64603f`). The counters at `+0x40c`, `+0x410` and `+0x414` change by −8, −size and
size − 2^class + 8 (`0x645ee6`, `0x645eec`, `0x645f0d`). The buddy test is `0x645f2a` to `0x645f37` (class `and 0x1f`,
then `test [eax],1`). A join runs `0x6453b0` (`0x645f3d`), `0x6458b0` (`0x645f46`) and `0x6460c0` (`0x645f5a`); the counter
reaches 24 at `0x645f66`, and a block that gets there is a whole segment, released with `0x645090` (`0x645f9f`). `0x6460c0`
decommits 4096 bytes when the class it is given is 12 or more; `0x646170` is called on both exits (`0x645f80`, `0x645fe9`) with the
joined block's base and the freed block's own size (`[ebp-4]`, stored at `0x645ed2`); `0x6454e0` stores 0x1000 at `+0`, 0xfff at `+4` and 12 at `+8`.

## What a free leaves

A free writes the freed block's header, the headers of blocks it moves in the arrays, the arrays and the counters, and
for a join of class-12-or-larger halves it decommits a page, and for a final class above 12 the pages from base + 0x1000
to the page-rounded end of base + size + 8; it writes no byte of the user region. The queued header keeps only the segment bits and the class. User bytes stay until a later allocation takes the block
or a free block containing it, or a decommit takes their page. Splitting a class-5 block into two class-4 blocks writes
the upper half's header over bytes 8 to 15 of its user region.

Addresses: `0x645eb0`, `0x645fad`, `0x645fb5`, `0x645fbc`, `0x645fcf`, `0x645fda`, `0x645fdc`, `0x646170`, `0x646179`, `0x646188`, `0x6461a2`, `0x6460c0`, `0x645910`, `0x6459db`, `0x6459dd`
Confidence: verified
Proof: one
Test: none

The no-join leg `0x645fad` writes the class into the second dword (`xor`/`and 0x1f`/`xor`, `0x645fb5` to `0x645fbc`), clears
the in-use bit and the size with `and edx,0xfe` (`0x645fcf`), stores the dword (`0x645fda`) and pushes the block with
`0x645170` (`0x645fdc`). `0x646170` decommits from `header + 0x1000` to `(header + 0xfff + size + 8) & ~0xfff`
(`0x646188` to `0x6461a2`) and does nothing at class 12 or below (`0x646179`, `jbe`). The census of `0x645eb0`
(`tools/callers.py`, control `0x4baee0`) finds one call, in `0x645790`.
