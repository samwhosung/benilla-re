# ui: channel administration verbs

The Lua functions that list the player's channels and send one channel command: the list, the owner, the password,
moderation, invitations, bans and the announcement switch. They are in `ChatFrame.cpp` beside the join and leave verbs of
`ui/channelverbs`; the slots they read are `ui/chatchannels`.

Roots: `0x4a00d0`, `0x4a01c0`, `0x4a02d0`, `0x4a03a0`, `0x4a0490`, `0x4a05c0`, `0x4a06b0`

## Commands naming a channel

Fourteen verbs pass a fixed message number and their argument strings to one of two helpers. The first argument is the channel,
a name or the number of a joined channel, turned into the channel's name by `ParseChannelKey`; a verb whose helper takes a second
argument also needs it as a string. A missing or non-string argument raises `Usage: <Verb>("channel")` or
`Usage: <Verb>("channel", "name")` with the verb's own name (`SetChannelPassword` raises `Usage: SetChannelPassword("name", "password")`, pushed at `0x4a046b`); a channel number with no joined channel sends nothing. Each
verb sends its message with the channel and, for the two-argument ones, the second string, and returns no value.

| verb | message | arguments |
|---|---|---|
| `SetChannelPassword` | 0x9c | channel, password |
| `SetChannelOwner` | 0x9d | channel, name |
| `ChannelModerator` | 0x9f | channel, name |
| `ChannelUnmoderator` | 0xa0 | channel, name |
| `ChannelMute` | 0xa1 | channel, name |
| `ChannelUnmute` | 0xa2 | channel, name |
| `ChannelInvite` | 0xa3 | channel, name |
| `ChannelKick` | 0xa4 | channel, name |
| `ChannelBan` | 0xa5 | channel, name |
| `ChannelUnban` | 0xa6 | channel, name |

`ListChannelByName` (0x9a), `DisplayChannelOwner` (0x9e), `ChannelToggleAnnouncements` (0xa7) and `ChannelModerate` (0xa8) take the channel alone.

Addresses: `0x4a00d0`, `0x4a00f0`, `0x4a0114`, `0x4a014f`, `0x4a0188`, `0x4a03a0`, `0x4a03df`, `0x4a03f0`, `0x4a046b`, `0x4a0490`, `0x4a04b0`, `0x4a04cb`, `0x4a04e1`, `0x4a04f5`, `0x4a0527`, `0x4a0580`, `0x4a05c0`, `0x4a06b0`, `0x4a06d0`, `0x4a06f0`, `0x4a0710`, `0x4a0730`, `0x4a0750`, `0x4a0770`, `0x4a0790`, `0x4a07b0`, `0x4a07b0`, `0x4a07d0`, `0x49e880`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

Each short verb pushes its name string and loads the message number into `edx` before calling the helper: `0x4a00f0` takes one
channel string (`Usage: %s("channel")`), `0x4a04b0` two (`Usage: %s("channel", "name")`). Both call
`ParseChannelKey` (`0x4a014f`, `0x4a04f5`), then `PutUInt32` the message, `PutCString` the key and the second string and
`ClientServices::Send`. `ListChannelByName`, `DisplayChannelOwner`, `ChannelToggleAnnouncements` (0xa7) and `ChannelModerate`
(0xa8) use the one-string helper; `SetChannelPassword` has its own copy (`0x4a03f0`) of the two-string shape with 0x9c.

## Listing channels

`ListChannels()` prints one chat line of type 0x11 listing the joined channels as `[number. name] `, skipping empty slots and
slots whose flag at `+0x9c` is set, and starting a new line when the text reaches 2048 characters. `GetChannelList()` returns,
for each such slot, its number and then its name: the ChatChannels row's name in the current locale (`+0x30`) when the slot's
row id at `+0x94` names a row, else the slot's own name. Both walk the 0xa0-byte slots of `0xb4fe04` up to the count `0xb4fe00`.

Addresses: `0x4a01c0`, `0x4a01e8`, `0x4a01f2`, `0x4a020a`, `0x4a023c`, `0x4a0257`, `0x4a02b5`, `0x49a870`, `0x4a02d0`, `0x4a0302`, `0x4a0323`, `0x4a034b`, `0x4a0351`, `0x4a0382`, `0xb4fe00`, `0xb4fe04`
Confidence: verified
Proof: one
Test: none

`0x4a020a` formats `[%d. %s] ` (the number from the slot's first dword, the name at `+4`) into a 0x8a-byte buffer and
`0x4a023c` compares the line length with 0x800; `0x4a0257` and `0x4a02b5` flush with `AddChatLine(0x11, ...)`. `GetChannelList`
pushes the number with `fild` (`0x4a030d`), the name through `lua_pushstring` (`0x4a0357`) and returns twice the count (`0x4a0382`).
