# net: dispatch

How a received world packet reaches the function that handles its opcode.

The `NetClient` (`NetClient.cpp`) holds two tables of 828 entries, a handler and a context per
opcode. Code anywhere in the client fills them through `NetClient::SetMessageHandler`
(`0x537a60`), mostly by way of `ClientServices::SetMessageHandler` (`0x5ab650`), which addresses
the one live client; `tables/opcodes.tsv` lists all 387 registrations. Packets arrive on
a network worker thread (`threads`), which copies each into a node of the `NETEVENTQUEUE` (`NetInternal.cpp`)
instead of handling it. The main thread drains the queue from an event handler and the
dispatcher `0x537aa0` calls each packet's handler. The realm connection registers one handler,
`RealmConnection::MessageHandler` (`0x5b3ea0`), for the ten opcodes of logon to the world
server and the character screens.

Roots: `0x537a60`, `0x537a80`, `0x5ab650`, `0x5ab670`, `0x537aa0`, `0x5b3db0`, `0x537b10`, `0x537c50`

## Handler tables

Each `NetClient` holds a handler table at `+0x74` and a context table at `+0xd64`, 828 dwords
each, so opcodes `0x000` to `0x33b`. `NetClient::Initialize` zeroes both. Registering stores the
handler and the context at the opcode's index; clearing stores zeros. Neither checks the
opcode. `ClientServices::SetMessageHandler` and `ClearMessageHandler` do the same on the client
`0x5ab490` returns, taking the opcode in `ecx`, the handler in `edx` and the context on the
stack.

Addresses: `0x537600`, `0x537665`, `0x537a60`, `0x537a80`, `0x5ab650`, `0x5ab670`, `0x5ab490`
Confidence: verified
Proof: one
Test: none

`0x537665` to `0x53768a` are two `rep stos` of `0x33c` dwords from `+0x74` and from `+0xd64`.
`0x537a60` is `mov [ecx+eax*4+0x74],edx` and `mov [ecx+eax*4+0xd64],edx` from its three
stack arguments, `ret 0xc`; `0x537a80` stores zero at both, `ret 4`. `0x5ab650` pushes its stack
argument, `edx` and `ecx` and calls `0x537a60` on `0x5ab490()`, which returns the pointer held at `0xc28128`.

## Dispatch

`0x537aa0` reads the packet's 16-bit opcode, calls the client's slot 16 with it, and calls the
handler registered for it with the context in `ecx`, the opcode in `edx`, and the packet's time
and data store on the stack. A handler returns with `ret 8`. With no handler, it calls the data
store's slot 6 instead, and the packet is dropped. The opcode is not range-checked: one above
`0x33b` reads past the tables. A packet with no opcode bytes leaves the 16-bit read unwritten,
so the opcode is the high half of the store pointer it is read over. Each call counts one in
`0xc0d46c`. Slot 16 is `0x537b00`,
which returns, in all three vtables that hold it.

Addresses: `0x537aa0`, `0x537abe`, `0x537acc`, `0x537acf`, `0x537ae5`, `0x537af2`, `0x537b00`, `0xc0d46c`
Confidence: verified
Proof: one
Test: none

`0x537ab4` increments `0xc0d46c`; `0x537abe` is the data store's 16-bit get; `0x537acc` is
`call [edx+0x40]` on the client's vtable; `0x537acf` loads `[esi+edi*4+0x74]`, and a zero jumps
to `0x537aee`, `call [edx+0x18]` on the data store. Otherwise `ecx` is `[esi+edi*4+0xd64]`,
`edx` the opcode, and the pushes are the store, then the time (`0x537ada`, `0x537adb`). The
only bound on `edi` is the 16-bit read. That read goes to `[ebp+0xe]`, the high half of the
store-pointer argument (`0x537aaf`); a store of 0 bytes, which a frame with `N = 0` gives
(`wire`), fails it and leaves that half in place. The data hook reads the same way into
`[ebp+0x12]` (`0x537b39`). `0x537b00` is `ret 4`, slot 16 of `0x809070`, `0x80a398`
and `0x80a658`.

## Network thread handoff

The connection's handler on a network worker thread (`threads`) is the `NetClient`'s slot 1, `0x537b10`. It adds
the packet's length to the received-byte count and peeks the opcode: `SMSG_PONG` (`0x1dd`) is
handled there and then (`session`, Ping and latency); every other packet is copied into a
queue node of type `0x12`. Connecting, a failed connection and disconnecting queue nodes of
types `0x13`, `0x15` and `0x14`. A node records the clock when it is queued, which is the time
its handler gets.

Addresses: `0x537b10`, `0x537b34`, `0x537b4b`, `0x537b56`, `0x537b7c`, `0x537b90`, `0x537c00`, `0x537c30`, `0x538790`
Confidence: verified
Proof: one
Test: none

`0x537b34` is an interlocked add of the store's size to `+0x1ab8`. `0x537b4b` compares the 16-bit
opcode with `0x1dd` and calls `0x537d60` at `0x537b56`; else `0x538790(0x12, …, this, data,
length)` at `0x537b7c`. `0x537b90` (slot 2), `0x537c30` (slot 3) and `0x537c00` (slot 4) queue
`0x13`, `0x15` and `0x14`. `0x538790` allocates a 24-byte node tagged `.?AUNETEVENTQUEUENODE@@`,
stores the type at `+0x8`, a copy of the data at `+0x10` and its length at `+0x14`, stores
`OsGetAsyncTimeMs()` at `+0xc` (`0x538851`), and increments the client's `+0x1a5c`, all under the
queue's lock. Its second argument is not read.

## Main-thread drain

The handler the client registers for its event 6 calls `0x5b3db0`, which polls the queue of
every registered client. The poll handles the queued nodes in order, under the queue's lock: type `0x12` calls slot 12 with
the time, the data and its length; `0x13`, `0x14` and `0x15` call slots 13, 14 and 15; `0x16`
marks the client for deletion. A node is skipped once the client is so marked. After the
nodes, unless this poll handled a `0x16`, the client runs its keepalive check. Slot 12
dispatches only in state 6.

Addresses: `0x403620`, `0x5b3db0`, `0x538040`, `0x5384d0`, `0x538529`, `0x538597`, `0x537c50`, `0x537c71`, `0x537ca0`, `0xc2a2d8`, `0xc2a314`
Confidence: verified
Proof: one
Test: none

`0x403620` is registered for category 6, which the loop dispatches every tick (`core/loop`), by
the client's start code at `0x402b84` (in `0x402ad0`), the image's only reference to it, so it is never removed:
the poll and its keepalive run in every screen, the character screens included. It calls
`0x5b3db0`,
which walks the list at `0xc2a2d0` (head word `0xc2a2d8`) under the lock `0xc2a314` and
calls `0x538040` for each client; that tail-jumps to `0x5384d0` with the queue at `+0x1a54`. At
`0x538529`, `jmp [eax*4+0x538610]` on the type less `0x12`, five entries, reached only while the
client's `+0x1a60` is zero; type `0x16` sets it. `0x538597` calls `0x537ff0` unless a `0x16` node set the local flag at `0x53855f`.
Slot 12, `0x537c50`, adds the length plus 2 to `0xc0d468`, requires `+0x70 == 6` (`0x537c71`), wraps
the data in a data store and calls `0x537aa0` (`0x537ca0`). The client's `+0x1a5c` is a reference count: one per
queued node and one for the poll in progress (`0x5384f1`); each handled node and the end of the poll
release one (`0x53856b`, `0x5385a4`), and when it reaches zero on a marked client the poll calls the
client's slot 0 with 1.

## Realm connection messages

`RealmConnection::MessageHandler` handles ten opcodes and returns the handler's result. After
the handler, a packet the handler did not read to the end is reported as `Message under read`
and discarded.

| opcode | message | handler |
|---|---|---|
| `0x1ec` | `SMSG_AUTH_CHALLENGE` | `0x5b4000` |
| `0x1ee` | `SMSG_AUTH_RESPONSE` | `0x5b41b0` |
| `0x2ef` | `SMSG_ADDON_INFO` | `0x5b4260` |
| `0x3b` | `SMSG_CHAR_ENUM` | `0x5b42a0` |
| `0x3a` | `SMSG_CHAR_CREATE` | `0x5b4590` |
| `0x3c` | `SMSG_CHAR_DELETE` | `0x5b45c0` |
| `0x41` | `SMSG_CHARACTER_LOGIN_FAILED` | `0x5b45f0` |
| `0x4c` | `SMSG_LOGOUT_RESPONSE` | `0x5b4630` |
| `0x4d` | `SMSG_LOGOUT_COMPLETE` | `0x5b4620` |
| `0x4f` | `SMSG_LOGOUT_CANCEL_ACK` | `0x5b4680` |

Addresses: `0x5b3a80`, `0x5b3ea0`, `0x5b3ecb`, `0x5b3fd0`, `0x5b3fe8`, `0x5b3f9a`, `0x5b3fc1`
Confidence: verified
Proof: one
Test: none

The constructor `0x5b3a80` registers `0x5b3ea0` for each opcode with the connection as context
(`0x5b3ab7` onwards). In `0x5b3ea0`, `0x4d` and the opcodes above it are compared directly; `0x3a`
to `0x4c` go through the byte table `0x5b3fe8` (`movzx eax,byte [eax+0x5b3fe8]`, 19 entries) into
the jump table `0x5b3fd0` at `0x5b3ecb`, whose five targets call the handlers for `0x3a`, `0x3b`,
`0x3c`, `0x41` and `0x4c`; the sixth entry is the exit for the opcodes between. `0x5b3f9a` calls the
store's slot 5, and a zero result logs through the notify object's slot 5 with the strings
`RealmConnection::MessageHandler` and `Message under read` and calls the store's slot 6
(`0x5b3fc1`). The message names are the server's names for these opcodes; each handler's own
debug string names the first three (`SMSG_AUTH_CHALLENGE` at `0x5b401e`).
