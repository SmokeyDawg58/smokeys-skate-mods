# Grounded Motion selection: original skitch predicates

2026-09-19 bounded unregistered draft. Confidence **high** for raw source
predicate/order and selected immutable Vault binding. No Cargo or live
Actor-next-tick execution by this author; no sustained-frame parity claim.

`actor_motion_runtime_skitch_conditions.rs` is a proposed child
`actor_construction::motion_runtime_session::skitch_conditions`. Root owns
registration, selected shared bundle integration, dispatch and corpus gates.

## Reached branch and original dispatch

After actual Filtered.Ground, with no earlier Trick/runout/handplant selection,
Motion0/OnBoard3/OnGround24 visits EnterSkitching87/Into270. Its expression201
is the first currently missing expression. No Riding leaf is assumed.

Fresh raw switch table8290aa90 gives Pre201 at8290fee0, loading condition289
from condition-array+484 and calling StateConditionDirect_IsEnteringSkitch
82ca7600 at8290ff04. Pre203 at8290ffb8 loads condition293 from+494 and calls
StateConditionDirect_IsSkitching82ca75f0 at8290ffdc. The existing generated
condition cache and parent write must remain the only owners of those stores.

Root selector additions: `201 => c(289)?`, `203 => c(293)?`. Both predicates
already have loaded declarations and are Motion `ConditionPayload::Plain`
variants `PlainCondition::IsEnteringSkitch` / `PlainCondition::IsSkitching`
(neither is a shared Action class). Do not manufacture false values to pass
the selection.

## IsEnteringSkitch82ca70b8

Resolves actual cached IPhysical and verifies UCOM identity before the global.
At7180 captures GlobalAttributeCollection, asserts it, loads member+190 at71ac.
Generic GetAttributePointer71c4 uses keybdfd21f2a96828fd, element0. Absent field
uses DefaultDataArea4; missing global allocation is not that default.

71dc reads threshold BEFORE fresh IPhysical.GetPhysOut virtual+28 at71ec.
71f0 loads actual Ground pointer from PhysOut+20,71f8 reads timeToSkitch+104.
Root raw-byte correction:7204 instruction41980010 is BO12/BI24 (LT), rejecting
ordered negative time.7210 instruction40990008 is BO4/BI25 (!GT), accepting
unordered as well as ordered <=threshold. NaN time succeeds; NaN threshold
succeeds unless the earlier time-negative branch rejects. The first author
and independent review misread the BLE alias as ordered LE; raw CR bits
supersede that interpretation. The regression now covers both unordered
inputs and negative-time with unordered threshold.

Actual Ground.Update prefix already stores time_to_skitch=-1 and existing
Ground.Fill copies it to this output. That does not remove the preceding global
lookup. No DMO admission or query substitution is needed for these predicates.

## IsSkitching82ca6e08

Same cached IPhysical/UCOM validation. Fresh GetPhysOut at6ee0, actual State
pointer PhysOut+1c at6ee4, State.physics_state+10 at6ee8. Compare to104 via
subtract/count-leading-zero. This is not a Skitch output flag.

Both draft adapters use existing same-owner accessors and preserve source
component checks. Native does not read the output write latch; existing
try_ground_output/try_state_output check actual constructed components only.

## Global identity and exact constructor origin

This slot does NOT use a generated cached-layout wrapper. Global constructor
828b1608 allocates generic Instance size10 at828b3470. Raw3480..3494 establish
class07d39b41aacee50f, with r28 defaultd7edbd362d7d2152 established at1630..1644.
FindCollection3498 precedes generic Instance constructor34a8 (msgPort0), and
34b4 publishes Global+190. Allocation-null stores NULL through34b0/34b4.
Do not add an invented cached-layout requirement.

Existing ActionRuntimeServices in actor_action_runtime_session.rs is explicitly
a shared selected GlobalAttributeCollection bundle (+128 BodyFlip,+13c Pushing).
It is bound once from the runtime Vault and installed as one Arc in the Actor
using try_install_action_services. Extend this SAME bundle with
`skitching: SkitchMotionAttributes`, initialized by `bind(vault)`. Keep it shared
across participating Actors; its use by Motion does not require another copy.
The draft retains the exact Vault Arc and one Instance and checks Vault identity
against the resolved PhysicalPlayerAssemblyOwner.inputs.vault at the reached
global stage. It never reconstructs the Instance during a predicate.

When dispatching289, borrow the optional installed bundle through the existing
action_runtime_services accessor and pass its skitching reference. Missing
bundle remains None until the adapter reaches its global requirement after
IPhysical resolution. The new attribute error maps to existing
MotionUpdateError::Load(MotionLoadError::AttributeRuntime); no error variant is
required. Missing generic collection/field follows current immutable reader
semantics, consistent with existing selected global adapters. Allocation failure
parity and mutable global replacement remain outside this selected host seam.

Root integration should then call the actual next Motion update after completed
conditioned output, through existing ClearMgAttributes/try_update_motion and
existing lifecycle. This packet does not implement subsequent brake/kickturn/
idle expressions or any OnBoard callback. No callbacks should be bypassed.

One focused test calls the production interval helper for exact endpoints,
negative zero, negative thresholds, infinities and NaNs. Actual-owner identity,
missing global ordering and corpus validation remain root integration work.
