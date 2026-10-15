#!/usr/bin/env python3
"""Calling convention of every function row that has a signature, decided from the bytes on both
sides of the call.

    tools/conventions.py                  the counts of rows per convention and what changes
    tools/conventions.py --report FILE    one line per row (tab-separated) with the evidence
    tools/conventions.py --check          every row whose signature disagrees with its verdict
    tools/conventions.py --apply          rewrite the signatures in symbols/functions.tsv
    tools/conventions.py --older FILE     with --apply: an older functions.tsv whose names fill
                                          the slots a change of convention moved
    tools/conventions.py --fill           with --apply: give a registered row with no signature
                                          its registration's signature
    tools/conventions.py --test           run the built-in checks

The body, from dumps/disasm.txt, walked over every path from the entry (a jump table's cases
inside the row's extent, a `jmp` to another row as a tail call):
  - whether ecx and edx are read before they are written. A thunk's `jmp`, and a `call` that finds
    the register unwritten, take what the target reads. A `call` writes only what its target
    writes (a callee that never touches edx leaves it), and a callee-saved register keeps its
    value. A partial write (`dl`, `cx`) is a write; `xor r,r`, `or r,-1` and `and m,0` read
    nothing. A register pushed as an argument is a read. A `push ecx` or `push edx` that reserves
    a local (in the prologue, or before an x87 store to [esp]) is a read only when the slot is
    read before it is written or a call takes it from the top of the stack;
  - the object test: the value ecx came in with, followed through register copies, offsets and
    frame slots, is the base of a memory operand (a string operation's esi or edi too), or of an
    `lea` or an `add` of a field offset (4 or more) or an `inc` whose result is pushed, or is returned in eax, in the body or in a callee it is passed
    to;
  - the `ret N` of every return, a tail target's included, and the stack words read.

Every direct `call` in the listing is a call site of its target (a call to a thunk is a call of
the thunk). In the block before it, back to a label, a `ret` or `jmp`, the entry or 16
instructions:
  - ecx and edx are `loaded` (written and not read again before the call: a temporary is not a
    load), `not` loaded (a `call` that writes the register comes first), or `unknown` (the block
    starts at a label, after a `ret` or `jmp`, at the caller's entry where the register is the
    caller's own, or at the limit); ecx is also `live` when the code after the call reads it
    before writing it;
  - ecx is loaded as a constant (zero or a small integer), the address of a global, an `lea`, or
    otherwise;
  - the words pushed in the block and the `add esp,N` right after the call.

A function reached through a pointer has the type of the pointer's registration, whatever its body
reads:
  - a row in a table of (name, function) pairs, or the function a direct call of
    FrameScriptRegisterFunction passes in edx, is a Lua C function: `int __fastcall (lua_State
    *L)`, the state in ecx (luaD_precall: `mov ecx,esi; call edi`);
  - the function a direct call of ConsoleCommandRegister passes in edx is a console handler,
    `__fastcall (const char *name, const char *args)` with its own return type, the name in ecx
    and the arguments in edx (ConsoleCommandExecute: `mov edx,[ebp-4]; mov ecx,[ebp-8]; call
    [ebx+0x18]`);
  - the function ClientServices::SetMessageHandler or NetClient::SetMessageHandler is given is a
    message handler, `__fastcall (void *param, uint32_t opcode, uint32_t time, CDataStore
    *msg)` with its own return type: the dispatcher at 0x537ac3 loads the registered parameter
    into ecx and the opcode into edx, pushes the time and the message, and returns `ret 8`.

The rules, per row, for the rest:
  - edx read by the body: __fastcall, ecx a parameter when read, else `unused`. A caller that
    leaves edx undefined, or no caller that loads it, is a disagreement.
  - edx not read and loaded by every caller: __fastcall, edx `unused`.
  - ecx read and edx neither read nor loaded by a caller: __thiscall when ecx is an object (the
    object test, a caller that loads it with `lea` of a local or field or with the address of a
    global, a name `Class::Method`, or, as a lead no byte contradicts, a signature that already
    types it as a pointer to a class) and the callers do not load a constant; a signature that
    types it as `char *` makes it a string, not an object, unless the name is `Class::Method`;
    otherwise __fastcall with one register parameter. Where the next stack parameter could take edx, the signature gives edx an
    `unused` parameter.
  - ecx read and edx loaded by some callers: a disagreement among callers.
  - neither read: __thiscall when every caller loads ecx (the constants alone do not count) and
    none reads ecx again after the call, which means the value is the caller's own; a method of
    the callee's class that writes nothing to ecx before the call passes its own `this` and
    counts as a load; or the
    function is a class method in a vtable. With no direct caller, the convention of the older
    signature (the `--older` table, else the current one) stays when it is __fastcall or
    __thiscall: a pointer the tool cannot see may pass the registers. Otherwise __stdcall when a
    return pops bytes, else __cdecl. With no return, or no stack parameters and `ret`, the
    signature's own __stdcall or __cdecl stays. A function the callers call with neither register
    loaded loses its register parameters; its stack parameters stay.
  - returns that pop different bytes, and callers that clean the bytes the body pops, are
    disagreements.
A disagreement is listed and left alone; tools/conventions-resolved.tsv holds the rows a reader
settled at the bytes, each with the reason, and its verdict replaces the tool's.

How far the answer goes: a function with no direct call is decided by its body alone (vtable and
callback calls are not seen). Of a call site only its own block is read; a register set earlier,
or left by a callee the tool cannot follow, is `unknown`. A register loaded for another use and
untouched until the call reads as loaded. An integer pointer the body uses only through a store
to a global, or a string in ecx, looks like an integer, or an object, by the test above. The
stack words are the `ret N` of the body; for __cdecl the stack part of the existing list stays.

Names: the stack parameters keep the names of their slots (the first list of names whose words
are the `ret N` bytes; the older table's first where a change of convention may have slid them).
`this` is only __thiscall's first parameter. A register no one reads is `unused` (`unused1` and
`unused2` for two). Any other parameter the tool adds is `argN`, N the position among all the
parameters, `this` counting as 1.

Standard library only. --exe and --disasm are as for the table generators.
"""

import re
import sys
from collections import namedtuple
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / 'tables'))
from lib import ROOT, Functions, Image, addr  # noqa: E402

GP32 = ('eax', 'ecx', 'edx', 'ebx', 'esp', 'ebp', 'esi', 'edi')
REG_CANON = {}
for _i, _n in enumerate(GP32):
    REG_CANON[_n] = _n
    REG_CANON[_n[1:]] = _n
for _n, _w, _lo, _hi in (('eax', 'ax', 'al', 'ah'), ('ecx', 'cx', 'cl', 'ch'),
                         ('edx', 'dx', 'dl', 'dh'), ('ebx', 'bx', 'bl', 'bh')):
    REG_CANON[_lo] = _n
    REG_CANON[_hi] = _n
REG_RE = re.compile(r'\b(' + '|'.join(sorted(REG_CANON, key=len, reverse=True)) + r')\b')
SIZE_RE = re.compile(r'^(BYTE|WORD|DWORD|QWORD|TBYTE|XMMWORD|FWORD|OWORD) PTR ')
MEM_RE = re.compile(r'\[([^\]]*)\]')
HEX = re.compile(r'^0x[0-9a-f]+$')
SIZES = {'BYTE': 1, 'WORD': 2, 'DWORD': 4, 'QWORD': 8, 'TBYTE': 10, 'FWORD': 6, 'XMMWORD': 16,
         'OWORD': 16}

# Mnemonics whose first operand is only written.
WONLY = {'mov', 'movzx', 'movsx', 'lea', 'pop', 'movss', 'movaps', 'movups', 'movlps', 'movhps',
         'movd', 'movq', 'movdqa', 'movdqu', 'movntq', 'movnti', 'fst', 'fstp', 'fist', 'fistp',
         'fisttp', 'fnstcw', 'fstcw', 'fnstsw', 'fstsw', 'fnstenv', 'stmxcsr', 'movmskps',
         'pmovmskb', 'cvttss2si', 'cvtss2si', 'cvttsd2si', 'cvtsd2si', 'bsf', 'bsr', 'sldt',
         'str', 'smsw', 'fnsave', 'fxsave', 'movsx', 'movsxd', 'popcnt', 'lzcnt', 'tzcnt',
         'movbe', 'rdtsc', 'cmpxchg8b', 'movlpd', 'movhpd', 'movapd', 'movupd'}
# Read and written.
RMW = {'add', 'adc', 'sub', 'sbb', 'and', 'or', 'xor', 'inc', 'dec', 'neg', 'not', 'shl', 'shr',
       'sar', 'sal', 'rol', 'ror', 'rcl', 'rcr', 'shld', 'shrd', 'bts', 'btr', 'btc', 'xadd',
       'bswap', 'cmpxchg'}
CC_JUMP = re.compile(r'^j[a-z]+$')
STRING_OP = re.compile(r'^(movs|stos|lods|scas|cmps)[bwd]?$')

# A name that gives a class: `Class::Method`. `Script::` is the Lua API's namespace, not a class.
CLASS_RE = re.compile(r'^(?!Script::)([A-Za-z_][A-Za-z_0-9]*)::')


class Listing:
    """dumps/disasm.txt as parallel lists: address, mnemonic, operand strings, rep flag."""

    def __init__(self, path):
        self.addrs = []
        self.mn = []
        self.ops = []
        self.rep = []
        for line in Path(path).read_text(errors='replace').splitlines():
            head, tab, text = line.partition('\t')
            head = head.strip()
            if not tab or not head.endswith(':'):
                continue
            try:
                a = int(head[:-1], 16)
            except ValueError:
                continue
            text = text.strip()
            if not text:
                continue
            self.add(a, text)
        self.at = {a: i for i, a in enumerate(self.addrs)}

    def add(self, a, text):
        rep = False
        words = text.split(None, 1)
        mn, rest = words[0], (words[1].strip() if len(words) > 1 else '')
        while mn in ('rep', 'repz', 'repnz', 'repe', 'repne', 'lock', 'data16', 'addr16',
                     'notrack', 'bnd') and rest:
            rep = rep or mn.startswith('rep')
            words = rest.split(None, 1)
            mn, rest = words[0], (words[1].strip() if len(words) > 1 else '')
        self.addrs.append(a)
        self.mn.append(mn)
        self.ops.append(split_ops(rest))
        self.rep.append(rep)

    def following(self, i):
        return i + 1 if i + 1 < len(self.addrs) else None


def split_ops(text):
    out, depth, cur = [], 0, ''
    for ch in text:
        if ch in '[(':
            depth += 1
        elif ch in '])':
            depth -= 1
        if ch == ',' and depth == 0:
            out.append(cur.strip())
            cur = ''
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def strip_size(op):
    m = SIZE_RE.match(op)
    size = SIZES[m.group(1)] if m else None
    op = op[m.end():] if m else op
    op = re.sub(r'^(cs|ds|es|fs|gs|ss):', '', op)
    return op, size


def is_mem(op):
    op, _ = strip_size(op)
    return '[' in op


def reg_of(op):
    """The canonical 32-bit register an operand names exactly, else None."""
    op, _ = strip_size(op)
    return REG_CANON.get(op)


def mem_terms(op):
    """(base, index, disp, size) of a memory operand; None for a non-memory operand."""
    op, size = strip_size(op)
    m = MEM_RE.search(op)
    if not m:
        return None
    body = m.group(1)
    base = index = None
    disp = 0
    for sign, term in re.findall(r'([+-]?)([^+-]+)', body):
        term = term.strip()
        neg = sign == '-'
        if '*' in term:
            r, _, _scale = term.partition('*')
            if r in REG_CANON:
                index = REG_CANON[r]
        elif term in REG_CANON:
            if base is None:
                base = REG_CANON[term]
            else:
                index = REG_CANON[term]
        elif HEX.match(term):
            v = int(term, 16)
            disp += -v if neg else v
    return base, index, disp, size


class Effect:
    """What one instruction does to ecx and edx and to memory."""

    __slots__ = ('reads', 'writes', 'mem_reads', 'mem_writes')

    def __init__(self):
        self.reads = set()       # canonical registers read (including in addresses)
        self.writes = set()      # canonical registers written
        self.mem_reads = []      # memory operands read
        self.mem_writes = []     # memory operands written (and not read)


def effect(mn, ops, rep):
    e = Effect()

    def regs_in(op):
        op, _ = strip_size(op)
        return {REG_CANON[r] for r in REG_RE.findall(op)}

    def addr_regs(op):
        t = mem_terms(op)
        if t is None:
            return
        base, index, _, _ = t
        for r in (base, index):
            if r:
                e.reads.add(r)

    # implicit operands first
    if STRING_OP.match(mn) and not (
            mn in ('movsd',) and ops and ops[0].startswith('xmm')):
        if rep:
            e.reads.add('ecx')
            e.writes.add('ecx')
        if mn.startswith(('movs', 'cmps')) and not (ops and ops[0].startswith('xmm')):
            e.reads.update(('esi', 'edi'))
        if mn.startswith(('stos', 'scas')):
            e.reads.update(('edi', 'eax'))
        return e
    if mn in ('cdq', 'cwd'):
        e.reads.add('eax')
        e.writes.add('edx')
        return e
    if mn in ('cwde', 'cbw'):
        e.reads.add('eax')
        e.writes.add('eax')
        return e
    if mn in ('mul', 'imul') and len(ops) == 1:
        e.reads.add('eax')
        e.writes.update(('eax', 'edx'))
        if is_mem(ops[0]):
            addr_regs(ops[0])
            e.mem_reads.append(ops[0])
        else:
            e.reads |= regs_in(ops[0])
        return e
    if mn in ('div', 'idiv'):
        e.reads.update(('eax', 'edx'))
        e.writes.update(('eax', 'edx'))
        if is_mem(ops[0]):
            addr_regs(ops[0])
            e.mem_reads.append(ops[0])
        else:
            e.reads |= regs_in(ops[0])
        return e
    if mn == 'cpuid':
        e.reads.update(('eax', 'ecx'))
        e.writes.update(('eax', 'ebx', 'ecx', 'edx'))
        return e
    if mn == 'rdtsc':
        e.writes.update(('eax', 'edx'))
        return e
    if mn in ('loop', 'loope', 'loopne', 'jecxz', 'jcxz'):
        e.reads.add('ecx')
        if mn.startswith('loop'):
            e.writes.add('ecx')
        return e
    if mn == 'pushad':
        e.reads.update(GP32)
        return e
    if mn == 'popad':
        e.writes.update(GP32)
        return e
    if mn == 'xchg' and len(ops) == 2:
        for op in ops:
            if is_mem(op):
                addr_regs(op)
                e.mem_reads.append(op)
            else:
                e.reads |= regs_in(op)
                e.writes |= regs_in(op)
        return e
    if mn == 'cmpxchg8b':
        e.reads.update(('eax', 'ebx', 'ecx', 'edx'))
        e.writes.update(('eax', 'edx'))
        if ops:
            addr_regs(ops[0])
            e.mem_reads.append(ops[0])
        return e

    if mn in ('xor', 'sub', 'sbb') and len(ops) == 2 and ops[0] == ops[1] and ops[0] in REG_CANON:
        e.writes.add(REG_CANON[ops[0]])
        return e
    if mn == 'or' and len(ops) == 2 and ops[0] in REG_CANON and ops[1] == '0xffffffff':
        e.writes.add(REG_CANON[ops[0]])     # sets every bit: no read
        return e
    wonly = mn in WONLY or mn.startswith('set') or mn.startswith('cvt') and mn.endswith('2si')
    rmw = mn in RMW
    cmov = mn.startswith('cmov')
    for k, op in enumerate(ops):
        if is_mem(op):
            addr_regs(op)
            dest = k == 0 and len(ops) > 0
            if mn == 'lea':
                continue
            # `and m,0` and `or m,-1` store a constant: the stored-over value is not read
            store = dest and len(ops) == 2 and (
                (mn == 'and' and ops[1] == '0x0') or (mn == 'or' and ops[1] in (
                    '0xffffffff', '0xff', '0xffff')))
            if dest and (wonly or store):
                e.mem_writes.append(op)
            else:
                e.mem_reads.append(op)
            continue
        rs = regs_in(op)
        if not rs:
            continue
        if k == 0 and (wonly or rmw or cmov or mn == 'imul' or mn == 'pop'):
            if wonly or mn == 'pop' or (mn == 'imul' and len(ops) == 3):
                e.writes |= rs
            else:
                e.reads |= rs
                e.writes |= rs
        else:
            e.reads |= rs
    return e


class Region:
    """What a walk learns about one function."""

    def __init__(self):
        self.read = {'ecx': None, 'edx': None}   # first address of a read before a write
        self.rets = set()       # the bytes the returns pop, a tail target's included
        self.deref = False      # ecx's value is a memory operand's base, or is returned
        self.slots = set()      # stack parameter words read, 0 for the first
        self.tails = set()      # rows the body jumps to
        self.opaque = False     # the walk met something it cannot follow
        self.written = set()    # registers some path writes, callees and tail targets included


DIRECT = re.compile(r'^0x([0-9a-f]+)$')
State = namedtuple('State', 'i cw dw eo bo taint pend pro')
# i: instruction index. cw, dw: ecx or edx written on this path. eo, bo: esp and ebp as offsets
# from the entry's esp, None when unknown. taint: registers and frame slots ('s', offset) that
# hold the value ecx came in with. pend: (slot, register) of a prologue `push ecx` or `push edx`
# whose slot is not yet read, written or passed. pro: only prologue instructions so far.


def direct_target(ops):
    """The address of a `call` or `jmp` operand that names one, else None."""
    if len(ops) == 1:
        m = DIRECT.match(ops[0])
        if m:
            return int(m.group(1), 16)
    return None


def ret_bytes(ops):
    try:
        return int(ops[0], 16) if ops else 0
    except ValueError:
        return 0


class Analyzer:
    """Walks a body from its entry over every path, with a state per instruction."""

    def __init__(self, listing, rows, image):
        self.L = listing
        self.rows = rows
        self.img = image
        self.starts = set(rows.starts)
        self.memo = {}
        self.busy = set()

    def extent(self, entry):
        r = self.rows.at.get(entry)
        return (entry, entry + r['size']) if r and r['size'] else (entry, None)

    def table_targets(self, base, lo, hi):
        """The dwords from `base` that lie in [lo, hi): a jump table's entries."""
        out = []
        a = base
        while a + 4 <= self.img.base + len(self.img.mem):
            v = self.img.u32(a)
            if not lo <= v < hi:
                break
            out.append(v)
            a += 4
        return out

    def analyze(self, entry):
        if entry in self.memo:
            return self.memo[entry]
        if entry in self.busy or entry not in self.L.at:
            r = Region()
            r.opaque = True
            r.written = set(GP32)
            return r
        self.busy.add(entry)
        reg = self.walk(entry)
        self.busy.discard(entry)
        self.memo[entry] = reg
        return reg

    def reserves_float(self, i):
        """Whether the `push ecx` or `push edx` at index i, with the pushes of the same two
        registers after it, only reserves room that an x87 store to [esp] fills a few
        instructions later in the same block."""
        L = self.L
        j = i + 1
        while j < len(L.addrs) and L.mn[j] == 'push' and L.ops[j] in (['ecx'], ['edx']):
            j += 1
        for _ in range(6):
            if j >= len(L.addrs):
                return False
            mn, ops = L.mn[j], L.ops[j]
            if mn in ('fstp', 'fst', 'fistp', 'fist') and len(ops) == 1:
                return re.search(r'PTR \[esp\]$', ops[0]) is not None
            if mn in ('call', 'jmp', 'push', 'pop') or mn.startswith(('j', 'ret')):
                return False
            j += 1
        return False

    def walk(self, entry):
        L = self.L
        reg = Region()
        work = [State(L.at[entry], False, False, 0, None, frozenset(['ecx']), frozenset(), True)]
        seen = set()
        budget = 40000
        while work:
            st = work.pop()
            while st is not None and st not in seen:
                seen.add(st)
                budget -= 1
                if budget < 0:
                    reg.opaque = True
                    work.clear()
                    break
                st = self.step(entry, reg, st, work)
        return reg

    @staticmethod
    def note_read(reg, r, at, st):
        """Records a read of ecx or edx that no write on the path precedes."""
        if reg.read[r] is None and not (st.cw if r == 'ecx' else st.dw):
            reg.read[r] = at

    def slot(self, op, st):
        """The offset from the entry's esp of a [ebp+d] or [esp+d] operand, else None."""
        t = mem_terms(op) if is_mem(op) else None
        if not t or t[1] is not None:
            return None
        if t[0] == 'ebp' and st.bo is not None:
            return st.bo + t[2]
        if t[0] == 'esp' and st.eo is not None:
            return st.eo + t[2]
        return None

    def step(self, entry, reg, st, work):
        """Applies the instruction at st.i; returns the state that follows it, or None where
        the path ends. Other branch states go to `work`."""
        L = self.L
        i = st.i
        a = L.addrs[i]
        mn, ops = L.mn[i], L.ops[i]
        if a != entry and a in self.starts:
            return None             # falls into another row
        if mn.startswith('ret'):
            reg.rets.add(ret_bytes(ops))
            if 'eax' in st.taint:
                reg.deref = True        # returns the value ecx came in with: a pointer
            return None
        if mn == 'int3':
            return None
        nxt = i + 1 if i + 1 < len(L.addrs) else None
        e = effect(mn, ops, L.rep[i])
        reg.written |= e.writes

        # reads of ecx and edx; a push in the prologue that reserves a local, or a float's room,
        # is no read
        reads = set(e.reads)
        pending = None
        if mn == 'push' and ops in (['ecx'], ['edx']):
            if self.reserves_float(i):
                reads.discard(ops[0])
            elif st.pro and st.eo is not None and not (st.cw if ops[0] == 'ecx' else st.dw):
                reads.discard(ops[0])
                pending = (st.eo - 4, ops[0])
        for r in ('ecx', 'edx'):
            if r in reads:
                self.note_read(reg, r, a, st)

        # memory: stack words read, slots the pending pushes occupy, pointer use of ecx
        npend = set(st.pend)
        for op in e.mem_reads + e.mem_writes:
            t = mem_terms(op)
            if not t:
                continue
            base, index, disp, size = t
            off = self.slot(op, st)
            if off is not None:
                for w in range(off, off + (size or 4), 4):
                    if w >= 4 and op in e.mem_reads:
                        reg.slots.add((w - 4) // 4)
                for slot, r in list(npend):
                    if slot <= off < slot + 4:
                        npend.discard((slot, r))
                        if op in e.mem_reads:
                            self.note_pending(reg, r, a)
            if base in st.taint and index is None:
                reg.deref = True
        if STRING_OP.match(mn) and ('edi' in st.taint or 'esi' in st.taint):
            reg.deref = True            # a string operation addresses through them
        if mn == 'lea' and len(ops) == 2:
            t = mem_terms(ops[1])
            if t:
                base, index, disp, size = t
                if base in st.taint and index is None and disp >= 4:
                    reg.deref = True            # the address of a field of the object
                off = self.slot(ops[1], st)
                if off is not None:
                    npend = {(s, r) for s, r in npend if not s <= off < s + 4}
                    if off >= 4:
                        reg.slots.add((off - 4) // 4)

        if mn == 'call':
            return self.step_call(reg, st, nxt, ops, a, npend)
        if mn == 'jmp' or (CC_JUMP.match(mn) and mn != 'jmp'):
            return self.step_jump(entry, reg, st, nxt, mn, ops, npend, work)

        # state: registers written, taint, esp and ebp
        cw = st.cw or 'ecx' in e.writes
        dw = st.dw or 'edx' in e.writes
        taint = set(st.taint)
        if mn == 'mov' and len(ops) == 2 and ops[0] in GP32 and ops[1] in GP32:
            (taint.add if ops[1] in st.taint else taint.discard)(ops[0])
        elif mn == 'mov' and len(ops) == 2 and ops[0] in GP32 and self.slot(ops[1], st) is not None:
            (taint.add if ('s', self.slot(ops[1], st)) in st.taint else taint.discard)(ops[0])
        elif mn == 'mov' and len(ops) == 2 and self.slot(ops[0], st) is not None:
            key = ('s', self.slot(ops[0], st))
            (taint.add if ops[1] in st.taint else taint.discard)(key)
        elif mn == 'push' and len(ops) == 1 and st.eo is not None:
            (taint.add if ops[0] in st.taint else taint.discard)(('s', st.eo - 4))
        elif mn in ('add', 'sub') and len(ops) == 2 and ops[0] in st.taint \
                and HEX.match(ops[1]):
            if mn == 'add' and 4 <= int(ops[1], 16) < 0x10000:
                reg.deref = True    # the address of a field, as `lea` takes it
            # an offset into the same object
        elif mn == 'inc' and len(ops) == 1 and ops[0] in st.taint:
            # a step into the same object; passed on, it is an address
            k = self.L.at[a]
            if any(self.L.mn[j] == 'push' and self.L.ops[j] == ops for j in range(
                    k + 1, min(k + 4, len(self.L.mn)))):
                reg.deref = True
        elif mn == 'lea' and len(ops) == 2 and ops[0] in GP32 and self.field_of_tainted(
                ops[1], st):
            taint.add(ops[0])
        else:
            taint -= e.writes
        eo, bo = st.eo, st.bo
        if mn == 'push':
            eo = eo - 4 if eo is not None else None
            if pending:
                npend.add(pending)
        elif mn in ('pop', 'popfd'):
            eo = eo + 4 if eo is not None else None
        elif mn == 'pushfd':
            eo = eo - 4 if eo is not None else None
        elif mn == 'pushad':
            eo = eo - 32 if eo is not None else None
        elif mn == 'popad':
            eo = eo + 32 if eo is not None else None
        elif mn == 'leave':
            eo, bo = (bo + 4 if bo is not None else None), None
        elif mn in ('add', 'sub') and ops and ops[0] == 'esp':
            try:
                v = int(ops[1], 16)
                eo = eo + v if mn == 'add' and eo is not None else (
                    eo - v if eo is not None else None)
            except ValueError:
                eo = None
        elif mn == 'mov' and ops and ops[0] == 'esp':
            eo = bo if ops[1] == 'ebp' else None
        elif mn == 'lea' and ops and ops[0] == 'esp':
            t = mem_terms(ops[1])
            eo = bo + t[2] if t and t[0] == 'ebp' and bo is not None else None
        elif 'esp' in e.writes:
            eo = None
        if mn == 'mov' and ops == ['ebp', 'esp']:
            bo = st.eo
        elif 'ebp' in e.writes:
            bo = None
        pro = st.pro and (mn in ('push', 'sub', 'nop') or ops == ['ebp', 'esp']
                          or (mn == 'mov' and len(ops) == 2 and 'fs:' in ops[1]))
        if nxt is None:
            return None
        return State(nxt, cw, dw, eo, bo, frozenset(taint), frozenset(npend), pro)

    @staticmethod
    def field_of_tainted(op, st):
        """Whether a memory operand is `[base+disp]` with a tainted base: lea of it is an offset
        into the same object."""
        t = mem_terms(op)
        return bool(t) and t[1] is None and t[0] in st.taint

    @staticmethod
    def note_pending(reg, r, at):
        """A register pushed unwritten holds the caller's value whatever happens after."""
        if reg.read[r] is None:
            reg.read[r] = at

    def step_call(self, reg, st, nxt, ops, a, npend):
        L = self.L
        target = direct_target(ops)
        known = target is not None and target in L.at
        eo = st.eo
        pops = None
        if known:
            sub = self.analyze(target)
            for r in ('ecx', 'edx'):
                if sub.read[r] is not None:
                    self.note_read(reg, r, a, st)
            if len(sub.rets) == 1:
                pops = next(iter(sub.rets))
            if sub.deref and 'ecx' in st.taint:
                reg.deref = True
            if eo is not None:
                # the call's argument words: the bytes it pops and the words it reads
                width = max(pops or 0, 4 * (max(sub.slots) + 1) if sub.slots else 0)
                for slot, r in list(npend):
                    if eo <= slot < eo + width:
                        npend.discard((slot, r))
                        self.note_pending(reg, r, a)
            if not sub.rets and not sub.tails and not sub.opaque:
                return None         # the callee never returns
            clobbers = set(sub.written)
        else:
            if eo is not None:
                # an unknown callee takes at least the word on top of the stack
                for slot, r in list(npend):
                    if slot == eo:
                        npend.discard((slot, r))
                        self.note_pending(reg, r, a)
            clobbers = {'eax', 'ecx', 'edx'}
        reg.written |= clobbers
        if pops is not None and eo is not None:
            eo += pops
        elif not known and st.bo is None:
            eo = None
        # the callee-saved registers keep their values across the call
        taint = frozenset(x for x in st.taint if x not in clobbers
                          or x not in ('eax', 'ecx', 'edx'))
        if nxt is None:
            return None
        return State(nxt, st.cw or 'ecx' in clobbers, st.dw or 'edx' in clobbers, eo, st.bo,
                     taint, frozenset(npend), False)

    def step_jump(self, entry, reg, st, nxt, mn, ops, npend, work):
        L = self.L
        target = direct_target(ops)
        lo, hi = self.extent(entry)
        follow = st._replace(pend=frozenset(npend), pro=False)
        if mn != 'jmp':
            # conditional: the target joins the work, the fall-through continues
            if target in L.at and not (target in self.starts and target != entry):
                work.append(follow._replace(i=L.at[target]))
            return follow._replace(i=nxt) if nxt is not None else None
        if target is not None:
            if target in self.starts and target != entry:
                # a tail call: the body takes what the target reads and pops
                sub = self.analyze(target)
                reg.tails.add(target)
                reg.written |= sub.written
                for r in ('ecx', 'edx'):
                    if sub.read[r] is not None:
                        self.note_read(reg, r, sub.read[r], st)
                if st.eo == 0:
                    reg.rets |= sub.rets
                reg.deref = reg.deref or (sub.deref and 'ecx' in st.taint)
                return None
            if target in L.at:
                return st._replace(i=L.at[target], pend=frozenset(npend))
            reg.opaque = True
            return None
        t = mem_terms(ops[0]) if ops and is_mem(ops[0]) else None
        if t and t[1] and t[0] is None and hi is not None:
            for dest in self.table_targets(t[2], lo, hi):    # a jump table
                if dest in L.at:
                    work.append(follow._replace(i=L.at[dest]))
            return None
        reg.opaque = True
        return None


# ----------------------------------------------------------------------------------- call sites

class Site:
    __slots__ = ('addr', 'ecx', 'edx', 'pushes', 'cleanup', 'ecx_kind', 'caller', 'live_ecx', 'own')


def block_facts(L, i, labels, rows, az, window=16):
    """For the `call` at listing index i: the state of ecx and edx before it, the pushes in the
    block and the cleanup right after."""
    s = Site()
    s.addr = L.addrs[i]
    s.ecx = s.edx = None
    s.ecx_kind = None
    s.pushes = 0
    row = rows.holding(s.addr)
    entry = row['address'] if row else None
    s.caller = entry
    state = {'ecx': None, 'edx': None}      # loaded, unknown or not, once decided
    read_after = {'ecx': False, 'edx': False}
    j = i - 1
    n = 0
    while j >= 0 and n < window and None in state.values():
        a = L.addrs[j]
        mn, ops = L.mn[j], L.ops[j]
        if mn.startswith('ret') or mn == 'jmp' or mn == 'int3':
            break
        if mn == 'call':
            target = None
            if len(ops) == 1 and DIRECT.match(ops[0]):
                target = int(DIRECT.match(ops[0]).group(1), 16)
            sub = az.analyze(target) if target in L.at else None
            for r in ('ecx', 'edx'):
                if state[r] is not None:
                    continue
                if sub is None or r in sub.written:
                    state[r] = 'unknown' if read_after[r] else 'not'
                elif sub.read[r] is not None:
                    read_after[r] = True
        else:
            e = effect(mn, ops, L.rep[j])
            if L.rep[j]:
                e.writes.discard('ecx')     # a counted string operation leaves ecx at zero
            if mn == 'push':
                s.pushes += 1
            for r in ('ecx', 'edx'):
                if state[r] is not None:
                    continue
                if r in e.writes and r in e.reads:
                    # `add ecx,edi` goes on computing a value: look for its start; an operation
                    # whose result only a conditional jump reads (`or edx,esi; je`) is a test
                    follows = L.mn[j + 1] if j + 1 < len(L.mn) else ''
                    if CC_JUMP.match(follows) and follows != 'jmp':
                        read_after[r] = True
                    continue
                if r in e.writes and mn != 'xchg':
                    state[r] = 'unknown' if read_after[r] else 'loaded'
                    if r == 'ecx':
                        s.ecx_kind = ecx_kind(mn, ops)
                elif r in e.reads or r in e.writes:
                    read_after[r] = True
        n += 1
        if a in labels or (entry is not None and a == entry):
            break
        j -= 1
    s.ecx = state['ecx'] or 'unknown'
    s.edx = state['edx'] or 'unknown'
    # cleanup right after the call
    s.cleanup = None
    s.live_ecx = ecx_live_after(L, i)
    s.own = class_of_caller(L, i, entry, row['name'] if row else None)
    if i + 1 < len(L.addrs):
        mn, ops = L.mn[i + 1], L.ops[i + 1]
        if mn == 'add' and ops and ops[0] == 'esp':
            try:
                s.cleanup = int(ops[1], 16)
            except ValueError:
                pass
    return s


def class_of_caller(L, i, entry, name):
    """The class of the method that holds the `call` at index i when nothing in it writes ecx
    before the call (ecx is the method's own `this`), else ''."""
    m = CLASS_RE.match(name or '')
    if not m or entry not in L.at:
        return ''
    for j in range(L.at[entry], i):
        if 'ecx' in effect(L.mn[j], L.ops[j], L.rep[j]).writes:
            return ''
        if L.mn[j] == 'call':
            return ''
    return m.group(1)


def ecx_live_after(L, i, window=16):
    """Whether the code after the `call` at index i reads ecx before it writes it: the caller keeps
    a value of its own there, so a load before the call is not an argument."""
    for j in range(i + 1, min(i + 1 + window, len(L.addrs))):
        mn = L.mn[j]
        if mn in ('call', 'jmp', 'int3') or mn.startswith('ret'):
            return False
        e = effect(mn, L.ops[j], L.rep[j])
        if 'ecx' in e.reads:
            return True
        if 'ecx' in e.writes:
            return False
    return False


def ecx_kind(mn, ops):
    """How a caller loads ecx: `lea`, the `addr` of a global, a `const` (zero or a small
    integer), or `other`."""
    if mn == 'lea' and len(ops) == 2:
        t = mem_terms(ops[1])
        # the address of a local or of a field; `lea ecx,[eax-1]` is arithmetic
        if t and t[1] is None and (t[0] in ('ebp', 'esp') or t[2] >= 4):
            return 'lea'
        return 'other'
    if mn in ('xor', 'sub') and len(ops) == 2 and ops[0] == ops[1]:
        return 'const'
    if mn == 'mov' and len(ops) == 2 and re.match(r'^0x[0-9a-f]+$', ops[1]):
        v = int(ops[1], 16)
        return 'addr' if 0x400000 <= v < 0x1000000 else 'const'
    return 'other'


# ----------------------------------------------------------------------------------- verdict

class Verdict:
    """A convention and the use of each register parameter; `conv` is None for a disagreement,
    with its reason in `why`."""

    def __init__(self, conv, ecx=None, edx=None, why='', basis='', fixed=None):
        self.conv, self.ecx, self.edx, self.why, self.basis = conv, ecx, edx, why, basis
        self.fixed = fixed      # the callback type a registration gives, else None

    def __repr__(self):
        return 'Verdict(%s,%s,%s,%s)' % (self.conv, self.ecx, self.edx, self.why)


def hexlist(sites):
    return ' '.join('%#x' % s.addr for s in sites[:4])


# The types a registration gives: the Lua C function (the state in ecx, the result count in eax)
# and the console command handler (the command name in ecx, its arguments in edx).
LUA_CFUNCTION = ('int', 'lua_State *L')
CONSOLE_HANDLER = (None, 'const char *name, const char *args')
NET_HANDLER = (None, 'void *param, uint32_t opcode, uint32_t time, CDataStore *msg')
# kind: (what it is, the bytes its return pops, its type, whether edx is a parameter)
CALLBACKS = {
    'lua': ('a Lua C function', 0, LUA_CFUNCTION, None),
    'console': ('a console command', 0, CONSOLE_HANDLER, 'used'),
    'net': ('a message handler', 8, NET_HANDLER, 'used'),
}


def decide(reg, sites, name, vslot=False, current=None, current_words=None, typed=False,
           text=False, variadic=False, callback=None, older=None):
    """The verdict for one body and its call sites; see the header for the rules."""
    e = reg.read['ecx'] is not None
    d = reg.read['edx'] is not None
    n = len(sites)
    loaded_d = [s for s in sites if s.edx == 'loaded']
    not_d = [s for s in sites if s.edx == 'not']
    cls = bool(CLASS_RE.match(name or ''))
    # every caller loads ecx and none keeps a value of its own in it past the call
    # (a method that passes its own `this` on to a method of its class loads nothing)
    own = class_of(name) if cls else None
    c_all = n > 0 and all(
        (s.ecx == 'loaded' or own and s.own == own) and not s.live_ecx for s in sites)
    d_all = n > 0 and len(loaded_d) == n
    loaded_c = [s for s in sites if s.ecx == 'loaded']
    integer = bool(loaded_c) and all(s.ecx_kind == 'const' for s in loaded_c)
    obj = cls or (not integer and not text and (reg.deref or typed or any(
        s.ecx_kind in ('lea', 'addr') for s in loaded_c)))
    if variadic:
        # only __cdecl takes `...`: the callee cannot pop what the caller pushed
        if e or d:
            return Verdict(None, why='a variadic function reads a register')
        return Verdict('__cdecl', basis='variadic')
    if callback:
        # reached through a pointer: the type it is registered as decides, whatever the body reads
        kind, pops_n, fixed, edx_ = CALLBACKS[callback]
        if reg.rets and reg.rets != {pops_n}:
            return Verdict(None, why='registered as %s, a return pops %s'
                           % (kind, sorted(reg.rets)))
        return Verdict('__fastcall', 'used', edx_, basis='registered as ' + kind, fixed=fixed)
    if len(reg.rets) > 1:
        return Verdict(None, why='returns pop different bytes: %s' % sorted(reg.rets))
    pops = next(iter(reg.rets)) if reg.rets else None
    if pops:
        cleaned = [s for s in sites if s.cleanup == pops]
        if cleaned:
            return Verdict(None, why='the body pops %d bytes and callers %s clean %d after the '
                            'call' % (pops, hexlist(cleaned), pops))
    if d:
        if n and len(not_d) == n:
            return Verdict(None, why='body reads edx, no caller loads it (%s)' % hexlist(not_d))
        if not_d:
            return Verdict(None, why='body reads edx, callers %s leave it undefined'
                            % hexlist(not_d))
        return Verdict('__fastcall', 'used' if e else 'unused', 'used', basis='edx read')
    if d_all:
        return Verdict('__fastcall', 'used' if e else 'unused', 'unused',
                        basis='edx loaded by every caller')
    if e:
        if loaded_d:
            return Verdict(None, why='edx loaded by some callers (%s), not by %s' % (
                hexlist(loaded_d), hexlist([s for s in sites if s.edx != 'loaded'])))
        if obj:
            return Verdict('__thiscall', 'used', None,
                            basis='ecx deref' if reg.deref else 'ecx object')
        return Verdict('__fastcall', 'used', None, basis='ecx integer')
    # neither register is read
    if c_all:
        kinds = {s.ecx_kind for s in sites}
        if obj or kinds - {'const'}:
            return Verdict('__thiscall', 'unused', None, basis='every caller loads ecx')
        return Verdict(None, why='every caller loads ecx with a constant, body ignores it')
    if vslot and cls:
        return Verdict('__thiscall', 'unused', None, basis='vtable slot of a class')
    kept = older or current
    if not n and kept in ('__fastcall', '__thiscall'):
        # reached by a pointer the tool does not see: the registers stay as the signature has them
        return Verdict(kept, 'kept', 'kept' if kept == '__fastcall' else None,
                       basis='no direct caller, reads neither register')
    if not reg.rets and not reg.tails:
        # nothing returns: __stdcall and __cdecl look the same, so the signature's stays
        return Verdict(current if current in ('__cdecl', '__stdcall') else '__cdecl',
                        basis='no return')
    if not pops and current == '__stdcall' and current_words == 0:
        # no stack parameters: __stdcall and __cdecl look the same
        return Verdict('__stdcall', basis='ret 0, no parameters')
    return Verdict('__stdcall' if pops else '__cdecl', basis='ret %s' % pops)


# -------------------------------------------------------------------------- signature handling

def split_params(text):
    """The top-level comma separated parameters of a parameter list."""
    text = text.strip()
    if text in ('', 'void'):
        return []
    out, depth, cur = [], 0, ''
    for ch in text:
        if ch in '([':
            depth += 1
        elif ch in ')]':
            depth -= 1
        if ch == ',' and depth == 0:
            out.append(cur.strip())
            cur = ''
        else:
            cur += ch
    out.append(cur.strip())
    return out


SIG_RE = re.compile(r'^(.*?)\s*(__cdecl|__stdcall|__fastcall|__thiscall)\s*\((.*)\)$')


def parse_sig(sig):
    """(return type, convention, parameters), or None."""
    m = SIG_RE.match(sig)
    if not m:
        return None
    return m.group(1), m.group(2), split_params(m.group(3))


TYPE_WORDS = {'void', 'char', 'short', 'int', 'long', 'float', 'double', 'unsigned', 'signed',
              'const', 'struct', 'bool', 'size_t', 'ptrdiff_t', 'intptr_t', 'uintptr_t',
              'wchar_t', 'va_list'}
STRUCT_NAMES = set()


def is_type_word(w):
    return (w in TYPE_WORDS or w in STRUCT_NAMES or re.match(r'^u?int(8|16|32|64)_t$', w)
            is not None)


def param_name(p):
    m = re.search(r'\(\s*\*+\s*(\w+)\s*\)', p)
    if m:
        return m.group(1)
    m = re.search(r'[\s*](\w+)(\s*\[\d*\])?$', ' ' + p)
    if not m:
        return None
    words_ = re.findall(r'[\w:]+', p)
    if all(is_type_word(w) for w in words_):
        return None
    return m.group(1)


def param_type(p):
    n = param_name(p)
    if n is None:
        return p.strip()
    if '(*' in p.replace(' ', ''):
        return p.replace(n, '', 1).strip()
    return re.sub(r'\s*' + re.escape(n) + r'(\s*\[\d*\])?$', '', p).strip()


def words(p, structs):
    t = param_type(p)
    if t in ('uint64_t', 'int64_t', 'double'):
        return 2
    if t in structs:
        return (structs[t] + 3) // 4
    return 1


def reg_eligible(p, structs):
    t = param_type(p)
    return t not in ('float', 'double', 'uint64_t', 'int64_t') and t not in structs


def stack_words(params, structs):
    return sum(words(p, structs) for p in params)


def split_regs(conv, params, structs, k=None):
    """The register parameters and the stack parameters of a list read under `conv`. A
    __thiscall list whose first parameter is not `this` has it first only when its words are one
    more than the k stack words."""
    if conv == '__thiscall':
        if params and (param_name(params[0]) == 'this'
                       or k is not None and stack_words(params, structs) == k + 1):
            return params[:1], params[1:]
        return [], params
    if conv == '__fastcall':
        regs, stack = [], []
        for p in params:
            if len(regs) < 2 and reg_eligible(p, structs):
                regs.append(p)
            else:
                stack.append(p)
        return regs, stack
    return [], params


def is_arg(p):
    n = param_name(p)
    return bool(n and re.match(r'^arg\d+$', n))


def join_type(t):
    """A type as the table writes it before a name: `T *` takes no space before the name."""
    return t if t.endswith('*') else t + ' '


def class_of(name):
    m = CLASS_RE.match(name or '')
    return m.group(1) if m else 'void'


def trim_args(params, k, structs):
    """Drops trailing `argN` placeholders while the list is longer than k words."""
    out = list(params)
    while out and stack_words(out, structs) > k and is_arg(out[-1]):
        out.pop()
    return out


def trim_to(params, k, structs):
    """The list without its trailing parameters when that makes it exactly k words, else None."""
    out = list(params)
    while out and stack_words(out, structs) > k:
        out.pop()
    return out if stack_words(out, structs) == k and len(out) < len(params) else None


def renumber(params):
    """`argN` names count from 1 over every parameter of the signature, `this` included."""
    out = []
    for pos, p in enumerate(params, 1):
        n = param_name(p)
        if n and re.match(r'^arg\d+$', n):
            p = p[:len(p) - len(n)] + 'arg%d' % pos if p.endswith(n) else p
        out.append(p)
    return out


def layout(sig, dec, k, structs):
    """Where a signature puts its names: ({0: ecx parameter, 1: edx parameter}, stack list).
    A __thiscall list that omits `this` lists a register parameter first when the body takes edx
    (it is the edx one) or when the words are one more than the k stack words (it is ecx)."""
    _, conv, params = sig
    named = {}
    if conv == '__thiscall':
        if params and param_name(params[0]) == 'this':
            named[0], stack = params[0], params[1:]
        elif dec.conv == '__fastcall' and dec.edx == 'used' and params:
            named[1], stack = params[0], params[1:]
        elif k is not None and stack_words(params, structs) == k + 1:
            named[0], stack = params[0], params[1:]
        else:
            stack = list(params)
    elif conv == '__fastcall':
        regs, stack = split_regs(conv, params, structs)
        named = dict(enumerate(regs))
    else:
        used = [i for i, u in ((0, dec.ecx), (1, dec.edx)) if u == 'used']
        if dec.conv in ('__fastcall', '__thiscall') and k is not None and used \
                and stack_words(params, structs) == len(used) + k:
            named = {slot: params[n] for n, slot in enumerate(used)}
            stack = params[len(used):]
        else:
            stack = list(params)
    return named, stack


def compose(row, cur, orig, dec, reg, structs, no_args=False):
    """The signature the verdict gives, and notes. Stack parameter names keep their slots:
    the stack list is the first candidate whose words are the `ret N` bytes, the older table's
    first where its convention differs from the current one (a change of convention can slide
    names between slots)."""
    ret, cconv, cparams = cur
    notes = []
    if dec.fixed:
        # a registered callback has the type of its registration
        return '%s __fastcall (%s)' % (dec.fixed[0] or ret, dec.fixed[1]), notes
    k = next(iter(reg.rets)) // 4 if len(reg.rets) == 1 else None
    cnamed, cstack = layout(cur, dec, k, structs)
    onamed, ostack = layout(orig, dec, k, structs) if orig else ({}, None)
    cands = [('current', cstack)]
    if orig:
        first = orig[1] != cconv
        cands = ([('the older table', ostack)] + cands) if first else (cands + [('the older table', ostack)])
        if orig[1] == dec.conv and k is not None:
            trimmed = trim_to(ostack, k, structs)
            if trimmed is not None:
                cands.insert(1 if first else len(cands), ('the older table, trailing names dropped',
                                                          trimmed))
    if dec.conv == '__cdecl' or k is None:
        # no ret to count: every existing parameter except a register placeholder stays
        if dec.conv == '__cdecl':
            # no register is loaded: a register parameter of the older convention is dropped
            regconv = orig[1] if orig and orig[1] in ('__fastcall', '__thiscall') else (
                cconv if cconv in ('__fastcall', '__thiscall') else None)
            if regconv:
                stack = list(ostack if orig and orig[1] == regconv else cstack)
                notes.append('register parameters of the %s dropped' % regconv)
            else:
                stack = list(cparams)
            if no_args and regconv and '...' not in cparams:
                # nothing reads a stack word and no caller pushes one
                stack = []
                notes.append('no register or stack parameter is read or pushed')
        else:
            stack = cstack
            notes.append('no ret: stack list kept')
    else:
        stack = None
        for label, cand in cands:
            cand = trim_args(cand, k, structs)
            if stack_words(cand, structs) == k:
                stack = cand
                if label != 'current':
                    notes.append('stack names from %s' % label)
                break
        if stack is None:
            base = trim_args(cands[0][1], k, structs)
            stack = [] if stack_words(base, structs) > k else list(base)
            while stack_words(stack, structs) < k:
                stack.append('uint32_t arg0')
            notes.append('stack names padded: names uncertain')
    taken = {param_name(p) for p in stack}
    order = [onamed, cnamed] if orig and orig[1] != cconv else [cnamed, onamed]

    def existing(i):
        cs = [m[i] for m in order if i in m]
        for p in cs:
            n = param_name(p)
            if n and n != 'this' and not n.startswith('unused') and n not in taken:
                taken.add(n)
                return p
        for p in cs:
            if param_name(p) == 'this' and param_type(p).endswith('*'):
                return join_type(param_type(p)) + 'arg0'
        return 'uint32_t arg0'

    def unused(i):
        for m in order:
            if i in m and (param_name(m[i]) or '').startswith('unused'):
                return m[i]
        return 'uint32_t unused'

    regs = []
    if dec.conv == '__thiscall':
        regs = ['%s *this' % class_of(row['name'])]
        old_this = next((m[0] for m in order if 0 in m and param_name(m[0]) == 'this'
                         and (param_type(m[0]) != 'void *' or class_of(row['name']) == 'void')),
                        None)
        first = cnamed.get(0)
        if old_this:
            regs = [old_this]
        elif first and param_type(first).endswith('*') and re.match(
                r'^[A-Z][\w:]*\s*\*$', param_type(first)):
            regs = [join_type(param_type(first)) + 'this']
    elif dec.conv == '__fastcall':
        def kept(i):
            return next((m[i] for m in order if i in m), None)

        if dec.ecx == 'kept':
            regs = [kept(0) or unused(0)]
        else:
            regs = [unused(0) if dec.ecx == 'unused' else existing(0)]
        if dec.edx == 'kept' and kept(1):
            regs.append(kept(1))
        elif dec.edx == 'used':
            regs.append(existing(1))
        elif dec.edx == 'unused':
            regs.append(unused(1))
        elif any(reg_eligible(p, structs) for p in stack):
            regs.append('uint32_t unused')
    plist = regs + stack
    un = [i for i, p in enumerate(plist) if param_name(p) == 'unused']
    if len(un) > 1:
        for n, i in enumerate(un, 1):
            plist[i] = plist[i].replace('unused', 'unused%d' % n)
    plist = renumber(plist)
    seen = set()
    for i, p in enumerate(plist):
        n = param_name(p)
        if n and n in seen:
            plist[i] = join_type(param_type(p)) + 'arg%d' % (i + 1)
            notes.append('name %s repeated' % n)
        seen.add(n)
    empty = '()' if cparams == [] and row['signature'].endswith('()') else '(void)'
    text = '%s %s %s' % (ret, dec.conv, '(%s)' % ', '.join(plist) if plist else empty)
    return text, notes


def shape_problems(row, dec, reg, structs):
    """What in a row's signature disagrees with the verdict: the convention, `this`, `unused`,
    the stack words and the register parameters."""
    parsed = parse_sig(row['signature'])
    if not parsed:
        return ['signature does not parse']
    _, conv, params = parsed
    out = []
    if conv != dec.conv:
        out.append('convention %s, decided %s' % (conv, dec.conv))
    regs, stack = split_regs(conv, params, structs)
    for p in params:
        if param_name(p) == 'this' and conv != '__thiscall':
            out.append('`this` outside __thiscall')
    if conv == '__thiscall' and not (params and param_name(params[0]) == 'this'):
        out.append('__thiscall without `this` first')
    if len(reg.rets) == 1 and conv != '__cdecl':
        k = next(iter(reg.rets)) // 4
        if stack_words(stack, structs) != k:
            out.append('stack words %d, ret pops %d' % (stack_words(stack, structs), k))
    if reg.slots and max(reg.slots) + 1 > stack_words(stack, structs):
        out.append('the body reads stack word %d, the signature has %d'
                   % (max(reg.slots) + 1, stack_words(stack, structs)))
    if conv == '__fastcall' and dec.conv == '__fastcall':
        if dec.ecx == 'unused' and regs and param_name(regs[0]) != 'unused' \
                and not param_name(regs[0] or '').startswith('unused'):
            out.append('ecx is not read: its parameter is not named unused')
        if dec.edx == 'unused' and len(regs) > 1 and not param_name(regs[1]).startswith('unused'):
            out.append('edx is not read: its parameter is not named unused')
        if dec.edx == 'used' and len(regs) < 2:
            out.append('edx is a parameter and the signature has none')
    return out


def load_resolved(path):
    """Hand verdicts for rows the tool reports: address, convention, ecx, edx, reason."""
    out = {}
    if not Path(path).exists():
        return out
    with open(path, encoding='utf-8') as f:
        next(f)
        for line in f:
            c = line.rstrip('\n').split('\t')
            if len(c) >= 5:
                out[int(c[0], 16)] = Verdict(c[1], c[2] or None, c[3] or None,
                                              basis='resolved: ' + c[4])
    return out


def vtable_slots(img, rows, text):
    """Addresses of rows stored in a data table next to another row's address."""
    starts = set(rows.starts)
    found = set()
    for name, (lo, hi, _) in img.sections.items():
        if name == '.text':
            continue
        a = lo + (-lo) % 4
        prev = False
        while a + 4 <= hi:
            v = img.u32(a)
            cur = v in starts
            if cur and prev:
                found.add(v)
                found.add(img.u32(a - 4))
            prev = cur
            a += 4
    return found


def lua_registered(img, rows):
    """Addresses of rows that sit in a table of (name, function) pairs: two or more pairs in a
    row, or one followed by a pair of zeros, in a data section."""
    starts = set(rows.starts)
    found = set()
    for name, (lo, hi, _) in img.sections.items():
        if name == '.text':
            continue
        a = lo + (-lo) % 4
        run = []
        while a + 8 <= hi:
            if img.u32(a + 4) in starts and img.cstr(img.u32(a)):
                run.append(img.u32(a + 4))
                a += 8
                continue
            if len(run) > 1 or run and img.u32(a) == 0 == img.u32(a + 4):
                found.update(run)
            run = []
            a += 4
    return found


# The register functions and where a direct call passes the function: `edx` is a `mov edx,imm`;
# `push2` is the second `push imm` back from the call.
REGISTERERS = (('lua', 'FrameScriptRegisterFunction', 'edx'),
               ('console', 'ConsoleCommandRegister', 'edx'),
               ('net', 'ClientServices::SetMessageHandler', 'edx'),
               ('net', 'NetClient::SetMessageHandler', 'push2'))


def registered_by_call(L, rows, labels):
    """The functions the direct calls of the register functions pass, by kind."""
    out = {kind: set() for kind, _, _ in REGISTERERS}
    for kind, fn, how in REGISTERERS:
        target = next((r['address'] for r in rows.rows if r['name'] == fn), None)
        if target is None:
            continue
        for i, mn in enumerate(L.mn):
            if mn != 'call' or L.ops[i] != ['0x%x' % target]:
                continue
            j, pushes = i - 1, 0
            while j >= 0 and i - j <= 8 and L.mn[j] not in ('call', 'ret', 'jmp'):
                ops = L.ops[j]
                v = int(ops[-1], 16) if ops and HEX.match(ops[-1]) else None
                if how == 'edx' and L.mn[j] == 'mov' and ops[0] == 'edx' and v in rows.at:
                    out[kind].add(v)
                    break
                if how == 'push2' and L.mn[j] == 'push':
                    pushes += 1
                    if pushes == 2:
                        if v in rows.at:
                            out[kind].add(v)
                        break
                if L.addrs[j] in labels:
                    break
                j -= 1
    return out


def build_all(args, original=None):
    rows = Functions()
    img = Image(args.exe)
    L = Listing(args.disasm)
    az = Analyzer(L, rows, img)
    labels = set()
    for i in range(len(L.addrs)):
        mn, ops = L.mn[i], L.ops[i]
        if (mn == 'jmp' or CC_JUMP.match(mn)) and len(ops) == 1 and DIRECT.match(ops[0]):
            labels.add(int(DIRECT.match(ops[0]).group(1), 16))
    sites = {}
    for i, mn in enumerate(L.mn):
        if mn == 'call' and len(L.ops[i]) == 1 and DIRECT.match(L.ops[i][0]):
            t = int(DIRECT.match(L.ops[i][0]).group(1), 16)
            if t in rows.at and rows.at[t]['signature']:
                sites.setdefault(t, []).append(i)
    structs = {}
    with open(ROOT / 'symbols' / 'structs.tsv', encoding='utf-8') as f:
        head = next(f).rstrip('\n').split('\t')
        name, size = head.index('name'), head.index('size')
        for line in f:
            c = line.rstrip('\n').split('\t')
            structs[c[name]] = int(c[size])
    STRUCT_NAMES.update(structs)
    vslots = vtable_slots(img, rows, img.text)
    by_call = registered_by_call(L, rows, labels)
    lua = lua_registered(img, rows) | by_call['lua']
    callbacks = {a: 'lua' for a in lua}
    callbacks.update({a: 'net' for a in by_call['net']})
    callbacks.update({a: 'console' for a in by_call['console']})
    resolved = load_resolved(ROOT / 'tools' / 'conventions-resolved.tsv')
    results = []
    for row in rows.rows:
        if not row['signature'] or row['address'] not in L.at:
            continue
        a = row['address']
        reg = az.analyze(a)
        ss = [block_facts(L, i, labels, rows, az) for i in sites.get(a, [])]
        cp = parse_sig(row['signature'])
        cw = None
        if cp:
            cw = stack_words(split_regs(cp[1], cp[2], structs)[1], structs)
        typed = text = False
        if cp:
            first = split_regs(cp[1], cp[2], structs)[0][:1]
            typed = bool(first) and re.match(r'^(const )?[A-Z][\w:]*\s*\*+$',
                                             param_type(first[0])) is not None
            text = bool(first) and re.match(r'^(const )?(w?char)\s*\*+$',
                                            param_type(first[0])) is not None
        old = (original or {}).get(a)
        dec = decide(reg, ss, row['name'], a in vslots, cp[1] if cp else None, cw, typed, text,
                     bool(cp) and '...' in cp[2], callbacks.get(a), old[1] if old else None)
        if a in resolved:
            dec = resolved[a]
        results.append((row, reg, ss, dec))
    rows.registered = (callbacks, az, L, labels)
    return rows, structs, results


def unpaired(rows, L, labels):
    """Addresses a direct call of a register function passes that no function row starts at."""
    seen = {}
    for kind, fn, how in REGISTERERS:
        target = next((r['address'] for r in rows.rows if r['name'] == fn), None)
        for i, mn in enumerate(L.mn):
            if target is None or mn != 'call' or L.ops[i] != ['0x%x' % target]:
                continue
            j, pushes = i - 1, 0
            while j >= 0 and i - j <= 8 and L.mn[j] not in ('call', 'ret', 'jmp'):
                ops = L.ops[j]
                v = int(ops[-1], 16) if ops and HEX.match(ops[-1]) else None
                if how == 'edx' and L.mn[j] == 'mov' and ops[0] == 'edx' and v:
                    if v not in rows.at:
                        seen[v] = kind
                    break
                if how == 'push2' and L.mn[j] == 'push':
                    pushes += 1
                    if pushes == 2:
                        if v and v not in rows.at:
                            seen[v] = kind
                        break
                if L.addrs[j] in labels:
                    break
                j -= 1
    return seen


def fill_empty(rows, structs):
    """Edits giving each empty-signature row the signature its registration gives, and the
    registered rows left alone with the reason."""
    callbacks, az, L, _ = rows.registered
    edits, left = {}, []
    for a, kind in sorted(callbacks.items()):
        row = rows.at.get(a)
        if row is None or row['signature']:
            continue
        what, pops, fixed, _ = CALLBACKS[kind]
        if a not in L.at:
            left.append((a, 'no instruction at the address'))
            continue
        reg = az.analyze(a)
        if reg.rets and reg.rets != {pops}:
            left.append((a, 'a return pops %s, registered as %s' % (sorted(reg.rets), what)))
            continue
        edits[a] = '%s __fastcall (%s)' % (fixed[0] or 'uint32_t', fixed[1])
    return edits, left


def main():
    import argparse
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--exe', default=str(ROOT / 'WoW' / 'WoW.exe'))
    ap.add_argument('--disasm', default=str(ROOT / 'dumps' / 'disasm.txt'))
    ap.add_argument('--older', help='a functions.tsv whose stack names are tried where '
                    'a row changes convention')
    ap.add_argument('--report', help='write one line per row')
    ap.add_argument('--apply', action='store_true', help='rewrite symbols/functions.tsv')
    ap.add_argument('--check', action='store_true',
                    help='list every row whose signature disagrees with its verdict')
    ap.add_argument('--fill', action='store_true',
                    help='with --apply: fill the empty signature of a registered row')
    ap.add_argument('--test', action='store_true')
    args = ap.parse_args()
    if args.test:
        return selftest()
    original = {}
    if args.older:
        with open(args.older, encoding='utf-8') as f:
            next(f)
            for line in f:
                c = line.rstrip('\n').split('\t')
                original[int(c[0], 16)] = parse_sig(c[6])
    rows, structs, results = build_all(args, original)
    counts, report, edits, bad = {}, [], {}, []
    for row, reg, ss, dec in results:
        cur = parse_sig(row['signature'])
        conv = dec.conv or '?'
        counts[conv] = counts.get(conv, 0) + 1
        status = 'disagree: ' + dec.why if dec.conv is None else (
            'agree' if cur and cur[1] == dec.conv else 'change')
        if dec.conv is None:
            bad.append((row, dec.why))
        else:
            no_args = not reg.slots and bool(ss) and all(s.pushes == 0 for s in ss)
            text, notes = compose(row, cur, original.get(row['address']), dec, reg, structs,
                                  no_args)
            if text != row['signature']:
                edits[row['address']] = (text, notes)
            if args.check:
                for p in shape_problems(row, dec, reg, structs):
                    bad.append((row, p))
        report.append((addr(row['address']), row['name'], cur[1] if cur else '?', conv, status,
                       dec.basis, 'ecx' if reg.read['ecx'] is not None else '-',
                       'edx' if reg.read['edx'] is not None else '-',
                       ','.join(str(x) for x in sorted(reg.rets)), len(ss),
                       sum(s.ecx == 'loaded' for s in ss), sum(s.edx == 'loaded' for s in ss),
                       sum(s.edx == 'not' for s in ss), max((s.pushes for s in ss), default=0),
                       sum(s.cleanup is not None for s in ss)))
    decided = sum(v for k, v in counts.items() if k != '?')
    print('rows with a signature: %d, decided: %d' % (len(results), decided))
    for k in sorted(counts):
        print('  %-11s %d' % (k, counts[k]))
    print('convention differs from the signature: %d' % sum(1 for r in report if r[4] == 'change'))
    print('signature text would change: %d' % len(edits))
    print('reported, not decided: %d' % sum(1 for r in report if r[4].startswith('disagree')))
    print('settled by hand (tools/conventions-resolved.tsv): %d'
          % sum(1 for r in results if r[3].basis.startswith('resolved')))
    if args.report:
        with open(args.report, 'w', encoding='utf-8') as f:
            f.write('address\tname\tcurrent\tdecided\tstatus\tbasis\tecx\tedx\tret\tsites'
                    '\tecx_loaded\tedx_loaded\tedx_not\tpushes\tcleaned\n')
            for r in report:
                f.write('\t'.join(str(x) for x in r) + '\n')
    if args.check:
        for row, why in bad:
            print('%s %s: %s' % (addr(row['address']), row['name'], why))
        return 1 if bad else 0
    if args.apply:
        path = ROOT / 'symbols' / 'functions.tsv'
        lines = path.read_text(encoding='utf-8').split('\n')
        for k, line in enumerate(lines[1:], 1):
            c = line.split('\t')
            if len(c) > 7 and c[0].startswith('0x') and int(c[0], 16) in edits:
                c[6] = edits[int(c[0], 16)][0]
                lines[k] = '\t'.join(c)
        if args.fill:
            fills, left = fill_empty(rows, structs)
            for k, line in enumerate(lines[1:], 1):
                c = line.split('\t')
                if len(c) > 7 and c[0].startswith('0x') and int(c[0], 16) in fills:
                    c[6] = fills[int(c[0], 16)]
                    lines[k] = '\t'.join(c)
            kinds = {}
            for a in fills:
                kinds[rows.registered[0][a]] = kinds.get(rows.registered[0][a], 0) + 1
            print('filled %d: %s' % (len(fills), sorted(kinds.items())))
            for a, why in left:
                print('not written %s: %s' % (addr(a), why))
            for a, kind in sorted(unpaired(rows, *rows.registered[2:]).items()):
                print('not paired %s: registered as %s, no function row' % (addr(a), kind))
        path.write_text('\n'.join(lines), encoding='utf-8')
        for a, (_, notes) in sorted(edits.items()):
            for n in notes:
                print('%s: %s' % (addr(a), n))
        print('applied %d' % len(edits))
    return 0


class FakeRows:
    """The parts of Functions the walks use: starts, at, holding."""

    def __init__(self, spans):
        self.starts = sorted(spans)
        self.at = {a: {'address': a, 'size': n, 'signature': 'x', 'name': ''}
                   for a, n in spans.items()}

    def holding(self, va):
        for a in self.starts:
            if a <= va < a + self.at[a]['size']:
                return self.at[a]
        return None


class FakeImage:
    base = 0x400000
    mem = b''


def listing(text):
    """A Listing from `addr: instruction` lines."""
    L = Listing.__new__(Listing)
    L.addrs, L.mn, L.ops, L.rep = [], [], [], []
    for line in text.strip().splitlines():
        head, _, ins = line.strip().partition(':')
        L.add(int(head, 16), ins.strip())
    L.at = {a: i for i, a in enumerate(L.addrs)}
    return L


def region(text, spans):
    az = Analyzer(listing(text), FakeRows(spans), FakeImage())
    return az, az.analyze(min(spans))


def selftest():
    ok = []

    def check(name, got, want):
        if got != want:
            print('FAIL %s: got %r, want %r' % (name, got, want))
            ok.append(name)

    def reads(text, spans=None):
        _, r = region(text, spans or {0x1000: 0x100})
        return (r.read['ecx'] is not None, r.read['edx'] is not None)

    # reads before writes
    check('plain read', reads('1000: mov eax,ecx\n1002: ret'), (True, False))
    check('write first', reads('1000: mov ecx,1\n1005: mov eax,ecx\n1007: ret'), (False, False))
    check('xor zero', reads('1000: xor edx,edx\n1002: mov eax,edx\n1004: ret'), (False, False))
    check('or -1', reads('1000: or edx,0xffffffff\n1003: mov eax,edx\n1005: ret'), (False, False))
    check('partial write', reads('1000: xor dl,dl\n1002: mov eax,edx\n1004: ret'), (False, False))
    check('read one path', reads('1000: test eax,eax\n1002: je 0x1008\n1004: mov ecx,1\n'
                                 '1009: jmp 0x100a\n1008: mov eax,ecx\n100a: ret'),
          (True, False))
    check('rep reads ecx', reads('1000: rep stos DWORD PTR es:[edi],eax\n1002: ret'),
          (True, False))
    check('movsx is no string op', reads('1000: movsx eax,BYTE PTR [edx]\n1003: ret'),
          (False, True))
    check('and m,0 stores', reads('1000: push ebp\n1001: mov ebp,esp\n1003: push ecx\n'
                                  '1004: and DWORD PTR [ebp-0x4],0x0\n1008: mov eax,[ebp-0x4]\n'
                                  '100b: pop ecx\n100c: pop ebp\n100d: ret'), (False, False))
    # a push that reserves a local, passes a register, or spills it
    check('push reserves', reads('1000: push ebp\n1001: mov ebp,esp\n1003: push ecx\n'
                                 '1004: mov DWORD PTR [ebp-0x4],0x5\n100b: mov esp,ebp\n'
                                 '100d: pop ebp\n100e: ret'), (False, False))
    check('push spills', reads('1000: push ebp\n1001: mov ebp,esp\n1003: push ecx\n'
                               '1004: mov eax,DWORD PTR [ebp-0x4]\n1007: mov esp,ebp\n'
                               '1009: pop ebp\n100a: ret'), (True, False))
    check('push float room', reads('1000: push ebp\n1001: mov ebp,esp\n1003: fld DWORD PTR [ebp+0x8]\n'
                                   '1006: push ecx\n1007: fmul DWORD PTR [ebp+0xc]\n'
                                   '100a: fstp DWORD PTR [esp]\n100d: pop ecx\n100e: pop ebp\n'
                                   '100f: ret'), (False, False))
    arg = '1000: push ebp\n1001: mov ebp,esp\n1003: push ecx\n1004: call 0x2000\n1009: pop ecx\n' \
          '100a: pop ebp\n100b: ret\n'
    check('push passes a register',
          reads(arg + '2000: mov eax,DWORD PTR [esp+0x4]\n2004: ret', {0x1000: 0x20, 0x2000: 8}),
          (True, False))
    check('push reserves for a call that takes none',
          reads(arg + '2000: xor eax,eax\n2002: ret', {0x1000: 0x20, 0x2000: 8}), (False, False))
    # inheritance through jmp and call
    spans = {0x1000: 0x10, 0x2000: 0x10}
    check('thunk jmp inherits', reads('1000: jmp 0x2000\n2000: mov eax,edx\n2002: ret', spans),
          (False, True))
    check('call inherits when unwritten',
          reads('1000: call 0x2000\n1005: ret\n2000: mov eax,ecx\n2002: ret', spans),
          (True, False))
    check('call inherits nothing after a write',
          reads('1000: mov ecx,1\n1005: call 0x2000\n100a: ret\n2000: mov eax,ecx\n2002: ret',
                spans), (False, False))
    check('call that keeps edx', reads('1000: call 0x2000\n1005: mov eax,edx\n1007: ret\n'
                                       '2000: xor eax,eax\n2002: ret', spans), (False, True))
    check('call that writes edx', reads('1000: call 0x2000\n1005: mov eax,edx\n1007: ret\n'
                                        '2000: mov edx,1\n2005: ret', spans), (False, False))
    # ret N, thunks, tails
    _, r = region('1000: ret 0x8', {0x1000: 4})
    check('ret N', r.rets, {8})
    _, r = region('1000: jmp 0x2000\n2000: ret 0xc', spans)
    check('tail inherits ret', r.rets, {12})
    check('tail recorded', sorted(r.tails), [0x2000])
    _, r = region('1000: call 0x2000\n1005: nop\n2000: call 0x2000\n', {0x1000: 6, 0x2000: 5})
    check('no return', (r.rets, r.opaque), (set(), False))
    # object test
    _, r = region('1000: mov esi,ecx\n1002: mov eax,DWORD PTR [esi+0x8]\n1005: ret', {0x1000: 8})
    check('deref through a copy', r.deref, True)
    _, r = region('1000: mov eax,ecx\n1002: shl eax,0x3\n1005: ret', {0x1000: 8})
    check('integer use', r.deref, False)
    _, r = region('1000: mov eax,ecx\n1002: ret', {0x1000: 4})
    check('returns the pointer it got', r.deref, True)
    _, r = region('1000: add ecx,0xc\n1003: jmp 0x2000\n2000: mov eax,DWORD PTR [ecx+0x4]\n'
                  '2003: ret', {0x1000: 8, 0x2000: 4})
    check('an offset into the object', r.deref, True)
    _, r = region('1000: lea ecx,[ecx+0x10]\n1003: mov eax,DWORD PTR [ecx]\n1005: ret', {0x1000: 8})
    check('a field address is the object', r.deref, True)
    _, r = region('1000: mov esi,ecx\n1002: add esi,0x4\n1005: push esi\n1006: ret', {0x1000: 8})
    check('a field address by add', r.deref, True)
    _, r = region('1000: mov esi,ecx\n1002: add esi,0x1\n1005: push esi\n1006: ret', {0x1000: 8})
    check('a small offset is arithmetic', r.deref, False)
    _, r = region('1000: mov esi,ecx\n1002: inc esi\n1003: push esi\n1004: ret', {0x1000: 8})
    check('a stepped address passed on', r.deref, True)
    _, r = region('1000: push ebp\n1001: mov ebp,esp\n1003: push ecx\n1004: mov DWORD PTR [ebp-0x4],ecx\n'
                  '1007: mov eax,DWORD PTR [ebp-0x4]\n100a: mov eax,DWORD PTR [eax+0x4]\n'
                  '100d: pop ecx\n100e: pop ebp\n100f: ret', {0x1000: 0x20})
    check('deref through a spill', r.deref, True)

    # call sites
    def site(text, spans, labels=()):
        L = listing(text)
        rows = FakeRows(spans)
        az = Analyzer(L, rows, FakeImage())
        i = max(i for i, m in enumerate(L.mn) if m == 'call')
        s = block_facts(L, i, set(labels), rows, az)
        return s.ecx, s.edx

    sp = {0x1000: 0x80, 0x2000: 0x10}
    tail = '\n2000: ret'
    check('loaded', site('1000: mov ecx,esi\n1002: mov edx,0x1\n1007: call 0x2000' + tail, sp),
          ('loaded', 'loaded'))
    check('a value computed in steps', site('1000: mov ecx,DWORD PTR [esi+0x8]\n1003: add ecx,edi\n'
                                            '1005: call 0x2000' + tail, sp),
          ('loaded', 'unknown'))
    check('a test is no load', site('1000: mov edx,edi\n1002: or edx,esi\n1004: je 0x1008\n'
                                    '1006: call 0x2000' + tail, sp, [0x1008]),
          ('unknown', 'unknown'))
    check('temporary', site('1000: mov edx,DWORD PTR [esi]\n1002: mov eax,DWORD PTR [edx+4]\n'
                            '1005: call 0x2000' + tail, sp), ('unknown', 'unknown'))
    check('after a call', site('1000: call 0x2000\n1005: call 0x2000\n2000: xor ecx,ecx\n'
                               '2002: xor edx,edx\n2004: ret', sp), ('not', 'not'))
    check('after a call that keeps them',
          site('1000: call 0x2000\n1005: call 0x2000' + tail, sp), ('unknown', 'unknown'))
    check('past a call that keeps them', site(
        '1000: mov ecx,esi\n1002: call 0x3000\n1007: call 0x2000\n2000: ret\n3000: ret',
        {0x1000: 0x80, 0x2000: 1, 0x3000: 1}), ('loaded', 'unknown'))
    check('label', site('1000: nop\n1001: mov ecx,esi\n1003: call 0x2000' + tail, sp, [0x1001]),
          ('loaded', 'unknown'))
    check('entry', site('1000: call 0x2000' + tail, sp), ('unknown', 'unknown'))
    check('push then read', site('1000: mov ecx,esi\n1002: push ecx\n1003: call 0x2000' + tail,
                                 sp), ('unknown', 'unknown'))

    # verdicts
    def reg_of(read_c=False, read_d=False, rets=(0,), deref=False):
        r = Region()
        r.read['ecx'] = 1 if read_c else None
        r.read['edx'] = 1 if read_d else None
        r.rets = set(rets)
        r.deref = deref
        return r

    def sites(n, ecx='loaded', edx='unknown', kind='other', cleanup=None, live=False, own=''):
        out = []
        for k in range(n):
            s = Site()
            s.addr, s.ecx, s.edx, s.ecx_kind, s.cleanup, s.pushes = 0x5000 + k, ecx, edx, kind, \
                cleanup, 0
            s.live_ecx = live
            s.own = own
            out.append(s)
        return out

    def d(*a, **k):
        x = decide(*a, **k)
        return (x.conv, x.ecx, x.edx) if x.conv else ('?',)

    check('thiscall', d(reg_of(True, deref=True), sites(2), 'X'), ('__thiscall', 'used', None))
    check('edx read', d(reg_of(True, True), sites(2, edx='loaded'), 'X'),
          ('__fastcall', 'used', 'used'))
    check('edx read, ecx not', d(reg_of(False, True), sites(2, edx='loaded'), 'X'),
          ('__fastcall', 'unused', 'used'))
    check('edx loaded by all', d(reg_of(True), sites(2, edx='loaded'), 'X'),
          ('__fastcall', 'used', 'unused'))
    check('mixed edx is reported', d(reg_of(True), sites(2) + sites(1, edx='loaded'), 'X'),
          ('?',))
    check('edx read, none loads', d(reg_of(False, True), sites(2, edx='not'), 'X'), ('?',))
    check('integer in ecx', d(reg_of(True), sites(2, kind='const'), 'X'),
          ('__fastcall', 'used', None))
    check('a class-typed parameter is an object', d(reg_of(True), [], 'X', typed=True),
          ('__thiscall', 'used', None))
    check('typed, but a caller loads a constant', d(reg_of(True), sites(1, kind='const'), 'X',
                                                    typed=True), ('__fastcall', 'used', None))
    check('a char pointer is a string', d(reg_of(True, deref=True), [], 'X', text=True),
          ('__fastcall', 'used', None))
    check('a variadic function is __cdecl', d(reg_of(), sites(2), 'X', variadic=True),
          ('__cdecl', None, None))
    check('class name is an object', d(reg_of(True), [], 'C::M'), ('__thiscall', 'used', None))
    check('stdcall', d(reg_of(rets=(8,)), [], 'X'), ('__stdcall', None, None))
    check('cdecl', d(reg_of(), [], 'X'), ('__cdecl', None, None))
    check('every caller loads ecx, body ignores it', d(reg_of(), sites(2), 'X'),
          ('__thiscall', 'unused', None))
    check('most callers load ecx is not every caller',
          d(reg_of(), sites(3) + sites(1, ecx='unknown'), 'X'), ('__cdecl', None, None))
    check('a method of the class passes its own this on',
          d(reg_of(), sites(2) + sites(1, ecx='unknown', own='C'), 'C::f'),
          ('__thiscall', 'unused', None))
    check('a method of another class does not',
          d(reg_of(), sites(2) + sites(1, ecx='unknown', own='D'), 'C::f'),
          ('__cdecl', None, None))
    check('a caller that keeps its own ecx passes none',
          d(reg_of(), sites(2) + sites(1, live=True), 'X'), ('__cdecl', None, None))
    check('no caller: the registers stay', d(reg_of(), [], 'X', current='__fastcall'),
          ('__fastcall', 'kept', 'kept'))
    check('no caller, a thiscall stays', d(reg_of(), [], 'X', current='__thiscall'),
          ('__thiscall', 'kept', None))
    check('no caller, a stack return is read', d(reg_of(rets=(8,)), [], 'X', current='__cdecl'),
          ('__stdcall', None, None))
    check('a Lua C function', d(reg_of(rets=(0,)), [], 'X', current='__cdecl', callback='lua'),
          ('__fastcall', 'used', None))
    check('a console handler', d(reg_of(rets=(0,)), [], 'X', callback='console'),
          ('__fastcall', 'used', 'used'))
    check('a Lua C function that pops', d(reg_of(rets=(4,)), [], 'X', callback='lua'), ('?',))
    check('one caller leaves ecx undefined', d(reg_of(), sites(3) + sites(1, ecx='not'), 'X'),
          ('__cdecl', None, None))
    check('constant in ecx, body ignores it', d(reg_of(), sites(1, kind='const'), 'X'), ('?',))
    check('two rets differ', d(reg_of(rets=(0, 4)), [], 'X'), ('?',))
    check('callee pops and caller cleans', d(reg_of(rets=(8,)), sites(1, ecx='unknown',
                                                                    cleanup=8), 'X'), ('?',))
    check('no return keeps stdcall', d(reg_of(rets=()), [], 'X', current='__stdcall'),
          ('__stdcall', None, None))

    # signatures
    s = {}

    def comp(sig, orig, dec, rets, name='A::b', no_args=False):
        r = reg_of(rets=rets)
        row = {'name': name, 'signature': sig}
        text, _ = compose(row, parse_sig(sig), parse_sig(orig) if orig else None, dec, r, s,
                          no_args)
        return text

    check('a register parameter nothing reads is dropped from a __cdecl',
          comp('void * __fastcall (uint32_t index)', None, Verdict('__cdecl'), (0,), 'f', True),
          'void * __cdecl (void)')
    check('a callback has its registration type',
          comp('uint32_t __thiscall (void *this)', None,
               Verdict('__fastcall', 'used', None, fixed=LUA_CFUNCTION), (0,), 'Script::F'),
          'int __fastcall (lua_State *L)')
    check('a handler keeps its return type',
          comp('uint32_t __cdecl (const char *a, const char *b)', None,
               Verdict('__fastcall', 'used', 'used', fixed=CONSOLE_HANDLER), (0,), 'F'),
          'uint32_t __fastcall (const char *name, const char *args)')
    check('callers load neither register: they are dropped',
          comp('void __cdecl (uint32_t index, uint32_t a)', 'void __fastcall (uint32_t index, '
               'uint32_t unused, uint32_t a)', Verdict('__cdecl'), (0,), 'f'),
          'void __cdecl (uint32_t a)')
    check('the registers stay with their names',
          comp('void __cdecl (uint32_t index, uint32_t unused, uint32_t a)',
               'void __fastcall (uint32_t index, uint32_t unused, uint32_t a)',
               Verdict('__fastcall', 'kept', 'kept'), (4,), 'f'),
          'void __fastcall (uint32_t index, uint32_t unused, uint32_t a)')
    check('a __cdecl keeps its list',
          comp('void * __cdecl (uint32_t index)', None, Verdict('__cdecl'), (0,), 'f'),
          'void * __cdecl (uint32_t index)')

    check('class A: names stay in their stack slots',
          comp('bool __fastcall (C *this, float fraction, bool immediate, uint32_t arg4)',
               'bool __thiscall (C *this, float fraction, bool immediate)',
               Verdict('__thiscall', 'used', None), (8,), 'C::f'),
          'bool __thiscall (C *this, float fraction, bool immediate)')
    check('class B: both registers are parameters',
          comp('void __fastcall (uint64_t summoner)', None,
               Verdict('__fastcall', 'used', 'used'), (8,), 'f'),
          'void __fastcall (uint32_t arg1, uint32_t arg2, uint64_t summoner)')
    check('an unread register is unused',
          comp('void __thiscall (C *this, uint32_t a)', None,
               Verdict('__fastcall', 'unused', 'used'), (4,), 'f'),
          'void __fastcall (uint32_t unused, uint32_t arg2, uint32_t a)')
    check('two unused registers',
          comp('void __cdecl (uint32_t a)', None,
               Verdict('__fastcall', 'unused', 'unused'), (4,), 'f'),
          'void __fastcall (uint32_t unused1, uint32_t unused2, uint32_t a)')
    check('this only in __thiscall',
          comp('void __thiscall (C *this, uint32_t a)', None,
               Verdict('__fastcall', 'used', None), (4,), 'f'),
          'void __fastcall (C *arg1, uint32_t unused, uint32_t a)')
    check('a float stack parameter needs no placeholder',
          comp('void __thiscall (C *this, float a)', None,
               Verdict('__fastcall', 'used', None), (4,), 'f'),
          'void __fastcall (C *arg1, float a)')
    check('argN counts this',
          comp('void __thiscall (C *this, uint32_t arg3)', None,
               Verdict('__thiscall', 'used', None), (4,), 'C::f'),
          'void __thiscall (C *this, uint32_t arg2)')
    check('dropped trailing names',
          comp('void * __fastcall (void *this, uint32_t count)',
               'void * __thiscall (void *this, void **block, uint32_t count)',
               Verdict('__thiscall', 'used', None), (4,), 'f'),
          'void * __thiscall (void *this, void **block)')
    check('stdcall from a thiscall', comp('void __thiscall (C *this, uint32_t a)', None,
                                          Verdict('__stdcall'), (4,), 'f'),
          'void __stdcall (uint32_t a)')
    check('float skips a register', stack_words(split_regs(
        '__fastcall', ['uint32_t a', 'float f', 'void *p', 'uint32_t q'], {})[1], {}), 2)
    check('bare types have no name', param_name('uint32_t'), None)
    check('function pointer name', param_name('void (**begin)(void)'), 'begin')

    print('%d checks failed' % len(ok) if ok else 'all checks passed')
    return 1 if ok else 0


if __name__ == '__main__':
    sys.exit(main())
