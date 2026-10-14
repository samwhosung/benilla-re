# minimap: update

The per-frame driver `UpdateMinimap` and the reset. The frame code calls the driver once a frame with the tracked object's light node, the
map and the player's position; it chooses the outdoor or the interior tiles, keeps the interior window and the zoom state, expires the
gossip marker and runs the blip pass.

Roots: `0x6d93a0`, `0x6d92c0`

## Frame update

`UpdateMinimap(node, map, position, offset, scale, records, state, flag)` returns 1 when anything changed. It first expires the gossip marker (`Gossip marker`). When the player's position or the map differs from the cached one, it stores them and asks the node for the interior it is in (`Indoor state`). It then builds the interior window or, outdoors, prepares the tile block with `WorldToScreenScale`, when that happened or a change is pending in masks `0x1` and `0x2` of the dirty word `0xcea8bc`. A pending change with no movement skips the store and the node query. Then it runs the blip pass `PlaceBlips`. With no
change and the flag set, an outdoor frame still calls `WorldToScreenScale` for the offset and scale.

Addresses: `0x6d93a0`, `0x6d93a9`, `0x6d93b3`, `0x6d9419`, `0x6d9453`, `0x6d945f`, `0x6d948e`, `0x6d94bb`, `0x6d95ca`, `0x6d95cd`, `0x6d998e`, `0x6d999f`, `0x6d9984`, `0x6d9a58`, `0x6d9a75`, `0x6d9a87`, `0x6da380`, `0x6d9a90`, `0x4ecc14`, `0x4ecbe3`, `0x4ecbea`, `0x4ecbef`, `0xcea8bc`, `0xceaa7c`, `0x86f694`, `0xbc8300`
Confidence: verified
Proof: one
Test: none

With no movement `0x6d9459` jumps to `0x6d95bf`, and with a pending change `0x6d95ca` follows, skipping `0x6d945f` to `0x6d95b8`; the held state byte decides. The caller `0x4ecc14` runs with `ecx` the light node `[object + 0xe0]` (`0x4ecbe3`) and `edx` the map; its stack carries the position, two outputs, the records (at 0xbc83a8), the state (`0xbc8300`) and a flag. `0x6d93a9` to `0x6d93b3` read the dirty mask and test mask `0x3`. The position test is `0x6d9419` to `0x6d9453`,
comparing the position's x and y with `0xceaa7c`, `0xceaa80` and the map with `0x86f694`; a difference reaches `0x6d945f`, which marks the frame changed, stores the position (`0x6d948e`) and the map (`0x6d94bb`) and computes the displacement. The outdoor branch is `0x6d998e`:
`0x6d999f` calls `WorldToScreenScale` and sets the changed flag from its result. The indoor branch is `0x6d95cd` onward. `0x6d9984` runs the blip pass. When nothing changed and the flag is set, `0x6d9a58` to `0x6d9a75` call `WorldToScreenScale` for an outdoor frame. The function ends with `ret 0x18` (`0x6d9a87`).

## Indoor state

The node is asked which interior it is in (`0x670540`, three words). A positive answer sets the state byte to 1 and the group identity words are cached; a change of the first two words, or the third word differing from `0x86f6a0`, rebuilds the interior: it refreshes the orientation, builds the world name and sets all tile keys to −1. A negative answer clears the
state byte and sets the cached words to −1. When the byte `0xceaa60` differs from the state byte it takes its value, sets mask `0x1` of the dirty word and signals the event 0xcf, `MINIMAP_UPDATE_ZOOM`, because that byte chooses the zoom table (`ui/minimap#the-zoom-radius`).

Addresses: `0x670540`, `0x6d94c7`, `0x6d94d4`, `0x6d94d7`, `0x6d94da`, `0x6d94e5`, `0x6d94ed`, `0x6d94f3`, `0x6d94fa`, `0x6d9502`, `0x6d950d`, `0x6d9512`, `0x6d9518`, `0x6d951b`, `0x6d9527`, `0x6d957f`, `0x6d9585`, `0x6d9588`, `0x6d958d`, `0x6d9592`, `0x6d9597`, `0x6d9599`, `0x6d95a1`, `0x6d95b3`, `0x6d95b8`, `0x703e50`, `0x6da180`, `0x6da330`, `0x6d9e10`, `0xceaa64`, `0xceaa68`, `0x86f6a0`, `0xceaa60`, `0xcea8bc`
Confidence: verified
Proof: one
Test: none

`0x6d94c7` calls `0x670540` with the node and receives three words at `[ebp-8]`, `[ebp-0x10]` and `[ebp-0x14]`. A true result stores 1 in the state byte (`0x6d94d7`); the first two words are compared with `0xceaa68` and `0xceaa64` (`0x6d94da`, `0x6d94e5`), and a difference marks the frame changed, refreshes the orientation (`0x6d94f3`), the world name (`0x6d94fa`) and clears the
keys (`0x6d9502`), then caches the words (`0x6d950d`, `0x6d9512`). The third word is compared with `0x86f6a0` at `0x6d951b` and stored at `0x6d9527`. A false result clears the state byte (`0x6d9585`) and sets `0x86f6a0`, `0xceaa64` and `0xceaa68` to −1 (`0x6d9588` to `0x6d9592`). `0x6d9599` compares the cell `0xceaa60` with the state byte, stores it (`0x6d95a1`), ORs `0x1` into the mask (`0x6d95b3`) and
signals the event through `0x703e50` with `ecx = 0xcf` (`0x6d95b8`). `0xceaa60` has this one writer.

## Interior window

Indoors, the window is a square of the width `c` of the indoor zoom table `{150, 120, 90, 60, 40, 25}` at the index `0x86f69c`, snapped to the player: `min.x = round(px · r − 0.5) · c`, `min.y = round(py · r − 0.5) · c` with `r = 1/c`, `min.z = pz − c/2`, `max.x = min.x + c`, `max.y = min.y + c`, `max.z = c/2 + min.z`, rounded half to even.
The node (`0x670520`) then gives the descriptors of the groups touching the window grown by `c` all round, at most 0x200. Held tiles are refreshed, the others loaded (`tiles#interior-tiles`), and the state record gets the window's centre, the position, the radius and the matrices.

Addresses: `0x6d95cd`, `0x6d95e8`, `0x6d95f8`, `0x6d9620`, `0x6d9634`, `0x6d9661`, `0x6d96ab`, `0x6d96b1`, `0x6d96bc`, `0x6d96f3`, `0x6d9706`, `0x6d9716`, `0x6d9730`, `0x6d975f`, `0x6d9772`, `0x6d979f`, `0x6d97ba`, `0x6d97ce`, `0x6d97de`, `0x6d9833`, `0x6d9877`, `0x6d9887`, `0x6d98e5`, `0x6d98ee`, `0x6d9a12`, `0x6d9984`, `0x670520`, `0x7bca80`, `0x4549a0`, `0x5f8cf0`, `0x6372d0`, `0x8116e8`, `0x86f69c`, `0x86f690`, `0xcea9c4`, `0xcea9d8`, `0xcea9e0`, `0xceaa20`, `0x6d9e30`, `0x6d9ed0`, `0x61e9c0`, `0x7bc1a0`
Confidence: verified
Proof: one
Test: `minimap::minimap_update_snap_difftest`

`0x6d95f8` loads the indoor index and the table entry; `0x6d9620` and `0x6d9634` subtract 0.5 (`0x86f690`) before the `fistp` rounds; the bounds are stored at `0xcea9c4` to `0xcea9d8` (`0x6d9661` to `0x6d96ab`). The block `0x6d95f8` to `0x6d96b1` is the tested slice. The two axes differ in the reciprocal: y multiplies by the unrounded register value (`fst` at `0x6d9614`, `fmul` at `0x6d9617`), x by the float stored at `0x6d9614` (`0x6d9629`). `0x6d96bc` copies the window to a query box, lowers
its minimum by `c` and `0x6d96f3` raises its maximum by `c` through `0x6372d0`; `0x6d9706` calls `0x670520` with the node, the box, a list and the limit 0x200 (`0x6d96f8`). `0x6d9716` transforms the player's position by the matrix `0xcea9e0` (`0x7bca80`); its z is the bias. `0x6d9730` clears mask `0x2` of the records. The first
descriptor's group index is the player's group. The first loop (`0x6d975f`) calls `TileLodDistance` and marks each descriptor already held with −1 (`0x6d9772`); the second loop (`0x6d979f`) loads the others. `0x6d97ba` sets mask `0x2` of the dirty word and clears `0x1`. `0x6d97ce` calls `0x61e9c0`, which returns at once, for each descriptor. `0x6d97de` is the readiness test; when ready, `0x6d9877` forms the window centre, `0x6d98e5` marks the state valid and `0x6d98ee` stores the radius.

## Gossip marker

The gossip marker is the static blip record 1 (`ui/minimapblips#setting-a-static-blip`). Each update clears it in two cases and then runs the landmark rebuild of the world map. First, when its expiry time, kept at `0xceaaa4`, has passed. Second, when the player is strictly nearer than 10 yards,
by the squared distance 100.0; at exactly 10 yards nothing is cleared. Clearing zeroes the record's x and y and the expiry. The party ping is a different store (`ui/minimap#the-ping`) and nothing clears it.

Addresses: `0x6d93b9`, `0x6d93cb`, `0x6d93d0`, `0x6d93e7`, `0x6d93f1`, `0x6d93fb`, `0x6d9411`, `0x6d99aa`, `0x6d99bb`, `0x6d99ce`, `0x6d99d4`, `0x6d9a08`, `0x6d9a12`, `0x6d9a15`, `0x6d9a26`, `0x6d9a30`, `0x6d9a3a`, `0x6d9a4c`, `0x4a67a0`, `0x429580`, `0xcea7e4`, `0xcea7e8`, `0xceaaa4`
Confidence: verified
Proof: one
Test: none

`0x6d93b9` reads the expiry; when it is nonzero `0x6d93cb` gets the Unix time and `0x6d93d0` compares: at or past it, `0x6d93e7` to `0x6d93fb` zero the marker's x and y and the expiry and `0x6d9411` calls the world map rebuild `0x4a67a0` with the map. The proximity test starts at `0x6d99aa`: a marker with a zero x and y is skipped (`0x6d99bb`, `0x6d99ce`); `0x6d99d4` subtracts the marker from the player and sums the squares, `0x6d9a08` compares with 100.0 and `0x6d9a12` tests the flags: `0x6d9a15` takes `jp`
on the parity of the masked bits, so only a strictly smaller value reaches `0x6d9a17`, whose stores clear the marker (`0x6d9a26`, `0x6d9a30`, `0x6d9a3a`) and call `0x4a67a0` (`0x6d9a4c`).

## Reset

`ResetMinimap` runs when the minimap frame is destroyed (the interface shutdown, `0x490a99`): it sets the cached position to the largest float, the map, the four tile indices and `0x86f6a0` to −1, the two group words and the dirty mask to 0, and empties the catalog. The next world entry reads the catalog again.

Addresses: `0x6d92c0`, `0x6d92d3`, `0x6d92f9`, `0x6d9304`, `0x6d9314`, `0x6d931a`, `0x6d9320`, `0x6d9326`, `0x6d932b`, `0x6d9331`, `0x6d9337`, `0x6d934d`, `0x6d935f`, `0x6d937f`, `0x4eda89`, `0x490a99`, `0xceaa7c`, `0xceaa80`, `0xceaa84`, `0x86f694`, `0xceaa6c`, `0xceaa70`, `0xceaa74`, `0xceaa78`, `0xceaa64`, `0xceaa68`, `0x86f6a0`, `0xcea8bc`
Confidence: verified
Proof: one
Test: none

`0x6d92d3` stores 3.4028235e38 in the cached x, y and z (`0xceaa7c`, `0xceaa80`, `0xceaa84`). `0x6d92f9` stores −1 in the map and `0x6d9304` to `0x6d9314` in the four cached tile indices; `0x6d931a` and `0x6d9320` zero the group words `0xceaa68` and `0xceaa64`, `0x6d9326` stores −1 in `0x86f6a0`, `0x6d932b` zeroes the dirty mask and `0x6d9331` the catalog's cursor.
The loop from `0x6d9337` unlinks every record from the list (`0x6d934d`) and the loop from `0x6d935f` frees the records through slot 0 (`0x6d937f`). The only caller is `0x4eda89`, in the frame's destructor.
