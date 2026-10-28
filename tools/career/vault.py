"""Research port of the repo's AttribSys VLT/BIN reader (skate2-formats attrib.rs
and skate2-core attribute_runtime.rs). Big-endian VLT; BIN starts with StrE.
"""
import struct

DATABASE = 0x0B38846845E9C175
CLASS = 0x2A7895AC4A876152
COLLECTION = 0xAD303B8F42B3307E
MASK = (1 << 64) - 1


def _mix64(a, b, c):
    a = ((a - b - c) & MASK) ^ (c >> 43)
    b = ((b - c - a) & MASK) ^ ((a << 9) & MASK)
    c = ((c - a - b) & MASK) ^ (b >> 8)
    a = ((a - b - c) & MASK) ^ (c >> 38)
    b = ((b - c - a) & MASK) ^ ((a << 23) & MASK)
    c = ((c - a - b) & MASK) ^ (b >> 5)
    a = ((a - b - c) & MASK) ^ (c >> 35)
    b = ((b - c - a) & MASK) ^ ((a << 49) & MASK)
    c = ((c - a - b) & MASK) ^ (b >> 11)
    a = ((a - b - c) & MASK) ^ (c >> 12)
    b = ((b - c - a) & MASK) ^ ((a << 18) & MASK)
    c = ((c - a - b) & MASK) ^ (b >> 22)
    return a, b, c


def hash64(text):
    if not text:
        return 0
    key = text.encode('latin-1') if isinstance(text, str) else text
    a = b = 0xABCDEF0011223344
    c = 0x9E3779B97F4A7C13
    n = len(key)
    while len(key) >= 24:
        a = (a + int.from_bytes(key[:8], 'little')) & MASK
        b = (b + int.from_bytes(key[8:16], 'little')) & MASK
        c = (c + int.from_bytes(key[16:24], 'little')) & MASK
        a, b, c = _mix64(a, b, c)
        key = key[24:]
    c = (c + n) & MASK
    for i, byte in enumerate(key):
        if i < 8:
            a = (a + (byte << (i * 8))) & MASK
        elif i < 16:
            b = (b + (byte << ((i - 8) * 8))) & MASK
        else:
            c = (c + (byte << ((i - 15) * 8))) & MASK
    return _mix64(a, b, c)[2]


def u16(d, o):
    return struct.unpack_from('>H', d, o)[0]


def u32(d, o):
    return struct.unpack_from('>I', d, o)[0]


def u64(d, o):
    return struct.unpack_from('>Q', d, o)[0]


class Vault:
    def __init__(self, vlt, deps):
        """deps: list of (name, bytes) for dependency 1.. in DepN order."""
        self.vlt = vlt
        self.chunks = {}
        off = 0
        while off < len(vlt):
            tag = vlt[off:off + 4]
            size = u32(vlt, off + 4)
            self.chunks[tag] = (off, size)
            off += size
        dep_off, _ = self.chunks[b'DepN']
        count = u64(vlt, dep_off + 8)
        ids = [u64(vlt, dep_off + 16 + i * 8) for i in range(count)]
        offs = [u32(vlt, dep_off + 16 + count * 8 + i * 4) for i in range(count)]
        names_start = dep_off + 16 + count * 12
        self.dep_names = []
        for i in range(1, count):
            s = names_start + offs[i]
            self.dep_names.append(vlt[s:vlt.index(b'\0', s)].decode())
        by_name = dict(deps)
        self.buffers = [vlt] + [by_name[n] for n in self.dep_names]
        exp_off, _ = self.chunks[b'ExpN']
        n = u64(vlt, exp_off + 8)
        self.exports = []
        for i in range(n):
            o = exp_off + 16 + i * 24
            self.exports.append((u64(vlt, o), u64(vlt, o + 8), u32(vlt, o + 16), u32(vlt, o + 20)))
        ptr_off, ptr_size = self.chunks[b'PtrN']
        count = (ptr_size - 8 - 24) // 16
        self.pointers = {}
        target = None
        for i in range(count):
            o = ptr_off + 8 + i * 16
            delta = u32(vlt, o)
            enc = u32(vlt, o + 4)
            kind, dep = enc >> 16, enc & 0xFFFF
            arg = u64(vlt, o + 8)
            if kind == 2:
                target = dep
                continue
            if kind == 1:
                self.pointers[(target, delta)] = None
            elif kind == 3:
                self.pointers[(target, delta)] = ('direct', dep, arg)
            elif kind == 4:
                self.pointers[(target, delta)] = ('export', dep, arg)
        self.classes = {}
        self.collections = {}
        self.database = None
        for oid, tid, size, doff in self.exports:
            if tid == CLASS:
                self.classes[oid] = self._class(doff)
            elif tid == COLLECTION:
                col = self._collection(doff, size)
                self.collections[(col['class'], col['id'])] = col
            elif tid == DATABASE:
                self.database = (doff, size)

    def ptr(self, dep, offset):
        return self.pointers.get((dep, offset), 'missing')

    def _class(self, doff):
        v = self.vlt
        class_id = u64(v, doff)
        ndef = u32(v, doff + 12)
        p = self.ptr(0, doff + 16)
        defs = {}
        if isinstance(p, tuple) and p[0] == 'direct':
            buf = self.buffers[p[1]]
            for i in range(ndef):
                o = p[2] + i * 24
                key = u64(buf, o)
                defs[key] = dict(key=key, type=u64(buf, o + 8), offset=u16(buf, o + 16),
                                 size=u16(buf, o + 18), count=u16(buf, o + 20),
                                 flags=buf[o + 22], align=buf[o + 23])
        return dict(id=class_id, defs=defs, layout_size=u32(v, doff + 28))

    def _collection(self, doff, size):
        v = self.vlt
        entry_count = u32(v, doff + 32)
        types_len = u16(v, doff + 38)
        types = [u64(v, doff + 48 + i * 8) for i in range(types_len)]
        entries = {}
        base = doff + 48 + types_len * 8
        for i in range(entry_count):
            o = base + i * 16
            key = u64(v, o)
            entries[key] = dict(key=key, word=u32(v, o + 8), type_index=u16(v, o + 12),
                                node_flags=v[o + 14], entry_flags=v[o + 15], slot=o + 8,
                                type=types[u16(v, o + 12)] if u16(v, o + 12) < len(types) else None)
        return dict(id=u64(v, doff), cls=None, parent=u64(v, doff + 16), class_=u64(v, doff + 8),
                    **{'class': u64(v, doff + 8)}, layout_slot=doff + 40,
                    layout_word=u32(v, doff + 40), entries=entries)

    def layout(self, col):
        p = self.ptr(0, col['layout_slot'])
        if isinstance(p, tuple) and p[0] == 'direct':
            return p[1], p[2]
        return None

    def node(self, class_id, col_id, key):
        """Collection node search through parents, then class layout node."""
        col = self.collections.get((class_id, col_id))
        cls = self.classes.get(class_id)
        cur = col
        while cur is not None:
            if key in cur['entries']:
                return ('entry', cur, cur['entries'][key])
            if cur['parent'] == 0:
                break
            cur = self.collections.get((class_id, cur['parent']))
        d = cls['defs'].get(key) if cls else None
        if d and d['flags'] & 2 and not d['flags'] & 8 and col is not None and self.layout(col):
            return ('layout', col, d)
        return None

    def data(self, class_id, col_id, key):
        """Returns (bytes, where) for scalar data, following the runtime rules."""
        found = self.node(class_id, col_id, key)
        if not found:
            return None
        kind, col, n = found
        d = self.classes[class_id]['defs'].get(key)
        if d is None:
            return None
        size = d['size']
        if kind == 'layout':
            dep, off = self.layout(col)
            return self.buffers[dep][off + d['offset']:off + d['offset'] + size], 'layout'
        flags = n['node_flags'] | 0x80
        if flags & 0x40:
            return struct.pack('>I', n['word'])[:size], 'inline'
        if flags & 0x10:
            lay = self.layout(col)
            if not lay:
                return None
            dep, off = lay
            return self.buffers[dep][off + n['word']:off + n['word'] + size], 'layout-rel'
        p = self.ptr(0, n['slot'])
        if isinstance(p, tuple) and p[0] == 'direct':
            buf = self.buffers[p[1]]
            count = d['count'] if (flags & 2) else 1
            return buf[p[2]:p[2] + size * max(count, 1)], 'stored'
        return None, 'null'

    def text_at(self, dep, offset):
        p = self.ptr(dep, offset)
        if isinstance(p, tuple) and p[0] == 'direct':
            buf = self.buffers[p[1]]
            end = buf.index(b'\0', p[2])
            return buf[p[2]:end].decode('latin-1')
        return None
