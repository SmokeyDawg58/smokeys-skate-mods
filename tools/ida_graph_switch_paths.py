"""IDA-side extractor for the graph-expression proof (run with py_exec_file).

Symbolically executes every case of a native StateGraph EvaluateExpression switch,
forking at each StateConditionDirect_* call, and writes every decision path
(condition assignment -> bool result) to JSON. Condition-cache stores are modelled,
so a re-read condition keeps its first value.

Set JOBS/OUT_PATH before exec'ing this file, e.g.
  OUT_PATH = "native_motion_paths.json"
  JOBS = [("pre", 0x8290aa50), ("sustain", 0x8291e218)]   # Motion graph
  JOBS = [("pre", 0x828ff990), ("sustain", 0x82906048)]   # Action graph
Then run tools/compare_graph_expressions.py on that JSON.
"""
import idc, idautils, ida_bytes, json, re

M32 = 0xFFFFFFFF
THIS = 0x10000000

def mask(mb, me):
    m = 0
    if mb <= me:
        for b in range(mb, me + 1):
            m |= 1 << (31 - b)
    else:
        for b in list(range(mb, 32)) + list(range(0, me + 1)):
            m |= 1 << (31 - b)
    return m

def rotl(v, n):
    n &= 31
    return ((v << n) | (v >> (32 - n))) & M32 if n else v & M32

def reg(ea, n):
    t = idc.print_operand(ea, n)
    m = re.match(r"^r(\d+)$", t)
    return int(m.group(1)) if m else None

def base_reg(ea, n):
    t = idc.print_operand(ea, n)
    m = re.search(r"\(r(\d+)\)\s*$", t)
    return int(m.group(1)) if m else None

def imm(ea, n):
    v = idc.get_operand_value(ea, n)
    if v is None:
        return 0
    return v & M32

def packed(ea, n):
    t = idc.print_operand(ea, n)
    return [int(x, 0) for x in t.replace(" ", "").split(",") if x]

def simm16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v

class Fail(Exception):
    pass

def _interp(start, conds_base, assignment, mem, reached, gpr, cr):
    ea = start
    steps = 0
    results = []
    while True:
        steps += 1
        if steps > 4000:
            raise Fail("step limit at %x" % ea)
        mn = idc.print_insn_mnem(ea)
        nxt = idc.next_head(ea)
        if mn in ("lwz", "lbz", "lhz", "lha"):
            d = reg(ea, 0); b = base_reg(ea, 1)
            disp = simm16(idc.get_operand_value(ea, 1))
            if b is None:
                raise Fail("load base %x" % ea)
            if isinstance(gpr[b], tuple):
                raise Fail("tuple base %x" % ea)
            addr = (gpr[b] + disp) & M32
            if conds_base <= addr < conds_base + 0x2000 and mn == "lwz":
                gpr[d] = ("COND", (addr - conds_base) // 4)
            else:
                size = {"lwz": 4, "lbz": 1, "lhz": 2, "lha": 2}[mn]
                val = 0
                for i in range(size):
                    val = (val << 8) | mem.get(addr + i, 0)
                gpr[d] = val
        elif mn in ("stw", "stb", "sth"):
            s = reg(ea, 0); b = base_reg(ea, 1)
            disp = simm16(idc.get_operand_value(ea, 1))
            addr = (gpr[b] + disp) & M32
            size = {"stw": 4, "stb": 1, "sth": 2}[mn]
            val = gpr[s]
            if isinstance(val, tuple):
                raise Fail("store tuple %x" % ea)
            for i in range(size):
                mem[addr + i] = (val >> (8 * (size - 1 - i))) & 0xFF
        elif mn == "li":
            gpr[reg(ea, 0)] = simm16(imm(ea, 1)) & M32
        elif mn == "lis":
            gpr[reg(ea, 0)] = (imm(ea, 1) << 16) & M32
        elif mn == "addi":
            a = reg(ea, 1)
            base = 0 if a == 0 else gpr[a]
            if isinstance(base, tuple):
                raise Fail("addi tuple %x" % ea)
            gpr[reg(ea, 0)] = (base + simm16(imm(ea, 2))) & M32
        elif mn == "mr":
            gpr[reg(ea, 0)] = gpr[reg(ea, 1)]
        elif mn in ("ori", "oris"):
            v = imm(ea, 2)
            if mn == "oris":
                v = (v << 16) & M32
            s = gpr[reg(ea, 1)]
            if isinstance(s, tuple):
                raise Fail("ori tuple %x" % ea)
            gpr[reg(ea, 0)] = (s | v) & M32
        elif mn == "rlwinm":
            s = gpr[reg(ea, 1)]
            if isinstance(s, tuple):
                raise Fail("rlwinm tuple %x" % ea)
            sh, mb, me = packed(ea, 2)
            gpr[reg(ea, 0)] = rotl(s, sh) & mask(mb, me)
        elif mn == "clrlwi":
            s = gpr[reg(ea, 1)]; (n,) = packed(ea, 2)
            if isinstance(s, tuple):
                raise Fail("clrlwi tuple %x" % ea)
            gpr[reg(ea, 0)] = s & mask(n, 31)
        elif mn == "cntlzw":
            s = gpr[reg(ea, 1)]
            if isinstance(s, tuple):
                raise Fail("cntlzw tuple %x" % ea)
            s &= M32
            n = 0
            while n < 32 and not (s & (1 << (31 - n))):
                n += 1
            gpr[reg(ea, 0)] = n
        elif mn == "rlwimi":
            a = reg(ea, 0); s = gpr[reg(ea, 1)]
            if isinstance(s, tuple) or isinstance(gpr[a], tuple):
                raise Fail("rlwimi tuple %x" % ea)
            sh, mb, me = packed(ea, 2)
            m = mask(mb, me)
            gpr[a] = (rotl(s, sh) & m) | (gpr[a] & ~m & M32)
        elif mn == "clrrwi":
            s = gpr[reg(ea, 1)]; (n,) = packed(ea, 2)
            if isinstance(s, tuple):
                raise Fail("clrrwi tuple %x" % ea)
            gpr[reg(ea, 0)] = s & mask(0, 31 - n)
        elif mn == "extrwi":
            s = gpr[reg(ea, 1)]; n, b = packed(ea, 2)
            if isinstance(s, tuple):
                raise Fail("extrwi tuple %x" % ea)
            gpr[reg(ea, 0)] = rotl(s, b + n) & mask(32 - n, 31)
        elif mn == "insrwi":
            a = reg(ea, 0); s = gpr[reg(ea, 1)]; n, b = packed(ea, 2)
            if isinstance(s, tuple) or isinstance(gpr[a], tuple):
                raise Fail("insrwi tuple %x" % ea)
            sh = (32 - (b + n)) & 31
            m = mask(b, b + n - 1)
            gpr[a] = (rotl(s, sh) & m) | (gpr[a] & ~m & M32)
        elif mn in ("cmplwi", "cmpwi", "cmplw", "cmpw"):
            crf = idc.print_operand(ea, 0)
            if mn in ("cmplwi", "cmpwi"):
                x = gpr[reg(ea, 1)]; y = imm(ea, 2)
            else:
                x = gpr[reg(ea, 1)]; y = gpr[reg(ea, 2)]
            if isinstance(x, tuple) or isinstance(y, tuple):
                raise Fail("cmp tuple %x" % ea)
            def s32(v):
                v &= M32
                return v - (1 << 32) if v & 0x80000000 else v
            if mn == "cmpwi":
                x2, y2 = s32(x), simm16(y)
            elif mn == "cmpw":
                x2, y2 = s32(x), s32(y)
            else:
                x2, y2 = x & M32, y & M32
            cr[crf] = (x2 < y2, x2 > y2, x2 == y2)
        elif mn in ("beq", "bne", "blt", "bgt", "ble", "bge"):
            op0 = idc.print_operand(ea, 0)
            if op0.startswith("cr"):
                crf = op0; target = idc.get_operand_value(ea, 1)
            else:
                crf = "cr0"; target = idc.get_operand_value(ea, 0)
            lt, gt, eq = cr[crf]
            take = {"beq": eq, "bne": not eq, "blt": lt, "bgt": gt, "ble": not gt, "bge": not lt}[mn]
            if take:
                nxt = target
        elif mn == "b":
            target = idc.get_operand_value(ea, 0)
            name = idc.get_name(target) or ""
            if "restgprlr" in name:
                r = gpr[3]
                if isinstance(r, tuple):
                    raise Fail("return tuple")
                results.append((dict(assignment), (r & 0xFF) != 0))
                return results
            nxt = target
        elif mn == "bl":
            target = idc.get_operand_value(ea, 0)
            name = idc.get_name(target) or ""
            if "StateConditionDirect" in name:
                c = gpr[3]
                if not (isinstance(c, tuple) and c[0] == "COND"):
                    raise Fail("cond call without COND at %x" % ea)
                idx = c[1]
                reached.add(idx)
                if idx in assignment:
                    gpr[3] = 1 if assignment[idx] else 0
                else:
                    for val in (False, True):
                        a2 = dict(assignment); a2[idx] = val
                        g2 = list(gpr); g2[3] = 1 if val else 0
                        results.extend(_interp(nxt, conds_base, a2, dict(mem), reached, g2, dict(cr)))
                    return results
            else:
                raise Fail("unknown call %s at %x" % (name, ea))
        elif mn == "blr":
            r = gpr[3]
            if isinstance(r, tuple):
                raise Fail("blr tuple")
            results.append((dict(assignment), (r & 0xFF) != 0))
            return results
        elif mn == "nop":
            pass
        else:
            raise Fail("unhandled %s at %x: %s" % (mn, ea, idc.GetDisasm(ea)))
        ea = nxt


def switch_info(func):
    ea = func
    count = None
    table = None
    for _ in range(40):
        d = idc.GetDisasm(ea)
        m = re.search(r"cmplwi\s+cr\d+,\s*r\d+,\s*(0x[0-9A-Fa-f]+|\d+)", d)
        if m and "switch" in d:
            count = int(m.group(1), 0) + 1
        if idc.print_insn_mnem(ea) == "lis" and "jpt_" in d:
            table = idc.get_name_ea_simple(re.search(r"(jpt_[0-9A-F]+)", d).group(1))
        ea = idc.next_head(ea)
    return table, count

def conds_base_for(func):
    end = idc.get_func_attr(func, idc.FUNCATTR_END)
    ea = func
    while ea < end:
        d = idc.GetDisasm(ea)
        m = re.search(r"sConditions.*?- (0x[0-9A-Fa-f]+)\)", d)
        if m:
            return int(m.group(1), 16)
        ea = idc.next_head(ea)
    return None

def main():
    out = {}
    for label, func in JOBS:
        table, count = switch_info(func)
        conds = conds_base_for(func)
        cases = {}
        for i in range(count):
            t = ida_bytes.get_dword(table + 4 * i)
            try:
                gpr = [0] * 32
                gpr[31] = THIS
                gpr[1] = 0x20000000
                paths = _interp(t, conds, {}, {}, set(), gpr, {})
                cases[i] = {"target": t, "paths": [[{str(k): v for k, v in a.items()}, r] for a, r in paths]}
            except Fail as e:
                cases[i] = {"target": t, "error": str(e)}
            except Exception as e:
                cases[i] = {"target": t, "error": "exc " + repr(e)}
        out[label] = cases
        out[label + "_meta"] = {"table": table, "count": count, "conds": conds}
    with open(OUT_PATH, "w") as f:
        json.dump(out, f)
    return {k: (sum(1 for c in v.values() if "error" in c), len(v)) for k, v in out.items() if not k.endswith("_meta")}

print(main())
