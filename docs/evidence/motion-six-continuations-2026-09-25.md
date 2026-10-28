# Motion continuations from the next six reported errors

Source edits only; no Cargo invocation or
game launch for this batch. Rustfmt parse checks passed. Expression regression
tests were added/extended but have not been run.

- Pre250 at829112e4 loads condition356 (+590 in sConditions), caches the raw
  HasIntent result, then negates it. Sustain250 jump-table entry at8291e254
  points to829221c8: constant true, no condition reads. Added both masks.
- Callback89 and1302: authored ScoringGrabs configurations contain only
  grabName=Coffin / christ_air. Both Begin callbacks825187c0 are native no-ops;
  Update82c8ef88 dispatches Create82ccb910. They now reuse the existing scoring
  handler's non-tweak path: loaded name, zero XY, existing Motion trick bits.
  Directional/polar configurations are not broadened speculatively.
- Update828f0770: StandingJump106 ancestry (18 slots), PlayAnimation18 and
  CreateAttribute19. Exact generated occurrence sequences added for313/316.
- Update828f4e38: Manual1027 ancestry32 slots, score augmentation32,
  AttachIntent33, PlayAnimation34, CreateAttribute35, score augmentation36.
  Same thunk covers1331/1332/1336/1337; all four exact occurrence sets added.
- Callback2015: JumpInto.CreateInstance82cba730 allocates12bytes and sets the
  first-update byte at+8 to1. Existing SetPushCoefs also uses this folded
  allocator, but JumpInto now has its own typed one-byte instance state.
  Begin/End825187c0 are no-ops. Update82cab358 acts once: GetBaseAnim82592b48,
  SetAttributes virtual+76, Animation.GetAttribute(name,31), and if present
  SetTime(attribute.mBeginTime at+24). Clear the first-update byte even if the
  marker is absent. Reuse original attribute lookup, tree clock setters, and
  SetAttributes pipeline; do not use cached active-only frame attributes.
  Transition.GetAttribute82d6db68 and SetTime82d6db38 raw instructions delegate
  to mToAnim at+0x0c. Sequence/TransitionBlend vtable SetTime pointers confirmed
  at8233f028/8233f0d8; retained transitions follow that same child.
- Follow-through Update828f31c0 was also missing. It expands In565 then
  PlayAnimation27, native no-op28, JumpInto29. Added all four folded FromManual
  states1076/1084/1092/1100, using their existing30-occurrence sequences.

Confidence high for source dispatch, marker-query flags, lifecycle and ordering.
Compilation and end-to-end behavior remain unverified until a test.
Existing unsupported animation-node operations still report explicit errors.
