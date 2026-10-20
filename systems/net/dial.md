# net: dial

How the client turns an address text into a dial: the logon server's realmlist text into a list of socket addresses and one dial among them, and a realm's `host:port` text into a world dial.

The logon link hands its host text to a resolver thread of `OsTcp.cpp` (`os`) and dials one address of the list the thread builds. The world connection splits its text into a host and a port itself and resolves the host in the dial. The logon text is the value of the CVar `realmList` (`connection`, Realm selection); the world text is the address field of the selected realm's record (`logonlink`, Realm record fill). The client imports `inet_addr`, `gethostbyname` and `htons` from WSOCK32 by ordinal (10, 52 and 9), so what they make of a text is the platform's. The models in `crates/net/src/dial.rs` take them as parameters, and the tests replace them.

Roots: `0x5bc3e0`, `0x537820`

## Logon dial

`ClientLink::Connect` (`0x5bc3e0`) passes its host text, the port 3724 (`0xe8c`, loaded at `0x5bc400`), `ConnectToRandomAddress` as the callback and the link as the callback's argument to `0x43f5c0`. That calls `0x43c230` on the OsTcp object (`0x885400`), which spawns the resolver thread `0x43b890` to build a list of socket addresses from the text. The thread's list goes to the callback (`threads`, Random address pick).

Addresses: `0x5bc3e0`, `0x5bc3fa`, `0x5bc3fb`, `0x5bc400`, `0x5bc405`, `0x43f5c0`, `0x43f5c3`, `0x43f5d9`, `0x43c230`, `0x43c36f`, `0x43c374`, `0x43b890`, `0x885400`
Confidence: verified
Proof: two
Test: none

`0x5bc3fa` and `0x5bc3fb` push the link and `0x5bc2b0`, `0x5bc400` loads `0xe8c` into `edx`, and `0x5bc405` calls `0x43f5c0` with the host in `ecx`. `0x43f5c3` loads `0x885400` and `0x43f5d9` calls `0x43c230` with the object, the host, the port, the callback and the argument. `0x43c374` calls `0x64bd20` with `0x43b890` as the thread function (`0x43c36f` is the only reference to its address in the image), a parameter block of object, id, event and port, and the name `OsTcp_DNS`. Two readings, by a benilla builder and its auditor, agree on this chain from `0x5bc3e0` to the thread.

## Logon dial start

`ClientLink::Connect` acts only when the link's state `+0x80` is 0, and sets state 1 first. `0x43f5c0` returns 0 and starts nothing while the OsTcp object is null. `0x43c230` returns 0 for a null host or callback, with error `0x57`, or when the thread cannot start. Otherwise it copies the text, gives the request an id that is not 0 and not another live request's, starts the thread, waits until the thread has read its parameters and returns 1. Fault: when the allocation of the 52-byte request returns null, `0x43c230` stores the host copy's pointer through the null (`0x43c2bf`).

Addresses: `0x5bc3ee`, `0x5bc3f2`, `0x5bb430`, `0x43f5e3`, `0x43c23a`, `0x43c257`, `0x43c27b`, `0x43c29d`, `0x43c2bf`, `0x43c310`, `0x43c379`, `0x43c468`, `0x43b8b6`, `0x885400`, `0x438cb4`, `0x438e81`
Confidence: verified
Proof: one
Test: none

`0x5bc3ee` skips the dial for any state but 0 and `0x5bc3f2` calls `ClientLink::SetState` (`0x5bb430`) with 1. `0x43f5e3` returns 0 for a null object. `0x43c23a` and `0x43c257` test the host and the callback. `0x43c310` increments the id at `+0x61c` and repeats while it is 0 or a request in the list at `+0x628` has it. `0x43c379` returns 0 when the thread call returns 0. `0x43b8b6` is the thread's `SetEvent` on the event that `0x43c468` waits on. `0x43c27b` calls `SMemAlloc` for `0x34` bytes; a null result takes `0x43c29d`, which zeroes `esi`, and `0x43c2bf` stores `[esi+8]`. `0x885400` is stored at `0x438cb4` and cleared at `0x438e81`, its only writers.

## Resolver list split

The resolver thread takes the request's text one piece at a time: the text up to the first `;`, which is overwritten with a NUL, the rest kept for the next piece. The list ends at the first empty piece: `a;;b` ends after `a`, and a trailing `;` ends the list. A piece whose first byte is `0` to `9` is an address piece; any other first byte makes a name piece.

Addresses: `0x43b890`, `0x43b8ed`, `0x43b8f6`, `0x43b901`, `0x43b91a`, `0x43b941`, `0x43b947`, `0x43b94f`, `0x43b957`, `0x43ba07`, `0x43bb1e`, `0x64a3f0`
Confidence: verified
Proof: two
Test: `net::dial_resolve_thread`

`0x43b8c9` clears the buffer's first byte each round. `0x43b8f4` sets `dl` to `;` and `0x43b8f6` calls `SStrChr2` on `+0xc` of the request (`0x43b8ed`); a hit is zeroed and stepped over (`0x43b901`, `0x43b904`), and `0x43b91a` stores the pointer past the `;`, or null, at `+0xc`. A null `+0xc` skips the copy. `0x43b941` `je 0x43bb1e` on the buffer's first byte ends the list. `0x43b947` `cmp al,0x30; jl` and `0x43b94f` `cmp al,0x39; jg` send a non-digit to `0x43ba07`, the name piece, and a digit on to `0x43b957`. The test runs the thread whole with the platform calls replaced. Two readings, by a benilla builder and its auditor, agree on the `;` split (`0x43b8f6`), the end at the first empty piece (`0x43b941`) and the branch on `0` to `9` (`0x43b947`, `0x43b94f`).

## Address piece

A piece that begins with a digit goes to `0x43f3d0` with the request's port. With a `:` in it, the text before the first `:` goes to `inet_addr`, and `SStrToInt` of the text after it is the port. Without one, the whole piece goes to `inet_addr` and the port is the request's. The entry holds the address `inet_addr` returned, whatever it is: a text it rejects is stored and added like any address. The piece is never looked up by name, so a host name that begins with a digit, such as `3com.example`, is stored with whatever `inet_addr` returns for it.

Addresses: `0x43f3d0`, `0x43b957`, `0x43b964`, `0x43b9ba`, `0x43b9e2`, `0x43f3df`, `0x43f407`, `0x43f422`, `0x43f428`, `0x64a3c0`, `0x64ac60`
Confidence: verified
Proof: two
Test: `net::dial_digit_piece`, `net::dial_resolve_thread`

`0x43b957` loads the request's port word into `edx` and `0x43b964` calls `0x43f3d0` with the buffer in `ecx` and the output at `[ebp-0x18]`. `0x43f3df` is `SStrChr` for `:`; on a hit `0x43f407` replaces the port with `SStrToInt` of the text after the colon. `0x43f422` calls `inet_addr` (ordinal 10) and `0x43f428` stores the result at `+4` with no test. `0x43b9ba` to `0x43b9e2` append the 16 bytes whatever they hold. Two readings, by a benilla builder and its auditor, agree on `0x43f3d0` with `SStrChr`, `SStrToInt` and `inet_addr`, and on what a failed parse stores.

## Name piece

A piece that begins with any other byte goes whole to `gethostbyname`, a `:` and the digits after it included. Each AF_INET address in the result adds an entry with that address and the request's port, 3724 for the logon link. A port in the realmlist text therefore reaches the dial only through a piece that begins with a digit; a name keeps the port 3724, and the stock default `us.logon.worldofwarcraft.com:3724` is one such name.

Addresses: `0x43ba07`, `0x43ba0e`, `0x43ba1e`, `0x43ba60`, `0x43ba80`, `0x43ba86`, `0x43ba89`, `0x43f370`, `0x5bc400`
Confidence: verified
Proof: two
Test: `net::dial_resolve_thread`

`0x43ba0e` is `call [0x7ff6d4]`, WSOCK32 ordinal 52, with the buffer. `0x43ba1e` `cmp word [ebx+8],2` tests `h_addrtype`. `0x43ba60` loads `h_addr_list` (`+0xc`); `0x43ba80` reads the first dword of each address, `0x43ba86` loads the request's port into `edx` and `0x43ba89` calls `0x43f370`. The port is `0xe8c` from `0x5bc400`. The ordinals match their other uses: `0x43f4f0` hands ordinal 11's result to a format string, and `0x43f550` fills a 256-byte buffer with ordinal 57 and looks the name up with ordinal 52. Two readings, by a benilla builder and its auditor, agree on the whole-name lookup (`0x43ba0e`) and on each AF_INET address taking the request's port (`0x43ba86`, from `0x5bc400`).

## Resolver piece limits

A piece is copied at most 1,023 bytes into a 1,024-byte buffer and is not trimmed. A first byte of `0x80` or more makes a name piece, because the digit compares are signed. In an address piece the text before the `:` is cut to 31 bytes and the port is the low 16 bits of the number. An entry is 16 bytes: family 2, the port in network order, the address and eight zero bytes. A name not found, or found with an address type other than 2 or a length other than 4, adds nothing and the thread takes the next piece. The pieces' entries share one list, in the order the pieces and their addresses come.

Addresses: `0x43b908`, `0x43b915`, `0x43b947`, `0x43b94f`, `0x43f3ea`, `0x43f3fd`, `0x43f407`, `0x43f412`, `0x43f417`, `0x43f370`, `0x43ba18`, `0x43ba1e`, `0x43ba29`, `0x43bac4`, `0x43baec`, `0x43baef`
Confidence: verified
Proof: one
Test: `net::dial_digit_piece`, `net::dial_resolve_thread`

`0x43b908` pushes `0x400` as the size of the `SStrCopy` at `0x43b915`. The compares at `0x43b947` and `0x43b94f` are signed, so a byte of `0x80` to `0xff` is below `0x30`. `0x43f3ea` to `0x43f3f7` take the distance to the colon plus 1, capped at `0x20`, as the size of the `SStrCopy` into a 32-byte local (`0x43f3fd`), which leaves at most 31 bytes and a NUL; `htons` (ordinal 9, `0x43f417`) takes the port's low word. `0x43f412` stores family 2 and the last eight bytes are zeroed; `0x43f370` writes the same layout for a name's addresses. A null result of `gethostbyname` returns to the loop (`0x43ba18`), as does a miss at `0x43ba1e` or `0x43ba29` (`h_addrtype`, `h_length`). `0x43bac4` to `0x43baec` append an entry, and `0x43baef` repeats while the next address pointer is not null.

## Logon host text

The text `ClientLink::Connect` resolves is the value of the CVar `realmList`, whose default is `us.logon.worldofwarcraft.com:3724`. `ClientServices::Logon` passes it to `ConnectToLogonServer`, which does nothing while the logon object is connected. Otherwise it clears the reconnect byte `+0xc`, sets `+8` to -1, clears the security byte `+0x1ac`, reports `LOGIN_STATE_CONNECTING` through the listener, and dials the text, or the default when the text is null. The realm list's re-dial `0x5b2200` sets `+0xc` and dials the address its owner returns.

Addresses: `0x5ab4b0`, `0x5ab57a`, `0x5ab586`, `0x5ab680`, `0x5ab6bf`, `0x5b2fb0`, `0x5b2fbe`, `0x5b300b`, `0x5b3016`, `0x5b3022`, `0x5b2200`, `0x5b2219`
Confidence: verified
Proof: one
Test: none

`0x5ab57a` calls `0x5ab680` with `ecx` 0 and `0x5ab586` passes its result to `0x5b2fb0`. `0x5ab6bf` loads the CVar's value text, `[handle+0x20]`. `0x5b2fbe` `jne` returns when `[esi+5]` is set. `0x5b300b` is the listener's slot 4 with state 1 and the names `LOGIN_STATE_CONNECTING` and `LOGIN_OK`. `0x5b3016` substitutes the default for a null text, and `0x5b3022` calls `0x5bc3e0`. `0x5b2200` stores 1 at `+0xc` and calls slot 3 of its owner before `0x5b2219`.

## Resolver hand-off

The thread adds each entry, under the OsTcp object's lock (an event at `+0x618` used as a mutex), to the list at request `+0x24`: capacity, count `+0x28`, data `+0x2c`, step. At the list's end it stores 1 at `+0x20`, the flag, and sets the event at the object's `+0x544`. A request missing from the object's list, or a hostent whose first address pointer is null, ends the thread before the flag. The OsTcp thread `0x43ae20`, named `OsTcp_Base`, takes requests off the list's head while the head's flag is set and, on that thread, calls each one's callback through `0x43b810` with the data pointer, the count and the argument, then frees its host copy, thread handle and list.

Addresses: `0x43b8e9`, `0x43ba5a`, `0x43ba6a`, `0x43bafe`, `0x43bb47`, `0x43bb4a`, `0x43bb5e`, `0x43ae20`, `0x43b5bf`, `0x43b5e8`, `0x43b5f5`, `0x43b61e`, `0x43b625`, `0x43b810`, `0x43b790`, `0x439800`, `0x439859`, `0x43985e`, `0x64bd40`, `0x43c200`
Confidence: verified
Proof: one
Test: `net::dial_resolve_thread`

`0x43b8e9` and `0x43ba5a` return when the request is not found. `0x43ba6a` `je 0x43bafe` leaves on a null first pointer with `esi` still 0, so the flag store at `0x43bb47` is skipped; `0x43bb4a` unlocks and `0x43bb5e` wakes the OsTcp thread only after the flag. `0x43b5bf` tests `+0x20` of the head of the list at the object's `+0x628`; a set flag reaches `0x43b5e8`, which links the request into a local list after `0x441170` unlinks it, and `0x43b5f5` decrements the count at `+0x614`. `0x43b61e` calls `0x43b810`, which pushes `+0x1c`, `+0x28` and `+0x2c` and calls `[request+0x18]`; `0x43b625` calls `0x43b790`. `0x439859` pushes `0x43ae20` for the thread `0x439800` makes with the name `OsTcp_Base`; `0x64bd40` calls `CreateThread`. The test runs the resolver side whole.

## Resolver scan stall

The OsTcp thread's scan of the request list holds the lock at `+0x618` and tests only the head's flag. An unflagged head sends it straight back to reload the head, so it retests the same request with the lock held. The resolver sets the flag only after it waits on that same lock, so neither thread proceeds. The scan runs when the count at `+0x614` is above 0. A new request goes in at the head, unflagged. A request that a hostent with no address leaves unflagged for good reaches the head once the ones in front are taken. Whether play reaches the scan with an unflagged head is open: what starts the scan is not traced.

Addresses: `0x43b580`, `0x43b588`, `0x43b5aa`, `0x43b5b0`, `0x43b5bf`, `0x43b5c4`, `0x43b5fd`, `0x43ba6a`, `0x43b8d2`, `0x43bb29`, `0x43bb47`, `0x43c43a`, `0x43c452`
Confidence: verified
Proof: one
Test: none

`0x43b580` loads the count at `+0x614` and `0x43b588` skips the scan at 0. `0x43b5aa` waits on `+0x618` with no timeout. `0x43b5b0` loads the head, `[edi+0x628]`; an end marker or null leaves (`0x43b5b9`, `0x43b5bd`). `0x43b5bf` loads the head's `+0x20`, and `0x43b5c4` `je 0x43b5b0` returns to the head load with the lock still held. Only a set flag goes on, and the loop ends at `0x43b5fd`, which releases the lock. The resolver waits on the same lock at `0x43b8d2` and `0x43bb29`, before it stores the flag at `0x43bb47`. `0x43c43a` to `0x43c452` link a new request in at the head of the list.

## World address

`NetClient::Connect` (`0x537820`) cuts the address text at its first `:`. The port is `_atoi`, a call of `_atol`, of the text after it, a full dword; an address with no `:` has port 9090, and `host:` has port 0. `0x5378c0` dials the host with that port through `ConnectToHost`, which keeps the port's low 16 bits.

Addresses: `0x537820`, `0x53785a`, `0x53786c`, `0x53787a`, `0x5378c0`, `0x5378f6`, `0x41449c`, `0x64a3f0`, `0x5b6e40`, `0x5b6e94`
Confidence: verified
Proof: two
Test: `net::dial_world_connect`

`0x53785a` loads `0x2382` into `edi` before the copy. `0x537864` sets `dl` to `:` and `0x53786c` calls `SStrChr2` on the copy; a hit is zeroed and the text after it goes to `_atoi` (`0x53787a`), whose result replaces `edi`. `0x5378e5` to `0x5378f6` push -1, the dword and the host for `0x5b6e40`, where `0x5b6e94` `mov cx,[ebp+0xc]` keeps the low word. The test runs `0x537820` whole with the dial recorded. {READ} the default 9090 (`0x53785a`), the cut at the first `:` by `SStrChr2` (`0x64a3f0`), and the port's low 16 bits (`0x5b6e94`).

## World dial states

`NetClient::Connect` acts only in state 2; in any other it passes the state to `SErrDisplayAppFatal` ("Expected (m_netState == NS_INITIALIZED), got %d") and dials nothing. The text is the selected realm's 32-byte address field, copied at most 1,023 bytes. It stores state 3, turns the header cipher off (`wire`, Arming the cipher), stores state 2 again, and `0x5378c0`, which needs state 2, enters state 5 and dials with a timeout of -1.

Addresses: `0x537820`, `0x53782f`, `0x53785f`, `0x53788c`, `0x537893`, `0x5378a2`, `0x5378a9`, `0x5378c0`, `0x5378e8`, `0x5ab800`, `0x5ab823`, `0x5ab82f`, `0x64d0e0`
Confidence: verified
Proof: one
Test: `net::dial_world_connect`

`0x53782f` compares `+0x70` with 2. `0x53785f` calls `SStrCopy(buffer, address, 0x400)`. `0x53788c` stores state 3, `0x537893` calls `0x5b73c0` with 0 and `0x5378a2` stores state 2 before `0x5378a9` calls `0x5378c0`. `0x5378e8` stores state 5. `0x5ab823` passes `0xc280c1`, the selected realm's `+0x109`. The test runs the function whole with the dial and the cipher call recorded.

## Host dial

`WowConnection::ConnectToHost` (`0x5b6e40`, host, port, timeout) stores the host's address at `+0x64` and the port's low 16 bits at `+0x68` and connects. The address is `inet_addr` of the host text unless that is `0xffffffff` or 0; then it is the first address in the list `gethostbyname` returns, read as a dword with its first byte lowest.

Addresses: `0x5b6e40`, `0x5b6e4b`, `0x5b6e51`, `0x5b6e56`, `0x5b6e5a`, `0x5b6e60`, `0x5b6e6a`, `0x5b6e86`, `0x5b6e94`, `0x5b6e9e`, `0x5b6c80`
Confidence: verified
Proof: two
Test: `net::dial_connect_host`

`0x5b6e4b` is `call [0x7ff718]`, WSOCK32 ordinal 10; `cmp eax,-1` (`0x5b6e51`) and `test eax,eax` (`0x5b6e56`) both go to the lookup at `0x5b6e5f`, and any other value is stored (`0x5b6e5a`). `0x5b6e60` calls ordinal 52. `0x5b6e6a` loads `h_addr_list` and `0x5b6e6f` to `0x5b6e86` build the dword from the first address's four bytes. `0x5b6e94` stores the port word, and `0x5b6e9e` calls `0x5b6c80`, which builds the `sockaddr` with `htons` of that word (`wire`, Connecting). The test runs the function whole with the system calls and the connect replaced. {READ} `0x5b6e40`: `inet_addr`, else the first `gethostbyname` address.

## Host dial edges

A failed lookup stores 0, so the connect goes to 0.0.0.0. The hostent's type and length are not read, and a null first address pointer is read without a test. The timeout argument is not read. The function returns 1.

Addresses: `0x5b6e6d`, `0x5b6e8d`, `0x5b6e6f`, `0x5b6e86`, `0x5b6ea4`
Confidence: verified
Proof: one
Test: `net::dial_connect_host`

`0x5b6e8d` stores 0 for a null `gethostbyname` result. `0x5b6e6d` loads the first pointer of `h_addr_list` and `0x5b6e6f` reads a byte through it with no test. The function reads `[ebp+8]` and `[ebp+0xc]` only, never `[ebp+0x10]`, and `0x5b6ea4` sets the result 1. The test runs the function whole with a hostent of type 23 and length 16.

## Integer from text

`_atol` (`0x414411`) skips white space, which in the C locale is a tab to a carriage return and a space; reads one `-` or `+`; and then reads decimal digits, wrapping modulo 2^32, until the first byte that is not one. A `-` negates last. A text with no digit gives 0. The client never changes the locale, so the other form, `__isctype` for a locale whose characters take more than one byte, does not run.

Addresses: `0x414411`, `0x41449c`, `0x414419`, `0x414431`, `0x41443d`, `0x414484`, `0x41448e`, `0x831710`, `0x83171c`, `0x40f8ae`, `0x4109d6`, `0x410c1c`, `0x412454`
Confidence: verified
Proof: two
Test: `net::crt_atol`

`0x414419` `cmp [0x831710],1; jle` takes the table form: `0x41443d` is `and eax,8` on the low byte of the entry `_pctype` (`0x83171c`) holds for the byte, and `0x41447a` `and eax,4` tests the digit bit in the loop. The table in the file has bit 8 on 9 to 13 and 32, bit 4 on `0` to `9`, and no bit on a byte of `0x80` or more. `0x41444b` and `0x414452` compare the first non-space byte with `-` and `+`; `0x414481` and `0x414484` are `ebx = ebx * 10 + byte - 0x30`; `0x41448e` negates for `-`. `0x412454` is the only store to `__mb_cur_max`, in an initializer only `_setlocale` reaches; no call, jump or stored address names `_setlocale`, where the same scan finds three callers of `0x410c1c` (`who`, Who query packet). `_atoi` (`0x41449c`) pushes its argument and calls `_atol`. Two readings, by a benilla builder and its auditor, agree on `_atol`'s C-locale spaces (`ctype & 8`), one sign, wrapping digits and the stop at a non-digit.
