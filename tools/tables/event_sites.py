#!/usr/bin/env python3
"""tables/event_sites.tsv: every place the client fires a C-to-Lua event, with the ids it can
fire and the argument format it pushes.

A site is a call or a tail jmp to one of the two dispatchers:

    FrameScript_SignalEvent  0x703e50   fastcall, ecx = event id, no arguments
    SignalEvent2             0x703f50   cdecl, [esp] = event id, [esp+4] = printf-style format,
                                        then one vararg per directive (%f takes eight bytes)

MSVC compiles `fire(cond ? A : B)` as `mov ecx, A; jcc L; mov ecx, B; L: call`, so the nearest
`mov` before the call names one arm only. The generator walks the control-flow graph backward
from each site over every predecessor (the fall-through instruction, every direct jmp or jcc
that targets it, and every jump-table entry that names it) and collects every definition that
reaches the site. Per path, a walk ends at the first definition of what it tracks:

    mov R, imm32          the value is imm32
    mov R, R2             track R2 instead
    push R                (SignalEvent2) track R
    add R, K              track R and add K
    xor R, R              the value is 0
    setcc R8              the value is 0 or 1, given a clearing xor above it
    sbb R, R              the value is 0 or -1

The walk the sites use, `Tracer.walk`, also takes these steps, which `resolve` (the walk
tables/tutorial_triggers.py imports) does not:

    shr R, K              track R and shift it right by K
    cmp R, K; jne / je    on the edge where R equals K, the value is K

and ends at five computed shapes, each named in `notes`:

    table-<T>             mov R, [X*4 + T]: the dwords of T at the indexes 0 to N - 1, where the
                          index is the value a `cmp Y, N; jae` past the site bounds, stored once
                          into the stack slot X is loaded from
    table-signed-<T>      the same with a signed `jge`, which does not reject a negative index:
                          the dwords below T that a negative index reads are not listed, and
                          whether one arrives is the site's own finding
    through-<F>           R is ecx or edx at the entry of F: the values every call and tail jump
                          to F passes, each walked the same way
    watch-arg-<F>         R is loaded from [ebp+0x14] of F, the argument a watcher callback is
                          given: the argument pushed just before F at every registration of F
                          with ClntObjMgrSetObjectMirrorHandler 0x467e70
    watch-offset-<F>      R is ecx at the entry of F, a watcher callback's field offset: the
                          offset in edx at every registration of F with 0x467e70
    byte-plus-<K>         movzx R, BYTE PTR [...] plus K: K plus a byte the code does not bound;
                          its values are not listed in event_ids

A registration's offset may be the index of a loop, `lea edx, [X*4]` at the head of a loop that
runs X from 0 below N and skips the registration where the event-name array 0xbe1198 holds null
at X: the offsets are then 4·X for the named X below N. A removal of F (0x467fb0) passes the same
callback and argument and fires nothing. A callback address stored any other way, or a walk that
meets any other definition, a call that clobbers the register or a stack parameter of a function
with direct callers, is unresolved, and the generator stops and lists every unresolved site with
the instruction where its walk ended. A walk ends at every function row's first instruction, not
only at call targets, so it never falls through padding into the function above.

event_ids lists every id that reaches the site, `|`-separated in ascending order. The format is
the string pushed just before the id push; it is a declaration, not a count of what the caller
pushed. Nine control sites are checked on every run.

The glue screens run a different event table (tables/glue_events.tsv), so the same integer names a
different event there. The glue code is glue's run of object files, [0x46a260, 0x475b70): its first
function is 0x46a260 and the next file starts at 0x475b70. Sites there are space `glue` and take
their names from the glue table, all others are `game`.
"""

import re
import sys
from collections import defaultdict

from lib import Disasm, Functions, Image, addr, note, parse_args, write_table
import events

SE, SE2 = 0x703E50, 0x703F50
GLUE_TU = (0x46A260, 0x475B70)
MIRROR_ADD, MIRROR_REMOVE = 0x467E70, 0x467FB0
# the pushes between a callback's push and the call: length, GUID high, GUID low for the
# registration; GUID high, GUID low for the removal
REGISTRARS = {MIRROR_ADD: ('add', 3), MIRROR_REMOVE: ('remove', 2)}
WATCH_ARG_SLOT = 3  # [ebp+0x14]: FireWatchers 0x465570 pushes the record's +0x14 first
# site -> (ids, exact): the ids the site must give, all of them (exact) or among others
CONTROLS = {0x46AA34: ({22}, True), 0x4930C7: ({208, 209}, True), 0x49211A: ({397, 398}, True),
            0x4945CA: ({224, 225}, True), 0x46C0B4: ({4}, True),
            0x5DE4C1: ({149, 189, 201, 372, 406, 523, 524}, True),
            0x5DE4E8: ({406}, True),
            0x49B199: ({228, 238, 540, 547}, False),
            0x515E7A: ({0, 16, 41, 183, 186, 522}, False)}

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


TABLE_LOAD = re.compile(r'^DWORD PTR \[(e[a-ds][xip])\*4\+0x([0-9a-f]+)\]$')
STACK_ARG = re.compile(r'^DWORD PTR \[ebp\+0x([0-9a-f]+)\]$')
SLOT_LOAD = re.compile(r'^DWORD PTR (\[ebp-0x[0-9a-f]+\])$')
LOOP_LEA = re.compile(r'^\[(e[a-ds][xip])\*4\+0x0\]$')
CMP_IMM = re.compile(r'^(e[a-ds][xip]),0x([0-9a-f]+)$')
FLAG_SAFE = ('mov', 'lea', 'push')


class Tracer:
    """The walk of `resolve`, taught the computed shapes the module docstring lists."""

    def __init__(self, cfg, img, funcs, names):
        self.cfg, self.img, self.funcs, self.names = cfg, img, funcs, names
        self.at = {a: i for i, (a, _, _) in enumerate(cfg.ins)}
        self.starts = set(cfg.entries) | set(funcs.at)
        self.calls = defaultdict(list)
        self.cases = defaultdict(list)
        for i, (a, mn, ops) in enumerate(cfg.ins):
            if mn in ('call', 'jmp') and IMM.match(ops):
                self.calls[int(ops, 16)].append(i)
            m = TABLE_LOAD.match(ops) if mn == 'jmp' else None
            if m:
                for t in self.table_targets(a, int(m.group(2), 16)):
                    self.cases[t].append(i)
        self.memo, self.busy = {}, set()

    def table_targets(self, a, base):
        """The entries of the jump table at `base` that the jmp at `a` reads: dwords from `base`
        while each is an instruction of the function row that holds `a`."""
        row = self.funcs.holding(a)
        out = []
        if row is None or not self.img.in_image(base):
            return out
        lo, hi = row['address'], row['address'] + row['size']
        k = base
        while self.img.in_image(k + 3):
            t = self.img.u32(k)
            if not (lo <= t < hi and t in self.at):
                break
            out.append(t)
            k += 4
        return out

    def preds(self, i):
        return self.cfg.predecessors(i) + self.cases.get(self.cfg.ins[i][0], [])

    def stored(self, fn):
        """Every address in the image whose dword is `fn`."""
        pat = fn.to_bytes(4, 'little')
        mem, out = self.img.mem, []
        k = mem.find(pat)
        while k >= 0:
            out.append(self.img.base + k)
            k = mem.find(pat, k + 1)
        return out

    def registration(self, o, fn):
        """(kind, index of the push of fn, index of the registrar call) for the dword at o, or a
        reason it is not a push of fn into a mirror registration or removal."""
        k = self.at.get(o - 1)
        if k is None or self.cfg.ins[k][1:] != ('push', '0x%x' % fn):
            return None, None, 'the address is stored at %s, not pushed' % addr(o)
        pushes = 0
        for j in range(k + 1, min(k + 12, len(self.cfg.ins))):
            _, mn, ops = self.cfg.ins[j]
            if mn == 'push':
                pushes += 1
            elif mn == 'call':
                t = int(ops, 16) if IMM.match(ops) else None
                if t in REGISTRARS and REGISTRARS[t][1] == pushes:
                    return REGISTRARS[t][0], k, j
                break
            elif mn.startswith('j') or mn.startswith('ret'):
                break
        return None, None, 'the push at %s reaches no mirror registration' % addr(o - 1)

    def guard(self, p, i, trk):
        """The value `trk` holds on the edge p -> i when p is a jne or je after `cmp trk,K` and
        the edge is the one where the two are equal; else None."""
        if p < 1:
            return None
        _, mn, ops = self.cfg.ins[p]
        _, cmn, cops = self.cfg.ins[p - 1]
        m = CMP_IMM.match(cops) if cmn == 'cmp' else None
        if not m or m.group(1) != trk or not IMM.match(ops):
            return None
        here, target = self.cfg.ins[i][0], int(ops, 16)
        if mn == 'jne' and p == i - 1 and target != here:
            return int(m.group(2), 16)
        if mn == 'je' and p != i - 1 and target == here:
            return int(m.group(2), 16)
        return None

    @staticmethod
    def value(v, xf):
        shr, add = xf
        return (((v & 0xFFFFFFFF) >> shr) + add) if shr else v + add

    def walk(self, start, trk, xf=(0, 0)):
        """-> (ids, notes, unresolved) for what `trk` holds just before instruction `start`;
        `trk` 'push' is the value of the first push before it."""
        ins = self.cfg.ins
        vals, notes, bad = set(), set(), []
        work, seen = [], set()

        def back(i, trk, xf, d, pend):
            a = ins[i][0]
            if a in self.starts and i != start:
                if pend is None and trk in ('ecx', 'edx'):
                    self.param(a, trk, xf, vals, notes, bad)
                else:
                    bad.append((a, 'reached the entry of %s tracking %s' % (addr(a), trk)))
                return
            ps = self.preds(i)
            if not ps:
                bad.append((a, 'no predecessor'))
            for p in ps:
                g = self.guard(p, i, trk) if pend is None and trk in REG else None
                if g is not None:
                    vals.add(self.value(g, xf))
                else:
                    work.append((p, trk, xf, d + 1, pend))

        back(start, trk, xf, 0, None)
        while work:
            i, trk, xf, d, pend = work.pop()
            if (i, trk, xf, pend) in seen:
                continue
            seen.add((i, trk, xf, pend))
            a, mn, ops = ins[i]
            if d > 400:
                bad.append((a, 'depth limit'))
                continue
            if trk == 'push':
                if mn == 'push':
                    if IMM.match(ops):
                        vals.add(self.value(int(ops, 16), xf))
                    elif ops in REG:
                        back(i, ops, xf, d, None)
                    else:
                        bad.append((a, 'push ' + ops))
                    continue
                if mn == 'call' or (mn in ('add', 'sub') and dest(ops) == 'esp'):
                    bad.append((a, '%s %s before the id push' % (mn, ops)))
                    continue
                back(i, trk, xf, d, pend)
                continue
            names = REG[trk]
            if mn == 'call':
                if trk in ('eax', 'ecx', 'edx'):
                    bad.append((a, 'clobbered by call ' + ops))
                    continue
                back(i, trk, xf, d, pend)
                continue
            dd = dest(ops)
            writes = dd in names and not (mn.startswith('j') or mn in ('cmp', 'test', 'push'))
            if mn == 'xchg' and any(n in (dd, src(ops)) for n in names):
                writes = True
            if mn == 'cdq' and trk == 'edx':
                writes = True
            if not writes:
                back(i, trk, xf, d, pend)
                continue
            s = src(ops)
            if pend is not None:
                if mn == 'xor' and dd == trk and s == trk:
                    vals.update({self.value(0, xf), self.value(1, xf)})
                else:
                    bad.append((a, 'setcc into %s without a clearing xor (%s %s)' % (trk, mn, ops)))
                continue
            if mn == 'mov' and dd == trk and IMM.match(s):
                vals.add(self.value(int(s, 16), xf) & 0xFFFFFFFF)
            elif mn == 'mov' and dd == trk and s in REG:
                back(i, s, xf, d, None)
            elif mn == 'add' and dd == trk and IMM.match(s) and xf[0] == 0:
                back(i, trk, (0, xf[1] + int(s, 16)), d, None)
            elif mn == 'shr' and dd == trk and IMM.match(s) and xf == (0, 0):
                back(i, trk, (int(s, 16), 0), d, None)
            elif mn.startswith('set') and dd == LOW8.get(trk):
                back(i, trk, xf, d, ('setcc', trk))
            elif mn == 'xor' and dd == trk and s == trk:
                vals.add(self.value(0, xf))
            elif mn == 'sbb' and dd == trk and s == trk:
                vals.update({self.value(0, xf), self.value(-1, xf)})
            elif mn == 'mov' and dd == trk and TABLE_LOAD.match(s):
                m = TABLE_LOAD.match(s)
                self.table(i, m.group(1), int(m.group(2), 16), xf, vals, notes, bad)
            elif mn == 'mov' and dd == trk and STACK_ARG.match(s):
                k = int(STACK_ARG.match(s).group(1), 16)
                self.stack_param(i, (k - 8) // 4, xf, vals, notes, bad)
            elif mn == 'movzx' and dd == trk and s.startswith('BYTE PTR ') and xf[0] == 0:
                notes.add('byte-plus-%d' % xf[1])
            elif mn == 'lea' and dd == trk and LOOP_LEA.match(s):
                self.loop(i, LOOP_LEA.match(s).group(1), xf, vals, bad)
            else:
                bad.append((a, '%s %s' % (mn, ops)))
        return vals, notes, bad

    def merge(self, key, vals, notes, bad):
        v, n, b = self.memo[key]
        vals.update(v)
        notes.update(n)
        bad.extend(b)

    def param(self, fn, reg, xf, vals, notes, bad):
        """`reg` at the entry of fn: what every caller passes, or a watcher's field offset."""
        key = ('reg', fn, reg, xf)
        if key in self.busy:
            return
        if key not in self.memo:
            self.busy.add(key)
            v, n, b = set(), set(), []
            for o in self.stored(fn):
                kind, _, j = self.registration(o, fn)
                if kind is None:
                    b.append((o, j))
                elif kind == 'add':
                    if reg != 'ecx':
                        b.append((o, '%s of watcher callback %s' % (reg, addr(fn))))
                        continue
                    n.add('watch-offset-%s' % addr(fn))
                    self.extend(self.walk(j, 'edx', xf), v, n, b)
            for c in self.calls.get(fn, []):
                n.add('through-%s' % addr(fn))
                self.extend(self.walk(c, reg, xf), v, n, b)
            if 'through-%s' % addr(fn) not in n and 'watch-offset-%s' % addr(fn) not in n:
                b.append((fn, '%s at the entry of %s, which nothing calls or registers'
                          % (reg, addr(fn))))
            self.memo[key] = (v, n, b)
            self.busy.discard(key)
        self.merge(key, vals, notes, bad)

    def stack_param(self, i, n, xf, vals, notes, bad):
        """Stack parameter n of the function holding instruction i."""
        a = self.cfg.ins[i][0]
        row = self.funcs.holding(a)
        fn = row['address'] if row else None
        if fn is None or fn not in self.at or self.cfg.ins[self.at[fn]][1:] != ('push', 'ebp') \
                or self.cfg.ins[self.at[fn] + 1][1:] != ('mov', 'ebp,esp'):
            bad.append((a, 'a stack slot outside a framed function row'))
            return
        key = ('stack', fn, n, xf)
        if key not in self.memo:
            v, ns, b = set(), set(), []
            if self.calls.get(fn):
                b.append((a, 'stack parameter %d of %s, which has direct callers' % (n, addr(fn))))
            for o in self.stored(fn):
                kind, k, j = self.registration(o, fn)
                if kind is None:
                    b.append((o, j))
                elif kind == 'add':
                    if n != WATCH_ARG_SLOT:
                        b.append((o, 'stack parameter %d of watcher callback %s' % (n, addr(fn))))
                        continue
                    ns.add('watch-arg-%s' % addr(fn))
                    self.extend(self.walk(k, 'push', xf), v, ns, b)
            if not ns and not b:
                b.append((a, 'stack parameter %d of %s, which nothing registers' % (n, addr(fn))))
            self.memo[key] = (v, ns, b)
        self.merge(key, vals, notes, bad)

    @staticmethod
    def extend(res, v, n, b):
        v.update(res[0])
        n.update(res[1])
        b.extend(res[2])

    def table(self, i, x, base, xf, vals, notes, bad):
        """mov R, [x*4 + base]: the bound of the index, read where it is stored and tested."""
        ins = self.cfg.ins
        a = ins[i][0]
        row = self.funcs.holding(a)
        prev = ins[i - 1]
        m = SLOT_LOAD.match(src(prev[2])) if prev[1] == 'mov' and dest(prev[2]) == x else None
        if row is None or not m:
            bad.append((a, 'table index %s not loaded from a stack slot just before' % x))
            return
        slot = m.group(1)
        lo, hi = self.at[row['address']], self.at[row['address']] + 1
        while hi < len(ins) and ins[hi][0] < row['address'] + row['size']:
            hi += 1
        stores = [j for j in range(lo, hi) if ins[j][1] == 'mov' and dest(ins[j][2]) == slot]
        if len(stores) != 1 or src(ins[stores[0]][2]) not in REG:
            bad.append((a, 'table index slot %s stored %d times' % (slot, len(stores))))
            return
        y, st = src(ins[stores[0]][2]), stores[0]
        bound, signed = None, False
        for j in range(st - 1, max(st - 6, lo - 1), -1):
            _, mn, ops = ins[j]
            c = CMP_IMM.match(ops) if mn == 'cmp' else None
            if c and c.group(1) == y:
                for t in range(j + 1, st + 6):
                    tmn, tops = ins[t][1], ins[t][2]
                    if tmn in ('jge', 'jae') and IMM.match(tops) and int(tops, 16) > a:
                        bound, signed = int(c.group(2), 16), tmn == 'jge'
                    if tmn not in FLAG_SAFE:
                        break
                break
            if dest(ops) in REG[y] and mn not in ('cmp', 'test', 'push'):
                break
        if bound is None:
            bad.append((a, 'table index %s has no bound before its store' % y))
            return
        notes.add('table-%s%s' % ('signed-' if signed else '', addr(base)))
        for k in range(bound):
            vals.add(self.value(self.img.u32(base + 4 * k), xf))

    def loop(self, i, x, xf, vals, bad):
        """lea R, [x*4] at the head of a loop over x, gated on the event-name array."""
        ins = self.cfg.ins
        a = ins[i][0]
        row = self.funcs.holding(a)
        end = row['address'] + row['size'] if row else a
        n, gate, q = None, None, i + 1
        while q + 2 < len(ins) and ins[q][0] < end:
            if ins[q][1:] == ('inc', x) and ins[q + 1][1] == 'cmp' and ins[q + 2][1] == 'jl' \
                    and ins[q + 2][2] == '0x%x' % a:
                c = CMP_IMM.match(ins[q + 1][2])
                n = int(c.group(2), 16) if c and c.group(1) == x else None
                break
            q += 1
        reg = dest(ins[i][2])
        for j in range(i + 1, q):
            _, mn, ops = ins[j]
            if mn == 'mov' and src(ops) == 'DWORD PTR [%s+0x%x]' % (reg, events.ARRAY) \
                    and ins[j + 1][1:] == ('test', '%s,%s' % (dest(ops), dest(ops))) \
                    and ins[j + 2][1] == 'je' and q < len(ins) and ins[j + 2][2] == '0x%x' % ins[q][0]:
                gate = j
        if n is None or gate is None or ins[i - 1][1:] != ('xor', '%s,%s' % (x, x)):
            bad.append((a, 'lea %s is no gated loop over the event names' % ins[i][2]))
            return
        for k in range(n):
            if k in self.names:
                vals.add(self.value(4 * k, xf))


def main():
    args = parse_args(__doc__, disasm=True)
    img, dis = Image(args.exe), Disasm(args.disasm)
    cfg = Cfg(dis)
    game = {r[0]: r[1] for r in events.event_rows(img, dis, events.ARRAY, events.SLOTS)[0]}
    glue = {r[0]: r[1] for r in events.event_rows(img, dis, events.GLUE_ARRAY,
                                                  events.GLUE_SLOTS)[0]}
    tracer = Tracer(cfg, img, Functions(), game)
    found = []
    for i, (a, mn, ops) in enumerate(cfg.ins):
        if mn in ('call', 'jmp') and IMM.match(ops) and int(ops, 16) in (SE, SE2):
            found.append((i, a, 'SignalEvent' if int(ops, 16) == SE else 'SignalEvent2'))
    res = {a: (kind, i) + tracer.walk(i, 'ecx' if kind == 'SignalEvent' else 'push')
           for i, a, kind in found}
    unresolved = ['%s: %s' % (addr(a), '; '.join(sorted({'%s %s' % (addr(b), why)
                                                          for b, why in r[4]})))
                  for a, r in sorted(res.items()) if r[4] or not (r[2] or r[3])]
    if unresolved:
        sys.exit('event_sites: %d sites with an id the walk cannot resolve:\n  %s'
                 % (len(unresolved), '\n  '.join(unresolved)))
    for a, (want, exact) in CONTROLS.items():
        vals = res[a][2]
        if (vals != want) if exact else not want <= vals:
            sys.exit('control failed at 0x%x: want %s%s, got %s'
                     % (a, '' if exact else 'at least ', sorted(want), sorted(vals)))

    rows = []
    for a in sorted(res):
        kind, i, vals, notes, _ = res[a]
        space = 'glue' if GLUE_TU[0] <= a < GLUE_TU[1] else 'game'
        table = glue if space == 'glue' else game
        ids = '|'.join(str(v) for v in sorted(vals))
        nm = [table.get(v, '') for v in sorted(vals)]
        names = '|'.join(nm) if any(nm) else ''
        fmt = format_of(cfg, img, i) if kind == 'SignalEvent2' else None
        if kind == 'SignalEvent2' and fmt is None:
            note('no format string at 0x%x' % a)
        rows.append((addr(a), kind, ids, names, fmt or '', space, ';'.join(sorted(notes))))
    note('%d sites (%d SignalEvent2), %d with a computed id, %d controls passed'
         % (len(rows), sum(1 for r in rows if r[1] == 'SignalEvent2'),
            sum(1 for r in rows if r[6]), len(CONTROLS)))
    write_table(['site', 'dispatcher', 'event_ids', 'event_names', 'arg_format', 'space', 'notes'],
                rows, args.out)


if __name__ == '__main__':
    sys.exit(main())
