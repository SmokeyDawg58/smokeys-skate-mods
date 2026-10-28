# MotionGraph `HasIntent` and `WillExpire`

Date: 2026-09-07. This checkpoint reconstructs two concrete condition bodies
used by the compiled onboard MotionGraph. It is intentionally narrower than a
functional generated onboard controller or animation system.

## Evidence boundary

The formulas, field layouts, constructor order, and collaborator call order are
derived from the matched Skate 2 debug XEX/PDB/MAP and PPC instructions. The
mounted `MotionGraph_OnBoard.stategraph` supplies an independent corpus oracle.

| Artifact | SHA-256 |
|---|---|
| Corrected semantic proof | `BB434DF3A561779BAB3F03F0881EE382CDD08CD5F8E2BEFF6F573734ACD8C73F` |
| Implementation-audit packet | `B7B87AC37EF26A9F133E088D1C6BE641EF3A466D804FB4C80EC63647CEDA71B6` |
| Main raw instruction packet | `6398A335359DF88FDF1299D823AAFA0643294791BB6CC90E424F2997C28B73B2` |
| Interface/layout raw packet | `5F246F7B58A9AD14156784F34F2A790E005D4B5A102D1675F3E16C4709B61080` |
| Supplemental raw packet | `32045E6FE67AA5F471D9D80C68FAD7A97D9A0D9AB25719E8F0E2DE453FC209D2` |
| Mounted Motion payload | `E5F317CCAC9FAD0D126B3DF6810C9602CBB081F23FE39FB44662C92D4F13F7ED` |
| Focused mounted oracle | `9716F1E652D4CA7F5150BEAB48612BEC97488E9209FED1A19DDCE38D3B4DBED6` |

The focused oracle validates 2,402 leaf occurrences and 165 direct
`PlayAnimation` Params. This implementation consumes only the 815 `HasIntent`
and 780 `WillExpire` records; `PlayAnimation` remains research evidence for the
next checkpoint.

## Compiled attribute identity

Compiled attributes retain independent byte-string, raw IEEE-754 float, and
raw bool-byte fields. Lookup computes `stringhash32` and returns the first
serialized attribute with that hash. It does not compare diagnostic key bytes,
parse the text field, or replace an earlier duplicate. Tests include both a
same-key duplicate and the distinct `a05763`/`a063d3` collision at
`0xFE11CD43`.

`GetBool` returns the stored byte. Consequently `mAbs` and
`mWaitForTransitions` are represented as `u8`, retain values such as `0x80`
and `0xA5`, and are interpreted as true only by a nonzero test. Float storage
uses `f32::from_bits`, preserving NaNs and signed zero.

## `HasIntent`

The 0x40-byte native object contains comparison operator at `+0x1C`, reference
float at `+0x20`, raw abs byte at `+0x24`, and intent name at `+0x28`.
Construction preserves the long-form `comparison` branch, its required
`value` read before enum validation, exact case-sensitive enum strings, the
six shorthand keys in native precedence order, NOP default, final `abs` read,
and required `intent`.

Activation preserves the cached/generic `IMotionGraph` identity check,
nonnull requirement, `GetMotionGraphIntents_2`, named intent lookup, miss
short-circuit, and NOP-hit short-circuit. The six comparisons reproduce PPC
`fcmpu` branches. In particular, unordered/NaN is false for equals, greater,
and less, but true for not-equal, greater-equal, and less-equal. Absolute value
clears only the input operand's sign bit.

The mounted corpus contains 815 occurrences and 118 distinct intent names:
800 NOP, nine greater, four less, one greater-equal, and one less-equal. Its
canonical occurrence digest is
`BD94466E48246A4F54AA28E24DF715904B7E3D67ABA8C0289E4477867A6478E8`.

## `WillExpire`

The 0x48-byte native object stores time at `+0x1C`, a 37-byte tag buffer at
`+0x20`, and the raw wait byte at `+0x45`. Construction reads exact `InTime`,
then `waitForTransitions`, initializes an empty tag, and only then handles
exact `InTimeTag`. The host rejects embedded NUL and more than 36 payload bytes
instead of reproducing the native unchecked overwrite; that is explicit host
safety policy.

Activation resolves a nonempty special tag before `IAnimatable`, validates the
cached/generic interface identity, and calls `Transitioning` unconditionally.
Only afterward can a nonzero wait byte and a transitioning animation return
false. `GetAnimProp` is called once on the remaining path. Its raw expired byte
short-circuits true; otherwise PPC `ble` is represented as not-greater, so
less, equal, and unordered/NaN return true. The playing and expired-by-time
fields are retained in the snapshot but deliberately unused.

The mounted corpus contains 780 occurrences. Exact `InTime` occurs 765 times;
15 wrong-case `inTime` attributes correctly take the `0.0f` default. There are
309 explicit false wait values and 471 true defaults, with no time tags. Its
canonical occurrence digest is
`B594419415B53139AD132E6F1C6B9141F3FF964C9582381AD91FD564C1A65B0D`.

## Integration limits

Entity, intent, animation-info, and animatable access remains behind explicit
collaborator traits. The checked Rust errors model native assertion or unsafe
nonnull assumptions without claiming identical post-assert continuation.

These types are not yet installed in the generic StateGraph leaf factory or a
live entity backend. Generated direct helpers also remain unimplemented: each
must write the current generated State pointer into the concrete generated
condition object's inherited `Base::mParent` at `+0x10` before activation.
That field is distinct from the dynamic arena's immutable structural `BaseId`
and must not be emulated by mutating the parsed graph edge.

No Vault lookup occurs in either condition. No generated frame, animation
playback, skater physics, or end-to-end gameplay parity is claimed here.

## Verification

Fourteen focused tests cover parsing, raw storage, lookup identity, all PPC
comparison predicates, signed zero, NaN, collaborator call order, tag paths,
transition gating, property short-circuits, and checked pointer assumptions.
The external oracle independently passes all 2,402 occurrences / 165 Params.
Two post-fix source audits compared the implementation with the corrected
proof and raw instructions.

| Source | SHA-256 |
|---|---|
| `motion_graph_conditions.rs` | `8BDD3E7A1ABB1747B53CA45EFFCAD8F498EC5F9838D0C4D55B4275939BFA6546` |
| `lib.rs` | `8A83ABF9480E31B26FCC9F241DCBA66536D79B0E750270A5C23B74E1E0732178` |
