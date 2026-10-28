# SequenceAnim reached by the authored Ollie preview

The original-data public facade reaches a Motion PlayAnimation request on the
first chronological Ollie right-stick sample. Before this patch it stopped at
`AnimationBody(UnsupportedDispatch("SequenceAnim"))` in
`artifacts/public-facade-ollie-transition103-2026-09-23.log`. With the native
Sequence producer integrated, the same tick passed that dispatch and stopped
later at Motion state Update `0x828f3170`, recorded in
`artifacts/public-facade-ollie-sequence-2026-09-23.log`. The Ollie match and
release are not yet proven by this replay.

IDA `Animatable::SequenceAnim` `0x82d7ae30` calls shared `TransitionAnim`
`0x82d7aa68` with zero transition length and current-frame flags. No base
animation falls back to `PlayAnim`; otherwise the shared capacity prune,
NewTree/GetAnimTree, name/speed and positive offset stores run before the
Sequence branch at `0x82d7ac60`. NewSequence `0x82d7a3d0` appends to the same
time-ordered transition list as blends. With a younger transition and nonzero
placement, the new Sequence replaces that transition's TO child; otherwise
it becomes the body base. A missing tree clears both stores and reports false.

The Sequence's `playingNew` bit is the native completion predicate
(`0x826b65c0`). `AddTime` `0x82d6dbd8` advances FROM with local properties
until expiry and passes only positive overrun to TO; subsequent time advances
TO. `Eval` `0x82d6dd00` and `EvalBlendPose` `0x82d6dd38` tail-call the active
child without setting a Sequence evaluated bit. Length, time and SetTime
forward to TO. Attribute getters select the active child; before completion
SetAttributes calls TO then FROM. Release returns Sequence to its own free
list while removing it from the shared transition order.

Rust now uses a tagged blend/sequence transition handle and Sequence body
reference through release, time, evaluation, attribute and expiry consumers.
This reuses the retained animation owner and existing tree allocator; no
second animation system or Blend substitute was introduced. Independent IDA
review found no blocking parity issue. The mounted lifecycle corpus checks
basic completion, mixed placement/release and missing-name Clear and passes
in `artifacts/original-animation-sequence-lifecycle-repaired-2026-09-23.log`.
After the source-backed Takeoff update dispatch was admitted, the scripted
public Ollie replay also passes in
`artifacts/public-facade-ollie-takeoff-2026-09-23.log`.
Later read-only Motion observation shows this combined Kickflip-then-Ollie
replay remains in Kickflip Motion state 1078 and then 792 while Action emits
the Ollie intent. It therefore proves the Sequence producer and Action
input route, not a fresh Ollie Motion transition or physical jump. A separate
fresh-session Ollie replay is being used to exercise those later boundaries.
