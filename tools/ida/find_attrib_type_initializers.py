"""Report code references that initialize the three Attrib export type IDs."""

from __future__ import annotations

import traceback
from pathlib import Path

import ida_auto
import ida_bytes
import ida_funcs
import ida_lines
import ida_name
import ida_pro
import idautils


OUTPUT = Path(__file__).resolve().parents[2] / "artifacts" / "attrib-type-initializers.txt"
TARGETS = {
    "gClassType": 0x8307D248,
    "gDatabaseType": 0x8307D250,
    "gCollectionType": 0x8307D258,
}


def disassembly(ea: int) -> str:
    rendered = ida_lines.generate_disasm_line(ea, ida_lines.GENDSM_FORCE_CODE) or ""
    return ida_lines.tag_remove(rendered)


def main() -> None:
    ida_auto.auto_wait()
    lines: list[str] = []
    seen_functions: set[int] = set()
    for label, target in TARGETS.items():
        lines.append(f"{label} {target:08x} {ida_name.get_name(target)}")
        for reference in idautils.DataRefsTo(target):
            function = ida_funcs.get_func(reference)
            start = function.start_ea if function else reference
            lines.append(
                f"  reference {reference:08x} function {start:08x} "
                f"{ida_funcs.get_func_name(start)}"
            )
            if function is None or start in seen_functions:
                continue
            seen_functions.add(start)
            for ea in idautils.Heads(function.start_ea, function.end_ea):
                if ida_bytes.is_code(ida_bytes.get_flags(ea)):
                    lines.append(f"    {ea:08x} {disassembly(ea)}")
    OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")


exit_code = 0
try:
    main()
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
