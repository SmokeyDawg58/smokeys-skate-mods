# Crail scoring, late-flip updates and Motion409

Implemented solo; source checked in IDA.

- Motion Pre4098291a118: c259 && c260 && !c262. The first two are HasFilteredMotionGraphIntent (sConditions+40c/+410), then raw HasIntent(+418). Preserves ordered short-circuit and existing cache semantics. Independent Sustain switch maps409 to true829221c8.
- State update2190419208 /828f2508 Into984: Lateflip80 parent (828eeddc call site), then instance23 ScoringTrick and24 PlayAnimation. Lateflip parent invokes OnBoard3,20 empty,21 AttachIntent,22 CreateAttribute. Wired all12 authored shared aliases652/663/672/673/679/680/966/968/978/980/982/984 using exact inherited global callback arrays, each25 entries once. Inactive authored states remain inactive.
- Callback2260 is ScoringGrabs on state1248, authored CrailGrab: angle BoardAdjustAngle, magnitude BoardAdjustMag, invertY true, left/right directional labels and down CrailGrab. Vtable82302b78+2c null-instance82bb3da8,+30 Begin825187c0 empty,+34 Update82c8ef88,+38 End825187c0 empty,+3c Create82ccb910. Admitted2260 to the existing lifecycle and implemented its previously rejected polar branch in the shared scoring function.
- ScoringGrabs::Create82ccb910 polar input reads angle then magnitude, defaults each tozero if absent, computes X=XMVectorSin(angle)*magnitude, Y=XMVectorCos(angle)*magnitude, multiplies Y by-1 only for polar invertY. Reuses existing source polynomial helpers (separate sine/cosine, not host libc). XY input ignores invertY as native does.
- Corrected directional intervals to depend on authored neighboring directions, as source82ccbbb8..bdf0 requires. Priority up,left,down,right; retained threshold>0.5. InAngle82c8c2d0..2ec uses LT/GT-clear branches, including unordered behavior; retained existing atan2 projection. Missing matched direction still reports error rather than picking an arbitrary grab.

Validation: 12-alias descriptor ancestry/uniqueness audit PASS; rustfmt parse PASS across five touched Rust files. Added polar axis/inversion/signed-zero and expression409 short-circuit/Sustain regression coverage, not executed. No compilation or runtime tests. Renderer/quiet unchanged. Check with build-release.bat and play-quiet.bat.

Separate outstanding work: Boneless602 remains unfinished; this batch does not claim to fix it.
