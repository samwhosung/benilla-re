# ui: entering and leaving the world

The events the interface gets when the character enters or leaves the world, and the order of everything
around them: the enter cascade, the leave function, the three ways into the cascade, what a reload
and a world transfer re-run, and why the guild message of the day shows above the welcome lines.

The code is `GameUI.cpp`: the enter cascade `0x4908c0`, the leave function `0x490a80`, the shutdown
`0x490bd0` and the initializer `0x48fbf0`. The object side that calls them is `objectlayer/players`; the
chat latch they release is `chatevents#the-display-latch`.

Roots: `0x4908c0`, `0x490a80`, `0x4989c0`, `0x498a10`

## The enter cascade

`0x4908c0` runs when the active player arrives. A once-per-world latch `0xb4b424` makes a second entry
return; otherwise it sets the latch, runs set-up calls on the player, then fires `PLAYER_LOGIN` (id 270)
if it is armed and `PLAYER_ENTERING_WORLD` (id 272), which it fires every time. After the events it
makes 38 calls: the minimap initializer `0x4ed9d0`, the chat release `0x4989c0` (which clears the
display latch and shows the queued lines) and 36 more that rebuild each frame's state for the new
world. It ends by setting tutorial flag 0x29 and calling `0x4b5390` again.

Addresses: `0x4908c0`, `0x4908c7`, `0x4908ce`, `0x490941`, `0x490946`, `0x49094b`, `0x490952`, `0x490954`, `0x490959`, `0x49095e`, `0x490965`, `0x49096a`, `0x49096f`, `0x490974`, `0x4989c0`, `0x490979`, `0x490a28`, `0x490a43`, `0x490a48`, `0x490a51`, `0x490a78`, `0xb4b424`, `0xb4e260`, `0x703e50`
Confidence: verified
Proof: one
Test: none

The end of the cascade: a dead player gets `0x5e0ae0` first (`0x490a38`); the second `0x4b5390` call is tutorial 0 (`0x490a4f`); `0x458f60` follows, pairing with `0x458f50` at `0x4908d5`; and during a cinematic `0x4c0d10(0x48edd0, ...)` runs at `0x490a73`.

The guard is `mov al,[0xb4b424]; test al,al; jne 0x490a78` (`0x4908c0`, `0x4908c7`) and the set is `0x4908ce`. The
login arm reads `0xb4e260` (`0x49094b`): `je 0x490965` skips `PLAYER_LOGIN` when it is 0, and `0x49095e` disarms it.
`PLAYER_ENTERING_WORLD` follows at `0x49096a` on both paths. The 38 calls run from `0x49096f` to `0x490a28`. The
latch `0xb4b424` has two writers, `0x4908ce` (1) and `0x490a8d` (0).

## Arming PLAYER_LOGIN

`PLAYER_LOGIN` is a one-shot: the interface initializer `0x48fbf0` arms it with a plain store of 1 at
`0x49011d`, the enter cascade consumes it, and nothing else can arm it. So it fires once for each initializer
run, once at login and again after a reload, never after a world transfer.

Addresses: `0x48fbf0`, `0x49011d`, `0x49094b`, `0x49095e`, `0xb4e260`
Confidence: verified
Proof: one
Test: `ui::glue_48fbf0_finite_diff`

An image scan for the dword `0xb4e260` finds exactly three references: the store of 1 at `0x49011d`, the read
at `0x49094b` and the store of 0 at `0x49095e`.

## The three ways into the cascade

The cascade has three callers. The initializer calls it at `0x490168` when the active player's GUID is
non-zero, which is the reload case. The player's arrival `0x5dea50` calls it at `0x5deb60` when the manager's
`+0xc8` is 0, which is a fresh login or a world transfer. The player's reactivation `0x5e9bc0` jumps to it
at `0x5e9c08` for the active player. A player created for the first time never takes the third, because an
object enters the manager's second hash only by leaving range or being destroyed.

Addresses: `0x490168`, `0x49015f`, `0x490164`, `0x490166`, `0x5deb60`, `0x5deb49`, `0x5deb50`, `0x5dec7b`, `0x5e9c08`, `0x5e9bf1`, `0x5e9bfa`, `0x464530`, `0x4646d9`, `0x466136`, `0x466150`, `0x465dbc`, `0x465e78`, `0x466245`, `0x466254`, `0x468550`
Confidence: verified
Proof: one
Test: none

At `0x490168` the initializer first asks `0x468550` for the active GUID and skips the call when it is 0
(`or eax,edx; je`). A fresh create block sets the GUID pair at `0x466245` and `0x466254` before the object exists, then
dispatches by object type through `0x465e78`, where the player's slot reaches `0x5debe0` (`0x465dbc`) and its
arrival `0x5dea50` (`0x5dec7b`). `0x464530` marks a lookup that moves an object back from the released
table by storing 1 through its out-parameter (`0x4646d9`); only that case runs the reactivation slot.

## Leaving the world

`0x490a80` leaves the world: if the latch `0xb4b424` is set it clears it (`0x490a8d`), runs a battery of 36
interface cleanups with no branch in between (closing the trade and loot windows among them), and fires
`PLAYER_LEAVING_WORLD` (id 273), the only place that event is fired. A running cinematic then fires
`CINEMATIC_STOP`. It has three callers: the shutdown `0x490bd0` (`0x490c20`), the player destructor
chain (`0x5dd543`, when the manager's `+0xc8` is not 1) and the player release `0x5e9aa0` (`0x5e9b5a`).

Addresses: `0x490a80`, `0x490a85`, `0x490a87`, `0x490a8d`, `0x490b48`, `0x490b4d`, `0x490b69`, `0x490b70`, `0x490b77`, `0x490c20`, `0x5dd543`, `0x5e9b5a`, `0x5dd539`, `0x5dd53e`, `0x5dd71c`, `0x5dd72c`, `0x490bc0`, `0xb4e310`
Confidence: verified
Proof: one
Test: none

`mov ecx,0x111` occurs once in the image, at `0x490b48`, followed by the only call to the event helper with
that id (`0x490b4d`); the event ids 273 and 371 (`mov ecx` before `0x490b77`) are fired nowhere else.
The span `0x490a8d` to `0x490b48` is 36 `call` instructions. A cinematic running (`0xb4e310`) adds
`CINEMATIC_STOP` (`0x490b77`). A world transfer reaches it through the manager's destruction (`0x467800` runs the
type map `0x467ad4` and calls each object's destructor with flag 0), and a reload through the shutdown.

## A reload

A reload runs the shutdown `0x490bd0` and then the initializer `0x48fbf0` (`0x495664`, `0x495669`). The
shutdown leaves the world (clearing the latch), signals `PLAYER_LOGOUT` (id 271), writes the saved variables
and destroys the interface; the initializer then arms `PLAYER_LOGIN`, loads the interface and the addons
and reaches `0x490168` with the player's GUID still set, so the cascade runs again and both login events
fire in the new Lua state. `PLAYER_LEAVING_WORLD` and `PLAYER_LOGOUT` reach the old state.

Addresses: `0x495664`, `0x495669`, `0x490bd0`, `0x490bee`, `0x490bf5`, `0x490c20`, `0x490c25`, `0x490c2a`, `0x48fe97`, `0x48feab`, `0x490168`, `0x703b80`, `0x703d90`
Confidence: verified
Proof: one
Test: none

`0x490bf5 je 0x490c25` skips the leave when the active GUID is 0, the skip side being the `PLAYER_LOGOUT`
block; in the world it is not taken. `0x48fe97` replaces the Lua state and `0x48feab` rebuilds the event
names, both after the shutdown has finished.

## A world transfer

`SMSG_NEW_WORLD` (opcode 0x3e, `0x401b00`) schedules `0x401bc0`, which destroys the object manager
(`0x467700`) and builds a new one for the new map. The destruction destroys the player, which runs the
leave function and fires `PLAYER_LEAVING_WORLD`. The initializer does not run, so `PLAYER_LOGIN` does not
fire again. When the server sends the player's create block the arrival takes the fresh-login route and
fires `PLAYER_ENTERING_WORLD`.

Addresses: `0x401b00`, `0x401756`, `0x401bc0`, `0x401bf7`, `0x401c1c`, `0x401c22`, `0x467700`, `0x467711`, `0x46771e`, `0x467800`, `0x467ad4`, `0x467ac8`, `0x4678f8`, `0x464ff0`
Confidence: verified
Proof: one
Test: none

`0x401bc0` calls `0x467700` (`0x401bf7`) and `0x464ff0` with the map id `[0x88262c]` (`0x401c22`). `0x467700` calls `0x467800` before it
clears the singleton `0xb41414` (`0x46771e`). The type map `0x467ad4` has 129 bytes; exactly eight
values dispatch (1, 3, 7, 9, 25, 33, 65, 129), and a player of type 25 reaches the destructor arm.

## The order at login

World entry runs inside the glue manager's per-frame function, a callee of event category 5 (`0x46a52d`),
after the same loop iteration's network drain (category 6, `0x420d55`); no network message or window
message is read while it runs. The interface initializer runs from there: it loads the `.toc` files and
the bindings, every addon not loaded on demand (each firing `ADDON_LOADED`, id 429), the saved variables
files (`VARIABLES_LOADED`, id 430), and then registers the chat settings reader (`chatwindows`), which is
only a deferral. The reader runs when the account data for slot 7 arrives, so `UPDATE_CHAT_WINDOWS`
and the colour events follow `VARIABLES_LOADED` and are not before it, and the enter cascade, fired by the
player's arrival, comes after those.

Addresses: `0x401570`, `0x46b930`, `0x46a52d`, `0x420d55`, `0x420d63`, `0x4015e7`, `0x401602`, `0x48f595`, `0x48f681`, `0x48fbf0`, `0x4900a3`, `0x4900b2`, `0x4900d6`, `0x498a20`, `0x5afe50`, `0x5af840`, `0x5afeae`, `0x5afa42`, `0x5afd01`, `0x46c236`, `0x401789`
Confidence: verified
Proof: one
Test: none

`0x5af840` (called at `0x4015e7`) sets all eight account-data sections to deferred. The register-or-fire
function `0x5afe50` stores the callback for a section whose pending byte is non-zero (`0x498a20` registers
`0x498a60` as section 7), and the arrival of the section's data clears the byte and fires it
(`0x5afa42`, `0x5afd01`, `0x5afeae`). `0x401570` registers the login-verify handler last (`0x401789`), after the
interface has loaded, so the packet that completes the player's create block can only arrive later.
The whole of `0x401570` is called from `0x46c236`, the per-frame function.

## Handlers and frames during the load

`0x401570` registers the game's packet handlers in three stretches. The object manager's constructor `0x464ff0`, called at `0x4015f3`, registers 0xa9, 0x1f6 and 0xaa through the connection's setter `0x537a60` (`0x465157`, `0x46516f`, `0x46518c`). The interface initializer `0x48f480` is called at `0x401602`. The body then registers 0x1cb, 0x1cd, 0x3e, 0x3f, 0x40 and 0x236 through `SetMessageHandler` (`0x5ab650`, `0x401734` to `0x401789`).
Of the 374 calls of `SetMessageHandler`, 358 are in the direct-call closure of `0x401570` and none in that of the interface initializer `0x48fbf0`. The load draws its progress bar with a callback and reads no network or window message.

Addresses: `0x401570`, `0x4015f3`, `0x401602`, `0x464ff0`, `0x465125`, `0x465140`, `0x465157`, `0x46516f`, `0x46518c`, `0x537a60`, `0x5ab650`, `0x401734`, `0x401745`, `0x401756`, `0x401767`, `0x401778`, `0x401789`, `0x48f480`, `0x48fbf0`, `0x48ffd4`, `0x4900ad`, `0x407fd0`, `0x406900`, `0x406920`, `0x406a60`, `0x58a960`, `0x58a96b`, `0x6ef160`, `0x6edd91`
Confidence: verified
Proof: one
Test: none

The closures follow direct calls and jumps only; indirect calls, the device slot among them, are not followed. The 16 sites outside the closure of `0x401570` belong to process start, authentication and the glue screens. `0x48ffd4` installs `0x407fd0` as the `.toc` progress callback (`0x6ef160`) and `0x4900ad` clears it. For each line `0x6edd91` calls it with the fraction.
It redraws only when the fraction is above the last drawn one (`0x882894`): `0x406900` computes the bar (`0x406920`), draws its quads (`0x406a60`) and calls slot `0x68` of the device with 3 (`0x58a96b`). The direct closure of `0x406900` (713 functions) holds no call of the message pump `0x42c9f0`, the event dispatcher `0x4245b0`, the dequeue `0x5384d0` or the drain handler `0x403620`. The packets that arrive meanwhile wait in the queue (`net/dispatch`).

## Welcome lines and the guild message

The guild message of the day shows above the realm's welcome lines although it arrives after them. Its
event handler fires `GUILD_MOTD` (id 284) at once, and an event that no frame has registered for is
dropped. The welcome lines are system chat messages, which are queued while the display latch
`0x8435fc` is set; the latch is 1 from start-up and is cleared only by the chat release inside the enter
cascade, after both login events, so the queue is shown last. A player's own chat before the cascade is
therefore kept, and a plain event fired before a frame registers is lost.

Addresses: `0x5e7180`, `0x5e725a`, `0x5e7277`, `0x5e728d`, `0x49db5c`, `0x49db62`, `0x49db8c`, `0x4989fd`, `0x498a03`, `0x703f50`, `0x70404e`, `0x8435fc`
Confidence: verified
Proof: one
Test: none

The guild event handler `0x5e7180` runs the profanity masker and then `0x703f50` with id `0x11c` and the
string (`0x5e728d`), with no queue on the path. `0x703f50` returns at `0x70404e` when the event has no
listener. The chat message handler sends the welcome line through `cmp [0x8435fc],ebx; je 0x49db9f`
(`0x49db5c`, `0x49db62`) to the queue call `0x49cae0` (`0x49db8c`) while the latch is set. The latch is 1 in the
file, set by `0x498a10` in the world teardown and cleared by `0x4989fd`.
