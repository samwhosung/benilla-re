# ui: the guild tabard designer

The `TabardModel` frame the tabard vendor window uses: its design state, its ten Lua methods, how the window opens and
closes, and its four events. The save request and the server's reply are `objectlayer/party#guild-tabards`; the pane
class, its method tables and its camera are `ui/playermodel` and `ui/model`; the emblem blit onto the character is the
character model system.

Roots: `0x502780`, `0x502ac0`, `0x502b10`, `0x502bd0`, `0x502ca0`, `0x502dc0`, `0x502ea0`, `0x502f80`, `0x503070`, `0x503160`, `0x503540`, `0x503910`, `0x4f5840`, `0x4f58a0`, `0x4f5910`

## Design state

A `TabardModel` keeps its design as five signed integers at `+0x3f8`: emblem style, emblem colour, border style, border
colour and background colour. Their counts are 170, 17, 6, 17 and 51, five dwords of read-only data; no table of the
game files is read. The constructor does not set the five, and the allocator does not clear them, so they are defined
only after `InitializeTabardColors()` ran. Every method that changes the preview repaints the model through the
character component's emblem installer.

Addresses: `0x502780`, `0x502a00`, `0x502a51`, `0x47a610`, `0x5029b6`, `0x502ad1`, `0x502af1`, `0x5e03f0`, `0x808220`
Confidence: verified
Proof: one
Test: none

The counts at `0x808220` are read by the draw loop (`0x5029b6`), `0x502ad1` and `0x502af1`, and again as five
immediates in the save sender `0x5e03f0`. `0x502a00` finds the active player, loads its character component
(`[player+0xd30]`) and calls `0x47a610(component, style, colour, border, borderColour, background)`; it does no guild
lookup. The data section holding the counts has no write permission.

## Cycling a variation

`CycleVariation(index, delta)` moves one design value by `delta` with wrap-around. The index is 1 to 5, in the order
above. Both numbers are truncated toward zero. A first argument outside 1 to 5 raises `Invalid variationIndex in
CycleVariation`, and a missing or non-numeric argument raises `Usage: CycleVariation(variationIndex, delta)`. A delta
whose size reaches the count changes nothing and does not repaint. Otherwise the value becomes `(v + count + delta)`
modulo the count, then the model is repainted.

Addresses: `0x502ca0`, `0x502d46`, `0x502d56`, `0x502d6b`, `0x502d72`, `0x502d73`, `0x502d76`, `0x502d98`, `0x502da1`, `0x502ac0`, `0x502ada`, `0x502adc`, `0x502ae5`, `0x502af1`, `0x502aff`
Confidence: verified
Proof: one
Test: none

The index test is `dec edi; cmp edi,5; jb`, unsigned, so 0 is rejected. The worker `0x502ac0(slot, delta)` takes the
absolute value of the delta with `cdq; xor; sub` and compares it with the count by a signed `jge`; the update is an
`idiv` by the count. The raise never returns, so the code laid after each `luaL_error` call does not run.

## Starting values

`InitializeTabardColors()` seeds the five values from the player's guild: the guild cache record of `PLAYER_GUILDID`
supplies the emblem fields at `+0x2e4` to `+0x2f4`. A missing record, or any field equal to -1, draws each value at
random in `[0, count)`. Both ways end by repainting the preview. It returns nothing.

Addresses: `0x502b10`, `0x502b9a`, `0x502bab`, `0x5028e0`, `0x502904`, `0x502914`, `0x502921`, `0x50292a`, `0x50299f`, `0x5029a3`, `0x5029a8`, `0x5029c9`, `0x5029d3`, `0x44ea80`, `0x4531e0`, `0x455c70`, `0x560e30`, `0xc0e1ec`
Confidence: verified
Proof: one
Test: none

The record is fetched with `0x560e30(0xc0e1ec, guild id, ...)`; the five `cmp ...,-1` are `0x502933` to `0x50294f`. The
random leg seeds a generator with 0 and then with the tick (`0x42c010`), draws a word per count with `0x4531e0` and scales
it with `0x455c70`, which is `(count * word) >> 32`.

## Tabard file names

`GetUpperBackgroundFileName()` and `GetLowerBackgroundFileName()` return the path of the background texture for the
current background colour; `GetUpperEmblemFileName()` and `GetLowerEmblemFileName()` return the path of the emblem for the
current style and colour. Each returns one string with no extension; the loader adds it. The upper half is the 128 by 64
texture and the lower half the 128 by 32. There is no border file name function.

Addresses: `0x502dc0`, `0x502ea0`, `0x502f80`, `0x503070`, `0x502e7a`, `0x47a4d0`, `0x47a520`, `0x47a570`, `0x6f3890`
Confidence: verified
Proof: one
Test: none

The path forms are `Textures\\GuildEmblems\\Background_%02d_TU_U` and `_TL_U`, `Emblem_%02d_%02d_TU_U` and `_TL_U`,
built by `0x47a4d0` (background, part 3 or 4) and `0x47a520` (emblem) into a 0x104-byte buffer. The border builder
`0x47a570` is used by the model install and has no method.

## Emblem textures

`GetUpperEmblemTexture(texture)` and `GetLowerEmblemTexture(texture)` fill the Texture widget they are given and return
nothing. Each decodes the emblem file for the current style and colour and keeps only its alpha: the texture is white
with the emblem's alpha, 128 by 64 for the upper half and 128 by 32 for the lower. A file that fails to open or decode
leaves the previous contents, all clear on first use, and still installs the texture. A missing, non-texture or wrong-type
argument raises the method's usage text.

Addresses: `0x503160`, `0x503314`, `0x503324`, `0x503380`, `0x503396`, `0x5033eb`, `0x503405`, `0x503423`, `0x503437`, `0x50348c`, `0x503497`, `0x5034f0`, `0x503540`, `0x503520`, `0x5038f0`, `0x5a7b40`, `0x5a7f00`, `0x449bf0`, `0x770300`
Confidence: verified
Proof: one
Test: none

Each half owns a static pixel array, grown once to 8192 elements (upper) or 4096 (lower) (`0x503324`), and a cleanup
registered with `atexit`. The path gets an explicit upper-case `.BLP` (`0x503396`). The decode is `0x5a7f00(2, ...)`; the
pixel loop stores `(alpha << 24) | 0xffffff` (`0x503423` to `0x503437`). The texture is created with
`0x449bf0(128, 64, ...)` (`128, 32` for the lower) and a lock callback `0x5034f0` that returns the pitch (width times 4)
and the array; `0x770300` installs it in the widget.

## Saving

`Save()` sends the staged design for the vendor the window is open at: it finds the active player and calls the save
sender with the five values and the interaction target GUID (`0xb4e2d0`). `CanSaveTabardNow()` returns the number 1 or
nil, takes no argument and never resolves its object: the player exists, the player's guild record is cached, and no save
is in flight (`0xc4d780`). It does not compare the design with the guild's tabard or test the guild rank; those refusals come
when Save is pressed (`objectlayer/party#guild-tabards`).

Addresses: `0x502bd0`, `0x502a60`, `0x502ab5`, `0x5e03f0`, `0x503910`, `0x503913`, `0x5f0800`, `0x5f0866`, `0xb4e2d0`, `0xc4d780`, `0x5e05f5`, `0x5e7128`
Confidence: verified
Proof: one
Test: none

The latch `0xc4d780` is set to 1 after the send (`0x5e05f5`), cleared by the reply before it branches (`0x5e7128`) and by
the player module's teardown, and read only by `0x5f0800`. `0x503910` pushes the double 1.0 or nil and returns 1. The
GUID `Save` sends is the global interaction target, which `0x4f5840` sets to the vendor when the window opens.

## Opening and closing

The window opens only when the server sends `MSG_TABARDVENDOR_ACTIVATE` (0x1f2, `0x5e70c0`): a u64 vendor GUID. No Lua
function opens it. The handler `0x4f5840` stores the GUID in `0xbdcee8`, switches the character component to the
designer preview, makes the vendor the interaction target and signals `OPEN_TABARD_FRAME` (374). `CloseTabardCreation()`
and the engine's interaction clear (out of range, another window, leaving the world) run `0x4f58a0`, which does nothing
without a stored GUID; otherwise it undoes the preview, releases the target and signals `CLOSE_TABARD_FRAME` (375). Closing
sends nothing. `GetTabardCreationCost()` returns 100000 copper.

Addresses: `0x5e70c0`, `0x5e70d5`, `0x4f5840`, `0x4f5874`, `0x4f5882`, `0x4f588d`, `0x4f58a0`, `0x4f58ab`, `0x4f58ce`, `0x4f58da`, `0x4f58f8`, `0x4f5830`, `0x4f5900`, `0x4934a0`, `0x490b02`, `0x4f5910`, `0x4f5919`, `0x6d6de0`, `0x5e07d0`, `0x5e0830`, `0x4930d0`, `0x4931c0`, `0xbdcee8`
Confidence: verified
Proof: one
Test: none

The handler reads a u64 (`0x4190b0`) and calls `0x4f5840` (its only caller). `0x4f58a0` has three callers: the Lua binding,
the interaction-clear dispatcher (`0x4934a0`) and a thunk `0x4f5830` used by the leave-world step (`0x490b02`). The
tail jumps to the signaller are `0x4f588d` and `0x4f58f8`. `CloseTabardCreation` called while the vendor is still the
target signals `CLOSE_TABARD_FRAME` twice: the clear `0x4931c0` re-enters `0x4f58a0` through `0x4934a0` before the outer call
zeroes the GUID. The cost is the constant behind `0x6d6de0` (`mov eax,0x186a0`), shared with the save sender's money test.

## Starting a vendor visit

`MSG_TABARDVENDOR_ACTIVATE` goes the other way as a request: `0x5e00e0(npc GUID)` sends opcode 0x1f2 and the u64 GUID. It
sends nothing when the window is already open at that NPC, and shows `ERR_EMBLEMERROR_NOTABARDGEOSET` when the player's
displayed model differs from the native one (shapeshifted or transformed). The senders are the NPC click for a tabard
vendor and `GetTabardInfo()`, which sends for the guild registrar's stored GUID and then closes the registrar window.

Addresses: `0x5e00e0`, `0x5e00fc`, `0x5e0108`, `0x5e010e`, `0x5e011b`, `0x5e012d`, `0x5e015e`, `0x5e0169`, `0x4f5300`, `0x4f5350`, `0x4f5355`, `0x5f051a`, `0xbdceb0`
Confidence: verified
Proof: one
Test: none

The first gate compares the stored GUID `0xbdcee8` with the argument. The model test compares `UNIT_FIELD_DISPLAYID` and
`NATIVEDISPLAYID` at `[player+0x110]+0x1f4` and `+0x1f8`; the refusal pushes message 0x130. The packet is `push 0x1f2` then
the GUID (`0x418370`) and `0x5ab630`. `GetTabardInfo` (`0x4f5300`) passes a copy of the registrar GUID `0xbdceb0` and then
calls the registrar close `0x4f5010`.

## Tabard events

Four events belong to the window; none has an argument.

| id | name | signalled from |
|---|---|---|
| 374 | `OPEN_TABARD_FRAME` | `0x4f588d`, the only site |
| 375 | `CLOSE_TABARD_FRAME` | `0x4f58f8`, the only site |
| 376 | `TABARD_CANSAVE_CHANGED` | `0x5e0907`, the guild appearance refresh of a player |
| 521 | `TABARD_SAVE_PENDING` | `0x5e05fc` when a save is sent, `0x5e7172` when the reply is not a success |

Addresses: `0x4f588d`, `0x4f58f8`, `0x5e0907`, `0x5e05fc`, `0x5e7172`, `0x5e08e0`, `0x5e0922`, `0x5dec19`, `0x5e2a21`, `0x703e50`
Confidence: verified
Proof: one
Test: none

`0x5e08e0` signals 376 as its second act and then `PLAYER_GUILD_UPDATE` (345) for each unit token that names the player;
its callers include the player-object update (`0x5dec19`) and the guild-record refresh that enumerates every player with
the arriving guild id (`0x5e2a21`). After a successful save the cached guild record is dropped without a request
(`objectlayer/party#guild-tabards`); the next cache miss asks for it again, and whether that arrival reaches `0x5e08e0` is
not traced.
