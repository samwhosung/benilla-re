#!/usr/bin/env python3
"""tables/opcode_sends.tsv: where the client puts an opcode into an outgoing packet.

An outgoing packet is a CDataStore built on the stack (its vtable 0x7ff9e4 stored first). The
first write is the opcode as a dword through CDataStore::PutUInt32 (0x418190, or one of its two
identical copies 0x4181f0 and 0x4182b0), `push <opcode>; lea ecx,[store]; call put`, and the
packet goes out through ClientServices::Send 0x5ab630 (`lea ecx,[store]; call`) or through
NetClient::Send 0x5379a0 (`lea ecx,[store]; push ecx; mov ecx,[client]; call`).

For each call to either send in .text the generator reads the function that holds it, straight
down the listing, and gathers the opcode writes to the send's store: the first write after the
store's vtable is stored, and the first after each unconditional `jmp` or `ret` (a function that
sends one of several opcodes writes each on its own path). The opcode is the pushed immediate,
or a register's value when one `mov reg,imm` reaches the push with no other write to the register
between (note `via-register`). Where neither holds the opcode is `?` (note `opcode-not-constant`),
or the store is not built in the function (note `store-not-built-here`). The scan follows the
listing, not the branches: a send reached from several writes lists each opcode, and a payload
dword equal to an opcode that comes first on a path would be listed too, so a row is a lead to the
opcode's send site, which the finding it links reads. Rows are sorted by opcode, then site;
the unresolved `?` rows come last.

Four kinds of send are resolved beyond the straight scan. A function that takes the opcode as an
argument (PARAM_SENDERS) gives one row per caller whose argument is an immediate, with the
caller's call as the site (note `through-<address>`). A cache lookup that writes the opcode it
reads from its cache object's `+0x30` gets the opcode its cache's constructor call pushes
(note `cache-object`): each constructor is `push` ×6 and `mov ecx,<store>` before the call, the
opcode the fourth push, and a lookup's store is the `mov ecx,<store>` of its first caller. The few
sends whose opcode is a computed register or sits beyond the scan's window are read by hand
(HAND, note `hand-resolved`). NetClient::Send's wrappers `0x5ab630` and `0x728640` are not sites:
their callers are.

`put` is the call that writes the opcode, empty for `?`. `name` is the opcode's name in vmangos's
`Opcodes_1_12_1.h`, from tools/tables/opcode_names.tsv (the client holds no names).
"""

import re
import sys

from lib import Disasm, Functions, addr, note, opcode_names, parse_args, write_table

PUTS = {0x418190, 0x4181F0, 0x4182B0}
SEND_A = 0x5AB630
SEND_B = 0x5379A0
SEND_C = 0x728640
WRAPPERS = {SEND_A, SEND_C}
VTABLE = '0x7ff9e4'

# function -> where its callers put the opcode: ('stack', n) is the nth argument pushed last
# first (n = 0 the last push before the call), ('reg', name) a `mov name,imm` before the call.
PARAM_SENDERS = {
    0x4A00F0: ('reg', 'edx'),    # ChannelCommandOne
    0x4A04B0: ('reg', 'edx'),    # ChannelCommandTwo
    0x600A10: ('stack', 0),      # CGUnit_C::SendMovementPacketNow(op)
    0x600A30: ('stack', 1),      # SendMovementPacket(step time, op, 0, 0)
    0x617FE0: ('stack', 1),      # CMovement::SendFlushAck(time, op, send, ...)
    0x60E0A0: ('stack', 1),      # ProcessLocalMoveEvent(time, op, send, value, counter)
}

# send site -> [(opcode, instruction that supplies it)], read by hand.
HAND = {
    0x4A9EBA: [(0x2D3, 0x4A9EAF)],               # push 0x2d3 at 0x4a9e4d, store held in edi
    0x5AB095: [(0x4A, 0x5AB087), (0x4B, 0x5AB087)],  # 0x4b + (bl != 0 ? -1 : 0) at 0x5ab07a
    0x5AC560: [(0x1CA, 0x5AC502)],               # push 0x1ca at 0x5ac4f3, store in [ebp-0x28]
    0x600B9A: [(0x2C9, 0x600B24)],               # push 0x2c9 at 0x600b24, written by 0x600860
}

CACHE_OFFSET = '+0x30]'

LOOK = 14
IMM = re.compile(r'0x[0-9a-f]+')
REGS = ('eax', 'ebx', 'ecx', 'edx', 'esi', 'edi', 'ebp')


def target(ops):
    return int(ops, 16) if IMM.fullmatch(ops) else None


def key_of(store):
    return store[store.index('['):] if store and store.startswith('[') else store


def opcode_push(ins, j):
    """The operand of the push that feeds the put at ins[j], and the store (the `ecx` the put
    is called on, as a `[ebp-N]` slot or a register), reading back to the previous call or jump."""
    push = store = None
    at = None
    for b in range(1, LOOK):
        _, _, mn, ops = ins[j - b]
        if mn in ('call', 'ret', 'jmp'):
            break
        if mn in ('lea', 'mov') and ops.startswith('ecx,') and store is None:
            store = ops[4:]
        if mn == 'push' and push is None:
            push, at = ops, j - b
    return push, store, at


def writer_of(ins, lo, j, reg):
    """The instruction that last writes `reg` before ins[j], reading back to `lo`: its index, or
    None when a call or a jump lies between (a call clobbers eax, ecx and edx)."""
    for b in range(j - 1, lo - 1, -1):
        _, _, mn, ops = ins[b]
        if ops.startswith(reg + ',') and mn in ('mov', 'lea', 'xor', 'add', 'sub', 'movzx'):
            return b
        if mn == 'pop' and ops == reg:
            return b
        if mn == 'call' and reg in ('eax', 'ecx', 'edx'):
            return None
        if mn in ('ret', 'jmp') or mn.startswith('j'):
            return None
    return None


def register_value(ins, lo, j, reg):
    """The immediate a `mov reg,imm` leaves in `reg` at ins[j], or None."""
    b = writer_of(ins, lo, j, reg)
    if b is not None and ins[b][2] == 'mov' and IMM.fullmatch(ins[b][3][len(reg) + 1:]):
        return int(ins[b][3][len(reg) + 1:], 16)
    return None


def cache_stores(dis):
    """The nine cache constructors' stores and query opcodes: `mov ecx,<store>` after six
    consecutive pushes, the fourth of which is the opcode."""
    out = {}
    ins = dis.ins
    for k, (a, _, mn, ops) in enumerate(ins):
        m = re.fullmatch(r'ecx,(0xc0e[0-9a-f]{3})', ops)
        if mn == 'mov' and m and k >= 6 and all(ins[k - i][2] == 'push' for i in range(1, 7)):
            op = ins[k - 3][3]
            if IMM.fullmatch(op) and ins[k + 1][2] == 'call':
                out[int(m.group(1), 16)] = int(op, 16)
    return out


def callers(ins, fn):
    """Every `call fn` as (instruction index, address)."""
    return [(k, ins[k][0]) for k in range(len(ins))
            if ins[k][2] == 'call' and target(ins[k][3]) == fn]


def caller_argument(ins, k, spec):
    """The immediate that the call at ins[k] passes as the opcode, and the instruction that
    holds it, or (None, None)."""
    kind, which = spec
    pushes = []
    for b in range(1, 12):
        a, _, mn, ops = ins[k - b]
        if mn in ('call', 'ret', 'jmp') or mn.startswith('j'):
            break
        if kind == 'reg' and mn == 'mov' and ops.startswith(which + ',') \
                and IMM.fullmatch(ops[len(which) + 1:]):
            return int(ops[len(which) + 1:], 16), a
        if kind == 'stack' and mn == 'push':
            pushes.append((ops, a))
    if kind == 'stack' and which < len(pushes) and IMM.fullmatch(pushes[which][0]):
        return int(pushes[which][0], 16), pushes[which][1]
    return None, None


def main():
    args = parse_args(__doc__, disasm=True)
    dis, fns = Disasm(args.disasm), Functions()
    names = opcode_names()
    ins = dis.ins
    sends = (SEND_A, SEND_B, SEND_C)
    rows = []        # (opcode or None, site, function, put, notes, path)
    for row in fns.rows:
        k = dis.at.get(row['address'])
        if k is None or row['address'] in WRAPPERS:
            continue
        end = row['address'] + row['size']
        initial = {}     # store -> [(put address, push operand, instruction index)]
        fresh = set()    # stores whose next put starts a path
        j = k
        while j < len(ins) and ins[j][0] < end:
            a, _, mn, ops = ins[j]
            if mn == 'mov' and ops.endswith(',' + VTABLE) and ops.startswith('DWORD PTR [ebp'):
                slot = ops[len('DWORD PTR '):ops.index(',')]
                initial[slot] = []
                fresh.add(slot)
            elif mn in ('jmp', 'ret'):
                fresh = set(initial)
            elif mn == 'call' and target(ops) in PUTS:
                push, store, at = opcode_push(ins, j)
                if store is None and len(initial) == 1:
                    store = next(iter(initial))
                if store is not None:
                    key = key_of(store)
                    if key in fresh or key not in initial:
                        initial.setdefault(key, []).append((a, push, at))
                        fresh.discard(key)
            elif mn == 'call' and target(ops) in sends:
                path = 'A' if target(ops) == SEND_A else 'B'
                store = None
                for b in range(1, 6):
                    _, _, bm, bo = ins[j - b]
                    if bm == 'call':
                        break
                    if target(ops) == SEND_A and bm in ('lea', 'mov') and bo.startswith('ecx,'):
                        store = bo[4:]
                        break
                    if target(ops) != SEND_A and bm == 'lea' \
                            and bo.split(',', 1)[1].startswith('[ebp'):
                        store = bo.split(',', 1)[1]
                        break
                puts = initial.get(key_of(store))
                found = []
                for pa, push, pj in puts or []:
                    if push is not None and IMM.fullmatch(push):
                        found.append((int(push, 16), pa, ''))
                        continue
                    v = register_value(ins, k, pj, push) if push in REGS else None
                    w = writer_of(ins, k, pj, push) if push in REGS else None
                    if v is not None:
                        found.append((v, pa, 'via-register'))
                    elif w is not None and ins[w][3].endswith(CACHE_OFFSET):
                        found.append((None, pa, 'cache-object'))
                    else:
                        found.append((None, pa, 'opcode-not-constant'))
                if a in HAND:
                    found = [(op, pa, 'hand-resolved') for op, pa in HAND[a]]
                elif not puts:
                    found = [(None, None, 'store-not-built-here')]
                elif any(op is not None for op, _, _ in found):
                    found = [f for f in found if f[0] is not None]
                seen = set()
                for op, pa, why in found:
                    if (op, why) not in seen:
                        seen.add((op, why))
                        rows.append((op, a, row['address'], pa, why, path))
            j += 1

    stores = cache_stores(dis)
    out = []
    for op, site, fn, put, why, path in rows:
        if why == 'cache-object':
            held = set()
            for kk, _ in callers(ins, fn):
                for b in range(1, 9):
                    if ins[kk - b][2] == 'call':
                        break
                    if ins[kk - b][2] == 'mov' and ins[kk - b][3].startswith('ecx,0xc0e'):
                        held.add(ins[kk - b][3])
                        break
            opcodes = {stores.get(int(h[4:], 16)) for h in held}
            if len(opcodes) == 1 and None not in opcodes and 0 not in opcodes:
                op = opcodes.pop()
        if fn in PARAM_SENDERS and op is None:
            todo, done = [fn], set()
            while todo:
                f = todo.pop()
                done.add(f)
                for kk, ca in callers(ins, f):
                    value, where = caller_argument(ins, kk, PARAM_SENDERS[f])
                    holder = fns.holding(ca)
                    if holder is None:
                        continue
                    if value is not None:
                        out.append((value, ca, holder['address'], where,
                                    'through-0x%x' % f, path))
                    elif holder['address'] in PARAM_SENDERS:
                        if holder['address'] not in done:
                            todo.append(holder['address'])
                    else:
                        out.append((None, ca, holder['address'], None,
                                    'opcode-not-constant', path))
            continue
        out.append((op, site, fn, put, why, path))
    known = sorted((r for r in out if r[0] is not None and r[0] <= 0x33B),
                   key=lambda r: (r[0], r[1]))
    unresolved = sorted((r for r in out if r[0] is None), key=lambda r: r[1])
    table = []
    for op, site, fn, put, why, path in known + unresolved:
        code = '0x%03x' % op if op is not None else '?'
        if fns.at[fn]['class'] == 'unreached':
            why = (why + ';' if why else '') + 'unreached'
        table.append((code, names.get(code, ''), addr(site), addr(fn), addr(put) if put else '',
                      path, why))
    note('%d rows: %d with an opcode (%d distinct), %d unresolved'
         % (len(table), len(known), len({r[0] for r in known}), len(unresolved)))
    write_table(['opcode', 'name', 'site', 'function', 'put', 'send', 'notes'], table, args.out)


if __name__ == '__main__':
    sys.exit(main())
