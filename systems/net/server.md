# net: server view

A server developer arrives with an opcode name or a server constant, not an address. This page says where the client's side of each is derived. `tables/opcodes.tsv` lists the handler of every received opcode, `tables/opcode_sends.tsv` every place the client sends one, and `tables/opcode_entries.tsv` the entry of `systems/` that covers each, or `unread`. The entries below give the checks the client runs before it sends, with their constants, and set each constant beside the vmangos one it explains or differs from, by file and line. vmangos is the tree at commit `b7cd0e1`; its 1.12.1 opcode list is `src/game/Server/Protocol/Opcodes_1_12_1.h`. A line cited as `file:N` is read in that tree. Entries that list the client's own values rest on the entry they link.

Roots: `0x5379a0`, `0x5ab630`, `0x537aa0`, `0x5ab650`, `0x537a60`

## Opcode names

The client holds no message names. The `name` column of `opcodes.tsv`, `opcode_sends.tsv` and `opcode_entries.tsv` is the enum name in vmangos `Opcodes_1_12_1.h`, the list `Opcodes_active.h:7-8` includes for a build above 5464 (`Progression.h:30`, 5875 at `:31`). `tools/tables/opcode_names.tsv` holds its 825 entries with their line numbers and the generators join it. Every opcode the client handles or sends has a name there. The ids `0x13f`, `0x140` and `0x33a` have none.

Addresses: `0x537a60`, `0x5ab650`, `0x537aa0`
Confidence: verified
Proof: one
Test: none

The handler tables hold 828 entries (`net/dispatch#handler-tables`), the ids `0x000` to `0x33b`, and the vmangos enum ends with `NUM_MSG_TYPES = 828`. The names are a fact of vmangos, not of the exe; the check regenerates the tables from the committed input.

## Outgoing packet shape

Every packet the client sends is a `CDataStore` built on the stack. Its first write is the opcode as a u32, `PutUInt32` (`0x418190`) or one of its two identical copies (`0x4181f0`, `0x4182b0`), then the body. It goes out through `ClientServices::Send` (`0x5ab630`) or `NetClient::Send` (`0x5379a0`), which sends only in connection state 6. The frame is the size of opcode and body, big-endian, two bytes up to 32,767 and three above, then the opcode as four bytes little-endian (`net/wire#size-header`). vmangos reads a fixed 6-byte client header, a u16 size and a u32 command (`WorldSocket.h:50-54`), so it has no counterpart for the three-byte form.

Addresses: `0x418190`, `0x4181f0`, `0x4182b0`, `0x5ab630`, `0x5379a0`, `0x5b5630`
Confidence: verified
Proof: one
Test: none

The three puts are byte-identical bodies. `0x5379a0` tests connection state 6 and a non-empty store; `0x5ab630` sends only when a connection exists (`net/wire#sending`). The size rule is `net/wire#size-header`, where it is read twice; the vmangos side is one reading.

## Send site rows

`tables/opcode_sends.tsv` has one row per place a send call follows an opcode write. Most write the opcode as an immediate. Others take it from a register (`via-register`), from a caller's argument (`through-<function>`, for the movement and channel senders), from the cache object (`cache-object`), or by hand (`hand-resolved`); a row with opcode `?` is one of three: the `wowcache` lookup `0x56b5fa`, whose cache object holds opcode 0, and the two writes in `0x619b50` (`0x619bb9`, `0x619c05`), whose opcode is not a constant and which no live code reaches. The scan follows the listing, not the branches, so a send reached from several writes lists each opcode.

Addresses: `0x5ab630`, `0x5379a0`, `0x728640`, `0x600a30`, `0x600a10`, `0x60e0a0`, `0x4a00f0`, `0x4a04b0`, `0x5ab000`
Confidence: verified
Proof: one
Test: none

The control is the cast request `0x6e54f0`, which `spell/cast#cast-request-packets` reads to the bytes: the scan gives 0xab, 0x12e and 0x1f0 for it. `0x728640` is the minigame's wrapper around `0x5ab630`, so its caller `0x728650` is the site. The movement opcodes pass through `0x60e0a0`, `0x617fe0`, `0x600a10` and `0x600a30`, which take the opcode as an argument; `0x61a820` chooses its opcode from state, and its row is `?`.

## Formats vmangos reads

The bodies the client writes for these opcodes are the bodies vmangos reads, field for field. A client body that vmangos reads under `SUPPORTED_CLIENT_BUILD > CLIENT_BUILD_1_9_4` is the 1.12.1 body.

| opcode | client entry | vmangos reader |
|---|---|---|
| `CMSG_MESSAGECHAT` | `ui/chatsend#the-chat-packet` | `Packets/Chat.cpp:3-12` |
| `CMSG_SEND_MAIL` | `ui/mailsend#sending` | `Packets/Mail.cpp:3-20` |
| six `CMSG_AUCTION_*` | `ui/auction#requests-the-client-sends` | `Packets/AuctionHouse.cpp:10-65` |
| five trade opcodes with a body | `objectlayer/trade#outgoing-trade-messages` | `Packets/Trade.cpp:5-31` |
| `CMSG_MOVE_TIME_SKIPPED` | `collision/mover#heartbeat-and-skipped-time` | `Packets/Movement.cpp:9-13` |
| `MSG_MOVE_TELEPORT_ACK` | `objectlayer/moveemitter#emitter-routing` | `Packets/Movement.cpp:15-22` |
| `CMSG_MOVE_SPLINE_DONE` | `collision/status#spline-completion` | `Packets/Movement.cpp:70-75` |
| `CMSG_WHOIS` | this page, Social messages | `Packets/Query.cpp:30-33` |

Addresses: `0x49f72a`, `0x4aea6c`, `0x4cd7fd`, `0x5d4230`, `0x600c67`, `0x60e25a`, `0x5adee0`
Confidence: verified
Proof: one
Test: none

Each row's client entry reads the sender's puts at the bytes. The vmangos readers were compared against them one field at a time: the chat target is read for types 6 and 14 only, the mail body ends in a zero u64 and a zero u8, the auction bodies start with the auctioneer GUID, `CMSG_ACCEPT_TRADE` carries a u32 that vmangos skips (`Trade.cpp:29-30`), the time-skipped body is a GUID and a u32, the teleport acknowledgement is a GUID and two u32, which vmangos reads as a counter and a time, and the spline-done body is the movement status, a u32 spline id and a float vmangos skips.

## Chat sends

`SendChatMessage` and `SendAddonMessage` run these checks before the packet leaves; the packet is opcode 0x95, a u32 type, a u32 language, the target for types 6 and 14, and the text. Anything the checks refuse is not sent.

| check | value | entry |
|---|---|---|
| type names | `SAY` 0, `PARTY` 1, `RAID` 2, `GUILD` 3, `OFFICER` 4, `YELL` 5, `WHISPER` 6, `EMOTE` 8, `CHANNEL` 14, `AFK` 20, `DND` 21, `RAID_WARNING` 88, `BATTLEGROUND` 92 | `ui/chatsend#arguments-of-sendchatmessage` |
| group types | party needs a party, raid and raid warning a raid, raid warning a leader or assistant; a leader's raid goes as 0x57 and battleground as 0x5d | `ui/chatsend#group-and-channel-checks` |
| dead or ghost | twelve types only, else error 0xef | `ui/chatsend#text-steps` |
| text buffer | 256 bytes, `%N`, `%T`, `%n` and `%t` expanded to the target's name | `ui/chatsend#target-token-expansion` |
| escape codes | a pipe must be followed by a pipe or `c` | `ui/chatsend#the-escape-check` |
| addon message | prefix, tab, message in 2047 characters; language 0xffffffff | `ui/chatsend#sendaddonmessage` |

Addresses: `0x49f1e0`, `0x49f7a0`, `0x49f604`, `0x5083a0`, `0x49f661`, `0x49f920`, `0x49f9a2`, `0x49fab9`, `0x49f72a`
Confidence: verified
Proof: one
Test: none

The client sends language 0xffffffff for an addon message, `LANG_ADDON` (`SharedDefines.h:270`), which vmangos tests at `ChatHandler.cpp:165`. The type bytes 6 and 14 are `CHAT_MSG_WHISPER` and `CHAT_MSG_CHANNEL` (`SharedDefines.h:1200`, `:1208`). The 256-byte buffer is `push 0x100` before `0x5083a0` (`0x49f604`), and the addon text is formatted with the size `0x800` (`0x49f9a2`).

## Chat types and notices

The client indexes 94 chat types (`tables/chat_types.tsv`) and 32 channel notices (`tables/channel_notices.tsv`). The types equal vmangos `ChatMsg` (`SharedDefines.h:1191`) except 89: the client names it `FOREIGN_TELL`, with the whisper colour `ff80ff` and event id 540, which has no name, where vmangos has `CHAT_MSG_RAID_BOSS_WHISPER = 0x59` and the comment "appears as whisper, yet name is not in client" (`SharedDefines.h:1292`). The notices equal `ChatNotify` (`Channel.h:35-71`) in number. The client shows `SUSPENDED` for notice 3 and `YOU_CHANGED` for notice 2 by the state of its own slot; vmangos comments `CHAT_SUSPENDED_NOTICE` as 1 (`Channel.h:39`), a number the client does not use for it.

Addresses: `0x49bf80`, `0x49c036`, `0x49c0cc`, `0x49b0b0`
Confidence: verified
Proof: one
Test: none

The comparison is by index and name against the two tables; `chat_types.tsv` row 89 has an empty `event`. `0x49c0cc` takes `SUSPENDED` only for a found slot in state 3 (`ui/chatchannels#zone-changes-keep-channels`).

## Mail sends

`SendMail(target, subject, body)` sends opcode 0x238 and checks, in order, a pending or in-flight send, the mailbox, a non-empty target and subject, a selected stationery, and the attachment and money.

| check | value | entry |
|---|---|---|
| attachment | one item GUID; bound, quest, conjured and container-with-contents items are refused | `ui/mailsend#attaching-an-item` |
| money | `SetSendMailMoney` stores the amount only when coinage covers it | `ui/mailsend#money-and-cod` |
| COD | needs an attachment; money and COD together send nothing | `ui/mailsend#sending` |
| postage shown | the money being sent, plus 30 copper, plus the stationery's price when the player does not carry it, plus the package's cost | `ui/mailsend#postage-and-the-attachments-details` |
| body | GUID, target, subject, body, stationery u32, package u32, item GUID, money u32, COD u32, zero u64, zero u8 | `ui/mailsend#sending` |

Addresses: `0x4ae800`, `0x4ae8dd`, `0x4ae99e`, `0x4ae756`, `0x4ae730`, `0x4aea6c`
Confidence: verified
Proof: one
Test: none

vmangos has one attachment (`Mail.h:51`, `MAX_MAIL_ITEMS 1`) and charges `money + 30` (`MailHandler.cpp:243`), so the stationery and package cost the client adds to its displayed price (`0x4ae756` loads 30, `0x4ae7a0` and `0x4ae7c6` add the rest) is not part of what that server takes. The client's money test compares coinage with the amount alone, not with the postage.

## Trade sends

The trade senders put the opcode and a small body, send through `0x5ab630`, and check nothing about a window. The two that raise the money offer check coinage.

| opcode | body | check |
|---|---|---|
| `CMSG_INITIATE_TRADE` 0x116 | u64 GUID | none; no range test |
| `CMSG_BEGIN_TRADE` 0x117, `CMSG_BUSY_TRADE` 0x118, `CMSG_IGNORE_TRADE` 0x119, `CMSG_UNACCEPT_TRADE` 0x11b, `CMSG_CANCEL_TRADE` 0x11c | none | none |
| `CMSG_ACCEPT_TRADE` 0x11a | u32 from `0xc4be78`+4, set to 1 when the window opens | none |
| `CMSG_SET_TRADE_ITEM` 0x11d | u8 trade slot, u8 container index, u8 slot | an active player and the item |
| `CMSG_CLEAR_TRADE_ITEM` 0x11e | u8 trade slot | none |
| `CMSG_SET_TRADE_GOLD` 0x11f | u32 copper | add and set: coinage at least the new total |

Addresses: `0x5d3fb0`, `0x5d4230`, `0x5d4380`, `0x5d4480`, `0x5d4500`, `0x5d4570`, `0x5d4680`, `0x5d45f0`
Confidence: verified
Proof: one
Test: none

Details are `objectlayer/trade#outgoing-trade-messages` and `#offering-money`. The client keeps seven item records for each side, indexes 0 to 6 (`0xc4bab8`, 14 records); vmangos has `TRADE_SLOT_COUNT 7` with six traded and slot 6 not traded (`TradeData.h:28-30`) and refuses a slot of 7 or more (`TradeHandler.cpp:694`). Its gold test, `packet.gold > _player->GetMoney()` (`TradeHandler.cpp:674`), is the client's `0x5d4570` test. vmangos's accept body is a u32 it skips (`Trade.cpp:29-30`).

## Auction sends

Each auction request starts with the auctioneer GUID. The client throttles one request and pages the lists.

| opcode | body after the GUID | entry |
|---|---|---|
| `CMSG_AUCTION_SELL_ITEM` 0x256 | item GUID, minimum bid, buyout, duration | `ui/auction#requests-the-client-sends` |
| `CMSG_AUCTION_REMOVE_ITEM` 0x257 | auction id | same |
| `CMSG_AUCTION_LIST_ITEMS` 0x258 | offset, text, two level bytes, four u32, usable byte | same |
| `CMSG_AUCTION_LIST_OWNER_ITEMS` 0x259 | offset | same |
| `CMSG_AUCTION_PLACE_BID` 0x25a | auction id, bid | same |
| `CMSG_AUCTION_LIST_BIDDER_ITEMS` 0x264 | offset, count, ids | same |
| page | offset is the page times 50 | `ui/auction#requests-the-client-sends` |
| query throttle | 5000 ms after each 0x258 | `ui/auction#the-query-throttle` |

Addresses: `0x4ceb44`, `0x4cec47`, `0x4cec4c`, `0xb72638`, `0x4cd720`, `0x4cfbb0`
Confidence: verified
Proof: one
Test: none

`0x4ceb44` is `imul ebx,ebx,0x32`; `0x4cec47` is `add eax,0x1388` into `0xb72638`. vmangos returns at most 50 entries per list from the offset (`AuctionHouseMgr.cpp:730`, `:688`, `:704`; `AuctionHouseHandler.cpp:675`).

## Cache query opcodes

Twelve caches ask the server for a record the first time a key is wanted. The query is the cache's opcode, the key and, for caches with the owner flag, the caller's GUID. The lookup function reads the opcode from its cache object at `+0x30`, which the cache's start-up constructor call sets.

| cache | store | opcode | lookup |
|---|---|---|---|
| creature | `0xc0e354` | `CMSG_CREATURE_QUERY` 0x60 | `0x556aa0` |
| game object | `0xc0e318` | `CMSG_GAMEOBJECT_QUERY` 0x5e | `0x558560` |
| item name | `0xc0e2dc` | `CMSG_ITEM_NAME_QUERY` 0x2c4 | `0x559fe0` |
| item | `0xc0e2a0` | `CMSG_ITEM_QUERY_SINGLE` 0x56 | `0x55ba30` |
| NPC text | `0xc0e264` | `CMSG_NPC_TEXT_QUERY` 0x17f | `0x55d530` |
| name | `0xc0e228` | `CMSG_NAME_QUERY` 0x50 | `0x55f080` |
| guild | `0xc0e1ec` | `CMSG_GUILD_QUERY` 0x54 | `0x560e30` |
| quest | `0xc0e1b0` | `CMSG_QUEST_QUERY` 0x5c | `0x562a40` |
| page text | `0xc0e174` | `CMSG_PAGE_TEXT_QUERY` 0x5a | `0x564730` |
| pet name | `0xc0e138` | `CMSG_PET_NAME_QUERY` 0x52 | `0x566240` |
| petition | `0xc0e0fc` | `CMSG_PETITION_QUERY` 0x1c6 | `0x567d60` |
| item text | `0xc0e0c0` | `CMSG_ITEM_TEXT_QUERY` 0x243 | `0x569980` |

Addresses: `0x554b40`, `0x554cd0`, `0x556ba5`, `0x55866b`, `0x55a0e5`, `0x55bb3b`, `0x55d63b`, `0x55f1c5`, `0x560f3b`, `0x562b4b`, `0x56483b`, `0x566345`, `0x567e6b`, `0x569a8b`
Confidence: verified
Proof: one
Test: none

Each lookup puts its opcode from `[cache+0x30]`; the caller passes the store in `ecx` (the first caller of each lookup gives the store above), and the constructor call, six pushes then `mov ecx,<store>`, pushes the opcode fourth (`0x554b46` for creatures, `0x554cd6` for names). The first writes after the vtable store are the puts listed. `net/dispatch` and `dbcache/overview#asking-for-a-record` describe the lookup. A thirteenth cache, `wowcache.wdb` at `0xc0e084`, has opcode 0 and its lookup `0x56b430` is not read here.

## Social messages

Friend, ignore and whois traffic. `CMSG_ADD_IGNORE` 0x6c sends the name; `CMSG_DEL_IGNORE` 0x6d the GUID; `CMSG_DEL_FRIEND` 0x6a the GUID; `CMSG_WHOIS` 0x64 a string. The ignore list is 25 GUIDs and the friend list 50 slots; `SMSG_IGNORE_LIST` 0x6b is a count byte and that many GUIDs, which replace the list, and `SMSG_WHOIS` 0x65 is one string of up to 255 characters handed to the console writer. `AddOrDelIgnore` sends `CMSG_DEL_IGNORE` with the GUID of an ignored player whose cached name matches, else `CMSG_ADD_IGNORE` with the name. vmangos has the same limits, 50 and 25 (`SocialMgr.h:114-115`).

Addresses: `0x5ae760`, `0x5af4b0`, `0x5af1a0`, `0x5af300`, `0x5adee0`, `0x5af530`, `0x5ae2b0`, `0x5adda0`, `0x5af5d0`
Confidence: verified
Proof: one
Test: none

`0x5ae760` puts 0x6a and a GUID, `0x5af4b0` 0x6d and a GUID, `0x5adee0` 0x64 and a string. `0x5af1a0` walks the 25 slots from `+0x650` (`cmp edi,0x19`, `0x5af212`), asks the name cache (`0x55f080`) for each GUID and compares the name with `0x64a4c0`; a match sends 0x6d (`0x5af27f`), none sends 0x6c with the name (`0x5af21f`). `0x5af530` zeroes `0x32` dwords, reads a count byte and that many GUIDs through `0x5af5d0`, which stores each in the first empty slot of 25 and ignores any more. `0x5adda0` reads a string with capacity 0x100. vmangos reads the whois name and writes the reply string (`Packets/Query.cpp:30-33`, `Packets/Misc.cpp:289-292`).

## Cast early refusals

The cast attempt `0x6e4b60` refuses in this order before anything is sent; a refusal that names a reason reports it through `0x6e1a00`.

| check | refused when | reason |
|---|---|---|
| inputs | no caster, no record, or `Attributes & 0x40` | none |
| `0x494a50(2)` | it returns 0 | none |
| profession | first effect 47 | a window toggles, nothing sent |
| dead | caster dead and not `Attributes & 0x800000` | 0x13 |
| attack | first effect 78 | none, nothing sent |
| looting | first effect 99, 127, or 53, 54, 92 | 0x82, 0x83, 0x61 in that order |
| same spell | the spell is the one in flight | a sound |

Addresses: `0x6e4b60`, `0x6e4bac`, `0x6e4bc1`, `0x6e4c49`, `0x6e4d43`, `0x494a50`, `0x6e1a00`
Confidence: verified
Proof: one
Test: none

The table is `spell/cast#cast-attempt-refusals`. The reason bytes are the failure ids of Cast failure reasons below.

## Cast gates

After the early refusals the attempt checks, in this order, and the first failure is the only one reported.

| check | refused when | reason |
|---|---|---|
| in flight | a spell is in flight and the new one lacks `Attributes & 0x404` | 0x61 |
| reagents, totems | `0x6e4000` fails | by the check |
| equipped item | `0x6e40e0` fails | by the check |
| ranged | `Attributes & 2` and no ranged weapon | 0x18 |
| combo points | `AttributesEx & 0x500000`, no combo points | 0x46 or 0x12 |
| range | `0x6e4440` fails | 0x59 or 0x76 |
| validator | `0x6094f0` fails | one of its rungs |

Addresses: `0x6e4d62`, `0x6e4dbe`, `0x6e4e50`, `0x6e4e85`, `0x6e4f26`, `0x6e4f3b`, `0x6094f0`, `0x609549`, `0x609616`
Confidence: verified
Proof: one
Test: none

The table is `spell/cast#cast-attempt-gates`. The validator's rungs run in the order `spell/castgates#rung-order` gives: item cooldown 0x28, spell cooldown 0x3c, power 0x4d, summons and pets, control, stance, mount, stand, daytime, water, moving 0x2e, form, stealth, aura state, reputation 0x8f, place and taming. All of this runs before the request is sent.

## Cast failure reasons

The reason bytes the client reports are the vmangos `SpellCastResult` values. `tables/spell_failed.tsv` lists the GlobalStrings key of each of the client's 146 reasons, `0x00` to `0x90` and the unknown key; every key is `SPELL_FAILED_` and the name of the vmangos enumerator with the same value, in the same order, for the 1.12.1 branch (`SpellDefines.h:352-523`, where `CANT_BE_PROSPECTED` at `:366` sits under `SUPPORTED_CLIENT_BUILD > CLIENT_BUILD_1_11_2`). `SPELL_CAST_OK = 0xFF` (`SpellDefines.h:525`) is never reported.

Addresses: `0x6e23e0`, `0x6e23f6`, `0x86faa0`, `0x870b64`, `0x6e1a00`
Confidence: verified
Proof: one
Test: none

`0x6e23e0` returns the string r places below `0x870b64` for r up to 0x90 and `0x86faa0` above it; the generator checks that each of the 145 `mov eax,imm32; ret` pairs at `0x6e23f6 + 6r` returns the string it names. The 146 names were compared with the enumerators one by one, with the `#if` at `SpellDefines.h:365` taken for build 5875. The local reasons the cast checks raise are `spell/reasons#reasons-raised-locally`.

## Cast range constants

The cast range check, `0x6e47b0`, compares the squared 3D distance of the two positions with the squares of a minimum and a maximum from `0x6e3480`; equality passes both bounds. The melee row gives `max(R(target) + R(caster) + 4/3, 5.0)` with the combat reaches `R`; other rows give the range row's minimum and maximum, plus both reaches for a target of type bit 3 or 7. When both units have a movement flag of `0x200d`, each moves faster than twice its own walk speed, and the row is a melee row or the unit a player, `8/3` is added.

| constant | client | vmangos |
|---|---|---|
| melee offset | 4/3 as float, `0x3faaaaab` | `BASE_MELEERANGE_OFFSET 1.333333373069763` (`UnitDefines.h:70`, `Unit.cpp:10613`) |
| melee floor | 5.0 | `ATTACK_DISTANCE 5.0` (`ObjectDefines.h:25`, `Unit.cpp:10614`) |
| moving bonus | 8/3 = 2.6666667, `0x402aaaab` | `LEEWAY_BONUS_RANGE 2.66` (`ObjectDefines.h:57`) |
| moving speed | strictly above twice the walk speed | above `LEEWAY_MIN_MOVE_SPEED 4.97` (`ObjectDefines.h:56`) |
| tolerance | none | players 1.25 when strict, 6.25 when not; others 0 or 2.25 (`Spell.cpp:6941`) |

Addresses: `0x6e47b0`, `0x6e3480`, `0x6e35bf`, `0x6e369c`, `0x6e3658`, `0x5fc350`, `0x6e4440`
Confidence: verified
Proof: one
Test: `spell::spell_range_check_difftest`, `spell::spell_minmax_range_difftest`

The derivation is `spell/records#range` and `#range-bonuses-and-modifier`. `0x5fc350` returns 0 when the current speed exceeds twice the float at `+0xa30`, the walk speed; with walk speed 2.5 (`Unit.cpp:68`) that is 5.0 where vmangos uses 4.97. The client's melee compare is 3D and passes at equality; vmangos compares the 2D distance with `<` (`Object.cpp:1859`).

## Interaction and inspect ranges

The client's own range tests, and the vmangos constants they meet.

| client check | value | vmangos |
|---|---|---|
| NPC click | walks up when farther than 5.5555553 yards | `INTERACTION_DISTANCE 5.0` plus both bounding radii (`ObjectDefines.h:24`, `Player.cpp:2556`, `Object.h:595`) |
| inspect | squared distance at most 10² | `INSPECT_DISTANCE 10.0` (`ObjectDefines.h:26`, `MiscHandler.cpp:951`) |
| `CheckInteractDistance` 1 to 4 | 10, 11.111111 (`0x4131c71c`, 100/9), 10, 30 | `TRADE_DISTANCE 11.11`, slightly below the client's value (`ObjectDefines.h:27`), trade checks center to center (`TradeHandler.cpp:272`) |
| spirit healer | chosen within 20, kept to 22.000001 | not read |
| corpse | within 40 | not read |

Addresses: `0x5f0130`, `0xc4c28c`, `0x4944a0`, `0xb4d918`, `0x48ba00`, `0x48ba30`, `0x4924c0`, `0x4923b0`, `0x492130`
Confidence: verified
Proof: one
Test: `objectlayer::p3_in_range_gate_5f0130`, `ui::dist2_4944a0`, `ui::check_interact_dist2`

The NPC range is `InitPlayerSquare4` (`0x5f1350`): `fld [0x80ae48]; fmul [0x80ae48]` with 5.5555553 at `0x80ae48`; its square, 30.864195, is the float at `0xc4c28c`. `CheckInteractDistance` is an interface function; the client's trade sender `0x5d3fb0` tests no range, so only an interface that calls it first applies the 11.111111. The entries are `objectlayer/interaction#talking-to-an-npc` and `ui/distancegates#thresholds`.

## Movement sends and cadence

A movement report that carries no GUID is built only for the active mover; the acknowledgements, which carry one, skip that test (`objectlayer/unitmovement#who-may-send-a-report`).

| send | when | entry |
|---|---|---|
| `MSG_MOVE_HEARTBEAT` 0xee | the mover has a movement flag of `0x200f` and 500 ms have passed since its last movement message | `collision/mover#heartbeat-and-skipped-time` |
| `CMSG_MOVE_TIME_SKIPPED` 0x2ce | a frame longer than 250 ms reports `w - 250` ms | `collision/mover#local-player-step` |
| `CMSG_MOVE_SPLINE_DONE` 0x2c9 | the active player's server spline is marked done | `collision/status#spline-completion` |
| `CMSG_MOVE_FALL_RESET` 0x2ca | the unit's byte `+0xc58` has bit 7 | `collision/handlers#opcode-0x2ca-report` |
| `MSG_MOVE_FALL_LAND` 0xc9 | landing | `collision/fallwire#airborne-sends` |

Addresses: `0x615b80`, `0x615ba5`, `0x600a30`, `0x600be0`, `0x616642`, `0x616651`, `0x602cf0`, `0x600b10`, `0x602e13`
Confidence: verified
Proof: one
Test: none

`0x615ba5` is `lea esi,[eax+0x1f4]`: every message `0x600a30` sends re-arms the deadline to its time plus 500. vmangos expects a heartbeat every 500 ms (`MovementAnticheat.cpp:1328`) and flags a mover whose last movement is more than 1000 ms old while moving (`:1329`), twice the client's interval. `0x616642` is `cmp esi,0xfa`: the 250 ms window, with the rest sent as `0x2ce`.

## Unread opcodes

These opcodes have a handler or a send site that no entry covers; `tables/opcode_entries.tsv` marks them `unread`.

| opcode | role | function |
|---|---|---|
| `SMSG_GOSSIP_COMPLETE` 0x17e | handler | `0x4e2800` |
| `SMSG_INSTANCE_SAVE_CREATED` 0x2cb | handler | `0x4e7e60` |
| `SMSG_RAID_INSTANCE_INFO` 0x2cc, `SMSG_RAID_INSTANCE_MESSAGE` 0x2fa | handler | `0x49e070`, `0x49e1c0` |
| `SMSG_INSTANCE_RESET_FAILED` 0x31f | handler | `0x49e540` |
| `MSG_SET_DUNGEON_DIFFICULTY` 0x329 | handler | `0x4e8f70` |
| `SMSG_LOTTERY_QUERY_RESULT_OBSOLETE` 0x335, `SMSG_LOTTERY_RESULT_OBSOLETE` 0x337 | handler | `0x4c3d50`, `0x4c3e60` |
| `CMSG_TELEPORT_TO_UNIT` 0x9, `MSG_MOVE_TELEPORT_CHEAT` 0xc6 | send | `0x404a60` |
| `CMSG_MOVE_SET_RAW_POSITION` 0xe1 | send | `0x405000` |
| `CMSG_REQUEST_PET_INFO` 0x279 | send | `0x4bc6f0` |
| `CMSG_TOGGLE_HELM` 0x2b9, `CMSG_TOGGLE_CLOAK` 0x2ba | send | `0x405250`, `0x4052f0` |

Addresses: `0x537aa0`, `0x5379a0`
Confidence: inferred
Proof: none
Test: none

Their bodies are not derived. `0x405250` and `0x4052f0` put 0x2b9 and 0x2ba and send with no body; `0x4bc6f0` puts 0x279 with no body.
