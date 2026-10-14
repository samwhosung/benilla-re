# net: the wire

How a world packet is framed, ciphered, sent and received.

A world connection is one `WowConnection` (0x130 bytes, built by `0x5b4f10`), owned by the
`NetClient` at its `+0x1a58`. Every outgoing packet starts at `NetClient::Send` (`0x5379a0`),
which most of the client reaches through the one-line `ClientServices::Send` (`0x5ab630`);
it hands the packet's bytes to `WowConnection::Send` (`0x5b5630`), which frames them, ciphers
the header and calls the socket on the caller's thread. Incoming bytes are read on a network
thread by `0x5b65f0`, which splits the stream into frames, deciphers each header and passes
the frame to the `NetClient`'s data hook. The opcode dispatch that follows is in
`dispatch`; the logon server's protocol, which has no size header, is in `login`.

The packet layouts are the same in both directions except for the opcode: a size header, then
the opcode, then the body. The client writes a four-byte opcode and reads a two-byte one.

Roots: `0x5379a0`, `0x5ab630`

## Size header

A world packet starts with its size `N`, the count of opcode and body bytes, big-endian. For
`N` up to 32,767 the header is two bytes, `N >> 8` and `N & 0xff`. Above that it is three bytes,
`(N >> 16) | 0x80`, `N >> 8` and `N & 0xff`. A reader picks the form from the top bit of the
first byte. The client sends the opcode as four bytes little-endian and reads it as two bytes
little-endian. It applies the same size rule in both directions.

Addresses: `0x5b5630`, `0x5b5450`, `0x5b65f0`, `0x5b56da`, `0x5b5712`, `0x5b674d`, `0x5b6765`, `0x5b677b`, `0x537aa0`, `0x537b10`, `0x5b5495`
Confidence: verified
Proof: two
Test: `net::header_decode`, `net::header_encode`

`0x5b56da` `cmp eax,0x7fff; jg` picks the form for the length of the data store's bytes. The
three-byte form shifts by `0x10` and ORs `0x80` (`0x5b5712`, `0x5b5718`); the two-byte form
shifts by 8 (`0x5b56f9`). `0x5b5450`, which builds a heap node for packets of 100,000 bytes and
more (`0x5b56a3` `cmp esi,0x186a0`) and for the delayed path (`0x5b59a3`), repeats the rule at
`0x5b5495`.

On receive, `0x5b674d` `test al,al; jns` takes the two-byte form when the top bit is clear:
`((b0 & 0x7f) << 8 | b1) + 2` (`0x5b677b`). Otherwise `((b0 & 0x7f) << 16 | b1 << 8 | b2) + 3`
(`0x5b6765` to `0x5b6772`), the frame's whole length.

Every opcode the client builds goes in through the data store's 32-bit put, as in
`0x5b407f`/`0x5b4087` (`0x1ed`) and `0x537e49` (`0x1dc`). The receive side reads 16 bits:
`0x537abe` in the dispatcher and `0x537b46` in the worker thread's data hook.
`header_decode` runs `0x5b6730` to `0x5b6789`; `header_encode` runs `0x5b5495` to `0x5b54f0`.

## Header cipher

Once armed, the connection ciphers the first `h + 4` bytes of every packet it sends and the
first `h + 2` bytes of every packet it receives, `h` being that packet's size-header length:
the size header and the opcode. Bodies stay in clear. With a 40-byte key `K`, an index `j`
and a feedback byte `f`, one state per direction:

- send: `c = (K[j] ^ p) + f`, then `f = c`;
- receive: `p = K[j] ^ (c − f)`, then `f = c`;
- after each byte, `j = (j + 1) mod 40`. Arithmetic is modulo 256.

The index and feedback carry over from packet to packet. Arming zeroes both states.

Addresses: `0x5b5630`, `0x5b5842`, `0x5b59d2`, `0x5b65f0`, `0x5b6848`, `0x5b73c0`
Confidence: verified
Proof: two
Test: `net::cipher_recv_direct`, `net::cipher_recv_roundtrip`, `net::cipher_send`

The state is a run of bytes in `WowConnection`: `+0x120` armed, `+0x121`/`+0x122` the send
index and feedback, `+0x123` the send span, `+0x124`/`+0x125` the receive index and
feedback, `+0x126` the receive span, `+0x127` the key length, `+0x128` the key pointer.

Send (`0x5b5842` to `0x5b5894`): the count is `span − N + total`, capped at the total, `total`
being header plus data, so `h + span`. Each byte is `xor` with `K[j]`, then `add` of `+0x122`;
the result is stored and becomes `+0x122`; `j` is incremented and reduced by `idiv` by `+0x127`.
`0x5b59d2` repeats the loop for the delayed path.

Receive (`0x5b6848` to `0x5b68a6`): bytes from the old fill count up to `h + span`, only among
those just received: `sub cl,[+0x125]` then `xor` with `K[j]`; the original byte becomes `+0x125`.
Because `0x5b65f0` reads a frame's header before its body (Receive loop), every header byte and
both opcode bytes pass through this loop exactly once.

`0x5b73c0` takes the connection's lock, zeroes `+0x121`, `+0x122`, `+0x124` and `+0x125`, and stores
its argument in `+0x120`.
`cipher_recv_direct` runs `0x5b6848` to `0x5b68a8`; `cipher_recv_roundtrip` and `cipher_send` run
`0x5b5842` to `0x5b5896`.

## Arming the cipher

The cipher is armed by the first packet the client sends on a connection: after
`NetClient::Send` passes a packet to the socket, if the connection is not armed, it sets the
key to the `NetClient`'s 40 bytes at `+0x48`, the send span to 4, the receive span to 2 and the
key length to 40, then arms it. That packet and every one received before it are in clear.
Connecting disarms the cipher. In the normal exchange the first packet is
`CMSG_AUTH_SESSION`, sent in reply to the server's challenge (`session`), so the session key
from the logon server is the key. With no challenge within 30 seconds of connecting, the
keepalive sends `CMSG_PING` first, in clear, and arms the cipher.

Addresses: `0x5379a0`, `0x5379e0`, `0x537a20`, `0x537a50`, `0x537820`, `0x537893`, `0x538597`, `0x537ff0`
Confidence: verified
Proof: two
Test: none

`0x5379e0` tests `+0x120` after the send at `0x5379c1`; the arming block stores `0x537a50`'s
result (`lea eax,[ecx+0x48]`) at `+0x128`, `0x28` at `+0x127`, 4 at `+0x123` and 2 at `+0x126`,
and calls `0x5b73c0(1)` at `0x537a20`. `NetClient::Connect` (`0x537820`) calls `0x5b73c0(0)` at
`0x537893` before it dials. The key's writers are in `session`, Session key. Every poll of the
queue calls the keepalive `0x537ff0` (`0x538597`, `dispatch`), whose clock starts at connecting;
its ping goes through `NetClient::Send` in state 6 like any packet.

## Sending

`NetClient::Send` sends only in connection state 6 and only a non-empty packet, and counts its
bytes before arming the cipher. `WowConnection::Send` refuses a packet unless the socket is connected
(state 5). With nothing queued it frames the packet on the stack and calls `send` once; what
`send` does not take, or all of it when other packets wait, goes to a queue the network
thread drains. Both `WSAEWOULDBLOCK` and any other error queue the packet; the other error, or a
queue reaching its limit, also sets the connection's state to 7, as closing it does. The send
returns 0 when all went out, 1 when part did and 2 when none did.

Addresses: `0x5379a0`, `0x5379a6`, `0x5379c6`, `0x5b5630`, `0x5b5675`, `0x5b58a4`, `0x5b58e3`, `0x5b58ee`, `0x5b5957`, `0xc0d45c`
Confidence: verified
Proof: one
Test: none

`0x5379a6` `cmp [esi+0x70],0x6; jne` returns; the size is the data store's `+0x10` less its
`+0x14`, and zero returns. `0x5379c6` adds it to `0xc0d45c` and to `+0x1ab4`. In
`0x5b5630`, `0x5b5675` requires `+0x10 == 5`; the stack path needs `+0xd0` empty and the latency
global `0xc2a3b8` zero (`0x5b567f`). `send` is `0x5b58a4`; a short count copies the remainder to a
heap node (`0x5b58c3`). The result starts at 2 (`0x5b565b`); a positive count sets 1 (`0x5b58d4`)
and a full send 0 (`0x5b598e`). `0x5b58e3` compares `WSAGetLastError` with `0x2733` and `0x5b58e8`
jumps to the queueing at `0x5b58fa`; otherwise `0x5b72d0(7)` at `0x5b58ee`, then the same queueing; `0x5b72d0` stores its argument in `+0x10` and wakes the network
thread, and the close `0x5b5370` passes 7 to it when the state is 5. Each queued packet increments `+0xd4` and adds to `+0xd8`; `+0xd4`
reaching `+0xdc` calls `0x5b72d0(7)` at `0x5b5957`. Each queued packet ends in `0x5bcab0`, which sends one byte on the socket held at `0x85e9e4`
(`0x5bcad0`); the network thread's `select` loop `0x5bcb00` watches the pair's other end.
`0x5b5450` checks its allocation at `0x5b546d` but its failure branch lands on `0x5b5539`, a store
to `[ebx+0x1c]` with `ebx` 0.

## Receive loop

On a network worker thread, `0x5b65f0` keeps one buffer per connection, starting at 1,024 bytes.
It reads only the bytes still missing from the current frame: the header's bytes until the
header is complete, then the rest of the frame. A complete frame is handed to the connection's
handler as a data store over the opcode and body, with the current clock, and the loop reads
again while the last read returned data. A buffer that must grow past 32,000,000 bytes closes
the connection; a read of 0 bytes or an error other than `WSAEWOULDBLOCK` closes the socket and
sets the connection's state to 6.

Addresses: `0x5b65f0`, `0x5b6613`, `0x5b67c7`, `0x5b67b9`, `0x5b6800`, `0x5b68f3`, `0x5b699e`, `0x5b69e1`, `0x5b6a32`
Confidence: verified
Proof: one
Test: none

The buffer is `+0x1c`, its fill `+0x20` and its capacity `+0x24`, allocated at `0x5b6613`. With
the frame length unknown, the read size is the header length less the fill (`0x5b67ce`);
otherwise the frame's remainder, capped by the free space (`0x5b67d6` to `0x5b67e4`). A full
buffer grows by `0x400` (`0x5b67a6`) and `0x5b67b9` `cmp eax,0x1e84800; ja` closes through
`0x5b5370`. `recv` is `0x5b6800`; `WSAEINTR` (`0x2714`) retries. A complete frame builds a
data store at `+0x1c + h` of `N` bytes (`0x5b68c5` to `0x5b68ec`) and, the latency global being
zero, calls the handler's slot 1 (`0x5b699e`) outside the lock, then clears the fill.
`0x5b69e1` loops while the read count is positive. The disconnect path calls `0x5b72d0(6)` at
`0x5b6a32` and the handler's slot 4.

## Simulated latency

The connection can delay packets in both directions by a time drawn between two globals,
`0xc2a3b8` and `0xc2a3bc`. Nothing in the image writes either; both lie in the zero-filled
part of `.data`, so the delay paths never run.

Addresses: `0xc2a3b8`, `0xc2a3bc`, `0x5b553c`, `0x5b567f`, `0x5b68f8`
Confidence: verified
Proof: one
Test: none

The delay is `min − rand() · (max − min + 1) · (−1/32768)` milliseconds after the current clock
(`0x5b554f` to `0x5b5587`, the factor a double at `0x80a700`). A scan of the image for the two
addresses as little-endian dwords finds six and two occurrences, all inside the instructions
that read them (`0x5b553c`, `0x5b5545`, `0x5b555a`, `0x5b567f`, `0x5b5b50`, `0x5b63e0`,
`0x5b68f8`); none is a store. The control is the same scan for `0xc0d45c`, which finds the
instruction at `0x5379c6` that adds to it.

## Disabling Nagle

When a connection is added to the network thread's select list, `WowConnectionNet::PlatformAdd` sets `TCP_NODELAY` on its socket, then wakes the select loop so it watches the new socket. The packets the client sends are therefore not held back to fill a segment.

Addresses: `0x5bca60`, `0x5bd6e0`, `0x5bd77a`, `0x5bca7f`, `0x5bca87`, `0x5bcad0`
Confidence: verified
Proof: one
Test: none

`0x5bca78` stores 1 in a local; `0x5bca7f` is `call [0x7ff6f8]` with the socket (`[arg+4]`), level 6, option 1, that local and length 4. The import is WSOCK32 ordinal 21, `setsockopt`. `0x5bca87` calls `0x5bcad0`, the one-byte send that wakes `select` (`Sending`). `0x5bd6e0` calls `0x5bca60` at `0x5bd77a` after it appends the connection to the list.

## Connecting

`WowConnection::Connect` closes any old socket, creates one, sets it non-blocking, lists the connection for the select loop, enters state 2 and calls `connect`. A result below zero that is not `WSAEWOULDBLOCK` removes the connection from the list, closes the socket and sets state 8, and nothing is reported to the owner. Otherwise the socket's write or exception event runs `CompleteConnect`, which reads `SO_ERROR`: a non-zero error closes the socket, sets state 6 and calls the owner's slot 3, the connect failure; zero sets state 5, stores the peer and local addresses (`+0x78`, `+0x88`) and calls the owner's slot 2, connected. No timer ends a dial: the 5,000 ms the logon link passes is never read.

Addresses: `0x5b6c80`, `0x5b6d42`, `0x5b6d4c`, `0x5b6d75`, `0x5b60a0`, `0x5b60c6`, `0x5b6137`, `0x5b6140`, `0x5b61ac`, `0x5b6e20`, `0x5bc293`
Confidence: verified
Proof: one
Test: none

`0x5b6cdc` pushes `FIONBIO` (`0x8004667e`) for `ioctlsocket` and `0x5b6cfb` the port for `htons`; `0x5b6d1d` calls `0x5bd6e0` unless the connection is listed (`+0xc4`); `0x5b6d2d` sets state 2. `0x5b6d3c` is `connect`; `0x5b6d4c` compares `WSAGetLastError` with `0x2733` and `0x5b6d75` stores state 8. `0x5b60c6` is `getsockopt` with level `0xffff` and option `0x1007`; a non-zero value reaches `0x5b6137`, the call of slot 3 (`[ebx+0xc]`), after state 6 (`0x5b60f4`). Zero stores state 5 (`0x5b613c`), calls WSOCK32 ordinal 5, `getpeername`, into `+0x78` and ordinal 6, `getsockname`, into `+0x88` (`0x5b6174`, `0x5b6190`), and calls slot 2 (`0x5b61ac`). `0x5b6e20` stores the address at `+0x64` and the port at `+0x68`, calls `0x5b6c80` and returns 1; its third, `0x1388` pushed at `0x5bc293`, is not read.
