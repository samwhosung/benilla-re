# ui: protected actions

The gate that decides whether a script may run an action reserved for the player's own input,
such as moving. It sits in `WoW\Source\Ui` beside the Lua context word (`ui/luastate#the-context-word`)
and is called by the input functions (`input#toggle-run`).

Roots: none

## Protected action gate

`0x494a50(tag)` answers whether an action of a kind may run. The Lua context word `0xceeac0` decides first. A nonzero (script) context
with tag 0 or 11 always returns 0, signalling the forbidden-action event. A nonzero context with
tag 2, 4 or 10 returns 1 only while one of the three input-event flags below is set; any other tag
returns 1. A zero (client) context with tag 0 or 11 returns 1 only while a flag is set; any other
tag returns 1.

Addresses: `0x494a50`, `0x494a5a`, `0x494a7b`, `0x494a84`, `0x494a92`, `0x494b11`, `0x494b1a`, `0x494b40`, `0x494ab8`, `0x494b48`, `0x494b54`, `0xceeac0`
Confidence: verified
Proof: one
Test: none

`0x494a5a je 0x494b11` takes the zero context. `0x494a7b cmp esi,0xb; ja 0x494b11` and the byte table
`0x494b54` (12 entries `0 2 1 2 1 2 2 2 2 2 1 0`) index the three arms of `0x494b48`: arm 0 `0x494ab8`
(the refusal), arm 1 `0x494a92` (the flags test) and arm 2 `0x494b11`. `0x494b11` returns 1 at
`0x494b40` for any tag but 0 and 11, which test the flags at `0x494b1a`.

## Input event flags

Three flags record that code runs inside a real input event, and all three are read by the gate:
`0xcf4dc4` is 1 while an edit box runs its enter script; `0xcf4da0` is 1 while a frame's mouse
down or mouse up handler runs; and a dword at `+0xd8` of the object at `0xb71290` is 1 while the
binding runner `0x4b7990` runs a key binding, which also zeroes the Lua context. So a key binding, a click handler and
an enter script can move the player, and a macro or an add-on's own call cannot.

Addresses: `0x77e430`, `0x779890`, `0xcf4dc4`, `0xcf4da0`, `0xb71290`, `0x4b7990`, `0x4b6c50`, `0x483bbb`, `0x77b438`, `0x77b459`, `0x77b160`, `0x779275`, `0x77928b`, `0x779352`, `0x7793b8`, `0x779210`, `0x7792d0`, `0x4b7aee`, `0x4b7b02`, `0x4b7b11`, `0x4b7b21`, `0x4b6d7c`, `0x77a328`, `0x879c8c`
Confidence: verified
Proof: one
Test: none

`0x77e430` and `0x779890` load `0xcf4dc4` and `0xcf4da0` and return. Writes to `0xcf4dc4`: 1 at
`0x77b438` and 0 at `0x77b459`, around the call of the script in slot `+0x3f8` (named
`OnEnterPressed` by the string `0x879c8c` pushed at `0x77a328`). Writes to
`0xcf4da0`: `0x779275` (1) and `0x77928b` (0) around the virtual at `[vtable+0x94]`, `0x779352` and
`0x7793b8` around the next call. Writes to `[0xb71290]+0xd8`: 0 at `0x4b6d7c` (the object's constructor `0x4b6c50`), 1 at `0x4b7b02` and 0
at `0x4b7b11`, in `0x4b7990`, which finds a binding by command name and is called from the key
handlers (`0x483bbb`). `0x4b7aee` zeroes `0xceeac0` and `0x4b7b21` restores it, around the same
call. Each global has exactly these writers in a scan of the listing (control: `0xceeac0`'s stores
are found by the same pattern).
