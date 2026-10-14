# ui: unit portraits

`SetPortraitTexture(texture, "unit")` puts a portrait on a texture widget. A unit the client holds is drawn once from its own model into a 64 by 64 texture that is cached and marked for a redraw when the model or the appearance changes. A player or pet the client does not hold gets a cached bake or a stand-in texture. This doc is the dispatch, the caches, the update signal and the bake. The camera and the projection builder are `camera`; the model scene is `models/scene`.

Roots: `0x519ef0`, `0x524f60`, `0x524cd0`, `0x524e10`, `0x525ba0`, `0x525cb0`

## SetPortraitTexture

The function raises `Usage: SetPortraitTexture(texture, "unit")` when the second argument is not a string, and clears the widget's texture first (the empty path). A token that names no GUID ends there. A unit object found with type mask 8 goes to the bake `0x524f60(widget, unit)`.
With no object, a GUID whose high dword has its top nibble clear (a player) asks the name cache `0x55f080` for the record, with the callback `0x51a100` that marks the portrait dirty when the record arrives, and passes the record's GUID, race and sex to `0x525ba0`. Any other GUID looks up the group's pet record (`0x496420`) and passes the word at `+0x5a` (the display id) to `0x525cb0`.

Addresses: `0x519ef0`, `0x519fcc`, `0x519fe6`, `0x51a00c`, `0x51a02b`, `0x51a03b`, `0x51a065`, `0x51a06d`, `0x51a08e`, `0x51a097`, `0x51a0b4`, `0x51a0c4`, `0x51a0d0`, `0x51a0d6`, `0x51a100`, `0x55f080`, `0x496420`, `0x491a50`, `0xc0e228`
Confidence: verified
Proof: one
Test: none

The name record's GUID is at `+0x130` and `+0x134`, its race at `+0x138` and its sex at `+0x13c`. A record that is not found, or a pet record that is not found, leaves the widget blank. The corpse branch at `0x51a03b` cannot run: a lookup with mask 8 never returns a corpse object, whose type mask is 0x81.
`0x51a100` calls `0x491a50`, the portrait refresh (`0x524e10`), when its second argument is nonzero.

## The caches

The bake keeps two hash tables of finished portraits. A player is keyed by GUID (table `0xc0ce7c`, mask `0xc0ce84`); any other unit by its display id, `UNIT_FIELD_DISPLAYID` at `+0x1f4` of the fields (table `0xc0cea4`, mask `0xc0ceac`). A GUID entry holds the GUID at `+0x18` and `+0x1c`, a dirty byte at `+0x20` and the texture at `+0x24`;
a display-id entry holds the texture at `+0x18` and no dirty byte. A hit on a clean entry binds the cached texture with `0x770300` and draws nothing.

Addresses: `0x524f60`, `0x524fc8`, `0x524fe2`, `0x525057`, `0x525062`, `0x525077`, `0x525086`, `0x5250dd`, `0x5256e5`, `0x5256f8`, `0x52572c`, `0x526ad0`, `0x526d70`, `0x770300`, `0xc0ce60`, `0xc0ce7c`, `0xc0ce84`, `0xc0ce88`, `0xc0cea4`, `0xc0ceac`
Confidence: verified
Proof: one
Test: none

A GUID entry that is dirty, or a unit absent from the table, is baked. The bake clears the dirty byte at `0x5256e5` or inserts an entry (`0x526ad0` for a GUID, `0x526d70` for a display id) and stores the GUID or display id and the texture. A display-id entry is never marked dirty, so a creature's portrait is drawn once per display id.

## Marking and draining

`0x524e10(&guid)` marks a portrait dirty: it appends the GUID to the pending list (`0xc0cdd0`, head `0xc0cdd4`) unless it is already there and sets the dirty byte of the GUID cache entry, if one exists. Each frame `0x524cd0` walks the list; a node whose unit is held but not ready (the latches below) stays,
and every other node leaves the list and signals the unit events `UNIT_PORTRAIT_UPDATE` (0xb8) and `UNIT_MODEL_CHANGED` (0xb9) for the GUID. `0x524f60` returns at once for a unit whose GUID is on the pending list.

Addresses: `0x524e10`, `0x524e44`, `0x524f49`, `0x524cd0`, `0x524d37`, `0x524d42`, `0x524d50`, `0x524d68`, `0x524d77`, `0x524d87`, `0x524dd6`, `0x524de4`, `0x524de9`, `0x483228`, `0x491a50`, `0x524f96`, `0x515e50`, `0xc0cdcc`, `0xc0cdd0`, `0xc0cdd4`, `0x607e0f`
Confidence: verified
Proof: one
Test: none

`0x491a50` is a jump to `0x524e10`, called from the name callback `0x51a100` and from nine callers (`0x5d991b`, `0x5df119`, `0x5e081e`, `0x5e08c9`, `0x5e587f`, `0x5eeda9`, `0x5fb5d0`, `0x5ffbbe`, `0x607e0f`); `0x524cd0` has one caller, the world frame update at `0x483228`. The draining never bakes: the next `SetPortraitTexture` does.
`0x524cd0` also needs the render device (`0x58a260`, `0x58a280`).

## The readiness gate

`0x524f60` bakes only when the device is up and the unit is ready: the unit's two work latches at `+0xcd0` and `+0xccc` are 0, its character compositor (`+0xd30`), when it has one, reports its textures loaded (`0x477860(0)`), its model (`+0xd8`) exists and is ready (`0x710450`) and has at least one camera. A unit that fails any of these but the camera count
is marked dirty (`0x524e10`) and nothing is drawn; a model with no camera binds the empty texture path.

Addresses: `0x5250f5`, `0x5250fa`, `0x525102`, `0x52510f`, `0x52511d`, `0x52512b`, `0x525137`, `0x525144`, `0x525152`, `0x525169`, `0x525172`, `0x525874`, `0x525885`, `0x477860`, `0x710450`, `0x7134d0`, `0x607da0`, `0x5faf0e`, `0x5faf14`
Confidence: verified
Proof: one
Test: none

The latches are set to 1 by the unit constructor (`0x5faf0e`, `0x5faf14`) and by the two model-creation paths, and cleared by `0x607da0`, which then marks the portrait dirty (`0x607e0f`). The camera count is `0x7134d0`.

## The bake

The bake draws a fresh copy of the unit's model (`0x707400` of the model `+0xd8`) in a new scene (`0x706e10`), with the attachments removed (`0x47a230`) and the animation set to sequence 0 (`0x7121a0`). The lights are an ambient 0.45 grey and a white directional light along (-1, 0, -1) (callback `0x525b10`).
The camera is the model's own camera 0 (`0x713540`), built with `0x7ada40` over a rectangle (0, 0, 1, D). The viewport is 64 by 64 pixels in the corner of the render surface. The surface is cleared to 0, the scene is drawn twice (passes 0 and 1), the corner is read back into the texture `Portrait1`, and the device's matrices and viewport are restored.

Addresses: `0x524f60`, `0x525193`, `0x5251b2`, `0x5251d3`, `0x525227`, `0x525244`, `0x52525f`, `0x52526a`, `0x525285`, `0x5253d0`, `0x5253db`, `0x5253f6`, `0x52542b`, `0x525433`, `0x525446`, `0x5255a4`, `0x5255d6`, `0x5255f2`, `0x525600`, `0x52560a`, `0x525645`, `0x52564e`, `0x52575b`, `0x525760`, `0x52576f`, `0x525798`, `0x52580a`, `0x525823`, `0x525828`, `0x52584b`, `0x58a250`, `0x41ade0`, `0x41ae60`, `0x41ae70`, `0x58af60`, `0x589cf0`, `0x5258a0`, `0x449bf0`, `0x525b10`, `0x808de4`
Confidence: verified
Proof: one
Test: none

In the code's order, with float stores: `p = G44 * (64 / W)`, `q = G48 - (G48 / H) * 64` and `D = (p - 0) / (G48 - q)` (`0x5251b7` to `0x525430`). Each x87 operation rounds once, to the client's 53-bit precision, and each store to 24 bits. `0x58a250` reads the live window rect `{top, left, bottom, right}` (device slot `+0x60`), W is `rect[3] - rect[1]`, H is `rect[2] - rect[0]`, and G44 and G48 are the scale globals `0x832a44` and `0x832a48` (`0x41ad10`), whose ratio is the configured aspect. D is 1 within a few ulps, not exactly 1.0,
so the projection aspect is too (`camera#view-and-projection`): the half-angle is `fov / (2 * sqrt(2))` and a sphere lands nearly round. The viewport fractions are `(0, 64/W)` by `(1 - 64/H, 1)` after `0x41ade0` divides by G44 and G48, a 64 by 64 pixel box, read back from (0, 0), no render target.
After the readback the device is cleared again and the mask `0x5258a0(64)` is copied byte by byte into the alpha byte of each texel, which makes the round shape; the texture is created once per entry by `0x449bf0` and updated in place afterwards.

## Stand-in portraits

`0x525ba0(widget, race, sex, guid)` binds the cached GUID entry's texture when the entry has one; otherwise it loads the stand-in `TemporaryPortrait-<sex>-<race>` from the `CharacterFrame` folder, the sex word picked from the table at `0x856450` and the race name from the `ChrRaces` row `+0x3c`.
`0x525cb0(widget, displayId)` binds the display-id entry's texture, else loads `TemporaryPortrait-Pet`.

Addresses: `0x525ba0`, `0x525bff`, `0x525c2b`, `0x525c46`, `0x525c5b`, `0x525c62`, `0x525c68`, `0x525c76`, `0x525c7c`, `0x525c95`, `0x525cb0`, `0x525ce9`, `0x525d00`, `0x525d17`, `0x856450`, `0xc0dee0`, `0xc0dee4`, `0x770200`
Confidence: verified
Proof: one
Test: none

The race row is looked up in the id array `0xc0dee0` with its bound `0xc0dee4`; an id outside it gives a null row and a read of its `+0x3c` at address 0x3c. `0x770200` and `0x770300` bind a path or a texture into the widget's texture field (`+0xcc`).

## Clearing the caches

`0x524b70` empties the GUID table (`0xc0ce60`), the display-id table (`0xc0ce88`) and a third hash table at `0xc0ce30`, then frees the pending list nodes and the spare nodes. Its one caller is `CGGameUI::ShutdownGame` (`0x491180`, call at `0x491187`).

Addresses: `0x524b70`, `0x491180`, `0x491187`, `0x524b72`, `0x524bc1`, `0x524c10`, `0x524c60`, `0x524c90`, `0xc0ce60`, `0xc0ce88`, `0xc0ce30`
Confidence: verified
Proof: one
Test: none

Each table is emptied by calling the table's remove slot on its first entry until the slot is empty, for every bucket. The third table's use is not read here.
