# Action grab selection42/43

Actual queued tick frame10 first reached expression42 after Action7 completion.
Pre table828ffa74 ->82900fd4 and Sustain8290612c ->82906d5c both read
condition38 then lazy39 (HasAGIntent), offsets98/9c: double grab is c38 && c39.
Adjacent LeftGrab43 Pre828ffa78 ->829010a0 uses (c38 && !c42) || (c39 && c42),
where42 is IsMirrored, offseta8. Sustain82906130 ->82906e28 uses c38 || c39,
with no mirroring read. Raw branches01154/01204 and06e84/06ee0 checked.
Existing condition-cache dispatch preserves repeated42 caching; expression
callbacks preserve short-circuit order. IsMirrored activation82c9a210 now validates Action context and reuses existing
gesture Source.resolve_animation + mirrored: same cached/registered ISkaterAnim
identity, then GetIsMirrored82cb7598 (+e4c bit40000000), retaining unknown-bit
refusal. No mapping-service dependency or copied flag arithmetic. Tests cover all boolean
combinations, both masks, callback order and reached failure. Source confidence
high; independent source/code review approves42/43 and IsMirrored; execution pending.

Action42-wheel-2 gate passes fmt, strict Clippy30.44s,4686 workspaceunits and six
docs. Original19/21 in6.51s reaches RightGrab46 at frame10. Initial gate found a
test match-same-arms lint; combined equivalent test patterns only.

Follow-up RightGrab46: Pre828ffa84->829014c0 samples39 then NOT42, otherwise38
then42; expression(c39 && !c42)||(c38 && c42). Sustain8290613c->8290708c
samples39 OR38, without mirror. Common boolean tail8290a8ec..908 freshly checked.
Tests verify allboolean combinations, eachmask, exactlazyreadorder and failure.
Registered aftergate; review/executionpending.

RightGrab46 source review approved. Action46-pause-3 gate: fmt/Clippy pass,
4691 units +6 docs, original20/21; frame10 next reaches expression47.

UpGrab47 Pre828ffa88->82901614 reads c56 HasAGIntent (+e0), otherwise c57
CurrentState (+e4); Sustain82906140->8290a90c returns true without reads.
CurrentState82c94ae0 resolves relative to tagged parent then tests whether current
state descends from the resolved state, including self. Action GetSubState829285c0
and StateHasDescendent829283b0 freshly confirm the existing Motion state_matches
helper's child/ancestor/dotted-name lookup and ancestry semantics. That helper is
shared with Action rather than duplicated; dispatch uses the held same controller
states/parent/current/name. Tests cover lazy/error/no-read paths and actual Up
state-table lookup. Independent source/code review approves; new execution pending.

Focus-count original rerun:20/22 in6.83s, both queued policies completeframes0..9
and now fail at Action49 inframe10. Down49 Pre table828ffa90->829017dc uses
c62 HasAGIntent(offsetf8) OR c63 CurrentState(offsetfc), direct tail82901894
true /8290188c false. Sustain table82906148->8290a90c is unconditional true.
Fresh raw switch slots and full pre block checked. Reuses same existing
CurrentState dispatch. Both directional expressions now share exhaustive
short-circuit/no-read/error tests. Registered; independent review pending.

Action49 gate: fmt/Clippy26.75s,4710 units+6docs pass; original20/22 in7.11s
both now reach85 atframe10. Source audit artifacts/action-directional-grab-source-
2026-09-22.md records exact85/86 slots/blocks: Pre85 c61&&c66&&c67, Pre86
c61&&c68&&c69, Sustainboth onlyc61. Registered with fullboolean/mask/lazy/error
tests over both sides, existing HasAGIntent/cache reused. Reviewpending.

Action85 gate: fmt/Clippy28.36s,4712units+6docs pass; original20/22 in7.16s
both reach Action9 atframe10. Fresh Pre9 slot828ff9f0->828ffed8 activates
HasAGIntent c43 (+ac), returns directly/cachedbit20. Sustain9 slot829060a8
->8290a90c constanttrue. Actual NewOneFoot children under17/73/74. Registered
Pre9c43/Sustaintrue with bothboolean/mask/error tests; reviewpending.
