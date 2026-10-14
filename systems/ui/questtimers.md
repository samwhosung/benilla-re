# ui: quest timers

The clock timed quests count against, how the client learns the server's time, and the three Lua
functions that report a quest's time left: `GetQuestTimers`, `GetQuestIndexForTimer` and
`GetQuestLogTimeLeft`. The quest slots and the list they index are `questlog`.

The code is `QuestLog.cpp`: the time handler `0x4de400`, the clock setter `0x4deef0`, the world-enter
set-up `0x4de430`, the latch pair `0x4df480` and `0x4df4f0`, and the three bindings `0x4e13d0`, `0x4e15f0`
and `0x4e08a0`.

Roots: `0x4de400`, `0x4de430`, `0x4deef0`, `0x4e13d0`, `0x4e15f0`, `0x4e08a0`, `0x4df480`, `0x4df4f0`

## The clock

A timed quest's third slot dword is an expiry in the server's epoch seconds. The time left is
`timer + G - now - 1`, with `now` the local machine's wall clock in whole seconds (a clock call cached for
500 ms) and `G` the offset stored by the last time sync, local minus server. So the epoch is the server's and
the advance between syncs is the player's own clock. An expiry `N` seconds ahead reads `N - 1`, the last
second 0.

Addresses: `0xbb7494`, `0x429580`, `0x4deef0`, `0x4deef3`, `0x4deef8`, `0x4deefa`, `0x4def02`, `0x4def0c`, `0x4def11`, `0x4def16`, `0x4de3d3`, `0x4de5f0`, `0x4e08f6`, `0x4e1424`, `0x4e166c`, `0x4e0905`, `0x4e14b9`, `0x4e16c9`, `0x4de6b6`
Confidence: verified
Proof: one
Test: none

`0x429580` reads `GetTickCount` and, when the cached sample is at least `0x1f4` ms old, the C runtime's
`time` equivalent (`GetLocalTime`, `GetSystemTime`, `GetTimeZoneInformation`). The setter `0x4deef0(serverTime)`
computes `now - serverTime` (`0x4deef8`), stores it at `0xbb7494` (`0x4deefa`) and stores 1 instead when the
difference is 0, so 0 means never synced (zeroed at `0x4de3d3`). The same formula is used at `0x4e0905`,
`0x4e14b9`, `0x4e16c9` and in the rebuild (`0x4de6b6`). Seven instructions reference `0xbb7494`.

## The time sync

The client sends `CMSG_QUERY_TIME` (opcode 0x1ce, no body) once for each world entry, from `0x4de430`, and again from
a list rebuild once its deadline passes. The answer is `SMSG_QUERY_TIME_RESPONSE` (opcode 0x1cf), whose
handler `0x4de400` reads one `u32` and passes it to the setter, which also sets the next deadline to an hour
ahead and fires `QUEST_LOG_UPDATE`. That handler is the only caller of the setter, and `G` has no other writer.

Addresses: `0x4de430`, `0x4de45b`, `0x4de47d`, `0x4de488`, `0x4de390`, `0x4de3e0`, `0x4de3e5`, `0x4de3ef`, `0x4de400`, `0x4de40a`, `0x4de412`, `0x4de81b`, `0x4de822`, `0x4de82b`, `0x4de836`, `0x4de85f`, `0x4de867`, `0xbb749c`, `0x4def1b`, `0x4def20`, `0x4909a1`
Confidence: verified
Proof: one
Test: none

The set-up `0x4de430` pushes 0x1ce (`0x4de45b`), builds the packet with `0x418190` and sends it with `0x5ab630`; its
only caller is `0x4909a1` in the world-enter cascade. The rebuild's resync tests the deadline `0xbb749c`
(`0x4de81b`), skips while it is 0, compares it with the clock (`0x4de82b`), sends the request (`0x4de85f`) and
clears the deadline (`0x4de867`); the response re-arms it by adding 0xe10 (`0x4def11`, `0x4def16`). The handler is
registered by `0x4de390` (`mov ecx,0x1cf`, `0x4de3e5`). A resync is only tried from a rebuild.

## GetQuestTimers

`GetQuestTimers()` returns one number for each timed quest that is still running, in list order, and no
value when there are none. A quest is skipped when its row is a header, its slot is empty, its timer is
0, its state byte has bit 1 (failed) or its expiry latch is set. A negative time is not returned: for a row whose quest cannot yet be turned in, it sets the latch, calls `0x5eb070` with the player and,
when the quest is cached, prints the quest-failed line (message 0x8b with the quest's title); a quest that
can be turned in is left unlatched. A time of 0 is returned. The function fires `QUEST_LOG_UPDATE` when it ends.

Addresses: `0x4e13d0`, `0x4e13f7`, `0x4e1417`, `0x4e1433`, `0x4e1439`, `0x4e1460`, `0x4e1475`, `0x4e148a`, `0x4e1493`, `0x4e149f`, `0x4e14a7`, `0x4e14c5`, `0x4e14cd`, `0x4e14d8`, `0x4e14dd`, `0x4e14e8`, `0x4e14ef`, `0x4e1515`, `0x4e1524`, `0x4e1554`, `0x4e155e`, `0x4df480`, `0x4df4f0`, `0x4df50d`, `0x5eb070`, `0x4df580`, `0x496720`
Confidence: verified
Proof: one
Test: none

The skips are the compares in that order (`0x4e1460` to `0x4e14a7`); `js` at `0x4e14c5` takes the
expired branch, which calls `0x4df4f0(row)`: unless the turn-in test `0x4df580` passes it stores 1 at `[row+0xc]` (`0x4df50d`), calls `0x5eb070` and returns 1, and on a true result the function looks the
quest up with `0x562a40` (callback `0x4e1580`) and calls `0x496720` with 0x8b. The count is the accumulator
initialised at `0x4e13f7`. The value is an exact integer pushed with `fild` and `lua_pushnumber` (`0x4e14dd`,
`0x4e14e8`). Neither this function nor `GetQuestIndexForTimer` tests `G`, so before the first response every timed
quest reads as expired.

## GetQuestIndexForTimer

`GetQuestIndexForTimer(n)` returns the 1-based list index of the n-th quest that `GetQuestTimers` would
report, or nil. It applies the same skips except the expiry latch, counting survivors, and its index is
the space `GetQuestLogTitle` and `SelectQuestLogEntry` take.

Addresses: `0x4e15f0`, `0x4e1603`, `0x4e1612`, `0x4e162a`, `0x4e162f`, `0x4e1634`, `0x4e16c2`, `0x4e16d7`, `0x4e16df`, `0x4e170b`, `0x4e1716`, `0x4e171f`, `0x4e16f7`, `0x4e16fe`
Confidence: verified
Proof: one
Test: none

A non-number raises `Usage: GetQuestIndexForTimer(index)` (`0x4e1612`). The argument is truncated and decremented
(`0x4e1634`); a match increments the 0-based array position (`0x4e170b`) and pushes it; no match pushes nil
(`0x4e16f7`). The latch is never read, but it is set only for a negative value, so the two lists agree.

## GetQuestLogTimeLeft

`GetQuestLogTimeLeft()` has no argument: it reports the selected quest's slot, slot 0 when no quest is
selected. It returns one value, nil for no player, an out-of-range or empty slot, an untimed quest, a failed
quest or an unsynced clock, else the time left, clamped to 0 and not suppressed.

Addresses: `0x4e08a0`, `0x4e08c8`, `0x4e08ca`, `0x4e08d1`, `0x4e08e7`, `0x4e08ee`, `0x4e08f4`, `0x4e08fc`, `0x4e08fe`, `0x4e0900`, `0x4e090f`, `0x4e0918`, `0x4e093a`, `0x4e094d`, `0x4df050`, `0x4df063`, `0x4df010`
Confidence: verified
Proof: one
Test: none

`0x4df050` returns the selected row's slot or 0 when the scan finds nothing (`0x4df063 xor eax,eax`). The
sign is tested from one clock sample (`0x4e0900`) and the pushed value computed from a second (`0x4e0918`).
`0xbb7494 == 0` returns nil at `0x4e08fe`.
