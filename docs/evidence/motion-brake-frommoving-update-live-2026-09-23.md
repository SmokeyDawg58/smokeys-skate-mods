# Shared Brake FromMoving update in the neutral PC preview

The rebuilt release app completed 81 neutral frames, then stopped at Motion
state update `0x828f3958` in
`artifacts/controlled-live-after-ollie-120-2026-09-23.log`. The same build
completed a bounded 60-frame run and saved
`artifacts/controlled-live-after-ollie-60-2026-09-23.png`. The error contains
an address, not the selected Motion state index.

IDA names `0x828f3958` `StateUpdate_FromMoving_1164`, but states 1160, 1162,
1163, 1164, 1165 and 1166 share the address. The thunk calls `Riding_277`
at `0x828f3974`, then slot 28 `AttachIntent::Update` at `0x828f3988` and
slot 29 `PlayAnimation::Update` at `0x828f399c`. Riding expands OnGround24,
OnBoard3 and Motion0 once. Each state has the same 29-slot inherited prefix
and its own leaf PlayAnimation behavior. Adjacent state 1161 uses a different
address and is excluded.

Two independent IDA reviews found that Rust already implements all the
callbacks in this chain. Six state/address rows with exact generated active
behavior IDs were added to the existing guarded Motion update dispatcher;
no second Riding or PlayAnimation callback was introduced. The fresh release
preview exits zero at 120 and 600 completed neutral frames, with capture
`artifacts/controlled-live-after-brake-600-2026-09-23.png` and log of the
same stem. This proves the neutral selected route survives ten simulated
seconds; it does not establish active controller paths or smooth frame rate.
