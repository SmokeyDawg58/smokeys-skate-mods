# Actor animation physics publication and selected wipeout attribute

Read-only source audit and implementation, 2026-09-12. Confidence **exact**
for the bounded copies, native flag branch and order below; no complete Actor
tick, live input-produced bail or reference per-tick numerical parity claim.

## Source proof

Fresh IDA decompilation and raw instructions from the debug reference:

| Symbol | Address | Relevant source behavior |
| --- | --- | --- |
| `Sk8::Actor::SetUpPhysics(float)` | `825ff6c0` | Reset input at `f740`; SkaterAnim.GetPhysUpdateData at `f75c`; Motion.GetPhysUpdateData at `f778`; Motion-intent map assignment `f79c`; clear attribute end `f7ac`; MG AddAttrs `f7cc`; animation AddAttrs `f7f0`; further Actor/backend data then physical SetPhysUpdateData `f978` |
| `Sk8::Skater::SkaterMotionGraph::GetPhysUpdateData` | `825f7e50` | Copy trick `7e70`, reset source `7e78`; copy gesture `7e90`, set source enum `35`/false `7ea0/7ea8`; copy wipeout XY `7eb0`, reset source positive zero `7ebc/7ec0` |
| `AnimOutPhysInAtts::AddAttrs(MgAttributes const&)` | `825fdc98` | Native source vector order; value first four payload bytes, begin/end `-1.0`, flags `6`, type `0`, sequence `-1`, same five packed name words |
| `AnimOutPhysInAtts::AddAttrs(Andale::Attributes const&)` | `825fdbf0` | Source begin/end capture and ordered Attribute assignment; fixed capacity 120, no overflow allocator |
| `Andale::Attribute::operator=` | `825f7658` | Metadata copied first; scalar four payload bytes `7730`, vector sixteen `7720/7724`, contact twenty-four `76f0..7718`; other types leave payload alone |
| fixed FastString36-to-float map `operator=` | `825fd730` | Free destination nodes `d75c`, count zero `d768`, iterate source and insert `d78c/d7a0`; collision chains must be reconstructed instead of cloned |
| `Skeleton::ProcessAnimAttributes` | `82b59de8` | Type-zero branch compares WipeoutAtt at `82b5ada8`; match reads SAME processed `+8c4` at `adb8`, ORs `0x40000` at `adbc`, stores at `adc0` |
| `Sk8::WipeoutAtt` initializer | `82f82288` | Assigns literal `Wipeout` to global `830a4318`; unloaded static global zero bytes are not its runtime name |

Raw assignment control is `lbz` type, `extsb`, then unsigned compare to one.
Thus only type zero reaches scalar assignment; negative types do not. The
Wipeout attribute branch has no value, active-status or sequence-ID condition.
Its OR marks only the selected bit known when neighboring source bits are
unrepresented. Reset later restores the real shared `+8c4` word to `0x2000`.

## Current implementation and ownership

`actor_animation_physics_publication.rs` is registered under the existing
Actor-owned AnimOutPhysIn module. Its Actor method reads this Actor's retained
constructed Motion graph, its main intents, animation tree attributes and
published AnimOutPhysIn allocation. It drains Motion payloads, rebuilds the
destination map, resets the destination attribute count, then appends Motion
and animation records in source order. A late refusal preserves all completed
source effects. It does not call Reset or the preceding SkaterAnim output
producer, and does not invoke the later physical SetPhysUpdateData.

The main-map selection is verified against PDB vtables: base Motion vtable
offset16 is GetMotionGraphIntents, used at825ff78c. GetFilteredMotionGraphIntents
is a different method at ISkaterMotionGraph vtable offset8. Both registered
Motion interface occurrences must match the retained graph; the published
animation must match its ISkaterAnim registry occurrence. These are safe-host
ownership checks, not newly invented source lookup calls. The method is an
outer Actor operation: it performs no animation callbacks or ACS/service
calls under locks, and releases the Actor prefix before the tree append phase.

Occupied attribute slots use safe optional values; unoccupied storage is not
read. Payload bytes have individual known/unknown state. Clearing the vector
end retains old slot storage: a subsequent scalar assignment changes its four
payload bytes while preserving the previous remaining twenty bytes. Initially
unknown bytes remain unknown. The prior unsafe-to-read MaybeUninit schema
had no production readers; this change introduces no fabricated zeros.

The current Andale runtime tree projection has no sequence ID. The source
assignment copies this field, so transport marks it unknown and overwrites any
previous known sequence value. No format schema was widened. Consumers needing
sequence information must establish its original tree producer first. The
selected Wipeout consumer does not read it.

`skeleton_processed_animation_wipeout.rs` lives under the existing shared
processed jump-event word owner. The Actor bridge obtains the genuinely cached
physical allocation, then reads one selected record from the same Actor input
and changes that physical owner's same Processed object. It takes a record
index, not a caller-supplied synthetic attribute or replacement physics owner.
This remains a selected per-record continuation: it does not skip or claim the
other per-record branches or the surrounding ProcessData/Update1 effects.

## Outer schedule and remaining dependencies

Fresh source xrefs show serial Actor.StartAnimUpdatePhase1 `825ff988` calls
GenerateActionGraphIntents `825ffc50` before UpdatePhase1 `825ffc5c`. Phase1
`825f41a8` calls Action controller Update `825f4374`, clears MG attributes
`825f43a0`, then Motion controller Update `825f43cc`. Phase3 `825f43e0`
conditionally calls AddTime/ProcessTreeAttrs before ToggleSkaterStates and
body dispatch. SkaterManager owns these phases. Its UpdatePhysics `825ffd98`
calls Actor.SetUpPhysics at `825ffed4`. Update1_ProcessInput `82e01f98`
is a separate physical virtual phase with Reset/prior-PhysOut copies, toolkit,
Skeleton.ProcessData and later trajectory/grind work. Chaining selected
helpers does not establish this complete frame schedule.

Actual production callers before this change: GenerateActionGraphIntents is
called only in unit tests and input corpus; the actual pair Update2 bridge is
called only from corpus and a selected transition bridge, itself corpus-driven.
No normal executable Actor frame loop was identified. This transport's normal
schedule likewise still needs root integration.

Live bail is not merely request6: HeavyLanding request6 is produced by actual
post-physics landing detectors. The animation WipeoutAtt path sets processed
bit18, which the state selector consumes. It still requires actual Action/Motion
evaluation, Motion attributes or animation tree processing, complete prior
SkaterAnim.GetPhysUpdateData and Update1/ProcessData scheduling. The cached
physical pointer additionally requires the independently assigned Skeleton
constructor/Init closure. Broader DMO work remains parked.

## Focused validation

Tests cover native partial-payload copy/reuse including NaN and signed zero,
unknown sequence/type behavior, capacity failure prefix, retained Motion
payload drain/reset, fixed-map collision traversal reinsert order, exact
Wipeout type/name dispatch with arbitrary status and unknown payload, retained
neighbor flags and Reset. Root owns original-data mounting, independent review
and the complete strict integration gate. Focused run results are reported in
the agent handoff; no earlier checkpoint is attributed to this batch.

## Root continuous integration, September12

Original9 executes two actual Action/Motion updates, phase2 SetAttributes and phase3 AddTime/ProcessTreeAttrs/Toggle/Dispatch with same startup services, then Sync, full GetPhysUpdateData, Motion transport, retained physical pointer binding and ProcessAnimOut. Workspace9 passes3,734units+6docs; core original9 all17 and formats exceptional61frames pass. Later source ClearMgAttributes and full external/settings suffix are wired but require fresh gates. Root independently reviewed825ff6c0, AI assignment825f0f60, ProcessAnimOut82e00030 and native setter82dfe688. Graph reviewer corrected premature GameInfo service lookup to preserve earlier suffix stores. AI assignment preserves destination low25bits while copying seven source highbits in order. No complete Update1 or playable Actor frame is implied.

Root raw vtable correction: SetUpPhysics825ff708 loads concrete SkaterAnim vtable+88, call825ff710. Actual vtable82302d28+88 bytes825187c0 resolve to shared BLR4e800020. Thus this virtual call is a proven no-op; it is not an additional body ACS Sync. The manager's earlier named ACS synchronization remains required. New try_set_up_physics combines source Reset/GetPhys/Motion/attributes and retained external/settings suffix through SetPhysUpdateData. Earlier notes treating710 as a missing Sync continuation are superseded. Motion interface base adjustment825ff6e0 is -A78; ClearMgAttributes/GetMgAttributes relative+A64 therefore addresses base14dc, not the earlier speculativeFA0.
