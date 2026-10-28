# Live Wipeout and runout decisions

Author draft, 2026-09-13. Fresh IDA disassembly inspected for DoesPlayerRequestWipeout82dd9988, IsRunoutRequested82dffe88, IsWipeoutRequested82e00a50 and Update5 callers5438/5470. New files are unregistered pending root independent review, runtime owner integration and Cargo gates.

`physical_player_wipeout_decision.rs` retains lazy source callbacks. Disable-all true returns before all other reads. Otherwise all thirty enable target captures and values occur before the first request read, including an empty ledger. Captures retain target identity separately from value reads. The source schedule is in ENABLE_READS: first captures0..9,13,10..12 then reads0..13; late reads15,17,18,19,20,21,22,26,23,16,24,25,27,28,29. Native addresses99a0 (disable),9a0c..9bf0 (captures/values),9c60 (loop request),9c94..9cb8 (required DebugOverlay),9cc0/9cc4 (display target/value). Every requested index reaches overlay/display even disabled; accumulation never short-circuits. Both enabled/ignored display branches call the same empty825187c0 at9d04/9d5c and9d8c, so the adapter preserves required owner and scalar reads without inventing drawing effects.

Runout reads actual request event count first and returns false unless exactly1. It then lazily reads request2, request24, Processed8c4 mask0x20,8cc mask0x10000000, and Processed220 dot global J. Dot must use existing source numeric service; selecting only Y loses NaN-times-zero behavior. Absolute value is sign clearing, comparison is ordered strictly greater than bits3f4f5c29 atfff10/fff14 (NaN false). Wipeout first calls runout: true returns false, otherwise DoesPlayer; only false DoesPlayer reads Processed8d0 mask0x40. Actual Update5 calls Wipeout5438 and independently calls Runout5470 again: do not cache the earlier result.

Owner integration caution: existing WipeoutState.request_counter is ctor/reset scalar, while initialized WipeoutRequestLedger owns event count and increments even duplicate requests. Use the initialized ledger count, or consolidate/synchronize actual mutations; stale scalar0 is not proof of no requests. Before ledger initialization only runout's count-first false exit can use native ctor0; any reached request read must demand initialized ledger. The new kernel contains no fabricated owner or default request state.

`input_cvar_wipeout_decision.rs` proposes indices159 disable,160 display,161..190 enables in WipeoutType index order. These definitions were absent at audit. Root must append definitions and insert registrations according to native initializer-table order, then register this child. All are cVarBoolRef with null callbacks; disable/display seedsfalse, enables seedtrue. No startup-negative evidence substitutes for current live values. The separate kWipeoutOffboardEnabled initializer82f9c500 is not consumed by this function and is excluded.

| Index | Label suffix (prefix kWipeout) | Initializer | Table entry | Section | Title |
|---|---|---|---|---|---|
|159|DisableAllWipeouts|82f9c320|82fd6084|Physics_Cheats|Disable ALL wipeouts (trailing space)|
|160|DisplayDebugInfo|82f9c370|82fd6088|Physics_Wipeouts_Debug|Display Debug Info (trailing space)|

All enables below use section Physics_Wipeouts; exact labels are existing WIPEOUT_ENABLE_CVAR_LABELS in wipeout.rs, in this table's enum order. Initializer/table columns have82f9/82fd prefixes.

| Type index | Enable label suffix | Init | Table | Title |
|---|---|---|---|---|
|0|SkeletonContact|c5a0|60a4|Skeleton Contact|
|1|SkeletonDisplacement|c550|60a0|Skeleton Displacment|
|2|DeckAcceleration|c5f0|60a8|Planar Deck Acceleration|
|3|DeckFlipped|c640|60ac|Deck Flipped|
|4|BodyFlip|c690|60b0|Upside down no grabbing|
|5|BadAngleLanding|c6e0|60b4|Bad Angle Landing|
|6|HeavyLanding|c730|60b8|Landed Too Hard|
|7|VehicleCollision|c780|60bc|Hit Vehicle|
|8|AirToHardGrind|c7d0|60c0|Air To Hard Grind|
|9|HardGrindEngagement|c820|60c4|Hard Grind Engagement|
|10|BoardslideStopOnRail|c870|60c8|Boardslide Stop On Rail|
|11|OffBalance|c8c0|60cc|Off Balance|
|12|HungUpOnRail|c910|60d0|Hung Up On Rail|
|13|OffBalanceGrind|c960|60d4|Off Balance Grind|
|14|HardHorizontalGrind|c9b0|60d8|Hard Horizontal Grind|
|15|UpsideDownFalling|ca00|60dc|Upside down falling|
|16|OpposingContacts|ca50|60e0|Skeleton Opposing contacts|
|17|FailureToInvExit|caa0|60e4|Auto Exit Failure|
|18|SkeletonSquashed|caf0|60e8|Skeleton Squashed|
|19|LeaningContact|cb40|60ec|Skeleton Leaning Contact|
|20|DeckOpposingContacts|cb90|60f0|Deck Opposing Contacts|
|21|SensitiveSkeletonColl|cbe0|60f4|Sensitive Skeleton|
|22|BadSkateboard|cc30|60f8|Bad Skateboard|
|23|TooFastOffBoard|cc80|60fc|Too Fast OffBoard|
|24|FailureToGoOffboard|ccd0|6100|OffBoard Failure|
|25|OB_BadGround|cd20|6104|BadGround|
|26|OB_DeathLeap|cd70|6108|Death Leap|
|27|OB_LandingTooFast|cdc0|610c|OB Landing Too Fast|
|28|OB_NotUpright|ce10|6110|OB Not Upright|
|29|OB_LandedOnDeck|ce60|6114|OB Landed On Deck|

Focused kernel tests cover early disable, empty-ledger late cvar failure, all controls before requests, disabled request debug sampling, no successful-request short circuit, duplicate-event count, strict/NaN threshold and lazy outer animation fallback. Author has not run Cargo or registered modules. These decisions alone do not complete the first playable frame, request producers or full numeric parity.
