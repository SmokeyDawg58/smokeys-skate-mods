# Connected Air priority predicates

The prior callback-tail audit records the exact generated ancestry/priority order. After Air691, priority3 runout/teleport gates precede priority2 Air338, then existing OnBoard420. Priority1 AirStatic292/335 precedes targetOneFoot81, whose ordered children191/192 precede336/337. Only afterward does AirLand339 run. Direct priority0 targets1176/1334 use242/243; this patch admits their gates only and adds no manual/grind/offboard lifecycle branches.

Fresh native Pre table8290aa90 and Sustain8291e254:

|Expression|Pre address/operands|Sustain|
|---|---|---|
|338|82915358 c160 && c87|829221c8 true|
|339|82915424 c89 && !c282|829221c8 true|
|191|8290faf4 c218|8291fd48 c218|
|192|8290fb54 !c218|8291fda8 !c218|
|336|829151c8 c240 && c159|829221c8 true|
|337|82915290 c244 && c159|829221c8 true|
|242|82910fa0 c350|829200c0 c350|
|243|82911000 !c351|829221c8 true|

Every operand uses existing retained condition parent/cache publication. Boolean short-circuit remains native; source condition callbacks are not eagerly invoked.

IsBodyFlipping82c9cb40 verifies IPhysical and reads PhysOut+8/Air+1a9 atcc24. Fresh PDB confirms is_flipping. IsRidingGoofy82c9b308 reads PhysOut+38/Animation+9a regular_stance atb3f8, then a fresh getter reads+99 riding_switch atb410, returning boolean equality. It does not use backend stance or mirrored/fakie. TimeToLand82c9cc40 first reads actual Air+1a5 is_traj_valid atcd1c; onlytrue performs fresh output sample collision_time+a8 and existing ComparisonCondition comparison. No substitute estimated time.

HasTiltToLargeForPreland82c9c1c0 resolves actual ISkaterMotionGraph and returnsfalse for an absent interface. The adapter preserves this nullablegate before full bound-context checks; nonnull uses existing identity/controller validation, then existing prepare_land Source and has_tilt_too_large825f48d0 kernel. That reuses existing actual PhysOut, retained global attributes, pointgraphs and numeric profile. No second tilt calculation or owner.

Tests check every ordered two-operand truth combination, skipped and propagated late errors, negated stance/manualgate arms, and separate Sustain behavior. Existing prepare_land tests continue exercising actual numeric branches. Author ran standalone rustfmt and apply-check --ignore-space-change; no Cargo. Root's newest original run reaches frame11 Action55 before these next Motion checks, so this packet is connected source recovery, not an assertion that runtime passed all gates.
