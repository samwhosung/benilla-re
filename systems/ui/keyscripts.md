# ui: key scripts and typing

How a key press, a key release and a typed character reach a frame's scripts: the walk of the
top's key lists, the test a frame makes before it runs `OnKeyDown`, `OnKeyUp` or `OnChar`, the
key names the scripts receive, what an edit box does in the walk, and what that means for
movement keys held while the player types or the window loses focus. The bindings are
`keybindings`; the movement bits are `input`.

The top registers five key and character handlers (`input`), and every frame's base class
carries the three virtual methods that fire the scripts. The world frame is the last frame in the
walk and runs the binding system only if no frame ahead of it took the key.

Roots: `0x765f10`, `0x765fd0`, `0x766070`, `0x765df0`, `0x765e80`, `0x76b7d0`, `0x76bba0`, `0x76b760`, `0x766650`, `0x493050`

## The five handlers

Key down walks the key lists from stratum 8 to 0, each list from its first entry, and gives the
event to each frame's key-down slot until one answers nonzero, which then owns that key code. Key
up and key repeat do no walk: they go to the frame recorded for the code. Characters walk the
character lists the same way. Another channel, walked like characters, reaches only edit boxes.

| function | category | slot | walks |
|---|---|---|---|
| `0x765df0` | 1 characters | 0x5c | character lists |
| `0x765e80` | 0x1b input language | 0x54 | character lists |
| `0x765f10` | 8 key down | 0x60 | key lists |
| `0x765fd0` | 9 key up | 0x64 | the recorded frame |
| `0x766070` | 0xa repeat | 0x58 | the recorded frame |

Addresses: `0x765f10`, `0x765f56`, `0x765f61`, `0x765f70`, `0x765f7a`, `0x765f96`, `0x765fa2`, `0x765fb4`, `0x765fd0`, `0x765fde`, `0x765ff2`, `0x766025`, `0x766028`, `0x76603c`, `0x766048`, `0x76605b`, `0x766070`, `0x76607b`, `0x7660b8`, `0x765df0`, `0x765e80`, `0x764aa0`, `0x764ac3`, `0x46ff80`, `0x46ff90`, `0x765bf0`
Confidence: verified
Proof: one
Test: `ui::input_hittest_index_insert`

`0x765f10` starts at `root+0xf20` and subtracts 0x1c before each use, so the records are strata 8 to 0, and
a record is `root + 0xd10 + (stratum + 9·kind)·0x1c` (`0x764ac3`); it stores the cursor before the call, calls `[vt+0x60]` and on
nonzero stores the frame at `[root + 4·code + 0x84]` (`0x765fa2`) and stops; it answers 1 when nothing took the key. `0x765fd0`
reads that slot (`0x765fde`) and calls `[vt+0x64]`, answering 0 whatever the frame returned (`0x766028`). With no
frame it answers 1, except for code `0x212`, which has no key down and is offered to the walk `0x765f10`
(`0x766048`). The slot is written elsewhere only at `0x76605b`, which stores 0 on the leg where the slot is already null, so a release
with a frame in the slot leaves it set and the slot is the last taker of the code. On that null leg for code `0x212` the walk `0x765f10` may first store a frame at
`0x765fa2`, and `0x76605b` then clears it. The slots `0x46ff80` and `0x46ff90` of the base are `xor eax,eax; ret 4`.
Four instructions address the slot, `0x765fa2`, `0x765fde`, `0x76605b` and `0x76607b`.

`ui::input_hittest_index_insert` runs `0x764aa0` whole.

## The test a frame makes

A frame answers a key down or key up with a script only when it is visible, and answers the key down
with 1 when either its `OnKeyDown` or its `OnKeyUp` script is set, running only `OnKeyDown`. A key up needs
`OnKeyUp`. A character needs `OnChar`. The script's own result is never read, so a handler cannot decline.
A frame that merely has the keyboard enabled does not take keys.

Addresses: `0x76b7d0`, `0x76b7d6`, `0x76b7df`, `0x76b7e5`, `0x76b7f5`, `0x76b7fd`, `0x76ba0f`, `0x76ba12`, `0x76ba14`, `0x76ba1b`, `0x76ba25`, `0x76bba0`, `0x76bbaf`, `0x76bbc3`, `0x76bdf5`, `0x76b760`, `0x76b772`, `0x76b782`, `0x7026f0`, `0x76af00`, `0x76af38`, `0x7770a0`
Confidence: verified
Proof: one
Test: none

`0x76b7d0` returns 0 when `+0xd4` is 0 (`0x76b7df`) or both `+0x188` and `+0x190` are 0 (`0x76b7fd`); past that gate every
path ends in `mov eax,1` (`0x76ba25`) and the script fires only if `+0x188` is set (`0x76ba12`). A frame with only `OnKeyUp`
therefore consumes every key down and runs nothing. `0x76bba0` gates on `+0xd4` and `+0x190` only (`0x76bbaf`, `0x76bbc3`)
and `0x76b760` on `+0xd4` and `+0x180`. The call `0x7026f0` is a varargs thunk whose return the three ignore.
`0x76af00` sets a bit of `+0xcc` and enters the lists without touching a script slot.

## Key names

The argument of `OnKeyDown` and `OnKeyUp` is a string: the key's name without modifiers, from the same
table the binding chord builder uses (`keybindings`, "Key names"). The character itself stands for the codes
`0x21` to `0xff`, and any code outside the table is `UNKNOWN`. `OnChar` receives the character as UTF-8.

Addresses: `0x76b7d0`, `0x76b816`, `0x76b81b`, `0x76b8a9`, `0x76bba0`, `0x76b760`, `0x76b78d`, `0x41abb0`, `0x4b66b0`, `0x64a7f0`
Confidence: verified
Proof: one
Test: none

`0x76b7d0` and `0x76bba0` each decode the code with their own copy of the table, as the chord builder `0x4b66b0` does.
The jump table of the codes `0x206` to `0x212` is not in arm order: code `0x20e` reaches the arm that pushes `CAPSLOCK`
and `0x20f` the arm for `NUMLOCK`.
`0x41abb0` encodes a codepoint above `0x20` and up to `0xff` into a UTF-8 buffer, and `OnChar` encodes with it without
a range test (`0x76b78d`).


## The edit box in the walk

An edit box registers itself in the character, key and mouse lists when it is constructed, whatever
the `enableKeyboard` attribute says, and replaces the base slots without calling them. Focus is read nowhere
in the walk, only inside the edit box's own handlers: a box that is visible and has the focus, or has none and
nobody else does and auto focus is set, takes every key it is reached with, except `PRINTSCREEN` and, with
the arrow switch set, the arrow keys while Alt is not held. A frame ahead of it in the walk can take the key first.

Addresses: `0x779ce8`, `0x779cf2`, `0x779cfd`, `0x77a900`, `0x77a917`, `0x77a956`, `0x77b160`, `0x77b179`, `0x77b17f`, `0x77b18e`, `0x77b1c7`, `0x77b1e0`, `0x77b215`, `0x77b35e`, `0xcf4dc8`, `0x77e3d0`, `0x77e410`, `0x77afc0`, `0x77b770`, `0x41f8f0`
Confidence: verified
Proof: one
Test: none

The constructor calls `0x76af00` with kinds 0, 1 and 2 and `-1`. `0xcf4dc8` has twelve references, all in the edit box code,
and two writers, `0x77e3d0` and `0x77e410`. `0x77b160` declines at `0x77b172` (not visible), `0x77b187` (`0x212`) and in the arrow gate
(`0x77b18e`, which tests `0x41f8f0(2)`, the Alt key), then takes the focus test `0x77b1c7`/`0x77b215`; every processing arm ends in
`mov eax,1` including the default (`0x77b35e`). Slot 0x58 `0x77b770` sends a repeat on to the box's key down when it has the focus or takes it, so a held key types again; slot 0x54 `0x77afc0` takes the input method events the same way.

## Typing and movement

Typing does not stop movement by clearing anything. The focused box takes the key down, so the
walk never reaches the world frame and no binding starts. Nothing a focus change runs touches the
movement bits, and a key already held is released by its key up, which goes to the world frame
that took the press, whoever holds the focus at the release. A shown frame with a key script, such as the world map,
takes new key downs and nothing else. A held key never fires its binding twice: the world frame's repeat goes to
a base slot that does nothing.

Addresses: `0x77e3d0`, `0x77e410`, `0x77af50`, `0x483b70`, `0x483b7b`, `0x483bd0`, `0x4b7b50`, `0x4b7bf1`, `0x4b7bf4`, `0x4b7c6d`, `0x765fde`, `0x766070`, `0x8043e8`, `0x803960`, `0x513e20`, `0x513e50`, `0x494a50`
Confidence: verified
Proof: one
Test: none

The world frame records itself at `0x765fa2` only when `0x483b70` returned nonzero, which it does when `0x4b7b50` ran the command's
script (`0x4b7c6d`). The release `0x483bd0` replays the chord latched at the press and `0x4b7bf1` lets it run only for a
command with `runOnUp`. The focus code (`0x77e3d0`, `0x77e410`, `0x77af50`) calls only the IME refcount, the focus
scripts and a cursor refresh. The movement verbs gate on the taint word through `0x494a50`, not on focus.

## Deactivating the window

Losing the window's activation clears the eight movement direction bits at one instruction, the only
bulk release in the client, and re-sends the state; gaining it does nothing. Mouse look and autorun stay set. The same
event zeroes the modifier and button masks and the held-key set. A UI focus change has no such effect, and neither does
an OS focus message, which the window procedure drops.

Addresses: `0x766650`, `0x766657`, `0x766660`, `0x766687`, `0x765cf4`, `0x493050`, `0x48fe88`, `0x514490`, `0x51449e`, `0x5144a3`, `0x5144aa`, `0x5144b5`, `0x514640`, `0x5143e0`, `0x42d080`, `0x423cb0`, `0x423900`, `0x424790`, `0x4245b0`
Confidence: verified
Proof: one
Test: none

`0x42d080` queues the activation flag; `0x423cb0` zeroes the masks and raises category 2 with it. `0x765cf4` registers
`0x766650` on category 2; it calls `0x42c8a0`, then, when the root's callback `[root+0x1134]` is set, builds a five-dword record whose
last dword is the flag and calls it. `0x48fe88` installs `0x493050` there, once, in the game interface only, and it returns 1
after calling `0x514490`. `0x514490` skips on a nonzero flag (`0x51449e`); otherwise `and eax,0xfffff00f` (`0x5144a3`) clears bits 4
to 11 of the input word, and `0x514640` re-sends. A scan of the whole image for the dword `0xfffff00f` finds one
immediate, `0x5144a3`; the callers of `0x514490` and `0x493050` are single.
