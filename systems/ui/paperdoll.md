# ui: the paper doll and its model events

When the interface is told that a unit's model or portrait changed, and what that does to an open character sheet. The
pane copies the model when it is shown (`ui/playermodel#the-pane-model`) and never reads the live model again, so the
event is the only way the doll follows a change while it stays open. This is the portrait list of `PortraitButton.cpp`
(`0x524cd0` to `0x524f60`) and the code that marks a unit's portrait dirty.

Roots: `0x524cd0`, `0x524e10`, `0x491a50`, `0x51bdb0`

## Portrait events

`UNIT_PORTRAIT_UPDATE` (0xb8) and `UNIT_MODEL_CHANGED` (0xb9) are signalled together from one place, the drain of the
pending-portrait list, once per drained unit, for every unit token that names it. Nothing else in the client signals
0xb9. The generic field watchers `0x51bdb0` register `0x51bd50` for event ids up to 0xb5 only, so a unit field never
raises either event by itself.

Addresses: `0x524cd0`, `0x524de4`, `0x524de9`, `0x524df1`, `0x515e50`, `0x51bdb0`, `0x51bdb6`, `0x51bde0`, `0x51bd50`, `0x483228`
Confidence: verified
Proof: one
Test: none

The names are stored by static initializers (`0x51acf3` for 0xb9, one slot after 0xb8). `mov edx,0xb9` appears once in the
image, at `0x524de9`; the other immediates of 0xb9 are the movement opcode registrations (`0x60340b`, `0x604f6d`), two
message pushes (`0x5d403d`, `0x5d40c5`), a five-argument call (`0x615de3`) and a line number (`0x72ca10`). The callers
of the per-token signaller `0x515e50` take their id from an immediate, a power-type offset or the field index; none
reaches 0xb9. The watcher loop `0x51bdb6` to `0x51bde6` walks the name table by byte offset to `0x2d8`, which stops at id
0xb5. The drain has one caller, the per-frame interface step (`0x483228`).

## Draining the pending list

The drain returns at once unless two graphics readiness tests pass. For each pending GUID it looks up the unit and puts the
node back, firing nothing, unless the unit has: no portrait-invalidate latch (`+0xcd0`), no appearance latch (`+0xccc`),
a ready component texture (or none), a model (`+0xd8`) and a resident skin. A GUID that resolves to no unit is drained and
fires anyway. A drained node moves to the free list.

Addresses: `0x524cd0`, `0x524cd8`, `0x524ce5`, `0x524d37`, `0x524d42`, `0x524d50`, `0x524d5e`, `0x524d6a`, `0x524d77`, `0x524d87`, `0x524d90`, `0x524dc6`, `0x58a260`, `0x58a280`, `0x477860`, `0x710450`, `0xc0cdd4`, `0xc0ceb8`
Confidence: verified
Proof: one
Test: none

The two readiness calls are `0x58a260` and `0x58a280`, both guarded by the same graphics object cell. `0x524d37 call 0x468460` uses the
unit mask 8. A null unit takes `0x524d40 je 0x524d90`, past the tests. The component test is `[unit+0xd30] == 0` or
`0x477860(component, 0)` returns true; the skin test is `0x710450(model, 0, 1)`. The pending list head is `0xc0cdd4`
and the free list head `0xc0ceb8`.

## Dirty marks

`0x524e10(guid)` marks a unit's portrait dirty and queues it if absent; `0x491a50` is a tail jump to it. Twelve calls
mark: two direct and ten through the thunk.

| call | in | when it marks |
|---|---|---|
| `0x5df119` | `0x5dee30` visible item changed | every time |
| `0x5ffbbe` | `0x5ffb10` sheathe event | every time |
| `0x5eeda9` | `0x5eed50` enchant visual | a weapon with a visual id other than -1 |
| `0x5d991b` | `0x5d97c0` item field watcher | every time |
| `0x5eea22` | `0x5ee990` player flags changed | the hide-helm and hide-cloak bits changed |
| `0x5fb5d0` | `0x5fb200` character component build | every time |
| `0x607e0f` | `0x607da0` model update | the portrait latch was set |
| `0x5e081e`, `0x5e08c9` | `0x5e07d0`, `0x5e0830` component refresh and tabard reapply | every time |
| `0x5e587f` | `0x5e5720` party record changes | a record bit is set |
| `0x525885` | `0x524f60` portrait bake | the bake was not ready |
| `0x51a10c` | `0x51a100` portrait texture callback | the texture arrived |

Addresses: `0x524e10`, `0x491a50`, `0x5df119`, `0x5ffbbe`, `0x5eeda9`, `0x5d991b`, `0x5eea22`, `0x5fb5d0`, `0x607e0f`, `0x5e081e`, `0x5e08c9`, `0x5e587f`, `0x525885`, `0x51a10c`, `0x5dee30`, `0x5ffb10`, `0x5eed50`, `0x5d97c0`, `0x5ee990`, `0x5fb200`, `0x607da0`, `0x5e07d0`, `0x5e0830`, `0x5e5720`, `0x524f60`, `0x51a100`
Confidence: verified
Proof: one
Test: none

`0x5dee30` and `0x5ffb10` each have one `ret` (`0x5df12b`, `0x5ffbcb`) and the mark is the last call before it, so their
marks are unconditional. `0x5eed50` returns before `0x5eeda9` when `0x62ec70` gives -1 or the hand attachment is absent
(`0x5eed7a`, `0x5eed8d`). `0x5eea0c test ah,0xc` selects the flag bits. `0x5e5878 test bh,0x20` gates `0x5e587f`.
`0x525885` only requeues what `0x524f60` could not bake.

## Visible items and the sheathe event

Equipping or removing a visible item always reaches the mark: the visible-item block watcher `0x5e2810` calls `0x5dee30`,
which re-dresses the slot and marks. A player's own draw or stow, which plays the sheath clip, marks at the clip's keyframe
event `$SHL` or `$SHR`, so an open sheet's doll changes after the weapon snaps. A transition that is applied at once
(`SetSheatheState` with a nonzero second argument: an attack start, a server byte, a stand-state change) takes the
re-attach path instead, which marks only for a weapon with an enchant visual.

Addresses: `0x5e2810`, `0x5dee30`, `0x5ffb10`, `0x5ffbd0`, `0x611cf0`, `0x611de3`, `0x611dea`, `0x611df7`, `0x611770`, `0x611b60`, `0x60b590`, `0x60b770`, `0x60b96a`, `0x5eed00`, `0x5eed50`
Confidence: verified
Proof: one
Test: none

The tail of `0x611cf0` is `cmp [ebp+0xc],edi; je 0x611df7`: a zero argument runs the animated ceremony `0x611b60`, a
nonzero one the snap `0x611770` (`0x611de3`, `0x611dea`). The snap moves the weapon through `0x60b590`, which on a real
transition reaches `0x60b770`; for a player that calls `0x5eed00` (`0x60b96a`) and so `0x5eed50`. The keyframe handler is
`objectlayer/unitanim#model-events`; the states are `objectlayer/sheathing`.

## Latches that hold a unit back

Two per-unit latches keep the drain from firing: `+0xccc` (the appearance is being rebuilt) and `+0xcd0` (the portrait must
be invalidated). A display id change sets both, as do the unit constructor and its post-init; `0x607da0` clears them in
turn: it rebuilds the component once the model is ready, then marks the portrait when `+0xcd0` was set.

Addresses: `0x60ad28`, `0x60ad2e`, `0x5faf0e`, `0x5faf14`, `0x5fb9d1`, `0x5fb9d7`, `0x5fb228`, `0x607dea`, `0x607e14`, `0x607da0`, `0x7103d0`, `0x60abe0`, `0x5fad50`, `0x5fb880`
Confidence: verified
Proof: one
Test: none

The stores to `+0xccc` are `0x5faf0e`, `0x5fb228` (the component build clearing its own latch), `0x5fb9d1`, `0x607dea` and
`0x60ad28`; the stores to `+0xcd0` are `0x5faf14`, `0x5fb9d7`, `0x607e14` and `0x60ad2e`. Other readers are the drain and
the portrait bake (`0x52510f`, `0x52511d`).

## The pane's attachment reset

A duplicated model is cleaned before it is shown: `0x47a230` detaches the effect, name, blood and spell-socket
attachments (ids 0xf, 0x10, 0x11, 0x12, 0x1d, 0x13 to 0x19, 0x22 and 0x23, with 0xf detached twice), sets the additive
colour to black, the multiplicative colour to white and the alpha to 1.0. The worn equipment, the hand and sheath
points are not touched, so a pane keeps armor and weapons but loses a nocked arrow, a bow string and any glow.

Addresses: `0x47a230`, `0x47a237`, `0x47a240`, `0x47a249`, `0x47a252`, `0x47a25b`, `0x47a264`, `0x47a26d`, `0x47a276`, `0x47a27f`, `0x47a288`, `0x47a291`, `0x47a29a`, `0x47a2a3`, `0x47a2ac`, `0x47a2b5`, `0x47a2d9`, `0x47a2f9`, `0x47a305`, `0x7130a0`, `0x710d40`, `0x710cf0`, `0x710cb0`
Confidence: verified
Proof: one
Test: none

Each detach is `push id; mov ecx,esi; call 0x7130a0` (`0x47a237` to `0x47a2b5`). The colours are `0x710d40(0,0,0)`, `0x710cf0(1,1,1)` and
`0x710cb0(1.0)`. The string of a bow is a draw callback installed on the bow model by `0x611e10` through `0x710db0`; none
of its nine callers is in interface code and the duplicate starts with the callback slot zero (`ui/playermodel`,
`objectlayer/unitdisplay#the-bowstring`).
