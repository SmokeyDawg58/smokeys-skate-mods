# Reached ControlAirLegExtension / Air lifecycle

The original queued120-frame run advanced from Motion157 to frame10 PendingCallback650/address82cb0838 after the Air selection packet. This is the native ControlAirLegExtension instance constructor, not a synthetic follow-up state. Existing occurrence650 is inherited by both Sequence690 and Default691.

## Source and retained owners

CreateInstance82cb0838 allocates0x14 aligned0x10; actual allocation success initializes phase+8=0, distance+c=+0, down-time+10=+0. Allocation failure remains no instance. Begin and End vtable slots are825187c0 source no-ops. The shared generated controller keeps the new instance and original occurrence/token; no second state machine owner is introduced.

StateUpdate690/691828f1898 calls Air252828efec0 then packed PlayAnimation25. Air252 calls OnBoard3 first, then slots20 ForcePhysics no-op,21 AttachIntent,22 ControlAirLegExtension,23 ClearTrickAttr no-op,24 ApplyingBodyTilt no-op. The retained active IDs are0..3,17..32,75,76,650..652,1359(or1360). Exact list/address guards preserve current callback ordering and reject mismatched active sets. Existing callback implementations remain shared.

Update82cbfba0 verifies Animatable then Physical then Motion interfaces, samples actual PhysOut System COM velocityY and only then reads the instance phase. Initial phase stores1/2 before cached GetTreeAttr(disttocog). Missing attribute samples OffBoard+42 can_land_on_skateboard. True uses actual Skeleton right bone+70; false stores Animation+48 COM-to-deck first and optionally replaces it for a toe enum. The geometric path samples bone then COM then Up and dot3(COM-bone,Up). Existing same-owner PhysOut publication/getters and host_dot3 numeric boundary are reused; no local-pose or authored fallback.

GoingUp82cbc8f0, GoingDown82cb0900, PreLand82ca1ab0 and IsTimeToPrepareToLand82cabab8 are connected. Ordered phases/partial stores are preserved. The existing actual actor.motion_time_to_prepare_land is the Motion virtual query. FullExtension lookup uses the same cached animation rows; its first payload word is read as the native union without a fabricated float-type assertion. Unknown payload storage remains a typed error only after the live custom gate actually requires it.

Update publishes AddSettableAtt(disttocog) before updating down-time; only afterward it samples the retained generated in-air graph atlayout+530 (x540/y550), and publishes extend. Same animation setter handles sequence-1/no normalization. Earlier stores survive late attribute/overlay failures.

The global in-air instance is the EXISTING PrepareLandAttributes.inair bound from the same runtime Vault; no duplicate global wrapper is made. Field-only cached layout projection avoids unrelated relocated pointers. Generated null layout uses the existing documented DefaultDataArea5c0 bytes; generic absent scalar uses DefaultDataArea4 zero. Dynamic keys are UpSpeed FA94C1BF409104FA, DownSpeed FFF01289F3DA3EF6, Minimum DE42E61B3197EE74, PrelandSpeed174776FA810206FF, FinalDistanceA766556A9F24A64B, OffboardMultiplier98BE256AAAFEFCD1.

## Numerical/source branch details

Reviewer raw proof is artifacts/motion-air-leg-source-review-2026-09-22.md. Up only transitions on ordered velocityY<0; NaN stays Up. Fused old-dt*speed then FSEL selects candidate ifcandidate-minimum>=0, otherwise minimum includingNaN. Down uses actual velocity/up dot3, sign-bit negation and absolute-up-J dot. Its mixed speed is(1-upright).mul_add(given,upright*downward), then FSEL cap. Preland increment is FMADDS1be4/store1be8. Custom timing returnsfalse unless ordered available<duration; NaN does not demand overlay. Existing host fused-negative-product/dot3 exceptional signed-zero behavior remains a documented host numeric boundary, not unconditional Xenon parity.

## Cvars and integration

Reserved stable slots308..313 in native initializer order: DebugText false82f86138/ref830a37a8; Up true82f86188/ref830a3f14; Down true82f861d8/ref830a36d4; Preland true82f86228/ref830a38e8; CustomEarlier0.06 (bits3d75c28f,range0..1)82f86278/ref830a35d8; UseCustom true82f862d8/ref830a5768. Table82fd4ee8..4efc precedes DistComToBoard4f00; sectionAnimation_Skater_InAir. Live reads stay in source branches, not eager startup capture. Manager title-order test inserts Custom Earlier,Debug,EnableLanding(existing),Down,Preland,Up,OffboardTime(existing),UseCustom.

Focused tests cover constructor/null allocation, early phase publication, partial initial-distance store, AddSettableAttr-before-timer/graph failures, disabled-adjustment read timing, NaN Up/FSEL/custom-time behavior, Down?Preland failure prefix, live cvar defaults/rebinding, and exact inherited callback topology. Author uses standalone rustfmt and apply-check only; root owns Cargo/original execution. This connects the reached Air occurrence, not unrelated grab/manual/grind/offboard graph branches.
