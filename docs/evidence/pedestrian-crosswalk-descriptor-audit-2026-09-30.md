# Crosswalk distance: independent packaged-descriptor audit

The missing distance is present in the original assets, not introduced by the
host parser. No radius can be recovered from these descriptors. Confidence:
**exact** for serialized omission and native attribute lookup; no claim about
the original allocation's incidental bytes.

Original archive: `<game folder>/data/big/miscload.big`.
Read-only extraction used the existing `tools/extract_big.py`, filter
`PluginDescriptor`, into a scratch folder
outside the repository.
The archive contains 24 matching entries: ten compiled descriptors and fourteen
XML descriptors. Original assets remain external to the repository.

All paths below are relative to `data/livingworld/PluginDescriptor/` in that
archive. Offsets identify the beginning of the compiled `TransferCondition`
element after RefPack decompression. Every compiled descriptor was decoded
independently with the native recursive wire grammar and consumed exactly to
EOF, including all attribute float words and boolean bytes.

| Descriptor | XML line | Compiled size | Condition offset |
|---|---:|---:|---:|
| `sit.xml` / `sit.stategraph` | 17 | 635 | `0x1f2` |
| `useatm.xml` / `useatm.stategraph` | 16 | 584 | `0x1bf` |
| `usenewspaperbox.xml` / `usenewspaperbox.stategraph` | 16 | 608 | `0x1d7` |
| `usetrashbin.xml` / `usetrashbin.stategraph` | 14 | 525 | `0x184` |
| `usevendingmachine.xml` / `usevendingmachine.stategraph` | 16 | 595 | `0x1ca` |
| `usewaterfountain.xml` / `usewaterfountain.stategraph` | 16 | 594 | `0x1c9` |

Each XML element is exactly `<TransferCondition name="IsNearCrossWalk"/>`.
Each corresponding binary element has attribute count **1**, containing only:

```
name: name
text: IsNearCrossWalk
float_bits: 0x00000000
boolean_raw: 0
child_count: 0
```

The zero float word belongs to the `name` attribute's serialized numeric view;
it is not a `distance` attribute or a default distance. The native string hashes
are distinct: `name=ba41a46a`, `distance=c5857615`.

All six conditions occur at:
`BehaviourSource / BehaviourPlugin(name=UseWorldProp) /
TransferExpression(op=and) / TransferExpression(op=not) / TransferCondition`.
The other clauses check hand-prop/waypoint eligibility, `not IsOnRoad`, and
`not IsChasing`; sit also checks `InFrontOfFirstWaypoint` and `CanSitWithHandProp`.
None of these ancestors supplies `distance`. The newspaper plugin has its own
`radius="10"` attribute, which is neither named distance nor inherited by the
condition. There are no XML includes or external attribute references here.

Compiled descriptor SHA-256, in table order:

```
4495acda14d7f2db8a3dd88df63a727c480e7ac867352639f93bb7e50c8dfd6d
df380683f9e804e7862b3f20d587b6dc208893189943639babc1cfadb82b691b
d9faf9e190dba4359ee6057c670207844dba35a96236f68cf922fd5f0ad3d24a
68f70ab92be06e6d6daf48bde84e77f69e6df213b9ea2cad7cfa031c2aee66aa
62a1ea2ae91479122633688ef27fc2d935204f5357920d97b62675d7c3dba2e6
296261309f444f86fb957de6e58e9efef21926f88d0e5c42ba6cfad1f27f65a1
```

Native parser evidence:

- `BehaviourParser.ReadElement82cf6070` reads the element string, constructs a
  fresh stack-local `BinaryAttributeMap`, calls StartProcessingElement with that
  map, then reads the child count and recursively repeats. Ancestor maps are not
  merged into child maps.
- `BinaryAttributeMap.ctor82cf4298` initializes an empty fixed map, reads the
  explicit count, then exactly that many name/string/float-word/boolean-byte
  tuples. It synthesizes no attributes.
- `StartProcessingElement82cf5730` passes the same current-node map and the
  parent expression separately to the condition factory at `82cf5988`.
- BinaryAttributeMap vtable `82319078` slot0 binds folded
  `XMLAttributeMap.HasAttribute82cf5e68`; that implementation hashes the queried
  name and searches only `this->mMap`. There is no fallback/inheritance/default
  query.
- `IsNearCrossWalk.ctor826fd468` calls HasAttribute(distance), skips the write
  when false (`826fd4c8`), and only on true calls GetFloat and stores at object
  `+0x24` (`826fd4e4`). The existing host `NativeOptionalF32::Unwritten` correctly
  represents this omission.

This investigation did not repeat the allocator audit. Combined with the
previously established absence of allocation clearing and base-constructor
writes, the native predicate reads unspecified prior allocation contents when
it reaches the corner-distance comparison. Recovering deterministic intended
behavior would require additional original-runtime evidence or a deliberate
documented compatibility policy; the assets and parser do not justify zero,
ten, or any other replacement radius. No implementation radius was added.
No compilation, tests, or game runtime was run.

## Approved compatibility policy

The chosen option is **Use a documented zero-distance fallback**.
`selected_living_world_transfer.rs` now resolves only `NativeOptionalF32::Unwritten`
to `0.0` when evaluating IsNearCrossWalk. Written float bits are unchanged, and
the parser still distinguishes omitted values from authored zero. This is a
deterministic compatibility policy for an original uninitialized field, not a
recovered native default or proof of original runtime behavior.

The existing intersection query, four-corner XYZ comparison and expression
short-circuiting remain unchanged. With zero distance, the predicate can still
return true at an exactly coincident corner; it is not replaced with a constant
false result. Compilation and gameplay validation remain pending.
