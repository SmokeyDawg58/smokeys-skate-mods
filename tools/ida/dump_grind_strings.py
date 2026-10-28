"""Dump IDA strings containing grind terms from the batch database."""

from pathlib import Path

import ida_auto
import ida_pro
import idautils


OUTPUT = Path(__file__).resolve().parents[2] / "artifacts" / "ida-grind-strings.txt"


def main() -> None:
    ida_auto.auto_wait()
    lines = []
    for item in idautils.Strings():
        value = str(item)
        lowered = value.lower()
        if "grind" in lowered or "pair dist" in lowered:
            lines.append(f"{item.ea:08x} {value}")
    OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")


exit_code = 0
try:
    main()
except Exception:
    exit_code = 1
    raise
finally:
    ida_pro.qexit(exit_code)
