# core: closing the window

What the client does when its window is closed (the close box, Alt+F4, `WM_CLOSE`, a log-off), at the
glue screens and in the world: which window messages start a quit, how the context and its handlers
for categories 3 and 4 run, what goes on the wire, what is written to disk and in what order, and how
the process ends. A close sends no logout request and starts no timer or dialog; it runs the same
teardown as `ForceQuit` and the `quit` console command. The code is the window procedure and pump
(`core/input`), the event library's context teardown (`core/loop`), the client's start and stop
handlers and `WinMain`'s tail (`client`), and the C runtime's exit.

Roots: `0x42cfe0`, `0x423b50`, `0x420e60`

## Close messages

`WM_CLOSE` and `SC_CLOSE` queue kind 5 and return 0. Alt+F4 queues the F4 key-down (kind 7) first, then
reaches `DefWindowProcA`, whose `SC_CLOSE` queues kind 5. A press and release on the close box (hit test
20), or a double click on the system menu box (3) or the close box, calls `PostQuitMessage(0)`; the pump
turns the `WM_QUIT` into kind 0xe. `WM_QUERYENDSESSION` and `WM_ENDSESSION` have no arm and the chain hook is
null, so they reach `DefWindowProcA` and no client code runs: a log-off saves nothing. The graphics window
procedures take `WM_DESTROY` before the hook sees it and quit nothing (see the end of this entry).

Addresses: `0x42cfe0`, `0x42d292`, `0x42d287`, `0x42d28a`, `0x42d261`, `0x42d276`, `0x42d35c`, `0x42d5a9`, `0x42e067`, `0x42e130`, `0x42cbe4`, `0x42e000`, `0x42e0a0`, `0x42cfd0`, `0x884e6c`, `0x63a477`, `0x58c6d0`, `0x598750`, `0x58ca60`, `0x58ca79`, `0x58c640`
Confidence: verified
Proof: two
Test: none

The byte map `0x42d5e0` over messages 5 to 0xa3 leaves 5, 6, 0x10, 0x51, 0x7e and 0xa1 to 0xa3 off the
default; the map `0x42d69c` over 0x100 to 0x112 names 0x112 and the key messages; 0x11 and 0x16 take the
default `0x42d35c`, which tests the chain hook `0x884e6c` and calls `DefWindowProcA` (`0x42d5a9`). The only store
of the chain hook, `0x42cfd0`, has no caller. Device creation installs `0x42cfe0` as the game's hook (`0x63a477`);
the OpenGL and Direct3D window procedures (`0x58c6d0`, `0x598750`) pass every message they do not handle to it. `0x42d292` queues kind 5 and returns 0. The non-client messages 0xa1,
0xa2 and 0xa3 return 0 without `DefWindowProcA`, so the system never forms an `SC_CLOSE` for the close box:
release (`0x42e130`) and double click (`0x42e067`) call the imported `PostQuitMessage`; `0x42cbe4`
yields kind 0xe for the `GetMessageA` that returns 0. Message 2 (`WM_DESTROY`) never reaches the hook: both
graphics window procedures send it, through their jump tables, to a call of slot `+0x50` of the window object
with command 2, and return 0. The OpenGL method
(`0x58ca60`, case `0x58ca79`) releases the device and its context (`0x58c640`) and calls `0x58c140`.

## Confirm-close callback

Kind 5 and kind 0xe ask the callback `[0x883f5c]` before anything runs. The client installs `0x63c9a0` at
context start, which closes the current context and answers 0, so the quit flag stays clear and the
direct path through `0x423ca0`, `0x41f5d0` and `0x422030` does not run from the window. The context's run
state goes from 0 to 1 (`0x41f9b0`); a second close finds 1 and does nothing. In the same pass the loop skips
the context's category 6, frame time and after-frame work, and tears the context down.

Addresses: `0x402840`, `0x423a21`, `0x423a28`, `0x423af8`, `0x423b50`, `0x63b599`, `0x402858`, `0x63c9a0`, `0x63cdf0`, `0x41f9b0`, `0x420d6f`, `0x420e60`, `0x63c9b5`
Confidence: verified
Proof: two
Test: none

`0x423a21` calls `0x423b50`, which jumps to `[0x883f5c]` with `ecx = [0x883f60]`; the `je` at `0x423a28`
skips `0x423ca0` and the flag store on 0. `0x402858` (`ecx` 0) calls `0x63b460`, which stores `0x63c9a0` at
`0x63b599`; `0x63c9a0` is `call 0x63cdf0; xor eax,eax`, and `0x63cdf0` loads the current handle and jumps to
`0x41f9b0`. The stop handler removes the callback at `0x63c9b5`. `0x420d6f` finds run state 1 and calls
`0x420e60`; `core/loop#start-and-stop` has the state machine.

## Categories three and four

Category 3 has no handler. Category 4 has one, the stop handler `0x403640`, which the context
constructor registers with priority 1000. The context teardown `0x420e60` runs: the interactive count
decrement and, at zero, `0x422030` (shutdown event); category 3; run state 2; the queued messages
(`0x424ad0`); category 4; the context's deletion. The loop then finds the shutdown event and returns.

Addresses: `0x420e60`, `0x420e8f`, `0x420e9e`, `0x420eaf`, `0x420eb9`, `0x420ec7`, `0x422280`, `0x403640`, `0x41fc90`, `0x41fca0`, `0x424f00`, `0x422030`
Confidence: verified
Proof: two
Test: none

The census covers every `call` and `jmp` to `0x41fc90`, `0x41fca0` and `0x424f00` (64 sites, with the two
inside the library's own wrappers) and every dword of the image equal to one of the three. The category is
a constant at each site: 1, 2, 5, 6, 8 to 0x11, 0x1b and 0x1c, and 7 and 4 in the constructor
(`0x422265`, `0x422280`). None is 3. The control is that `0x422280` is found and its handler, passed by
`WinMain` at `0x402816`, is `0x403640`. The only other context creator, `0x41f600`, has no caller.

## Stop handler order

The stop handler `0x403640` calls, in order: `0x401ee0` with all arguments 0, the game teardown, which acts
only when `[0x882734]` is set (a world has been entered); `0x4036c0`, client systems; `0x5ab3d0`, network
services; `0x4036a0`; `0x443d30`; `0x66fb00`, the world renderer; `0x706de0`, models; `0x448d30`, textures;
`0x443520`; the connection library's shutdown `0x5bf300`, through the pointer `0x85f3f0`; `0x63c9b0`, which removes the callback and
shuts the console; `0x442140`; `0x4633c0`; `0x463bb0`; `0x419d30`. It returns 1. Outside `0x401ee0` its direct calls
write no file, signal no Lua event and send no packet.

Addresses: `0x403640`, `0x401ee0`, `0x882734`, `0x4017e5`, `0x40204d`, `0x4036c0`, `0x5ab3d0`, `0x4036a0`, `0x66fb00`, `0x706de0`, `0x448d30`, `0x63c9b0`, `0x85f3f0`, `0x5bf300`, `0x5bf250`
Confidence: verified
Proof: one
Test: none

`0x401ef3` returns when `[0x882734]` is clear; `0x4017e5` sets it at the end of the game initialization
and `0x40204d` clears it. The closure of the other fourteen callees, `0x5bf300` among them, by direct calls and jumps, reaches no
`CreateFile` wrapper (`0x42a460`, `0x42a6d0`), write (`0x42a5d0`), delete (`0x42a910`), move (`0x42a820`),
`0x5ab630` or `0x5379a0` send, and none of the 485 event sites of `tables/event_sites.tsv`. The pointer `0x85f3f0` keeps its image value `0x5bf300`: its one writer, `0x5bf250`, has no reference.
The control is the closure of `0x401ee0`, which reaches all three kinds (69 event sites). Indirect calls are
not followed.

## Network teardown

`0x5ab3d0` cancels the logon link (`0x5b3320`), then, when the world connection's connected flag
(`+0x1b00`) is set, disconnects it (`0x5ab1a0`, `0x537910`: state 6 to 7, the socket closes) and deletes it.
It sends no packet, so at the login and character screens the server sees only a closed connection.

Addresses: `0x5ab3d0`, `0x5b3320`, `0x5ab1a0`, `0x537910`, `0x5ab402`, `0x5ab40a`, `0x5ab41d`
Confidence: verified
Proof: one
Test: none

`0x5ab4a0` returns the logon session `[0xc28120]`; `0x5ab3e9` calls `0x5b3320` on it. `0x5ab402` tests
`[+0x1b00]` of `[0xc28114]` and `0x5ab40a` calls `0x5ab1a0`; `0x5ab41d` runs the virtual destructor (slot 0,
flag 1). `0x5ab1a0` calls `0x537910` and clears `+0x1b00`. `0x537910` stores state 7 when the state is 6
(`0x53791f`) and tail-jumps to `0x5b5370`, which has the socket close. The closure of `0x5ab3d0` reaches
no `0x5ab630`, `0x5379a0` or `0x5ab000`.

## World teardown

With a world entered, `0x401ee0` runs: it unregisters its category 5 handler `0x401ec0` and ten opcode handlers
(`0x1cb`, `0xc4`, `0xc8`, `0x1cd`, `0x3e`, `0x3f`, `0x40`, `0x99`, `0x9b`, `0x236`); frees the object array
`[0x8827c8]`; shuts thirteen modules (`0x401fc1` to `0x401ffd`); unregisters `0x616800`; destroys the object
manager (`0x402011`), whose player destructor runs the leave-world cascade `0x490a80` while the connection is
up; shuts the other world modules; runs the interface shutdown `0x491180` (`0x490bd0` first); and kills the
account-data timers (`0x5afe00`). With `edx` zero, as the stop handler passes it, it ends there.

Addresses: `0x401ee0`, `0x401f04`, `0x401f0e`, `0x401f6d`, `0x402011`, `0x467700`, `0x467800`, `0x5dd600`, `0x5dd530`, `0x5dd543`, `0x490a80`, `0x402039`, `0x491180`, `0x490bd0`, `0x5afe00`, `0x402062`, `0x4650e7`
Confidence: verified
Proof: one
Test: none

`0x467700` calls `0x467800`, which runs each object's slot-0 destructor; the player's `0x5dd600` calls `0x5dd530`
at `0x5dd72c` for the active player, and `0x5dd543` calls `0x490a80` when the manager's `+0xc8` is not 1; the
manager constructor stores 0 there (`0x4650e7`). `0x490bd0` runs after `0x467700` has stored 0 in `[0xb41414]`
(`0x46771e`), so it finds no active player (`0x490bee`) and skips the cancel-move and the sweep.
`0x402062` tests `edx` and jumps to the return.

## Leave-world cascade

The cascade `0x490a80` runs once per world (latch `0xb4b424`): 36 interface cleanups with no branch between
them, then `PLAYER_LEAVING_WORLD` (`0x490b4d`), the interaction closer `0x493310` and, when a cinematic is
playing (`0xb4e310`), `CINEMATIC_STOP` (`0x490b77`). Four of the steps send: the loot closer (`0x15f` and `0x13d`,
each only for what it finds open), the quest closer (`0x276`, when the quest frame holds an NPC), the trade
closer (`0x11c`, always) and, through the interaction closer, the trade partner's `0x11c`. The rest only clear
state.

Addresses: `0x490a80`, `0x490a8d`, `0x490adf`, `0x4bf220`, `0x5d4310`, `0x490ab2`, `0x4c1ca0`, `0x48f200`, `0x48f311`, `0x48f369`, `0x493910`, `0x490b5f`, `0x490b1b`, `0x4acc00`, `0x4acd50`, `0x4adde0`, `0x490b77`
Confidence: verified
Proof: one
Test: none

`0x4bf220` is `jmp 0x5d4310` (`0x11c`, no body). `0x4c1ca0` calls `0x48f200(1, 0)`. With a loot source in
`0xb71b48` it sends `0x15f` with the source GUID (`0x48f311`); for a disenchant window with contents
(`0x48f24a`) it calls `0x4c1fa0` instead, which takes the rows. For a dead unit's loot it then calls
`0x493910(1)` (`0x48f369`), which sends `0x13d` with a zero GUID when the target is that unit. `0x4acc00`, before the interaction closer, closes the mailbox hard (`cl` 1): no send, and the mailbox GUID is cleared, so
`0x4adde0`'s `0x284` does not run. Lua handlers that the closers' events run are not traced.

## Interaction closer

`0x493310` returns at once with no interaction target; otherwise it closes the window whose stored GUID matches:
the trade partner through `0x4bf4e0`, which reaches `0x5d4310` at `0x4bf527` for a zero GUID, and the quest
frame through `0x501130`. The quest closer `0x500bc0` (`0x490ac6`, before `0x493310`) is `edx` 1, `ecx` 0 and a
jump to `0x501130`: when `0xbe0810` holds a GUID that resolves to a unit (`0x501173`) it sends `0x276` with the
GUID and a byte 3 (`0x5011bf`), clears the GUID and signals `QUEST_FINISHED` (`0x501217`), whether or not that NPC is
the interaction target. The other windows it closes send nothing in their own bodies.

Addresses: `0x493310`, `0x4bf4e0`, `0x4bf527`, `0x5d4310`, `0x501130`, `0x500bc0`, `0x490ac6`, `0x501173`, `0x5011bf`, `0x501217`, `0x4931c0`, `0x60fb84`, `0x60fb60`
Confidence: verified
Proof: one
Test: none

The object and cache code those windows' closers call is not traced. Their closers end an interaction
through `0x4931c0`, which acts only when the GUID is the interaction target and then reaches `0x493310`; the event
`AUTOFOLLOW_END` (`0x60fb84`, state 3 of `0xc4d888`) lies on that path, in `0x60fb60`.
`0x4bf4e0` with a zero GUID tests the partner GUID `0xb71728` and in either case ends at `0x5d4310`. `0x501130` with
`edx` 0 sends nothing; `0x4933da` and `0x500bc7` pass `edx` 1.

## No logout request

A close sends no logout request and no account-data upload. The closure of `0x403640` contains no call to
`0x5ab000`, `0x5aaff0` or `0x5aaf80`, the only builders of `0x4a`, `0x4b` and `0x4e`; the closure's one
disconnect only closes the socket. The account-data section timers are killed without running
(`0x5afe00`), so no `0x20b` follows. At the login and character screens the stop handler sends nothing.

Addresses: `0x5ab000`, `0x5aaff0`, `0x5aaf80`, `0x5ab049`, `0x5afd30`, `0x5afe00`, `0x402048`, `0x537910`, `0x5ab630`, `0x5379a0`
Confidence: verified
Proof: two
Test: none

`0x5ab000` has callers `0x482f2f`, `0x48939b`, `0x4893bb`, `0x5aaff4` and `0x5ab3ba`; `0x5aaf80` has one, `0x48ab47`.
None is in the closure. `0x5ab049` is the only call of `0x5afd30`, which would run the armed upload timers, and
sits inside `0x5ab000`. `Script::ForceQuit` (`0x48ab60`) is `0x401560` with code 0: the same close.
`0x5379a0` sends in state 6; `0x5b5630` calls `send` once when its queue is empty.

## Packets sent in the world

With the socket still open and each packet written at once when nothing is queued, a close in the world
sends, in this order: `0x27f` for each party or raid member's player object, from the destructors; from the
cascade, `0x15f` (a loot window with a source), `0x13d` with a zero GUID (that source is a dead unit and the
target), `0x276` (the quest frame holds an NPC) and `0x11c` (always); `0x11c` again for a trade partner that is
the interaction target; `0x11c` from `0x490be4` when the interface shutdown starts. The shutdown's
`0x493540` sends nothing: its zero GUID takes `0x4936c4` to `0x493910`, which returns at `0x4939df` with no object
manager; the `0x13d` at `0x493857` needs a nonzero GUID.

Addresses: `0x5dd69b`, `0x4e85d0`, `0x4badc0`, `0x4e8646`, `0x4bae27`, `0x5d4310`, `0x490be4`, `0x493540`, `0x4936c4`, `0x4939df`, `0x493857`, `0x493a2a`, `0x48f2da`, `0x5011bf`, `0x4bf220`, `0x46771e`
Confidence: verified
Proof: one
Test: none

`0x5dd600` calls `0x4e85d0` and `0x4badc0` with mode 0 for a player whose released bit
is clear (`0x5dd69b`); each looks the GUID up in the party table `0xbc6f48` or the raid roster `0xb712a8` and
sends `0x27f` (`0x4e8646`, `0x4bae27`). `0x48f200` sends `0x15f` at `0x48f2da` when `cl` is 1 and a loot source is set.
`0x5d4310` builds `0x11c` and sends it through `0x5ab630` with no test. `0x467700` stores 0 in `0xb41414`
(`0x46771e`) after the objects are gone, so the cascade runs with a manager and `0x490bd0` without one. A
zero GUID in `0x493540` skips to `0x493910` with `ecx` 1, whose `0x13d` (`0x493a2a`) sits behind a manager.

## Events a close signals

No dialog and no timer runs. In the world a close signals `PLAYER_LEAVING_WORLD` (273, `0x490b4d`), then
`PLAYER_LOGOUT` (271, `0x490c2a`), both unconditionally; the Lua state is still alive, so frame and add-on
handlers run. Before them, each closer that finds its window open signals that window's event (`QUEST_FINISHED`,
`TRADE_CLOSED`, `TRAINER_CLOSED`, `MAIL_CLOSED` and the like); the target release signals `PLAYER_TARGET_CHANGED`
(`0x493a73`) with its `0x13d`, `AUTOFOLLOW_END` fires for a following player on the end-interaction path, and
`CINEMATIC_STOP` (`0x490b77`) fires after `PLAYER_LEAVING_WORLD` when a cinematic plays. `PLAYER_QUITING` and
`PLAYER_CAMPING` never fire: their one signal site is the logout reply handler. At the login and character
screens a close signals none.

Addresses: `0x490b4d`, `0x490c2a`, `0x490c25`, `0x490bf5`, `0x703e50`, `0x401ef3`, `0x493a73`, `0x60fb84`, `0x490b77`, `0x5aaef0`, `0x5aaf17`
Confidence: verified
Proof: two
Test: none

`0x490c25` loads `0x10f` for `0x703e50` and is reached from both sides of the guard `0x490bf5`, so
`PLAYER_LOGOUT` is unconditional in `0x490bd0`; `0x490b48` loads `0x111` once in the image. No event site lies in
the closure of the stop handler outside `0x401ee0` (`Stop handler order`). The table of event sites has one
site for events 276 and 277, `0x5aaf17`, inside `0x5aaef0`; no direct call reaches that handler from `0x403640` or
`0x403bc0`, and its address is stored once in the image, in the reply slot table that the packet dispatch calls.

## Interface shutdown writes

In the world `0x490bd0` runs after `PLAYER_LOGOUT` and writes, in this order: `Config.wtf` (`0x490c50`, only when a
setting changed since the last save); the chat settings, section 7, `chat-cache.txt` (`0x498a40`); the macros,
sections 4 and 5, `macros-local.txt` and `macros-cache.txt` (`0x4f0c60`, when the save flag `0xbdcbf8` is set,
which it clears); the key bindings, `bindings-cache.wtf` (`0x4b65e0`: sections 2 and 3 when bit 1 of the mask
`[[0xb71290]+0xd4]` is set, else section 3 when bit 2 is); the layout, section 6, `layout-cache.txt`
(`0x490c79`); the flat saved-variables file `WTF\Account\<account>\SavedVariables.lua` (`0x491730`); each loaded
add-on's account and character `SavedVariables\<addon>.lua` files (`0x51f650`); and `AddOns.txt` for each
changed character list (`0x51ef20`). The section files are the account-data files under `WTF\Account`
(`net/accountdata`), written locally; `config-cache.wtf` is not.

Addresses: `0x490bd0`, `0x490c50`, `0x63d980`, `0x490c55`, `0x490c5a`, `0x490c64`, `0x490c79`, `0x490c7e`, `0x490c83`, `0x490c88`, `0x5afed0`, `0x5afee3`, `0x5af845`, `0x5afa42`, `0x5afd01`, `0x5afd30`, `0x5ab049`, `0x5afe00`, `0x4917a1`, `0x42a820`
Confidence: verified
Proof: two
Test: none

`0x5afed0` reads the section's pending byte (`0x5afee3`) and writes nothing while it is set. The byte is 1 from
game start (`0x5af845`) and clears when the server's digest for the section is checked (`0x5afa42`) or the
section arrives (`0x5afd01`): a client that closes before the digests arrive writes none of the sections 2 to 7.
A write that happens arms a 30-second upload timer; `0x5afe00` clears every such timer without running it, so the
server keeps its older copies and sends no `0x20b`. The saved-variables writers move an existing file to `.bak`
first (`0x4917a1` calls `0x42a820`, the move), delete a file whose add-on has no variables, and delete the
realm-less character file. `AddOns.txt` lines are `name: enabled` or `name: disabled`. `ui/addons` has the saved
variables, `net/accountdata` the file names. The call `0x4effb0` between the macros and the bindings frees the macro
tables and writes nothing.

## Tail

After the loop returns, `WinMain` runs `0x403bc0` on every route, at the login and character screens as in the
world: the input window's IME reassociation (`0x42c960`), the time manager, the event library's free (`0x41f5a0`),
the console device, the deletion of `RunOnce.wtf` when it exists (`0x63d910`), the registry's shutdown, which
writes `Config.wtf` again when it is dirty (`0x63daf0`, `0x403bec`; the earlier write in the world cleared the flag),
and, for a nonzero quit code, the error text. `WinMain` then returns 0 and the process exits through the C
runtime.

Addresses: `0x403bc0`, `0x42c960`, `0x41f5a0`, `0x63d910`, `0x63daf0`, `0x403bec`, `0x4021e9`, `0x401560`, `0x882738`, `0x40a3ea`
Confidence: verified
Proof: two
Test: none

`0x4021e4` calls `0x41f5c0`, then `0x403bc0` (`0x4021e9`). `0x401560` stores its code in `0x882738` before
closing the context; a window close leaves it 0.

## C runtime exit

`0x4021e4` then calls `0x64cbf0`, which registers `0x64cc30` (`0x64cc18`) last. `WinMain` returns to the start
code (`0x40999b`), which calls `_exit`. Storm's exit function, registered last, runs first and calls `__cexit`,
whose `_doexit` runs every other exit function from the last registered to the first; Storm then shuts down and
`__exit` ends with `ExitProcess`.

Addresses: `0x4098c0`, `0x40999b`, `0x40a30d`, `0x40a32f`, `0x40a31e`, `0x40a34d`, `0x40a39e`, `0x40a3ea`, `0x64cbf0`, `0x64cc18`, `0x64cc30`, `0x644ff0`
Confidence: verified
Proof: one
Test: none

`0x40999b` calls `WinMain` and `0x4099a4` calls `_exit`, which calls `_doexit` with quick 0. `0x64cc30` counts
its entry and calls `__cexit` (`0x40a32f`: `_doexit` with quick 0, return to caller), whose walk
`0x40a39e` meets `0x64cc30` again and skips it; it then calls `0x644ff0` and `__exit` (`0x40a31e`: quick 1), which
skips the table and reaches `ExitProcess` (`0x40a3ea`).

## Cache files at exit

At exit each cache that persists writes its `.wdb` file anew from every valid record, in this order:
`wowcache.wdb`, `itemtextcache.wdb`, `pagetextcache.wdb`, `questcache.wdb`, `npccache.wdb`, `itemcache.wdb`,
`itemnamecache.wdb`, `gameobjectcache.wdb`, `creaturecache.wdb`. The four caches that do not persist write
nothing. A log-off or the system ending the process runs none of them.

Addresses: `0x56b040`, `0x569570`, `0x564320`, `0x562630`, `0x55d120`, `0x55b650`, `0x559c10`, `0x558180`, `0x5566d0`, `0x409aef`, `0x829e90`, `0x829ec0`
Confidence: verified
Proof: one
Test: none

The initializer table slots `0x829e90` to `0x829ec0` register the exit functions with `_atexit` in this
order: creature, game object, item name, item, npc, name (no persist), guild (none), quest, page text, pet
name (none), petition (none), item text, wow cache. The nested `_doexit` of the previous entry runs the last registered first. `dbcache` has
the file layout; `warden/cache` the wow cache's.
