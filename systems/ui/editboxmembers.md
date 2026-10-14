# ui: edit box members

The smaller members of `CSimpleEditBox` (`editbox`): its destructor, the history array, the cursor and selection setters, the
paste loop, the key-listener and focus slots, the line lookup, and the input-method candidate window. The object keeps its text at `+0x32c`
with a class dword per byte at `+0x330`, its line starts at `+0x380` with their count `+0x37c`, the cursor `+0x36c` and the selection
`+0x35c` to `+0x360`; the history is a three-dword array at `+0x390` (capacity, count, records of two dwords) with the line limit `+0x388`
and the position `+0x38c`.

Roots: `0x779df0`, `0x77a8d0`, `0x77a960`, `0x77a9f0`, `0x77ad10`, `0x77b7c0`, `0x77b890`, `0x77cd90`

## Destruction

The destructor body `0x779df0` stores the three vtables back (`0x81c910`, `0x81c8e0` at `+0x24`, `0x81c8dc` at `+0x314`), clears the
focus (`0x77e410`), empties the history (`0x77cd90(0)`), deletes the own font `+0x324` and the label `+0x328` through their virtual
destructors, frees the text `+0x32c` and the classes `+0x330`, the password copy `+0x334` when set, deletes the caret `+0x368` and the
candidate frame `+0x3e0`, releases the ten script slots `+0x3f8` to `+0x440`, frees the line starts `+0x380` and ends in
`CSimpleFrame::~CSimpleFrame`.

Addresses: `0x779df0`, `0x779e0a`, `0x779e13`, `0x779e48`, `0x779e60`, `0x779e7b`, `0x779ea1`, `0x779eab`, `0x779f2e`, `0x779f81`, `0x779f99`, `0x779fa2`, `0x879a90`, `0x77e410`, `0x77cd90`, `0x702670`, `0x7693b0`
Confidence: verified
Proof: one
Test: none

`0x779dc0` calls it and frees the box when its flag is set (`0x779dc6`). The text is freed with `SMemFree` at `0x779e48` (line 0x7f), the classes at
`0x779e60` (0x80), the password copy at `0x779e7b` (0x81). The ten slots are released by `0x702670` at `0x779eab` to `0x779f2b`, each with its
`+0x8` stride. The history loop at `0x779f40` frees nothing after `0x77cd90(0)` has zeroed the count.

## History array

`SetHistoryLines(n)` sets the line limit through `0x77cd90(n)`. A different count than the stored one frees the records the new size drops (the string
of each, through `0x77cf20`), reallocates the array to `n` records of 8 bytes, zero-fills the new ones and stores the count. A reallocation that
fails allocates a new array and copies the smaller count of records, duplicating each string. A count of 0 frees every
string and the array and zeroes capacity, count and data. It then stores `n` at `+0x388` and clamps the position `+0x38c` to `n − 1`
(0 for none).

Addresses: `0x77cd90`, `0x77cdb5`, `0x77cdcf`, `0x77ce11`, `0x77ce2a`, `0x77ce79`, `0x77ceb2`, `0x77cee0`, `0x77cf20`, `0x77e440`, `0x646320`, `0x6462e0`, `0x64a620`, `0x879ad4`, `0x879af8`
Confidence: verified
Proof: one
Test: none

`0x77cd90` compares `n` with the count at `+0x394` (`0x77cdaa`). For `n = 0` it calls `0x77e440` (free every record string, then the array; line 0x17f
and tag `0x879ad4`). A smaller `n` calls `0x77cf20` on each dropped record (`0x77cde4`). The reallocation is `0x646320` with alignment 0x10
(`0x77ce11`); the fallback `0x6462e0` (`0x77ce2a`) copies with the string duplicate `0x64a620` at line 0x17e (`0x77ce79`) and frees the old block (`0x77ceb2`).
`0x77cee0` stores `n` and tests the position `+0x38c` against it. `0x799450` is the Lua caller.

## Cursor and selection setters

`0x77e360(pos)` sets the cursor to `pos` clamped to 0 and the length `+0x338` and marks the cursor dirty (`+0x31c` bit 2). `0x77ccf0` sets both ends
of the selection to the cursor. `0x77cd10(d)` moves the selection with a cursor step of `d`: for a negative step the start `+0x35c` takes the new
position when it lies below it, else the end `+0x360`; for a positive step the end takes it when it lies above, else the start; the selection is
marked dirty (bit 1). `0x77cac0(extend)` steps the cursor forward one character with `0x77c6b0(1, extend)` until it reaches the length, and with `extend` 0 clears a
nonempty selection. `0x77cc60(a)` and `0x77cc80(a)` call `0x77cb20` with +1 and −1 (a line down and up).

Addresses: `0x77e360`, `0x77e36e`, `0x77e37a`, `0x77e380`, `0x77ccf0`, `0x77cd10`, `0x77cd26`, `0x77cd3f`, `0x77cd58`, `0x77cac0`, `0x77cad9`, `0x77cb04`, `0x77cc60`, `0x77cc80`, `0x77c6b0`, `0x77cb20`
Confidence: verified
Proof: one
Test: none

`0x77e360` tests the argument below 0 first (`0x77e368`), then above `+0x338` (`0x77e376`). `0x77cd10` returns at once for 0 (`0x77cd18`).

## Text capacity and lines

`0x77bd90(n)` makes room for `n + 1` bytes: when that passes the capacity `+0x344` it rounds up to a multiple of 32, and reallocates the text `+0x32c` (bytes)
and the classes `+0x330` (four bytes each). `0x77bd50(pos)` returns the index of the line holding the byte position, walking the line starts from
the second one while `pos` is at or past the next start and the index is below the line count less 2. `0x77e250` pastes: it takes the clipboard text
(`0x7a77b0`), decodes it one UTF-8 character at a time with `0x41aab0` and inserts each with `0x77c200`, then frees the text (`0x7a78a0`).

Addresses: `0x77bd90`, `0x77bd9f`, `0x77bdab`, `0x77bdc4`, `0x77bdec`, `0x77bd50`, `0x77bd6b`, `0x77bd7f`, `0x77e250`, `0x77e258`, `0x77e27d`, `0x77e290`, `0x7a77b0`, `0x7a78a0`, `0x41aab0`, `0x77c200`
Confidence: verified
Proof: one
Test: none

`0x77bd90` reallocates with line 0x469 for the text and 0x46a for the classes (`shl ecx,2`). A null clipboard string ends `0x77e250` (`0x77e25f`).

## Slots and listeners

`0x77b730(kind)` calls the listener stored for a key kind: when `[this + kind·8 + 0x3a0]` is nonzero it builds a record (vtable `0x81c898`, 0, the word
at `[this + kind·8 + 0x39c]`, the box) and calls the listener's slot 4 with it. `0x77b7c0`, a vtable slot, answers 0 when the box is hidden;
with no focused box (`0xcf4dc8` zero) and autoFocus set it takes the focus (`0x77e3d0`) and answers 1; else whether it is the focused box (keyboard intake). `0x77b890`, the mouse-up slot,
runs the frame's and, when that answers 0 with the drag flag `+0x364` set, clears it. `0x77a8d0` runs `0x76b660` and lays out; `0x77a710(a, b, c, d)` writes the four insets through its pointers, then lays out.

Addresses: `0x77b730`, `0x77b73b`, `0x77b75a`, `0x77b767`, `0x77b7c0`, `0x77b7cf`, `0x77b7e1`, `0x77b890`, `0x77b89a`, `0x77b8ad`, `0x77a8d0`, `0x77a8e0`, `0x77a8e7`, `0x77a710`, `0x77a73f`, `0x77e3d0`, `0x77b8c0`, `0x76b660`, `0x76c040`, `0x81c898`, `0xcf4dc8`
Confidence: verified
Proof: one
Test: none

`0x77b730` is called by `0x77b160` for Escape, the Enter kinds and Tab. `0x77a8d0` is stored in the box's layout vtable at `0x81c958` and `0x77b7c0` and `0x77b890` at
`0x81c974` and `0x81c97c`. `0x77a710` is called by the Lua `GetTextInsets` at `0x798ce5`.

## Candidate window

The input method's candidate list is shown in a message frame the box creates once (`0x77a9f0`): `CSimpleMessageFrame`'s constructor (`0x785640`) on a 0x378-byte
block, stratum 8, following the box's own font with `LEFT` justify and a width of ten label line heights (`10.0` at `0x801624`, slot 5), and two textures: a background
`0xff606060` over all its points and a row highlight `0xff808080`. `0x77ad10(hwnd)` fills it from `0x42c500`; a failure answers 0.
The frame's height becomes `(page + 1) · line + 0.1 · line`, the highlight spans row `selection mod page`, a first line `> %d/%d` (selection + 1, total) is added, and
each candidate follows as `%d: %s`, last to first; an empty candidate is a space.

Addresses: `0x77a9f0`, `0x77aa26`, `0x77aa33`, `0x77aa46`, `0x77aa61`, `0x77aa9d`, `0x77aaac`, `0x77aae3`, `0x77ab42`, `0x77ab8d`, `0x77ad10`, `0x77ad41`, `0x77ad6b`, `0x77ada1`, `0x77adb6`, `0x77addd`, `0x77ade8`, `0x77ae28`, `0x77ae70`, `0x77aecf`, `0x77a960`, `0x833394`, `0x42c500`, `0x785640`, `0x76a470`, `0x770c60`, `0x801620`, `0x801624`, `0x879c9c`, `0x879ca4`
Confidence: verified
Proof: one
Test: `ui::glue_77a_77a9f0_height_x10`, `ui::glue_77a_77ad10_row_geom`

`ui::glue_77a_77a9f0_height_x10` runs the `fmul` of the label height by 10.0 at `0x77aa9d` as a copied-byte island with an appended `fstp`; `ui::glue_77a_77ad10_row_geom` the three float regions of
`0x77ad10` (`0x77ada1` to `0x77adb3`, `0x77ade8` to `0x77adf0`, `0x77ae28` to `0x77ae30`) the same way. `0x77ad10` is called by `0x77afc0` at `0x77b0c1`.
`0x77a960` creates the composition texture `+0x3d4` once: colour `0xff202010`, blend mode 3, sized to the label's line height through slot 6.
`0x77e390` and `0x77e3b0` store 1 or 0 at `+0xd0` of the frame `+0x3e0` and jump to its slot 0x88 or 0x84.

## Font follower slots

The follower tables `0x81c8bc` and `0x81cb20` (an edit box's own font among them) hold small slots, each used by two tables. `0x77e4b0(block)`
merges a font block into the block at `this + 0x14` under the mask at `this + 8` (`FontBlockMergeMasked`) and then calls slot 5 of the object at `this − 0x24`. `0x77e530` calls
`0x784180` on the font's followers and then slot 0 of the object at `+0x7c` with the object and the changed word `+0x38`. `0x77e4a0` returns the dword at `+0x68`. `0x77e4e0(p)`
answers whether `p` equals the dword at `0xcf4ea8`.

Addresses: `0x77e4b0`, `0x77e4cc`, `0x77e4d7`, `0x77e530`, `0x77e537`, `0x77e543`, `0x77e4a0`, `0x77e4e0`, `0x770910`, `0x784180`, `0x81c8bc`, `0x81cb20`, `0xcf4ea8`
Confidence: verified
Proof: one
Test: none

`0x77e4b0` pushes the mask `[esi+8]` and the destination (`0x77e4c7`, `0x77e4cb`) before `0x770910` and calls `[[esi-0x24]+0x14]` (`0x77e4d7`). `0x77e530` reads `[esi+0x38]` before the
call to `0x784180` (`0x77e537`) and ends in `call [[esi+0x7c]]` with the two (`0x77e543`). The slots are stored at `0x81c8b8`, `0x81c8c0`, `0x81c8cc`, `0x81c8d0` and their twins `0x81cb24`, `0x81cb2c`, `0x81cb38`. What the objects
behind `+0x7c` and `0xcf4ea8` are is not read here.

## Unreached edit box code

`0x77e2e0(word)` stores a justify word in the box's own font block (`+0x324`, block `+0x38`, word `+0x20`) and marks it changed (`0x404`); nothing
calls it and no section stores its address.

Addresses: `0x77e2e0`, `0x77e2ff`, `0x77e302`
Confidence: verified
Proof: one
Test: none

The census is every `call` and `jmp` to `0x77e2e0` in the listing, and every little-endian occurrence of the address in each section of the image: none of either. The same two scans find the callers of
`0x77e250` (`0x77b3bb`, `0x77b55c`) and the one vtable slot that holds `0x77a8d0` (`0x81c958`), which is the control.
