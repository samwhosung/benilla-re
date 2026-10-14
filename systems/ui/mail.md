# ui: the mailbox and the inbox

The mail session and the inbox half of `MailInfo.cpp` (`0x4ac980` to `0x4aff50`): opening and closing at a mailbox,
the list the server sends, the Lua functions that read it, the requests that change a mail, the replies, and the
pending-mail timer. Composing and sending is `ui/mailsend`; the mailbox object is `objectlayer/gameobjecttypes`; the
profanity filter and name caches are shared with chat and quests.

Roots: `0x4acb10`, `0x4acc00`, `0x4acc40`, `0x4acd10`, `0x4acd50`, `0x4ad050`, `0x4ad1b0`, `0x4ad5f0`, `0x4ad620`, `0x4ade60`, `0x4afee0`

## Mail events

Mail raises ten events. The table gives each id, the raiser and the arguments (none unless stated).

| id | event | raised by |
|---|---|---|
| 0x19a | MAIL_SHOW | opening a mailbox |
| 0x19b | MAIL_CLOSED | a soft close |
| 0x19c, 0x19d | SEND_MAIL_MONEY_CHANGED, SEND_MAIL_COD_CHANGED | the compose amounts; `ui/mailsend` |
| 0x19e | MAIL_SEND_INFO_UPDATE | the attachment changes; `ui/mailsend` |
| 0x19f | MAIL_SEND_SUCCESS | every compose reset that is not silent |
| 0x1a0 | MAIL_INBOX_UPDATE | a list, a changed or removed mail, a late name, text or item, a refused `CheckInbox` |
| 0x1f4 | UPDATE_PENDING_MAIL | the next-mail time changes |
| 0x202 | MAIL_FAILED | every send result, success included |
| 0x203 | CLOSE_INBOX_ITEM | an automatic delete; one argument, the 1-based inbox index |

Addresses: `0x4acd42`, `0x4acda3`, `0x4adc44`, `0x4adc88`, `0x4ace19`, `0x4ace23`, `0x4ace2d`, `0x4ae0de`, `0x4ae71f`, `0x4ae98d`, `0x4ad5d6`, `0x4aeaf8`, `0x4ad605`, `0x4ad66b`, `0x4adeba`, `0x4ad16a`, `0x4ad772`, `0x4ad87d`, `0x703e50`, `0x703f50`
Confidence: verified
Proof: one
Test: none

The names are stored by static initializers (`0x51b5b3` to `0x51b5ef` for 0x19a to 0x1a0, `0x51b937`, `0x51b9c3`,
`0x51b9cd`). Each raiser is `push id; call 0x703e50` or a tail `jmp` to it; the one event with an argument,
CLOSE_INBOX_ITEM, goes through `0x703f50` with the format `%d` and the index plus one (`0x4ad76d` to `0x4ad772`).
MAIL_FAILED fires from `0x4ad16a` after the error or success text, so a listener cannot tell the two apart by the
event alone.

## Mail packets

Every request carries the mailbox GUID (u64, not packed) first. The client sends and reads these.

| opcode | direction | what | body after the GUID |
|---|---|---|---|
| 0x23a | send | `CheckInbox` | none |
| 0x245 | send | `TakeInboxMoney` | mail id u32 |
| 0x246 | send | `TakeInboxItem` | mail id u32 |
| 0x247 | send | mark read | mail id u32 |
| 0x248 | send | `ReturnInboxItem` | mail id u32 |
| 0x249 | send | `DeleteInboxItem`, automatic delete | mail id u32 |
| 0x24a | send | `TakeInboxTextItem` | mail id u32, template id u32 |
| 0x284 | both | next-mail time | send: none; reply: f32 |
| 0x239 | receive | send result | `ui/mail#mail-results` |
| 0x23b | receive | the list | `ui/mail#the-list-packet` |
| 0x285 | receive | a mail arrived | f32 |

Addresses: `0x4aeb0b`, `0x4af85c`, `0x4af9d2`, `0x4adcfe`, `0x4afc4d`, `0x4afd79`, `0x4ad716`, `0x4afb08`, `0x4adde9`, `0x4acad0`, `0x4acae1`, `0x4acaf2`, `0x4acb03`, `0x5ab630`, `0x5ab650`
Confidence: verified
Proof: one
Test: none

The game init `0x4ac980` registers the handlers: `0x4ad050` for 0x239, `0x4ad1b0` for 0x23b, `0x4ad5f0` for 0x284
and `0x4ad620` for 0x285 (`0x4acad0` to `0x4acb03`); `0x4acc40` clears them (`0x4acce6` to `0x4acd07`). Each sender builds the opcode with
`0x418190`, the GUID with `0x418370` and the id with `0x418190`, then `0x5ab630` sends. The mail id is the first field
of the stored record (`[record+0]`).

## Opening and closing

A mailbox's use handler (`0x5f6820`) and the lottery kiosk's (`0x5f6e10`) pass the object's GUID to `0x4acd10` and send
nothing. It makes the mailbox the interaction object with the walk-away range, stores the GUID, clears the compose
state with the events, and signals MAIL_SHOW. `CloseMail()` and the interaction drop-all call `0x4acd50`: it clears
the compose state silently, ends the interaction, signals MAIL_CLOSED, zeroes the GUID and, when a mail was marked read
since the last query, asks for the next-mail time again. The hard form, used by the world teardown, zeroes the GUID and
the list count and signals nothing. The range is the square of 5.5555553 yards.

Addresses: `0x4acd10`, `0x4acd1c`, `0x4acd27`, `0x4acd38`, `0x4acd50`, `0x4acd61`, `0x4acd85`, `0x4acdb1`, `0x4aded0`, `0x4934bd`, `0x4acc00`, `0x4acc04`, `0x5f6820`, `0x5f6e10`, `0x4930d0`, `0x4931c0`, `0xb6ee1c`, `0x4b0050`, `0xb6ef88`, `0xb6ef8c`
Confidence: verified
Proof: one
Test: none

`0x4acd13` pushes the range cell for `0x4930d0`, which `0x4b0050` fills with the constant at `0x8065ec` squared
(`fld; fmul` of the same operand). `0x4acd50` takes `cl`: nonzero reaches `0x4acd5f` and clears `0xb6ef88`, `0xb6ef8c`
and `0xb6efc0`; zero returns at `0x4acd7c` when the GUID is 0, else calls `0x4931c0`, signals `0x19b` and zeroes the GUID.
`CloseMail` is `xor cl,cl; call 0x4acd50`. The drop-all `0x493310` calls it with `cl` 0 when the dropped GUID equals
the mail GUID (`0x4934a7` to `0x4934bd`). The teardown `0x4acc00` calls it with `cl` 1.

## The list packet

`SMSG_MAIL_LIST_RESULT` (0x23b, `0x4ad1b0`) carries a u8 mail count and that many mails. The array of mail records
grows to the count (never shrinks) and each record is 0x260 bytes. The reader is the stream reader of `0x418cb0` to
`0x4191b0`; the table gives the wire order with the record offset each field lands in.

| offset | wire field |
|---|---|
| +0x00 | mail id u32 |
| +0x04 | type u8: 0 player, 2 auction, 3 creature, 4 game object |
| +0x08 or +0x10 | type 0: sender GUID u64 at +8 and 0 at +0x10; types 1 to 4: sender id u32 at +0x10 and 0 at +8; above 4: none |
| +0x14 | subject, a string copied with no bound into the 0x100 bytes to +0x113 |
| +0x114, +0x118, +0x11c | item text id, package id, stationery id (u32 each) |
| +0x120, +0x124, +0x128, +0x12c | item entry, enchantment id, random property id, suffix factor |
| +0x130 | item count u8 |
| +0x134, +0x138, +0x13c | charges, then two more u32, unread |
| +0x140, +0x144 | money, COD amount |
| +0x148 | flags: 1 read, 2 returned, 4 text turned into an item, 8 COD |
| +0x14c | days left, f32 |
| +0x25c | mail template id u32 |

Addresses: `0x4ad1b0`, `0x4ad1c7`, `0x4ad1eb`, `0x4ad210`, `0x4ad23e`, `0x4ad24c`, `0x4ad27c`, `0x4ad2a1`, `0x4ad2c5`, `0x4ad2dc`, `0x4ad378`, `0x4ad3a5`, `0x4ad3d1`, `0x4ad3fe`, `0x4ad414`, `0x4ad42b`, `0x4ad5c3`, `0x4ad5d0`, `0x4b03c0`, `0x418cb0`, `0x418e30`, `0x418eb0`, `0x4190b0`, `0x4191b0`, `0x419130`, `0xb6ef4c`, `0xb6ef50`, `0xb6ef54`, `0xb6efc0`, `0xb6efc8`
Confidence: verified
Proof: one
Test: none

The loop body starts at `0x4ad233` and stores through the record pointer `[0xb6ef54+i*4]`. `0x4ad26a cmp al,4; ja`
bounds the one-word sender read; `0x4ad292` is the player leg with the u64 reader `0x4190b0`. The subject read
`0x4ad2c5` passes the limit `0x7fffffff`, and the `+0x114` fields are read after it (`0x4ad2dc` on). So the numeric fields are
correct and the damage falls on the subject: past 255 bytes it loses its terminator and runs into those fields, and
one longer than 0x24b bytes writes past the 0x260-byte record (`0x4ad210` allocates it).
`0x4ad5c3` stores the count (reduced by ignored senders, below) into `0xb6efc0`, clears the pending flag `0xb6efc8`
and signals MAIL_INBOX_UPDATE.

## Ignored senders and defaults

After a record is read, the loop rewrites it. A type 0 mail from an ignored player (`0x5ae5a0`) whose flags lack
returned and COD is answered at once: opcode 0x248 when it holds an item or money, else 0x249, with the mailbox GUID
and the mail id. The record is then dropped, the next mail reusing its slot and the count falling by one. Every other
record whose stationery id is not positive gets the first stationery of the sorted list, or crashes when that list is
empty. A type 2 record has its subject parsed (`ui/mail#auction-mail-subjects`). The subject is then run through the
profanity filter.

Addresses: `0x4ad463`, `0x4ad47f`, `0x4ad496`, `0x4ad4cf`, `0x4ad4d6`, `0x4ad512`, `0x4ad51d`, `0x4ad525`, `0x4ad551`, `0x4ad565`, `0x4ad56d`, `0x4ad57a`, `0x4ad5a0`, `0x4ad5b2`, `0x5ae5a0`, `0x4ace70`, `0x4a1a60`, `0x64a5a0`
Confidence: verified
Proof: one
Test: none

`0x4ad438` and the two stores after it zero the invoice entry and random property (`+0x250`, `+0x254`) and
`0x4ad459` sets the result (`+0x258`) to -1.
`0x4ad47f` asks the friend list `[0xc28168]` whether the GUID is ignored; `0x4ad496 test [ecx+0x148],0xa` skips the
return. `0x4ad4bf cmp [eax+0x120],0` and `0x4ad4c7 cmp [eax+0x140],0` choose `push 0x249` or `push 0x248`;
`0x4ad51d dec esi` and `0x4ad51e dec ecx` drop the slot. The default test `0x4ad551 cmp [ecx+0x11c],0; jg` reads
`[[0xb6ee30]]` at `0x4ad565` to `0x4ad56d` (a null record when `0xb6efb0` is 0). `0x4ad592` copies the subject to
`+0x150` for the parser; `0x4ad5b2` calls the filter with `dl` 0.

## Mail results

`SMSG_SEND_MAIL_RESULT` (0x239, `0x4ad050`) is three u32: the mail id, the action and the error; an error of 1 adds
a u32 inventory result. Only error 0 runs the action; every other error shows its text. Errors map to texts:
1 the inventory result's error, 2 `ERR_MAIL_TO_SELF`, 3 `ERR_NOT_ENOUGH_MONEY`, 4 `ERR_MAIL_TARGET_NOT_FOUND`,
5 `ERR_PLAYER_WRONG_FACTION`, 14 `ERR_RESTRICTED_ACCOUNT`, 15 `ERR_MAIL_REACHED_CAP`, and 6 to 13 and above 15
`ERR_MAIL_DATABASE_ERROR`. Action 0 shows `ERR_MAIL_SENT` and resets the compose state with events; 1 and 2 update
the stored mail (below); 3 and 4 remove it; 5 sets the text-created flag. After any result the pending flag is
cleared, an action 0 also clears the send-in-flight flag, and MAIL_FAILED fires.

Addresses: `0x4ad050`, `0x4ad067`, `0x4ad07d`, `0x4ad0aa`, `0x4ad0b4`, `0x4ad0b8`, `0x4ad0ca`, `0x4ad0dc`, `0x4ad0eb`, `0x4ad0fa`, `0x4ad10e`, `0x4ad12e`, `0x4ad154`, `0x4ad159`, `0x4ad164`, `0x4ad6b0`, `0x4ad7b0`, `0x4ad8c0`, `0x4ad910`, `0x622630`, `0x496720`, `0xb6efac`
Confidence: verified
Proof: one
Test: none

`0x4ad060` preloads the inventory result with 0x28. The dispatch is a byte map and a jump table that follow the function: `movzx` and `jmp [ecx*4+table]` at `0x4ad0a3`
to `0x4ad0aa`, bounded by `0x4ad09a cmp eax,0xf`; the pushes are `0x164`, `0x25`, `0x165`, `0xff`, `0x1be`,
`0x1c4` and the default `0x166` (`0x4ad115` to `0x4ad147`). The action tests run in the order 0, 1, 2, 5, then 3 and 4
(`0x4ad0b4` to `0x4ad109`); any other action does nothing. The text ids are the keys in the stored names.

## Mail changes after a result

A taken-money result zeroes the money. A taken-item result zeroes the item entry. Each then deletes the mail on its
own (opcode 0x249, then CLOSE_INBOX_ITEM with the index) when nothing more can be taken: money taken deletes a COD
mail, or an auction mail with no item; item taken deletes an auction mail, or a mail with no text and no template,
that has no money left. MAIL_INBOX_UPDATE follows either way. A removed mail shifts the later records down and lowers
the count; the made-permanent result only sets flag 4.

Addresses: `0x4ad6b0`, `0x4ad6e8`, `0x4ad6f7`, `0x4ad700`, `0x4ad70a`, `0x4ad716`, `0x4ad79e`, `0x4ad7b0`, `0x4ad7e8`, `0x4ad7f7`, `0x4ad7fd`, `0x4ad809`, `0x4ad815`, `0x4ad821`, `0x4ad8c0`, `0x4ad8ed`, `0x4ad910`, `0x4ad933`, `0x4ad952`, `0x4ad961`
Confidence: verified
Proof: one
Test: none

Both look the mail up by id in the array (`0x4ad6d5`, `0x4ad7d5`); a missing id returns. `0x4ad6f7 test [eax+0x148],8`
and `0x4ad700 cmp [eax+0x4],2` with `0x4ad70a cmp [eax+0x120],0` choose the delete for money; `0x4ad7f7 cmp [eax+0x4],2`,
`0x4ad7fd`, `0x4ad809` and `0x4ad815 cmp [eax+0x140],0` choose it for the item. The removal copies 0x98 dwords per
record (`0x4ad94d mov ecx,0x98; rep movs`).

## Reading the header

`GetInboxNumItems()` returns the record count. `GetInboxHeaderInfo(index)` returns 13 values: package icon,
stationery icon, sender, subject, money, COD, days left, has item, read, returned, text created, can reply, GM. The
first icon is the package's when its id resolves, else the item's when cached, else nil; the second is the stationery
item's when cached. The sender is the name by type: player, auction house, creature or game object, nil for type 1. On
a type 0 mail, stationery 61 makes the sender `GM_EMAIL_NAME`, can-reply nil and GM 1. A COD mail's subject is the
`COD_PAYMENT` text formatted with it. A bad index returns four nils, three zeros and six nils.

Addresses: `0x4aeb90`, `0x4aebc0`, `0x4aebdc`, `0x4aec07`, `0x4aec76`, `0x4aeca6`, `0x4aed25`, `0x4aed99`, `0x4aedb6`, `0x4aeeb0`, `0x4aeef3`, `0x4aef2c`, `0x4aef4b`, `0x4aef69`, `0x4aef87`, `0x4aefa5`, `0x4aeff0`, `0x4af080`, `0x4af0a0`, `0x55f080`, `0x556aa0`, `0x558560`, `0x5d88b0`, `0x53f810`, `0xb6ee48`, `0xc0dff8`, `0xc0da30`, `0xc0d710`
Confidence: verified
Proof: one
Test: none

`0x4aec07 mov edx,0xd; call 0x6f2f30` reserves 13 slots; `0x4aeffe` to `0x4af05e` is the miss shape. The sender switch is
the jump at `0x4aedb6` over types 0 to 4: the player leg asks the name cache `0xc0e228` with callback `0x4af080`, the
auction leg reads the house name from the AuctionHouse store `0xc0dff8`, the creature leg the creature cache `0xc0e354`
and the game object leg `0xc0e318` (`0x4aee48`, `0x4aee78`). A type above 4 pushes 12 values and returns 13, so the
first result is the index. A late name or item raises MAIL_INBOX_UPDATE through `0x4af080` and `0x4af0a0`. A non-number
raises `Usage: GetInboxHeaderInfo(index)`. The icons use `0x5d88b0` on the item's display id and the icon prefix of `0x53f810`.

## Reading a mail's text

`GetInboxText(index)` returns four values: the body, the stationery texture, whether the text can become an item, and
whether the mail is an invoice. It first marks the mail read. The body comes from the item-text cache by the item text
id: nil until it arrives, and nil for an auction mail. A mail with a template id and no text id takes the localized
template body. Both are expanded for `$` tokens against the player. The text-item flag is 1 when a text or template
exists, flag 4 is clear and the stationery is not 61. A bad index returns four nils.

Addresses: `0x4af110`, `0x4af18e`, `0x4af19b`, `0x4af1bf`, `0x4af1cf`, `0x4af1f6`, `0x4af203`, `0x4af217`, `0x4af223`, `0x4af266`, `0x4af27c`, `0x4af2c2`, `0x4af2cb`, `0x4af2eb`, `0x4af311`, `0x4af340`, `0x569980`, `0x506f70`, `0x4a1a60`, `0xc0e0c0`, `0xc0dab4`, `0xc0dabc`, `0xc0dac0`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

A non-number raises `Usage: GetInboxText(index)`. The item-text leg `0x4af19b` calls the cache `0xc0e0c0` (`0x569980`) with the callback `0x4af340`, which raises
MAIL_INBOX_UPDATE when the text lands. `0x4af1cf cmp [esi+0x4],2` sends an auction mail to the nil leg. The expander
`0x506f70` and the filter `0x4a1a60` run only on the cached text; the template leg (`0x4af217`) takes the string
`[row+locale*4+4]` of the 10-field MailTemplate store and expands it without the filter. The template store is
`0xc0dab4` (record size 0x28).

## Reading a mail's item

`GetInboxItem(index)` returns five values: the item name with its random-property suffix, the icon path, the count,
the quality and whether the player can use it. A mail with no item, an uncached item or a bad index returns nil, nil, 0, 0, nil. Quality is the template `+0x1c` when its `+0x2c` is nonzero, else -1. The use flag is 1 with no player
object and nil when `0x5ea930` rejects the template. A non-number raises `Usage: GetInboxItem(index)`.

Addresses: `0x4af5d0`, `0x4af64a`, `0x4af658`, `0x4af698`, `0x4af6d0`, `0x4af704`, `0x4af721`, `0x4af76d`, `0x4af776`, `0x4af792`, `0x4af7b6`, `0x5d8b00`, `0x5ea930`
Confidence: verified
Proof: one
Test: none

The name call `0x4af698` passes the entry (`+0x120`) and the random property (`+0x128`); the count is the byte at
`+0x130` (`0x4af704 movzx`). The icon path is `%s%s%s` of the icon prefix, a backslash and the display icon
(`0x4af6de`). `0x4af792` to `0x4af7b6` is the miss shape.

## Invoices

`GetInboxInvoiceInfo(index)` returns seven values for an auction mail whose text is cached: `seller` or `buyer`, the
item name, the other party's name, and four integers. A seller invoice has all four (bid, buyout, deposit, consignment);
a buyer invoice has the first two and zeros. Anything else returns `nil, nil, nil, 0, 0, 0, 0`: a bad index, a mail with
no text id or no invoice item, an uncached text, or a party name still being fetched. The text is `guid:n:n[:n:n]`.
A non-number raises `Usage: GetInboxInvoiceInfo(index)`.

Addresses: `0x4af360`, `0x4af3da`, `0x4af3e8`, `0x4af407`, `0x4af430`, `0x4af435`, `0x4af471`, `0x4af495`, `0x4af4b7`, `0x4af4c7`, `0x4af4d0`, `0x4af4dc`, `0x4af559`, `0x4af5b0`, `0x73fb5d`, `0x5d8b00`, `0x55f080`
Confidence: verified
Proof: one
Test: none

`0x4af3e8 cmp [edi+0x250],0` requires the invoice item. The parse picks by the result code at `+0x258`: 2 uses the format
`%16I64X:%d:%d:%d:%d` and 1 uses `%16I64X:%d:%d` (`0x4af471`, `0x4af495`); any other code leaves the values at zero and
the name lookup uses guid 0. The name cache call `0x4af4b7` has the callback `0x4af5b0`. Code 2 pushes `seller`
(`0x4af4d0`), the others `buyer` (`0x4af4d7`). The integers are pushed in text order.

## Auction mail subjects

For a type 2 mail the subject on the wire is `entry:randomProperty:result`. `0x4ace70` splits it, resolves the item
name with its suffix through the item cache and rewrites the subject to the global string of the result code:
`AUCTION_OUTBID_MAIL_SUBJECT` (0), `_WON_` (1), `_SOLD_` (2), `_EXPIRED_` (3), `_REMOVED_` (4 and 5), formatted with the
item name. An uncached item leaves the subject empty until the callback `0x4ad010` redoes it. The entry, random
property and result are kept at `+0x250`, `+0x254` and `+0x258` only for results 1 and 2; they mark an invoice.

Addresses: `0x4ace70`, `0x4ace97`, `0x4acea3`, `0x4acefd`, `0x4acf38`, `0x4acf5d`, `0x4acf67`, `0x4acf94`, `0x4acfb3`, `0x4acfd1`, `0x4ad010`, `0x4ad020`, `0x4ace40`, `0x64ac60`, `0x64a3f0`, `0x5d8b00`, `0x703bf0`, `0x64a7f0`
Confidence: verified
Proof: one
Test: none

The copy of the subject is at `+0x150`; the splits are at the colons (`0x4acea3`, `0x4acec5`) and the numbers parse with
`0x64ac60`. The switch `0x4acf67 jmp [edi*4+...]` covers results 0 to 5 with the global names `AUCTION_OUTBID`, `WON`,
`SOLD`, `EXPIRED`, `REMOVED` and `REMOVED` again. `0x4acf94` reads the global; `0x4acfb3` formats it with the item name
into the record's subject (`+0x14`, limit 0x100). `0x4acfc1` to `0x4acfdd` store the triple for results 1 and 2.
On a miss `0x4acf06` allocates a 4-byte holder with the mail id and registers `0x4ad010` as the callback, which finds the
record by id, parses it again (`0x4ace40`) and signals MAIL_INBOX_UPDATE.

## Taking, returning and deleting

Five functions send one request each and return nothing. They share three guards: no request is pending (`0xb6efc8`),
a mailbox is open and the index names a stored mail; the pending flag is then set until the result arrives. Then
`TakeInboxMoney` needs money in the mail; `TakeInboxItem` needs an item and, for COD, coinage of at least the COD
amount (else `ERR_NOT_ENOUGH_MONEY`); `TakeInboxTextItem` needs a text id or template and flag 4 clear;
`ReturnInboxItem` needs a type 0 mail with an item or money and neither flag 2 nor 8; `DeleteInboxItem` needs a
returned or COD flag, a type other than 0, or no item and no money. `InboxItemCanDelete(index)` applies the same rule.

Addresses: `0x4af7d0`, `0x4af8e0`, `0x4afa60`, `0x4afba0`, `0x4afcd0`, `0x4afe00`, `0x4af800`, `0x4af854`, `0x4af8aa`, `0x4af9c5`, `0x4afa4c`, `0x4afae7`, `0x4afb02`, `0x4afc27`, `0x4afc44`, `0x4afd57`, `0x4afd79`, `0x4afe4c`, `0x4afe70`, `0x4afe85`, `0x496720`
Confidence: verified
Proof: one
Test: none

Every function raises `Usage: <name>(index)` for a non-number. `InboxItemCanDelete` returns 1 or nil, nil for a bad
index. The coinage read is `[[player+0xe68]+0xf70]` for the active player (`0x4af9b7`); an index miss, a missing mail or any
failed guard returns without a message except the money one at `0x4afa4c push 0x25`. `0x4afe70` pushes 1.0 and
`0x4afe85` nil. The pending flag is set at `0x4af8aa`, `0x4afa1f`, `0x4afb68`, `0x4afc9e` and `0x4afdca`.

## Opening the inbox

`CheckInbox()` asks for the list: it returns at once while a request is pending or no mailbox is open; within 60
seconds of the last request it only signals MAIL_INBOX_UPDATE; otherwise it sends opcode 0x23a, sets the pending flag and
stamps the next allowed time 60000 ms ahead. A mail is marked read by the first `GetInboxText`: opcode 0x247 is sent
once per mail, flag 1 is set at once and days left above 3.0 fall to 3.0 for a mail that is not COD.

Addresses: `0x4aeab0`, `0x4aeab6`, `0x4aead8`, `0x4aeade`, `0x4aeae3`, `0x4aeaf1`, `0x4aeb06`, `0x4aeb13`, `0x4aeb18`, `0x4adcd0`, `0x4adcee`, `0x4add62`, `0x4add72`, `0x4add81`, `0x4add8e`, `0x4add9d`, `0xb6efc4`, `0xb6efc8`, `0xb6efcc`, `0x42c010`
Confidence: verified
Proof: one
Test: none

`0x4aeae3` compares the stored deadline with `0x42c010`; `0x4aeaf1 jns` passes when the deadline is not in the future.
`0x4aeb06 add eax,0xea60` sets the deadline. The marker `0x4adda6` sets `0xb6efcc` to 1, which `0x4acda8` reads at
close. The days-left clamp is `fcomp [0x80297c]` (3.0) at `0x4add81` and `mov [esi+0x14c],0x40400000` at `0x4add8e`.

## Pending mail

The float `0x845eac` is the time until the next mail the server will deliver: -1.0 means unknown, 0 means mail is
waiting. `HasNewMail()` returns 1 when its absolute value is below 2^-22 and nil otherwise. Init sets -1.0 and asks
the server (opcode 0x284); the answer overwrites it and raises UPDATE_PENDING_MAIL. A received-mail packet (0x285, one
f32) sets the flag only to remember a re-query when the mailbox is open; closed, a value near zero is stored and
signalled, a larger one is stored only when the old value is unknown or larger. A per-frame step lowers a positive value
by the frame time to a floor of 0 and signals once on reaching it.

Addresses: `0x4acb10`, `0x4acb87`, `0x4acbdc`, `0x4adde0`, `0x4adde9`, `0x4ade1f`, `0x4ade25`, `0x4ad5f0`, `0x4ad5f6`, `0x4ad620`, `0x4ad642`, `0x4ad65d`, `0x4ad679`, `0x4ad69f`, `0x4ade60`, `0x4ade8c`, `0x4ade96`, `0x4afea0`, `0x4818d0`, `0x4909f6`, `0x845eac`, `0x8029d4`
Confidence: verified
Proof: one
Test: `ui::glue_4a8_4ad620_set_value`, `ui::glue_4a8_4ade60_step_value`, `ui::frameapi_fpleaf_fabs_cmp`

The init `0x4acb10` runs once per world entry from `0x4909f6`; it requests every stationery item from the cache and
then queries the mail time (`0x4acbdc`). The step `0x4ade60` is called with the frame time from `0x4818d0`; its guard
`0x4ade69` to `0x4ade74` rejects values that are not above 0, so it fires exactly once. The epsilon at `0x8029d4` is
`0x34800000`. Nothing on the inbox path clears the value: taking mail does not.

Bit-exact coverage: `ui::frameapi_fpleaf_fabs_cmp` covers `0x4afea0` to `0x4afeb3` of `0x4afea0`.

## Mail Lua functions

The registrar `0x4afee0` walks 30 name and function pairs (`0x4afef7 cmp esi,0xf0`); the unregister loop `0x4aff10` mirrors
it. The inbox functions are `GetInboxNumItems`, `GetInboxHeaderInfo`, `GetInboxText`, `GetInboxInvoiceInfo`,
`GetInboxItem`, `TakeInboxMoney`, `TakeInboxItem`, `TakeInboxTextItem`, `ReturnInboxItem`, `DeleteInboxItem`,
`InboxItemCanDelete`, `CheckInbox` and `HasNewMail`, with `CloseMail`; the rest are the compose functions of
`ui/mailsend`.

Addresses: `0x4afee0`, `0x4aff10`, `0x4aded0`, `0x4aeab0`, `0x4aeb90`, `0x4aebc0`, `0x4af110`, `0x4af360`, `0x4af5d0`, `0x4af7d0`, `0x4af8e0`, `0x4afa60`, `0x4afba0`, `0x4afcd0`, `0x4afe00`, `0x4afea0`, `0x704120`, `0x704160`, `0x49034b`
Confidence: verified
Proof: one
Test: `ui::frameapi_fpleaf_fabs_cmp`

The pairs start at `0x845eb0`. The only caller of the registrar is `0x49034b`; `0x4afee9` calls `0x704120` once per pair.

Bit-exact coverage: `ui::frameapi_fpleaf_fabs_cmp` covers `0x4afea0` to `0x4afeb3` of `0x4afea0`.
