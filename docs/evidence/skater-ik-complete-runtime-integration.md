# Ordinary retained skater IK integration

2026-09-08, implementation and separate independent source/
code review. Broader DMO admission/publication remains parked. This packet
describes the bounded actual-owner path and its original-data validation,
not a complete Actor-driven skating tick or deterministic reference parity.

## Source order and actual owners

`SkeletonIK::Update` at `82b59c60..82b59cc8` calls Status at `59c74`,
BlendValues at `59c78`, InitialTransforms at `59c80`, and BlendTransforms at
`59c88`. Only then does it load `kEnableIKContactOffset` at `59c94/98`.
Contact (`82b55560`, call `59ca8`) is optional; DriveFrames (`82b52e90`, call
`59cb0`) follows unconditionally. Both contact and prephysics gates default
true; diagnostic reads use independently confirmed original defaults.

The same `SkeletonCurrentAnimation` retains the `SkeletonIk`, its target
mapping and animation record. `skeleton_ik.rs::try_update_suffix` and
`skeleton_current_animation.rs::try_update_ik_suffix` borrow these owners and
the actual `SkeletonAnimationTargets` rows. The existing global animation
attribute provider supplies prephysics limits; no replacement IK instance,
pose mapping or copied mutable row authority is created. The row adapter
checks each source access individually, preserving earlier stores on error.

`dynamic_assembly_skeleton_general_update.rs::try_update_grounded_complete`
continues the already executed roots/hooks/reset/DataIn/initial/blend prefix
through Contact and DriveFrames, then the same retained Drives, all 24 Part
force additions and grounded trajectory reset. The earlier bounded entry
point still reports `UpdateIkContact`; it remains useful for prefix tests.
State `0x2be` retains its native whole-IK skip route.

`physical_player_grounded.rs` binds the selected actual paired bodies and
`physical_player_grounded_services.rs` routes Contact queries into that
session's retained World query owner and the actual borrowed static terrain.
`simulation_world_line_query.rs` owns the real batches and actual constructor-
empty dynamic collections used in this scope. Missing owners and unsupported
unpublished membership fail; they never become geometric misses. Broader DMO
spawning, dynamic mesh publication and native World scheduling are not added.

Gate failure leaves the preceding IK prefix intact and does not enter Contact
or DriveFrames. Disabled Contact preserves nonzero contact histories and both
cached feedback flags. Contact failure prevents DriveFrames. DriveFrames and
later drive/force errors retain all completed source mutations. There is no
early feedback handoff into General: later PhysOut/next-frame consumers remain
the separate source scheduling boundary recorded in the suffix source packet.

## Processed inputs and physical mass

`skeleton_processed_contact.rs` produces a bounded ready projection rather
than accepting a caller-selected contact bone index. Reset's ID is `Some(0)`
(`ProcessedPhysIn::Reset`, `82b39500`); failed hierarchy lookup is `None`.
The raw `0x40`-byte attribute prefix checks type byte `+35 == 3` and active
flags `+34 & 0x0c != 0` before reading five big-endian payload name words.
It searches the actual `SkeletonData` hierarchy in ascending order and stores
the first match or absence. Every accepted record overwrites the ID before
attribute-name dispatch, including an unknown attribute name; the last accepted
record wins. Push/brake dispatch is not inferred from this prefix.

Toolkit sampling follows `PrepareToolkitInput` `82b26920..82b269e0`: actual
board mass first, then a Part 6 transform copied to DeckTransform, then a
second independent Part 6 transform read. `GetEffectiveTransform`
`82b23778..82b237f8` reads the current board-flipped flag `+8c4 & 0x00100000`
after that second read and XORs the sign bit into all four X/Z lanes. Y/P and
unknown W knowledge are retained. A second-read failure preserves mass/deck
and any old effective observation but prevents ready publication. The physical
effective snapshot (`processed+80`) is distinct from DeckTransform (`+40`)
and effective animation transform (`+c0`).

`skateboard_body_mass.rs` recovers `SkateboardBody::GetMass`
`82b243e8..82b244b4` from the actual attached Part body cached inverse masses.
The source groups four Parts then handles the tail, using single-precision
division `1/inverse_mass` and ordered additions. The first group term and each
tail term add mass before the accumulator; remaining group additions add to
the accumulator. No zero/nonfinite guard or deck-only replacement is added.
Actual Part observation checks are the existing safe-host ownership boundary.

The ready Contact projection contains explicit current retained flags supplied
after the caller's attribute work, the produced hierarchy ID and the sampled
physical effective transform. Its immutable snapshot cannot model native
callback alias mutation. The corpus supplies controlled raw attributes and
flags; full native input history, raw attribute dispatch and Actor timing are
not claimed. Existing actual-body transform getters retain their documented
canonical-W host boundary; no unknown velocity/IK W is filled to pass a store.

## Validation and corrected geometric expectation

Focused checks passed: two suffix tests, three processed-contact tests and
seven World route tests. Separate review passed the source,
owner interfaces, retained suffix, processed producer, mass reader and corpus.
Suffix coverage includes nonzero contact histories and both true feedback
flags produced by Contact itself, followed by disabled Contact preservation
and successful disabled-prephysics return. Failure tests discriminate late
gate/contact/DriveFrames ordering without eager row or attribute reads.

Original-data run **38857/ae24ce** passed all **nine** original checks. The new
`physical_player_ik_complete_corpus.rs` continues the same actual original
board/skater pair and Rob's Park terrain after the existing grounded prefix
corpus. Both IK gates remain enabled. It proves ordinary completion, original
Vault attribute reads, actual World query traversal, prephysics execution,
unchanged raw/record poses, changed mapped rows, the same 44 retained bone
drive payloads, all 24 force additions and the final trajectory reset. The
ordered diagnostic reads also verify initial < contact-enable < line <
prephysics-enable < prephysics < retained drives.

The first run **56533/cce4a4** already completed the ordinary enabled suffix
but failed a corpus assumption that the selected foot segment must hit ground.
The actual segment starts at Y **-54.659584** and ends at **-55.659584**; the
original terrain directly above it is at Y **-54.431007**. The entire finite
segment is below that surface. A test-only retained World input observation
captures the exact query before DriveFrames can alter its target position.
Independent `thin_segment_intersections` over every published original mesh
finds zero intersections; a separate extended vertical diagnostic identifies
the surface above. The corrected oracle requires this explicit zero-hit result
and source Contact state **2**, rather than accepting arbitrary hit/miss output.
No production math, query routing, gates, pose or placement changed to satisfy
the test. A separate original World corpus proves positive terrain queries;
this ordinary IK case proves a genuine finite-segment miss followed by full
DriveFrames/drives/force completion, not positive foot contact feedback.

Accepted checkpoint **96455/672297**: workspace **13488/060f9c** passed
**3,158 units** (2,915core+202formats+38app+3symbols) and six docs;
original **38857/ae24ce** passed all nine, and all-target Clippy with warnings
denied passed **96455/672297** after documentation/import/lint-only corrections.
The original
run does not establish live controller-driven skating, state transitions,
positive-contact multiframe behavior or deterministic native numeric parity.

Confidence: **exact** for rechecked native call/load/store sites, field/index
distinctions and source constants; **high** for the actual Rust owner binding
and original-data producer-to-consumer validation. Native full-phase timing,
concurrent alias behavior and full playable-session parity remain unproven.

Related source packets: [Update suffix](skeleton-ik-update-suffix-source.md),
[Contact](skeleton-ik-contact-source.md),
[prephysics](skeleton-ik-prephysics-source.md),
[World owners](skeleton-ik-world-line-owners-source.md), and
the parked DMO boundary.
