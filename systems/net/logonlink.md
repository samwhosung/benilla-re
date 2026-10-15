# net: logon link

What the logon link does around the proof: its events, the realm list, file transfers, the checksum and the pin.

The `ClientLink` in `Grunt.cpp` speaks the logon protocol of `login` over its own `WowConnection`; the logon object (`Login.cpp`) listens to it through the vtable `0x80a5d4` and reports to the login screen. This doc holds what surrounds the challenge and the proof: how a failed dial ends, the realm list the server returns, a file the server offers, the checksum of the client's own files that the proof carries, and the pin grid.

Roots: `0x5bb360`, `0x5b3320`, `0x5ba8d0`, `0x5b2230`, `0x5baf70`, `0x5bb050`, `0x5b1170`, `0x5b1ed0`, `0x5b3120`

## Logon link events

The link tells the logon object when it connects, cannot connect and disconnects, through slots of the vtable `0x80a5d4`. A dial that fails, by a socket error or an empty address list, runs the link's slot 3 (`0x5bb360`): it releases the connection and calls the logon object's slot 2, `0x5b1b70`. That reports `LOGIN_STATE_FAILED` (5) with `LOGIN_SERVER_DOWN` (`0x0c`) to the listener, then calls slot 1, `0x5b1bd0`, which reports `LOGIN_STATE_DISCONNECTED` (11) with code 0. When the byte `+4` is clear, so the link never reached the post-proof state, `0x5b1bd0` also marks `+8` and calls slot 10 with `0xff`, which reports state 5 with code `0x14`. It then clears `+5` and `+4` and returns false. `0x5b3320` disconnects the link when `+5` is set.

Addresses: `0x5bb360`, `0x5b1b70`, `0x5b1bd0`, `0x5b1af0`, `0x5b3320`, `0x5b3327`, `0x5b332d`, `0x5bb3b0`, `0x5bb290`, `0x5bb3d0`, `0x5b2c90`, `0x5b2dea`, `0x5b1bb2`, `0x5b1bb7`, `0x5b1c15`, `0x5b1c29`, `0x5b1c30`, `0x5b1c38`
Confidence: verified
Proof: one
Test: none

`0x5b3320` tests the byte `+5` (`0x5b3327`) and, when set, loads the link from `+0x1ec` and tail-jumps to `0x5bb3b0` (`0x5b332d`), a jump to `0x5bb290`, which asks the link's connection to disconnect under the link's lock (`glue/connect`). `0x5b1bb2` is listener slot 4 with (5, `0x0c`) and the strings `LOGIN_STATE_FAILED` and `LOGIN_SERVER_DOWN`; `0x5b1c12` the same call with (`0x0b`, 0) and `LOGIN_STATE_DISCONNECTED`. `0x5b1c15` tests `[esi+4]`; `0x5b1c29` stores 1 at `+8` and `0x5b1c30` is `call [edx+0x28]` with `0xff`, 0, 0. In `0x5b2c90` the byte `0xff` less 3 indexes `0x5b2ea4` (value 9, `0x5b2dea`), which reports (5, `0x14`). `0x5bb360` calls the slot at `0x5bb390` (`call [eax+8]`) after `0x5bb2c0`. `0x5b1af0` stores 1 at `+4`; the link's timer `0x5bb3d0` calls it in state 6 (`0x5bb3f8` to `0x5bb400`). `0x5b2c90` with `+0xc` set, a reconnect, empties the realm holder, calls its slot 5 with 1 and disconnects the link, whatever the result.

## Realm list packet

`CMD_REALM_LIST` (`0x10`) is answered with a u16 size and then, counted by that size, a u32, a count byte, the records and a u16. Each record is a u32, a byte, two C strings of up to 255 characters, a float and three bytes. The link waits for the whole size, checks that the bytes it read after the size field equal it, and hands the store, set at the count byte, to the logon object's slot 13, `0x5b2230`. A wrong size returns 1, the malformed code, and the link disconnects.

Addresses: `0x5ba8d0`, `0x5ba8fd`, `0x5ba9c9`, `0x5ba9e7`, `0x5ba9eb`, `0x5b2230`
Confidence: verified
Proof: one
Test: `net::grunt_realm_list`

`0x5ba8d0` needs 2 bytes, reads the size (`0x418db0`, `0x5ba903`) and needs that many more (`0x5ba90e`), returning 0, incomplete, otherwise. It reads the u32 and the count, then per record `0x418fb0`, `0x418cb0`, two `0x4191b0` with cap `0x100`, `0x419130` and three `0x418cb0` (`0x5ba950` to `0x5ba9b3`); `0x5ba9c9` reads the tail u16 and `0x5ba9e7` compares the distance from the saved cursor with the size. `0x5ba9eb` restores the cursor to the count byte, calls `[owner+0x34]` and restores the end. A name of 256 characters or more poisons the store and returns 1. The test runs `0x5ba8d0` whole on lists of 0 to 5 records, names past the capacity, sizes off by one or two and every truncation.

## Realm record fill

`0x5b2230` empties the holder, sizes it to the count, and for record `i` stores: `i` at `+0`, the u32 at `+4`, the byte at `+8`, the name at `+9` (256 bytes), the address at `+0x109` (32 bytes), the float at `+0x12c` and the three bytes at `+0x130`, `+0x131` and `+0x132` (`glue/realms` rewrites three population values). Each name is cleaned: a character of `"*/:<>?\|` becomes a space, and trailing such characters, spaces and dots are cut. It then reads a u16 into the holder's `+0x10` and calls the holder's slot 5 with 0 when the store was read exactly to its end; otherwise it empties the holder and passes 4. A null packet passes 4 through `0x5b2560`.

Addresses: `0x5b2230`, `0x5b2274`, `0x5b2321`, `0x5b2370`, `0x5b23dd`, `0x5b23f4`, `0x5b2420`, `0x5b242b`, `0x5b3360`, `0x5b24b0`, `0x5b2560`, `0x5b2590`, `0x5b25b9`, `0x5b26f0`, `0x5b28c8`
Confidence: verified
Proof: one
Test: `net::realm_list_fill`, `net::realm_list_result`, `net::realm_name_char`

`0x5b227a` reads the count and `0x5b2286` calls `0x5b3360`; `0x5b22b9` to `0x5b2319` read the fields in the order above. `0x5b2321` `cmp ecx,[edi+0x10]; ja` stops on a read past the end. `0x5b2376` to `0x5b2396` classify each name byte: `byte - 0x20` indexes the table `0x5b2450` (up to `0x7c`) and the jump table `0x5b2444`; the characters `"*/:<>?\|` reach `0x5b2391`, which stores a space. `0x5b23a6` loops `0x5b24b0(char, 1)` from the end, which is 0 for those characters, for space and for dot. `0x5b23dd` reads the tail; `0x5b23f4` is the call of the store's end check (`[edi]+0x14`) and `0x5b2420` the success call. A record whose reads run past the end is left uncleaned, and the
whole array is then emptied. With no byte left the count is stale stack. `0x5b2560(result, store)` empties the array and passes a nonzero result to slot 5
(`0x5b2590`); for 0 it runs its own copy of the fill, trimming with inline tables
(`0x5b26f0`). Its callers, `0x5b242b` and `0x5b28c8`, pass 4 and 2 with no store, so that copy
never runs, and `0x5b25b9` is dead: the array is empty when it compares the count. The tests run
the three functions whole but for `0x5b25b9`, and `0x5b24b0` over every byte.

## File transfer

The logon server can offer a file to download: `0x30` is a name length byte, the name, a u64 size and a 16-byte digest; `0x31` is a u16 length and that many data bytes. The client replies `0x32` to accept from the start, `0x33` and a u64 offset to resume, or `0x34` to cancel. The name length is one byte, so the test of it against `0x103` never fires; the 272-byte name buffer holds the 255 name bytes at most and the NUL.

Addresses: `0x5baf70`, `0x5bb050`, `0x5bbf20`, `0x5bc050`, `0x5b2050`, `0x5bafc4`, `0x5bafcc`, `0x5bafdf`, `0x5bb039`, `0x5bb0cb`, `0x5bb008`
Confidence: verified
Proof: one
Test: `net::grunt_xfer_initiate`

`0x5baf70` waits for 1 byte and then for `n + 0x18` (`0x5bafae`), compares `n` (a zero-extended byte, `0x5bafa7`) with `0x103` (`0x5bafc4`), a test that never passes because `n` is at most 255, so the `mov bl,3` at `0x5bafcc` is dead; it reads the name with `0x419400` into a 272-byte buffer and terminates it (`0x5bafdf`, `0x5baff0`), the size with `0x4190b0`, borrows the 16 digest bytes with `0x419ac0`, returns 1 past the end, and calls the owner's slot 9 with the name, the size and the digest (`0x5bb039`), returning 2. `0x5bb050` reads the length (`0x418db0`), waits for it, borrows the bytes and calls slot 12 with the pointer and length (`0x5bb0cb`). `0x5bbf20(accept, offset)` puts `0x34` for accept 0, `0x32` for a zero offset and `0x33` with `0x418370` otherwise; `0x5bc050` puts `0x34`; `0x5b2050` sends through the link. The test runs `0x5baf70` whole on names of 0 to 255 bytes and every truncation, but for the dead `0x5bafcc` and the return of 1 past the end (`0x5bb008`), which the byte count it waits for leaves unreachable.

## Logon checksum

The checksum the logon proof carries is `ChecksumExecutables`, an HMAC-SHA-1 keyed by the 16-byte checksum salt the server sent. It hashes, in this order, the whole contents of `WoW.exe`, `fmod.dll`, `ijl15.dll`, `dbghelp.dll` and `unicows.dll`, each opened by that name in the current directory. A file that does not exist, does not open, or is not read in full adds nothing. The 20-byte result goes to the proof, which sends `SHA-1(A ‖ result)`. On a reconnect the checksum is 20 zero bytes and no file is read. With the salt `BA A3 1E 99 A0 0B 21 57 FC 37 3F B3 69 CD D2 F1` and the five stock files of this build the construction gives `95EDB27C7823B363CBDDAB56A392E7CB73FCCA20`.

Addresses: `0x5b1170`, `0x5b1c70`, `0x5b1f40`, `0x5b1c7b`, `0x5b1c98`, `0x5bbad0`
Confidence: verified
Proof: one
Test: `net::hmac_sha1`

`0x5b1170(salt, length, out)` fills two 64-byte blocks with `0x36` and `0x5c`, xors the salt into both (`0x5b11a5` to `0x5b11b3`), starts a SHA-1 with the inner block (`0x5b11be` to `0x5b11d2`) and loops the five names of the table `0x85def0` (`0x5b11e0` to `0x5b127c`, stride 4, bound `0x14`): exists (`0x42a4e0`), open (`0x42a460` for reading), size (`0x42a670`), read (`0x42a520`), and update only when the read count equals the size (`0x5b1248`). It finishes the inner hash, hashes the outer block and the 20 inner bytes, and copies 5 dwords to the output. A key longer than 64 bytes would overflow the blocks; the caller passes 16. `0x5b1c70` tests the reconnect byte `+0xc` first (`0x5b1c7b`) and then calls `0x5bbad0` with 20 zero bytes (`0x5b1c98`). The salt is kept at `+0x19c` otherwise. `net::hmac_sha1` runs `0x5b1170` whole with the file-exists call stubbed to 0, so it checks the HMAC framing and not which files are read.

## Pin challenge

When the logon challenge carries a non-zero security byte, the logon object keeps a pin grid and a salt. `0x5b1ed0(flag, seed, salt)` stores the flag at `+0x1ac`; with the flag set it fills the 10-byte grid at `+0x1ae` from the 32-bit seed and copies the 16-byte salt to `+0x1b8`. The grid is a permutation of 0 to 9: its `i`th entry is the remaining digit at index `seed mod (10 − i)`, after which the seed is divided by `10 − i`. The checksum salt callback then reports state 7, the pin state, when the flag is set, and 9, the checksum state, when it is not.

Addresses: `0x5b1ed0`, `0x5b1e70`, `0x5b1c70`, `0x5b1cd7`, `0x5b1edb`, `0x5b1eec`, `0x4100f0`
Confidence: verified
Proof: one
Test: `net::pin_shuffle`

`0x5b1edb` stores the flag; `0x5b1ee7` calls `0x5b1e70`; `0x5b1eec` to `0x5b1f08` copy 16 bytes to `+0x1b8`. `0x5b1e70` writes 0 to 9 into a local array, then 10 times: `div` the seed by the count left (`0x5b1e97`), store the digit at the remainder to the grid, and close the gap with `0x4100f0` (`0x5b1eb1`). `0x5b1cd7` is `lea eax,[eax+eax*1+7]` on `sete` of the flag: 9 for 0, 7 otherwise. `net::pin_shuffle` runs `0x5b1e70` whole.

## Pin proof

`0x5b3120(count, positions)` turns the pin positions into ASCII digits, seeds a local random pool with the clock, draws a 16-byte client salt into `+0x1c8`, stores `SHA-1(client salt ‖ SHA-1(server salt ‖ digits))` at `+0x1d8`, and reports state 9. The logon proof sends the salt and the hash when the flag at `+0x1ac` is set, taking their addresses from `0x5b1f10`.

Addresses: `0x5b3120`, `0x5b31e0`, `0x5b1f10`, `0x5b3147`, `0x5b3163`, `0x5b3177`, `0x5b318f`, `0x5b3152`, `0x5b31cc`
Confidence: verified
Proof: one
Test: `net::checksum_proof`, `net::logon_pin_proof`

`0x5b3147` adds `0x30` to each position; `0x5b3163` seeds with the clock `0x42c010`; `0x5b3177` draws 16 bytes with `0x5d1cf0`; `0x5b318f` calls `0x5b31e0`, whose two hashes are `0x5b31ef` to `0x5b3212` (inner: server salt, then digits) and `0x5b321a` to `0x5b323f` (outer: client salt, then the inner digest). `0x5b1f10` returns the flag and the addresses of `+0x1c8` and `+0x1d8`. `net::checksum_proof` runs `0x5b31e0` whole. The digits sit in a 12-byte stack buffer and
`0x5b3152` puts their NUL at the count, so a count above 11 overruns it; a position of `0xd0`
makes a NUL digit, and the hash stops there. `0x5b31cc` reports state 9 through the holder's slot
4 with the names `LOGIN_STATE_CHECKSUM` and `LOGIN_OK`. `net::logon_pin_proof` runs `0x5b3120`
whole for counts 0 to 11, the clock and the C runtime's time and `rand` stubbed.


## Pending request expiry

`ClientLink::ClientLink` builds a `Grunt::Pending` (`0x5b8490`): a timeout at `+4` and a list of 24-byte requests linked at their first bytes, each with a time at `+0x10`, 0 for none. `ClientLink::OnTimer` (`0x5bb3d0`) calls `0x5b8650(0)` each tick. The walk skips a time of 0, stops at the first request younger than the timeout, and moves the first older one to a local list; its link then ends the walk, so one request expires per call, going to slot 0 of the owner and then freed. With `force` the first request goes whatever its time. The only adder, `0x5b84e0`, is called from the unreached `ServerLink::ProveSession`, so the client's list stays empty.

Addresses: `0x5bc426`, `0x5b8490`, `0x5bb3d0`, `0x5b8650`, `0x5b86a7`, `0x5b86b6`, `0x5b86e7`, `0x5b8719`, `0x5b874b`, `0x5b84e0`, `0x5ba000`
Confidence: verified
Proof: one
Test: `net::grunt_pending_expire`

`0x5b84c3` stores the link offset 0 and `0x5b84bb` to `0x5b84c0` make the empty list. In `0x5b8650`, `0x5b86a7` skips a zero time and `0x5b86b6` (`cmp ecx,[edi+4]; jb`) stops the walk; `0x5b86bd` unlinks the node and `0x5b86d9` to `0x5b86e4` link it at the local list's tail, at the same bytes. `0x5b86e7` reads the next node from those bytes, which now hold the local list's end, marked by bit 0. `0x5b8719` calls `[[edi]]` with the node and `0x5b874b` frees it. `0x5b84e0` has the one caller `0x5ba000`, and no stored address. `0x5b849f` zeroes the id counter `+8` and `0x5b84a6` builds the lock at `+0xc`. After the expiry `0x5bb3d0` takes the lock at `+0x130` and reads the state `+0x4c` (the link's `+0x80`): in state 2 it calls slot 4 of the owner `+0x164` (`0x5bb420`), and in state 6 slot 3 (`0x5bb400`), disconnecting through `0x5bb3b0` when that returns 0.

## Server link commands

The start-up initializer `0x5b88a0` fills the server's command table `0x85e2e8`: seven 16-byte entries of a command byte, a name, a handler and an offset added to `this`, like the client's `0x85e278`; entry 0's byte and name come from the file. Its one reader, `ServerLink::OnData` (`0x5b9b50`), is unreached (`layout`). `0x5b9630`, command 2 (`CMD_GRUNT_AUTH_VERIFY`), reads and drops a name under `0x20` bytes and 16 and 20 bytes: 0 while the store lacks them, 1 for a long name, else 2. `0x5b8c40`, command `0x20` (`CMD_GRUNT_HELLO`), stores a u32 at `+0x74` and, by the mode `+0x104`, replies `20 00` (mode 1) or `20 0e`, a u16 length and the record its realm source gives (mode 0), then sets the state `+0x90` to 3.

Addresses: `0x5b88a0`, `0x85e2e8`, `0x5b9b50`, `0x5b9ba2`, `0x5b9bc2`, `0x5b9bcb`, `0x5b9630`, `0x5b9665`, `0x5b967c`, `0x5b96c4`, `0x5b8c40`, `0x5b8c8c`, `0x5b8ccd`, `0x5b8ce2`, `0x5b8d14`, `0x5b8e6e`, `0x5b8f67`
Confidence: verified
Proof: one
Test: `net::grunt_auth_verify`, `net::grunt_hello`

`0x5b9ba2` starts the scan at `0x85e2e8`, comparing each entry's byte with `cmp byte [ecx],dl` and stepping `0x10`; `0x5b9bc2` adds `[eax+0x85e2f4]` to `this` and `0x5b9bcb` calls `[eax+0x85e2f0]`. `0x5b88e9` and `0x5b88aa` pair the byte 2 (`0x85e2f8`) with `0x5b9630` (`0x85e300`); `0x5b892e` and `0x5b88c8` pair `0x20` (`0x85e328`) with `0x5b8c40` (`0x85e330`). In `0x5b9630`, `0x5b9642` and `0x5b967c` are `0x5b90d0` with 1 and the length plus `0x24`; `0x5b9665` returns 1 at `0x20` or more, and `0x5b96c4` returns 1 when the reads passed the size, else 2. In `0x5b8c40`, `0x5b8c72` reads the u32 and `0x5b8c8c` stores it; `0x5b8ccd` puts `0x20`; `0x5b8ce2` tests the mode; `0x5b8d14` calls slot 4 of `+0x108`; `0x5b8e6e` sets the length; `0x5b8f67` stores 3.
