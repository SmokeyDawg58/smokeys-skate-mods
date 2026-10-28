# Action mirror selection reached by the controlled live preview

The release Windows preview completed 83 selected frames, then the next queued
Actor update stopped at `Action(Update(PendingExpression(13)))` in
`artifacts/controlled-live-after-kickflip-120-2026-09-23.log`. This establishes
an evaluated Pre candidate, not which Action leaf would have won selection.

IDA's `EvaluateExpression<1>` at `0x828ff990` routes expression 13 to
`0x828ffff8`; its condition call at `0x8290001c` reads `sConditions[42]`
(`StateConditionDirect_IsMirrored`) and returns its negation. Expression 14
starts at `0x82900070`, calls the same condition at `0x82900094`, and returns
it directly. The condition's cache uses active word +4 mask `0x400` and value
word +0x18 bit 10. `EvaluateExpression<2>` at `0x82906048` sends both 13 and
14 to the unconditional true return `0x8290a90c`. Two independent IDA reviews
agreed on these arms.

The generated Rust condition 42 already delegates through retained
`action_runtime_is_mirrored` and the existing parent-tagged cache. Only the
Pre/Sustain selector arms were added in
`crates/skate2-core/src/animation/graphs/action/generated_action_runtime_selection.rs`. The focused
test checks both mirror polarities and that Sustain reads no condition. No
additional mirror state or callback was introduced.

Loaded expression 13 is used by UnMirrored state 26 and Regular leaves
156, 158, 164, 166, 168, 170, 172, 174, 176, and 178. The preview log has no
Action state observation at the failure, so it cannot identify the candidate.
