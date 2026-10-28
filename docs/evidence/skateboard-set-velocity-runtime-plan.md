# Skateboard SetVelocity: native stores and actual owner seam

Independent read-only production audit, 2026-09-08. Fresh IDA raw instructions and current Rust owners inspected; no Cargo or implementation changes. Exact confidence for the native loop. Host freshness policy below is a proposal, not a recovered native field or side effect.

## Native graph

`Skateboard::SetVelocity` is only **82b29ee0..82b29ee8**: load the existing SkateboardBody pointer from this+0c, then tail-branch to **SkateboardBody::SetVelocity82b244d8**. Do not include the following material-pointer function beginning29ee8 in this wrapper.

Body function **82b244d8..82b2452c**:

1. Read this+18 Assembly244d8, initialize unsigned index0 at244dc, read Assembly+1c numParts244e0. Zero count returns244e8 without reading input velocity or any body.
2. Initialize byte offset0 at244ec. Each iteration reads the current Assembly Part-list pointer+8 at244f0.
3. Load full supplied velocity at244f4, BEFORE resolving this Part's body pointer. Increment unsigned index244f8, compute Part address244fc, increment byte offset by60 at24500.
4. Read Part+4c actual body pointer24504. Address its velocity at body+20 at24508; load the existing full packed vector2450c.
5. **24510 `vrlimi128 v63,v62,1,0`, raw word1be1f715**, inserts the old body's W into the supplied vector. Store full resulting vector at24514. Thus source XYZ replaces body XYZ, while the old body W survives exactly. Input W never survives this operation.
6. Reload this+18 Assembly24518, then its current numParts+1c2451c. Unsigned comparison24520; if index<count branch24524 to the next Part-list read244f0. Otherwise return24528.

There is no mass multiply, force addition, angular velocity write, cooldown store, body activation, transform update, queue clear, contact traversal, cvar read, normalization, numerical estimate, timestep update, or explicit native frame invalidation. All Parts participate according to the current count, not a hardcoded deck or four-wheel subset. The present original board has seven Parts, but that is data rather than the loop bound.

Input is reloaded each iteration. A borrowed immutable XYZ value is a valid scoped host projection when mutation/aliasing of the source vector during this loop is excluded. A staged generic source should preserve the load before body resolution if it permits fallible input acquisition. Do not preflight all body observations: failure on Part k leaves already completed velocity stores for Parts0..k-1 in place. No rollback is native.

SHA-256 for Body bytes244d8..2452c, end exclusive: `16792172af05c38485486f52735ee1f3dbfc367197496ebabf0b8547e7569ee7`.

## Existing actual body representation

`simulation_rigid_body_pool.rs::RigidBodySlot` already separates `velocity: [f32;3]` from intrusive `links: Links`. The packed native velocity W is a link word, not a fourth semantic velocity. A narrow assignment to **only slot.velocity** faithfully preserves that separate ownership; do not supply a guessed float W, reset links, or rewrite a whole body snapshot.

Existing `try_contact_spy_linear_velocity(observation)` validates the occurrence and ring then reads exactly these XYZ. Existing `try_reset_part_body_motion_host` also writes velocity but additionally clears omega and rewrites force/torque: it is NOT a reusable SetVelocity implementation. `try_apply_skateboard_queued_force` and Skeleton displacement are also the wrong operations.

No standalone current production velocity setter was found. Minimal pool addition: occurrence-qualified actual body setter, `validate_observation` then `validate_integrity` before mutation, followed by the one XYZ assignment. Keep arbitrary f32 bit patterns, including signed zero, infinities and NaNs. No finite gate is present natively. There are no recoverable native operations inside that individual commit after host authority validation.

Use the existing actual backend forwarding pattern. Above it, use the attached board session's real Part-body authority (`try_part_contact_spy_observation`), or a small generic Part loop patterned on `dynamic_assembly_part_activation.rs`. Validate attached backend/board kind before entering the native loop; resolve each Part independently at its source site. The owned session's Part vector cannot mutate through the same exclusive backend call, so repeated `.len()` checks preserve the count-read site without inventing mutable topology callbacks. Prefer the actual retained attached Part count over a new fixed7 value. Do not build a copied array of seven observations before writing.

The existing `dynamic_assembly_skater_forces.rs::try_set_board_linear_drag` and world bridge demonstrate backend/member/kind authorization and scoped session access. They are patterns for ownership only; grouped-four inertia writes and drag scaling are unrelated to SetVelocity.

## Freshness is a separate host concern

Current pool `frame_identity` advances only in `begin_dynamic_update_frame`, which also writes timestep and frequency. Existing Part transform and motion-reset setters do not advance it. There is no separate body velocity revision. `SimulationContactSpyFrame::try_validate_current` checks manager, pool, pool frame identity and retained body occurrences; it does NOT compare captured versus current velocity. A new velocity write alone therefore leaves the preceding solved frame accepted by that check.

The world owns `contact_spies`, contact-list progress, and various retained samples. Board motion publication explicitly checks the same current frame Arc and calls `try_validate_current`; COM publication also validates this frame and completed Skeleton history. Consequently clearing only a local sample, or assuming observation identity changes on a velocity write, is insufficient to reject all stale solved-state certificates.

Do not call `begin_dynamic_update_frame` from native SetVelocity merely to invalidate tokens: that would silently advance a simulation frame and rewrite timestep/frequency. Nor should the native body setter clear historical PhysOut or Skeleton history numerics; those are legitimate retained previous-output inputs and source SetVelocity does not reset them.

Two honest integration boundaries are available:

- Invoke the selected setter only in the explicit pre-solve schedule, and require the already existing next real ground-frame start before consumers needing a newly solved frame. This is narrow scheduling, not a generally fresh post-write world API.
- For a reusable world mutation API, add a documented host-only mutation invalidation mechanism. It must invalidate prior pool-backed frame certificates without changing timestep/frequency or body occurrences, and ensure world wrappers cannot relabel old output as current. A private epoch invalidation (e.g. unavailable frame identity until the next actual frame start) can reject existing `try_validate_current` clients; audit all other frame-token consumers before choosing this design. Alternatively add a separate mutation revision checked by those certificates. Neither mechanism currently exists as a ready reusable setter facility.

Apply host invalidation at the first successfully committed write and retain it after later Part failure. A bad board/foreign backend/first invalid body or zero-Part no-op should not claim a native mutation occurred. If choosing earlier conservative invalidation for a simpler API, document and test that additional host refusal behavior rather than describing it as exact source prefix.

## Focused validation for implementation

Prove all-Part ascending writes and repeated count/body stages; late Part refusal retains earlier XYZ changes; zero count never reads input; existing list topology/state/cooldown/omega/force/torque/inertia/pose remain unchanged. Include signed-zero/NaN bit preservation and a source input W unrelated to the body's existing packed link. Use actual slot snapshots for the link invariant, not a synthetic affine W assertion.

An original-data fixture may call this source setter with explicitly controlled velocity and then execute the real solver. That is a diagnostic native setter input, not a live Actor jump producer or applied force. If a general world setter exposes it, add a stale pre-write motion/COM-frame refusal test plus successful new-solve recovery under the chosen explicit host invalidation policy.
