# ui: the craft window list

Where the craft window's rows come from and the order they show in: the window that opens for Beast Training, Disguise,
Enscribe and Enchanting (`CraftInfo.cpp`, `0x4f60c0` to `0x4f6b28`). The profession window is a separate frame with its
own comparators.

Roots: `0x4f60c0`, `0x4f6630`, `0x4f67a0`, `0x4f6920`, `0x4f6ac0`

## Craft lists

A learned spell enters craft list N when its attribute word has bit 0x20 clear and its cast-UI field (`+0xc` of the spell
record) is N above 0; the lists are four vectors in the player object at `+0x1cd0`, 16 bytes apiece. Learning one appends
it and rebuilds the window list. The window's craft type is the misc value of the first effect of the opener spell whose
first effect is 47; 1 is Beast Training, 2 Disguise and 3 Enscribe or Enchanting, while a misc value of 0 opens the
profession window.

Addresses: `0x5e9c8e`, `0x5e9c92`, `0x5e9d83`, `0x5e9d88`, `0x5e9d93`, `0x5e9e12`, `0x5ea330`, `0x4f60c0`, `0x4f601c`, `0x4f6c90`, `0x6e4bce`, `0x6e4bd7`
Confidence: verified
Proof: one
Test: none

`0x5e9c8e test [edi+0x18],0x20; je` leaves the trade-skill branch for the craft branch at `0x5e9d83`, which requires the
cast-UI field above 0 and appends the id to the vector at `0x1cd0 + 0x10 * field`. `0x5ea330(type)` returns that vector
for type below 4. The craft type is stored at `0x4f601c`; the button token table is `USE`, `TRAIN`, `DISGUISE`, `ENSCRIBE`
for types 0 to 3 (`0x4f6c90`).

## Craft rows

A craft row is 16 bytes: the spell id (-1 for a header), the group key (the skill line of the spell's ability), the count
available and a difficulty bucket from 0 to 5. A header is 12 bytes: the group key, its member count and a visibility word.
The bucket is the string `GetCraftInfo` returns as the craft type: none, optimal, medium, easy, trivial, used. The window
shows the sent name and rank of the row's own spell.

Addresses: `0x4f6e90`, `0x4f6f9b`, `0x4f6fae`, `0x4f6fbc`, `0x4f61dd`, `0x4f6222`, `0x4f623f`, `0x4f6255`, `0x4f6267`, `0x4f6360`, `0x4f63cc`, `0x4f65c5`, `0x4f666d`
Confidence: verified
Proof: one
Test: none

The record sizes are the allocation sizes `push 0x10` (`0x4f6146`, `0x4f650e`) and `push 0xc`. The bucket is set to 0 at
`0x4f61dd`, to 1 to 4 by the difficulty ladder (`0x4f6222` to `0x4f6267`) and to 5 when the pet already knows the ability
(`0x4f6360`, `0x4f63cc`); the strings come from the table at `0x807e00`, 32 bytes apiece (`0x4f6fbc`). A header is visible when
its bit is clear in the collapse mask `0x84d0f0`, which opening sets to all ones.

## Craft order

Rows and headers are sorted by the recount `0x4f6630`. The row comparator is `0x4f6920` for craft type 1 and `0x4f67a0`
for the others. Both order by group visibility, the group's position in the name-sorted header array, the header row first,
the difficulty bucket (ascending), then the spell name case-insensitively; only `0x4f6920` adds the spell's level
(`+0x74`) last, ascending. There is no rank key. Headers sort by the skill line's display name. With a single group the
header count becomes 0, so no header shows and the buckets, names and levels decide the order.

Addresses: `0x4f675b`, `0x4f6765`, `0x4f6785`, `0x4f6756`, `0x4f6920`, `0x4f67a0`, `0x4f6984`, `0x4f69cc`, `0x4f69e5`, `0x4f6a34`, `0x4f6a52`, `0x4f6a5b`, `0x4f6a65`, `0x4f6ac0`, `0x4f6b1e`, `0x4f65fb`, `0x4f6602`, `0x4f6630`, `0x64a4c0`
Confidence: verified
Proof: one
Test: `ui::listorder_craft_beast_training_e2e`, `ui::sortcmp_4f67a0`, `ui::sortcmp_4f6920`

`0x4f6a57 test eax,eax; jne` reaches the level key only when the name compare gives 0. The level key never returns 0 for equal
levels, which is harmless for the shipped spells. `0x4f65fb cmp ecx,1; jne` followed by the store of 0 at `0x4f6602` is the
single-group suppression. The three comparators contain three calls in total, all to the case-insensitive compare, and no
read of the rank text (`+0x204`).

`ui::listorder_craft_beast_training_e2e` runs `0x4f6630` whole on a synthesized list, with the real `qsort`, comparator and collator, and reads the sorted rows.
Bit-exact coverage: `ui::sortcmp_4f67a0` covers `0x4f67a0` to `0x4f67ef`, `0x4f67f3` to `0x4f691c` of `0x4f67a0`; `ui::sortcmp_4f6920` covers `0x4f6920` to `0x4f696f`, `0x4f6973` to `0x4f6abc` of `0x4f6920`.
