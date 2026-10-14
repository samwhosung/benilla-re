# glue: connect paths

The paths of the logon and realm screens that the flow doc leaves to the bytes around them: the launch
auto-connect, the queue dialog, a failed logon, the cancel of the realm list.

The flow doc gives each screen state, its dialogs and the per-frame arms. This doc follows three
chains across them: the remembered realm from the logon to the character list, the queued world
connect and what its dialog shows and sends, and the realm list's cancel and layering. The wire
of the logon is `net/login`; the operation record is `net/session`.

Roots: `0x46b1f0`, `0x46b210`, `0x46b720`, `0x46b810`, `0x46b140`, `0x46ae50`

## Launch auto-connect

On launch the client connects to the realm named by the persisted `realmName` CVar without showing the
realm list. The logon state 4 sets the latch `0xb41e18`; state 1's arm of `Idle` clears the screen
state and calls `0x46b1f0`, which sets state 2, opens no dialog and begins operation 2 (`0x5aab20`),
whose first act is the realm list request. On the reply the operation arm calls `0x5ab800`, which finds
the realm by name and dials it, or reports `REALM_LIST_REALM_NOT_FOUND` (0x26). Success reports
nothing, so the operation stays incomplete and `0x46e510` never runs: the realm list appears only on
a failed lookup, a failed list query or a request from the player.

Addresses: `0x46b12c`, `0x46ba85`, `0x46ba8d`, `0x46ba93`, `0x46b1f0`, `0x46b20a`, `0x5aab20`, `0x46b217`, `0x5ab7b0`, `0x5ab7c4`, `0x5ab800`, `0x5ab809`, `0x5ab812`, `0x5ab823`, `0x5ab82f`, `0x5aa920`, `0x5aa960`, `0x46be9e`, `0x46b27b`, `0x46c5bf`
Confidence: verified
Proof: one
Test: none

`0x46b1f0` is `cmp [0xb41da0],0; jne ret; mov [0xb41da0],2; call 0x5ab490; jmp 0x5aab20`. `0x5ab800`:
`0xc28134` nonzero or `0x5ab6f0` true goes to `0x5ab823` (a call to `0x537820`, no status);
otherwise `push 0x26; push 0` and `0x5aa920`. `0x5aa920` stores `[+0x1b0c]` the result, `[+0x1b08]` 1;
`0x5aa400` clears `+0x1b08`; `0x5aa960` returns it, and `0x46be9e cmp [ebp-0x10],0; je` skips the
report while it is 0. `0x5aab20` has three call sites: `0x46b20a`, `0x46b27b` and the redial tick
`0x46c5bf`, so a census of `0x46b210` alone misses this path. `ConnectToRealm` fills the CVar: `0x46b217 call 0x5ab7b0` sets
`realmName` ("Last realm connected to") to the record's name, then selects by it (`0x5ab7c4`).

## Change Realm link

Change Realm on character select requests the realm list (`RequestRealmList(1)`) and raises it over the
unchanged screen; the world connection drops only when the player picks a realm. Choosing one runs
`0x46b210`, which stores the realm's name in `realmName`; with a live connection it then sets the
expected-disconnect flag `0xb41d98` and the reconnect flag `0xb41d9c` and ends in `Disconnect`
(`0x5ab1a0`). The disconnect handler `0x46c540` clears the flags and, for the reconnect, enters state 2
with `GAME_SERVER_LOGIN` and begins operation 2.

Addresses: `0x46b210`, `0x46b221`, `0x46b230`, `0x46b235`, `0x46b241`, `0x46b246`, `0x5ab1a0`, `0x46c540`, `0x46c56f`, `0x46c579`, `0x46c585`, `0x46c594`, `0x46b8d0`, `0x46ecf0`
Confidence: verified
Proof: one
Test: none

`CharacterSelect_ChangeRealm` calls only `RequestRealmList(1)`; the `ChangeRealm` verbs are called from
`RealmList.lua` and `GlueDialog.lua`. `0x46b227 test [svc+0x1b00]` splits the live case (`0x46b22b` to
`0x46b241`) from the new connection (`0x46b246`, state 2 at `0x46b250`). `0x46c56f` reads `0xb41d9c`,
`0x46c579` clears `0xb41d98`, and `0x46c594` stores state 2 on the reconnect path.

## Queue dialog

A queued connect shows one dialog, the `CANCEL` dialog `ConnectToRealm` opened with
`GAME_SERVER_LOGIN`, and every frame of state 2 re-texts it with `UPDATE_STATUS_DIALOG`, whose second
argument `CHANGE_REALM` relabels the button. The text is one of six: `QUEUE_TIME_LEFT_UNKNOWN`,
`QUEUE_TIME_LEFT_SECONDS` or `QUEUE_TIME_LEFT`, each also with `NAME` when `realmName` is non-empty.
The status text before it is empty, because `GetStatus` returns the empty string while the queue's
operation is incomplete, so the two estimate legs gain a leading blank line and `AUTH_WAIT_QUEUE` is never
shown; the unknown leg shows its text alone. The G15 display draws the `QUEUE_*` keys without `NAME` from the raw estimate.

Addresses: `0x46b210`, `0x46b26c`, `0x46bc52`, `0x46bcb8`, `0x46bdd7`, `0x46bdfc`, `0x5aa991`, `0x5aa9e9`, `0x5aa400`, `0x5aa420`, `0x5ab280`, `0x5ab29b`, `0x5ab2a6`, `0x46ae50`, `0x72a8d0`, `0x72b000`
Confidence: verified
Proof: one
Test: none

`0x5aa960` early-outs at `0x5aa991 je 0x5aaa35` while `+0x1b08` is 0; the queue formatter at
`0x5aa9e9` to `0x5aaa05` lies after that test. `0x5ab280`, the listener's slot `+8`, reaches
`0x5aa400` (op 0x0a, status 0x1b) at `0x5ab29b`, which clears the flag, then `0x46ae50` at `0x5ab2a6`.
`0x46bc52 cmp [ebp-8],0x1b` picks the queue legs and `0x46bcb8 jle` the unknown estimate.
`GlueDialog.lua` uses a second argument of `UPDATE_STATUS_DIALOG` as the button text. The keyboard display `0x72a8d0` picks
`QUEUE_TIME_LEFT` for an estimate over 60000 ms (`0x72b000`), the `SECONDS` key for a smaller nonzero one and
`UNKNOWN` for 0.

## Queue cancel

Cancelling a queued connect sends nothing to the world server. `StatusDialogClick` in state 2 sets
status 2 (`RESPONSE_CANCELLED`) and tears the roster down; the next frame's failure arm sees status 2,
sets the state to 0 and `0xb41d98` to 1 and disconnects the socket, with no packet built. The screen
that follows is the realm list, which is why the button reads Change Realm. When no realm list is
cached the cancel also requests one from the realmd (`0x46b902`).

Addresses: `0x46b720`, `0x46b754`, `0x46beb3`, `0x46bef2`, `0x46bef5`, `0x46befd`, `0x46bf0e`, `0x5ab1a0`, `0x5aaa50`, `0x472580`, `0x46b8d0`, `0x46b902`
Confidence: verified
Proof: one
Test: none

State 2's arm of the table `0x46b7a8` is `0x46b754`: `0x5aaa50(2)` and a jump to `0x472580`. The
failure arm compares the status with 2 at `0x46beb3` and `0x46bef2`, stores 0 and 1 at `0x46bef5` and
`0x46befd`, and calls `0x5ab1a0` at `0x46bf0e`, which closes the socket through `0x5b5370`. A
send needs a `CDataStore`; none is built on this path.

## Failed logon retry

A failed logon locks nothing on the client. The logon object counts failures in one byte, `+0x1ad`,
and only when the challenge reply's security byte `+0x1ac` is set; the count only changes which of
three texts is shown (`LOGIN_UNKNOWN_ACCOUNT`, `..._PIN`, `..._CALL`). Wire results 4 and 5 take one
arm and show the same text. After a failure the screen state returns to 0, the Login button is never
disabled, `0x46afb0` clears the dedupe cells, and the logon is cancelled, closing the socket, so the
next attempt is a new connection with a new challenge.

Addresses: `0x5b2d00`, `0x5b2d0d`, `0x5b2d1b`, `0x5b2e1f`, `0x5b32dc`, `0x46afb0`, `0x46b009`, `0x46b00f`, `0x46b015`, `0x46b01b`, `0x46bbbf`, `0x46b749`, `0x5b3320`, `0x5bb3b0`
Confidence: verified
Proof: one
Test: none

`+0x1ad` has four references: `0x5b2d0d` and `0x5b2d1b` (the count), `0x5b2e1f` (zeroed on a good
proof) and `0x5b32dc` (the constructor). `0x5b2d00 cmp [ebx+0x1ac],dl` returns code 7 at once when the
byte is 0; otherwise `inc cl`, `cmp al,3; sbb eax,eax; add eax,9` gives 8 below 3 and 9 from 3. The
counter wraps at 256. `0x46bbbf` is the failed arm's cancel and `0x46b749` the button's.

## Login dialog texts

The dialogs resolve their keys through the script function `GetText`, so a key the shipped
`GlueStrings.lua` does not define gives an empty text, never null. Five keys of the two login tables
are undefined: `LOGIN_OK`, `LOGIN_STATE_FAILED`, `LOGIN_STATE_PIN`, `LOGIN_STATE_PIN_WAIT` and
`LOGIN_STATE_DISCONNECTED`. Only `LOGIN_STATE_PIN` can be shown: logon state 7 opens a `CANCEL` dialog
with no text for one frame, until the arm fires `PLAYER_ENTER_PIN` and sets state 8. `OKAY_WITH_URL` is
raised in one place, the world failure arm of state 2, so no logon refusal can show a URL.

Addresses: `0x46b140`, `0x46b1c5`, `0x46b1c9`, `0x46ba7c`, `0x46bac3`, `0x46bf63`, `0x46bec2`, `0x703bf0`, `0x836b40`, `0x836b78`
Confidence: verified
Proof: one
Test: none

`0x46b140` pushes only `OKAY` or `CONNECTION_HELP` for a result and `CANCEL` for a state. The
reference census over the image: `"OKAY_WITH_URL"` has one reference (`0x46bf63`), the table `0x803740`
one (`0x46bec2`), `CONNECTION_HELP` one (`0x46b17f`); the controls `"OKAY"` and `"CANCEL"` hit
10 and 11. `0x46b140` has one caller, `0x46ba80`, in state 1's arm, and state 8 skips the call.
The undefined keys are read over the whole extracted GlueXML tree.

## Realm list cancel

`RealmListDialogCancelled` compares the current screen name `0xb41478` with `login` and does
nothing else on any other screen, so cancelling the list on character select is a no-op. On the
login screen it cancels the logon object (`0x5b3320`), which closes the realmd socket
asynchronously: it moves the link state 5 to 7, the net thread's poll turns state 7 into a flag
that runs `0x5b53a0` (deregister, close, state 6) and the disconnect reaches the logon object,
which reports `DISCONNECTED`. That report opens a dialog only in state 1, and the list is open only in
states 0 and 4, so the player sees nothing.

Addresses: `0x46b810`, `0x46b81f`, `0x46b82f`, `0x46ed20`, `0x5b3320`, `0x5bb3b0`, `0x5bb290`, `0x5b5370`, `0x5b53a0`, `0x5bcf07`, `0x5bdb16`, `0x5bdb1d`, `0x5b1bd0`, `0x5b1c15`, `0x46b140`, `0x46afb0`
Confidence: verified
Proof: one
Test: none

`0x46b81f call 0x64a4c0` compares `0xb41478` with `login` (`0x82e3e0`), ignoring case; a mismatch
returns. `0x5b5370` stores state 7 through `0x5b72d0` only from state 5. `0x5bcf07 cmp [esi+0x10],7`
sets bit 8 and `0x5bdb16 test bl,8` reaches `0x5b53a0` (`0x5bdb1d`), which ends in state 6. `0x5b1bd0`
reports state `0xb`, then a synthesised result `0xff` when `[+4]` is 0 (`0x5b1c15`), and clears
`+4` and `+5`. `0x46afb0` clears all four report cells.

## Realm list layering

The realm list is a dialog over the current screen, not a screen. `RealmList` is `toplevel`,
`setAllPoints` to `GlueParent`, with mouse and keyboard enabled and strata DIALOG, over screens at
MEDIUM; `GlueDialog` uses the same set. The hit test stops at the first frame hit, so the screen
below takes no clicks, and the list's background layer is a full-screen quad that dims it. No native
code shows or hides the list, and the glue's `GlueScreenInfo` names seven screens, none of them
the list, so asserting a screen leaves an open list in place.

Addresses: `0x46e510`, `0x46b840`, `0x46c019`, `0x46c0a5`, `0x46c0e2`
Confidence: inferred
Proof: none
Test: none

`RealmList.xml` declares the frame with the attributes above and `GlueParent.lua` lists the seven
screens. The strata order and the hit test are `ui`'s; the claim that no native code of the realm
list's object file shows or hides the frame is not censused here. The success path that raises the list
after a request is the state 4 arm at `0x46c0a5`, which calls `0x46e510` at `0x46c0e2`.
