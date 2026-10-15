# warden: overview

The client's side of Warden, the server's anticheat channel. `WardenClient.cpp` (path string
`0x86f074`, code `0x6c9f80` to `0x6ca9cd`, initializers `0x82cdac` and `0x82cdb0`) holds no check and no
cipher of its own. It carries one message pair, `0x2e6` from the server and `0x2e7` back, and hosts one
module at a time: a signed, encrypted, compressed code image that it decodes, maps into the process and
calls. The first module is built into the exe; later ones are stored in the module cache, `WDB/wowcache.wdb`.
The module does the wire work (the cipher, the download protocol); this file hands it packets, time and
seven services. The methods of its buffer class begin at `0x6c9f30`, in rows still unassigned (`0x6c9f30`,
`0x6c9f40`, `0x6c9f60`).

The system is split by topic: `wire` (the messages, the services, the module's interface, activation, the
tick), `modules` (decoding, the signature, the image and its loader), `cache` (the disk cache), `embedded`
(the built-in module and its cipher). This page holds the state, what the server can reach and the
unreached code. Neighbours this system uses and others own: the cache objects (`dbcache`), the image
loader (rows owned by `os`), and Storm's RC4, SHA-1, RSA and zlib.

Roots: `0x6ca130`, `0x6ca170`, `0x6ca2f0`, `0x6ca190`, `0x6ca640`, `0x6ca5c0`, `0x6ca6a0`, `0x6ca770`, `0x6ca840`, `0x6ca8c0`, `0x6ca8e0`, `0x6ca900`, `0x6ca960`, `0x6ca070`, `0x6ca0c0`, `0x6c9ff0`, `0x6c9f80`, `0x6ca120`

## Globals and lifetime

The state is eight globals and two structs. The table pointer `0xce8954` is the object a module receives; the
lock `0xce8958` guards the rest. Static initializers build the lock and store the table pointer. The connect
event runs `InitWardenHandler`, each frame runs the tick, and a disconnect, a failed connect or a
`Disconnect` call runs `ShutdownWardenHandler`. No `atexit` entry unloads a module; `Disconnect` is reached from the
client's stop handler through `ClientServices::Destroy` (`0x5ab40a`), and whether process exit takes that path is not
read here.

| address | holds |
|---|---|
| `0xce8954` | a dword, the service table `0x811434` |
| `0xce8958` | the critical section, 24 bytes |
| `0xce8974` | the pending module, a `WardenModule` or null |
| `0xce8978` | the active module, or null |
| `0xce897c` | the active or starting module's object; its first dword is the module's table |
| `0xce8980` | the handover buffer's bytes, or null |
| `0xce8984` | the handover buffer's length |
| `0xce8988` | the time of the last tick, ms |

Addresses: `0x86f074`, `0xce8954`, `0xce8958`, `0xce8974`, `0xce8978`, `0xce897c`, `0xce8980`, `0xce8984`, `0xce8988`, `0x6ca2f0`, `0x6ca190`, `0x6ca640`, `0x5aaaa6`, `0x5aaae0`, `0x5aab06`, `0x5ab1b2`, `0x5ab40a`, `0x403629`
Confidence: verified
Proof: one
Test: none

`WardenModule` is 12 bytes: the image base `+0`, its size `+4`, and the count of libraries it loaded `+8`.
`WardenBuffer` is 8: a length `+0`, a data pointer `+4`. The lock is entered at `0x6ca195` (tick),
`0x6ca5dc` (handler), `0x6ca2f5` (init) and `0x6ca645` (shutdown); the shutdown runs inside the init's hold, so
the lock is recursive. `0x6ca640` has four callers: `0x5aaae0` (`HandleDisconnect`), `0x5aab06`
(`HandleConnectFailure`), `0x5ab1b2` (`Disconnect`) and `0x6ca2fa` (init); no `atexit` registers it
(a scan of every call, jump and stored dword finds only these; the control, the same scan for `0x6ca5c0`,
finds its stored address). `0x403629` calls the tick from `PollNet`.

## What the server can reach

Through this file the server can make the client do five things, all by way of the active module: hand it
bytes (`0x2e6`), have it send bytes back (`0x2e7`), have a record stored in the module cache and written to
`WDB/wowcache.wdb` at exit, have a cached record decrypted, signature-checked, inflated and mapped as code,
and have that code run with the client's own privileges. With no active module the handler reads the packet
and drops it. No path in this file runs a module without a valid signature.

Addresses: `0x6ca5c0`, `0x6ca6a0`, `0x6ca840`, `0x6ca770`, `0x6ca360`, `0x7a79d0`, `0x6ca1e0`, `0x56b040`
Confidence: verified
Proof: one
Test: none

The module's entry point is called by `0x6ca204` with no check beyond a non-null address. What the module does
with the bytes is its own code (`embedded` for the built-in one). The signature check (`modules`) covers the
decrypted blob and the name `MAIEV.MOD`; the key is fixed in the exe, so a blob the key owner has not signed
fails at `0x6ca467`. The module cache is read at start-up, so a file on disk reaches the same check. Section
protections come from the signed header, including write and execute.

## Static initializers

Two initializers, in the table `0x827000`, belong to this file. `0x6ca130` calls `0x6ca140`, which constructs
the lock at `0xce8958`, then jumps to `0x6ca150`, which registers `0x6ca160` (the lock's destructor) with
`atexit`. `0x6ca170` jumps to `0x6ca180`, which stores the service table address `0x811434` in `0xce8954`.

Addresses: `0x6ca130`, `0x6ca140`, `0x6ca150`, `0x6ca160`, `0x6ca170`, `0x6ca180`, `0x82cdac`, `0x82cdb0`, `0xce8954`, `0xce8958`, `0x811434`
Confidence: verified
Proof: one
Test: none

The slots `0x82cdac` and `0x82cdb0` hold the two thunks (the table's dwords at those addresses). `0x6ca140` is
`mov ecx,0xce8958; jmp 0x657990`, the lock's constructor; `0x6ca160` is `mov ecx,0xce8958; jmp 0x6579a0`, its
destructor; `0x6ca180` is `mov [0xce8954],0x811434`. The slot before `0x82cdac`, `0x82cda8`, holds `0x6c9e80`, a
counter initializer that stores `0xc2a3c4` and belongs to the code before this file.

## Unreached code

`0x6c9fa0` compares two `WardenBuffer`s (length, then bytes) and returns 1 when equal; with no data in either it
returns whether neither has data. No call or jump targets it and no dword in the image holds its address, at
any alignment.

Addresses: `0x6c9fa0`
Confidence: verified
Proof: one
Test: none

The census `cargo xtask census --why 0x6c9fa0` reports that nothing live reaches it. The control, the same
command on `0x6ca130`, finds the chain through `0x82cdac` to a root. A scan of every call and jump operand and
every little-endian occurrence of the address finds none; the same scan finds the four callers of `0x6ca640`. When
either buffer lacks data it returns whether neither has.
