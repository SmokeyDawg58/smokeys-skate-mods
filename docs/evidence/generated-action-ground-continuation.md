# Grounded Action continuation draft

Unregistered source-backed draft. No Cargo run or original grounded graph
execution is claimed. Existing registered runtime/corpus files remain unchanged.

The original grounded path is Action0 -> OnBoardGeneral1 -> OnBoard6 ->
OnGround19 (expression56: filtered Ground74 && !ToggleOffBoard75) -> Grabbing45
-> DBLGrab80. Existing execution first stops at expression57 before examining
grab input. New ground-selection child covers actual Pre828ff990 and
Sustain82906048 expressions18/19,39/40,57..69,87,102, with original lazy condition
calls and existing shared condition cache. Pre58 checks AnticMag>0.9 then its
presence; Sustain58 checks presence only. Kickturn102 has distinct Pre and
Sustain speed/slope conditions and preserves repeated cached fakie calls.
This child must be delegated to only by the old evaluator's unknown-case branch;
it does not replace the already audited Default cases or fabricate conditions.

The new exact generated dispatch admits IdleTurning94 only: update828ed788
tailcalls OnGround828ed1f0, then OnBoard828ecfe0 and root828ecbc0. Packed Update
slots0..30 correspond to occurrences0..10,42..49,68..79. No generic ancestor
walk or arbitrary active-set order replaces these generated calls. Antic83 and
grab leaves still require their own selected dispatch/constant-intent callbacks.

## Common callbacks

BodyFlippingSignal occurrence45 allocates2c/alignment16 at82caec70. Data ctor
82caebd8 clears timer+8, FastString+c..20, flag+24 and air timer+28. Begin82c94840
loads live tuning and writes timer+8 before its second lookup/assertion and
air timer+28. End is the actual shared BLR825187c0.

Update82cd5cd8 always calls BodyFlipWindow82cc2108, then TakeOffWindow82c990d8,
then obtains the actual Motion map. Body window searches FrontFlip first and
BackFlip only on absence, resetting time and copying the selected name. Their
global initializers82f81b68/82f81b80 establish literals FrontFlip/BackFlip.
When absent, it reads the extension, samples controller dt, performs dt+oldtime,
subtracts limit and FSEL-clamps to the limit. A fresh extension read follows
even after an immediate gesture reset. Its final strict LT rejects unordered.

Takeoff uses the actual filtered state word: Air2 advances the air timer with
its own live limit/dt and FSEL; any other state resets it to positive zero.
Raw4099006c at82c99268 skips on NOT-GT, including unordered, before the final
fresh limit read. Final41980008 at82c992d0 admits strict LT only. If both windows
are true, insert retained flip-name=1; otherwise erase FrontFlip then BackFlip.
Prior timer/name stores survive later attribute, physical, map or insert errors.

CreateMGTimeIntentFromAGIntent occurrence48 uses shared allocator82712148,
size0xc/alignment16, float+8 initialized positive zero. Begin is BLR825187c0.
Update82cd5928 resolves Action then Motion, obtains destination before source,
and tests AG intent *presence*, including zero/NaN values. Present: late
controller dt+oldtime store then destination insert. Absent: reset time then
erase. End82cc2028 erases destination only and does not reset instance time.
The adapter passes dt from the already locked SAME controller to avoid a
recursive lock or unrelated clock. Existing Actor context issuance and graph
UCOM identity checks, actual AG/MG maps and allocation errors are reused.

## Actual global binding prerequisite

No existing retained projection for GlobalAttributeCollection+128 was found.
The new BodyFlipAttributes owner maps precisely that member: allocation10 at
828b2cec; generated constructor828b1590 invoked at828b2d10 with collection
3fc308e69aea6385; class90c3decf0105631e; cached layout5c0 initialized by the
generated ctor. This is a selected global member, not full singleton startup.
The root must retain it once from the actual session Vault and pass its presence
to the callback; no missing-owner fallback is admitted. Reads remain live
GetAttributePointer(index0), DefaultDataArea4 only for a genuinely absent key:
48cfd7fd67dc55a2 (extend_bodyflip_gesture) and0307546c31db9472 (takeoff window).
Helper82c8ad18 uses the same live key lookup, not a cached-layout scalar.

## Integration proposal and remaining boundaries

New files only: generated_action_runtime_ground_selection.rs,
generated_action_runtime_ground_dispatch.rs, actor_action_ground_callbacks.rs,
actor_action_ground_runtime.rs and focused callback tests. Register ground
selection/dispatch below generated_action_runtime; register ground callbacks
below actor_action_motion_intent_bridge (runtime is its child). Add explicit
GeneratedActionInstance variants for BodyFlippingSignal and TimeIntent,
allocator sizes2c/c, token mapping, Begin/Update/End dispatch and separate
GroundActionRuntimeError wrapping. Extend supported Update dispatch to exact
IdleTurning94 through the new method. Do not mount unsupported grabbed/Antic
states as generic successful callbacks.

Allocation-null slots must preserve their original late dereference boundaries;
the new nonnull instance methods do not authorize replacing null allocation
with a fresh instance. The root's final shared dispatch needs explicit checked
null handling when those source operations are selected.

Ground-neutral selection next encounters condition96 DisablePushBrake82c9a8f0
in expression67. It is not a constant: fresh physical Ground normal+50.Y,
live Global+anim_motion_pushing tuning, source ASin*57.29578, 90-angle test,
then a fresh output call and Skeleton+242 byte. Thus this draft alone does not
yet execute the grounded graph to completion; condition96 and reachable
speed/slope/fakie predicates must use real retained producers. The existing
Default two ticks and codec-to-Actor intent corpus do not cover this route.

Eight focused draft tests discriminate grab short circuits, Sustain differences,
repeated fakie cache calls, first/second tuning failure prefixes, both-window
execution, front-before-back precedence, unordered takeoff, late Motion error,
and timer-before-insert/reset-before-erase behavior. Rustfmt completed; tests
remain unregistered and unrun pending independent review and parent gate.

## Root review in progress, September12

Root inspected fresh Hex-Rays Pre/Sustain cases18/19/20,39/40,57..69,87,102,
including original lazy order, Sustain58 presence-only and Sustain20 defaulttrue.
Read complete BodyFlipWindow/TakeOffWindow/TimeIntent decompilation plus fresh
window clamp/final-compare raw and Update's unconditional two-window calls.
These inspected algorithms agree. Owner/null lifecycle/shared integration and
original graph execution remain unaccepted until the complete batch review.

Root registration in progress: actual shared ActionRuntimeServices retain Global+128/+13c instances from the existing Vault and are installed as host capabilities, never native Actor fields or per-call rebindings. Root fresh raw confirms ctor2cec/2e54 keys/slots, DisablePushaac0 decoded VA57 (IDA renders89), late slope pointer read and unconditional secondGetPhysOut. Independent reviewer confirmed exact31/34slot dispatch. Clippy65 passes before the new controlled graph-boundary corpus; workspace41 compiling. Corpus intentionally injects FilteredGround and AG anticipation at a test boundary, does not claim full-frame or gesture producer execution.
