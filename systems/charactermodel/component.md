# charactermodel: component

The component object, its pool, how one is built, cloned and deleted, how its section textures load, and the driver that
composites it. The driver runs for every visible component each frame it is asked to; `customization`, `equipment` and
`geosets` change the state it reads, and `composite` is what its handlers blit.

Roots: none

## Component object

A component is 0x4e4 bytes from a pool. The first 0x16c bytes after the link, from `+0x18`, are copied as one block from a
build descriptor, so the descriptor has the layout of those fields.

| offset | holds |
|---|---|
| `+0x00`, `+0x04` | pending-list link: next, and previous with a tag bit |
| `+0x08` to `+0x0c` | flags: full rebuild, sections pending, geoset dirty, race flag, tabard preview |
| `+0x10`, `+0x14` | dirty mask of the ten groups, -1 for all; pool handle |
| `+0x18` to `+0x34` | race, sex, hair colour, skin colour, unused, face, facial hair, hair style |
| `+0x38`, `+0x3c`, `+0x3d` | model; baked skin flag; baked skin path |
| `+0x144` to `+0x180` | geoset bases: 16 dwords |
| `+0x184` | composite target texture |
| `+0x188`, `+0x228` | skin cells (5 × 8 dwords); equipment cells (10 rows of 0x40) |
| `+0x4a8`, `+0x4d8` | item records by body slot (12 dwords); held items |

Addresses: `0x476810`, `0x476b90`, `0x476cb0`, `0x4783f0`, `0x478790`, `0x4785b0`, `0x478660`, `0x4784c0`, `0x477520`, `0x476eb0`, `0x47a950`, `0x47a900`, `0x479e80`
Confidence: verified
Proof: one
Test: none

The constructor `0x476810` zeroes `+0x00`, `+0x04` and `+0x18` to `+0x38`, sets the bases `1, 0x65, 0xc9, 0x12d, 0x191, 0x1f5, 0x259, 0x2be` at `+0x144` to
`+0x160` and `0x321, 0x385, 0x3e9, 0x44d, 0x4b1, 0x515, 0x579, 0x5dd` at `+0x164` to `+0x180`, builds the cell rows with `0x47a900` (8 zero dwords) and
`0x47a950` (16), sets `+0x08`, `+0x09` and `+0x0a` to 1 and `+0x10` to -1, and zeroes the 12 dwords at `+0x4a8`. The setters store `+0x20` (`0x4783f0`), `+0x24` (`0x478790`),
`+0x2c` (`0x4785b0`), `+0x30` (`0x478660`) and `+0x34` (`0x4784c0`). No instruction of the file reads `+0x09` or `+0x28`.

## Pool and statics

`0x476020` runs once at start-up. It allocates the 8-byte pool record, creates the pool of 0x4e4-byte blocks (0x20 to a chunk) and stores it in
`0xb42720`, stores the end of each path buffer (`0xb422f0` for `0xb42510`, `0xb42508` for `0xb42618`), fills the ten group handlers and the ten
choosers, creates the composite target table `0xb422d8` (`0x44af90(5, 256, 256)`), resets the load counters (`0x475f20`), then builds the CharSections grid and the facial hair count table (`customization#grid-build`). It returns 0 when the
density test of the grid build fails. `0x4766e0` frees the grid, the table, the composite target table and the pool at shutdown.

Addresses: `0x476020`, `0x4766e0`, `0x4766c0`, `0x4766d0`, `0x476800`, `0x475f20`, `0x476188`, `0x44af90`, `0x463920`, `0x6462e0`, `0x838be4`, `0xb42720`, `0xb422f0`, `0xb42510`, `0xb42508`, `0xb42618`, `0xb422d8`, `0xb4231c`, `0xb4244c`, `0xb4250c`
Confidence: verified
Proof: one
Test: none

`0x476037 call 0x6462e0` allocates 8 bytes and `0x476051 call 0x463920` creates the pool with block size `0x4e4`, count `0x20` and the name `CCharacterComponent` (`0x838be4`). The record is `{pool, byte 0}` (`0x476056`, `0x476058`). `0x47606e call 0x4766d0` is a bare `ret`, so
both cursors sit at the start of their buffers (`0x476088`, `0x4760a1`). `0x4766c0` zeroes two dwords and is the constructor the grid's vector construction runs per entry. `0x476800`
and `0x4766d0` are bare `ret`s. The shutdown frees the grid's records (`0x646430` at `0x476726`, `0x47674f`), `0xb4231c`, the target table (`0x44b110`), the counts (`0x4767d1`) and the pool (`0x4767ed`).
The init is called once from `0x402b07`, the shutdown from `0x4036f2`.

## Create, build and delete

`0x475fb0` takes a block from the pool, constructs it with `0x476810` and stores its handle at `+0x14`. `0x476b90` builds a component from a
descriptor: it releases the old model, copies 91 dwords to `+0x18`, zeroes both cell grids and `+0x4d8` to `+0x4e3`, and fails (returns 0) unless the race is in
range and CharSections has a skin row for it. It then marks a full rebuild, drops a stale target, sets the race flag from the race's flags bit 1, loads the baked skin when `+0x3c` is
set, and applies skin, hair style and facial hair. `0x476000` destroys with `0x476ac0` and frees the block.

Addresses: `0x475fb0`, `0x476000`, `0x476ac0`, `0x476b90`, `0x476810`, `0x476960`, `0x477c20`, `0x479580`, `0x478790`, `0x4784c0`, `0x478660`, `0x41aed0`, `0x7103a0`, `0x463a60`, `0x463b00`, `0x47a920`, `0x47a970`
Confidence: verified
Proof: one
Test: none

`0x475fcd call 0x463a60` returns the block and handle; `0x475fdb` to `0x475fec` skip the constructor only when the pool record's byte is set, and nothing sets it. `0x476bb3 rep movs` copies `0x5b` dwords. The checks are
`0x476be3 jl`/`0x476bef jg` on `[0xc0dee4]` and the call `0x477c20(0, 0, skin)` at `0x476bff`. The race flag is `test cl,2; setg dl` on the race record's `+4` (`0x476c5a`). The build order is
`0x476c76` skin, `0x476c81` hair style (`0x4784c0`), `0x476c8e` facial hair. `0x476ac0` releases the target (`0x41aed0`), the model (`0x7103a0`), unlinks the pending-list entry and clears both grids (`0x47a970`, `0x47a920`).
`0x476960` is the constructor with a build call and nothing reaches it.

## Clone

`0x476cb0(model, source)` copies the appearance block of another component, takes the given model, marks the rebuild flags, and gives every non-null cell its own reference with `0x47b000`, with the
records copied beside it, then copies the 12 item records and three held-item dwords. A clone has `+0x10` at -1. It drops its target when it has no baked skin.

Addresses: `0x476cb0`, `0x47b000`, `0x41aed0`, `0x7103a0`
Confidence: verified
Proof: one
Test: none

`0x476cd5 rep movs` copies `0x5b` dwords from `source+0x18`; `0x476cda` stores the model; `0x476ce1` sets `+0x09` and `0x476cef`/`0x476cf4` copy `+0x0b`, `+0x0c`. The 5 × 7 skin cells copy at `0x476d17`
to `0x476d3c` and the 10 × 7 equipment cells at `0x476d68` to `0x476dac`, each through `0x47b000`, the record beside each cell copied by value (`0x476d7c`). `0x476dc1` copies the
item records and `0x476dd3` the held items. The only caller is `0x5043d5`.

## Section loading

A cell holds a texture wrapper made by `0x47ae30` from a path. `0x476eb0(force)` walks the 5 × 7 skin cells and polls each non-null cell with `0x47b150`; it returns 0 at the first one still
loading, else clears `+0x09` and returns 1. `0x476f30(force)` first builds a missing equipment cell from its record with `0x476e20`, then polls the cell, 10 rows by 7 columns. With `force` set it builds and loads every
cell with the blocking flag and always returns 1; without it, it returns 0 at the first cell not ready. Its polls are the second argument of `0x47b150`, which also copies the cell's six
descriptor dwords (`+0x1c` to `+0x30`) to the caller.

Addresses: `0x476eb0`, `0x476f30`, `0x47b150`, `0x47b120`, `0x47ae30`, `0x476e20`, `0x476ee5`, `0x476ef2`, `0x476f18`, `0x476f8f`, `0x477017`, `0x477025`
Confidence: verified
Proof: one
Test: none

`0x476ee5` to `0x476ef9` poll and bail; `0x476f18` clears `+0x09`. In `0x476f30`, `0x476f3b cmp [ebp+8],edx` splits on `force`: `0x476f70` to `0x476faf` is the forced walk (pushing 1 at `0x476f9f`),
`0x476fcc` onward the polling walk (pushing 0 at `0x477020`). The polling walk calls `0x476e20` only for a null cell with a record (`0x476fff` to `0x477017`), and a failed poll takes `0x477060` to return 0. `0x47b120`
returns the cell's texture data, or 0 for a null cell, a cell without an image or one with `+0xc4` set.

## Texture path names

`0x476e20(record, group)` names the section texture of an item record. The path is
`Item\TextureComponents\<dir>\<name>_U.blp` with `dir` the group's directory name (ArmUpperTexture, ArmLowerTexture, HandTexture,
TorsoUpperTexture, TorsoLowerTexture, LegUpperTexture, LegLowerTexture, FootTexture) and `name` the record's texture string `+0x38 + 4·group`. When that file does not exist, the letter
before `.blp` becomes the sex letter, M or F. It then makes the cell with kind 9. The `_U` name is tried first and wins when both exist.

Addresses: `0x476e20`, `0x476e64`, `0x476e8f`, `0x476e9d`, `0x648a10`, `0x64a7f0`, `0x838a94`, `0x838ab4`, `0x838abc`, `0x838c3c`
Confidence: verified
Proof: one
Test: none

`0x838a94` holds the eight directory names, pointers at 4-byte steps, with `M`, `F` and `U` at `0x838ab4`. `0x476e4c` pushes the format `0x838c3c` (`Item\TextureComponents\%s\%s_%s.blp`) with the third string
`[0x838abc]`, which is `U`. `0x476e64 call 0x648a10` tests the file; `0x476e6b jne` skips the patch when it exists. The patch writes `[0x838ab4 + sex·4]`'s first byte at `path + strlen − 5` (`0x476e8f`). The
call `0x47ae30` at `0x476e9d` runs once.

## Update driver

`0x477860(force)` brings a component up to date and returns whether the model may draw. A component with a baked skin only picks geosets. Otherwise, with no dirty mask and no
full rebuild it picks geosets; with a mask and no rebuild it loads the sections and, when they are all ready, composites the dirty groups; a full rebuild needs, unless `force`, a load counter under 0x100, and
returns 0 when any section is not ready. A composite creates the target on first use, runs the handler of every set bit, and uploads each handled tile on an incremental pass or the whole target on a full one.

Addresses: `0x477860`, `0x477520`, `0x476eb0`, `0x476f30`, `0x47b200`, `0x449cc0`, `0x44acf0`, `0x58ac70`, `0x58ad00`, `0x710ec0`, `0x477ba0`, `0x4778cd`, `0x4778fd`, `0x4779d5`, `0x477a2b`, `0x477a4a`, `0xb4271c`, `0x607da0`, `0x607e7c`, `0x4815d0`
Confidence: verified
Proof: one
Test: none

`0x477866` tests `+0x3c`; `0x47787b` to `0x477898` split on the mask and `+0x08`; `0x4778cd call 0x47b200` and `0x4778d2 cmp eax,0x100; jge 0x477a5c` return 0. The target is made at `0x47793e call 0x449cc0` (256 × 256), bound as model
texture type 1 (`0x477970 call 0x710ec0`). The handler call is `0x477999 call [ebx*4+0xb422f4]`. A full pass uploads once (`0x477a2b`), clears `+0x08`, `+0x10`, counts a composite in `0xb4271c`
(`0x477a4a`; nothing reads it) and runs `0x477ba0`, which releases the equipment cells whose record is set, as the records stay and the cells reload on the next composite. The flag `0xb42724` is 1 while handlers run. The unit model update `0x607da0` calls the driver with `force` 0 (`0x607e7c`) and answers 0 when it returns 0 (`0x607e81`), which `0x4815d0` tests at `0x48161d`, so a unit whose first composite waits is not drawn.

## Target callback

The composite target registers `0x477a70` with the texture (`0x477958`). Event 0 composites everything again with `0x477b50` unless a composite is running; event 1 answers with a pitch of twice the width and the pointer
`[0xb422d8][mip]`. `0x477b50` marks a full rebuild, forces both loads, runs every handler in `0xb422f4` to `0xb4231c` and clears the flags.

Addresses: `0x477a70`, `0x477b50`, `0x477958`, `0xb42724`, `0xb422d8`, `0xb422f4`, `0xb4231c`, `0x4420b0`
Confidence: verified
Proof: one
Test: none

`0x477a76 sub ecx,0; je 0x477a97` takes event 0 and `0x477a78 dec ecx; jne` event 1. Event 0 tests `0xb42724` (`0x477a97`) and calls `0x477b50` with `ecx` the component pointer from the callback argument (`0x477aa0`). `0x477b65` sets `+0x08`, the
handlers run in a loop from `0x477b77` with the end `0xb4231c`, and `0x477b94` clears `+0x10`. `0x477b59 call 0x4420b0` is a bare `ret`.

## Load budget and pump

`0x47b1f0` zeroes the load counter that `0x47b200` reads and `0x47ac10` increments for each section texture it starts. `0x475f20` zeroes `0xb4271c` and then the counter; the world render calls it once
a frame (`0x4835f7`). The pump `0x475f30`, called later in the frame (`0x48373b`), walks the pending list and, for each component while the counter is under 0x100, polls its skin and equipment sections without forcing them; it unlinks
every entry whether or not they were ready. `0x477ab0` queues a component that has a dirty mask or a rebuild.

Addresses: `0x475f20`, `0x475f30`, `0x477ab0`, `0x47b1f0`, `0x47b200`, `0x47ac10`, `0x4835f7`, `0x48373b`, `0x475f5a`, `0x477ac2`, `0x5d67fd`, `0x607e63`
Confidence: verified
Proof: one
Test: none

`0x475f55 call 0x47b200; cmp eax,0x100; jge 0x475f73` skips the loads and falls to the unlink at `0x475f73`. `0x47ac10` reads and increments `[0xb42764]` at `0x47ac1b` and `0x47ac24`. `0x477ab0` returns at once when both
`+0x10` and `+0x08` are 0 (`0x477ab5`, `0x477abc`), unlinks the entry and inserts it at `0x477b37` to `0x477b49`. Its callers are the unit model update (`0x607e63`) and the corpse update (`0x5d67fd`).

## Baked skin

A character with a baked skin shows a prebuilt texture. `0x479580` clears `+0x3c`; with no path in `+0x3d` it returns 0. Otherwise it copies the path to the path buffer, loads the texture, binds it as model texture type 1, applies the body
skin (`0x478160`), sets `+0x3c`, `+0x08` and `+0x0a`, and returns 1. A failed load leaves `+0x3c` clear. The update driver and every cell store treat `+0x3c` as an off switch: no sections load and nothing composites.

Addresses: `0x479580`, `0x478160`, `0x449d90`, `0x710ec0`, `0x419e30`, `0x477866`, `0x478900`, `0x4782e0`, `0x478790`, `0x4784c0`, `0x4783f0`, `0x4785b0`, `0x478660`
Confidence: verified
Proof: one
Test: none

`0x479592` clears `+0x3c` first; `0x479596 jne` tests the path byte. The load is `0x479601 call 0x449d90` with the static status, the bind `0x479622 call 0x710ec0` with type 1, and `0x479638` stores 1 only on success. The setters test `+0x3c` before building cells
(`0x4787a2`, `0x4784eb`, `0x47845a`, `0x4785b6`, `0x4786f7`), and `0x478908` makes the item store a no-op.
