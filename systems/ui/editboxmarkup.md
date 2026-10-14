# ui: EditBox markup

How an `EditBox` treats the inline escapes `|cAARRGGBB`, `|r`, `|H…|h…|h`, `|n` and `||`: the font string
parses and hides them, the box keeps a class for every byte, and the cursor, the deletions and the letter
count work on whole tokens, with a hyperlink as one unit for the keyboard but not for the mouse. The keys
and the text limits are in `editbox`; the font string's own methods are in `fontstring`.

The box stores the raw bytes, escapes included, and gives the raw text to its label font string, whose
decoder gives every escape no width. The box does not strip anything.

Roots: `0x77ba90`, `0x77bb30`, `0x77c510`

## The escapes

The decoder `0x5c2810` reads one token at a pointer and returns its class and byte length. A line break
(`\r`, `\n`, `\r\n`) is always class 2, even with all markup off. There is no inline texture escape: `|T` and
`|t` are ordinary characters.

| class | token | bytes |
|---|---|---|
| 0 | `\|cAARRGGBB` | 10 |
| 1 | `\|r` | 2 |
| 2 | a line break, or `\|n` | 1 or 2 |
| 3 | `\|\|`, one literal `\|` | 2 |
| 4 | `\|H…\|h`, the whole opening | the span |
| 5 | `\|h` closing | 2 |
| 6 | any other character | its UTF-8 length |

A `|H` with no `|h` after it, whose span is exactly four bytes, or whose visible text is empty, is a literal
`|` of class 6. The `AA` is parsed and dropped: the colour comes out opaque, and the font string's own alpha
replaces it when the escape is drawn.

Addresses: `0x5c2810`, `0x5c2835`, `0x5c286e`, `0x5c2896`, `0x5c28b8`, `0x5c28d1`, `0x5c2972`, `0x5c2992`, `0x5c29f2`, `0x5c2a0e`, `0x5c2a22`, `0x5c29d3`, `0x5c2ab2`, `0x5c2ace`, `0x5c2ad0`, `0x5c2ae5`, `0x5c2af4`, `0x5c2b10`, `0x5cceb0`, `0x5cceb9`, `0x5ccea6`
Confidence: verified
Proof: one
Test: none

The remap table at `0x5c2b10`, indexed by the character minus `C`, sends only `C c`, `H`, `N n`, `R r`, `h`
and `|` to the arms of the jump table `0x5c2af4`; every other byte, `T` and `t` included, takes the literal
arm. The arms write the byte length as an immediate (`0x5c2ad0`, `0x5c2a0e`, `0x5c29d3`, `0x5c2a22`); the
open link's length is the span (`0x5c29b8`). The line-break arms precede every flag test (`0x5c286e`,
`0x5c2896`, then the master test `0x5c28b8`). The alpha is forced at `0x5c2aba`; the draw routine copies
the font string's alpha byte `[fs+0x2f]` over it (`0x5cceb0` to `0x5cceb9`), and the `|r` arm restores the
base colour whole.

## Which escapes are on

The decoder takes a flag word: `0x100` turns off `|c` and `|r`, `0x200` `|n`, `0x400` `|H` and `|h`, `0x800`
all of them. A font string builds it from its own flags (`0x2000`, `0x4000`, `0x8000`, `0x0080` give those four),
and no code of the font string or the edit box sets the first, third or fourth, so every font string of this build parses `|c`, `|r`, `|H`, `|h` and
`||`. Only `|n` varies: the box sets it off for a single-line box (flag `0x4000`) and on for a multi-line
one. The box's own class array is built with no flag, so it always reads `|n` as a line break.

Addresses: `0x44d670`, `0x44d6a4`, `0x44d6a9`, `0x44d6d0`, `0x44d6d8`, `0x44d6e0`, `0x44d6ea`, `0x44d6f3`, `0x44d6fc`, `0x5c2825`, `0x5c2835`, `0x5c283f`, `0x770dd3`, `0x77a4a0`, `0x77a552`, `0x77a5e2`, `0x77ba90`, `0x77bad9`
Confidence: verified
Proof: one
Test: none

`0x44d670` maps the font string flags to the decoder's (`0x44d6a9` to `0x44d6fc`): `0x2000` to `0x100` (`0x44d6e0`),
`0x4000` to `0x200` (`0x44d6ea`), `0x8000` to `0x400` (`0x44d6f3`), `0x0080` to `0x800` (`0x44d6fc`). The decoder
masks them at `0x5c2825` to `0x5c283f`. The font string constructor sets the flags to `0x212` (`0x770dd3`),
and no instruction in the font string or edit box code ors in `0x2000`, `0x8000` or `0x0080`. `SetMultiLine`
clears `0x4000` for a multi-line box (`0x77a552`) and sets it for a single-line one (`0x77a5e2`). `0x77bad9`
pushes a flag word of 0.

## The class array

The box keeps a dword for every byte of its text at `+0x330`, rebuilt after every insert by `0x77ba90`. The
dword at a token's first byte holds the token's length in bits 0 to 15, its class in bits 16 to 23 and in bit
31 whether it is inside a hyperlink; every other byte of the token holds 0. Bit 31 is raised at the opening of
a link and lowered after the closing `|h` is stored, so both ends of the link are marked. The slot after the
last byte is 0.

Addresses: `0x77ba90`, `0x77ba98`, `0x77bab5`, `0x77bac8`, `0x77bad0`, `0x77bae2`, `0x77bae7`, `0x77baec`, `0x77baf3`, `0x77bafc`, `0x77bb01`, `0x77bb0f`, `0x77bb19`, `0x77bb1e`, `0x77c022`, `0x77c670`
Confidence: verified
Proof: one
Test: none

`0x77ba90` zeroes four times `+0x344` bytes (`0x77baa2` to `0x77bac8`), then decodes token after token
(`0x77bae2`). A class of 4 ors in the sticky bit (`0x77baec`); the dword is the length masked to 16 bits, the
class shifted left 16 and the sticky bit (`0x77baf3` to `0x77bb03`), stored at the first byte (`0x77bb0f`). A class of
5 clears the sticky bit after the store (`0x77bb19`, `0x77bb1e`). The rebuild is called at the tail of an insert
(`0x77c022`); `0x77c670` stores the zero for the terminator after a deletion.

## Boundaries and letters

Three small functions walk the array: `0x77bd10(i)` is the start of the next token, `0x77bd30(i)` the start of
the previous one (the nearest lower index with a nonzero entry, or −1), and `0x77bc80(start, count)` counts the
letters in a range. Only classes 2, 3 and 6 count as letters, so a color escape and a hyperlink's markers cost
none, and a multibyte character costs one. The letter limit and the byte limit trim whole tokens from the
end with the deletion that Backspace uses, so a whole link goes in one bite.

Addresses: `0x77bd10`, `0x77bd30`, `0x77bc80`, `0x77bcb0`, `0x77c085`, `0x77c0e4`, `0x77c280`, `0x77c02d`
Confidence: verified
Proof: one
Test: none

`0x77bcb0` compares each nonzero slot's class with 2, 3 and 6. The trim at `0x77c085` to `0x77c0f9` counts the
whole buffer on each insert and again after each removal (`0x77c0e4`); the removal is `0x77c280(−1)`.

## How the box draws and measures

The label gets the raw text from the scroll position, or, when that position falls inside a colour span (the
first escape in view is a `|r`), the governing `|c` copied to the front of the visible text. The caret's
x is the width of the raw bytes from the line start to the cursor measured by the same kernel, which gives
every escape no width, so the caret sits right after the last drawn glyph. Kerning looks past escapes to the next
visible character. A password box masks every raw byte with `*`, so a link's whole markup shows as stars.

Addresses: `0x77d4d0`, `0x77d52e`, `0x77d807`, `0x77d81e`, `0x77d822`, `0x77d8ee`, `0x77d8f3`, `0x77d93c`, `0x77da80`, `0x77dcde`, `0x77dcf1`, `0x772ae0`, `0x5c6940`, `0x5c6a13`, `0x5c6a29`, `0x5c6b70`, `0x5c6bb5`
Confidence: verified
Proof: one
Test: none

`0x77d807` scans the window for `|` (skipping `||`): a `|c` first needs nothing (`0x77d81e`); a `|r` first
(`0x77d822`) walks back for the `|c`, copies exactly ten bytes of it in front of the window (`0x77d8f3`) and sets
the composite text (`0x77d93c`). The caret uses `0x772ae0(line start pointer, byte count)` (`0x77dcf1`) into
`0x5c6940`, whose per-token loop spends a byte budget and adds width only for classes 3 and 6 (`0x5c6a13` to
`0x5c6a2f`). `0x5c6b70` skips to the next class 6 or 2 for kerning (`0x5c6bb5`). The password fill is
`0x2a2a2a2a` (`0x77d52e`).

## Steps

One step of `0x77bb30(start, steps, atomic)` over the class array moves over one token and its neighbours. It
always skips colour and link-opening tokens before the next visible character, and, after the character,
absorbs the `|r` and `|h` that follow. With `atomic` set, a character inside a link does not end the step: the
whole link, up to and including its `|h`, is crossed at once. It returns the number of bytes crossed. A step
is a whole UTF-8 character, never part of one. A backward step mirrors the forward one.

Addresses: `0x77bb30`, `0x77bb42`, `0x77bb47`, `0x77bb52`, `0x77bb56`, `0x77bb60`, `0x77bb7d`, `0x77bb84`, `0x77bb89`, `0x77bb8b`, `0x77bb8f`, `0x77bb93`, `0x77bb98`, `0x77bbad`, `0x77bbb2`, `0x77bbcc`, `0x77bbe2`, `0x77bc13`, `0x77bc18`, `0x77bc24`, `0x77bc28`, `0x77bc59`, `0x77bc78`, `0x41aab0`
Confidence: verified
Proof: one
Test: none

The forward loop at `0x77bb60` adds the token's length (`0x77bb7d`) and keeps going for class 0 (`0x77bb84`) and
class 4 (`0x77bb89`); then `atomic` clear ends the step (`0x77bb8b`), bit 31 clear ends it (`0x77bb8f`), and a link
token that is not class 5 keeps going (`0x77bb93`). The post loop (`0x77bb98`) absorbs classes 1 and 5
(`0x77bbad`, `0x77bbb2`). The backward walk (`0x77bbcc` onward) takes trailing class 1 and 5 first (`0x77bc13`,
`0x77bc18`), crosses a link back to its class-4 start with `atomic` (`0x77bc24`, `0x77bc28`), then back over leading
colour and link tokens (`0x77bc59`). `0x41aab0` decodes up to six UTF-8 bytes; the lengths are the class
array's. The token length is read as the step's advance, so no step lands inside a character.

## Where a step is atomic

The keyboard crosses a link in one step; the mouse does not. Left, Right, Shift with either, Ctrl with either
(word steps), Home, End, Backspace, Delete and Ctrl with the last two all pass `atomic` set. A mouse click
or drag, Up and Down, the sizing of the scroll window and the composition span pass it clear, so the
mouse can put the cursor between the characters of a link's name. No path can put the cursor inside an escape.

| operation | functions | atomic |
|---|---|---|
| Left, Right, Shift+Left, Shift+Right | `0x77c6b0` | yes |
| Ctrl+Left, Ctrl+Right, Home, End | `0x77c8c0`, `0x77c7a0`, `0x77c980`, `0x77c9f0` | yes, each step |
| Backspace, Delete, Ctrl+Backspace, Ctrl+Delete | `0x77c280`, `0x77c3f0`, `0x77c310` | yes |
| mouse click and drag | `0x77d0d0` | no |
| Up and Down | `0x77cb20` | no |
| scroll sizing, composition | `0x77d4d0`, `0x77d320`, `0x77ac60` | no |

Addresses: `0x77c6b0`, `0x77c6d2`, `0x77c72c`, `0x77c280`, `0x77c2a3`, `0x77c3f0`, `0x77c310`, `0x77c8c0`, `0x77c7a0`, `0x77c980`, `0x77c9f0`, `0x77d0d0`, `0x77d295`, `0x77d2f6`, `0x77cb20`, `0x77cba4`, `0x77cbcf`, `0x77d4d0`, `0x77d320`, `0x77d733`, `0x77ac60`, `0x77ac8d`, `0x77a860`
Confidence: verified
Proof: one
Test: none

The sites that push 1 for `atomic` are `0x77c6d2` and `0x77c2a3` (the others reach them through
`0x77c6b0` and `0x77c280`); the sites that push 0 are `0x77d295`, `0x77d2f6`, `0x77cba4`, `0x77cbcf`, `0x77d733`, `0x77d34e`,
`0x77d395` and `0x77ac8d` to `0x77acb9`. `0x77c6b0` adds the byte count to the cursor (`0x77c72c`) and passes the same count
to the selection, so Shift with an arrow selects a whole link. A drag asks the hit test `0x77d0d0` (`0x77a88c`).

## Deleting through a link

Removing a range moves each end out of a link: an end inside a link goes back to the link's start,
including the colour escape before it, and an end inside goes forward past the closing `|h` and any `|r`
after it. Any deletion that touches a link removes the whole of it, colour and reset included, whether it came
from the keyboard or from a selection the mouse made. A deletion also clears the selection.

Addresses: `0x77c510`, `0x77c516`, `0x77c532`, `0x77c543`, `0x77c550`, `0x77c566`, `0x77c59d`, `0x77c5ba`, `0x77c5c9`, `0x77c5f8`, `0x77c60e`, `0x77c618`, `0x77c626`, `0x77c64b`, `0x77c660`, `0x77c670`, `0x77c683`, `0x77cd70`, `0x77c500`, `0x77bd30`, `0x77bd10`
Confidence: verified
Proof: one
Test: none

`0x77c510(lo, hi)` first clears a selection (`0x77c516` to `0x77c532`). If the class of `lo` has bit 31
(`0x77c543`, `0x77c548`) it walks back with `0x77bd30` to a class 0 or 4 token start and over the run before it
(`0x77c550` to `0x77c571`); if that of `hi` has it (`0x77c59d`) it walks forward with `0x77bd10` over the
closing `|h` and `|r` (`0x77c5ba`, `0x77c5c9`). It then moves the bytes and the classes down (`0x77c626`,
`0x77c64b`), shortens the length (`0x77c60e`), puts the cursor at `lo` (`0x77c618`), terminates the text
(`0x77c660`, `0x77c670`) and sets dirty bits 1 and 4 (`0x77c683`). It is called by every deletion, the
selection delete `0x77cd70` and the clear `0x77c500`.

## Faults

Three client faults follow from the class array. A text whose last token is a colour escape (`|cffffffff`),
or, for a keyboard step, an unclosed link, hangs the step loop, which has no test for the end: `SetText` can set
such a text and one press of Right then freezes the client, though typing cannot, since `|` is typed as
`||`. A single-line box reads `|n` as a line break in its class array but draws and measures it as the two
characters `|` and `n`, which skews the caret by one glyph. The right-justified hit test counts glyphs from the
right edge and converts them from the line start, so markup or multibyte text gives a wrong byte index.

Addresses: `0x77bb56`, `0x77bb60`, `0x77bb98`, `0x77c750`, `0x77c75c`, `0x77ba90`, `0x77a5e2`, `0x77d27f`, `0x77d292`, `0x77d2a6`, `0x772c30`
Confidence: verified
Proof: one
Test: none

The outer test `0x77bb56` is passed once per step; the inner loop re-enters at `0x77bb60`. A trailing `|r` or `|h` is
safe because the post loop (`0x77bb98`) ends on the terminator. `0x77c750` tests only that the cursor is below
the length (`0x77c75c`) before the step. The skew comes from the class array being built with no flags
(`0x77bad9`) while `0x77a5e2` turns the font string's `|n` off. The right-justified case converts at
`0x77d27f` to `0x77d2a6`.
