# ui: the lottery and minigame functions

The Lua functions of the lottery window (`0x4c4140` to `0x4c4720`) and of the minigame window (`0x4c4d20` to `0x4c4e04`). No server of 1.12 uses
either; the client keeps the state of each in globals that its message handlers fill. The minigame object itself is its own system.

Roots: `0x4c3d20`, `0x4c3d50`, `0x4c3e60`, `0x4c4210`, `0x4c4220`, `0x4c4290`, `0x4c43d0`, `0x4c4400`, `0x4c44b0`, `0x4c44e0`, `0x4c45d0`, `0x4c45f0`, `0x4c4630`, `0x4c4660`, `0x4c4690`, `0x4c4d50`, `0x4c4d80`, `0x4c4df0`

## Buying

`GetLastLotteryNumbers()` sends opcode 0x334 with no body. `BuyRandomPicks(n)` sends opcode 0x336 with `n` and 0 for `n` from 1 to 10, and
raises `Invalid number of randompicks` otherwise; its argument is not checked to be a number. `SubmitNumbers(a, b, c, d)` converts four numbers, all of
which must be from 1 to 31 and distinct, into a mask of one bit each (bit `n - 1`) and sends opcode 0x336 with 0 and the mask. With an invalid or repeated number it
sends nothing and returns 1 without pushing a value, so Lua receives its fourth argument. `CloseLottery()` does nothing. The window's open, `0x4c3eb0`, sends opcode 0x334 and has no caller (`ui/loot#no-walk-away-leash`).

Addresses: `0x4c4220`, `0x4c4210`, `0x4c3eb0`, `0xb71c38`, `0x4c4256`, `0x4c4290`, `0x4c42f8`, `0x4c4301`, `0x4c4310`, `0x4c431b`, `0x4c4332`, `0x4c4334`, `0x4c4342`, `0x4c433e`, `0x4c4365`, `0x4c4373`, `0x4c4400`, `0x4c441b`, `0x4c4420`, `0x4c4427`, `0x4c4495`
Confidence: verified
Proof: one
Test: none

`0x4c42f8` loads each pick, `0x4c42fc` and `0x4c4301` bound it to 1 to 31, `0x4c4310` scans the earlier picks and `0x4c431b` to `0x4c4323` set the bit. The invalid return has two exits: `0x4c4342`, from a pick out of range or after a duplicate flagged by an earlier pick, and `0x4c4332`, from a duplicate in the last pick; both end in `mov eax,1`. `0x4c441b` and `0x4c4420` bound the random count.
`0x4c4365` pushes the opcode 0x336, `0x4c4378` the zero and `0x4c4385` the mask; `0x4c4427`, `0x4c444e` and `0x4c4457` push the opcode, the count and the zero. `0x4c3eb0` (`ret 8`) passes the kiosk GUID of its two arguments to `SetInteractionTarget` (`0x4930d0`) with the range read from `0xb71c38` (`0x4c3eb6`), stores the GUID in two globals, writes the opcode into a fresh `CDataStore` with `CDataStore::PutUInt32` (`0x4c3efb`) and sends it with `ClientServices::Send`; no `call`, `jmp` or stored address anywhere in the image reaches it, and the same two scans find the call of `0x4dd2f0` (`0x4db7b7`) as the control.

## Query result

The handler of opcode 0x335 (`0x4c3d50`) reads 32-bit words: the vendor value (`0xb71df0`), then the vendor delay (`0xb71bd0`),
whose zero ends the read. Otherwise the vendor time becomes now plus the delay (`0xb71bd4`), the draw delay (`0xb71bd8`) plus now the next
draw time (`0xb71bdc`), two words go to `0xb71be0` and `0xb71be4`, unread, then four slots of a money word
(`0xb71be8`, the fourth the jackpot `0xb71bf4`), a prize item id (`0xb71bf8`) and a display id (`0xb71c08`), then four draw records of
0x18 bytes from `0xb71c48`: four numbers into `+0x8` to `+0x14`, an age into `+0x0`, and `+0x4` set to now minus the age. Every path signals `LOTTERY_SHOW` (543). The 0x337 handler `0x4c3e60` returns 0. Now is `0x429580`, in seconds.

Addresses: `0x4c3d50`, `0x4c3d7b`, `0x4c3d9c`, `0x4c3db8`, `0x4c3dce`, `0x4c3dfc`, `0x4c3e33`, `0x4c3e48`, `0x4c3e60`, `0x4c3d20`, `0xb71df0`, `0xb71bd0`, `0xb71bd4`, `0xb71bd8`, `0xb71bdc`, `0xb71be0`, `0xb71be4`, `0xb71be8`, `0xb71bf4`, `0xb71bf8`, `0xb71c08`, `0xb71c48`
Confidence: verified
Proof: one
Test: none

`0x4c3d20` registers `0x4c3d50` for 0x335 and `0x4c3e60` (`xor eax,eax; ret 8`) for 0x337. Each field is one
`CDataStore::GetInt32`. `0x4c3d7b` tests the delay and jumps to the signal at `0x4c3e43`. The slot loop at `0x4c3dce` reads
`[edi-0x10]`, `[edi]` and `[edi+0x10]` from `edi` = `0xb71bf8`; the record loop at `0x4c3dfc` reads `edi+8` to `edi+0x14`, then
`[edi]`, and `0x4c3e33` stores `now - [edi]` at `[edi+4]`. `0x4c3e48` signals event 0x21f. The image holds the addresses
`0xb71be0` and `0xb71be4` once each, in the handler's pushes, against four for `0xb71bf8` as the control.

## Prizes

`IsVendorActive()`, `GetNumLotteryPrizes()`, `GetJackpotAmount()` and `GetMoneyPrizes()` push their numbers and return 0, so Lua receives
nothing. They push the vendor value once now passes the vendor time, else 0; the count of nonzero prize ids; the jackpot;
the three money words. `GetLotteryPrizeInfo(i)` returns the item's name, icon path, the
count 1, the quality, and 1 when the player can use it, else nil. It picks the last slot at which the running count of nonzero ids equals
`i - 1`, so with all four slots set `i` from 2 to 5 gives slots 0 to 3 and 1 fails. A non-number, `i - 1` above 4 unsigned or no slot raises `Invalid prize item in GetPrizeItemInfo("type", index)`.

Addresses: `0x4c43d0`, `0x4c43ea`, `0x4c43ef`, `0x4c4140`, `0x4c44b0`, `0x4c44ce`, `0x4c4160`, `0x4c45d0`, `0x4c45e1`, `0x4c45f0`, `0x4c462a`, `0x4c44e0`, `0x4c452e`, `0x4c4535`, `0x4c459a`, `0x4c45b3`, `0x4c3f50`, `0x4c3f96`, `0x4c3fb0`, `0x4c3fbf`, `0x4c4019`, `0x4c4024`, `0x4c403d`, `0x4c405a`, `0x4c40b8`
Confidence: verified
Proof: one
Test: none

Each of the four getters calls `lua_pushnumber` and ends `xor eax,eax; ret` (`0x4c43ef`, `0x4c44ce`, `0x4c45e1`, `0x4c462a`).
`0x4c4140` returns `0xb71df0` when it is zero or `[0xb71bd4]` is below now (`jb`), else 0. `0x4c452e` passes `i - 1` to `0x4c3f50`,
which fails above 4 (`cmp esi,4; jbe` at `0x4c3f96`) and at `0x4c3fbf` keeps the slot each time the count equals it. It sets the name
empty, the count to 1, the quality to 0 and usable to 1; with the cached item record of the id (`0x55ba30`) it copies the name at `+0x08` (`0x4c4019`) and takes the quality
at `+0x1c` when `+0x2c` is nonzero, else −1 (`0x4c4024`). The icon is `%s%s%s` of the string `0x53f810` returns for 3 (`0x4c403d`), `\`
when that is not empty, and the inventory icon of the slot's display id (`0x4c405a`). `0x4c40b8` clears usable when the active
player's `CGPlayer_C::CanUseItem` fails. `0x4c459a` returns 5; `0x4c45b3` raises the error.

## Draws

`GetNextDrawTime()` pushes the next draw time minus now and `GetNumPastDrawResults()` the count of records whose first number is
nonzero; both return 0, so Lua receives nothing. `GetPastDrawResult(i)` returns five values: the record's four numbers and its age
in seconds, now minus `+0x4`. It picks the first record at which the running count of used records equals `i - 1`, so `i` from 2 to
5 gives the used records in order and 1 gives the first record only when that record is empty. No such record raises
`Invalid index in GetPastDrawResults("type", index)`.

Addresses: `0x4c4630`, `0x4c4657`, `0x4c4660`, `0x4c467e`, `0x4c4180`, `0x4c4690`, `0x4c46ab`, `0x4c46b2`, `0x4c470b`, `0x4c4715`, `0x4c41a0`, `0x4c41b6`, `0x4c41e0`, `0x4c41f3`, `0x429580`
Confidence: verified
Proof: one
Test: none

`0x4c4630` subtracts now from `[0xb71bdc]` and `0x4c4180` counts the nonzero words at `+0x8` of the records (`0xb71c50`, step
0x18); both getters end `xor eax,eax; ret` (`0x4c4657`, `0x4c467e`). `0x4c46ab` decrements the index into `ecx` for `0x4c41a0`,
which counts used records and stops at the first `cmp edi,ecx; je` (`0x4c41b6`), copies the four numbers (`0x4c41e0`) and
writes now minus `[record+4]` (`0x4c41f3`). `0x4c470b` returns 5 after five `lua_pushnumber`; `0x4c4715` raises the error.

## The minigame

`GetMinigameType()` returns one value, always nil: it pushes nil when there is no game object or its kind is not 1, and for kind 1 the
null slot 1 of the name table `0x848384` (`minigame#lua-queries`), which `lua_pushstring` turns into nil. `MakeMinigameMove(moveType, param)` needs two numbers
(`Usage: MakeMinigameMove(moveType, param)`), passes `moveType - 1` and `param - 1` to the game object, which sends opcode 0x2f8 with the game GUID, the move type byte and the
parameter when the object accepts the move, and returns nothing. `GetMinigameState()` calls the object's virtual at `+8` with the Lua state and returns its result count, nothing with no game.

Addresses: `0x4c4d50`, `0x4c4d57`, `0x4c4d5c`, `0x4c4d5e`, `0x4c4d70`, `0x4c4d80`, `0x4c4dae`, `0x4c4dd1`, `0x4c4dd7`, `0x4c4ddb`, `0x4c4df0`, `0x4c4dfe`, `0x4c4d20`, `0x4c4d40`, `0x6f3890`, `0x728650`, `0x72866b`, `0x728674`, `0x7286b2`
Confidence: verified
Proof: one
Test: none

`0x4c4d40` returns the kind at `+8` of the game object (`0xb71e5c`), 0 without one. `0x4c4d57` (`jle`) and `0x4c4d5c` (`cmp eax,2; jge`) send every kind but 1 to
`lua_pushnil` (`0x4c4d70`); `0x4c4d5e` loads `[eax*4+0x848384]`, the dword 0 at `0x848388`, and `0x6f3890` jumps to `lua_pushnil` for a null pointer. Both paths return 1.
`0x4c4dae` subtracts 1 from the move type (`fsub` of the double 1.0) and `0x4c4dcc` decrements the parameter; `0x4c4dd1` calls `0x4c4d20`, which calls `0x728650`, and `0x4c4dd7` returns 0.
`0x72866b` tests the object's virtual `+4`, `0x728674` pushes the opcode 0x2f8 and `0x7286b2` writes the move type as a signed byte. `0x4c4dfe` pushes the Lua state and calls `[vtable+8]`, whose `eax` is the function's return.
