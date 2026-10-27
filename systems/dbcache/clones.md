# dbcache: per-cache code

The code each of the thirteen caches has a copy of: the static initializers that build the cache
objects and register their savers, and the response code of the game object and quest caches. The
cache objects, the files and the shared shapes are in `dbcache/overview`. The initializer run is the
slots `0x829e8c` to `0x829ec0` of the C runtime's table, run in address order: DBCache.cpp's +∞ copy,
then one thunk per cache: creature, game object, item name, item, NPC text, name, guild, quest, page
text, pet name, petition, item text and wow cache.

Roots: `0x554b10`, `0x554b30`, `0x554b80`, `0x554bd0`, `0x554c20`, `0x554c70`, `0x554cc0`, `0x554d10`, `0x554d60`, `0x554db0`, `0x554e00`, `0x554e50`, `0x554ea0`, `0x554ef0`, `0x554f40`, `0x5588e0`, `0x562c80`, `0x563150`

## Initializers of the saved caches

Each cache's thunk calls a body that runs the cache's constructor on its store with six arguments
(tag, file name, query opcode, a dword, owner flag, persist flag), then jumps to a function that
registers an exit with `_atexit`. The exit loads the store into `ecx` and jumps to the cache's save.
These nine pass persist 1, so their save writes the file at exit.

| thunk | body | store | constructor | owner | at-exit, exit | save |
|---|---|---|---|---|---|---|
| `0x554b30` | `0x554b40` | `0xc0e354` creature | `0x556060` | 1 | `0x554b60`, `0x554b70` | `0x5566d0` |
| `0x554b80` | `0x554b90` | `0xc0e318` game object | `0x557bd0` | 1 | `0x554bb0`, `0x554bc0` | `0x558180` |
| `0x554bd0` | `0x554be0` | `0xc0e2dc` item name | `0x5596c0` | 1 | `0x554c00`, `0x554c10` | `0x559c10` |
| `0x554c20` | `0x554c30` | `0xc0e2a0` item | `0x55b0a0` | 1 | `0x554c50`, `0x554c60` | `0x55b650` |
| `0x554c70` | `0x554c80` | `0xc0e264` NPC text | `0x55cba0` | 1 | `0x554ca0`, `0x554cb0` | `0x55d120` |
| `0x554d60` | `0x554d70` | `0xc0e1b0` quest | `0x561fa0` | 0 | `0x554d90`, `0x554da0` | `0x562630` |
| `0x554db0` | `0x554dc0` | `0xc0e174` page text | `0x563d60` | 1 | `0x554de0`, `0x554df0` | `0x564320` |
| `0x554ea0` | `0x554eb0` | `0xc0e0c0` item text | `0x568fb0` | 1 | `0x554ed0`, `0x554ee0` | `0x569570` |
| `0x554ef0` | `0x554f00` | `0xc0e084` wow cache | `0x56aaf0` | 1 | `0x554f20`, `0x554f30` | `0x56b040` |

Addresses: `0x554b10`, `0x554b20`, `0x554b30`, `0x554b40`, `0x554b60`, `0x554b70`, `0x554b80`, `0x554b90`, `0x554bb0`, `0x554bc0`, `0x554bd0`, `0x554be0`, `0x554c00`, `0x554c10`, `0x554c20`, `0x554c30`, `0x554c50`, `0x554c60`, `0x554c70`, `0x554c80`, `0x554ca0`, `0x554cb0`, `0x554d60`, `0x554d70`, `0x554d90`, `0x554da0`, `0x554db0`, `0x554dc0`, `0x554de0`, `0x554df0`, `0x554ea0`, `0x554eb0`, `0x554ed0`, `0x554ee0`, `0x554ef0`, `0x554f00`, `0x554f20`, `0x554f30`, `0x554f40`, `0x560460`, `0x409aef`, `0x809128`, `0xc0e07c`
Confidence: verified
Proof: two
Test: none

A thunk is `call body; jmp at-exit` (`0x554b30`, `0x554b35`). A body pushes persist, owner, the dword,
the opcode, the file name and the tag, in that order, and calls the constructor with the store in `ecx`
(`0x554b40` to `0x554b57`); the constructor takes the tag from `[ebp+8]` to `+0x28` and persist from
`[ebp+0x1c]` to `+0x39` (`0x56050e`, `0x560530` in the guild copy). The dword is 0 except the item
cache's `0x57` (`0x554c34`). An at-exit is `push exit; call _atexit; pop ecx; ret` (`0x554b60`); an exit
is `mov ecx,store; jmp save` (`0x554b70`). The run opens with `0x554b10`, a `jmp` to `0x554b20`, which
copies +∞ (0x7f800000 at `0x809128`) into `0xc0e07c`; no instruction names `0xc0e07c` but that store (census of both
addresses as dwords: one site each; control `0x554b30`, found at slot `0x829e90`). `0x554f40`, past the
run, is `jmp 0x554f50` and is called once, from `0x402b1b` in `WowClientInit`.

## Initializers of the unsaved caches

The name, guild, pet name and petition caches are built the same way with persist 0, so their exits
reach a save that writes nothing and tears the cache down, and the world entry's `0x555740` clears them.

| thunk | body | store | constructor | owner | at-exit, exit | save |
|---|---|---|---|---|---|---|
| `0x554cc0` | `0x554cd0` | `0xc0e228` name | `0x55e6a0` | 0 | `0x554cf0`, `0x554d00` | `0x55ec70` |
| `0x554d10` | `0x554d20` | `0xc0e1ec` guild | `0x560460` | 0 | `0x554d40`, `0x554d50` | `0x560a20` |
| `0x554e00` | `0x554e10` | `0xc0e138` pet name | `0x5658a0` | 1 | `0x554e30`, `0x554e40` | `0x565e70` |
| `0x554e50` | `0x554e60` | `0xc0e0fc` petition | `0x567350` | 1 | `0x554e80`, `0x554e90` | `0x567980` |

Addresses: `0x554cc0`, `0x554cd0`, `0x554cf0`, `0x554d00`, `0x554d10`, `0x554d20`, `0x554d40`, `0x554d50`, `0x554e00`, `0x554e10`, `0x554e30`, `0x554e40`, `0x554e50`, `0x554e60`, `0x554e80`, `0x554e90`, `0x5557c0`, `0x55580c`
Confidence: verified
Proof: two
Test: none

The bodies' first two pushes are `push 0; push 0` for names and guilds (`0x554cd0`, `0x554d20`) and
`push 0; push 1` for pet names and petitions (`0x554e10`, `0x554e60`). `0x555740` calls the guild clear
at `0x5557c0` and the petition clear at `0x55580c` after testing their `+0x39` bytes.

## Game object records received

`0x5588e0(msg, single)` is the game object cache's `Receive`. Its one caller, the handler for opcode
0x5f, passes single 1: the response is one record and no count byte. The record opens with its key; a
top bit set refuses the key without it. A refusal for a key with a node sets the dispatching byte
`+0xa9`, unlinks the node, runs its callbacks with status 0, clears the byte and frees the node
through vtable slot 0; without a node it does nothing. An accepted key finds or makes the node,
decodes the record into `node + 0x18`, sets the valid byte `+0x98` and the key copy `+0x94`, and runs
the callbacks with status 1, dispatching byte set, and frees them.

Addresses: `0x5588e0`, `0x555100`, `0x555106`, `0x55510e`, `0x5588ee`, `0x558907`, `0x558925`, `0x558985`, `0x55898d`, `0x5589a4`, `0x5589d8`, `0x558a02`, `0x558a16`, `0x558a26`, `0x558a2b`, `0x558a69`, `0x558a97`, `0x558aa5`, `0x555d60`, `0x56d310`
Confidence: verified
Proof: two
Test: none

`0x555106` pushes 1 and `0x55510e` calls with the cache `0xc0e318`. `0x5588ee` takes single to a count of 1, where
0 would read a count byte and `0x558907 jbe` return on 0; `0x558aa5 dec; jne` loops.
`0x558925 jns` keeps a clear top bit; else `and ebx,0x7fffffff` and a refusal flag. The chain walk is
`0x558934` to `0x55897e`. The refusal leg (`0x558985`, `0x55898d` for no node) unlinks the hash and list
links with `0x555d60` when `[node+8]` is set (`0x5589a4`), calls `[cb+8]` with `push 0` (`0x5589d8`) and
frees through `call [edx]` (`0x558a02`). The accept leg inserts with `0x56d310` (`0x558a16`), decodes
with `0x7c87c0` (`0x558a26`), stores `+0x98`, `+0x94` and `+0xa9` (`0x558a2b` to `0x558a3b`), calls with
`push 1` (`0x558a69`) and frees each callback with `SMemFree` (`0x558a97`).

## Quest records stored and refused

The quest handler `0x555300` decodes a quest record on its stack, the quest id being its first dword. A
record whose title (`+0x9c`) is empty is a refusal: `0x563150(id)` finds the node, sets the dispatching
byte `+0x1909`, runs its callbacks with status 0, clears the byte, unlinks the node and frees it, valid or
pending; no node, no effect. A titled record goes to `0x562c80(record, id)`, which finds or makes the
node, copies the record into `node + 0x18`, sets the valid byte `+0x18f8` and the key `+0x18f4`, and
then runs the callbacks with status 1 in list order and frees them. Each dispatched message stores one record, so
its callbacks run before the next message's record is stored.

Addresses: `0x555300`, `0x555410`, `0x55542c`, `0x555448`, `0x563150`, `0x5631b6`, `0x5631de`, `0x5631f5`, `0x56326a`, `0x562c80`, `0x562ce3`, `0x562cf0`, `0x562dec`, `0x562ea6`, `0x562eeb`, `0x562efb`, `0x562f10`, `0x56dc10`
Confidence: verified
Proof: two
Test: none

`0x555410` compares the title's first byte with 0 and `je 0x55543c` takes the refusal; both legs pass
`[ebp-0x18dc]`, the record's first dword, as the key. `0x563150` stores `+0x1909` at `0x5631b6`, calls with
`push 0` (`0x5631de`), clears it at `0x5631f5`, unlinks `+4` and `+0xc` inline and frees through
`call [edx]` (`0x56326a`). `0x562c80` inserts with `0x56dc10` (`0x562ce3`), sets `+0x1909` (`0x562cf0`),
copies the dwords, the four pairs and six pairs, `+0x8c` to `+0x98`, the texts with `SStrCopy` at limits
0x200, 0x800, 0x800 and 0x200 (`0x562dec` on), and the four objective groups with their 0x100-byte
texts; `0x562ea6` sets `+0x18f8`, `0x562eeb` calls with `push 1`, and `0x562f10` frees the list.
