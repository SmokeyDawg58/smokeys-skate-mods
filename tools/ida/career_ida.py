"""Read-only IDA helpers used for the career/challenge research (docs/career).

Load inside an IDAPython session:
    import sys, importlib; sys.path.insert(0, r"<repo>/tools/ida"); import career_ida; importlib.reload(career_ida)

dec(ea)      cleaned Hex-Rays text. FailAssert/DebugBreak lines are dropped, so an
             assert shows as an empty "if"; check disasm(ea) before trusting one.
disasm(ea)   instruction lines of the function at ea.
keys64(ea)   64-bit attribute/collection keys built with lis/ori + insrdi:
             [(address, '0xHIGH_LOW')]. IDA shows such keys as split signed words.
find([...])  functions whose names contain every substring.
show_type / members   IDB struct and enum layouts.
"""
import re

import ida_funcs
import ida_hexrays
import idautils
import idc

_DECL = re.compile(r'^  [\w:<>,\s\*\[\]&()`\']+; // (r\d+|f\d+|fp\d+|\[sp|cr\d+|v\d+|ctr|lr)')
_NOISE = re.compile(r'FailAssert|DebugBreak|checkouts|^\s*nullptr,\s*$|^\s*\d+\);\s*$')


def dec(ea, keep_decls=False):
    f = ida_funcs.get_func(ea)
    if f is None:
        return '<no function at %x>' % ea
    text = str(ida_hexrays.decompile(f.start_ea))
    text = re.sub(r' /\*0x[0-9a-f]+\*/', '', text)
    out = []
    for line in text.splitlines():
        if not keep_decls and _DECL.match(line):
            continue
        if _NOISE.search(line):
            continue
        out.append(line)
    text = '\n'.join(out)
    text = re.sub(r'\n\s*\{\s*\n\s*\}', '', text)
    return text


def find(substrings, exclude=()):
    hits = []
    for ea in idautils.Functions():
        name = idc.get_func_name(ea)
        if not name:
            continue
        if all(s in name for s in substrings) and not any(x in name for x in exclude):
            hits.append((ea, name))
    return hits


def decs(substrings, exclude=(), limit=20):
    out = []
    for ea, name in find(substrings, exclude)[:limit]:
        out.append('===== %x %s' % (ea, name[:150]))
        out.append(dec(ea))
    return '\n'.join(out)


def disasm(start, end=None, filters=None):
    """Disassembly lines of a function (or range), optionally filtered."""
    if end is None:
        f = ida_funcs.get_func(start)
        start, end = f.start_ea, f.end_ea
    out = []
    ea = start
    while ea < end:
        line = idc.GetDisasm(ea)
        keep = True
        if filters:
            keep = False
            for k in filters:
                if k in line:
                    keep = True
        if keep:
            out.append('%x %s' % (ea, line))
        ea = idc.next_head(ea, end)
    return '\n'.join(out)


def show_type(name):
    """Struct members or enum values of a named type from the IDB."""
    import ida_typeinf as ti
    til = ti.get_idati()
    tif = ti.tinfo_t()
    if not tif.get_named_type(til, name):
        return 'no type %s' % name
    if tif.is_enum():
        ed = ti.enum_type_data_t()
        tif.get_enum_details(ed)
        return '== enum %s %s' % (name, [(m.name, m.value) for m in ed])
    udt = ti.udt_type_data_t()
    tif.get_udt_details(udt)
    lines = ['== %s %d' % (name, tif.get_size())]
    for m in udt:
        lines.append('  %4d %-40s %s' % (m.offset // 8, m.name, m.type.dstr()))
    return '\n'.join(lines)


def show_types(names):
    return '\n'.join(show_type(n) for n in names)


def dump(required, any_of, path, exclude=('rbtree', 'fixed_', 'vector', 'Create@', 'hashtable')):
    """Write cleaned decompiles of functions whose name contains every
    `required` substring and at least one `any_of` substring."""
    out = []
    names = []
    for ea, name in find(required, exclude):
        if any_of and not any(k in name for k in any_of):
            continue
        names.append(name)
        out.append('===== %x %s' % (ea, name[:140]))
        text = dec(ea)
        lines = [l for l in text.splitlines() if not re.match(r'^  [\w:<>, \*\[\]&`\']+; // ', l)]
        out.append('\n'.join(lines))
    open(path, 'w', encoding='utf-8').write('\n'.join(out))
    return names


def members(name, lo=0, hi=1 << 30):
    """Members of a struct type within an offset range (flattens nothing)."""
    import ida_typeinf as ti
    til = ti.get_idati()
    tif = ti.tinfo_t()
    if not tif.get_named_type(til, name):
        return 'no type %s' % name
    udt = ti.udt_type_data_t()
    tif.get_udt_details(udt)
    out = ['== %s %d' % (name, tif.get_size())]
    for m in udt:
        off = m.offset // 8
        if lo <= off <= hi:
            out.append('  %5d %-44s %s' % (off, m.name, m.type.dstr()))
    return '\n'.join(out)


def keys64(ea):
    """64-bit constants assembled with lis/ori + insrdi (high word inserted)."""
    import re as _re
    d = disasm(ea).splitlines()
    regs = {}
    out = []
    for line in d:
        m = _re.match(r'(\w+) lis\s+(r\d+), (-?0x[0-9A-F]+|-?\d+)', line)
        if m:
            v = int(m.group(3), 16) if 'x' in m.group(3) else int(m.group(3))
            regs[m.group(2)] = (v << 16) & 0xFFFFFFFF
            continue
        m = _re.match(r'(\w+) ori\s+(r\d+), (r\d+), (0x[0-9A-F]+|\d+)', line)
        if m:
            src = regs.get(m.group(3))
            if src is not None:
                regs[m.group(2)] = src | (int(m.group(4), 16) if 'x' in m.group(4) else int(m.group(4)))
            continue
        m = _re.match(r'(\w+) insrdi\s+(r\d+), (r\d+), 32,0', line)
        if m:
            lo = regs.get(m.group(2))
            hi = regs.get(m.group(3))
            if lo is not None and hi is not None:
                out.append((m.group(1), '0x%08x_%08x' % (hi, lo)))
    return out
