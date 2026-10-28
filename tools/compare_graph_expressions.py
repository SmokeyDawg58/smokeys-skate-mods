"""Prove the port's Motion/Action graph expressions against native decision paths.

Usage:
  python compare_graph_expressions.py motion native_motion_paths.json
  python compare_graph_expressions.py action native_action_paths.json

The JSON comes from tools/ida_graph_switch_paths.py (IDA side). Each native path is
(condition assignment -> result); the port's Rust match arm is transcribed to Python
and evaluated under four completions of the conditions native did not read.
"""

import json
import random
import re
import sys
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "crates" / "skate2-core" / "src"


def strip_comments(text):
    out = []
    for line in text.split("\n"):
        in_str = False
        for i, ch in enumerate(line):
            if ch == '"':
                in_str = not in_str
            if not in_str and line.startswith("//", i):
                line = line[:i]
                break
        out.append(line)
    return "\n".join(out)


def block_after(text, marker, opener):
    start = text.index(marker)
    m = text.index(opener, start)
    i = text.index("{", m) + 1
    depth = 1
    j = i
    while depth:
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
        j += 1
    return text[i:j - 1]


def split_arms(block):
    arms = []
    i, n = 0, len(block)
    while i < n:
        while i < n and block[i] in " \t\r\n,":
            i += 1
        if i >= n:
            break
        if block.startswith("#[", i):
            depth = 0
            while True:
                if block[i] == "[":
                    depth += 1
                elif block[i] == "]":
                    depth -= 1
                    if depth == 0:
                        i += 1
                        break
                i += 1
            continue
        arrow = block.index("=>", i)
        pattern = block[i:arrow].strip()
        j = arrow + 2
        while block[j] in " \t\r\n":
            j += 1
        if block[j] == "{":
            depth, k = 0, j
            while True:
                if block[k] == "{":
                    depth += 1
                elif block[k] == "}":
                    depth -= 1
                    if depth == 0:
                        k += 1
                        break
                k += 1
            expr, i = block[j + 1:k - 1].strip(), k
        else:
            depth, k = 0, j
            while k < n:
                ch = block[k]
                if ch in "([{":
                    depth += 1
                elif ch in ")]}":
                    depth -= 1
                elif ch == "," and depth == 0:
                    break
                k += 1
            expr, i = block[j:k].strip(), k + 1
        arms.append((pattern, expr))
    return arms


IDS = {}


def load_ids():
    """`expr::NAME` and `cond::NAME` values from the generated id modules of both graphs."""
    for path in SRC.rglob("generated_*_ids.rs"):
        graph = path.name.split("_")[1]
        text = path.read_text(encoding="utf-8")
        for module, alias in (("condition", "cond"), ("expression", "expr")):
            body = re.search(rf"pub\(crate\) mod {module} \{{(.*?)\n\}}", text, re.S).group(1)
            for name, value in re.findall(r"pub\(crate\) const (\w+): u16 = (\d+);", body):
                IDS.setdefault(graph, {})[f"{alias}::{name}"] = int(value)


def index_value(item, graph):
    return IDS[graph][item] if item in IDS.get(graph, {}) else int(item)


def indices(text, graph):
    out = []
    for item in text.split("|"):
        item = item.strip()
        if item == "NULL_INDEX":
            out.append(65535)
        elif "..=" in item:
            a, b = item.split("..=")
            out.extend(range(index_value(a.strip(), graph), index_value(b.strip(), graph) + 1))
        else:
            out.append(index_value(item, graph))
    return out


def to_python(expr, graph):
    e = " ".join(expr.split())
    if "grounded_selection::expression" in e:
        return "FALLBACK"
    if "trick_selection::expression" in e:
        return "TRICK"
    if "ground_selection::expression" in e:
        return "GROUND"
    if "Err(" in e and "Ok(" not in e:
        return "PENDING"
    if e.startswith("Ok(") and e.endswith(")"):
        e = e[3:-1]
    e = re.sub(
        r"\b(?:c|condition)\(\s*(cond::\w+|\d+)\s*,?\s*\)\??",
        lambda m: f"C[{index_value(m.group(1), graph)}]",
        e,
    )
    e = e.replace("&&", " and ").replace("||", " or ")
    e = re.sub(r"!\s*(?=[C(])", " not ", e)
    return e.replace("true", "True").replace("false", "False")


def source(fname):
    found = sorted(SRC.rglob(fname))
    if len(found) != 1:
        raise SystemExit(f"{fname}: expected one source file, found {len(found)}")
    return found[0]


def load_index_arms(fname, marker, graph):
    text = strip_comments(source(fname).read_text(encoding="utf-8"))
    table, default = {}, None
    for pattern, expr in split_arms(block_after(text, marker, "match index")):
        py = to_python(expr, graph)
        if pattern == "_":
            default = py
        elif " if " not in pattern:
            for idx in indices(pattern, graph):
                table.setdefault(idx, py)
    return table, default


def load_tuple_arms(fname, marker, graph):
    text = strip_comments(source(fname).read_text(encoding="utf-8"))
    arms = []
    for pattern, expr in split_arms(block_after(text, marker, "match (mask,")):
        p = pattern.strip()
        if p == "_":
            arms.append(({"pre", "sustain"}, "any", to_python(expr, graph)))
            continue
        mask_text, idx_text = re.match(r"^\((.*?),(.*?),?\s*\)$", p, re.S).groups()
        masks = set()
        for item in mask_text.split("|"):
            item = item.strip()
            masks |= {"_": {"pre", "sustain"}, "Mask::Pre": {"pre"}, "Mask::Sustain": {"sustain"}}[item]
        idx_text = idx_text.strip()
        arms.append((masks, "any" if idx_text == "_" else indices(idx_text, graph), to_python(expr, graph)))
    return arms


def tuple_lookup(arms, mask, idx):
    for masks, idxs, expr in arms:
        if mask in masks and (idxs == "any" or idx in idxs):
            return expr
    return None


def evaluate(e, assignment, fill):
    class C(dict):
        def __missing__(self, key):
            return fill(key)
    env = C({int(k): v for k, v in assignment.items()})
    return bool(eval(e, {"C": env, "True": True, "False": False}))


def port_expression(graph):
    if graph == "motion":
        pre, pre_default = load_index_arms("generated_motion_runtime_selection.rs", "pub(super) fn expression(", graph)
        grounded, grounded_default = load_index_arms("actor_motion_runtime_grounded_selection.rs", "pub(super) fn expression(", graph)
        sustain, sustain_default = load_index_arms("generated_motion_runtime_sustain.rs", "pub(super) fn expression(", graph)

        def lookup(mask, idx):
            if mask == "pre":
                e = pre.get(idx, pre_default)
                return grounded.get(idx, grounded_default) if e == "FALLBACK" else e
            return sustain.get(idx, sustain_default)
        return lookup
    main_arms = load_tuple_arms("generated_action_runtime_selection.rs", "pub(super) fn evaluate_expression(", graph)
    ground_arms = load_tuple_arms("generated_action_runtime_ground_selection.rs", "pub(super) fn expression(", graph)

    def lookup(mask, idx):
        e = tuple_lookup(main_arms, mask, idx)
        if e == "TRICK":
            gesture = {91: 108, 92: 111, 93: 112}.get(idx)
            return None if gesture is None else "C[%d] or (C[109] and (C[110] or C[36]))" % gesture
        if e == "GROUND":
            return tuple_lookup(ground_arms, mask, idx)
        return e
    return lookup


def main():
    graph, path = sys.argv[1], sys.argv[2]
    native = json.loads(Path(path).read_text())
    load_ids()
    lookup = port_expression(graph)
    matched, pending, mismatches = 0, 0, []
    fills = [lambda k: False, lambda k: True,
             lambda k: random.Random(1000 + k).random() < 0.5,
             lambda k: random.Random(2000 + k).random() < 0.5]
    for mask in ("pre", "sustain"):
        for idx_text, case in sorted(native[mask].items(), key=lambda kv: int(kv[0])):
            idx = int(idx_text)
            if "error" in case:
                print("native extraction error", mask, idx, case["error"])
                continue
            e = lookup(mask, idx)
            if e in (None, "PENDING"):
                pending += 1
                continue
            bad = []
            for assignment, result in case["paths"]:
                for fill in fills:
                    got = evaluate(e, assignment, fill)
                    if got != result:
                        bad.append((assignment, result, got))
                        break
            if bad:
                mismatches.append((mask, idx, e, bad[:3]))
            else:
                matched += 1
    print(f"{graph}: matched={matched} unported={pending} mismatches={len(mismatches)}")
    for mask, idx, e, bad in mismatches:
        print(f"== {mask} {idx}: port {e}")
        for assignment, result, got in bad:
            print(f"   native={result} port={got} on {assignment}")


if __name__ == "__main__":
    main()
