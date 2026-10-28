"""Read-only dump of a SKATER.P: savable order and the SignPost leaves."""
import struct
import sys

path = sys.argv[1]
b = open(path, 'rb').read()


def r32(buf, off):
    return struct.unpack_from('>I', buf, off)[0]


def name_hash(name):
    h = 0xFFFFFFFF
    for c in name.encode():
        h = (h * 33 + c) & 0xFFFFFFFF
    return h


def split(buf):
    out = []
    pos = 0
    while pos < len(buf):
        size = r32(buf, pos + 8)
        ln = 16 + ((size + 3) & ~3)
        out.append(buf[pos:pos + ln])
        pos += ln
    return out


names = ['SignPost', 'SKATER_PROFILE', 'CAC', 'UNLOCKABLES', 'SK82_PC', 'STATS', 'Stats', 'MUSIC', 'PHOTO', 'Photos',
         'cSignPost::Data', 'cSignPost::Position', 'cSignPostMessageMan::Data', 'Sponsor', 'SPONSOR', 'GAMEPLAY',
         'ProfileBackEnd::ProfileData', 'ProfileBackEnd::Settings', 'CACBackEnd::Data', 'Achievements', 'ACHIEVEMENTS',
         'IMemcardSavable::SaveMemcardSaveVersion', 'MemcardProfile', 'PROFILE', 'Unlockables', 'UnlockablesBackEnd::Data',
         'StatsBackEnd::Data', 'SessionMarker', 'SESSIONMARKER', 'Replay', 'REPLAY', 'Skater', 'SKATER', 'Filmer', 'VIDEO',
         'BreadCrumb', 'Network', 'NETWORK', 'Leaderboard', 'Tutorial', 'TUTORIAL', 'Options', 'OPTIONS']
known = {name_hash(n): n for n in names}
end = 36 + r32(b, 28)
chunks = split(b[36:end])
for i, c in enumerate(chunks):
    cid = r32(c, 0)
    size = r32(c, 8)
    print('savable', i, hex(cid), known.get(cid, '?'), 'size', size)
    if i == 0:
        continue
    for leaf in split(c[16:]):
        lid = r32(leaf, 0)
        print('   leaf', hex(lid), known.get(lid, '?'), 'version', r32(leaf, 4), 'size', r32(leaf, 8))


def leaf_tuples(savable, leafname):
    for c in chunks[1:]:
        if r32(c, 0) != name_hash(savable):
            continue
        for leaf in split(c[16:]):
            if r32(leaf, 0) != name_hash(leafname):
                continue
            size = r32(leaf, 8)
            t = []
            pos = 16
            while pos + 4 <= min(size, len(leaf)):
                n = r32(leaf, pos)
                if n == 0:
                    break
                t.append(leaf[pos + 4:pos + 4 + n])
                pos += 4 + n
            return r32(leaf, 4), t
    return None, None


ver, t = leaf_tuples('SignPost', 'cSignPost::Data')
if t is None:
    print('no SignPost data')
    sys.exit()
print('cSignPost::Data version', ver, 'tuples', len(t))
i = 0


def take(n):
    global i
    v = t[i:i + n]
    i += n
    return v


def iv(x):
    return struct.unpack('>i', x)[0] if len(x) == 4 else (x[0] if len(x) == 1 else struct.unpack('>Q', x)[0])


sections = [('available', 2048, 2), ('groups', 80, 1), ('completed', 2048, 3), ('group_exits', 80, 2),
            ('group_counts', 80, 2), ('point_pools', 80, 2), ('rankings', 32, 2), ('challenge_save', 512, 3),
            ('dummy', 256, 2), ('blog', 512, 4), ('news', 32, 1)]
for name, count, width in sections:
    rows = [tuple(iv(x) for x in take(width)) for _ in range(count)]
    nonzero = [r for r in rows if any(r)]
    sizes = sorted({tuple(len(x) for x in t[i - width * count + k * width:i - width * count + k * width + width]) for k in range(min(count, 3))})
    print(name, 'nonzero', len(nonzero), 'tuple sizes', sizes)
    for r in nonzero[:12]:
        print('    ', r)
print('consumed', i, 'of', len(t))
ver, t2 = leaf_tuples('SignPost', 'cSignPost::Position')
print('Position version', ver, [x.hex() for x in (t2 or [])][:13])
ver, t3 = leaf_tuples('SignPost', 'cSignPostMessageMan::Data')
if t3:
    print('MessageMan version', ver, 'tuples', len(t3), 'first', iv(t3[0]), 'count', iv(t3[1]))
    for k in range(min(iv(t3[1]), 200)):
        print('    ', hex(iv(t3[2 + 2 * k])), iv(t3[3 + 2 * k]) & 0xFFFFFFFF)
