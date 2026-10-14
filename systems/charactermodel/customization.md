# charactermodel: customization

The appearance of a character comes from CharSections rows grouped by race, sex and section, and from two smaller tables of geoset
numbers. This doc covers the grid built from the rows at start-up, its lookup, the counts the character-create screen reads, and the setters that turn an
appearance index into cells and geosets. The cells are blitted by `composite`; the geosets are shown by `geosets`.

Roots: none

## Grid build

`0x476020` builds a grid of `(2·maxRace + 2) · 5` entries of 8 bytes from the CharSections rows (`[0xc0df64]`, 0x28 bytes, count `[0xc0df68]`) in three passes. Entry `5·(sex + 2·race) + section` holds the number of
variations and a pointer to one 12-byte record per variation: its colour count, its count of base colours (rows whose flag bit 0 is clear) and a pointer to the row ids by colour. Base colours take the slots
`0 ..` and overlay colours (flag bit 0 set) the slots from the base count. Section is 0 skin, 1 face, 2 facial hair, 3 hair, 4 underwear. A group whose variations are not dense from 0 makes the build return 0.

Addresses: `0x476020`, `0x4761e7`, `0x4762fc`, `0x476306`, `0x47642d`, `0x476470`, `0x476628`, `0x4766ad`, `0x6462e0`, `0x404130`, `0x4766c0`, `0xc0df64`, `0xc0df68`, `0xc0dee4`, `0xb4231c`, `0xb4250c`, `0xb4244c`, `0xc0df28`, `0xc0df2c`
Confidence: verified
Proof: one
Test: none

`0x476194` sizes the grid at `(2·maxRace + 2) · 5 · 8` bytes and `0x4761ca call 0x404130` constructs it five entries at a time with `0x4766c0`. The first pass compares each row's race, sex and section with the previous row's (`0x47625b` to `0x476268`), stores the
variation count into the previous group's entry (`0x47627f`) and allocates `12·(n + 1)` bytes of records (`0x476292` to `0x4762a4`); `0x476306 cmp ebx,edx; jge 0x4766ad` is the density test, where `ebx` is the previous row's variation and `edx` the distinct variations so far.
The second pass stores the base count at `+4` (`0x47642d`) and the colour count at `+0` (`0x47643b`) of each record and allocates its ids. The third stores `row.id` at `slot` (`0x476628`), `slot = colour` for a row with flag bit 0 clear (`0x4765e5`)
and `base count + colour` otherwise (`0x4765e0`). The facial hair table `0xb4244c` counts the rows of CharacterFacialHairStyles per `sex + 2·race` (`0x476695`).

## Grid lookup

`0x477c20(section, variation, colour)` is true when `0 ≤ section < 5`, `0 ≤ variation < count` of the component's entry, the entry's count is nonzero, `0 ≤ colour < colours` of the variation's record and that count is nonzero. The entry is the component's race and sex.
The row for a colour is `[0xc0df6c][ids[colour]]` when the id is within `[0xc0df70]`, else null. Five counts read the grid for a race and sex: `0x47a3e0` hair styles, `0x47a400` hair colours of a style, `0x47a440` skin colours
(the base count of skin variation 0), `0x47a4a0` faces, and `0x47a4c0` facial hair styles from the count table. `0x47a470` returns the colour count of skin variation 0 and nothing calls it.

Addresses: `0x477c20`, `0x47a3e0`, `0x47a400`, `0x47a440`, `0x47a470`, `0x47a4a0`, `0x47a4c0`, `0xc0df6c`, `0xc0df70`, `0xb4231c`, `0xb4244c`
Confidence: verified
Proof: one
Test: none

`0x477c2b cmp esi,5; jge` and `0x477c35 jl` bound section and variation; `0x477c45` reads the grid base and `0x477c4b lea eax,[ecx+eax*8]` takes the entry. `0x477c50 cmp edx,ecx; jge` and `0x477c56 je` test the count, and
`0x477c68 cmp ecx,eax; jge` with `0x477c6e je` test the colour count at the record `[recptr + 12·variation]`. `0x47a3e0` reads the count at entry 3's `+0x18` and returns 0 without a record pointer (`0x47a3f2`). `0x47a440` returns the record's `+4`
(`0x47a45b`). `0x47a4a0` needs entry 1's record pointer at `0x47a4af` and a positive count. `0x47a4c0` reads `[0xb4244c][sex + 2·race]`.

## Section cell

`0x4782e0(section, sub, record, colour, group)` makes the cell for one texture name of a CharSections row: the skin row of the component's race and sex, section and `record` variation, colour. The cell is `+0x188 + 4·(sub + 8·section)`. It
releases the old cell, and when the lookup passes and the row's name `sub` (`+0x18 + 4·sub`) is not empty, loads the name as kind 5 for section 0 and kind 9 for the others. The group bit is set in `+0x10` whenever the lookup
passes, for an empty name or a failed load too.

Addresses: `0x4782e0`, `0x47b010`, `0x47ae30`, `0x477c20`, `0x4420b0`, `0xb422f0`
Confidence: verified
Proof: one
Test: none

`0x4782f2 lea edi,[esi+ecx*4+0x188]` is the cell; `0x478302 call 0x47b010` releases the old one. `0x478318 call 0x477c20` gates; a failure jumps to `0x4783d9` and sets no bit. The id is bounds-checked against `[0xc0df70]` (`0x478351`), and an id outside it
gives a null row that `0x478369` reads at `+0x18 + 4·sub`. `0x478382 test ebx,ebx` picks kind 5 (`0x47838b`) for section 0, else kind 9 (`0x47839e`). `0x4783c7` to `0x4783d6` or the bit `1 << group` into `+0x10` on every path past the gate; an empty name (`0x47836d`) skips the load only.
The name is copied byte by byte to the path cursor (`0x478372` to `0x47837e`).

## Skin

`0x478790(skin, face detail)` stores `+0x24` and, unless baked, builds the skin. When the skin is below the base count of skin variation 0 it loads the underwear cells `+0x208` and `+0x20c` from section 4: the two texture names of the row `ids[skin]` of variation 0,
kind 5, each skipped when empty. It then applies the body skin (`0x478160`), makes the skin cell (section 0, sub 0, group 3), applies the face (`0x4785b0`), and marks a full rebuild with every group dirty.

Addresses: `0x478790`, `0x478160`, `0x4782e0`, `0x4785b0`, `0x477c20`, `0x47b010`, `0x47ae30`, `0x449d90`, `0x710ec0`, `0x58a980`, `0x41aed0`
Confidence: verified
Proof: one
Test: none

`0x47879f` stores the skin before the baked test (`0x4787a2`); `0x4787ad call 0x477c20(0, 0, skin)` gates. `0x4787d2 cmp edi,[ecx+4]; jge 0x4788b8` skips the underwear for a skin at or above the base count. The section 4 record is `[entry + 0x24]`
(`0x4787db`), its ids `[+8]` (`0x4787de`). The loads are `0x478854` and `0x478892` (`edx = 5`); the old cells are released first (`0x478807`, `0x478820`). `0x478160` gates with `0x477c20(0, 0, skin)`, takes the same row's `+0x1c` name (the second texture name),
loads it and binds it as replaceable type 8 (`0x478204 call 0x710ec0`); an empty name or a failed load binds nothing. `0x4788e1` stores -1 at `+0x10` and `0x4788e8` sets `+0x08`.

## Face

`0x4785b0(face, detail)` runs when the component is not baked and the skin is below the base count. A face passes `0x477c20(1, face, skin)`; it stores `+0x2c` and makes two cells of section 1 with the skin as the colour: sub 0 for group 9 and sub 1 for
group 8. With the detail flag it also makes the section 3 cells for the hair style and colour: sub 1 for group 9 and sub 2 for group 8.

Addresses: `0x4785b0`, `0x4782e0`, `0x477c20`, `0x4785db`, `0x478602`, `0x478617`, `0x478633`, `0x478648`
Confidence: verified
Proof: one
Test: none

`0x4785b6` tests `+0x3c` and `0x4785db cmp eax,[ecx+4]; jge 0x47864d` tests the skin against the base count. The cells are made at `0x478602` (section 1, sub 0, bit 9), `0x478617` (sub 1, bit 8), `0x478633` (section 3, sub 1, bit 9) and `0x478648` (section 3, sub 2, bit 8), the last two after `0x47861c`'s test of the detail flag.
The face index is stored at `0x4785ff` after the gate passes.

## Hair

`0x4784c0(style)` applies a hair style: it needs `0x477c20(3, style, hairColor)`, stores `+0x34`, sets the first geoset base `+0x144` to the CharHairGeosets value of `0x478540` (the row of the race, sex and style, minimum 1, 1 when no row), makes the section 3 cells for
the style (sub 1 for group 9, sub 2 for group 8) unless baked, applies the hair colour (`0x4783f0` with the detail flag clear), and marks `+0x08` and `+0x0a`. `0x4783f0(colour, detail)` stores `+0x20` after
its own gate, binds the hair texture, and remakes the hair and facial hair cells with the new colour. `0x478220(variation)` binds the section 3 row's first texture name as replaceable type 6.

Addresses: `0x4784c0`, `0x478540`, `0x4783f0`, `0x478220`, `0x478445`, `0x478450`, `0x4782ca`, `0xc0df78`, `0xc0df7c`, `0x477c20`, `0x4782e0`, `0x58a980`, `0x449d90`, `0x710ec0`
Confidence: verified
Proof: one
Test: none

`0x4784e0 call 0x478540` with `ecx = &cc+0x18`: it scans `[0xc0df78]` (0x18-byte rows, count `[0xc0df7c]`) for race `+4`, sex `+8` and variation `+0xc` equal to the component's and returns `+0x10` when positive (`0x47859a`). `0x4783f0` stores `+0x20` at `0x478413` and calls
`0x478220(1)` at `0x478445` only for sex 0, hair style 0 and a race whose flags have bit 3 (`0x478432`, `0x478439`, `0x47843b test [eax+4],8`), then `0x478220(hair style)` at `0x478450` always. `0x478220` gates with `0x477c20(3, variation, hairColor)`, builds the path
from the row's `+0x18`, and binds at `0x4782ca push 6`; it leaves the slot as it was for a failed gate, an empty name or a failed load. Then `0x4783f0` makes the section 3 cells with the detail flag set (`0x478473`, `0x478488`) and the section 2 cells for the facial hair (`0x47849d`, `0x4784b2`).

## Facial hair

`0x478660(style, detail)` takes a facial hair style from 0 to the count of `0x47a4c0` inclusive, stores `+0x30`, and looks the style up in `[0xc0df28]` (`0x478740`: race `+0`, sex `+4`, style `+8`). A row sets the bases `+0x148 = +0x18 + 0x64`, `+0x150 = +0x1c + 0x12c`
and `+0x14c = +0x20 + 0xc8` and marks the geosets dirty. When the row has a nonzero geoset and the section 3 cells for group 9 and 8 are not both set, it binds the hair texture of variation 1 with `0x478220(1)`. Unless baked and with the detail flag, it makes
the section 2 cells (sub 0 for group 9, sub 1 for group 8).

Addresses: `0x478660`, `0x478740`, `0x47a4c0`, `0x4786f2`, `0x4786da`, `0x478715`, `0x47872a`, `0xc0df28`, `0xc0df2c`, `0x478220`, `0x4782e0`
Confidence: verified
Proof: one
Test: none

`0x478667 test ebx,ebx; jl` and `0x478681 cmp ebx,eax; jg` bound the style. `0x47868b` stores `+0x30`. `0x478697` to `0x4786bb` add the bases; `0x4786c1` sets `+0x0a`. The three geoset ints are tested at `0x4786c8`, `0x4786cc` and `0x4786d3`, the cell pair at `0x4786da` to `0x4786ec`, and
`0x4786f2 call 0x478220` pushes 1. `0x478740` scans rows of 0x28 bytes by count `[0xc0df2c]` and returns null when none matches (`0x47877d`).

## Replaceable texture types

The file binds five kinds of replaceable texture to a character model, by the type `0x710ec0(model, type, texture)` takes. A call binds every texture record of the model with that type.

| type | bound to | site |
|---|---|---|
| 1 | composite target, baked skin | `0x477970`, `0x479622` |
| 2 | item model and cloak textures | `0x479935`, `0x479c1a`, `0x47a045` |
| 6 | hair | `0x4782cc` |
| 8 | extra skin | `0x478204` |
| 11 to 13 | creature skins | `0x479891`, `0x5f7ba5` |

Addresses: `0x710ec0`, `0x477970`, `0x479622`, `0x479935`, `0x479c1a`, `0x47a045`, `0x4782cc`, `0x478204`, `0x479891`, `0x5f7ba5`, `0x7101d6`, `0x76d05c`
Confidence: verified
Proof: one
Test: none

A scan of the listing finds eleven calls to `0x710ec0`. Nine push the types above: a literal 1, 2, 6 or 8 before the call (`push 0x1` at `0x477970`, `push 0x6` at `0x4782cc`, `push 0x8` at `0x478204`), or `lea edx,[edi+0xb]` with `edi` below 3 (`0x4798a1 cmp edi,3`) at `0x479891` and `lea ecx,[edi+0xb]` at `0x5f7ba5` (`0x5f7bb5 cmp edi,3`). The other two pass a variable type: the deferred-call replay at `0x7101d6` and `0x76d05c`. No call pushes a literal 7.
