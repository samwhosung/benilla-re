# core: Mac key input

How the 1.12.1 Mac client turns Carbon keyboard events into the key codes its binding layer names.
Every `mac:` address here is the i386 slice's (`docs/METHOD.md`, The binary decides;
`symbols/README.md`, The Mac client); a `0x` address is `WoW.exe`'s. The keycodes are Carbon's
virtual keycodes, the `kVK_` constants of `Events.h`.

`InputCarbonEvents.cpp` holds the Carbon event handler `mac:089e3e` and the pump `mac:08abe4`. The
handler maps a key's keycode through a 128-entry live table, `mac:991d20`, which starts as the fixed
table `mac:5bf320` and takes the keyboard layout's characters in its free slots, and queues
five-dword records in a 32-slot ring. The event library's switch `mac:1d6326` turns the key records
into the event categories `WoW.exe` uses (`core/input`, `core/events`), and the binding layer's
chord builder `mac:1de176` names them. The Windows counterpart of the key table is
`core/input#key-codes`.

Roots: `mac:089e3e`

## Carbon keyboard events

`mac:089e3e` is a Carbon event handler, `OSStatus (EventHandlerCallRef, EventRef, void *)`, that
switches on the event's class. Of the keyboard class (`keyb`) it takes kinds 1, 2 and 3, a raw key
down, repeat and up (Raw key events), and kind 4, a modifiers change (Modifier keys), and answers
`noErr` (0) for each; any other keyboard kind it answers `eventNotHandledErr` (−9874) untouched. The
text-input class (`text`) goes to `mac:033a7c`, whose answer it returns (Text input events), and the
command class (`cmds`) to the handler registered for the command (Edit menu commands).

Addresses: `mac:089e3e`, `mac:089e5e`, `mac:089e6b`, `mac:089e73`, `mac:08a003`, `mac:08a007`, `mac:08a011`, `mac:08a01b`, `mac:089f15`, `mac:089e85`, `mac:08a277`, `mac:089f31`, `mac:991a00`
Confidence: verified
Proof: one
Test: none

`GetEventClass` and `GetEventKind` are called at `mac:089e5e` and `mac:089e6b`, the kind kept at
`ebp−0x6c`. `cmp ebx,0x6b657962` (`keyb`, `mac:089e73`) leads to `mac:08a003`, where `jb`
(`mac:08a007`) sends kind 0 to `mac:089f15`, `mov eax,0xffffd96e`; kinds 1 to 3 pass `jbe`
(`mac:08a011`) to the raw key code and kind 4 passes the `jne` at `mac:08a01b` to the modifier code,
both of which end in `xor eax,eax`. `text` is compared at `mac:089e85`, and `mac:08a277` passes the
next handler, the event and the kind to `mac:033a7c`; `cmds` is compared at `mac:089f31`. The
handler's address occurs once in the slice as a dword, an immediate a static initializer passes to
`NewEventHandlerUPP`, whose result it stores at `mac:991a00`; no `e8` or `e9` in the code sections
targets it, where the same scan finds the one call of `mac:08abe4`.

## The fixed key table

`mac:5bf320` holds 128 signed dwords: the client key code for each keycode 0 to 0x7f, or −1 for a
slot the keyboard layout fills (Filling the table from the layout). Raw key events read its live
copy, `mac:991d20`. Its 66 fixed entries, and the rest:

| keycodes | key codes |
|---|---|
| 0x12 to 0x17, 0x19, 0x1a, 0x1c, 0x1d (1 2 3 4 6 5 9 7 8 0) | the digit characters 0x31 to 0x39 and 0x30 |
| 0x24 Return, 0x4c keypad Enter | 0x201 |
| 0x30 Tab, 0x31 Space, 0x33 Delete, 0x35 Escape | 0x203, 0x20, 0x202, 0x200 |
| 0x38 and 0x3c Shift, 0x3b and 0x3e Control, 0x3a and 0x3d Option, 0x39 Caps Lock | 0, 1, 2, 0x20e |
| 0x41, 0x43, 0x45, 0x4b, 0x4e keypad `.` `*` `+` `/` `-` | 0x10e, 0x10c, 0x10a, 0x10d, 0x10b |
| 0x47 keypad Clear, 0x51 keypad `=` | 0x20f, 0x30c |
| 0x52 to 0x59, 0x5b, 0x5c keypad 0 to 9 | 0x100 to 0x109 |
| 0x7a, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, 0x65, 0x6d, 0x67, 0x6f, F1 to F12 | 0x300 to 0x30b |
| 0x69 F13, 0x6b F14, 0x71 F15, 0x6a F16 | 0x212, 0x210, 0x211, 0x30d |
| 0x72 Help, 0x73 Home, 0x74 Page Up, 0x75 forward Delete, 0x77 End, 0x79 Page Down | 0x208, 0x20a, 0x20c, 0x209, 0x20b, 0x20d |
| 0x7b, 0x7c, 0x7d, 0x7e, the arrows left, right, down, up | 0x204, 0x206, 0x207, 0x205 |
| the other 62: the letters and punctuation, 0 to 0x11, 0x18, 0x1b, 0x1e to 0x23 and 0x25 to 0x2f, the ISO key 0x0a among them; 0x32, 0x34, Command 0x36 and 0x37, fn 0x3f, F17 to F20 (0x40, 0x4f, 0x50, 0x5a), the volume keys 0x48 to 0x4a, the JIS keys 0x5d to 0x5f, 0x66 and 0x68, and 0x42, 0x44, 0x46, 0x4d, 0x6c, 0x6e, 0x70, 0x7f | −1 |

Addresses: `mac:5bf320`, `mac:991d20`, `mac:08a351`, `mac:08a359`, `mac:08a368`
Confidence: verified
Proof: one
Test: `core::mac_fixed_key_codes`

The table is the source of the one `memcpy` that writes the live table (`mac:08a368`: length 0x200
at `mac:08a351`, source at `mac:08a359`, destination `mac:991d20`), and that operand is the only
reference to its address in the slice. `core::mac_fixed_key_codes` reads its 512 bytes from the
slice and compares them with the transcription in `crates/core`. A modifier keycode's entry is read
only for a raw key event with that keycode (Modifier keys).

## Filling the table from the layout

Each raw key event reads the current `KCHR` with `GetScriptManagerVariable(smKCHRCache)` (38). When
it differs from the one kept at `mac:991f20`, the handler copies the fixed table over the live one
and, if the `KCHR` is not null, fills every −1 slot from it, then keeps it. For slot `k` it calls
`KeyTranslate(KCHR, k, &state)` with the state 0, no modifier and key down, and when that leaves the
state nonzero calls it again with that state, taking the second result. The result's low byte, when
nonzero, is uppercased if it is `a` to `y`, read as one MacRoman character and converted to Latin-1;
the slot takes the Latin-1 byte when the conversion writes one nonzero byte, and otherwise stays −1.

Addresses: `mac:08a33d`, `mac:08a345`, `mac:08a368`, `mac:08a374`, `mac:08a37c`, `mac:08a3a3`, `mac:08a3af`, `mac:08a3c3`, `mac:08a3ce`, `mac:08a848`, `mac:08a3da`, `mac:08a3df`, `mac:08a40f`, `mac:08a46b`, `mac:08a475`, `mac:08a47d`, `mac:08a482`, `mac:08a494`, `mac:991f20`, `mac:991d20`
Confidence: verified
Proof: one
Test: none

`mac:08a33d` passes 0x26, and `cmp eax,[mac:991f20]` at `mac:08a345` skips the copy when they match.
A second call (`mac:08a374`) gives the `KCHR` the fill reads; a null one skips the loop
(`mac:08a37c`), and `mac:08a494` keeps the first either way. The loop runs `ebx` from 0 to 0x7f and
acts on `cmp dword [esi],-1` (`mac:08a3a3`); the keycode goes as `movzx edi,ax` (`mac:08a3af`), so
no modifier byte and no key-up bit. A nonzero state (`mac:08a3ce`) calls `KeyTranslate` again with
it and the keycode `edi` (`mac:08a848`). `test dl,dl` skips a zero byte (`mac:08a3da`);
`lea eax,[edx-0x61]; cmp al,0x18; ja` (`mac:08a3df`) bounds the uppercase to 0x61 to 0x79.
`CFStringCreateWithBytes(NULL, &b, 1, 0, false)` (`mac:08a40f`) reads the byte as MacRoman, encoding
0, and `CFStringGetBytes(s, {0, 1}, 0x201, 0, false, &b, 1, &used)` (`mac:08a46b`) converts it to
Latin-1, 0x201, with no loss byte; `used` and the byte are tested at `mac:08a475` and `mac:08a47d`
before `mac:08a482` stores the byte.

## Two faults in the fill

Only `a` to `y` are uppercased, so the Z key's slot holds 0x7a and the chord builder names it `z`,
while every other key whose layout character is `a` to `y` is named in capitals (Key names on the
Mac). The live table and the kept `KCHR` start as zeros, and only the rebuild writes them: while
every raw key event since start-up has seen a null `KCHR`, no copy is made, and every keycode below
0x80 maps to code 0, `SHIFT`.

Addresses: `mac:08a3df`, `mac:08a3e3`, `mac:991d20`, `mac:991f20`, `mac:08a345`, `mac:08a34b`, `mac:08a4a3`, `mac:08a494`
Confidence: verified
Proof: one
Test: none

`cmp al,0x18` (`3c 18` at `mac:08a3df`): 0x7a − 0x61 is 0x19, so `ja` skips the subtraction of 0x20
at `mac:08a3e3`. `mac:991d20` and `mac:991f20` lie in `__DATA,__bss`, zero at load. A dword census
of the slice finds three references to `mac:991d20` (the `memcpy` destination, the loop's base and
the read at `mac:08a4a3`) and two to `mac:991f20` (the compare at `mac:08a345` and the store at
`mac:08a494`); the same census finds the one reference to `mac:5bf320`. With both null, `je` at
`mac:08a34b` skips the copy, and the read finds 0, which is not −1.

## Raw key events

A raw key event reads its modifiers (`kmod`) and keycode (`kcod`). While the text-input flag
`mac:6e10c0` is set, the event is first passed to the next handler. A keycode above 0x7f, a live
slot of −1, and code 0x202 (`BACKSPACE`) while the flag is set queue nothing; any other key queues a
record of kind 8 for an up or 7 for a down or repeat, with its code and three zeros. Last, with
Command not held, the flag clear and the event not an up, the event's own character (`kchr`), if
0x20 to 0x7f, queues a record of kind 1 with that character and the count 1. Text input events queue
characters and `BACKSPACE` too.

Addresses: `mac:08a2bc`, `mac:08a2c4`, `mac:08a2cd`, `mac:08a2ea`, `mac:08a32b`, `mac:08a49e`, `mac:08a4aa`, `mac:08a4af`, `mac:08a4b8`, `mac:08a4c2`, `mac:08a4f5`, `mac:08a51e`, `mac:08a527`, `mac:08a530`, `mac:08a56b`, `mac:08a577`, `mac:08a5a7`, `mac:03342c`, `mac:03337e`, `mac:0334bc`, `mac:0338c4`, `mac:6e10c0`
Confidence: verified
Proof: one
Test: none

`GetEventParameter` reads `kmod` as a 32-bit word at `mac:08a2bc`, and `shr edx,8; and edx,1`
(`mac:08a2c4`) keeps Command; `kcod` follows at `mac:08a32b`. `mac:03342c` returns the flag, and
`mac:03337e` stores its argument in it, calling `ActivateTSMDocument` first for a nonzero one and
`FixTSMDocument` and `DeactivateTSMDocument` for 0, unless the flag holds it. The one other store to
the flag is the clear at `mac:0338c4`, in `mac:0334bc`, which creates and activates the
text-services document and, if the flag is set, deactivates it. `CallNextEventHandler` is at
`mac:08a2ea`. `ja` on the keycode (`mac:08a49e`) and `cmp ebx,-1; je` (`mac:08a4aa`) skip the queue,
as does `cmp ebx,0x202` (`mac:08a4b8`) after a nonzero `mac:03342c` (`mac:08a4af`).
`cmp [ebp-0x6c],3; sete cl; add ecx,7` (`mac:08a4c2`) gives the kind, which `mac:08a4f5` stores
before the code and the zeros. The character path tests Command (`mac:08a51e`), the flag
(`mac:08a527`) and the up (`mac:08a530`), reads `kchr` as one `TEXT` byte (`mac:08a56b`), bounds it
with `lea eax,[edx-0x20]; cmp al,0x5f; ja` (`mac:08a577`) and stores kind 1 (`mac:08a5a7`) with the
count 1.

## Modifier keys

A modifiers change reads the new modifier word (`kmod`). For each of the bits Shift (0x200), Control
(0x1000) and Option (0x800), in that order, that differs from the last word `mac:991d08`, it queues
key code 0, 1 or 2: kind 7 when the bit is now set, 8 when clear. It then keeps the new word. No
other bit is tested, so Command (0x100), Caps Lock (0x400) and fn queue nothing; and a key pressed
with Command held queues as any other (Raw key events), with nothing of Command in the modifier
mask, which has no bit for it.

Addresses: `mac:08a056`, `mac:08a060`, `mac:08a06c`, `mac:08a07f`, `mac:08a0b1`, `mac:08a0d3`, `mac:08a118`, `mac:08a13a`, `mac:08a17f`, `mac:08a1a1`, `mac:991d08`
Confidence: verified
Proof: one
Test: none

`GetEventParameter` reads `kmod` at `mac:08a056`, and `xor ebx,[mac:991d08]` (`mac:08a060`) gives
the changed bits. `test bh,0x2` (`mac:08a06c`), `test bh,0x10` (`mac:08a0d3`) and `and bh,0x8`
(`mac:08a13a`) select 0x200, 0x1000 and 0x800. Each computes
`and eax,bit; cmp eax,1; sbb ecx,ecx; not ecx; add ecx,8` (ending at `mac:08a07f` for Shift), 7 when
the bit is set and 8 when clear, and stores the code 0, 1 or 2 at `+4` of the record (`mac:08a0b1`,
`mac:08a118`, `mac:08a17f`). `mac:08a1a1` stores the new word over the changed bits stored at
`mac:08a066`.

## Text input events

`mac:033a7c` answers the text-input class. For kind 2, a key event's Unicode text, it reads the text
as UTF-16 (`utxt`, at most 0x200 bytes) and the modifiers of the raw key event behind it. With
Command not held, each UTF-16 unit of 0x20 or above queues a record of kind 1 with the unit and the
count 1, and U+0008 queues `BACKSPACE` (0x202) down and up, kinds 7 and 8, with three zeros; another
unit below 0x20 queues nothing, and it answers `noErr`. Kinds 1 and 3, the input method's, are not
read here; another kind it answers `eventNotHandledErr`. Which raw keys Carbon turns into these
events is the system's, not the slice's.

Addresses: `mac:033a7c`, `mac:033a92`, `mac:033aa9`, `mac:033afc`, `mac:034295`, `mac:033b40`, `mac:033dd5`, `mac:033de5`, `mac:033df0`, `mac:033e21`, `mac:033e29`, `mac:033e0b`, `mac:033e55`, `mac:034758`, `mac:034784`, `mac:033fab`
Confidence: verified
Proof: one
Test: none

`cmp eax,2; je` (`mac:033a92`) takes kind 2, and kinds other than 1 to 3 reach `mov eax,0xffffd96e`
(`mac:033aa9`). `GetEventParameter` reads the raw key event (`tske`, `mac:033afc`), then its `kmod`
(`mac:034295`), 0 when either is missing, and the text (`tstx`, `mac:033b40`) into a 0x200-byte
buffer. The type `utxt` (`mac:033dd5`), a size up to 0x200 (`mac:033de5`) and bit 0x100 of `kmod`
clear (`test byte [ebp-0x2f],1`, `mac:033df0`) lead to the loop over the size halved (`mac:033e21`):
`cmp ax,0x1f; jbe` (`mac:033e29`) sends units below 0x20 to `cmp ax,8` (`mac:033e0b`), and the rest
to `mac:089dd6` with kind 1, the unit and 1 (`mac:033e55`); U+0008 calls it with kind 7 and with
kind 8 for 0x202 (`mac:034758`, `mac:034784`). Every kind 2 path returns through `xor eax,eax`
(`mac:033fab`), and these three are the only queue calls it reaches.

## The input ring

Records wait in a ring of 32 five-dword slots, `mac:991a80`, with the read index `mac:991d00` and
the write index `mac:991d04`. It holds at most 31: queuing into a full ring drops the oldest. The
handler queues the fifth dword as 0. The pump `mac:08abe4` yields the oldest record when one waits.
Otherwise it pulls and dispatches Carbon events without waiting while fewer than 16 records wait,
until it has twice found no event or 16 records; then, if any record waits, it writes one value read
once into every waiting record's fifth dword and yields the oldest, and otherwise yields nothing.

Addresses: `mac:991a80`, `mac:991d00`, `mac:991d04`, `mac:08a4dd`, `mac:08a86b`, `mac:08a513`, `mac:08abed`, `mac:08ac9e`, `mac:08acc4`, `mac:08ace8`, `mac:08ad85`, `mac:08acea`, `mac:08acf2`, `mac:08ad00`, `mac:08ad21`
Confidence: verified
Proof: one
Test: none

A queue site takes the next write index, 0 after 0x1f, and when it equals the read index advances
the read index the same way (`mac:08a4dd`, `mac:08a86b`) before storing at `mac:991a80` plus 20
times the old write index and storing the new one (`mac:08a513`). `mac:08abed` compares the indices
and copies the oldest record out through the five pointers.
`ReceiveNextEvent(0, NULL, 0.0, true, &e)` (`mac:08ac9e`) is followed by `SendEventToEventTarget` to
the dispatcher target (`mac:08acc4`) and `ReleaseEvent`. The count waiting is `write − read`, plus
32 when negative; above 15 (`mac:08ace8`) or on an error (`mac:08ad85`) the try count rises
(`mac:08acea`), and the loop ends at 2 (`mac:08acf2`). `mac:08ad00` reads the value and `mac:08ad21`
stores it at `+0x10` of each record from read to write.

## Records to key events

The event library's switch `mac:1d6326` takes the pump's records. Kind 7 becomes category 8 and kind
8 category 9, with the payload {code, modifier mask, the record's third dword, its fifth}; for codes
0 to 2 it first sets (7) or clears (8) bit `1 << code` of the mask `mac:9a364c`. Kind 1 becomes
category 1 with {character, mask, count}. Kind 6 zeroes the mask before category 2. Every kind 7 or
8 record is queued with its third dword 0, so every key payload carries the count 0.

Addresses: `mac:1d6326`, `mac:1d635a`, `mac:1d636c`, `mac:1d64a4`, `mac:1d65d4`, `mac:1d6644`, `mac:1d664d`, `mac:1d666e`, `mac:1d668b`, `mac:1d66a2`, `mac:1d66c2`, `mac:1d693d`, `mac:9a364c`, `mac:089dd6`, `mac:02684a`, `mac:026872`, `mac:02689a`, `mac:0268c6`, `mac:034758`, `mac:034784`, `mac:08a0b8`, `mac:08a11f`, `mac:08a186`, `mac:08a4fe`
Confidence: verified
Proof: one
Test: none

`mac:1d635a` calls the pump and `mac:1d636c` jumps through a table on kinds 0 to 0xe: kind 1 to
`mac:1d64a4`, 6 to `mac:1d65d4`, 7 to `mac:1d6644` and 8 to `mac:1d668b`. Kind 7 tests
`cmp ecx,2; jle` (`mac:1d664d`) to `mac:1d693d`, `shl eax,cl; or`; kind 8 rotates −2 by the code and
ANDs it into the mask (`mac:1d66a2`). Both build the payload at `ebp−0x50` and pass categories 8
(`mac:1d666e`) and 9 (`mac:1d66c2`). Of the 15 calls of `mac:089dd6`, the six that pass kind 7 or 8
(`mac:02684a`, `mac:026872`, `mac:02689a`, `mac:0268c6`, `mac:034758`, `mac:034784`) pass a third
dword of 0, and the other nine pass kinds 0, 1, 3, 4 and 5. Of the code's other stores into the
ring's slots, the four of kind 7 or 8 store 0 there (`mac:08a0b8`, `mac:08a11f`, `mac:08a186`,
`mac:08a4fe`), and the rest store kinds 0, 1, 5 and 9 to 0xd.

## The dispatch

The dispatch `mac:1d4f2c` calls each handler in the category's list, at `+0x70` plus 12 times the
category in the context, with the payload, until one returns 0. Before that, on categories 8 and 9,
it removes the code from the context's held-key list and, on 8, appends it again and dispatches
category 0xa instead when it was held. So a Carbon repeat, queued as a down, arrives as category
0xa.

Addresses: `mac:1d4f2c`, `mac:1d4f59`, `mac:1d53d6`, `mac:1d5705`, `mac:1d5602`, `mac:1d5695`, `mac:1d5294`, `mac:1d4f97`, `mac:1d4f9d`, `mac:1d50a2`, `mac:1d50ba`, `mac:1d50bf`
Confidence: verified
Proof: one
Test: none

A jump table on the category (`mac:1d4f59`) sends 8 and 9 to a walk of the list at `+0x1e0` of the
context, which compares each node's `+8` with the code (`mac:1d53d6`), unlinks and frees a match and
remembers it (`mac:1d5705`); for 8, a `cmovne` of 0xa into the category (`mac:1d5602`) and a new
12-byte node holding the code (`mac:1d5695`) follow, and the bookkeeping goes on to the handlers
(`mac:1d5294`). There `lea eax,[esi+esi*2]` and `lea edi,[edx+eax*4+0x70]` (`mac:1d4f97`,
`mac:1d4f9d`) give the list; a node whose `+0x14` is set is passed over (`mac:1d50a2`), and
`call [esi+8]` (`mac:1d50ba`) calls the handler with the payload and the node's `+0xc`, the walk
going on while it returns nonzero (`mac:1d50bf`).

## Key names on the Mac

The chord builder `mac:1de176` names a key payload. It fails for a count above 1 or a name that does
not fit, clears a modifier code's own bit, writes `ALT-`, `CTRL-` and `SHIFT-` for the held bits
from the table `mac:699fc8`, walked from its last entry, then the code's name. The names are those
of `ui/keybindings#key-names`: 0, 1 and 2 `SHIFT`, `CTRL`, `ALT`; 0x20 `SPACE`; 0x21 to 0xff the
character in UTF-8 (`mac:3260aa`); 0x100 to 0x10e, 0x200 to 0x20f, 0x212 and 0x300 to 0x30d as
listed there; any other code `UNKNOWN`. So F13 is `PRINTSCREEN`, F14 and F15 are `UNKNOWN`, F16 is
`F16`, and the Z key is `z`.

Addresses: `mac:1de176`, `mac:1de18e`, `mac:1de1ab`, `mac:1de1b6`, `mac:1de1ea`, `mac:1de218`, `mac:1de2b1`, `mac:1de30e`, `mac:1de334`, `mac:1de26f`, `mac:1de3a4`, `mac:1de48a`, `mac:699fc8`, `mac:3260aa`
Confidence: verified
Proof: one
Test: `core::mac_chord_prefixes`

It lies among functions that pass the path of `UIBindings.cpp`, and no other file's path is passed
in that stretch. `cmp [eax+0x18],1; jbe` (`mac:1de18e`) passes a count of 0 or 1, and `rol` of −2 by
the code clears its bit for codes up to 2 (`mac:1de1ab`). The prefix loop starts at `mac:699fc8`
(`mac:1de1b6`), reads an entry's code and string at `+0x10` and `+0x14`, and steps down 8 through
three entries; each copy first checks the room left (`mac:1de1ea`).
`lea eax,[esi-0x21]; cmp eax,0xde; jbe` (`mac:1de218`) sends 0x21 to 0xff to `mac:3260aa`
(`mac:1de2b1`), which writes one byte below 0x80 and two, `0xc0 | c >> 6` and `0x80 | c & 0x3f`, up
to 0x7ff. The rest is a tree of compares: `cmp esi,0x212; je` (`mac:1de30e`) loads `PRINTSCREEN`;
0x210 and 0x211 fail `cmp esi,0x20f; jne` (`mac:1de334`) to `UNKNOWN` (`mac:1de26f`); 0x300 to 0x30b
format `F%d` with the code less 0x2ff (`mac:1de3a4`); 0x30d is `F16` (`mac:1de48a`).

## Edit menu commands

The menu set-up `mac:0268f2` registers `mac:0267e0` for the commands `undo`, `cut `, `copy`, `past`
and `sall` through `mac:08bf80`, which stores a command ID with its handler. The Carbon handler
gives a `cmds` event to the handler stored with its command ID. For these five, `mac:0267e0` finds
the ID in `mac:693d60` and queues four key records: kind 7 for code 1 (`CTRL`), kinds 7 and 8 for
the letter's code (`Z`, `X`, `C`, `V`, `A`), and kind 8 for code 1. An Edit menu command thus builds
the chord `CTRL-Z`, `CTRL-X`, `CTRL-C`, `CTRL-V` or `CTRL-A`, pressed and released. Which keys
choose a menu item is the menu bar's resource, not code.

Addresses: `mac:0268f2`, `mac:026957`, `mac:0269b6`, `mac:08bf80`, `mac:08c349`, `mac:08c34e`, `mac:089f6e`, `mac:089fc9`, `mac:08aad3`, `mac:0267e0`, `mac:0267ea`, `mac:026809`, `mac:026813`, `mac:02684a`, `mac:026872`, `mac:02689a`, `mac:0268c6`, `mac:089dd6`, `mac:693d60`
Confidence: verified
Proof: one
Test: `core::mac_edit_command_keys`

`mac:026957` to `mac:0269b6` push `mac:0267e0` with each ID as a big-endian word and call
`mac:08bf80`, which ends by storing the ID at `+0` of a node and the handler at `+0x18`
(`mac:08c349`, `mac:08c34e`). For the class `cmds` the handler reads the direct object as an
`HICommand` of 14 bytes (`mac:089f6e`), finds the node whose `+0` equals its command ID
(`mac:089fc9`) and calls that node's `+0x18` with the event and the ID (`mac:08aad3`). `mac:0267e0`
compares the ID with the five pairs of `mac:693d60` (`mac:0267ea`, `mac:026809`), loads the code
from the pair (`mac:026813`) and calls the enqueue `mac:089dd6` four times (`mac:02684a`,
`mac:026872`, `mac:02689a`, `mac:0268c6`); an ID it does not find is answered `eventNotHandledErr`.
`mac:089dd6` queues as Raw key events' sites do.
