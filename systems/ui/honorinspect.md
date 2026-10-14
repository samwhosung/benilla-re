# ui: the inspected player's honor

The honor figures of the player being inspected: the one-slot cache, the request that fills it, the reply layout, the
six Lua functions that touch it and how the target changes. The honor functions of the active player are `ui/honor`.

Roots: `0x4c6d10`, `0x4c6e40`, `0x4c6f70`, `0x4c80a0`, `0x4c95e0`, `0x4c9610`, `0x4c9620`, `0x4c9780`, `0x48a310`

## Inspect honor slot

The client keeps the honor figures of one target, in fixed cells, with the target's GUID, a has-data flag and a pending
flag. The cells are the highest rank (byte), the session, yesterday, last-week and this-week kill pairs (16 bits per
half), the two lifetime counts, the yesterday, last-week and this-week honor, the last-week standing and the rank bar
byte (32 bits and a byte). The second halves of the yesterday, last-week and this-week pairs are stored and returned by
no function. The slot has no age: nothing compares a time against it.

| cell | holds |
|---|---|
| `0xb71fc0` | target GUID, 8 bytes |
| `0xb71fc8` | has data, 1 once a reply stored |
| `0xb71fcc` | pending, 1 after a request |
| `0xb71f28` | highest rank |
| `0xb71f74`, `0xb71f88` | session pair |
| `0xb71f4c`, `0xb71f66` | yesterday pair |
| `0xb71f10`, `0xb71f64` | last-week pair |
| `0xb71f86`, `0xb71f7c` | this-week pair |
| `0xb71f90`, `0xb71f8c` | lifetime counts |
| `0xb71f3c`, `0xb71f80`, `0xb71f30` | yesterday, last-week, this-week honor |
| `0xb71fb8` | last-week standing |
| `0xb71f84` | rank bar |

Addresses: `0xb71fc0`, `0xb71fc8`, `0xb71fcc`, `0xb71f28`, `0xb71f74`, `0xb71f88`, `0xb71f4c`, `0xb71f66`, `0xb71f10`, `0xb71f64`, `0xb71f86`, `0xb71f7c`, `0xb71f90`, `0xb71f8c`, `0xb71f3c`, `0xb71f80`, `0xb71f30`, `0xb71fb8`, `0xb71f84`
Confidence: verified
Proof: one
Test: none

`0xb71f64`, `0xb71f66` and `0xb71f7c` occur once each in the image, as the destination pushed for the reply reader
(`0x4c6ec5`, `0x4c6ead`, `0x4c6edd`); no instruction reads them. The GUID cell is written at `0x4c6f8b` and `0x4c6f92`
only and read at `0x4c6e58`, `0x4c6e63` (the reply), `0x4c6f73`, `0x4c6f83` and `0x4c80c2` to `0x4c80fc` (the request).
The reply tests two compares against it, so the cache cannot hold a second target.

## Inspect target

`SetInspectTarget(guid)` does nothing when the GUID equals the stored one; otherwise it stores the GUID and clears the
has-data and pending flags, and leaves the figures. `ClearInspectPlayer()` calls it with a zero GUID and returns nothing.
`NotifyInspect(unit)` raises `Usage: NotifyInspect(unit)` for a non-string, does nothing without an active player, and
otherwise sends the inspect request for the unit's GUID and sets the target to it (`0x5decc0`, in
`objectlayer/interaction#other-units`). Inspecting the same unit again keeps the figures, and nothing else clears them.

Addresses: `0x4c6f70`, `0x4c6f79`, `0x4c6f83`, `0x4c6f89`, `0x4c6f8b`, `0x4c6f98`, `0x4c6f9d`, `0x4c9780`, `0x4c9784`, `0x48a310`, `0x48a332`, `0x48a351`, `0x48a36a`, `0x48a37b`, `0x48a38b`, `0x5decc0`, `0x5ded1c`
Confidence: verified
Proof: one
Test: none

The equal-GUID test `je 0x4c6fa2` (`0x4c6f89`) skips the writes; the flags are written from a zero register
(`0x4c6f90`, `0x4c6f98`, `0x4c6f9d`). The two callers of `0x4c6f70` are `0x4c9780` and `0x5ded1c`. The request packet is
sent before the target is set (`0x5ded10`, `0x5ded1c`).

## Inspect honor request

`RequestInspectHonorData()` returns nothing. It sends the honor request only when the has-data flag is 0, the pending
flag is 0 and the target GUID is not zero. The request is opcode 0x2d6 with the 8-byte target GUID and nothing else.
Sending sets the pending flag at once, whether or not a connection took the packet. The same opcode number carries the
reply.

Addresses: `0x4c9610`, `0x4c80a0`, `0x4c80a6`, `0x4c80b6`, `0x4c80c2`, `0x4c80cf`, `0x4c80f1`, `0x4c8107`, `0x4c8112`, `0x4c811b`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

Each failed test jumps to the same return (`0x4c80ae jne`, `0x4c80bc jne`, `0x4c80cd je`, all to `0x4c8143`). The packet is built
with `PutUInt32` of the opcode (`0x4c80f1`) and `PutGuid` (`0x4c8107`), then sent (`0x4c8112`). The pending flag is stored
after the send call (`0x4c811b`).

## Inspect honor reply

Opcode 0x2d6 from the server is 50 bytes: the player's 8-byte GUID; the highest rank, a byte; eight 16-bit values, the
pair halves in the order session first and second, yesterday first and second, last week first and second, this week first
and second; the six 32-bit values lifetime first, lifetime second, yesterday honor, last-week honor, this-week honor and
last-week standing; and the rank bar byte. A GUID other than the target's drops the packet whole. A match stores every
value, sets the has-data flag, clears the pending flag and signals `INSPECT_HONOR_UPDATE` (525).

Addresses: `0x4c6d10`, `0x4c6e30`, `0x4c6e40`, `0x4c6e50`, `0x4c6e58`, `0x4c6e63`, `0x4c6e6b`, `0x4c6e84`, `0x4c6e90`, `0x4c6e9c`, `0x4c6ea8`, `0x4c6eb4`, `0x4c6ec0`, `0x4c6ecc`, `0x4c6ed8`, `0x4c6ee4`, `0x4c6ef0`, `0x4c6efc`, `0x4c6f08`, `0x4c6f14`, `0x4c6f20`, `0x4c6f2c`, `0x4c6f38`, `0x4c6f42`, `0x4c6f4c`, `0x4c6f56`, `0x5ab650`, `0x5ab670`
Confidence: verified
Proof: one
Test: none

`0x4c6d10` registers the handler for 0x2d6 (`push 0; mov edx,0x4c6e40; mov ecx,0x2d6`) and `0x4c6e30` clears it. The
reader widths are those of the called functions: `GetUInt8` (`0x418cb0`), `GetUInt16` (`0x418db0`), `GetUInt32`
(`0x418eb0`), `GetGuid` (`0x4190b0`), in the order of the stores at the addresses listed. On a mismatch `0x4c6e6b` moves
the read cursor to the end of the packet and returns 1 with no cell touched. A short packet leaves the unread cells as
they were. The signal is `mov ecx,0x20d` (`0x4c6f3d`) and `0x703e50` (`0x4c6f56`), the only fire site of the event.

## Inspect honor functions

`HasInspectHonorData()` returns the number 1 when the has-data flag is set and nil otherwise. `GetInspectHonorData()`
returns 12 numbers with no check of the flag: session first and second, yesterday first, yesterday honor, this-week first,
this-week honor, last-week first, last-week honor, last-week standing, lifetime first, lifetime second and the highest
rank. All are unsigned. `GetInspectPVPRankProgress()` is `ui/honor#rank-progress`.

Addresses: `0x4c9610`, `0x4c95e0`, `0x4c95e9`, `0x4c95fb`, `0x4c9620`, `0x4c9626`, `0x4c9642`, `0x4c965c`, `0x4c9676`, `0x4c9693`, `0x4c96ad`, `0x4c96c9`, `0x4c96e3`, `0x4c96ff`, `0x4c971b`, `0x4c9736`, `0x4c9752`, `0x4c976d`
Confidence: verified
Proof: one
Test: none

`HasInspectHonorData` pushes the double 1.0 (`push 0x3ff00000; push 0`, `0x4c95e9`) or nil (`0x4c95fb`), one value either way,
never a boolean. `GetInspectHonorData` loads the cells in the order named, 16-bit ones with `movzx` and 32-bit ones as a
qword with a zero high half, and returns the count 12 (`0x4c976d`). Reading before a reply returns the previous figures
or zeros.
