# ui: game start-up and shutdown

How the interface session begins and ends. `CGGameUI::InitializeGame` runs when the client enters the game state and `CGGameUI::ShutdownGame` when it leaves it. Each runs the start-up or shutdown of the interface modules in a fixed order, around three CVars and the handler that carries 14 server messages. The modules are those of the other docs of this system, listed here with the pair of functions that begin and end their state.

Roots: `0x48f480`, `0x491180`

## Game start-up

`0x48f480` installs the handler `0x48f690` for the opcodes 0x28c, 0x216, 0x269, 0x230, 0x22f, 0x277, 0x2d2, 0x278, 0x2b8, 0x2c2, 0x2c3, 0x2e4, 0x1e7 and 0x2f4, sets flag bit 2 (read-only) of the CVars realmList, realmName and scriptMemory, calls `DBCacheClearUnsaved`, and runs the 31 module initializers of the next three entries. It then sets the player-has-control flag, clears the cinematic flag, arms the billing warning (`ui/billing`) and jumps to `CGGameUI::Initialize`, which builds the interface (`ui/framexml`).

Addresses: `0x48f480`, `0x48f690`, `0x555740`, `0x63de30`, `0x63e030`, `0x48fbf0`, `0x5ab650`
Confidence: verified
Proof: one
Test: none

The 14 handler installs are `mov ecx,opcode; mov edx,0x48f690; push 0; call 0x5ab650`. The CVars come from the three-pointer table at 0x83de4c, looked up with `0x63de30`; `0x63e030` sets bit 2 of the flag word at +0x1c for a nonzero argument and clears it for zero, and `SetCVar` refuses a CVar with that bit (`ui/cvarverbs`).

## Game shutdown

`0x491180` runs `CGGameUI::Shutdown`, then the 34 module shutdowns of the next three entries, `ResetFrameScript`, clears flag bit 2 of the same three CVars, frees the four zone name strings and zeroes the zone ids, and clears the handlers of the 14 opcodes that start-up installed. The modules run in the order of the call list, which is not the reverse of start-up.

Addresses: `0x491180`, `0x490bd0`, `0x703b80`, `0x5ab670`
Confidence: verified
Proof: one
Test: none

The four zone strings are s_zoneText, s_subZoneText, s_realZoneText and s_minimapZoneText, each freed through `SMemFree` with the file `GameUI.cpp` and a line number (`ui/zonetext`). The clears are `mov ecx,opcode; call 0x5ab670`, the last a tail jump.

## Start-up display and chat

Eleven modules that draw or hold state shown on screen. Each has an initializer that `0x48f480` calls and a shutdown that `0x491180` calls; three of the shutdowns and two of the initializers are a lone `ret`.

| Module | Start-up | Shutdown |
|---|---|---|
| Portraits | `0x524b60` | `0x524b70` |
| Minimap | `0x4ed810` | `0x4edb70` |
| Chat | `0x4982c0` | `0x498700` |
| Chat bubbles | `0x4b0f30` | `0x4b0f40` |
| Party | `0x4e7c70` | `0x4e7f20` |
| Spell book | `0x4b21c0` | `0x4b22e0` |
| Action bar | `0x4e4f50` | `0x4e4fb0` |
| Buffs | `0x4e40c0` | `0x4e40f0` |
| Loot | `0x4c1c20` | `0x4c1c50` |
| Item text | `0x4e3170` | `0x4e31f0` |
| World map | `0x4a5d00` | `0x4a64b0` |

Addresses: `0x524b60`, `0x524b70`, `0x4ed810`, `0x4edb70`, `0x4982c0`, `0x498700`, `0x4b0f30`, `0x4b0f40`, `0x4e7c70`, `0x4e7f20`, `0x4b21c0`, `0x4b22e0`, `0x4e4f50`, `0x4e4fb0`, `0x4e40c0`, `0x4e40f0`, `0x4c1c20`, `0x4c1c50`, `0x4e3170`, `0x4e31f0`, `0x4a5d00`, `0x4a64b0`
Confidence: verified
Proof: one
Test: none

`0x524b60`, `0x4b0f30`, `0x4e4fb0`, `0x4e40f0` and `0x4c1c50` return at once: the portrait, chat bubble, action bar, buff and loot modules have nothing to free there. `0x4b21c0` jumps to `0x4b2f70` after sizing its arrays.

## Start-up world and quests

Eleven modules of quest, trade, social and world state. The same pairing as the first group:

| Module | Start-up | Shutdown |
|---|---|---|
| Gossip | `0x4e1ef0` | `0x4e1f30` |
| Quest log | `0x4de390` | `0x4de420` |
| Taxi | `0x4db340` | `0x4db5b0` |
| Trainer | `0x4d7340` | `0x4d7350` |
| Pet | `0x4bc690` | `0x4bc7b0` |
| Reputation | `0x4d5290` | `0x4d5360` |
| Duel | `0x4d4710` | `0x4d4780` |
| Tutorials | `0x4b5330` | `0x4b5380` |
| Skills | `0x4d2ab0` | `0x4d2c00` |
| Guild roster | `0x4d0a00` | `0x4d0a70` |
| Mail | `0x4ac980` | `0x4acc40` |

Addresses: `0x4e1ef0`, `0x4e1f30`, `0x4de390`, `0x4de420`, `0x4db340`, `0x4db5b0`, `0x4d7340`, `0x4d7350`, `0x4bc690`, `0x4bc7b0`, `0x4d5290`, `0x4d5360`, `0x4d4710`, `0x4d4780`, `0x4b5330`, `0x4b5380`, `0x4d2ab0`, `0x4d2c00`, `0x4d0a00`, `0x4d0a70`, `0x4ac980`, `0x4acc40`
Confidence: verified
Proof: one
Test: none

`0x4d7340` returns at once. The quest log initializer clears 160 row words with `rep stos` and installs the handler `0x4de400` for opcode 0x1cf; its shutdown clears only that handler.

## Start-up services and events

Nine modules that talk to the server in a service flow, and the three that have a shutdown only (trade skill, craft and talents):

| Module | Start-up | Shutdown |
|---|---|---|
| Battlefield | `0x4a9c40` | `0x4a9d30` |
| Auction house | `0x4cc0f0` | `0x4cc300` |
| Stable | `0x4caa80` | `0x4caaf0` |
| Raid | `0x4ba0b0` | `0x4ba1d0` |
| Meeting stone | `0x4c9ee0` | `0x4c9fb0` |
| Inspect honor | `0x4c6d10` | `0x4c6e30` |
| World state | `0x4c5590` | `0x4c55b0` |
| Minigame | `0x4c4be0` | `0x4c4c50` |
| Lottery | `0x4c3d20` | `0x4c3e90` |
| Trade skill | none | `0x4fc790` |
| Craft | none | `0x4f5f00` |
| Talents | none | `0x4f2ea0` |

Addresses: `0x4a9c40`, `0x4a9d30`, `0x4cc0f0`, `0x4cc300`, `0x4caa80`, `0x4caaf0`, `0x4ba0b0`, `0x4ba1d0`, `0x4c9ee0`, `0x4c9fb0`, `0x4c6d10`, `0x4c6e30`, `0x4c5590`, `0x4c55b0`, `0x4c4be0`, `0x4c4c50`, `0x4c3d20`, `0x4c3e90`, `0x4fc790`, `0x4f5f00`, `0x4f2ea0`, `0x4fc750`, `0x4f5ee0`, `0x4909bf`, `0x4909c4`
Confidence: verified
Proof: one
Test: none

Trade skill, craft and talents have no initializer in the start-up list; their lists are built when a window opens (`tradeskill/recipes`, `tradeskill/craft`, `ui/talents`); the shutdown frees them, and `EnterWorldUI` resets the trade skill list (`0x4fc750` at `0x4909bf`) and the craft list (`0x4f5ee0` at `0x4909c4`). The raid shutdown clears the handlers for opcodes 0x321 and 0x322 and frees the 40 roster records.

## Interaction close

`0x493310` is the one closer of the interaction target, the unit the player is talking to (the dword pair at 0xb4e2d0). It clears the target, compares its GUID in turn with the GUID each window stored and closes the first that matches: trade partner, item text, gossip, quest frame, merchant, taxi master, trainer, banker, guild registrar, tabard vendor, mailbox, auctioneer, stable master and loot. It then looks the object up and calls `0x5f8c40`, which runs slot 32 of the object's handler as the interface closes its frame (`objectlayer/gameobjecttypes`).

Addresses: `0x493310`, `0x4c3e80`, `0x4bf4e0`, `0x4e32e0`, `0x4e1fa0`, `0x501130`, `0x4fadf0`, `0x4dc0f0`, `0x4d7440`, `0x4f84d0`, `0x4f5010`, `0x4f58a0`, `0x4acd50`, `0x4cd5f0`, `0x4cae10`, `0x4c3f40`, `0x5f8c40`
Confidence: verified
Proof: one
Test: none

Each test is `cmp stored,esi; jne next; cmp stored+4,edi; jne next` against the target GUID in `esi` and `edi`. The trade partner is closed by `CGTradeInfo::SetTradePartner` with a zero GUID, the item text, gossip and trainer windows by setting their object to zero, and the quest frame by reporting the quest declined. The loot close `0x4c3f40` is a lone `ret`, and `LeaveWorldUI` reaches it through `0x4c3e80`, which loads 1 into `cl` and jumps there; a stable master is closed twice, as `ui/stable` describes.

## Frame factories

Five of the eight game frame types registered by `CGGameUI::RegisterFrameFactories` (`ui/framexml`) have a factory of this shape: allocate the frame's bytes with `SMemAlloc` under its header file and line, then construct it with the parent: GameTooltip 0x460 bytes, Minimap 0x344, PlayerModel 0x3f8, DressUpModel 0x410 and TabardModel 0x410.

| Factory | Type | Bytes | Constructor |
|---|---|---|---|
| `0x495c60` | GameTooltip | 0x460 | `0x529240` |
| `0x495a00` | Minimap | 0x344 | `0x4edbc0` |
| `0x495bd0` | PlayerModel | 0x3f8 | `0x505680` |
| `0x495c00` | DressUpModel | 0x410 | `0x5041d0` |
| `0x495c30` | TabardModel | 0x410 | `0x502780` |

Addresses: `0x495c60`, `0x495a00`, `0x495bd0`, `0x495c00`, `0x495c30`, `0x495940`
Confidence: verified
Proof: one
Test: none

A factory is `push esi; push 0; push line; push header; push size; mov esi,ecx; call SMemAlloc; test eax; je null; push esi; mov ecx,eax; call constructor`. It returns 0 when the allocation fails. The method tables of these types are the ones `ui/scriptapi` unregisters.

## Cursor and camera

`GetCursorPosition()` (`0x48b820`) pushes two numbers from the top frame's dwords x and y at +0x1118 and +0x111c: `768·a·x` and `768·y`, with `a` the screen aspect. `0x48ec90(guid)` sets the world frame's camera target to the object of a GUID, the active player's when the GUID is zero; `0x4841a0` is the camera call. `0x4913a0` drains the world's fading models when a world frame exists, through the jump `0x483e10`.

Addresses: `0x48b820`, `0x41ad80`, `0x41ad70`, `0x41ae40`, `0x48ec90`, `0x4841a0`, `0x4913a0`, `0x483e10`, `0x672c40`
Confidence: verified
Proof: one
Test: none

`0x48b820` is the `ScaleScreenDelta` of the two dwords (x by `K1`, y by `K2`, each stored as a float; `K1 = a·K2` and `K2 = 1/√(a²+1)`, `0x41ad10`), then for each axis `GetScaledScreenAspect` (`0.75a`) times 1024.0 (`0x7ffd68`) times the scaled dword, rounded to a float and divided by `K1` (`DivideByScreenScaleX`, `0x41ae40`) for both axes. `0x4913a0` is reached from the console video verb (`ui/videoverbs`).

## Interface state helpers

Five small functions of `GameUI.cpp` that set or read interface state:

| Function | Does |
|---|---|
| `0x4920d0` | Stores a corpse GUID, written by the corpse object |
| `0x492090` | Copies the corpse's display map, position and map id to its arguments |
| `0x4946d0` | Passes a swing record to `SignalUnitCombat` with the event name of its kind |
| `0x496510` | Frees the ten survey answers and the comment and zeroes the answer ranks |
| `0x495ab0` | Answers 1 for the LootButton, Button, Frame and Region type ids |

Addresses: `0x4920d0`, `0x492090`, `0x4946d0`, `0x496510`, `0x495ab0`, `0x494600`
Confidence: verified
Proof: one
Test: none

`0x4920d0` is called by `CGCorpse_C::OnCreate`, `Deactivate` and `Reactivate`; `0x492090` by `ProjectCorpsePosition`. `0x4946d0` reads the GUID at +8, the values at +0x10, +0x14 and +0x18 and the kind at +0xa4, which indexes the name table at 0x83de28. `0x496510` frees the answers with the line numbers of `GameUI.cpp` and is called at start-up, shutdown and by `SendSurvey`.
