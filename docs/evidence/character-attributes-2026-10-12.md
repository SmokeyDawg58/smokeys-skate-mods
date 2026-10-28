# Selected skater attributes: Big Black bounce and pro animation styles

## Request and status

Implement the original selectable skaters' special behavior, especially Big
Black's bouncy physics in Free Skate. PC only, solo. Android remains parked.

Implemented the missing connection between selecting a character and the retained
first Actor's bounce flag, plus the confirmed pro/style carryover mistake.
Party Play's retained players are refreshed on Change Game as well. Fresh AI and
later party players already construct these attributes from their own identity;
they share the same native bounce predicate with the repaired entry path.

Full PC `cargo check -p skate2-pc --bin skate2-pc -j 1 --offline` passed in 53.61 s
with `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0` in the check process only.
Existing warnings: core 435, PC 18. No tests added or run, no release rebuild,
no game launch or deterministic comparison against reference runtime traces.
The current executable still needs rebuilding for these changes.

## Original behavior

| Reference | Recovered behavior |
| --- | --- |
| `Actor::IsSkaterBouncy` `825ed010` | Checks `kBouncyBigBlackEnabled`, then rejects network games and the global ghost-system flag, then compares this Actor's character against the full 64-bit Big Black ID `0376a7b6e3aadc5c`. No human-only or particular offline-mode gate. |
| Raw PPC `825ed098..825ed0ac` | Constructs both halves of that ID and uses `cmpld`. The decompiler's 32-bit rendering is incomplete. |
| Actor constructor `825fe850`, call `825ff278` | Samples bounce after the shared `phys_Tpose` parameters, before bind pose and the hat query. It is a constructor attribute, not a per-frame or per-bail character lookup. |
| `SkeletonBody` constructor `82b62200`, store `82b62594` | Retains `mBouncyRagdoll` on the body. |
| `SkeletonBody::SetUpRagdoll` `82b4ffd8` | Body flag OR the separate global bouncy-ragdoll cheat selects restitution. Reads the live restitution cvar independently for head and body. Otherwise restores both original Vault values. |
| Cvar initializer `82f56570` | Big Black enabled by default; no callback that updates already constructed actors. |
| Restitution initializer `82f7d418` | Default `0.99f`, exact bits `3f7d70a4`, range 0..1. Runtime code continues reading the actual cvar. |
| `SkaterManager::OnCommand` `82600920`, case 3 `82600984..826009bc` | A player spawn destroys the previous Actor and creates a new one. Case 5 ordinary teleport preserves the Actor. |
| `PartyPlayLoadingState` `8276e6c8`, `8276e874..8276e94c` | Dispatches the per-player spawn-location messages, then restores local player zero. Applies to Change Game as well. |
| `PlayerLoaderImpl::ApplyPlayerAttributes` `828158f0` | Marquee selection loads its own character ID, stance and hat. CAC selection also supplies its saved gestures/style and equipment settings. Existing local Actor reload follows. |
| `Actor::ReloadCharacterData` `825f4690`; `GetCACSettings` `825f0770` | Reads actual Actor player ID, natural stance, default gestures for marquees and saved gestures/style for CAC. Six characters supply special pro animation strings. |
| `SkaterAnim` constructor `82cc0a90`, assignment `82cc0ec8` | Starts `mProSkater` at `Default`. |
| `SkaterAnim::SetCharacterData` `82cae1d0`, `82cae2a0..82cae2d8` | Replaces the pro string only if its input is non-null. Ordinary reload does not itself reset a prior pro string. |

Big Black's special bounce is a **ragdoll** property. The traced branch supplies
no character-specific push, ollie, gravity, drag or mass multiplier. Existing
joint-limit, drag, body-mass and contact-material calculations are retained.
Ragdoll head material remains assigned to parts 1, 3 and 7. The separate global
`kBouncyRagdollCheat` can still make other characters bouncy; the new selection
logic does not suppress it.

### Native pro animation selections

Verified the entire `GetCACSettings` PPC comparison tree, including the two
upper-half cases that the decompiler loses:

| Character ID | Native animation name |
| --- | --- |
| `1d939610fea090ee` | `JerryHsu` |
| `0ded833242f2f857` | `JasonDill` |
| `146d3b7d88385267` | `MikeCarroll` |
| `1733a97b7064d4c2` | `PJLadd` |
| `42545b0938e341c9` | `DannyWay` |
| `be3309ce9ba41097` | `RobDyrdek` |

The existing host table matches. Big Black and Dem Bones supply no special pro
string here. CAC ID `ffffffffffffffff` selects Normal/null, Loose or Aggressive
and the four saved gestures; other marquees use the native default gesture set.

### Original data audit

Read `data/db/skater.vlt` and `.bin` directly in memory from the supplied Xbox
`data/db/db.big`. No original binary assets were copied into the repository.
The report inventories all 51 `characters_marquee` collections and all 11 class
fields, retaining parent IDs, stance, hat and speech-character values.

IDA type `Attrib::Gen::characters_marquee::_LayoutStruct` confirms recipe@0,
halname@4, valid@8, UsableAsAmbientSkater@9, speechcharacter@12,
regularstance@16, AmbientOnlySkater@17. The remaining fields include the hat,
AI-profile and challenge-equivalent references. This schema is not a table of
per-pro human skating statistics. AI profiles belong to AI control decisions;
they are not added as invented stat multipliers to a human-selected skater.

Big Black's authored entry is exactly `0376a7b6e3aadc5c`, regularstance 0,
hat type 0, speech character 26, parent `default`. The character ID is kept
distinct from its recipe name and player index.

## Host mismatch and changes

1. The host keeps its first Actor/physical pair across front-end game starts.
   Recipe/PlayersBackEnd selection could change, while `ragdoll_inputs_mut`
   always reread the **initial immutable construction input**. Selecting Big
   Black therefore left the body with the previously constructed bounce flag.
2. The body now consumes its retained `SkeletonConstructorBindings` byte.
   New-game entry resamples the same native `IsSkaterBouncy` predicate and
   updates that byte and the retained animation constructor flag together.
   Pair ownership/attachment is checked before the mutation.
3. The refresh is queued even if the requested recipe is already loaded. It
   runs after character loading and destination/presentation readiness, before
   the spawn teleport is dispatched. Unload clears a pending refresh.
4. Free Skate and Career entry use this boundary. Party Play entry does too;
   Change Game marks player zero pending and refreshes each retained later
   player before its own spawn teleport. Fresh party/AI actors continue through
   their real constructor. The lookup uses the requested Actor identity, not
   whichever party player is currently local.
5. At this same new-Actor boundary, absent pro/style strings now restore the
   constructor's `Default`. The ordinary edit/reload function keeps the original
   null-string behavior. This prevents a previous pro, Loose or Aggressive
   character from leaving its animation style on the next default character.

Normal map travel, session-marker use, turn switching and bails do not resample
the constructor flag. The ragdoll setup still reads restitution on each actual
entry and invalidates collision publication before those material changes.
Ground-contact material forwarding retains the live restitution. The existing
native skater-versus-DMO restitution cap is intentionally preserved.

## Files and review

- `animation/runtime/actor_animation_player_services.rs`: one shared native predicate.
- `physics/skeleton/body/skeleton_assembly_construction.rs`: consume the live retained byte.
- `physics/assembly/runtime/dynamic_assembly_skater_pool_ragdoll.rs`: checked spawn-byte update.
- `physics/player/skeleton/physical_player_skeleton_controller_session.rs`: world forwarding.
- `session/edit_skater_player_attributes.rs`: constructor-default animation handling.
- `session/selected_session_freeskate.rs`: pending spawn attributes and Actor selection.
- `session/selected_session_party_play.rs`: retained-player Change Game handling.
- `session/selected_session_public_facade.rs`: apply at ready spawn and shorten input borrow.

Paths are relative to `crates/skate2-core/src`. Eight source files changed.
Source review followed the selection, loading, first/later Actor, ragdoll and
material-consumer paths. Initial compile attempts found a mutable input borrow
crossing the new refresh; it was corrected and the full PC type check passed.
No new warnings were added. Scoped source whitespace review found no added
whitespace errors; global formatting was not applied to unrelated work.

Confidence: **high** for the recovered predicates, constants, string selection
and host mismatch, backed by live IDA/PPC and original assets. Runtime behavior
and numerical parity of the full engine remain unverified by this task. This
change does not replace the host's entire retained-Actor lifecycle with native
destruction/reconstruction; other constructor-owned resources, such as facial
pose and hat collision geometry, are not rebuilt by this narrow attribute hook.

Research packet: `.scratch/character-attributes-20261012/` contains source/doc
backups, `reference.json`, the read-only asset audit script and
`marquee-catalog.json`, `scoped.diff`, and the compile logs. The final check is
`cargo-check-final.log`. Earlier intentional gameplay/presentation preferences,
unlock persistence, the real save and the Android tree were left intact.
