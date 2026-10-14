# ui: Model and its kinds

Where a `Model` frame draws its scene among the interface's layers, how its animation clock runs,
how its sequence is set, and how the game's model kinds and the stock cooldown spiral sit on it.

A `CSimpleModel` is a frame with the vtable 0x81c608 that holds an M2 model at `+0x318`. The
game's `PlayerModel`, `DressUpModel` and `TabardModel` are model frames whose own Lua methods come
first: a `DressUpModel` or a `TabardModel` looks a method up in its own table, then
`PlayerModel`'s, `Model`'s and `Frame`'s. The scene, the camera and the model's own animation
belong to the model system (`models`).

Roots: `0x76c8e0`, `0x76f840`, `0x506230`, `0x5050a0`, `0x503ba0`

## Where the scene draws

A model frame's scene draws inside the interface's batches, not in a pass of its own: when its
frame's ARTWORK layer (2) is emitted and it holds a model, it adds a draw callback to that batch.
A batch draws its quads, then its strings, then its callbacks, so every BACKGROUND, BORDER and
ARTWORK region of the frames in that stratum and level draws under the scene, and every OVERLAY
and HIGHLIGHT region over it.

Addresses: `0x76d160`, `0x76d170`, `0x76d175`, `0x76d17f`, `0x76d18c`, `0x76d240`, `0x773110`, `0x76fb00`, `0x76fc0f`, `0x76fc2e`
Confidence: verified
Proof: one
Test: none

`0x76d160`, slot 17 of the model vtable, calls the frame's `0x76b3a0` (`0x76d170`) and then, when
`+0x318` is set (`0x76d175`) and the layer is 2 (`0x76d17f`), `0x773110(0x76d240, this)` on the
batch (`0x76d18c`), which `0x773110` links into the batch's list at `+0x20`. The batch drain
`0x76fb00` draws the quads of `+0x10`, then the strings of `+0x18` through `0x5c1ef0`
(`0x76fc0f`), then calls each callback of `+0x24` (`0x76fc2e`); nothing else names `0x76d240`.

## The animation clock

Each update of a visible model frame advances its own scene's millisecond clock by the frame time
times 1000, truncated, not rounded, so the sub-millisecond part of every frame is lost: the
model's animation runs slightly slower than the wall clock, and not at all at frame times below a
millisecond.

Addresses: `0x76d7f0`, `0x76d7fa`, `0x76d843`, `0x76d846`, `0x76d854`, `0x76d85c`, `0x76d863`, `0x8022cc`
Confidence: verified
Proof: one
Test: none

`0x76d7f0`, slot 14, calls `0x76b2c0` (`0x76d7fa`), multiplies the frame time by 1000.0 at
`0x8022cc` (`0x76d846`), truncates with `0x40a2b0` (`0x76d854`), and passes the result to
`0x7074b0` on the scene that `0x76cfc0` returns (`0x76d85c`, `0x76d863`). The `fcom` at `0x76d84c`
leaves no branch; the conversion has no `+0.5`.

## Sequences

`SetSequence(seq)` plays the sequence from its start and `SetSequenceTime(seq, t)` from the time
`t` into it, both at rate 1.0; each needs a model. The stock `Cooldown` frames are model frames driven by
the stock `Cooldown.lua`: `CooldownFrame_SetTimer` sets sequence 0 and shows the frame, and its
`OnUpdateModel` and `OnAnimFinished` handlers (`xmltypes`, Handler names by type) do the rest.

Addresses: `0x76dec0`, `0x76df6a`, `0x76cf50`, `0x76cf70`, `0x76dfc0`, `0x76e08c`, `0x76cf80`, `0x76cfa2`, `0x7121a0`
Confidence: verified
Proof: one
Test: none

`0x76dec0` reads a number, truncates it and calls `0x76cf50` (`0x76df6a`), which calls `0x7121a0(−1,
seq, −1, 0, 1.0, 0, 1)` on `+0x318` (`0x76cf70`). `0x76dfc0` reads two numbers for `0x76cf80`
(`0x76e08c`), which passes `t` in place of the 0 (`0x76cfa2`). The cooldown's Lua is the stock
`Cooldown.lua`, not the engine's.

## Lua methods

`Model` has 23 methods at `0x878948`, registered by `0x76f840`. `PlayerModel` adds 3 at `0x84f1fc`
(`SetUnit`, `RefreshUnit`, `SetRotation`), registered by `0x506230`; `DressUpModel` 3 at
`0x84f190` (`Undress`, `Dress`, `TryOn`), registered by `0x5050a0`; `TabardModel` 10 at
`0x84ee40`, registered by `0x503ba0`.

Addresses: `0x878948`, `0x76f840`, `0x76f870`, `0x84f1fc`, `0x506230`, `0x506260`, `0x84f190`, `0x5050a0`, `0x5050d0`, `0x84ee40`, `0x503ba0`, `0x503bd0`, `0x76d950`, `0x76da50`, `0x76db20`, `0x76dc00`, `0x76dce0`, `0x76ddd0`, `0x76dec0`, `0x76dfc0`, `0x76e0e0`, `0x76e1e0`, `0x76e7d0`, `0x76ea40`, `0x76eb20`, `0x76ebe0`, `0x76eca0`, `0x76ed70`, `0x76ee60`, `0x76f080`, `0x76f1e0`, `0x76f2d0`, `0x76f390`, `0x76f480`, `0x76f540`, `0x505d70`, `0x505e40`, `0x505f00`, `0x504c00`, `0x504cd0`, `0x504d90`, `0x502b10`, `0x502bd0`, `0x502ca0`, `0x502dc0`, `0x502ea0`, `0x502f80`, `0x503070`, `0x503160`, `0x503540`, `0x503910`
Confidence: verified
Proof: one
Test: none

The tables are pairs of a name and a function: `Model` from `SetModel` (`0x76d950`) to `ClearFog`
(`0x76f540`), `TabardModel` from `InitializeTabardColors` (`0x502b10`) to `CanSaveTabardNow`
(`0x503910`). The registrars call `0x701d80` with 0x17, 3, 3 and 0xa. Slot 2 of each looks in its
own hash: `0x5050d0` and `0x503bd0` then call `0x506260`, which calls `0x76f870`, which calls
`0x778590`.
