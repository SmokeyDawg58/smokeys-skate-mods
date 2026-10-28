import collections
import struct
import sys

SEP = chr(92)


def entries(path):
    with open(path, 'rb') as f:
        head = f.read(16)
        assert head[:4] in (b'BIGF', b'BIG4'), head[:4]
        count, header_size = struct.unpack('>II', head[8:16])
        data = f.read(header_size - 16)
    pos = 0
    out = []
    for _ in range(count):
        off, size = struct.unpack('>II', data[pos:pos + 8])
        pos += 8
        end = data.index(b'\0', pos)
        name = data[pos:end].decode('latin-1')
        pos = end + 1
        out.append((name, off, size))
    return out


def read(path, name):
    for n, off, size in entries(path):
        if n.lower() == name.lower():
            with open(path, 'rb') as f:
                f.seek(off)
                return f.read(size)
    raise KeyError(name)


if __name__ == '__main__':
    es = entries(sys.argv[1])
    ext = collections.Counter(n.rsplit('.', 1)[-1].lower() for n, _, _ in es)
    print(len(es), 'entries', ext.most_common(40))
    folders = collections.Counter()
    for n, _, _ in es:
        parts = n.split(SEP)
        folders[parts[3] if len(parts) > 4 else '(root)'] += 1
    print(len(folders), 'folders')
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 60
    for f, c in sorted(folders.items())[:limit]:
        print(c, f)
