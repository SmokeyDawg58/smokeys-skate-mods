# Mount and four Motion expressions, September 25

Implemented solo against IDA.

- Update2190404136 / 828eea28 is Mount34, the only authored state using this address. Calls OffBoard1 then instance14 PlayAnimation.Update and15 CreateAttribute.Update. OffBoard1 calls Motion0 then AttachIntent slots4..11, ToggleBoard12, CreateAttribute13. Motion0 updates CharacterGesture, UpdateTimeSinceTeleport, UpdateRidingFakie and FakieHeadChannel at0..3. Exact authored ancestry34->8->1->0 produces global callbacks0..13,111,112, each once. Existing callback implementations reused.
- Motion Pre117 at8290dd78 reads condition197 (sConditions+314), HasIntent(N_FS360PopShuvit).
- Motion Pre143 at8290e790 reads condition237 (+3b4), HasAnimAttribute(ChristEarlyOut). Existing shared attribute query used, with authored default sequence -1 and NOP comparison.
- Motion Pre300 at82913420 reads !condition89 && !condition85, preserving early-out order and caching raw condition values before negation.
- Motion Pre322 at829146b4 reads condition262 && condition263 (sConditions+418/+41c), both HasIntent, ordered lazy evaluation.
- Independent Sustain switch8291e250 maps117/143/300/322 to native true default829221c8; no condition reads there.

Static Mount ancestry/unique dispatch/complete alias audit PASS. Rustfmt parse-only PASS on four touched files. Added expression truth-table/order and Sustain-default regression coverage; tests NOT executed. No build/runtime. Check with build-release.bat, then play-quiet.bat. Renderer and quiet mode unchanged in this batch.
