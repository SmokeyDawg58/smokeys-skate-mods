# Living-world PlayRemappedAnimation

Date: 2026-09-07. Confidence: `high` for the constructor and direct lifecycle
on the declared checked host domain; `exact` for the inspected offsets,
constants, and no-op/null bodies. This reconstructs the leaf's calls into
`LWAnimHelper`, not the helper internals or visible animation.

## Source identity and ownership

The matched `sk82_na_m.xex` SHA-256 is
`3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
PDB is `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
MAP is `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.
Original files and raw evidence stay outside the repository under
`world_probe_temp`.

| Evidence | SHA-256 |
|---|---|
| `play_remapped_animation_proof.md` | `55E2D4DDACF2F10D2AD57075B68ADE2DBF8620FF71BCA72B3B70055F2028F950` |
| `play_remapped_animation_raw.txt` | `621EE428819E16EB9A77048BC7C42D9346A6E77A1E6DEB3D07FD1A697C405667` |
| `play_remapped_animation_oracle.json` | `466831C801E35C22ADDA56A8896E2B84CD84B6B8C4DA5E6FDE5F46E07F64EC0B` |
| `motiongraph_test_dynamic_trace_proof.md` | `2D221E4AD5932C505F77D1C6E0FBA9B123AB953AB6F0520FFF421FDD06AD6021` |

The demangled class is `Sk8::Behaviours::PlayRemappedAnimation`. MAP ownership
is `livingworld_lib.obj`; helper calls belong to
`sk8lwpedestrian:sk8lwpedestrian_lib.obj`. Assertion paths identify
`state/LWPedestrian/PedestrianMGBehaviours.cpp` and
`LWPedestrian/anim/LWAnimHelper.cpp`. Folded IDA names are resolved using the
PDB vtable and MAP aliases, not the arbitrary alias at a shared address.

| Native entry | Address | Recovered behavior |
|---|---|---|
| Registration initializer | `0x82F63AA8` | Exact Behaviour key `PlayRemappedAnimation` |
| ClassHelper | `0x8271A750` | Allocate `0xA8`, alignment `0x10`; construct only on success |
| Constructor | `0x82717C60` | Attribute order and stored values below |
| Process / End | `0x825187C0` | Exactly `blr` |
| Inherited AddParam | `0x82CEEC78` | Assertion / DebugBreak |
| Default CreateInstance | `0x82BB3DA8` | Return null, no instance allocation |
| Begin | `0x82718068` | Optional raw mirror write, then Setup |
| Update | `0x826F33F0` | One Update helper tail call |
| Setup helper | `0x82E90730` | Explicit unreconstructed collaborator |
| Update helper | `0x82E90B50` | Explicit unreconstructed collaborator |

The vtable is at `0x820D3F9C`: Process `+0x24`, AddParam `+0x28`,
CreateInstance `+0x2C`, Begin `+0x30`, Update `+0x34`, End `+0x38`.
The deleting destructor at `0x826FF508` performs base destruction and
conditional StateGraphFree, with no derived animation cleanup.

## Stored semantics

PDB size is `0xA8`, including a `0x1C` Behaviour base. The Rust type is a
semantic representation and makes no native ABI or padding claim.

| Offset | Field | Default |
|---|---|---|
| `+0x1C` | TransitionType | Blend (`2`); Play `0`, Sequence `1` |
| `+0x20` | StateAnimationName, `char[37]` | Required `anim` |
| `+0x45` | FrameOffsetTag, `char[37]` | Empty |
| `+0x6C` | FrameOffset | `0x00000000` |
| `+0x70` | CycleSwappingPercentageTag, `char[37]` | Empty |
| `+0x98` | CycleSwappingPercentage | `0x00000000` |
| `+0x9C` | PlaybackSpeed | `0x3F800000` |
| `+0xA0` | BlendLength | Blend: `0x3DCCCCCD`; otherwise positive zero |
| `+0xA4/+0xA5` | HasMirrorOverride / MirrorOverride | Absent / raw zero |

Constructor lookup order is transType, anim, playBackSpeed, frameOffset,
frameOffsetTag, cycleSwappingPercentage, cycleSwappingPercentageTag,
Blend-only blendTime, then mirrorOverride. Attribute lookup is hash-only and
first-match. Floats retain all 32 bits. GetBool at `0x82CF3678` returns the
stored byte, so mirror is `Option<u8>`: absent differs from present zero.

The three C arrays accept arbitrary non-NUL bytes through 36 bytes in the
checked host representation. Overlong values and embedded NUL are explicitly
rejected instead of reproducing overflow or first-NUL truncation. transType
uses a distinct FastString<30> domain: reject embedded NUL, overlength, and
bytes outside `[A-Za-z0-9_]`, then compare play/sequence/blend without ASCII
case sensitivity. Invalid-character errors preserve byte position and value.
Native packing at `0x824CA698` / `0x824B1A78` admits malformed aliases and
truncation; host rejection is a documented deviation, not native parity for
malformed data. Unknown canonical transitions and missing anim stop at a
checked assertion boundary instead of continuing after DebugBreak.

## Lifecycle and authority

Begin optionally writes the exact mirror byte through Entity `+0x1A14` to
target `+0x5F6C`, then calls Setup with Entity followed by these eight semantic
arguments: animation, playback, frame tag, frame offset, cycle tag, cycle
percentage, transition, blend. The PPC at `0x827180DC..0x82718100` establishes
the order; the decompiler's ABI annotations are misleading. The external
two-tick proof's abbreviated Setup display omits playback, but the full source
proof and production request include it.

Update passes only Entity, animation, and playback. Neither callback reads an
instance. Checked collaborator failures preserve completed effects, including
a mirror write before failed Setup. Factory Params return a typed inherited
assertion error. Process and End have no effects. Callback identity validation
precedes service/allocation work; the authority session additionally validates
Context binding before Context-bearing effects. CreateInstance returns `None`
without calling the PlayAnimation allocator.

The facade supplies the same privately issued Entity authority to mirror,
Setup, and Update. It does not establish a live pedestrian, native pointer,
UCOM interface, remapping table, random choice, or animation engine.

## Mounted oracle and two-tick boundary

The four original graphs contain 206 Pedestrian, one test, five Marquee, and
nine Photographer leaves: 221 total and zero Params. Their reconstructed
semantic JSON digest is frozen as
`505513295F907F203040CF50DBC68811883490427F39488E923AD1DF438A81CC`.
This oracle is independently derived from serialized data; it is not a
captured native runtime trace.

The minimal graph is the 183-byte
`data\state\livingworldentities\pedestrian\motiongraph_test.stategraph` in
`data\big\miscload.big`, header index 143, raw offset `0x35980`, SHA-256
`23FFF26C4BB98ED4BD821F49DBDE9937EE925D605D4F5B8D3CECFF3AC2460A9D`.
Its exact tree is Motion -> Locomotion -> PlayRemappedAnimation, anim
`FACE_HAND_TEST`. Both State policy attributes are present on the wire;
the recovered State constructor does not interpret them as new policy objects.
Process visits `[State(0), State(1), Leaf(0)]`; preflight admits one typed
leaf with zero diagnostics.

With deltas `0.25` then `0.5`, the first tick selects State 1 and performs
Create(None), Setup, Update. Both newly entered timers stay positive zero and
the active pair retains a null instance. The second tick performs only Update;
current and last are State 1 and both timers become exactly `0.5`. No allocator,
mirror, condition, hook, or unrelated animation service is needed. This is
explicit generic dynamic dispatch, not proof of the generated onboard route,
production frame timing, or visible animation.

Acceptance commands and final independent audit results are recorded in the
research log when the checkpoint passes its strict workspace and mounted gates.
