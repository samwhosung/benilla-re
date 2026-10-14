# ui: key bindings

How a key or mouse press becomes a command: the binding table and its Lua functions, the chord string the
client builds from an event, the lookup with its one modifier fallback, what a command runs, how a
binding is set and refused, and the edit-box gate that lets arrow keys through to the game.

The code is `UIBindings.cpp`, `0x4b6050` to `0x4b8300`, with a manager object at `0xb71290`, and the
world frame's input handlers (`0x483b70` to `0x483d90`). The Windows messages and the key code
translation are in `core/input` and `core/events`; the frame side that hands a key to the world frame is
`ui/input`; the Lua functions of mouse look and movement are in `ui/mouse`.

Roots: `0x4b6050`, `0x4b60b0`, `0x4b7490`, `0x4b7990`, `0x4b7b50`, `0x4b7f40`, `0x483b70`, `0x483bd0`, `0x483c40`, `0x483cb0`, `0x483d20`, `0x77b160`, `0x7996e0`, `0x799790`

## Bindings.xml

`Interface\FrameXML\Bindings.xml` is loaded by `0x4b6f70`. A `<Binding>` is read only if its `debug`
attribute is absent or false and its `platform` attribute is absent or equals `windows` without case;
any other node is skipped whole. A node without a name raises a load error. It keeps the name, a
display ordinal from one counter shared by normal and header rows, and the compiled script. A `hidden`
attribute that is true takes the ordinal from a second counter, negated. A `header` attribute adds a
row named `HEADER_<header>` with no script. `runOnUp` (bool, default false) says the command also runs
when its key is released.

Addresses: `0x4b6f70`, `0x4b70a4`, `0x4b70b6`, `0x4b70c3`, `0x4b70e5`, `0x4b73da`, `0x4b73e5`, `0x4b73f9`, `0x4b73fe`, `0x4b741e`, `0x4b745b`, `0x6f1b30`, `0x6f2cf0`, `0x704c70`, `0x490013`
Confidence: verified
Proof: one
Test: none

`0x4b70a4` fetches `debug`, `0x4b70c3` `platform`, and `0x4b70d3` to `0x4b70e5` compares it with the
string `windows` (`0x64a4c0`, so without case), a mismatch jumping to the next sibling at
`0x4b745b`. `0x4b73da` to `0x4b73e5` store the negated ordinal of a hidden row; `0x4b73f9` compiles the
script (`0x704c70`); `0x4b73fe` fetches `runOnUp` and `0x4b741e` stores it at `+0x20`. The file is
named at `0x490013`. `0x6f1b30` is the first-letter bool parse of `ui/actionuse#reading-a-script-flag`.

## Binding functions

Nine Lua functions come from the table at 0x846c88. `GetNumBindings` returns the count of displayed
commands. `GetBinding(index)` returns the command name at that ordinal, then every key bound to it; hidden
rows have no ordinal. `GetBindingKey(command)` returns every key of a command, none when it has none.
`GetBindingAction(key)` returns the command bound to exactly that key and the empty string, not nil, when
none: it has no modifier fallback. `RunBinding(command[, "up"])` runs the command by name as a press unless
the second argument equals `up`, and returns nothing.

Addresses: `0x4b7f40`, `0x4b7f60`, `0x4b7fb5`, `0x4b8000`, `0x4b80a0`, `0x4b8120`, `0x4b8151`, `0x4b8180`, `0x4b81ae`, `0x4b81e0`, `0x4b7c80`, `0x4b7d20`, `0x4b7da0`
Confidence: verified
Proof: one
Test: none

`0x4b7f60` loads the name with `0x4b7c80` and loops `0x4b7d20(manager, name, i)` for the keys
(`0x4b7fb5`), returning one more than the key count. `0x4b8151` loads the empty string for a miss.
`0x4b81ae` compares the second argument with `up` (0x847074), `0x4b81e0` calls `0x4b7b50`.
Each raises its usage text for a bad argument.

## Binding sets

`GetCurrentBindingSet` returns the number at `+0xd0` of the manager. The live set is 3. `LoadBindings(set)`
for a set below 3 copies that set into the live set; `SaveBindings(set)` for set 1 or 2 stores the number at
`+0xd0` and copies the live set into it, and into set 2 as well when the set is 1; any other argument raises
its usage text. A copy marks its target in the bit mask at `+0xd4` and, for the live set, signals
`UPDATE_BINDINGS` (386).

Addresses: `0x4b8200`, `0x4b8220`, `0x4b8232`, `0x4b8260`, `0x4b8275`, `0x4b829d`, `0x4b82ad`, `0x4b6310`
Confidence: verified
Proof: one
Test: none

`0x4b8232 cmp eax,3` bounds the load. `0x4b6310(from, to)` copies set `from` to set `to`: `0x4b8220` pushes
`(set, 3)` and `0x4b8260` pushes `(3, set)`. `0x4b8275` and `0x4b827d` accept 1 and 2, `0x4b829d` stores the set
and `0x4b82ad` mirrors set 1 into 2. The copy sets bit `to` of `+0xd4` and signals when `to` is 3.

## Chord names

The client names a key press as modifier prefixes then a key name. The prefixes are `ALT-`, `CTRL-`
and `SHIFT-` in that order, each present only if held (mask bits 4, 2 and 1). A modifier key clears
its own bit, so Shift alone is `SHIFT`, never `SHIFT-SHIFT`, and no binding can be named that. Nothing is
built for a key code from 0x30e on, a repeat count above 1, or a name that overflows the 32-byte buffer.

Addresses: `0x4b66b0`, `0x4b6630`, `0x4b66c1`, `0x4b66cb`, `0x4b66db`, `0x4b6906`, `0x483b97`
Confidence: verified
Proof: one
Test: none

`0x4b6630(mask, &cursor, &remaining)` walks the 3-entry table at 0x846bd4 downward, so it emits ALT, CTRL, SHIFT.
`0x4b66cb cmp esi,2` and `0x4b66db` clear the key's own bit. `0x4b66c1 cmp [ecx+0x18],1; ja` refuses a
repeat count above 1. `0x4b6906` fails the build when the name does not fit, and the world frame rejects codes
from 0x30e on (`0x483b97`).

## Key names

The key name by code: 0 `SHIFT`, 1 `CTRL`, 2 `ALT`; 0x20 `SPACE`; 0x21 to 0xff the character in UTF-8;
0x100 to 0x109 `NUMPAD0` to `NUMPAD9`; 0x10a to 0x10e `NUMPADPLUS`, `NUMPADMINUS`, `NUMPADMULTIPLY`,
`NUMPADDIVIDE`, `NUMPADDECIMAL`; 0x200 to 0x20f `ESCAPE`, `ENTER`, `BACKSPACE`, `TAB`, `LEFT`, `UP`, `RIGHT`,
`DOWN`, `INSERT`, `DELETE`, `HOME`, `END`, `PAGEUP`, `PAGEDOWN`, `CAPSLOCK`, `NUMLOCK`; 0x212 `PRINTSCREEN`;
0x300 to 0x30b `F1` to `F12`; 0x30c `NUMPADEQUALS`; 0x30d `F16`. Every other code is `UNKNOWN`, so Scroll Lock
and Pause cannot be bound.

Addresses: `0x4b66b0`, `0x4b66f3`, `0x4b66fd`, `0x41abb0`
Confidence: verified
Proof: one
Test: none

The arm for 0x21 to 0xff (`0x4b66f3`, `0x4b66fd`) sends the code to the UTF-8 encoder `0x41abb0`; the rest is decoded
from the byte tables at 0x4b693c and 0x4b6968 with the jump tables at 0x4b6930, 0x4b6948 and 0x4b6a60.

## Mouse names

`0x4b6aa0` names a mouse event the same way, with the modifier mask from the event's `+0x1c`. A wheel event
is `MOUSEWHEELUP` for a delta of 0 or more and `MOUSEWHEELDOWN` below. The buttons are named by the single bit of the
button mask: bit 1 `BUTTON1`, bit 2 `BUTTON3`, bit 4 `BUTTON2`, bit 8 `BUTTON4`, bit 0x10 `BUTTON5`, and a
higher bit `BUTTON<n>` counted from bit 3. So the middle button is `BUTTON3` and the right is `BUTTON2`.

Addresses: `0x4b6aa0`, `0x4b6aa9`, `0x4b6ae5`, `0x4b6b41`, `0x4b6b5e`, `0x4b6b7b`, `0x4b6b98`, `0x4b6bb5`, `0x4b6bd2`
Confidence: verified
Proof: one
Test: none

The button arm is `movzx edx,[eax+0x4b6c40]` and `jmp [edx*4+0x4b6c28]` with `eax` the mask minus 1; the byte
table is `00 01 05 02 05 05 05 03 05 05 05 05 05 05 05 04` and the jump table `0x4b6b41`, `0x4b6b7b`, `0x4b6b5e`,
`0x4b6b98`, `0x4b6bb5`, `0x4b6bd2`; arm 1 pushes `BUTTON3` and arm 2 `BUTTON2`, so the arms are not in
the order of their addresses.

## Dispatch

`0x4b7990(chord, isDown)` finds the command for a chord. It returns 0 for an empty chord. It probes the live set
(set 3) for the exact chord, ignoring case. On a miss it cuts the chord after its first `-` and probes the rest
once. So `SHIFT-W` falls back to `W`; `ALT-CTRL-Z` falls back to `CTRL-Z`, never to `ALT-Z` or `Z`; `-`
alone probes the empty string. There is no third probe. A hit copies the action name, runs `0x4b7b50`
with `isDown` and returns its answer: 1 when a script ran, 0 otherwise. It runs with the context word
cleared and the manager's flag at `+0xd8` set, which makes protected functions legal, and restores both afterwards.

Addresses: `0x4b7990`, `0x4b799d`, `0x4b79b9`, `0x4b7a2b`, `0x4b7a2d`, `0x4b7a4d`, `0x4b7a57`, `0x4b7aca`, `0x4b7ad9`, `0x4b7b02`, `0x4b7b0c`, `0x4b7b11`, `0x4b7b41`, `0x64b3f0`, `0x64a3c0`, `0xceeac0`, `0xceeac4`
Confidence: verified
Proof: one
Test: none

`0x4b79b9` hashes the chord and walks the bucket chain with the case-insensitive compare; `0x4b7a2d` is `strchr(chord, '-')`
(`0x64a3c0`, forward), `0x4b7a4d` stores the suffix as the new key and `0x4b7a57` is the second and last hash. Both
miss paths go to `0x4b7b41`. `0x4b7ad9` to `0x4b7af4` save and zero the context word `0xceeac0` and raise the nesting
count; `0x4b7b02` sets `+0xd8`; `0x4b7b0c` calls the command layer; the restores follow at `0x4b7b11`. The hash folds
a to z to upper case and maps `/` to `\`.

## Running a command

`0x4b7b50(name, isDown)` finds the command by name in the second table (without case) and returns 0 for an
unknown name. A press always runs it. A release runs it only if the command has `runOnUp`, and otherwise does
nothing and returns 0. A run sets the global `keystate` to `down` or `up`, runs the command's compiled script
as client code, clears `keystate` and returns 1. The decision belongs to the command, not to the key or the
event, and applies equally to keys, mouse buttons and the wheel.

Addresses: `0x4b7b50`, `0x4b7bed`, `0x4b7bf1`, `0x4b7bf4`, `0x4b7c01`, `0x4b7c08`, `0x4b7c14`, `0x4b7c3b`, `0x4b7c47`, `0x4b7c6d`, `0x4b7c77`, `0x704cf0`, `0x7040d0`
Confidence: verified
Proof: one
Test: none

`0x4b7bed` to `0x4b7bf4` are the release test: with `isDown` 0 the command's `+0x20` must be nonzero, else
`0x4b7c77` returns 0. `0x4b7c01` and `0x4b7c08` choose the strings `down` and `up`, `0x4b7c14` names the global
`keystate`, `0x4b7c3b` runs the script (`0x704cf0`) and `0x4b7c47` onward set the global back to nil. `0x4b7c6d`
returns 1.

## Key press and release

The world frame handles a key press at `0x483b70`. The base frame's key script runs first (`0x76b7d0`); if it
consumes the key, nothing else happens. Otherwise the chord is built into a 32-byte slot of the frame indexed
by the key code (`this + 0x3f8 + code·0x20`) and `0x4b7990` runs with `isDown` 1. A release (`0x483bd0`)
replays the chord latched in that slot, rebuilding it only when the slot is empty, and clears the slot, so
modifiers changed while a key is held cannot strand a command. A held key fires once. A mouse release
rebuilds its name from the modifiers at that moment. A wheel notch dispatches its chord twice, down then up.

Addresses: `0x483b70`, `0x483b7b`, `0x483b9c`, `0x483ba9`, `0x483bbb`, `0x483bd0`, `0x483c0b`, `0x483c10`, `0x483c1a`, `0x483c2d`, `0x483c34`, `0x483c40`, `0x483cb0`, `0x483ce9`, `0x483d03`, `0x483d20`, `0x483d40`, `0x483d6f`, `0x483d82`, `0x76b7d0`, `0x46ff90`, `0x766070`, `0x424810`, `0x4248b3`
Confidence: verified
Proof: one
Test: none

`0x483b9c` computes the slot and `0x483ba9` builds into it (`0x4b66b0`, size 0x20); `0x483c10 test al,al; jne` skips
the rebuild for a latched release and `0x483c34` zeroes the slot. The mouse handlers (`0x483c40` and `0x483cb0`) build into a stack buffer
(`0x483ce9`) from the up event's own mask. `0x483d40` returns when the delta is 0, and `0x483d6f` and `0x483d82` are
the two dispatches of one notch. A repeat: the demux `0x424810` promotes category 8 to 0xa when the key is already
pressed (`0x4248b3`); category 0xa is handled by `0x766070`, which calls slot `+0x58` of the frame, and the world
frame's slot is the stub `0x46ff90` (`xor eax,eax; ret 4`) shared with the base frame
(`core/events#held-keys-and-repeats`).

## Setting a binding

`SetBinding(key[, command])` raises unless argument 1 is a string or number. It calls `0x4b7490(3, key, command)`
on the live set, a missing or non-string command meaning unbind, and returns the number 1 and signals
`UPDATE_BINDINGS` (386) on success, nil and no event on a refusal. The engine first replaces the whole-string
aliases `LEFTBRACKET`, `RIGHTBRACKET`, `SLASH`, `BACKSLASH`, `SEMICOLON`, `APOSTROPHE`, `COMMA`, `PERIOD`,
`TILDE`, `PLUS` and `MINUS` (compared without case) by `[`, `]`, `/`, `\`, `;`, `'`, `,`, `.`, a backquote, `=`
and `-`. Then `0x4b7890` is the only thing that can refuse. The command name is never checked.

Addresses: `0x4b8000`, `0x4b8008`, `0x4b8050`, `0x4b805b`, `0x4b8075`, `0x4b8083`, `0x4b7490`, `0x4b74a3`, `0x4b74b2`, `0x4b74c4`, `0x4b7616`, `0x4b761d`, `0x4b762c`, `0x4b77fb`, `0x4b7889`, `0x494a50`
Confidence: verified
Proof: one
Test: none

`0x4b8008 call 0x6f3510` tests argument 1, the usage raise is `0x4b8083`; `0x4b8050` splits success from refusal.
`0x4b7490` has five `ret 0xc`: `0x4b74b2` (the kind 8 check, which cannot fail), `0x4b74c4` (a null key, unreachable
from Lua), `0x4b762c` (the validator refused), `0x4b77fb` and `0x4b7889` (success). The alias text goes through a
one-byte buffer in zeroed data (`0x4b7616`). Nothing touches the table before the validator (`0x4b761d`).
`0x494a50` with kind 8 returns 1 on every path.

## Valid key strings

`0x4b7890` accepts a key string after stripping any number of leading `SHIFT-`, `CTRL-` and `ALT-`, in any order,
if the rest is exactly one character; or begins with `F`, `NUMPAD` or `BUTTON` followed only by digits; or is
one of 26 names (below). All compares are case-sensitive, so `shift-w` and `mousewheelup` are refused though the
lookup ignores case, while a lower-case single character is accepted. The empty string is accepted. A bare
modifier is refused.

Addresses: `0x4b7890`, `0x4b78a0`, `0x4b78b3`, `0x4b78ce`, `0x4b78d2`, `0x4b78de`, `0x4b78e6`, `0x4b7911`, `0x4b7924`, `0x4b7941`, `0x4b7953`, `0x4b795a`, `0x41aab0`, `0x64a480`
Confidence: verified
Proof: one
Test: none

The strip loop `0x4b78a0` to `0x4b78d0` walks the table at 0x846bec and restarts while any prefix matched; the
single-character arm is `0x41aab0` and the test `0x4b78e6 cmp byte [edx+esi],0`, which also accepts the empty
string; `0x4b7911` compares the prefixes of the table at 0x846c04 and `0x4b7924`, `0x4b7928` test for a digit, the run
being consumed from `0x4b795a`; `0x4b7941` is the compare in the loop over the 26 pointers from 0x846c20. All
three are `0x64a480` (`strncmp`); the alias step and the table lookup use the case-insensitive `0x64a4c0`.
The 26 names are `SPACE`, `NUMPADPLUS`, `NUMPADMINUS`, `NUMPADMULTIPLY`, `NUMPADDIVIDE`, `NUMPADDECIMAL`, `ESCAPE`,
`ENTER`, `BACKSPACE`, `TAB`, `LEFT`, `UP`, `RIGHT`, `DOWN`, `INSERT`, `DELETE`, `HOME`, `END`, `PAGEUP`, `PAGEDOWN`,
`NUMLOCK`, `CAPSLOCK`, `PRINTSCREEN`, `NUMPADEQUALS`, `MOUSEWHEELDOWN`, `MOUSEWHEELUP`. No test relates a wheel key to
`runOnUp`.

## Arrow keys in edit boxes

An edit box has a flag word at `+0x318` whose bit 0x10 is set by the XML attribute `ignoreArrows` and by
`SetAltArrowKeyMode(flag)`, the flag being argument 2 and true when absent; `GetAltArrowKeyMode` returns 1 or
nil. There is no `SetIgnoreArrows` function. The edit box's key press handler (`0x77b160`) returns 0, leaving the
key unconsumed, when the box is hidden; for `PRINTSCREEN` (0x212) always; and for LEFT, RIGHT, UP and DOWN
(0x204 to 0x207) when bit 0x10 is set and Alt is not held, before it considers focus. The key then falls
through the lower frames to the world frame's binding dispatch.

Addresses: `0x77b160`, `0x77b17f`, `0x77b18e`, `0x77b197`, `0x77b1b3`, `0x77b1b8`, `0x77b1c4`, `0x77b1c7`, `0x77a6b0`, `0x77a159`, `0x7996e0`, `0x79976e`, `0x799790`, `0x799815`, `0x41f8f0`, `0x765f96`
Confidence: verified
Proof: one
Test: none

`0x77b17f cmp eax,0x212` returns 0 at once; `0x77b18e test byte [esi+0x318],0x10` and the four compares `0x77b197`
to `0x77b1ac` select the arrows; `0x77b1b3` passes 2 to `0x41f8f0`, a test whether the key of that code is held
(code 2 is Alt in the translator of `core/input#key-codes`); a held Alt goes to `0x77b1c7`, the normal handling,
else `0x77b1c4` returns 0. The frame walk treats 0 as not consumed and goes on (`0x765f96`). The attribute is read
through the first-letter bool parse and written by `0x77a6b0` (`0x77a159`); the Lua function writes through the
same setter with the default 1 (`0x79976e`). The getter test is `0x799815`.
