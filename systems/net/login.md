# net: login

The logon server's protocol: the challenge, SRP6, the proof and the session key.

The logon object (`Login.cpp`, vtable `0x80a5d4`) drives the login screen; it owns a
`ClientLink` (`Grunt.cpp`) that speaks the logon protocol over its own `WowConnection` on
port 3724. The link's messages are a command byte and a body, with no size header and no
cipher. The client's half of SRP6 is `0x5d3650`, over Storm's big numbers and the client's own
SHA-1 (`0x5d1db0` family); it leaves a 40-byte session key that the logon object keeps and the
world session later uses as the header cipher's key (`wire`, `session`). Randomness for SRP6
comes from a SHA-1 random pool at `0xc2a428`, seeded at startup.

`Grunt.cpp` also holds `ServerLink`, the logon server's link to a realm server; nothing live
constructs it (`layout`, Unreached functions).

Roots: `0x5b20e0`, `0x5b2c90`, `0x5bb0e0`, `0x5bbad0`, `0x5bc3e0`, `0x5b8000`, `0x5b7ff0`

## Logon commands

The link appends received bytes to a buffer and reads commands from it: a command byte,
looked up in a table of seven, whose handler returns 0 when the command is incomplete, 1 when
it is malformed, and 2 when it was consumed. An incomplete command waits for more bytes; a
malformed one, or an unknown byte, logs `Bad command received`, empties the buffer and
disconnects. The commands, by byte: `0x00` logon challenge, `0x01` logon proof, `0x02` reconnect
challenge, `0x03` reconnect proof, `0x10` realm list, `0x30` transfer initiate, `0x31` transfer
data.

Addresses: `0x5bb0e0`, `0x5bb132`, `0x5bb15b`, `0x5bb17d`, `0x5bb1df`, `0x5b87b0`, `0x85e278`
Confidence: verified
Proof: one
Test: none

`0x85e278` holds 16-byte entries {command byte, name, handler, `this` offset}; the first entry's
byte and name are in the file and `0x5b87b0`, a static initializer, stores the rest: handlers
`0x5baa20`, `0x5bacb0`, `0x5bae00`, `0x5baec0`, `0x5ba8d0`, `0x5baf70` and `0x5bb050`, named
`ClientLink::CMD_AUTH_LOGON_CHALLENGE` and so on. `0x5bb0e0`, slot 7 of `0x80a7dc`, appends the bytes at `+0x17c`,
needs one byte (`0x5b90d0`), searches the table linearly (`0x5bb132` to `0x5bb142`) and calls
the handler at `0x5bb15b`. Result 0 keeps the bytes from the command's start (`0x5bb1ed`
onwards); 2 loops; 1 or no match logs at `0x5bb17d` and calls `0x5bb290`, which closes the
connection (`0x5bb1df`).

## Logon challenge

The client opens with command `0x00`, then `0x03`, a 16-bit size of the rest, and: game
`'WoW'`, version 1, 12, 1, build 5875, platform `'x86'`, system `'Win'`, the locale, the
time-zone bias, the local IPv4 address, and the account's length and bytes. The four-letter
values are little-endian dwords, so `'x86'` goes out as the bytes `36 38 78 00`. The account and
password are upper-cased first. A reconnect sends the same body after `0x02`, `0x02`.

| field | size | value |
|---|---|---|
| game | 4 | `0x00576f57` |
| version | 1, 1, 1 | 1, 12, 1 |
| build | 2 | `0x16f3` |
| platform, system | 4, 4 | `0x00783836`, `0x0057696e` |
| locale | 4 | the client's locale code |
| time zone | 4 | minus the system's bias in minutes, or 0 |
| address | 4 | the socket's local address |
| account | 1 + n | length, then the upper-cased bytes |

Addresses: `0x5b20e0`, `0x5bb540`, `0x5bb460`, `0x5bb6d0`, `0x5bb5f9`, `0x5bb605`, `0x5bb523`
Confidence: verified
Proof: two
Test: none

`0x5b20e0` fills the record: `0x00576f57` at `0x5b20f6`, `0x429f40()` (`0x00783836`) and
`0x429f50()` (`0x0057696e`), bytes 1, `0xc`, 1 (`0x5b210d` to `0x5b211a`), `0x16f3` at `0x5b211e`,
the locale global, and `0x4297c0()`, which negates `GetTimeZoneInformation`'s bias and returns 0
on failure. `0x5bb540` stores the account at `+0x19c`, upper-cases it and a copy of the password
with `0x64b460`, hashes both (`0x5d3590`), writes `0x00` and `0x03` (`0x5bb5f9`, `0x5bb605`), calls
`0x5bb460` and sends; it then zeroes the password copy. `0x5bb460` writes the fields in the order
above, the address from `+0x7c`, and patches the size at `0x5bb523` as the bytes after it.
`0x5bb6d0` writes `0x02`, `0x02`.

## Challenge reply

The reply to `0x00` is a zero byte, then a result. A non-zero result is reported to the logon
object (results 16 and above as `0xff`). Result 0 carries `B` (32 bytes), `g` with a one-byte
length, `N` with a one-byte length, the salt (32), a 16-byte checksum salt and a security byte;
a non-zero security byte adds a dword and 16 bytes. The client keeps `B`, runs SRP6, and on
success reports the security fields and then the checksum salt; a failed SRP6 reports result 5.
A non-zero first byte is malformed.

Addresses: `0x5baa20`, `0x5baa57`, `0x5baa77`, `0x5babdf`, `0x5bac05`, `0x5bac16`, `0x5bac58`
Confidence: verified
Proof: one
Test: none

Each read is guarded by `0x5b90d0`, which returns 1 when the store holds that many unread
bytes; a shortfall returns 0. `0x5baa57` tests the first byte, `0x5baa77` the result. The reads
are the data store's borrowing get with lengths `0x20`, `g`'s, `N`'s, `0x20`, `0x10`, then the
security byte, then a dword and `0x10` when it is set. A cursor past the end returns 1
(`0x5bab91`). `B` is copied to `+0x1dc`; `0x5babdf` calls `0x5d3650(N, |N|, g, |g|, salt, 32, B,
32, 0xc2a428)` on `+0x84`. Success sets state 4 and calls the owner's slots 6 and 5
(`0x5bac05`, `0x5bac16`); failure sets state 2 and calls slot 10 with 5. `0x5bac58` `cmp al,0x10;
jb` keeps results below 16.

## SRP6 client

With `N`, `g`, salt `s` and `B` as little-endian numbers: `x = H(s ‖ H(U:P))`, `v = gˣ`,
`a = r + bitlen(N)` for 32 random bytes `r`, `A = gᵃ`, `u = H(A ‖ B)`,
`S = (B + 3(N − v))^(a + u·x) mod N`, `K = Interleave(S)`, and
`M1 = H((H(N) ⊕ H(g)) ‖ H(U) ‖ s ‖ A ‖ B ‖ K)`; all powers are mod `N`. `A`, `M1` and `K` are
stored; a running hash keeps `A ‖ M1 ‖ K` for the server's proof. It refuses `N`, `g` or the
salt empty or longer than 32 bytes, `B` shorter than 31, `N` under 256 bits, `B ≥ N` and `B = 0`.

Addresses: `0x5d3650`, `0x5d3590`, `0x5d3874`, `0x5d3895`, `0x5d38c2`, `0x5d3970`, `0x5d3981`, `0x5d39cc`, `0x5d3a0b`
Confidence: inferred
Proof: none
Test: `net::srp6_client_session`, `net::srp6_credential_hash`

The step order and every hash are read at the bytes; the operation each Storm call performs is
read from its structure, not its arithmetic, which leaves the entry `inferred`. `0x5d3590` sets
`+0x5c = H(U)` and `+0x70 = H(U ‖ ':' ‖ P)` and starts the running hash at `+0x84`. In `0x5d3650`,
`x` hashes the salt then `+0x70`; `0x5d36e3` to `0x5d374f` check the lengths, the salt's at `0x5d372b` to `0x5d373a`; `0x5d3874`
requires a bit length of at least `0x100`; `0x5d3895`
draws 32 bytes from the pool; `0x5d38c2` adds the bit length to `a`; `A` goes to `+0x00` as 32
little-endian bytes, zero-padded. `0x5d3970` and `0x5d3981` compare `B` with `N` and with 0.
`N − v` is added to `B` three times, then reduced mod `N` (`0x5d398e` to `0x5d39be`); `0x5d39cc`
multiplies `x` by `u`, and `a` is added. `0x5d3a0b` interleaves `S` into `+0x20`; `M1` is stored at
`+0x48`. Big numbers convert little-endian: byte `i` is bits `8(i mod 4)` of word `i/4`
(`0x665910`, at `0x665970`). Return 0, −1 or −2.
`srp6_client_session` runs `0x5d3650` whole; `srp6_credential_hash` runs `0x5d3590` whole.

## Session key interleave

`K` is built from `S`'s 32 little-endian bytes. Leading zero bytes are dropped from the start of
the buffer, the low end of the number, and then one more byte if the count left is odd. Of
what remains, the even-indexed bytes are hashed to `G` and the odd-indexed bytes to `H`;
`K[2i] = G[i]` and `K[2i+1] = H[i]` for `i` from 0 to 19.

Addresses: `0x5d3360`, `0x5d3375`, `0x5d337e`, `0x5d33a3`, `0x5d33d0`, `0x5d33f0`, `0x5d3423`
Confidence: verified
Proof: two
Test: `net::srp6_client_session`, `net::srp6_interleave`

`0x5d3375` `cmp byte [ebx],0; jne` with `inc ebx; dec eax` skips zeros from the buffer's first
byte; `0x5d337e` `test al,1` drops one more. `0x5d33a3` gathers bytes `2i`, `0x5d33f0` bytes
`2i + 1`, each hashed with SHA-1 over half the length; `0x5d33d0` and `0x5d3423` store the digests
at even and odd offsets of the 40-byte output. `ret 4`: output in `ecx`, input in `edx`, length
on the stack.
`srp6_client_session` and `srp6_interleave` run `0x5d3360` whole.

## Logon proof

In state 4 the client sends `0x01`, `A` (32), `M1` (20), the checksum hash
`H(A ‖ C)` (20), a count of CD-key records with each record's fields and hash, and a security
byte; a non-zero security byte adds 16 and 20 bytes from the logon object. `C` is the 20-byte
checksum the logon object passes in. A reconnect sends `0x03` instead (Reconnect).

Addresses: `0x5bbad0`, `0x5bbb38`, `0x5bbb4e`, `0x5bbb62`, `0x5bbb76`, `0x5bbbae`, `0x5bbbce`, `0x5bbc80`, `0x5b1170`, `0x5bba80`
Confidence: verified
Proof: two
Test: `net::hmac_sha1`, `net::srp6_file_hmac`

`0x5bbb38` `cmp [edi+0x80],4`. `0x5bbb4e` puts 1; `0x5bbb62` 32 bytes from `+0x84`, the SRP
object's `A`; `0x5bbb76` 20 from `+0xcc`, its `M1`; `0x5bbb7b` to `0x5bbbae` hash `A` and the 20 bytes
at the argument. `0x5bbbce` puts the count from the owner's slot 8; each record puts a 16-bit
value, a dword, 4 bytes and the 20-byte hash from `0x5bba80`. `0x5bbc80` puts the byte from the
owner's slot 7, and a non-zero one adds 16 and 20 bytes. The argument comes from `0x5b1f40`,
which the login screen calls with the checksum of `0x5b1170` over the salt that slot 5
(`0x5b1c70`) kept at `+0x19c`; on a reconnect, slot 5 calls `0x5bbad0` with 20 zero bytes
(`0x5b1c98`).
`hmac_sha1` runs `0x5b1170` whole; `srp6_file_hmac` runs `0x5bba80` whole.

## Proof reply

A proof reply is a result byte. Result 0 carries the server's proof `M2` (20) and a dword the
link keeps. When `M2` equals `H(A ‖ M1 ‖ K)`, the link enters state 6 and hands `K` to the logon
object; otherwise it reports result `0x0b`. Result `0x0a` ends the reply with no report; others
are reported, 16 and above as `0xff`.

Addresses: `0x5bacb0`, `0x5bad40`, `0x5bada2`, `0x5d3ad0`
Confidence: verified
Proof: one
Test: none

`0x5d3ad0` returns −2 unless the length is 20, finalises the running hash at `+0x84` and compares
20 bytes, returning 0 when equal. `0x5bad40` calls it; on 0 the link stores state 6 and calls the
owner's slot 10 with 0, `+0xa4` (the SRP object's `K`) and 40; on mismatch it stores state 2 and
calls slot 10 with `0x0b`. `0x5bada2` returns 2 for result `0x0a` before any call.

## Logon results

Outside a reconnect, the logon object's slot 10 (`0x5b2c90`) keeps the key, up to 40 bytes, at
`+0x54` for result 0 or `0x0e`, and moves to `LOGIN_STATE_AUTHENTICATED` (4), or to state 12 when the link kept a
non-zero dword from the proof. Other results become `LOGIN_STATE_FAILED` (5) with a code:

| result | code | name |
|---|---|---|
| 3 | 13 | `LOGIN_BANNED` |
| 4, 5 | 7, 8 or 9 | `LOGIN_UNKNOWN_ACCOUNT`, `…_PIN`, `…_CALL` |
| 6 | 15 | `LOGIN_ALREADYONLINE` |
| 7 | 16 | `LOGIN_NOTIME` |
| 8 | 17 | `LOGIN_DBBUSY` |
| 9 | 14 | `LOGIN_BADVERSION` |
| `0x0c`, `0x0f`, `0xff` | 18, 19, 20 | unnamed in the log table |
| other | 11 | `LOGIN_FAILED` |

Result `0x0a` moves to `LOGIN_STATE_DOWNLOADFILE` (6).

Addresses: `0x5b2c90`, `0x5b2cf9`, `0x5b2d00`, `0x5b2e16`, `0x5b2e38`, `0x5b2e78`, `0x5b2ea4`, `0x5b2920`
Confidence: verified
Proof: one
Test: none

`0x5b2cf9` indexes the byte table `0x5b2ea4` with the result less 3 (253 entries; a larger index
goes to the default) into the 11-entry jump table `0x5b2e78`; it is slot 10 of the logon object's
vtable `0x80a5d4`. Results 4 and 5 share `0x5b2d00`:
code 7 when `+0x1ac` is zero; otherwise `+0x1ad` counts the failures and the code is 8 for the
first two and 9 after (`cmp al,3; sbb; add 9`). `0x5b2e16` handles 0 and `0x0e`: the copy to
`+0x54` at `0x5b2e38` caps the length at `0x28`. The names are the strings `0x5b2920` logs through
its tables `0x5b2c00` (states) and `0x5b2c40` (18 codes).

## Random pool

The pool is a SHA-1 generator of 64 bytes: a 20-byte seed hash, a 20-byte copy of it, a
20-byte output block and a count of unused output bytes. Seeding with `s` hashes the 12 bytes
`time(0)`, `s` and the C runtime's `rand()` after `srand(s)`. Drawing takes the unused tail of
the block; when it runs out, the block becomes `H(seed ‖ block ‖ copy)`. The pool at `0xc2a428`
is seeded at startup with the client clock and is the one SRP6 draws `a` from.

Addresses: `0x5d1c70`, `0x5d1cf0`, `0x5b8000`, `0x5b7ff0`, `0xc2a428`
Confidence: verified
Proof: one
Test: `net::sha1_prng_next`, `net::sha1_prng_seed`, `net::srp6_client_session`

`0x5d1c70`: `_time(&t)` into `[ebp-0xc]`, `_srand(s)`, `_rand()` into `[ebp-0x4]`, SHA-1 over the
12 bytes, the digest to `+0x00` and copied to `+0x14`, `+0x28` to `+0x3c` zeroed, `ret 4`.
`0x5d1cf0` copies from `+0x3c − count`; the refill hashes `+0x00`, `+0x28` and `+0x14`, 20 bytes
each, into `+0x28` and sets the count to 20. `0x5b8000`, a static initializer reached through
`0x5b7ff0`, calls `0x5d1c70` on `0xc2a428` with the clock; the only other reference to `0xc2a428`
is the push at `0x5babae` before the SRP6 call.
`sha1_prng_next` and `srp6_client_session` run `0x5d1cf0` whole; `sha1_prng_seed` runs `0x5d1c70`
whole.

## Reconnect

With a kept session key the logon object sends the reconnect challenge instead of a logon: `0x02`, `0x02`, then the logon challenge body. The reply is a result byte, a 16-byte seed and a 16-byte checksum salt; a non-zero result, or a reply that does not hold the two values, is malformed and the link disconnects. The link keeps the seed, enters state 5 and gives the salt to the logon object, which answers with the proof below. The reply to the proof is a result byte: non-zero, as `0xff` from 16 up, enters state 2 and reports it; zero enters state 6 and hands the key back.

Addresses: `0x5b2134`, `0x5bb6d0`, `0x5bae00`, `0x5baec0`, `0x5bae94`, `0x5baf0b`, `0x5baf3c`
Confidence: verified
Proof: one
Test: none

`0x5b2134` tests the logon object's `+0xc` and calls `0x5bb6d0` with the account and the key (`+0xd`, `+0x54`), which enters state 3, upper-cases the account into `+0x19c`, copies the key to `+0x1fc` and sends. `0x5bae00` waits for 17 bytes (`0x5bae0d`) but reads 33 (the result byte and two 16-byte slices), so a reply that arrives split after 17 bytes is taken as malformed (`0x5bae66`) instead of waited for; it reads the result byte (`0x5bae2d`; non-zero returns 1) and two borrowed 16-byte slices (`0x5bae4e`, `0x5bae5b`), returns 1 when the cursor passed the end (`0x5bae66`), copies the first to `+0x1dc`, sets state 5 (`0x5bae94`) and calls the owner's slot 5 with the second (`0x5baea9`). `0x5baec0` stores state 2 and calls slot 11 with the result for a non-zero byte (`0x5baf0b`, `0x5baf15` to `0x5baf19`), else state 6 and slot 11 with `+0x1fc` and 40 (`0x5baf3c`).

## Reconnect proof

The reconnect proof is `0x03`, `R1 = MD5(account ‖ unix time as a dword)` (16 bytes), `R2 = SHA-1(account ‖ R1 ‖ seed ‖ K)` (20, `K` the kept 40-byte key), `R3 = SHA-1(R1 ‖ C)` (20, with `C` 20 zero bytes on a reconnect), the count of key records, and the key records, 30 bytes from each 32-byte source: its u16 at `+0` (`0x5bbe10`), its u32 at `+4` (`0x5bbe23`), its 4 bytes at `+8` (`0x5bbe38`), and the 20-byte hash `0x5bba80` of the record with `R1` (16 bytes) and the seed at `+0x1dc` (16 bytes) (`0x5bbe43` to `0x5bbe6a`). No security byte follows them (`0x5bbe7a` to `0x5bbe8b`).

Addresses: `0x5bbad0`, `0x5bbcb7`, `0x5bbd06`, `0x5bbd24`, `0x5bbd7c`, `0x5bbd92`, `0x5bbdc6`, `0x4294c0`
Confidence: verified
Proof: one
Test: none

In `0x5bbad0`, `0x5bbcb7` hashes the account (`strlen`, `0x661dd0`) and the time (`0x4294c0` is `time(0)`) into R1; `0x5bbd06` puts `0x03` and R1; `0x5bbd24` to `0x5bbd7c` hash the account, R1, `+0x1dc` (16) and `+0x1fc` (40) into R2; `0x5bbd92` to `0x5bbdc6` hash R1 and the argument into R3, which `0x5b1c98` makes 20 zeros.

