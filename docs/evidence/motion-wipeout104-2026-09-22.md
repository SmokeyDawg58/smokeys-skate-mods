# Wipeout104 source review and integration

Root independently reviewed author packet artifacts/motion-wipeout-2026-09-22,
fresh CreateInstance82cb0098, Begin82cafe80 and Update82cc5c88 decompilation,
and numeric raw82cc619c..62d8. Allocation initializes all payload fields;
Begin count reset precedes resolution and retains unused deltaLean. Update
preserves five initial scalar reads, separate lean/twist PhysOut calls, mirroring,
FSEL clamps, rounded products then fused additions, unsigned update-two gate,
three late scalar reads after GetIntents and final controlled/payload/count order.
Raw NOT-GT/NOT-LT branch encodings establish unordered skipping independently
from FSEL's unordered negative arm. The actual source field is Skeleton+1f0/1f4.

The adapter reuses the published Actor physical/animation/Motion interfaces,
retained global plain anim_wipeout instance from the same Vault, original
animatable attribute writer and Motion wipeout payload consumed by physics.
No cached tuning or duplicate state owner. Five focused kernel tests authored.

Root also independently read BraceForImpact98 thunk828ef378, Air29 thunk828ee8e8
and WipeOut4 parent thunk. State98/99 have exact ordered active IDs
0,1,2,3,33,34,35,104,105,106,191/192. Each parent expands once; existing callbacks
are reused. Both exact state/address arms are registered along with lifecycle.

Validation: fmt-wipeout104-2026-09-22-2 and strict clippy-wipeout104-2026-09-22-2
pass (48.13s); workspace passes4414 core+210 formats+45 app+3 symbols=4672 units
plus6 docs. First lint pass needed two documentation backticks and replacement
of an unused too-many-lines expectation with a narrow assign-op expectation
preserving native clamp-result+old-lean operand order.

Original-wipeout104-2026-09-22-2 is18/20 in6.34s. Bail frame2 now completes Actor
animation and reaches SystemLogic UnsupportedCurrentState300; neutral still
completes3frames and later reaches the isolated nonroot Skeleton contact
assertion. Native reference traces and the playable app remain outstanding.
