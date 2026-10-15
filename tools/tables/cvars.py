#!/usr/bin/env python3
"""tables/cvars.tsv: every CVar registration in the client, with its decoded arguments.

CVar::Register 0x63db90 reads its arguments off its own stores:

    ecx        name                 edx        help text (null for none)
    [ebp+0x08] flags                [ebp+0x0c] default value, a string
    [ebp+0x10] change callback      [ebp+0x14] category
    [ebp+0x18] script hidden        [ebp+0x1c] callback argument

and returns with `ret 0x18`: six stack dwords, pushed right to left, so walking back from the
call the pushes are flags, default, callback, category, script-hidden, callback argument. A zero script-hidden sets flag bit 31 (0x63dc78,
0x63ddb4), and the script lookup 0x63dec0 finds only records with that bit. The
record pointer the call returns is stored by the `mov [global], eax` that follows it (MSVC
schedules a call's result store after it), up to the next call, jmp or ret.

A register argument is resolved backward through the idioms MSVC uses to hoist a constant:
`xor R,R`, `mov R,imm`, `lea R,[imm]` and `mov R,R2`, stopping at any call for eax, ecx and edx.
A value that is not a constant at the site (a stack slot, a table entry, a register set
elsewhere) is `?` in the table and named in `notes`; nothing is guessed. A default selected by a
condition (`mov R,A; jcc L; mov R,B; L:`) is listed as `A|B`, both arms, and noted
`default-conditional`.

A registration made by a loop or from a table (`mov ecx,[esi+T]`, a name built by string appends
from several tables) has no constant at its site. For each function that holds a site with an
unresolved argument, `walk.py` runs the function's instructions from its start with concrete
registers and memory, reading tables from the image, and records every call to `CVar::Register`
with its arguments read at the call: one row per call, the same `site` on each, noted `looped`
where a site registers more than once. A function whose run leaves a name or a default unknown
keeps the static rows.
A run that stops (a branch on a value only the running client knows, an instruction it does not
model) or that reads an image address some instruction in the listing writes leaves that
function's rows as the static pass has them. A row's text arguments (name, help, default) are
read where the pointers point at the call, so a name built in a stack buffer is the built name.

Columns that hold an address are `name_address`, `default_address`, `callback` and `record`. A name built in a buffer has no
address: `name_address` is empty and `notes` says `name-built`.
`flags`, `category`, `script_hidden` and `callback_arg` are decimal. A null help text or
callback is an empty cell.
"""

import re
import sys

from lib import Disasm, Functions, Image, addr, note, parse_args, write_table
from walk import Unknown, Walk

REGISTER = 0x63DB90
BACK = 640
STACK = 0xF00000
VOLATILE = ('eax', 'ecx', 'edx')
GPR = ('eax', 'ebx', 'ecx', 'edx', 'esi', 'edi', 'ebp')
ABS = re.compile(r'^(?:\w+ PTR )?ds:0x([0-9a-f]+)$')


def num(s):
    s = s.strip()
    if re.fullmatch(r'0x[0-9a-f]+', s):
        return int(s, 16)
    return None


class Window:
    """The straight run of instructions before a call, nearest last."""

    def __init__(self, dis, site):
        k = dis.at[site]
        j = k
        while j > 0 and dis.ins[j - 1][0] >= site - BACK:
            j -= 1
        self.ins = [(a, mn, ops) for a, _, mn, ops in dis.ins[j:k]]


def resolve_reg(w, idx, reg, depth=0):
    """-> (value | (armA, armB) | None, definition address | reason)."""
    if depth > 4:
        return None, 'depth'
    for j in range(idx - 1, -1, -1):
        a, mn, ops = w.ins[j]
        if mn == 'call':
            if reg in VOLATILE:
                return None, 'clobbered by the call at 0x%x' % a
            continue
        if ',' not in ops:
            continue
        dst, src = [x.strip() for x in ops.split(',', 1)]
        if dst != reg:
            continue
        if mn == 'xor' and src == reg:
            return 0, a
        if mn == 'mov':
            v = num(src)
            if v is not None:
                if j >= 1:
                    pa, pmn, pops = w.ins[j - 1]
                    if pmn.startswith('j') and pmn != 'jmp' and num(pops) == a + _size(w, j):
                        alt, _ = resolve_reg(w, j - 1, reg, depth + 1)
                        if isinstance(alt, int) and alt != v:
                            return (alt, v), a
                return v, a
        if mn == 'lea':
            m = ABS.match(src)
            if m:
                return int(m.group(1), 16), a
        if mn == 'mov' and src in GPR:
            return resolve_reg(w, j, src, depth + 1)
        return None, '%s %s at 0x%x' % (mn, ops, a)
    return None, 'not set in the window'


def _size(w, j):
    return (w.ins[j + 1][0] - w.ins[j][0]) if j + 1 < len(w.ins) else 0


def reg_arg(w, reg):
    """The value of ecx or edx at the call: the nearest mov, lea or xor into it, else unknown."""
    for k in range(len(w.ins) - 1, -1, -1):
        a, mn, ops = w.ins[k]
        if mn == 'call':
            return None, 'clobbered'
        if ',' not in ops or ops.split(',')[0].strip() != reg:
            continue
        if mn == 'xor' and ops.split(',')[1].strip() == reg:
            return 0, a
        if mn in ('mov', 'lea'):
            src = ops.split(',', 1)[1].strip()
            v = num(src)
            if v is not None:
                return v, a
            m = ABS.match(src)
            if mn == 'lea' and m:
                return int(m.group(1), 16), a
            if mn == 'mov' and src in GPR:
                return resolve_reg(w, k, src)
            return None, src
    return None, 'not set'


def analyse(w):
    pushes = []
    for k in range(len(w.ins) - 1, -1, -1):
        a, mn, ops = w.ins[k]
        if mn == 'push' and len(pushes) < 6:
            v = num(ops)
            if v is not None:
                pushes.append((v, a))
            elif ops in GPR:
                pushes.append(resolve_reg(w, k, ops))
            else:
                pushes.append((None, 'non-constant'))
    pushes += [(None, 'missing')] * (6 - len(pushes))
    return pushes, reg_arg(w, 'ecx'), reg_arg(w, 'edx')


WRITERS = ('mov', 'add', 'sub', 'and', 'or', 'xor', 'inc', 'dec', 'neg', 'not', 'shl', 'shr',
           'sar', 'xchg', 'adc', 'sbb', 'fstp', 'fst', 'fistp', 'stos', 'movs')
CONST = re.compile(r'0x([0-9a-f]+)')


def written(dis, spans):
    """The addresses of instructions whose memory destination holds a constant inside a span."""
    hits = []
    for a, _, mn, ops in dis.ins:
        if mn not in WRITERS or ',' not in ops and mn not in ('inc', 'dec', 'neg', 'not'):
            continue
        dest = ops.split(',', 1)[0]
        if '[' not in dest and 'ds:' not in dest:
            continue
        for m in CONST.finditer(dest):
            v = int(m.group(1), 16)
            if v >= 0x400000 and any(lo - 3 <= v <= hi for lo, hi in spans):
                hits.append(a)
                break
    return hits


def spans_of(reads):
    spans, lo, hi = [], None, None
    for r in sorted(reads):
        if lo is None or r > hi + 8:
            if lo is not None:
                spans.append((lo, hi))
            lo = r
        hi = r
    if lo is not None:
        spans.append((lo, hi))
    return spans


def walk_function(dis, img, fns, start):
    """Every `CVar::Register` call a run of the function at `start` makes, as
    {site: [dict of the arguments]}, or the reason the run could not be trusted."""
    w = Walk(dis, img, start, fns)
    found = {}

    def on_call(w, site, target):
        if target != REGISTER:
            return False
        esp = w.reg['esp']
        arg = [w.read_mem(esp + 4 * i, 4) for i in range(6)]
        ecx, edx = w.reg['ecx'], w.reg['edx']
        text = lambda p: '' if p == 0 else w.cstring(p)  # noqa: E731
        found.setdefault(site, []).append(dict(
            name_ptr=ecx, name=text(ecx), help=text(edx), flags=arg[0], default_ptr=arg[1],
            default=text(arg[1]), callback=arg[2], category=arg[3], hidden=arg[4], cbarg=arg[5]))
        w.reg['esp'] = (esp + 0x18) & 0xFFFFFFFF
        w.reg['eax'] = w.reg['ecx'] = w.reg['edx'] = None
        return True

    try:
        w.run(on_call)
    except Unknown as e:
        return 'stopped: %s' % e
    w.reads = {r for r in w.reads if img.section_of(r) not in (None, '.text')}
    if written(dis, spans_of(w.reads)):
        return 'a table it reads is written'
    return found


def main():
    args = parse_args(__doc__, disasm=True)
    img, dis = Image(args.exe), Disasm(args.disasm)
    sites = [a for a, _, mn, ops in dis.ins
             if mn in ('call', 'jmp') and num(ops) == REGISTER]
    rows, unresolved = [], 0
    for site in sites:
        w = Window(dis, site)
        pushes, (ecx, _), (edx, _) = analyse(w)
        notes = []

        def string(v, label):
            """-> (text, address text) for a pointer-to-string argument."""
            if v is None:
                notes.append(label + '-computed')
                return '?', '?'
            if isinstance(v, tuple):
                ss = [img.cstr(x) for x in v]
                notes.append(label + '-conditional')
                return '|'.join('?' if s is None else s for s in ss), '|'.join(addr(x) for x in v)
            if v == 0:
                return '', ''
            s = img.cstr(v)
            if s is None:
                notes.append(label + '-not-a-string')
                return '?', addr(v)
            return s, addr(v)

        def number(p, label):
            v = p[0]
            if v is None or isinstance(v, tuple):
                notes.append(label + '-computed')
                return '?'
            return str(v)

        name, name_addr = string(ecx, 'name')
        help_, _ = string(edx, 'help')
        flags = number(pushes[0], 'flags')
        default, default_addr = string(pushes[1][0], 'default')
        cb = pushes[2][0]
        if cb is None or isinstance(cb, tuple):
            callback = '?'
            notes.append('callback-computed')
        else:
            callback = addr(cb) if cb else ''
        category = number(pushes[3], 'category')
        hidden = number(pushes[4], 'script_hidden')
        cbarg = number(pushes[5], 'callback_arg')

        recs = []
        k = dis.at[site]
        for a, _, mn, ops in dis.ins[k + 1:k + 40]:
            if a >= site + 5 + 96 or mn in ('call', 'jmp', 'ret', 'retn'):
                break
            if mn == 'mov' and ops.endswith(',eax'):
                m = ABS.match(ops.split(',')[0])
                if m:
                    recs.append(addr(int(m.group(1), 16)))
        record = '|'.join(recs)
        if len(recs) > 1:
            notes.append('record-ambiguous')
        if '?' in (name, default, flags, callback, category, hidden, cbarg, help_):
            unresolved += 1
        rows.append((addr(site), name, name_addr, default, default_addr, flags, callback,
                     category, hidden, cbarg, record, help_, ';'.join(notes)))
    fns, walked, looped = Functions(), {}, 0
    out = []
    for row in rows:
        site = int(row[0], 16)
        if '?' not in row[1:11] or '-computed' not in row[12] and '-not-a-string' not in row[12]:
            out.append(row)
            continue
        start = fns.holding(site)['address']
        if start not in walked:
            walked[start] = walk_function(dis, img, fns, start)
        got = walked[start]
        if isinstance(got, str) or site not in got:
            if isinstance(got, str):
                note('%s: %s' % (addr(start), got))
            out.append(row)
            continue
        if any(r['name'] is None or r['default'] is None for r in got[site]):
            out.append(row)
            continue
        many = len(got[site]) > 1
        for r in got[site]:
            cb = '?' if r['callback'] is None else (addr(r['callback']) if r['callback'] else '')
            built = r['name_ptr'] >= STACK
            notes = (['looped'] if many else []) + (['name-built'] if built else [])
            out.append((row[0], r['name'], '' if built else addr(r['name_ptr']), r['default'],
                        addr(r['default_ptr']) if r['default_ptr'] else '',
                        *(('?' if r[k] is None else str(r[k]))
                          for k in ('flags',)), cb,
                        *(('?' if r[k] is None else str(r[k]))
                          for k in ('category', 'hidden', 'cbarg')),
                        '' if many else row[10], r['help'] or '', ';'.join(notes)))
            looped += 1
    rows = out
    unresolved = sum(1 for r in rows if '?' in r[1:12])
    note('%d registrations, %d with a computed argument, %d from walked loops'
         % (len(rows), unresolved, looped))
    write_table(['site', 'name', 'name_address', 'default', 'default_address', 'flags',
                 'callback', 'category', 'script_hidden', 'callback_arg', 'record', 'help',
                 'notes'], rows, args.out)


if __name__ == '__main__':
    sys.exit(main())
