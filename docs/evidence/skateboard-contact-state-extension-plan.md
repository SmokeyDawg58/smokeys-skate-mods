# Same-owner board contact-state extension

Read-only audit, 2026-09-08. This supplements
`skateboard-body-postphysics-runtime-plan.md`; it does not implement another
contact producer or traversal. Fresh debug-XEX raw reads covered Reset
82b23630..23760 and the initialization/winner/debug portions of PostPhysics.

## Existing authority and storage

`skateboard_contact_surface.rs` already traverses actual `PhysicalContactInfo`
in retained order and owns the seven-part winner and four-wheel vote.
`physical_player_board_surface.rs` validates the actual board even on an empty
frame, validates current/completed contacts, performs the real GetCollisions
map obtain/create, and then borrows a completed token. Keep this route. Its
Arc/Mutex is the SAME `SkateboardForceRuntime.contact_surface`, allocated once;
the immutable World adapter allows owner reads at their source sites.

The current `new()` is a surface-only host projection, not native CollisionInfo
Reset. Extend that owner with a distinct source Reset initialization stage at
the actual body-constructor boundary. Do not reset per-part history every frame
or reconstruct a second owner when the completed construction is handed off.

## Reset: corrected array identity

CollisionInfo is Body+40. Reset's r11 starts at CollisionInfo+90 = Body+d0.
Across seven iterations, 236fc writes full UP to Body+60+i*10 (normal),
23700 writes full zero to Body+d0+i*10 (position), 23704 writes full zero to
Body+140+i*10 (velocity), and 23708 writes full zero to Body+1b0+i*10
(acceleration). **It does not initialize relative velocity at Body+220.**
An earlier agent message incorrectly called the third zero array relative
velocity; this paragraph explicitly supersedes that statement.

236a4/236ac initialize average and last-valid normal to full UP. Reset clears
seven in-contact bytes at Body+34c, three seven-word surface arrays (physics
Body+2f4, audio Body+310, pattern Body+32c), and four validity bytes Body+2f0.
Relative velocity must remain unready until its accepted-contact store36e08.
The actual PhysicalContactInfo methods expose XYZ normal/position/relative
velocity; copying these must preserve unknown W rather than pretend the
Reset W survives a native full-vector replacement.

## Validity alias and source stages

The per-frame loop uses seven byte stores at Body+2f0+i. Bytes 0..3 are wheel
validity; bytes 4..6 alias the top three big-endian bytes of physics_type[0].
Represent one shared byte/word backing (or exact setters/getters over the same
u32 word), never an independent seven-bool array. Initializing bytes4..6 to1
can make physics[0] 0x010101xx until a winner overwrites its entire word.
Later group validity clears and grip reads must observe that same alias.

For each of seven parts, native order is in_contact=false36b10,
validity_byte=1 36b14, max_contact_speed=0 36b18, full max-contact-velocity
zero36b24, then that part's debug range writes. Keep the repeated scalar/vector
stores interleaved with each range; hoisting them would change a fallible
debug adapter's prefix. Only after all parts does the flags mask store36b8c
occur. The existing `begin_selection()` cannot remain a separate all-parts
clear in front of this extended stage.

The existing loop should capture contact fields once, run pre-filter opposing
normal/max-speed effects using OLD retained velocity, then perform owner/group
reads. For nonnull other: other owner36c9c, this owner36ca0, comparison36ca4,
other Assembly group36ca8, then same-Entity skip36cac. The group read must occur
even when the Entity owners compare equal. Ground/null-other reads neither.
Current `same_entity` helper omits group and must be extended rather than
followed by a second traversal.

Keep group4 GetBodyTransform/cvar geometry and groups14/15/16 effects before
the winner. The existing BodyPost plan gives the exact dependencies; group4
must use the Body getter, not live Part::GetTransform. Current retained owners
support group4 boards and skeletons; do not invent group14/15/16 owners.

Accepted first/strict-higher-Y winner stores in order: full normal36dd4,
position36e00, relative velocity36e08, pattern36e10=(surface>>12)&15,
physics36e14=(surface>>7)&31, audio36e18=surface&127. Best-part tracking reads
the RETAINED winner normal after each admitted contact, then in_contact=true
36e68 and debug writes follow. Losing admitted contacts still execute the
latter tracking/in-contact/debug stages. NaN never satisfies strict greater.

No-contact wheel-line fallback, seven live velocity/acceleration updates and
grip precede the existing vote/late forced-surface cvar. Preserve those source
positions as integration expands; do not publish a completed normal endpoint
merely because the older surface projection can vote.

## Concrete actual owner inputs

Add a late `other_group(body)` capability alongside `owner(body)` on the
existing World adapter. Reuse the pool member resolution already demonstrated
in `dynamic_assembly_skater_pool_skeleton.rs`: Board reads its retained
construction.group(); Skeleton reads owner.state().assembly_group after
checking actual construction. This is Assembly group, not volume group/tag,
Part userdata or a caller-supplied integer. Group read errors remain after both
owner reads and all earlier contact effects. The same actual session/backend
supplies Part6 Body transform and seven current body velocities.

The same collision definition is retained by the actual session template
(`DynamicAssemblyRuntimeTemplate::collision()`). Wheel part.debug() gives
index..end_index(); each truck part.debug().index gives index..index+1;
deck.debug_range() supplies the complete variable child range. Construction
starts wheels at0, trucks at wheels.debug_end(), deck at trucks.debug_end().
Do not hardcode one debug object per deck or re-read Vault to reconstruct it.

Definitions are immutable construction evidence. Wheel/deck records do not
currently carry mutable color fields; truck color is constructor evidence.
Add a retained mutable color projection bound once to this exact definition
and its ranges, with readiness for unconstructed entries. No renderer or
second shape allocation is required for these source color stores. Existing
construction explicitly excludes the temporary debug Instance lifetime, so
the new projection must not claim that unrelated collaborator was implemented.

Frame-reset color raw words are [3dcccccd,3dcccccd,3e4ccccd,3f800000]
(.1,.1,.2,1); admitted-contact color is
[3f19999a,3f19999a,3f19999a,3f800000] (.6,.6,.6,1).
Native ranges are Body+1a20+part*4 (start), Body+1a3c+part*4 (end),
with color at Body+4c0+index*b0. Reset reads start then end; contact stage
reads end36e7c then start36e80. Each loop rereads end after every color store
(36b50 /36ea4). A staged sink should retain that order and completed colors
on refusal; a retained immutable-range implementation can document that
range mutation is excluded by its owner borrow.

## Minimal change set and validation

Extend `skateboard_contact_surface.rs`/tests in place; extend its existing
World adapter and pool group/body read accessors; bind Reset/debug projection
through the same actual body-construction occurrence. Root owns shared
constructor/caller integration. Reuse existing typed vector readiness and
body getter/numeric services rather than duplicating their arithmetic.

Focused checks should distinguish Reset acceleration from unready relative
velocity; BE validity/physics alias writes in both directions; group read
failure even for same-Entity contacts; pre-filter effects surviving that skip;
losing/NaN winner retention with admitted debug writes; variable actual deck
debug ranges; and late failures retaining earlier scalar/vector/color stores.
The actual corpus should use existing solved contacts and original definitions.
No code changes or Cargo runs were performed for this audit.
