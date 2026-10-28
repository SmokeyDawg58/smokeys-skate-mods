# Same-owner selected presentation matrix and palette publication

## Connected path

The original-data repeated-frame driver now supplies the actual mounted scene
decoded from the SAME retained restore recipe. The existing PC host functions
`own_character_scene`, arena ownership helpers and texture ownership helpers
are moved into `mounted_character_scene`; the executable imports these same
functions. No decoder, fixed corpus result, synthetic model membership, or
inverse-bind fallback is introduced. This is explicit selected host asset/LOD
admission, not recovered asynchronous `SkaterPresEntity`/frame-map construction.

`CharacterScene::build_skinning_state` retains its existing native remapping and
inverse-bind behavior. Its produced inverse-bind vector becomes one immutable
Arc slice at construction, cloned only as an ownership handle for SetMatrix.
The selected palette holds that same state and actual scene. The read pass
returns the exact current full ACS owner and actual replay allocation it used;
the second pass therefore does not look up another manager or substitute the
constructor's cached full playback.
The token retains the originating hips-storage Arc and validates the exact
replay allocation/view for the selected row before BeginCommandSet; foreign
owners cannot combine their output or saved hips with this captured ACS.
Existing build_skinning_state identity initialization and named-Hips fallback
are reused host model-admission policy, not newly proven native model construction.

After existing MoveRoot and Expressions, the final selected ACS pass queues
SetSQT, SetMatrix, PoseWithMatMult and ExtractMatrix into the existing
`PresentationAnimation.interpolated[row]` allocation. EndSet and Sync precede
the actual saved hips matrix overwrite at slot zero. The palette then uses
existing `CharacterScene::pack_skin_matrices` remaps, the source four-term
upload multiply, and XYZ row transpose. Rows and generation are exposed only
after complete packing. A late failure retains earlier part results but
invalidates publication and prevents packing a mix of prior/current inputs.

## Native order and arithmetic

- `UpdateBodies827de988`: MoveRoot827deb24, Expressions827deb2c; second set
  begins827deb34; selected model Begin827deb70, SetSQT827deb7c,
  SetMatrix827deb8c, PoseWithMatMult827deb94, ExtractMatrix827deba0,
  End827deba8; EndSet827debc8/Sync827debd0; hips overwrite827dec00..dec24.
- `UpdateSkater827d9e10` SAME source pointer branch827da108..a1cc produces
  the frame root. Raw a138 multiplies quaternion by gSqrt2s; a184 rounded
  paired products; a188 fused 0.5-minus-square; a18c W products; a194/a198
  sums/differences; a19c diagonal sums. Existing MoveRoot `rigid_matrix`
  matches this graph and is reused. Position XYZ is copied; native a168
  overwrites W with1 before a16c store. The new same-frame root getter reads
  actual validated gather bytes; it never promotes opaque header W metadata
  into physical knowledge. Distinct-frame interpolation is not inferred.
- `StartJobs827dbb90` calls `UpdateBoneTransforms827efb78`: actual mesh
  palette index → model bone remap → hierarchy result. Existing remap
  implementation is reused, including actual geometry/palette ordering.
- `EndJobs827dbd50` ultimately calls
  `PackAndMultiplyMatricesForUpload828221e0`. Raw82822218..22308 computes
  localToWorld * each packed bone with rounded X product, Y/Z/W fused adds,
  then transposes and retains the first three rows (48bytes per bone).
  Existing MoveRoot `multiply` is exactly this graph and is reused. This
  upload DOES consume right-hand W; affine inverse-bind compose is not used.

Independent raw review confirmed both reused numeric graphs. Existing host
signed-zero/exceptional fused-negative boundary remains documented; this does
not claim unconditional Xenon floating-point parity.

## Admission and remaining work

The driver explicitly admits one same-frame model and LOD0, using real mounted
assets and already corrected replay. It does not replace native model maps,
previous/next interpolation, damage/dirt effects, pose saving, CPU cloth, or
GPU material admission. Existing renderer `CharacterPoseRows` consumes the
published rows/generation without another pose decoder. The selected runtime
owners are currently crate-private; a real executable skating loop still needs
the production bootstrap/tick facade across that boundary. No corpus bridge,
neutral tuning, forced GPU ropa parameter or fictional completed render loop
is provided.

## Validation

Draft tests cover same-frame opaque-W overwrite with preserved None metadata,
slot-zero overwrite only after actual synchronization, missing saved hips
retaining completed matrix work, fused upload cancellation, upload W liveness,
mesh row slicing/generation and late partial packing failure. Original corpus
asserts finite actual interpolated matrices and every loaded mesh palette,
and checks slot zero equals the real retained hips transform. Root owns Cargo
and original-data execution; no validation run by the implementation agent.

Confidence: high for source graph/order and same-owner connection; mounted
execution and GPU frame-loop completion remain separate validation claims.

Root checkpoint: fmt-palette-3 and strict clippy-palette-3 PASS; workspace-palette-1 4,622 unit plus six doc tests PASS. Original-palette-1 18/20 (5.00s), with actual mounted model palette assertions completing frames0/1. Remaining failures are subsequent Actor state/BlendSpace boundaries, not palette failures. GPU adapter unit checks pass; live window/session rendering remains unconnected.
