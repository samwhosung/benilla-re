# net: messages

How the handlers of other systems read the packets the dispatcher gives them.

Every handler registered through `dispatch` receives its packet as a data store (`CDataStore`, from the base library) whose cursor stands at the first byte after the opcode, and reads the body field by field with the store's `Get` functions; the layout of a packet is the order and width of those reads. The functions are shared by every packet in the client. This doc holds the rules they have in common, which a reimplementation of any handler must reproduce, and the packets whose handlers belong to no other system's doc.

Roots: `0x537aa0`

## Data store reads

A data store holds a size at `+0x10` and a cursor at `+0x14`. Each read tests `cursor + n > size`, as an unsigned add with no overflow guard. On failure it sets the cursor to `size + 1`, so every later read on the store fails too; it sets no error flag and returns the store as it does on success. The fixed-width reads leave their destination unwritten on failure. Four of them, `0x418e30`, `0x418eb0`, `0x418fb0` and `0x419130`, are the same 121 bytes: a field's type, integer or float, comes from the code that uses it.

Addresses: `0x418cb0`, `0x418db0`, `0x418e30`, `0x418eb0`, `0x418fb0`, `0x419130`, `0x4190b0`, `0x419400`, `0x418cbf`, `0x418cc4`
Confidence: verified
Proof: one
Test: none

`0x418cbc` to `0x418cc2` are `lea edx,[eax+1]; cmp edx,ecx; jbe` on the cursor and size, and `0x418cc4` is `inc ecx; mov [esi+0x14],ecx`. `0x418db0`, `0x4190b0` and `0x419400` open with the same test for 2, 8 and the requested count. The four dword reads have no sign or zero extension and no floating-point instruction; their bytes are identical.

## Data store strings and pointers

`GetDataInSitu` (`0x419ac0`, destination, `n`) first stores 0 in its destination, so a failed read gives a null pointer; on success it stores a pointer into the packet buffer and advances the cursor by `n`, copying nothing. `GetCString` (`0x4191b0`, destination, capacity) copies bytes up to and including the packet's NUL. When the cursor is past the size at exit it stores 0 at the destination's first byte: a string of `capacity` or more characters, or one with no NUL before the end of the packet, reads as `""` and poisons the store.

Addresses: `0x419ac0`, `0x4191b0`, `0x419acc`, `0x419b3b`, `0x419274`, `0x4192a1`, `0x4192aa`, `0x4192b9`, `0x4192bb`
Confidence: verified
Proof: one
Test: none

`0x419acc` is `mov dword [eax],0` before any test; `0x419b3b` stores the pointer. In `0x4191b0`, the copy loop `0x419274` to `0x41928d` writes each byte, stopping after the NUL; `0x4192a1` `cmp edx,[ebp+0xc]; jb` ends the loop at the capacity, `0x4192aa` poisons the cursor, and `0x4192b9` `jbe` sends a poisoned store to `0x4192bb`, which writes 0 at `[ebp+8]`, the original destination.

## Time speed packet

`SMSG_LOGIN_SETTIMESPEED` (`0x42`) is a u32 packed date and a float rate. The packed date is the fields minute (bits 0 to 5), hour (6 to 10), weekday (11 to 13), day (14 to 19), month (20 to 23), year (24 to 28) and a two-bit field (29 and 30); a field whose bits are all ones reads as `-1`. A packet not read to its end logs `Malformed message recieved: Id = %d, Len = %d, Read = %d` and the handler returns 0. Otherwise it passes the date to the clock object `0xce8538`, calls `0x6c5f50`, passes the rate to the clock and logs `Gamespeed set from <old> to <new>`. The three callees are not read here.

Addresses: `0x6c5e80`, `0x642c30`, `0x642b30`, `0x6c5e96`, `0x6c5ea1`, `0x6c5eaa`, `0x6c5ed9`, `0x6c5ee9`, `0x6c5eee`, `0x6c5efc`
Confidence: verified
Proof: one
Test: none

`0x6c5e96` reads the dword with `0x418eb0` and `0x6c5ea1` the rate with `0x419130`; `0x6c5eaa` is `call [edx+0x14]`, the store's read-to-end test. `0x6c5ed9` calls `0x642c30`, which passes the seven field addresses of its output record, `+0` to `+0x18`, to `0x642b30`; that function extracts each field with a shift and mask and stores `-1` for an all-ones value (`0x642b39` to `0x642bee`): minute `& 0x3f`, hour `>> 6 & 0x1f`, weekday `>> 0xb & 7`, day `>> 0xe & 0x3f`, month `>> 0x14 & 0xf`, year `>> 0x18 & 0x1f`, and `>> 0x1d & 3`. `0x6c5ee9` is `0x6434d0` on the clock with the record, `0x6c5eee` calls `0x6c5f50` and `0x6c5efc` is `0x643820` with the rate; the old speed in the log is its return.
