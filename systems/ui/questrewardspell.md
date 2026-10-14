# ui: quest reward spell functions

`GetRewardSpell` and `GetQuestLogRewardSpell` (`0x501df0`, `0x4e1130`): the three values they return for a spell, where
the spell id comes from in the quest messages and in the cached quest record, and what is never cleared. The dialog
messages are `objectlayer/quests`; the page material functions are `ui/questmaterial`.

Roots: `0x501df0`, `0x4e1130`

## Reward spell values

`GetRewardSpell()` always returns three values for the reward spell id of the last quest details or offer reward message:
the texture file name of the spell's icon, the spell's name for the client locale, and the number 1 when bit 5 (0x20) of the
spell's attributes is set, else nil. The id must be above 0, at most the spell table's highest id and have a row; otherwise
the three values are nil. The icon is nil when the spell's icon id is negative, above the icon table's highest id, has no row,
or the row's file name is null. The name is nil only when its pointer is null. The three are decided independently.

Addresses: `0x501df0`, `0x501df5`, `0x501dff`, `0x501e07`, `0x501e0d`, `0x501e14`, `0x501e1e`, `0x501e26`, `0x501e2c`, `0x501e33`, `0x501e46`, `0x501e4b`, `0x501e59`, `0x501e61`, `0x501e75`, `0x501e85`, `0x535bb0`, `0xc0d780`, `0xc0d788`, `0xc0d78c`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

The spell row is `index[id]` of the spell store, bounded by `[0xc0d78c]` (`0x501dff`, `0x501e07`, `0x501e0d`). The icon id is the dword at
`+0x1d4` of the spell record (`0x501e14`), looked up by id in the icon store, bounded by its highest id; the file name
is `+4` of the icon row (`0x501e33`). The name is `+0x1e0 + locale * 4` of the spell record (`0x501e4b`) with the global locale index.
The third value tests `[record+0x18]` against 0x20 (`0x501e59`); a clear bit takes `0x501e85`, which pushes nil, and a set bit pushes
the double 1.0 (`0x501e61`). The failure leg pushes three nils (`0x501e75`). Every path returns 3. The attribute bit is the one
that makes a spell show the item it creates (`ui/tooltipspell`).

## Reward spell source

The reward spell id of `GetRewardSpell` is one cell. It has one writer, the quest panel publisher `0x500ef0`, which stores its
ninth stack argument, and two readers, `GetRewardSpell` and the reward spell tooltip. Two messages call the publisher: the
quest details message (0x188), which reads the id after the money and before the emote list, and the offer reward message
(0x18d), which reads one unused u32 after the money and then the id. The request-items message (0x18b) publishes through
another function. Nothing clears the cell: the world-enter reset and the dialog teardown skip it, so the function answers
from the latest details or offer reward message, whichever panel is open, and from 0 before any.

Addresses: `0xbe0830`, `0x500ef0`, `0x501040`, `0x50104a`, `0x500af0`, `0x501130`, `0x5dbd50`, `0x5dbe87`, `0x5dbe92`, `0x5dc030`, `0x5dc1e0`, `0x5dc375`, `0x5dc380`, `0x5dc38b`, `0x5dc3e3`, `0x5dc050`, `0x535c35`, `0x535bb0`
Confidence: verified
Proof: one
Test: none

The cell has three references in the image: the store `0x50104a`, the read `0x501df0` and the read `0x535c35`. The store takes
`[ebp+0x28]` (`0x501040`), the ninth of the ten stack arguments. The callers of the publisher are `0x5dc030` and `0x5dc3e3`
only. In the details reader the id is the dword read after the money (`0x5dbe87` money, `0x5dbe92` id). In the offer reward reader
the dword read at `0x5dc380` is stored to a local that nothing in the function reads again, and the id is read at `0x5dc38b`. The
reset `0x500af0` writes the dialog cells one by one and does not name `0xbe0830`.

## Quest log reward spell

`GetQuestLogRewardSpell()` returns the same three values for the reward spell of the selected quest log entry. It reads the
cached quest record of the selected quest id, with no query sent; a quest that is not cached gives three nils. The spell id is
the dword at `+0x30` of that record, the thirteenth u32 of the quest query reply (opcode 0x5d). From the spell id on the
function is the body of `GetRewardSpell`.

Addresses: `0x4e1130`, `0x4e1149`, `0x4e115c`, `0x4e1167`, `0x4e116c`, `0x4e1174`, `0x4e117d`, `0x4e1192`, `0x4e11d8`, `0xbb7480`, `0xc0e1b0`, `0x562a40`, `0x562c4c`, `0x555300`, `0x55507a`, `0x6dbf60`, `0x6dbfec`, `0x562c80`, `0x562d40`
Confidence: verified
Proof: one
Test: none

The selected quest id is the cell `0xbb7480` (`0x4e1149`); `0x562a40` returns the value area of the node, `node + 0x18` (`0x562c4c`), or 0
(`0x4e116c`). The reply handler `0x555300`, registered for 0x5d (`0x55507a`), decodes into a staging record with `0x6dbf60`, which reads
thirteen consecutive u32 fields into offsets 0 to 0x30; the thirteenth is `+0x30` (`0x6dbfec`). The insert `0x562c80` copies the
record dword by dword (`0x562d40`). The id then passes the same four gates as `0x501df0`: not 0 (`0x4e1177`), not negative, at most the
spell table's highest id (`0x4e117d`) and a row (`0x4e118e`); the rest repeats it (`0x4e1192` to `0x4e11d8`).
