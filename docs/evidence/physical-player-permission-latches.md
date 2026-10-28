# Actual jump/grind permission latches

Consecutive Update1 after Heading publication:3ab8..3ac8 copies actual HiLOD120 bit01000000 (existing tail.collide_with_world_only) into processed8d4/08000000, preserving other bits and knowledge. Calls CheckAllowGrindLogic82e00130 at3acc. This consumes jump/preparation flags; it remains scheduled while GrindManager investigation is explicitly parked. No query, investigation, or fake grind result is modified.

Source014c..0168: PREPARE8c4/00200000 clears actual HiLOD138 counter and sets HiLOD120/02000000. Then016c..0198 samples8c8/80 and, only if false, fresh8c4/JUMP00400000; either clears138 and sets120/06000000.019c captures120; only captured02000000 enters timed branch. Fresh8c8/40 or (captured120/08000000 and no8c8/10000) clears06000000 at01d8. Regardless of that clear, source captures138 at01e0 then samples live kMaxGrindIllegalFrames at01e8/ec. Signed count>maximum then tests fresh120/08000000. If absent, actual DebugOverlay.Get020c precedes another clear06000000. All entered timed branches increment138 at0220/24; failed late services preserve earlier stores.

Unconditional suffix0228..026c uses fresh reads:8c8/40 ->120/10000000, then8c8/10000 ->120/08000000, then120/04000000 ->8c8/20, finally fresh120/02000000 ->fresh8c8/10. Complete source-word copies preserve all other bits. Runtime uses actual HiLOD prefix and SAME processed owner.

Cvar82: reference830e0af8; initializer82f9e770; table82fd629c, before heading81 table62a8; sectionPhysics_States_StateChanger; labelkMaxGrindIllegalFrames; titleMaxGrindIllegalFrames; default20; GUI1..240; nullcallback. GUI limits do not clamp live target values.

Focused controlled owner tests prove (1) prior latch clear does not skip late cvar and missing target preserves counter; (2) inactive branch needs no cvar/overlay and publishes exact latch words; (3) timed-out branch requires real overlay before counter increment, then succeeds using same holder. Root owns same-pair corpus integration and Cargo gates. No standalone investigation assumptions enter these tests.
