# Same-player presentation initialization and authored first restore

This packet reuses the existing unregistered InitAnimation, offsets and restore
drafts; it does not implement another pose writer, decompressor, body/hand/face
producer, inverse-bind composition or rendered-pose fallback. Registered path is
`actor_face_playback::presentation`, with `offsets`, `restore` and `startup` children.

## Reuse and ownership

- `skater_presentation_animation.rs`: original retained full/face databases,
  authored bind, eighteen replay SQT allocations, nine interpolated matrix
  allocations, save pose, hips/board and fourteen twist indices.
- `skater_presentation_offsets.rs` and its existing tests: native named part order
  and compressed stream sizes. Compression primitives remain root-owned.
- `skater_presentation_restore.rs`: first selected recipe's authored face/full
  base, actual schema lookup and source fallback, same-allocation extraction.
- Existing `actor_animation_sqt_output_ranges.rs` and eight tests, registered
  using the existing reviewed SQT-range amendment without a competing buffer.
- Existing `animation_job_affinity.rs` and tests, registered using the reviewed
  category2 amendment. The actual SQT dispatcher now passes the retained category
  to entry affinity and caller wait; a separate scheduler is not created.
- Existing buffered RAW constructor, StreamCommands, mapped RAW fetch and job
  execution. Only exact full/face presentation descriptors are additionally
  admitted; existing hand/face category0 admission and behavior are preserved.

`SelectedPlayerStartupOwners` retains the new presentation owner alongside its
actual loader, player settings, simulation startup and faces. Presentation gets
its own native command/playback registries with the SAME global database manager.
The same mounted source supplies bytes and its actual job Arc. Dispatch uses the
already reviewed ActorAnimationPlayerServices job binding, resolved before entering
buffered locks. The first recipe is decoded from the actual selected marquee
definition's binary path in the original-data driver and retained as an Arc.
Loader registration remains pending; authored base restore is not live pose
publication or full UpdatePresEntity recovery.

## Fresh source checks

`SkaterPresentation::InitAnimation` 0x827e1c08 loads full, face, FaceSimPres,
left-hand, right-hand and body databases in the existing draft order. Full bind
GetPose is at 0x827e2070 (arguments prepared at2068), with its nonnull assertion
before BeginCommandSet20a4. The draft was corrected to preserve this failure
prefix. Full bind extraction ends20fc, then fourteen named indices precede
Sync29c4. Temporary databases are removed FaceSim/left/right/body at2d3c/4c/5c/6c,
before initialized2d7c. Existing source/table proof remains in
`skater-presentation-animation-owner.md`.

Fresh InitOffsets827dd548 confirms Torso, Head, Arms, Legs, Skateboard, Trucks
order, assertion count<=120 atd948, then Face0/1/2 and
`8*Q4 + 6*(Q3+T)` atddb10. No body-length-derived or alphabetical remapping is used.

The two RAW descriptors remain exactly the previously recovered full/face shapes:
category2, nine characters, capacities13/6/3/1/2 versus2/1/1/0/0; averages
6/3/1/1/2 versus1/.5/.5/0/0; fetchmode1 versus0, trajectory true versus false.
ACS constructor raw3236c..32384 computes per-character SQT capacity using
single-precision FMA/divide/truncate from `(9*max(avgFetch+avgSet,2)+8)/9`.
The admitted descriptors therefore allocate36 and18 dynamic SQT slots overall.
Raw32648..32658 passes `2*MaxSetReplace*9`, giving set capacities54/18.
Raw32688..32698 passes `(MaxTotalMatrix-MaxSetNonRegisteredMatrix)*9`, giving pose maxima9/0.
PoseEngine82a2a170..1f8 only initializes null lazy pools, dependency count and
category; initialization does not manufacture a matrix job or pose.
FetchSys82d6d9d8 keeps64-byte records, RAW512/job, and one record per skeleton part.

Restore still samples existing cvar115 live at827e6834/6838, resolves the actual
manager playback after that selection, copies authored bind by source hierarchy
parts, looks up actual recipe schema, and falls back only on a missing pose.
The existing registered cvar descriptor/default is reused; no new tuning index.
Face extraction targets a view into the actual odd replay allocation. EndSet and
Sync precede the odd-to-even OverrideWith. UpdateBodies remains outside this packet.

Fresh CharacterEntity::IsMale82cfe138 reads entity+184; null returns1 at14c,
nonnull reads customizations+10 at144. Unpack82d11d90 reads the serialized
customizations presence byte at12774 and constructs/assigns/deserializes only
when nonzero at12780..127cc. The existing recipe parser only admits that byte0.
The adapter therefore uses actual parsed null provenance, never backend gender;
present customizations produce an explicit missing-owner error. IsMale remains
lazy behind the original missing-schema-pose/category3 branch.

## Validation boundary

New focused tests cover separate presentation registries sharing one global DB,
live cvar changes and null/non-null customization behavior, and first-load failure
retaining the allocated database slot without beginning commands or allowing a
whole-constructor retry. Existing offset/range/affinity tests are registered.

The original-data driver now initializes and restores presentation using the
same selected owners before physical construction. Its checks verify actual job
identity, descriptor capacities, allocation counts, offsets, same-player recipe,
synchronized authored bind and bit-identical even/odd restore buffers. These are
corpus assertions, not manufactured production replies. Root owns Cargo and the
original-data run; no successful full-rig runtime result is claimed yet.

Raw ACS register mapping:323ac loads r19 from parameter offset8 (MaxSetReplaceNonRegisteredSQT);3243c loads r26 from offset12 (MaxSetNonRegisteredMatrix). PDB ACSParams confirms these distinct fields. The set pool uses index2, not index3. Twist stores publish individually as signed results. Fresh GetBone82590fc0 returns -1 on a missing name, which is retained without aborting the twist loop. Hips/board retain the same signed GetBone result; future index consumers must explicitly validate it.
