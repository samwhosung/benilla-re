"""Shared helpers for the table generators and the tools beside them: the exe check, a PE reader,
the disassembly reader and the function rows.

Standard library only. Every generator takes the same options:

    --exe PATH      the client (default WoW/WoW.exe at the repo root)
    --disasm PATH   the disassembly (default dumps/disasm.txt at the repo root), for the
                    generators that read instructions
    --tables DIR    where a generator that reads other tables finds them (default tables/)
    -o PATH         write the table there instead of standard output

The exe must be build 5875, by the size and SHA-256 in tools/exe.env. Another build is refused.
"""

import argparse
import bisect
import csv
import hashlib
import re
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent


def _identity():
    """EXE_SIZE and EXE_SHA256 from tools/exe.env, the one place every tool reads them."""
    values = {}
    for line in (ROOT / 'tools' / 'exe.env').read_text(encoding='utf-8').splitlines():
        if line and not line.startswith('#'):
            key, _, value = line.partition('=')
            values[key] = value
    return int(values['EXE_SIZE']), values['EXE_SHA256']


EXE_SIZE, EXE_SHA256 = _identity()
IMAGE_BASE = 0x400000


def addr(v):
    """An address as the tables write it: 0x and six lowercase hex digits."""
    return '0x%06x' % v


class Image:
    """The exe as a PE: the raw file, its sections, and the memory-mapped image."""

    def __init__(self, path):
        data = Path(path).read_bytes()
        if len(data) != EXE_SIZE or hashlib.sha256(data).hexdigest() != EXE_SHA256:
            sys.exit('%s is not WoW.exe build 5875 (size %d, sha256 %s)'
                     % (path, len(data), hashlib.sha256(data).hexdigest()))
        self.data = data
        pe = struct.unpack_from('<I', data, 0x3C)[0]
        nsec = struct.unpack_from('<H', data, pe + 6)[0]
        opt = struct.unpack_from('<H', data, pe + 20)[0]
        self.base = struct.unpack_from('<I', data, pe + 24 + 28)[0]
        size = struct.unpack_from('<I', data, pe + 24 + 56)[0]
        self.sections = {}
        self.mem = bytearray(size)
        for i in range(nsec):
            o = pe + 24 + opt + i * 40
            name = data[o:o + 8].rstrip(b'\0').decode('latin1')
            vsize, rva, rsize, rptr = struct.unpack_from('<IIII', data, o + 8)
            self.sections[name] = (self.base + rva, self.base + rva + vsize, rsize)
            n = min(rsize, vsize)
            self.mem[rva:rva + n] = data[rptr:rptr + n]
        self.text = self.sections['.text'][:2]

    def u32(self, va):
        return struct.unpack_from('<I', self.mem, va - self.base)[0]

    def u8(self, va):
        return self.mem[va - self.base]

    def bytes(self, va, n):
        return bytes(self.mem[va - self.base:va - self.base + n])

    def in_image(self, va):
        return self.base <= va < self.base + len(self.mem)

    def cstr(self, va, cap=160):
        """The NUL-terminated printable-ASCII string at va, or None."""
        if not self.in_image(va):
            return None
        o = va - self.base
        e = self.mem.find(b'\0', o, o + cap)
        if e < 0:
            return None
        s = bytes(self.mem[o:e])
        if not all(32 <= c < 127 or c == 9 for c in s):
            return None
        return s.decode('ascii')

    def section_of(self, va):
        """The name of the section that holds va, or None."""
        for name, (lo, hi, _) in self.sections.items():
            if lo <= va < hi:
                return name
        return None

    def import_ranges(self):
        """The import directory's own tables as [lo, hi) address ranges: the descriptors and each
        library's name table and address table. Their dwords are relative addresses of import
        names, which can read as code addresses but never point at code."""
        pe = struct.unpack_from('<I', self.data, 0x3C)[0]
        rva = struct.unpack_from('<I', self.data, pe + 24 + 96 + 8)[0]
        out = []
        d = self.base + rva
        while True:
            names, _, _, _, thunks = struct.unpack_from('<IIIII', self.mem, d - self.base)
            if not names and not thunks:
                out.append((self.base + rva, d + 20))
                break
            for t in (names, thunks):
                if t:
                    e = self.base + t
                    while self.u32(e):
                        e += 4
                    out.append((self.base + t, e + 4))
            d += 20
        return sorted(out)

    def call_sites(self, target, kinds=(0xE8, 0xE9)):
        """Every direct call (e8) or jmp (e9) rel32 in .text whose target is `target`."""
        lo, hi = self.text
        o = lo - self.base
        out = []
        buf = self.mem
        for i in range(o, hi - self.base - 5):
            if buf[i] in kinds:
                rel = struct.unpack_from('<i', buf, i + 1)[0]
                if self.base + i + 5 + rel == target:
                    out.append(self.base + i)
        return out


class Disasm:
    """The disassembly of .text: instructions in address order.

    `ins` holds (address, size, mnemonic, operands); a prefix (rep, lock) is dropped and the
    mnemonic is the instruction proper. A size runs to the next instruction, so the one before a
    jump table, which the listing leaves out, spans the table. `at` maps an address to its index in `ins`.
    """

    PREFIXES = ('rep', 'repz', 'repnz', 'repe', 'repne', 'lock', 'data16', 'addr16')

    def __init__(self, path):
        raw = []
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
            words = text.split(None, 1)
            mn, ops = words[0], (words[1].strip() if len(words) > 1 else '')
            while mn in self.PREFIXES and ops:
                words = ops.split(None, 1)
                mn, ops = words[0], (words[1].strip() if len(words) > 1 else '')
            raw.append((a, mn, ops))
        if not raw:
            sys.exit('no instructions in %s' % path)
        self.ins = []
        for k, (a, mn, ops) in enumerate(raw):
            nxt = raw[k + 1][0] if k + 1 < len(raw) else a + 1
            self.ins.append((a, nxt - a, mn, ops))
        self.at = {t[0]: k for k, t in enumerate(self.ins)}
        self.exe = None
        self.extra = {}

    def lookup(self, va):
        """The instruction starting exactly at va as (address, size, mnemonic, operands).

        The listing restarts at every function row and jump-table entry, but data no row or table
        accounts for can still swallow the first bytes of the code after it and leave an entry
        mid-instruction. When va is not an instruction start, the bytes from va are disassembled
        afresh with objdump, if it is on the path (the same tool that wrote the dump), and those
        instructions are kept apart from the listing. Without objdump, or without an exe path, the
        answer is None.
        """
        if va in self.extra:
            return self.extra[va]
        k = self.at.get(va)
        if k is not None:
            return self.ins[k]
        if self.exe is None or not self.resync(va):
            return None
        return self.extra.get(va)

    def resync(self, va, span=0x600):
        import shutil
        import subprocess
        tool = shutil.which('i686-w64-mingw32-objdump')
        if not tool:
            return False
        out = subprocess.run(
            [tool, '-d', '-M', 'intel', '--no-show-raw-insn', '--start-address=0x%x' % va,
             '--stop-address=0x%x' % (va + span), self.exe],
            capture_output=True, text=True).stdout
        raw = []
        for line in out.splitlines():
            head, tab, text = line.partition('\t')
            head = head.strip()
            if not tab or not head.endswith(':'):
                continue
            try:
                a = int(head[:-1], 16)
            except ValueError:
                continue
            words = text.strip().split(None, 1)
            if not words:
                continue
            mn, ops = words[0], (words[1].strip() if len(words) > 1 else '')
            while mn in self.PREFIXES and ops:
                words = ops.split(None, 1)
                mn, ops = words[0], (words[1].strip() if len(words) > 1 else '')
            raw.append((a, mn, ops))
        for k in range(len(raw) - 1):
            a, mn, ops = raw[k]
            self.extra[a] = (a, raw[k + 1][0] - a, mn, ops)
        return bool(raw)


class Functions:
    """The rows of symbols/functions.tsv in address order, each a dict of its columns with
    `address` and `size` as integers. A row's extent is [address, address + size)."""

    def __init__(self, path=None):
        path = path or ROOT / 'symbols' / 'functions.tsv'
        with open(path, newline='', encoding='utf-8') as f:
            reader = csv.DictReader(f, delimiter='\t', quoting=csv.QUOTE_NONE)
            self.rows = []
            for r in reader:
                r['address'] = int(r['address'], 16)
                r['size'] = int(r['size'] or 0)
                self.rows.append(r)
        self.rows.sort(key=lambda r: r['address'])
        self.starts = [r['address'] for r in self.rows]
        self.at = {r['address']: r for r in self.rows}

    def holding(self, va):
        """The row whose extent holds va, or None."""
        i = bisect.bisect_right(self.starts, va) - 1
        if i >= 0 and va < self.rows[i]['address'] + self.rows[i]['size']:
            return self.rows[i]
        return None

    @staticmethod
    def label(row):
        """A row as a reader names it: its address, its name where it has one, and its system."""
        if row is None:
            return 'no function row'
        name = (' ' + row['name']) if row['name'] else ''
        return '%s%s (%s)' % (addr(row['address']), name, row['system'])


SIGNAL2 = 0x703F50
_CLEANUP = re.compile(r'^(?:add\s+esp,0x([0-9a-f]+)|lea\s+esp,\[esp\+0x([0-9a-f]+)\])$')
_DIRECTIVE = re.compile(r'%([a-zA-Z])')


def declared(fmt):
    """The dwords a SignalEvent2 format declares: one per directive, two for `%f`."""
    return sum(2 if d == 'f' else 1 for d in _DIRECTIVE.findall(fmt))


def _cleanup(dis, k):
    """The size of the first `add esp,N` or `lea esp,[esp+N]` after instruction k, before any
    other call, jump or return; None if there is none."""
    for j in range(k + 1, min(k + 8, len(dis.ins))):
        _, _, mn, ops = dis.ins[j]
        m = _CLEANUP.match('%s %s' % (mn, ops.replace(' ', '')) if mn == 'add'
                           else '%s %s' % (mn, ops))
        if m:
            return int(m.group(1) or m.group(2), 16)
        if mn in ('call', 'ret') or mn.startswith('j'):
            return None
    return None


def _pushes(dis, k):
    """The pushes walking back from instruction k to the previous call, jump or return."""
    n = 0
    for j in range(k - 1, max(k - 40, -1), -1):
        _, _, mn, _ = dis.ins[j]
        if mn in ('call', 'ret') or mn.startswith('j'):
            break
        if mn == 'push':
            n += 1
    return n


def signal2_passed(dis, k, want):
    """What the caller of the SignalEvent2 call at instruction k passed, as (values, how), or
    None for a tail jump or a site with no count. `want` is the dwords its format declares.

    The first cleanup after the call frees the id, the format and the values, so the values are
    N/4 - 2 dwords. A cleanup larger than the format accounts for is shared with an earlier call
    MSVC did not clean up separately; then the count is the pushes back to the previous call.
    """
    if dis.ins[k][2] == 'jmp':
        return None
    n = _cleanup(dis, k)
    if n is not None and n // 4 - 2 <= want:
        return n // 4 - 2, 'add esp,0x%x' % n
    have = _pushes(dis, k) - 2
    if have < 0:
        return None
    return have, ('%d pushes, cleanup 0x%x shared' % (have + 2, n) if n is not None
                  else '%d pushes, no cleanup' % (have + 2))


def read_tsv(path):
    """The rows of a table as dicts, keyed by its header."""
    with open(path, newline='', encoding='utf-8') as f:
        return list(csv.DictReader(f, delimiter='\t', quoting=csv.QUOTE_NONE))


def parse_args(doc, disasm=False, extra=None):
    ap = argparse.ArgumentParser(description=doc)
    ap.add_argument('--exe', default=str(ROOT / 'WoW' / 'WoW.exe'))
    if disasm:
        ap.add_argument('--disasm', default=str(ROOT / 'dumps' / 'disasm.txt'))
    ap.add_argument('--tables', default=str(ROOT / 'tables'),
                    help='the directory the generators that read other tables take them from')
    ap.add_argument('-o', '--out')
    if extra:
        extra(ap)
    return ap.parse_args()


def write_table(header, rows, out=None):
    """Write the header and rows, tab-separated, newline-terminated."""
    lines = ['\t'.join(header)]
    for r in rows:
        assert len(r) == len(header), r
        for c in r:
            assert '\t' not in str(c) and '\n' not in str(c), r
        lines.append('\t'.join(str(c) for c in r))
    text = '\n'.join(lines) + '\n'
    if out:
        Path(out).write_text(text, newline='\n')
    else:
        sys.stdout.write(text)


def note(msg):
    print(msg, file=sys.stderr)
