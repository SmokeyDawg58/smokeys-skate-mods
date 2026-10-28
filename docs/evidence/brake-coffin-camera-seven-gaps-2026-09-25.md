# Seven live dispatch gaps, September 25

Implemented against IDA; compilation and runtime testing remain pending.

## Source and implementation

- Action update `0x828ed0b8`, StartBrake14: parent Action0 plus instance slot11 CreateConstMGIntent.Update. Wired all authored aliases 5/10/11/14/15 using their exact inherited behavior arrays. States14/15 inherit the constant behavior through state5; it is not called twice.
- Motion update `0x828eef80`, CoffinCyc85: Coffin23 parent (`0x828ee6d0`), then slots25/26/27 native empty,28 CreateAttribute,29 PlayAnimation. Parent retains OnBoard, AttachIntent, SetTurning, CreateAttribute and ScoringGrabs. Existing callbacks reused.
- Motion update `0x828f51c0`, Stationary1353: Holding1273 parent (`0x828f4490`), with Manual1027 and SetScoreAugmentation, then35 CreateAttribute,36 AttachIntent,37 PlayAnimation. Flattened authored ancestry includes Brake1335 without duplicating inherited work.
- Motion Pre115 at `0x8290dcb8`: condition195. Pre406 at `0x82919db0`: `c226 || (c238 && c239)`, preserving lazy evaluation. Independent Sustain switch sends both to native true default `0x829221c8`.
- Action expression104, Pre `0x829057ec` and Sustain `0x8290a0bc`: ordered lazy OR of conditions59,53,54,60,52,55 (HasAGIntent).
- Camera blend16: GetBlendRawValue `0x82e44898`, branch `0x82e44bf0`, calls GetTrajectoryCollisionTime `0x82e3bfb0`. Returns subject trajectory total end time if valid, otherwise zero. Does not subtract elapsed time or gate on dt. Added shared CameraSubject getter, reused by existing PredictedAirTime graph condition and blend source. Existing shot setup and live updates both use this source. Existing trajectory producer retained; missing subject still errors.

## Validation and limits

- Static descriptor audit PASS: Action5/10/11/14/15 and Motion85/1353 arrays exactly match authored ancestry, each behavior once, each dispatch arm unique.
- Rustfmt parse-only PASS across ten touched Rust implementation/test files.
- Added focused regression coverage for expression short-circuit order, Sustain defaults, trajectory validity/total-time semantics; expanded existing Action ancestry regression. Tests have NOT been executed.
- No Cargo compilation or game launch. Check with build-release.bat, then play-quiet.bat. Quiet logging behavior unchanged.
- Remaining streaming hitches are not claimed fixed by this dispatch batch.
