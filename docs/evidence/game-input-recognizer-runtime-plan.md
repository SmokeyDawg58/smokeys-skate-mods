# Same gameplay owner to retained recognizers and messages

2026-09-08 read-only source audit. No Rust edits/Cargo. Exact confidence for listed raw sequencing and gates; high for proposed scoped interfaces. Reuses independently reviewed input_pattern/parser/matcher and command-owned GameInputPlayer. This is not Actor listener or full GameInputManager scheduling implementation.

## Entry and map sampling

GameInputManager::Update826e2578 requires actual DebugOverlay even before iterating players. Snapshot vector begin/end; for each player read mInput->mPad, skip null/count0, then require retained flags&updateMask nonzero. Invoke UpdateRecognizer then GenerateGameInputEvents; after all players call EventSys::Flush with actual broadcaster QueueID. This flush is not part of each stick or each player's UpdateRecognizer. Existing selected map supports74..95; GenerateGameInputEvents additionally uses59..63 and remains unsupported rather than silent no-op.

UpdateRecognizer826e2008 obtains actual PlayerInput and its retained mInput at+1b0. Map reads:

- ID78 at204c: globals g3, pad, g2, map, g1, g0 are read2034..2048.
- Preserve returned Y; ID77 setup2058..2070 reads freshglobals/pad/map. Negate retained Y at2074 BEFORE ID77 call2078; then invoke right-stick(type1) UpdateInput2090.
- Only after the entire right update finishes, read ID75 at20b0, negate its result20d4 before fresh ID74 call20d8; invoke left-stick(type0) UpdateInput20f0.
- Only after BOTH updates, run separate right then left display-input branches described below.

Do not eagerly sample all four axes or bind a copied mapped snapshot: a right-stick failure must prevent left map reads. GameInputPlayer::try_value can reuse its SAME map/pad with selected grammar's explicit no-global-expression limitation. Its retained globals are still native owner fields, not synthesized action values.

## Deadzone and same history mutation

PlayerInput::UpdateInput826e18d8 receives X and already-negated Y. It computes fabs(X) and constructs scratch lanes, captures actual kInputHistoryDeadZone target at192c, then loads target vector+100 once at1940. X threshold splat1944 and fabs(X) splat194c feed vcmpgtfp.1950; read CR6 all-true bit1954/58, branch1968, assign literal+0 only if thresholdX>abs(X). Compute fabs(Y)1974, use Y threshold from SAME captured vector1980, compare1998 and assign+0 at19ac only if thresholdY>abs(Y). Equality and unordered preserve the original scalar, including signedzero/NaN. This is independent axis suppression, not radial scaling. Meaningful lanes are XY; no additional W source is required.

Cvar symbol8304e240 initializer82f5c7f0 constructs Input/Dead Zone default[.1,.1], bounds[0,0]..[1,1]. Defaults are evidence only; runtime must supply actual captured target value. No second target/vector read between axes.

Store filtered X/Y19b4/19bc, unused packed lanes zero19c4, obtain SAME PlayerInput history pointer for selected type19c8. Ring storage201/maximum200 behavior is already implemented by InputPatternHistory::publish_selected_point. Raw19cc..1a24 adjusts begin/end/size then writes point1a40. The typed owner operation is non-fallible after actual point sampling; do not insert a speculative cvar/map callback after ring cursor mutation but before point publication.

## Two passes and retained event identity

After history publication, snapshot matcher vector begin/end1a44/48. FIRST pass in file/insertion order calls each matcher IsHoldingLastMatchedPattern at1a6c. A successful hold constructs cMsgInputPatternHeld: kind95d81395, native size24, port0, messageID0, actual pattern NAME pointer, player index. It checks broadcaster QueueID==6 at1a8c, then reads broadcaster port at1ab8 and enqueues on6 at1acc. No actual InputSK8 pointer is in this held payload. Failure after prior enqueue retains prior effects; held messages do not mean a new match.

After ALL held checks, resnapshot matcher begin/end1adc/e0. SECOND pass initializes a local Match every iteration: error1e6 at1b50, patternnull1b54, normalizedSpeed+0 at1b58, time+0 at1b60; calls FindMatch1b68. On false continue. On true, matcher already retained lastpattern/success before enclosing code proceeds; execute lazy display-reference initialization, nonnull pattern assertion, and conditional debug branch below. Only afterward construct cMsgInputPattern: kind6889aa9b, native size40, port0/messageID0, actual PlayerInput.mInput freshly read1f6c, exact Match four fields copied1f8c..1fb0, index1fb4. Check QueueID6 at1f84/1fb8; read port1fd0; enqueue1fe4. Do not publish before debug checks or roll back a completed matcher success if enqueue fails.

Typed messages should retain Arc<InputPattern> so names point to the same original pattern occurrence, and an opaque gameplay-input occurrence token for match payload. Equal name is not sufficient: duplicate patterns are real alternatives. Do not fabricate an Actor receiver to consume them. A real retained broadcaster projection may own ordered typed messages awaiting explicit dispatch; native Hermes routing/handlers and flush lifecycle remain separate work. Existing command manager is not this queue6 broadcaster.

Per UpdateRecognizer order is right:held-pass?match-pass, then left:held-pass?match-pass; it is NOT all right/left holds globally before all matches.

## Required conditional debug effects

Successful FindMatch triggers process/function-static lazy reference initialization at1b78..1c20 on its FIRST success. It copies BOTH original left and right gesture-display cvar reference objects, including mTarget pointers (left1bb0/right1bd0), into kDisplayGestures, and registers its destructor. Later successful matches select copied target pointer1c74 and read byte+cd1c78. Do not recapture original references every call or initialize one copy per player. An honest selected service must retain this shared reference-capture state; editor replacement of the original ref's target after first success is not equivalent to reading the copied ref's current target.

After initialization, assert match.pattern nonnull1c44..60. If copied display valuefalse, skip directly to message construction. If true: require actual Simulation1c84, actual SkaterMan+ d0 at1ca8, invoke its GetLocalIndex1cd8, and compare player index1cdc. Only equal local index continues. Require overlay1ce8, then create FRESH recognizer attribute instance: class7f086875d621c88c and RecognizerKeys[type] at1d24/34, fallback cached layout30 bytes. This fresh debug instance must not replace the matcher's constructor-retained waiting-limit instance.

Then perform exact repeated GetAttributePointer calls (all index0), with source fallback request if null:

| Calls | Full key | Default bytes |
| --- | --- | --- |
| 1d6c,1d9c | caafaf92d3893b92 | 4 each |
| 1dcc,1dfc,1e2c,1e5c | 06ec96d7ee3f354f | 16 each |
| 1e8c,1ebc,1eec,1f1c | 23ff57fdeddc69ab | 16 each |
| 1f4c | 0fecb44c96791635 | 4 |

Destroy temporary instance1f64 before message construction. In this matched binary those pointer results are not scalar-read and there is NO drawing call in this branch. Do not invent visible rendering or collapse repeated pointer lookups into a scalar coefficient read. Immutable Vault provider can model actual pointer lookup/default resolution stages.

Separate UpdateRecognizer display-input branches: right target capture2108/value210c, branch2114. If true create right recognizer instance2134, lookup key0fecb44c96791635 with default4, then read maps78 followed77, require overlay21a8, destroyinstance21cc. No history mutation or drawing call. Then left target21d8/value21dc, branch21e4. If true create leftinstance2204, same key lookup2224/default2234, fresh maps75at2254 then74at2274, requireoverlay2278, destroy229c. Disabled right still proceeds to left gate. Failures preserve completed history/matches/messages and prevent subsequent display stages.

## Concrete integration proposal

Extend SAME GameInputPlayer with optional retained recognizer substate (two InputPatternHistory owners and ordered matcher collections). Its existing selected constructor intentionally omits these native history allocations; add explicit constructor-stage methods instead of calling a new unrelated player factory. Native PlayerInput history construction precedes GameInputSK8 creation, while AddPatterns follows PlayerInput publication in Reset; root must label existing late diagnostic construction if it cannot yet schedule this order. Do not claim a second equal-valued history proves authority.

Bind immutable original pattern collections and InputPatternVaultAttributes per native AddPatterns order, left collections then right, with exact collection key and the corresponding same history occurrence. No new matcher/math implementation is needed. Use existing matcher.try_holding and try_find_match, providing a properly initialized local Match (its existing empty() helper error0 differs enclosing sourceerror1e6 and needs explicit caller error initialization).

Minimum ports: staged history-deadzone capture/value; shared lazy gesture-display-reference capture/current value; actual local-index source; overlay existence; scoped fresh debug recognizer pointer lookup/default service; queue6 broadcaster identity/port and typed enqueue. Reuse InputPatternCvars for matcher internals. Avoid a single unconditional ignored debug callback. Missing enabled debug service must refuse at the correct stage, not return false by default. Actual disabled cvar values legitimately skip their branches.

An `update_recognizer(&mut same_player, shared_services)` entry can call player.try_value sequentially and mutate retained substate between calls. Do not hold a separate second ArcMutex lock on the same player for message identity; issue immutable occurrence capability from the actual owner and retain it in message. A stronger root wrapper should validate bound pattern/Vault/history/player occurrences before invoking selected runtime, without eagerly sampling source scalar values.

This closes command-owned mapped controls?retained recognition?actual ordered message objects. Full GameInputManager.Update additionally requires all-player flags/count filtering, map59..63 debug event support and EventSys queue flush; Actor listener construction/subscriptions/Fill and generated graph remain separate actual consumers. No false complete-frame/readiness token should be issued by the selected UpdateRecognizer leaf.

Tests needed: right map failure prevents right history; deadzonevalue failure prevents history; X/Y equality/NaN/-zero and sharedvector read; right enqueue failure blocks left mapread; held-pass completes before new-match pass perstick; first success captures BOTH shared display targets once; enabled nonlocal skipsoverlay/attrs butenqueues; localenabled repeated11lookup order beforeenqueue; late display-input resamples mapwithout history; messages preserve actual duplicate pattern/player identity; sourceflags/count outer filtering onlywhen that owner is added. Original mounted patterns and commandcodec map corpus should drive a labelled real recognizer trace, not direct synthetic jump events.
