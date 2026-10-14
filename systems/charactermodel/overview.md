# charactermodel: overview

The character compositor of the client. A component holds one character model's appearance (race, sex, skin, face, hair
colour, hair style and facial hair), the display records of its equipped items, and the 256×256 body texture composited
from CharSections and item textures. The player, other players, corpses, the character-select list, the character-create
screen and the paperdoll all build one. The component picks the geosets that show, binds the composite and the creature skins
to the model's replaceable textures, and hangs item models at the model's attachments.

The system owns one object file, CharacterComponent.cpp. The docs split by topic: `component` is the object, its pool and its update
driver, `customization` the CharSections grid and the appearance setters, `composite` the tile table and the blit kernels,
`geosets` the visible geoset set, and `equipment` the item records, the held items, the hand grips and the guild tabard. The
model calls, texture loads and tables of DBC records are the callees' (`models`, `gx`, `dbc`).

Roots: `0x475b70`, `0x475b90`, `0x475bb0`, `0x475be0`, `0x475c10`, `0x475c40`, `0x475db0`, `0x475e20`, `0x475f20`, `0x475f30`, `0x475fb0`, `0x476000`, `0x476020`, `0x4766e0`, `0x476b90`, `0x476cb0`, `0x477860`, `0x477ab0`, `0x4783f0`, `0x4784c0`, `0x4785b0`, `0x478660`, `0x478790`, `0x478960`, `0x478aa0`, `0x478bd0`, `0x478cb0`, `0x479010`, `0x479660`, `0x4796b0`, `0x479700`, `0x4797b0`, `0x479f40`, `0x47a070`, `0x47a0c0`, `0x47a230`, `0x47a310`, `0x47a3e0`, `0x47a400`, `0x47a440`, `0x47a4a0`, `0x47a4c0`, `0x47a4d0`, `0x47a520`, `0x47a610`, `0x47a860`

## Object file

CharacterComponent.cpp runs from `0x475b70` to its last function, `0x47a970`. Its static initializers are the eight slots `0x827d3c` to `0x827d58` of
the initializer table, from the +∞ copy `0x475b70` to the pending list `0x475e20`. The next slot, `0x827d5c`, is `0x47a9a0`, the +∞
copy of TextureCache.cpp, which `gx` owns: the section-texture entries, their release and the load counter that this file calls
(`0x47b010`, `0x47b120`, `0x47b150`, `0x47ae30`, `0x47b1f0`, `0x47b200`).

Addresses: `0x475b70`, `0x47a970`, `0x47a9a0`, `0x47a9b0`, `0x476030`, `0x47664d`, `0x4767cb`, `0x827000`, `0x47b010`, `0x47b120`, `0x47b150`, `0x47ae30`, `0x47b1f0`, `0x47b200`
Confidence: verified
Proof: one
Test: none

The path `...\ComponentCore\CharacterComponent.cpp` is pushed at `0x476030`, `0x47664d` and `0x4767cb`, and TextureCache.cpp's path at
`0x47acb7`. `0x47a9a0` jumps to `0x47a9b0`, which copies a pooled +∞ constant into a global like `0x475b70`'s body at `0x475b80`.
The last function of this file, `0x47a970`, ends at `0x47a999`. Code before `0x475b70` belongs to the glue name generator
(`glue`), and no function of this file is reached by falling through from a neighbour.

## Static initializers

Eight initializers run before the first component: `0x475b70` copies +∞ into `0xb422dc`, `0x475b90` stores 2π as π + π
in `0xb42504`, `0x475bb0` stores 1/2π in `0xb422ec`, `0x475be0` squares 10.0 into `0xb422d0` and `0x475c10` stores
0.5 / 10.0 in `0xb422d4`. `0x475c40` fills the tile table (`composite#tile-table`) and registers an empty exit routine.
`0x475db0` builds the static status object at `0xb424f0` that every texture load of the file passes, with its exit
routine `0x475e10`. `0x475e20` builds the empty pending list at `0xb422e0` and `0x475e60` registers `0x475e70`, which unlinks
every remaining entry at exit.

Addresses: `0x475b70`, `0x475b80`, `0x475b90`, `0x475ba0`, `0x475bb0`, `0x475bc0`, `0x475be0`, `0x475bf0`, `0x475c10`, `0x475c20`, `0x475c40`, `0x475c50`, `0x475d90`, `0x475da0`, `0x475db0`, `0x475dc0`, `0x475e00`, `0x475e10`, `0x475e20`, `0x475e30`, `0x475e60`, `0x475e70`, `0xb422dc`, `0xb42504`, `0xb422ec`, `0xb422d0`, `0xb422d4`, `0xb424f0`, `0xb422e0`
Confidence: verified
Proof: one
Test: none

Each odd-numbered slot target is a five-byte `jmp` to its body (`0x475b70` to `0x475b80`, `0x475b90` to `0x475ba0`). The status object
gets its vtable `0x7ffa10`, capacity 8 and an inline buffer (`0x475deb`), and its destructor is `0x419e30`. The list's
sentinel is the tagged address `0xb422e8 | 1` (`0x475e35`). No other code reads `0xb422dc`, `0xb42504`, `0xb422ec`, `0xb422d0` or `0xb422d4`
in this file except `0x475bc0`, which divides by `0xb42504`.

## Unreached code

Four functions have no caller. `0x476960` is the component constructor with a build call. `0x478dc0(inventory type)` returns the display id worn in the slot of an inventory type, or 0. `0x47a470(race, sex)` returns the colour count of skin variation 0. `0x47a8c0(i)` shows geoset base `i`.

Addresses: `0x476960`, `0x478dc0`, `0x47a470`, `0x47a8c0`, `0x476ac0`, `0x477a70`, `0x477958`, `0x478f04`, `0x478ec8`
Confidence: verified
Proof: one
Test: none

The census scans every `e8` and `e9` whose rel32 lands on the function and every little-endian occurrence of its address in each section, and walks the direct jumps. None finds any of the four, and `cargo xtask census --why 0x478dc0` finds nothing live that reaches it. Each follows a `ret` or a jump table and padding, so none is entered by falling through.
Controls: the same scan finds `0x476ac0`'s call at `0x47600a` and `0x477a70`'s address stored once, in the `mov edx,0x477a70` at `0x477958`. `0x478dc0` indexes a byte table `0x478f04` and a jump table `0x478ec8` whose arms return the record's first dword for the slots `+0x4a8` to `+0x4c8`, `+0x4d8`, `+0x4dc`, `+0x4d0` and `+0x4cc`.
