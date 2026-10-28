# Air selection and retained offline input checkpoint

Root independent source/code review covers Motion269 (Pre82911a60 and
Sustain82920528 -> condition403 WipeoutTimeToLand82c9d2c8), Air200 selection
CalcSuggestedState82ddbd58/82ddc420..51c, and the queue packet described in
selected-command-queue-audit-2026-09-22.md. Original tuning remains live Vault
data; actual IPhysical Animation+90 supplies time to land. Air selection keeps
wipeout/hippy priority, lazy special-mode reads, ordered timer>minimum including
unordered skip, same Skeleton animation landing query and explicit pending
grind investigation. New destinations remain explicit pending transitions.

Queue integration preserves same dispatcher/gameplay/map identities, copied
payloads, mailbox send/get commit order, actual RNG mutation before each normal
command, shared source counters and failure prefixes. Recognition is a separate
same-owner stage so it cannot accidentally redispatch the batch. Production
GameModule admission/scheduling is still pending; SelectedPlayerTick's controlled
immediate path was unchanged by this packet.

Validation: fmt-air-queue-2026-09-22-2 and strict clippy-air-queue-2026-09-22-2
pass (52.26s). Workspace passes4409 core+210 formats+45 app+3 symbols=4667 units
and6 docs. First lint attempt found five nonsemantic issues: MSRV-incompatible
cast_unsigned replaced with bit-preserving byte conversion, borrowed large
command and dispatch result, Copy sample request, and documentation backticks.

Original-air-queue-2026-09-22-2:18/20 in6.35s. Neutral completes all3 selected
frames with contact counts0/38/44,28 joints and51 drives. Bail remains at
Wipeout104 CreateInstance82cb0098 on frame2. The neutral aggregate now reaches a
later isolated fixture and fails its nonroot Skeleton contact assertion at
physical_player_collision_corpus.rs:304. No assertion was weakened; diagnosis is
pending. These controlled original-data checks are not native deterministic
reference traces or a playable app.
