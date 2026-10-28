"""Dump collections of a class across the shared database (skater.vlt classes,
records in the other vaults), decoding Text/Bool/Int/Float/RefSpec/Blob fields."""
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from vault import Vault, hash64, u16, u32, u64
from luadump import chunks, walk

DB = os.environ.get('SKATE2_CAREER_DB', os.path.join(HERE, 'db'))
NAMES = os.environ.get('SKATE2_CAREER_NAMES', os.path.join(HERE, 'names.txt'))


def candidate_names():
    """Strings to hash for attribute/class names: BIN strings, Lua constants and an
    optional newline-separated list (PDB identifiers with V/U prefixes stripped)."""
    import re
    names = set()
    for fn in os.listdir(DB):
        data = open(os.path.join(DB, fn), 'rb').read()
        for s in re.findall(rb'[A-Za-z_][A-Za-z0-9_ .:\-]{1,80}', data):
            names.add(s.decode('latin-1'))
        if fn.endswith('.bin'):
            for start, end, main in chunks(data):
                for f in walk(main):
                    for k in f.consts:
                        if isinstance(k, str):
                            names.add(k)
    if os.path.exists(NAMES):
        for line in open(NAMES, encoding='latin-1'):
            if line.strip():
                names.add(line.strip())
    return names
TYPE_TEXT = hash64('EA::Reflection::Text')
TYPE_BOOL = hash64('EA::Reflection::Bool')
TYPE_INT = hash64('EA::Reflection::Int32')
TYPE_UINT = hash64('EA::Reflection::UInt32')
TYPE_FLOAT = hash64('EA::Reflection::Float')
TYPE_REF = hash64('Attrib::RefSpec')
TYPE_BLOB = hash64('Attrib::Blob')


def open_all():
    deps = [(n, open(os.path.join(DB, n), 'rb').read()) for n in os.listdir(DB)]
    vaults = {}
    for n in ('skater', 'ChallengeCore', 'progressioncore'):
        vaults[n] = Vault(open(os.path.join(DB, n + '.vlt'), 'rb').read(), deps)
    classes = {}
    for v in vaults.values():
        classes.update(v.classes)
    for v in vaults.values():
        v.classes = classes
    return vaults, classes


class Db:
    def __init__(self):
        self.vaults, self.classes = open_all()
        names = candidate_names()
        self.names = {hash64(n): n for n in names}
        self.where = {}
        for vn, v in self.vaults.items():
            for key in v.collections:
                self.where[key] = v

    def name(self, h):
        return self.names.get(h, '%016x' % h)

    def collections_of(self, class_name):
        cid = hash64(class_name) if isinstance(class_name, str) else class_name
        return sorted(k[1] for k in self.where if k[0] == cid)

    def raw(self, cid, col, key):
        v = self.where.get((cid, col))
        if v is None:
            return None
        found = v.node(cid, col, key)
        if not found:
            return None
        kind, c, n = found
        d = self.classes[cid]['defs'][key]
        return v, kind, c, n, d

    def value(self, cid, col, key):
        got = self.raw(cid, col, key)
        if got is None:
            return None
        v, kind, c, n, d = got
        t = d['type']
        is_array = d['flags'] & 1 or d['count'] > 1
        # Locate the payload (dependency, offset).
        if kind == 'layout':
            lay = v.layout(c)
            if not lay:
                return None
            dep, off = lay[0], lay[1] + d['offset']
        else:
            flags = n['node_flags'] | 0x80
            if flags & 0x40:
                return self.decode_inline(t, n['word'], d['size'])
            if flags & 0x10:
                lay = v.layout(c)
                if not lay:
                    return None
                dep, off = lay[0], lay[1] + n['word']
            else:
                p = v.ptr(0, n['slot'])
                if not (isinstance(p, tuple) and p[0] == 'direct'):
                    return None
                dep, off = p[1], p[2]
        buf = v.buffers[dep]
        if is_array and kind != 'layout':
            # Attrib::Array: u16 alloc, u16 count, u16 elementSize, u16 pad.
            count = u16(buf, off + 2)
            size = u16(buf, off + 4)
            items = []
            for i in range(count):
                items.append(self.decode_at(v, t, dep, off + 8 + i * size, size))
            return items
        return self.decode_at(v, t, dep, off, d['size'])

    def decode_inline(self, t, word, size):
        b = struct.pack('>I', word)
        if t == TYPE_BOOL:
            return bool(b[0])
        if t == TYPE_INT:
            return struct.unpack('>i', b)[0]
        if t == TYPE_UINT:
            return word
        if t == TYPE_FLOAT:
            return struct.unpack('>f', b)[0]
        return b[:size].hex()

    def decode_at(self, v, t, dep, off, size):
        buf = v.buffers[dep]
        if t == TYPE_TEXT:
            return v.text_at(dep, off)
        if t == TYPE_BOOL:
            return bool(buf[off])
        if t == TYPE_INT:
            return struct.unpack_from('>i', buf, off)[0]
        if t == TYPE_UINT:
            return u32(buf, off)
        if t == TYPE_FLOAT:
            return struct.unpack_from('>f', buf, off)[0]
        if t == TYPE_REF:
            return ('ref', self.name(u64(buf, off)), self.name(u64(buf, off + 8)))
        if t == TYPE_BLOB:
            p = v.ptr(dep, off + 4)
            if isinstance(p, tuple) and p[0] == 'direct':
                return ('blob', p[1], p[2], u32(buf, off))
            return ('blob?', u32(buf, off), u32(buf, off + 4))
        return buf[off:off + size].hex()

    def record(self, class_name, col):
        cid = hash64(class_name) if isinstance(class_name, str) else class_name
        out = {}
        for key, d in self.classes[cid]['defs'].items():
            try:
                val = self.value(cid, col, key)
            except Exception as error:  # noqa: BLE001
                val = 'ERR %s' % error
            if val is not None:
                out[self.name(key)] = val
        return out


if __name__ == '__main__':
    db = Db()
    cls = sys.argv[1]
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 20
    cols = db.collections_of(cls)
    print(cls, len(cols), 'collections')
    for col in cols[:limit]:
        print('-', db.name(col), db.record(cls, col))
