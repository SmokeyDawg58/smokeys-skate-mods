# Actual retained selector polling

Source Update1 82e03a84 calls TrajectorySelector.Update82dc6528;3a9c..3aa4 inserts returned low bit into Processed8c4 mask400. Constructor82dc4af8 writes false justStarted2392 at4e74, ready2393 at4e78, changed2395 at4e80. Actual retained OnboardTrajectoryOwner is installed by PhysicalPlayerAssembly construction and binds the same owner/processed Arc.

Update6544 captures justStarted,6548 always clears changed.6550 skips future access when unstarted. Otherwise6554 reads actual future0.ready and655c skips completed-query branch if false.6910 returns retained ready, including true retained from prior selection. No assumption based on current physics state replaces these reads. When started and future ready, adapter returns typed CompletedQueryPending after the source changed=false prefix, preserving all other fields and leaving caller ready-bit publication untouched.

Focused controlled states test the real retained owner method: actual constructor idle; unstarted ignores future readiness; started unfinished preserves ready=true; completed query preserves query and ready while clearing changed and reporting pending. Root owns all Cargo and original corpus gates.

Next source continuation is HeadingAdjust82e00288 called3aa8. Predictor launch/completion and selector scoring are separate active-jump dependencies; no completed prediction is manufactured.
