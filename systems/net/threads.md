# net: threads

The threads that read and write the sockets, and how they hand work to each other.

`WowConnectionNet` (`WowConnection.cpp` and the files around it) is the engine under every `WowConnection`, of the logon link and the world session alike. It runs one thread named `Network`, which selects on the sockets, and up to 32 worker threads named `Net Thread <n>`, which run the handlers of a connection that the select loop found ready: the receive loop `0x5b65f0`, the send queue drain, the connect finish and the close. Everything a socket delivers is therefore handled on a worker; the main thread sees it only through the queue of `dispatch`. `wire` gives the framing and the cipher those handlers apply.

Roots: `0x5bd910`, `0x5bda30`, `0x5bd6e0`, `0x5bcad0`, `0x5bd5f0`

## Starting the threads

`WowConnectionNet::Start` creates one thread for each of the engine's worker count (at most 32), each named `Net Thread <n>` and running `WorkerProc`, which runs `RunWorker(n)`. It then creates the thread `Network` running `0x5bda10`, which runs the select loop and, on its return, sets the engine's stop event. `Start` returns once the flag `+0x65d` is set, which the select loop sets as it begins; it polls for it every 100 ms. The engine keeps a slot of `0x30` bytes per worker from `+0x1c`: the connection it is serving, its event and its stop flag.

Addresses: `0x5bd910`, `0x5bd956`, `0x5bd9f0`, `0x5bda10`, `0x5bdfe0`, `0x5bdb50`, `0x5bd5f0`, `0x5bda30`, `0x659ac0`
Confidence: verified
Proof: one
Test: none

`0x5bd923` loops while the index is below `[ebx+0xc]` and below `0x20`; `0x5bd943` formats the name `Net Thread %d` and `0x5bd95b` calls `0x659ac0` with the procedure `0x5bd9f0` (`0x5bd956`). `0x5bd99d` formats `Network` and `0x5bd9c3` calls `0x659ac0` with `0x5bda10`. `0x5bd9c8` to `0x5bd9e4` poll `+0x65d` with `0x429d50(0x64)`. `0x5bda10` calls `0x5bdfe0`, which runs `0x5bcb00` and then sets the event at `+4`. The slots beyond the worker count get a stop flag of 1 (`0x5bd982`).

## Select loop

`0x5bcb00` builds the wake pipe, then repeats until the stop flag is set: it puts each listed connection into the read, write and exception sets by its state, selects, drains the wake pipe, and hands each connection with a ready socket to a worker. State 2 (connecting) goes in the write and exception sets; 3 (listening) in the read set; 5 (connected) in the read and exception sets, and in the write set only while its send queue is not empty or it asked for a write notice; 7 (closing) is counted and closed. The timeout is 30 seconds, or zero when a connection is closing or has events pending. The wake pipe is always in the read set.

Addresses: `0x5bcb00`, `0x5bcb18`, `0x5bcb24`, `0x5bcbcc`, `0x5bcf70`, `0x5bce70`, `0x5bce84`, `0x5bcea1`, `0x5bcf2a`, `0x5bcf90`, `0x5bcad0`, `0x5bdc10`
Confidence: verified
Proof: one
Test: none

`0x5bcb18` calls `0x5bcf90` on the pipe pair `0x85e9e0`; `0x5bcb24` sets `+0x65d`. The three sets are arrays of up to `0x400` sockets with a leading count, at `[ebp-0x1030]` (read, with `0x85e9e0` first), `[ebp-0x2034]` (write) and `[ebp-0x3038]` (exception); `0x5bcbcc` jumps through `0x5bcf70` by the state less 2: states 2, 3, 5 and 7 have arms, 4 and 6 none. `0x5bcc4a` to `0x5bcc66` test the send queue `+0xd0` and `+0x118`. `[ebp-0x2c]` is the timeout's seconds, `0x1e`; it is zeroed at `0x5bce23` for a closing connection and at `0x5bce4c` for one with events pending in `+0xe8`. `0x5bce70` is the call of `select`, WSOCK32 ordinal 18; `0x5bce84` tests the pipe with `__WSAFDIsSet` and `0x5bcea1` reads it a byte at a time (ordinal 16) until a read returns 0 or less. For each connection `0x5bcec3` to `0x5bcf04` build the bits: 1 writable, 2 readable, 4 exception, 8 for state 7, and 2 when events are pending at `+0xec`; `0x5bcf2a` calls `0x5bdc10`.

## Handing work to workers

`WowConnectionNet::SignalEvent(connection, bits)` waits up to 500 ms for an idle worker. On a timeout it returns without posting and the bits are not kept; the select loop tests every socket again on its next pass. Otherwise it stores the bits in the connection's `+0xe0`, assigns the connection to the first free slot and sets the slot's event. A worker waits on its event in 1,000 ms steps until its stop flag is set; on waking it takes the bits, runs the handler of each: bit 1 `0x5b6370`, bit 2 `0x5b6bc0`, bit 4 `0x5b6c40`, bit 8 `0x5b53a0`, repeats while more bits have accumulated, then frees its slot, releases the idle semaphore and wakes the select thread.

Addresses: `0x5bdc10`, `0x5bdc23`, `0x5bdb50`, `0x5bdb88`, `0x5bdbb2`, `0x5bdae0`, `0x5b6370`, `0x5b6bc0`, `0x5b6c40`, `0x5b53a0`, `0x5bdbf4`
Confidence: verified
Proof: one
Test: none

`0x5bdc18` pushes `0x1f4` and `0x5bdc23` waits on the semaphore at `+0x634`; a non-zero result jumps to the return (`0x5bdc2a`). `0x5bdc3f` stores the bits in `+0xe0`; the slot scan at `0x5bdc53` to `0x5bdc74` takes the first slot whose connection field `+0x1c` is zero, stores the connection (`0x5bdc8c`) and sets the event at `+0x20` (`0x5bdc97`). In `0x5bdb50`, `0x5bdb80` waits `0x3e8` on the slot event and loops while it times out; `0x5bdb96` tests the stop flag `+0x24`; `0x5bdbb2` calls `0x5bdae0` with the connection and the bits. `0x5bdaf2` to `0x5bdb1d` test bits 1, 2, 4 and 8; `0x5bdb29` takes `+0xe0` again under the lock `+0x610` and loops when it is not zero. `0x5bdbf4` calls `0x5bcad0`, the wake byte.
