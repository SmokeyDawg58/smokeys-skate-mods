# Mongo push foot distance in the selected runtime

**Correction September25:** the deck-X/deck-Z interpretation below was wrong.
Raw VMX128 words and the permutation constant prove flipped deck-Z then deck-Y.
See [corrected source audit](trick-six-errors-push-projection-2026-09-25.md).

The raw-A original-data replay reached Motion condition358
`MongoPushFootToFar`. The selected first-push frame has a real static
`push_contact` type-3 tree attribute, so the condition cannot use its early
missing-attribute return. Its exact source binding is
`MongoPushFootToFar::ActivationFunc` `0x82caa870`; the generated declaration
retains `push_contact`, `RightToeBase`, and `LeftToeBase` names.

IDA shows the condition resolves `IAnimatable`, `ISkaterAnim`, and
`ISkaterMotionGraph` from the same Actor, reads `GetTreeAttr(push_contact)`,
and returns false if absent. Type3 is required when present. It checks
`GetIsMirrored` against the contact bone side: unmirrored right or mirrored
left is accepted; the native branch can call the mirror getter twice. It then
calls `ISkaterMotionGraph::GetFootPos` with `rightFoot` determined by whether
the contact name equals `RightToeBase`, samples the live
`kMongoPushToAnticThresh` CVar and returns the strict comparison
`threshold > footPos.x` (`0x82caab8c..0x82caabd4`).

The virtual `GetFootPos` target is `SkaterMotionGraph::GetFootPos`
`0x825f5100`. It reads `PhysOut_Skeleton` right/left foot position at
`+0xa0/+0x90`, `PhysOut_SkateboardReckoning` deck position at `+0x90` and
transform basis at `+0x10/+0x20`, and
`PhysOut_SkateboardMotion.mSkateboardFlipped` at `+0x111`. The source
subtracts deck position from foot position, flips the deck Z basis when
the board is flipped, and returns the projections in `Vector2`. The
condition consumes its first, deck-X coordinate. An independent PushOut
review on September 23 caught a transient implementation mismatch that had
selected the second, flipped deck-Z coordinate; the consumer and focused
test now use deck X. A valid Actor without `IPhysical` now reports an
explicit source assertion error instead of returning zero; native only
initializes a zero Vector2 for a null Actor, which the selected context
already rejects. The Rust implementation reads
these **already retained** physical output components and the existing
animation tree attribute owner. It uses the existing Action-side mirror-bit
lookup after Motion context validation, rather than adding a second mirror
state. The finite XYZ projection is currently host arithmetic; exact VMX128
rounding and exceptional float parity have not been established.

IDA initializer `0x82f838a0` and static table entry `0x82fd4d9c` register
`Animation_Skater/kMongoPushToAnticThresh` titled `MongoPush to Antic thresh`,
initial `-0.5`, UI range `[-1,1]`, before `kUseBump` at table
`0x82fd4da0`. The runtime adds this definition once at index345 and samples
the linked live target only after GetFootPos. The source-title-order test
passes. The raw-A replay advanced beyond condition358 and the following
source-backed Pre356/Pre430 branches, then completed 52 visual frames with
Action Pushing48 through held A and release
(`artifacts/public-facade-grounded-push-after-pre430-2026-09-23.log`). A
longer replay is checking speed and position; this is not yet a native
trajectory comparison.
