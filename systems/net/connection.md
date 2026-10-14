# net: connection

The operations the login and character screens run on the world session, and the packets they send and read.

`ClientServices.cpp` holds the `ClientConnection` methods and the static `ClientServices` functions that reach the one live connection `0xc28128` (`session`). Each screen call begins an operation, which stores an operation number, a status code and an optional completion callback in the session's status block (`session`, Session status), sends a packet when the connection is up, and completes later: a reply parser calls a slot of the `ClientConnection` vtable `0x80a398`, or of the response object `0x80a3f4` at `+0x1adc`, which sets the status. The screens poll that status. This doc gives each operation's numbers, packets and replies; `dispatch` gives the opcode routing and `login` the logon server's side.

Roots: `0x5aab20`, `0x5aaba0`, `0x5aabb0`, `0x5aabe0`, `0x5aac50`, `0x5aae20`, `0x5aaf80`, `0x5ab000`, `0x5ab100`, `0x5ab4b0`, `0x5ab630`, `0x5ab680`, `0x5ab7d0`, `0x5ab840`, `0x5ab930`, `0x5ab9a0`, `0x5abc10`, `0x5abd00`, `0x5abd20`, `0x5abd50`, `0x5abd90`, `0x5abdc0`, `0x5aa2d0`

## Operations

`0x5aa400(op, code, callback)` begins an operation: it stores the callback at `+0x1b20`, the operation at `+0x1b10` and the code at `+0x1b14`, clears the done flag `+0x1b08` and logs "Initiating". The operations are 0 `COP_NONE`, 1 `COP_INIT`, 2 `COP_CONNECT`, 3 `COP_AUTHENTICATE`, 4 `COP_CREATE_ACCOUNT`, 5 `COP_CREATE_CHARACTER`, 6 `COP_GET_CHARACTERS`, 7 `COP_DELETE_CHARACTER`, 8 `COP_LOGIN_CHARACTER`, 9 `COP_GET_REALMS` and 10 `COP_WAIT_QUEUE`. `0x5aaa50(code)` fails the operation: `0x5aa920(0, code)`. The completions of the character operations call `0x5aa920(flag, code)`; three of them pass the reply byte as the code (see below), and the others pass constants.

| begin | operation | code at begin | success code |
|---|---|---|---|
| `0x5aab20` | 2 | 7 `CSTATUS_CONNECTING` | 5 `RESPONSE_CONNECTED` |
| `0x5aaba0` | 3 | `0x0b` `CSTATUS_AUTHENTICATING` | `0x0c` `AUTH_OK` |
| `0x5aabb0` | 6 | `0x2a` `CHAR_LIST_RETRIEVING` | `0x2b` |
| `0x5aac50` | 5 | `0x2d` `CHAR_CREATE_IN_PROGRESS` | `0x2e` |
| `0x5ab100` | 7 | `0x38` `CHAR_DELETE_IN_PROGRESS` | `0x39` |
| `0x5aae20` | 8 | `0x3c` `CHAR_LOGIN_IN_PROGRESS` | `0x3d` |
| `0x5ab840` | 9 | `0x22` `REALM_LIST_IN_PROGRESS` | `0x23` |

Addresses: `0x5aa400`, `0x5aa440`, `0x5aa920`, `0x5aaa50`, `0x5aaba0`, `0x5aab80`, `0x5aac30`, `0x5ab0e0`, `0x5ab300`, `0x5aa854`, `0x85cae8`, `0x5aab90`, `0x5aac43`, `0x5ab0f3`
Confidence: verified
Proof: one
Test: none

`0x5aa400` is `mov [ecx+0x1b20],eax`, `mov [ecx+0x1b10],edx`, `mov [ecx+0x1b14],eax`, `mov [ecx+0x1b08],0`, then `0x5aa440` with 1. The operation names are the strings `0x5aa440` selects through the jump table `0x5aa854`; the code names are `0x85cae8`. `0x5aaba0` ignores its four stack arguments (`ret 0x10`). `0x5aac30` and `0x5ab0e0` respectively compare the reply byte with `0x2e` and `0x39` (`cmp al,0x2e`, `cmp al,0x39`; `sete`) and call `0x5aa920(flag, byte)`; `0x5ab300` calls it with (1, `0x2b`) or (0, `0x2c`). A reply byte outside the code table is stored as it is; `GetStatus` prints its number (`session`). Of the 16 call sites of `0x5aa920`, found by a scan of `call` and `jmp` (the same scan finds the 4 of `0x5ab1a0`), exactly three pass a server byte as the code: `0x5aab90`, `0x5aac43` and `0x5ab0f3`.

## Without a link

`0x5aabb0`, `0x5aac50`, `0x5ab100` and `0x5aae20` begin their operation and then test the connected flag `+0x1b00`. With it zero they fail the operation at once with code 4, `RESPONSE_FAILED_TO_CONNECT`, and send nothing. `0x5aab20` is the exception: it completes its own operation as `RESPONSE_CONNECTED` when the flag is set.

Addresses: `0x5aabb0`, `0x5aac50`, `0x5ab100`, `0x5aae20`, `0x5aab20`, `0x5aaa50`
Confidence: verified
Proof: one
Test: none

Each begin is followed by `cmp [reg+0x1b00],0`; the zero branch is `push 4; call 0x5aaa50` (`0x5aabca`, `0x5aac6e`, `0x5ab11e`, `0x5aae3b`) and a return before any packet is built. In `0x5aab20`, `0x5aab4f` loads `+0x1b00` and `0x5aab59` calls `0x5aa920(1, 5)`.

## Starting the logon

`0x5ab4b0(account, password)` (`ecx`, `edx`) starts the login session. When the rebuild byte `0xc2811c` is set, it destroys the existing logon object and clears the byte. When there is no logon object it allocates one of `0x1f8` bytes, first creating the realm holder `0xc28118` (`0x14` bytes, vtable `0x803808`, count, capacity and pointer zero) if that is absent, and constructs the logon object over the holder. It then stores the account and password in the logon object (`0x5b30b0`), takes the address from `0x5ab680(0)` and connects the logon link to it (`0x5b2fb0`). `0x5ab5d0`, which answers a file transfer from the logon server, sets the rebuild byte.

Addresses: `0x5ab4b0`, `0x5ab4b4`, `0x5ab4f2`, `0x5ab5d0`, `0x5ab5e8`, `0xc2811c`, `0x5b3250`, `0x5b32b0`, `0x5b30b0`, `0x5b2fb0`, `0xc28118`, `0xc28120`
Confidence: verified
Proof: one
Test: none

`0x5ab4b4` loads `0xc2811c`; the destroy path calls `0x5b3250` and frees (`0x646430`) at `0x5ab4d8` to `0x5ab4e6`, stores 0 to `0xc28120` and clears the byte (`0x5ab4f2`). `0x5ab507` allocates `0x1f8` and `0x5ab52d` `0x14`; `0x5ab53f` stores the vtable. `0x5ab573` calls `0x5b30b0` with the account and password, `0x5ab57a` `0x5ab680` with `ecx` 0, `0x5ab586` `0x5b2fb0`. `0x5ab5e8` is the only store of 1 to the byte.

## Connect and realm list

`0x5aab20` clears two eight-dword blocks at `0xb41d18` and `0xb41cf8` and the dword `0xb41df0` (the glue's queue-position history, `glue/flow`), begins operation 2, completes it as `RESPONSE_CONNECTED` when linked, and tail-jumps to `0x5b2220` on the login session. `0x5b2220` tail-jumps to `0x5bc140`, which sends command `0x10` and a zero dword (`0x5ba880`) only when the logon link's state `+0x80` is 6. `0x5ab840` begins operation 9: with the login session's connected byte `+0x05` set it calls `0x5b2220`, else `0x5b2200`, which sets the logon object's `+0x0c` and connects the link to the address its owner returns (`0x5bc3e0`). The realm list parser ends by calling the owner's slot 5, `0x5abb90`, with 0, or 4 when the list was malformed.

Addresses: `0x5aab20`, `0x5b2220`, `0x5bc140`, `0x5ab840`, `0x5b2200`, `0x5abb90`, `0x5ab800`, `0x5b2230`, `0x803808`
Confidence: verified
Proof: one
Test: none

`0x5bc140` opens `cmp [edi+0x80],6; jne` past the send; its packet is a data store of vtable `0x80a7a8` with `0x418070(0x10)` and `0x4182b0(0)` (`0x5bc1a9`, `0x5bc1b6`). `0x5ab840` tests `[eax+5]` at `0x5ab857`. `0x5abb90` maps its argument: 1 to `0x5aa920(0, 0x17)` (`AUTH_SESSION_EXPIRED`); 3, 2 and 4 to `0x5aa920(0, 0x24)` (`REALM_LIST_FAILED`); any other value acts by the operation: 2 calls `0x5ab800` (`session`, Connecting) and 9 completes (1, `0x23`).

## Realm selection

`0x5ab680` returns the text of the CVar `realmList`. On first use, or when `ecx` is non-zero, it registers the CVar with the default `us.logon.worldofwarcraft.com:3724` and the help text "Address of realm list server" and loads `realmlist.wtf`; otherwise it returns the held value. `0x5ab7d0` returns `realmName`, registering it on first use with an empty default and "Last realm connected to". `0x5ab930` returns record `i` of the realm holder `0xc28118` (stride `0x134`), creating the holder first when it is absent, or null when `i` is not below the holder's count. `0x5ab9a0` returns `0xc27fb8`, the selected-realm copy, when the byte `0xc28134` is non-zero, else null.

Addresses: `0x5ab680`, `0x5ab7d0`, `0x5ab930`, `0x5ab9a0`, `0xc2812c`, `0xc28130`, `0xc28134`, `0xc27fb8`
Confidence: verified
Proof: one
Test: none

`0x5ab680` tests the CVar handle `0xc2812c` and `ecx` (`0x5ab685`, `0x5ab689`): a handle with `ecx` zero jumps to the load of `[eax+0x20]`, the value text. Otherwise `0x63db90` registers the CVar with flags 6 and `0x63d820` reads `realmlist.wtf`. `0x5ab7d0` does the same for the handle `0xc28130`, the name `realmName` and the help text "Last realm connected to". `0x5ab930` compares the index with `[ecx+8]` unsigned (`0x5ab97a`) and returns `index * 0x134 + [ecx+0xc]`. `0x5ab9a0` is `neg al; sbb eax,eax; and eax,0xc27fb8` on the byte.

## Character list request

`0x5aabb0` begins operation 6 and sends `CMSG_CHAR_ENUM` (`0x37`, no body) through `0x5b4690` when linked. The reply, `SMSG_CHAR_ENUM`, is read by `0x5b42a0`; on return it calls slot 4 of the response object, `0x5ab300`, with the connection, the record array and a flag, 1 when the packet was read to its end. `0x5aabe0(callback, argument)` calls `callback(record, argument)` with `ecx` the record and `edx` the argument for each of the held records, stride `0x110`, and returns their count.

Addresses: `0x5aabb0`, `0x5b4690`, `0x5aabe0`, `0x5b42a0`, `0x5ab300`, `0x80a3f4`
Confidence: verified
Proof: one
Test: none

`0x5b4690` puts `0x37` through `0x418190` and sends with `0x5379a0`. `0x5b4574` calls `[vtable+0x10]` of the object at `+0x1adc` with the connection, `&conn+0x1ae0` and `esi`; slot 4 of `0x80a3f4` is `0x5ab300`. `0x5aabe0` loops `[ebx+0x1ae4]` times over `[ebx+0x1ae8] + 0x110·i` (`0x5aac00` to `0x5aac15`); the callback is `call [ebp+8]`.

## Character list packet

`SMSG_CHAR_ENUM` (`0x3b`) is a count byte, then that many records. A count above 10 is treated as 0. A packet whose count is above 10, or that is not read to its end, empties the list and reports flag 0. A record is read in this order, each field to the record offset given: guid, 8 bytes `+0x00`; name, a C string up to 48 bytes `+0x08`; nine bytes `+0x100` to `+0x108` (race, class, sex, skin, face, hair style, hair colour, facial hair, level); u32 `+0x3c`; u32 `+0x38`; three floats `+0x44`, `+0x48`, `+0x4c`; u32 `+0x40`; u32 flags `+0xfc`; byte `+0x109`; u32 `+0xf0`, `+0xf4`, `+0xf8`; then 20 pairs of u32 `+0x50 + 4k` and byte `+0xa0 + 4k`.

Addresses: `0x5b42a0`, `0x5b42d5`, `0x5b4385`, `0x5b44b2`, `0x5b44e8`, `0x5b4840`, `0x5b47b0`, `0x5b47f0`
Confidence: verified
Proof: one
Test: none

The reads are the getters `0x4190b0` (guid), `0x4191b0` (string, cap `0x30`), `0x418cb0` (byte), `0x418eb0` and `0x419130` (dword, float) in the order above (`0x5b4398` to `0x5b44ab`). `0x5b42d5` `cmp [ebp-1],0xa; jbe` zeroes the count and sets a flag that forces the flag argument to 0. The record is `0x110` bytes (`0x5b4840`, `0x5b4385` `imul esi,esi,0x110`). `0x5b44e8` tests the byte at `+0x108`, the ninth of the first run: when it is zero it becomes 1 and `+0x10a` is set to 1; otherwise `+0x10a` is 0. `0x5b4523` to `0x5b4563` free and zero the array when the end check fails. Roles of the fields are in `glue/select`.

## Character create and delete

`0x5aac50(record)` begins operation 5 and sends `CMSG_CHAR_CREATE` (`0x36`): the name, a C string from the record, then the nine bytes `record+0x30` to `record+0x38`, one byte each. `0x5ab100(guid)` begins operation 7 and sends `CMSG_CHAR_DELETE` (`0x38`) with the 8-byte guid. Each reply is one byte read by `0x5b4590` (`0x3a`) and `0x5b45c0` (`0x3c`) and passed to the connection's vtable slot 17 (`0x5aac30`) and slot 18 (`0x5ab0e0`), which complete the operation (Operations).

Addresses: `0x5aac50`, `0x5ab100`, `0x5b4590`, `0x5b45c0`, `0x5aac30`, `0x5ab0e0`
Confidence: verified
Proof: one
Test: none

`0x5aac9f` puts `0x36` with `0x418190`, `0x5aacab` puts the name with `0x418430` and `0x5aacb9` to `0x5aad29` nine `0x418070` bytes from `[esi+0x30]` to `[esi+0x38]`. `0x5ab12f` puts `0x38` and `0x5ab15e` the guid with `0x418370`. `0x5b45aa` and `0x5b45da` are `call [edx+0x44]` and `call [edx+0x48]` after `0x418cb0` reads the byte; the vtable `0x80a398` holds `0x5aac30` and `0x5ab0e0` there.

## Character login

`0x5aae20(guid)` begins operation 8 and sends `CMSG_PLAYER_LOGIN` (`0x3d`) with the guid through `0x5b4700`, then stores 1 at `+0x1b04`; without a link it fails with code 4. The reply that refuses is `SMSG_CHARACTER_LOGIN_FAILED` (`0x41`): `0x5b45f0` reads one byte and calls slot 19, `0x5aad70`. That function clears `+0x1b04`, leaves the world through `0x401ee0(1, 1, 1)` when it is the current connection, and completes the operation as failed with a code chosen by the byte: 1 `CHAR_LOGIN_NO_WORLD` (`0x3e`), 2 `CHAR_LOGIN_DUPLICATE_CHARACTER` (`0x3f`), 3 `CHAR_LOGIN_NO_INSTANCES` (`0x40`), 4 `CHAR_LOGIN_DISABLED` (`0x42`), 5 `CHAR_LOGIN_NO_CHARACTER` (`0x43`), 6 `CHAR_LOGIN_LOCKED_FOR_TRANSFER` (`0x44`), and `CHAR_LOGIN_FAILED` (`0x41`) for 0 and every byte from 7 up. The connection stays open.

Addresses: `0x5aae20`, `0x5b4700`, `0x5b45f0`, `0x5aad70`, `0x5aaf40`, `0x5aae70`, `0x401ee0`, `0x5aae08`, `0x5aad97`, `0x5aad9c`, `0x80a3e4`
Confidence: verified
Proof: one
Test: none

`0x5aad97` `cmp eax,5; ja 0x5aadf7` on the byte less 1, then `jmp [eax*4+0x5aae08]`; the six legs push `0x3e`, `0x3f`, `0x40`, `0x42`, `0x43`, `0x44` and call `0x5aaa50`; `0x5aadf7` pushes `0x41`. `0x5aad70` calls `0x5aaf40` first, which stores 0 at `+0x1b04` and, when `+0x1b18` is set, calls `0x5aae70(0)`. The vtable slot `0x80a3e4` holds `0x5aad70`; the image has no other reference to it and no call. `0x5aad8d` is `call 0x401ee0` with `ecx = edx = 1` and 1 on the stack. `0x5b45f4` reads the byte over the low byte of the packet-store argument slot, so a body-less packet hands `0x5aad70` the low byte of that pointer. Operation 8 stays the current operation: `0x5aa920` writes the code, the failed result and the done flag, and leaves `+0x1b10` at 8.

## Logout

`0x5ab000(quit, instant)` sends `CMSG_PLAYER_LOGOUT` (`0x4a`) when `instant` is set and `CMSG_LOGOUT_REQUEST` (`0x4b`) otherwise, only while a world handle is held at `+0x1b18` and the connection is the current one; it stores `quit` at `+0x1b1c`, and for a non-instant request sets the pending byte `+0x1b1d`. It returns without sending when a logout is already pending and `instant` is clear. `0x5aaff0` is `0x5ab000(0, 1)`. `0x5aaf80` sends `CMSG_LOGOUT_CANCEL` (`0x4e`, no body) and clears the pending byte. `0x5ab0d0` returns the pending byte.

Addresses: `0x5ab000`, `0x5aaff0`, `0x5aaf80`, `0x5ab0d0`, `0x5afd30`, `0x5ab049`, `0x5ab07c`
Confidence: verified
Proof: one
Test: none

`0x5ab00d` loads `+0x1b1d`; with it set and `bl` (instant) zero `0x5ab01a` returns. `0x5ab020` compares `+0x1b18` with 0 and returns when equal. `0x5ab049` calls `0x5afd30` (the account-data flush, `accountdata`) before the packet. `0x5ab062` `neg bl; sbb ebx,ebx; add ebx,0x4b` gives `0x4a` for a non-zero `bl` and `0x4b` for zero. A connection that is not the current one only runs `0x5aaf40`.

## Logout replies

`SMSG_LOGOUT_RESPONSE` (`0x4c`) is a u32 result and a byte, the instant flag. Result 0 with the flag clear fires `PLAYER_CAMPING` when the quit byte `+0x1b1c` is zero and `PLAYER_QUITING` when it is set; with the flag set it fires nothing; a non-zero result raises game error `0x180` (`ERR_LOGOUT_FAILED`) and clears the pending byte. `SMSG_LOGOUT_COMPLETE` (`0x4d`, empty) runs `0x5aaeb0`: it leaves the world (`0x5aaf40`), tears the game down through `0x401ee0(1, 1, 0)` when it is the current connection, clears the pending byte, and quits the client through `0x401560(0)` when the quit byte was set. `SMSG_LOGOUT_CANCEL_ACK` (`0x4f`, empty) fires `LOGOUT_CANCEL` and clears the pending byte, only while one is pending.

Addresses: `0x5b4630`, `0x5b4620`, `0x5b4680`, `0x5aaef0`, `0x5aaeb0`, `0x5aaf60`, `0x401560`, `0x5aaf11`, `0x5aaf21`
Confidence: verified
Proof: one
Test: none

The slots are `0x80a398` +0x50 (`0x5aaeb0`), +0x54 (`0x5aaef0`), +0x58 (`0x5aaf60`), called at `0x5b4622`, `0x5b4669` and `0x5b4682`. `0x5b4630` reads a dword with `0x418e30` and a byte with `0x418cb0`. In `0x5aaef0`, `0x5aaefb` tests the result; zero reaches `0x5aaf02`, which tests the flag, then `0x5aaf11` adds `0x114` to `(+0x1b1c != 0)`: event 276 `PLAYER_CAMPING` or 277 `PLAYER_QUITING` through `0x703e50`. `0x5aaf21` pushes `0x180` for `0x496720`. `0x5aaf6d` loads `0x116`, event 278 `LOGOUT_CANCEL`. `0x5aaecf` loads `+0x1b1c` after the teardown call and `0x5aaee3` jumps to `0x401560`.

## Launch URL allow list

`0x5abc10(url)` returns 1 for a URL that begins with `http://` in any case, whose remaining characters are all letters, `.`, `-` or `/`, and whose host, the text up to the first `/`, read to at most its 128th character, matches one of ten patterns; otherwise 0. A pattern that starts with `*` matches a host whose text from its first `.` equals the pattern without the star, so exactly one label may precede it. The patterns are `*.blizzard.com`, `*.blizzcon.com`, `www.battle.net`, `*.worldofwarcraft.com`, `www.battlenet.co.kr`, `*.worldofwarcraft.co.kr`, `*.blizzard-europe.com`, `*.wow-europe.com`, `*.wowchina.com` and `*.wowtaiwan.com.tw`. `0x5abd00` opens the URL through `0x429ff0` only when `0x5abc10` accepts it.

Addresses: `0x5abc10`, `0x5abd00`, `0x429ff0`, `0x85cc34`, `0x5abc64`, `0x5abca1`, `0x5abcac`
Confidence: verified
Proof: one
Test: none

The prefix test is `0x64a4c0` (case-insensitive) against `0x85d6bc` for its length (`0x5abc2e`). Each later character must be `/`, `.`, `-` or satisfy `_isalpha` (`0x5abc64`; mask `0x103`), so digits are rejected, in the path as well as the host. The host is copied until a NUL, `/`, or the end of the 128-byte buffer (`0x5abc78` to `0x5abc9c`); the terminator is then stored at `[ecx]`, which is the saved frame pointer's low byte when 128 characters were copied. The table `0x85cc34` holds ten string pointers (`cmp esi,0x28`); `0x5abcac` tests a leading `*`, finds the first `.` with `0x64a3f0` and compares the rest with the pattern from its second byte, both with the length bound `0x7fffffff`, so the comparison covers the whole string.

## Status text and names

`0x5abd20(code)` returns the token string of a status code from 1 to `0x52`, and the empty string otherwise. `0x5abd40` returns 1 when its connection is the current one. `0x5abd50(name)` returns the name validator's result plus `0x45`, a `CHAR_NAME_*` status code. `0x5abd90` copies a `0x110`-byte character record into `0xc27d80`, the selected-character record; `0x5abdc0` returns `0xc27d88`, the name field at `+0x08` of it, or null when it is empty. `0x5abd70` and `0x5abd80` set and return the account name `0xc27d08`, copied in `0x40` bytes when the logon object authenticates (`session`, Session key).

Addresses: `0x5abd20`, `0x5abd40`, `0x5abd50`, `0x5abd90`, `0x5abdc0`, `0x5abd70`, `0x5abd80`, `0x6c9ae0`, `0xc27d80`, `0xc27d88`, `0xc27d08`
Confidence: verified
Proof: one
Test: none

`0x5abd20` is `test ecx,ecx; jle`, `cmp ecx,0x53; jge`, then `mov eax,[ecx*4+0x85cae8]`; the other branch returns `0x882748`, an empty string. `0x5abd50` calls `0x6c9ae0` with the locale `[0xc0e080]` in `ecx` and the name in `edx`, then `add eax,0x45`; the codes from `0x45` (`CHAR_NAME_NO_NAME`) to `0x52` (`CHAR_NAME_SUCCESS`) are the validator's results from 0 up. `0x5abd90` is `rep movs` of `0x44` dwords to `0xc27d80`. `0x5abdc0` is `neg al; sbb eax,eax; and eax,0xc27d88`.

## Response object

The response object `0x80a3f4` is the connection's `+0x1adc` listener. Slot 2 is `0x5ab280`, which handles the world auth result; slot 3 is `0x5ab2f0`, which fires script event 6, `ADDON_LIST_UPDATE` in the glue event table, when `SMSG_ADDON_INFO` has been parsed; slot 4 is `0x5ab300` (Character list request). For `AUTH_WAIT_QUEUE` (`0x1b`), `0x5ab280` begins operation 10 with the same callback and code and passes the queue position `+0x1af0` to `0x46ae50`; for any other result it calls `0x5aab80` with the byte.

Addresses: `0x80a3f4`, `0x5ab280`, `0x5ab2f0`, `0x5ab300`, `0x5b4260`, `0x46ae50`, `0x5ab288`, `0x5ab2a6`
Confidence: verified
Proof: one
Test: none

`0x5ab286` `cmp al,0x1b`; the wait branch pushes `[esi+0x1b20]`, `0x1b` and `0xa` for `0x5aa400` and then `ecx = [esi+0x1af0]` for `0x46ae50`. `0x5ab2f0` is `mov ecx,6; call 0x703e50`. `0x5b4260` calls `0x51da70` on the data store, then slot 3 of the object (`0x5b4290`).

## Telemetry record

At start-up `0x5aa2d0` clears 32 bytes at `0xc27e90`, opens `Data\base.MPQ`, reads the 28-byte file `telemetry.dat` from it, and on success copies the 28 bytes to `0xc27e94`, stores the word `0x14` at `0xc27e90` and sets the byte `0xc28135`. `0x5ab9b0`, slot 1 of the realm holder's vtable `0x803808`, returns 0 while that byte is clear; otherwise it appends the 32-byte record to the logon object's record array (count `+8`, capacity `+4`, pointer `+0xc`, growth step `+0x10`), sets the array's first dword to 1 and returns 1. The logon proof sends the array's count and records (`login`, Logon proof).

Addresses: `0x5aa2d0`, `0x5ab9b0`, `0xc27e90`, `0xc28135`, `0x803808`
Confidence: verified
Proof: one
Test: none

`0x5aa2d6` zeroes 8 dwords at `0xc27e90`; `0x5aa2f2` is the archive open, `0x5aa31f` the file open; `0x5aa32f` `cmp [ebp-4],esi` requires a read length of `0x1c`; `0x5aa334` to `0x5aa350` copy 7 dwords, set `0xc28135` and the word `0x14`. `0x5ab9b4` loads `0xc28135`. `0x5abab4` to `0x5abac7` store the count, the first dword 1 and `rep movs` 8 dwords from `0xc27e90`. The byte and the record have no other writer.

## Sending a packet

`ClientServices::Send` (`0x5ab630`), the one send entry of the rest of the client, takes the packet's data store in `ecx` and sends it on the current connection through `NetClient::Send` (`wire`, Sending); with no current connection it does nothing and the caller still owns the store.

Addresses: `0x5ab630`, `0x5ab490`, `0x5379a0`
Confidence: verified
Proof: one
Test: none

`0x5ab630` calls `0x5ab490` and returns when the result is null (`0x5ab635`, `0x5ab637`); otherwise it pushes the store, calls `0x5ab490` again to load `ecx`, and calls `0x5379a0` (`0x5ab641`).
