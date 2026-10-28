"""Two online consoles' challenge traces side by side (research aid).

Run both games with SKATE2_TRACE_CAREER=1 (each state change carries its round tick,
`career graph <name> @<tick>`), then

  python online_diverge.py A.log B.log

prints the first state change the consoles made differently, with the lines around it. Every
console must enter the same states at the same round ticks (docs/online/facts.md).
"""
import re
import sys

PATTERN = re.compile(r"career graph (\S+) @(\d+): (.*)")


def entries(path):
    out = []
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            m = PATTERN.search(line)
            if m:
                out.append((int(m.group(2)), m.group(3).strip()))
    return out


a, b = entries(sys.argv[1]), entries(sys.argv[2])
print(f"A {len(a)} entries, B {len(b)} entries")
for i, (x, y) in enumerate(zip(a, b)):
    if x != y:
        print(f"first difference at entry {i}:")
        for j in range(max(0, i - 4), min(len(a), len(b), i + 6)):
            mark = "  " if a[j] == b[j] else "!!"
            print(f"{mark} A @{a[j][0]:>6} {a[j][1][:60]:<60} | B @{b[j][0]:>6} {b[j][1][:60]}")
        break
else:
    print("no difference in the common entries")
