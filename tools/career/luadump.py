"""Lua 5.1 bytecode reader for Skate 2's embedded chunks.

Header: ESC 'Lua' 0x51, format 0, little-endian, int/size_t/Instruction 4 bytes,
lua_Number 4-byte float (integral flag 0).
"""
import collections
import struct
import sys

HEADER = bytes.fromhex('1b4c75615100010404040400')
OPS = ['MOVE', 'LOADK', 'LOADBOOL', 'LOADNIL', 'GETUPVAL', 'GETGLOBAL', 'GETTABLE',
       'SETGLOBAL', 'SETUPVAL', 'SETTABLE', 'NEWTABLE', 'SELF', 'ADD', 'SUB', 'MUL',
       'DIV', 'MOD', 'POW', 'UNM', 'NOT', 'LEN', 'CONCAT', 'JMP', 'EQ', 'LT', 'LE',
       'TEST', 'TESTSET', 'CALL', 'TAILCALL', 'RETURN', 'FORLOOP', 'FORPREP',
       'TFORLOOP', 'SETLIST', 'CLOSE', 'CLOSURE', 'VARARG']


class Reader:
    def __init__(self, data, pos):
        self.d = data
        self.p = pos

    def u8(self):
        v = self.d[self.p]
        self.p += 1
        return v

    def i32(self):
        v = struct.unpack_from('<i', self.d, self.p)[0]
        self.p += 4
        return v

    def u32(self):
        v = struct.unpack_from('<I', self.d, self.p)[0]
        self.p += 4
        return v

    def f32(self):
        v = struct.unpack_from('<f', self.d, self.p)[0]
        self.p += 4
        return v

    def string(self):
        n = self.u32()
        if n == 0:
            return None
        s = self.d[self.p:self.p + n - 1]
        self.p += n
        return s.decode('latin-1')


class Proto:
    pass


def read_function(r):
    f = Proto()
    f.source = r.string()
    f.line, f.lastline = r.i32(), r.i32()
    f.nups, f.numparams, f.is_vararg, f.maxstack = r.u8(), r.u8(), r.u8(), r.u8()
    f.code = [r.u32() for _ in range(r.i32())]
    f.consts = []
    for _ in range(r.i32()):
        t = r.u8()
        if t == 0:
            f.consts.append(None)
        elif t == 1:
            f.consts.append(bool(r.u8()))
        elif t == 3:
            f.consts.append(r.f32())
        elif t == 4:
            f.consts.append(r.string())
        else:
            raise ValueError('constant type %d at %x' % (t, r.p))
    f.protos = [read_function(r) for _ in range(r.i32())]
    f.lineinfo = [r.i32() for _ in range(r.i32())]
    f.locvars = []
    for _ in range(r.i32()):
        f.locvars.append((r.string(), r.i32(), r.i32()))
    f.upnames = [r.string() for _ in range(r.i32())]
    return f


def read_chunk(data, pos):
    assert data[pos:pos + 12] == HEADER, data[pos:pos + 12].hex()
    r = Reader(data, pos + 12)
    f = read_function(r)
    return f, r.p


def decode(i):
    op = i & 0x3F
    a = (i >> 6) & 0xFF
    c = (i >> 14) & 0x1FF
    b = (i >> 23) & 0x1FF
    bx = i >> 14
    return op, a, b, c, bx, bx - 131071


def walk(f):
    yield f
    for p in f.protos:
        yield from walk(p)


def chunks(data):
    pos = 0
    while True:
        i = data.find(HEADER, pos)
        if i < 0:
            return
        f, end = read_chunk(data, i)
        yield i, end, f
        pos = end


if __name__ == '__main__':
    data = open(sys.argv[1], 'rb').read()
    globals_read = collections.Counter()
    globals_written = collections.Counter()
    field_calls = collections.Counter()
    ops = collections.Counter()
    n = 0
    instr = 0
    for start, end, main in chunks(data):
        n += 1
        for f in walk(main):
            instr += len(f.code)
            for pc, ins in enumerate(f.code):
                op, a, b, c, bx, sbx = decode(ins)
                ops[OPS[op]] += 1
                if op == 5:
                    globals_read[f.consts[bx]] += 1
                elif op == 7:
                    globals_written[f.consts[bx]] += 1
                elif op in (6, 11) and c >= 256:
                    k = f.consts[c - 256]
                    if isinstance(k, str):
                        field_calls[k] += 1
    print('chunks', n, 'instructions', instr)
    print('ops', ops.most_common())
    print('globals written (top):', [g for g, _ in globals_written.most_common(10)])
    print('GLOBALS READ (%d):' % len(globals_read))
    for g, c in sorted(globals_read.items(), key=lambda x: -x[1]):
        if not (g or '').startswith('h_'):
            print('  %6d %s' % (c, g))
    print('FIELDS (%d), top 150:' % len(field_calls))
    for g, c in field_calls.most_common(150):
        print('  %6d %s' % (c, g))
