# ui: composing and sending mail

The compose half of `MailInfo.cpp`: the attachment, the money and COD amounts, the postage, the send request, and the
stationery and package lists that feed the compose window. The mailbox session and the inbox are `ui/mail`; the item
lock the attachment takes is `ui/cursor#the-item-lock`.

Roots: `0x4ac980`, `0x4acdc0`, `0x4ad970`, `0x4adef0`, `0x4ae0f0`, `0x4ae230`, `0x4ae450`, `0x4ae590`, `0x4ae730`, `0x4ae800`

## The compose state

The draft is a few cells. The attached item is a GUID (`0xb6ef90`), the container GUID it came from (`0xb6ef98`) and
its slot byte (`0xb6efa0`). The amounts are money (`0xb6efa4`) and COD (`0xb6efa8`). `0xb6efac` is 1 while a send
awaits its result. The selections are a stationery id (`0xb6efb4`) and a package id (`0xb6efb8`). `ClearSendMail()` and
the compose reset `0x4acdc0` unlock the attached item, zero every cell and, when asked, signal SEND_MAIL_MONEY_CHANGED,
SEND_MAIL_COD_CHANGED and MAIL_SEND_SUCCESS in that order. Opening a mailbox, a successful send and `ClearSendMail`
reset with the events; closing the window resets without them. A reset clears the stationery selection, so a draft
cannot be sent until `SelectStationery` runs again.

Addresses: `0x4acdc0`, `0x4acdd6`, `0x4acdf3`, `0x4ace07`, `0x4ace14`, `0x4ace19`, `0x4ace23`, `0x4ace2d`, `0x4adee0`, `0x495420`, `0xb6ef90`, `0xb6ef94`, `0xb6ef98`, `0xb6ef9c`, `0xb6efa0`, `0xb6efa4`, `0xb6efa8`, `0xb6efac`, `0xb6efb4`, `0xb6efb8`
Confidence: verified
Proof: one
Test: none

`0x4acdc0` takes `ecx`: nonzero reaches the three signals (`0x4ace12 je` skips them). `0x4acdd6` calls the unlock for
the attached GUID when its two words are not both zero. `ClearSendMail` is `mov ecx,1; call 0x4acdc0`
(`0x4adee0`). The init `0x4acb10` zeroes the same cells.

## Attaching an item

`ClickSendMailItemButton()` swaps the cursor's item with the attachment. It reads the cursor payload without testing
its kind. An item the object manager knows is refused with `ERR_MAIL_BOUND_ITEM` when it is bound (`ERR_MAIL_QUEST_ITEM`
when its template's bonding is 4 or 5), `ERR_MAIL_CONJURED_ITEM` for the conjured flag or either of two record flags, and
`ERR_MAIL_BAG` for a bag that holds anything. An unknown item passes. If the cursor item is already the attachment the
cursor is cleared; otherwise the new GUID, container and slot are stored, the old attachment goes onto the cursor, and
MAIL_SEND_INFO_UPDATE fires. An empty cursor therefore picks the attachment back up.

Addresses: `0x4adef0`, `0x4adf15`, `0x4adf31`, `0x4adf42`, `0x4adf7f`, `0x4adf97`, `0x4adfbb`, `0x4adfdb`, `0x4adfe6`, `0x4ae015`, `0x4ae019`, `0x4ae05a`, `0x4ae08b`, `0x4ae0d4`, `0x4ae0de`, `0x494c80`, `0x5da2c0`, `0x5da500`, `0x5da570`, `0x494b60`, `0x495190`, `0x496720`
Confidence: verified
Proof: one
Test: none

The errors are `0x160` quest, `0x161` bound, `0x162` conjured and `0x163` bag (`0x4adf7f`, `0x4adf97`, `0x4adfc1`,
`0x4ae042`). The test for the bag type is bit 2 of the descriptor type word (`0x4ae00a` to `0x4ae013`); the contents are
the non-zero GUIDs of the container's slot list (`0x4ae019` to `0x4ae03c`). `0x4ae08b` calls the clear with both
arguments 1; `0x4ae0d4` puts the previous attachment on the cursor.

## Money and COD

`SetSendMailMoney(amount)` stores the amount when the active player's coinage covers it, signals
SEND_MAIL_MONEY_CHANGED and returns 1; otherwise it shows `ERR_NOT_ENOUGH_MONEY` and returns nil. A negative amount
fails the unsigned test. `SetSendMailCOD(amount)` stores the amount only while an item is attached, signals
SEND_MAIL_COD_CHANGED and returns nothing. It tests no coinage and does not clear the money. `GetSendMailMoney()` and
`GetSendMailCOD()` return the cells. A non-number raises `Usage: SetSendMailMoney(amount)` or `Usage:
SetSendMailCOD(amount)`.

Addresses: `0x4ae0f0`, `0x4ae150`, `0x4ae180`, `0x4ae1c0`, `0x4adbe0`, `0x4adc24`, `0x4adc39`, `0x4adc44`, `0x4adc53`, `0x4adc70`, `0x4adc7d`, `0x4adc88`
Confidence: verified
Proof: one
Test: none

The coinage read is `[[player+0xe68]+0xf70]` (`0x4adc24`) for the player whose descriptor GUID equals the active
player's. `0x4adc33 cmp eax,ecx; jb` is unsigned, the failure `0x4adc53 push 0x25`. The Lua wrapper converts with
`__ftol` (`0x4ae11f`) and pushes 1.0 or nil (`0x4ae131`, `0x4ae144`).

## Sending

`SendMail(target, subject, body)` sends opcode 0x238 and returns nothing. A target that is not a string raises
`Usage: SendMail(target, subject, body)`; a subject or body that is not a string counts as empty. Every other refusal
is silent: a request pending or in flight, no mailbox, an empty target or subject, no stationery selected, money and COD
both set, or COD set with no item. An attachment that is gone or not the player's shows `ERR_ITEM_NOT_FOUND`, clears
and fires MAIL_SEND_INFO_UPDATE. The packet is the mailbox GUID (u64), target, subject, body, stationery id (u32),
package id (u32, 0 without an item), item GUID (u64), money (u32), COD (u32), a zero u64 and a zero u8. Both flags are
then set.

Addresses: `0x4ae800`, `0x4ae810`, `0x4ae830`, `0x4ae852`, `0x4ae858`, `0x4ae870`, `0x4ae8a9`, `0x4ae8dd`, `0x4ae91d`, `0x4ae957`, `0x4ae9a1`, `0x4ae9be`, `0x4aea01`, `0x4aea36`, `0x4aea58`, `0x4aea61`, `0x4aea6c`, `0x4aea76`, `0x4aea7b`, `0x468460`, `0x468550`, `0x622630`
Confidence: verified
Proof: one
Test: none

The guards are in this order: `lua_isstring(1)` (`0x4ae810`), the pending flag (`0x4ae830`), the GUID (`0x4ae84d`),
the in-flight flag (`0x4ae858`), the target's first byte (`0x4ae870`), the subject's (`0x4ae8a9`), the stationery
(`0x4ae8dd`). The item lookup `0x4ae91d` asks for an item object by GUID and compares the first descriptor GUID with
the active player (`0x4ae94d`, `0x4ae952`; the item fields start at the owner, `objectlayer/items`); the failure text is the inventory result 0x17 (`0x4ae957`). The money and
COD tests are `0x4ae99e` to `0x4ae9b8`. The writers are `0x418190` (u32), `0x418370` (u64), `0x418430` (string) and
`0x418070` (u8); the send is `0x5ab630` (`0x4aea6c`). The result clears both flags (`ui/mail#mail-results`).

## Postage and the attachment's details

`GetSendMailPrice()` returns the money plus the postage: 30 copper, plus the buy price of the selected stationery's item
when the player does not carry one and the item is cached, plus the cost of the selected package. `GetSendMailItem()`
returns the attachment's name, icon path, count and quality; with no attachment, a missing object or an uncached item
it returns nil, nil, 0, 0. Quality is the template `+0x1c` when its `+0x2c` is nonzero, else -1, and the count is the
item's stack count field. An item that arrives late raises MAIL_SEND_INFO_UPDATE.

Addresses: `0x4ae730`, `0x4ae756`, `0x4ae75d`, `0x4ae77d`, `0x4ae7a0`, `0x4ae7a6`, `0x4ae7c6`, `0x4ae7d2`, `0x4ae590`, `0x4ae5c5`, `0x4ae5f5`, `0x4ae622`, `0x4ae68e`, `0x4ae6a7`, `0x4ae6bf`, `0x4ae6d1`, `0x4ae6ec`, `0x4ae710`, `0x5d8bc0`, `0x5d88b0`, `0xc0d710`, `0xc0da30`, `0xc0da34`
Confidence: verified
Proof: one
Test: none

`0x4ae756 mov esi,0x1e` is the base. The stationery is found in the usable list by its database id (`0x4ae765` to
`0x4ae774`); a nonzero owned GUID (`0x4ae77a`) skips the price; the price is the record `+0x24` (`0x4ae7a0`). The package
row comes from the id map of the Package store (`0x4ae7b1` to `0x4ae7c6`) and adds its cost `[row+8]`. The count read is
`[[obj+0x114]+0x20]` (`0x4ae68e`). The callback `0x4ae710` signals MAIL_SEND_INFO_UPDATE.

## The stationery list

The game init builds one entry per row of Stationery.dbc (`ID`, `ItemID`, `Texture`, `Flags`): the id, the item id, an
always-available bit (flag bit 0) and a carried-item GUID. The refresh `0x4ad970` finds the carried item for each (by
entry, in the player's items), counts as usable those that are available or carried and whose item is cached, and sorts
the whole list. The order is: available or carried first; carried before merely available; cached before uncached;
lower buy price first; then the item name, case-insensitive. The refresh runs after every requested stationery item has
arrived and on `GetNumStationeries()` when none is awaited; that function returns the usable count.

Addresses: `0x4ac980`, `0x4ac9d2`, `0x4aca2a`, `0x4acb10`, `0x4acba8`, `0x4acbc3`, `0x4acbd7`, `0x4acbf0`, `0x4ad970`, `0x4ad9fc`, `0x4ada5a`, `0x4ada7d`, `0x4ada90`, `0x4adaed`, `0x4adb7b`, `0x4adba6`, `0x4ae1f0`, `0x622270`, `0x73f727`, `0xb6ee28`, `0xb6ee2c`, `0xb6ee30`, `0xb6efb0`, `0xb6efbc`, `0xc0d708`, `0xc0d70c`
Confidence: verified
Proof: one
Test: none

An entry is 0x18 bytes: the id at +0, the item id at +4, the always bit at +8 (`0x4aca2d`) and the carried item's GUID
at +0x10. `0x4ac980` is called once, from the game init at `0x48f5ef`. The counter `0xb6efbc` counts items still being
fetched: `0x4acbc3` raises it per miss, `0x4acbf0` lowers it per arrival and refreshes at zero. The comparator
`0x4ada90` returns -1 or 1 by the first differing key; the list is sorted by `0x73f727` with a four-byte element.
The carried test `0x4ad9fc` passes the player's item list (`player+0x1d38`) to `0x622270`.

## Stationery lookups

`GetStationeryInfo(index)` returns three values for a usable entry: the item name, the icon path and the buy price,
nil in place of the price when the player carries the item. An index outside the usable list, or an uncached item,
gives three nils; a non-number raises `Usage: GetStationeryInfo(index)`. `SelectStationery(index)` stores the entry's id as
the selection, or 0 for an index outside the list, and returns nothing. `GetSelectedStationeryTexture()` returns the
selection's bare texture name from the store, nil for 0, a negative or an unknown id.

Addresses: `0x4ae230`, `0x4ae28c`, `0x4ae2bd`, `0x4ae310`, `0x4ae331`, `0x4ae33e`, `0x4ae35a`, `0x4ae380`, `0x4ae3bb`, `0x4ae3ca`, `0x4ae3d3`, `0x4ae3f0`, `0x4ae3f9`, `0x4ae401`, `0x4ae410`, `0x4ae41e`, `0x53f810`, `0x5d88b0`, `0xb6efb0`, `0xb6efb4`, `0xc0d710`, `0xc0d714`
Confidence: verified
Proof: one
Test: none

The icon path is `%s%s%s` of the icon prefix of `0x53f810`, a backslash when the prefix is not empty, and the display icon
(`0x4ae310`). The bounds `0x4ae28c cmp eax,ecx; jae` and `0x4ae3bb` are unsigned on index minus one. The price is the
item record `+0x24` (`0x4ae33e`), the field before the sell price `+0x28` that the item parser `0x7c9640` reads in wire
order. `0x4ae3f9 jl` and `0x4ae401 jg` bound the id.

## Packages

`GetNumPackages()` returns the row count of the Package store. `GetPackageInfo(index)` returns the localized name, the
icon path and the cost for a row, or nil, nil, 0 outside the list. `SelectPackage(index)` stores the row's id as the
package selection, or 0 for an index outside the list. The selection is sent only with an attached item. Non-numbers
raise `Usage: GetPackageInfo(index)` and `Usage: SelectPackage(index)`.

Addresses: `0x4ae430`, `0x4ae450`, `0x4ae487`, `0x4ae48d`, `0x4ae4d5`, `0x4ae4f6`, `0x4ae50d`, `0x4ae525`, `0x4ae550`, `0x4ae587`, `0x4adc90`, `0x4adc94`, `0x4adcb0`, `0x4adcb6`, `0x64a760`, `0xc0da28`, `0xc0da2c`, `0xc0da30`, `0xc0da34`
Confidence: verified
Proof: one
Test: none

The Package store (`0xc0da28`) has 48-byte records: the id at +0, an icon string at +4, a cost at +8 and the localized
names from +0xc, one per locale. `0x4ae4f6` indexes the name by the locale cell `0xc0e080`. The path is the icon prefix, a
backslash when needed, then the icon name appended with `0x64a760` (`0x4ae4eb`). `0x4adc90` bounds the index with
`test ecx,ecx; jl` and `cmp ecx,[0xc0da2c]; jge`.
