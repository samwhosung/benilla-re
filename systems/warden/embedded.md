# warden: the built-in module

The module the exe carries, the one that runs first and, in the code read here, performs the whole wire
protocol: the cipher, the download of further modules and the answer to the hash request. It is a signed
module like any other, stored encrypted in the exe's data. Its addresses below are offsets inside its own
image, four hex digits, not exe addresses. Nothing here describes a module the server sends.

Roots: none

## The built-in module

The exe holds 4787 bytes at `0x86ddb0` and a 16-byte RC4 key at `0x86f064`. `InitWardenHandler` decodes them
with `LoadWardenModule` at connect. The decrypted blob is the dword 6584, a zlib stream and the 260-byte
signature tail. The stream inflates to 6584 bytes: an image of `0x6000` bytes with three sections (`0x1000`,
size `0x1914`, execute and read; `0x3000`, `0x87`, read only; `0x4000`, `0x9c`, read and write), 20
relocations, no imports and one export, ordinal 1. The export, at `0x2460`, allocates a `0x214`-byte object
through service 3, stores its table `0x3000` and the client's pointer, and returns it.

Addresses: `0x86ddb0`, `0x86f064`, `0x6ca2f0`, `0x6ca2ff`, `0x6ca304`, `0x6ca309`, `0x6ca30e`, `0x6ca360`
Confidence: verified
Proof: one
Test: `warden::decoder_real_signature`, `warden::builtin_cipher_and_messages`

`0x6ca2ff` pushes the key, `0x6ca304` loads the length `0x12b3` and `0x6ca309` the blob address; `0x6ca30e` calls
`LoadWardenModule`. The key is `35 86 61 01 bc 27 ad 47 18 64 f7 52 5d 7c 3c 18`; the blob decrypts to
`b8 19 00 00 78 da…`; its last 260 bytes begin `NGIS` and verify against `MAIEV.MOD` and the public key. The
header is `0x6000, 0x1381, 0x5000, 0x14, 0x3078, 1, 1, 0, 0, 3`: the dword `0x1381` is not read by the loader. The
runs end at byte 6584 of 6584. The module's table at `0x3000` is `0x2893`, `0x2416`, `0x26f6`, `0x2413`
(the four slots the client calls). The test loads the image with this crate's loader, maps it and runs
these slots.

## The module cipher

The module encrypts with RC4, one state per direction, both keyed from the session key. Start asks service 6 for a
handover buffer and, given exactly `0x208` bytes, uses them as the two states. Otherwise it hashes each 20-byte
half of the key with SHA-1 (`o1`, `o2`) and runs a stream: a block of 20 zeros, each refill `SHA-1(o1 ‖ block ‖ o2)`,
the first made at start. The first 16 stream bytes key the send state (client to server), the next 16 the receive
state. A `0x2e6` payload is decrypted in place with the receive state; each reply is encrypted with the send state
before service 0. Unload passes both states and a count to service 5.

Addresses: `0x6ca1e0`, `0x6ca227`, `0x6ca900`, `0x6ca960`, `0x6ca290`
Confidence: verified
Proof: one
Test: `warden::builtin_cipher_and_messages`

The start (`0x2893`) calls service 6 with `this+0xc` and a length of `0x208`; a result of true with that length skips the
derivation. `0x1098` hashes the halves into `this+0x18` and `this+0x2c` (`0x23bc`); `0x1112` makes a block
(`0x226a`, `0x2300`); `0x10e1` takes bytes, refilling at 20. `0x1473` is the RC4 key schedule over 256
bytes, then the indices at `+0x100` and `+0x101`; the send state is at `this+0xc`, the receive state at
`this+0x10e`, each `0x102` bytes. `0x1410` is the RC4 stream. The packet slot (`0x26f6`) decrypts with the
receive state; `0x248b` encrypts with the send state and tail-jumps to service 0. The unload (`0x2416`) calls
service 5 with `this+0xc` and `0x208`, then service 4. The dword at `this+0x210`, last of the `0x208` bytes, counts
bytes a failed packet left; the next packet's first so many bytes are taken as arrived, not decrypted.

## The module's messages

A decrypted payload holds messages, an opcode byte and a body each; opcode 3 or more fails the packet. Opcode 0
(16-byte id, 16-byte key, dword size) asks service 1 to load the id: loaded, the reply is the byte 1; else the module allocates size plus `0x80` bytes and
replies 0. Opcode 1 (16-bit length, bytes) appends to that buffer and MD5-hashes it; at full size, a hash equal to the id stores the bytes (service 2) and loads them (service 1),
replying 1 if that loads, and any other hash restarts the download with reply 0. Opcode 2 (length byte, string)
replies the byte 2, then the SHA-1 and MD5 of the string up to its first zero.

Addresses: `0x6ca770`, `0x6ca840`, `0x6ca6a0`
Confidence: verified
Proof: one
Test: `warden::builtin_cipher_and_messages`, `warden::builtin_download_in_pieces`

The packet slot (`0x26f6`) dispatches through the dword table at `0x4008`: `0x2627` for opcode 0, `0x24ce` for
opcode 1, `0x279f` for opcode 2, and a return of 3 fails the packet, 2 or 1 stops the loop with success, 0
continues. `0x2627` needs 36 bytes, reads the id, the key and the size, frees any download in progress
(service 4), and calls service 1 with the id and the key (`[eax+4]`); on false it allocates through
service 3 (`[eax+0xc]`) and starts MD5 at `+0x20`. `0x24ce` checks the piece fits (`+0x78`, `+0x7c`), copies it to
`+0x80`, hashes it (`0x1d54`), and when the size is reached finishes (`0x1e17`) and compares with the id by
`repz cmps`; a match calls service 2 (`[eax+8]`), frees the buffer, calls service 1 again. `0x279f` hashes with
`0x2360` (SHA-1) and `0x1e89` (MD5) and sends 37 bytes through `0x248b`. The test runs the downloads, the
hash requests and an opcode of 3 against the module's own code.
