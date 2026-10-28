> Agent research (2026-10-04), read-only IDA. Not ported yet: re-verify each fact in IDA
> before porting it. Section numbers refer to this file.

# NIS scene characters (sk82_na_m.xex, IDB 8ee0dbb0, read-only)

All addresses are from the IDB. "Verified" means read from decompile and/or disassembly at the cited
address. Anything not traced is listed under "Unverified / open questions" at the end.

Record layout of `SceneBinCharacterInfo_t` as the code reads it (offsets from the record start;
record address = `mBuffer + mHeaderSize + mScenePropertiesSize + 180*(mPosSceneInfoStart + i)`,
computed identically at 0x8286f330 and 0x82877930):

| off | field | used by |
|---|---|---|
| 0x00 | mName[32] ("schemaName") | AddLoaders 0x8286f37c, AddEntities 0x82877a50/0x82877a7c |
| 0x20 | mCategory[16] | compared with "dynamic" at 0x8286f33c and 0x828779b0 |
| 0x30 | mStartAnimDB[64] ("animDBName") | AddSceneCharacterAnimLoader 0x8286f398; InstanceCharacter r6 |
| 0x70 | mStartAnimName[36] ("animPlayName") | InstanceCharacter r7 |
| 0x94/0x98/0x9C | mLocationXYZ | 0x82877934..3c |
| 0xA0/0xA4/0xA8 | mOrientationXYZ | 0x8287796c..78 |
| 0xAC | mAnimOffset (float) | `lfs f1,0xAC(r31)` 0x828779d8 / 0x82877a64 |
| 0xB0 | mSnapToGround (u32) | `lwz r9,0xB0(r31)` 0x8287797c -> bool (`!=0`) passed as last stack arg |

Data census of scene_000.bin (from dump_scene.py, not IDA): categories livingworld 486, marquee 224,
dynamic 100, vehicle 8; character orientation (0,0,0) in 433 records, (-1,0,0) in 53, others
arbitrary yaw vectors; mSnapToGround 0:474 / 1:344; mAnimOffset 0.0 in 549 records, otherwise small
positive values (0.0067 .. 1.1 s).

## Summary for the port (details and addresses in the sections below)

1. Every scene character record becomes one `SceneCharacter` entity plus two loaders (recipe +
   `.abin`); the only category string the scene code tests is "dynamic" (plus "marquee" in the
   resolver); the category only selects the recipe category and the render class
   (LivingWorld/Vehicle -> ColorizedSkaterPresEntity, else SkaterPresEntity).
2. "current_player" (and "dynamic" records with a player-id override) borrow the existing gameplay
   `SkaterPresEntity` (retyped 0x822, bone/hips callbacks redirected to the scene); every other name
   gets a new render entity from `CharacterAssetManager::LoadSchemaFromFile(name)`.
3. Per frame: `clip time = mAnimOffset + sceneTime` (absolute jump on SetTime, +dt on AddTime);
   non-looping clips clamp/hold the last frame; looping is a per-clip header flag. Local pose =
   bind(+face) pose composed with the clip (AddSQT); hips local transform and trajectory bone
   (root motion) are moved into the object matrix `Obj = H * Traj * mUserTransform * SceneTransform`.
4. `SceneTransform = ComputeTransform(loc, orient, normal)`: rows X, Y, Z, loc with
   Z = -normalize(orient), X = normalize(normal x Z), Y = Z x X. The per-character `mUserTransform`
   uses the same construction with up (0,1,0) but stores the 3x3 **transposed** (columns), with the
   (possibly ground-snapped) character location in row 3.
5. During a basic NIS the simulation is paused (PauseMan state 11) and, in "character" NIS types,
   gameplay skaters (0x122), pedestrians (0x222), traffic (0x422) and dynamic objects (0x42, unless
   enabled) are culled by the render-type mask 0x282F; they reappear (still frozen) when the scene is
   destroyed (`mNISHandle = 0`) and resume when `EndNIS -> ResumeSim` pulls the pause state.
6. `NISCharacterResolver` only enforces a 6-marquee-model memory budget (unload by priority, reload
   challenge AI skaters afterwards); it polls asset managers with no timeout.

---

## 1. SceneBinDirectory::AddLoaders (0x8286f2c0) and ::AddEntities (0x82877838)

### 1.1 The only category string compared is "dynamic"

Neither function compares "marquee", "livingworld" or "vehicle". The only string test on
`mCategory` is `StringCompare(rec+0x20, "dynamic")` (AddLoaders 0x8286f33c, AddEntities 0x828779b0).
The real category is resolved later:

* `SceneCharacterLoader::Load` 0x82863840 calls `Character::GetCategory(mCategoryName)` 0x82cfd2b0
  (only when the category is not "dynamic", 0x8286394c..58).
* `Character::GetCategory` 0x82cfd2b0 walks `Character::gCategoryMapping` 0x83024804 (6 entries):
  0 "marquee", 1 "livingworld", 2 "vehicle", 3 "createacharacter", 4 "undefined", 5 "test"
  (strings at 0x8231BD00/08/14/1C/30/3C). A name not found returns 4.
* The category decides the render class in `SceneCharacterManager::CreateCharacterRenderable`
  0x82871150: recipe category LivingWorld or Vehicle -> `ColorizedSkaterPresEntity` (0x970 bytes,
  ctor 0x827e5e20); anything else (marquee, createacharacter, ...) -> plain `SkaterPresEntity`
  (0x950 bytes, ctor 0x827e5740) + `SetTechnique("nis")` (0x828712b0). Both are constructed with
  entity type `eEntTypNISChar` (0x822) and the scene-character index. See section 2.7.

There is no separate entity class per category: every scene character is a
`Sk8::Scene::SceneCharacter` (section 2). Props such as "zprop_pop_can", vehicles such as
"minivan_01" and "giovanni_reda_prop" go through exactly the same path (character recipe + .abin).

### 1.2 AddLoaders 0x8286f2c0 (pseudo-code, verified)

```
props = GetSceneProperties(sceneHash)                 // 0x8286f2d8
if !props: return
nonDynamic = 0
for i in 0 .. props.mNumSceneInfoCharacters-1:
    rec = record(props.mPosSceneInfoStart + i)          // 0x8286f330
    if StringCompare(rec.mCategory, "dynamic") != 0: nonDynamic++     // 0x8286f33c..48
    if nonDynamic <= kNISCharacterLimit:                // 0x8286f358
        addLoader->AddSceneCharacterLoader(sceneHash, i, rec.mName, rec.mCategory)  // 0x8286f37c
        addLoader->AddSceneCharacterAnimLoader(sceneHash, rec.mStartAnimDB)          // 0x8286f398
```

* `kNISCharacterLimit` (cVarIntRef 0x830589ac, registered at 0x82f71fe8 as
  "NIS"/"kNISCharacterLimit", default 32, min 0, max 32). Only non-"dynamic" characters count, but
  once the count is exceeded every later record (dynamic too) is skipped.
* The IAddLoader implementation is `SceneAssetLoaderManager`:
  * `AddSceneCharacterLoader` 0x82874dd0: `new SceneCharacterLoader(schemaName, categoryName)`
    (0x40 bytes, ctor 0x8286f3b0) pushed onto `GetLoadersForScene(sceneHash)` (0x82871d78).
  * `AddSceneCharacterAnimLoader` 0x82874e68: `new SceneCharacterAnimLoader(animDBName)` (0x24 bytes,
    ctor 0x8286f4c8) pushed onto the same per-scene vector.
  So every character contributes exactly two loaders: one character (recipe) loader, one .abin loader.

### 1.3 SceneCharacterLoader (character model / recipe) - verified

* ctor 0x8286f3b0: copies mSchemaName (= mName) and mCategoryName; mFoundLoaded=0, mError=0;
  embedded `CharacterAssetsLoaded` callback (mAssetID = kInvalidAssetID, mComplete=0).
* `Load` 0x82863840:
  ```
  mError = 0
  isDynamic = (mCategoryName == "dynamic")
  if (isDynamic && !ICEManager::IsEditorOn()) || mSchemaName == "current_player":
       mCharacterAssestsLoaded.mComplete = 0; mAssetID = kInvalidAssetID
       mFoundLoaded = 0; mError = 1                      // 0x828638b8..cc  -> nothing is loaded
  else:
       cloneOn  = ScenePlayer->vtbl[+0xC]()  // 0x82863930..40 -> 0x82861b20 IsCharacterCloneOn = cvar kNISCharacterCloneOn (default 1, 0x82f72180)
       category = isDynamic ? 0 (Marquee) : Character::GetCategory(mCategoryName)   // 0x82863950
       CharacterAssetManager::LoadSchemaFromFile(schemaName=mSchemaName, category,
            schemaLoadedCallback=null, assetsLoadedCallback=&mCharacterAssestsLoaded,
            memCategory=kMemCategory_CharMarquee, userDataCallback=null,
            cloneType = cloneOn ? 2 (EverythingButBlendTargetsClone) : 0 (NoClone))  // 0x828639b0
  ```
  `LoadSchemaFromFile` 0x82d09720 only lower-cases the name, hashes it (`RwHash64String`) into
  `schemaAssetID` and queues a `Request` (state `RequestState_LoadSchemaFromFile`) on
  CharacterAssetManager::mRequests under a critical section; the file path is built later by the
  request processor (not traced, see open questions).
* `IsLoaded` 0x82861930: `if !mFoundLoaded && mComplete: mFoundLoaded=1; return mError || mComplete`.
  => "current_player" and non-editor "dynamic" characters report loaded immediately (mError=1).
* `Unload` 0x828639c0: if !mError: `recipe = CharacterAssetManager::GetRecipe(mAssetID)`; if recipe:
  `CharacterAssetManager::RequestUnload(recipe, null)`; then clears mComplete/mAssetID/mFoundLoaded.

### 1.4 AddEntities 0x82877838 (pseudo-code, verified from disassembly)

```
charMgr = SceneManager.mSceneManagerImpl->mSceneCharacterManager    // 0x828778b0
props   = GetSceneProperties(sceneHash)
nonDynamic = 0
for i in 0 .. props.mNumSceneInfoCharacters-1:
    rec = record(props.mPosSceneInfoStart + i)
    location    = rec.mLocationXYZ (w=0)                              // 0x82877934..54
    if i < characterLocations.mNumLocations (u32 at +0x200):          // 0x82877920/0x82877958
        location = characterLocations.mLocations[i]  (16-byte Vector3 array at +0, max 32)  // 0x82877960
    orientation = rec.mOrientationXYZ (w=0)                           // 0x8287796c..ac
    snap        = (rec.mSnapToGround != 0)                            // 0x8287797c..a4
    isDynamic   = (rec.mCategory == "dynamic")                        // 0x828779b0..b8
    if isDynamic && i < overrides.playerIndexArray.mPlayerIndexCount (u32 at +0x90):  // 0x828779c8
        e = charMgr->InstanceCharacter(sceneHash,
                playerIndex = (s8) overrides.playerIndexArray.mPlayerIndexes[i] (+0x94+i),
                rec.mStartAnimDB, rec.mStartAnimName, applyFace = 1,
                location, orientation, rec.mAnimOffset, snap)         // 0x828769f0, call 0x82877a04
    else:
        if !isDynamic: nonDynamic++                                   // 0x82877a0c..14
        if nonDynamic > kNISCharacterLimit: continue                  // 0x82877a24
        if isDynamic && !ICEManager::IsEditorOn(): continue           // 0x82877a28..40
        applyFace = (rec.mName == "current_player")                   // 0x82877a50..5c
        e = charMgr->InstanceCharacter(sceneHash, schemaName = rec.mName,
                rec.mStartAnimDB, rec.mStartAnimName, applyFace,
                location, orientation, rec.mAnimOffset, snap)         // 0x82876bd8, call 0x82877a88
    if e: addEntities->AddEntity(e)                                   // vtbl+4, 0x82877aa8
```

* `addEntities` is the `Scene` itself (Scene::Init passes `this`, 0x82877e6c);
  `Scene::AddEntity` 0x82874f10 calls `entity->Init()` (empty for SceneCharacter) and appends to
  `Scene::mSceneEntities`.
* `NISPlaybackOverridesInternal::playerIndexArray` (count at +0x90, s8 player ids at +0x94) maps the
  i-th character record to a *game player id* - only used when that record's category is
  "dynamic". A "dynamic" record without an override is dropped outside the ICE editor.
* `SceneLocationArray` (`Vector3 mLocations[32]` at +0, `u32 mNumLocations` at +0x200) overrides
  `mLocationXYZ` of record i (same index i as the scene's character list), for every category.
* **mSnapToGround is not used by AddEntities / InstanceCharacter** (it is passed as the last stack
  argument, but the SceneCharacter ctor
  `??0SceneCharacter@...@@QAA@HPBD00_NABVVector3@vpu@math@rw@@2M@Z` 0x82875100 has no such
  parameter and neither InstanceCharacter reads it). It **is** used earlier, by the scene manager's
  ground-snap pre-pass that produces the `characterLocations` array (see 1.4b).

### 1.4b Where mSnapToGround is really used: the character location snap pass (verified)

* `SceneManagerImpl::PreSnapFillLocationList` 0x82874ad8 (scene = `gNISSceneHash` 0x83058834):
  for every character record i: `mCharacterLocationList.mLocations[i] = rec.mLocationXYZ` (w=0) and
  `mLocationNeedsSnapList.push_back(rec.mSnapToGround != 0)` (0x82874bfc); returns true if any record
  has mSnapToGround != 0.
* `SceneManagerImpl::SnapLocationsToGround` 0x8286e1d0: `T = Scene::ComputeTransform(mSceneLocation,
  mSceneOrientation, mSceneNormal)`; copies only the flagged locations into a temp list and calls
  `SceneLocationSnapper::SnapLocationsToGround(list, T)` 0x8286d6d8, which stores
  `mInverseSceneTransform = Inverse(T)` (`rw::math::vpu::Inverse` 0x82600f38), transforms every
  location to world space as a point (`(x,y,z,1) * T`) and posts a message (kind 0xFE94DD4B,
  size 0x220) to the simulation broadcaster.
* `SceneLocationSnapper::ReceiveNISMgrLocationListSnapResponseMsg` 0x8286ae98: copies the returned
  world positions, transforms each back with `(x,y,z,1) * mInverseSceneTransform` (only xyz
  replaced, 0x8286af1c..0x8286af70), sets `mSnappedLocationsRecieved = 1`.
* `SceneManagerImpl::UpdateLocationListWithSnappedLocations` 0x82868a00: writes the snapped
  scene-local positions back into the flagged slots of `mCharacterLocationList` (order preserved;
  unflagged slots keep mLocationXYZ). This list is the `characterLocations` that reaches
  `ScenePlayer::Create` -> `Scene::Init` -> `AddEntities` (row 3 of mUserTransform, section 2.2).
* The scene placement itself is ground-snapped too: `ReceiveNISMgrWorldCollisionResponseMsg`
  0x82861808 (only in mNISLoadState 3) sets `mSceneLocation = msg+0x10`, `mSceneNormal = msg+0x20`
  and re-projects the orientation onto the ground plane:
  `mSceneOrientation = cross(N, cross(O, N))` (= O*(N.N) - N*(N.O), not normalised; decoded from
  0x82861838..0x82861898), then `mGroundLocationRecieved = 1`.
* How the simulation side performs the snap (ray/collision query) was not traced (open question).

### 1.5 SceneCharacterManager::InstanceCharacter

Two overloads; both return null (character silently skipped) when
`SceneAssetLoaderManager::GetAnimLoaderData(animDBName)` 0x8286f990 is null (the .abin is not
loaded), 0x82876a80 / 0x82876ca0. Both pick a slot with `FindFreeManagerIndex` 0x82865e10 (first
null `mSceneRenderModels[k]`, k < 32; asserts at 32), allocate a `SceneCharacter` (0xB0 bytes) and
store it in `mSceneCharacters[k]` (`this + 0x20 + 4k`).

**(a) by player id** 0x828769f0 (used for "dynamic" with override):
`pe = SkaterPresentation::GetSkaterPresEntityFromPlayerId(playerIndex)` 0x827d3380; null -> return
null. `schemaName = pe->m_Recipe->mSchemaName` (or "" if none). `new SceneCharacter(k, schemaName,
animDB, animName, applyFace, location, orientation, animOffset)`. If `mCharacterAnim->IsValid()` and
`ControlCharacterRenderable(k, pe)` 0x82870fb0 succeed -> `mCharacterAnim->InitFace()` and return;
otherwise slot cleared and the SceneCharacter deleted.

**(b) by name** 0x82876bd8:
```
isCurrentPlayer = (schemaName == "current_player")                       // 0x82876c30
clothDisabled   = !ScenePlayer->vtbl[+8]()   // 0x82876d00..1c -> 0x82861b08 IsCharacterClothOn = cvar kNISCharacterClothOn (default 0, 0x82f72130) => cloth disabled by default
if !isCurrentPlayer:                                                     // marquee/livingworld/vehicle/...
    sc = new SceneCharacter(k, schemaName, ...)                          // 0x82877008
    id = SceneAssetLoaderManager::GetSchemaAssetID(sceneHash, schemaName)  // 0x8287705c
    if sc.anim valid && CreateCharacterRenderable(k, playbackData->mHir (+0x34), id, clothDisabled): ok
    else: fail (slot cleared, sc deleted)
elif mSceneRenderMode != TRICKBOOK(1) || !SceneManagerImpl.mInFE:        // 0x82876d3c  (normal NIS)
    pe = GetSkaterPresEntityFromIndex(SkaterPresentation::GetLocalIndex())  // 0x82876ed0/ee0
    if !pe: return null
    sc = new SceneCharacter(k, pe->m_Recipe->mSchemaName, ...)           // 0x82876f5c
    ok = sc.anim valid && ControlCharacterRenderable(k, pe)              // borrow the gameplay skater
else:                                                                    // trickbook in FE
    sc = new SceneCharacter(k, "current_player", ...)                    // 0x82876d8c
    id = PlayerLoader::GetPlayerAssetID(Simulation::GetLocalPlayerLoader())  // 0x82876db8
    ok = sc.anim valid && CreateCharacterRenderable(k, hir, id, clothDisabled)
    if ok: sc.mSchemaName = CharacterAssetManager::GetRecipe(id)->mSchemaName   // 0x82876e88
on success: sc->mCharacterAnim->InitFace()                               // 0x828770bc
```

So: "current_player" (and "dynamic" + player-id override) **re-use the existing gameplay
`SkaterPresEntity`** of the local player (or of the given player id) - nothing is loaded for it;
every other name (marquee pros such as "rob_dyrdek"/"slappy", pedestrians, vehicles, props such as
"giovanni_reda_prop") gets a **new** render entity built from the recipe the SceneCharacterLoader
loaded (schema asset id from `GetSchemaAssetID(sceneHash, schemaName)` 0x82872f18).

### 1.6 SceneCharacterAnimLoader (.abin) and loader lookup - verified

* Loader objects for **all** scenes are created up-front: `SceneAssetLoaderManager::Init` 0x82871ca8
  calls `Purge`, then `AddLoaders(dir, GetSceneHashFromIndex(i), this)` for every scene index
  (0x82871d5c). The per-scene vectors live in `mIDToLoadersMap` (hash map sceneHash -> vector,
  `GetLoadersForScene` 0x82871d78 = find-or-create). `Load()`/`Update()`/`Unload()` are driven later by the
  scene load state machine (already documented elsewhere; not re-traced here).
* ctor 0x8286f4c8: mAnimDBName = arg; mAsyncOp = mData = 0; mDataSize = 0; mComplete = 0.
* `Load` 0x8286f5e8:
  ```
  path = "data\\" + mAnimDBName                       // 0x8286f660 / 0x8286f690
  pos = path.find("_Player")                           // 0x8286f6a8
  if pos != npos:
      pe = GetSkaterPresEntityFromPlayerId(GetLocalPlayerId())        // 0x8286f6c8/d4
      if pe && pe->m_Recipe->mCategory == Category_CreateACharacter && !pe->m_Recipe->IsMale():
          path.replace(pos, 7, "_Female")              // 0x8286f710
  mAsyncOp = FileLoader::ASyncLoad(path, 0, 0x10 /*align*/, null, SceneAnimMemoryManager::ms_AnimAllocator, 0)  // 0x8286f760
  ```
  e.g. "scene/anim/tutorial_m_1a_goto_slappys_A01_01.abin" -> file "data\scene\anim\...abin".
* `Update` 0x82861a00: when the async op is Complete -> `Release(&mData,&mDataSize)` (asserts both
  non-zero) ; on Failed/Cancelled -> Release(null) ; in all three cases mAsyncOp=0, mComplete=1.
  (A failed load therefore completes with mData == null -> InstanceCharacter later returns null and
  the character is simply absent.)
* `Unload` 0x82861970: frees mData through `ms_AnimAllocator`, resets mData/mAsyncOp/mComplete.
* `SceneAssetLoaderManager::GetAnimLoaderData(animDBName)` 0x8286f990: walks every vector in
  `mIDToLoadersMap`, for loaders with `GetType()==LOADER_TYPE_SCENE_CHARACTER_ANIM` compares the
  stored mAnimDBName byte-for-byte (length + memcmp, case-sensitive, *without* the "_Female"
  substitution) and returns the first non-null `mData` (+0x18). If none, falls back to
  `mAnimPoolLoader` (vtbl+0xC test, then compares the pool entry name at +4 and returns +0x18)
  (0x8286fc8c..dc). Note: any scene's loaded copy of the same .abin satisfies the lookup.
* `SceneAssetLoaderManager::GetSchemaAssetID(sceneHash, schemaName)` 0x82872f18: in that scene's
  loader vector, first loader with `GetType()==LOADER_TYPE_SCENE_CHARACTER` whose mSchemaName
  matches **case-insensitively** (`StringNoCaseCompare`, 0x82872f80) -> returns its
  `mCharacterAssestsLoaded.mAssetID` (+0x30); else kInvalidAssetID.
* The anim DB content is turned into Andale data in `SceneCharacterAnim::Init` (section 2.3): a new
  `Andale::DataBase` over the raw buffer (`SetDBContent(data, E_DONT_DELETE)`), a new ACS and a new
  `Andale::PlaybackData` registered in `Presentation::mPlaybackDataManager` under the name
  **mStartAnimName** (the PlaybackData name is the anim name, not the file name).


---

## 2. The scene character entity: `Sk8::Scene::SceneCharacter` (+ `SceneCharacterAnim`)

### 2.1 Classes, vtables, lifetime

* `SceneCharacter` (0xB0 bytes): `IEntity` vptr @0, `UCOM::Object` @4, `mManagerIndex` @0x14,
  `mSchemaName` (eastl string) @0x18, `mCharacterAnim` (ISceneCharacterAnim*) @0x28,
  `mTransform` (Matrix44) @0x30, `mUserTransform` (Matrix44) @0x70.
* vtable 0x82140ee8 (12 IEntity slots, names from `Sk8::Scene::IEntity_vtbl`):
  `dtor 0x828729f0 | Init 0x825187c0 (empty blr stub) | Purge 0x825187c0 (empty) |
  GetEntityType 0x82784530 (returns 0) | SetTime 0x82862080 | AddTime 0x828620d0 |
  GetSceneTransform 0x82862120 | SetSceneTransform 0x82862158 | GetObjectTransform 0x82865f08 |
  SetObjectTransform 0x82865fd0 | GetAnimationLength 0x82862230 | SwapAnimation 0x828754d0`.
* **There is no Update / BindToGame / UnBindToGame on the character.** The per-frame driver is
  `Scene::Update` 0x82870bb8 -> `Scene::AddTime` 0x82865700 -> `entity->AddTime(dt)` (section 5);
  `BindToGame`/`UnBindToGame` exist only on `Scene` and do not touch characters. The entity `Purge`
  slot is an empty stub; cleanup happens in the destructor 0x82871370:
  `SceneCharacterManager::RemoveCharacterRenderable(mManagerIndex)` (0x828713f0),
  `mSceneCharacters[idx] = null`, delete `mCharacterAnim`, free mSchemaName.
* `SceneCharacterAnim` (0x5390 bytes, vtable 0x82140f70; ISceneCharacterAnim slots: dtor,
  GetPlaybackData 0x82862260, GetPoseBuffer 0x82862278, EvalCurrAnim 0x82866cd0, AddTime 0x82862290,
  InitFace 0x82873918, ResetAnim 0x82873c00, SetAnimToTime 0x82873c80, GetHipsPosition 0x828622a8,
  IsValid 0x828749b8 (returns mAnimValid), GetLength 0x828622b8).
  Fields: twist-bone indices @0x4..0x38, `Andale::Animatable mAnimatable` @0x40,
  `mAnimOffset` @0x52F0, `mAnimValid` @0x52F4, `mApplyFace` @0x52F5, `mAnimDBName` @0x52F8,
  `mAnimPlayName` @0x5308, `mAnimRealPlayName` @0x5318, `mSceneCharacter` @0x5328,
  `mTrajBoneIndex` @0x532C, `mHipsBoneIndex` @0x5330, `mSkateboardBoneIndex` @0x5334,
  `mHipsPos` @0x5340, `mSetSQT` @0x5350, `mAddSQT` @0x5354, `mTree` @0x5358,
  `mBPSQTBuffer` @0x5374, `mTwistSQTBuffer` @0x537C. SQT = `AnimCmdSys::SQTExt`
  {mScale @0, mQuat @0x10, mTrans @0x20}, 48 bytes.

### 2.2 Constructor 0x82875100 and the per-character placement matrix `mUserTransform`

`SceneCharacter(int idx, schemaName, animDB, animName, bool applyFace, Vector3& location,
Vector3& orientation, float animOffset)`:

* `mManagerIndex = idx`, `mSchemaName = schemaName`, `mTransform = identity` (gI/gJ/gK/gL vectors
  0x8227e8f0.., stores 0x828751bc..0x828751ec).
* `mUserTransform` (@0x70) is built inline (decoded from the VMX128 code 0x828751f4..0x82875474;
  see the VMX128 register note at the end of section 5):
  ```
  o = orientation
  o.y = 0                                                     // stfs 0 over orientation.y, 0x82875208
  if o.x == 0 && o.z == 0:  o = (0, 0, -1)                    // 0x8287522c..0x828752c8
  up = (0, 1, 0)                                              // var_70 = (0,1,0,0)
  Z = normalize(0 - o)            // vsubfp 0x82875338; rsqrte + 2 Newton steps 0x8287534c..80
  X = normalize(cross(up, Z))     // 0x8287539c..0x828753e4
  Y = cross(Z, X)                 // 0x828753fc..0x82875404
  mUserTransform rows (stores 0x82875468..0x82875474):
     row0 = (X.x, Y.x, Z.x, 0)
     row1 = (X.y, Y.y, Z.y, 0)
     row2 = (X.z, Y.z, Z.z, 0)
     row3 = (location.x, location.y, location.z, 1)
  ```
  (Corrected while porting: the first draft missed the `o.y = 0` store; the port's construction
  matches the emulated native bit for bit over 26 cases.)
  i.e. the **basis vectors are stored as columns** (transposed 3x3) while the translation sits in
  row 3. This differs from `Scene::ComputeTransform` (section 5.1), which stores X, Y, Z as rows.
  With the default o = (0,0,-1) the 3x3 is identity (433 of 818 records have o = (0,0,0)). For a
  yaw-only o the transposed 3x3 is the inverse of the rotation ComputeTransform would build from the
  same vector. Reproduce it exactly as is.
  Newton step used by every normalize in this file:
  `y0 = rsqrte(d); y1 = y0 + 0.5*y0*(1 - d*y0*y0); y2 = y1 + 0.5*y1*(1 - d*y1*y1); v*y2`
  (no zero-length guard).
* `mCharacterAnim = new SceneCharacterAnim(owner=&this->UCOM::Object, animDB, animName, applyFace,
  animOffset)` (0x5390 bytes, 0x8287549c).

### 2.3 SceneCharacterAnim ctor 0x82874828, Init 0x82873088, InitFace 0x82873918

* ctor: `Andale::Animatable(owner, TRANSIENT)`; mAnimOffset = animOffset; mApplyFace; mAnimValid=0;
  mAnimDBName = animDB; mAnimPlayName = mAnimRealPlayName = animName; mSceneCharacter = owner-4;
  bone indices = -1; mHipsPos = 0; then `Init()` (0x828749a4).
* `Init` 0x82873088:
  1. `data = GetAnimLoaderData(mAnimDBName)`; `mAnimValid = data && kNISCharacterAnimsOn`
     (cVarBoolRef 0x83058b64, default true, registered 0x82f723b0). When not valid nothing else
     happens and SetAnimToTime / EvalCurrAnim / InitFace / ResetAnim are no-ops; InstanceCharacter
     then rejects the character because `IsValid()` is false.
  2. `pd = PlaybackDataManager::GetPlaybackData(mAnimPlayName)` (0x82873174). If it already exists
     (same anim name registered by another character) it is reused with its ACS
     (`AnimCommandSystemManager::GetACS(mAnimPlayName)`, asserts non-null). Otherwise
     (0x82873194..0x8287335c): `db = new Andale::DataBase(AnimDataBaseManager)`;
     `db->SetDBContent(data, E_DONT_DELETE)`; hierarchy = 16-byte aligned pointer inside the DB;
     `ACS = AnimCommandSystemManager::NewACS(dbMgr, mAnimPlayName, hir, &fetchSys, ...)`;
     `pd = new PlaybackData(mPlaybackDataManager, mAnimPlayName)`; pd->mACS/mFetchSys/mHir set;
     db pushed on pd->mDBList.
     `NewACS` 0x824d62c0 parameters (verified while porting): MaxTotalSQT 13, mode 0, fetch mode 1,
     category 1, truncation 0.5, `hasTrajectoryNode` false, codec 0 (RAW); it builds
     `Skeleton(nbBones, trajNode = 0, parents)`. With `hasTrajectoryNode` false,
     `FetchSys::BatchFetch` 0x82d737d0 takes no trajectory delta (that needs `mHasTrajectoryBone`
     and a part starting at bone 0), so the trajectory bone is fetched absolute: the clip's root
     motion in world coordinates, with the hips near zero.
  3. Twist-bone lookups (LeftArm, LeftArmTwist, LeftForeArm, LeftForeArmTwist, LeftForeArmTwist1,
     LeftUpLeg, LeftUpLegTwist and the Right* equivalents) 0x828733e4..0x82873664.
  4. `mBPSQTBuffer.Init(hir)`, `mTwistSQTBuffer.Init(hir)`.
  5. `mTrajBoneIndex = bone("Trajectory")`, else bone("Vehicle_Root") (assert if -1);
     `mHipsBoneIndex = bone("Hips")`, else bone("Chassis") (assert if -1);
     `mSkateboardBoneIndex = bone("Skateboard_Root")` (may stay -1). 0x82873690..0x8287379c.
  6. Tree pool `mTree` (1000 nodes): `mSetSQT = Push<SetSQT>()` with
     `mSetSQT->mSqts = mBPSQTBuffer.mPoseBuffer`; `mAddSQT = Push<AddSQT>()` (0x828737c8..0x82873804).
  7. Evaluates the hierarchy's bind pose (`Andale::Pose` node, `pd->GetPose(...)`) through the ACS
     into `mBPSQTBuffer` (0x82873844..0x828738a8) -> mBPSQTBuffer = bind-pose local SQTs.
  8. `mAnimatable.Init(attrPool, pd, bindPose, 1024, 2, 1, 32)` (0x82873904).
* `InitFace` 0x82873918 (only if mAnimValid && mApplyFace; called by InstanceCharacter on success):
  face pose source = PlaybackData "SkaterFace" (`Skater::Names::szSkaterFacePlaybackData`, set at
  0x82f7f698). Pose name = the SceneCharacter's `mSchemaName`; if missing and the render recipe is
  CreateACharacter (`IsCaC` 0x82866108): male -> "CAC_male"; female -> "CAC_female" **and**
  `mAnimRealPlayName`'s "_Player"/"_player" is replaced by "_Female" (0x828739d8..0x82873a10);
  if still missing (or not CaC) -> "rig_Tpose". The face pose is evaluated into a temp SQT buffer
  and `mBPSQTBuffer.OverrideWith(temp)` (0x82873b00; `OverrideWith` 0x825920b0 copies whole parts
  matched by part name) - so the face bones become part of the base pose.
  applyFace is true only for "current_player" and for dynamic+override characters (section 1.4).

### 2.4 SetSceneTransform 0x82862158 / GetSceneTransform 0x82862120

```
SetSceneTransform(T):  mTransform = mUserTransform * T     // row-major 4x4 product
   row i of result = U[i][0]*T.row0 + U[i][1]*T.row1 + U[i][2]*T.row2 + U[i][3]*T.row3
   (vmaddfp chain 0x82862178..0x82862220, stores to mTransform 0x30)
GetSceneTransform():   returns mTransform (+0x30)
```
Called once per entity in `Scene::Init` with the scene matrix (0x82877e94). RW uses row vectors
(v' = v*M, translation in row 3), so a character point goes through mUserTransform first, then the
scene transform.

### 2.5 Time: SetTime / AddTime and the Andale clip semantics

```
SceneCharacter::SetTime(t)  0x82862080: anim->SetAnimToTime(t); anim->EvalCurrAnim()
SceneCharacter::AddTime(dt) 0x828620d0: anim->AddTime(dt);      anim->EvalCurrAnim()
SceneCharacterAnim::SetAnimToTime(t) 0x82873c80:
   if mAnimValid: mAnimatable.PlayAnim(FastString36(mAnimRealPlayName), speed = 1.0, time = mAnimOffset + t)
SceneCharacterAnim::ResetAnim() 0x82873c00: same with time = mAnimOffset
SceneCharacterAnim::AddTime(dt) 0x82862290: mAnimatable.AddTime(dt)
```
* `Animatable::PlayAnim(name, playBackSpeed, timeOffset)` 0x82d7a7a0: clears the transition and
  tree stores, builds a **new** tree for `name` from the PlaybackData (`GetAnimTree` 0x82585340 ->
  `PlaybackData::GetAnimTree` 0x82d7e468), `SetSpeed(speed)`, and **only if `timeOffset > 0.0`**
  calls `SetTime(timeOffset)` (0x82d7a8a4); then `SetBaseAnim`. Scene time + offset <= 0 leaves
  the clip at its start.
* `Animatable::AddTime(dt)` 0x82592b58: `UpdateTransitions`, then `GetCurrAnim()->AddTime(dt)`.
* For a plain `Andale::Clip` node (the expected node for these anims; the actual node type comes
  from the .abin data - see open questions):
  * `Clip::Init` 0x82807260: mNbFrames = hdr+0x24, mFps = hdr+0x20, mBaseSpeed = hdr+0x28,
    **mLoopingAnim = (u32 at hdr+0x34 & 0x10000000) != 0** (hdr = 16-aligned clip header after the
    DataHeader), mLength = FrameToTime(mNbFrames-1).
  * `Clip::SetTime(t)` 0x828071f0: mTime = mOldTime = t, mNumLoops = 0 (asserts t >= 0).
  * `Clip::AddTime(dt)` 0x82d6d238: mTime += dt; if mTime > length: looping -> subtract length
    repeatedly (mNumLoops++); non-looping -> mTime keeps growing (only an "expired" flag is set).
  * `Clip::GetTime` 0x828070c0: `min(mTime, GetLength())` (fsel).
  * `Clip::Eval` 0x82d6ef78: fetch range = [clamp(mOldTime*s, 0, (nbFrames-1)/fps),
    clamp(GetTime()*s, 0, (nbFrames-1)/fps)] with s = mBaseSpeed*mSpeed; if mNumLoops == 0 and
    start > end then start = end; `ACS::Fetch(clipGUID, start, end)`.
  * `Clip::FrameToTime(f)` 0x82805ed0 = `f / (mBaseSpeed * mFps * mSpeed)`; `GetLength` 0x829d5378
    returns mLength (+0x1C). So mTime / mLength are in playback seconds and the sampled data time is
    `mTime * mBaseSpeed * mSpeed`.
  * => **non-looping clips clamp and hold their last frame**; looping clips wrap. With the speed 1.0
    used by PlayAnim:
    `clipTime = mAnimOffset + (scene time at last SetTime) + sum(dt since then)` (playback s),
    `sampleTime = clamp(clipTime * mBaseSpeed, 0, (nbFrames-1)/fps)` (data s);
    the last frame is reached at scene time `(nbFrames-1)/(fps*mBaseSpeed) - mAnimOffset`.
* `GetAnimationLength` 0x82862230 -> `SceneCharacterAnim::GetLength` 0x828622b8 = current anim's
  GetLength (vtbl+0x14) or -1.0. The scene does **not** end when a character clip ends; the end is
  driven by the camera track (`Scene::Update` 0x82870d04).
* `SwapAnimation(animDB, animName, applyFace)` 0x828754d0 (reached from `ScenePlayer::SwapAnimation`
  0x8286d0e0): deletes the SceneCharacterAnim, creates a new one with **animOffset 0.0**, then
  `InitFace()`. It does not call SetTime.

### 2.6 Per-frame pose: `SceneCharacterAnim::EvalCurrAnim` 0x82866cd0

Runs after every SetTime/AddTime (only if mAnimValid). Steps (disassembly-verified):

1. **Local pose** (0x82866d3c..0x82866dec): eval params {ACS, FetchSys, UpdateOldTime=1,
   BlendFrames=2, cull 0.0}. `cur = mAnimatable.GetCurrAnim()`; if cur:
   `mAddSQT->mOperandA = cur; mOperandB = mSetSQT; mMotionOperand = cur; mAddSQT->Eval()`;
   then (unconditionally, 0x82866dd4) `ExtractSQT -> mTwistSQTBuffer`. `AddSQT::Eval` 0x82588308 (A == Motion) evaluates A (clip),
   then B (SetSQT = bind/face pose), then `ACS::AddSQT(ChannelFromFirstOperand=1, LeftToRight=1)`.
   Immediate-mode math of `AnimCommandSystem::AddSQT` 0x82a2d5a0 (LeftToRight branch
   0x82a2db60..0x82a2dc58), per bone, A = clip SQT, B = bind SQT:
   ```
   out.scale     = A.scale * B.scale                      (component-wise)
   out.trans.xyz = rotate(B.quat, A.trans) + B.trans      rotate(q,v) = v + 2*q.xyz x (q.xyz x v + q.w*v)
   out.trans.w   = A.trans.w                              (ChannelFromFirstOperand)
   out.quat      = B.quat (x) A.quat                       Hamilton: w = wB*wA - dot(B,A),
                                                           xyz = wA*B.xyz + wB*A.xyz + B.xyz x A.xyz
   ```
   i.e. **scene clips are applied on top of the (face-overridden) bind pose**: final local =
   bind o clip.
2. **Hips separated from the skeleton** (0x82866df0..0x82866ec8): with
   h = mTwistSQTBuffer[mHipsBoneIndex], q = h.mQuat, t = h.mTrans: then h.mScale = (1,1,1,1),
   h.mQuat = (0,0,0,1), h.mTrans = (0,0,0,1). The removed transform becomes matrix H (row-vector
   form; computed with q' = sqrt(2)*q via gSqrt2s 0x8227e8e0):
   ```
   H.row0 = (1-2(y^2+z^2), 2(xy+wz),     2(xz-wy),     *)
   H.row1 = (2(xy-wz),     1-2(x^2+z^2), 2(yz+wx),     *)
   H.row2 = (2(xz+wy),     2(yz-wx),     1-2(x^2+y^2), *)
   H.row3 = t                                            (* = junk lanes, see step 6)
   ```
3. If `mSkateboardBoneIndex != -1` (skater rigs only): `ComputeExpressions` 0x82866760 -> 8x
   `TwistThatBone` 0x82866258 (L/R Arm->ArmTwist, ForeArm->ForeArmTwist, ForeArm->ForeArmTwist1,
   UpLeg->UpLegTwist) with quats from mBPSQTBuffer and mTwistSQTBuffer and weights from the
   `animation_twistbones` attribute collection (attribute keys not extracted).
4. **Skinning palette** (0x82866ed8..0x82866f50): `ACS.SetSQT(mTwistSQTBuffer)`,
   `SetMatrix(SceneCharacter::GetInvBindPoseBuffer())` (render entity's `InvBindPoseBuffer` @0x214,
   0x82866068), `PoseWithMatMult()`, `ExtractMatrix(GetPoseBuffer()->mPoseBuffer)` (the Animatable's
   Matrix44Affine pose buffer). `UpdateCloth` 0x82865e90 is empty in this build (asserts only).
5. **Root motion / object matrix** (0x82866f5c..0x82867160):
   ```
   Tr = pose[mTrajBoneIndex] with rows0..2.w = 0 and row3.w = 1     // vrlimi 0x82866f90..0x82866fa4
   S  = SceneCharacter::GetSceneTransform()     (= mUserTransform * sceneTransform)
   R  = Tr * S
   Obj = Hm * R          Hm = H with rows0..2.w = 0 and row3.w = 1
   mHipsPos = (t.x, t.y, t.z, t.w_original) * R                     // stored @0x5340 (0x8286713c)
   SceneCharacter::SetObjectTransform(Obj) -> renderEntity->SetLocalToWorldMatrix(Obj)  (0x8286605c)
   ```
   So the trajectory bone's animated transform (root motion) and the hips' local transform both go
   into the render entity's local-to-world matrix; the skeleton itself is posed with the hips at
   identity.
6. Finally `pose[mTrajBoneIndex] = {H.row0, H.row1, H.row2, t}` **including the junk lanes**
   (row0.w = M22, row1.w = M10, row2.w = M01, t.w original) (0x82867164..0x8286717c): the
   trajectory bone's skinning-matrix slot is overwritten with the hips' local matrix. (Traced while
   porting: `SkaterPresEntity::StartJobs` 0x827dbb90 copies bone-buffer entry 0, the trajectory
   slot, into `mHipsTransform`, so the entity's hips matrix is H, as with the gameplay skater.)

`GetHipsPosition` 0x828622a8 returns mHipsPos; the render entity reads it through the `GetHipsPos`
callback (2.7).

### 2.7 How a scene character is drawn (SceneCharacterManager)

* `SceneCharacterManager` (0x2C0 bytes): RenderPresentation @0, ISkaterPresentationEntityCallbacks
  @0x18, `mSceneRenderMode` @0x1C, `mSceneCharacters[32]` @0x20, `mSceneRenderModels[32]`
  (intrusive_ptr<SkaterPresEntity>) @0xA0, `mRenderModelOwnerships[32]` @0x120,
  `mRenderModelSavedEntityTypes[32]` @0x140, `mRenderModelSavedEntityIndexes[32]` @0x1C0,
  `mSkaterPresCallbackHandlers[32]` @0x240. RenderPresentation vtable 0x82140edc:
  dtor 0x82870ef8, Init 0x8286d3e0 (clears all 32 slots), Purge 0x82870f68
  (RemoveCharacterRenderable for all 32, clears mSceneCharacters).
* Callback vtable 0x82140ecc: `GetHipsPos(idx)` 0x82865d10 =
  `mSceneCharacters[idx]->mCharacterAnim->GetHipsPosition()`; `GetSkaterBoneBuffer(idx)` 0x82865d60 =
  `mSceneCharacters[idx]->mCharacterAnim->GetPoseBuffer()->mPoseBuffer`; `RemoveSkater(pe)` 0x82865da0
  (removes from views). The skater render entity pulls its bones through its
  `mPresentationCallbackHandler` (the gameplay handler is `SkaterPresentation::GetSkaterBoneBuffer`
  0x827e3fa8), so the installed handler decides which pose is drawn.
* **New render entity** - `CreateCharacterRenderable(idx, hir, schemaAssetID, clothDisabled)`
  0x82871150: `recipe = CharacterAssetManager::GetRecipe(id)`; fail if no recipe / no LODs / no
  parts. LivingWorld or Vehicle recipe -> `new ColorizedSkaterPresEntity(eEntTypNISChar, idx,
  callbacks=this+0x18, hir, recipe, clothDisabled)`; otherwise `new SkaterPresEntity(...)` +
  `SetTechnique("nis")`. `mRenderModelOwnerships[idx] = 1`; `AddToView(pe)`.
  (`hir` = the scene anim's PlaybackData hierarchy, `playbackData+0x34`.)
* **Borrowed gameplay entity** - `ControlCharacterRenderable(idx, pe)` 0x82870fb0 (for
  "current_player" and dynamic+override): `pe->WaitForJobsToFinish()`; mSceneRenderModels[idx] = pe;
  `SetTechnique("nis")`; save `pe->mPresentationCallbackHandler` (+0x1E4), `pe->mIndex` (+0x1E0),
  `pe->m_EntityType` (+8) in the saved arrays; ownership = 0; then
  `pe->mPresentationCallbackHandler = this+0x18`, `pe->mIndex = idx`, `pe->m_EntityType = 0x822`
  (eEntTypNISChar), `pe->mIsGhosted (+0x61A) = 0`, `pe->i_params[0].x (+0x870) = 1.0`;
  `AddToView` only in TRICKBOOK render mode (in normal mode the entity is already in the views).
* `AddToView` 0x82861fa0 / `RemoveFromView` 0x82862010: render mode 0 -> `eRenderViewAll`;
  mode 1 (SCENE_RENDER_MODE_TRICKBOOK) -> `eShadowView` + `eLooseWorldView`.
* `RemoveCharacterRenderable(idx)` 0x8286d478 (from ~SceneCharacter and
  SceneCharacterManager::Purge): `SetTechnique("default")`; owned -> RemoveFromView + release;
  borrowed -> (TRICKBOOK only: RemoveFromView) restore m_EntityType, mIndex and
  `SetPresentationCallbacksHandler(saved)` 0x828603d8, release. TRICKBOOK also clears
  `RenderMan::GetRenderView(eLooseWorldView)`. mIsGhosted / i_params.x are **not** restored here.


---

## 5. Scene transform and scene time (Scene::ComputeTransform / Init / BindToGame / AddTime / SetTimeElapsed)

(Written before sections 3 and 4 so it survives an interruption; sections 3 and 4 follow below.)

### 5.1 `Scene::ComputeTransform(Matrix44& M, location, orientation, normal)` 0x828652f0 (static)

Decoded from the VMX128 code (0x828652f4..0x82865484):
```
Z = normalize(0 - orientation)          // vsubfp 0x82865358 (zero vector at sp+0x10 minus orientation)
X = normalize(cross(normal, Z))         // 0x828653c4..0x8286540c
Y = cross(Z, X)                         // 0x82865424..0x82865430
M.row0 = (X.x, X.y, X.z, 0)             // stvx 0x8286547c  (+0x00)
M.row1 = (Y.x, Y.y, Y.z, 0)             // stvx 0x82865484  (+0x10)
M.row2 = (Z.x, Z.y, Z.z, 0)             // stvx 0x82865478  (+0x20)
M.row3 = (location.x, location.y, location.z, 1)   // stvx 0x82865480 (+0x30)
```
* normalize = rsqrte + two Newton-Raphson steps (as in 2.2); there is **no** zero-vector guard here
  (unlike the character constructor) - a zero orientation or an orientation parallel to normal gives
  NaNs.
* Row-vector convention: local -Z maps to +orientation (the scene's forward), local +Y maps to the
  component of `normal` orthogonal to forward (Y = Z x X), local +X = normalize(normal x Z).
* Scene directory data: 520 of 561 scenes have mOrientationXYZ = (0,0,-1) (identity rotation when
  normal = (0,1,0)).
* Callers: `Scene::Init` 0x82877c58 (`ComputeTransform(&mTransform @0x60, &mLocation @0x30,
  &mOrientation @0x40, &mNormal @0x50)`), and for the ICE camera's time-zero matrix
  `ComputeTransform(&tmp, out1, out3, sceneNormal)` 0x82877fe0 with out1/out3 from
  `SceneCamera::GetCameraComponentsAtTimeZero` (meaning of those outputs not traced) ->
  `CameraPresentation::SetICEMatrix` 0x828780b8.

### 5.2 Scene::Init 0x82877ac8 - order of operations relevant to characters

```
mInstanceID = id; ClearFreezeTimeControl()
props = dir->GetSceneProperties(mSceneHash); mSceneType = props.mType
mLocation/mOrientation/mNormal = args; ComputeTransform(&mTransform, ...)          // 0x82877c58
mTimeElapsed = 0; mTimeDelta = 0; mTimeLength = props.mLength
mIsControllingReplay = (props.mIsReplayOn != 0) ...                              // 0x82877c84
overrides = ScenePlayer->GetPlaybackOverrides()
dir->AddEntities(mSceneHash, this, overrides, characterLocations)                // 0x82877e6c
for e in mSceneEntities: e->SetSceneTransform(&mTransform)                       // 0x82877e94
SetTimeElapsed(0.0)          // every entity SetTime(0) -> clip at mAnimOffset, pose evaluated  0x82877eb0
camera / streamer / postfx (RenderMan.m_isNIS = 1) / audio / world render / subtitler setup
if mIsControllingWorldRender: WorldPresEntity::DisableWorldPresEntityRender()    // 0x82878378
```
`ScenePlayer::Create` 0x82878590 builds the Scene with defaults mIsControllingSimulation=1,
FrontEnd=1, Replay=0, Streamer=1, Camera=1, PostFX=1, Audio=1, **WorldRender=0**, Subtitler=1
(0x82878690..0x828786b0), then `CreateInstance` and `Scene::Init`.

### 5.3 Scene::Update 0x82870bb8 and Scene::AddTime 0x82865700

```
Scene::Update(dt):
  if mPlayState is STOPPED or PAUSED: return
  if mEditorWaitForQueuing: QueuingCheck(); return
  mTimeDelta = ScenePlayer->GetDeltaTimeMult() * dt                              // 0x82870c98
  AddTime(mTimeDelta)                                                            // 0x82870ca4
  subtitler->Update(mTimeDelta * 100000)
  if GetDeltaTimeMult() > 0 && (!mCameraTrackFound || mCameraFinished):           // 0x82870d04
      PHOTO_OUTRO: (not editor) SetFreezeTimeMult(0) + send cMsgTerminateStateMachine
      "ViewScene":  mCameraFinished = 0; ICE active scene SetTime(0)
      else:         Stop()
Scene::AddTime(dt) 0x82865700:
  mTimeElapsed += dt
  for e in mSceneEntities: e->AddTime(dt)        // SceneCharacter::AddTime -> clip AddTime + EvalCurrAnim
```
* `ScenePlayer::GetDeltaTimeMult` 0x82861db8 = `mOverrideTimeMult * mStreamTimeMult * mFreezeTimeMult *
  mSceneTimeMult` (mOverrideTimeMult = the `speedMod` passed to `ScenePlayer::Create`, 0x828785b8).
  `ScenePlayer::Update(dt)` 0x82872558 counts down `mFreezeCount` (then `SetFreezeTimeMult(1.0)`) and
  calls `Scene::Update(dt)` for every scene instance. So every character clip advances by
  `dt * GetDeltaTimeMult()` per frame, the same value added to `Scene::mTimeElapsed`.

### 5.4 Scene::SetTimeElapsed 0x82865690 (the "SetTime" used by ICE camera cuts)

```
Scene::SetTimeElapsed(t):
  mTimeElapsed = t
  for e in mSceneEntities: e->SetTime(mTimeElapsed)
```
For a SceneCharacter `SetTime(t)` (2.5) = `PlayAnim(mAnimRealPlayName, 1.0, mAnimOffset + t)`
(tree rebuilt; clip time set only if > 0) followed by `EvalCurrAnim()` - i.e. an absolute jump of
every character's clip to `mAnimOffset + t`, pose recomputed immediately (no blending).

Call chain verified: `ICESceneBridge::SetTime(t)` 0x826cbd48 ->
`SceneManager::Player()->SetTimeElapsed(mSceneHandle, t)` (IScenePlayer vtbl slot 0x50, vtable
0x82140df0) -> `ScenePlayer::SetTimeElapsed(handle, t)` 0x8286c950 (handle -> Scene* lookup in
mInstanceVector) -> `Scene::SetTimeElapsed` 0x8286c9cc. (Lite variant: `ICELITESceneBridge::SetTime`
0x826d7790 -> `SceneLitePlayer::SetTimeElapsed` 0x8287b958.) The ICE side calls the active scene's
`SetTime` from `ICELITEAuthor::SetSceneTime` 0x82a5dd50 (verified: `TheICELITESceneManager->
GetActiveScene()->SetTime(t)` when the context is scene-based and mState != 5) and
`ICEAuthor::SetSceneTime` 0x82a3fcd8 (not decompiled, assumed equivalent); their callers include
`AdvanceAssemblyAnimation` (0x82a406d0 / 0x82a5e708), `ICEController::Update` 0x82a4d778,
`SetState`, `SetSceneToStart/End`. That the active ICE scene object is the Sk8 bridge is inferred
from the bridge being the only Sk8 `SetTime` implementation. The other caller of
Scene::SetTimeElapsed is Scene::Init (t = 0).

### 5.5 Scene::BindToGame 0x82865c18 / UnBindToGame 0x8286d278 / Pause / UnPause / Stop / Purge

* `UnPause` 0x82870a18: STOPPED -> `BindToGame()` then PLAYING; PAUSED -> PLAYING then `BindToGame()`.
* `BindToGame` 0x82865c18: `ClearFreezeTimeControl()`; editor: if (camera take ? take->fParameter :
  mTimeElapsed) == 0 -> `QueuingStart()`; game: if (camera take exists ? take->fParameter == 0 :
  mTimeElapsed == 0) -> `ICELITEChannelManager::OnTakeStart()` + `TheICEWorldManager->Reset()`.
  **Nothing character-related.**
* `Pause` 0x82870ab8 / `Stop` 0x82870b38 -> `UnBindToGame` 0x8286d278: `ClearFreezeTimeControl`,
  `QueuingKill`, editor-only audio finish and cMsgNISFEEvent. Nothing character-related.
* `Scene::Purge` 0x8286a908 (from `ScenePlayer::Destroy` 0x8287304c): entities in reverse order:
  `e->Purge()` (empty) + `delete e` (-> ~SceneCharacter -> RemoveCharacterRenderable, which hands a
  borrowed gameplay skater entity back to the game, section 2.7); then restores subtitler, world
  render (`EnableWorldPresEntityRender` if it was disabled), audio finish, post-FX/light rig
  (`RenderMan.m_isNIS = 0`), `m_SunLightMan.mManualModeNIS = 0`, streamer controller
  (`CreateGameStreamerController`), camera (CameraControlDisconnect; ICE camera off and
  mNISSequenceCameraMatrix = mICEMatrix unless camera-hold-at-end), replay time controller
  (`Presentation::EnterRealTime(0)` unless camera-hold-at-end).

### Note on IDA's VMX128 operand display (applies to all matrix decodes in this file)

In this IDB the **vA operand of VMX128 instructions is displayed with register bits 5 and 6
swapped** (v64..v95 shown for real v32..v63 and vice versa). Every decode above was cross-checked
against the producing instruction, e.g. in 0x82875338 `vsubfp128 v44, v77, v62` reads v45 (the zero
vector just loaded at 0x82875334) and `vmsum3fp128 v43, v76, v44` reads v44. Also, IDA prints
`vmaddfp/vnmsubfp vD,vA,vB,vC` meaning vD = vA*vC + vB and vD = vB - vA*vC respectively (checked on
the rsqrte Newton-Raphson sequences). PermConst tables read from 0x82fd6f00..: <0,0,0,5>,
<1,4,0,0>, <2,7,0,0>, <2,5,0,0>, <0,6,0,0> as words (A[i], B[j-4]).


---

## 3. NISCharacterResolver (Start 0x828a5458 / Update 0x828a49a0 / ReloadMarquees 0x828a2578)

### 3.1 What "resolving" means

The resolver does **not** map scene character names to models - that is done by the scene loaders
(section 1: `SceneCharacterLoader::Load` -> `CharacterAssetManager::LoadSchemaFromFile(mName,
category)`, and for "current_player"/dynamic the existing gameplay `SkaterPresEntity` is borrowed).
The resolver only enforces a **memory budget of 6 loaded marquee (pro-skater) character models**
before the scene's own assets are loaded: it unloads marquee models that are loaded in the living
world but not needed by the NIS, and remembers challenge AI skaters it unloaded so they can be
reloaded after the NIS.

Object: `NISManager::mNISCharacterResolver` (constructed in `NISManager::NISManager` 0x828a7898;
ctor 0x828a3728 sets mIsCharacterResolverFinished = mIsCharacterReloadingFinished = 1, default
overrides, empty `mMarqueesThatNeedReloading` list of u64 asset keys).

### 3.2 Start(sceneName, overrides) 0x828a5458

```
mSceneName = sceneName; mStatus = WaitingToResolve; mPlaybackOverrides = overrides
mIsCharacterResolverFinished = 0; mMarqueesThatNeedReloading.clear()
if MarqueeLoadingAllComplete(): BeginResolve()
```
Callers: `NISManager::RunBasicNIS` 0x828a8fd0 (normal NIS start), `NISManager::Update` 0x828a6904
(preload finished with a pending play), `NISManager::PreloadNISList` 0x828a69f8.

### 3.3 BeginResolve 0x828a48a8

```
mStatus = Resolving
loaded   = FillListOfLoadedMarquees()                         // 0x828a3eb8
required = FillListOfRequiredMarquees(mSceneName, overrides)  // 0x828a41d8
excess = GetNumMarqueesThatNeedLoading(loaded, required) + loaded.size - 6     // 0x828a4928
if excess <= 0: mStatus = None; mIsCharacterResolverFinished = 1
else:
    unneeded = FillListOfUnnecessaryButLoadedMarquees(loaded, required)        // 0x828a43b8
    UnloadNMarqueesOfLowestPriority(excess, unneeded)                          // 0x828a4650
```
* **FillListOfRequiredMarquees** 0x828a41d8: for each character record i of the scene
  (`GetSceneProperties(const char*)` 0x8286f160, `GetSceneCharacterInfoFromIndex` 0x828618c0):
  - category "dynamic" and i < override count -> the schema name of
    `GetSkaterPresEntityFromPlayerId(playerIndexes[i])->m_Recipe` (skipped if no entity);
  - otherwise only category **"marquee"** records are added: their mName, except "current_player"
    which is replaced by the local player's recipe schema name
    (`GetSkaterPresEntityFromIndex(GetLocalIndex())->m_Recipe->mSchemaName`, 0x828a4360).
  "livingworld"/"vehicle" records never count against the budget.
* **FillListOfLoadedMarquees** 0x828a3eb8: for every key in `Character::MarqueeDefs::GetMarqueeList()`
  (0x82cfdc00), the marquee's Attrib collection gives its name (first field of the layout). It is
  "loaded" if `RecipeMan::IsLivingWorldModelLoaded(name)` (0x8270f590) or
  `SkaterAssetManager::IsRecipeUsedByPlayer(key)` (0x824b3500) or
  `AISkaterManager::IsSkaterAssetInUse(key)` (0x824b9070) != 0. Each entry = {key, priority, name}:
  | priority | condition |
  |---|---|
  | 1 | living-world model loaded (RecipeMan) |
  | 5 | recipe used by the player |
  | 4 | AI asset LoadedAsChallenge, or LoadedButUnused and listed in the challenge AI manager's `GetChallengeCharacters` |
  | 2 | AI asset LoadedButUnused, not a challenge character |
  | 3 | AI asset LoadedAsAmbient (any other usage value -> 2) |
* **GetNumMarqueesThatNeedLoading** 0x828a44b8: number of distinct required names that are not
  "current_player" and not already in the loaded list (name compare, `StringCompare`).
* **FillListOfUnnecessaryButLoadedMarquees** 0x828a43b8: loaded entries whose name is not in the
  required list.
* **UnloadNMarqueesOfLowestPriority(n, unneeded)** 0x828a4650 - walks `unneeded` once per class, in
  this order, decrementing n per unload and stopping at 0:
  1. priority 2 -> `AISkaterManager::UnspawnAndUnloadSkater(key)` 0x824ce050
  2. priority 1 -> `RecipeMan::UnloadLivingWorldModel(name)` 0x82715278
  3. priority 3 -> `AISkaterManager::UnspawnAndUnloadSkater(key)`
  4. priority 4 -> key appended to `mMarqueesThatNeedReloading`, then
     `SkaterAssetManager::CharacterLoaderUnload(key)` 0x824b3628
  5. priority 5 -> `Character::PlayerLoader::Unload(Simulation.mWorkspace.mLocalPlayerLoader)` 0x828182e0
  Only class-4 (challenge) skaters are remembered for reloading; ambient/unused AI skaters and
  living-world marquee models are simply unloaded (their owners reload them later by their own
  logic - not traced).

### 3.4 Update 0x828a49a0 (polled every frame by NISManager::Update 0x828a67e8)

```
if !mIsCharacterResolverFinished:
    WaitingToResolve: if MarqueeLoadingAllComplete(): BeginResolve()
    Resolving:        if MarqueeLoadingAllComplete(): mIsCharacterResolverFinished = 1; mStatus = None
if !mIsCharacterReloadingFinished:
    if every key in mMarqueesThatNeedReloading has SkaterAssetManager::CharacterLoaderIsLoaded(key) (0x824bb660):
        mIsCharacterReloadingFinished = 1; clear the list
```
`MarqueeLoadingAllComplete` 0x828a2610 is true only when **all** of:
every key in mMarqueesThatNeedReloading has `CharacterLoaderIsUnloaded(key)` (0x824bb6c0);
the local `PlayerLoader` is either `IsLoaded` (0x828141c8) or `IsUnloaded` (0x82815ca0) (not mid-transfer);
`RecipeMan::NumActiveLoadRequests == 0` and `NumActiveUnloadRequests == 0` (atomic reads 0x828a2748..0x828a278c);
`!AISkaterManager::AreAnyAssetsLoadingOrUnloading()` (0x824c3260).

So resolving takes at least one call (Start) and, when unloads are needed, as many frames as the
asset managers need to finish all pending load/unload requests; there is **no timeout** in this
code - the NIS start waits indefinitely on these conditions.

### 3.5 Who waits on it (NISManager)

* `RunBasicNIS` 0x828a8d70 (when not in STATE_Loading and not already in a NIS):
  TeleportToLastBreadCrumb, HUD NIS-disable + NIS load-screen transition (non-FE / non-VIEW_MODE),
  mIsInNIS = 1, store name/overrides/flags, **`PauseSim()`** (0x828a8fa0), then
  `Start(resolver)`; finished -> `StartPendingNIS` (0x828a8fe4) else
  mResolvingStatus = ForPlay (or ForPreloadWithPendingPlay if a preload is resolving).
* `NISManager::Update` 0x828a67e8: ForPreload -> on finish StartPendingPreload; ForPreloadWithPendingPlay
  -> on finish re-Start for the play and StartPendingNIS / ForPlay; ForPlay -> on finish
  `StartPendingNIS` 0x828a2d30; Reloading -> when `mIsCharacterReloadingFinished` -> `EndNIS` 0x828a3200.
* `ReloadMarquees` 0x828a2578 is called only from `NISManager::ReceiveNISMgrCompleteMsg` 0x828a3440
  (cMsgNISMgrComplete, i.e. after the scene was stopped/destroyed/unloaded):
  ```
  mIsCharacterReloadingFinished = 0
  if list empty: mIsCharacterReloadingFinished = 1
  else: for key in mMarqueesThatNeedReloading: SkaterAssetManager::CharacterLoaderLoad(key)  (0x824b3678)
  ```
  then `EndNIS` immediately if finished, else mResolvingStatus = Reloading and NISManager::Update
  polls `Update()` until every reloaded challenge skater reports loaded, then `EndNIS`.


---

## 4. Local skater, AI skaters, pedestrians and traffic during a NIS

Three independent mechanisms; none of them despawns the gameplay entities (except the marquee
budget unloads of section 3).

### 4.1 Frozen: the simulation is paused (PauseMan state NIS)

* `NISManager::RunBasicNIS` 0x828a8d70 calls `NISManager::PauseSim` 0x828a1070 (0x828a8fa0)
  **before** the character resolver / scene load:
  `PauseMan::Push(kPAUSE_STATE_NIS = 11)` (`li r4,0xB` 0x828a10e8), `Net::PauseGame()`, and in
  online games `cGameAssetManager::LockStreamFocus(1, 7)`.
* `PauseMan::Push` 0x827381c8 sets `mFlags = kFlagsTable[top state]`; table at 0x820d8548,
  `kFlagsTable[11] = 0xFFFFFFF1` (flags 0x2/0x4/0x8 clear, everything else set). `IsPaused(flag)`
  0x824e4778 = `(mMask & mFlags & flag) != 0`, mMask = -1 (ctor 0x82738d68).
* Effects relevant here:
  * `kPAUSE_FLAG_SIM` (1): `GameModule::DoGameUpdate` 0x82730418 tests it (0x827305ac) and does not
    generate the next sim sync command while paused -> the whole simulation (local skater physics,
    AI skaters, living-world pedestrians and traffic) stops ticking.
  * `kPAUSE_FLAG_DATASYSRESTORE` (0x10): `GameModule::Render` 0x827320f0 skips
    `DataSysMan::StepTime` + `RestoreFrame` (0x82732374..0x827323ac) -> presentation entities are no
    longer refreshed from simulation state, they keep their last pose/position.
  * `SceneManagerImpl::UpdateLoadState` state 1 0x82875c64 **waits** until `IsPaused(SIM)` is true
    (offline) before starting to load the scene.
* Replay NIS (`RunReplayNIS` 0x828a3c70, not the basic path) additionally pushes state 7
  (`kFlagsTable[7] = 0xFFFFFFE1`, 0x10 clear) so the data-system restore runs with the
  `SceneReplayTimeController` installed by `Scene::Init` 0x82877e40; that is not used by RunBasicNIS.
* `TeleportToLastBreadCrumb` 0x828a0db8 (start of RunBasicNIS/RunReplayNIS): for STORY and GOTO
  scenes, unless `mIgnoreBreadcrumbTeleportOnNextNIS`, teleports the local player to the last
  breadcrumb if that does not need a load screen (`ITeleportManager::TeleportLocalToLastBreadcrumb`).

### 4.2 Hidden: render-type visibility mask while a NIS scene exists

* `RenderView::DetermineVisibilityFromGameState` 0x82829698 (used by `SceneRenderView::Cull`
  0x828501e4, `ShadowView::Cull` 0x82850bb8, `WorldShadowView::Cull` 0x82850f78 and
  `PostFX::CalculateDeckShadowMatrix` 0x82846018): an entity is drawn only if
  `(RenderMan.mVisibleTypes & entity.m_EntityType) == entity.m_EntityType` (0x828296e0..f4) and
  `entity->IsEnabled()`.
* `RenderMan::UpdateVisibleTypes` 0x82839be0 (called from `RenderMan::UpdateRenderParams`
  0x82852858, i.e. every frame) sets `mVisibleTypes` (RenderMan+0x32C):
  | situation | mVisibleTypes |
  |---|---|
  | `IsInNIS()` false (`SceneManagerImpl::IsInNIS` 0x82860a28 = `mNISHandle != 0`, or ICE editor with an active scene) | -1 (all); trickbook / Create-a-Spot special cases |
  | flythrough NIS (`IsInFlythroughNIS` 0x82860ab8: scene type FLYTHROUGH(2) or PHOTO_INTRO(5)) | 0x286F |
  | "character NIS" (`IsInCharacterNIS` 0x82860ee8 -> `SceneBinDirectory::IsSceneWithCharacters` 0x8286ece0: `mIsReplayOn == 0` and mType not in {0,2,5,6,13}; its view-scene-handle branch is unreachable here because the mask code only asks while `IsInNIS()`) | 0x282F, plus 0x42 if online, or the scene's `mBitField & 4` (DMOs enabled, `IsSceneWithDMOsEnabled` 0x8286edc0 via `IsDMOsEnabled` 0x828612a0), or cvar `kRenderDynamicObjectInCharacterNISs` (default 0, 0x82f702a0) |
  | any other NIS (replay NISes, PHOTO_OUTRO, SPOT_OUTRO, type NONE) | -1 |
* Entity type constants (from the constructors):
  | entity | m_EntityType | in 0x282F? |
  |---|---|---|
  | gameplay skaters, local and AI (`SkaterPresentation::UpdatePresEntity` 0x827e6308) | 0x122 | **hidden** (0x100) |
  | NIS characters (`CreateCharacterRenderable`, `ControlCharacterRenderable` 0x82871100) | 0x822 | visible |
  | pedestrians `cPresPedestrian` 0x828122d4 | 0x222 | **hidden** (0x200) |
  | vehicles/traffic `cPresVehicle` 0x8280fd6c | 0x422 | **hidden** (0x400) |
  | dynamic objects `cPresDynamicObject` 0x8280bb9c, DMOs `DmoPresentationEntity` 0x825e90dc | 0x42 | hidden unless 0x42 added |
  | props `cPresProp` 0x8280bb40 | 0x1 | visible |
  | world `WorldPresEntity` 0x827d3ea0 | 0x9 | visible |
  This is why `ControlCharacterRenderable` retypes a borrowed gameplay skater to 0x822: otherwise
  "current_player" would be culled in a character NIS.
* Photo-challenge NIS (`IsInPhotoChallengeNIS` 0x82861158) overrides the mask: everything visible,
  except type-0x222 entities whose Attrib collection is 0x88D72DF4_43D56751 or 0x4E6E5331_D54E4F61,
  and type-0x122 skaters that are not the player's recipe when
  `SceneManagerImpl.mHideAISkatersForPhotoOutro` (set in UpdateLoadState state 7 0x828764b0 for
  PHOTO_OUTRO scenes with fixed-frame photos) (0x82829718..0x828297e0).
* Scene data census (dump_scene.py): 283 scenes are "character NIS" (190 of them have no character
  records - the player is still hidden), 68 flythrough/photo-intro, 210 use mask -1.
* Also at NIS start: HUD disabled (`FrontEndManager::HudManagerSetNISDisableHUD(1)` 0x828a8f0c for
  non-FRONTEND/VIEW_MODE scenes), visual indicators off for "non-VIS-indicator" scenes
  (`VIPresentation::SetRenderTypes(0)` in `SceneManagerImpl::StartNIS` 0x8286ba18; restored with
  `|= 0xFE000000` in StopDestroyUnloadNIS 0x8286e180). World render is only disabled when
  `Scene::mIsControllingWorldRender` is set, which ScenePlayer::Create initialises to 0.

### 4.3 The local skater's model is re-used (only when the scene uses it)

"current_player" (and dynamic+override players) borrow the gameplay `SkaterPresEntity`
(section 2.7): handler swapped to the SceneCharacterManager (bones and hips come from the scene
anim), technique "nis", type 0x822, mIsGhosted = 0, i_params.x = 1. Its local-to-world matrix is
overwritten every scene frame by `SetObjectTransform` (2.6). The simulation-side skater is
untouched (frozen by 4.1).

### 4.4 When everything comes back (order)

1. Scene end -> `SceneManagerImpl::StopDestroyUnloadNIS` 0x8286dfe0: `ScenePlayer::Stop`,
   `ScenePlayer::Destroy` -> `Scene::Purge` 0x8286a908 -> `~SceneCharacter` ->
   `RemoveCharacterRenderable` 0x8286d478 (borrowed skater gets back its type 0x122, index and
   gameplay callback handler -> gameplay pose again; scene-created entities released), then
   **`mNISHandle = 0`** (0x8286e0e8) -> `IsInNIS()` false -> next `UpdateVisibleTypes` sets the mask
   back to -1 (skaters/peds/traffic visible again, still frozen). Scene loader unload, then
   `SendCompletionMessages` 0x8286e1c0 (cMsgNISMgrComplete).
2. `NISManager::ReceiveNISMgrCompleteMsg` 0x828a3440 -> `ReloadMarquees`; when reload finished
   (immediately or polled, section 3.5) -> `NISManager::EndNIS` 0x828a3200: popup unhide,
   `mIsInNIS = 0`, `FrontEndManager::PopNIS`, cMsgTerminateStateMachine + cMsgNISComplete to the
   challenge system, pull REPLAY pause state if it was added, transition manager End/Begin(exit
   transition), then **`ResumeSim`** 0x828a1170: `PauseMan::Pull(kPAUSE_STATE_NIS)` (0x828a11ec),
   `Net::ResumeGame`, `LockStreamFocus(0, 7 or 5)` -> simulation and data-system restore run again.
   Pedestrians/AI skaters/traffic resume from the exact state they had when the NIS started (sim
   was never advanced), except marquee AI skaters unloaded by the resolver (section 3).


---

## 5 (addendum). Where the ComputeTransform inputs come from: SceneManagerImpl::UpdateLoadState 0x82875bc0

Character-relevant part of the load state machine (verified):

* **State 1** (0x82875c4c..0x82876074); offline it does nothing until `IsPaused(kPAUSE_FLAG_SIM)`
  (the NIS pause of 4.1) is in effect, online it proceeds immediately (0x82875c4c..0x82875c80):
  ```
  if overrides.useLocationValues (+0x1C):                       // 0x82875d3c
      mSceneLocation    = overrides.locationPosition (+0x20)
      mSceneOrientation = overrides.locationDirection (+0x30)
      mSceneNormal      = (0, 1, 0)
  elif overrides.locationName[0] (+0x50):                      // 0x82875ddc
      M = cLocationManager::GetLocationOrientation(name)        (Matrix44Affine)
      mSceneLocation    = cLocationManager::GetLocationPosition(name)
      mSceneOrientation = (0,0,-1) * M  = -M.row2               (3x3 part only, 0x82875eb4..)
      mSceneNormal      = (0, 1, 0) * M =  M.row1
  else:
      mSceneLocation    = props.mLocationXYZ    (w = 0)
      mSceneOrientation = props.mOrientationXYZ (w = 0)
      mSceneNormal      = (0, 1, 0)
  mSnapSceneToGround = props.mBitField & 1                      // 0x82875f94
  anyCharSnap = (!mSnapSceneToGround) ? PreSnapFillLocationList() : false
  if mSnapSceneToGround || anyCharSnap: move stream focus to mSceneLocation, frozen streamer, state 2
  else: SceneLoader::LoadAndCallback(...), state 5
  ```
  (mSceneLocation @+0xF0, mSceneOrientation @+0x100, mSceneNormal @+0x110 of SceneManagerImpl.)
* **State 2**: wait `IsStreamFocusLoaded(3, GameLoad)` (or online); scene snap -> send
  `cMsgNISWorldCollisionQuery(mSceneLocation)` (kind 0x27A08C41) -> state 3; else
  `SnapLocationsToGround()` -> state 4.
* **State 3**: on `mGroundLocationRecieved` (set by `ReceiveNISMgrWorldCollisionResponseMsg`
  0x82861808, section 1.4b: location/normal from the collision, orientation re-projected):
  `PreSnapFillLocationList(); SnapLocationsToGround();` -> state 4.
* **State 4**: when `LocationSnapper.mSnappedLocationsRecieved`:
  `UpdateLocationListWithSnappedLocations()`, `LoadAndCallback`, state 5.
* **State 5**: when loaded + audio ready + FE preloaded + no inbound visual transition:
  `mNISHandle = ScenePlayer::Create(gNISSceneHash, SelectTrack(), SelectSpeed(), &mSceneLocation,
  &mSceneOrientation, &mSceneNormal, &mCharacterLocationList)` (0x828762e4) -> Scene::Init ->
  ComputeTransform / AddEntities / SetSceneTransform / SetTimeElapsed(0). From here
  `IsInNIS()` is true (render mask of section 4.2 applies).
* **State 7**: `Play(mNISHandle)` (0x82876530) -> `Scene::UnPause` -> `BindToGame`, PLAYING;
  from then on `Scene::Update` advances the characters.

So for a non-snapped scene with directory placement, the scene matrix is
`ComputeTransform(mLocationXYZ, mOrientationXYZ, (0,1,0))`.


---

## Unverified / open questions

1. **Anim tree node type.** `PlaybackData::GetAnimTree` 0x82d7e468 (called by PlayAnim) was not
   decoded; the clip semantics in 2.5 assume the scene anim name resolves to a plain `Andale::Clip`.
   Which scene clips set the loop bit (clip header +0x34 & 0x10000000) must be read from the .abin
   files, not from code.
2. **AddSQT / PoseWithMatMult in deferred mode.** Only the immediate-mode branch of
   `AnimCommandSystem::AddSQT` 0x82a2d5a0 was decoded; the command-buffer path is assumed to do the
   same math. `PoseWithMatMult` 0x82a282d8 (local SQT -> model matrices x inverse bind pose) and
   `ExtractMatrix` were not decoded; the exact multiplication order of the skinning matrices is
   unverified.
3. **Trajectory-bone slot overwritten with H** (2.6 step 6) and the use of `GetHipsPos` /
   `mHipsTransform` by `SkaterPresEntity` rendering were not traced, nor how SkaterPresEntity
   combines `GetLocalToWorldMatrix` with the bone buffer. Whether the junk w lanes matter is unknown.
4. **TwistThatBone** 0x82866258 math and the `animation_twistbones` attribute keys/default weights
   (`ComputeExpressions` 0x82866760) were not extracted.
5. **Schema name -> files.** How `CharacterAssetManager` turns a lower-cased schema name into recipe /
   model files (request processing after `LoadSchemaFromFile` 0x82d09720) was not traced, nor the
   effect of `Character::Category` on loading.
6. **Simulation-side snapping.** The handlers of `cMsgNISWorldCollisionQuery` (kind 0x27A08C41) and
   of the location-list snap message (kind 0xFE94DD4B) on the sim thread (ray/collision query,
   fallback when nothing is hit) were not traced.
7. `RemoveCharacterRenderable` does not restore `mIsGhosted` / `i_params[0].x` of a borrowed skater;
   whether gameplay code resets them itself is unverified.
8. PauseMan flags other than `kPAUSE_FLAG_SIM` (1) and `kPAUSE_FLAG_DATASYSRESTORE` (0x10) were not
   identified (NIS state value 0xFFFFFFF1 also sets every bit >= 0x10, e.g. audio/other systems).
9. `ICEAuthor::SetSceneTime` 0x82a3fcd8 was not decompiled (assumed equivalent to the LITE version),
   and the exact times the assembly camera takes jump to are documented elsewhere, not here.
10. The physical meaning of the transposed `mUserTransform` (2.2) cannot be checked from code alone
    (data may have been authored against it); the port must replicate it bit-for-bit.
11. `mHipsPos.w` uses the original hips translation w produced by the ACS (value not verified,
    probably 0 or 1).
12. `GetAnimLoaderData` matches .abin names case-sensitively and without the `_Player -> _Female`
    substitution, while the loaded file may be the `_Female` variant; behaviour for female CaC
    players therefore depends on the stored (unsubstituted) name - consistent, but not tested.
13. (Resolved) `SceneAssetLoaderManager::GetLoadersForScene` 0x82871d78 is find-or-create: an
    existing key returns its vector (0x82871dc4), otherwise a new empty vector is allocated and
    inserted (0x82871de8..0x82871e30).
14. The 190 "character NIS" scenes without character records hide the player model (mask 0x282F);
    this follows from code, not observed in game.
