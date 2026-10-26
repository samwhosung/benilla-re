# gx: window

Each backend owns its window and the procedure behind it: `0x598750` for Direct3D, registered
with the class `GxWindowClassD3d` (`device.md`, Direct3D device start), and `0x58c6d0` for
OpenGL. A procedure takes its device from the window's user data, turns messages into the
device's window events (`slots.md`, Window events) and cursor updates (`slots.md`, Cursor),
holds the drag rectangle to the format's aspect and the minimum size, and hands every other
message to the procedure the device was created with or to `DefWindowProcA`.

Roots: `0x598750`, `0x58c6d0`

## Direct3D window messages

`0x598750(hwnd, msg, wParam, lParam)` reads the device with `GetWindowLongA(hwnd,
GWL_USERDATA)` and handles these messages. A message marked passed on, and every message not
listed, goes to the procedure saved at the device's `+0xf28` (`hwnd` and `msg` in `ecx` and
`edx`, `wParam` and `lParam` on the stack) when there is a device and it is set, else to
`DefWindowProcA`, and that result is returned.

| message | action | returns |
|---|---|---|
| `WM_CREATE` (1) | stores the create parameter as the user data | 0 |
| `WM_DESTROY` (2) | event 2 | 0 |
| `WM_SIZE` (5) | event 0 with `{0, 0, height, width}`; type 1 for `SIZE_MINIMIZED`, 2 for `SIZE_MAXHIDE`, else 0 | passed on |
| `WM_ACTIVATE` (6) | a full-screen device losing activation: event 0, a zero rectangle, type 1 | passed on |
| `WM_SETFOCUS` (7), `WM_KILLFOCUS` (8) | events 3 and 4 | 0 |
| `WM_PAINT` (0xf) | `BeginPaint` and `EndPaint` | 0 |
| `WM_ERASEBKGND` (0x14) | nothing | 0 |
| `WM_SETCURSOR` (0x20) | over the client area of a device with an `IDirect3DDevice9`: the system cursor off, the device's shown (`slots.md`, Cursor) | 1 |
| `WM_DISPLAYCHANGE` (0x7e) | a full-screen device: event 1 with the new size | passed on |
| `WM_SYSCOMMAND` (0x112) | `SC_SCREENSAVE` (0xf140) and `SC_MONITORPOWER` (0xf170) swallowed | 0 |
| `WM_SIZING` (0x214) | Window sizing, below | 1 |

Addresses: `0x598750`, `0x598770`, `0x598784`, `0x598b00`, `0x598b20`, `0x598792`, `0x5987af`, `0x598825`, `0x5987eb`, `0x5987f8`, `0x59888b`, `0x5988a3`, `0x5987c7`, `0x5988bb`, `0x59891d`, `0x598925`, `0x59892f`, `0x598937`, `0x598964`, `0x5988d2`, `0x598975`, `0x598993`, `0x59899b`, `0x5989b3`, `0x5989c8`, `0x592220`
Confidence: verified
Proof: one
Test: none

`0x598770` splits at 0x14; up to it `0x598784` indexes the byte table `0x598b20` with `msg − 1`
(15 entries: 0, 1, 7, 7, 2, 3, 4, 5, 7, 7, 7, 7, 7, 7, 6) into the eight cases of `0x598b00`, and
case 7, `0x59899b`, passes on. `WM_CREATE` (`0x598792`) calls `SetWindowLongA(hwnd, −21, [lParam])`. The size
words are `fild`ed into floats (`0x598832`, `0x598851`). `WM_ACTIVATE` needs the low word of
`wParam` 0 and `0x592220`, the windowed byte `+0x20b`, clear (`0x5987f8`); `WM_DISPLAYCHANGE`
the same byte clear (`0x5988dd`). `WM_SETCURSOR` with no device goes straight to
`DefWindowProcA` (`0x59891f`); `0x598925` needs `+0x38a8` and `0x59892f` the hit code 1 in the
low word of `lParam`; it calls `SetCursor(0)` (`0x598937`) and `IDirect3DDevice9::ShowCursor`
(slot `+0x30`, `0x598964`) with `+0x2880 && +0x2884`. The fallback is `call esi` at `0x5989b3`
and `DefWindowProcA` at `0x5989c8`.

## Window sizing

On `WM_SIZING` both procedures adjust the drag rectangle `lParam` points to (the whole window, in
screen coordinates) in place and return 1; one of zero width or height returns 0 unchanged. On
`{top, left, bottom, right}`, with the ratio `r = [0xc0ef60]` (width over height of the format,
0 unless it constrains the aspect, `device.md`) and the minimum 320 by 240 (`0xc0ef64`, `0xc0ef68`), the
dragged edge runs these steps in order; any other `wParam` changes nothing.

| `wParam` | edge | steps |
|---|---|---|
| 1 | left | min left, fit top |
| 2 | right | min right, fit bottom |
| 3 | top | min top, fit left |
| 4 | top left | min top, min left, fit top, fit left |
| 5 | top right | min top, min right, fit top, fit right |
| 6 | bottom | min bottom, fit right |
| 7 | bottom left | min bottom, min left, fit left, fit bottom |
| 8 | bottom right | min bottom, min right, fit bottom, fit right |

Addresses: `0x58c6d0`, `0x5989d7`, `0x5989f3`, `0x5989fb`, `0x598a05`, `0x598a0e`, `0x598b30`, `0x598ad9`, `0x58c835`, `0x58c87a`, `0x58ca34`, `0x58c945`, `0x5910a0`, `0x5910ab`, `0x5910b5`, `0x5910c8`, `0x5910ce`, `0x5910e0`, `0x591120`, `0x591147`, `0x591160`, `0x5911a0`, `0x5911b4`, `0x5911b6`, `0x5911c0`, `0x5911d4`, `0x5911e0`, `0x591200`, `0xc0ef60`, `0xc0ef64`, `0xc0ef68`, `0x40a2b0`
Confidence: verified
Proof: one
Test: `gx::sizing_rect_helpers_difftest`, `gx::sizing_wm_sizing_difftest`

A min step moves the dragged edge when the width (`right − left`) is below 320 or the height
below 240, as 32-bit differences compared signed: left to `right − 321`, right to `left + 319`,
top to `bottom − 241`, bottom to `top + 239`. A fit sets the edge from the other dimension: top
to `bottom − w / r`, bottom to `top + w / r`, left to `right − h · r`, right to `left + h · r`,
each the integer and the `f32` ratio at 53-bit precision, truncated by `__ftol`'s low dword. A
fit is skipped when `r` equals 0 (`0x5910ab`, `test ah,0x44; jnp`); a NaN runs it. `0x5989d7`
copies the `RECT` to `{top, left, bottom, right}`, `0x5989f3` and `0x5989fb` test the width and
height, `0x598a05` bounds `wParam − 1` by 7 for `0x598b30`, and `0x598ad9` writes back. OpenGL's
block (`0x58c835`) runs them through `0x58ca34`. The minimums differ by one by side
(`0x5911b6 sub eax,edx; dec eax`, `0x5911d4 lea eax,[eax+edx-1]`), a side drag can fit the other
dimension below its minimum, and a client size's ratio shapes the outer window. The tests run the
helpers and both blocks, `0x5989d7` to `0x598af1`, `0x58c841` to `0x58c95d`.
