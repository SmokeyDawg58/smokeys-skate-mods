"""Generates the park editor's Skate 3 piece catalog (each keyed by its Skate 3 asset ID) from parkassets.big's recipes and the
catalog dump (park_assets_catalog_dump).

Usage: SKATE3_PARKASSETS=<parkassets.big extracted> python tools/gen_park_pieces.py <dump.tsv>
           crates/skate2-core/src/ui/object_dropper_park_pieces.rs <props rows out> <Skate 2 prop IDs, comma separated>
"""
import os, re, sys, collections

ROOT = os.path.join(os.environ["SKATE3_PARKASSETS"], "data", "content") + os.sep
DUMP = sys.argv[1]
OUT_TABLE = sys.argv[2]
OUT_PROPS = sys.argv[3]
EXISTING = set(int(x, 16) for x in sys.argv[4].split(",")) if len(sys.argv) > 4 else set()

# (recipe composite, tab, subcategory shown)
GROUPS = [
    ("PreMade", 0, "Premade"),
    ("Street", 0, "Street"),
    ("Vert", 0, "Vert"),
    ("Hubbas", 0, "Hubbas"),
    ("Rails", 0, "Rails & ledges"),
    ("Plaza", 0, "Plaza"),
    ("Mega", 0, "Mega"),
    ("Misc", 0, "Catwalks & misc"),
    ("Branded", 1, "Branded"),
    ("DownTown", 1, "Downtown"),
    ("Foliage", 1, "Foliage"),
    ("OTS", 1, "Big objects"),
    ("DMO_Barriers", 2, "S3 barriers"),
    ("DMO_Bins", 2, "S3 bins"),
    ("DMO_Boxes", 2, "S3 boxes"),
    ("DMO_Misc", 2, "S3 misc"),
    ("DMO_Rails", 2, "S3 rails"),
    ("DMO_Ramps", 2, "S3 ramps"),
    ("DMO_Tables", 2, "S3 tables"),
]

dump = {}
for line in open(DUMP, encoding="utf-8"):
    c = line.rstrip("\n").split("\t")
    folder, stem = c[0], c[1]
    lo = eval(c[3]); hi = eval(c[4]); rlo = eval(c[5]); rhi = eval(c[6])
    if not c[11]:
        continue
    model = int(c[11].split(":")[0])
    dump[int(stem, 16)] = (folder, stem, lo, hi, rlo, rhi, model, int(c[2]))

mods = {}
for xml in ("recipe/static/static_db_xbfdba595_000.xml", "recipe/dynamic/dynamic_db_x946ad45d_000.xml"):
    s = open(ROOT + xml, encoding="utf-8", errors="replace").read()
    for comp, body in re.findall(r'<comp n="([^"]*)">(.*?)</comp>', s, re.S):
        for name, mbody in re.findall(r'<mod id="0x[0-9a-f]+" n="([^"]*)">(.*?)</mod>', body, re.S):
            arena = re.search(r'<lod idx="0" id="0x[0-9a-f]+" arenaid="(0x[0-9a-f]+)"', mbody)
            if arena:
                mods.setdefault(comp, []).append((name, int(arena.group(1), 16)))

THEMES = {"downtown": "Downtown", "industrial": "Industrial", "university": "University"}
SIZES = {"lrg": "large", "lg": "large", "large": "large", "xlrg": "extra large", "md": "medium", "med": "medium", "medium": "medium",
         "sml": "small", "sm": "small", "small": "small"}
DROP = {"dmo", "glbl", "gl", "ind", "un", "dt", "skatepark"}


def words(token):
    token = re.sub(r"([a-z0-9])([A-Z])", r"\1 \2", token)
    token = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1 \2", token)
    token = re.sub(r"([a-zA-Z])(\d)", r"\1 \2", token)
    return token.split()


def display(name):
    parts = name.split("_")
    if re.fullmatch(r"[a-z]{2}", parts[0]) or re.fullmatch(r"[A-Z]{2}", parts[0]):
        parts = parts[1:]
    theme, size, rest = None, None, []
    for part in parts:
        low = part.lower()
        if low in THEMES:
            theme = THEMES[low]
        elif low in SIZES:
            size = SIZES[low]
        elif low in DROP:
            continue
        else:
            rest.extend(words(part))
    text = " ".join(rest).strip() or name
    text = text[0].upper() + text[1:].lower()
    extra = [x for x in (size, theme) if x]
    return f"{text} ({', '.join(extra)})" if extra else text


pieces = []  # (tab, sub, display, path, model, lo, hi)
seen_ids = set()
skipped = []
for comp, tab, sub in GROUPS:
    rows = sorted(mods.get(comp, []), key=lambda m: m[0].lower())
    names = collections.Counter()
    for name, arena in rows:
        if arena not in dump:
            skipped.append((comp, name, "no model"))
            continue
        if arena in EXISTING:
            skipped.append((comp, name, "already a Skate 2 prop"))
            continue
        if arena in seen_ids:
            continue
        seen_ids.add(arena)
        folder, stem, lo, hi, rlo, rhi, model, tris = dump[arena]
        kind = "dynamic" if folder.startswith("dmo") else "static"
        path = f"data\\\\content\\\\{kind}\\\\model\\\\{kind}\\\\{folder}\\\\{stem}.rx2"
        label = display(name)
        names[label] += 1
        both_lo = [min(a, b) for a, b in zip(lo, rlo)]
        both_hi = [max(a, b) for a, b in zip(hi, rhi)]
        pieces.append([tab, sub, label, path, model, both_lo, both_hi, name, arena])
    # Repeated labels in a subcategory get their recipe name's number.
dup = collections.Counter((p[1], p[2]) for p in pieces)
count = collections.Counter()
for p in pieces:
    if dup[(p[1], p[2])] > 1:
        count[(p[1], p[2])] += 1
        p[2] = f"{p[2]} {count[(p[1], p[2])]}"


def f(v):
    return "[" + ", ".join(f"{x:.3f}" for x in v) + "]"


with open(OUT_TABLE, "w", encoding="utf-8", newline="\n") as out:
    out.write("// Generated from Skate 3's parkassets.big recipes (static_db, dynamic_db) by tools/gen_park_pieces.py; do not edit.\n")
    out.write("pub const PARK_PIECES: &[ParkPiece] = &[\n")
    for p in pieces:
        out.write(f'    ParkPiece {{ id: 0x{p[8]:016x}, tab: {p[0]}, recipe: "{p[7]}", path: "{p[3]}", model: {p[4]}, min: {f(p[5])}, max: {f(p[6])} }},\n')
    out.write("];\n")
with open(OUT_PROPS, "w", encoding="utf-8", newline="\n") as out:
    for p in pieces:
        out.write(f'    ("{p[1]}", "{p[2]}", 0x{p[8]:016x}),\n')
print(len(pieces), "pieces;", collections.Counter(p[1] for p in pieces))
print("skipped", collections.Counter(s[2] for s in skipped))
for s in skipped[:10]:
    print("  ", s)
