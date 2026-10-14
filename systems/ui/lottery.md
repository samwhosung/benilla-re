# ui: the lottery and minigame functions

The Lua functions of the lottery window (`0x4c4140` to `0x4c4720`) and of the minigame window (`0x4c4d20` to `0x4c4e04`). No server of 1.12 uses
either; the client keeps the state of each in globals that its message handlers fill. The minigame object itself is its own system.

Roots: `0x4c4220`, `0x4c4290`, `0x4c4400`, `0x4c44e0`, `0x4c4690`, `0x4c4d50`, `0x4c4d80`

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

## Prizes

The window keeps up to four prize item ids (from 0xb71bf8), the jackpot (0xb71bf4) and three money prizes (0xb71be8 to 0xb71bf0).
`GetNumLotteryPrizes()` counts the nonzero prize ids, `GetJackpotAmount()` returns the jackpot and `GetMoneyPrizes()` the three money words.
`GetLotteryPrizeInfo(i)` returns five values for prize `i`: the item name, the icon path, the count, the quality and 1 when usable, and raises
`Invalid prize item in GetPrizeItemInfo("type", index)` for a prize with no data.

Addresses: `0x4c44b0`, `0x4c4160`, `0x4c44e0`, `0x4c4535`, `0x4c45d0`, `0x4c45f0`, `0x4c3f50`, `0x4c4140`, `0x4c43d0`
Confidence: inferred
Proof: none
Test: none

`0x4c4160` counts the nonzero entries of the prize array. `0x4c4535` fetches a prize through `0x4c3f50`, which reads the n-th nonzero id. `IsVendorActive()` returns the dword at 0xb71df0 once the clock has passed 0xb71bd4, else 0 (`0x4c4140`).

## Draws

The window keeps the time of the next draw (0xb71bdc) and four past draw records of 0x18 bytes from 0xb71c50. `GetNextDrawTime()` returns the seconds
from now to the next draw. `GetNumPastDrawResults()` counts the used records. `GetPastDrawResult(i)` returns the four numbers of the `i`-th used record and the seconds since it, and
raises `Invalid index in GetPastDrawResults("type", index)` past the records.

Addresses: `0x4c4630`, `0x4c4660`, `0x4c4180`, `0x4c4690`, `0x4c46b2`, `0x4c4715`, `0x4c41a0`, `0x4c41e0`, `0x4c41ee`, `0x429580`
Confidence: inferred
Proof: none
Test: none

`0x4c4180` counts the nonzero first words of the records. `0x4c41a0` copies the four numbers of the n-th used record and `0x4c41ee` subtracts the record's time stamp, stored just before it, from the clock.

## The minigame

`GetMinigameType()` returns `TicTacToe` for the active game object when its kind is 1 and nil otherwise. `MakeMinigameMove(moveType, param)` needs two numbers
(`Usage: MakeMinigameMove(moveType, param)`) and passes `moveType - 1` and `param - 1` to the game object, which sends opcode 0x2f8 with the game GUID, the move type byte and the
parameter when the object accepts the move. `GetMinigameState()` calls the object's virtual at `+8` and returns what it leaves in `eax`, 0 with no game.

Addresses: `0x4c4d50`, `0x4c4d5e`, `0x4c4d80`, `0x4c4dae`, `0x4c4dd1`, `0x4c4ddb`, `0x4c4df0`, `0x4c4dfe`, `0x4c4d20`, `0x4c4d40`, `0x728650`, `0x72866b`, `0x728674`, `0x7286b2`
Confidence: verified
Proof: one
Test: none

`0x4c4d40` returns the kind at `+8` of the game object (`0xb71e5c`). `0x4c4d5e` reads the name table at 0x848384. `0x4c4dae` subtracts 1 from the move type (`fsub` of the double 1.0) and `0x4c4dcc` decrements the parameter; `0x4c4dd1` calls `0x4c4d20`, which calls `0x728650`. `0x72866b` tests the object's virtual `+4`, `0x728674` pushes the opcode 0x2f8 and `0x7286b2` writes the move type as a signed byte.
