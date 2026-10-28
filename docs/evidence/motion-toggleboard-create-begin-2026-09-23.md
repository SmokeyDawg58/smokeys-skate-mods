# Motion ToggleBoard instance creation and Begin

The grounded raw-Y replay reached `Motion(PendingCallback { index: 12,
address: 0x82cbb1c0 })` while selecting Motion state 101 `IdleDismount`.
This callback is `Sk8::Behaviours::ToggleBoard::CreateInstance`, not a
condition. The generated graph's state 1 `OffBoard` owns behaviours 4..13;
behaviour 12 is `ToggleBoard`. State 101 inherits it through `Dismount` 32,
`OBGround` 8, and `OffBoard` 1. Native `StateUpdate_IdleDismount_101`
`0x828ef428` calls `StateUpdate_OffBoard_1` `0x828eddc8`, which updates
slot 12 at `0x828ede98`. The existing raw-Y packet
`offboard-y-motion-idle-dismount-2026-09-23.md` records Action 8,
`NewToggleOffBoardState=1`, and presence-valued `OB_Dismount`/`OffBoard`
intents. It also records expression 274 as `c16 && !c17 && !c18`.
None of this proves a completed physical or animation dismount.

IDA debug executable evidence:

- `ToggleBoard::CreateInstance` `0x82cbb1c0` calls
  `StateGraphAlloc(0x64, "unknown", 0x10)` at `0x82cbb1e4`, calls
  `ToggleBoard::Data::Data` `0x82cbb0a8` on success, returns a null instance
  on allocation failure, and increments the reference count of a successful
  instance.
- `Data::Data` initializes the refcount and zeros the 18 words at offsets
  `+0x18..+0x5c`, three `FastString<36>` values. It does not initialize
  the state at `+0x08`, floats at `+0x0c/+0x10/+0x14`, or flags at `+0x60`.
  The Rust instance retains these unwritten values as absent.
- `ToggleBoard::Begin` `0x82cbb098` writes state `0` at `+0x08`, then
  calls `ResetYawPitch` `0x82caad80`. That writes float zero to
  `+0x0c/+0x10/+0x14`, clears bit 7 of byte `+0x60`, and copies the
  three front-retrieval `FastString<36>` values from behaviour
  `+0x94..+0xd8` to instance `+0x18..+0x5c` in word order. Bit 6 is
  preserved, so its value remains unknown until a source write.
- The existing parsed `ToggleBoardNames` holds the source constructor's
  front/back retrieval names. `ToggleBoard::Update` `0x82cc2c08` is a
  separate, substantial callback. Its command and active substate branches
  remain explicit pending callbacks beyond the narrow path described below.
  The bound End address `0x825187c0` is a single `blr`.

Implementation adds one retained `ToggleBoardInstance` in the existing
generated Motion controller, allocation and Begin dispatch in its existing
lifecycle, and a focused constructor/Begin test. Confidence is **high** for
the observed writes and graph ownership, **medium** for the host's missing
allocation failure path because the replay has not exercised it. A compiled
raw-Y replay is required to confirm the next callback boundary.

The next compiled raw-Y replay reached frame 32, Motion state 101, and stopped
at `PendingStateUpdate(0x828ef428)`. IDA's `StateUpdate_IdleDismount_101`
calls `StateUpdate_OffBoard_1` once at `0x828ef444`; the latter calls
`StateUpdate_Motion_0` once at `0x828edde4`. Root slots 0..3 run first,
OffBoard slots 4..13 run next (slot 12 is `ToggleBoard`), and the leaf then
runs slots 14..17, corresponding to behaviour indices 195 `ForcePhysics`,
196/197 `CreateAttribute`, and 198 `PlayAnimation`. `OBGround` 8 and
`Dismount` 32 contribute no behaviours. The generated Motion dispatcher now
accepts exactly `[0..13, 195..198]` for `(state 101, 0x828ef428)` and retains
its existing active-set equality check and ordered callback loop. The replay
has not yet tested this new dispatch; the expected next boundary is
`ToggleBoard::Update` `0x82cc2c08` if all earlier callbacks complete.

The following compiled raw-Y replay confirmed that next boundary at frame 32:
`PendingCallback { index: 12, address: 0x82cc2c08 }`, with Action state 8,
Motion state 101, and physical Ground. The no-intent state-zero Update path is
now implemented from IDA disassembly `0x82cc2c24..0x82cc3144`:

1. Resolve the same Actor's cached `IPhysical`, `IAnimatable`,
   `IChannelAnimatable`, `ISkaterAnim`, `ISkaterMotionGraph`, `IMotionGraph`,
   and `IActor` in that order. Native checks Entity UCOM lookups against the
   cached interface pointers and asserts any mismatch, then requires all
   seven nonnull at `0x82cc2f7c..0x82cc2fcc`. The host returns typed
   missing/foreign-owner errors using existing retained interface identities.
2. Read the separate Motion intent map through `IMotionGraph`, then
   `PhysOut.mOffBoard` (`PhysOut+0x48`) through `IPhysical::GetOutput`.
   The first flag read is `mIsGrabbingObject` (`OffBoard+0x38`) at
   `0x82cc2ffc`. If false, call `HasIntent` for `OB_DropBoardIntent`
   (`0x82cc3014`), `OB_ThrowBoardIntent` (`0x82cc3084`), and
   `OB_RetrieveBoardIntent` (`0x82cc30e8`) in lazy order. Stored zero still
   counts as present. When each intent is absent, native never reads
   `mIsHoldingSkateboard` (`OffBoard+0x3f`). For a present command, native
   first tests instance state zero, then calls `IPhysical::GetOutput` again
   to read holding. Drop and Throw act only while holding; Retrieve acts
   only while not holding. A failed state or holding gate continues to the
   next intent. A true grabbing flag skips all three intents and reaches
   the final state switch.
3. After the command gates, read instance state `+0x08` at `0x82cc3130`.
   State zero calls `ResetYawPitch` at `0x82cc3144`: zero the three floats,
   clear flag bit 7, and restore front retrieval names. This preserves state
   and flag bit 6. The host retains the original Update callback as pending
   only for a command whose state and holding gates actually select its
   action, or a nonzero substate whose update remains unimplemented.

The existing physical output owner supplies both PDB fields. The retained
animation occurrence and `animation_interfaces[0..2]` supply the three
animation identities; the Motion facade and graph registrations supply the
two graph identities and `ActorGraphIntents::get` presence semantics. Focused
tests cover the native intent query order and `ResetYawPitch` preserving state
and bit 6. The subsequent raw-Y replay validated the reached branch.

The raw-Y replay then completed frame 32 through this Update branch. It
stopped on frame 33 at Motion `PendingExpression(379)`, with Action state 9,
Motion state 101, and physical Ground. Before the next build, the no-intent
Update implementation was refined to capture the validated Motion occurrence
once, release the Actor lock for the physical read, then reacquire the Actor
lock once and query the three intents from its same retained map. The
reviewed command gates now use source fallthrough and fresh physical output
reads; the actual selected command actions remain pending.

Expression 379 belongs to inactive `Mount` under `OBGround` in the loaded
Motion graph. The generated Pre switch jump-table entry 379 is
`0x82917f0c`: condition 26 `HasIntent(OB_Mount)` is read first at
`0x82917f30`. Only when true does it read condition 67
`IsRetrievingSkateboard` at `0x82917f88`; only when that is false does it
read condition 68 `IsDroppingSkateboard` at `0x82917fdc`. The result is
`c26 && !c67 && !c68`, with cached condition bits used on repeat reads.
Condition 26 and its separate Motion intent map already had a runtime owner.
Direct native activations `0x82c94ca8` and `0x82c94d18` return false when
Entity or registered `IPhysical` is absent. Otherwise they call GetOutput,
read `PhysOut.mOffBoard` at `+0x48`, then the bytes
`mIsRetrievingBoard` at OffBoard `+0x48` and `mIsDroppingBoard` at `+0x47`
respectively. Both fields already exist in the retained physical output.
The narrow implementation adds their two condition dispatches and the
ordered expression, with a focused lazy-order test. This expression branch
has not yet been compiled or replayed.
