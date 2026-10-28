"""Export IDA's discovered strings from the batch database, then exit."""

from __future__ import annotations

import json
import traceback
from pathlib import Path

import ida_auto
import ida_pro
import idautils


OUTPUT = Path(__file__).resolve().parents[2] / "artifacts" / "ida-strings.jsonl"


exit_code = 0
try:
    ida_auto.auto_wait()
    strings = idautils.Strings()
    strings.setup(strtypes=[0, 1], minlen=3)
    with OUTPUT.open("w", encoding="utf-8") as output:
        for item in strings:
            try:
                value = str(item)
            except UnicodeDecodeError:
                continue
            output.write(json.dumps({"address": int(item.ea), "value": value}) + "\n")
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
