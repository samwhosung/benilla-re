# warden: the wire and the services

The messages the client exchanges with the server for Warden, the seven services it gives a module, the four
slots it calls in a module, and when it starts, ticks, switches and stops a module. The client reads no field
of the payload and applies no cipher to it: both belong to the active module.

Roots: `0x6ca2f0`, `0x6ca5c0`, `0x6ca190`, `0x6ca640`, `0x6ca6a0`, `0x6ca770`, `0x6ca840`, `0x6ca8c0`, `0x6ca8e0`, `0x6ca900`, `0x6ca960`

## The server's message

Opcode `0x2e6` has the handler `0x6ca5c0`, registered when a connection is made and cleared when it ends.
The handler takes the lock, passes every byte after the opcode to the active module in one block, in place,
through module slot 2, then runs `ActivateNextModule`, and returns 1. With no active module it takes the
bytes and does nothing more. The client does not decrypt, parse or check the payload.

Addresses: `0x6ca5c0`, `0x6ca2f0`, `0x6ca338`, `0x6ca33d`, `0x6ca347`, `0x6ca5c6`, `0x6ca5e7`, `0x6ca5ef`, `0x6ca615`, `0x6ca618`, `0x6ca627`, `0x419ac0`, `0x5ab650`
Confidence: verified
Proof: two
Test: `warden::handler_whole`

`tables/opcodes.tsv` has `0x2e6 0x6ca5c0 0x6ca347 A`. A handler gets its context in `ecx`, the opcode in `edx`
and the time and the data store on the stack, and returns with `ret 8` (`net/dispatch#dispatch`). `0x6ca5c6`
compares `edx` with `0x2e6` and returns 0 otherwise. The length is the store's `+0x10` less its `+0x14`
(`0x6ca5e4`, `0x6ca5e7`); `0x419ac0` returns that many bytes in place and moves the cursor. With an active module
(`[0xce8978]`), `0x6ca615` calls slot 2 with the pointer, the length and the address of a dword set to 0; the
dword is not read afterwards, nor is the call's return value (`0x6ca618` follows at once). `0x6ca627` returns 1.
`0x6ca338` and `0x6ca33d` load the handler and the opcode `0x2e6` for the registration call `0x6ca347`.

## The client's message

The client sends opcode `0x2e7` only from service slot 0. The slot builds a data store, puts the dword `0x2e7`,
then the module's bytes one at a time, and sends it on the current connection. The payload is exactly the
module's bytes: this file adds no length, tag or checksum, and a length of 0 sends the opcode alone. The
connection layer frames the message and enciphers its header as for any message; the body goes out as given
(`net/wire#header-cipher`).

Addresses: `0x6ca6a0`, `0x6ca6da`, `0x6ca6e2`, `0x6ca6fc`, `0x6ca70c`, `0x418190`, `0x418070`, `0x5ab630`
Confidence: verified
Proof: two
Test: none

`0x6ca6da` pushes `0x2e7` and `0x6ca6e2` calls `0x418190` (`PutUInt32`) on a data store built in the frame.
The loop at `0x6ca6f0` to `0x6ca704` calls `0x418070` (`PutInt8`, `0x6ca6fc`) once per byte, in order. `0x6ca70c`
calls `ClientServices::Send`. A scan of the listing for the immediate `0x2e7` finds one instruction, `0x6ca6da`;
the control, the same scan for `0x2e8`, finds twenty.

## The service table

The module receives `0xce8954`, whose dword points at the table `0x811434`: seven stdcall services, in this
order. The service functions take no lock and none reads `ecx` before writing it, so what a module passes there is
unused. A service called from inside a module slot runs under the client's lock, which every call into a slot
holds (inferred: a call from another thread would run without it).

| slot | function | arguments | does |
|---|---|---|---|
| 0 | `0x6ca6a0` | bytes, length | sends the bytes as message `0x2e7` |
| 1 | `0x6ca770` | id16, key16 | looks the 16-byte id up in the cache; if found, frees the pending module and decodes the record with the key into the pending slot; returns whether it did |
| 2 | `0x6ca840` | id16, bytes, length | removes the id's record, then stores a copy of the bytes under it |
| 3 | `0x6ca8c0` | size | allocates, returns the pointer |
| 4 | `0x6ca8e0` | pointer | frees |
| 5 | `0x6ca900` | bytes, length | replaces the handover buffer with a copy |
| 6 | `0x6ca960` | destination, length pointer | copies out up to the length, stores the count, frees the buffer; returns 0 with no buffer |

Addresses: `0x811434`, `0xce8954`, `0x6ca6a0`, `0x6ca770`, `0x6ca810`, `0x6ca840`, `0x6ca8c0`, `0x6ca8e0`, `0x6ca900`, `0x6ca960`, `0x6ca7cf`, `0x6ca7e6`, `0x6ca7f4`, `0x6ca801`, `0x6ca86f`, `0x6ca8a0`
Confidence: verified
Proof: two
Test: none

The dwords at `0x811434` to `0x81144c` are the seven function addresses; `0x6ca180` stores the table's address in
`0xce8954`, and `0x6ca204` passes `0xce8954` as `this`. Slot 1 copies the id to the stack (`0x6ca810`), calls the
cache's lookup `0x56b430` with no callback (`0x6ca7cf`), frees the pending module (`0x6ca7e6`), decodes the
record's bytes (`0x6ca7f4`, `cache`) and stores the result at `0xce8974` (`0x6ca801`). Slot 2 calls the remove
`0x56c5f0` (`0x6ca86f`), then the store `0x56b6f0` (`0x6ca8a0`). Slots 3 and 4 call `SMemAlloc` (`0x6ca8d3`,
line `0x2ab`) and `SMemFree` (`0x6ca8f3`, line `0x2b2`).

## The module interface

The client calls four slots of the object a module returns, with `ecx` that object: slot 0 is the start
`Init(key, 0x28)`, with the 40-byte session key and its length; slot 1 is the unload, no arguments; slot 2 is the
packet, `(bytes, length, result pointer)`; slot 3 is the tick, `(milliseconds)`. Each is stdcall. The module's
entry, export ordinal 1, is called with `ecx = 0xce8954` and no stack arguments, and returns that object, or 0.

Addresses: `0x6ca1e0`, `0x6ca204`, `0x6ca227`, `0x6ca2a2`, `0x6ca615`, `0x6ca1c0`, `0x6ca214`, `0x6ca21b`, `0x537a50`
Confidence: verified
Proof: two
Test: none

`0x6ca1f4` pushes 1 and `0x6ca1f6` calls the export lookup `0x7a7de0`; `0x6ca204` calls the result. `0x6ca214`
and `0x6ca21b` take the key's address from the current connection (`0x537a50`, `NetClient+0x48`); `0x6ca227`
calls `[esi]`, the first dword of the module's table, after `push 0x28` at `0x6ca212` and the key at `0x6ca226`.
Slot 1 is called at `0x6ca2a2` after `push esi` at `0x6ca2a1`, which saves the register the function uses
below; the unload takes no argument. Slot 2 at `0x6ca615` and slot 3 at `0x6ca1c0`. The built-in module's slots end
in `ret 8`, `ret`, `ret 0xc` and `ret 4` (`embedded`).

## Activating a module

`ActivateNextModule` does nothing when no module is pending. Otherwise it unloads the active module, looks up
the pending module's export 1, calls it, and calls slot 0 of the object it returns with the session key. It then
makes the pending module active and empties the pending slot, returning 1. A null entry or object frees the
pending module and returns 0, leaving no active module. The old module is unloaded first, so a failure
leaves none. It runs from `InitWardenHandler`, and after each tick and each packet only while a module is active.

Addresses: `0x6ca1e0`, `0x6ca1e5`, `0x6ca1e9`, `0x6ca1f6`, `0x6ca204`, `0x6ca208`, `0x6ca227`, `0x6ca22e`, `0x6ca233`, `0x6ca241`, `0x6ca246`, `0x6ca250`, `0x6ca290`, `0x6ca2f0`, `0x6ca190`, `0x6ca5c0`
Confidence: verified
Proof: one
Test: none

`0x6ca1e5` tests `[0xce8974]`. `0x6ca1e9` unloads the active module: slot 1 (`0x6ca2a2`), then
`0x7a7d80` on its image, the object freed under the tag `delete` (`0x6ca2cb`), and `0xce8978` and `0xce897c`
zeroed (`0x6ca2d0`, `0x6ca2da`). `0x6ca208` stores the object in `0xce897c` before slot 0 runs; `0x6ca22e` copies
the pending module to `0xce8978` and `0x6ca233` zeroes `0xce8974`. A failure calls `0x6ca250` (`0x6ca241`), which
unloads and frees the pending module. Slot 0 runs while `0xce8978` is 0 and `0xce8974` still holds the module. A call to
service 1 from the module's start frees the module being started (`0x6ca7e6`, `0x6ca250`: its libraries are freed,
its image stays mapped by the rejection `modules` names, its `WardenModule` is freed) and stores the module it decodes
in `0xce8974`. When the start returns, `0x6ca22e` and `0x6ca233` make that module active without running its entry
point, and `0xce897c` still holds the first module's object: the tick, the packet handler and the unload call the
first module's slots for the second's life. Only a signed module can do this. The calls from the tick (`0x6ca1c3`) and the
handler (`0x6ca618`) lie inside the branch that tests `[0xce8978]`.

## The tick

Each frame the tick enters the lock, reads the millisecond clock, and stores it. With an active module it calls
slot 3 with the milliseconds since the previous tick (a 32-bit difference), then `ActivateNextModule`.
`InitWardenHandler` stores the clock at start, so the first tick's value runs from the connect. With no active
module only the stamp changes.

Addresses: `0x6ca190`, `0x6ca195`, `0x6ca19a`, `0x6ca1a7`, `0x6ca1b1`, `0x6ca1c0`, `0x6ca1c3`, `0x6ca1cd`, `0x42c010`, `0x403620`, `0x403629`, `0x6ca331`, `0x6ca342`
Confidence: verified
Proof: one
Test: `warden::tick_whole`

`0x403620` is `PollNet`, registered for event category 6 and run every loop tick (`net/dispatch`); it calls
`0x5b3db0` (the connection event queues, where the handler runs) and then `0x403629` the tick. `0x6ca19a` calls
`OsGetAsyncTimeMs`; `0x6ca1a7` stores the value in `0xce8988`; `0x6ca1b1` subtracts the old one. `0x6ca1b3` skips to
the unlock when `[0xce8978]` is 0. `0x6ca331` stores the clock in `0xce8988` at start.

## The handover buffer

One buffer survives a module swap. Service 5 frees any buffer held and keeps a copy of the bytes it is given;
service 6 gives out up to the caller's length and frees the whole buffer, so a short request discards the rest.
Shutdown, and the start of a connection, free and clear it. No other code in this file reads it.

Addresses: `0x6ca900`, `0x6ca960`, `0x6ca640`, `0x6ca66a`, `0x6ca674`, `0x6ca67e`, `0x6ca97b`, `0x6ca981`, `0x6ca993`, `0x6ca9af`, `0x6ca9b5`, `0x6ca9bf`
Confidence: verified
Proof: one
Test: `warden::handover_buffer`

`0x6ca900` frees `[0xce8980]` when set (`0x6ca919`), allocates the length (`0x6ca930`) and stores the pointer and the
length at `0xce8980` and `0xce8984` (`0x6ca937`, `0x6ca93c`). `0x6ca960` returns 0 when `[0xce8980]` is 0
(`0x6ca96c`), clamps `*length` to the stored length (`0x6ca97b`, `0x6ca97f`), copies that many bytes
(`0x6ca993`), frees the buffer (`0x6ca9af`) and zeroes both globals (`0x6ca9b5`, `0x6ca9bf`). `0x6ca66a` frees it in
the shutdown. The test runs `0x6ca900` and `0x6ca960` whole; the shutdown's free is not run.
