"""Dump exact SerialisedAssembly AABB construction evidence.

Run against the symbolized Skate 2 IDA database.  The output deliberately
contains both Hex-Rays pseudocode and raw PowerPC instructions so numeric and
control-flow claims can be checked independently of the decompiler.
"""

from __future__ import annotations

import hashlib
import traceback
from pathlib import Path

import ida_auto
import ida_bytes
import ida_funcs
import ida_hexrays
import ida_lines
import ida_name
import ida_nalt
import ida_pro
import idautils
import idc


OUTPUT = Path(__file__).resolve().parents[2] / "artifacts" / "serialised_assembly_bbox_raw.txt"
TARGETS = (
    0x827C94C0,  # non-namespaced collision-job twin
    0x82E0B760,  # EA::Physics::CalculatePartBBox
    0x82E0B970,  # EA::Physics::CalculatePartBBoxUnion
    0x82ACDD28,  # rw::collision::SphereVolume::GetBBox
    0x82ACDFE8,  # rw::collision::TriangleVolume::GetBBox
    0x82AD7E78,  # rw::collision::BoxVolume::GetBBox
    0x82AD8038,  # rw::collision::CapsuleVolume::GetBBox
    0x82AD8450,  # rw::collision::CylinderVolume::GetBBox
)


def display_name(address: int) -> str:
    raw = ida_name.get_name(address) or ida_funcs.get_func_name(address)
    if not raw:
        return "<unnamed>"
    return ida_name.demangle_name(raw, ida_name.MNG_LONG_FORM) or raw


def function_identity(address: int) -> str:
    function = ida_funcs.get_func(address)
    start = function.start_ea if function else address
    return f"{start:#010x} {display_name(start)}"


def clean_disassembly(address: int) -> str:
    rendered = idc.generate_disasm_line(address, 0) or ""
    return ida_lines.tag_remove(rendered)


def dump_function(lines: list[str], target: int) -> None:
    function = ida_funcs.get_func(target)
    if function is None:
        raise RuntimeError(f"no function at {target:#010x}")

    lines.append("=" * 100)
    lines.append(function_identity(function.start_ea))
    lines.append(
        f"range {function.start_ea:#x}..{function.end_ea:#x} "
        f"size={function.end_ea - function.start_ea:#x}"
    )
    lines.append("CALLERS:")
    for reference in idautils.CodeRefsTo(function.start_ea, False):
        lines.append(f"  at {reference:#010x}: {function_identity(reference)}")

    lines.append("CALLEES:")
    seen: set[int] = set()
    for instruction in idautils.FuncItems(function.start_ea):
        for target_address in idautils.CodeRefsFrom(instruction, False):
            callee = ida_funcs.get_func(target_address)
            if (
                callee is not None
                and callee.start_ea != function.start_ea
                and callee.start_ea not in seen
            ):
                seen.add(callee.start_ea)
                lines.append(
                    f"  at {instruction:#010x}: "
                    f"{function_identity(callee.start_ea)}"
                )

    lines.append("PSEUDOCODE:")
    try:
        lines.append(str(ida_hexrays.decompile(function.start_ea)))
    except Exception as error:  # IDA exposes several decompiler exception types.
        lines.append(f"DECOMPILATION FAILED: {error!r}")

    lines.append("DISASSEMBLY:")
    for instruction in idautils.FuncItems(function.start_ea):
        raw = ida_bytes.get_bytes(instruction, 4)
        raw_text = raw.hex() if raw else "????????"
        lines.append(
            f"{instruction:#010x}  {raw_text}  "
            f"{clean_disassembly(instruction)}"
        )


def main() -> None:
    ida_auto.auto_wait()
    if not ida_hexrays.init_hexrays_plugin():
        raise RuntimeError("PowerPC decompiler is unavailable")

    lines: list[str] = []
    input_path = Path(idc.get_input_file_path())
    input_hash = ida_nalt.retrieve_input_file_sha256()
    lines.append(f"input_file_path={input_path}")
    lines.append(
        "input_sha256="
        + (bytes(input_hash).hex().upper() if input_hash else "<unavailable>")
    )
    for target in TARGETS:
        dump_function(lines, target)

    OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    digest = hashlib.sha256(OUTPUT.read_bytes()).hexdigest().upper()
    print(f"wrote {OUTPUT} sha256={digest}")


exit_code = 0
try:
    main()
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
