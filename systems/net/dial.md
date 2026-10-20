# net: dial

How the client turns an address text into a dial: the logon server's realmlist text into a list of socket addresses and one dial among them, and a realm's `host:port` text into a world dial.

The logon link hands its host text to a resolver thread of `OsTcp.cpp` (`os`) and dials one address of the list the thread builds. The world connection splits its text into a host and a port itself and resolves the host in the dial. The logon text is the value of the CVar `realmList` (`connection`, Realm selection); the world text is the address field of the selected realm's record (`logonlink`, Realm record fill). The client imports `inet_addr`, `gethostbyname` and `htons` from WSOCK32 by ordinal (10, 52 and 9), so what they make of a text is the platform's. The models in `crates/net/src/dial.rs` take them as parameters, and the tests replace them.

Roots: `0x5bc3e0`, `0x537820`

## Logon dial

`ClientLink::Connect` (`0x5bc3e0`) acts only when the link's state `+0x80` is 0. It sets state 1 and asks the OsTcp object (`0x885400`) to resolve its host text with port 3724, `ConnectToRandomAddress` as the callback and the link as the callback's argument. `0x43f5c0` returns 0 and starts nothing while the object is null. `0x43c230` returns 0 for a null host or callback, with error `0x57`, or when the thread cannot start. Otherwise it copies the text, gives the request an id that is not 0 and not another live request's, starts the thread `0x43b890`, named `OsTcp_DNS`, waits until the thread has read its parameters and returns 1. The thread's list goes to the callback (`threads`, Random address pick).

Addresses: `0x5bc3e0`, `0x5bc3ee`, `0x5bc3f2`, `0x5bc400`, `0x5bc405`, `0x43f5c0`, `0x43f5c3`, `0x43f5d9`, `0x43c230`, `0x43c23a`, `0x43c257`, `0x43c310`, `0x43c374`, `0x43c468`, `0x43b890`, `0x43b8b6`, `0x885400`, `0x5bb430`
Confidence: verified
Proof: two
Test: none

`0x5bc3ee` skips the dial for any state but 0, `0x5bc3f2` calls `ClientLink::SetState` with 1, and `0x5bc400` loads `0xe8c` into `edx` after the link and `0x5bc2b0` are pushed. `0x43f5c3` loads `0x885400` and `0x43f5e3` returns 0 for null. `0x43f5d9` calls `0x43c230` with the object, the host, the port, the callback and the argument. `0x43c23a` and `0x43c257` test the host and the callback. `0x43c310` increments the id at `+0x61c` and repeats while it is 0 or a request in the list at `+0x628` has it. `0x43c374` calls `0x64bd20` with `0x43b890`, a parameter block (object, id, event, port) on the stack and the name `OsTcp_DNS`; `0x43b8b6` is the thread's `SetEvent` on the event that `0x43c468` waits on.

## Resolver list split

The resolver thread takes the request's text one piece at a time: the text up to the first `;`, copied at most 1,023 bytes into a 1,024-byte buffer, the `;` overwritten with a NUL and the rest kept for the next piece. The list ends at the first empty piece: `a;;b` ends after `a`, and a trailing `;` ends the list. The end marks the request done and wakes the OsTcp thread. A piece whose first byte is `0` to `9` is an address piece; any other first byte, a space or a byte of `0x80` or more included, makes a name piece. Neither is trimmed. Both kinds add to one list in the order the pieces come.

Addresses: `0x43b890`, `0x43b8ed`, `0x43b8f6`, `0x43b901`, `0x43b915`, `0x43b91a`, `0x43b941`, `0x43b947`, `0x43b94f`, `0x43ba07`, `0x43bb1e`, `0x43bb47`, `0x43bb5e`, `0x64a3f0`, `0x64a5a0`, `0x43c200`
Confidence: verified
Proof: two
Test: `net::dial_resolve_thread`

`0x43b8c9` clears the buffer's first byte each round. `0x43b8f4` sets `dl` to `;` and `0x43b8f6` calls `SStrChr2` on `+0xc` of the request (`0x43b8ed`); a hit is zeroed and stepped over (`0x43b901`, `0x43b904`); `0x43b915` is `SStrCopy(buffer, text, 0x400)`, and `0x43b91a` stores the pointer past the `;`, or null, at `+0xc`. A null `+0xc` skips the copy. `0x43b941` `je 0x43bb1e` on the buffer's first byte ends the list: `0x43bb47` stores 1 at `+0x20` and `0x43bb5e` sets the event at the object's `+0x544`. `0x43b947` `cmp al,0x30; jl` and `0x43b94f` `cmp al,0x39; jg` are signed, so `0x80` to `0xff` branch to `0x43ba07`, the name piece. The test runs the thread whole with the platform calls replaced.

## Address piece

A piece that begins with a digit goes to `0x43f3d0` with the request's port. With a `:` in it, the text before the first `:` (at most 31 bytes) goes to `inet_addr`, and `SStrToInt` of the text after it, taken to 16 bits, is the port. Without one, the whole piece goes to `inet_addr` and the port is the request's. The result is 16 bytes: family 2, the port in network order, the address `inet_addr` returned and eight zero bytes. A text `inet_addr` rejects gives `0xffffffff`, which is stored and added like any address. The piece is never looked up by name, so a host name that begins with a digit, such as `3com.example`, is stored with `inet_addr`'s failure value.

Addresses: `0x43f3d0`, `0x43b957`, `0x43b964`, `0x43b9ba`, `0x43b9e2`, `0x43f3db`, `0x43f3df`, `0x43f3ea`, `0x43f3fd`, `0x43f407`, `0x43f412`, `0x43f417`, `0x43f422`, `0x43f428`, `0x64a3c0`, `0x64a5a0`, `0x64ac60`
Confidence: verified
Proof: two
Test: `net::dial_digit_piece`, `net::dial_resolve_thread`

`0x43b957` loads the request's port word into `edx` and `0x43b964` calls `0x43f3d0` with the buffer in `ecx` and the output at `[ebp-0x18]`. `0x43f3df` is `SStrChr` for `:`; on a hit `0x43f3ea` to `0x43f3f7` take the distance plus 1, capped at `0x20`, as the size of the `SStrCopy` into a 32-byte local (`0x43f3fd`), which leaves at most 31 bytes and a NUL, and `0x43f407` replaces the port with `SStrToInt` of the text after the colon. `0x43f412` stores family 2, `0x43f417` calls `htons` (ordinal 9) on the low word, `0x43f422` calls `inet_addr` (ordinal 10) and `0x43f428` stores the result at `+4` with no test; the last eight bytes are zeroed. `0x43b9ba` to `0x43b9e2` append the 16 bytes whatever they hold.

## Name piece

A piece that begins with any other byte goes whole to `gethostbyname`, a `:` and the digits after it included. A name not found adds nothing, and the thread takes the next piece. A name found with an address type other than 2 or a length other than 4 adds nothing. Otherwise each address in the hostent's list, in order, adds an entry of family 2, that address and the request's port, 3724 for the logon link. A port in the realmlist text therefore reaches the dial only through a piece that begins with a digit; a name keeps the port 3724, and the stock default `us.logon.worldofwarcraft.com:3724` is one such name.

Addresses: `0x43ba07`, `0x43ba0e`, `0x43ba18`, `0x43ba1e`, `0x43ba29`, `0x43ba60`, `0x43ba80`, `0x43ba86`, `0x43ba89`, `0x43baec`, `0x43baef`, `0x43f370`, `0x5bc400`
Confidence: verified
Proof: two
Test: `net::dial_resolve_thread`

`0x43ba0e` is `call [0x7ff6d4]`, WSOCK32 ordinal 52, with the buffer. A null result returns to the loop (`0x43ba18`). `0x43ba1e` `cmp word [ebx+8],2` and `0x43ba29` `cmp word [ebx+0xa],4` test `h_addrtype` and `h_length` and return to the loop on a miss. `0x43ba60` loads `h_addr_list` (`+0xc`); `0x43ba80` reads the first dword of each address, `0x43ba86` loads the request's port into `edx` and `0x43ba89` calls `0x43f370`, which stores family 2, `htons` of the low word, the address and eight zero bytes. `0x43bac4` to `0x43baec` append the 16 bytes, and `0x43baef` repeats while the next pointer is not null. The port is `0xe8c` from `0x5bc400`. The ordinals match their other uses: `0x43f4f0` hands ordinal 11's result to a format string, and `0x43f550` fills a 256-byte buffer with ordinal 57 and looks the name up with ordinal 52.

## Logon host text

The text `ClientLink::Connect` resolves is the value of the CVar `realmList`, whose default is `us.logon.worldofwarcraft.com:3724`. `ClientServices::Logon` passes it to `ConnectToLogonServer`, which does nothing while the logon object is connected. Otherwise it clears the reconnect byte `+0xc`, sets `+8` to -1, clears the security byte `+0x1ac`, reports `LOGIN_STATE_CONNECTING` through the listener, and dials the text, or the default when the text is null. The realm list's re-dial `0x5b2200` sets `+0xc` and dials the address its owner returns.

Addresses: `0x5ab4b0`, `0x5ab57a`, `0x5ab586`, `0x5ab680`, `0x5ab6bf`, `0x5b2fb0`, `0x5b2fbe`, `0x5b300b`, `0x5b3016`, `0x5b3022`, `0x5b2200`, `0x5b2219`
Confidence: verified
Proof: one
Test: none

`0x5ab57a` calls `0x5ab680` with `ecx` 0 and `0x5ab586` passes its result to `0x5b2fb0`. `0x5ab6bf` loads the CVar's value text, `[handle+0x20]`. `0x5b2fbe` `jne` returns when `[esi+5]` is set. `0x5b300b` is the listener's slot 4 with state 1 and the names `LOGIN_STATE_CONNECTING` and `LOGIN_OK`. `0x5b3016` substitutes the default for a null text, and `0x5b3022` calls `0x5bc3e0`. `0x5b2200` stores 1 at `+0xc` and calls slot 3 of its owner before `0x5b2219`.

## Resolver hand-off

The thread adds each entry under the OsTcp object's lock, an event at `+0x618` used as a mutex, to the list at request `+0x24`: capacity, count `+0x28`, data `+0x2c` and growth step. A request that is not in the object's list ends the thread without the flag. A hostent whose first address pointer is null also ends it before the flag, and the request stays in the list. The OsTcp thread `0x43ae20`, named `OsTcp_Base`, takes each request with the flag set off the list and calls `0x43b810`, which calls the request's callback with the data pointer, the count and the argument, on that thread. It then frees the host copy, the thread handle and the list.

Addresses: `0x43b8e9`, `0x43ba5a`, `0x43ba6a`, `0x43bafe`, `0x43bb4a`, `0x43ae20`, `0x43b5bf`, `0x43b5f5`, `0x43b61e`, `0x43b625`, `0x43b810`, `0x43b790`, `0x439800`, `0x439859`, `0x43985e`, `0x64bd40`, `0x43c200`
Confidence: verified
Proof: one
Test: none

`0x43b8e9` and `0x43ba5a` return when the request is not found. `0x43ba6a` `je 0x43bafe` leaves on a null first pointer with `esi` still 0, so the flag store at `0x43bb47` is skipped; `0x43bb4a` unlocks and `0x43bb5e` wakes the OsTcp thread only after the flag. `0x43b5bf` tests `+0x20` of each request in the list at the object's `+0x628` and `0x43b5f5` decrements the count at `+0x614`. `0x43b61e` calls `0x43b810`, which pushes `+0x1c`, `+0x28` and `+0x2c` and calls `[request+0x18]`; `0x43b625` calls `0x43b790`. `0x439859` pushes `0x43ae20` for the thread `0x439800` makes with the name `OsTcp_Base`; `0x64bd40` calls `CreateThread`.

## World address

`NetClient::Connect` (`0x537820`) acts only in state 2; in any other it passes the state to `SErrDisplayAppFatal` ("Expected (m_netState == NS_INITIALIZED), got %d") and dials nothing. It copies the address text, the selected realm's 32-byte address field, into a 1,024-byte buffer and cuts the copy at the first `:`. The port is `_atoi` of the text after it, a full dword; an address with no `:` has port 9090. `host:` has port 0. The cipher goes off (`wire`, Arming the cipher) and `0x5378c0` enters state 5 and dials the host with that port and a timeout of -1.

Addresses: `0x537820`, `0x53782f`, `0x53785a`, `0x53785f`, `0x53786c`, `0x53787a`, `0x537893`, `0x5378a2`, `0x5378a9`, `0x5378c0`, `0x5378e8`, `0x5378f6`, `0x41449c`, `0x64a3f0`, `0x5ab800`, `0x5ab823`, `0x5ab82f`, `0x64d0e0`
Confidence: verified
Proof: two
Test: `net::dial_world_connect`

`0x53782f` compares `+0x70` with 2. `0x53785a` loads `0x2382` into `edi` before `0x53785f` calls `SStrCopy(buffer, address, 0x400)`. `0x537864` sets `dl` to `:` and `0x53786c` calls `SStrChr2`; a hit is zeroed and the text after it goes to `_atoi` (`0x53787a`), whose result replaces `edi`. `0x537893` calls `0x5b73c0` with 0 and `0x5378a2` restores state 2, which `0x5378c0` requires. `0x5378e8` stores state 5 and `0x5378e5` to `0x5378f6` push -1, the dword and the host for `0x5b6e40`. `0x5ab823` passes `0xc280c1`, the selected realm's `+0x109`. The test runs the function whole with the dial and the cipher call recorded.

## Host dial

`WowConnection::ConnectToHost` (`0x5b6e40`, host, port, timeout) stores an address at `+0x64` and the port's low 16 bits at `+0x68`, calls the connect and returns 1. The address is `inet_addr` of the host text unless that is `0xffffffff` or 0; then it is the first address in the list `gethostbyname` returns, read as a dword with its first byte lowest, and 0 when the lookup fails, so a failed lookup connects to 0.0.0.0. The hostent's type and length are not read. The timeout is not read.

Addresses: `0x5b6e40`, `0x5b6e4b`, `0x5b6e51`, `0x5b6e56`, `0x5b6e5a`, `0x5b6e60`, `0x5b6e6a`, `0x5b6e86`, `0x5b6e8d`, `0x5b6e94`, `0x5b6e9e`, `0x5b6c80`
Confidence: verified
Proof: two
Test: `net::dial_connect_host`

`0x5b6e4b` is `call [0x7ff718]`, WSOCK32 ordinal 10; `cmp eax,-1` (`0x5b6e51`) and `test eax,eax` (`0x5b6e56`) both go to the lookup at `0x5b6e5f`, and any other value is stored (`0x5b6e5a`). `0x5b6e60` calls ordinal 52; a null result stores 0 (`0x5b6e8d`). `0x5b6e6a` loads `h_addr_list`, `0x5b6e6d` its first pointer, and `0x5b6e6f` to `0x5b6e86` build the dword from its four bytes with no test of the pointer. `0x5b6e94` `mov cx,[ebp+0xc]` stores the port word, and `0x5b6e9e` calls `0x5b6c80`, which builds the `sockaddr` with `htons` of that word (`wire`, Connecting). The test runs the function whole with the system calls and the connect replaced.

## Integer from text

`_atol` (`0x414411`) skips white space, which in the C locale is a tab to a carriage return and a space; reads one `-` or `+`; and then reads decimal digits, wrapping modulo 2^32, until the first byte that is not one. A `-` negates last. A text with no digit gives 0. `_atoi` (`0x41449c`) is a call of it. The client never changes the locale, so the other form, `__isctype` for a locale whose characters take more than one byte, does not run.

Addresses: `0x414411`, `0x41449c`, `0x414419`, `0x414431`, `0x41443d`, `0x414484`, `0x41448e`, `0x831710`, `0x83171c`, `0x40f8ae`, `0x4109d6`, `0x410c1c`, `0x412454`
Confidence: verified
Proof: two
Test: `net::crt_atol`

`0x414419` `cmp [0x831710],1; jle` takes the table form: `0x41443d` is `and eax,8` on the low byte of the entry `_pctype` (`0x83171c`) holds for the byte, and `0x41447a` `and eax,4` tests the digit bit in the loop. The table in the file has bit 8 on 9 to 13 and 32, bit 4 on `0` to `9`, and no bit on a byte of `0x80` or more. `0x41444b` and `0x414452` compare the first non-space byte with `-` and `+`; `0x414481` and `0x414484` are `ebx = ebx * 10 + byte - 0x30`; `0x41448e` negates for `-`. `0x412454` is the only store to `__mb_cur_max`, in an initializer only `_setlocale` reaches; no call, jump or stored address names `_setlocale`, where the same scan finds three callers of `0x410c1c` (`who`, Who query packet).
