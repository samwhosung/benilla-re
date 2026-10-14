# net: crypto

The two SHA-1 implementations the logon code uses.

The client carries two SHA-1 implementations that differ in context layout and in whether the compression leaves its input intact. The session challenge, the random pool, SRP6 and the logon proof use the one at `0x5d1db0`; the logon checksum and the pin hash use the one at `0x5d1a40`. The hash is SHA-1 by the standard constants and padding. The doc records what differs and what a reimplementation must reproduce.

Roots: `0x5d1db0`, `0x5d1de0`, `0x5d1a40`, `0x5d1a80`, `0x5d1b60`

## SHA-1 overwrites its input

The client's SHA-1 (`0x5d1db0` family: init, update, transform, final) keeps its 16-word
message schedule in the 64-byte block it is given, so a transformed block is left holding
schedule words. The update hands the transform the caller's own bytes for every full 64-byte
block that follows the context's buffered bytes; those spans of the caller's buffer are
overwritten. A short input, or the part copied into the context's buffer, is not. The hash
itself is SHA-1 by its constants; its rounds are not recorded here.

Addresses: `0x5d1db0`, `0x5d1de0`, `0x5d1ec0`, `0x5d32b0`, `0x5d1e48`, `0x5d1e69`, `0x5d233f`, `0x5d2386`
Confidence: verified
Proof: one
Test: `net::sha1_family_b`, `net::sha1_prng_next`, `net::sha1_prng_seed`, `net::srp6_client_session`, `net::srp6_credential_hash`, `net::srp6_file_hmac`, `net::srp6_interleave`

`0x5d1db0` stores `0x67452301`, `0xefcdab89`, `0x98badcfe`, `0x10325476` and `0xc3d2e1f0` and zeroes
the 64-bit bit count at `+0x14`. `0x5d1de0` fills the buffer at `+0x1c` and transforms it
(`0x5d1e48`, `edx` the buffer), then calls the transform with `edx` pointing into the caller's
data (`0x5d1e63`, `0x5d1e69`) for each further 64 bytes. In `0x5d1ec0`, `edx` is the block: from
the seventeenth word on, each schedule word is stored back into it, `mov [edx],esi` at
`0x5d233f`, `mov [edx+0x4],esi` at `0x5d2386` and so on. The round additions use `0x5a827999` and
`0x6ed9eba1`, and the subtraction forms of `0x8f1bbcdc` and `0xca62c1d6`.
`sha1_family_b`, `sha1_prng_next`, `sha1_prng_seed`, `srp6_client_session`, `srp6_credential_hash`,
`srp6_file_hmac` and `srp6_interleave` run `0x5d1db0`, `0x5d1de0`, `0x5d1ec0` and `0x5d32b0` whole.

## SHA-1 first context

The logon checksum and the pin hash use a second SHA-1 implementation. Its context is a 64-bit bit count (low dword `+0`, high `+4`), five state words from `+8`, and a 64-byte buffer from `+0x1c`. Init zeroes the count and loads the five standard words. Update adds 8 per byte to the count, tops up the buffer, compresses whole blocks straight from the caller's bytes, and buffers the rest. Final saves the count big-endian, pads with `0x80` and zeros to 56 bytes modulo 64, adds the count, and writes the five state words big-endian.

Addresses: `0x5d1a40`, `0x5d1a80`, `0x5d1b30`, `0x5d1b60`, `0x80a930`, `0x5d1ad3`, `0x5d1af8`
Confidence: verified
Proof: one
Test: `net::sha1_family_a`, `net::hmac_sha1`, `net::checksum_proof`

`0x5d1a40` stores 0, 0 and the words at `+8` to `+0x18`. `0x5d1a80` multiplies the length by 8 into the count (`0x5d1a97`, `0x5d1a9f`), takes `(count >> 3) & 0x3f` as the buffered length, and calls `0x5d1420` with `ecx` the state and `edx` the block, for the filled buffer (`0x5d1ad3`) and for each further block of the caller's data (`0x5d1af8`). `0x5d1b60` writes the count to a local (`0x5d1b74`), updates with `0x80a930`, a block of `0x80` and 63 zero bytes, for `(55 − (count >> 3)) mod 64 + 1` bytes, then with the 8 count bytes, and stores the words with the loop at `0x5d1bc0`. `0x5d1b30` updates with a C string's bytes. The users are `0x5b1170` and `0x5b31e0`; every other hash in net uses `0x5d1db0`. `net::sha1_family_a` runs `0x5d1a40`, `0x5d1a80` and `0x5d1b60` whole, with `0x5d1420` inside them; `net::hmac_sha1` and `net::checksum_proof` run them under `0x5b1170` and `0x5b31e0`.

## SHA-1 first compression

The compression `0x5d1420` copies the block big-endian into a local array of 80 words and expands it, so the caller's block is read and not written, unlike `0x5d1ec0`. Its code carries the four round constants `0x5a827999`, `0x6ed9eba1`, `0x8f1bbcdc` and `0xca62c1d6`, each in five instructions. The rounds themselves are not read.

Addresses: `0x5d1420`, `0x5d1431`, `0x5d1461`, `0x5d1487`, `0x5d1a35`
Confidence: inferred
Proof: none
Test: `net::sha1_family_a`

`0x5d1431` to `0x5d1458` build each word from four bytes with shifts and ors into `[ebp+esi*4-0x158]` for 16 words; `0x5d1461` to `0x5d1485` extend to 80 with the rotate-left-1 of four earlier words xored; `0x5d1487` loads the five state words. The function ends at `0x5d1a35` after storing the five sums back to the state. Whether every round matches SHA-1 is a lead for a bit-exact test. `net::sha1_family_a` runs `0x5d1420` inside the whole hash and compares its digest with SHA-1.

