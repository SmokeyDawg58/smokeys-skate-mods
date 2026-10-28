# BehaviourGraph unprocessed runtime construction

## Proven boundary

The ten shipped living-world plugin descriptors now pass from the checked
compiled decoder into a typed, owned construction snapshot. The implementation
matches the final object tree produced by `BehaviourParser::StartProcessingElement`
for this corpus:

- tag and attribute dispatch uses only recovered `stringhash32` values;
- colliding attributes are first-in-wire-order wins;
- the five plugin and fourteen condition factory names compare without ASCII
  case, while nested operator text remains exact and case-sensitive;
- the first expression under a plugin is always `None`, even when its serialized
  `op` is `and`; nested `and`, `or`, `not`, and omitted operators map to the four
  recovered values;
- plugin file strings remain bare `state/...` paths, conversation stores three
  participants, and the other nine use the constructor default of one;
- recognizer count conversion produces `4`, waypoint distance retains
  `0x40a00000`, and the single-precision odds calculation produces
  `0x3d8f5c29` from serialized `7.0` and multiplier bits `0x3c23d70a`;
- all six missing crosswalk distances remain explicitly `Unwritten`. No zero or
  allocator-dependent value is invented.

The mounted corpus freezes every complete tree and also independently checks
the aggregate operator, plugin-class, and condition histograms. It checks the
otherwise discarded serialized inputs: nine top-level `and` attributes versus
the attribute-less `lookat` root, the exact source float bits, the nine absent
participant attributes, and the ignored newspaper-box radius `10.0`.

## Exact corpus result

The reference archive run constructs:

- 10 plugins, 39 expressions, and 41 conditions (100 structural nodes);
- operators `None=12`, `Not=27`, `And=0`, `Or=0` after native construction;
- plugins `UseWorldProp=6` and one each of `ActorTrackerSpectate`, `LookAt`,
  `SpectatorWaypointGeneric`, and `PedestrianConversation`;
- 12 total participant slots.

Each descriptor's class, bare file path, participant value, child order,
condition identities, and meaningful stored bits match the frozen runtime-tree
oracle.

## Deliberate limits

This type is an unprocessed final-tree snapshot, not a byte-for-byte C++ object
layout or parser-stack emulator. Native constructor-time attachment, intrusive
reference counts, push/pop events, and partial-failure side effects are not
represented. Malformed inputs receive checked host errors. In particular, this
corpus boundary requires exactly one plugin, while native parsing retains the
first nonnull root and may still construct later plugins. Repeated top
expressions and unknown leaf tags are also rejected here even though native
code can replace/release the prior expression or push/pop a null leaf. Runtime
recursion enforces the decoder's checked maximum zero-based wire depth of
1,024 for publicly constructed trees.

`TransferableBehaviour::Process`, controller-wrapper allocation, its exact
`HierarchicalController::Load` call, generated/dynamic controllers, and all
condition activation functions remain separate work. Nothing here claims that
the living-world state machines execute yet.

## Evidence

- factory/object-stack/Process proof packet:
  `6EE60E9167B7B262B92770974171B9D22010B18557E332A2F1677198EB1784CF`;
- ten-tree oracle:
  `97B6204673510C330D2E26FB14BE5310C84609C4EFB142018773DD5F237A238F`;
- compiled decoder packet:
  `0884392FD9FA945A98701E897EEB90FC2C8BFE7CEED517989C6257AD1ACF991E`;
- supplied `miscload.big`:
  `0512157AAADD88A6E1992509944D0AAFF49EFAD2DB7BDD8C32AA4774A92F7F13`;
- debug XEX:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`.
