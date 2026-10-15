# warden: the module cache

Where a module is kept between sessions. The wow cache, `0xc0e084`, is one of the thirteen `DBCache` objects
(`dbcache`): a hash table of nodes keyed by a 16-byte id, each holding one `WardenBuffer`, written to
`WDB/wowcache.wdb` at exit and read at start-up. The services 1 and 2 are its only users. The table code lives
in `DBCache.cpp`; this page states what Warden asks of it and what the file holds.

Roots: `0x6ca070`, `0x6ca0c0`

## Lookup and store

The id's 16 bytes give a 31-bit hash: the id's last dword with its bytes reversed and its top bit cleared. The
bucket is the hash masked by the table's mask; a node matches on the hash and all 16 bytes. Lookup with no
callback returns a valid node's record, else null; an all-zero id never matches, and a lookup never creates a node.
Store finds or makes the node, copies the buffer into it, sets its valid byte and copies the id. Remove unlinks a
valid node and deletes it. Service 2 removes, then stores.

Addresses: `0x56b430`, `0x56b440`, `0x56b469`, `0x56b4a5`, `0x56b4ec`, `0x56b621`, `0x56b62c`, `0x56b6c4`, `0x56b6f0`, `0x56b7b0`, `0x56b7de`, `0x56b7e9`, `0x56b7ed`, `0x56c5f0`, `0x56c6bd`, `0x56c743`, `0x56c7a9`
Confidence: verified
Proof: one
Test: none

`0x56b430` returns 0 for an id whose 16 bytes are all 0 (`0x56b440` to `0x56b456`). `0x56b469` to `0x56b4a5` build the
hash: byte 0 of the last dword masked with `0x7f` is the top byte, then bytes 1, 2, 3 below it. `0x56b4ec`
compares four dwords with `repz cmps`. A node found with the valid byte `+0x3c` set returns `node+0x24`
(`0x56b6c4`), the record; an unfilled node, or a miss, returns 0 for a null callback (`0x56b621`, `0x56b6b7`), since
creating a node needs the callback (`0x56b516`). Service 1 passes zeros for the callback, parameter and flag. Store
creates the node at `0x56b7b0` (`0x56e390`), assigns the record (`0x56b7de`, `0x6c9ff0`), sets `+0x3c` (`0x56b7e9`)
and copies the id to `+0x2c` (`0x56b7ed`). Remove unlinks the node's two chains and calls the cache's slot 0
(`0x56c7a9`) when valid (`0x56c743`).

## The cache file

The file is `WDB/wowcache.wdb`, relative to the working directory, made with the directory first. A header of five
dwords: the tag bytes `NDRW`, the build `0x16f3` (5875), the locale tag, the record size 8, the version 1. Entries
follow: the 16-byte id, a dword size, and the record, which is a dword length and that many bytes; the size is the
record's total, length included. Twenty zero bytes (an all-zero id and a zero dword) end the file. The exit function
writes the file anew, every valid node; the start-up function reads it.

Addresses: `0x56b040`, `0x56b05c`, `0x56b07b`, `0x56b09c`, `0x56b137`, `0x56b13c`, `0x56b149`, `0x56b158`, `0x56b162`, `0x56b1ed`, `0x56b1fc`, `0x56b208`, `0x56b21b`, `0x56b268`, `0x56be60`, `0x56bf46`, `0x56bf5a`, `0x56bf82`, `0x56bfce`, `0x56bff8`, `0x56c091`, `0x56c09d`, `0x56c05c`, `0x56c0a4`, `0x56c277`, `0x554f00`, `0x554f20`, `0x554f30`, `0x554f50`, `0x402b1b`
Confidence: verified
Proof: two
Test: none

`0x554f00` constructs the cache with the tag `0x5752444e`, a name, opcode 0, and both flags 1; `0x554f20` registers
`0x554f30`, which jumps to the save `0x56b040`, with `atexit`. `0x56b05c` makes `WDB`; `0x56b07b` formats `%s/%s` and
`0x56b09c` opens it for writing, creating always. The header is built from `0x56b137` (tag), `0x56b13c` (`0x16f3`),
`0x56b149` (the dword at `0xc2a2a4`), `0x56b158` (8) and `0x56b162` (`0x6ca120`, returning 1). Per node with `+0x3c`
set and `+0x4c` clear, `0x56b1ed` writes the id's 16 bytes, `0x56b1fc` a zero size, `0x56b208` the record, and
`0x56b21b` patches the size with the written length less 20. The id's bytes at `+0x2c` are written; `+0x4c` is set
nowhere in this cache's code. `0x56b268` writes 16 zero bytes and a zero dword, the end marker. The load `0x56be60`
(called from `0x554f50` by `0x402b1b`) compares tag, build, locale, size and version (`0x56bf46` to `0x56bff8`) and on
any difference closes the file and leaves the cache empty. It reads 20 bytes per entry head (`0x56c05c`); an all-zero
id (`0x56c0a4`) ends the file; a short read of an entry clears the cache.

## The stored module

The record of a module is the blob as the server sent it: RC4-encrypted under the key the server gives with the id,
and signed. The file holds no plain module code.

Addresses: `0x6ca770`, `0x6ca7f4`, `0x6ca840`, `0x6ca6a0`
Confidence: verified
Proof: two
Test: none

The built-in module stores the bytes it received (service 2, `0x6ca840`; `0x25cd` to `0x25e3` in the module,
`embedded`), after their MD5 matches the id. A load passes the stored record and the key to `LoadWardenModule`
(`0x6ca7f4`), which decrypts it with that key (`modules`).

## The cache record

A `WardenBuffer` is stored as a dword length and then the bytes one at a time. The reader allocates the length
and fills it byte by byte. A reader given a store that holds fewer than four bytes writes no length, and the
length it uses is the store's own address, read from the slot that held the argument. A length longer than the
rest of the store leaves the bytes past the end as the allocator gave them.

Addresses: `0x6ca070`, `0x6ca089`, `0x6ca0a1`, `0x6ca0c0`, `0x6ca0c9`, `0x6ca0d1`, `0x6ca0d6`, `0x6ca0f7`, `0x6ca10b`, `0x6ca010`, `0x6c9ff0`, `0x6c9f80`, `0x4181f0`, `0x418070`, `0x418eb0`, `0x418cb0`
Confidence: verified
Proof: one
Test: `warden::record_writer`, `warden::record_reader`, `warden::record_reader_short_stores`

`0x6ca089` calls `PutUInt32` (`0x4181f0`) with the length, or 0 when the data pointer is null; `0x6ca0a1` calls `PutInt8`
once per byte. `0x6ca0c9` takes the address of the argument slot `[ebp+8]` as the output of `GetUInt32`
(`0x6ca0d1`, `0x418eb0`), so a short store leaves the slot, the store pointer, as the length (`0x6ca0d6`). `0x6ca0f7`
allocates the length (line `0xf9`); `0x6ca10b` calls `GetUInt8` for each byte, which leaves its output when the
store ends. `0x6ca010` sets a buffer from a length and a pointer by allocating (line `0xd4`) and copying; a null pointer
empties it. `0x6c9ff0` does the same from another buffer without freeing the one it overwrites; `0x6c9f80`
frees the data (line `0xad`).
