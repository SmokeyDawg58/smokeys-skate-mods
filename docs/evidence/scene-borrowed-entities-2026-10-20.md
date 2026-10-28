# A scene's borrowed skater entities (2026-10-20)

`SceneCharacterManager::InstanceCharacter` has two overloads, and each borrows one skater entity
with `ControlCharacterRenderable` 0x82870fb0:

- `0x828769f0` (a "dynamic" record lent by player id): `GetSkaterPresEntityFromPlayerId(playerIndex)`
  0x82876aac, the entity of that player.
- `0x82876bd8` ("current_player"): `GetSkaterPresEntityFromIndex(GetLocalIndex())` 0x82876ee0, the
  entity `SkaterPresentation::GetLocalIndex` 0x827d0090 flags local. `SkaterManager::SetLocalIndex`
  0x82600380 moves that flag; a challenge's `Online.SwitchCamera` 0x8288a4e0 and its teleports with
  a camera move it to the skater it follows, an AI skater on its S.K.A.T.E. turn too.

The entity keeps the borrow (an intrusive reference in `mSceneRenderModels`) until the scene
releases it. The ICE camera anchors (`User_ICECameraAnchor::GetPosition` 0x826cfd04 and
`GetOrientation` 0x826cd310, `User_ICEWorldManager::GetSkaterHipsPosition` 0x826cfe2c) read the
local index's skater as well.

## The port

- `CareerSceneSource` holds the first player's id (`INetworkGameQuery::GetLocalPlayerID`, 0
  offline) apart from the local index. A "current_player" record takes the local index's player
  and recipe, as a lent record takes its player's.
- `CareerSession::publish_borrowed_skaters` poses the entity of the character's player: an AI or
  later player's by its index, else the first player's. Each update first ends every borrow, and a
  borrow lives on the entity (`SelectedPresentationRuntime::borrowed`, `first_player_borrowed`),
  so a skater spawned while a scene runs is drawn as itself.
- Before, a character's player was compared with the challenge's local index: a scene that borrowed
  the skater the camera followed posed the first player's entity instead, and an AI skater whose
  index a scene named but never posed failed the visual ("AI skater hips unpublished").
- The scene's camera anchors read `local_index_presentation` (the local presented skater, else the
  first player's).

The first leader of a single player S.K.A.T.E. game is an AI skater: `setup_skate` sets
`LeaderID` to the id of the last `npc` in `SkatersList`, so the camera follows a pro first.
