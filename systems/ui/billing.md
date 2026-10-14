# ui: play time and the billing rest time

Three in-game Lua functions read the play-time state: `GetBillingTimeRested`, `PartialPlayTime` and `NoPlayTime`. The first
returns a number the authentication response stored in the session; the other two test two bits of the player's flags. The
glue-screen twins and the parsing of the response are `glue/script#billing` and `net/session`.

Roots: `0x48ec50`, `0x48eb70`, `0x48ebe0`

## The rest time

`GetBillingTimeRested()` takes no argument, reads the session object's word at `+0x1af8` and pushes it as one number, whole, unsigned and unscaled. It never pushes nil and cannot
raise. The glue twin `0x46dcf0` is the same 56 bytes apart from its two call targets. The word is written once per authentication response by `0x5b41b0` (the third value of the
trailing billing trio, only when the result is 0x0c or 0x1b and at least 5 bytes remain), and by nothing else; the session object is
allocated with the zero-fill flag off and its constructors do not write the word, so a response without the trio leaves it unset.

Addresses: `0x48ec50`, `0x48ec59`, `0x48ec5e`, `0x48ec67`, `0x48ec6e`, `0x48ec79`, `0x5b421a`, `0x5ab490`, `0x46dcf0`
Confidence: verified
Proof: one
Test: none

The explicit `mov [ebp-4],0` before a 64-bit `fild` zero-extends the dword, so values from 0x80000000 up stay positive. A census of the dword `0x1af8` as an immediate finds three
sites: the one writer `0x5b421a` and the two readers. The engine applies no scale; the client's own start-up code reads the neighbouring remaining-time word as minutes, comparing it with
`0x10e0`, 4320 minutes or 72 hours (`0x48f647`), and multiplying it by 60000 for milliseconds (`0x48f666`).

## Partial play and no play

`PartialPlayTime()` pushes the number 1 when the active player object has bit 12 (0x1000) of `PLAYER_FLAGS` set, else nil, also nil with no player object. `NoPlayTime()` is the same
for bit 13 (0x2000). Both read `[[player+0xe68]+8]`, shifted and masked, with the player type mask 0x10, and push exactly one value. Neither reads the billing words.

Addresses: `0x48eb70`, `0x48eb8d`, `0x48eb9c`, `0x48eb9f`, `0x48eba2`, `0x48ebc7`, `0x48ebe0`, `0x48ec0f`
Confidence: verified
Proof: one
Test: none

`shr eax,0xc; and al,1` selects bit 12 at `0x48eb9f`, and `shr eax,0xd` bit 13 in `NoPlayTime`. The value pushed is the masked bit, 1.

## The warning timer

`CGGameUI::InitializeGame` zeroes the word `0xb4e360` and, when the session's remaining billing time (`+0x1af4`) is nonzero and at most 4320 and bit 1 of the flag byte `+0x1afc`
is clear, stores `now + remaining * 60000` in it and zeroes `0xb4e364` and `0xb4e368` (`0x48f630` to `0x48f679`). The interface poller reads these three words.

Addresses: `0x48f480`, `0x48f630`, `0x48f636`, `0x48f641`, `0x48f647`, `0x48f654`, `0x48f661`, `0x48f666`, `0x48f66e`, `0x48f673`, `0x48f679`, `0xb4e360`, `0xb4e364`, `0xb4e368`
Confidence: verified
Proof: one
Test: none

`imul esi,esi,0xea60` turns minutes into milliseconds. The readers of the three words are the poller at `0x495679`, `0x49568b`, `0x4956c6`, `0x49570d` and `0x4957d6`; their use is not read here.
