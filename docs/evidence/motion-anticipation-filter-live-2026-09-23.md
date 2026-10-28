# Motion anticipation Filter and selected fresh Ollie replay (2026-09-23)

The original-data public facade's fresh Ollie input first entered Motion
`Anticipation` state 1030. Its behavior occurrence 1935 is
`FilterMotionGraphIntent`, even though the shared Create address `0x82ca7628`
decompiles under an `EnterSkitchingBehaviour` symbol. The Filter class vtable
at `0x823032bc` points to that Create at offset `0x2c`; the loaded occurrence
and its class vtable, rather than the shared function label, establish the
class. The Create allocates `0x14` bytes and zeros Data `+8/+c/+10`.

The loaded attributes for occurrence 1935 are input/output `Compression`,
`blendFalling=0.05`, `blendRising=0.166`, and `rampTime=0.2`. Existing typed
Filter payload and instance paths for occurrences 648/649 now also dispatch
1935. Native Begin `0x82cd5e30` resets elapsed/delta, sets the current value
to `startingValue`, and publishes the filtered output key. Native Update
`0x82cd5f40` increments elapsed first. Only its present-source branch
multiplies the selected blend by `Andale::Curve::Get` at `0x82ca8c48`; the
constructed two-point curve is `(0,0)` to `(0.2,1)`, with endpoint clamps and
single-precision divide followed by fused multiply-add. Native End
`0x82cc7388` erases the filtered key through the already-retained intent map.

The adjacent `IsAnticipating` behavior 1936 has null Create `0x82bb3da8`,
Begin `0x82c9f700`, no-op Update `0x825187c0`, and End `0x82c9f7f8`.
Begin/End call `SkaterMotionGraph::SetIsAnticipating` at `0x825fe050` with
true/false; the setter changes Motion graph flag bit `0x40000000`. The Rust
callback uses the existing grounded Motion owner and verifies the cached
`ISkaterMotionGraph` identity before that bit write.

Native state Update `0x828f2f38` for Anticipation1030 expands its ancestors,
then executes SetTurning, CreateAttribute1934, Filter1935, and
IsAnticipating1936 in slot order. Its `AnticInto/AnticTail` child state 1286
uses Update `0x828f47b8`, expanding Anticipation then PlayAnimation2367 and
the no-op SetDistComToBoard2368. The selected replay completed frame 6 after
these guarded arrays were added.

At frame 7 the replay next evaluated Pre expression 259. Native switch arm
`0x82911688` reads only cached condition 381, `WillExpire` with
`InTime=0.07` and `waitForTransitions=false`. The existing condition backend
is reused. Its implementation advanced the same replay to Pre expression
439. Arm `0x8291cfcc` is the inverse of the ordered lazy OR over cached
conditions 371..375, the five Ollie/PopShuvit anticipation intents; these
conditions were already implemented. Its implementation advanced the replay
to frame 8 and Motion state `AnticOut/AnticTail` 1183, whose Update
`0x828f3b70` executes Riding's inherited callbacks, SetTurning1413,
IsAnticipating1933's no-op Update, and PlayAnimation2182. A guarded array
for this exact active set is now present.

With that 1183 update admitted, the selected test reached frame 9 and chose
Motion state 789, `Ollie/Takeoff/FromAntic`, before stopping at its shared
Update address `0x828f20d0`. This same code address is also used by states
786, 865, and 868; IDA's function label names 868, which does not identify
the selected occurrence. The diagnostic reads after the failed tick were
Action state 52 with `Trick=1` and `Ollie=1`, Motion state 789 with `Trick=1`
and `Ollie=0`, physical `Ground`, and relative stance 0. A native audit of
the selected 789 path gives inherited slots 0..27, PlayAnimation1472 at
slot 28, and SetTrickHeight1473 at slot 29. Physical takeoff still needs a
completed frame and observed Air state.

Replay evidence: `artifacts/public-facade-fresh-ollie-after-antic259-2026-09-23.log`
stops at expression 439 on frame 7; `artifacts/public-facade-fresh-ollie-after-expr439-2026-09-23.log`
stops at state Update `0x828f3b70` on frame 8. The next rebuild after the
1183 guard first hit an unrelated in-progress `input_cvar_runtime.rs`
array-length mismatch; that edit was fixed and the later selected-state
diagnostic ran in `artifacts/public-facade-fresh-ollie-selected-state-2026-09-23.log`.
With the source-backed state789 update, Pre expression93
(`HasAnimAttribute(canBranch)` condition168) and constant-true Sustain
expression92 admitted, the fresh-session replay now completes 16 queued
visual frames. Its observed sequence is Motion1170 through step5,
Anticipation/AnticTail1286 at steps6-7, AnticOut/AnticTail1183 at step8,
then Motion Ollie/Takeoff/FromAntic789 from step9 through step15. Action
emits `Trick=1` and `Ollie=1` at step9. Physical state remains Ground at
every step, so this is a source-selected Ollie Motion proof, not a physical
launch proof. See
`artifacts/public-facade-fresh-ollie-after-sustain92-2026-09-23.log`.

IDA's transition index indirection matters: the Ollie188 ancestor's
expression93 transition is its second priority-zero entry, after an
unconditional GrabsTweaks target. Earlier direct reads of the transition
pool would have assigned the wrong owner to expression93. The native Sustain
expression92 returns true without checking Ollie intent, letting the
selected Motion ancestry persist after input release.

Extending the same fresh replay to 60 requested ticks shows physical
GroundAnimation103 at completed steps 16 and 17. The next tick selects Motion
state516 and stops at its still-unimplemented update `0x828f0d30` while
physical state remains GroundAnimation. This confirms the native animated
skateboard gate and Ground100→103 transition are active in the selected
runtime; StartJump, board velocity and Air200 remain unverified. See
`artifacts/public-facade-fresh-ollie-60-2026-09-23.log`.

After admitting state516's exact 29-slot Ollie/LeftGround update, step18
completes with its authored `B_OLLIE_A` animation and a visual. The processed
StartJump bit `0x00400000` is known and clear, and GroundAnimation reports
`jump_info.do_jump=false` and `jumped_this_frame=false` through that step.
Frame19 stops in Motion Pre expression11 before its physical stage. This
does not yet show a failed launch; the authored `jump` clip cue may occur
later. See `artifacts/public-facade-fresh-ollie-after-leftground516-2026-09-23.log`.

Native Pre expression11 at `0x8290b598` is one cached WillExpire condition28
with `InTime=0.05`; the condition runtime already existed. With that arm
admitted, the same 60-frame original-data replay completes every queued
visual frame. At step21 processed StartJump `0x00400000` is set,
GroundAnimation reports `jump_info.do_jump=true` and
`jumped_this_frame=true`; step22 enters physical Air200, and step55 is back
on Ground100 after the landing sequence. The selected Motion path passes
Ollie FromAntic789, LeftGround516 and Air Default691. This proves a
scripted source-data Ollie launch, air and landing in the host path. It does
not yet compare numeric trajectory, pose or input latency against the native
build or establish hands-on controller behavior. See
`artifacts/public-facade-fresh-ollie-after-expr11-2026-09-23.log`.
