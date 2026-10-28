"""Disassemble Xbox 360 (Xenos) compiled shader containers.

Research aid for effects that ship only microcode (render.big `.fpo`/`.vpo`)
and for the shader blobs embedded in the executable (for example
`Variance_HBlur_PS` at 0x8212f4e0 in the debug xex).

  python xenos_disasm.py FILE [FILE...]

Container: header word 6 (+0x18) is the info block offset; info word 1 is the
microcode size. The microcode is the last `size` bytes. Pixel shaders place
their literal constants (`def`) just before it: info word 0 bytes of vec4s,
the last one being c255.

Field layouts follow Xenia's gpu/ucode.h (bitfields LSB-first inside each
big-endian dword):
- control flow: two 48-bit instructions per three dwords; `exec` carries the
  first instruction slot, the count and two sequence bits per instruction
  (bit 0 set = fetch);
- ALU: vector op in word 2 bits 24-28, scalar op in word 0 bits 26-31; sources
  are temporaries when their select bit is set, else constants; source
  swizzles are relative (`((swizzle >> 2i) + i) & 3`);
- a two-operand scalar op reads src3's w-slot then x-slot components;
  `mulsc`/`addsc`/`subsc` read the constant from the src3 register field and
  the temporary index from `(swizzle & 0x3c) | (sel << 1) | (opcode & 1)`;
- texture fetch offsets are in half texels.
Vertex-fetch format fields are printed raw and were not verified.
"""

import struct
import sys

VECTOR_OPS = [
    "add", "mul", "max", "min", "seq", "sgt", "sge", "sne", "frc", "trunc", "floor", "mad",
    "cndeq", "cndge", "cndgt", "dp4", "dp3", "dp2add", "cube", "max4", "setp_eq_push",
    "setp_ne_push", "setp_gt_push", "setp_ge_push", "kill_eq", "kill_gt", "kill_ge", "kill_ne",
    "dst", "maxa",
]
VECTOR_ARGUMENTS = {
    "frc": 1, "trunc": 1, "floor": 1, "max4": 1,
    "mad": 3, "cndeq": 3, "cndge": 3, "cndgt": 3, "dp2add": 3,
}
SCALAR_OPS = [
    "adds", "adds_prev", "muls", "muls_prev", "muls_prev2", "maxs", "mins", "seqs", "sgts",
    "sges", "snes", "frcs", "truncs", "floors", "exp", "logc", "log", "rcpc", "rcpf", "rcp",
    "rsqc", "rsqf", "rsq", "maxas", "maxasf", "subs", "subs_prev", "setp_eq", "setp_ne",
    "setp_gt", "setp_ge", "setp_inv", "setp_pop", "setp_clr", "setp_rstr", "kills_eq",
    "kills_gt", "kills_ge", "kills_ne", "kills_one", "sqrt", "op41", "mulsc0", "mulsc1",
    "addsc0", "addsc1", "subsc0", "subsc1", "sin", "cos", "retain_prev",
]
SCALAR_TWO_OPERANDS = {"adds", "muls", "maxs", "mins", "subs"}
SCALAR_CONSTANT_OPS = {"mulsc0", "mulsc1", "addsc0", "addsc1", "subsc0", "subsc1"}
CONTROL_OPS = [
    "nop", "exec", "exec_end", "cond_exec", "cond_exec_end", "cond_pred_exec",
    "cond_pred_exec_end", "loop_start", "loop_end", "cond_call", "return", "cond_jmp", "alloc",
    "cond_exec_pred_clean", "cond_exec_pred_clean_end", "mark_vs_fetch_done",
]
EXECS = {name for name in CONTROL_OPS if "exec" in name}
EXEC_ENDS = {name for name in EXECS if name.endswith("_end")}
LANES = "xyzw"


def bits(value, low, count):
    return (value >> low) & ((1 << count) - 1)


def signed_bits(value, low, count):
    field = bits(value, low, count)
    return field - (1 << count) if field & (1 << (count - 1)) else field


def swizzle(value):
    return "".join(LANES[((value >> (2 * lane)) + lane) & 3] for lane in range(4))


def write_mask(value):
    return "".join(LANES[lane] if value & (1 << lane) else "_" for lane in range(4))


def operand(register, temporary, negate, absolute_constants):
    if temporary:
        name = "r%d" % (register & 0x3F)
        if register & 0x80:
            name = "|%s|" % name
    else:
        name = "c%d" % register
        if absolute_constants:
            name = "|%s|" % name
    return ("-" if negate else "") + name


def alu(w0, w1, w2):
    vector_dest, absolute = bits(w0, 0, 6), bits(w0, 7, 1)
    scalar_dest, export = bits(w0, 8, 6), bits(w0, 15, 1)
    vector_mask, scalar_mask = bits(w0, 16, 4), bits(w0, 20, 4)
    vector_clamp, scalar_clamp = bits(w0, 24, 1), bits(w0, 25, 1)
    scalar_op = bits(w0, 26, 6)
    swizzles = [bits(w1, 16, 8), bits(w1, 8, 8), bits(w1, 0, 8)]
    negates = [bits(w1, 26, 1), bits(w1, 25, 1), bits(w1, 24, 1)]
    predicate = ""
    if bits(w1, 28, 1):
        predicate = "(%sp) " % ("" if bits(w1, 27, 1) else "!")
    registers = [bits(w2, 16, 8), bits(w2, 8, 8), bits(w2, 0, 8)]
    vector_op = bits(w2, 24, 5)
    temporaries = [bits(w2, 31, 1), bits(w2, 30, 1), bits(w2, 29, 1)]
    out = []
    vector_name = VECTOR_OPS[vector_op] if vector_op < len(VECTOR_OPS) else "vop%d" % vector_op
    if vector_mask or not scalar_mask:
        destination = ("oC%d" % vector_dest) if export else "r%d" % vector_dest
        arguments = ", ".join(
            "%s.%s" % (operand(registers[i], temporaries[i], negates[i], absolute), swizzle(swizzles[i]))
            for i in range(VECTOR_ARGUMENTS.get(vector_name, 2))
        )
        out.append("%s%s%s %s.%s, %s" % (
            predicate, vector_name, "_sat" if vector_clamp else "", destination,
            write_mask(vector_mask), arguments,
        ))
    scalar_name = SCALAR_OPS[scalar_op] if scalar_op < len(SCALAR_OPS) else "sop%d" % scalar_op
    if scalar_name != "retain_prev" or scalar_mask:
        a = ((swizzles[2] >> 6) + 3) & 3
        b = swizzles[2] & 3
        base = operand(registers[2], temporaries[2], negates[2], absolute)
        if scalar_name in SCALAR_TWO_OPERANDS or scalar_name in {"seqs", "sgts", "sges", "snes"}:
            arguments = "%s.%s, %s.%s" % (base, LANES[a], base, LANES[b])
        elif scalar_name in SCALAR_CONSTANT_OPS:
            temporary = (swizzles[2] & 0x3C) | (temporaries[2] << 1) | (scalar_op & 1)
            arguments = "%sc%d.%s, r%d.%s" % (
                "-" if negates[2] else "", registers[2], LANES[a], temporary, LANES[b])
        else:
            arguments = "%s.%s" % (base, LANES[a])
        destination = ("oC%d" % scalar_dest) if export else "r%d" % scalar_dest
        target = "%s.%s" % (destination, write_mask(scalar_mask)) if scalar_mask else "ps"
        out.append("%s%s%s %s, %s" % (
            predicate, scalar_name, "_sat" if scalar_clamp else "", target, arguments))
    return "  ||  ".join(out) if out else "nop"


def fetch(w0, w1, w2):
    opcode = bits(w0, 0, 5)
    if opcode == 0:
        return "vfetch r%d, r%d, vf%d  (raw %08x %08x %08x)" % (
            bits(w0, 12, 6), bits(w0, 5, 6), bits(w0, 20, 5) * 3 + bits(w0, 25, 2), w0, w1, w2)
    names = {1: "tfetch", 16: "getBCF", 17: "getCompTexLOD", 18: "getGradients",
             19: "getWeights", 24: "setTexLOD", 25: "setGradientH", 26: "setGradientV"}
    name = names.get(opcode, "fop%d" % opcode)
    source = "".join(LANES[bits(bits(w0, 26, 6), 2 * lane, 2)] for lane in range(3))
    components = "xyzw01?_"
    destination = "".join(components[bits(bits(w1, 0, 12), 3 * lane, 3)] for lane in range(4))
    filters = ["point", "linear", "basemap", "fetchconst"]
    extra = []
    offsets = [signed_bits(w2, 16, 5), signed_bits(w2, 21, 5), signed_bits(w2, 26, 5)]
    if any(offsets):
        extra.append("Offset=(%g,%g,%g)" % tuple(offset * 0.5 for offset in offsets))
    extra.append("Mag=%s Min=%s Mip=%s" % (
        filters[bits(w1, 12, 2)], filters[bits(w1, 14, 2)], filters[bits(w1, 16, 2)]))
    if bits(w0, 25, 1):
        extra.append("UnNormalized")
    lod_bias = signed_bits(w2, 2, 7)
    if lod_bias:
        extra.append("LODBias=%g" % (lod_bias / 16.0))
    return "%s%s r%d.%s, r%d.%s, tf%d  %s" % (
        name, ["1D", "2D", "3D", "Cube"][bits(w2, 14, 2)], bits(w0, 12, 6), destination,
        bits(w0, 5, 6), source, bits(w0, 20, 5), " ".join(extra))


def disassemble(code):
    words = struct.unpack(">%dI" % (len(code) // 4), code[: len(code) // 4 * 4])
    controls = []
    index = 0
    while index + 2 < len(words):
        a, b, c = words[index:index + 3]
        pair = [a | ((b & 0xFFFF) << 32), (b >> 16) | (c << 16)]
        for word in pair:
            name = CONTROL_OPS[bits(word, 44, 4)]
            if name in EXECS:
                controls.append((name, bits(word, 0, 12), bits(word, 12, 3), bits(word, 16, 12)))
            else:
                controls.append((name, None, None, None))
        index += 3
        if any(control[0] in EXEC_ENDS for control in controls[-2:]):
            break
    lines = []
    for name, address, count, sequence in controls:
        if address is None:
            lines.append("  %s" % name)
            continue
        lines.append("  %s addr=%d count=%d" % (name, address, count))
        for k in range(count):
            slot = address + k
            w0, w1, w2 = words[slot * 3:slot * 3 + 3]
            text = fetch(w0, w1, w2) if (sequence >> (2 * k)) & 1 else alu(w0, w1, w2)
            lines.append("    %2d: %s" % (slot, text))
    return lines


def main():
    for path in sys.argv[1:]:
        blob = open(path, "rb").read()
        info = struct.unpack_from(">I", blob, 0x18)[0]
        literal_bytes, size = struct.unpack_from(">2I", blob, info)
        start = len(blob) - size
        print("== %s: %d bytes, microcode at %#x (%#x bytes)" % (path, len(blob), start, size))
        # Container +0x10 points at a size-prefixed big-endian D3DX CTAB.
        # Resolve names from this final program rather than an older UPDB.
        table = struct.unpack_from(">I", blob, 0x10)[0] + 4
        if table + 28 <= len(blob) and struct.unpack_from(">I", blob, table)[0] == 28:
            constants, entries = struct.unpack_from(">2I", blob, table + 12)
            if table + entries + constants * 20 <= len(blob):
                for entry in range(constants):
                    name, regset, register, registers, _, _, _ = struct.unpack_from(
                        ">I4H2I", blob, table + entries + entry * 20)
                    begin = table + name
                    end = blob.find(b"\0", begin)
                    if 0 <= begin < end < len(blob):
                        prefix = {0: "b", 1: "i", 2: "c", 3: "tf"}.get(regset, "?")
                        print("  bind %s%d[%d] = %s" % (
                            prefix, register, registers, blob[begin:end].decode("ascii")))
        count = literal_bytes // 16
        if 0 < count and start >= literal_bytes:
            literals = struct.unpack_from(">%df" % (4 * count), blob, start - literal_bytes)
            for k in range(count):
                values = ", ".join("%.9g" % v for v in literals[4 * k:4 * k + 4])
                print("  c%d = (%s)" % (256 - count + k, values))
        for line in disassemble(blob[start:]):
            print(line)
        print()


if __name__ == "__main__":
    main()
