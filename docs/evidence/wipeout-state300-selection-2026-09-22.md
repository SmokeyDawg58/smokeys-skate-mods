# Wipeout current-state selection

After Wipeout104 integration, original bail frame2 completes Actor and reaches
SystemLogic UnsupportedCurrentState300. Root independently inspected existing
selector ownership and fresh CalcSuggestedState82ddbd58 decompile/raw.

Common counter, teleport, fall-through reset and AI-follow-path priorities run
before the current-state dispatch. At82ddc538 subtract201; table dispatchc558
uses the result. State300 therefore uses entry99 at82ddc6e8; fresh bytes
82 dd d2 94 point to82ddd294: mr r3,r27, then epilogue. This is a genuine
unchanged-state return, with no state-specific reads or recovery decision.

Root adds only the300 arm to the existing selector. Existing request clearing
runs afterward on successful unchanged selection. Common prefix and pending
noncurrent transition behavior stay intact. One focused test checks counter
effects, no Ground/Air field reads and common AI priority before the sticky arm.
Independent agent source/code review approves: fresh raw c538=397bff37,
c558=4e800420, table c6e8=82ddd294, d294=7f63db78 plus epilogue.
Execution accepted in queued120-2 gate: fmt/strict Clippy, 4675 units and six
docs pass. Original bail frame2 now completes through post-physics/presentation;
frame3 reaches the later Motion expression270 boundary. Original aggregate
18/21; other failures are queued frame3 expression5 and isolated collision
fixture's zero Skeleton impulses. No full bail/recovery claim.
