"""Read-only query over skate2-recomp generated Motion/Action graph data.

Usage:
  python graph_query.py [motion|action] expr N      states/transitions using expression N
  python graph_query.py [motion|action] state N     one state: ancestry, behaviours, transitions
  python graph_query.py [motion|action] update ADDR all states sharing a native update (hex or decimal)
  python graph_query.py [motion|action] cond N [M..] condition declarations + attributes
  python graph_query.py [motion|action] name TEXT   states whose name contains TEXT
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "crates/skate2-core/src/animation/graphs"
FILES = {
    "motion": ROOT / "motion/generated_motion_load_data.rs",
    "action": ROOT / "action/generated_action_load_data.rs",
}
NULL = 65535
SECTION = re.compile(r"(\w+):\s*Section\s*\{\s*count:\s*(\d+),\s*index:\s*(\d+),?\s*\}")
STRING = re.compile(r'(\w+):\s*"((?:[^"\\]|\\.)*)"')
SCALAR = re.compile(r"(\w+):\s*(0x[0-9a-fA-F_]+|-?\d+|true|false)\b")


def array_body(text, name):
    match = re.search(rf"\b{name}:\s*\[[^\]]*\]\s*=\s*\[", text)
    if not match:
        return None
    start = match.end()
    depth = 1
    for position in range(start, len(text)):
        if text[position] == "[":
            depth += 1
        elif text[position] == "]":
            depth -= 1
            if depth == 0:
                return text[start:position]
    raise ValueError(name)


def blocks(body, type_name):
    result = []
    for match in re.finditer(rf"\b{type_name}\s*\{{", body):
        depth = 1
        position = match.end()
        while depth:
            if body[position] == "{":
                depth += 1
            elif body[position] == "}":
                depth -= 1
            position += 1
        result.append(parse_fields(body[match.end() : position - 1]))
    return result


def parse_fields(block):
    fields = {}
    for key, count, index in SECTION.findall(block):
        fields[key] = (int(count), int(index))
    block = SECTION.sub("", block)
    for key, value in STRING.findall(block):
        fields[key] = value
    block = STRING.sub("", block)
    for key, value in SCALAR.findall(block):
        if value in ("true", "false"):
            fields[key] = value == "true"
        else:
            fields[key] = int(value.replace("_", ""), 0)
    return fields


def load(graph):
    text = FILES[graph].read_text(encoding="utf-8")
    data = {
        "attributes": blocks(array_body(text, "ATTRIBUTES"), "Attribute"),
        "behaviours": blocks(array_body(text, "BEHAVIOURS"), "BehaviourDeclaration"),
        "conditions": blocks(array_body(text, "CONDITIONS"), "ConditionDeclaration"),
        "states": blocks(array_body(text, "STATES"), "StateDeclaration"),
        "transitions": blocks(array_body(text, "TRANSITIONS"), "TransitionDeclaration"),
        "indices": [int(v) for v in re.findall(r"\d+", array_body(text, "INDICES"))],
    }
    return data


def attrs(data, section):
    count, index = section
    if count == 0 or index == NULL:
        return []
    return [(a["key"], a["value"]) for a in data["attributes"][index : index + count]]


def ancestry(data, state):
    chain = []
    while state != NULL:
        chain.append(state)
        state = data["states"][state]["parent"]
    return list(reversed(chain))


def label(data, state):
    return f"{data['states'][state]['name']}{state}"


def owners_of_transition(data, transition):
    owners = []
    for number, state in enumerate(data["states"]):
        count, index = state["transitions"]
        for position in range(index, index + count):
            if data["indices"][position] == transition:
                owners.append(number)
    return owners


def behaviour_slots(data, state):
    slots = []
    for ancestor in ancestry(data, state):
        count, index = data["states"][ancestor]["behaviours"]
        slots.extend(range(index, index + count))
    return slots


def describe_behaviours(data, state, indent="    "):
    for ancestor in ancestry(data, state):
        count, index = data["states"][ancestor]["behaviours"]
        for slot in range(index, index + count):
            b = data["behaviours"][slot]
            extra = attrs(data, b["attributes"])
            print(f"{indent}[{slot}] {b['name']}  (from {label(data, ancestor)}) {extra if extra else ''}")


def show_state(data, number, verbose=True):
    s = data["states"][number]
    print(f"state {label(data, number)}: parent={s['parent']} expr={s['expression']} active={s['active']} "
          f"interrupt={s['interrupt_type']}/{s['interrupt_target']} update=0x{s['update_address']:08x}")
    print("  ancestry: " + " -> ".join(
        f"{label(data, a)}(expr {data['states'][a]['expression']})" for a in ancestry(data, number)))
    count, index = s["children"]
    if count and index != NULL:
        print("  children: " + ", ".join(label(data, c) for c in range(index, index + count)))
    if verbose:
        print("  ordered behaviour slots:", behaviour_slots(data, number))
        describe_behaviours(data, number)
        count, index = s["transitions"]
        for position in range(index, index + count):
            t_index = data["indices"][position]
            t = data["transitions"][t_index]
            print(f"  transition {t_index}: priority {t['priority']} -> {label(data, t['target'])} expr {t['expression']}")


def main():
    graph, command, *args = sys.argv[1:]
    data = load(graph)
    if command == "expr":
        n = int(args[0])
        for number, s in enumerate(data["states"]):
            if s["expression"] == n:
                print("STATE guard:")
                show_state(data, number, verbose=False)
        for number, t in enumerate(data["transitions"]):
            if t["expression"] == n:
                owners = ", ".join(label(data, o) for o in owners_of_transition(data, number))
                print(f"TRANSITION {number}: priority {t['priority']} from [{owners}] -> {label(data, t['target'])}")
    elif command == "state":
        show_state(data, int(args[0]))
    elif command == "update":
        address = int(args[0], 0) if args[0].startswith("0x") else int(args[0])
        print(f"update 0x{address:08x} = {address}")
        for number, s in enumerate(data["states"]):
            if s["update_address"] == address:
                show_state(data, number)
    elif command == "cond":
        for n in map(int, args):
            c = data["conditions"][n]
            print(f"condition {n}: {c['name']} {attrs(data, c['attributes'])}")
    elif command == "name":
        for number, s in enumerate(data["states"]):
            if args[0].lower() in s["name"].lower():
                print(f"{label(data, number)} expr={s['expression']} update=0x{s['update_address']:08x} "
                      f"ancestry={'/'.join(data['states'][a]['name'] for a in ancestry(data, number))}")


if __name__ == "__main__":
    main()
