# ui: MessageFrame and ScrollingMessageFrame

How the two message widgets take a line, colour it, keep it, fade it out and bring it back. The
`ScrollingMessageFrame` is the chat window's type; the `MessageFrame` is a plain one, the error
line's.

The two are siblings, not one type over the other. A `CSimpleMessageScrollFrame` is 0x3c4 bytes,
vtable 0x81ce18, built on the hyperlinked frame `0x7a3540`; its fields are the line count
`+0x33c`, the line limit `+0x340`, the write position `+0x344` (−1 when empty), the scroll
position `+0x348`, the lines shown `+0x34c`, at top `+0x350`, at bottom `+0x354`, its font
`+0x35c`, fading `+0x360`, the visible time `+0x364` and fade time `+0x368`, and the ring of 24-byte
line records at `+0x394`. A `CSimpleMessageFrame` is 0x378 bytes, vtable 0x81cc68, built on the
frame; its font is `+0x344`, fading `+0x348`, the two times `+0x34c` and `+0x350`, and the insert
mode `+0x354`.

Roots: `0x787670`, `0x785640`, `0x794190`, `0x795b70`

## Construction

Both start with fading on, 10 seconds visible and a 3-second fade, and their own font. The factory
gives a scrolling frame a limit of 8 lines and starts it empty and at the bottom. A message frame
starts with the insert mode 1, `BOTTOM`.

Addresses: `0x787670`, `0x78767f`, `0x78775f`, `0x78776b`, `0x7877b7`, `0x7877c7`, `0x7877d2`, `0x7878d7`, `0x785640`, `0x78572a`, `0x785736`, `0x785749`, `0x785742`
Confidence: verified
Proof: one
Test: none

`0x787670` calls `0x7a3540` (`0x78767f`), stores its second argument at `+0x340` (`0x78775f`), which
the factory pushes as 8, 1 at `+0x354` (`0x78776b`) and `+0x360` (`0x7877b7`), 10.0 and 3.0 from
0x81cddc and 0x81cde0 (`0x7877c7`, `0x7877d2`), and −1 at `+0x344` (`0x7878d7`). `0x785640`
stores 1 at `+0x348` (`0x78572a`), 10.0 and 3.0 from 0x81cc2c and 0x81cc30 (`0x785736`,
`0x785749`) and 1 at `+0x354` (`0x785742`). The factory allocates 0x3c4 and 0x378 bytes.

## A scrolling frame's AddMessage

`AddMessage(text, r, g, b, id)` does nothing for an absent, nil or empty text. With r, g and b all
numbers, each is clamped to 0 to 1 (NaN to 1) and becomes `trunc(v·255 + 0.5)`; the alpha is
always 255, and the id is argument 6. Without all three, the colour is the frame font's and the id
is argument 3. An absent or non-number id is 0, and `UpdateColorByID` ignores id 0.

Addresses: `0x792900`, `0x79299c`, `0x7929be`, `0x7929c7`, `0x7929da`, `0x792a1b`, `0x792a2d`, `0x792ac0`, `0x792ac6`, `0x792acc`, `0x792add`, `0x792b13`, `0x792b39`, `0x792b48`, `0x7929b7`, `0x792b7c`, `0x788250`, `0x788259`
Confidence: verified
Proof: one
Test: none

`0x792900` requires `0x6f3510` on argument 2 and a nonempty string (`0x79299c` to `0x7929c7`), then
`0x6f34d0` on 3, 4 and 5 (`0x7929da`). Each value is compared with 0.0 and then 1.0, `test ah,5;
jp` (`0x792a1b`, `0x792a2d`), multiplied by 255.0, added 0.5 and truncated by `0x40a2b0`
(`0x792ac0` to `0x792acc`); `or edi,0xffffff00` sets the alpha (`0x792add`). The id is read at 6
(`0x792b13`) or, after `[[+0x35c]+0x58]` (`0x792b39`), at 3 (`0x792b48`), else stays the 0 of
`0x7929b7`. `0x788250` returns for id 0 (`0x788259`).

## The ring of lines

A scrolling frame keeps at most its line limit of lines in a ring: each new line goes in the next
slot, replacing the oldest when the ring is full, and its record keeps the text, the colour, the
id and a copy of the frame's two times. `SetMaxLines` empties the frame before setting the new
limit.

Addresses: `0x788150`, `0x788177`, `0x78817b`, `0x788189`, `0x788194`, `0x788196`, `0x7881df`, `0x787dd0`, `0x787e95`, `0x787e9b`, `0x787f95`, `0x7938a0`
Confidence: verified
Proof: one
Test: none

`0x788150` raises the count up to `+0x340` (`0x788177`, `0x78817b`), advances `+0x344`, wrapping at
the limit (`0x788189` to `0x788196`), and frees the slot's old text (`0x7881df`). `0x787dd0`, which
`SetMaxLines` (`0x7938a0`) calls, frees the records and nodes, stores the count 0 and the limit
(`0x787e95`, `0x787e9b`) and −1 as the position (`0x787f95`).

## Fading a scrolling frame

Each update, a scrolling frame with fading on, at the bottom and showing lines counts each shown
line down: first its visible time; when that runs out a line with no fade time is hidden, else the
fade runs, its alpha `trunc(left / frame fade time · 255)`; a line whose fade runs out is hidden.
Scrolled up, the clock stops.

Addresses: `0x788460`, `0x788471`, `0x788480`, `0x78848c`, `0x78849b`, `0x7884bb`, `0x7884d4`, `0x7884e5`, `0x7884f9`, `0x788513`, `0x788521`, `0x788525`, `0x78852b`, `0x788547`, `0x78854d`, `0x788553`, `0x788558`
Confidence: verified
Proof: one
Test: none

`0x788460` (slot 14) calls `0x76b2c0`, then requires `+0x360` (`0x788480`), `+0x354` (`0x78848c`)
and `+0x34c > 0` (`0x78849b`). A record with `+0x8` clear is skipped (`0x7884bb`). A nonzero `+0xc`
is reduced by the frame time (`0x7884d4`); still above 0 it waits (`0x7884e5`); else a zero `+0x10`
hides the line (`0x7884f9` to `0x788525`). A zero `+0xc` reduces `+0x10` (`0x788513`); at or below
0 it hides (`0x788521`); else `0x788547` divides by `+0x368`, multiplies by 255 and truncates into
the alpha byte `+0x7` (`0x788558`).

## Scrolling brings lines back

Every scroll call restores the shown lines to full alpha and restarts their times from the frame's:
`ScrollUp` at the top, `ScrollDown` at the bottom and a `SetScrollFromBottom` to the current line
re-arm every shown line in place; any scroll that moves lays the lines out again, re-arming each
unless the frame is at the bottom. A hidden line comes back, since nothing frees a faded record.
The mouse plays no part.

Addresses: `0x788b80`, `0x788baf`, `0x788bb3`, `0x788bc0`, `0x788bc9`, `0x788bdb`, `0x788af0`, `0x788aff`, `0x788b09`, `0x788610`, `0x788626`, `0x78863e`, `0x788650`, `0x788666`, `0x788700`, `0x788739`, `0x788750`, `0x7888be`
Confidence: verified
Proof: one
Test: none

`0x788b80` sets each shown record's alpha to 0xff (`0x788baf`), `+0x8` to 1 (`0x788bb3`), copies
`+0x364` and `+0x368` into it (`0x788bc0`, `0x788bc9`) and shows its node (`0x788bdb`).
`0x788af0(node, record, force)` does the same unless `force` is 0 and `+0x354` is set (`0x788aff`).
`0x788610` (`ScrollUp`) calls `0x788b80` at the top (`0x788626`), else `0x788750` (`0x78863e`);
`0x788650` and `0x788700` likewise; `0x788750` calls `0x788af0` (`0x7888be`).

## Paging

`PageUp` scrolls up one line as many times as lines are shown and then back down one, so a page
moves one line fewer than are shown. A step that cannot move, at the top or with no lines, ends
the page at once with no step back. With no lines shown the page is the step back alone: `PageUp`
moves one line down. `PageDown` is the mirror.

Addresses: `0x7885b0`, `0x7885b4`, `0x7885c3`, `0x7885ca`, `0x7885d4`, `0x7885e0`, `0x7885f3`, `0x7885fa`, `0x788604`, `0x788610`, `0x788619`, `0x78862b`, `0x788643`, `0x792d30`, `0x792e00`
Confidence: verified
Proof: one
Test: none

`0x7885b0` loads `+0x34c` (`0x7885b4`) and, while it counts down from nonzero, calls `0x788610`
(`0x7885c3`) and returns when that answers 0 (`0x7885ca`); after the loop it jumps to `0x788650`
(`0x7885d4`). `0x7885e0` swaps the two (`0x7885f3`, `0x7885fa`, `0x788604`). `0x788610` answers 0
when empty (`0x788619`) or at the top (`0x78862b`) and 1 after moving (`0x788643`). `PageUp`
(`0x792d30`) and `PageDown` (`0x792e00`) call them.

## GetFading and SetFading

`GetFading` returns the number 1 when fading is on and nil when it is off, for both types.
`SetFading` returns nothing.

Addresses: `0x793a40`, `0x793ac5`, `0x793ada`, `0x793ae6`, `0x7939a0`, `0x795170`, `0x7950d0`
Confidence: verified
Proof: one
Test: none

`0x793a40` reads `+0x360` (`0x793ac5`) and pushes 1.0 (`0x793ada`) or nil (`0x793ae6`);
`0x795170` reads the message frame's `+0x348` the same way.

## A message frame's AddMessage

The message frame's `AddMessage(text, r, g, b, a)` takes an alpha where the scrolling frame takes
an id: argument 6, clamped to 0 to 1, default 1. Without r, g and b the colour is opaque white, not
the font's. The line is queued and placed at the next update. Its fade divides the time left by
the frame's fade time like the scrolling frame's.

Addresses: `0x795590`, `0x795625`, `0x79564b`, `0x795658`, `0x795752`, `0x795769`, `0x795825`, `0x785d00`, `0x786200`, `0x786214`, `0x78625a`, `0x78635e`, `0x786376`
Confidence: verified
Proof: one
Test: none

`0x795590` returns for a missing or empty text (`0x795625` to `0x79564b`), starts the colour at
0xffffffff (`0x795658`) and the alpha at 1.0 (`0x795752`), reads argument 6 (`0x795769`) and
calls slot 0x90, `0x785d00`, with the text and colour (`0x795825`). `0x786200` (slot 14) places the
`+0x35c` queued lines through `0x7863b0` (`0x78625a`) and fades with `+0x350` (`0x78635e`) into
byte `+0xb` (`0x786376`).

## Lua methods

`ScrollingMessageFrame` has 40 methods at `0x87b5c0`, registered by `0x794190`; `MessageFrame`
has 26 at `0x87b960`, registered by `0x795b70`. Each type then asks the `Frame` methods. Their font
methods call the shared font functions (`fontstring`, Lua methods) on the frame's font.

Addresses: `0x87b5c0`, `0x794190`, `0x7941c0`, `0x87b960`, `0x795b70`, `0x795ba0`, `0x791d00`, `0x791dc0`, `0x791e80`, `0x791f40`, `0x792000`, `0x7920c0`, `0x792180`, `0x792240`, `0x792300`, `0x7923c0`, `0x792480`, `0x792540`, `0x792600`, `0x7926c0`, `0x792780`, `0x792840`, `0x792900`, `0x792b90`, `0x792c60`, `0x792d30`, `0x792e00`, `0x792ed0`, `0x792fa0`, `0x793070`, `0x793130`, `0x7931f0`, `0x7932b0`, `0x7934d0`, `0x793590`, `0x793650`, `0x793720`, `0x7937e0`, `0x7938a0`, `0x7939a0`, `0x793a40`, `0x793b00`, `0x793bf0`, `0x793cb0`, `0x793da0`, `0x793e60`, `0x7942b0`, `0x794370`, `0x794450`, `0x794510`, `0x7945d0`, `0x794690`, `0x794750`, `0x794810`, `0x7948d0`, `0x794990`, `0x794a50`, `0x794b10`, `0x794bd0`, `0x794c90`, `0x794d50`, `0x794e10`, `0x794ed0`, `0x794ff0`, `0x7950d0`, `0x795170`, `0x795230`, `0x795320`, `0x7953e0`, `0x7954d0`, `0x795590`, `0x795840`
Confidence: verified
Proof: one
Test: none

The tables are pairs of a name and a function: from `SetFontObject` (`0x791d00`) to `Clear`
(`0x793e60`), and from `SetFontObject` (`0x7942b0`) to `Clear` (`0x795840`). The registrars call
`0x701d80` with 0x28 and 0x1a. `0x7941c0` and `0x795ba0`, slot 2 of each vtable, look in the type's
hash and then call `0x778590`. The scroll methods call `0x788610`, `0x788650`, `0x7885b0`,
`0x7885e0`, `0x788690`, `0x7886d0` and `0x788700`.
