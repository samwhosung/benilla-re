# net: session

The world connection's life: connecting, authenticating, keeping alive and disconnecting.

One object is the client's world session: a `ClientConnection` (`ClientServices.cpp`), built at
startup and kept at `0xc28128`, which the static `ClientServices` functions address. It is a
`RealmConnection`, which is a `NetClient`; the three vtables are `0x80a398`, `0x80a658` and
`0x809070`. The class names come from the image: its log line `ClientConnection %s: %s code=%s
%s` and the tag `.?AUREALMCONNECTIONNODE@RealmConnection@@`. `NetClient` keeps the connection state at
`+0x70`, the packet tables (`dispatch`), the cipher key at `+0x48` and the ping record;
`RealmConnection` adds the handlers for the session's own opcodes; `ClientConnection` adds the
status the login and character screens poll. The logon object (`login`) hands the session its
account and key once the logon server accepts the player.

Roots: `0x5ab1d0`, `0x5ab490`, `0x5ab4a0`, `0x5ab7b0`, `0x5aa960`, `0x5ab1a0`, `0x537f20`

## Session status

The session reports progress as a status: an operation, a code, a result flag and a done
flag. Setting a status stores the code and the result and marks it done; the screens poll it
and get the code's text from the localized string whose key is the code's name in an 83-entry
table, from `RESPONSE_SUCCESS` (0) through `CHAR_NAME_SUCCESS` (`0x52`). The codes used here are
`RESPONSE_FAILED_TO_CONNECT` (4), `RESPONSE_CONNECTED` (5), `AUTH_OK` (`0x0c`),
`AUTH_WAIT_QUEUE` (`0x1b`) and `REALM_LIST_REALM_NOT_FOUND` (`0x26`).

Addresses: `0x5aa920`, `0x5aa960`, `0x5aa9a2`, `0x5aa9e9`, `0x5aab80`, `0x85cae8`
Confidence: verified
Proof: one
Test: none

`0x5aa920(result, code)` stores `code` at `+0x1b14`, `result` at `+0x1b0c` and 1 at `+0x1b08`, then
calls `0x5aa440` with them and the operation at `+0x1b10`, `ret 8`. `0x5aa960` returns the four
fields; for a code in `[0, 0x53)` it passes `0x85cae8[code]` to the localization lookup
(`0x5aa9a2`), else prints the number; for `0x1b` it formats the text with the queue position at
`+0x1af0` (`0x5aa9e9`). `0x5aab80`, the session's report of an auth result, passes
`result == 0x0c` as the result flag.

## Connecting

Selecting a realm stores its name and looks it up in the realm list; connecting dials the
realm's `host:port` text, port 9090 when the text has none. `NetClient::Connect` requires state
2, disarms the header cipher and dials in state 5. When the socket connects, the session
reports `RESPONSE_CONNECTED` and enters state 6, the only state in which it sends or
dispatches packets; a failed dial reports `RESPONSE_FAILED_TO_CONNECT` and returns to state 2.
A realm name not in the list reports `REALM_LIST_REALM_NOT_FOUND`.

Addresses: `0x5ab7b0`, `0x5ab6f0`, `0x5ab800`, `0x5ab82f`, `0x537820`, `0x53785a`, `0x5378c0`, `0x5aaa90`, `0x537ce0`, `0x5aaaf0`
Confidence: verified
Proof: one
Test: none

The session is the object at `0xc28128`. `0x5ab6f0` compares the name case-insensitively with each 0x134-byte record of the list at
`0xc28118` and copies a match to `0xc27fb8`; `0x5ab800` connects with the text at `0xc280c1`, the
record's `+0x109` (`0x5ab82f`). `0x537820` raises a fatal error unless `+0x70 == 2`, copies the
text, splits it at `:` with `_atoi` for the port, `0x2382` otherwise (`0x53785a`), calls
`0x5b73c0(0)` and `0x5378c0`, which sets `+0x70` to 5 and calls `0x5b6e40`, the dial. The connect
event's slot 13 is `0x5aaa90`: `0x5aa920(1, 5)`, `+0x1b00 = 1`, a call outside the system at
`0x5aaaa6`, then `0x537ce0`, which stores 6.
Slot 15, the failed dial, is `0x5aaaf0`: `0x5aa920(0, 4)` and state 2.

## Session key

When the logon object reports `LOGIN_STATE_AUTHENTICATED`, the session copies 108 bytes into
`NetClient` `+0x04`: the account name (64 bytes), a dword from the logon link, and the 40-byte
session key. The dword is the link's `+0x74`, which a successful logon proof sets to 0.

Addresses: `0x5abb10`, `0x5abb30`, `0x5abb71`, `0x537a30`, `0x5bad50`
Confidence: verified
Proof: one
Test: none

`0x5abb10` is slot 4 of the login listener `0x803808` that `0x5ab6f0` allocates. `0x5abb30`
`cmp esi,4; jne` skips other states; it copies the logon object's account from `+0xd`, the
link's `+0x74` (`[+0x1ec]`) and 10 dwords from the logon object's `+0x54` (`0x5abb71`), then calls
`0x537a30`, `rep movs` of `0x1b` dwords to `+0x04`. In the proof reply, `0x5bad50` stores the
verification's result, 0, at the link's `+0x74`; the only other writer is `ServerLink` code.

## Session challenge

`SMSG_AUTH_CHALLENGE` (`0x1ec`) carries the server's 32-bit seed. The client answers with
`CMSG_AUTH_SESSION` (`0x1ed`): build `0x16f3`, the dword from the session key record, the
account as a C string, a client seed, a 20-byte digest and the add-on block. The seed is the
first output of the client's random generator seeded with the clock. The digest is
`SHA1(account ‖ 0u32 ‖ client seed ‖ server seed ‖ K)`, the seeds as little-endian dwords and
`K` the 40-byte key. This packet arms the header cipher (`wire`).

Addresses: `0x5b4000`, `0x5b402b`, `0x5b403f`, `0x5b404b`, `0x5b4087`, `0x5b40b4`, `0x5b40ce`, `0x5b4130`, `0x5b4146`, `0x5b4154`
Confidence: verified
Proof: two
Test: none

`0x5b402b` clears `+0x1aec`. The seed state is seeded with 0 and then `0x42c010()` (`0x5b403f`,
`0x44ea80`); `0x5b404b` reads the server seed. The puts are `0x1ed`, `0x16f3`, `+0x44`
(`0x5b4087` to `0x5b40a0`), the C string at `+0x04` and `0x4531e0()` (`0x5b40b4`). SHA-1 at
`0x5b40ce` to `0x5b4130` hashes `SStrLen(+0x04)` bytes of the account, 4 bytes of a zeroed local,
the client seed, the server seed and 40 bytes at `+0x48`; the 20 bytes are put at `0x5b413e`.
`0x5b4146` calls `0x51d910`, the interface's add-on writer, and `0x5b4154` sends.

## Session response

`SMSG_AUTH_RESPONSE` (`0x1ee`) starts with a result byte. `AUTH_OK` (`0x0c`) and
`AUTH_WAIT_QUEUE` (`0x1b`) may carry 9 more bytes when at least 5 remain: a dword, a byte and a
dword, the last only when all 9 are there. `AUTH_WAIT_QUEUE` then carries the queue position, a dword. The client reports the
result through the session, which sets the status with it.

Addresses: `0x5b41b0`, `0x5b41e2`, `0x5b41e6`, `0x5b41f9`, `0x5b422b`, `0x5b424a`, `0x5ab2b8`
Confidence: verified
Proof: two
Test: none

`0x5b41e2` `cmp al,0xc`; `AUTH_OK` stores 1 at `+0x1aec`. `0x5b41f9` `cmp edx,5; jb` on the
store's size less its cursor skips the three reads into `+0x1af4`, `+0x1afc` and `+0x1af8`.
`0x5b422b` `cmp al,0x1b` reads the position into `+0x1af0`. `0x5b424a` calls the notify object's
slot 2 with the result; for the session that reaches `0x5aab80` (`0x5ab2b8`). `+0x1aec` has three
writers and no reader: a scan for `+0x1aec]` finds the stores at `0x5b3e22`, `0x5b402b` and
`0x5b41e6` only.

## Ping and latency

Every 30 seconds after the last ping, measured from connecting, the client sends `CMSG_PING`
(`0x1dc`): a sequence number, incremented first, and in state 6 the sample just before the
ring's write index, or 0 when that index is 0: before the first pong and after every 16th, when
the index wraps. `SMSG_PONG` (`0x1dd`) with the current number stores
`now − sent` in a ring of 16 on a network worker thread (`threads`); a pong with another number is logged and
ignored. The latency the interface reads is the integer mean of the samples present, at
most 15, or 0; the rates are kilobytes per second since connecting.

Addresses: `0x537ff0`, `0x537e10`, `0x537e80`, `0x537e85`, `0x537e87`, `0x537d60`, `0x537dd6`, `0x537de8`, `0x537b90`, `0x537bcf`, `0x537f20`, `0x537fce`, `0x537fd2`
Confidence: verified
Proof: two
Test: `net::netclient_idle`, `net::netclient_ping`, `net::netclient_pong`, `net::netclient_stats`

`0x537ff0` adds `−30000` to `now − [+0x1a64]` and pings unless the sign is set. `0x537e10` stores
the clock at `+0x1a64`, puts `0x1dc` and `++[+0x1a68]`; in state 6, `0x537e80` `cmp eax,ebx` and
`0x537e85` `jbe` on the write index `+0x1ab0` put 0 for 0, which `0x537dd6` stores after the 16th
sample, else `0x537e87` puts `[+0x1a68 + 4w]`, the sample before it. A
send outside state 6 is dropped by `NetClient::Send`. `0x537d60` compares the dword with `+0x1a68`,
stores at `+0x1a6c + 4w`, advances `w` mod 16, and `0x537de8` advances the read index `+0x1aac`
when they meet, so the ring holds 15. `0x537b90`, the connect hook, zeroes both indices and the
byte counts and stamps `+0x1a64` (`0x537bcf`). `0x537f20` divides the sum by the count with `div`
(`0x537fce`) or returns 0 (`0x537fd2`); each rate is bytes times `1/1024` over milliseconds times
`0.001`. The tests run the four functions whole: the idle check at the 30-second threshold and
the sign boundary, the ping at every write index in and out of state 6, the pong at every pair of
indices, and the statistics over every pair of indices and elapsed times of 0, 1 and the wrap.

## Disconnecting

`NetClient::Disconnect` in state 6 enters state 7 and closes the socket; a network worker thread then
reports the disconnect, and the session sets state 2. In any other state it tears the connection
down at once and builds a fresh one in state 2. On the disconnect event the session marks the
status done, clears its connected flag and calls the login screen's disconnect handler.

Addresses: `0x5ab1a0`, `0x537910`, `0x53791f`, `0x53792c`, `0x537975`, `0x5aaac0`, `0x537d00`, `0x537d20`
Confidence: verified
Proof: one
Test: none

`0x5ab1a0` calls `0x537910` on the session. `0x53791f` stores 7 and tail-jumps to `0x5b5370`, which
sets the socket's state to 7 when it is 5. Otherwise (`0x53792c`): `0x5b71e0(0, 0)`, `0x5b5370`, the
queue flush `0x538700`, `0x5b5200` and a new 0x130-byte `WowConnection` (`0x537975`), state 2. Slot 14
is `0x5aaac0`: `0x5aaa70`, `+0x1b08 = 1`, `+0x1b00 = 0`, `+0x1b1d = 0`, a call outside the system, then `0x537d00`,
which stores 2 and calls `0x46c540` (`0x537d20`).

## Ping sequence and statistics

`CMSG_PING` sequence numbers are not reset when the session connects, and the idle check pings before there is a connection, so the first ping a server receives can carry a number above 1: a ping sent in any other state than 6 still counts and is dropped by `NetClient::Send`. `GetNetStats` returns, through three out parameters, the bytes received and the bytes sent, each times 1/1024 over the seconds since connecting, and the latency. The seconds are the clock less the time recorded when the connection came up, times 0.001; with none elapsed the quotient follows the x87 rules. Pings, pongs, the connect hook and `GetNetStats` all run under the connection's statistics lock `+0x1ac0`.

Addresses: `0x537b90`, `0x537ff0`, `0x537e10`, `0x537d60`, `0x537f20`, `0x537b34`, `0x5379c6`, `0x537bcf`, `0x537e74`, `0x537fb5`
Confidence: verified
Proof: one
Test: `net::netclient_ping`, `net::netclient_stats`

`0x537b90` stores the node's time at `+0x1abc`, zeroes `+0x1ab8`, `+0x1ab4`, `+0x1aac` and `+0x1ab0`, and stamps `+0x1a64`; it never writes `+0x1a68`. `0x537ff0` runs for every polled client and pings once `now − [+0x1a64] ≥ 30000`, where `+0x1a64` is 0 until the first ping or connect. `0x537e10` increments `+0x1a68` before its state test at `0x537e74`. `0x537f20` takes `+0x1ac0` (`0x537f33`), scales `now − [+0x1abc]` by `0x801360` and the two byte counts (`+0x1ab8` received, `+0x1ab4` sent) by `0x8090d8`, divides, and stores the received rate through the first argument, the sent rate through the second and the mean through the third (`0x537f80`, `0x537f96`, `0x537fd7`). The seconds are a 64-bit
integer load of the elapsed dword, so a clock before the connect time reads as nearly 4.3 million
seconds; with none elapsed a rate is `+∞`, or NaN with no bytes. The mean's loop wraps its index
at 16 and stops there when the write index is 0 (`0x537fb5`), and its sum is a dword that
wraps. `0x537d60` and `0x537e10` take the same lock (`0x537d70`, `0x537e3f`).
