# ui: input, hit testing and focus

How the top delivers mouse and keyboard input to frames: which frames are asked, in what order,
which one keeps the input, and how a frame decides that the cursor is over it.

The top registers twelve handlers on the event categories of `core/events` and keeps 36 ordered
lists of frames, one per stratum and input kind. A frame enables input kinds in a mask at `+0xcc`:
0 characters, 1 keys, 2 the mouse, 3 the wheel. A visible frame is entered into the list of each
enabled kind for its stratum. The top remembers the hovered frame (`+0x7c`), the pressed frame
(`+0x80`), the frame that took each key (`+0x84`, 782 entries), and the frame being moved or sized
(`+0xcfc`). The frame-side handlers that run the `OnMouseDown` family of scripts are in
`CSimpleFrame.cpp`; the handlers that consult them are in `CSimpleTop.cpp`.

Roots: `0x765bf0`, `0x765d10`, `0x765df0`, `0x765e80`, `0x765f10`, `0x765fd0`, `0x766070`, `0x7660d0`, `0x766250`, `0x7662c0`, `0x766420`, `0x7664f0`, `0x7665c0`, `0x766650`, `0x764dc0`, `0x76b020`

## Input lists

The list for stratum `s` and kind `k` is the 0x1c-byte record at `+0xd10 + (s + 9k)·0x1c` of the
top. It holds 8-byte entries {frame, level}, ordered by level from high to low; a new entry goes
before the first entry of a lower level, so equal levels keep their order of entry. A new mouse
entry asks for a hover pick unless told not to. Removing a frame's entry, unless told to keep its
state, forgets it as the taker of any key (kind 1) or as the pressed and hovered frame (kind 2); a
hovered frame removed this way runs its leave handler unless it is being destroyed, and a new hover
pick is asked for.

Addresses: `0x764aa0`, `0x764aac`, `0x764ab2`, `0x764aba`, `0x764ac3`, `0x764b2d`, `0x764b84`, `0x764b8d`, `0x764ba0`, `0x766d90`, `0x766e80`, `0x766ec0`, `0x766ee0`, `0x76af00`, `0x76af50`, `0x76afa0`, `0x76afe0`, `0x8783a4`
Confidence: verified
Proof: one
Test: `ui::input_hittest_index_insert`

`0x764aa0(frame, kind, keep, level)` computes `lea eax,[eax+eax*8]` (`0x764ab2`) and `imul
edx,edx,0x1c` into `0xd10` (`0x764ac3`); a level of −1 means the frame's `+0xc4`. The entry is
`SMemAlloc(8)` under `CSimpleTop.cpp`; the scan stops at the first entry whose level is below the
new one (`cmp ebx,[edi+0x4]`, `0x764b2d`); kind 2 with `keep` 0 stores 1 at `+0x1100`
(`0x764b84`). `0x764ba0` splices the entry out, shrinks the array when it has more than eight
spare slots, frees the entry and, with `keep` 0, clears the frame from the 0x30e dwords at `+0x84`
for kind 1, or from `+0x80` and `+0x7c` for kind 2, calling slot 0x50 with 1 unless the frame's
`+0xac` is 3. `0x76af00` and `0x76af50` set or clear one bit of `+0xcc` and enter or remove the
entry when visible; `0x76afa0` and `0x76afe0` do kinds 0 to 3.

`ui::input_hittest_index_insert` runs `0x764aa0` whole, with the node allocation `0x6462e0` replaced by a bump allocator, and reads the list back.

## The handlers

The top's handlers and the categories they are registered on, each with the top as argument and
priority 1.0, are: 1 characters, 0x1b the input language, 8 key down, 9 key up, 0xa key repeat,
0xc mouse move, 0xd relative mouse move, 0xb mouse down, 0xe mouse up, 0x10 wheel, 0x1c window
size and 2 the window's activation change, on activation and deactivation alike. A handler answers 0 when the interface took the input and 1 to let the
lower-priority handlers have it.

Addresses: `0x765bf0`, `0x765bfa`, `0x765c11`, `0x765c28`, `0x765c3e`, `0x765c55`, `0x765c6c`, `0x765c82`, `0x765c99`, `0x765cb0`, `0x765cc6`, `0x765cdd`, `0x765cf4`, `0x765d10`, `0x81c378`, `0xcf0bc8`, `0x765f34`, `0x765fec`, `0x766152`, `0x766291`, `0x7662fb`, `0x766461`, `0x76651f`
Confidence: verified
Proof: one
Test: none

`0x765bf0` pushes `[0x81c378]` (1.0) and the top, then loads `edx` with the handler and `ecx`
with the category before each `EventRegisterEx` (`0x41fca0`): `0x765df0`/1, `0x765e80`/0x1b,
`0x765f10`/8, `0x765fd0`/9, `0x766070`/0xa, `0x7660d0`/0xc, `0x766250`/0xd, `0x7662c0`/0xb,
`0x766420`/0xe, `0x7664f0`/0x10, `0x7665c0`/0x1c, `0x766650`/2. `0x765d10` removes the same twelve
through `EventUnregisterEx` (`0x41fd90`). The category meanings are those of `core/input`. The key
and mouse handlers store the event's time, `+0xc` of a key payload and `+0x20` of a mouse payload,
in `0xcf0bc8` (`0x765f34`, `0x765fec`, `0x766152`, `0x766291`, `0x7662fb`, `0x766461`,
`0x76651f`); its 30 readers lie in the game-side files, which have no doc yet.

## Mouse move and the hover pick

On a mouse move the top stores the event as its last mouse event. A move to the position already
stored, unless it is a repeat pick, is ignored. While a frame is being moved or sized the move
goes to that. Otherwise the host's move callback may take it; then the pressed frame, if any, gets
the move first. The hover pick asks the mouse lists from stratum 8 down to 0, each in list order,
calling each frame's move slot until one answers nonzero; that frame becomes hovered. A change
runs the old hovered frame's leave slot with 0 and the new one's enter slot.

Addresses: `0x7660d0`, `0x7660da`, `0x766103`, `0x766122`, `0x766132`, `0x766164`, `0x766193`, `0x7661ab`, `0x7661ba`, `0x7661ca`, `0x7661d2`, `0x766207`, `0x766224`, `0x76622d`, `0x766239`, `0x766f70`, `0x7655b0`
Confidence: verified
Proof: one
Test: none

`0x7660d0` copies the event with `0x766f70` (`0x766103`); when the event is not the stored copy at
`+0x1104` it compares `+0x1118` and `+0x11c` with the new x and y and returns 0 when both are equal
(`0x766122`, `0x766132`). With `+0xcfc` set it calls `0x7655b0` and returns 0 (`0x766193`); a
nonzero answer of the callback `+0x112c` returns 0 (`0x7661ab`). Slot 0x3c of `+0x80`
(`0x7661ca`), then the walk from `+0x101c` backwards 0x1c at a time over the cursor, count and
array of each stratum's kind-2 list (`0x7661d2`) calls slot 0x3c (`0x766207`). The store is
`0x766224`; slot 0x50 and slot 0x4c follow (`0x76622d`, `0x766239`). It answers whether no frame
is hovered.

## Mouse buttons

A button press first may start a modifier drag (`Moving and sizing`). Otherwise, with no pressed
frame, the host's button callback may take it. The target is the pressed frame, else the hovered
one; without a target the press passes on. The target is raised; a press inside its title region
starts moving it; otherwise the target becomes the pressed frame and its button-down slot runs. A
release ends a move or size that a modifier drag or a title-region press started; otherwise, a
script's move or size included, it goes to the pressed frame's button-up slot, which is forgotten
when no button stays down.

Addresses: `0x7662c0`, `0x7662f3`, `0x766301`, `0x76634e`, `0x766363`, `0x766370`, `0x76637f`, `0x766389`, `0x766392`, `0x766397`, `0x7663b4`, `0x7663cc`, `0x7663e9`, `0x7663f4`, `0x766420`, `0x765640`
Confidence: verified
Proof: one
Test: none

`0x7662c0` copies the event, tests `+0xcf8` (`0x766301`) and the held keys 1 and 2 through
`EventIsKeyDown` before `0x765490` (`0x76634e`). The callback `+0x1128` runs only when `+0x80` is
null (`0x766363`, `0x766370`). The target is `+0x80` or `+0x7c` (`0x76637f`, `0x766389`); it calls
`0x76a5b0` (`0x766392`), tests the title region `+0xa8` with `0x768c70` (`0x7663b4`) and calls
`0x765320(target, 2, x, y, 0)` (`0x7663cc`); else `+0x80` = target (`0x7663e9`) and slot 0x68
(`0x7663f4`). `0x766420` calls `0x765640` when `+0xd00` is 1 or 2; otherwise slot 0x6c of `+0x80`
and the clear of `+0x80` when the event's button field is 0.

## Wheel, keys and characters

The wheel goes to the wheel lists from stratum 8 down, to each frame whose hit test contains the
cursor, until one answers nonzero. A key down goes down the key lists the same way to each frame's
key-down slot; a frame that answers nonzero is remembered for that key, else the entry stays. A
repeat and a key up go only to the remembered frame, which the key up keeps. A key up with no
remembered frame passes on and clears the entry; `PRINTSCREEN` (0x212), which has no key down, is
first offered as one, and the clear drops what that recorded. Characters and input-language changes
go down the character lists to each frame's slot.

Addresses: `0x7664f0`, `0x765f10`, `0x765f9b`, `0x765fa2`, `0x765fd0`, `0x766025`, `0x766028`, `0x76603b`, `0x766070`, `0x765df0`, `0x765e80`, `0x765f51`, `0x766048`, `0x76605b`
Confidence: verified
Proof: one
Test: none

`0x7664f0` scales the stored cursor `+0x1118`, `+0x111c` through `0x41ad80` and walks from `+0x1118`
backwards, the kind-3 lists, calling `0x76b020` and then slot 0x70. `0x765f10` walks the kind-1
lists from `+0xf20`, calls slot 0x60 and stores a frame at `+0x84 + 4·key` only when it answers
nonzero (`0x765f9b`, `0x765fa2`). `0x765fd0` calls slot 0x64 of `[+0x84 + 4·key]` (`0x766025`) and
returns 0 at `0x766028` to `0x76603b` without a store; with no frame it calls `0x765f10` when the
key is 0x212 (`0x766048`) and stores 0 in the entry (`0x76605b`). `0x766070` calls slot 0x58 of the
remembered frame. `0x765df0` and `0x765e80` walk the kind-0 lists from `+0xe24` with slots 0x5c and
0x54.

## Hit test

A frame contains a point when its rectangle is valid and `left ≤ x ≤ right` and `bottom ≤ y ≤
top` hold for its hit rectangle, the layout rectangle moved inward by the hit insets times the
layout scale; a NaN coordinate fails. A frame that draws itself (bit 0x2000) is tested against the
intersection of its own hit rectangle with that of its nearest ancestor that does not draw itself,
and contains nothing without such an ancestor.

Addresses: `0x76b020`, `0x76b116`, `0x76b119`, `0x76b121`, `0x76b151`, `0x76b175`, `0x76b17d`, `0x76b195`, `0x76b1b0`, `0x76b1e0`, `0x76b220`, `0x76b580`
Confidence: verified
Proof: one
Test: `ui::apply_rect_screen`, `ui::point_in_rect`

`0x76b020` tests bit 0 of `+0x60`, the layout flags, then `+0xb4` bit 0x2000; the plain case
compares x with `+0xdc` and `+0xe4`, y with `+0xd8` and `+0xe0`. A lower edge fails on
`test ah,0x1; jne`, below or NaN; an upper edge on `test ah,0x41; jp`, which an equal value (C3
alone) passes and a greater or NaN value fails (`0x76b175`, `0x76b17d`, `0x76b195` to `0x76b14c`).
The other case walks `+0x9c` past frames with the bit, copies the ancestor's `+0xd8` to `+0xe4` to
`ebp−0x20`, takes the larger lower edges and the smaller upper edges, and builds them into that same
copy with a rectangle constructor (`lea ecx,[ebp-0x20]` at `0x76b116`, `0x76b119`); the
compares at `0x76b121` to `0x76b151` read the intersection. `0x76b580`, the frame's rectangle
callback, stores `+0xd8 = bottom + inset_b·s`, `+0xdc = left + inset_l·s`,
`+0xe0 = top − inset_t·s`, `+0xe4 = right − inset_r·s` with `s` at `+0x7c`; `0x76b1b0` stores the
insets left `+0xec`, right `+0xf4`, top `+0xf0`, bottom `+0xe8`.

`ui::apply_rect_screen` runs `0x76b580` whole, and `ui::point_in_rect` runs `0x76b020` whole on a synthesized frame, its parent and its grandparent.

## Moving and sizing

The top keeps the frame being moved or sized, the kind (1 a modifier drag, 2 a
title-region press, 3 a script's `StartMoving` or `StartSizing`), the point and the last cursor
position, and applies each mouse move's difference to the frame's anchor. Point 4, the centre,
moves; any other point sizes from that corner or edge. A release ends kinds 1 and 2 only. A
modifier press with the flag `+0xcf8` set picks the topmost hit frame of strata 7 to 0, then its
toplevel ancestor, and moves it, or sizes it with the key-1 modifier, when movable (0x100) or
resizable (0x200); sizing picks the corner or edge by which third of the frame holds the cursor.

Addresses: `0x7652b0`, `0x7652c6`, `0x7652d7`, `0x7652e5`, `0x7652f3`, `0x7652fc`, `0x765320`, `0x765490`, `0x7654a3`, `0x76550f`, `0x76554c`, `0x765558`, `0x7655b0`, `0x765640`, `0x766fe0`, `0x8029b0`, `0x80297c`, `0x765332`, `0x765464`, `0x766346`, `0x7663c7`, `0x77681d`, `0x776819`, `0x77697c`
Confidence: inferred
Proof: none
Test: `ui::geo_765320`

`0x7652b0` calls `0x768430` on the frame's layout part, raises it (`0x7652d7`), sets bit 0x1000
(`0x7652e5`) and stores `+0xd00`, `+0xcfc`, `+0xd0c`, `+0xd04`, `+0xd08` (`0x7652f3` to
`0x76530f`); the kind is its second argument and the point its fifth. The kind is 1 from the
modifier drag (`0x766346`), 2 from the title-region press (`0x7663c7`) and 3 from the Lua
`StartMoving` and `StartSizing` (`0x77681d`, `0x77697c`); `StartMoving` passes point 4
(`0x776819`), and `0x765320` passes 4 when its size flag is 0 (`0x765332` to `0x765464`).
`0x765490` walks from `+0xcf4` backwards, eight strata, and tests 0x100 or 0x200 of
the ancestor `0x766fe0` returns. `0x765320` compares the cursor with the rectangle using 0.25
(`0x8029b0`) and 3.0 (`0x80297c`) of its extents. The point numbers and the layout effect of
`0x768430`, `0x768550` and `0x768710` were not traced to the end. `0x765640` clears `+0xcfc` and
`+0xd00`.

`ui::geo_765320` runs `0x765320` whole, with `0x768310`, `0x7680e0`, `0x768320` and `0x7652b0` stubbed; the test reads the rectangle it supplies and the grip passed to `0x7652b0`.

## Mouse focus from the game

`0x764dc0` makes a frame the hovered one from outside the hover pick: it runs the leave and enter
slots on a change, and when another frame is pressed it sends that frame a button-up built from the
stored mouse event, with x and y zeroed, and forgets it. Its one caller is in the game interface
(`GameUI.cpp`).

Addresses: `0x764dc0`, `0x764e87`, `0x764e8e`
Confidence: verified
Proof: one
Test: none

`0x764dc0` compares the frame with `+0x7c`, stores it, and calls slot 0x50 with 0 on the old one and
slot 0x4c on the new. When `+0x80` is set and differs, it builds an event from `+0x1104` to
`+0x1124`, passes the cursor through `0x41ad80`, stores the kind 0x400500c9, then 0 over the
converted x and y (`0x764e87`, `0x764e8e`), calls slot 0x6c of
`+0x80` and clears `+0x80`. The census of `e8` and `e9` bytes finds one call, at 0x493085, and no
stored address.
