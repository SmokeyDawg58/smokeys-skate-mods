# September 25 controller batch ? IN PROGRESS

Solo work; source only (no compile, replay or launch).
No complete-batch or runtime parity claim.

Implemented, source reviewed, rustfmt parsed (not type checked):
- Motion Pre/Sustain 369,245,419,112,239. Tables8290aa90/8291e254.
  Pre36982916ee4/shared82912464=(47&&46&&!48)||(49&&46&&48);
  245829110d4=!352;4198291ac08=59&&(9||87);1128290db98=192;
  23982910e6c=344. Sustain369/419/112 defaulttrue829221c8;
  24582920180=!352;23982920060=344 (raw offsets580/560 confirmed).
- FilterMotionGraphIntent642 now reuses existing Create/Begin/Update/End implementation,
  including authored acceleration/velocity clamps and removal on End.
- ScoringGrabs2174 uses existing scoring publication. Non-tweaked gnd_fsgrab does
  not require useXY/invertY: native82ccb910 tests these only inside hasTweak.
- SetManualOutTimer1930 native Begin/Update BLR; End82ca5310 ->825fe170 writes
  authored0.1 timer on actual cached Motion owner.
- PowerSlideManualAtt1405 Begin/End BLR; Update82cc8248 reads ManualIntent
  (initializer82f81ad8 -> string822f8940="Manual"), emits balance through
  ISkaterMotionGraph vtable+10 AddMgAttribute. Raw82cc83c8=40980030 means
  skip when LT clear, including NaN. Implementation uses value<0, no dedup.
- TransitionBlend82d6e198 bit40000000 now reaches buffered ChannelBlend command
  and executor. AnimCommandSystem82a2cb88 chooses first operand channel W when
  true, second when false, without swapping the interpolation operands.
- Ground RB: active branch evaluates ShouldLaunch82dbd158 after native manager
  Reset. Speed key63303ca6c611dc79, slope keyd1622917f6dd215f, debug key
  a4928ce79e99ab4f verified from raw64bit assembly. Native requires sufficient
  speed, positive vertical velocity, slope above threshold, abs(normalY)<=0.95.
  Ineligible launches return with IK preserved (different from inactive Reset).
  New live cvar522 kHandPlantDisplayDebug initializer82f96070/table82fd5c18.
  COMMITTED handplant and eligible ramp selector/IK remain explicitly pending.

Outstanding before claiming the task complete:
- Intermittent push after180turn: NOT FIXED. Existing corrected held-push keys,
  speed blending, simulation60Hz and standing-turn changes must be preserved.
- Landing/camera high-fall behavior: NOT FIXED. Important static finding:
  live code NEVER writes Air.is_traj_valid=true or its collision trajectory fields.
  Native PhysState_KnownAir.FillPhysOut82d8c348 publishes these. Remaining-state
  KnownAir constructor exists but CurrentPhysicalState lacks KnownAir (201).
  SystemLogic selects201 when processed8c4 bit400 set, but runtime cannot enter it.
  Existing selector completion/scoring exists and runs in ground_pre_solve Update1;
  do not mistake old process_input_selector.rs pre-scoring boundary for live path.
  Investigate why normal ollies never yield ready201 (prediction timing/geometry/
  NearLanding path), then implement real KnownAir lifecycle, not a forged valid flag.
  Native KnownAir methods: Enter82d9f828, InitTrajectoryInfo82d992a0,
  Update82d99820, Post82d99b78, Exit82d99728, Follow82d8c078,
  RestoreVelocity82d8b928, Spin82d8bbd0, Flip82d84400,
  upside-down wipeout82d8c1a0. Existing Skeleton.UpdateKnownAir is NOT this state.

Formatting passed on changed Rust files. No compile/tests/runtime started.

September25 build follow-up: compilation. Fixed the
new RB code's vector call (try_length accepts SkeletonRootVector directly) and
two Ground fields accesses (ground.fields.*). `cargo build --release -p
skate2-pc -j 1` succeeded, exit0, optimized build4m09s; core223 warnings and app4
warnings. No runtime launched. KnownAir and intermittent pushing remain unresolved.
