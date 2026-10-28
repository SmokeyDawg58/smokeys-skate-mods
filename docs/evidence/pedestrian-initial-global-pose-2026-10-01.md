# Pedestrian first prestate pose

Runtime `pedestrian-handprop-run.log` reached four presentation entities and
eight submitted meshes at frame900, then failed at924 with `prestate simulation
pose absent`. Submission alone does not prove visible moving pedestrians or
working collision. The separate870-frame capture completed, but its screenshot
does not clearly show a pedestrian; end-to-end verification remains open.

IDA checked this continuation:

- PedestrianPerception.Update82e6b3e0 directly copies SimLODBoneGlobal[0]'s
  translation and second basis vector. No renderability guard or fallback.
- tAnimData constructor82e934a0 does not initialize SimLODBoneGlobal.
- Pedestrian constructor82e97748 requests its animation block from memory
  store6 at82e97eb8, then invokes that constructor at82e97f4c.
- LWMemoryMan.RequestBlock82e4fb40 forwards to MemStore.RequestBlock82a6e450.
- MemStore.Init82a6dd98 zeroes the complete backing allocation at82a6de38.
- MemStore.FreeBlock82a6df20 also zeroes the released block at82a6df68, before
  returning its index to the free list. Reused blocks have the same zeros.
- Pedestrian prestate82e9f328 and execution task82e73230 read perception before
  graph execution. GetPreFrameBucketContents82e932e8 does not exclude new entities.
- UpdateTargetTransform82e7f660 publishes evaluated global bones at82e7fc94.

The Rust animation constructor now initializes only this retained global matrix
array to the original pool's all-zero contents. It does not manufacture identity
or bind matrices, alter scheduling, skip perception, or swallow evaluation errors.
The existing animation producer replaces this array after completed extraction.

Release rebuild `artifacts/pedestrian-pose-initialization-build.log` passed.
The2400-tick walk attempt passed the previous pose failure, then stopped at1151
on relocated pointer BIN589080. Runtime log: pedestrian-pose-initialization-run.log.

The original asset maps BIN589080 to livingworld_handprops collection `map`,
layout+56, keyfc1d2c4e5cca6aed, pointing to `CarryPaperRHChannel`. The hand-prop
flag helper incorrectly requested the complete layout prefix through byte60/61,
thus crossing this pointer. IDA IsHandPropDisposable826fd500/CanSitWithHandProp
826fd558 load only byte60/61 (826fd528/826fd580). The helper now uses the existing
one-byte cached-layout field reader. Pointer guards remain intact; no change to
HasHandProp gates or original false/true no-hand-prop results.

Follow-up release rebuild: artifacts/pedestrian-handprop-flags-build.log.
Sustained movement, visible rendering and collision verification remain pending.
