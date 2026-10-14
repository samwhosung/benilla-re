# ui: Hover, leave notifies and the tooltip of a hidden frame

What entering and leaving a frame runs, which events end a hover, how the hover is picked again with no
mouse move, and why the tooltip a frame owns goes away when that frame is hidden. The hover pick itself
and the input lists are in `input`; hiding is in `visibility`.

The top (`CSimpleTop`, `[0xcf0bd8]`) keeps the hovered frame at `+0x7c`, the capture frame at `+0x80`, a
re-pick flag at `+0x1100` and the last mouse event at `+0x1104` (cursor at `+0x1118`, `+0x111c`). A frame
has an enter notify in slot 0x4c and a leave notify in slot 0x50 whose one argument is 1 for a removal and
0 for a move.

Roots: `0x76b6a0`, `0x76b6f0`, `0x764ba0`, `0x7660d0`, `0x483b10`, `0x4eb880`, `0x7cb850`, `0x7cb8b0`

## Entering and leaving

A frame's enter notify shows its companion frame (`+0xa4`), turns on its highlight texture and runs its
`OnEnter` script; the leave notify does the same in reverse and runs `OnLeave`. The highlight changes only
when the frame's `+0xf8` is 0. A leave with argument 1, a removal, also zeroes the drag-arm pair at `+0x100`
and `+0x104`, so a pending drag start is cancelled; a leave on a move leaves it. The scripts run at once,
inside the call that caused the notify, with no Lua arguments.

Addresses: `0x76b6a0`, `0x76b6af`, `0x76b6b9`, `0x76b6bf`, `0x76b6c5`, `0x76b6cd`, `0x76b6d2`, `0x76b6e5`, `0x76b6f0`, `0x76b702`, `0x76b725`, `0x76b72c`, `0x76b736`, `0x76b753`, `0x702690`, `0x704d50`, `0x6f41a0`, `0x76a710`, `0x76a730`
Confidence: verified
Proof: one
Test: none

`0x76b6a0` stores 1 at `+0xd0` of the companion and calls its slot 0x88 (`0x76b6af`, `0x76b6b9`); `0x76b6f0`
stores 0 and calls slot 0x84 (`0x76b702`). Both test `+0xf8` and call `0x76a710(4)` or `0x76a730(4)`
(`0x76b6c5` to `0x76b6cd`). `0x76b725` tests the argument and clears the pair (`0x76b72c`, `0x76b736`). The
scripts are slots `+0x140` and `+0x148`, called through `0x702690` (`0x76b6e5`, `0x76b753`); `0x704d50`
sets `this` and calls `0x6f41a0` with an argument count of zero. A companion is set by no code the class
runs. The pair is overridden by buttons, the world frame, the minimap, the nameplate and the hyperlink
region (`The notifies by class`).

## What removes a hover

A frame loses the hover when it leaves the mouse list. If it is the hovered frame its slot is cleared, the
re-pick flag is set and its leave notify runs with argument 1, unless the frame is being destroyed. The
removal happens when the frame is hidden, when its mouse is disabled and when it is detached from the top,
but not when its strata or level changes: that re-buckets with the removal's flag set and the hover stays.
A frame hidden with a hovered descendant leaves the descendant's hover first, so the descendant's `OnLeave`
runs before its own `OnHide` and before every ancestor's.

Addresses: `0x764ba0`, `0x764c6e`, `0x764c9d`, `0x764ca5`, `0x764cad`, `0x764cb3`, `0x764cb8`, `0x764cbb`, `0x764cc5`, `0x764cd0`, `0x764cd4`, `0x764920`, `0x76afe0`, `0x76b00a`, `0x76af50`, `0x76af75`, `0x76af7f`, `0x76ad50`, `0x76ad9d`, `0x76adee`, `0x76adfd`, `0x764710`, `0x764718`, `0x76a470`, `0x76a495`, `0x76a4f0`, `0x76a523`, `0x7690ac`, `0x7693cc`, `0x764dc0`, `0x764ddd`, `0x76622b`
Confidence: verified
Proof: one
Test: none

`0x764ba0(root; frame, kind, flag)` runs its tail only for a flag of 0 (`0x764c6e`). Kind 2 clears the capture
frame (`0x764ca5`, `0x764cad`) and for the hovered frame stores 0 at `+0x7c` (`0x764cb8`) and 1 at `+0x1100`
(`0x764cbb`), then calls slot 0x50 with 1 unless `+0xac` is 3 (`0x764cc5` to `0x764cd4`). `+0xac` is stored
as 1 by the constructor (`0x7690ac`) and as 3 by the destructor (`0x7693cc`) and nowhere else. The
removal's callers pass 0: `0x76af50` (`0x76af75`), `0x76ad50` through `0x764920` (`0x76ad9d`) and the detach
`0x764710` (`0x764718`); the strata and level setters pass 1 (`0x76a495`, `0x76a523`). `0x76ad50` clears
`+0xd4` and deregisters (`0x76ad9d`) before it hides the children (`0x76adee`) and runs its own hide
(`0x76adfd`). The other two leave calls pass 0 (`0x764ddd`, `0x76622b`).

## Picking the hover again

Anything that can change what lies under a stationary cursor sets the re-pick flag `+0x1100`, and the top's
update pass then runs the hover pick at the saved cursor position with no mouse move. Setting it are the top's
constructor, a frame entering the mouse list by showing or attaching (but not by `EnableMouse(true)`), a
hovered frame leaving it, a frame's rectangle being recomputed, raising a toplevel frame, leaving freelook
and arming a spell that needs a target. The pick clears the flag and bypasses the zero-motion filter.

Addresses: `0x76422a`, `0x764b7b`, `0x764b84`, `0x764b8d`, `0x764cbb`, `0x764a20`, `0x764a51`, `0x76b648`, `0x7650f0`, `0x7651dd`, `0x4930a0`, `0x493090`, `0x6e4b60`, `0x6e51ab`, `0x76ae10`, `0x764780`, `0x76ae78`, `0x76afa0`, `0x765650`, `0x7657a1`, `0x7657b4`, `0x7657be`, `0x7660d0`, `0x766114`, `0x766122`, `0x766152`, `0x764330`, `0x764340`
Confidence: verified
Proof: one
Test: none

Seven stores of 1 write `+0x1100`; the update pass `0x765650` is the reader and the only clearer
(`0x7657a1`, `0x7657b4`) and calls `0x7660d0` with the saved event at `root + 0x1104` (`0x7657be`). The insert
`0x764aa0` sets the flag only for kind 2 and flag 0 (`0x764b7b`, `0x764b8d`); `EnableMouse(true)` passes −1
to it. `cmp ebx,edi` at `0x766114` finds the incoming event is the saved copy and skips the zero-motion test
(`0x766122`) and the activity stamp (`0x766152`). The pass is registered per frame by `0x764330` and called at
`0x764340`.

## The hover walk

On a mouse move the top, unless a frame is being moved or sized, probes the capture frame once and discards
the answer, then walks the mouse lists from stratum 8 down to 0 calling each frame's probe (slot 0x3c) until
one answers nonzero. The walk then stores the winner, runs the old hover's leave notify with 0 and then the
winner's enter notify. An unchanged winner does nothing. While a move or size is in flight the walk is not
run, `+0x7c` is not touched and no notify fires.

Addresses: `0x7660d0`, `0x766164`, `0x76616e`, `0x766193`, `0x7661a1`, `0x7661ba`, `0x7661ca`, `0x7661cd`, `0x7661d2`, `0x7661e1`, `0x766207`, `0x76620c`, `0x76621b`, `0x766224`, `0x766227`, `0x766229`, `0x76622d`, `0x766239`, `0x7655b0`, `0x492c00`
Confidence: verified
Proof: one
Test: none

The move or size target `+0xcfc` set sends the event to `0x7655b0` and returns (`0x766164` to `0x766193`).
The capture probe `0x7661ca` runs only when `+0x80` is set, and its result register is overwritten at
`0x7661cd`. The sweep counts `edi` from 9 and stops at the first nonzero probe (`0x76620c`), then compares the
winner with the old hover (`0x76621b`), stores it (`0x766224`), and calls the old frame's slot 0x50 with 0
unless the old hover was null (`0x766227` to `0x76622d`), then the new frame's slot 0x4c (`0x766239`). The
shipped pre-filter `0x492c00` passes the walk's event kind (0x400500ca) through.

## The notifies by class

Six functions implement the leave notify across the 27 frame vtables: the base for twenty, the button's for
three, and one each for the world frame, the minimap, the nameplate and the hyperlink region. The leave's
argument is read only by the base. A disabled button's leave notify, like its enter notify, skips the base
call, so it runs no script, drops no highlight and does not clear the drag arm. The hyperlink region's runs
no base leave.

Addresses: `0x76b6f0`, `0x7794e0`, `0x779490`, `0x7794e6`, `0x7794ee`, `0x7794f4`, `0x483b10`, `0x4eb880`, `0x7cb8b0`, `0x7cb850`, `0x7a34e0`, `0x8045c0`, `0x81c7f8`, `0x81cb80`, `0x81d5b0`, `0x8043e8`, `0x807a80`, `0x81de50`
Confidence: verified
Proof: one
Test: none

The slot 0x50 values of the vtables that hold the pairs `0x76ad50`/`0x76ae10`, `0x505ce0`/`0x505d00` or
`0x530a60`/`0x530a80` at slots 0x84 and 0x88 are: `0x76b6f0` twenty times, `0x7794e0` for `0x8045c0`,
`0x81c7f8` and `0x81cb80`, `0x483b10` for `0x8043e8`, `0x4eb880` for `0x807a80`, `0x7cb8b0` for `0x81de50` and
`0x7a34e0` for `0x81d5b0`. `0x7794e0` tests the enabled word `+0x328` and calls `0x76b6f0` only when it is nonzero
(`0x7794e6`, `0x7794ee`, `0x7794f4`).

## World hover and its tooltip

Leaving the world frame publishes a cleared world mouseover and resets the cursor. The world pick runs only
while the world frame is the top's hover. On a change of the pick the previous pair is published first
and the new pair stored after. The publisher fades the game tooltip out when a previous mouseover, or failing
that a published one, is nonzero, and later gives the tooltip back to `UIParent` with the cursor anchor and
zero offsets: the fade first, the new owner second.

Addresses: `0x483b10`, `0x483b1d`, `0x483b22`, `0x483b32`, `0x483b46`, `0x483b58`, `0x483b5d`, `0x523d30`, `0x492e70`, `0x492890`, `0x4928ba`, `0x4928c6`, `0x4928e8`, `0x492901`, `0x492909`, `0x492a1e`, `0x4817a8`, `0x4817b1`, `0x482090`, `0x4820ad`, `0x4820cf`, `0xb4e2c8`, `0xb4b3c4`, `0xb4b44c`, `0x83dc2c`, `0x530ae0`, `0x530ae3`
Confidence: verified
Proof: one
Test: none

`0x483b10` calls the base leave first (`0x483b1d`), then, when the cached pair at `+0x340`, `+0x344` is
nonzero, `0x492e70` with `(0, 0, prev)` (`0x483b46`) and zeroes the pair, then `0x523d30` and stores 3 at
`0x83dc2c` on both paths (`0x483b58`, `0x483b5d`). `0x4817b1` skips the pick unless `[root + 0x7c]` is the
world frame. `0x482090` returns when both halves of the new pair equal the cached ones (`0x4820ad`) and else
publishes (`0x4820cf`) before it stores. The publisher `0x492890` tears down on a nonzero previous GUID
(`0x4928ba`) or a nonzero global (`0x4928c6`), clearing `0xb4e2c8` (`0x4928e8`) and calling `0x530ae0` on the
tooltip with no null test (`0x492909`); `0x492a1e` calls `0x52ffe0(UIParent, 6, 0, 0)`.

## The tooltip of a hidden frame

Hiding the frame under the cursor makes its tooltip vanish only because the frame's own `OnLeave` script
hides it. The engine tracks no relation between a tooltip and the visibility of its owner: the owner is
stored at SetOwner and compared, nothing polls it, and nothing unlinks it when the owner is destroyed. Hiding
a tooltip un-owns it. A frame with no `OnLeave` leaves its tooltip shown until something else hides it or
gives it a new owner.

Addresses: `0x529240`, `0x529301`, `0x52ffe0`, `0x53000c`, `0x530a60`, `0x530a6b`, `0x530a80`, `0x530a83`, `0x530640`, `0x530649`, `0x530b20`, `0x5310d0`, `0x530fe0`, `0x53109f`, `0x531f00`, `0x7693b0`, `0x49009e`, `0x490caa`, `0xb4b3c4`
Confidence: verified
Proof: one
Test: none

The owner `+0x314` is written only by the constructor (`0x529301`) and the SetOwner core (`0x53000c`). The
tooltip's hide slot, `0x530a60`, calls the core with a null owner (`0x530a6b`) before the base hide; its show
slot `0x530a80` shows only with an owner and a line (`0x530a83`); its update `0x530b20` is the cursor anchor and
the fade. The frame destructor `0x7693b0` has no reference to `+0x314` or to the tooltip pointer
`0xb4b3c4`, which is set at interface build (`0x49009e`) and cleared at teardown (`0x490caa`). Lua
`FadeOut` is `0x531f00`.

## Nameplate and minimap notifies

The nameplate's enter publishes its unit as the world mouseover and its leave publishes none, then each
calls the button's notify; the leave fades the tooltip only if a mouseover was still published. The minimap's
leave hides the tooltip at once, before the base leave, on every leave, with no owner test, and does not clear
the minimap's set of hovered elements: when the cursor comes back onto the same blips the probe finds the
set unchanged and shows nothing until the set changes or a probe outside the rectangle clears it.

Addresses: `0x7cb850`, `0x7cb869`, `0x7cb87e`, `0x7cb8a5`, `0x7cb8b0`, `0x7cb8bf`, `0x7cb8ca`, `0x7cb8e1`, `0x7cb8ff`, `0x4eb880`, `0x4eb886`, `0x4eb898`, `0x4eb8a4`, `0x4eb0c0`, `0x4eb0e7`, `0x4eb12b`, `0x4eb133`, `0x4eb4b1`, `0x4eb4f1`, `0x4eb507`, `0x4eb546`, `0x4eb54c`, `0x4eb561`, `0x4eb567`, `0x4ed9d0`, `0x4eda51`, `0xbc81e4`, `0xbdcbac`, `0xbdcbb0`, `0xbdcbb4`
Confidence: verified
Proof: one
Test: none

`0x7cb850` calls `0x492890(lo, hi, 0, 0)` with the plate's GUID at `+0x4e8` (`0x7cb869`) and `0x779490`
(`0x7cb8a5`); `0x7cb8b0` calls it with four zeros (`0x7cb8bf`) and `0x7794e0` (`0x7cb8ff`). `0x4eb880` loads the
tooltip (`0x4eb886`), stores 0 at `+0xd0` and calls slot 0x84 (`0x4eb898`) and then `0x76b6f0` (`0x4eb8a4`);
it has no branch. The probe `0x4eb0c0` starts with the base probe `0x76b320` (`0x4eb0e7`). The set cells
`0xbc81e4` (five dwords), `0xbdcbac`, `0xbdcbb0` and `0xbdcbb4` are written only inside it, apart from the five-dword
clear at `0x4eda51` in `0x4ed9d0`; its clear block is `0x4eb12b` to `0x4eb158`, run only when the cursor is
outside the rectangle and a set is held. An unchanged set exits at `0x4eb4f1`.

## Freelook and the removal

Entering freelook removes every nameplate from the mouse list and publishes no world mouseover. For each
plate that is mouse-enabled and effectively visible it runs the removal with flag 0, so the hovered plate
gets its leave with argument 1 and the others are removed silently; the world frame's pair is then cleared.

Addresses: `0x493070`, `0x493079`, `0x493085`, `0x483e80`, `0x483e8f`, `0x483e9a`, `0x60f830`, `0x60f850`, `0x60f864`, `0x76af50`, `0x76af67`, `0x76af73`, `0x76af7f`, `0x764dc0`, `0xb4b2bc`, `0xc4d924`
Confidence: verified
Proof: one
Test: none

`0x493070` loads the world frame `[0xb4b2bc]`, calls `0x483e80` (`0x493079`) and the focus setter `0x764dc0`
(`0x493085`). `0x483e80` sets bit 1 of `+0x38c` (`0x483e8f`) and calls `0x60f830` (`0x483e9a`), which walks the
plate list `0xc4d924` and calls `0x76af50(2)` per plate when its argument is 1 (`0x60f864`). `0x76af50` removes
only a frame whose kind bit is set in `+0xcc` (`0x76af67`) and which is effectively visible (`0x76af73`).
