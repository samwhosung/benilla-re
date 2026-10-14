# console: screen

The console panel: the line list, the input line and its editing, the keys and the mouse, the two
paint layers, and the slide that opens and closes it. Screen coordinates are the unit box with y
up, so the panel is the strip from its lower edge `0x864548` to 1.0 along the top of the screen. The
panel's height is `0xc4ec10`; the line count `0x864560` (a float, default 10.0) is that height over the font
size `0x86455c` (default 0.02). The client starts and stops the screen with `0x63b460` and `0x63c9b0`.

Roots: `0x63b460`, `0x63c9b0`, `0x63cb00`, `0x63cb10`, `0x63cb50`, `0x63cd00`, `0x63cd50`, `0x63cdf0`, `0x63ce00`, `0x63c9a0`, `0x63b5c0`, `0x63b800`, `0x63bd50`, `0x63be90`, `0x63c480`, `0x63c860`, `0x63c970`, `0x63c0b0`, `0x63c3c0`, `0x63c400`, `0x638fd0`, `0x6390c0`, `0x6393b0`, `0x6397d0`, `0x639890`, `0x6395c0`, `0x639660`, `0x639700`, `0x639100`, `0x639300`, `0x638f40`, `0x639240`, `0x639250`

## Line records

A console line is a 0x24-byte record allocated zero-filled under the tag `CONSOLELINE`: next link
`+0`, previous link `+4`, text `+8`, length `+0xc`, capacity `+0x10`, cursor `+0x14`, floor `+0x18`,
colour class `+0x1c` and string handle `+0x20`. The lines are one intrusive list with the newest at the
head (`0xc4e9ac`) and a count (`0xc4ec24`); past 0x100 lines the oldest is freed. The input line is the
head while its cursor is nonzero. `0x63baa0` makes a new input line whenever the head's cursor is 0:
text `> `, floor = cursor = length 2, class 1, capacity 0x10, and the view anchor `0xc4e9b4` set to it.

Addresses: `0x63baa0`, `0x63bbd0`, `0x63cf80`, `0x63cfb0`, `0x638f40`, `0x638ff0`, `0x639020`, `0xc4e9ac`, `0xc4e9b4`, `0xc4ec24`, `0x864908`, `0x6462e0`, `0xc4e9a4`
Confidence: verified
Proof: one
Test: none

`0x63bac5` allocates `0x24` bytes with flags 8, which `0x6462e0` tests at `0x6462fc` to zero the block. The
text buffer is 0x10 bytes (`0x63bb6c`, `0x63bb6e`) seeded from the text `> `. `0x63bbd0` compares the
count with 0x100 and takes the oldest line from the sentinel `0xc4e9a8`; if it is the anchor, the anchor
moves to its neighbour, or to 0. It frees the text, the string through `0x5c1d00`, unlinks (`0x63cfb0`) and
frees the record. `0x63cf80` resolves a link's slot, a negative offset taken relative to the node.
`0x639020` makes a line's string from its text, size, style flags, spacing and the colour
`0xc4ead8 + 4·class`; `0x638ff0` does it for every line. `clear` (`0x638f40`) frees all lines.

## Colour classes

A line's class indexes the colour table `0xc4ead8`: 0 default, 1 input, 2 echo, 3 error, 4 warning, 5
global, 6 admin, 7 selection highlight, 8 background. The table starts as `ffffffff` for 0, 1, 5 and 6,
`ff808080` for echo, `ffff0000` for error, `ffffff00` for warning, `80ffffff` for the highlight and
`c0000000` for the background. `fontcolor <class> <r> <g> <b>` sets a class's colour from 0 to 255
channels (alpha 0xff); `bgcolor` and `highlightcolor` take `<a> <r> <g> <b>` for classes 8 and 7.

Addresses: `0x6393b0`, `0x6397d0`, `0x639890`, `0x6395a0`, `0xc4ead8`, `0x638a60`
Confidence: verified
Proof: one
Test: none

`0x6393b0` parses `%s %d %d %d` (`0x73fb5d`), needs 4 results, and maps the name by `SStrCmpI` against
`input` (1), `default` (0), `echo` (2), `error` (3), `warning` (4), `global` (5) and `admin` (6). A channel
above 0xff (unsigned compare) writes two error lines. The colour is `0xff000000 | r<<16 | g<<8 | b`
(`0x6394f6` to `0x63950c`); `0x6395a0` stores it when the class is below 9, and `0x639539` recolours the
lines of that class. `0x6397d0` and `0x639890` pass classes 8 and 7. `0x638a60` stores the initial table.

## Writing and executing

`ConsoleWrite` (`0x63cb50`, text in `ecx`, class in `edx`) does nothing without text, a device (`0xc4ec1c`) or a
font (`0xc4ec34`). Under the lock it links a new line behind the input line (at the head with none) and evicts past 0x100 lines.
`0x63cd00` and `0x63cd50` format into 1024 bytes and write with a class or with 0. `ConsoleCommandExecute` (`0x63ce00`, line in `ecx`, history flag in `edx`)
skips leading spaces, records the line in the history when the flag is set and it is new, and calls the command's handler with
the name and the rest. An unknown name goes to the fallback pointer `0xc518a4`, which nothing sets, then to a command `run`, which nothing registers,
and otherwise writes `Unknown command`.

Addresses: `0x63cb50`, `0x63cd00`, `0x63cd50`, `0x63ce00`, `0x63f880`, `0x63f9c0`, `0x641af0`, `0x6419e0`, `0x86547c`, `0xc4e64c`, `0xc518a4`, `0x42a320`, `0x6579b0`, `0x6579c0`, `0x63cb59`, `0x63cb67`, `0x63cb75`, `0x63cb82`, `0x63cb91`, `0x63ccea`, `0x63cbca`, `0x63ce15`, `0x63ce1e`, `0x63ce3e`, `0x63ce4e`, `0x63ce59`, `0x63ce81`, `0x63ce88`, `0x63ce8c`, `0x63ce9a`, `0x63cea4`, `0x63cf3a`, `0x63cf2e`, `0x864908`, `0x8648c8`, `0x63ccb3`, `0x63ccd5`, `0x63ccda`, `0x63cce0`
Confidence: verified
Proof: one
Test: none

`0x63cb50` tests the pointer (`0x63cb59`), the first byte (`0x63cb67`), the device (`0x63cb75`) and the font (`0x63cb82`), enters the lock `0xc4e64c` (`0x63cb91`)
and leaves it at `0x63ccea`. The link branch is `0x63cbca` (`[head+0x14]` against 0). The record's length and
capacity are both `strlen + 1` (`0x63ccb3`); the string is made at `0x63ccd5`, the count raised at `0x63ccda` and the limit enforced at `0x63cce0`. `0x63ce00` calls `0x6419e0` for a mode below 3
(`0x63ce15` to `0x63ce1e`), but `0x86547c` starts at 4 and only `0x6419e0` writes it, so the call never runs. The history test compares entry 0 case-sensitively (`0x63ce3e` to `0x63ce4e`) and
`0x641af0` copies 0x50 bytes into the ring (`0x63ce59`). A command found has its handler checked by `0x42a320`, then called through `[cmd+0x18]` (`0x63ce9a`). The fallback is called at `0x63ce81`; a return of 0 would
reach `0x63ce8c` with a null command (`0x63ce88`). `0x63cea4` looks up `run` and `0x63cf2e` writes `Unknown command`; `0x63cf3a` would call a `run` handler with an empty name and the whole line.

## Input line

The input line edits in place; every edit rebuilds its string (`0x638d10`). Backspace removes the byte
before the cursor above the floor; Delete removes the byte at the cursor, and at the end of the text it
lowers the length by one and changes no byte; Left and Right move the cursor between the floor and
the length. Text is inserted at the cursor, growing the buffer in 0x10 steps (`0x638e90`); a multi-byte
paste at a cursor inside the text splices head, paste and tail. Up and Down walk the history, 32
entries with index −1 for the empty line, by loading the entry as the text after the floor.

Addresses: `0x638cc0`, `0x638da0`, `0x638de0`, `0x638df0`, `0x638e00`, `0x638ef0`, `0x638e40`, `0x638e90`, `0x638d10`, `0x63bee0`, `0x63f9b0`, `0x63f9c0`, `0x864558`, `0x882748`, `0xc4eaac`, `0xc4eadc`, `0xc4ec50`, `0x864574`, `0xc4ee48`, `0xc518a0`
Confidence: verified
Proof: one
Test: none

Backspace is guarded by `floor < cursor` (`0x638cc3`, `0x638cc6`) and moves the tail with `0x4100f0`. Delete
(`0x638da0`) is guarded by `cursor ≤ len`; at `cursor = len` the move is 0 bytes and `0x638dc8` still
decrements the length. `0x638e00` stops at index 31 (`0x63f9b0` returns 32), `0x638ef0` uses the empty
string `0x882748` at −1. A history entry `i` is at `0xc4ee48 + 0x50·((head − i − 1) & 0x1f)` (`0x63f9c0`),
head `0xc518a0`. `0x638e40` sets cursor = length = floor, stores the NUL, grows and copies. `0x638d10`
frees `0xc4eaac` and, for non-empty text, makes the string (`0x5c1c30`) with size `0x86455c`, flags `0x864574`,
spacing `0xc4ec50` and colour `0xc4eadc`. The paste splice is `0x63bf58` to `0x63c022`.

## Keys

While the screen is open it swallows (returns 0 from) every key down, key repeat, key up and character,
and a click inside the panel. Key down: Enter runs the text after the floor (cursor reset to 0, the
line becomes an output line of class 1, the history index to −1); Escape clears the text, or closes
the console when it is empty; Tab completes the first word through the registry (Shift reverses),
repeated presses cycling; Home and End move the cursor, or with Ctrl scroll to the oldest and newest
line; Page Up and Page Down scroll one line, ten with Shift; Ctrl+C copies the selection and Ctrl+V pastes.

Addresses: `0x63bc90`, `0x63c480`, `0x63c860`, `0x63c970`, `0x63be90`, `0x63c7c0`, `0x63c070`, `0x63fc60`, `0x63c794`, `0x63c938`, `0x864554`, `0x41f8f0`, `0xc4ec28`, `0xc4eac8`, `0xc4ec20`, `0x63ca80`, `0xc4e738`, `0xc4ec2c`
Confidence: verified
Proof: one
Test: none

`0x63bc90` registers eight handlers at priority 7.0 on categories 1, 5, 8, 9, 0xa, 0xb, 0xe and 0xc, above
the interface's 1.0 and below the loading screen's veto at 8.0. The key codes are `core/input#key-codes`:
Escape 0x200, Enter 0x201, Backspace 0x202, Tab 0x203, Left to Down 0x204 to 0x207, Insert 0x208,
Delete 0x209, Home 0x20a, End 0x20b, Page Up 0x20c, Page Down 0x20d. The key table is `0x63c794` (keys 0x206 to
0x20d) and the repeat table `0x63c938` (0x202 to 0x20d: Tab, Insert, Home and End do nothing). Ctrl is
`modifiers == 2` (`0x63c58c` for C, `0x63c551` for V). Tab passes `modifiers == 1` to `0x63fc60`
(`0x63c677` to `0x63c683`).

## Console key

The console opens and closes on a key whose code is `0x864554` (default 0x60, the grave accent), and
only when `0xc4ec20` is nonzero, which `ConsoleDeviceInitialize` sets from the command-line switch
`-console`. The key down and the key repeat both flip the open flag `0xc4eac8` and are swallowed, so
holding the key flips it on every repeat. The key's own character is swallowed while it is held.
`SetConsoleKey` (`0x63cb00`) replaces the code. Closing clears the selection. Without the switch the
console never opens.

Addresses: `0x63cb00`, `0x488640`, `0x63c4a3`, `0x63c876`, `0x63be90`, `0x63bea6`, `0x864554`, `0xc4eac8`, `0xc4ec20`, `0x419c10`, `0x63c970`
Confidence: verified
Proof: one
Test: none

`0x63c49b` tests `0xc4ec20` and `0x63c4a3` to `0x63c4b2` negate the open flag; the repeat handler does the
same at `0x63c876` to `0x63c884`. `0x63a2b4` to `0x63a2bf` store 1 in `0xc4ec20` when `0x419c10(0x22)`
returns 1. The switch table at 0x7ff8b8 lists index `0x22` as `console`. The only other writers of the open
flag store 0 (`0x63c52c`, `0x639240`). The Lua function `SetConsoleKey` (`0x488640`) calls `0x63cb00`.

## Mouse

A click in the panel's lowest `0.75 · font size` starts a resize drag (`0xc4ec38 = 1`), and the move
handler then sets the height to `1 − y`, no lower than the font size. A click elsewhere in the panel
selects the line under the cursor: its text is copied to `0xc4eb00`, the selection flag `0xc4ec3c` is 1,
and the highlight band is one font size high at the line. Dragging sets the span's horizontal end; the
button up fixes it (`0xc4ec3c = 2`). Clicks above the panel pass, and the move and up handlers always
return 1.

Addresses: `0x63c0b0`, `0x63c3c0`, `0x63c400`, `0x63c220`, `0x63c290`, `0xc4ec38`, `0xc4ec3c`, `0xc4eb00`, `0x864564`, `0x864568`, `0x86456c`, `0x864570`, `0xc4ec40`, `0xc4ec44`, `0xc4ec48`, `0xc4ec4c`, `0x5c2190`, `0x63c070`
Confidence: verified
Proof: one
Test: `console::mouse_down_drag_difftest`, `console::mouse_move_drag_difftest`, `console::line_at_mouse_difftest`

With `t = 1 − y`, a click passes when `1 − height > y` (`0x63c0d7` to `0x63c0eb`). The resize band is
`min(height, 1) − 0.75·size ≤ t ≤ height` (`0x63c0f1` to `0x63c148`). The line under the cursor is
`idx = ftol((height − t) / size)` (`0x63c22c`, `0x63c232`): 1 is the input line, otherwise the list is walked from
the anchor `0xc4e9b4`, one older line per step, and 0 or less gives the anchor. The band is
`y1 = 1 − (height − 0.75·size − size − idx'·size)` with `idx' = ftol((height − t)/size − 1)` and `y0 = y1 − size`
(`0x63c189` to `0x63c1e5`). `0x63c290` takes the lower and higher of the two x values, clamps them to 0 and 1,
and measures the characters within each width (`0x5c2190`); a left edge under 0.015 becomes 0. The tests run
`0x63c0b0` and `0x63c400` whole with callees stubbed, and `0x63c223` to `0x63c23d` of `0x63c220`.

## Selection copy

Ctrl+C copies the selected span, at most 127 bytes from the span's first character, to the clipboard; the
span is the characters between `0xc4ec48` and `0xc4ec4c` of the copied line. Ctrl+V pastes the clipboard
text into the input line and releases it. Any other key but Tab, Shift, Alt or a Ctrl chord clears the
selection and the Tab cycle.

Addresses: `0x7a78c0`, `0x7a77b0`, `0x63c58c`, `0x63c551`, `0x63c75d`, `0xc4eb00`, `0xc4ec48`, `0xc4ec4c`, `0xc4ec28`
Confidence: verified
Proof: one
Test: none

`0x63c596` tests the first byte of the copy buffer; `0x63c59f` to `0x63c5ac` take `end − start`, capped at 0x7f;
`0x63c5d7` calls `0x7a78c0` with the bytes. `0x63c55b` gets the clipboard string (`0x7a77b0`), pastes it with
`0x63bee0` and frees it (`0x63c57d`). The clearing test is `0x63c75d` to `0x63c77c`: key 0x203, 0 and 2 and
`modifiers == 2` skip it.

## Paint

Two layers draw the panel, created at start-up with the rectangle `0x864544` (left, lower edge, right, upper
edge; 0, 0, 1, 1 at first): the background at priority 6.0 and the text at 7.0. Both draw only while the lower
edge is below 1.0. The background is a quad in class 8's colour, and in the selection state a second quad in
class 7's colour. The text layer runs the blink clock, places the input string at
`(left, 0.75·size + lower edge)`, draws the caret, then the older lines upward, one size apart while their y
is below 1.0. A line with no string, and the input line, take no row.

Addresses: `0x63b5c0`, `0x63b5f0`, `0x63b710`, `0x63b800`, `0x63b990`, `0x864544`, `0x864548`, `0x86454c`, `0x864550`, `0xc4e6f4`, `0xc4e6f8`, `0xc4eac0`, `0xc4eaf4`, `0xc4eaf8`, `0x44d340`, `0x5c2050`, `0x5c2770`, `0x5c1e60`, `0x5c1ef0`, `0x5c1f00`, `0x63b806`, `0x63b868`, `0x63b87e`, `0x63b89c`, `0x63b9af`, `0x63ba01`, `0xc4ec10`
Confidence: verified
Proof: one
Test: `console::paint_text_layout_difftest`, `console::draw_caret_difftest`

The text layer adds the frame time to `0xc4e6f4`; with the phase `0xc4e6f8` set it flips at 0.3 s, with the
phase clear at 0.2 s, and zeroes the clock (`0x63b81d` to `0x63b863`). Nothing reads the phase: its only
load is `0x63b823` and its only store `0x63b863` (a byte scan of the image finds no other reference), so the
caret is drawn every frame its cursor index is nonzero. The caret is a quad `(x, y)` to `(x + 2·scaleX, y + size)`
in class 1's colour (`0x63b9bc` to `0x63b9fe`), where `x, y` is the pen after the measured prefix (`0x5c2050`).
The tests run `0x63b806` to `0x63b868` and `0x63b87e` to `0x63b89c` of the text layer and `0x63b9af` to
`0x63ba01` of the caret.

## Slide

Each frame (category 5) the panel's lower edge `0x864548` moves toward its target at 5 screen units a second.
The target is 1.0 while closed and `min(1, 1 − height)` while open, and never below 0. While the resize-drag flag
(`0xc4ec38`) is set, the edge jumps to it (`0x63bdea`). The new edge is clamped so it does not pass the target, and both layers get the
rectangle again. A pending `repeat` command runs once a frame.

Addresses: `0x63bd50`, `0x4429c0`, `0x639250`, `0xc4eabc`, `0xc4e960`, `0xc4eac8`, `0xc4ec10`
Confidence: verified
Proof: one
Test: `console::slide_animate_difftest`

`0x63bd60` to `0x63bdb2` compute the target; `0x63bddc` returns when the edge equals it (`test ah,0x44; jnp`).
The step is `direction · dt · 5.0` with the direction −1.0 when the edge is above the target and 1.0
otherwise (`0x63be0d`, `0x63be15`); closing clamps with `min` and opening with `max` (`0x63be4b`, `0x63be39`).
`repeat <n> <command>` (`0x639250`) stores a count and the command in `0xc4eabc` and `0xc4e960`; the slide
runs `0x63ce00` and decrements while the count and command are set (`0x63bdb2` to `0x63bdd0`). The test
runs the function whole with the command count held at 0.

## Start-up and shutdown

`ConsoleScreenInitialize` (`0x63b460`, at context start) reads the window rectangle, makes the scale
factors, loads `Fonts\ARIALN.ttf`, creates the two layers, registers the eight handlers and the 13 console
commands, installs the confirm-close callback, runs `ver` and creates the text batch. `0x63c9b0` undoes
it: it clears the callback, unregisters the handlers, closes the font and layers, and frees all lines,
the batch and the input string. The confirm-close callback `0x63c9a0` closes the context and answers 0.

Addresses: `0x63b460`, `0x63c9b0`, `0x63c9a0`, `0x63cdf0`, `0x63ca80`, `0x58a240`, `0x442800`, `0x41fe40`, `0xc4eac0`, `0xc4ea8c`, `0xc4e9b0`, `0xc4e668`, `0x63b48a`, `0x63b4e6`, `0xc4ec18`
Confidence: verified
Proof: one
Test: `console::console_scale_init_difftest`

`0x58a240` fills four floats; `0xc4eac0` is `1 / (r3 − r1)` and `0xc4ea8c` is `1 / (r2 − r0)`, each 1.0 when the
difference is 0 (`0x63b48a` to `0x63b4e6`; a NaN divides). The layers are `0x442800` calls with the
rectangle `0x864544`, flags 3 and the priorities `0x40c00000` and `0x40e00000`, callbacks `0x63b5c0` and
`0x63b800`, handles `0xc4e9b0` and `0xc4e668`. The commands are the 13 of the table `0x80e148`
(`commands#screen-commands`), category 2. The test runs `0x63b48a` to `0x63b4e6`.
