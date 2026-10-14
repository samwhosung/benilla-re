# ui: EditBox

How an `EditBox` holds the keyboard focus, which keys it takes and what each does, how text gets
in and what limits it, the history, the caret, and the scripts it runs.

A `CSimpleEditBox` is a frame (`frames`) of 0x448 bytes with the vtable `0x81c910`. Its fields are
the flags `+0x318` (bit 0 `autoFocus`, 1 `multiLine`, 2 `numeric`, 3 `password`, 4
`ignoreArrows`), the dirty bits `+0x31c` (1 text, 2 selection, 4 cursor), the text's context word
`+0x320`, its own font `+0x324`, the label `FontString` `+0x328`, the text `+0x32c` and its byte
classes `+0x330`, the length `+0x338`, the byte limit `+0x33c` (−1 for none) and letter limit
`+0x340` (0 for none), the visible window `+0x348` and `+0x34c`, the selection `+0x35c` to
`+0x360`, the caret texture `+0x368`, the cursor `+0x36c`, the blink period `+0x370` and the
history. The global `0xcf4dc8` is the box with the focus.

Roots: `0x7799b0`, `0x799ab0`

## Construction

The constructor enables character, key and mouse input for the box whatever scripts it has, sets
`autoFocus` and nothing else, gives the box its own font with `LEFT` justify, makes a label that
follows it, sets single-line, makes three selection textures in ARTWORK coloured 0xff606060 and a
caret in OVERLAY coloured white, hidden, and sets the blink period to 0.5 seconds.

Addresses: `0x7799b0`, `0x779a19`, `0x779a2e`, `0x779bd2`, `0x779c0b`, `0x779c27`, `0x779c2f`, `0x779c5d`, `0x779c81`, `0x779ca3`, `0x779ccc`, `0x779ced`, `0x779cf8`, `0x779d03`, `0x81c910`
Confidence: verified
Proof: one
Test: none

`0x779a19` stores 0.5 at `+0x370`; `0x779a2e` stores 1 at `+0x318`. The font at `+0x324` is built
with `0x783a40`; `0x779bd2` sets its justify word to `(x & ~6) | 1` with flag 0x202.
`0x779c0b` builds the label with `0x770d30(this, 2, 1)` and `0x779c27` links it to the font;
`0x779c2f` calls `0x77a4a0(0)`. `0x779c5d` creates three textures with `0x76fc40(this, 2, 0)`,
coloured by `0x77e330` (`0x779c81`); `0x779ca3` creates the caret with `(this, 3, 1)`, hidden, and
`0x779ccc` colours it 0xffffffff. `0x76af00` enables kinds 0, 1 and 2 (`0x779ced` to `0x779d03`).
The factory allocates 0x448 bytes.

## Keyboard focus

One edit box at a time holds the focus. Focusing requires the box to be visible and not already
focused: the old box loses it first, running `OnEditFocusLost`, then the new one gains it,
running `OnEditFocusGained`. Clearing acts only on the box that holds it. Showing a box focuses it
when nothing holds the focus and `autoFocus` is set; hiding a box clears its focus. A focus change
switches the input method on or off unless the box is `numeric` or `password`, and repaints the
caret.

Addresses: `0x77e3d0`, `0x77e3db`, `0x77e3e5`, `0x77e3ed`, `0x77e3f6`, `0x77e410`, `0x77e418`, `0x77af50`, `0x77af63`, `0x77af6c`, `0x77af7f`, `0x77afa3`, `0x77afa8`, `0x77a750`, `0x77a75f`, `0x77a76d`, `0x77a780`, `0x77a78b`, `0xcf4dc8`
Confidence: verified
Proof: one
Test: none

`0x77e3d0` returns when `+0xd4` is 0 (`0x77e3db`) or the box is `[0xcf4dc8]` (`0x77e3e5`); it calls
`0x77af50(0)` on the old box (`0x77e3ed`), stores the new (`0x77e3f6`) and calls `0x77af50(1)`.
`0x77e410` compares with `[0xcf4dc8]` (`0x77e418`). `0x77af50` tests `+0x318 & 0xc` (`0x77af63`)
before `0x42c850`, and on loss calls `0x77e3b0` and zeroes `+0x3cc` (`0x77af7f`); it runs the
script slot `+0x438` or `+0x440` (`0x77afa3`) and sets dirty bit 4 (`0x77afa8`). `0x77a750`
(slot 12) calls `0x76b260`, then jumps to `0x77e3d0` when `0xcf4dc8` is 0 and bit 0 is set;
`0x77a780` (slot 13) jumps to `0x77e410`.

## Which box takes a key

The frame core offers keys and characters to frames by stratum and level (`input`); an edit box
takes one only when it is visible and either holds the focus or nothing does and it has
`autoFocus`, which then focuses it. A box that takes a key consumes it, except PRINTSCREEN
(0x212), which it always declines, and, with `ignoreArrows`, an arrow key without Alt. A mouse
press the frame's own press handling leaves to the box focuses it whatever `autoFocus` says.

Addresses: `0x77b160`, `0x77b16c`, `0x77b17f`, `0x77b18e`, `0x77b1b3`, `0x77b1c7`, `0x77b1db`, `0x77b215`, `0x77a900`, `0x77a92b`, `0x77b800`, `0x77b881`
Confidence: verified
Proof: one
Test: none

`0x77b160` (slot 24) returns 0 for `+0xd4` clear (`0x77b16c`) and for code 0x212 (`0x77b17f`);
with bit 0x10 (`0x77b18e`) it returns for 0x204 to 0x207 unless `0x41f8f0(2)`, Alt, is down
(`0x77b1b3`). With `0xcf4dc8` empty and bit 0 set it focuses the box (`0x77b1db`); otherwise it
returns 0 unless the box holds the focus (`0x77b215`). Every later path returns 1. `0x77a900`
(slot 23) repeats the guard (`0x77a92b`). `0x77b800` (slot 26) returns when `0x76bf70` answers
nonzero and otherwise calls `0x77e3d0` on every path (`0x77b881`).

## Keys

Enter in a multi-line box inserts a newline; otherwise it runs `OnEnterPressed`. Escape runs
`OnEscapePressed` and Tab `OnTabPressed`; none of them changes the focus. Each then notifies the
box's C++ listener. Left and Right move one character, or one word with Ctrl; Home and End go to
the line's edge, or the text's with Ctrl; Shift extends the selection. Up and Down move a line in
a multi-line box and walk the history otherwise. Backspace and Delete remove the selection or one
character, a word with Ctrl. Ctrl+Insert copies, Shift+Insert pastes, Shift+Delete cuts.

Addresses: `0x77b3cb`, `0x77b3de`, `0x77b3e7`, `0x77b405`, `0x77b40c`, `0x77b422`, `0x77b438`, `0x77b44a`, `0x77b459`, `0x77b46d`, `0x77b622`, `0x77b5f6`, `0x77b499`, `0x77b4d9`, `0x77b64e`, `0x77b675`, `0x77b5c6`, `0x77b56c`, `0x77b519`, `0x77c3c0`, `0x77c310`, `0x77c3f0`, `0x77b730`, `0x77c6b0`, `0x77c7a0`, `0x77c8c0`, `0x77c980`, `0x77ca60`, `0xcf4dc4`
Confidence: verified
Proof: one
Test: none

Escape (0x200) runs `+0x400` (`0x77b3de`) and `0x77b730(1)`. 0x201 to 0x20b index the table at
0x77b704 (`0x77b405`): Enter inserts `"\n"` with bit 2 (`0x77b422`), else sets `0xcf4dc4` around
`+0x3f8` and `0x77b730(0)` (`0x77b438` to `0x77b459`); Tab runs `+0x410` and `0x77b730(3)`. The
arrows (`0x77b622`, `0x77b5f6`) call `0x77c870`, `0x77c750`, or `0x77c8c0` and `0x77c7a0` with Ctrl,
passing Shift; `0x77c6b0` moves by characters, `0x77c7a0` over non-spaces then spaces. Home and End
(`0x77b499`, `0x77b4d9`) call `0x77c980` (back to a `"\n"`) and `0x77c9f0`, or `0x77ca60` and
`0x77cac0`. Up and Down test bit 2 (`0x77b64e`, `0x77b675`). Backspace calls `0x77c3c0`, or
`0x77c3f0` with Ctrl; Delete `0x77c2e0`, or `0x77c310` with Ctrl.

## Word steps

A word step tests characters with `iswspace`. Ctrl+Right moves over non-spaces, then over spaces;
Ctrl+Left moves back over spaces, then over non-spaces, testing the character decoded from the
byte before the cursor. Ctrl+Delete removes the spaces after the cursor, then the word;
Ctrl+Backspace removes the spaces before it, then the word, decoding each from the start of the
character before the cursor.

Addresses: `0x77c7a0`, `0x77c7d3`, `0x77c81b`, `0x77c8c0`, `0x77c8e7`, `0x77c924`, `0x77c310`, `0x77c351`, `0x77c393`, `0x77c3f0`, `0x77c423`, `0x77c443`, `0x77c485`, `0x77bd30`, `0x77c6b0`, `0x77c280`, `0x74006f`
Confidence: verified
Proof: one
Test: none

`0x77c7a0` steps forward with `0x77c6b0` until `_iswspace` (`0x74006f`) holds (`0x77c7d3`), then
while it holds (`0x77c81b`). `0x77c8c0` decodes `sgetu8` at `+0x32c + cursor − 1` and steps back
with `0x77c6b0(−1)` while `_iswspace` holds (`0x77c8e7`), then while it fails (`0x77c924`); without
Shift it empties a selection. `0x77c310` deletes forward with `0x77c280` while `_iswspace` holds
(`0x77c351`), then while it fails (`0x77c393`). `0x77c3f0` deletes backward the same way
(`0x77c443`, `0x77c485`), decoding at `0x77bd30(cursor)` (`0x77c423`), the nearest index below
with a nonzero class entry.

## Ctrl letter keys

With Ctrl held, letters act in the Emacs way: A selects all, B and F move back and forward a
character, D deletes forward, K deletes to the end and U to the start, W deletes a word back, P
and N walk the history back and forward, C copies, X cuts, and V pastes. Other letters do nothing.
A password box copies an empty string instead of its text.

Addresses: `0x77b1fb`, `0x77b20e`, `0x77b221`, `0x77b239`, `0x77b265`, `0x77b291`, `0x77b2b5`, `0x77b2d9`, `0x77b2f9`, `0x77b319`, `0x77b339`, `0x77b359`, `0x77b387`, `0x77b39b`, `0x77b3bb`, `0x77c2e0`, `0x77c4a0`, `0x77c4d0`, `0x77c3f0`, `0x77e1d0`, `0x77e250`, `0x882748`
Confidence: verified
Proof: one
Test: none

Codes 0x41 to 0x78 index the bytes at 0x77b6cc into the cases at 0x77b69c (`0x77b20e`); each case
tests `0x41f8f0(1)`. A calls `0x77cca0(0, −1)` (`0x77b239`); F `0x77c750` (`0x77b265`); B
`0x77c870` (`0x77b291`); D `0x77c2e0` (`0x77b2b5`), which deletes the selection or `0x77c280(1)`;
W `0x77c3f0`; U `0x77c4a0`, `0x77c280` of minus the letters before the cursor; K `0x77c4d0`; P
`0x77d030`; N `0x77cfd0`; C and X `0x77e1d0` (`0x77b387`), X then `0x77cd70` (`0x77b39b`); V
`0x77e250` (`0x77b3bb`). `0x77e1d0` tests bit 3 and then passes the empty string at `0x882748`
to `0x7a78c0`.

## Typed characters

A typed character is inserted as UTF-8 with the scripts run. A `|` is inserted as `||`, so typed
text never starts an escape. A tab character inserts four spaces. A newline is inserted only in a
multi-line box; every other control character and DEL are dropped.

Addresses: `0x77c200`, `0x77c20c`, `0x77c216`, `0x77c228`, `0x77c234`, `0x77c242`, `0x77c254`, `0x77c263`, `0x77c274`, `0x879cb0`, `0x879cac`
Confidence: verified
Proof: one
Test: none

`0x77c200(ch)` sends a character below 0x20 or equal to 0x7f to the control path (`0x77c20c`):
9 inserts the four spaces at `0x879cb0` (`0x77c228`); 0xa continues only with bit 2
(`0x77c234`). 0x7c inserts `||` from `0x879cac` (`0x77c254`). Anything else is encoded by
`0x41abb0` (`0x77c263`) and inserted (`0x77c274`). Each insert passes 1 for the scripts.

## Inserting text

An insertion inside an escape or a link is refused. Otherwise it first deletes the selection. In a
`numeric` box a text with any character outside `0` to `9` is then refused whole, so typing a
letter over a selection only deletes it. The text is spliced in at the cursor, which moves past
it. Then the byte limit and the letter limit each trim characters from the end. With scripts on,
it runs `OnChar` with the inserted text, and `OnSpacePressed` once when the text holds a space.

Addresses: `0x77bee0`, `0x77bef6`, `0x77bf0d`, `0x77bf23`, `0x77bf41`, `0x77bf6a`, `0x77c01c`, `0x77c022`, `0x77c033`, `0x77c03e`, `0x77c066`, `0x77c08d`, `0x77c0d0`, `0x77c101`, `0x77c13c`, `0x77c153`, `0x77c186`
Confidence: verified
Proof: one
Test: `ui::editbox_markup_insert_refusal_range`

`0x77bee0(text, word, scripts, compose)` refuses when the cursor's byte class is negative and
`0x77bd30` finds no nonnegative one (`0x77bf0d`); `0x77bf23` deletes the selection; `0x77bf41`
tests bit 2 and `0x77bf6a` each decoded character, skipped when `compose` is set. It sets dirty
bits 1 and 4 (`0x77c033`). With `+0x33c` not negative (`0x77c03e`) it calls `0x77c280(−1)` from the
end while the length exceeds it (`0x77c066`); with `+0x340` nonzero (`0x77c08d`) while the
letters exceed it (`0x77c0d0`). It stores the word at `+0x320` when that is 0 (`0x77c101`). With
`scripts` it runs slot `+0x180` with `"%s"` (`0x77c13c`) and `+0x408` at the first space
(`0x77c153`, `0x77c186`).

Bit-exact coverage: `ui::editbox_markup_insert_refusal_range` covers `0x77bee0` to `0x77bf0d`, `0x77c1b8` to `0x77c1be` of `0x77bee0`.

## A negative letter limit hangs

A negative letter limit is never met: the trim loop deletes backward from the end until the text
is empty, and an empty text still has more letters than the limit, so the next insertion of any
text never returns. That includes `SetText("")` on a box with text; on an empty box `SetText`
returns early, since the texts are equal.

Addresses: `0x77c08d`, `0x77c0d0`, `0x77c0e9`, `0x77c280`, `0x77c2a9`, `0x77bb30`, `0x77bbd7`, `0x799110`, `0x7991f2`, `0x77be4b`
Confidence: verified
Proof: one
Test: `ui::editbox_markup_has_no_inline_texture_escape`, `ui::editbox_markup_reachable_cursor_indices`, `ui::editbox_markup_step_consumes_whole_characters`, `ui::editbox_markup_step_law_hangs_on_a_trailing_skip_class`

`0x77c08d` skips only a limit of 0. The loop calls `0x77c280(−1)` and `0x77bc80` and repeats on
`jg` (`0x77c0e9`). At cursor 0, `0x77bb30` with −1 returns 0 at `0x77bbd7` (`cmp ecx,edi; jbe`),
so `0x77c510` deletes nothing. `SetMaxLetters` (`0x799110`) stores the truncated number as given
(`0x7991f2`). `0x77be00` returns when `0x64a480` finds the texts equal (`0x77be4b`), before
inserting.

Bit-exact coverage: `ui::editbox_markup_has_no_inline_texture_escape` covers `0x77bb30` to `0x77bb91`, `0x77bb98` to `0x77bbb5`, `0x77bbbe` to `0x77bbc9` of `0x77bb30`; `ui::editbox_markup_reachable_cursor_indices` covers the whole of `0x77bb30`; `ui::editbox_markup_step_consumes_whole_characters` covers `0x77bb30` to `0x77bb91`, `0x77bb98` to `0x77bbb5`, `0x77bbbe` to `0x77bbc9` of `0x77bb30`; `ui::editbox_markup_step_law_hangs_on_a_trailing_skip_class` covers `0x77bb30` to `0x77bbc9` of `0x77bb30`.

## Setting and reading text

`SetText` and `SetNumber` are the same: each takes a string or a number, which Lua turns into its
string, and raises a usage error otherwise. The box clears the selection, and when the text
differs it replaces it, without the scripts of typing, then runs `OnTextSet`. On a `numeric` box
a text with a non-digit, `SetNumber(0.5)` or `SetNumber(-1)` among them, empties the box, inserts
nothing and still runs `OnTextSet`. `Insert` inserts at
the cursor with them. Each passes the current context word, and the box keeps the word of its
first text. `GetText` returns the text and `GetNumber` its value as a number; both make the box's
word current (`luastate`, The context word).

Addresses: `0x7984c0`, `0x79854c`, `0x7985ab`, `0x798690`, `0x79871c`, `0x79877b`, `0x798400`, `0x79849e`, `0x7984b0`, `0x77be00`, `0x77be16`, `0x77be4b`, `0x77be56`, `0x77be66`, `0x77be7e`, `0x77bf6d`, `0x77bf76`, `0x7985c0`, `0x79865f`, `0x798790`, `0x798852`
Confidence: verified
Proof: one
Test: none

`0x7984c0` and `0x798690` test argument 2 with `0x6f3510` (`0x79854c`, `0x79871c`) and pass
`0x6f3690` of it with `[0xceeac0]` to `0x77be00` (`0x7985ab`, `0x79877b`); only their usage strings
differ. `0x798400` passes `(text, [0xceeac0], 1, 0)` (`0x79849e`, `0x7984b0`). `0x77be00` zeroes a
selection (`0x77be16`), returns when `0x64a480` finds the texts equal (`0x77be4b`), else empties
(`0x77be56`), inserts with scripts off (`0x77be66`) and runs `+0x420` (`0x77be7e`) whatever the
insert did; `0x77bee0` returns at its first character outside 0x30 to 0x39 (`0x77bf6d`, `0x77bf76`)
after the box was emptied. `0x7985c0` and `0x798790` copy `+0x320` into `0xceeac0` when it and
`0xceeac4` are nonzero (`0x79865f`); `0x798790` converts with `0x64aaa0` (`0x798852`).

## Limits from Lua

`SetMaxBytes(n)` keeps at most n − 1 bytes, room for the terminator, and n of 0 or less removes
the limit; `GetMaxBytes` returns the limit plus 1, so 0 when there is none. `SetMaxLetters(n)`
stores n as it is, 0 meaning none. Each takes exactly one argument, read as a number truncated
toward zero.

Addresses: `0x798f30`, `0x798fb7`, `0x79900d`, `0x799016`, `0x799022`, `0x799040`, `0x7990e9`, `0x799110`, `0x7991ed`
Confidence: verified
Proof: one
Test: none

`0x798f30` requires `0x6f3070` to count 2 (`0x798fb7`), truncates with `0x40a2b0` (`0x79900d`) and
stores n − 1 for n above 0 (`0x799016`), else −1 (`0x799022`). `0x799040` pushes `+0x33c` + 1
(`0x7990e9`). `0x799110` truncates the same way (`0x7991ed`).

## Text changes and their scripts

Each update flushes the dirty bits in order: a text change rebuilds the label's text and, in a
multi-line box, sets the box's height to the text's height plus the top and bottom insets; a
selection change redraws the highlight; a cursor change re-windows the text and places the caret,
running `OnCursorChanged` with x, y, width and height. `OnTextChanged` runs last, when the text
had changed, typing and `SetText` alike. A cursor leaving the visible window rebuilds the text
without `OnTextChanged`.

Addresses: `0x77a790`, `0x77a7a1`, `0x77d3e0`, `0x77d3f2`, `0x77d433`, `0x77d447`, `0x77d45e`, `0x77d475`, `0x77d498`, `0x77d4d0`, `0x77d863`, `0x77d872`, `0x77d88e`, `0x77d8ad`, `0x77da80`, `0x77de1b`
Confidence: verified
Proof: one
Test: `ui::glue_77a_77a790_fill_accumulate`

`0x77a790` (slot 14) calls `0x76b2c0` then `0x77d3e0` (`0x77a7a1`). `0x77d3e0` keeps bit 1 of
`+0x31c` in `edi` before it sets bit 1 for a cursor outside `[+0x348, +0x348 + +0x34c]`
(`0x77d433`), then calls `0x77d4d0`, `0x77d950` and `0x77da80` for bits 1, 2 and 4, and with the
kept bit runs `+0x418` (`0x77d498`). `0x77d4d0` tests bit 2 (`0x77d863`), takes the label's
height from `0x7729b0`, or `0x7727b0(1)` when it is 0, adds `+0x3f0` and `+0x3e8`, and calls
layout slot 0x18 (`0x77d8ad`). `0x77da80` runs `+0x428` through `0x7026f0` (`0x77de1b`).

## The caret

The caret is a solid quad 4 script units wide and as tall as a line. It is white until the box's
font changes colour, and then takes that colour.
It shows only in the box with the focus, and only while the cursor is inside the visible window.
There it toggles each time more than the blink period has passed, 0.5 seconds by default, so it
blinks once a second; a period of 0 stops the blinking.

Addresses: `0x77ba2d`, `0x77ba39`, `0x77ba42`, `0x77ba67`, `0x77a7a6`, `0x77a7b7`, `0x77a7cb`, `0x77a7dc`, `0x77a7ef`, `0x77a7f1`, `0x77a80e`, `0x77a82c`, `0x77de23`, `0x77e2a0`, `0x77e2cc`, `0x77e310`, `0x80306c`
Confidence: verified
Proof: one
Test: none

`0x77ba39` divides 4.0 (`0x80306c`) by `[0x832a4c]·1024` and `0x77ba42` converts the result into
layout units for the caret's width (`0x77ba67`). `0x77a7a6` compares `+0x370` with 0 and skips on
equal (`0x77a7b7`); the cursor must lie in the window (`0x77a7cb`); `0x77a7dc` adds the frame
time to `+0x374`; only the focused box (`0x77a7ef`) compares it with the period (`0x77a7f1`,
skipping on less or equal) and flips the caret, zeroing the sum (`0x77a80e`, `0x77a82c`).
`0x77de23` shows the caret only for `[0xcf4dc8]`. The font's listener `0x77e2a0` passes the
font's colour to `0x77e310` for a colour change (`0x77e2cc`).

## Selection

The selection is two byte positions. `HighlightText(start, end)` selects from start to end,
where an end past the text or before the start means the end of the text, so `(0, -1)` selects
all. Moving the cursor without Shift clears the selection. The highlight is drawn whether the box
has the focus or not, and losing the focus leaves the selection.

Addresses: `0x77cca0`, `0x77ccaf`, `0x77ccbe`, `0x77c70b`, `0x77d950`, `0x77de70`, `0x798870`, `0x798951`
Confidence: verified
Proof: one
Test: `ui::glue_77de70`

`0x77cca0` clamps the start to the length (`0x77ccaf`), the end likewise, and takes the length for
an end below the start (`0x77ccbe`). `0x77c70b` zeroes both ends without Shift. Neither `0x77d950`
nor `0x77de70` reads `0xcf4dc8`, and `0x77af50` does not write the selection. `0x798870` passes
two truncated numbers to `0x77cca0` (`0x798951`).

## History

A box with history lines keeps a ring of that many lines. `AddHistoryLine` writes at the ring's
position and advances it, keeping empty and repeated lines. The same position is the browsing
cursor: Up or Ctrl+P sets the text to the nearest stored line before it, wrapping, and Down or
Ctrl+N to the nearest after it, and the next added line overwrites the slot the browsing reached.
No draft is kept.

Addresses: `0x77cf40`, `0x77cf4e`, `0x77cf7b`, `0x77cfab`, `0x77cfb8`, `0x77cfd0`, `0x77cffb`, `0x77d014`, `0x77d023`, `0x77d030`, `0x77d087`, `0x77d0aa`, `0x77d0be`, `0x77cd90`, `0x798960`, `0x799450`
Confidence: verified
Proof: one
Test: none

`0x77cf40` returns for a count `+0x388` of 0 (`0x77cf4e`), reallocates the slot at `+0x38c`
(`0x77cf7b`), copies the line (`0x77cfab`) and stores `(+0x38c + 1) mod count` (`0x77cfb8`).
`0x77cfd0` scans forward from `+0x38c + 1` for a nonempty slot (`0x77cffb`), stores its index
(`0x77d014`) and calls `0x77be00` (`0x77d023`); `0x77d030` scans backward from `+0x38c − 1`
(`0x77d087`, `0x77d0aa`, `0x77d0be`). `0x799450` passes a positive count to `0x77cd90`.

## Multi-line and the label's justify

`SetMultiLine` sets the flag and fixes the label's vertical justify, `TOP` for multi-line and
`MIDDLE` otherwise, cutting it off from the font's justify for good; it also switches the label's
line break flags and lays the text out again. The constructor makes every box single-line this
way, so an edit box's label never takes its vertical justify from a font. `SetTextInsets(left,
right, top, bottom)` stores the insets and lays out again.

Addresses: `0x77a4a0`, `0x77a4b3`, `0x77a509`, `0x77a517`, `0x77a599`, `0x77a5a7`, `0x77a5e2`, `0x77a58f`, `0x77a63f`, `0x77a6e0`, `0x77b8c0`
Confidence: verified
Proof: one
Test: `ui::glue_77a_77b8c0_fill_edges`

`0x77a4a0` sets or clears bit 2 (`0x77a4b3`), clears bits 3 to 5 of the label's inherit mask
`+0x124` on both arms (`0x77a509`, `0x77a599`), and sets `+0x120`'s vertical bits to 8 (`0x77a517`)
or 0x10 (`0x77a5a7`); multi-line clears bit 0x4000 and sets 0x1000 (`0x77a58f`), single-line the
reverse (`0x77a5e2`). It ends with `0x77b8c0` (`0x77a63f`). `0x77a6e0` stores its four arguments
at `+0x3ec`, `+0x3f4`, `+0x3f0` and `+0x3e8`.

## Lua methods

`EditBox` has 48 methods at `0x87bb68`, registered by `0x799ab0`. The font methods act on the
box's own font `+0x324` through the shared font functions. There is no method to read or set the
cursor.

Addresses: `0x87bb68`, `0x799ab0`, `0x799ae0`, `0x797090`, `0x797150`, `0x797210`, `0x7972d0`, `0x797390`, `0x797450`, `0x797510`, `0x7975d0`, `0x797690`, `0x797750`, `0x797810`, `0x7978d0`, `0x797990`, `0x797a50`, `0x797b10`, `0x797bd0`, `0x797c90`, `0x797d40`, `0x797e00`, `0x797eb0`, `0x797f70`, `0x798020`, `0x7980e0`, `0x798190`, `0x798250`, `0x798340`, `0x798400`, `0x7984c0`, `0x7985c0`, `0x798690`, `0x798790`, `0x798870`, `0x798960`, `0x798a60`, `0x798c30`, `0x798d90`, `0x798e60`, `0x798f30`, `0x799040`, `0x799110`, `0x799200`, `0x7992c0`, `0x799390`, `0x799450`, `0x799550`, `0x799610`, `0x7996e0`, `0x799790`, `0x77a650`, `0x77a680`, `0x77a6b0`
Confidence: verified
Proof: one
Test: `ui::luabind_798a60_set_text_insets`, `ui::luabind_798c30_get_text_insets`

The table holds 48 pairs from `SetFontObject` (`0x797090`) to `GetAltArrowKeyMode` (`0x799790`);
`0x799ab0` calls `0x701d80` with 0x30. `0x799ae0` (slot 2) looks in the `EditBox` hash, then calls
`0x778590`. The font methods load `+0x324` and call `0x79ef10`, `0x79f210` and their siblings.
`SetFocus` and `ClearFocus` call `0x77e3d0` and `0x77e410`; `SetNumeric`, `SetPassword` and
`SetAltArrowKeyMode` set bits 4, 8 and 0x10 through `0x77a650`, `0x77a680` and `0x77a6b0`. No method
calls `0x77e360`, the cursor setter.

Bit-exact coverage: `ui::luabind_798a60_set_text_insets` covers 4 ranges from `0x798b49` to `0x798bd9` of `0x798a60`; `ui::luabind_798c30_get_text_insets` covers 4 ranges from `0x798cea` to `0x798d68` of `0x798c30`.
