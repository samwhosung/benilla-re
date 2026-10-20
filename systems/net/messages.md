# net: messages

How the handlers of other systems read the packets the dispatcher gives them.

Every handler registered through `dispatch` receives its packet as a data store (`CDataStore`, from the base library) whose cursor stands at the first byte after the opcode, and reads the body field by field with the store's `Get` functions; the layout of a packet is the order and width of those reads. The functions are shared by every packet in the client. This doc holds the rules they have in common, which a reimplementation of any handler must reproduce, and the packets whose handlers belong to no other system's doc.

Roots: `0x537aa0`

## Data store reads

A data store's buffer (`+0x4`) holds the bytes from position `+0x8` for `+0xc` bytes; the size is `+0x10`, the cursor `+0x14`. Each read of `n` bytes tests `cursor + n > size`, as an unsigned add with no overflow guard, then asks vtable slot 2 to move the window when the bytes lie outside it; the plain store (`0x417c20`) and the network layer's always refuse. Either failure sets the cursor to `size + 1`, so every later read fails, with no error flag; a fixed-width read leaves its destination unwritten. `GetData` copies chunks of at most the bytes remaining, the window and the count, at least 1: a count past the size copies what remains, then poisons the cursor.

Addresses: `0x418cb0`, `0x418d30`, `0x418db0`, `0x418e30`, `0x418eb0`, `0x418fb0`, `0x419130`, `0x4190b0`, `0x419400`, `0x419aa0`, `0x401d10`, `0x417c20`, `0x418cbf`, `0x418cc4`, `0x418cf5`, `0x418cfc`, `0x419441`, `0x41945a`, `0x419467`
Confidence: verified
Proof: one
Test: `net::datastore_heap`, `net::datastore_inline`, `net::datastore_window`

`0x418cbc` to `0x418cc2` are `lea edx,[eax+1]; cmp edx,ecx; jbe` on the cursor and size, and `0x418cc4` is `inc ecx; mov [esi+0x14],ecx`. The window test is `cmp eax,[esi+8]; jb` and `cursor + n` against `[esi+8] + [esi+0xc]`; `0x418cf5` is `call [edx+8]`, and a zero return reaches `0x418cfc`, which stores `size + 1`. `0x417c20` is `xor eax,eax; ret 0x14`. The other fixed-width reads repeat this for 2, 4 and 8; the four dword reads have no sign or zero extension and no floating-point instruction, and their bytes are identical. In `0x419400` the chunk is `size − cursor`, lowered to the count and to `[esi+0xc]` (`0x419441` to `0x419458`) and raised to 1 (`0x41945a`); `0x419467` latches a chunk past the size. `0x419aa0` forwards to it. `0x401d10` returns `cursor == size` (`sub`, `neg`, `sbb`, `inc`).

The tests run sequences of reads and writes on three stores, comparing the six fields, the bytes and every result after each call: the plain store, the network store, and a store whose vtable moves a window of 1, 4, 7 or 16 bytes and refuses wider reads, which reaches every refusal path. They run each function here whole.

## Data store strings and pointers

`GetDataInSitu` (`0x419ac0`, destination, `n`) first stores 0 in its destination, so a failed read gives a null pointer; on success it stores a pointer into the packet buffer and advances the cursor by `n`, copying nothing. `GetCString` (`0x4191b0`, destination, capacity) copies bytes up to and including the packet's NUL, a window at a time. When the cursor is past the size at exit it stores 0 at the destination's first byte: a string of `capacity` or more characters, or one with no NUL before the end of the packet, reads as `""` and poisons the store. A null destination with a nonzero capacity sets error 87 and reads nothing.

Addresses: `0x419ac0`, `0x4191b0`, `0x419acc`, `0x419b3b`, `0x419274`, `0x4192a1`, `0x4192aa`, `0x4192b9`, `0x4192bb`, `0x4191cb`
Confidence: verified
Proof: one
Test: `net::datastore_heap`, `net::datastore_inline`, `net::datastore_window`

`0x419acc` is `mov dword [eax],0` before any test; `0x419b3b` stores the pointer. In `0x4191b0`, each pass copies at most the bytes left in the window, cut to the size, and the capacity left; the copy loop `0x419274` to `0x41928d` writes each byte, stopping after the NUL; `0x4192a1` `cmp edx,[ebp+0xc]; jb` ends the loop at the capacity, `0x4192aa` poisons the cursor, and `0x4192b9` `jbe` sends a poisoned store to `0x4192bb`, which writes 0 at `[ebp+8]`, the original destination. `0x4191cb` pushes 87 for `SErrSetLastError`. The tests run both functions whole, with cursors at 0, inside, at the size and past it, and capacities from 0 to 47.

## Data store writes

Each write of `n` bytes at a position (the size for `Put`, an argument for `Set`) asks vtable slot 3 to move the window when the position is below the window or its end past it, ignores the answer, stores, and a `Put` adds `n` to the size. The plain store's slot 3 (`0x417c30`) makes the buffer the end rounded up to 256 bytes; the network store's (`0x5bc6c0`) starts in 1,024 bytes of its own at `+0x18` and grows to exactly the end. `PutArray` writes chunks of at most the window. `PutFloat` passes the value through `fld` and `fstp`, so a signalling NaN is stored quiet. A null source with a nonzero count sets error 87.

Addresses: `0x418070`, `0x4180d0`, `0x418130`, `0x418190`, `0x4181f0`, `0x4182b0`, `0x418370`, `0x4183d0`, `0x417cd0`, `0x417d70`, `0x417dc0`, `0x417e10`, `0x418430`, `0x418570`, `0x418c10`, `0x417c30`, `0x5bc6c0`, `0x5bc630`, `0x7ff9e4`, `0x80a7a8`, `0x4180a4`, `0x417c3c`, `0x5bc6d1`, `0x5bc777`, `0x4185dd`, `0x41840a`
Confidence: verified
Proof: one
Test: `net::datastore_heap`, `net::datastore_inline`, `net::datastore_window`

`0x418076` to `0x41808e` compare the size with `[esi+8]` and `size + 1` with `[esi+8] + [esi+0xc]`; `0x4180a4` is `call [edx+0xc]` with the position, the count, the three field addresses and two zeros, and the store follows whatever it returns. `0x417c3c` is `lea eax,[ecx+eax+0xff]`, then `and eax,0xffffff00`, stored to the window length and passed to `SMemReAlloc`. `0x5bc6d1` returns at once when the end is within `[edx]`; otherwise a buffer other than `this + 0x18` is reallocated, and the inline buffer is replaced by `SMemAlloc` and `min(position, length)` bytes copied (`0x5bc777` to `0x5bc791`), which the write then covers. `0x5bc630` sets the inline buffer and the length 1,024. `0x4185dd` to `0x4185f3` take the chunk as the remaining count, or the window length when that is not larger, and at least 1. `0x41840a` is `fld dword [ebp+8]`. The tests run every function here whole, `0x417c30` and `0x5bc6c0` but for the paths of a caller's own file name (`0x417c58`, `0x5bc6f0` to `0x5bc70e`, `0x5bc744` to `0x5bc754`), which no write passes, and `0x5bc76f`, a zero end, which the early return excludes. A signalling NaN is left out of `PutFloat`'s inputs: the emulator's `fld` keeps it signalling where the processor quiets it.

## Time speed packet

`SMSG_LOGIN_SETTIMESPEED` (`0x42`) is a u32 packed date (`gametime/messages#packed-date-fields`) and a float rate. A packet not read to its end logs `Malformed message recieved: Id = %d, Len = %d, Read = %d` and the handler returns 0. Otherwise it decodes the date, sets the clock `0xce8538` to it with `force` (`gametime/overview#setting-the-time`), calls the bridge `0x6c5f50` (`gametime/messages#time-changed-bridge`), stores the rate clamped (`gametime/overview#time-scale`) and logs `Gamespeed set from <old> to <new>`, the new value being the rate as received, not as clamped (`gametime/messages#game-speed-set`). The bridge runs before the rate is stored.

Addresses: `0x6c5e80`, `0x642c30`, `0x642b30`, `0x6c5e96`, `0x6c5ea1`, `0x6c5eaa`, `0x6c5ed9`, `0x6c5ee9`, `0x6c5eee`, `0x6c5efc`, `0x6c5f01`
Confidence: verified
Proof: one
Test: none

`0x6c5e96` reads the dword with `0x418eb0` and `0x6c5ea1` the rate with `0x419130`; `0x6c5eaa` is `call [edx+0x14]`, the store's read-to-end test. `0x6c5ed9` calls `0x642c30` on the dword. `0x6c5ee9` is `0x6434d0` on the clock with the record, `0x6c5eee` calls `0x6c5f50` and `0x6c5efc` is `0x643820` with the rate; the old speed in the log is its return, and `0x6c5f01` loads the received float for the line.
