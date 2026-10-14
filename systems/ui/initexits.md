# ui: start-up counts and exit callbacks

A call thunk (`ui/initthunks`) runs its body and then jumps to a registrar, which hands one callback to the C runtime's exit list through `0x409aef`. 206 thunks of the ui files register 206 different callbacks. The callbacks uncount the shared start-up count, free blocks, unlink the entries of lists and hash tables, release arrays, or return at once. The bodies that make what they undo are `ui/initstores`.

Roots: `0x4805b0`, `0x4c53a0`, `0x4f7e50`, `0x76fa80`

## Start-up counts

56 bodies count one more user in a global shared by files across the client: `mov eax,[c]; inc eax; cmp eax,1; mov [c],eax; jne; jmp 0x5b71c0`. The first user jumps to `0x5b71c0`, a lone `ret` (`net/layout`).

Addresses: `0x4842e0`, `0x48cb00`, `0x4a2150`, `0x4a9320`, `0x4ac490`, `0x4affa0`, `0x4b19e0`, `0x4b4bc0`, `0x4b5b20`, `0x4b8950`, `0x4bc1e0`, `0x4bec40`, `0x4c0900`, `0x4c1320`, `0x4c3880`, `0x4c47d0`, `0x4c4eb0`, `0x4c6080`, `0x4c9ad0`, `0x4ca670`, `0x4cbc70`, `0x4d0440`, `0x4d25c0`, `0x4d42c0`, `0x4d4da0`, `0x4d6d80`, `0x4da820`, `0x4dd380`, `0x4e1ae0`, `0x4e2c00`, `0x4e3c50`, `0x4e4b40`, `0x4e77e0`, `0x4e9c90`, `0x4ef400`, `0x4f2070`, `0x4f39a0`, `0x4f4ab0`, `0x4f5410`, `0x4f59f0`, `0x4f7e20`, `0x4f88b0`, `0x4fa840`, `0x4fc260`, `0x500380`, `0x502370`, `0x503db0`, `0x505170`, `0x506350`, `0x506b50`, `0x508b50`, `0x515900`, `0x51c310`, `0x528e00`, `0x72c5b0`, `0x7cbf20`
Confidence: verified
Proof: one
Test: none

`0x4842e0` is the first. All 56 count in the dword at 0xc2a3c4. Each is reached by a call thunk whose callback uncounts it.

## Exit registrars

206 registrars, 12 bytes each: `push callback; call 0x409aef; pop ecx; ret`. The call is the C runtime's `atexit`; the callback address is the dword pushed.

Addresses: `0x4805a0`, `0x4805f0`, `0x484300`, `0x484a40`, `0x484c00`, `0x484c60`, `0x48bb90`, `0x48bbf0`, `0x48cb20`, `0x497770`, `0x497860`, `0x497950`, `0x497a30`, `0x497ab0`, `0x497b40`, `0x497ba0`, `0x497ca0`, `0x497ef0`, `0x498140`, `0x4a2170`, `0x4a5b30`, `0x4a5be0`, `0x4a5c50`, `0x4a5cc0`, `0x4a9340`, `0x4a9ae0`, `0x4a9b50`, `0x4a9bb0`, `0x4a9c10`, `0x4ac4b0`, `0x4ac8d0`, `0x4ac940`, `0x4affc0`, `0x4b0730`, `0x4b0830`, `0x4b0920`, `0x4b1a00`, `0x4b1ee0`, `0x4b1f50`, `0x4b1fc0`, `0x4b2030`, `0x4b20a0`, `0x4b2110`, `0x4b2180`, `0x4b4be0`, `0x4b5180`, `0x4b51f0`, `0x4b5270`, `0x4b5b40`, `0x4b6000`, `0x4b8970`, `0x4bc200`, `0x4bc650`, `0x4bec60`, `0x4c0920`, `0x4c1340`, `0x4c1a20`, `0x4c38a0`, `0x4c47f0`, `0x4c4ed0`, `0x4c5390`, `0x4c5550`, `0x4c60a0`, `0x4c6cd0`, `0x4c9af0`, `0x4ca690`, `0x4cbc90`, `0x4cc0b0`, `0x4d0460`, `0x4d0970`, `0x4d25e0`, `0x4d2a00`, `0x4d2a70`, `0x4d42e0`, `0x4d4dc0`, `0x4d51e0`, `0x4d5250`, `0x4d6da0`, `0x4d7290`, `0x4d7300`, `0x4da840`, `0x4db060`, `0x4db240`, `0x4db290`, `0x4db300`, `0x4dd3a0`, `0x4e1b00`, `0x4e2c20`, `0x4e3130`, `0x4e3c70`, `0x4e4090`, `0x4e4b60`, `0x4e7800`, `0x4e7c20`, `0x4e9cb0`, `0x4ea210`, `0x4ea400`, `0x4ea460`, `0x4ea650`, `0x4ea6b0`, `0x4ea710`, `0x4ea760`, `0x4ea820`, `0x4ea8e0`, `0x4ea930`, `0x4ea9a0`, `0x4eaa10`, `0x4ef160`, `0x4ef420`, `0x4efc00`, `0x4efdc0`, `0x4f2090`, `0x4f2bc0`, `0x4f39c0`, `0x4f3de0`, `0x4f4ad0`, `0x4f5430`, `0x4f5a10`, `0x4f5e30`, `0x4f5ea0`, `0x4f7e40`, `0x4f88d0`, `0x4fa860`, `0x4fc280`, `0x4fc6a0`, `0x4fc710`, `0x5003a0`, `0x502390`, `0x503a10`, `0x503dd0`, `0x504f10`, `0x505190`, `0x5060a0`, `0x506370`, `0x506790`, `0x5067e0`, `0x506840`, `0x506930`, `0x506b70`, `0x508b70`, `0x508fa0`, `0x509020`, `0x509110`, `0x515920`, `0x51c330`, `0x51c420`, `0x51c5d0`, `0x51c640`, `0x5242d0`, `0x5243c0`, `0x524540`, `0x524790`, `0x5249e0`, `0x528e20`, `0x528e60`, `0x528ea0`, `0x528ee0`, `0x528f20`, `0x528f60`, `0x528fa0`, `0x528fe0`, `0x529020`, `0x529060`, `0x5290a0`, `0x5290e0`, `0x529120`, `0x529160`, `0x5291a0`, `0x529220`, `0x536710`, `0x6ed950`, `0x6eda40`, `0x6eda80`, `0x7019e0`, `0x728f30`, `0x728fc0`, `0x72c5d0`, `0x7670e0`, `0x76f6b0`, `0x76fa10`, `0x76fa70`, `0x76fad0`, `0x7783d0`, `0x779470`, `0x782ad0`, `0x7836e0`, `0x78dad0`, `0x78ef20`, `0x78ff10`, `0x790cf0`, `0x791a30`, `0x794000`, `0x7959e0`, `0x796dc0`, `0x799920`, `0x79a410`, `0x79c460`, `0x79ec60`, `0x7a0f40`, `0x7a2ce0`, `0x7cb130`, `0x7cb170`, `0x7cb1b0`, `0x7cb1f0`, `0x7cb230`, `0x7cbf40`
Confidence: verified
Proof: one
Test: none

`0x4805a0` pushes `0x4805b0`. Each registrar is the target of the `jmp` that ends its thunk.

## Exit counts

56 callbacks uncount: `dec [c]; jne ret; jmp 0x5b71d0; ret`. The last file to exit jumps to `0x5b71d0`, a lone `ret`.

Addresses: `0x484310`, `0x48cb30`, `0x4a2180`, `0x4a9350`, `0x4ac4c0`, `0x4affd0`, `0x4b1a10`, `0x4b4bf0`, `0x4b5b50`, `0x4b8980`, `0x4bc210`, `0x4bec70`, `0x4c0930`, `0x4c1350`, `0x4c38b0`, `0x4c4800`, `0x4c4ee0`, `0x4c60b0`, `0x4c9b00`, `0x4ca6a0`, `0x4cbca0`, `0x4d0470`, `0x4d25f0`, `0x4d42f0`, `0x4d4dd0`, `0x4d6db0`, `0x4da850`, `0x4dd3b0`, `0x4e1b10`, `0x4e2c30`, `0x4e3c80`, `0x4e4b70`, `0x4e7810`, `0x4e9cc0`, `0x4ef430`, `0x4f20a0`, `0x4f39d0`, `0x4f4ae0`, `0x4f5440`, `0x4f5a20`, `0x4f7e50`, `0x4f88e0`, `0x4fa870`, `0x4fc290`, `0x5003b0`, `0x5023a0`, `0x503de0`, `0x5051a0`, `0x506380`, `0x506b80`, `0x508b80`, `0x515930`, `0x51c340`, `0x528e30`, `0x72c5e0`, `0x7cbf50`
Confidence: verified
Proof: one
Test: none

`0x484310` is the first; each pairs with a start-up count body.

## Exit frees

46 callbacks free one block through `SMemFree` (`0x646430`): they test the global that holds it, push 0, -2, the type name and the block, and call. 34 first set a bit of a guard byte, so a second run skips the free. The type name is a compiler-mangled string such as .H for int or .?AUNearestUnitData@@ for a struct; the blocks are of 35 types, int (9) the commonest.

Addresses: `0x484c70`, `0x48bba0`, `0x48bc00`, `0x497a40`, `0x497b50`, `0x497bb0`, `0x4a5bf0`, `0x4a5c60`, `0x4a5cd0`, `0x4a9af0`, `0x4ac8e0`, `0x4ac950`, `0x4b1ef0`, `0x4b1f60`, `0x4b1fd0`, `0x4b2040`, `0x4b20b0`, `0x4b2120`, `0x4b2190`, `0x4b5190`, `0x4b5200`, `0x4bc660`, `0x4c5560`, `0x4c6ce0`, `0x4cc0c0`, `0x4d0980`, `0x4d2a10`, `0x4d2a80`, `0x4d51f0`, `0x4d5260`, `0x4d72a0`, `0x4d7310`, `0x4db2a0`, `0x4e3140`, `0x4ea6c0`, `0x4ea940`, `0x4efdd0`, `0x4f2bd0`, `0x4f3df0`, `0x4f5e40`, `0x4f5eb0`, `0x4fc6b0`, `0x4fc720`, `0x5067f0`, `0x506850`, `0x51c5e0`
Confidence: verified
Proof: one
Test: none

`0x484c70` tests the global and frees it. `0x4a5bf0` loads a guard byte, tests its bit, sets it and then does the same.

## Exit hash tables

35 callbacks take a hash table apart: they unlink every entry of the main list, unlink the entries of each 12-byte bucket list, free the bucket array through `SMemFree` and clear the head.

Addresses: `0x484a50`, `0x497cb0`, `0x497f00`, `0x498150`, `0x4c1a30`, `0x4c53a0`, `0x4db070`, `0x4ef170`, `0x4efc10`, `0x503a20`, `0x504f20`, `0x5060b0`, `0x51c430`, `0x524550`, `0x5247a0`, `0x5249f0`, `0x536720`, `0x76f6c0`, `0x7783e0`, `0x782ae0`, `0x7836f0`, `0x78dae0`, `0x78ef30`, `0x78ff20`, `0x790d00`, `0x791a40`, `0x794010`, `0x7959f0`, `0x796dd0`, `0x799930`, `0x79a420`, `0x79c470`, `0x79ec70`, `0x7a0f50`, `0x7a2cf0`
Confidence: verified
Proof: one
Test: none

`0x484a50` is the first. It calls three list helpers, `0x496ea0`, `0x496e60` and `0x496be0`, and `SMemFree` once.

## Exit lists

11 callbacks unlink every entry of a list that a list-head body built, then clear the head.

Addresses: `0x497870`, `0x497960`, `0x4b0740`, `0x4b0840`, `0x4b5280`, `0x509030`, `0x509120`, `0x51c650`, `0x5242e0`, `0x5243d0`, `0x7670f0`
Confidence: verified
Proof: one
Test: none

`0x497870` is the first. It loops while the list's first link is neither zero nor marked with bit 0, unlinking each entry, and then clears the head.

## Exit releases

9 callbacks free what the records of a table or the elements of an array point at: a loop over the table with an `SMemFree` or a destructor call for each entry, and for an array its block last.

| Callback | Does |
|---|---|
| `0x497780` | Releases the storage behind what 0x497750 set up, through SMemFree |
| `0x497ac0` | Releases the storage behind what 0x497a70 set up, through SMemFree |
| `0x4a5b40` | Releases the storage behind what 0x4a5b10 set up, through SMemFree, at most once |
| `0x4db310` | Releases the storage behind what 0x4db2d0 set up, through SMemFree |
| `0x4e7c30` | Releases the storage behind what 0x4e7c00 set up, through SMemFree, at most once |
| `0x4ea9b0` | Releases the storage behind what 0x4ea970 set up, through SMemFree |
| `0x508fb0` | Releases the storage behind what 0x508f70 set up, through SMemFree |
| `0x7019f0` | Releases the storage behind what 0x7019c0 set up, through SMemFree |
| `0x728f40` | Releases the storage behind what 0x728f10 set up, through SMemFree, at most once |

Addresses: `0x497780`, `0x497ac0`, `0x4a5b40`, `0x4db310`, `0x4e7c30`, `0x4ea9b0`, `0x508fb0`, `0x7019f0`, `0x728f40`
Confidence: verified
Proof: one
Test: none

`0x497780` is the first; the others differ in record size, in the number of pointers freed per record and in the helper called.

## Exit calls

7 callbacks run one function on a global object: `mov ecx,G; jmp N`. 3 of them test and set a guard bit first.

| Callback | Does |
|---|---|
| `0x4b6010` | Runs 0x419e30 (CStatus::~CStatus) on the object at 0xb7124c |
| `0x6ed960` | Runs 0x6ed970 (`JumpToCStatusDestroy`, the status destructor) on the object at 0xcee9e8 |
| `0x6eda50` | Runs 0x6ef430 on the object at 0xceea00 |
| `0x6eda90` | Runs 0x6ef780 on the object at 0xcee9bc |
| `0x728fd0` | Runs 0x728ff0 on the object at 0xcf062c, at most once |
| `0x76fa20` | Runs 0x7603f0 (CDataAllocatorPool::Destroy) on the object at 0xcf4ce0, at most once |
| `0x76fa80` | Runs 0x7603f0 (CDataAllocatorPool::Destroy) on the object at 0xcf4d10, at most once |

Addresses: `0x4b6010`, `0x6ed960`, `0x6eda50`, `0x6eda90`, `0x728fd0`, `0x76fa20`, `0x76fa80`
Confidence: verified
Proof: one
Test: none

`0x4b6010` is the first; the destructor returns to the exit list. `0x6ed970` is a bare `jmp 0x419e30`, so it is a second name for the destructor.

## Exits that return

32 callbacks are a lone `ret`.

Addresses: `0x4805b0`, `0x480600`, `0x4b0930`, `0x4db250`, `0x4ea410`, `0x4ea720`, `0x4ea770`, `0x4ea830`, `0x4ea8f0`, `0x4eaa20`, `0x5067a0`, `0x506940`, `0x528e70`, `0x528eb0`, `0x528ef0`, `0x528f30`, `0x528f70`, `0x528fb0`, `0x528ff0`, `0x529030`, `0x529070`, `0x5290b0`, `0x5290f0`, `0x529130`, `0x529170`, `0x5291b0`, `0x529230`, `0x7cb140`, `0x7cb180`, `0x7cb1c0`, `0x7cb200`, `0x7cb240`
Confidence: verified
Proof: one
Test: none

`0x4805b0` is the first.

## Exits that mark

8 callbacks only set a bit of a guard byte: `mov cl,[g]; mov al,bit; test al,cl; jne; or cl,al; mov [g],cl; ret`.

Addresses: `0x484c10`, `0x4a9b60`, `0x4a9bc0`, `0x4a9c20`, `0x4e40a0`, `0x4ea220`, `0x4ea660`, `0x76fae0`
Confidence: verified
Proof: one
Test: none

`0x484c10` is the first.

## Other exits

2 callbacks do something else: one runs destructors on a record table, one puts a vtable back.

| Callback | Does |
|---|---|
| `0x4ea470` | Runs array destructors on each of the 512 records that 0x4ea430 set up |
| `0x779480` | Puts the base vtable 0x80131c back in the object at 0xcf4d90 that `0x779440` builds, an inlined destructor |

Addresses: `0x4ea470`, `0x779480`
Confidence: verified
Proof: one
Test: none

`0x4ea470` is the first.
