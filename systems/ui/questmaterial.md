# ui: quest page material and the reward error

The Lua functions that name the page material of an open quest dialog or item text, the table of material names they read,
and the error the quest frame raises when no reward was chosen. The quest messages are `objectlayer/quests`; the reward spell
functions are `ui/questrewardspell`.

Roots: `0x502230`, `0x4e39f0`, `0x5021a0`, `0x54b640`

## Quest background material

`GetQuestBackgroundMaterial()` always returns one value, a string or nil. It looks up the object whose GUID the open quest
dialog holds, under any type mask; no object gives nil. An item object gives the page material of its cached item record
(nil when the record is not cached, and no query is sent). A game object gives the page material column of its template
(`objectlayer/gameobjecttypes#quest-givers-text-and-mailboxes`). Any other object, a creature or a player, gives nil. The
material must be above 0, at most the table's highest id and have a row; the result is that row's name. `ItemTextGetMaterial()`
is the same function on the GUID of the open item text.

Addresses: `0x502230`, `0x502250`, `0x502258`, `0x50226a`, `0x502273`, `0x502282`, `0x50228d`, `0x50229a`, `0x5022a1`, `0x5022a3`, `0x5022b3`, `0x5022b7`, `0x5022bc`, `0x5022c0`, `0x5022c6`, `0x5022ce`, `0x5022d5`, `0x5022db`, `0x5022ee`, `0x5022fd`, `0x4e39f0`, `0x4e3a18`, `0x4e3a63`, `0x4e3a77`, `0x4e3a80`, `0x4e3a88`, `0x4e3aa0`, `0x5f5950`, `0x55ba30`, `0x7c9640`, `0x7c9910`, `0xbe0810`, `0xbe0814`, `0xbc3fa0`, `0xc0e2a0`
Confidence: verified
Proof: one
Test: none

Both functions fetch the object with `0x468460` and a type mask of 1 (`0x50226a`, `0x4e3a2a`). The object's type word `+8` of its
descriptor block decides: bit 1 (`shr edx,1; test dl,1`, `0x50227d`) is an item, bit 5 (`shr ecx,5`, `0x5022ad`) a game object. The item
entry is the descriptor dword at `+0xc`; the record comes from `0x55ba30` with a null callback, so a miss answers 0 and the
function leaves with nil (`0x5022a1`). The record offset `0x1a4` is the third dword after the description in the item query
reply (`0x7c98ee`, `0x7c9910`). The string is pushed through `lua_pushstring` (`0x5022db`), so a null name is nil too. Every
failure is the nil leg at `0x5022ee`; the success leg and the failure leg both return 1.

## Quest dialog GUID

The quest dialog's source GUID is a pair of cells. Three places write it: the world-enter reset (`0x500af2`, `0x500af7`) zeroes
it, the dialog latch `0x500bd0` stores the GUID that opened the dialog (`0x500d7f`, `0x500d86`), and closing the quest frame zeroes it
(`0x501201`, `0x501207`). Each of the four quest-giver readers that opens a dialog calls the latch with the GUID its packet names. No
message carries a material: the value comes from the object's template through the caches.

Addresses: `0x500af0`, `0x500af2`, `0x500af7`, `0x500bd0`, `0x500d7f`, `0x500d86`, `0x501130`, `0x501201`, `0x501207`, `0x5dbba1`, `0x5dbfe6`, `0x5dc155`, `0x5dc3a7`
Confidence: verified
Proof: one
Test: none

The four callers of the latch are the quest list reader (`0x5dbba1`), the details reader (`0x5dbfe6`), the request-items reader
(`0x5dc155`) and the offer-reward reader (`0x5dc3a7`). `0x5dc3a2 mov ecx,3` selects the dialog kind of the last one.
The quest-giver dispatcher is `objectlayer/quests#quest-giver-messages`.

## Page material table

`PageTextMaterial.dbc` is loaded at startup. The loader accepts only the signature `WDBC`, 2 fields and 8 bytes a record, and
indexes the rows by id: the highest id is tracked and `index[row.id]` holds the row. A row is the id (`+0`) and the name (`+4`),
a pointer into the string block. The store keeps the records, their count, the index and the highest id, -1 until loaded. The
names are the file's; no material name occurs in the executable.

Addresses: `0x54b640`, `0x54b695`, `0x54b6e9`, `0x54b71e`, `0x54b7b6`, `0x54b840`, `0x54b84b`, `0x53c510`, `0x53c521`, `0x57eef0`, `0xc0da14`, `0xc0da18`, `0xc0da1c`, `0xc0da20`
Confidence: verified
Proof: one
Test: none

The checks are `cmp [..],0x43424457` (`0x54b695`), `cmp eax,2` (`0x54b6e9`) and `cmp eax,8` (`0x54b71e`). The index store is
`mov [edi+edx*4],ecx` (`0x54b84b`) with the row id in `edx`. The initializer `0x53c510` clears the cells and sets the highest id to -1
(`0x53c521`). The file name comes from the thunk `0x57eef0`.

## Reward error

`QuestChooseRewardError()` raises message 152 and returns no values. It reads no argument and has no condition. The message
is `ERR_QUEST_MUST_CHOOSE`, shown as an error line (the `UI_ERROR_MESSAGE` event), and it plays the interface sound named
`igQuestFailed`, because its tag word is 0x44 and its sound name is not `NONE`. Nothing is sent to the server.

Addresses: `0x5021a0`, `0x5021a5`, `0x48612f`, `0x486139`, `0x48613e`, `0x486144`, `0x48614a`, `0x484ca0`, `0x496720`, `0x49673d`, `0x496784`, `0x4967ab`, `0x4945b0`
Confidence: verified
Proof: one
Test: none

The whole body is `push 0x98; call 0x496720; add esp,4; xor eax,eax; ret` (`0x5021a0` to `0x5021af`). The row of message 152 is
written by constants: the key (`0x48612f`), kind 2 (`0x486139`), the sound name (`0x48613e`), the tag (`0x486144`) and channel 10 (`0x48614a`).
The kind sends the text to `0x4945b0` as an error, and the tag selects the by-name sound (`ui/inventory#failure-messages`,
`ui/taxiroute#new-flight-path`).
