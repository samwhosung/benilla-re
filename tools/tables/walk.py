"""A small interpreter for the straight integer code around a call, for the generators that read
the arguments of registrations a loop or a table makes.

It runs the listing's instructions from a function's start with concrete registers and a byte
memory. A value it cannot know (the result of a call it does not model, a byte of a stack buffer
nothing wrote, x87 state) is `None`; arithmetic on `None` gives `None`, and a branch on a flag
that is `None` stops the run (`Unknown`). Reads outside the stack come from the image, unless an
earlier store in the run wrote the place (then the value is whatever was stored). The image's
zero-filled tail (`.bss`) reads as unknown: another function may have written it. A call the walk
does not model returns `None` in eax, ecx and edx and may write through any pointer it was given,
so the caller's stack (above the entry `esp`) and the 0x400 bytes at each pointer among its
arguments (the register arguments and the first four stack dwords) become unknown; the callee
pops its own `ret N`, read from the first `ret` inside its function row, and a callee with no row,
no `ret` in its row or rets that disagree stops the run.

Modelled: the instructions below, `Image` reads, and one call, the bounded string append
`0x64a760` (destination, source, destination size; `ret 0xc`).
"""

import re

MASK = 0xFFFFFFFF
REGS = ('eax', 'ecx', 'edx', 'ebx', 'esp', 'ebp', 'esi', 'edi')
LOW8 = {'al': 'eax', 'cl': 'ecx', 'dl': 'edx', 'bl': 'ebx'}
SIZES = {'DWORD': 4, 'WORD': 2, 'BYTE': 1}
STACK_TOP = 0x1000000
APPEND = 0x64A760


class Unknown(Exception):
    pass


def split_ops(ops):
    depth, cur, out = 0, '', []
    for ch in ops:
        if ch == '[':
            depth += 1
        elif ch == ']':
            depth -= 1
        if ch == ',' and depth == 0:
            out.append(cur.strip())
            cur = ''
        else:
            cur += ch
    out.append(cur.strip())
    return out


class Walk:
    def __init__(self, dis, img, start, fns):
        self.dis, self.img, self.fns = dis, img, fns
        self.reg = {r: None for r in REGS}
        self.reg['esp'] = STACK_TOP
        self.flags = {'zf': None, 'sf': None, 'cf': None, 'of': None}
        self.mem = {}          # address -> byte or None, for every store of the run
        self.pc = start
        self.reads = set()     # image addresses an instruction read, as operands
        self.steps = 0
        self.hooks = {}        # call target -> function(walk)
        self.pops = {}         # call target -> bytes the callee pops
        self.clobbered = []    # (lo, hi) image ranges an unmodelled call may have written
        data = img.sections.get('.data')
        self.bss = (data[0] + min(data[2], data[1] - data[0]), data[1]) if data else (0, 0)

    # --- values -------------------------------------------------------------------------
    def addr_of(self, expr):
        """The value of a memory expression's address, or None."""
        expr = expr.strip()
        m = re.fullmatch(r'(?:\w+ PTR )?(?:ds:)?0x([0-9a-f]+)', expr)
        if m:
            return int(m.group(1), 16)
        inner = re.fullmatch(r'(?:\w+ PTR )?(?:ds:)?\[(.*)\]', expr)
        if not inner:
            raise Unknown('operand %r' % expr)
        total = 0
        for sign, term in re.findall(r'([+-]?)([^+-]+)', inner.group(1).replace(' ', '')):
            if re.fullmatch(r'0x[0-9a-f]+', term):
                v = int(term, 16)
            elif re.fullmatch(r'e[a-z]{2}', term) and term in self.reg:
                v = self.reg[term]
            elif re.fullmatch(r'e[a-z]{2}\*\d', term):
                r, s = term.split('*')
                v = None if self.reg[r] is None else self.reg[r] * int(s)
            else:
                raise Unknown('term %r' % term)
            if v is None:
                return None
            total += -v if sign == '-' else v
        return total & MASK

    def read_mem(self, a, size):
        if a is None:
            return None
        v = 0
        for i in range(size):
            if a + i not in self.mem and self.img.in_image(a + i):
                self.reads.add(a + i)
            b = self.read_byte(a + i)
            if b is None:
                return None
            v |= b << (8 * i)
        return v

    def read_byte(self, a):
        if a in self.mem:
            return self.mem[a]
        if a >= STACK_TOP - 0x100000:
            return None
        if not self.img.in_image(a) or self.bss[0] <= a < self.bss[1]:
            return None
        if any(lo <= a < hi for lo, hi in self.clobbered):
            return None
        return self.img.u8(a)

    def write_mem(self, a, size, v):
        if a is None:
            raise Unknown('store through an unknown address')
        for i in range(size):
            self.mem[a + i] = None if v is None else (v >> (8 * i)) & 0xFF

    def width(self, ops):
        for w, n in SIZES.items():
            if re.search(r'\b' + w + ' PTR', ops):
                return n
        return None

    def get(self, op, size=4):
        op = op.strip()
        if op in self.reg:
            return self.reg[op]
        if op in LOW8:
            v = self.reg[LOW8[op]]
            return None if v is None else v & 0xFF
        if re.fullmatch(r'0x[0-9a-f]+', op):
            return int(op, 16)
        return self.read_mem(self.addr_of(op), size)

    def put(self, op, v, size=4):
        op = op.strip()
        if op in self.reg:
            self.reg[op] = None if v is None else v & MASK
        elif op in LOW8:
            r = LOW8[op]
            old = self.reg[r]
            self.reg[r] = None if v is None or old is None else (old & ~0xFF) | (v & 0xFF)
        else:
            self.write_mem(self.addr_of(op), size, v)

    def push(self, v):
        self.reg['esp'] = (self.reg['esp'] - 4) & MASK
        self.write_mem(self.reg['esp'], 4, v)

    def pop(self):
        v = self.read_mem(self.reg['esp'], 4)
        self.reg['esp'] = (self.reg['esp'] + 4) & MASK
        return v

    def cstring(self, a, cap=256):
        """The NUL-terminated text at a, or None if a byte is unknown or not printable."""
        if a is None:
            return None
        out = bytearray()
        for i in range(cap):
            b = self.read_byte(a + i)
            if b is None:
                return None
            if b == 0:
                return out.decode('ascii')
            if not 32 <= b < 127:
                return None
            out.append(b)
        return None

    # --- flags --------------------------------------------------------------------------
    def set_flags(self, result, a=None, b=None, op=None):
        """Flags of a 32-bit result; a narrower result is passed shifted up to the top of 32 bits."""
        if result is None:
            self.flags = dict.fromkeys(self.flags)
            return
        r = result & MASK
        self.flags['zf'] = r == 0
        self.flags['sf'] = bool(r >> 31)
        if op == 'sub' and a is not None and b is not None:
            self.flags['cf'] = a < b
            self.flags['of'] = bool(((a ^ b) & (a ^ r)) >> 31)
        elif op == 'add' and a is not None and b is not None:
            self.flags['cf'] = a + b > MASK
            self.flags['of'] = bool((~(a ^ b) & (a ^ r)) >> 31)
        elif op == 'logic':
            self.flags['cf'] = False
            self.flags['of'] = False
        elif op == 'incdec':
            pass
        else:
            self.flags['cf'] = self.flags['of'] = None

    def condition(self, cc):
        f = self.flags
        z, s, c, o = f['zf'], f['sf'], f['cf'], f['of']

        def need(*vals):
            if any(v is None for v in vals):
                raise Unknown('branch on an unknown flag')
        table = {
            'e': lambda: (need(z), z)[1], 'z': lambda: (need(z), z)[1],
            'ne': lambda: (need(z), not z)[1], 'nz': lambda: (need(z), not z)[1],
            'b': lambda: (need(c), c)[1], 'c': lambda: (need(c), c)[1],
            'ae': lambda: (need(c), not c)[1], 'nb': lambda: (need(c), not c)[1],
            'be': lambda: (need(c, z), c or z)[1], 'a': lambda: (need(c, z), not (c or z))[1],
            'l': lambda: (need(s, o), s != o)[1], 'ge': lambda: (need(s, o), s == o)[1],
            'le': lambda: (need(z, s, o), z or s != o)[1],
            'g': lambda: (need(z, s, o), not z and s == o)[1],
            's': lambda: (need(s), s)[1], 'ns': lambda: (need(s), not s)[1],
        }
        if cc not in table:
            raise Unknown('condition j%s' % cc)
        return table[cc]()

    # --- the run ------------------------------------------------------------------------
    def callee_pops(self, target):
        """The bytes the callee at `target` pops: the operand of the `ret`s in its function row,
        which must exist, hold a `ret` and agree."""
        if target in self.pops:
            return self.pops[target]
        row = self.fns.at.get(target)
        k = self.dis.at.get(target)
        if row is None or k is None:
            raise Unknown('callee 0x%x has no function row' % target)
        end, pops = target + row['size'], set()
        for j in range(k, len(self.dis.ins)):
            at, _, mn, ops = self.dis.ins[j]
            if at >= end:
                break
            if mn == 'ret':
                pops.add(int(ops, 16) if ops else 0)
        if len(pops) != 1:
            raise Unknown('callee 0x%x pops %s' % (target, sorted(pops) or 'nothing known'))
        self.pops[target] = pops.pop()
        return self.pops[target]

    def clobber(self, extra_pointers):
        """What a call the walk does not model may have changed."""
        for a in [a for a in self.mem if a >= STACK_TOP]:
            self.mem[a] = None
        for p in extra_pointers:
            if p is None or not (self.img.in_image(p) or STACK_TOP - 0x100000 <= p < STACK_TOP + 0x100000):
                continue
            for a in [a for a in self.mem if p <= a < p + 0x400]:
                self.mem[a] = None
            self.clobbered.append((p, p + 0x400))

    def run(self, on_call, limit=400000):
        """Run to the function's `ret`; `on_call(walk, site, target)` sees every call and
        returns True when it handled it. Raises Unknown when the run cannot go on."""
        while True:
            self.steps += 1
            if self.steps > limit:
                raise Unknown('step limit')
            k = self.dis.at.get(self.pc)
            if k is None:
                raise Unknown('no instruction at 0x%x' % self.pc)
            a, size, mn, ops = self.dis.ins[k]
            nxt = self.dis.ins[k + 1][0] if k + 1 < len(self.dis.ins) else a + size
            self.pc = nxt
            if mn == 'nop' or mn.startswith('nop'):
                continue
            if mn == 'ret':
                return
            if mn == 'call':
                if not re.fullmatch(r'0x[0-9a-f]+', ops):
                    raise Unknown('indirect call at 0x%x' % a)
                target = int(ops, 16)
                if on_call(self, a, target):
                    continue
                if target == APPEND:
                    esp = self.reg['esp']
                    dst, src, cap = (self.read_mem(esp + 4 * i, 4) for i in range(3))
                    cur, add = self.cstring(dst), self.cstring(src)
                    if cur is None or add is None or cap is None:
                        raise Unknown('string append at 0x%x' % a)
                    text = (cur + add)[:max(cap - 1, 0)].encode('ascii') + b'\0'
                    for i, b in enumerate(text):
                        self.mem[dst + i] = b
                    self.reg['eax'] = dst
                else:
                    esp = self.reg['esp']
                    self.clobber([self.reg['ecx'], self.reg['edx']]
                                 + [self.read_mem(esp + 4 * i, 4) for i in range(4)])
                    self.reg['eax'] = None
                self.reg['ecx'] = self.reg['edx'] = None
                self.reg['esp'] = (self.reg['esp'] + self.callee_pops(target)) & MASK
                continue
            if mn == 'jmp':
                if not re.fullmatch(r'0x[0-9a-f]+', ops):
                    raise Unknown('indirect jmp at 0x%x' % a)
                self.pc = int(ops, 16)
                continue
            if mn.startswith('j'):
                if self.condition(mn[1:]):
                    self.pc = int(ops, 16)
                continue
            self.step(mn, split_ops(ops), a)

    def step(self, mn, ops, at):
        if mn == 'push':
            self.push(self.get(ops[0]))
        elif mn == 'pop':
            self.put(ops[0], self.pop())
        elif mn == 'mov':
            size = self.width(ops[0]) or self.width(ops[1]) or 4
            self.put(ops[0], self.get(ops[1], size), size)
        elif mn == 'movzx':
            self.put(ops[0], self.get(ops[1], self.width(ops[1]) or 1))
        elif mn == 'lea':
            self.put(ops[0], self.addr_of(ops[1]))
        elif mn in ('add', 'sub', 'and', 'or', 'xor', 'cmp', 'test'):
            size = self.width(ops[0]) or self.width(ops[1]) or (1 if ops[0] in LOW8 else 4)
            if mn == 'xor' and ops[0] == ops[1]:
                self.put(ops[0], 0)
                self.set_flags(0, op='logic')
                return
            a, b = self.get(ops[0], size), self.get(ops[1], size)
            if a is None or b is None:
                r = None
            else:
                r = {'add': a + b, 'sub': a - b, 'cmp': a - b, 'and': a & b, 'test': a & b,
                     'or': a | b, 'xor': a ^ b}[mn]
            if size != 4 and r is not None:
                r &= (1 << (8 * size)) - 1
            if size == 4 or r is None:
                self.set_flags(r, a, b, {'add': 'add', 'sub': 'sub', 'cmp': 'sub'}.get(mn, 'logic'))
            else:
                self.set_flags(r << (32 - 8 * size), None if a is None else a << (32 - 8 * size),
                               None if b is None else b << (32 - 8 * size),
                               {'add': 'add', 'sub': 'sub', 'cmp': 'sub'}.get(mn, 'logic'))
            if mn not in ('cmp', 'test'):
                self.put(ops[0], r, size)
        elif mn in ('inc', 'dec'):
            size = self.width(ops[0]) or (1 if ops[0] in LOW8 else 4)
            bits = 8 * size
            a = self.get(ops[0], size)
            r = None if a is None else (a + (1 if mn == 'inc' else -1)) & ((1 << bits) - 1)
            saved = self.flags['cf']
            if r is None:
                self.flags = dict.fromkeys(self.flags)
            else:
                self.flags['zf'] = r == 0
                self.flags['sf'] = bool(r >> (bits - 1))
                self.flags['of'] = a == ((1 << (bits - 1)) - 1 if mn == 'inc' else 1 << (bits - 1))
            self.flags['cf'] = saved
            self.put(ops[0], r, size)
        elif mn in ('shl', 'shr', 'sar'):
            size = self.width(ops[0]) or (1 if ops[0] in LOW8 else 4)
            bits = 8 * size
            a = self.get(ops[0], size)
            n = self.get(ops[1])
            if n is not None:
                n &= 31
            if n == 0:
                return
            if a is None or n is None:
                r = None
            elif mn == 'shl':
                r = (a << n) & ((1 << bits) - 1)
            elif mn == 'shr':
                r = a >> n
            else:
                r = ((a - (1 << bits) if a >> (bits - 1) else a) >> n) & ((1 << bits) - 1)
            self.put(ops[0], r, size)
            self.set_flags(None if r is None else r << (32 - bits))
        elif mn in ('fld', 'fstp', 'fst', 'fild', 'fmul', 'fdiv', 'fadd', 'fsub', 'fxch', 'fnstsw',
                    'fldcw', 'fnstcw'):
            if mn in ('fstp', 'fst') and 'PTR' in ops[0]:
                self.write_mem(self.addr_of(ops[0]), self.width(ops[0]) or 4, None)
        else:
            raise Unknown('%s %s at 0x%x' % (mn, ', '.join(ops), at))
