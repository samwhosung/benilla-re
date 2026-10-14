# ui: the interface Lua function tables

The interface registers its Lua functions as globals from tables of name and function pairs, one table for each file that defines some. A pair is two dwords: a pointer to the name, then the C function. `ui/luastate` holds the 216-pair game table and the frame types; this doc holds the 43 tables of the other files, 753 pairs in all, with the loops that register and unregister them and the unregistrars of the method tables (`ui/methods`).

Roots: `0x490250`, `0x490ce0`

## Function tables

43 tables in the data section, each an array of 8-byte pairs. A row holds the pair count and the first and last name of its table. The registrar of each table is in `Registrars`, its unregistrar in `Unregistrars`.

Addresses: `0x843618`, `0x845078`, `0x8457e0`, `0x845eb0`, `0x846648`, `0x846b04`, `0x846c88`, `0x847240`, `0x8475a0`, `0x8479f8`, `0x847cf0`, `0x8481b8`, `0x84838c`, `0x848414`, `0x8484d0`, `0x848ce8`, `0x848da0`, `0x8490a8`, `0x8496d8`, `0x849cf8`, `0x849fa8`, `0x84a0a8`, `0x84a3e0`, `0x84ada8`, `0x84b0f0`, `0x84b7e8`, `0x84ba00`, `0x84bb20`, `0x84bd10`, `0x84c170`, `0x84c990`, `0x84ccf0`, `0x84cea0`, `0x84d00c`, `0x84d0b4`, `0x84d108`, `0x84d5dc`, `0x84d688`, `0x84da28`, `0x84ddd0`, `0x84e968`, `0x8503f4`, `0x850438`
Confidence: verified
Proof: one
Test: none

The count is the registrar's loop limit divided by 8, and the name and function are the dwords at `[esi+table]` and `[esi+table+4]`. The last table, `0x850438`, holds the 83 unit and PvP functions from UnitExists to GetInspectPVPRankProgress; the first six of the file, GetTime to AppendToFile, are the table at 0x8503f4.

## Registrars

43 registrars, one for each table: a loop of `FrameScriptRegisterFunction(name, function)` over the pairs, 30 or 33 bytes (38 for `0x51bb60`), `ret` at the end. `0x490250` calls them in a fixed order after the 216-pair loop.

Addresses: `0x4a1a10`, `0x4a91c0`, `0x4ac430`, `0x4afee0`, `0x4b4b40`, `0x4b5a20`, `0x4b82c0`, `0x4bc130`, `0x4beb90`, `0x4c0860`, `0x4c3450`, `0x4c4730`, `0x4c4e10`, `0x4c5c00`, `0x4c9a30`, `0x4ca5d0`, `0x4cbbd0`, `0x4d03e0`, `0x4d2420`, `0x4d40e0`, `0x4d4d00`, `0x4d6c50`, `0x4da7c0`, `0x4dcf20`, `0x4e1a80`, `0x4e2b60`, `0x4e3bb0`, `0x4e4aa0`, `0x4e7780`, `0x4e9be0`, `0x4f1b90`, `0x4f3830`, `0x4f4a10`, `0x4f5370`, `0x4f5950`, `0x4f7d70`, `0x4f8810`, `0x4fa7a0`, `0x4fc1b0`, `0x500320`, `0x502310`, `0x51bb20`, `0x51bb60`
Confidence: verified
Proof: one
Test: none

A registrar is `push esi; xor esi,esi; mov edx,[esi+table+4]; mov ecx,[esi+table]; call 0x704120; add esi,8; cmp esi,count*8; jb; pop esi; ret`. `0x51bb60` first calls `0x51bb20` for the six shared functions and then runs the same loop over the 83-pair table. All but `0x51bb20` are reached from `0x490250` only, `0x4c4730` by a tail `jmp` (`0x490379`) and the rest by `call`; `0x51bb20` is also called from `0x46abb0`.

## Unregistrars

43 unregistrars, the mirror of the registrars: a loop of `FrameScript_UnregisterFunction(name)` over the same pairs. `0x490ce0` calls them in the registrars' order, with the tabard creation one first, between its other unloads.

Addresses: `0x490ce0`, `0x4a1a40`, `0x4a91f0`, `0x4ac460`, `0x4aff10`, `0x4b4b70`, `0x4b5a40`, `0x4b82e0`, `0x4bc160`, `0x4bebc0`, `0x4c0880`, `0x4c3480`, `0x4c4750`, `0x4c4e30`, `0x4c5c20`, `0x4c9a60`, `0x4ca5f0`, `0x4cbbf0`, `0x4d0410`, `0x4d2450`, `0x4d4100`, `0x4d4d20`, `0x4d6c70`, `0x4da7f0`, `0x4dcf40`, `0x4e1ab0`, `0x4e2b80`, `0x4e3bd0`, `0x4e4ac0`, `0x4e77b0`, `0x4e9c10`, `0x4f1bb0`, `0x4f3850`, `0x4f4a30`, `0x4f5390`, `0x4f5970`, `0x4f7da0`, `0x4f8830`, `0x4fa7c0`, `0x4fc1e0`, `0x500350`, `0x502340`, `0x51bb40`, `0x51bb90`
Confidence: verified
Proof: one
Test: none

The addresses are the 43 unregistrars and their caller `0x490ce0`. The loop is `push esi; xor esi,esi; mov ecx,[esi+table]; call 0x704160; add esi,8; cmp esi,count*8; jb`. `0x490ce0` unregisters the tabard functions, the frame types, the method tables, the 216 game functions and then these files.

## Method table unregistrars

21 wrappers, each `mov ecx,table; jmp 0x701f20`, release the method hash that the register function at the address 0x20 below (for example `0x5368a0` for the GameTooltip) built. `0x490ce0` calls them after `UnloadFrameScriptFunctions`.

Addresses: `0x4c1bd0`, `0x4ef310`, `0x503bc0`, `0x5050c0`, `0x506250`, `0x5368c0`, `0x76f860`, `0x782c80`, `0x78dc80`, `0x78f0d0`, `0x7900c0`, `0x790ea0`, `0x791be0`, `0x7941b0`, `0x795b90`, `0x796f70`, `0x799ad0`, `0x79a5c0`, `0x79c610`, `0x79ee10`, `0x7a10f0`
Confidence: verified
Proof: one
Test: none

`0x5368a0` pushes the hash table object 0xc0cf20 and the 46-method table for `FrameScriptRegisterMethods`; `0x5368c0` passes the same object to `FrameScriptUnregisterMethods`. The pair of every wrapper sits 0x20 bytes below it.
