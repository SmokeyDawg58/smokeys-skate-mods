# Original Xbox input patterns and retained matcher plan

Read-only audit, 2026-09-08. Confidence **exact** for the cited raw instructions, original bytes and layouts; **high** for the bounded integration plan. No runtime implementation, Cargo or assets added. See input-actor-next-closure.md for command-produced gameplay-pad ownership and Actor/listener scheduling. This packet does not establish a playable gesture-to-animation path.

## Original assets and loader

Used the existing compiled skate2_formats MountSet/MountedBig/SKATE2_ARCHIVES implementation via an external temporary rustc helper, without Cargo or a custom archive reader. Original root: <game folder>. Files remain only in external TEMP as skate2-original-<name>. GameInputManager::Load 826e3260 calls LoadDataFile in this exact order. LoadDataFile826e3180 formats literal `data\\joystick/%s` at31a4 and converts slashes31b0; actual mounted logical paths are `data\joystick\<name>`.

| File | Input type | Bytes | Patterns | Point range | SHA-256 |
|---|---:|---:|---:|---:|---|
| skater.pat | right1 | 10252 | 78 | 2–4 | 8d01a8246a05ba7f796174ee6c96219822c376a067bb5c3b96e50a9cd3923bcd |
| skater90.pat | right1 | 10486 | 78 | 2–4 | 43432ba702a10cb3055e648f7f90d92e39b74e747ce9003858e770e402e5ec15 |
| skaterN90.pat | right1 | 10560 | 78 | 2–4 | 8133ad5085036d4e491698cb76cd4925141746853781f81df6f34dc0ae283cd1 |
| skater_air.pat | right1 | 2835 | 21 | 2–4 | 7edc70a4719e77d1ba7ab8a0216acff02f912909adf46eb70f48400c0cb751d0 |
| skater_fingerflip.pat | right1 | 1670 | 14 | 2–3 | 17166257329b0230ebb5eba3aea3eeb288b231a1b6573f9f476da09a44f7350b |
| skaterls.pat | left0 | 1249 | 10 | 2–3 | 03487fc2c609f655444b62edfc53802b6ea7d69b1048b172d192b9a7b4c06ec8 |
| skaterstep.pat | left0 | 923 | 6 | 3 | 942685cd8d8c8b57461ac98372d081b5634fb867f4fc9213036d1afcc64645d6 |

All 285 patterns use coord, none clock. Duplicate names are real alternatives, not invalid declarations: preserve ordered nodes and pattern identity, never deduplicate into a name map. Authored left patterns include FrontFlip/BackFlip and StepForward/StepBackward; right collections include ordinary, rotated, air and fingerflip variants.

KnownPatterns::Load826e2d30 first Purges existing nodes2d60, then opens the file2d7c. Open failure asserts2da0; it is not a successful empty collection. TokenFile::Open826df7a0 uses actual FileLoader::SyncLoad atdf858, stores bytes and index0. ReadStr826df878 uses sscanf `%s`, advances by token length then SkipSpace826dda78 accepts only space/tab/LF/CR. There is no comment or quoted-name grammar in this tokenizer. ReadInt826df940 uses atol after required token; ReadFloat826df990 uses atof then frsp. A bounded parser must explicitly refuse unsupported malformed numeric syntax rather than claim Rust parse is generic CRT parity.

Recognized case-sensitive directives and effects:

- global_tolerance_dist: required token?atof?f32?separately rounded square?collection default store2e38.
- global_tolerance_time, global_tolerance_speed, global_anticipation_delay: required token, atof, **discard result** (shared2f04). These authored values do not configure candidate timers.
- pattern: required name2f24?AddPattern2f34?copy current squared default2f3c. Name storage32 bytes; exact truncation/null semantics should be retained when supporting non-original names.
- coord: ReadFloat X305c then Y3084, push_front30bc. Thus ring begin is LAST authored point, ring back is FIRST authored point. Only XY is meaningful; do not fabricate unused packed lanes.
- clock: integer then float radius. Zero clock gives XYzero independent of radius; otherwise X=rounded(sin(clock*0.52359879)*radius), Y=-rounded(cos(...)*radius), push_front. Original files do not exercise it; transcendental and constant parity must be separately validated before claiming general clock support.
- min_time: ReadFloat30dc, discard.
- tolerance_dist: assert current pattern3108, ReadFloat3114?rounded square?pattern store311c.
- Unknown directive asserts3130/debugbreak3134. Never skip it as an unrelated section.

LoadDataFile initializes collection default squared tolerance bits3e23d70b (0.1600000113), fixed node capacity80; each Pattern allocates a point ring with six slots. After loading, ResetPatternCaches3210 runs BEFORE collection pointer publication to the manager vector. Current original maximum78 nodes/four points stays inside these bounds. Capacity and malformed empty-pattern behavior remain explicit boundaries, not permission to manufacture points.

## Owners and representation

PDB Pattern size40: points ring+0, squared tolerance+1c, name[32]+20. Match size10: pattern pointer+0, normalized speed+4,error+8,time+c. PatternNode size20: pattern+0, BE flags word+4,error+8,cached squared tolerance+c,cached first-authored point+10. Masks recovered from raw: candidate80000000, finished40000000, point index3c000000 (shift26), total time03ff0000 (shift16,10 bits), waiting0000fc00 (shift10,6 bits), number of points000003c0 (shift6,4 bits), reserved low6.

Matcher sizea30: retained recognizer attribute instance+0, match-last-update highbit+10,last matched pattern+14, actual history pointer+18, ordered fixed node vector+20. Constructor826e2710 binds the supplied attribute collection2730, retains SAME history2740, nulls lastpattern2750, clears only match bit2758, copies ordered PatternNodes2770 and RemoveAllCandidates2778.

PlayerInput::AddPatterns826e2b18 raw fixes decompiler ABI: r4=patterns2b64, r5=64-bit RecognizerKeys[inputtype]2b68, r6=that player's SAME history pointer2b6c, then Matcher ctor2b70. RecognizerKeys at820b7f98 are BE left0 `0caf1482a6101fa3`, right1 `edfad6a73a279045`. Do not substitute a global default or bind a fresh instance on every match. The generated recognizer class key/cached layout producer still needs its own provider recovery before implementation; current audit establishes the waiting threshold read from that retained instance's cached layout byte+21.

PlayerInput constructor826e2440 creates two histories with201 storage slots and size0; it does not initialize 201 valid zero samples. Reset826e2c20 constructs players0..8 and adds left matchers then right matchers. PlayerInput owns a command-updated embedded InputPad; neither SIM cache pad nor an arbitrary equal-valued pad proves that authority. See the command-owner packet.

## Retained matching and held/released behavior

PatternNode::SetIsCandidate826df128 first changes candidate and clears total-time bits(df158), then waiting(df184), then error+0(df1ac); it preserves finished/index/number/reserved. Reset826e02e0 calls it false02f4, clears finished030c, sets point index n-1 (zero for empty)0328/33c, copies ring BACK to cached first point037c/380, caches tolerance0398, clears time039c/wait03c8, stores count03f4. Empty ring still reaches the back-point read; do not silently accept malformed empty patterns.

Matcher::FindMatch826e1408 checks prior-success bit before history. If set: RemoveAllCandidates1434, clear bit1440, returnfalse. This is one FindMatch-call cooldown and does not clear the caller Match. Otherwise history size must exceed1; read newest and previous, construct current/previous/delta, then FindMatchPattern14dc. On true store last pattern14f8 then set success flag14fc. No generic full-history template classifier belongs here.

UpdatePatterns826e0b58 visits nodes in authored order. Squared XY distance is separately rounded X² and Y² then rounded addition, not an FMA or length square root.

- Noncandidate: compare current against cached FIRST authored point. Raw0e0c bgt rejects only ordered greater; unordered admits. SetIsCandidate(true)0e18, index=n-2 with source assertion, error=distance0e58, increment10-bit time0e6c.
- Candidate already finished: no update.
- Candidate unfinished: read target at retained reverse-ring index and next point at index+1, compute both distances. Raw0cf0 BO4/BI25 branches on NOT ordered greater, so unordered target distance also counts as reaching target. Add error0d78, increment time0d88; nonzero index decrements then clears waiting0da8, zero index sets finished0dd0.
- Outside target: during first segment (n-index==2), staying within next point's tolerance suppresses both time/wait increments. Raw0d10 NOT ordered greater also suppresses on unordered next-distance. Otherwise increment time10-bit and wait6-bit0d34. Then read retained recognizer cached layout byte+21 at0d4c. If waiting exceeds it, SetIsCandidate(false)0d58 then clear finished0d64. Do not replace masked wraps with unbounded host counters.

IsHoldingLastMatchedPattern826df9e8 first reads last pattern; null returnsfalse without history or held output. It compares newest history point to pattern ring BEGIN, the LAST authored endpoint, with current pattern tolerance (not node cache). Rawdfa70 BO12/BI25 releases only ordered distance-minus-tolerance>0: clear lastpatterndfa8c and returnfalse. Otherwise publish retained name pointerdfa7c and true; NaN remains held. There is no extra hysteresis timer. Releasing produces absence of a held message; downstream listener state determines release intent, so a direct synthetic Jump event is not equivalent.

PlayerInput::UpdateInput826e18d8 applies actual history deadzone, inserts newest at front and evicts tail on collision, then makes two full passes: ALL held messages first, then ALL FindMatch messages. UpdateRecognizer826e2008 reads map78 then77 for right(Y negated), calls2090; then75 then74 for left(Y negated), calls20f0. Exact deadzone raw predicate/cvar timing and broadcaster/listener delivery need the next source stage; this packet does not claim that enclosing loop implemented.

## Winner and normalized speed

PickWinner826dfb00 visits only finished nodes, preserving ordered first ties. Capture min/max cvar targets at entry (full timing must stay in implementation), then for each finished node read min valuefb4c and max valuefb60. fsel stages: a=fsel(minError-totalError,minError,totalError); b=a/n; c=fsel(maxError-b,b,maxError). Compute n² then n³ then n4 with separate f32 multiplies; score=n4/(c*totalTime), denominator separately rounded. First finished always becomes winner, including NaN score; later replaces only ordered score>best (fb b0 BO4/BI25 skips LT/EQ/UN). Do not use max/min methods with different NaN or signed-zero semantics.

FindMatchPattern826e1038 calls UpdatePatterns1058 BEFORE clearing caller Match: patternnull106c,speed+01078,error1e6 107c,time+01080. PickWinner1084. A winner publishes pattern/time/error before normalized-speed cvar reads. Ratio is f32(totalTime)/f32(n). Fastest target/value read10c4/10e0; raw10f0 only orderedGT goes onward, otherwise speed1 (NaN included). Slowest target/value read110c/1110; raw1118 only orderedLT goes interpolation, otherwise speed0 (NaN included). Interpolation re-reads slowest value1124, computes r=1/(fastest-slowest), product=slowest*r, negates product, then **fmadds(r,ratio,-product)**1140. Preserve that late repeated value read.

Original Input cvar defaults independently decompiled: fastest8304e2a0 ctor82f5c998 value1.75; slowest8304e2cc ctor82f5c9f8 value4.4000001; minError8304e350 ctor82f5cb18 value0.15000001; maxError8304e37c ctor82f5cb78 same0.15000001. Runtime must sample actual retained cvar targets, not hardcode those defaults.

## Minimal implementation closure and remaining proof

Reuse existing formats mount service, input_map selected original IDs, input_pad retained pad, and the actual command-owned gameplay pad under construction. Existing skeleton_physics_history is physics history, not this ring; living-world recognizer fields are unrelated. No existing Rust Pattern/Matcher implementation was found.

1. Parse actual mounted files into ordered immutable pattern owners with bounded original coord grammar; preserve duplicate identity and source push_front ordering. Full clock/CRT malformed parsing can remain explicitly unsupported until separately proven. Loading must expose partial failure rather than successful empty content.
2. Construct SAME PlayerInput's two empty histories and per-collection retained Matchers, preserving key/instance ownership and cached node Reset. Recover generated recognizer cached layout and actual waiting byte provider first.
3. Implement exact incremental node updates, winner and held state with staged cvar/provider reads. Tests need first tie, duplicate names, reverse-point order, waiting suppression/wrap, NaN branches, cooldown and late failure prefixes. Original-file parsing alone is not runtime validation.
4. Connect command-owned mapped axes to source deadzone/insertion, held-first/match-second messages and actual listener. Then retained derived input and listener-to-generated ActionGraph intents must connect to animation's original RawPhysIn jump records. This is the held/released flick route; full Actor scheduling, grab combinations and multi-flip remain outstanding.

Fresh SHA-256 byte ranges (end exclusive):

- Load826e2d30..826e3178: 392faafda62fb036ac83b244b05b7df04fb72adc0b83179955a345eecdac7d70
- UpdatePatterns826e0b58..826e0e88: 92f524ab96a582ec08c0f98abbaa38dceecfbe210de69ae0e0ffb4fd7e1cdf17
- PickWinner826dfb00..826dfbd0: c27fe67582b7c6f6a3e113d35dac97431f7cb6761415f2010d019b8ca3423577
- FindMatch826e1408..826e1510: 2edb228abd913e71a18be0d2e0fbd816b20c526bc0995e1086049a863d5c1122
