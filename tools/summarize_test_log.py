"""Read a test run folder or collected ZIP without launching the game."""
import argparse
import collections
import json
from pathlib import Path
import re
import zipfile


def summarize(files):
    events = []
    invalid = []
    for name, data in sorted(files.items()):
        if not re.fullmatch(r"events-\d+\.jsonl", name):
            continue
        for number, line in enumerate(data.decode("utf-8", errors="replace").splitlines(), 1):
            try:
                record = json.loads(line)
                if not isinstance(record, dict) or "kind" not in record:
                    raise ValueError("event object absent")
                events.append(record)
            except (ValueError, TypeError):
                invalid.append(f"{name}:{number}")
    events.sort(key=lambda event: (event.get("ms", 0), event.get("seq", 0)))
    counts = collections.Counter(event["kind"] for event in events)
    failures = {
        "panic", "fatal_error", "tick_error", "save_error", "stage_stalled",
        "nonfinite_animation", "watchdog", "fingerprint_error",
    }
    last = {}
    dropped = 0
    for event in events:
        last[event["kind"]] = event
        if event["kind"] == "heartbeat":
            match = re.search(r"dropped_records=(\d+)", event.get("detail", ""))
            if match:
                dropped += int(match[1])
    return {
        "build": files.get("build.txt", b"").decode("utf-8", errors="replace"),
        "executable_sha256": files.get("executable.sha256", b"").decode("ascii", errors="replace").strip(),
        "retained_event_counts": dict(sorted(counts.items())),
        "retained_time_ms": [events[0].get("ms"), events[-1].get("ms")] if events else [],
        "reported_dropped_records_in_retained_heartbeats": dropped,
        "invalid_or_partial_json_lines": invalid,
        "normal_scope_exit_recorded": "session_end" in last,
        "recent_errors_and_stalls": [event for event in events if event["kind"] in failures][-8:],
        "last_observations": {
            kind: last[kind] for kind in [
                "frame", "raw_pad", "tick_stage", "skate3_physics", "animation_physics_input",
                "presentation_stage", "tick_consumed", "gesture_set", "gesture_assignment",
                "skater_save_requested", "save_end", "heartbeat",
            ] if kind in last
        },
        "failure_snapshots": sorted(name for name in files if re.fullmatch(r"failure-\d+\.jsonl", name)),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path, help="run directory or collected .zip")
    args = parser.parse_args()
    if args.run.is_dir():
        files = {path.name: path.read_bytes() for path in args.run.iterdir() if path.is_file()}
    else:
        with zipfile.ZipFile(args.run) as archive:
            files = {Path(name).name: archive.read(name) for name in archive.namelist() if not name.endswith("/")}
    print(json.dumps(summarize(files), indent=2))


if __name__ == "__main__":
    main()
