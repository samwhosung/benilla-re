# net: account data

How the client keeps its settings files in step with the server's copies.

Eight sections of player data (configuration, key bindings, macros, layout and chat settings) are kept both under `WTF\Account` and on the server. The table `0x85dcf0` holds one 44-byte record per section. When a world session starts the server sends an MD5 digest of each section; the client compares it with the digest of its own file and with the digest it cached the last time they agreed, then uploads or requests the section. Later writes upload the section 30 seconds after the last one. The code sits in the net object files between `FriendList.cpp` and the login code, and the code that reads and writes the sections lies in the interface, key binding and macro files.

Roots: `0x5af840`, `0x5afe00`, `0x5afd30`, `0x5afe50`, `0x5afed0`, `0x5af6b0`

## Section table

Each record of `0x85dcf0`, 44 bytes, holds: `+0` a scope byte, 1 for an account-wide section and 0 for a per-character one; `+4` the name of a second file, or 0; `+8` the cache file name; `+0xc` the pending byte; `+0x10` and `+0x14` a read callback and its argument saved while pending; `+0x18` the save timer; `+0x1c` the 16-byte cached digest. Only the `+8` file is hashed, uploaded and replaced.

| section | scope | `+8` file | `+4` file |
|---|---|---|---|
| 0 | account | `config-cache.wtf` | none |
| 1 | character | `config-cache.wtf` | none |
| 2 | account | `bindings-cache.wtf` | none |
| 3 | character | `bindings-cache.wtf` | none |
| 4 | account | `macros-cache.txt` | `macros-local.txt` |
| 5 | character | `macros-cache.txt` | `macros-local.txt` |
| 6 | character | `layout-cache.txt` | none |
| 7 | character | `chat-cache.txt` | none |

Addresses: `0x85dcf0`, `0x5af840`, `0x5af880`, `0x5af9c0`, `0x5af845`, `0x5afe00`
Confidence: verified
Proof: one
Test: none

The records are initialised data: the scope bytes are at `0x85dcf0`, `0x85dd1c` and so on at 44-byte steps, the name pointers at `+8`, and the records of sections 4 and 5 (`0x85dda0`, `0x85ddcc`) hold the second name at `+4`. `0x5af840` sets the byte at `+0xc` of each record to 1 (`0x5af845`, stride `0x2c`), loads the digests, and registers `0x5af8e0` for opcode `0x209` and `0x5afc60` for `0x20c`. Its one caller is `0x4015e7`. `0x5afe00`, called from `0x402048`, kills the timers and clears both handlers.

## Section files

`0x5af6b0(scope, name, write)` opens a section file and returns its handle, or `-1`. It needs the account name (`0xc27d08`); an account-wide path is `WTF\Account\<account>\<name>` and a per-character path is `WTF\Account\<account>\<realm>\<character>\<name>`, with the realm from the `realmName` CVar and the character the selected one. A write first creates the directory when it is missing, and opens with creation 2 (create or truncate); a read opens with creation 3 (existing only), share mode 0. A per-character read that fails is retried once at `WTF\Account\<account>\<character>\<name>`.

Addresses: `0x5af6b0`, `0x5af6c3`, `0x5af6f0`, `0x5af71d`, `0x5af72d`, `0x5af741`, `0x5af7a2`, `0x5af7cc`, `0x42a460`, `0x5abd80`
Confidence: verified
Proof: one
Test: none

`0x5af6c3` calls the account-name getter and returns `-1` for none, as `0x5af6d2` does for a null name. The formats are the strings `WTF\Account\%s` (`0x5af6f0`) and `WTF\Account\%s\%s\%s` (`0x5af718` with the account, `0x5ab7d0`'s realm and `0x5abdc0`'s character). `0x5af727` to `0x5af741` test the directory (`0x42a500`) and create it (`0x42a960`) for a write. `0x5af757` and `0x5af769` append `\` and the file name. `0x5af782` and `0x5af784` give the creation 2 and the access `0x40000000` for a write, `0x5af799` and `0x5af79d` the creation 3 and `0x80000000` for a read. The retry is `0x5af7a7` to `0x5af7f8`, with the format given the character where the realm was, and only for scope 0.

## Section digests

`0x5af940(scope, name)` returns the MD5 of a section file, read 1,024 bytes at a time, or 16 zero bytes when it cannot be opened. The cached digests of the eight sections are one file, `cache.md5` in the account directory: 8 × 16 bytes in section order, read by `0x5af880` and written by `0x5af9c0`. A read that fails leaves the digests as they were; each digest is cleared first, so a missing file reads as zeros.

Addresses: `0x5af940`, `0x5af880`, `0x5af9c0`, `0x5af96d`, `0x5af995`, `0x5af9ad`, `0x661da0`, `0x661dd0`, `0x661e90`
Confidence: verified
Proof: one
Test: none

`0x5af95e` opens the file with write 0; `0x5af96d` is `MD5Init`, `0x5af995` an update per 1,024-byte read (`0x5af976`), `0x5af9ad` `MD5Final`. `0x5af880` passes scope 1, name `cache.md5` and write 0, and loops 8 times over `+0x1c` with `0x10` bytes (`0x5af89b` to `0x5af8c1`); `0x5af9c0` is the same with write 1.

## Server digests

`SMSG_ACCOUNT_DATA_TIMES` (`0x209`) is eight 16-byte digests, one per section, in section order. For each section the client hashes its file, runs the check below with the local, cached and server digests, and after the eighth saves the cached digests. A digest of 16 zero bytes means the server has no copy.

Addresses: `0x5af8e0`, `0x5af909`, `0x5af91b`, `0x5af92c`
Confidence: verified
Proof: one
Test: none

`0x5af8e0` loops over the eight records (`0x5af8f3` to `0x5af924`, stride `0x2c`): `0x5af8fc` hashes the file with `0x5af940`, `0x5af909` reads 16 bytes of the packet with `0x419aa0`, and `0x5af91b` calls the check with the cache at `+0x1c`. `0x5af92c` is the call of `0x5af9c0`.

## Section check

`0x5afa10(section, local, cache, server)` first clears the section's pending byte. With a zero server digest it uploads the local file when the local digest is not zero. Otherwise, when the local digest differs from the cached one, it uploads when the local digest also differs from the server's; when the local digest equals the cached one, it requests the section if the server's differs. The cached digest becomes the local one, except when the local digest equalled the cached one: then it becomes the server's. If the section is not pending and a read callback is saved, the check runs it; the saved callback stays set.

Addresses: `0x5afa10`, `0x5afa26`, `0x5afa42`, `0x5afa54`, `0x5afa65`, `0x5afa6c`, `0x5afa8a`, `0x5afa95`, `0x5afab1`, `0x5afbe0`, `0x5afae0`
Confidence: verified
Proof: one
Test: none

`0x5afa26` tests the server digest for all zeros, leaving `al` 1; `0x5afa42` clears `+0xc`; `0x5afa49` `jne 0x5afa79` is taken for a zero server digest. `0x5afa54` (`repz cmps`, 4 dwords) compares local with cached and jumps to `0x5afa79` when they differ; `0x5afa65` compares local with server. `0x5afa6c` calls `0x5afbe0` (request) and `0x5afa8a` calls `0x5afae0` (upload). `0x5afa74` and `0x5afa92` select the digest copied to the cache at `0x5afa95` to `0x5afaa8`. `0x5afab1` tests the pending byte and calls `0x5afe50` with the saved callback.

## Section transfer

`0x5afbe0` sends `CMSG_REQUEST_ACCOUNT_DATA` (`0x20a`) with the section number and sets its pending byte. `0x5afae0` sends `CMSG_UPDATE_ACCOUNT_DATA` (`0x20b`): the section number, the file's size, and the file compressed with zlib level 1, to the end of the packet; nothing is sent when the file does not open. `SMSG_UPDATE_ACCOUNT_DATA` (`0x20c`) is a section number, a size and a zlib stream to the end of the packet: `0x5afc60` inflates it into a buffer of that size on the stack, overwrites the section file with the result unless the size is 0, clears the pending byte, and runs the saved read callback. It does not test the inflate's result or the section number, and the size is the server's.

Addresses: `0x5afbe0`, `0x5afae0`, `0x5afc60`, `0x5afb55`, `0x5afcbc`, `0x5afd01`, `0x5afc9c`
Confidence: verified
Proof: one
Test: none

The upload allocates the file size twice on the stack (`0x5afb1e`, `0x5afb45`), caps the compress output at the input size (`0x5afb42`) and tests neither the compress result nor the read count. `0x5afbec` puts `0x20a` and `0x5afc13` the section number; `0x5afc2f` stores 1 in the pending byte. `0x5afae0` reads the file with `0x42a520`, `0x5afb55` is the compress call with level 1 (`push 1`), and the packet is `0x5afb5c` (`0x20b`), the section (`0x418190`), the size (`0x4182b0`) and the bytes (`0x418c10`). `0x5afc72` and `0x5afc7d` read the section and size; `0x5afc98` borrows the rest of the packet; `0x5afca8` is `__alloca_probe` on the size; `0x5afcbc` calls `0x660740` and nothing tests its result; `0x5afd01` clears the pending byte of the section number, which is not range-checked.

## Section reads and writes

`0x5afe50(section, callback, argument)` runs `callback(file, file2, argument)` on the section's opened `+4` and `+8` files while the section is not pending, and closes them after; while it is pending the callback and argument are stored and the call returns. `0x5afed0(section, writer, argument)` does the same with both files opened for writing, then restarts the section's 30-second save timer, and does nothing while the section is pending. A section without a `+4` file passes `-1` for it. When the timer fires, `0x5afd70` hashes the file and, if the digest differs from the cached one, uploads it, stores the digest and saves `cache.md5`. `0x5afd30` runs every armed timer at once and is called before a logout request.

Addresses: `0x5afe50`, `0x5afed0`, `0x5aff40`, `0x5aff70`, `0x5afd70`, `0x5afd30`, `0x5afe00`, `0x401520`, `0x401550`, `0x5afe5a`, `0x5afee0`
Confidence: verified
Proof: one
Test: none

`0x5afe5a` tests the pending byte and stores the callback (`0x5afe69`, `0x5afe6f`); `0x5afeae` is `call ebx`, the callback, with `ecx` the first file and `edx` the second. `0x5afee0` returns when pending; `0x5aff30` calls `0x5aff40`, which kills the old timer (`0x5aff70`) and arms `0x401520` with 30,000 ms (`0x7530`), `0x5afd70` and the section number. `0x5afd70` clears the timer word at the end (`0x5afddd`); `0x5afd30` calls `0x401550` then `0x5afd70` for each armed timer. `0x5afe50` has eight call sites and `0x5afed0` seven, in code outside net, and `0x5ab049` is the only call of `0x5afd30`.
