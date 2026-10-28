"""Dump named target functions and their data references from a batch IDA DB."""

from __future__ import annotations

import re
import sys
import traceback
from pathlib import Path

import ida_auto
import ida_bytes
import ida_funcs
import ida_lines
import ida_name
import ida_pro
import idautils


PROJECT = Path(__file__).resolve().parents[2]
TARGETS = PROJECT / "artifacts" / "ida-targets.txt"
OUTPUT = PROJECT / "artifacts" / "ida-target-disassembly.txt"


def read_targets() -> list[int]:
    values: list[int] = []
    for line in TARGETS.read_text(encoding="utf-8").splitlines():
        match = re.match(r"\s*(0x[0-9a-fA-F]+)", line)
        if match:
            values.append(int(match.group(1), 16))
    return values


def clean_line(ea: int) -> str:
    text = ida_lines.generate_disasm_line(ea, ida_lines.GENDSM_FORCE_CODE) or ""
    return ida_lines.tag_remove(text)


def main() -> None:
    ida_auto.auto_wait()
    lines: list[str] = []
    seen_data: set[int] = set()
    data_refs: list[int] = []
    for target in read_targets():
        function = ida_funcs.get_func(target)
        if function is None or function.start_ea != target:
            continue
        lines.append(f"\nFUNCTION {target:08x} {ida_name.get_name(target)}")
        for ea in idautils.Heads(function.start_ea, function.end_ea):
            if not ida_bytes.is_code(ida_bytes.get_flags(ea)):
                continue
            refs = list(idautils.DataRefsFrom(ea))
            suffix = ""
            if refs:
                labels = []
                for ref in refs:
                    labels.append(f"{ref:08x}:{ida_name.get_name(ref)}")
                    if ref not in seen_data:
                        seen_data.add(ref)
                        data_refs.append(ref)
                suffix = "  ; data " + ", ".join(labels)
            lines.append(f"{ea:08x}  {clean_line(ea)}{suffix}")

    lines.append("\nDATA REFERENCES")
    for ea in sorted(data_refs):
        # Include the preceding vector lane: optimized VPU code frequently
        # addresses a scalar constant at a small negative displacement.
        start = ea - 16
        blob = ida_bytes.get_bytes(start, 48)
        rendered = blob.hex(" ") if blob else "<unreadable>"
        lines.append(f"{ea:08x} {ida_name.get_name(ea)} [{start:08x}]: {rendered}")
    OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")


exit_code = 0
try:
    main()
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
