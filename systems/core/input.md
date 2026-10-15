# core: window input

How Windows messages become events. The window procedure turns each input message into a
five-dword record in a ring; once per tick the loop drains the ring through the message pump and
a switch on the record's kind, which updates the shared modifier and button masks and dispatches
one event category (`core/events`).

The window code is the OS object file from `0x42c9d0` through `0x42e1d0`: the pump, the ring,
the window procedure, key translation, cursor placement and the window frame. The switch and the
per-kind handlers, `0x4239a0` to `0x424260`, are in the event library. The masks are `0x883f30`
(mouse mode), `0x883f34` (modifier keys) and `0x883f40` (held buttons). The window procedure is
installed by the window code of another system, which calls it through a pointer.

Roots: `0x42cfe0`, `0x42cd60`, `0x42cd70`, `0x42cde0`, `0x42cf10`, `0x420540`, `0x420600`, and the static initializers `0x42c9d0`, `0x42e1a0`, `0x42e1c0`

## Window messages

The window procedure queues these records. A key record carries the translated key and the
repeat count, a character the converted character, a mouse record the button or wheel delta and
a client position from `GetCursorPos`, not from `lParam`.

| message | kind |
|---|---|
| `WM_CAPTURECHANGED` (0x215), unless the client released capture itself | 0 |
| `WM_CHAR` (0x102), characters from 0x20 | 1 |
| IME: 0x51, 0x10d to 0x10f, 0x281, 0x282 | 3 |
| `WM_SIZE`, `WM_DISPLAYCHANGE`: a pending size check, made by the pump | 4 |
| `WM_CLOSE` (0x10), `WM_SYSCOMMAND` with `SC_CLOSE` | 5 |
| `WM_ACTIVATE` (6): whether the low word of `wParam` is nonzero | 6 |
| `WM_KEYDOWN`, `WM_SYSKEYDOWN` / `WM_KEYUP`, `WM_SYSKEYUP` | 7 / 8 |
| button down / up, with the button's bit (`Buttons and capture`) | 9 / 0xd |
| `WM_MOUSEMOVE` in absolute mode: the position | 0xa |
| `WM_MOUSEWHEEL` (0x20a): the signed delta | 0xb |
| `WM_MOUSEMOVE` in relative mode: the offset from the held point | 0xc |
| `GetMessageA` returning 0 (`WM_QUIT`), made by the pump | 0xe |

Addresses: `0x42cfe0`, `0x42d080`, `0x42d0d5`, `0x42d185`, `0x42d21e`, `0x42d292`, `0x42d2d1`, `0x42d31c`, `0x42d37d`, `0x42d3d0`, `0x42d479`, `0x42d54f`, `0x42db90`, `0x42dc00`, `0x42dd00`, `0x42dd90`, `0x42d47e`
Confidence: verified
Proof: two
Test: none

The procedure is `fastcall(hwnd, message, wParam, lParam)`, `ret 8`. Messages 5 to 0xa3 go through
the byte map `0x42d5e0` and table `0x42d5c0`, 0x100 to 0x112 through `0x42d69c` and `0x42d680`,
0x201 to 0x285 through `0x42d6d4` and `0x42d6b0`; `0x200` is tested first (`0x42cfff`). `SC_CLOSE`
is `cmp edi,0xf060` (`0x42d28a`). The relative test is `[0x884e5c]` at `0x42d31c`. `0x42dc00` maps
the button messages, `HIWORD(wParam)` selecting X1 or X2. `0x42dd00` reads the position, or the
saved one in relative mode, and queues the record. Other messages go to the hook `0x884e6c`, or to
`DefWindowProcA`. `[0x884e30]`, which the client sets around its own
`ReleaseCapture` (`0x42ddc7`), suppresses kind 0 (`0x42d47e`). The `WM_IME_NOTIFY` case for `wParam`
6 also queues kind 8 for key 0, a Shift release (`0x42d40e`).

## The ring

The records wait in a ring of 16 slots of five dwords: kind, three values, and the clock when
queued. It holds at most 15; queuing into a full ring drops the oldest record.

Addresses: `0x42d780`, `0x42cc00`, `0x42d7bb`, `0x884cc0`, `0x884e34`, `0x884e50`
Confidence: verified
Proof: two
Test: none

`0x42d780` is `fastcall(kind, a)` with three dwords pushed, `b`, `c` and one it ignores (`ret 0xc`).
The next write index wraps after 15 (`0x42d788` to `0x42d798`); when it meets the read index, the
read index advances (`0x42d7a0` to `0x42d7b6`). The clock is read at `0x42d7bb` and stored at
`+0x10`. `0x42cc00` copies the oldest record out and advances the read index the same way, returning
0 when the indices meet.

## The pump

Each call of `0x42c9f0` yields one record or none. A pending size check comes first: when the
window is not minimized and its client size has changed, it yields kind 4 with the width and
height. Otherwise it takes the oldest ring record; with the ring empty, it gets and dispatches
Windows messages until one queues a record. With none waiting it returns nothing, after a sleep
when the window is inactive and `0x429d60` returns nonzero.

Addresses: `0x42c9f0`, `0x42cada`, `0x42cb32`, `0x42cb43`, `0x42cbe4`, `0x42cbbb`, `0x423920`, `0x884e40`, `0x884e28`, `0x884e2c`, `0x8333b0`
Confidence: verified
Proof: two
Test: none

The size path runs when `[0x884e40]` is set and `IsIconic` is false, compares the client size
from `GetWindowRect` less `AdjustWindowRectEx` with `0x884e28` and `0x884e2c`, and on a change
stores it and yields kind 4 (`0x42ca0a` to `0x42cad7`). `PeekMessageA` without removal gates each
`GetMessageA` (`0x42cb32`); `0x42e3d0` may consume a message before `TranslateMessage` and
`DispatchMessageA` (`0x42cb43`). The sleep is `0x429d50(0x429d80())` when `0x8333b0`, the active
flag, is 0 and `0x429d60()` is nonzero. `0x423920` calls the pump until it yields nothing,
passing each record to `0x4239a0`.

## Message filter and idle sleep

Two settings of the pump are fixed in this build. The message filter `0x42e3d0` returns 0 at once
while its enable word `0x884e9c` is 0, and the only writer of that word, `0x42e3a0`, has no caller,
so the filter never consumes a message. An inactive window sleeps 0 ms per pump call: the sleep is
enabled by `0x832f70`, which holds 1, and lasts `0x884c08` ms, which holds 0, and their setters
`0x429d70` and `0x429d90` have no caller.

Addresses: `0x42e3d0`, `0x42e3a0`, `0x884e9c`, `0x429d50`, `0x429d60`, `0x429d70`, `0x429d80`, `0x429d90`, `0x832f70`, `0x884c08`, `0x42e510`, `0x42fe90`, `0x435c30`
Confidence: verified
Proof: one
Test: none

`0x42e3d6` tests `0x884e9c`, `0x42e3e9` to `0x42e3eb` return 0. A search of the disassembly for
`0x884e9c` finds the store at `0x42e3a0` and reads only; a scan for `e8`/`e9` and dwords finds no
reference to `0x42e3a0`, `0x429d70` or `0x429d90`, with `0x420c00` as the control. `0x429d60` and
`0x429d80` read the two words, `0x429d50` is `Sleep`. An enabled filter would offer the
message to `TranslateAcceleratorA` (`0x42e46f`) and to the filter list (`0x42e4aa`, `0x430160`), through
`0x42e510` (`GetPropA` of the window property `OsGuiPointer`) and `0x42fe90` (`GetParent`).
`0x435c30(0)` is the game window the pump asks about (`0x42ca12`): 0 returns it, 1
`GetActiveWindow`, 2 `GetForegroundWindow`.

## Kinds to categories

`0x4239a0` switches on the kind. Most kinds dispatch one category to the context's handlers; kinds 0
and 2 can dispatch several, kinds 6 and 0xd also dispatch 0xf when they end the mouse mode (`Buttons
and capture`), and 5 and 0xe dispatch none.

| kind | handler | category |
|---|---|---|
| 0 | `0x423b70` | 0xe for each held button, lowest bit first, marked synthetic |
| 1 / 2 | `0x423bb0` / `0x423be0` | 1, once per character of a run |
| 3 / 4 | `0x423c30` / `0x423c70` | 0x1b with the code page / 0x1c |
| 5 / 0xe | `0x423b50` | none: on the confirm-close query's consent, quit (`Close and quit`) |
| 6 | `0x423cb0` | 2, after clearing the modifier and button masks |
| 7 / 8 | `0x423d00` / `0x423d50` | 8 / 9, keeping modifier bits for keys 0 to 2 |
| 9 / 0xd | `0x423db0` / `0x4240b0` | 0xb / 0xe, keeping the button mask |
| 0xa / 0xc | `0x423fe0` / `0x424050` | 0xc / 0xd |
| 0xb | `0x424130` | 0x10 |

Addresses: `0x4239a0`, `0x423b10`, `0x423b70`, `0x423bb0`, `0x423be0`, `0x423c30`, `0x423c70`, `0x423cb0`, `0x423900`, `0x423d00`, `0x423d50`, `0x423db0`, `0x423fe0`, `0x424050`, `0x4240b0`, `0x424130`, `0x423fc0`
Confidence: verified
Proof: two
Test: none

The table `0x423b10` holds 15 entries for kinds 0 to 14, read from its dwords. Each handler loads
`edx` with its category before `0x4245b0`. No producer queues kind 2: the 18 calls of `0x42d780`
load kinds 0, 1, 3, 5, 6, 7 or 8, 8, 0xa, 0xb, 0xc, and 9 or 0xd through `0x42dd00`. Key handlers
set or clear `1 << key` in `0x883f34` when `cmp edx,2; jg` falls through (`0x423d06`, `0x423d56`),
so for the key codes 0, 1 and 2 (Shift, Control, Alt). `0x423b70` takes `(m − 1) ^ m) & m` of
`0x883f40` until it is 0, calling `0x4240b0` with flag 1 and the current clock. `0x423fc0` returns 2
when the mouse mode is 1, else 0.

## Payloads

Key events carry `{key, modifiers, repeat count, time}`; characters `{character, modifiers,
repeat count}` (1 for each character of a run); activation `{active}`. Mouse events carry nine
dwords: the mouse mode, the button (0 for moves and the wheel), the held buttons after the
change, the modifiers, flags (2 in relative mode, plus 1 for a synthetic release), x and y, the
wheel delta, and the time. For absolute events x and y are the cursor fractions of
`Cursor coordinates`; for relative moves they are the pixel offsets as floats.

Addresses: `0x423d24`, `0x423bc1`, `0x423c0c`, `0x423cbc`, `0x423dd4`, `0x423e01`, `0x4240e6`, `0x424159`, `0x424082`, `0x42408e`, `0x42409c`
Confidence: verified
Proof: two
Test: none

The key payload is built at `ebp−0x10` (`0x423d24` to `0x423d36`), the character at `ebp−0xc`.
The mouse payload starts at `ebp−0x24`: mode, button, mask (stored after the update), modifiers
and `0x423fc0()` (`0x423dd4` to `0x423de5`), the fractions through pointers to `ebp−0x10` and
`ebp−0xc`, the time at `ebp−0x4`. `0x4240b0` ORs its flag argument into the flags
(`0x4240e6`); `0x424130` puts the delta at `ebp−0x8` (`0x424159`). `0x424050` converts the
offsets with `fild`/`fstp DWORD` (`0x424082` to `0x42409c`); its `+0x1c` stays unwritten, as it
does for every kind but the wheel.

## Cursor coordinates

An absolute mouse event's position is given as fractions of the client area: `x / width` and `1 − y
/ height`, from the client rectangle's width and height, so y runs up from the bottom. x is a 53-bit
quotient rounded to a float; y is rounded three times: the quotient and the subtraction at 53 bits,
then to a float. The client rectangle comes from a cache filled by `GetClientRect` and by the pump's
size check. A confinement rectangle could clamp the point and move the OS cursor, but it is never
set: it stays zero, and the clamp is skipped, as it is only for a zero width or height; a NaN
difference would run it.

Addresses: `0x423e20`, `0x423e40`, `0x423f7a`, `0x423f8d`, `0x423fa1`, `0x423fa6`, `0x423fa9`, `0x42cea0`, `0x883f48`, `0x883f4c`, `0x883f50`, `0x883f54`, `0x884e20`
Confidence: verified
Proof: two
Test: `core::cursor_normalize_matches_binary`

The clamp runs only when both `0x883f54 − 0x883f4c` and `0x883f50 − 0x883f48` are nonzero
(`test ah,0x44; jnp`, `0x423e40`, `0x423e60`). Then `fild x; fidiv width; fstp DWORD`
(`0x423f7f` to `0x423fa1`) and `fild y; fidiv height; fsubr 1.0; fstp DWORD` (`0x423fa3` to
`0x423faf`), `d8 2d` giving `1.0 − st0`. x is not offset by the rectangle's left. The rectangle's
writers are the static initializer `0x4238a0`, which zeroes it, and `0x424370`, reached only from
the unreached `0x420740`. A zero width or height gives an infinity or NaN. The test runs
`0x423f7f` to `0x423fb1`.

## Buttons and capture

The window procedure gives the buttons the bits left 1, middle 2, right 4, X1 8 and X2 0x10. It
captures the mouse when the first button goes down and releases it when the last comes up. On the
event side, `EventSetMouseMode` (`0x420540`) switches the mouse mode for a set of buttons, and only
while all of them are held; that set is kept, and when a button-up leaves any of it released, the
mode returns to absolute.

Addresses: `0x42db90`, `0x42dd90`, `0x420540`, `0x4241a0`, `0x424200`, `0x423ce0`, `0x883f44`, `0x884e1c`, `0x884e30`, `0x42dc00`, `0x42dcf0`, `0x42dce0`, `0x42dc30`, `0x42dc6d`, `0x42dc5b`, `0x42dc7f`
Confidence: verified
Proof: one
Test: none

`0x42dc00` maps 0x201 and 0x202 to 1 (`0x42dc30`); 0x204 to 0x20c index the byte map `0x42dcf0`, `00
00 03 01 01 03 03 02 02`, into `0x42dce0`, so 0x204 and 0x205 store 4 (`0x42dc6d`), 0x207 and 0x208
store 2 (`0x42dc5b`), and 0x20b and 0x20c take 8 or 0x10 by `HIWORD(wParam)` (`0x42dc7f`).
`0x42db90` calls `SetCapture` when `0x884e1c` is 0 and ORs the button in; `0x42dd90` clears it and,
at 0, calls `ReleaseCapture` with `0x884e30` set. `0x420540` refuses a mode of 2 or more with error
0x57 and calls `0x4241a0` on the current context, which stores the button set in `0x883f44` only
when `(mask & 0x883f40) == mask`, and on a mode change calls the OS switch and `0x424200`; that
stores the mode in `0x883f30` and dispatches category 0xf with the mode, the buttons and the
modifiers. `0x423ce0`, after category 2 and every button-up, calls `0x420540(0, 0)` when `0x883f44`
is set and not all held.

## Mouse modes

In relative mode the OS cursor is held at a fixed point, half the window rectangle's right and
bottom in screen coordinates, which is the window's centre only when the window sits at the screen's
origin: every move queues the offset from that point and puts the cursor back. Leaving relative mode
puts the cursor back where it was last seen in client coordinates. `0x420600` writes the current
cursor position, as fractions passed through the interface's scaling `0x41ad80`.

Addresses: `0x42cc90`, `0x42ccd0`, `0x42cd20`, `0x42ce30`, `0x42ce50`, `0x42cde0`, `0x420600`, `0x424260`, `0x884e5c`, `0x884e38`, `0x884e3c`, `0x884e44`, `0x884e48`
Confidence: verified
Proof: one
Test: none

`0x42cc90` sets `0x884e5c` to 1 and tail-calls `0x42ccd0`, which stores half the window
rectangle's right and bottom, signed, in `0x884e38` and `0x884e3c` and calls `SetCursorPos`; back
to 0 it tail-calls `0x42cd20`, which converts `0x884e44`/`0x884e48` with `ClientToScreen`. The
move arm compares `GetCursorPos` with the held point (`0x42d32f`) and calls `0x42ccd0` after queuing.
`0x42ce30` saves the last position; `0x424260` passes `0x42cde0`'s position through `0x423e20`
and the interface scaling `0x41ad80`.

## Key codes

The window procedure translates the virtual key before queuing: F1 to F12 become 0x300 to 0x30b,
the digit keys keep their codes, a fixed table names Shift, Control and Alt (0, 1, 2), the
editing, navigation and keypad keys, Escape, Pause, Caps, Num and Scroll Lock and Print Screen,
and every other key becomes its `MapVirtualKeyA(vk, 2)` character. A key whose character is 0 or
above 0xff is dropped. The scan code is never read.

Addresses: `0x42d800`, `0x42d81c`, `0x42d844`, `0x42dafc`, `0x42da60`, `0x42da39`, `0x42da42`, `0x42da49`, `0x42da4b`, `0x42d223`
Confidence: verified
Proof: one
Test: none

`0x42d800` gets the virtual key in `ecx` and an out pointer. F1 to F12 add `0x290`
(`0x42d80d`); 0x30 to 0x39 are stored as they are (`0x42d81c` to `0x42d826`); 8 to 0x91 index the
byte map `0x42dafc` into the table `0x42da60`, whose cases store constants: 8 → 0x202, 9 → 0x203,
0xd → 0x201, 0x10/0x11/0x12 → 0/1/2, 0x13 → 0x211, 0x1b → 0x200, 0x25 to 0x28 → 0x204 to 0x207,
0x60 to 0x69 → 0x100 to 0x109. Anything else reaches `0x42da39`: `and eax,0xffff`, then `je` on
0 and `cmp eax,0xff; jg` refuse it (`0x42da42` to `0x42da4b`). A refused key queues nothing
(`0x42d228`).

## Key messages and Alt F4

A translated key queues kind 7 for `WM_KEYDOWN` and `WM_SYSKEYDOWN` and kind 8 for the releases, its
second value the low word of `lParam`, the repeat count. After queuing key code 0x303 (F4) the
procedure asks `0x436010(2)`, whether the Alt key is down. When it is, the message goes on to the
window hook or `DefWindowProcA`, so Alt+F4 reaches the system close command, which queues kind 5
(`Window messages`); otherwise the procedure returns 0. `0x436010` takes 0 for Control, 1 for Shift
and 2 for Alt and returns 1 when the high nibble of `GetKeyState` is nonzero.

Addresses: `0x42d21e`, `0x42d230`, `0x42d25c`, `0x42d261`, `0x42d26f`, `0x42d35c`, `0x436010`
Confidence: verified
Proof: one
Test: none

`0x42d230` to `0x42d249` set `ecx` to 1 for messages 0x100 and 0x104 and 0 otherwise, and `neg;
sbb; add 8` (`0x42d24f` to `0x42d259`) turns that into 7 or 8 before `0x42d780` (`0x42d25c`). The
`cmp [ebp+8],0x303` is at `0x42d261`, the call at `0x42d26f`, the branch to `0x42d35c` (hook, then
`DefWindowProcA`) at `0x42d276`, and `xor eax,eax` for the other case at `0x42d27e`. `0x436010`
maps its argument to the virtual keys 0x11, 0x10 and 0x12 (`0x43604e`, `0x436036`, `0x43601e`),
masks the state with 0xf000 and returns `neg; sbb; neg` of it.

## Character codes

A character from 0x80 is converted through a table chosen by the system code page: 1250, 1251,
1254 and 874 convert 0x80 to 0xff, 1252 only 0x80 to 0x9f; other code pages pass the byte
through.

Addresses: `0x42d185`, `0x42d19e`, `0x42d1ba`, `0x42d1f5`, `0x8333b8`, `0x833438`, `0x833838`, `0x833c38`, `0x834038`, `0x42cf10`
Confidence: verified
Proof: one
Test: none

`GetACP`, through `0x42cf10` (`0x42d19e`), selects `0x833438` for 1250 (`0x4e2`), `0x833838` for
1251, `0x833c38` for 1254, `0x834038` for 874, and `0x8333b8` indexed by `ch & 0x7f` for 1252 below
0xa0 (`0x42d1ea` to `0x42d1f5`). The record's second value is the low word of `lParam`,
sign-extended.

## Close and quit

A close request (`WM_CLOSE` or `SC_CLOSE`) and `WM_QUIT` ask the confirm-close callback first, and
quit only on a nonzero answer. With no callback the answer is 1. The client installs one in its
context start handler and removes it in its stop handler: it closes the current context and
answers 0, so closing the window tears the context down at the end of that tick, dispatching
categories 3 and 4, before the loop ends.

Addresses: `0x423a21`, `0x423af8`, `0x423b50`, `0x41fe40`, `0x424250`, `0x423ca0`, `0x883f5c`, `0x883f60`, `0x63c9b0`, `0x402840`, `0x403640`, `0x63c9a0`, `0x63cdf0`, `0x63b460`
Confidence: verified
Proof: one
Test: none

`0x423b50` jumps to `[0x883f5c]` with `ecx = [0x883f60]`, or returns 1 when it is null. Kind 5
then calls `0x423ca0` and sets the quit flag (`0x423a2e`, `0x423a36`); kind 0xe sets only the
flag (`0x423b04`). `0x41fe40` jumps to `0x424250`, which stores the callback and its argument.
The client's start handler `0x402840` installs `0x63c9a0` through `0x63b460` (`0x63b599`); that
calls `0x41f9b0` for the current context through `0x63cdf0` and returns 0. The stop handler
`0x403640` clears it (`0x63c9b5`). `core/close` follows the teardown from there.

## Mouse speed and Num Lock

At startup the window code saves `GetAsyncKeyState(VK_NUMLOCK)` and the Windows mouse speed. The
client's mouse speed setting, clamped to [0.1, 2] and times 10 truncated, becomes the Windows mouse
speed while the window is active; on deactivation and at shutdown the saved speed comes back, and at
shutdown a Num Lock key-down is sent when `GetAsyncKeyState(VK_NUMLOCK)` differs from the saved
value.

Addresses: `0x42cf20`, `0x42cf60`, `0x42cd70`, `0x42cd60`, `0x42d0ae`, `0x884cb0`, `0x884cb4`, `0x884e08`, `0x801620`, `0x801624`, `0x801628`
Confidence: verified
Proof: one
Test: none

`0x42cf20` stores `GetAsyncKeyState(VK_NUMLOCK)` and reads `SPI_GETMOUSESPEED` into `0x884cb0`,
copying it to `0x884cb4`. `0x42cd70` takes 0.1 when `s < 0.1`, else `s` when `s < 2.0`, else 2.0
(also for NaN), times 10.0, through `__ftol`, and sets it when active. `WM_ACTIVATE` sets
`SPI_SETMOUSESPEED` to `0x884cb4` or `0x884cb0` (`0x42d09c` to `0x42d0b2`). `0x42cf60` sends a
Num Lock key-down through `SendInput` when the state differs. `0x42cd60` returns `0x884cb4 · 0.1`.

## Window frame

The window procedure drags and sizes the frame. A press on the caption (hit test 2) or an
edge or corner (10 to 17) records the hit test, the window rectangle and the cursor point, sets
focus and captures the mouse. Moves under capture shift the rectangle by the cursor delta: the
caption moves the window; an edge or corner changes the sides it names, after `WM_SIZING`. The
release applies the last delta and ends capture. A press on the minimize (8), maximize (9) or close
(20) box acts on a release with the same hit test: minimize, toggle maximized, quit. A double click
does the same, toggles maximized on the caption, and quits on the system menu (3).

Addresses: `0x42df90`, `0x42e000`, `0x42e0a0`, `0x42de10`, `0x42de60`, `0x42dea0`, `0x42d04e`, `0x42d067`, `0x42d039`, `0x42d2d1`, `0x42d4e5`, `0x42e074`, `0x884e0c`, `0x884e54`, `0x884e60`, `0x884e64`, `0x884e68`
Confidence: verified
Proof: one
Test: none

Messages 0xa1, 0xa2 and 0xa3 reach `0x42d04e`, `0x42d067` and `0x42d039` through the byte map
`0x42d5e0`, and call `0x42df90`, `0x42e0a0` and `0x42e000` with the hit test in `edx`.
`0x42df90` clears the release flag `0x884e68`, stores the hit test in `0x884e60`, and for hit 2 or
10 to 17 calls `SetFocus`, stores `GetWindowRect` at `0x884e0c`, the point from `lParam` (signed
words) at `0x884e54`, calls `SetCapture` and sets `0x884e64`; otherwise it clears `0x884e64`. While
`0x884e64` is set, `WM_MOUSEMOVE` (`0x42d2d1`) and `WM_LBUTTONUP` (`0x42d4e5`) convert the point
with `ClientToScreen` and call `0x42de10` and `0x42e0a0`. `0x42de10` calls `0x42de60`
(`MoveWindow` at the rectangle plus the delta) for hit 2 and `0x42dea0` otherwise. `0x42dea0` adds
the delta to the left side for hits 10, 13 and 16, the right for 11, 14 and 17, the top for 12 to
14 and the bottom for 15 to 17 (byte maps `0x42df64`, tables `0x42df58`, `0x42df6c`), sends message
0x214 with `hit - 9` and the rectangle, and calls `MoveWindow`. `0x42e0a0` skips when `0x884e68` is
set, acts when the hit test matches `0x884e60` (maps `0x42e188`, `0x42e170`), releases capture and
clears the flag. `0x42e000` sets the flag first (`0x42e00d`), then acts by the map `0x42e084`.

## Static initializers

The window object file runs three static initializers: the +∞ store at its start and the 2π and
1/2π stores at its end. The three before them, `0x42c830`, `0x42c980` and `0x42c9a0`, are the
IME initializer file's below it, and below that is `OsIME.cpp`; both are `os`.

Addresses: `0x82740c`, `0x827418`, `0x42c830`, `0x42c840`, `0x42c980`, `0x42c990`, `0x42c9a0`, `0x42c9b0`, `0x42c9d0`, `0x42c9e0`, `0x42e1a0`, `0x42e1b0`, `0x42e1c0`, `0x42e1d0`
Confidence: verified
Proof: one
Test: none

The window file's are the slots `0x827418` through `0x827420`, matched by the census of
`core/loop#static-initializers`; the IME initializer file's run is `0x82740c` to `0x827414`. A
file that includes the shared header opens its run with its +∞ (`systems/README`, Object files):
`0x42c830` copies the +∞ at 0x801610 and `0x42c980` doubles the π 4 bytes on; `0x42c9d0` copies
0x801618 and `0x42e1a0` doubles 0x80161c, so the window file is `0x42c9d0` to `0x42e1d0` and the
IME initializer file holds 0x42c850 to 0x42c960, which call `ImmGetContext` and
`ImmAssociateContext` with the globals 0x884ca0 and 0x884ca8. `OsIME.cpp` is the code from 0x42c1a0
to 0x42c828: its functions call IMM32, directly or through each other, and its path is pushed at 0x42c5ab, 0x42c64c and
0x42c808. It has no run of its own and uses neither global, so whether it and the IME initializer
file are one object file is not settled. The initializers' globals are read only by each other.

## Unreached functions

`0x42cc80` would set the pending size check from two words; nothing calls it or stores its
address. `0x42cfd0` would set the hook `0x884e6c`; its two calls, at `0x42e34b` and `0x42e372`, are
in the unreferenced `0x42e340`.

Addresses: `0x42cc80`, `0x42cfd0`
Confidence: verified
Proof: one
Test: none

The census of `core/loop#unreached-functions` finds no reference.
