# `PlayAnimation` stored-data reconstruction

Date: 2026-09-07. This checkpoint reconstructs
`Sk8::Behaviours::PlayAnimation` construction and ordered `AddParam` parsing.
It does not implement `CreateInstance`, `Begin`, `Update`, animation dispatch,
StateGraph factory routing, or generated-controller wrappers.

## Evidence identity

The matched XEX/PDB/MAP and PPC instructions establish the layouts, defaults,
parse order, packed-string routines, and bool-bit writes. The exact mounted
`MotionGraph_OnBoard.stategraph` provides the independent real-data gate.

| Artifact | SHA-256 |
|---|---|
| Corrected Motion-leaf proof | `BB434DF3A561779BAB3F03F0881EE382CDD08CD5F8E2BEFF6F573734ACD8C73F` |
| Main raw instruction packet | `6398A335359DF88FDF1299D823AAFA0643294791BB6CC90E424F2997C28B73B2` |
| Interface/layout raw packet | `5F246F7B58A9AD14156784F34F2A790E005D4B5A102D1675F3E16C4709B61080` |
| Supplemental raw packet | `32045E6FE67AA5F471D9D80C68FAD7A97D9A0D9AB25719E8F0E2DE453FC209D2` |
| Focused FastString/PPC packet | `5E3EB4391D41E4FCBED685FF576FC2D92996D8A15EF57508FA008BAEBD82B8F1` |
| Mounted Motion payload | `E5F317CCAC9FAD0D126B3DF6810C9602CBB081F23FE39FB44662C92D4F13F7ED` |
| Focused corpus oracle | `9716F1E652D4CA7F5150BEAB48612BEC97488E9209FED1A19DDCE38D3B4DBED6` |

## Constructor storage

The native object is 0xA4 bytes. It contains 16-byte `TransitionParams` at
`+0x1C`, four `FastString<36>` animation names at `+0x2C`, `+0x44`, `+0x5C`,
and `+0x74`, playback speed at `+0x8C`, the Param vector at `+0x90`, and three
alternative-name presence bits at `+0xA0`.

Construction retains BLEND as the default transition type. Only exact
lowercase `play`, `sequence`, and `channelblend` replace it; `blend`, unknown
text, and wrong case remain BLEND without error. Required `anim` follows, then
`switchAnim`, `mirrorAnim`, and `noBoardAnim` in order. An absent alternative
copies the base animation and leaves its presence bit false; a present empty
string packs to zero while retaining a true presence bit.

The remaining reads preserve exact float bits and defaults: playback speed is
`1.0f`, blend time is the `0.2f` bit pattern, and placement begins OVER.
`blendWithCurrentFrame` and `useChannelFromWeights` insert only source bool bit
zero. Thus raw `0x80` is false and raw `0x81` is true. `transitionUnder` uses a
different PPC normalization and maps every nonzero byte, including `0x80`, to
UNDER. The unrelated `trans` attribute is not read.

## Packed FastStrings

Native FastString identity is its packed base-38 words, not original letter
case. Canonical digits, ASCII letters, and underscore map to values 1 through
37; case is folded. The implementation reuses typed five-word/30-character
and six-word/36-character packed values. Tests freeze `Trajectory` and a real
full-capacity shipped animation name against exact words.

The runtime `Andale::nullStringName` initializer packs literal `nullString` as
`[0x75460604, 0x86ADE6D4, 0, 0, 0]`, which is observably distinct from an empty
string. The AttributeParam constructor therefore preserves its asymmetric
defaults: intent is the extended null sentinel, attribute is empty/zero, and
rename is the null sentinel. A present `rename=""` replaces that sentinel with
zero.

Native malformed-input behavior is broader: its packer accepts arbitrary
non-NUL bytes with signed wrapping arithmetic and silently truncates past the
fixed capacity. The public reconstruction rejects noncanonical bytes and
overlength values as checked-host safety policy. All shipped values are in the
canonical alphabet and fit; this restriction is not claimed as native parity
for malformed data.

## Ordered Params

Each 0x4C-byte AttributeParam stores source type, intent, attribute, rename,
raw default float, normalize bit, and use-default bit. Exact, case-sensitive
sources are:

- `intent`: requires `intent` and reads normalize;
- `lastAnim`: requires `attribute` and does not read normalize;
- `filteredIntent`: requires `filteredIntent` and reads normalize.

Normalize again stores only raw bool bit zero. Optional rename is processed
after the selected source. Presence of `defaultValue` sets use-default and
copies its exact float bits independently of the numeric value. `SetBlend` and
all unmentioned attributes are ignored. Parsed Params append in serialized
order. Missing/unknown required fields, capacity/alphabet errors, and reserve
failure are checked-host errors; a failed append leaves the existing vector
unchanged.

## Mounted gate

The application reads the exact 641,241-byte mounted MotionGraph, parses its
binary tree, and walks every element in true preorder. For exact
`PlayAnimation` behaviour records it invokes the concrete constructor and then
feeds only direct `Param` children in serialized order. Diagnostic
canonicalization retains preorder index, structural locator, owning State
path, ordered raw attributes, child tags, and direct Param subtrees.

The mounted release probe constructed all 807 behaviours and all 165 Params.
It reproduced the independent occurrence digest
`AFFB45C92FD711D67F55296A7A938B71D4CA34969A9821C23563EBEE4050600B`
and Param-tree digest
`44A5BBD22961EAEE9183078B7ED9D66B9D254D6FCE5B577981E45FDBCD320A1C`.
The exact distributions also match: transitions 3 PLAY / 652 BLEND / 65
CHANNELBLEND / 87 SEQUENCE; alternatives 8 / 28 / 4; 111 last-animation and
54 filtered-intent Params; and zero intent Params. The gate additionally
checks all recovered presence, bool, default, and ignored-attribute counts.

Raw exact-key matching in this diagnostic census intentionally mirrors the
independent oracle builder. The concrete constructor and AddParam parser use
the recovered native hash-only first-match lookup. These roles are kept
separate.

## Verification and limits

Twelve focused core tests cover layouts, defaults, enum strings, hash
collisions, exact packed vectors and sentinels, raw bool behavior, float bits,
all Param branches, source order, ignored fields, and checked failure
atomicity. Five app tests cover corpus traversal and canonicalization. Two
independent code audits passed against the proof and raw PPC packets; two
independent corpus audits passed against the oracle construction. A separate
root release-mode `--probe-archives` run completed successfully against the
supplied data root.

No animation has been played by this checkpoint. Live interface resolution,
transition override, selected-skater construction attribute, alternative
selection from switch/mirror/offboard state, dispatch/fallback, instance
first-update state, and generated wrapper calls remain open.

| Source | SHA-256 |
|---|---|
| `play_animation.rs` | `CB57C737B206B453239D0EC2CD29D16F1DA55DCB49D2DD02D6128A2F799F67E8` |
| `play_animation_corpus.rs` | `3D71051D3F9DDF03532FF6DBB04A9F5BD40C2CB73C1086595708E9F687303C2E` |
| `lib.rs` | `9DE94DD26500DE0862F205EEC4F9CCA75926485CE6C47F1820021730D3E20A1F` |
| `main.rs` | `AAAF0A6CD46A6B21D3B3CB3F5BF95D79314EA5E27851F20978940C4B719E4643` |
