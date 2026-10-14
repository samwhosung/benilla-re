# ui: the edit box input language

The language an edit box reports and the toggle that changes the input method's mode: two edit box
methods over the operating system's input method, whose answer is a function of the keyboard layout
and never of the client's locale.

Roots: `0x799550`, `0x799610`, `0x77abb0`

## The language index

`EditBox:GetInputLanguage()` returns one of `ROMAN`, `KOREAN`, `CHINESE`, `JAPANESE`, read from the edit
box's language index at `+0x3d0`; `EditBox:ToggleInputLanguage()` returns nothing and asks the
operating system's input method to switch its conversion mode. The index is 0 (`ROMAN`) from
construction and changes only when the input method reports an event: it comes from the active
keyboard layout's primary language and the conversion mode, never from the client's locale.

Addresses: `0x799550`, `0x799610`, `0x77ac30`, `0x77ac40`, `0x77abb0`, `0x42c1a0`, `0x42c8e0`, `0x7799b0`, `0x779a76`, `0x77afc0`, `0x77b12d`, `0x77ac1c`, `0x879a5c`, `0x42c229`
Confidence: verified
Proof: one
Test: none

`0x77ac30` is `mov eax,[ecx+0x3d0]; mov eax,[eax*4+0x879a5c]; ret` over a table of four strings. The
toggle `0x77ac40` calls `0x42c8e0` with `[this+0x3d0] == 0`, which gets the window's input context, and
sets or clears the native-mode bit of its conversion status; a missing context ends it. The classifier
`0x42c1a0` returns 0 without a context or without the native-mode bit, else maps the primary language
byte of the keyboard layout: 0x04 to 3 (0 when the layout is exactly 0x08040804), 0x11 to 1, 0x12 to 2,
anything else 0. The table at `0x77ac1c` remaps these to string order. A changed index is stored
(`0x77abf2`) and, when the edit box has an `OnInputLanguageChanged` handler, passed to it as a
string. A client whose layouts are all non-CJK therefore answers `ROMAN` always and the toggle has no
effect a script can see.
