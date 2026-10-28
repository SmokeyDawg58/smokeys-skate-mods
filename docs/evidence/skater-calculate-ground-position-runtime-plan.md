# CalculateGroundPos and retained HiLOD spatial publication

Read-only source/code audit, 2026-09-08. Confidence **exact** for the raw kernel/caller graph below and **high** for the proposed actual-owner bridge. No Rust edits or Cargo. Companion: skater-jump-spatial-inputs-runtime-plan.md. This does not claim full Update1 or live Actor category/jump-history production.

## Actual kernel and ABI

`Sk8::Physics::Skateboard::CalculateGroundPos(void)` is **82b2f1b0..82b2f3e8**. Its only executable xref found is **82e03560** in `PhysicalPlayerHiLOD::Update1_ProcessInput`; the other xref82403f10 is function metadata. The decompiler prototype reverses hidden return storage and receiver: **r3 is result pointer, r4 is Skateboard this**. At caller3558 r4 receives actual GetSkateboard result;355c r3 points at return temporary. Kernel retains r3 in r29f1f8.

This is not a raycast or contact-point function. It reads no World query, contact flags, contact positions, wheel radius, gravity, or selected surface. It neither calls Part::GetTransform nor composes an inverse part-local transform. Actual body position XYZ from Parts0..3 is the sole physical geometry input.

1. Load SAME board+104 Reckoning pointerf1c8. Capture three full basis columns at Reckoning **+20/+30/+40** throughf204/f208/f210. PDB identifies this as `mGroundTransform` +20, distinct from ordinary transform+60. Translation+50 is not loaded.
2. Merge those XYZ columns with literal zero to transpose the basis. Let columns R,U,A; transpose columns are `[R.x,U.x,A.x,+0]`, `[R.y,U.y,A.y,+0]`, `[R.z,U.z,A.z,+0]`. Original basis W does not enter this transpose.
3. Publish **full +zero** to result atf238, BEFORE calling actual board virtual+4f240 and following its returned Body/Assembly +18f244. This getter failure must retain the zero result. Basis capture failures precede this zero publication.
4. Read actual Parts vector base through assembly+8; body pointers at offsets4c/ac/10c/16c (Part stride60) and full body positions at body+10. Source body0 loadf278; reread Part base then body1f284/loadf294; reread basef29c, body2f2ac/loadf2b0 and body3f2bc/loadf2c4. Preserve ascending0,1,2,3 and late body failures; do not preflight all body values before source entry.
5. Transform each physical position using only its XYZ: initial rounded multiply(transposeColumnX,body.x), then fused add(transposeColumnY,body.y), then fused add(transposeColumnZ,body.z). No translation subtraction. Body-position W is never used. Local W is computed through actual zero coefficients; do not replace 0*infinity with guessed zero for exceptional inputs.
6. Sum **((q0+q1)+q2)+q3**, each full-vector addition separately rounded; multiply all four lanes by original literal **0.25**. Publish this local average to result **f368**.
7. Replace ONLY local average Y with the selected minimum below, in a scratch vector atf3b4. The public result still holds the unmodified local average during this scratch operation.
8. Rotate scratch back using original full basis R,U,A: rounded R*local.x, FMA U*local.y, FMA A*local.z; publish full result **f3d4**. No affine translation or forced W. If retained ground basis W is unknown, track it rather than importing a host canonical affine lane.

Thus observable result prefixes are initial caller value ? full zero ? local average ? final world vector. A staged host error after f368 retains the local average, not the previous world position. The leaf has no native fallible math collaborator, no reciprocal, dot-reduction helper, normalization or estimate. Straight f32 multiplies/adds/mul_add express the recovered operation graph; introducing a generic normalization or projection changes it.

## Minimum-Y branch, ties and NaNs

Atf338, raw VMX compares replicated **q1.y > q0.y**, with record. mfocrf/extractf36c/f370 consumes the all-true flag; all lanes hold the same scalar comparison. It selects q0 when ordered greater, otherwise q1. Therefore equality and unordered choose q1.

Then load q2.yf388, q3.yf390, chosenYf39c. Compute rounded `chosenY - q2.y`f3a0; `fsel(test,q2.y,chosenY)`f3a8 chooses q2 for nonnegative test (both signed zeros), chosenY for negative or NaN. Next rounded `value - q3.y`f3ac; `fsel(test,q3.y,value)`f3b0. This is an asymmetric source selection chain, not Rust min/minimum. Examples: a NaN q0 with finite q1 is discarded by the first selection; a NaN q1 propagates through both fsel stages; a NaN q2 or q3 is ignored when the prior selected value is finite. Equal final candidates choose q3's lane bits.

## Caller source snapshot and stores

The category controlling this entire block is an **EARLY snapshot**, not a fresh category read at35508. Update1 first runs omitted OffboardJumpingTest.Tick, LandingOnDeckManager.Tick and StartSkeletonLineTests. It then reads actual aggregate PhysOut+2ac at1fdc, samples/checks write latch+4c at1fe0..1ffc, reads PhysOut_State pointer+1c at2000, and captures category field+0c into r18 at**2008**, BEFORE ProcessedPhysIn::Reset200c. Later processed category copies do not replace this retained local snapshot.

At**3508** compare that category with OFFBOARD=500:

- OFFBOARD: actual GetSkeleton virtual+48 at3524, load its **mAnimToWorldTransform+2d80 translation+30** at3534, full store HiLOD+e0 at3538. Then mTimeOffboard+160 += fixed source kUpdateTime through353c..3544.
- Otherwise: actual GetSkateboard virtual+40 at3554 ? CalculateGroundPos3560 ? full result load3568 ? HiLOD+e0 store356c. Then mTimeOffboard+160=+0 at3570.
- Independently copy current +160 to processed+a10 at3580.

Next actual GetSkeleton then **GetEffectiveAnimToWorldTransform359c** and four full stores to processed+c0..+f0 at35b0/35b8/35c0/35c8. This intervening producer cannot disappear from a claimed full continuation. A selected bridge can stop before it or require a staged collaborator with actual completion/failure.

Only after those stores does retained ground-position gate execute:

- If captured category AIR=200, skip at35cc.
- If GROUNDTRICK=600, skip at35d4.
- Otherwise read **HiLOD+128 mFramesSinceJump**35d8, signed compare with0 at35dc; only signed >0 proceeds35e0. Values with bit31 set are negative, not large positive unsigned counts.
- Load new +e035f0, old +d03600, and processed+93c scalar3604; then **store new +d0 at3608**. The scalar capture occurs before the ground-position store, though its reciprocal work is later.
- Only afterwards read processed+928 frames-since-teleport3624; signed >1 gate362c controls +f0 displacement and +100 filtered ground velocity. It does NOT undo/prevent the already completed +d0 write. Source displacement=new-old366c; filtering contains reciprocal estimate/two refinements and0.95 blending. That separate numerical suffix needs its existing estimate-port integration before claiming it complete.
- Regardless of either gate, load current retained +d036a8 ? processed **+1e0** store36ac. Fresh processed pointer then +e036b4 ? processed **+1f0** store36b8. Later +100?processed+200 follows. Preserve independent stores and late failure prefixes.

## Constructor and counter producer boundary

HiLOD source constructor initializes +d0 to full zero at3db0 as previously proven; +e0 also has its own full-vector constructor store (source r3=e0 at3d74, load3db8, store3dc0). These are genuine constructor values, not terrain observations. A retained projection should explicitly start uninitialized and run these source stores at the selected actual constructor execution, not allocation convenience.

**mFramesSinceJump+128 constructor is100**, not zero: r30=0x64 at3dc4, store3dfc. Its update occurs LATER in the same Update1, after ground-position gate and prepared-velocity work. At3914 actual PhysOut_Ground pointer+20 is read after its latch guard, wallRideExit byte+120 at3918 optionally resets +128 to0 at3924. Then count increments with raw addi393c/store3940. Subsequent actual processed+8d0 bit0 or processed JUMP bit22 combined with HiLOD+120 allow-jump bit30 chooses accepted jump: optional debug cvar/owner reads3994..39c8, reset count0 at39d0 and clear allow-jump39d8. Rejected branch updates allow-jump from NOT requested jump at397c/3980 and clears processed JUMP at398c. Final current count copies to processed+91c at39e4. Full byte/bit producer integration remains necessary; do not use the later processed+91c as though it were the earlier gate's pre-increment +128.

Thus constructor100 is a legitimate initial gate input if its native constructor actually ran. It is not permission to hardcode100 on every call or skip later accepted-jump/wallride history.

## Existing owners and smallest genuine integration

- SAME `PhysicalPlayerAssemblyOwner.reckoning()` already owns `PhysicalPlayerReckoningOwner`, shared by board+104 and Skeleton. Existing `Reckoning::ground_transform()` exposes exact +20 basis, while `transform()` is the wrong field. A completed `PhysicalPlayerReckoningPhase` guard can prove a selected completed update; constructor-backed access is distinct and must retain that origin instead of demanding a fabricated tick.
- Actual board session/backend and attached Part0..3 are retained. `DynamicAssemblySession::try_part_contact_spy_observation` gives occurrence-checked real Part/body identity; existing body-pool pose/field readers provide position XYZ. `try_attached_part_body_transform` contains the correct physical position but reads orientation too. For strict source read timing prefer a narrow existing actual body-position reader or narrow field getter over manufacturing a matrix/GetTransform. No new body/contact owner is needed.
- `SkeletonRootVector`/matrix typed W is suitable for actual source lanes. Raw body W is irrelevant to this kernel; Reckoning basis W controls final W. Do not fill unknowns merely to make a generic matrix multiply accept them.
- Existing world pair/backend/attachment authorization and SAME owner Arc checks should wrap a selected calculate-ground-position leaf with explicit result storage. Capture actual ground basis at source entry, then clear result, then obtain/read actual board body positions at source sites. No World collision service or completed contact-frame token belongs in this kernel merely because its name contains ground.
- The next smallest genuine production change is that leaf plus actual same-board bridge. This can calculate native all-states position from retained physics and Reckoning without claiming full Update1.
- Full +e0/+d0/processed publication additionally requires the actual early PhysOut_State category snapshot, actual retained HiLOD fields/counter producer and intervening effective-transform stage. Those sources are not established by current controlled GroundAnimation input literals, prior terrain intersections, contact winners or animation category guesses. Keep a selected caller boundary explicit until those owners exist.

Suggested focused tests for future implementation: identity basis/asymmetric wheel heights; tilted ground basis distinct from ordinary transform; nonzero ground translation ignored; source full4 output and unknown basisW; early and late body refusals retaining zero; minimum signed-zero and NaN cases; caller AIR/GROUNDTRICK/signed-count gates; teleport<=1 still retaining +d0; source early category snapshot distinct from later PhysOut changes. Original actual-board corpus should compare actual Parts0..3 and SAME retained ground basis, without relocating contacts or substituting terrain height.

Fresh raw SHA-256, end exclusive: CalculateGroundPos82b2f1b0..82b2f3e8 = **5f3eb87e1aa440b2411e46673ac8d46faf15555f29237f5bc88d1dd0656e74ea**. No test or reference-trace parity is claimed by this read-only packet.

