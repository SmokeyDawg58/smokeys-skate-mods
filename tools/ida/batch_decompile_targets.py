"""Run the target decompiler after batch auto-analysis, then close IDA."""

from __future__ import annotations

import sys
import traceback
from pathlib import Path

import ida_auto
import ida_pro


PROJECT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT / "tools" / "ida"))

from decompile_targets import main  # noqa: E402


exit_code = 0
try:
    ida_auto.auto_wait()
    main()
except Exception:
    traceback.print_exc()
    exit_code = 1
finally:
    ida_pro.qexit(exit_code)
