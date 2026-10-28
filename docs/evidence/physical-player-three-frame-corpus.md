# Three consecutive original-owner selected frames

2026-09-20. New test-only draft `physical_player_three_frame_corpus.rs`, intended
child of `physical_player_ground_startup_corpus`. Exact file paths checked absent
before creation. Root retains all original startup modes and adds a separate
fresh mode with completed constructor and contact capacity256. Invoke this child
immediately before the old `check_published` call and return from that new mode.
Do not execute the old two-frame diagnostic prefix first.

Entry is `check(world, board, skeleton, actor, physical_animation, input_session,
cvars, vault)` with mutable actual world/Actor/animation/input session, borrowed
original pair handles/cvars/Vault. Existing mounted owner allocation is retained.

One-time setup reuses the existing original Rob's Park ground mounting and grind
residency fixture; query manager capacities4 batches/5 tests per kind; original
Physics layout gravity; constructor settings; actual physics auxiliary creation;
retained blend token pool and host numeric services. These are controlled corpus
policies, not a recovered complete startup manager. The existing input-session
player/listener/recognizer histories are reused without Reset.

Each of three frames delivers a focused neutral mapped24-channel input through
the existing codec/command/update session (receiver10, mask1, CRC enabled,
controlled offline sync2). It then executes exactly one reviewed Actor prefix,
one reviewed Ground pre-solve, actual authored-ground solver/contact feedback,
and one reviewed post-solve/conditioner continuation. No direct ProcessBoundInput
is added: normal Update1 already performs it. No additional Motion call is
appended after conditioning; next-frame Motion consumes the preceding output.

The selected host quality policy is explicit native High/LOD0 and phase2 wait=true.
This does not reconstruct ActivityManager or quality arbitration. Locality is
read lazily from the actual retained `SkaterManagerLocalIdentity`, sampling its
index before the original Actor+1d8 field. The read-only Actor prefix accessor
performs the same field read as PhysicalStateActorQuery::player_id without needing
a mutable borrow alongside the Actor prefix call. The hands clamp panics if read:
native LOD0 skips that branch for both local and nonlocal Actors.

Grind pre-solve policy is ParkedByUser and post-solve policy explicitly excludes
GrindManager output, preserving the requested first-test scope. Real original
grind residency is still retained for other existing queries. Solver iteration
budget50, ControlledNoFreezing, group0, absent pipeline culling, write flag and
spy flags4 match the existing selected solve fixture; none claims native scheduler
policy parity. Ground reckoning's per-frame token is returned by pre-solve and
passed directly into that same frame's post-solve, rather than fabricated/hoisted.

Every major stage failure reports its zero-based frame number and typed nested
error. Assertions check actual input delivery/flushed pending queue, two solved
assemblies, same physical owner and completed animation/output identity. No
state, contact, transform, output value or query result is patched for success.
Grind residency is released only after all three frames complete.

Standalone rustfmt parsed the draft. No Cargo or registered files changed by
author. Independent review and root original-asset execution are pending; the
draft is intended to expose the first genuine consecutive-frame blocker, not
assert that three frames have already passed. Reused implementation/source
packets are actor-selected-frame-session.md, physical-player-post-solve-conditioning.md,
game-input-frame-session.md and the pre-solve wrapper's existing source evidence.
