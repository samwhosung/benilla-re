# ui: the input stamp and the idle law

The time of the player's last input, which events refresh it, and what the world frame does once the
player has been idle for 5 and for 30 minutes.

The stamp is the global `0xcf0bc8`, written by seven input handlers of the top (`CSimpleTop.cpp`, see
`ui/input`) and read by the idle check at the head of the world frame's render callback `0x482ea0`
(`WorldFrame.cpp`). Input events arrive on categories of the event bus (`core/events`, `core/input`);
only the category that carries a handler with a stamp store refreshes it.

Roots: `0x482ea0`, `0x765f10`, `0x765fd0`, `0x7660d0`, `0x766250`, `0x7662c0`, `0x766420`, `0x7664f0`

## What stamps

Seven handlers store the event's time in `0xcf0bc8`: the first press of a key (category 8, `0x765f34`), a key
release (9, `0x765fec`), a mouse button press (0xb, `0x7662fb`), a mouse move with the absolute pointer (0xc,
`0x766152`), a mouse move with the pointer captured for mouse look (0xd, `0x766291`), a button release (0xe,
`0x766461`) and the wheel (0x10, `0x76651f`). The stamp is process-wide: nothing resets it at world entry, and
before the first event it is 0.

Addresses: `0xcf0bc8`, `0x765f10`, `0x765f34`, `0x765fd0`, `0x765fec`, `0x7662c0`, `0x7662fb`, `0x7660d0`, `0x766152`, `0x766250`, `0x766291`, `0x766420`, `0x766461`, `0x7664f0`, `0x76651f`, `0x482ec3`
Confidence: verified
Proof: one
Test: none

A scan of the image for the dword finds 37 uses: the seven stores above, the idle check `0x482ec3`, and 29
command stubs that read it as their scheduling time (`ui/input#the-handlers`). The key handlers store
`[record+0xc]` and the mouse handlers `[record+0x20]`, both the time the ring writer took from the client clock
(`core/input#the-ring`, `core/time#the-millisecond-clock`), not the operating system's message time.

## What does not stamp

A held key stamps once: the repeats are promoted from category 8 to 0xa (`ui/keybindings#key-press-and-release`),
whose handler `0x766070` has no store, so holding a key, or autorun, counts as idle. A mouse move to the pointer
position already stored on both axes does not stamp, nor does the manager's replay of its saved move. Characters
do not stamp, though the key press that made them did. Joystick polling, double-click messages and the horizontal
wheel never enter the event bus, and a key the translator does not know is dropped before it is queued. While the
player drags or resizes the game window by its frame, moves and the button release do not stamp; mouse look does.

Addresses: `0x766070`, `0x7660d0`, `0x766114`, `0x766122`, `0x766130`, `0x76613e`, `0x766144`, `0x76614b`, `0x766158`, `0x42d2d1`, `0x42d4e5`, `0x42df90`, `0x42dfe3`, `0x42d228`, `0x424810`, `0x4248b3`
Confidence: verified
Proof: one
Test: none

`0x766114 cmp ebx,edi` is the bypass for the saved record (`edi` = the manager's `+0x1104`): equal skips both the
filter and the store (`0x766120`). `0x766122` to `0x766140` compare the saved x and y with the incoming and `jp` after
`test ah,0x44` takes a changed or unordered value to the store, equal on both returning 0 at `0x766144`.
`0x4248b3` is the promotion; `0x766070` calls the frame's slot `+0x58`. The window drag is the capture mirror
at 0x884e64: `0x42d2d1` and `0x42d4e5` test the flag set after `SetCapture` at `0x42dfe3`, taken for the hit-test codes of
the caption and the sizing borders (`0x42df90`). `0x42d228` drops an unmapped key.

## Handlers that consume first

The stamping handlers have priority 1.0. Two stubs that only return 0 are registered above them at 8.0: the
loading screen's (`0x406a50`, categories 1, 8, 0xa, 0xb, 0xc) and a world-entry container's (`0x4c0ea0`),
and the console's key handler at 7.0 returns 0 while the console is open. A consumed category never reaches
the stamper, so key presses, repeats, button presses and mouse moves do not stamp during a loading screen,
and typing in the console does not stamp. Key releases, button releases and the wheel still do during a
loading screen.

Addresses: `0x4069e0`, `0x4069e7`, `0x406a50`, `0x4c0ea0`, `0x4c0d10`, `0x63c480`, `0x63c4ca`, `0x63c4d7`, `0x63c789`, `0x41fca0`
Confidence: verified
Proof: one
Test: none

`0x4069e0` registers `0x406a50` for categories 1, 8, 0xa, 0xb and 0xc with the priority `0x41000000` (8.0);
`0x406a50` is `xor eax,eax; ret`. `0x4c0ea0` is the same stub, registered by the container `0x4c0d10`. The
console's handler returns 1, passing the key on, when no console key is held or the open flag at 0xc4eac8
is 0 (`0x63c4ca` to `0x63c4dd` jump to `0x63c789`).

## The idle law

Every frame the render callback computes `now - stamp` in milliseconds. Below 300000 it does nothing. From
300000 it runs two legs in sequence for a player whose object exists. The sit leg asks the player to sit
unless the stand state byte is nonzero, the unit is in combat (`UNIT_FIELD_FLAGS & 0x80000`), mounted
(`UNIT_FIELD_MOUNTDISPLAYID > 0`) or auto-moving (`0x6103a0`). The away leg marks the player away unless the
unit is on a taxi (flag 0x100000) or the away mirror `0xb6e5cc` is set, which makes it fire once. From 1800000
(from the stamp, not after the 5) it prints `IDLE_MESSAGE` as a system chat line and requests a logout, once,
unless a logout is pending.

Addresses: `0x482ea0`, `0x482ebe`, `0x482ec3`, `0x482ecd`, `0x482ed8`, `0x482ede`, `0x482ee5`, `0x482ef5`, `0x482f09`, `0x482f1a`, `0x482f2f`, `0x482f39`, `0x482f58`, `0x482f68`, `0x482f76`, `0x482f80`, `0x482f84`, `0x482f8b`, `0x482f91`, `0x482f99`, `0x482fab`, `0x482fb4`, `0x482fba`, `0x5ed430`, `0x5eb740`, `0x6103a0`, `0x5ab000`, `0x5ab0d0`, `0x703bf0`, `0x49a870`, `0xb6e5cc`
Confidence: verified
Proof: one
Test: none

`0x482ecd lea ecx,[eax-0x493e0]` (300000) sets no flags; `0x482ed3 test ecx,ecx; jl 0x482fbf` leaves. `eax` still holds the
elapsed time at `0x482ede add eax,0xffe488c0` (minus 1800000), so `0x482ee5 jl 0x482f39` runs the 5 to 30 minute
block. The sit gates `0x482f68`, `0x482f76`, `0x482f80` and `0x482f8b` all jump to `0x482f96` or `0x482f99`, the start
of the away leg; only `0x482f58`, no player, skips both. The sit is `0x5ed430(1)`. `0x482fab` tests the taxi bit and
`0x482fb4` the mirror before `0x5eb740(0)`. Past 30 minutes `0x482ef5` leaves when `0x5ab0d0` says a logout
is pending; otherwise `IDLE_MESSAGE` is looked up (`0x703bf0`) and shown as system chat type 10 (`0x49a870`)
and `0x5ab000(0, 0)` sends opcode 0x4b and sets the pending byte, so the next frames leave at `0x482ef5`.
