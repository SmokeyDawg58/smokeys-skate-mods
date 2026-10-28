# `PlayAnimation` runtime lifecycle

Date: 2026-09-07. This checkpoint reconstructs the observable standalone
`CreateInstance`, `UpdateAttributes`, `Begin`, `Update`, and `End` lifecycle of
`Sk8::Behaviours::PlayAnimation`. Animation playback, intent production,
interface discovery internals, dynamic/generated controller adapters, and
animation-resource loading remain explicit collaborators or open work.

## Evidence identity

The control flow, call order, constants, layouts, and raw-value behavior come
from the matched XEX/PDB/MAP and exact PPC instruction ranges. There is no
original-hardware execution trace for this checkpoint.

| Artifact | SHA-256 |
|---|---|
| Lifecycle proof | `0239DB0A94377DCEB7B128B15E4C676771727D237FE468F66DEBBA17BF2405DE` |
| Lifecycle raw PPC/type dump | `93C8B2C39076E1653E7845FF8C412D3C8255EF5FAB348BCF1490E60115555145` |
| Lifecycle dump script | `6BF5E0DC78A156FE28D4150B1CB1F6E79480C7C0D7A49307C1DB18349DEA1A05` |
| FastString 36-to-30 proof | `ADE3AE26DC034A953D8A53980519713B2D358B82647F1D2C326B35B79E1B7DB0` |
| FastString conversion raw dump | `F2B60A984558A937D599AD45981C33793391B3011D5EFF18884F41F50211C750` |
| FastString conversion script | `1791692B38D6A9C19AFE6E420F5DF5DB5FC264704878E7C8A36501EDF8D38C8A` |
| Corrected constructor/leaf proof | `BB434DF3A561779BAB3F03F0881EE382CDD08CD5F8E2BEFF6F573734ACD8C73F` |

Relevant functions are folded `CreateInstance` at
`0x82CBA730..0x82CBA7F0`, `UpdateAttributes` at
`0x82CC92D0..0x82CC9818`, `Begin` at `0x82CC9818..0x82CC9F30`, `Update` at
`0x82CCB840..0x82CCB888`, and folded no-op `End` at
`0x825187C0..0x825187C4`.

## Instance and checked boundary

The native creator requests 12 bytes named `unknown` at alignment 16. A null
allocation returns a null intrusive pointer. Success initializes refcount zero,
sets the byte at `+0x08` to one, installs the folded Data vtable, publishes the
pointer, and increments the refcount to one. The semantic host instance owns
only the observable private `first_update` byte and is deliberately not
cloneable or copyable. Ordinary Rust ownership does not claim native vptr,
atomic/MSR, destructor, or uninitialized padding equivalence.

Native assertions and unchecked null dereferences are exposed as checked
errors. Backend failures are a separately labeled host extension. Completed
effects are never rolled back. The operation backend represents one future
entity-bound adapter and records exact resolution/call order; it is not itself
claimed as a UCOM or animation implementation.

## Attribute projection

Every call first resolves Animatable, MotionGraph, and SkaterMotionGraph in
that order, even for an empty Param vector. Params then execute in serialized
order.

- Intent and filtered-intent Params derive the destination from the packed
  intent unless rename differs from the exact `nullString` sentinel. A hit
  forwards its raw float bits; a miss optionally forwards the stored default.
  Normalize and sequence `-1` are preserved.
- Last-animation Params do nothing when `updateFromLastAnim` is false after
  the required resolutions. Otherwise the tree query always uses the stored
  attribute, while the destination may use rename. Signed tree types 0 and 2
  forward the first value word, 1 and 3 skip, and all others reach a checked
  native-assertion boundary. Defaults retain raw bits and normalize is always
  false.

The tree callback is explicitly semantic. Native seeds only the first four
bytes of its 24-byte value buffer to positive zero and leaves the other 20
bytes untouched; it also seeds begin/end to `-1`, packed name to zero, flags
to 6, type to 0, and sequence to `-1`. A successful host sample promises the
tree implementation supplied the signed type and first value word.

The exact FastString conversion callee at `0x82594AD0..0x82594B40` and its
nested Read helpers were recovered after audit. It copies packed source words
0 through 4 bit-for-bit and never loads word 5. Thus characters 31 through 36,
or any arbitrary nonzero sixth word, are silently discarded without validation
or status, exactly matching the implementation.

## Begin, selection, and dispatch

`Begin` writes `first_update = 1` before its first collaborator call. It then
resolves Physical, Animatable, SkaterAnim, and MotionGraph in order, followed
by the nested three-resolution attribute pass. Missing interface values are
collected until the native aggregate assertion; Actor cached-pointer and
cached/generic identity failures stop at their earlier assertion boundaries.

After attributes, it reads the selected pro skater and writes the exact
`ProSkater` construction attribute. It initializes the transition local from
the stored semantic type, blend, placement, and dispatch-relevant flag bits,
then reads the next raw four-word override. Override type zero retains those
stored semantics; any nonzero type replaces all four local words. A null
returned pointer fails before clear. Every valid read is followed by
unconditional clear before selection or transition validation. Unnamed stored
flag bits are not modeled because the recovered dispatch never consumes them.

Optional predicates execute only when their stored presence flags are set.
Switch is considered first, mirror can replace it, and a zero raw
holding-skateboard byte can replace both with no-board. The resulting
precedence is `noBoard > mirror > switch > base`; PhysOut and OffBoard null
pointers are distinct checked boundaries.

Raw transition types 1 through 4 dispatch Play, Blend, ChannelBlend, and
Sequence with exact raw float bits, positive-zero time, raw placement, and the
relevant flag bits. Placement is not invented as a validated enum at this
boundary. Type zero or any other invalid selected type errors after clear and
selection without entering fallback. The dispatch return is a raw byte, so
every nonzero value succeeds. A zero primary result dispatches
`KeepDefaultAnim` at `1.0f` and positive-zero time; a second zero reaches the
checked fallback assertion. The native uppercase scratch conversion before
fallback is backend-unobservable and intentionally omitted, so instruction/
stack-memory trace parity is not claimed.

## Update, End, and integration limit

An Update with a nonzero latch skips attributes and then writes zero. A zero
latch performs the three-resolution attribute pass with
`updateFromLastAnim=false`, then writes zero. If that pass errors, the latch was
already zero and remains so. Create plus Begin plus the dynamic controller's
same-frame Update therefore has the recovered skip-and-clear shape. End is an
exact no-op.

This evaluator is not yet installed in a live controller. The dynamic fallback
now has a fallible/poisoning lifecycle path that stops at the exact controller
scheduling boundary instead of silently continuing. A future adapter must bind
all calls to one Context/entity, use a distinct per-activation instance, and
preflight every graph leaf before constructing the controller. The mounted
onboard graph must currently be refused because 1,173 Conditions, 1,662
Behaviours, and 67 Hooks remain opaque.
Generated PlayAnimation wrappers remain a separate parity target, as does the
condition-only generated evaluation-parent write seam. No mounted activation,
animation playback, physics step, or gameplay frame is claimed.

## Verification

Thirteen focused trace and adversarial tests cover allocation/null behavior,
repeated interface groups and failure prefixes, latch lifecycle, every Param
source and tree-type path, raw floats, defaults/renames, override retain/copy/
clear behavior, selection precedence and nulls, all four dispatch signatures,
raw bool return bytes, invalid transitions, fallback, and no-op End. Two
independent source audits passed; the post-audit FastString callee closure
removed the only remaining raw-code gap. Formatting, focused tests, strict
core clippy, and workspace gates pass.

| Source | SHA-256 |
|---|---|
| `play_animation_lifecycle.rs` | `E22F0536216276BCC60F8B36F6071FFD8C610EBEB0D9285B660D44D4FA2D8E88` |
| `lib.rs` | `361808408A70F689C63A69014D95288FC6A7B5CB0E94235B3D28ADC97F1FB04E` |
