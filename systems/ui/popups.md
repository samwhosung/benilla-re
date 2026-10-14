# ui: session and dialog verbs

The Lua verbs behind the stock confirmation dialogs: logging out and quitting, the instance boot countdown, the area spirit
healer, the pet unlearn, the trade enchant clash and the two trade bindings. Each is a thin function of the main
binding table that fixes an argument count, a return count and a packet; the state behind them is in the docs named
below. The battleground accept and the meeting stone cancel are `battlefield` and `meetingstone`; the summon dialog is `summon`.

Roots: `0x489390`, `0x48ab50`, `0x48ab60`, `0x48b620`, `0x48df20`, `0x48dca0`, `0x48d330`

## Logout and quit

Five verbs read no argument and return no value. `Logout()` calls `0x5ab000(0, 0)` and `Quit()` calls `0x5ab000(1, 0)` on the session
object: both send opcode 0x4b, and `Logout` also sets the pending latch at `+0x1b1d`. `ForceLogout()` calls
`0x5aaff0`, which runs `0x5ab000(0, 1)`: opcode 0x4a with no body, no latch, and no refusal for a pending latch.
`CancelLogout()` calls `0x5aaf80`, which sends opcode 0x4e with no body and clears the latch. `ForceQuit()` sends nothing: it
calls `0x401560(0)`, which stores the quit code 0 and posts the close event of the current context.

Addresses: `0x489390`, `0x4893b0`, `0x48ab40`, `0x48ab50`, `0x48ab60`, `0x5aaff0`, `0x5aaf80`, `0x5ab000`, `0x5ab00d`, `0x5ab020`, `0x5ab030`, `0x5ab053`, `0x5ab062`, `0x5aaef0`, `0x5aaf04`, `0x5ab3a0`, `0x401560`, `0x41f9a0`
Confidence: verified
Proof: one
Test: none

`0x5ab000(a, force)` returns without sending when the latch is set and `force` is 0 (`0x5ab00d`), and when the connection
word `+0x1b18` is zero (`0x5ab020`); when the session is not the current one it leaves the world locally (`0x5aaf40`). Else it stores `a`
at `+0x1b1c`, sets the latch when `force` is 0, and sends opcode `0x4b - (force != 0)` (`neg bl; sbb ebx,ebx; add ebx,0x4b`).
The reply handler `0x5aaef0` signals `PLAYER_CAMPING` for a stored `a` of 0 and `PLAYER_QUITING` for a nonzero one
(`0x114 + (a != 0)`), so a forced logout is a camp. The console command `logout` (`0x5ab3a0`) runs the forced path when `+0x1b18` is nonzero.

## The instance boot countdown

Opcode 0x286, an arm of the player-services dispatcher: `i32 delay`, `u32 reason`. A delay above 0 stores
`now + delay` in `0xb4e34c` and signals `INSTANCE_BOOT_START`; a delay of 0 or less clears it and signals `INSTANCE_BOOT_STOP`
with no argument. Only a delay of exactly 0 first shows a message, `ERR_RAID_GROUP_ONLY` for reason 1 or `ERR_RAID_GROUP_FULL` for 2.
`GetInstanceBootTimeRemaining()` pushes one number: whole seconds left (deadline minus now, truncated), 0 when nothing is pending.

Addresses: `0x5e48f1`, `0x5e4901`, `0x5e4914`, `0x5e4928`, `0x5e4921`, `0x496340`, `0x49634e`, `0x496353`, `0x49635e`, `0x496368`, `0x496380`, `0x48b620`, `0x48b629`, `0x48b630`, `0x48b635`, `0x5e4938`, `0xb4e34c`
Confidence: verified
Proof: one
Test: none

`0x496340(ms)` is `jle` on the signed delay: it signals through a tail jump, so a call-only census misses it. `0x496380` returns
`deadline - now` when positive, else 0. `0x48b630` divides by 1000 with the unsigned multiply by `0x10624dd3` and `shr 6`. The cell is
cleared only by the delay-0 message and by the reset block of `0x48fbf0` (`0x490144`); once the deadline passes it keeps its value
and the getter returns 0. The ids 394 and 395 of `tables/messages.tsv` are the two messages, not events.

## The area spirit healer

`AcceptAreaSpiritHeal()` calls `0x492310`: it refreshes the proximity scan `0x4923b0` and, when a healer GUID is cached in `0xb4e330`,
sends opcode 0x2e3 with that GUID; with none cached it sends nothing. `CancelAreaSpiritHeal()` calls `0x4923a0`, which runs
the aura cancel `0x6e7040(0xa18)`: that signals `AREA_SPIRIT_HEALER_OUT_OF_RANGE` for spell 2584 and sends opcode 0x136 with
the spell id. `GetAreaSpiritHealerTime()` pushes the whole seconds left of `0xb4e338` (0 when unset). Opcode 0x2e4,
`u64 guid, u32 ms`, runs `0x492290(ms)`: if the GUID is the cached healer and `ms` is positive it stores `now + ms`
(1 if that is 0) and signals `AREA_SPIRIT_HEALER_IN_RANGE`.

Addresses: `0x492310`, `0x492316`, `0x49232b`, `0x4923a0`, `0x6e7040`, `0x4921c0`, `0x4921fc`, `0x492211`, `0x492219`, `0x492290`, `0x4922b4`, `0x4922bb`, `0x4922d1`, `0x4922e0`, `0x48dee0`, `0x48df20`, `0x48df30`, `0xb4e330`, `0xb4e338`
Confidence: verified
Proof: one
Test: none

Adopting a new healer GUID (`0x4921c0`) first cancels spell 0xa18 when a deadline was pending, so walking away fires the out-of-range
event without a Lua call, then zeroes `0xb4e338` and, for a nonzero GUID, sends opcode 0x2e2 with it. The getter divides with the
signed multiply by `0x10624dd3`, `sar 6` and a sign fix. The accept and cancel verbs push nothing.

## Pet unlearn

`CheckPetUntrainerDist()` pushes 1 when the active player is the one at `0x5dfd30` and the remembered pet trainer in `0xc4d7b0`
is within interaction range, else nil, never 0. `ConfirmPetUnlearn()` pushes nothing: with a player it calls
`PetUnlearnDialog(0, 0)`, which sends opcode 0x2f0 with the remembered trainer GUID unless the cost in `0xc4d7b8` exceeds the money
(`ERR_NOT_ENOUGH_MONEY`) or the trainer is out of range. Opcode 0x2f1, `u64 trainerGuid, u32 cost`, calls
`PetUnlearnDialog(&guid, cost)`: it remembers both and signals `CONFIRM_PET_UNLEARN` with the cost in copper; a zero GUID shows
`ERR_TALENT_WIPE_ERROR`.

Addresses: `0x48d1c0`, `0x48d1e4`, `0x48dca0`, `0x48dce7`, `0x5dfd30`, `0x5dfd99`, `0x5dfdaa`, `0x5dfba0`, `0x5dfc2d`, `0x5dfc41`, `0x5dfc5a`, `0x5dfc5f`, `0x5dfc99`, `0x5dfcbc`, `0x5e4a26`, `0x5e4a3a`, `0x5e4a45`, `0x5e4a70`, `0x5e4a85`, `0xc4d7b0`, `0xc4d7b8`, `0xc4c28c`
Confidence: verified
Proof: one
Test: none

The range test of the dialog (`0x5dfc2d`, `je`) passes a distance equal to the limit and a NaN, while the Lua helper (`0x5dfdaa`, `jp`)
fails a NaN. The limit `0xc4c28c` is the squared interaction distance (`objectlayer/interaction#confirmation-dialogs`). The latch words
have three writers, all in the arrival arm (`0x5dfc41`, `0x5dfc54`, `0x5dfc5a`); nothing clears them, and nothing clears the binder's `0xc4d7c0` pair
either.

## The trade enchant clash

`ReplaceTradeEnchant()` calls `0x6e6900` and pushes nothing. `ClickTargetTradeButton(index)` raises
`Usage: ClickTargetTradeButton(index)` for a non-number, hands a held money cursor to `TradeAddMoney`, and for index 7 calls `0x496170`: with an
enchant already in the slot and the pending spell an enchant (effect 0x35) it signals `TRADE_REPLACE_ENCHANT` with two enchantment
names, else it applies the enchant through `0x6e6900` at once. `BeginTrade()` and `CancelTrade()` call `Trade_C_BeginTrade` and `TradeCancel`, which send opcodes
0x117 and 0x11c with no body.

Addresses: `0x48d330`, `0x6e6900`, `0x4c0080`, `0x4c00a3`, `0x4c00ad`, `0x4c00d7`, `0x4c00dd`, `0x496170`, `0x496180`, `0x4961c0`, `0x496211`, `0x496216`, `0x49622b`, `0x496230`, `0x496235`, `0xb7150c`, `0xb715d8`, `0x48aa60`, `0x48aa70`, `0x5d40e0`, `0x5d4310`
Confidence: verified
Proof: one
Test: none

The event call is `FrameScript_SignalEvent2(0x194, "%s%s%d", name, name)` with two pushed values (`add esp,0x10`): the third value
is the caller's saved `edi`, unset by the event. Both names are `SpellItemEnchantment` `Name_lang[locale]` (`+0x34 + 4*locale`):
the first is the enchant already on the item (`0xb715d8`, element 6 of a seven-element array cleared by `rep stosd` at
`0x4bf174`), the second the spell's `EffectMiscValue`. `0x496170` returns silently when the trade slot `0xb7150c` is empty,
when the cast spell id is out of range or has no row, or when no effect slot is an enchant with both names resolved. The
generated send of `0x6e6900` is the ordinary cast path (`spell/targeting`).
