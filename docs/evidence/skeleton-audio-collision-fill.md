# Skeleton collision audio-data publication

Fresh raw/PDB review, 2026-09-12. Source
`SkeletonCollision::FillPhysOut_AudioCollisionInfo` **82b3efc0..82b3f8d4** is
unconditionally called by Skeleton.Fill at59714. It produces retained physics
output data and mutates simulation accumulators; postponing sound playback does
not authorize skipping it.

New, unregistered drafts:

- `skeleton_collision_fill_audio.rs` / tests, proposed `fill_audio` child of
  existing `skeleton_collision_data`.
- `physical_player_phys_out_skeleton_audio.rs`, proposed output `construct`
  child; captures a single mutable borrow of the actual Collision output record.
- `skeleton_collision_audio_attributes.rs`, proposed child of existing
  `skeleton_collision_update_attributes`; reuses its retained global+10c
  instance, with no per-frame bind or copied tuning values.

No shared edits, parent registrations or Cargo were run. Six tests are saved
but uncompiled/unrun. Parent reserved cvar129 for IK ContactInfo and **130** for
this leaf's AudioCollisionInfo. No cvar registry edits were made here.

## Attribute and output capture order

All four attributes use actual Global+10c physics_collision.default, index0,
missing pointer DefaultDataArea4. Keys in source order:

| Register | Key | Native use |
|---|---|---|
| f27 | 06c10e1f5e1b4730 | accumulation lifetime/divisor |
| f25 | ad7ecc44569ebef9 | contact accumulation scalar |
| f26 | fbf5cd7734e8ae5a | slide accumulation scalar |
| f24 | 1430bd50f0a33475 | contact force scalar |

Each next global pointer capture precedes the preceding attribute float load:
f04c before f050, f0a4 before f0a8, f0fc before f100. Fourth pointer lookupf140
is followed by output write-guard samplef154, fourth float loadf158, then guard
check. Only then does f17c capture Collision output and f18c its nested+30
SkeletonCollisionInfo pointer. No subsequent output write guards occur in this
leaf. The GAT output-record borrow models that continuing pointer without
duplicating its allocation or accidentally rechecking a latch per field.

The existing CollisionUpdateAttributes provider retains global skeleton+ec and
collision+10c separately. New attribute child clones only an immutable captured
Instance view and uses existing exact float/default/error handling. Parent must
provide this retained owner; an absent global owner remains an integration
error, not permission to bind an unrelated default instance on demand. Native
editor/singleton replacement races remain outside that existing provider policy.

## Headers and 24 bones

Publish actual Collision unrideable_contact, compliant_active and
opposing_contacts_on_plane bytes to unrideable/partial-ragdoll/impaled output.
Then for part0..23:

1. Read same bone main normal, State.PhysRecord velocity delta, compute dot3.
   State-relative delta base is **f10**, not c10 position or d90 velocity.
2. Read same bone sub-normal and State.PhysRecord position (basec10), then
   compute second dot3. Only after this second dot succeeds store position.
3. Absolute main acceleration uses raw sign-bit clear; store main normal,
   slide, force, and original u32 surface ID.
4. Store absolute sub-acceleration, sub-normal, sub-slide, sub-force, surface;
   then vehicle, pedestrian, skater, world bytes in that order.

Corrected raw VX128 operands: f254 D60/A61(delta)/B63(main normal);
f26c D57/A61(delta)/B59(sub-normal). The same loaded delta feeds both dots.
World position and all collision vectors retain their actual known/unknown W.
Part bounds in this native constant loop are always valid; no generic unknown
part fallback or manufactured record is admitted. A later numeric/service
failure leaves earlier record writes intact.

## Eight category records and mutation

For each category, capture its actual contact_bone (None corresponds to native
-1), then zero output slide speed and force. None preserves its old output
surface and still reaches that iteration's live debug cvar query.

For a selected bone, read same slide speed and contact normal, then same State
velocity delta and its squared dot3, then same part mass. Raw f3b8 is delta·delta,
not delta·normal. The latter is the separate f3f4 dot. Both thresholds are
raw **4099000c**, BO4/BI25 NOTGT: squared>0.1 and slide_speed>0.5 produce one;
unordered produces zero. Preserve multiplication by zero rather than branching
away later exceptional operands.

Source scalar FDIVS is `inverse_lifetime=1/lifetime`, once per selected category;
it does not call the estimate service. Then:

```
acceleration = abs_bits(dot3(delta, normal))
slide_rate = (inverse_lifetime * slide_scalar) * slide_gate
contact_rate = (inverse_lifetime * contact_scalar) * acceleration
force = (acceleration * mass) * force_scalar
slide_total = fma(slide_rate, slide_speed, old_slide_total)   // f430/f434
contact_total = fma(contact_rate, contact_gate, old_contact_total) // f43c/f440
```

Both old totals are captured before either store. Slide total mutates first,
contact total second, then output slide speed, clamped force and raw surface.
Force follows three source FSELs: select at least .001 using `.001-force`,
select at least zero using `-value`, then at most one using `1-value`.
This is not Rust clamp/max/min: e.g. NaN finishes as **one** at the last FSEL.
The same CollisionData fields are mutated; no secondary accumulator owner exists.

## Final publication and unwritten-byte boundary

After eight categories, f5ac captures same `specific_parts.contact[1]` (face)
**before** clamping either total. Existing ctor/Reset intentionally retain
contact/recent-contact as Option because native never initializes them there.
If its actual producer has not written face, the draft errors at this precise
late point, preserving all earlier bone/category writes and unclamped totals.
It does not infer false to complete a first-frame test.

Clamp contact then slide total with the two source FSELs. Publish captured face,
fresh groin, recent groin, recent face, then reread/store contact total, slide
total, time in wipeout, vehicle, skater, pedestrian impacts. Missing later
specific bytes preserve already completed clamps and preceding fields.

## Live debug cvar and actual prerequisites

`kDrawAudioCollisionInfo` ref **8308eb70**, initializer **82f7bf00**, table
**82fd44f4**. Section `Physics_Skeleton_Collision`, label
`kDrawAudioCollisionInfo`, title `Draw Audio Info`, defaultfalse/null callback.
Parent reservation: **index130**.

The cvar is read eight times at loopf468 and once at suffixf62c. An enabled
category requires two actual DebugOverlay reads/calls. Enabled suffix requires
four fixed calls then24 bone calls. All terminal targets are verified shared
BLR825187c0; therefore all-enabled execution still makes **44** actual singleton
prerequisite queries. Disabled execution makes none. Debug formatting argument
arithmetic has no retained data result; native FP exception-status side effects
are outside the existing host numeric contract, not a claim of recovered audio
rendering or overlay output.

## Existing schema and numeric limits

The core passes source bool bytes as u8. The existing actual PhysOut collision
schema stores Rust bool; the sink follows that established truth-value policy.
Native noncanonical-byte bit-copy parity therefore remains a schema limitation
and is not claimed. Surface identifiers preserve all32 bits, including signed
audio category interpretation via a bit-preserving conversion.

No new host transcendental or estimate behavior is introduced. Dot3 uses the
existing explicit ReckoningNumeric primitive. Fused accumulator updates,
ordinary products/division, sign-clear absolute and FSEL predicates are directly
source-backed. Native VMX horizontal reduction/denormal/NaN-payload and FP-status
parity remain outside the host numeric policy.

Tests cover all24 records and stale-surface preservation for empty categories,
guard/fourth-float timing, second-dot failure before first position store,
accumulator updates before late debug failure, unwritten face before clamps,
all44 enabled singleton calls, and exceptional FSEL/sign-zero behavior.
