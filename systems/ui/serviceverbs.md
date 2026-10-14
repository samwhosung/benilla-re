# ui: GM, service and item lookup verbs

The Lua functions of `GameUI.cpp` that reach the GM ticket and survey system, the spirit healer, talent master and
innkeeper, the helm and cloak preferences, the item enchant confirmation, and the two item lookups `GetItemInfo` and
`GetItemQualityColor`. The wire messages and the player's methods they call are in `objectlayer`
(`playermessages`, `interaction`, `inventory`).

Roots: `0x48c9a0`, `0x48c9d0`, `0x48ca60`, `0x48cb70`, `0x48cc00`, `0x48cd30`, `0x48cfc0`, `0x48d120`, `0x48d4c0`,
`0x48d8d0`, `0x48dae0`, `0x48dd60`, `0x48dfb0`, `0x48e070`

## GM tickets

`GetGMTicket()` and `DeleteGMTicket()` call `SendGmTicketGetTicket` and `SendGmTicketDelete` on the active player.
`NewGMTicket(type, text)` and `UpdateGMTicket(type, text)` need a number and a string, raise
`Usage: NewGMTicket(type, text)` (and the `UpdateGMTicket` form) otherwise, and call `CreateGmTicket` and `UpdateGmTicketText`
with the truncated type and the text. `GetGMStatus()` sends message 0x21a. `GMRequestPlayerInfo()` always raises
`Access Denied`. `GetGMTicketCategories()` returns, for each of the categories in its table, the id and the category's
name in the current locale.

Addresses: `0x48c9a0`, `0x48c9d0`, `0x48ca36`, `0x48ca40`, `0x48ca60`, `0x48cac6`, `0x48c9c1`, `0x48cb70`, `0x48cb91`, `0x48cfc0`, `0x48cfc9`, `0x48d4c0`, `0x48d4c6`, `0x48d8d0`, `0x48d8d8`, `0x48d917`, `0x48d92c`, `0x5ef740`, `0x5efac0`, `0x5efbb0`, `0x5efc20`
Confidence: verified
Proof: one
Test: none

`0x48ca36` calls `0x5ef740` with the type and text. `0x48ca36` calls `0x5ef740` and `0x48cac6` `0x5efac0`; `0x48c9c1` and `0x48cb91` call `0x5efbb0` and `0x5efc20`. The category table is the
GMTicketCategory store (rows at 0xc0dc80, count at 0xc0dc84) of 0x28-byte
records: the id at `+0` and the name by locale at `+4 + 4·locale` (`0x48d92c`); the verb returns twice the count.

## GM surveys

`GMSurveyQuestion(index)` returns the survey's question text for an index from 1 to 10, or nil. `GMSurveyAnswerSubmit(question,
rank, comment)` stores an answer: it raises `Question limited from 1 to 10` for a question
outside 1 to 10 and `Ranks limited from 0 to 5` for a rank above 5, and keeps the rank byte and the comment for the
question. `GMSurveyCommentSubmit(comment)` keeps the final comment. `GMSurveySubmit()` sends message 0x32a: the survey id (u32), then for each
of up to ten questions its id (u32), and when that id is not 0 its rank (u8) and comment (zero-terminated); a written id of 0 ends the list, and ten answers have none; then the final comment. It then frees the answers.

Addresses: `0x48cc00`, `0x48cc34`, `0x48cc39`, `0x48cc40`, `0x48cd30`, `0x48cd9d`, `0x48cda7`, `0x48cdc6`, `0x48cef0`, `0x48cf21`, `0x48cf80`, `0x4964b0`, `0x496440`, `0x496470`, `0x496570`, `0x4965e0`, `0x496630`, `0x49663a`, `0x496510`, `0x4966bc`, `0xb4e380`, `0xb4b45c`, `0xb4e3a8`
Confidence: verified
Proof: two
Test: none

`0x48cc34` and `0x48cc39` bound the index to 1 to 10. `0x4964b0` reaches the question text through a survey store keyed by
`0x496440` (two stores deep) and the locale string at `+4 + 4·locale`. The answer rank bytes are at `0xb4b45c` and the ten
comment strings at `0xb4e380`, the final comment at `0xb4e3a8`; `0x496570` replaces them with `SStrDupA` copies of non-empty
text. `0x496470` returns the id of question `i` from the survey record (`+4 + 4·i`), 0 for an index of 10 or more or a missing survey.
`0x496630` writes 0x32a and the survey id, then for each question calls `0x496470` (`0x496674`) and writes its result (`0x496686`); `0x49668b` leaves the loop on a 0 id, after the write. A nonzero id is followed by the rank byte (`0x49669b`) and the comment, an empty string for none (`0x4966b1`); the final comment follows (`0x4966c1`). The loop bound is 10 (`0x4966bc`).

## Spirit healer, talent master and innkeeper

`CheckSpiritHealerDist()`, `CheckTalentMasterDist()` and `CheckBinderDist()` return 1 when the remembered NPC is in range
for the active player, else nil. `AcceptXPLoss()` calls `SpiritHealerDialog` and `ConfirmBinder()` calls `BinderDialog` on the
player. `GetBindLocation()` returns the name of the bind point's area in the current locale, or the text `HOME_INN` when the
area is unknown. `ConfirmBindOnUse()` confirms the use of the held bind-on-use item.

Addresses: `0x48d120`, `0x48d170`, `0x48d210`, `0x48d0c0`, `0x48d106`, `0x48dd00`, `0x48dae0`, `0x48dae3`, `0x48db07`, `0x48db23`, `0x48d770`, `0x5df8c0`, `0x5dfb10`, `0x5dff50`, `0x5dfdc0`, `0x5df730`, `0x5ed420`, `0x5d9290`, `0x703bf0`
Confidence: verified
Proof: one
Test: none

The distance verbs call `IsSpiritHealerInRange` (`0x48d144`), `IsTalentWiperInRange` and `IsBinderInRange` on the player and
push the number 1 or nil. `0x48d106` calls `SpiritHealerDialog(0, 0)`. `0x48dae3` gets the bind zone, `0x48db07` reads the area
name at `+0x2c + 4·locale` of its record, and `0x48db23` fetches the global string `HOME_INN`. `0x48d770` calls `0x5d9290`.

## Helm, cloak and enchants

`ShowingHelm()` and `ShowingCloak()` return 1 when the player's flags word is clear of bit 10 (`0x400`) and bit 11 (`0x800`), and
nil with no player. `ShowHelm([flag])` and `ShowCloak([flag])` call the show method for a true flag, true when absent, and the hide method
otherwise. `BindEnchant()` accepts the item enchant that waits on the bind confirmation, and `ReplaceEnchant()` binds the held
item to the pending spell target (`BindSpellTargetObject`).

Addresses: `0x48dd60`, `0x48dd82`, `0x48dd8b`, `0x48ddc0`, `0x48ddeb`, `0x48de20`, `0x48de4e`, `0x48de59`, `0x48de63`, `0x48de70`, `0x48d2e0`, `0x48d2f2`, `0x48d300`, `0x48d327`, `0xb4e3c0`, `0xb4e3c4`, `0x5ee660`, `0x5ee740`, `0x495d60`, `0x6e5b40`
Confidence: verified
Proof: one
Test: none

`0x48dd82` loads the flags dword at `[player+0xe68]+8`; `0x48dd8b` shifts it right 10 and `0x48ddeb` right 11. `0x48de4e` reads the flag
with `GetScriptFlagArg(1, default 1)`. `0x48d2f2` calls `ValidateSpellItemTarget(1, guid)` with the GUID kept at
`0xb4e3c0` and `0xb4e3c4`, and `0x48d327` looks the item up with type mask 2 before `BindSpellTargetObject`.

## Item quality colours

`GetItemQualityColor(quality)` returns the red, green and blue of the quality as numbers from 0 to 1, and the colour
escape string. It raises `Usage: GetItemQualityColor(index)` for a non-number. The channels are the stored bytes times
1/255 as doubles, read from the third, second and first byte of the colour entry.

Addresses: `0x48dfb0`, `0x48dff1`, `0x48dff8`, `0x48e007`, `0x48e015`, `0x48e032`, `0x48e050`, `0x52ad70`, `0x52ad90`, `0x804578`
Confidence: verified
Proof: one
Test: `ui::luabind_48dfb0_quality_color`

`0x48dff1` calls `QualityColorPointer(quality)` and `0x48dff8`, `0x48e015` and `0x48e032` load the bytes at `+2`, `+1` and `+0`, each
converted with `fild` and `fmul` by the double 1/255 (`0x804578`) and pushed. `0x48e050` pushes the string from
`QualityColorEscape`, so the verb returns 4.

`ui::luabind_48dfb0_quality_color` runs the three channel normalisations of `GetItemQualityColor` (`fild`, `fmul` by 1/255, `fstp` at `0x48e004`, `0x48e021` and `0x48e03d`), each at its own address.

## Item information

`GetItemInfo(item)` takes an item id as a number, or the text `item:` followed by the id and up to three more colon-separated
numbers, the enchant, the suffix and the unique id. A text without the `item:` prefix reads as id 0. An item not in the
cache returns no values; a non-number, non-string argument raises `Usage: GetItemInfo(itemID|"itemlink")`. Otherwise it
returns nine values:

| # | value |
|---|---|
| 1 | the name, with the random suffix of the suffix id |
| 2 | the text `item:id:enchant:suffix:unique` |
| 3 | the quality, record `+0x1c` |
| 4 | the required level, record `+0x3c` |
| 5 | the class name in the current locale, or empty |
| 6 | the subclass name: the subclass row's second name when nonempty, else its first, or empty |
| 7 | the stack size, record `+0x60` |
| 8 | the inventory type key text, indexed by record `+0x2c` |
| 9 | the icon path: the icon directory, a backslash when it is nonempty, and the display's icon name |

Addresses: `0x48e070`, `0x48e0c5`, `0x48e0e8`, `0x48e0f5`, `0x48e11d`, `0x48e140`, `0x48e16a`, `0x48e185`, `0x48e1a7`, `0x48e1c9`, `0x48e1da`, `0x48e1ef`, `0x48e1ff`, `0x48e215`, `0x48e230`, `0x48e24e`, `0x48e261`, `0x48e311`, `0x48e29e`, `0x48e2b1`, `0x48e2cf`, `0x48e2dd`, `0x48e303`, `0x48e32d`, `0x48e33b`, `0xc0e2a0`, `0x5d8b00`, `0x55ba30`, `0x5d88b0`, `0x53f810`, `0xc0dc24`, `0xc0db90`, `0x83ddb0`
Confidence: verified
Proof: one
Test: none

`0x48e0e8` compares the first five characters with `item:` without regard to case, and each following number is read with
`atoi` after the next colon (`0x48e11d` the enchant, `0x48e140` the suffix, `0x48e16a` the unique id). The record comes from the
item cache `0xc0e2a0` (`0x48e185`); a miss returns 0 values (`0x48e33b`). `0x48e1a7` writes the name with the suffix;
`0x48e1da` formats the link text. The class name is the locale string at `+0xc` of the item class row (`0x48e230`); the subclass rows
are 0x74 bytes, matched on class and subclass (`0x48e261`), and `0x48e311` prefers `+0x4c` over `+0x28`. The inventory type key comes from
the pointer table at `0x83ddb0`, and `0x48e2dd` formats the icon path as `%s%s%s`. The verb returns 9 (`0x48e303`).
