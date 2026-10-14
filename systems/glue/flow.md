# glue: the screen flow

The login screens' state machine: the screen state, its per-frame update and the verbs that move it.

`CGlueMgr.cpp` keeps the flow in statics, chief among them the screen state `0xb41da0`: 0 when idle,
otherwise the operation in flight, from the logon (1) to the world entry (8). The verbs the glue
scripts call set it and start a session operation (`net/session`); `CGlueMgr::Idle` (`0x46b930`), a
handler of event category 5, runs every frame, polls the session's status and acts by state. Its
results reach the GlueXML only as glue events: the status dialog's open (3), update (4) and close
(5), the screen (0) and a few others. `OPEN_STATUS_DIALOG` carries the button kind (`OKAY`,
`CANCEL`, `CONNECTION_HELP`, `OKAY_WITH_URL`) and then the text; `UPDATE_STATUS_DIALOG` the text and
then an optional button. The dialog's one button calls `StatusDialogClick`, a jump on the state.
States 10 and 11, the downloads, are `downloads`.

Roots: `0x46ae50`, `0x46aef0`, `0x46af20`, `0x46af50`, `0x46af80`, `0x46afb0`, `0x46b0c0`, `0x46b0f0`, `0x46b210`, `0x46b290`, `0x46b2e0`, `0x46b340`, `0x46b440`, `0x46b500`, `0x46b720`, `0x46b7e0`, `0x46b810`, `0x46b840`, `0x46b860`, `0x46b870`, `0x46b880`, `0x46b8d0`, `0x46b910`, `0x46b930`, `0x46c520`, `0x46c540`

## Screen states

`CGlueMgr::Resume` (`0x46a7b0`) and every terminal arm set the state to 0. The table gives each
value's setters, frame arm and status dialog button (`0x46b720`). "Cancel" is `FailOperation(2)`
(`RESPONSE_CANCELLED`) on the session, acted on next frame. Above 11 the button does nothing.

| state | set by | each frame | button |
|---|---|---|---|
| 0 | `0x46a7b0`, terminal arms | a pending glue reload | `RunCleanup` on the session |
| 1 logon | `0x46afb0`, `0x473ef9` | logon arm | cancels the logon, state 0 |
| 2 world connect | `0x46b1f0`, `0x46b210`, `0x46c540` | world connect arms | cancel, roster teardown `0x472580` |
| 3 character list | `0x46b880` | request arms | as 2 |
| 4 realm list | `0x46b8d0` | request arms | cancel, state 0 |
| 5 create | `0x46b290` | request arms | as 4 |
| 6 delete | `0x46b2e0` | request arms | as 4 |
| 7 rename | `0x46b340` | nothing | state 0 |
| 8 enter world | `0x46b500` | two-phase world entry | as 4 |
| 9 refused login | `0x401ee0` | `OKAY` with the status text, every frame | state 0, then `0x46b880` |
| 10 download | `0x474610`, `0x473c30` | `downloads` | with a survey open, `0x473ce0` and state 0; else nothing |
| 11 survey | `0x473d70` | `downloads` | as 10 |

Addresses: `0xb41da0`, `0x46b720`, `0x46b72a`, `0x46b7a8`, `0x402075`
Confidence: verified
Proof: one
Test: none

`0x46b72a` indexes `0x46b7a8` after `cmp eax,0xb; ja`. Its raw dwords: `0x46b731` (`0x5aaa70`),
`0x46b73d` (`0x5b3320`, store 0), `0x46b754` twice (`0x5aaa50(2)`, jump to `0x472580`), `0x46b776`
three times (`0x5aaa50(2)`, store 0), `0x46b79d` (store 0), `0x46b776`, `0x46b767` (store 0, jump to
`0x46b880`), `0x46b78f` twice (with `0xb421c0` null, `0x46b796` `je` returns without a store; otherwise `0x473ce0`,
then the store at `0x46b79d`). `ClientDestroyGame`
stores 9 at `0x402075` for a non-zero stack argument, which the refused character login `0x5aad70`
passes. All 46 dword occurrences of `0xb41da0` decode as instructions; no other store writes 7 or 9.

## Per-frame update

`CGlueMgr::Idle` returns 1 on every path. First, with the server-alert flag `0xb41e2c` set, it
presses the dialog button (`0x46b720`), fires `SHOW_SERVER_ALERT` (21) with the text in `0xb414f8`
past its prefix `SERVERALERT:`, and clears the flag. It updates the keyboard LCD (`0x72a8d0`). In
state 0 it only acts on a pending glue reload `0xb41d8c`: with the glue not suspended (`0xb41d94`
clear) it suspends and resumes the glue and signals the current screen `0xb41478` again; it clears
the flag either way. In any other state it writes out the console lines `0x46a610` queued, polls the
session status (`0x5aa960`) and runs the state's arm.

Addresses: `0x46b930`, `0x46b947`, `0x46b957`, `0x46b98b`, `0x46ba5a`, `0x46c2a2`, `0xb41e2c`, `0xb414f8`, `0x836b3c`, `0x836ed8`, `0xb41d8c`, `0xb41d94`, `0xb41478`
Confidence: verified
Proof: one
Test: none

`0x46a52d` registers it on category 5 (`0x41fca0`); every exit loads 1 (`0x46c2a2`). `0x46b957` adds
`0xb414f8` to the length of the string `0x836b3c` points at, `0x836ed8`; `0x46a5f1` sets the flag.
`0x46b98b` tests `0xb41d94`, which `0x46ac90` sets and `0x46a7b0` clears; `0x46ac76` sets
`0xb41d8c`. `0x46ba5a` passes `[ebp−0x14]`, `[ebp−0x4]`, `[ebp−0xc]`, `[ebp−0x8]` as operation,
text, result and code; the done flag stays in `esi` and `[ebp−0x10]`. A chain of compares picks the
arm; those of 10 and 11 reload the state and fall into the later compares.

## Login entry

`CGlueMgr::DefaultServerLogin` (`0x46afb0`) returns unless the TOS and EULA are accepted
(`0xb41dfc`, `0xb41e04`) and the state is 0. An empty account, then an empty password, opens `OKAY`
with `LOGIN_ENTER_NAME` or `LOGIN_ENTER_PASSWORD`. Otherwise it sets `0x836b24` to `0x836b30` to −1,
clears the latch `0xb41e18`, enters state 1, copies the account into the 64-byte `0xb41db0` and
upper-cases it, opens `CANCEL` with no text, starts the logon (`0x5ab4b0`), zeroes the password's
characters in the caller's buffer, and tail-calls `0x51c740`, the add-on list's load, with the
account. `0x46aef0`, `0x46af20`, `0x46af50` and `0x46af80`, the `Accept*` bodies, set an agreement's
accepted flag to 1, its notice flag to 0 and its CVar to "1".

Addresses: `0x46afb0`, `0x46b009`, `0x46b028`, `0x46b04d`, `0x46b084`, `0x83708c`, `0x8370a0`, `0x837084`, `0xb41db0`, `0xb41dfc`, `0xb41df8`, `0xb41e04`, `0xb41e00`, `0xb41e0c`, `0xb41e08`, `0xb41e14`, `0xb41e10`
Confidence: verified
Proof: one
Test: none

The error path signals 3 with `OKAY` (`0x837084`) and the text of `0x83708c` or `0x8370a0`
(`0x46b0ac`). `0x46b04d` signals 3 with `"%s"` and `CANCEL` alone. `0x46b05c` passes the account in
`ecx`, the password in `edx`; `repnz scas` measures the password and `rep stos` (`0x46b073`) zeroes
it; `0x46b084` jumps with `ecx` `0xb41db0`. `0x46aef0` stores 1 in `0xb41dfc`, 0 in `0xb41df8`, then
`CVar::Set(readTOS, "1", 1, 0, 0, 1)` (`0x63df50`); its siblings use `0xb41e04`/`0xb41e00`,
`0xb41e0c`/`0xb41e08`, `0xb41e14`/`0xb41e10`. `0x46a400` sets each accepted flag to `value == 1` and
each notice flag for a negative value.

## Logon state dialogs

Net's logon listener reports each change to `0x46b0f0`, which logs, stores the state in `0x836b28`
and the result in `0x836b24`, and sets the latch `0xb41e18` for state 4 (`AUTHENTICATED`). In screen
state 1, unless the logon state is 8, `0x46b140` compares the pair with the last shown (`0x836b30`,
`0x836b2c`) and on a change records it and opens a dialog: for state 5 (`FAILED`) and a result in
[0, 21), the result's key from `0x836b78` with `OKAY`, or with `CONNECTION_HELP` for result 11
(`LOGIN_FAILED`); for another state in [0, 13) but 11 and 12, the state's key from `0x836b40` with
`CANCEL`. Anything else opens nothing.

Addresses: `0x46b0f0`, `0x46b12c`, `0x46b140`, `0x46b15b`, `0x46b1c7`, `0x46ba7c`, `0x836b24`, `0x836b28`, `0x836b2c`, `0x836b30`, `0x836b40`, `0x836b78`, `0xb41e18`
Confidence: verified
Proof: one
Test: none

`0x5abb10` passes the state in `ecx`, the result in `edx` and three strings (`ret 0xc`); the log is
`GRUNT: state: %s result: %s %s` through `0x46a610`. `0x46ba7c` skips the call for state 8.
`0x46b15b` returns when both cells match; `0x46b173` `jl` and `0x46b178` `jae` bound the result;
`0x46b17d` picks the button. `0x46b1b2` to `0x46b1c7` return for a state outside [0, 13), 11 or 12.
The raw pointers name `LOGIN_STATE_INITIALIZED` to `LOGIN_STATE_SURVEY` (12) and `LOGIN_OK` to
`LOGIN_PARENTALCONTROL` (19), then `DISCONNECTED`.

## Logon arm

After the dialog, state 1 reads the latch: set, it goes to state 0, `0x46b1f0` enters state 2 and
connects (`0x5aab20`), and the latch is cleared. Otherwise, by logon state: 5 cancels the logon
(`0x5b3320`), state 0. 6 starts a download for a transfer named `Patch` or `Survey`. 7 closes the
dialog, fires `PLAYER_ENTER_PIN` (25) with the logon object's ten bytes `+0x1ae` as numbers, and
stores 8 in `0x836b28`. 9 checksums the files over the logon object's 16 bytes `+0x19c` and sends
the logon proof. Others do nothing. The script's `PINEntered` hands 4 to 10 digits to `0x46b0c0`,
which opens `CANCEL` and forwards them.

Addresses: `0x46ba85`, `0x46ba8d`, `0x46b1f0`, `0x46bab3`, `0x46c2ac`, `0x46bac3`, `0x46bb1d`, `0x46bb2f`, `0x46bb64`, `0x46bbbf`, `0x46b0c0`
Confidence: verified
Proof: one
Test: none

`0x46bab3` subtracts 5; the raw dwords of `0x46c2ac` are `0x46bbbf`, `0x46bb64`, `0x46bac3`,
`0x46c2a0` (the shared exit) and `0x46bb2f`. `0x46b1f0` acts only in state 0, hence the store at
`0x46ba8d`. `0x46bac3` signals 5, pushes `+0x1b7` down to `+0x1ae` and signals 25 with ten `%d`s.
`0x46bb2f` calls `0x5b1170(+0x19c, 16)`, then `0x5b1f40`. `0x46bb64` compares `+0x98` with `Patch`
(`0x474610`), then `Survey` (`0x473c30`). `0x46b0c0` jumps to `0x5ab5a0` with the count in `ecx` and
the digits in `edx`, as `0x46d22a` passes them.

## Queue history

The session hands each `AUTH_WAIT_QUEUE` position to `0x46ae50`, which keeps eight (position, time)
samples, newest first. When the position equals the newest and the oldest time is set, nothing
shifts; otherwise each sample moves one older and the position becomes the newest. The newest time
becomes now either way. Then, with the oldest time set and the newest position below the oldest, the
estimate `0xb41df0` becomes `−(trunc((now − t₇) / (p₀ − p₇)) · p₀)` in 32-bit signed arithmetic: the
divisor is negative, the quotient truncates toward zero, the multiply comes after and wraps.
Otherwise the estimate keeps its value. Connecting (`0x5aab20`) zeroes it and the samples.

Addresses: `0x46ae50`, `0x46ae67`, `0x46aece`, `0x46aed3`, `0x46aed5`, `0x46aeda`, `0xb41cf8`, `0xb41d14`, `0xb41d18`, `0xb41d34`, `0xb41df0`
Confidence: verified
Proof: one
Test: none

`0x5ab280` passes the session's `+0x1af0` in `ecx` for code `0x1b`. `0x46ae67` (`test; ja`) skips
the shift for a repeat with `0xb41d14` non-zero. The loop copies, for `i` from 7 to 1, position
`i − 1` to `i` in `0xb41d18` and the time likewise in `0xb41cf8`; `0x46aea8` stores the position,
`0x46aebb` the clock `0x42c010`. `0x46aece` `jns` skips unless `[0xb41d18] − [0xb41d34]` is
negative. `0x46aed3` is `idiv` of `now − [0xb41d14]` after `cdq`, `0x46aed5` `imul` by the position,
then `neg`. The other store of `0xb41df0` is `0x5aab45`, beside `0x5aab30` and `0x5aab3f`, which
clear both arrays.

## Queue dialog

In state 2 with the code `AUTH_WAIT_QUEUE` the frame updates the dialog with the button
`CHANGE_REALM` and a text from the newest position and, when the `realmName` CVar is non-empty, the
realm's name (the `QUEUE_NAME_*` keys, else `QUEUE_*`). With no estimate (`0xb41df0` ≤ 0) the text
is `QUEUE_TIME_LEFT_UNKNOWN` alone. Otherwise the time left is `t₀ + estimate − now`, signed: zero
or below 60,000 ms gives `QUEUE_TIME_LEFT_SECONDS`, else `QUEUE_TIME_LEFT` with the whole minutes.
That line follows the status text and a newline; the status text is empty while the operation runs.
The estimate changes only with a packet, so between packets the time left counts down.

Addresses: `0x46bc52`, `0x46bcb8`, `0x46bcfa`, `0x46bdd7`, `0x46bdfc`, `0x46be09`
Confidence: verified
Proof: one
Test: none

`0x46bc52` compares the code with `0x1b`; `0x5ab7d0` reads the CVar. `0x46bcb8` `jle` sends a
non-positive estimate to `0x46be09`. Otherwise `[0xb41cf8] + [0xb41df0] − now`: `0x46bcd9` takes
zero to the `SECONDS` keys, and after a second clock read `0x46bcfa` `jl` takes less than `0xea60`.
`0x46bd09` completes `imul 0x45e7b273`, `sar edx,14` and the sign add: the quotient by 60,000.
`0x46bdd7` joins with `"%s\n%s"`; `0x46bdfc` signals 4. `0x5aa960` gives an empty text while not
done, and `0x5ab29b` restarts the queue's operation.

## World connect success

Any other code in state 2 updates the dialog with the status text. Once the operation succeeds:
after the connect (operation 2) the frame begins authentication (`0x5aaba0`), staying in state 2;
after the authentication it goes to state 0, tears the roster down (`0x472580`), shows `charselect`,
zeroes the newest queue position and calls the keyboard LCD's `0x72b810`.

Addresses: `0x46be8b`, `0x46be9e`, `0x46bea7`, `0x46bfd9`, `0x46bffd`, `0x46c00e`, `0x46c023`
Confidence: verified
Proof: one
Test: none

`0x46be8b` signals 4 with `"%s"` and the text. `0x46be9e` returns while not done; `0x46bea7` sends a
non-zero result to `0x46bfd9`, which compares the operation with 2. `0x46bffd` calls `0x5aaba0`,
which begins operation 3 at status `0x0b` and discards its four arguments (`ret 0x10`). `0x46c00e`
stores 0; `0x46c023` zeroes `0xb41d18`.

## World connect failure

A failure opens a dialog unless the code is `RESPONSE_CANCELLED` (2): `OKAY_WITH_URL` with the text
and the URL key when `0x803740` lists the code, else `OKAY`. Its rows are `AUTH_BANNED` (28),
`AUTH_DB_BUSY` (31), `AUTH_NO_TIME` (30), `AUTH_SUSPENDED` (32) and `AUTH_PARENTAL_CONTROL` (33),
keyed `AUTH_BANNED_URL` and so on. Every failure goes to state 0; a cancel also sets `0xb41d98` and
disconnects (`0x5ab1a0`). `AUTH_FAILED` (13) then shows `login` when the current screen is
`charselect`, and ends. Any other code tears the roster down and, when the realm holder `0xc28118`,
created if absent, lists realms, closes the dialog and raises the list (`0x46e510`); with none it
asks for the list (`0x46b8d0` with 1).

Addresses: `0x46bec0`, `0x46bee7`, `0x46befd`, `0x46bf16`, `0x46bf1f`, `0x46bf6f`, `0x46bf86`, `0x46bfa7`, `0x46c06e`, `0x803740`, `0xc28118`
Confidence: verified
Proof: one
Test: none

`0x46beb3` sends code 2 past the dialog. `0x46bec0` compares the code with the first dword of each
0x24-byte row of `0x803740`, up to `0xb4`; a hit signals 3 with `OKAY_WITH_URL`, the text and the
row's `+4` string, unlocalised (`0x46bf6f`); the miss signals `OKAY` (`0x46bee7`). `0x46bef5` stores
0, `0x46befd` 1 in `0xb41d98`. `0x46bf16` tests 13; `0x46bfa7` compares `0xb41478` with `charselect`
ignoring case. `0x46bf3a` allocates 0x14 bytes and stores the vtable `0x803808`. `0x46bf86` `jbe` on
`+0x08` reaches `0x46c06e`.

## Realm change

`CGlueMgr::ChangeRealm` (`0x46b210`) selects a non-null record's realm by its name `+0x09`
(`0x5ab7b0`). With the session connected it sets `0xb41d98` and `0xb41d9c` and disconnects; the
disconnect handler then reconnects. Otherwise it enters state 2, opens `CANCEL` with
`GAME_SERVER_LOGIN` and connects (`0x5aab20`).

Addresses: `0x46b210`, `0x46b221`, `0x46b230`, `0x46b235`, `0x46b241`, `0xb41d98`, `0xb41d9c`
Confidence: verified
Proof: one
Test: none

`0x46b214` adds 9 to `ecx`. `0x46b221` reads the session's `+0x1b00`, which `0x5aaa90` sets and
`0x5ab1a0` clears. `0x46b230` and `0x46b235` store 1 and `0x46b241` tail-jumps to `0x5ab1a0`;
`0x46b250` stores 2 and `0x46b27b` tail-jumps to `0x5aab20`.

## Requests and their arms

`0x46b880` asks for the character list unless the state is 9: state 3, `CANCEL` with
`CHAR_LIST_RETRIEVING`, then `0x5aabb0`. `CGlueMgr::GetRealmList` (`0x46b8d0`) enters state 4, opens
`CANCEL` with `REALM_LIST_IN_PROGRESS` for a non-zero argument, and asks (`0x5ab840`). `0x46b290` (a
create request) enters state 5 and `0x46b2e0` (a non-zero guid) state 6, each opening `CANCEL` with
the status text of `0x2d` or `0x38` before the request. `0x46b7e0` cancels a state-4 query to state
0; `0x46b810` cancels the logon on the `login` screen. States 3 to 6 update the dialog with the
status text each frame; once done:

| state | success | failure |
|---|---|---|
| 3 | state 0, close, load the roster (`0x4724d0`) | code 2: teardown, `0x46b8d0` with 1; else `OKAY`, state 0 |
| 4 | state 0, close, raise the realm list (`0x46e510`) | `OKAY`, state 0 |
| 5 | state 0, close, `SELECT_LAST_CHARACTER` (12), `charselect` | `OKAY`, state 0 |
| 6 | `SELECT_FIRST_CHARACTER` (13), `0x46b880` | `OKAY`, state 0 |

Addresses: `0x46b880`, `0x46b887`, `0x46b8d0`, `0x46b290`, `0x46b2e0`, `0x46b7e0`, `0x46b810`, `0x46c03a`, `0x46c05f`, `0x46c11e`, `0x46c19d`, `0x46c173`
Confidence: verified
Proof: one
Test: none

`0x46b8d2` stores 4 before the `ecx` test. `0x46b290` and `0x46b2e0` pass `0x2d` and `0x38` to
`0x5abd20`, then call `0x5aac50` and `0x5ab100`; `0x46b2ed` tests both guid words. `0x46b7e0` calls
`0x5aaa50(2)` in state 4 only. Each arm tests the done flag, then the result; the failures share
`0x46c173`, which signals 3 with `OKAY` and the text and stores 0. State 3 tests the code at
`0x46c05f`. `0x46c11e` signals 5 and 12; `0x46c19d` signals 13 and calls `0x46b880` without storing
0. State 4's update pushes the state as the event id (`0x46c0b3`).

## Rename request and reply

`0x46b340` (name in `ecx`, guid on the stack) returns 1 without acting for a zero guid or an empty
name. A name the check `0x5abd50` does not pass as `CHAR_NAME_SUCCESS` (`0x52`) fires
`FORCE_RENAME_CHARACTER` (23) with that status's token and returns 0. Otherwise it enters state 7,
opens `CANCEL` with `CHAR_RENAME_IN_PROGRESS`, sends opcode `0x2c7` with the 8-byte guid and the
name with its terminator, and returns 1. `HandleCharRenamePacket` (`0x46b440`), opcode `0x2c8`'s
handler, reads a result byte, then a guid and a name of at most 48 bytes only while bytes remain. It
closes the dialog, leaving the state; result 0 renames the record (`0x4725f0`), `0x31` fires 23 with
`CHAR_CREATE_NAME_IN_USE`, any other with `CHAR_RENAME_FAILED`.

Addresses: `0x46b340`, `0x46b374`, `0x46b388`, `0x46b3a4`, `0x46b40b`, `0x46b440`, `0x46b467`, `0x46b498`, `0x46b49c`, `0x46b4eb`, `0x83711c`
Confidence: verified
Proof: one
Test: none

`0x46b355` and `0x46b35f` to `0x46b367` reach the `ret 8` with 1. `0x46b374` `cmp eax,0x52`;
`0x46b388` signals `0x17` with `0x5abd20`'s token. `0x46b3c8` puts `0x2c7` in a local store,
`0x46b3f7` the guid through `0x418370`, which writes eight bytes, then the string; `0x46b40b` sends.
`0x46aa44` registers `0x46b440` for `0x2c8`. `0x46b467` calls the store's slot 5, 1 when the read
position equals the size, which skips both reads and leaves the zeroed guid. `0x46b48b` signals 5;
`0x46b498` `cmp al,0x31`; `0x46b49c` passes `0x83711c`; `0x46b4eb` calls `0x4725f0`.

## Enter World

`CGlueMgr::EnterWorld` (`0x46b500`) returns when no realm is selected (`0x5ab9a0`). It points
`0xb41df4` at the selected record, `[0xb42144] + 0x120·i` for the index `i` in `0x83856c` when
`0 ≤ i < [0xb42140]`, else null, and returns on null. It sets the record's byte `+0x118`, returns
unless the session is connected, then tests the record's flags `+0xfc`: bit 2 opens `OKAY` with
`CHAR_LOGIN_LOCKED_FOR_TRANSFER` (`0x44`), bit 14 fires `FORCE_RENAME_CHARACTER` with
`CHAR_RENAME_DESCRIPTION`; either returns. Otherwise it sets `lastCharacterIndex` to `i` in decimal,
hands the session the account and record, picks the game tip, starts the loading screen for the map
`+0x38`, saves the configuration, empties the texture caches, cancels the logon and enters state 8.

Addresses: `0x46b500`, `0x46b50d`, `0x46b539`, `0x46b54a`, `0x46b56c`, `0x46b57e`, `0x46b5b1`, `0x46b5fa`, `0x46b6f6`, `0x46b700`, `0x46b711`, `0xb41df4`, `0x83856c`, `0xb42140`, `0xb42144`
Confidence: verified
Proof: one
Test: none

`0x46b521` `jl` and `0x46b529` `jge` bound the index; `lea` and `shl 5` make `0x120·i`. `0x46b56c`
tests the session's `+0x1b00`. `0x46b57e` `test al,0x4`; `0x46b5b1` `test ah,0x40`. `0x46b5fa` sets
`lastCharacterIndex` to the `"%d"` text. `0x46b604` and `0x46b60f` call the two setters. `0x46b6f1`
calls `0x406640` with `+0x38`; `0x46b6f6` calls `0x63d980`, which writes the file only when a CVar
changed; `0x46b6fb` calls a bare `ret` and `0x46b700` `0x449010`, which frees the `CGxTexCache`
lists. `0x46b70c` calls `0x5b3320`.

## Game tip choice

`EnterWorld` registers `gameTip` (default "0") and `showGameTips` (default "1") on first use. When
the record's new-character byte `+0x10a` is clear, `showGameTips` is non-zero and `GameTips.dbc`
holds `n > 0` rows, it takes `k` from `gameTip`, replaces it with 0 when `k < 0` or `k ≥ n`, hands
row `k`'s text in the client's locale to the loading screen (`0x406630`), and sets `gameTip` to
`k + 1` in decimal. A failed condition skips the tip and the write. So the tips run in file order,
and the clamp is the wrap.

Addresses: `0x46b633`, `0x46b658`, `0x46b671`, `0x46b678`, `0x46b682`, `0x46b693`, `0x46b6ad`, `0x46b6e3`, `0xb41e38`, `0xb41e3c`, `0xc0dcd0`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

`0x46b633` and `0x46b658` register through `0x63db90`, keeping the records in `0xb41e38` and
`0xb41e3c`. `0x46b671` skips on a non-zero `+0x10a`, `0x46b678` on a zero `showGameTips` value
`+0x28`. `0x46b682` loads `gameTip`; out of range reaches `0x46b68f` `xor esi,esi`; `0x46b693` `jle`
skips on a count `+0x04` of the store `0xc0dcd0` of 0 or less. The second range test cannot fail.
`0x46b6ad` reads the row's dword `+4 + 4·locale`, the locale from `0xc0e080`. `0x46b6bc` `inc esi`;
`0x46b6e3` sets the CVar. The list parse sets `+0x10a` when the byte `+0x108` reads 0 (`0x5b44f9`).

## Two-phase world entry

In state 8 the frame waits until `0x407e70` reports the loading screen up. With the glue not
suspended (`0xb41d94` clear) it keeps the record's guid in `0xb41e30`, takes its map `+0x38` and
position `+0x44`, replaces the position by the first-login camera's for a new character (`+0x10a`),
suspends the glue (`0x46ac90`, which sets `0xb41d94`) and calls `ClientInitializeGame` (`0x401570`)
with the map and position. On a later frame, once `0x443e20` reports nothing pending, it enters
state 0, stops the glue music with a fade of 3.0 (`0x45aeb0` with null, when `enableMusic` is on)
and sends the login for the kept guid (`0x5aae20`).

Addresses: `0x46c1c1`, `0x46c1ce`, `0x46c1dd`, `0x46c203`, `0x46c214`, `0x46c236`, `0x46c247`, `0x46c258`, `0x46c272`, `0xb41e30`
Confidence: verified
Proof: one
Test: none

`0x407e70` returns whether `0x882be0` is non-null, an object the loading screen's start creates
(`0x406800`) and `0x407e80` clears. `0x46c1dd` stores the guid's two words; `+0x44` to `+0x4c` go to
`[ebp−0x20]`. `0x46c214` calls `0x46c2c0` with `cl` the byte `+0x100` and `edx` the position,
ignoring the result. `0x46c236` passes the map in `ecx`, the floats on the stack. `0x443e20` returns
1 while either of two lists is non-empty or a count is non-zero. `0x45aeb0` with null calls
`0x45b050` with the constant 3.0 past its `enableMusic` test.

## First-login camera

`0x46c2c0` (race in `cl`, position in `edx`) looks the race up in `ChrRaces.dbc`, its
`CinematicSequenceID` (`+0x40`) in `CinematicSequences.dbc`, and that sequence's first camera
(`+0x08`) in `CinematicCamera.dbc`; a miss returns 0 and leaves the position. It loads the camera's model
(`+0x04`) and waits for it, places it at the origin `+0x0c`..`+0x14` turned about z by the facing `+0x18`,
and writes camera 0's eye, vector property 7, over the position, returning 1. The world loads where the
race's intro cinematic opens. `enterworld` holds the entry conditions.

Addresses: `0x46c2c0`, `0x46c2d5`, `0x46c2f8`, `0x46c320`, `0x46c357`, `0x46c36c`, `0x46c3e6`, `0x46c40d`, `0x46c4cf`, `0x46c4fa`, `0xc0ded8`, `0xc0deb0`, `0xc0dec4`
Confidence: verified
Proof: one
Test: none

Each lookup bounds the id by the store's maximum id `+0x0c` and reads the id map `+0x08`: `ChrRaces`
`0xc0ded8` (`0x46c2d5`), `CinematicSequences` `0xc0deb0` (`0x46c2f8`), `CinematicCamera` `0xc0dec4`
(`0x46c320`); the loader `0x5425a0` maps by the id column. `0x46c357` calls `0x707350` with `+0x04` and
`0x46c36c` `0x7103d0(model, 1, 0)`, which waits for the read. `0x46c3e6` translates an identity
matrix by `+0x0c`..`+0x14`; `0x46c40d` rotates it by `+0x18` about (0, 0, 1). `0x46c4cf` calls
`0x713590` for camera 0 and `0x46c4fa` `0x7abf80` with 7, the eye.

## Disconnect handler

`CGlueMgr::NetDisconnectHandler` (`0x46c540`), which the session's disconnect runs, zeroes the
state, noting whether it was 2. An expected disconnect (`0xb41d98`) clears the flag and, when
`0xb41d9c` asks for a reconnect, clears it, enters state 2, opens `CANCEL` with `GAME_SERVER_LOGIN`
and connects. Otherwise a connection that is not the current one is ignored (`0x5abd40`); the game
is destroyed (`0x401ee0`), the mouse mode reset and a suspended glue resumed. Out of state 2 it
fires `DISCONNECTED_FROM_SERVER` (2); in state 2 a done, failed operation opens `OKAY` with its text
instead, anything else fires 2. Either way the logon is cancelled. Returns 1.

Addresses: `0x46c540`, `0x46c555`, `0x46c55d`, `0x46c563`, `0x46c585`, `0x46c5cf`, `0x46c5f8`, `0x46c613`, `0x46c62e`, `0x46c67a`, `0x46c68f`, `0x537d20`
Confidence: verified
Proof: one
Test: none

`0x537d20` calls it with the `NetClient` in `ecx`. `0x46c555` `setne bl`; `0x46c55d` stores 0.
`0x46c563` branches on `0xb41d98`, set by `0x46b210`, `0x46befd` and `0x46d34f`; the reconnect runs
from `0x46c585`. `0x46c5cf` passes the caller's `ecx` to `0x5abd40`. `0x46c5f8` calls `0x401ee0`
with `ecx` 0, `edx` 1 and 0 pushed: with a game up it resumes the glue and shows `login`. `0x46c613`
resumes on `0xb41d94`. `0x46c618` tests `bl`; `0x46c62e` signals 2; `0x46c638` calls `0x5b3320`.
`0x46c67a` requires a zero result before `0x46c68f`. Each path prints a line through `0x63cd50`.

## Screens and callbacks

`CGlueMgr::SetScreen` (`0x46b840`) fires `SET_GLUE_SCREEN` (0) with a screen name. `0x46b860` copies
the script's current screen name into the 64-byte `0xb41478`. `0x46b870`, `QuitGame`'s body, calls
`0x401560` with 0, which stores the 0 in a client global and closes the current context. `0x46c520`,
the callback `Screenshot` passes, fires `GLUE_SCREENSHOT_SUCCEEDED` (14) for a non-zero argument,
else `GLUE_SCREENSHOT_FAILED` (15). `0x46b910` hands a received transfer chunk to the open patch
download, else to the open survey.

Addresses: `0x46b840`, `0x46b860`, `0x46b870`, `0x46c520`, `0x46c527`, `0x46b910`, `0x46b917`, `0x46b925`
Confidence: verified
Proof: one
Test: none

`0x46b848` signals 0 with `"%s"`. `0x46b868` is `SStrCopy` with 0x40. `0x401560` stores `ecx` and
jumps to `0x41f9a0`. `0x46d3d0` pushes `0x46c520`, whose `0x46c527` keeps 14 for a non-zero `ecx`.
`0x5abae0` calls `0x46b910`; `0x46b917` and `0x46b925` test `0xb42260` and `0xb421c0` and jump to
`0x474b50` or `0x473d50`.
