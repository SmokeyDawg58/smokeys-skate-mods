# Skeleton GetLaunchInfo retained-owner bridge

Status: proposed, independently review before root registration. No Cargo run.
Confidence: high for source copy ordering and typed-lane preservation. This does
not establish a completed Air launch, predictor query, or rendered skating frame.

Source `Sk8::Physics::Skeleton::GetLaunchInfo` at `0x82b46098` was freshly read
from the connected debug IDB on September 22. Air LaunchTrajectorySelector at
82d99010 and GetComTrajectoryLaunchInfo82d84050 consume this source function.
There is no numerical calculation here: every vector/matrix operation is a copy.

| Store order | Producer | Launch destination |
| --- | --- | --- |
| 82b460c8 | Processed deck translation+70 | boardPos+c0 |
| 82b460e0 | Processed COM+250 | comPos+d0 |
| 82b460f0 | Skeleton COMLocal+3d40 | animatedCOM+a0 |
| 82b46100 | Skeleton deckLocal+3d60 | animatedDeck+b0 |
| 82b46128..144 | retained Reckoning.transform | orientation+0 |
| 82b4615c..168 | retained Reckoning.inverseTransform | inverse+40 |
| 82b46174 | Processed velocity+190 | startVel+80 |
| 82b46180 | Processed prepared velocity+2c0 | velBeforeJump+90 |
| 82b4618c | Processed timeDelta+93c | timeDelta+108 |
| before82b461b0 | Processed flags+8c4 mask2000 | numTrajectories: clear1/set7 |
| 82b461e0 | global trajectory generated layout+248 | coneAngleX |
| 82b46214 | separately fetched global trajectory layout+240 | coneAngleZ |

The two attribute captures must remain separate. The existing
WallRideTrajectoryAttributes binds GlobalAttributeCollection's actual trajectory
class429a8f4385dd539d/defaultd7edbd362d7d2152, validates layout0x24c, and retains
generated null-layout fallback. This bridge reuses it; no new class or tuning
snapshot is introduced. Real layout errors remain errors.

Independent review checked generated trajectory owner constructor828b1068:
828b10ac..10c0 replaces a null layout pointer with DefaultDataArea(0x24c).
This constructor contract is why an absent generated layout reads zero here,
despite GetLaunchInfo's consumer using direct lfs without its own null branch.
Typed migration and source copy order received independent approval;
registration and focused tests are now pending root validation.

`physical_player_skeleton_launch_info.rs` is the preexisting unregistered draft,
now completed with a private read boundary for failure-prefix tests. Its live
implementation borrows actual constructor bindings, processed storage, outer
fields, same-owner Reckoning, and supplied retained global trajectory attributes.
Owner mismatches and unknown producer data remain explicit errors. Unknown W is
copied as unknown; this source has no operation that can establish it.

The caller must not hold the same processed or Reckoning mutex while invoking
this bridge. Root must schedule it with the actual normal Skeleton constructor
borrow and the same retained physical owner, as the existing draft requires.

The existing trajectory launch record needs the following minimal root amendment.
Only the fields written from typed producers change; both override vectors remain
raw constructor-known arrays because this function leaves them untouched.
`physical_player_onboard_launch_prefix.rs` is another existing unregistered draft:
its full-record copy continues to work without changes. No registered runtime
caller was found to index these launch fields.

```diff
--- a/crates/skate2-core/src/physics/player/prediction/physical_player_trajectory_owners.rs
+++ b/crates/skate2-core/src/physics/player/prediction/physical_player_trajectory_owners.rs
@@
     skeleton_processed_contact::SkeletonProcessedContact,
+    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
@@
+#[path = "physical_player_skeleton_launch_info.rs"]
+pub(crate) mod skeleton_launch_info;
@@
-    pub(crate) reckoning_orientation: [[f32; 4]; 4],
-    pub(crate) reckoning_orientation_inverse: [[f32; 4]; 4],
-    pub(crate) start_velocity: [f32; 4],
-    pub(crate) velocity_before_jump: [f32; 4],
+    pub(crate) reckoning_orientation: SkeletonRootMatrix,
+    pub(crate) reckoning_orientation_inverse: SkeletonRootMatrix,
+    pub(crate) start_velocity: SkeletonRootVector,
+    pub(crate) velocity_before_jump: SkeletonRootVector,
@@
-    pub(crate) board_position: [f32; 4],
-    pub(crate) com_position: [f32; 4],
+    pub(crate) board_position: SkeletonRootVector,
+    pub(crate) com_position: SkeletonRootVector,
@@
-    const fn constructed() -> Self {
+    fn constructed() -> Self {
         Self {
-            reckoning_orientation: AFFINE_DEFAULT,
-            reckoning_orientation_inverse: AFFINE_DEFAULT,
-            start_velocity: [0.0; 4],
-            velocity_before_jump: [0.0; 4],
+            reckoning_orientation: SkeletonRootMatrix::from_columns(
+                AFFINE_DEFAULT.map(SkeletonRootVector::known)),
+            reckoning_orientation_inverse: SkeletonRootMatrix::from_columns(
+                AFFINE_DEFAULT.map(SkeletonRootVector::known)),
+            start_velocity: SkeletonRootVector::known([0.0; 4]),
+            velocity_before_jump: SkeletonRootVector::known([0.0; 4]),
@@
-            board_position: [0.0; 4],
-            com_position: [0.0; 4],
+            board_position: SkeletonRootVector::known([0.0; 4]),
+            com_position: SkeletonRootVector::known([0.0; 4]),
--- a/crates/skate2-core/src/physics/reckoning/reckoning.rs
+++ b/crates/skate2-core/src/physics/reckoning/reckoning.rs
@@
     pub(crate) fn transform(&self) -> &Matrix {
         &self.transform
     }
+    /// Skeleton.GetLaunchInfo82b46148 copies the retained inverse directly.
+    pub(crate) fn launch_inverse_transform(&self) -> &Matrix {
+        &self.inverse_transform
+    }
```

Apply the `constructed` change specifically to `impl RetainedTrajectoryLaunchInfo`,
not other similarly named constructors in that file. The original default axes
and fourth lanes are preserved exactly by the typed wrappers.

Five focused tests in `physical_player_skeleton_launch_info_tests.rs` exercise:
ordered copies; unknown W and signed zero; untouched caller-owned control/override
fields; actual flag mask/count; separate late layout failure; early COM failure;
failure between orientation and inverse reads; generated null layout policy.
These are controlled read-boundary tests, not an original-assets live caller test.
Root must register, independently review, compile, run focused tests and include
the bridge in the eventual real Air launch corpus before claiming integration.
