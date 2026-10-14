#!/usr/bin/env python3
"""tables/event_sites.tsv: every place the client fires a C-to-Lua event, with the ids it can
fire and the argument format it pushes.

A site is a call or a tail jmp to one of the two dispatchers:

    FrameScript_SignalEvent  0x703e50   fastcall, ecx = event id, no arguments
    SignalEvent2             0x703f50   cdecl, [esp] = event id, [esp+4] = printf-style format,
                                        then one vararg per directive (%f takes eight bytes)

MSVC compiles `fire(cond ? A : B)` as `mov ecx, A; jcc L; mov ecx, B; L: call`, so the nearest
`mov` before the call names one arm only. The generator walks the control-flow graph backward
from each site over every predecessor (the fall-through instruction and every direct jmp or jcc
that targets it) and collects every definition that reaches the site. Per path, a walk ends at
the first definition of what it tracks:

    mov R, imm32          the value is imm32
    mov R, R2             track R2 instead
    push R                (SignalEvent2) track R
    add R, K              track R and add K
    xor R, R              the value is 0
    setcc R8              the value is 0 or 1, given a clearing xor above it
    sbb R, R              the value is 0 or -1

Anything else that writes the tracked register, a call that clobbers it, or a walk that reaches
a function entry (the id is then a parameter) leaves the site unresolved: `?` in the table. The
generator fails toward unknown, never toward a guess, and checks five control sites on every run.

event_ids lists every id that reaches the site, `|`-separated in ascending order. The format is
the string pushed just before the id push; it is a declaration, not a count of what the caller
pushed.

The glue screens run a different event table (tables/glue_events.tsv), so the same integer names a
different event there. The glue code is glue's run of object files, [0x46a260, 0x475b70): its first
function is 0x46a260 and the next file starts at 0x475b70. Sites there are space `glue` and take
their names from the glue table, all others are `game`.
"""

import re
import sys
from collections import defaultdict

from lib import Disasm, Image, addr, note, parse_args, write_table
import events

SE, SE2 = 0x703E50, 0x703F50
GLUE_TU = (0x46A260, 0x475B70)
CONTROLS = {0x46AA34: {22}, 0x4930C7: {208, 209}, 0x49211A: {397, 398},
            0x4945CA: {224, 225}, 0x515E7A: None}

REG = {
    'eax': ('eax', 'ax', 'al', 'ah'), 'ecx': ('ecx', 'cx', 'cl', 'ch'),
    'edx': ('edx', 'dx', 'dl', 'dh'), 'ebx': ('ebx', 'bx', 'bl', 'bh'),
    'esi': ('esi', 'si'), 'edi': ('edi', 'di'), 'ebp': ('ebp',),
}
LOW8 = {'eax': 'al', 'ecx': 'cl', 'edx': 'dl', 'ebx': 'bl'}
IMM = re.compile(r'^0x[0-9a-f]+$')


class Cfg:
    def __init__(self, dis):
        self.ins = [(a, mn, ops) for a, _, mn, ops in dis.ins if 0x401000 <= a < 0x800000]
        self.preds = defaultdict(list)
        self.entries = set()
        for i, (a, mn, ops) in enumerate(self.ins):
            if IMM.match(ops):
                if mn.startswith('j'):
                    self.preds[int(ops, 16)].append(i)
                elif mn == 'call':
                    self.entries.add(int(ops, 16))

    def predecessors(self, i):
        out = []
        if i > 0:
            pm = self.ins[i - 1][1]
            if pm != 'jmp' and not pm.startswith('ret') and pm != 'int3':
                out.append(i - 1)
        out.extend(self.preds.get(self.ins[i][0], ()))
        return out


def dest(ops):
    return re.sub(r'^(BYTE|WORD|DWORD|QWORD) PTR ', '', ops.split(',')[0].strip())


def src(ops):
    return ops.split(',', 1)[1].strip() if ',' in ops else ''


def resolve(cfg, site_i, kind, maxdepth=400, reg='ecx'):
    """-> (set of ints, list of (address, reason) unresolved). For kind SignalEvent the walk tracks
    `reg`, ecx by default."""
    ins = cfg.ins
    vals, bad = set(), []
    work, seen = [], set()

    def push_preds(i, trk, add, d, pend):
        if ins[i][0] in cfg.entries and i != site_i:
            bad.append((ins[i][0], 'reached function entry: the id is a parameter'))
            return
        ps = cfg.predecessors(i)
        if not ps:
            bad.append((ins[i][0], 'no predecessor'))
        for p in ps:
            work.append((p, trk, add, d + 1, pend))

    push_preds(site_i, reg if kind == 'SignalEvent' else 'push', 0, 0, None)
    while work:
        i, trk, add, d, pend = work.pop()
        key = (i, trk, add, pend)
        if key in seen:
            continue
        seen.add(key)
        a, mn, ops = ins[i]
        if d > maxdepth:
            bad.append((a, 'depth limit'))
            continue
        if trk == 'push':
            if mn == 'push':
                if IMM.match(ops):
                    vals.add(int(ops, 16) + add)
                elif ops in REG:
                    push_preds(i, ops, add, d, None)
                else:
                    bad.append((a, 'push ' + ops))
                continue
            if mn == 'call' or (mn in ('add', 'sub') and dest(ops) == 'esp'):
                bad.append((a, '%s %s before the id push' % (mn, ops)))
                continue
            push_preds(i, trk, add, d, pend)
            continue
        names = REG[trk]
        if mn == 'call':
            if trk in ('eax', 'ecx', 'edx'):
                bad.append((a, 'clobbered by call ' + ops))
                continue
            push_preds(i, trk, add, d, pend)
            continue
        dd = dest(ops)
        writes = dd in names and not (mn.startswith('j') or mn in ('cmp', 'test', 'push'))
        if mn == 'xchg' and any(n in (dd, src(ops)) for n in names):
            writes = True
        if mn == 'cdq' and trk == 'edx':
            writes = True
        if not writes:
            push_preds(i, trk, add, d, pend)
            continue
        s = src(ops)
        if pend is not None:
            if mn == 'xor' and dd == trk and s == trk:
                vals.update({0 + add, 1 + add})
            else:
                bad.append((a, 'setcc into %s without a clearing xor (%s %s)' % (trk, mn, ops)))
            continue
        if mn == 'mov' and dd == trk and IMM.match(s):
            vals.add((int(s, 16) + add) & 0xFFFFFFFF)
        elif mn == 'mov' and dd == trk and s in REG:
            push_preds(i, s, add, d, None)
        elif mn == 'add' and dd == trk and IMM.match(s):
            push_preds(i, trk, add + int(s, 16), d, None)
        elif mn.startswith('set') and dd == LOW8.get(trk):
            push_preds(i, trk, add, d, ('setcc', trk))
        elif mn == 'xor' and dd == trk and s == trk:
            vals.add(0 + add)
        elif mn == 'sbb' and dd == trk and s == trk:
            vals.update({0 + add, -1 + add})
        else:
            bad.append((a, '%s %s' % (mn, ops)))
    return vals, bad


def format_of(cfg, img, i):
    """The format string pushed just before the id push of the SignalEvent2 call at index i."""
    pushes = []
    j = i - 1
    while j >= 0 and len(pushes) < 2 and i - j < 60:
        a, mn, ops = cfg.ins[j]
        if mn in ('call', 'ret', 'jmp'):
            break
        if mn == 'push':
            pushes.append(ops)
        j -= 1
    if len(pushes) == 2 and IMM.match(pushes[1]):
        s = img.cstr(int(pushes[1], 16))
        if s is not None and '%' in s:
            return s
    return None


def main():
    args = parse_args(__doc__, disasm=True)
    img, dis = Image(args.exe), Disasm(args.disasm)
    cfg = Cfg(dis)
    game = {r[0]: r[1] for r in events.event_rows(img, dis, events.ARRAY, events.SLOTS)[0]}
    glue = {r[0]: r[1] for r in events.event_rows(img, dis, events.GLUE_ARRAY,
                                                  events.GLUE_SLOTS)[0]}
    found = []
    for i, (a, mn, ops) in enumerate(cfg.ins):
        if mn in ('call', 'jmp') and IMM.match(ops) and int(ops, 16) in (SE, SE2):
            found.append((i, a, 'SignalEvent' if int(ops, 16) == SE else 'SignalEvent2'))
    res = {a: (kind, i) + resolve(cfg, i, kind) for i, a, kind in found}
    for a, want in CONTROLS.items():
        kind, i, vals, bad = res[a]
        if (None if bad else vals) != want:
            sys.exit('control failed at 0x%x: want %s, got %s %s' % (a, want, vals, bad))

    rows, unresolved = [], 0
    for a in sorted(res):
        kind, i, vals, bad = res[a]
        space = 'glue' if GLUE_TU[0] <= a < GLUE_TU[1] else 'game'
        table = glue if space == 'glue' else game
        if bad:
            ids, names = '?', ''
            unresolved += 1
        else:
            ids = '|'.join(str(v) for v in sorted(vals))
            nm = [table.get(v, '') for v in sorted(vals)]
            names = '|'.join(nm) if any(nm) else ''
        fmt = format_of(cfg, img, i) if kind == 'SignalEvent2' else None
        if kind == 'SignalEvent2' and fmt is None:
            note('no format string at 0x%x' % a)
        rows.append((addr(a), kind, ids, names, fmt or '', space))
    note('%d sites (%d SignalEvent2), %d unresolved ids, 5 controls passed'
         % (len(rows), sum(1 for r in rows if r[1] == 'SignalEvent2'), unresolved))
    write_table(['site', 'dispatcher', 'event_ids', 'event_names', 'arg_format', 'space'],
                rows, args.out)


if __name__ == '__main__':
    sys.exit(main())
