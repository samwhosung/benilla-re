# ui: the layout cache

How the interface saves where the player left movable and resizable frames and puts them back: the
per-character section file `layout-cache.txt`, the frames it names, its text, and what applying a
record does to a frame's anchors.

The section is row 6 of the account data table (`net/accountdata`): a per-character file whose
read callback `0x490380` is registered when the interface is built and fires when the account data
sync has resolved the section, and whose write callback `0x490e10` runs at shutdown. A frame is
written only after the player moved or sized it, and is put back by a single anchor to the screen
root.

Roots: `0x490380`, `0x490e10`, `0x4905e0`, `0x7652b0`, `0x768430`

## Which frames are saved

The writer walks every frame of the top's list in registration order and writes a frame that has a
name, whose user-placed bit `0x1000` of `+0xb4` is set and that is movable (`0x100`) or resizable
(`0x200`). A frame failing any of the four writes nothing. The user-placed bit is set by dragging,
by a record being applied and by `SetUserPlaced`, which raises `Frame %s is not movable or
resizable` for a frame that is neither. Nothing clears it but `SetUserPlaced(false)`, and it
governs nothing but this file.

Addresses: `0x490c79`, `0x490e10`, `0x490e24`, `0x490e60`, `0x490e72`, `0x490e7f`, `0x490e88`, `0x490e8e`, `0x490e97`, `0x76a3c0`, `0x49067e`, `0x490706`, `0x7652e5`, `0x776b36`, `0x776a50`, `0x776ad5`, `0x776b40`, `0x776bc5`, `0x5afed0`
Confidence: verified
Proof: one
Test: `ui::glue_4c0_490e60_round_pos_legs`

`0x490c79` hands `0x490e10` to `0x5afed0` for section 6, which opens the file for writing only while
the section is not pending. `0x490e10` walks `[top+0xcc4]` by the link offset `[top+0xcbc]`, 0x304,
until the tagged terminator, and calls `0x490e60` on each frame. The tests are the name (`0x490e72`
to `0x490e82`), `test ah,0x10` (`0x490e8e`) and `test ah,0x3` (`0x490e97`). Every change of `+0xb4`
goes through `0x76a3c0`; its fourteen callers set `0x1000` at `0x49067e`, `0x490706`, `0x7652e5` and
`0x776b36` only, and a census of loads of `+0xb4` followed by a test of that bit finds `0x490e88` and
`0x776bc5` only.
`ui::glue_4c0_490e60_round_pos_legs` runs the four position legs (`(v − Y) / div · 1024`, `/ 0.8`, a `0.5` bias, `__ftol`) natively across `0x41ad70` and `0x41ae40`.

## The text of a record

A record is up to six lines: `Frame: <name>`, `FrameLevel: <level>`, then `X`, `Y`, `W` and `H`,
each `Label: <integer>`, separated by line feeds with no blank line. The level is always written.
`X` and `Y` follow only for a movable frame and `W` and `H` only for a resizable one. All four are in
interface units, so a frame at scale `s` has `X = round(768·√(a²+1)·left / s)`, `Y` the same of `top`
minus the root's top, `W` and `H` the same of the extents, each plus 0.5 and truncated.

Addresses: `0x490f1e`, `0x490f5c`, `0x490f93`, `0x490fa2`, `0x490fb7`, `0x490fbf`, `0x490fcc`, `0x490fd1`, `0x490fd9`, `0x490fdf`, `0x49101c`, `0x491099`, `0x4910a8`, `0x4910d3`, `0x49113b`, `0x490edf`, `0x490eed`, `0x490f10`, `0x768320`, `0x41ad70`, `0x41ae40`, `0x40a2b0`, `0x64a7f0`, `0x42a5d0`, `0x7ffd68`
Confidence: verified
Proof: one
Test: none

The formats are `Frame: %s\n`, `FrameLevel: %d\n`, `X: %d\n`, `Y: %d\n`, `W: %d\n` and `H: %d\n`, each
formatted into 128 bytes by `0x64a7f0` and written by `0x42a5d0`. The frame's rectangle comes from
`0x768320` on its layout part; when that fails the writer lays the frame out with `0x7680e0(1)` and retries
once, and a second failure ends the record without writing a line (`0x490f00`), and the root's rectangle is read with no retry; if that read fails the record ends the same way (`0x490f17`). Both rectangle reads come before the `Frame:` line, so a failed frame leaves nothing in the file. Each number is `0x41ad70·1024`,
multiplied by the edge over the frame's effective scale `+0x7c` and divided by `K1` (`0x41ae40`). `X`
and `Y` round half away from zero (`fcom 0.0; test ah,5; jp` then `fadd` or `fsub` 0.5, `0x40a2b0`
truncates). `W` and `H` always add 0.5, so an inverted rectangle truncates toward zero. `Y` is measured from the
root's top, which is why `X` needs no root term (the root's left is 0). The top is `[frame+0xa0]`, not the parent.

## Reading a record

The reader takes the file a line at a time, splitting at CR and LF into 128 bytes, and matches the
prefixes `Frame: `, `FrameLevel: `, `X: `, `Y: `, `W: ` and `H: ` in that order. A `Frame: ` line applies
the record in progress, then resets `X`, `Y`, `W` and the level to 0, 0, 0 and −1, but not `H`, so a
record without an `H:` line inherits the previous record's. The name is looked up as a frame
global; a name that does not resolve gives a null frame and the record, with all its lines, is
dropped without a message. The last record is applied at the end of the file.

Addresses: `0x490380`, `0x49038c`, `0x490391`, `0x4903ef`, `0x49040b`, `0x490421`, `0x49045b`, `0x49045d`, `0x49045f`, `0x490462`, `0x490469`, `0x490456`, `0x4905c0`, `0x4900c2`, `0x76c760`, `0x64ae50`, `0x64a4c0`, `0x64ac60`, `0x5afe50`
Confidence: verified
Proof: one
Test: none

`0x490380` returns when the handle is −1, sizes a buffer with the file size, reads it, writes a NUL
and loops `0x64ae50` over it with the delimiters `\r\n`. `0x49045b` to `0x490469` zero `ebx`, `edi`, `esi` and
store −1 at `[ebp-4]`; `[ebp-0xc]` (`H`) is zeroed once, at `0x4903d5`, before the loop. `0x490456` calls
`0x76c760` with the Frame type id `[0xcf0c10]` and `0x49045f` stores its result unchecked.
`0x4900c2` registers the reader with `0x5afe50` for section 6, which stores it while the section is
pending (`net/accountdata`).

## Applying a record

A record changes only what its frame allows. A level of 0 or more is set with propagation. A movable
frame with `X` or `Y` nonzero loses every anchor point and gets one: its top left to the root's top
left at `X` and `Y`. A resizable frame with `W` or `H` nonzero gets that width and, separately, that
height, each only when nonzero. Each block sets the user-placed bit, and the frame is then laid out
at once. A frame at the corner (0, 0) is therefore never moved, and any authored anchors, a
`setAllPoints` pair included, are gone.

Addresses: `0x4905e0`, `0x4905f5`, `0x490600`, `0x490607`, `0x49061a`, `0x490670`, `0x49067e`, `0x490689`, `0x4906c7`, `0x4906fa`, `0x490706`, `0x490710`, `0x767ed0`, `0x767620`, `0x767c70`, `0x76a4f0`, `0x7683d0`, `0x7683f0`
Confidence: verified
Proof: one
Test: `ui::glue_4905e0_device_position`

`0x4905e0` takes the frame in `ecx`, the level in `edx` and `X`, `Y`, `W`, `H` on the stack. It
returns for a null frame (`0x4905e6`), calls `0x76a4f0(level, 1)` for a level of 0 or more, and
tests `+0xb4` bit `0x100` (`0x490600`). `0x767ed0` jumps to `0x767620`, which empties all nine slots
(`anchors`). `0x490670` calls `0x767c70(0, root, 0, x, y, 1)` with each of `X` and `Y` converted by
dividing by `0x41ad70·1024` and `0x41ae60`. The size block tests bit `0x200` and calls the layout
slots 5 and 6, `0x7683d0` and `0x7683f0`. `0x490710` calls `0x7680e0(1)` on every path that gets
past the null test.
`ui::glue_4905e0_device_position` runs the `fmul` by 1024.0 and `fdivr` of the device position as a copied-byte island; the body is shared by four byte-identical sites.

## Starting a drag

Starting a move or a size gives the frame the same shape. `0x7652b0` lays the frame out if needed,
clears its anchors and gives it one top left anchor to the root at its current offset, with its
current width and height set, before it raises the frame and sets the user-placed bit. If either
rectangle cannot be read it does nothing and answers 0. Ending the drag clears the top's moving
frame and kind and leaves the bit set.

Addresses: `0x7652b0`, `0x7652c6`, `0x7652d7`, `0x7652e5`, `0x768430`, `0x76849f`, `0x7684dc`, `0x7684e1`, `0x765640`, `0x776990`, `0x776a35`, `0x776a40`, `0x765479`, `0x776822`, `0x776981`
Confidence: verified
Proof: one
Test: none

`0x7652b0` calls `0x768430` on the frame's layout part with zeros (`0x7652c6`) and continues only on a
nonzero answer (`0x7652cb`). `0x768430` answers 0 when either rectangle read fails (`0x768481`, `0x768497`). Otherwise it calls
`0x767620`, then `0x767c70(0, root, 0, x, y, 0)` with `x = (left + dx − root left) / scale` and `y = (top + dy − root top) / scale`
(`0x7684b4` to `0x7684dc`), uses the argument when nonzero or `(right − left) / scale` for the width
and `(top − bottom) / scale` for the height (`0x7684e1` to `0x768521`), sets them through slots 5 and
6 and lays out. `0x765640` clears `+0xcfc` and `+0xd00`; `0x776990` calls it only when the top's moving
frame is the frame itself. The three callers of `0x7652b0` are `0x765479`, `0x776822` and `0x776981`.
