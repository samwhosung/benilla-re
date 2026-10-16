# ui: play time and the billing rest time

Three in-game Lua functions read the play-time state: `GetBillingTimeRested`, `PartialPlayTime` and `NoPlayTime`. The first
returns a number the authentication response stored in the session; the other two test two bits of the player's flags. The
glue-screen twins and the parsing of the response are `glue/script#billing` and `net/session`.

The play-time warning is the other consumer of the response. `CGGameUI::InitializeGame` arms a deadline from the session's
remaining minutes at each world entry, and `CGGameUI::Update` polls it every tick, signalling a dialog event and writing system
chat lines as the end nears. The three words it keeps are `0xb4e360`, `0xb4e364` and `0xb4e368`.

Roots: `0x48ec50`, `0x48eb70`, `0x48ebe0`, `0x48f480`, `0x495590`

## The rest time

`GetBillingTimeRested()` takes no argument, reads the session object's word at `+0x1af8` and pushes it as one number, whole, unsigned and unscaled. It never pushes nil and cannot
raise. The glue twin `0x46dcf0` is the same 56 bytes apart from its two call targets. The word is written once per authentication response by `0x5b41b0` (the third value of the
billing group, only when the result is 0x0c or 0x1b and the group's 9 bytes are all there), and by nothing else; the session object is
allocated with the zero-fill flag off and its constructors do not write the word, so a response without the full group leaves it as it was.

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

## The billing group

After result 0x0c (`AUTH_OK`) or 0x1b (`AUTH_WAIT_QUEUE`), `0x5b41b0` reads the billing group when at least 5 bytes remain: a dword into `+0x1af4` (minutes remaining), a byte into `+0x1afc` (plan flags), a dword into `+0x1af8` (rested time). The gate counts 5 bytes and the group is 9: with 5 to 8 left the third read writes nothing and leaves `+0x1af8` as it was. Flag mask 0x16 selects the plan `GetBillingPlan` returns first (0x02 gives 1, 0x04 gives 2, none gives 3, 0x10 gives 4, other mixes 0); 0x02 also zeroes `GetBillingTimeRemaining` and arms no warning; 0x20 is the second value and 0x08 the third, the internet-cafe plan the poll tests.

Addresses: `0x5b41b0`, `0x5b41e2`, `0x5b41f3`, `0x5b41f9`, `0x5b41fe`, `0x5b420c`, `0x5b421a`, `0x418cb0`, `0x418eb0`, `0x418ec4`, `0x46dbff`, `0x46dc0a`, `0x46dc13`, `0x46dc4d`, `0x46dc5e`, `0x46dca8`, `0x46dcc5`, `0x46dcfe`, `0x48ec5e`, `0x48f641`, `0x48f654`, `0x4956c0`
Confidence: verified
Proof: one
Test: none

`0x5b41f3` computes the unread count as the store's size `[edi+0x10]` less its cursor `[edi+0x14]`, and `0x5b41f9` is `cmp edx,5; jb`. The reads are `0x418eb0` (`+0x1af4`), `0x418cb0` (`+0x1afc`) and `0x418eb0` (`+0x1af8`); a read past the size sets the cursor to size + 1 and returns without storing (`0x418ec4` to `0x418ec6`). The dwords 0x1af4, 0x1afc and 0x1af8 occur as displacements only at the three `lea` writes (`0x5b41fe`, `0x5b420c`, `0x5b421a`) and eight reads: the glue functions (`0x46dbff`, `0x46dc0a`, `0x46dc4d`, `0x46dcfe`), `0x48ec5e`, and the warning's `0x48f641`, `0x48f654` and `0x4956c0`; the scan is over every dword of `.text` and finds each known site. `0x46dc0a` tests bit 1 and `0x46dc13` zeroes the remaining time. `GetBillingPlan` masks with `and eax,0x16` (`0x46dc5e`) and compares with 2, 4, 0 and 0x10 for 1, 2, 3 and 4; `0x46dca8` shifts by 5 and `0x46dcc5` by 3 for the two values after.

## The warning timer

`CGGameUI::InitializeGame` zeroes the deadline `0xb4e360` at `0x48f636`. With the session's remaining minutes R (`+0x1af4`) and plan flags, it then arms when 0 < R <= 4320 (unsigned) and flag bit 1 is clear: the deadline becomes `now + R * 60000`, `now` being the millisecond clock `0x42c010`, and the line clock `0xb4e364` and dialog latch `0xb4e368` are zeroed. Otherwise those two keep their values and the deadline stays 0. It then jumps to `CGGameUI::Initialize`. The function has one caller, `0x401602` in `0x401570`, whose one caller is `0x46c236` in the glue manager's state 8: it runs once per login into the world and never on `ReloadUI`, which runs `0x490bd0` and `0x48fbf0` alone.

Addresses: `0x48f480`, `0x48f482`, `0x48f636`, `0x48f641`, `0x48f647`, `0x48f654`, `0x48f65d`, `0x48f661`, `0x48f666`, `0x48f66e`, `0x48f673`, `0x48f679`, `0x48f681`, `0x401570`, `0x401602`, `0x46b930`, `0x46c236`, `0x48fbf0`, `0x49015a`, `0x490bd0`, `0x490cc9`, `0x495664`, `0x495669`, `0xb4e360`, `0xb4e364`, `0xb4e368`
Confidence: verified
Proof: one
Test: `ui::billing_arm`

`0x48f647` is `cmp esi,0x10e0; ja`, `0x48f654` `test byte [eax+0x1afc],2; jne`, `0x48f65d` `cmp esi,edi; jbe` with `edi` zero from `0x48f482`; `imul esi,esi,0xea60` (`0x48f666`) cannot overflow at 4320. The deadline wraps with the clock, and 0 means unarmed, so a sum of exactly 2^32 reads as unarmed. `0x48f630` stores a different word, `0xb4e310`. A scan for `e8`/`e9` to `0x48f480` finds `0x401602` alone, and no section stores its address; the same scan finds `0x46c236` for `0x401570`. The three words occur as displacements only in this function and in `CGGameUI::Update`, with no bulk clear beside them. `0x48fbf0` registers `0x495590` on event category 5 (`0x49015a`) and `0x490bd0` removes it (`0x490cc9`).

Bit-exact coverage: `ui::billing_arm` covers `0x48f636` to `0x48f679` of `0x48f480`, with `0x42c010` and `0x5ab490` stubbed.

## The minutes reading

The poll reads the time left as minutes: `left = deadline - now` as a dword, zero-extended, times the `f32` `0x378bcf65` (a hair over 1/60000) at 53-bit precision, rounded up, truncated to an integer, and compared as unsigned. For 1 <= left < 89,999,999 the reading is left / 60000 + 1 (integer division): a whole minute reads as two, 0 reads 0. For n >= 2 the first left that reads n is floor(60000(n - 1) * 2^34 / 17179869375) + 1, which is 60000(n - 1) less k ms, k = 1 from n = 1501, 2 from 3000, 3 from 4499. The arm's 4320-minute cap reaches only k of 1 and 2; more needs a wrapped left.

Addresses: `0x495693`, `0x49569b`, `0x4956a1`, `0x4956a7`, `0x4956aa`, `0x4956b2`, `0x4956b7`, `0x804590`, `0x73fdf5`, `0x40a2b0`, `0x40cf81`, `0x42c010`, `0x404100`
Confidence: verified
Proof: one
Test: `ui::billing_minutes_left`, `ui::glue_geom_480_495590_tick_scale`

`0x495698` stores `ebx` (zero since `0x49559f`) as the high dword, `0x49569b` is `fild qword`, `0x4956a1` `fmul dword [0x804590]` and `0x4956a7` `fstp qword`. `ceil` (`0x73fdf5`) sets the control word to 0x1b3f, round up, through `0x40cf81`, rounds the finite double with `frndint` (`0x7458d2`) and restores the old word (`0x73fe92`). `__ftol` truncates it. The product has up to 56 bits and rounds once at 53, the control word WinMain sets (`0x404100`); a Rust `f64` product matches. A deadline passed by up to 2^31 ms wraps to a left of at least 2^31, reading 35,792 or more. The test runs `0x49569b` to `0x4956b7` in the exe, `ceil` and `__ftol` included, over the first left of every reading n to 71,583 and the left below it.

Bit-exact coverage: `ui::billing_minutes_left` covers `0x49569b` to `0x4956b7` of `0x495590`; `ui::glue_geom_480_495590_tick_scale` covers `0x49569b` to `0x4956aa`.

## The play-time poll

Each tick, while the deadline D is nonzero, the poll reads left and the minutes m above, and the flag byte's bit 3 (cafe). The steps run in this order:

| step | when | effect |
|---|---|---|
| first poll | line clock N is 0 | cafe: event 0x206 if m <= 600; else event 0x205 with m if m <= 30; either sets the latch L (L is not tested); N = now |
| line | N not 0, now - N >= 0 (signed), not cafe | a line if m <= 30; then D = 0 if now - D >= 0 (signed), else N = now + left % step, step 60000 when m <= 5, else 300000 |
| dialog | after the step above, every poll | cafe: event 0x206 if m <= 600 and L = 0; else event 0x205 with m if m <= 30 and L = 0; either sets L |

After its first poll a cafe plan never takes the line step, so its D and N stay as they are. The first poll never tests D.

Addresses: `0x495590`, `0x495679`, `0x495686`, `0x49568b`, `0x4956c0`, `0x4956c6`, `0x4956cb`, `0x4956d1`, `0x4956d6`, `0x4956d8`, `0x4956dc`, `0x4956e9`, `0x4956f0`, `0x495700`, `0x49570b`, `0x495715`, `0x495723`, `0x49572f`, `0x495741`, `0x49574b`, `0x49574f`, `0x495751`, `0x495755`, `0x4957d6`, `0x4957dd`, `0x4957e1`, `0x4957ef`, `0x4957f6`, `0x4957fb`, `0x4957ff`, `0x495805`, `0x49580a`, `0x49581e`, `0x495826`, `0x495830`, `0x703f50`, `0x703e50`, `0x835154`, `0xb4e360`, `0xb4e364`, `0xb4e368`
Confidence: verified
Proof: one
Test: `ui::billing_poll`

The function's first part, from `0x4955ab`, also signals events 0x1b1 and 0x1b2 (`0x49563d`, `0x495656`). The poll starts at `0x495679` (`cmp [0xb4e360],ebx; je 0x495830`), which every path of the function's first part reaches through `0x49565b` while the reload byte `0xb4b3f4` is clear; a reload frame returns after `0x48fbf0` without it (`0x495664`, `0x495669`). Event ids 0x205 and 0x206 are `BILLING_NAG_DIALOG` (`"%d"`, `0x835154`, `0x703f50`) and `IGR_BILLING_NAG_DIALOG` (`0x703e50`). The step `div` (`0x4957fb`) divides left by 60000 (`0x495841`) or `0x493e0`, and `0x4957ff` adds `now`. N and D take 0 as unset, so a sum that lands on 0 repeats the first poll. A left that is a whole step stores N = now, and the next poll prints again.

Bit-exact coverage: `ui::billing_poll` covers `0x495679` to `0x495830` of `0x495590`, all of it, with `0x42c010`, `0x5ab490`, `0x703f50`, `0x703e50`, `0x703bf0`, `0x64a7f0`, `0x49cae0` and `0x49a870` stubbed and logged.

## The warning lines

A line is `SStrPrintf(buf, 0x800, GetText("BILLING_NAG_WARNING", nil, m), m)` as chat type 10 (`CHAT_MSG_SYSTEM`): queued with `0x49cae0` while the display latch `0x8435fc` is set, else shown by `AddChatLine`. Lines are due from the poll after the first when m <= 30, then at each whole 5 minutes of time left and each whole minute from 5 (an arm at 32 minutes: 30, 25, 20, 15, 10, 5, 4, 3, 2, 1). A poll exactly on D prints 0; a later one, while now - D < 2^31 ms, prints nothing and clears D. A poll exactly on a mark reads one minute high and prints again on the next.

Addresses: `0x49575a`, `0x49575e`, `0x495763`, `0x495775`, `0x49577a`, `0x4957ad`, `0x4957d1`, `0x8433f0`, `0x8435fc`, `0x703bf0`, `0x64a7f0`, `0x49cae0`, `0x49a870`
Confidence: verified
Proof: one
Test: `ui::billing_poll`

`0x49575a` pushes m, `0x49575b` pushes the gender 0 (`ecx` is 0 after `0x495751`), `0x49575e` loads the key `0x8433f0` and the ordinal m, and `0x703bf0` (`ret 4`) pops the gender; the m below it is the vararg of `SStrPrintf` (`0x64a7f0`, `add esp,0x10` at `0x49577f`). A `GetText` with a non-negative ordinal calls the Lua global `GetText` (`ui/chatserver#the-text-lookup`), so the template follows the locale's plural rule. `0x4957ad` calls `0x49cae0` with type 10 and the text; the queued record's channel and tag are the empty string `0x882748`. `0x4957d1` calls `0x49a870` with zeros. The latch (`ui/chatevents#the-display-latch`) is 1 until the enter cascade. The test checks every argument of both calls.
