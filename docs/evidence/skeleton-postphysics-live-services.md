# Live Skeleton collision and post-record services

2026-09-12. Source-reviewed new-only draft; not registered or compiled. Existing
service search found only physical_player_collision_corpus and
physical_player_post_records_corpus plus unit fixtures. None of the ten new
collision/post-record references exist among current definitions0..128.
Root reserved131 for preparation and132..141 in the order below;129/130 are the
previous mandatory contact/audio debug references. No existing file was edited.

| Index | Reference | Initializer | Table slot | Label | Title | Default |
|---|---|---|---|---|---|---|
| 131 | 8308e4f0 | 82f7c3c8 | 82fd4538 | kDrawSkateboardError | Draw SkateboardError | Boolean(false) |
| 132 | 8308e7c8 | 82f7bf50 | 82fd44f8 | kDrawBodyPartCollision | Draw Body Part Info | Boolean(false) |
| 133 | 8308e5b4 | 82f7bfa0 | 82fd44fc | kAllowAnyContact | Allow Any Contact | Boolean(false) |
| 134 | 8308e7ec | 82f7bff0 | 82fd4500 | kDrawCollisionUpdate | Draw Collision Update | Boolean(false) |
| 135 | 8308ea50 | 82f7c040 | 82fd4504 | kDebugAlwayCollide | Always COLLIDE | Boolean(false) |
| 136 | 8308eec4 | 82f7c090 | 82fd4508 | kShowGroundPlane | Draw Groundplane Info | Boolean(false) |
| 137 | 8308e83c | 82f7c0e0 | 82fd450c | kSmallObjectForceScalarHack | Small DMO force scalar | Float(f32::from_bits(0x3f00_0000)) |
| 138 | 8308e6e8 | 82f7c140 | 82fd4510 | kSCMaxDistOffBoard | GroundPlane max OB | Float(f32::from_bits(0x3e99_999a)) |
| 139 | 8308ed48 | 82f7d058 | 82fd45e0 | kPS_UseNormalsForProxyOffset | Use Coll Normals | Boolean(false) |
| 140 | 8308e810 | 82f7d0a8 | 82fd45e4 | kPS_ScaleHookToHipsY | Scale Y Disp | Float(f32::from_bits(0x3f00_0000)) |
| 141 | 8308e164 | 82f7c558 | 82fd454c | kPSK_DrawHookToHips | Draw HookToHips | Boolean(false) |

All labels except131/139/140/141 use Physics_Skeleton_Collision; those four use
Physics_Skeleton. Exact Definition entries for parent integration are in
skeleton-postphysics-cvar-definitions.rs.txt. Default integers shown in Float
expressions are IEEE754 bits. All three floats have GUI range0..1, and all
references have null update callbacks. Raw float initializers load f1/f2/f3 and
set r10=0 before the ctor: c0e0..c120, c140..c180, d0a8..d0e8. The decompiler's
extra buffer/string arguments are signature-rendering artifacts, not callbacks.

## Constructor order and sorting

Fresh reads of the actual table words verify initializer order independently
from host API index order. The relevant subsequence is:

130(44f4),132(44f8),133(44fc),134(4500),135(4504),136(4508),137(450c),138(4510),
131(4538),129(4540),141(454c),139(45e0),140(45e4).

Intervening actual initializers remain in native order; do not concatenate this
list at the end of the existing REGISTRATION_ORDER. Existing116 table4490 comes
before this subsequence. Existing59 (ShowDangerZone) and later trajectory /
adjustment registrations lie between141 and139. Existing93 table4628 comes
after140. Table4550 after141 is UpdatePoseBuffer, and table45e8 after140 is
ShowAnimationSkeleton. Actual constructor-table order is distinct from the
manager's section/title sorted output oracle; preserve the exact titles above
when regenerating that oracle, including Alway spelling in the source label.

## Adapter ownership and timing

input_cvar_skeleton_postphysics_live.rs is a proposed child of
input_cvar_runtime. SkeletonPostPhysicsCvars implements existing collision
services and the new preparation trait. Each call resolves the current target
again with existing try_current_boolean/float; no defaults, prefetches, cached
values or callback-local tuning copies. Existing native caller gates determine
whether conditional float/debug reads occur. require_debug_overlay resolves the
actual shared DebugOverlaySlot only at its callback.

SkeletonPostRecordServices borrows the SAME physical owner Arc and resolves
owner.reckoning().lock().reckoning().ground_normal() only when called. It never
substitutes constructor J, processed prior output, or a normal captured when the
adapter was built. Normal CalculateHookToHips captures Reckoning at45940;
proxy CalculateHookToHipsWithProxy at4d740. Both load its actual+1f0 field.
The normal branch first performs its draw read; proxy reads ground before the
normals cvar and then Y scale. The adapter adds no speculative lock/read.

Parent pair scheduling must verify the supplied physical owner against the
actual Skeleton and release any already-held Reckoning lock before invoking
post-record ground_normal. This is a borrowing constraint, not native extra
physics work. Collision and post-record adapters can borrow the same references
and overlay slot simultaneously; no publication owner is moved or replaced.

Enabled DrawBodyPartCollision currently reaches the existing explicit debug
InitSpecificBodyParts-reload boundary. Enabled post-record DrawHookToHips reaches
the existing explicit debug-stage error in the arithmetic owner. This draft does
not silently turn either pending branch into success or claim rendering parity.

## Validation

Three tests drafted: exact defaults and same-target INI updates (including
negative zero), refusal of an unlinked target, and post-record cvar reads while
the actual Reckoning mutex is held followed by the actual late normal read.
The final test compares the retained owner itself, including unknown W, and
accepts only its explicit unconstructed error if no actual Reckoning exists.
No Cargo or shared edits performed; tests await parent registration/gates.

