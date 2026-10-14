# ui: chat colours

The colour each chat type is drawn in: the live registry that the chat frames read through Lua, its
defaults, and the three Lua functions that change and query it. The defaults and the event each type
fires are `tables/chat_types.tsv`; a window's own background colour is `chatwindows`.

Roots: `0x4982c0`, `0x4a07f0`, `0x4a09e0`, `0x4a0a80`

## The colour registry

The live registry is 94 entries of 0x43 bytes at `0xb4e518`: a 64-byte name, then red, green and blue as
bytes. A second array of 0x43-byte entries, ten of them, named `CHANNEL1` to `CHANNEL10`, is held
by pointer at `0xb4e504` with its count at `0xb4e500`. The start-up copies the 94 static entries at
`0x804710` (name and colour, `tables/chat_types.tsv`) and then colours every extra entry with the live
`CHANNEL` entry's bytes, so the extras start as `ffc0c0`. The file loader overwrites colours by
name afterwards (`chatwindows`).

Addresses: `0xb4e518`, `0xb4e500`, `0xb4e504`, `0x804710`, `0x4982c0`, `0x498402`, `0x498412`, `0x498446`, `0x49845f`, `0x498466`, `0x498483`, `0x4984c2`, `0xb4e902`, `0xb4e903`, `0xb4e904`, `0x64a5a0`, `0x64a7f0`
Confidence: verified
Proof: one
Test: none

The copy loop `0x498402` to `0x498444` runs 94 times with step 0x43 (bound 0x189a): `0x64a5a0(dst, src, 0x40)`
then the three bytes from `0x804750 + i*0x43`. `0x498466` stores the extra count 10 after growing the
array to 10 if it was smaller (`0x498446` to `0x49845f`); `0x498483` formats the name with the string
`CHANNEL%d` into 0x40 bytes; the colour bytes come from `0xb4e902`, `0xb4e903` and `0xb4e904`, which are
`0xb4e518 + 14*0x43 + 0x40` to `+0x42`, the fifteenth entry `CHANNEL` of the live registry.

## ChangeChatColor

`ChangeChatColor(name, r, g, b)` needs a string and three numbers (otherwise it raises the usage
error). Each colour is `int(255.0 * x)` truncated, with no clamp; the low byte is stored. The name
is matched without case against the 94 registry names and then against the extra names, and both
matches are written when both exist. If any entry matched, the function fires `UPDATE_CHAT_COLOR` with
the name given and the three stored bytes times 1/255 as numbers. It returns no values.

Addresses: `0x4a07f0`, `0x4a0805`, `0x4a0864`, `0x4a086a`, `0x4a08c1`, `0x4a08e7`, `0x4a08f9`, `0x4a0922`, `0x4a099c`, `0x4a09a7`, `0x4a093d`, `0x4a095a`, `0x4a098b`, `0x806498`, `0x8026c8`, `0x703f50`
Confidence: verified
Proof: one
Test: none

The four type tests are `lua_isstring` and three `lua_isnumber` (`0x4a0803` to `0x4a0841`), all
jumping to `0x4a09ca`, which pushes `0x844d28` and calls `luaL_error` (`0x6f4940`). The string is
`Usage(ChangeChatColor(chatType, r, g, b)`, malformed in the exe. Each `fmul qword [0x806498]` (255.0) is followed by `0x40a2b0`.
The fixed-entry match writes `+0x40`, `+0x41`, `+0x42` and sets a local flag (`0x4a08f9`); an extra match
writes its entry and jumps to the fire (`0x4a099c` to `0x4a093f`) without the flag. The event is id
0xe2 with the format `%s%f%f%f` (`0x4a098b`); the floats are `fild` of each byte times `0x8026c8`
(1/255).

## GetChatTypeIndex

`GetChatTypeIndex(name)` returns a number: the registry position plus one for a name among the 94
(1 to 94, matched without case), 95 plus the extra position for a name among the extras (95 to 104),
and 0 for any other name. A non-string argument raises the usage error.

Addresses: `0x4a0a80`, `0x4a0ada`, `0x4a0adf`, `0x4a0b2b`, `0x4a0b52`, `0x4a0b55`, `0x4a0b7c`, `0x6f3810`
Confidence: verified
Proof: one
Test: none

`inc ebx` at `0x4a0adf` precedes the match test, so the index is one-based; the extra scan starts
`edi` at 0 and `0x4a0b55` adds 0x5f (95). Both counts go through `lua_pushnumber` (`0x6f3810`) as a
double. The no-match path pushes the double 0 (`0x4a0b7c`).

## ResetChatColors

`ResetChatColors()` copies all 94 static entries back over the live registry, name and colour, and
sets every extra entry's colour to the live `CHANNEL` entry's colour; the extra names stay. It takes
no arguments, returns no values and fires no event.

Addresses: `0x4a09e0`, `0x4a09f3`, `0x4a09f8`, `0x4a0a1f`, `0x4a0a27`, `0x4a0a3b`, `0x4a0a6f`
Confidence: verified
Proof: one
Test: none

The loop is the start-up's (`0x64a5a0` with 0x40 and the bytes at `0x804750`), run from `0x4a09e3` to
`0x4a0a1f`; the extras are recoloured from `0xb4e902` to `0xb4e904` in the loop `0x4a0a35` to
`0x4a0a6f`. Because the live `CHANNEL` entry was just reset, the extras end at its default.
