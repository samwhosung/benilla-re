# charactermodel: geosets

A character model carries every body part and garment variant as a geoset: a submesh with a numeric id, grouped by hundreds. The component shows the ones its appearance and
items select by switching ranges of ids on the model. This doc covers the geoset bases, the pass that switches them, and the helm's mask that replaces some bases.

Roots: none

## Geoset bases

The component keeps 16 geoset ids at `+0x144`, one per group of 100: `1, 0x65, 0xc9, 0x12d, 0x191, 0x1f5, 0x259, 0x2be, 0x321, 0x385, 0x3e9, 0x44d, 0x4b1, 0x515, 0x579, 0x5dd`.
The first four are hair, facial hair 1, 2 and 3. The hair setter writes the first from CharHairGeosets, the facial hair setter writes the second to fourth, the helm mask (`geosets#helm-mask`) replaces five, and an unequipped head puts the
customization values back. The entry at `+0x160`, group 7, starts at `0x2be`, the ears.

Addresses: `0x4784e5`, `0x47869d`, `0x4786ac`, `0x4786bb`, `0x476841`, `0x476869`, `0x4789b2`, `0x4789c1`, `0x4789cf`, `0x4789d5`, `0x478996`, `0x479aed`, `0x479afc`, `0x479b0b`, `0x479b1a`, `0x479b29`
Confidence: verified
Proof: one
Test: none

The constructor stores the bases at `0x476841` to `0x4768d7`; the ears at `+0x160` are `0x2be` (`0x476869`). The hair setter stores `+0x144` at `0x4784e5`, the facial hair setter `+0x148`, `+0x150` and `+0x14c` at `0x47869d`, `0x4786ac` and `0x4786bb`. Unequipping the head (`0x478960` with slot 0) restores `+0x144`
from `0x478540` (`0x478996`) and `+0x148`, `+0x150`, `+0x14c` from the facial hair row (`0x4789b2`, `0x4789c1`, `0x4789cf`), and `+0x160` to `0x2be` (`0x4789d5`). The helm mask stores `1`, `0x65`, `0xc9`, `0x12d` and `0x2bd` (`0x479aed` to `0x479b29`), the ears one lower than their default.

## Geoset pass

`0x477520` runs when `+0x0a` is set. It hides ids 0 to 0x6a4, shows id 0 and every base, and then shows or hides ranges for the worn items (an item record has three geoset group values at `+0x18`, `+0x1c` and `+0x20`; "has group n" is `record != 0` and value `n` nonzero). The
pass ends by rebuilding the model's batches and clearing `+0x0a`.

| item | condition | effect |
|---|---|---|
| hands | group 0 | hide `0x191`..`0x1f3`, show `0x191 + value` |
| chest | no hands group 0, group 0 | show `0x321 + value` |
| shirt | group 0, and row 1 columns 1 to 6 empty | show `0x321 + value` |
| chest or legs | group 2 (a robe) | hide `0x1f5`..`0x257`, `0x386`..`0x3e7`, `0x44c`..`0x4af`, `0x514`..`0x577`; show `0x515` plus the chest's value 2, else the legs' |
| feet | no robe, group 0 | hide `0x1f5`..`0x257`, show `0x385` and `0x1f5 + value` |
| legs | no robe or feet group 0 | show `0x385 + value 1` if group 1, else `0x385` |
| tabard | no robe, group 0 | show `0x4b1 + value` |
| tabard preview | `+0x0c` | show `0x4b1`, and `0x4b2` unless a robe |
| shirt, legs | chest not a robe, no tabard group 0 | show `0x3e9 + shirt value 1` and `0x44e + legs value` |
| cloak | group 0 | hide `0x5dc`..`0x63f`, show `0x5dd + value` |

Addresses: `0x477520`, `0x4774f0`, `0x47a880`, `0x47a8a0`, `0x47a8c0`, `0x47a8e0`, `0x7110d0`, `0x711230`, `0x477530`, `0x477559`, `0x477564`, `0x47757a`, `0x477639`, `0x47768c`, `0x4776d0`, `0x477752`, `0x477799`, `0x47780a`, `0x477847`, `0x4775c8`
Confidence: verified
Proof: one
Test: none

`0x477530 push 0x6a4`/`0x477535 push 0` is the first call (hide). `0x477559 call 0x7110d0` shows each base, 16 times from `0x477550`. The slots are `+0x4c8` hands, `+0x4b4` chest, `+0x4b0` shirt, `+0x4bc` legs, `+0x4c0` feet, `+0x4cc` tabard, `+0x4d0` cloak. `0x4775c8 cmp [ecx],0` tests row 1's cells `+0x26c` to `+0x280`.
`0x4774f0(record, n)` is true when `record` is nonzero and `[record + 4n + 0x18]` is. The robe test is group 2 of the chest (`0x477601`) or of the legs (`0x477615`); its branch is `0x47768c`. Feet hide the whole boots range at `0x477639`. `0x47a8e0` calls `0x711230` and clears `+0x0a`. The preview shows `0x4b1` at `0x477759` and `0x4b2` at `0x47778d`.
`0x47a8c0(i)` shows base `i` and nothing calls it.

## Helm mask

`0x4799a0` applies the head item. It does nothing when the head record is null, its first model name is empty, the race is out of range or the race record is null. Otherwise it loads the helm model and attaches it at attachment 11, and looks up the HelmetGeosetVisData row of the
record's sex (`+0x30` male, `+0x34` female). For each of five fields of that row (`+4` to `+0x14`), a bit set for the component's race replaces one geoset base with its group default: hair `1`, facial hair `0x65`, `0xc9`, `0x12d` and the ears `0x2bd`. The next geoset pass then shows those defaults.

Addresses: `0x4799a0`, `0x4799b5`, `0x4799be`, `0x4799cc`, `0x4799d8`, `0x4799e9`, `0x479a05`, `0x479a41`, `0x479ab3`, `0x479ac1`, `0x479ac9`, `0x479ad1`, `0x479ae6`, `0x4798c0`, `0x838cd8`, `0x838cc8`, `0x838cbc`, `0x73f700`
Confidence: verified
Proof: one
Test: none

The early outs are `0x4799b5 je`, `0x4799c1 je` (an empty string, tested by its first byte), `0x4799cc jl`, `0x4799d8 jg` and `0x4799e9 je`, all to `0x479b33`. The model name is cut at its last dot (`0x479a08 call 0x73f700`) and the path is `Item\ObjectComponents\Head\<name>_<race string><sex letter>.mdx`
(`0x479a41`) with the texture `<dir><record +0xc>.blp` (`0x479a82`), loaded by `0x4798c0` with attach id 11 (`0x479aae mov edx,0xb`). The load's result is not tested (`0x479ac1` overwrites `eax`). The row index is bounds-checked against `[0xc0dc50]` (`0x479ac9`), the row is `[0xc0dc4c][id]` (`0x479ad1`), and the bit test is `1 << race` (`0x479ae6 shl eax,cl`).
