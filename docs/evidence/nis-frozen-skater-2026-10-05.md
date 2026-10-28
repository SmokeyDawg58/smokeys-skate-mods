# NIS frozen skater: ICE freeze command 5 (PhotoShot_FixedFrame)

> Agent research (2026-10-05), read-only IDA on `sk82_na_m.xex` (IDB session 07f41359). Nothing in the
> IDB was renamed, retyped, commented, patched or saved. Every fact cites the address it was read at.
> VMX128 operands were decoded from the raw instruction words with a scratch decoder, because IDA prints
> some VA operands 32 registers too high (e.g. `vmsum3fp128 v61, v95, v62` is `v61, v63, v62`).
> Anything not traced is marked UNVERIFIED. Companion to `nis-port-notes-2026-10-04.md` (section
> "ICE freeze channel") and `nis-scene-characters-2026-10-04.md`.

## 0. Summary

1. `ICEUserChannelFreeze::FireEvent` (0x826d1b50) case 5 runs only while `GetDeltaTimeMult() > 0`. It
   tests the gameplay local skater's `SkaterPresEntity` sphere (centre = its hips position with w = 1,
   radius `m_BR`, minimum projected size 19 px) against `GfxMan->mCameras[5].mFrustum`, the main
   viewport camera (`CAMERAIDX_GAMEVIEWPORT1`). That frustum is the one `RenderMan::UpdateCamera`
   built at the end of the PREVIOUS frame from `CameraPresentation`, i.e. from the ICE camera.
2. `TestSphere_VMX` (0x8282c208) returns 0x40 (`eFrustum_Outside`) when any of the six normalized
   planes has `dot3(c, n) + d < -r`, or when the projected size `trunc(m_H * r / (dist - r))` is below
   19; otherwise 0 (`eFrustum_Inside`) or an OR of per-plane straddle flags (unused by command 5).
3. The pose comes from `Sk8::Scene::CharacterPoseSaver` (SceneManagerImpl+0x38): one buffer of
   `SkaterPresentation::mNumBones` matrices plus one local-to-world matrix, overwritten every time
   `SkaterPresentation::UpdateBodies` (0x827de988, i.e. every replay restore) runs while a
   PHOTO_OUTRO scene plays. It holds the local skater's skinning palette (the `ExtractMatrix` output,
   slot 0 replaced by `mHipsTransform`) and `mHipsTransform x root`.
4. `UpdateFrameEnd` (0x826cf1a8) of the same frame copies that pose into a new `SceneCharacter`
   whose animation object is a `SceneCharacterPose` (every time/eval method is a no-op), gives it an
   owned `SkaterPresEntity` (type 0x822) of the player's recipe, sets its object matrix once and adds
   it to the playing scene. It renders the copied palette, static, until `Scene::Purge`.
5. `mInstanceFrozenSkater` (+0xF) is written only by the constructor (0), `FireEvent` case 5 (1) and
   `UpdateFrameEnd` (0). `OnTakeStart`/`OnTakeChange` do not touch it.
6. Data: all 64 PHOTO_OUTRO scenes are replay-controlled (`mIsReplayOn` 1) with no character records;
   11 of them use command 5, each as eight intervals (1..8, 10 frames each): a sequence shot that can
   leave up to eight frozen skaters.

## 1. Frame order (`GameModule::Render` 0x827320f0)

1. `SceneManager::UpdateFrameBegin` (0x827322e8) -> `SceneManagerImpl::UpdateDeltaFrameBegin`
   (0x828770f0): `UpdateNIS`, `UpdateSceneCharacterViewMode`, `ScenePlayer::Update` (0x82877120: freeze
   countdown, each scene's `Update`/`AddTime`), then `SceneCamera::UpdateFrameBegin` (0x8287712c):
   `ICEManager::Update` (interval crossing) and `ICECameraMover::UpdateFrameBegin` -> user channels'
   `UpdateFrameBegin` (overlays, then freeze) -> `FireEvent(5)`.
2. If in game, restore allowed and `kPAUSE_FLAG_DATASYSRESTORE` not paused (0x8273233c..0x827323ac):
   `DataSysMan::StepTime` (0x8277c1d0: time controller `Update`) and `RestoreFrame` (0x8277f040:
   `DataMan::RestoreData`) -> `SkaterPresentation::DoSimRestore` (0x827e6438) -> `UpdateBodies`
   (0x827e7014) -> `CharacterPoseSaver::SavePoseBuffer` (0x827dedac). Dispatch from `RestoreData` to
   `DoSimRestore` is from earlier port evidence (`mounted-camera-render-presentation-schedule-2026-09-23.md`),
   not re-traced here.
3. `Presentation::Update` (0x82732434) -> `PostfxPresentation::Update` (0x827f7608, bloom pulse).
4. `SceneManager::UpdateFrameEnd` (0x82732490) -> `SceneManagerImpl::UpdateFrameEnd` 0x82875a90 ->
   `SceneCamera::UpdateFrameEnd` (call 0x82875abc, function 0x828646f8; needs
   `mSceneCameraInitialized`, `mICECameraActive`, an
   active scene handle) -> `ICECameraMover::UpdateFrameEnd` (0x82a3b2e0; needs a take and a scene that
   is not queueing): transformation, cuts, lens, shake, constraints, then
   `ICEChannelManager::UpdateFrameEnd` (0x82a3b40c; every user channel, registration order, freeze
   last) -> freeze `UpdateFrameEnd` -> `InstanceFrozenSkater`; only after that
   `mCamera->SetCameraMatrix` (vtable +0x14, 0x82a3b43c).
5. `RenderMan::Render` (0x827324ec) -> `UpdateCamera(0, 0)` (0x828585b4): camera 5 matrices and
   frustum for the next frame's test.

Consequences: the frustum and the sphere centre that `FireEvent` reads in frame N are frame N-1's
(the last `UpdateCamera`, the last `DoSimRestore`); the pose that `UpdateFrameEnd` copies is frame N's
restore, which `SceneReplayTimeController::Update` (0x828727e8) scrubs from `mScene->mTimeElapsed`,
already advanced by step 1 before the event fired.

## 2. The frustum test (question 1)

### 2.1 FireEvent case 5 (0x826d1e34..0x826d1fb0)

- `Player()->GetDeltaTimeMult()` (IScenePlayer vtable +0xD0, 0x826d1e58); `fcmpu`/`ble` (0x826d1e64):
  nothing unless `> 0` (NaN fails).
- `p = Presentation+0xFCD0` (`mSkaterPresentation.mObject`), `GetLocalIndex` 0x827d0090,
  `GetSkaterPresEntityFromIndex` 0x827d3410 (0x826d1e80..0x826d1e8c). The result is not null-checked:
  with no local row (index -1) the native dereferences null.
- camera = `GfxMan + 0x7840` (0x826d1ea0) = `&mCameras[5]` (cameras at GfxMan+0x90, stride 0x17F0;
  `mCurrentCam` at +0x80, 0x82858688..0x8285869c).
- `minPixel = entity->GetMinPixelSize()` (vtable +0x3C, 0x826d1eac), `r = entity->m_BR` (`lfs 0x144`,
  0x826d1eb4), `c = entity->GetBoundsCenter()` (vtable +0x38, 0x826d1ec8).
- `TestSphere_VMX(&camera->mFrustum (camera+0x1A0), c, r, minPixel)` (0x826d1edc); result 0x40 ends
  the case (0x826d1ee0).
- Otherwise: `mInstanceFrozenSkater = 1` (`stb 0xF`, 0x826d1ef0), BEFORE the frames test; if `frames`:
  `SetFreezeTimeMult(0)` (vtable +0xC4) and `SetFreezeResetCounter(frames * 0.033333335)` (vtable
  +0xC8, 0x826d1f40); bloom pulse (2.10); `cMsgNISFEEvent{kind 0x4EB59F71, size 20, fID 85}` to
  `FrontEndManager::EventBroadcaster` (0x826d1f90..0x826d1fb0).

### 2.2 `Frustum::TestSphere_VMX(pos, fRadius, fMinPixelSize)` 0x8282c208

- `Pos` is one aligned 16-byte load (`lvx128 v63`, 0x8282c220). Planes `m_aFrustumPlane[i]` at
  this+0x80+16i.
- `d_i = vmsum3fp128(Pos, P_i) + splat(P_i.w)` (`vmsum3fp128` 0x8282c248..0x8282c26c, `vor128` copies,
  `vspltw128 .., 3`, `vaddfp128` 0x8282c2bc..0x8282c2d0); lane x is read back with `lfs`.
  (Planes 0 and 1 share stack slot sp+0x50; plane 0 is loaded before plane 1 overwrites it, so the
  values are correct.)
- `-r` is `fneg` (0x8282c228). All comparisons are `fcmpu` with `blt`/`bge`: a NaN distance never counts
  as below a bound.
- Order (0x8282c2dc..0x8282c394), F = `aIntersectFlag` (this+0x1630):

  | plane | outside if | flag if `d < r` |
  |---|---|---|
  | 0 | `d0 < -r` | `flags = F[0]` |
  | 1 | `d1 < -r` | `flags \|= F[1]` |
  | 2 | `d2 < -r` | `flags \|= F[2]` |
  | 3 | `d3 < -r` | none |
  | 4 | `d4 < -r` | `flags \|= F[3]` and `flags \|= F[4]` (both from d4) |
  | 5 | `d5 < -r` | `flags \|= F[5]` |

  Quirk: plane 3 never sets a flag; its flag F[3] is set from plane 4's distance (0x8282c350..0x8282c360).
- Then `px = GetPixelSize(Pos, fRadius)` (0x8282c398); `(float)px` (`extsw`, `fcfid`, `frsp`) below
  `fMinPixelSize` -> 0x40, else `flags` (0x8282c39c..0x8282c3bc).
- Returns: 0 (`eFrustum_Inside` in the decompiler), 0x40 (`eFrustum_Outside`), or the OR of flags.
  `aIntersectFlag = {4, 8, 1, 2, 16, 32}` for every camera, set by `ViewMan::ViewMan` 0x8282aed8
  (0x8282af7c..0x8282af94); the copy constructor 0x82800b18 / `operator=` 0x8282afd8 copy them. The
  enum member names of 1/2/4/8/16/32 are UNVERIFIED. Command 5 only compares against 0x40.

### 2.3 `Frustum::GetPixelSize(pos, r)` 0x828268a8

- `diff = Pos - m_CamPosition` (this+0xF0, all four lanes, `vsubfp128`); `sq = vmsum4fp128(diff, diff)`;
  `y = vrsqrtefp128(sq)` refined twice as `y += 0.5*y*(1 - sq*y*y)` (`vnmsubfp`/`vmaddfp`);
  `dist = sq*y`, 0 when `sq == 0` (`vcmpeqfp128`/`vsel`) (0x828268ac..0x82826910).
- `g = dist - r` (`fsubs`); if `g > r` (`fcmpu`/`ble`, unordered takes the else): `f = (m_H*r)/g`
  (`fmuls`, `fdivs`), else `f = m_H` (this+0xE0); return `fctiwz(f)` (truncate to int32)
  (0x82826914..0x82826938).
- `Pos.w` is 1 (2.11 / 3.2) and `m_CamPosition.w` is 1 (2.6), so the four-lane length is the XYZ
  distance.

### 2.4 Which camera is index 5

`RenderMan::Render` 0x82858360 calls `UpdateCamera(this, 0.0, 0.0)` (0x828585b4), then
`SetCurrentCameraIndex(5)` (`li r4, 5`, 0x8285867c; `CAMERAIDX_GAMEVIEWPORT1`) and `UpdateCullParams`
on it (0x828586a0). `Frustum::ComputeFrustum` has three callers: `RenderMan::UpdateCamera` (camera 5,
0x82848b94), `PostFX::RenderShadows_CSM` (camera 10: `li r4, 0xA` 0x82855a28 ->
`SetCurrentCameraIndex` 0x82855b10, frustum 0x8285726c) and `CACPresEntity::SetupSnapshotCamera`
0x827daaf0 (CAS). So only `UpdateCamera` writes camera 5's frustum: during a NIS it is the main game
viewport fed by `CameraPresentation`, which then returns the ICE camera (2.9).

### 2.5 `RenderMan::UpdateCamera(xJitter, yJitter)` 0x82848978

- `M = CameraPresentation::GetMatrix(allowManualCam = 1)` (0x827f6308, call 0x828489c0).
- View = affine inverse of M (0x828489c4..0x82848b24): cofactor rows `r1 x r2`, `r2 x r0`, `r0 x r1`
  (`vpermwi128 0x63` rotations, `vmulfp128`, `vnmsubfp`), `det = vmsum3fp128(r0, r1 x r2)`, `1/det` =
  `vrefp128` + two Newton steps, inverse 3x3 = transposed cofactors * `1/det` with w lanes stored 0;
  translation `= (-t.x)*I0 + (-t.y)*I1 + (-t.z)*I2` (`vmulfp128`, two `vmaddfp`), w 1.
- `ViewCamera::SetViewMatrix(V)` 0x82825098 (`mV`, `mTV`, `mVP` with the previous `mP`, `mTVP`).
- `far = GetFarPlane()` 0x827f2630 (rendering attribute layout +0x64), `near = GetNearPlane()`
  0x827ef8d8, `fovy = GetFOVY()` 0x827f64b0 (degrees).
- `SetPerspectiveMatrixRadians(fovy * 0.017453292, near, far, xJitter, yJitter)` (0x82848b6c).
- `ComputeFrustum(&cam->mFrustum, &GetMatrix(true).wAxis, &cam->mVP, bNormalize = 1)`
  (0x82848b74..0x82848b94; `GetMatrix` is called again for the position).
- The port already builds this projection: `camera/frame/camera_producer_projection.rs`
  (`try_ice_camera_projection`, `project_producer`; it notes that Xenon `vrefp` bit parity of the inverse
  is unresolved).

### 2.6 `ComputeFrustum(camPosition, comboMatrix, bNormalize)` 0x8282bd28 and `NormalizePlane` 0x8282bc60

- Whole body skipped when cvar `kFreezeCullCamera` (0x8305684c) is set.
- `m_CamPosition` (+0xF0) = camPosition with w replaced by 1.0 (`vrlimi128 .., 1, 1` from a splat of
  1.0, 0x8282bd50..0x8282bdc0); `m_matrix44` (+0x40) = comboMatrix.
- Planes, scalar `fadds`/`fsubs` per row r of `mVP` (0x8282bde8..0x8282bfa8):
  P0 = r.w + r.x, P1 = r.w - r.x, P2 = r.w - r.y, P3 = r.w + r.y, P4 = r.w + r.z, P5 = r.w - r.z
  (column w +/- column k; P4 is the symmetric `z >= -w` plane, P5 the far plane).
- With `bNormalize`, `NormalizePlane` on each: `sq = fma(z, z, fma(x, x, y*y))`, `e = vrsqrtefp(sq)`
  refined twice, `len = sq*e` (0 when `sq == 0`), `inv = 1.0f / len` (`fdivs`), plane *= inv on all
  four lanes (0x8282bc60..0x8282bd20). Then the planes are copied into `m_v4_aFrustumPlane`.
- Reusable in the port: `rendering/presentation_shadow_culling.rs` already has this plane order and
  `normalize_plane` (NormalizePlane8282bc60).

### 2.7 `ViewCamera::SetPerspectiveMatrixRadians` 0x82840770

- `s = (float)(radians * 0.63661975)` (`fmuls`, 0x82840808; 2/pi, not a tangent).
- Double-precision matrix, each element rounded with `frsp` when stored (0x82840960..0x828409f0):
  `[0][0] = -1/(float)(s * 1.7777777777777777)` (0x8284084c, 0x82840898, 0x828408ac),
  `[1][1] = 1/s` (0x82840850), `[2][2] = f/(f-n)` (0x82840878), `[2][3] = 1`,
  `[3][2] = f*n/(n-f)` (0x82840880), all others 0. Stereo (`Stereoscopy::m_bEnabled`, state != 1,
  `m_bActive`) multiplies an eye matrix (assumed off, UNVERIFIED); jitter above 1e-5 adds a
  translation (`UpdateCamera` passes 0).
- `mVP = mV x mP`, per row `P0*V.x`, then fused `+P1*V.y`, `+P2*V.z`, `+P3*V.w` (0x82840be8..0x82840c54).
- `mNearPlane`, `mFarPlane`, `mFOVRadians` stored; `CalculateH(&mFrustum, (float)GfxMan->
  mDeviceParameters.frontBufferWidth, radians)` (0x82840cb4..0x82840ccc).

### 2.8 `Frustum::CalculateH(width, fov)` 0x82826810

- `t = XMVectorTan(splat(fov * 0.5f)).x` (0x8282683c..0x8282684c); `m_H = (t == 0) ? 10.0 :
  (width / t) * 0.5f` (`fdivs` 0x8282686c, `fmuls` 0x82826870; 10.0 from 0x82010250).
- `XMVectorTan` 0x82581520 is the XNA Math routine; constants verified: TanCoefficients0
  {1, -0.46671683, 0.025663832, -3.1181533e-4} (0x8203e0a0), TanCoefficients1 {4.9819437e-7,
  -0.1333835, 3.4248878e-3, -1.7861707e-5} (0x8203e090), TanConstants {1.5707964, 6.0771006e-11,
  2.4414062e-4, 0.63661975} (0x8203e0b0). The port has its graph in
  `physics/skeleton/ik/skeleton_ik_tangent.rs`.
- `frontBufferWidth`: `XBox2GraphicsMan::Init` 0x8282c3d8 sets the 720p parameters (1280) for a
  widescreen video mode, else 960x720. So `m_H = 640 / tan(fov/2)` on a widescreen console.

### 2.9 Inputs while an ICE scene owns the camera

- `CameraPresentation::GetMatrix` 0x827f6308: challenge camera if active, else `mICEMatrix` if
  `mICECameraActive`, else `mNISSequenceCameraMatrix` if active, else `GetCurrentData()`; each override
  yields to the manual camera only with `kbAllowManualCamInICE`/`InNIS` and `mManualCamEnabled`.
- `mICEMatrix` is written by `CameraPresentation::SetICEMatrix` 0x82587130 from
  `User_ICECamera::SetCameraMatrix` 0x826cd060 (vtable 0x820b32f8 slot 5), called at the end of
  `ICECameraMover::UpdateFrameEnd` (0x82a3b43c).
- `GetFOVY`: `mICEFOV` when the ICE or NIS-sequence camera is active. `mICEFOV` =
  `SetFieldOfView(u16) * 0.0054931641` (0x826d82c8), called by `ICEUserChannelLens::UpdateFrameEnd`
  0x826cd7e8 with `ICE::ConvertLensLengthToFovAngle(clamp(element 9, or 10 when element 101, to element
  9's range))`.
- `GetNearPlane`: `mICENearPlane` (same lens function: element 14 clamped, stored only when `>= 0`,
  0x826cd914..0x826cd918); 0.8 for the NIS-sequence camera; else `Iris::kNearClipPlaneDistance`.
- `GetFarPlane`: rendering attribute +0x64, unchanged in a NIS.

### 2.10 Bloom pulse (also started by case 5)

- `FireEvent` (0x826d1f44..0x826d1f8c), `PostfxPresentation` at Presentation+0x7940:
  `bloomAnimBaseValue (+0x1CC) = bloomThreshold (+0x1C4)`, `bloomAnimValue (+0x1D0) = -0.6`
  (`s_fBloom` 0x82fe2d8c), `bloomAnimCurrentTime (+0x1DC) = 0`, `bloomAnimUpTime (+0x1D4) = 0.15`
  (0x82fe2d88), `bloomAnimDownTime (+0x1D8) = 0.1` (0x82fe2d84). The three statics have no other xref.
- `PostfxPresentation::Update(dT)` 0x827efe58 (from `Presentation::Update` 0x827f7608, real frame dT):
  `t < 0`: nothing; `t < up`: `time = t + dT`, `threshold = fmadds(value - base, t/up, base)`;
  `t < up + down`: `time = t + dT`, `threshold = fmadds(base - value, (t - up)/down, value)`; else
  `time = -1.0`, `threshold = fmadds(base - base, 0, base)`.
- `ICEUserChannelBloom::UpdateFrameEnd` 0x826c88e0 overwrites `bloomThreshold` with element 37 every
  frame when element 112 is set; without it, it restores the default threshold only while no pulse
  runs (`bloomAnimCurrentTime < 0`).

## 3. The sphere (question 2)

### 3.1 `GetMinPixelSize`

`SkaterPresEntity` vtable 0x82111238 slot 15 (+0x3C) is `PresentationEntity::GetMinPixelSize`
0x827d0ac8: constant 19.0 (0x41980000 at 0x82110f9c). `ColorizedSkaterPresEntity` (vtable 0x82111294)
has the same slot.

### 3.2 `GetBoundsCenter`

- Slot 14 (+0x38) `SkaterPresEntity::GetBoundsCenter` 0x827d79b0 tail-calls slot 16 (+0x40)
  `GetPosition` 0x827d79c0: `mPresentationCallbackHandler` (+0x1E4) slot 1 `GetHipsPos(result, handler,
  mIndex (+0x1E0))`, copied with w := 1.0 into `mSkaterPosition` (+0x1F0), whose address is returned.
- In a photo outro the local entity is the gameplay one, created by `SkaterPresentation::UpdatePresEntity`
  0x827e61e8 (`eEntTypSkater` 0x122, handler = SkaterPresentation+0x18, index = data row; `DoSimRestore`
  rewrites `mIndex` each restore). PHOTO_OUTRO scenes have no character records and are not
  "character NIS" (type 6), so it is never borrowed: the handler is `SkaterPresentation::GetHipsPos`
  0x827e3ef0 = `mInterpolatedData[row].mHipsTransform.row3 x root` (root at data+0x10, mHipsTransform at
  data+0x50, data = SkaterPresentation+0x20+0xB0*row; `vmulfp128` then three fused `vmaddfp`).
- Row = `GetLocalIndex` 0x827d0090: first row whose flag word (data+0x9C) has bit 0x04000000 (the
  `local` bitfield, LSB-first), -1 if none.

### 3.3 `m_BR` (+0x144)

Written only by `PresentationEntity::CalculateBoundsInfo` (0x827f2954) and the `SkaterPresEntity`
constructor (0x827e5b98); a text scan for `stfs f*, 0x144(` finds no other PresentationEntity writer
(other hits are unrelated classes). Computed once when the entity is built:

- `CalculateBoundsInfo` 0x827f2708 (from `InitFromRecipe` 0x827fab78): box = LOD0 part 0's model box,
  grown per lane x/y/z by the other LOD0 parts (w lanes from part 0); `D = max - min` (4 lanes),
  `L = |D|` (`vmsum4fp128`, rsqrt estimate + two Newton steps, times, 0 if zero), `R0 = 0.5*L`;
  `m_BB.min.w = R0`, `m_BR = R0`, `m_localBC = ((max + min)*0.5).xyz, w 1`, `m_localBB = m_BB`.
  (The port's `world_entity_bounds` in `presentation_shadow_culling.rs` reproduces R0.)
- `SkaterPresEntity` ctor 0x827e5740, after `InitFromRecipe` (0x827e5a18..0x827e5bb8): `min.y -= 1.0`
  and `max.y -= 1.0` (only lane y: `vperm128` with `gSwizzleStoreConstants[5]` =
  {00010203, 14151617, 08090A0B, 0C0D0E0F}, built by the dynamic initializer 0x82f7efe8), then
  `m_BR = 0.5 * |max' - min'|` over FOUR lanes, so the w term is `(max.w - R0)`; `m_localBC =
  ((max' + min')*0.5).xyz, w 1`, `m_localBB = m_BB`.
- So `m_BR = 0.5*sqrt(dx^2 + dy'^2 + dz^2 + (max.w - R0)^2)`. The numeric value depends on the recipe's
  model boxes, including their w lanes (UNVERIFIED data; if both w are 0 this is about
  0.559*|dxyz|).
- Not traced: whether `InitFromRecipe` (virtual slot 1) is ever re-run on the local entity, which
  would reset `m_BR` to R0 (UNVERIFIED).

## 4. `Sk8::Scene::CharacterPoseSaver` (question 3)

- Layout (type size 0x50): vtable +0 (0x82140a1c: dtor 0x8286d668, `IsSaving` 0x82a74658, `SavePoseBuffer`
  0x82862578), `mSaving` +4, `mNumBones` +8, `mPoseBuffer` +0xC (`Matrix44Affine*`), `mLocalToWorld`
  +0x10 (`Matrix44`).
- Built by `SceneManagerImpl::Init` (0x828759fc..0x82875a20) into SceneManagerImpl+0x38 with
  `mSaving = 0`, `mNumBones = 0`; `mPoseBuffer` and `mLocalToWorld` are not initialized.
- Start: `UpdateLoadState` 0x82875bc0 load state 7 (just before `Play`), for
  `GetSceneType(gNISSceneHash) == SCENE_TYPE_PHOTO_OUTRO` (6) (0x82876484): `mHideAISkatersForPhotoOutro
  = GetFixedFramePhotoCount() > 0` (0x828764a4..0x828764b0), `mSaving = 1` (0x828764d0),
  `CreateBuffer(Presentation->mSkaterPresentation.mNumBones)` (Presentation+0x104A0, 0x828764d8). Every
  PHOTO_OUTRO saves, with or without command 5. The ICE editor path does the same in
  `ICESceneBridge::ActivateICEScene` 0x826cc3a0 (0x826cc514..0x826cc518).
- `CreateBuffer` 0x82867428: `mNumBones = n`, `mPoseBuffer = Alloc(n*64, "CharacterPoseSaverBuffer",
  kMemCategory_Scene)`, `MemClear`. One buffer: no ring, no history.
- Record: `SavePoseBuffer(localToWorld, poseBuffer)` 0x82862578: if `IsSaving()`, copy the four rows into
  `mLocalToWorld` and `mNumBones` matrices into `mPoseBuffer` (overwrite). Its only caller is
  `SkaterPresentation::UpdateBodies` (0x827dec60..0x827dedac, through `SceneManager::PoseSaver`
  0x828606f8), whose only caller is `DoSimRestore` (0x827e7014). `DoSimRestore` does nothing in a
  character or flythrough NIS or with the CAS manager active (0x827e6488..0x827e64d4), and it only runs
  when the data system restores: during a NIS that needs `RunReplayNIS` (0x828a3c70 pushes pause state 7
  after `PauseSim`, 0x828a3df0), since a basic NIS's state 11 pauses the restore. So it records once
  per presentation frame of a replay-controlled photo outro.
- What is saved (0x827dec80..0x827dedac): row = first data row with the `local` bit (0x04000000), -1 if
  none (the native then reads before the array). `poseBuffer = mInterpolatedData[row].mPoseBuffer.
  mPoseBuffer` (data+4): the `ExtractMatrix` output of the second ACS pass (`SetSQT`, `SetMatrix(entity
  InvBindPoseBuffer (+0x214))`, `PoseWithMatMult`, 0x827deb70..0x827deba0), i.e. the skinning palette,
  with slot 0 then overwritten by that row's `mHipsTransform` (0x827debd4..0x827dec30).
  `localToWorld = mHipsTransform x root` (`vmulfp128` + three fused `vmaddfp`, 0x827dece4..0x827ded9c),
  the same product `DoSimRestore` passes to the entity's `SetLocalToWorldMatrix` (0x827e7160..0x827e7248).
- `GetMostRecentPoseBuffer` 0x82862648: `IsSaving() ? mPoseBuffer : null`. Before any restore it
  returns the zero-filled buffer (and an uninitialized or stale `mLocalToWorld`).
- End: `StopDestroyUnloadNIS` 0x8286dfe0 for PHOTO_OUTRO calls `EndSaving` 0x826c7278 (`mSaving = 0`,
  `DestroyBuffer` 0x828674b0) BEFORE `Stop`/`Destroy`; `DestroyBuffer` frees without nulling
  `mPoseBuffer`. (Editor: `DeactivateICEScene` 0x826cc548.) Native edge: a PHOTO_OUTRO stopped before
  load state 7 frees the previous buffer again or null; do not port that.

## 5. The frozen `SceneCharacter` (question 4)

- `UpdateFrameEnd` 0x826cf1a8: if the flag: clear it (0x826cf1d0), `pose = GetMostRecentPoseBuffer()`
  (0x826cf22c); if non-null `e = SceneCharacterManager::InstanceFrozenSkater(&saver->mLocalToWorld,
  pose)` (0x826cf270); if non-null `ScenePlayer::AddEntity(GetActiveSceneHandle(0), e)` (0x826cf2b0).
- `InstanceFrozenSkater` 0x82874f90: null pose -> null; `idx = FindFreeManagerIndex()` 0x82865e10 (first
  null `mSceneRenderModels[idx]`, asserts at 32); `new SceneCharacter(idx, localToWorld, pose)` (0xB0
  bytes, 0x82874fe8), stored in `mSceneCharacters[idx]`; `hir = GetPlaybackData("SkaterFullPlaybackData")
  ->mHir` (+0x34); `recipe = SkaterPresentation::GetPlayerRecipeID()` 0x827dd270; if `recipe` is valid,
  `mCharacterAnim->IsValid()` and `CreateCharacterRenderable(idx, hir, recipe, true)`:
  `SetObjectTransform(localToWorld)` (vtable +0x24, 0x828750a8) and return it; else undo and delete.
- `SceneCharacter(int, const Matrix44&, const Matrix44Affine*)` 0x82874700: `mSchemaName = ""`,
  `mTransform = mUserTransform = identity`, `mCharacterAnim = new SceneCharacterPose(localToWorld,
  pose)` (0x30 bytes).
- `SceneCharacterPose` ctor 0x82873d00: `mFrozenPoseBuffer.mHir` (+0x20) = the full skater hierarchy;
  `mFrozenPoseBuffer.mPoseBuffer` (+0x24) = new array of `mNbBones` matrices, a deep copy of `pose`
  (so the saver buffer can be freed); `mHipsPos` (+0x10) = `pose[GetBone("Hips")].row3 x localToWorld`
  (`vmulfp128` + three fused `vmaddfp`, 0x82873eb8..0x82873ef8; four lanes of the row). Whether this
  equals the gameplay `GetHipsPos` value is UNVERIFIED.
- `SceneCharacterPose` vtable 0x82140a6c (slots named from `SceneCharacterAnim` 0x82140f70): 0 dtor
  0x8286adb0 (deletes the copy), 1 `GetPlaybackData` -> null (0x82784530), 2 `GetPoseBuffer` -> +0x20
  (0x82d16218), 3 `EvalCurrAnim`, 4 `AddTime`, 5 `InitFace`, 6 `ResetAnim`, 7 `SetAnimToTime`: all the
  empty 0x825187c0, 8 `GetHipsPosition` -> copy of +0x10 (0x82e6cf20), 9 `IsValid` -> 1 (0x826c6d80),
  10 `GetLength` -> 0.0 (0x82867190).
- So it plays no animation: `SceneCharacter::SetTime` 0x82862080 / `AddTime` 0x828620d0 call no-ops, the
  object matrix is set once, and `Scene::AddEntity` 0x82874f10 only calls the entity's `Init` (empty for
  `SceneCharacter`) and appends it to `mSceneEntities`. It also becomes a scene entity for
  `GetEntityLocation` (ICE avatars 4/5+) from then on.
- `CreateCharacterRenderable` 0x82871150: the last argument is `clothDisabled`, passed as the
  `SkaterPresEntity` ctor's `isClothDisabled` (0x82871248); the ctor also gets `IsLocalPlayer = 0`
  (0x82871244 -> `mIsLocalPlayer` +0x628, 0x827e57e8) and peer id -1, and sets `mIsClothDisabled` when
  `!IsLocalPlayer || isClothDisabled` (0x827e5910), so cloth is off either way. For a recipe that is
  not LivingWorld/Vehicle (the player's CreateACharacter or marquee recipe) the branch is
  `SkaterPresEntity`, type `eEntTypNISChar` 0x822, `SetTechnique("nis")`; then
  `mRenderModelOwnerships[idx] = 1` and `AddToView` (0x8287134c..0x82871354).
- Rendering: `SkaterPresEntity::Update` 0x827e8568, non-local path: bones =
  `GetSkaterBoneBuffer(mIndex)` 0x82865d60 (`mCharacterAnim->GetPoseBuffer()->mPoseBuffer`, the copy),
  `UpdateBoneTransforms` 0x827efb78 and `PackAndMultiplyMatricesForUpload` 0x828221e0 with `m_MatLtoW`
  (the saved local-to-world); technique `default` (not `nis`, see the scene-character notes); light
  probe at `GetPosition` -> `SceneCharacterManager::GetHipsPos` 0x82865d10 -> `mHipsPos`.
- Visibility: `DetermineVisibilityFromGameState` 0x82829698: in a PHOTO_OUTRO
  (`IsInPhotoChallengeNIS` 0x82861158) everything is visible except the 0x222/0x122 special cases;
  0x822 matches neither (0x82829738, 0x82829798), so the frozen copies draw next to the replaying
  gameplay skater (type 0x122, the player's recipe, also visible).
- Lifetime: until `Scene::Purge` 0x8286a908 (`ScenePlayer::Destroy`, from `StopDestroyUnloadNIS`;
  editor `DeactivateICEScene`): each entity is purged and deleted -> `~SceneCharacter` 0x82871370 ->
  `RemoveCharacterRenderable` 0x8286d478 (owned: `SetTechnique("default")`, `RemoveFromView`, release),
  `mSceneCharacters[idx] = null`, delete `SceneCharacterPose` (frees the copy). Nothing removes it
  earlier; a sequence shot accumulates one per successful command 5.

## 6. `ICEUserChannelFreeze` state (question 5)

- Layout (0x24 bytes): `mPreviousInterval` u16 +0xC, `mIsTakeChange` +0xE, `mInstanceFrozenSkater`
  +0xF, `mTypeID` 34 +0x10. Ctor 0x826cf100: `mPreviousInterval = 0xFFFF`, both flags 0.
- Stores to +0xF: ctor (0x826cf158, 0), `FireEvent` (0x826d1ef0, 1), `UpdateFrameEnd` (0x826cf1d0, 0).
  An `stb` scan of the other class functions finds only `UpdateFrameBegin`'s `mIsTakeChange = 0`
  (0x826d2520); the UI functions (0x826cb410, 0x826cb4a8, 0x826cb508, 0x826cb5c8) and the dtor
  0x826d12d8 store no byte.
- Vtable 0x820b3c30: `Init`, `OnTakeSetup`, `OnRelease` are the empty 0x825187c0; `OnTakeStart`
  0x826cb400 only sets `mPreviousInterval = 0xFFFF`; `OnTakeChange` 0x826d6e78 sets it and
  `mIsTakeChange = 1`. Neither resets `mInstanceFrozenSkater`.
- The flag is consumed by the next `UpdateFrameEnd` that runs (normally the same frame). If that call
  is skipped (section 1 step 4: no ICE camera, no active scene, queueing scene) the flag would survive
  into a later scene; it only produces a character while a PHOTO_OUTRO is saving. The object is one
  registered channel, so the flag is not per scene.
- `UpdateFrameBegin` 0x826d2498 (for reference): fires every interval from `mPreviousInterval + 1` to the
  current one, element 141 = command, element 142 = frames.

## 7. Data census (scratch, `frozen/census_photo.py`, `frozen/census_cmd5.py`)

- `scene_000.bin`: 64 PHOTO_OUTRO (type 6) scenes, all `mIsReplayOn = 1`, none with character records;
  PHOTO_INTRO: 2 replay, 31 not.
- Freeze command 5 appears only in NIS_Photo, only in type-6 scenes, 88 intervals in 11 scenes:
  killed_m_2a_01_security_jerks, photo_killed_m_1a_05_photo_rail, photo_killed_m_2a_03_security_jerk,
  photo_killed_m_5a_01_thrash_haslam, photo_killed_m_6p_01_pool_thrash_4, photo_out_m_1a_05_photo_rail,
  photo_out_m_2a_01_security_jerks, photo_out_m_2a_03_security_jerks, photo_out_m_5a_01_thrash_haslam,
  photo_out_m_5a_05_sbm_about_city, photo_out_m_6p_01_pool_thrasher_4. Each has one take whose channel
  34 has 9 intervals: command 0, then command 5 (frames 10) in intervals 1..8, starting at packed
  parameters about 1/6, 0.28, 0.39, 0.50, 0.61, 0.72, 0.83, 0.94 (e.g. photo_out_m_1a_05_photo_rail:
  10924, 18272, 25498, 32775, 40049, 47342, 54646, 61924 / 65535).
- Which script or presentation event starts each photo outro (`RunReplayNIS` callers: Lua
  `ModuleNIS::EnterReplayNIS` 0x8288bba0, the presentation director's Replay event) is UNVERIFIED.

## 8. Port notes

- Today `fire_freeze_event` (`nis/manager.rs`) reports `FreezeCommandNotPorted(5)`, and `create_scene`
  gives replay scenes a 1.5 s length with no replay frames, so the port has no restore during a photo
  outro: the native pose source (`DoSimRestore` -> `UpdateBodies` -> `SavePoseBuffer`) does not exist
  yet. A 1:1 frozen skater needs the replay restore first; substituting the live gameplay pose would be
  invented behaviour.
- Pieces that already exist: the ICE projection (`camera_producer_projection.rs`), plane extraction and
  `normalize_plane` (`presentation_shadow_culling.rs`), `CalculateBoundsInfo`'s R0
  (`world_entity_bounds`), `XMVectorTan` (`skeleton_ik_tangent.rs`), the `UpdateBodies` palette chain
  (`presentation-palette-2026-09-22.md`), scene-character renderables
  (`session/selected_session_scene_characters.rs`) and the port's `vmsum3fp128`/`vmsum4fp128`
  reduction policy.
- Missing: `TestSphere_VMX` + `GetPixelSize` + `CalculateH` (`m_H`) against the previous frame's camera
  5 frustum; the local entity's `m_BR` (ctor y shift and four-lane radius); the frozen character
  (static palette = saved copy, object matrix = saved local-to-world, hips = `pose[Hips].row3 x L`, no
  evaluation, type 0x822, default technique, owned renderable, freed at scene purge); the bloom pulse
  and its interplay with the bloom channel; FE event 85 (overlay movies for 85 are not ported either).

## 9. UNVERIFIED / open

1. Enum names of the per-plane flags 1/2/4/8/16/32 (values verified).
2. The recipe model boxes' w lanes, hence the numeric `m_BR`; whether `InitFromRecipe` can be re-run on
   the local entity.
3. The console video mode (1280 vs 960 front buffer) for `m_H`.
4. Xenon rounding of `vmsum3fp128`, `vmsum4fp128`, `vrsqrtefp128`, `vrefp128` (port-wide open item).
5. `DataMan::RestoreData` -> `DoSimRestore` dispatch (taken from earlier port evidence).
6. `GetPlayerRecipeID`'s map key; `pose[Hips].row3 x L` versus the gameplay hips position.
7. Contents of `mLocalToWorld` before the first save (allocator fill), and in-game behaviour in general
   (nothing here was observed running).
8. `XMVectorTan`'s VMX sequence was not re-decoded here (constants checked; the port has a graph).
