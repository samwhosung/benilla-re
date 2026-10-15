# warden: the built-in module

The module the exe carries, the one that runs first and, in the code read here, performs the whole wire
protocol: the cipher, the download of further modules and the answer to the hash request. It is a signed
module like any other, stored encrypted in the exe's data. Its addresses below are offsets inside its own
image, four hex digits, not exe addresses. Nothing here describes a module the server sends.

Roots: none

## The built-in module

The exe holds 4787 bytes at `0x86ddb0` and a 16-byte RC4 key at `0x86f064`; `InitWardenHandler` passes them with the
length `0x12b3` to `LoadWardenModule` at connect. The decrypted blob is the dword 6584, a zlib stream and the 260-byte
signature tail: `NGIS` and a 256-byte signature (`modules`). The stream inflates to 6584 bytes: an image of `0x6000`
bytes with three sections (`0x1000`, size `0x1914`, execute and read; `0x3000`, `0x87`, read only; `0x4000`, `0x9c`,
read and write), 20 relocations, no imports and one export, ordinal 1. The export is called with `ecx = 0xce8954`.

Addresses: `0x86ddb0`, `0x86f064`, `0x6ca2f0`, `0x6ca2ff`, `0x6ca304`, `0x6ca309`, `0x6ca30e`, `0x6ca360`
Confidence: verified
Proof: two
Test: `warden::decoder_real_signature`, `warden::builtin_cipher_and_messages`

`0x6ca2ff` pushes the key, `0x6ca304` loads the length `0x12b3` and `0x6ca309` the blob address; `0x6ca30e` calls
`LoadWardenModule`. The key is `35 86 61 01 bc 27 ad 47 18 64 f7 52 5d 7c 3c 18`; the blob decrypts to `b8 19 00 00 78
da…`; its last 260 bytes begin `NGIS` and verify against `MAIEV.MOD` and the public key. The header is `0x6000,
0x1381, 0x5000, 0x14, 0x3078, 1, 1, 0, 0, 3`: the dword `0x1381` is not read by the loader. The runs end at byte 6584
of 6584. The export, at `0x2460`, allocates a `0x214`-byte object through service 3 (`[eax+0xc]` of the table in
`ecx`), stores the table `0x3000` and the client's pointer, and returns it; the client then calls the object's slot 0
with the session key. The module's table at `0x3000` is `0x2893`, `0x2416`, `0x26f6`, `0x2413` (the four slots the
client calls). The test loads the image with this crate's loader, maps it and runs these slots.

## The module cipher

The module encrypts with RC4, one state per direction, both keyed from the 40-byte session key (`NetClient+0x48`,
`net/session#session-key`). Start asks service 6 for a handover buffer and, given exactly `0x208` bytes, uses them as
the two states. Otherwise it hashes each 20-byte half of the key with SHA-1 (`o1`, `o2`) and runs a stream: a block of
20 zeros, each refill `SHA-1(o1 ‖ block ‖ o2)`, the first made at start. The first 16 stream bytes key the send state
(client to server), the next 16 the receive state. A `0x2e6` payload is decrypted in place with the receive state;
each reply is encrypted with the send state before service 0.

Addresses: `0x6ca1e0`, `0x6ca227`, `0x6ca900`, `0x6ca960`, `0x6ca290`
Confidence: verified
Proof: two
Test: `warden::builtin_cipher_and_messages`

The start (`0x2893`) calls service 6 with `this+0xc` and a length of `0x208`; a result of true with that length skips
the derivation. `0x1098` hashes the halves into `this+0x18` and `this+0x2c` (`0x23bc`); `0x1112` makes a block
(`0x226a`, `0x2300`); `0x10e1` takes bytes, refilling at 20. `0x1473` is the RC4 key schedule over 256 bytes, then the
indices at `+0x100` and `+0x101`; the send state is at `this+0xc`, the receive state at `this+0x10e`, each `0x102`
bytes. `0x1410` is the RC4 stream. The packet slot (`0x26f6`) decrypts with the receive state; `0x248b` encrypts with
the send state and tail-jumps to service 0. The unload (`0x2416`) calls service 5 with `this+0xc` and `0x208`, then
service 4. The dword at `this+0x210`, last of the `0x208` bytes, is the count of bytes of the last packet that no
message used (`0x2734`, `0x2775` to `0x2780`); the next packet's first so many bytes are not decrypted (`0x2704` to
`0x2725`), on the assumption that the sender repeats them. The client hands over each packet alone, so after a
leftover the next packet's first bytes are read as they arrived.

## The packet loop

A packet holds messages, an opcode byte and a body each; an opcode of 3 or more fails the packet. The slot `0x26f6`
decrypts the packet from the leftover count on and reads messages one at a time. A handler returns 0 (the message is
done, the next follows), 1 (done, stop), 2 (the bytes of a message are not all there, stop) or 3 (fail). The slot
returns 0 for 3 and 1 otherwise, and the client does not read it. Every reply is encrypted with the send state and
sent through service 0, so it goes out as message `0x2e7`.

Addresses: `0x6ca615`, `0x6ca6a0`
Confidence: verified
Proof: two
Test: `warden::builtin_cipher_and_messages`, `warden::builtin_download_in_pieces`

The handlers are the dword table at `0x4008`: `0x2627`, `0x24ce` and `0x279f`. A 3 and a 2 return before the count of
bytes used is kept (`0x276b`, `0x2770`); a 0 and a 1 reduce `this+0x210` and the count handed back by what the handler
used (`0x2775` to `0x2780`). The loop also ends when no byte is left (`0x278d`). The test runs the downloads, the hash
requests and an opcode of 3 against the module's own code.

## The id message

Opcode 0 is a 16-byte id, a 16-byte key and a dword size: 36 bytes, or the handler returns 2. It asks service 1 to
load the id. Loaded, the reply is the one byte 1 and the packet stops. Otherwise the module allocates size plus `0x80`
bytes, starts the MD5 and replies the one byte 0; the next message follows.

Addresses: `0x6ca770`, `0x6ca6a0`
Confidence: verified
Proof: two
Test: `warden::builtin_cipher_and_messages`, `warden::builtin_download_in_pieces`

`0x2627` needs 36 bytes (`0x2638`), reads the id, the key and the size, frees any download in progress (service 4),
and calls service 1 with the id and the key (`[eax+4]`); on false it allocates through service 3 (`[eax+0xc]`) and
starts the MD5 at `+0x20` (`0x23e8`). It returns 1 after a hit (`0x26ab` to `0x26ae`) and 0 after a miss (`0x26ee`).

## The piece message

Opcode 1 is a 16-bit length and that many bytes. They append to the download and update its MD5. At full size, a hash
equal to the id stores the bytes (service 2) and loads them (service 1): loaded, the reply is the one byte 1 and the
packet stops; not loaded, nothing is sent and the packet fails. Any other hash restarts the download and replies the
one byte 0. A piece longer than the bytes still expected, or any piece with no download started, fails the packet.
Fewer than 2 bytes, or fewer than the length, returns 2.

Addresses: `0x6ca770`, `0x6ca840`, `0x6ca6a0`
Confidence: verified
Proof: two
Test: `warden::builtin_cipher_and_messages`, `warden::builtin_download_in_pieces`

`0x24ce` returns 3 with no download (`0x24e0`), 2 under 2 bytes (`0x24f7`) or with fewer bytes than its length
(`0x2515`), and 3 when the length exceeds the bytes still expected (`0x2529`; `+0x78`, `+0x7c`). It copies the piece
to `+0x80`, hashes it (`0x1d54`) and returns 0 while the download is short (`0x2569`). At the size it finishes the
hash (`0x1e17`) and compares with the id by `repz cmps`. A mismatch restarts the download and replies 0 (`0x25b8`); a
match calls service 2 (`[eax+8]`), frees the buffer, calls service 1 again, and returns 3 when it declines (`0x2607`),
else replies 1 and returns 1.

## The hash request

Opcode 2 is a length byte of at most `0xfe` and that many bytes. The reply is the byte 2, then the SHA-1 and the MD5
of the string up to its first zero: 37 bytes in all. The handler returns 0. A length of `0xff` fails the packet, and a
string not all there returns 2.

Addresses: `0x6ca6a0`
Confidence: verified
Proof: two
Test: `warden::builtin_cipher_and_messages`, `warden::builtin_download_in_pieces`

`0x279f` checks that the length byte and the string are there (`0x27be`, `0x27cd`; else 2), reads the string
(`0x103e`; a length of `0xff` leaves the cursor past the end, so `0x27ea` goes to the fail at `0x27ec`) and hashes
with `0x2360` (SHA-1) and `0x1e89` (MD5), both over the bytes up to the first zero. `0x2854` stores the byte 2 and
`0x287d` sends the 37 bytes through `0x248b`.
