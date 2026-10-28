# Selected Map travel runtime

Source implementation 2026-09-30; compilation and runtime tests remain
pending. Confidence high for source dispatch/data; selected loading gate is a
bounded host integration, not full native loading-screen/Flow parity.

`FEStuffMenu::PopulateMenus`8263e170 and
`PopulateMenuItemsFromLocationGroup`82632cc0 select these original group keys,
in order: map_general0229d7022a15e01e, map_plazas10b16dc9c4fe004f,
map_skatepark17577ab7dec354e2, map_spawnpointsb632afae7d120e40. Categories are
19,21,22,23. Locations attributeF1C6D5FA4A23951E contains24-byte RefSpecs with
collection key at+8. Group Icon786255B8FF426A3D is a RefSpec to fe_map_icons.

Shared loaded Vault classes (not a second database): fe_locations
DDCD6BC23255E508 (constructor824ed740 raw insrdi), fe_location_group
D897E7DB651E58DC (constructor825652f0), fe_map_icons F788504A8D922CED
(constructor82561f90). Each location cached layout has unlock identifier+0,
authored spawn location string+4, localization key+8, image name+12.
GetLocationName82615b68/GetLocationImage82615950 confirm the text fields.
Icon layout+0 is its texture basename; AddIcon82634400 appends `.Texture`.
Catalog retains group/array order and resolves transforms through existing
`LocationTable` on original BAM_Sim.loc. No coordinates are manufactured.

Scope deliberately exposes installed authored main-city destinations; an entry
whose spawn name is absent from BAM_Sim.loc is excluded as unavailable for this
main-world session. Native IsLocationUnlocked82615868 checks actual career/DLC
unlock backends (the includeDLCUnlocks argument does not mean always unlocked).
Those backends are absent, and this freeroam catalog is explicitly not a claim
that career locks have been restored or that career progress was fabricated.

`MapBackEndManager::TeleportToFELocation`82565350 calls
`TeleportLocalToWorldLocation`828a91f8 with location key, layout+4 spawn name,
onboard=false and largeLoadRadius=true. GetSpawnLocation82a8d250 resolves the
local player's authored location orientation. For the selected local player0,
the authored matrix is used. TeleportLocalToWorldLocation828a9cf0 and
TeleportLocalPlayer828a9f40 route through Flow and send ClearSessionMarker.
Player stance comes from the existing PlayersBackendSettings, converted to the
native FootForward boolean. ClearSessionMarker828aa550 only clears valid plus
the dynamic locator table (empty in the selected runtime).

Native TeleportRequiresLoadScreen828a9420 asks IsAreaLoaded independently for
Sim and Pres using the original large-radius cvars. The selected implementation
does not fabricate an always-loaded answer: Map keeps FRONTEND3 paused while
the existing terrain focus loader completes **actual collision registrations**
for its selected destination collections. The same retained CityStreaming CPU
loader and Inspection GPU-generation builder load destination Pres/Tex; the app
acknowledges readiness only after neither is pending and both installed CPU
focus positions match the destination. No parallel streamer, direct camera jump
or replacement physics is introduced. This conservative selected focus gate is
not the full native large-radius sphere coverage query or native load screen.

After both gates pass, the existing SessionMarkerTeleportQueues carries the
source teleport request (session_marker=false, onboard=false) through Flow,
SetSpawnStance, PlayerUI command5, SkaterManager::AdjustSpawnPosition and
Actor::Teleport. Terrain queries and native spawn adjustment are retained.
The frontend closes and releases FRONTEND3, with the close poll consumed before
simulation resumes. Destination focus stays pinned in core and visual snapshots
until the requested Actor teleport generation publishes the matching adjusted
reset with PhysOut.is_teleport_finished and Motion.did_teleport
(FillPhysOut82d869f0), and then through the NEXT completed Update1, so
the actor's old transform cannot unload the destination during the teleport
state transition. CheckForTeleport82e01db8 consumes the prior completed
publication and physically relocates on that following Update1; a stale ready
flag from an earlier teleport cannot release the gate. New Start opening is suppressed during this pending handoff.

Map Back returns to StartHub, preserving its selected Map row and pause owner.
Only Map dispatch is admitted; Edit Skater and all other StartHub destinations
remain inactive. Existing input polling and original map expressions/repeats
are reused. The app clears host timer debt on open/close; paused loading time
is not replayed into physics.

Remaining validation: a build and live travel across distant city sectors,
original-data runtime catalog binding, collision and GPU readiness, repeated
travel, and source-faithful map visual inspection. No build/test result is
claimed. Additional worlds/DLC switching, career locks, native loading movie,
and native large-radius IsAreaLoaded coverage remain outside selected scope.

Read-only original corpus inspection (not a runtime test): actual skater.vlt/.bin
array headers have 8-byte headers, 24-byte RefSpec stride, and node flags0x0a;
the four selected groups contain10/6/10/11 entries (37 total). All strings and
icon references resolve. Reading the original worldbam.big BAM_Sim.loc found
787 records; all37 catalog spawn names resolve, with zero Sublocation tags.
The native RefSpec layout and all local-player0 selected transforms are thus
checked against actual installed data, not fabricated fixture records.

Local map arrow: MapIconManager::UpdateMiniMap826187b8 obtains Actor position
and Entity::GetHeading825f4880. Actor::GetTransform825f9e40 delegates to
GetEffectiveSkaterTransform825ed4c0, which reads PhysOut Skeleton+0x1b0.
The facade supplies completed skeleton-root position XZ and effective-skater
Z-axis XZ; the renderer converts this basis with atan2.
