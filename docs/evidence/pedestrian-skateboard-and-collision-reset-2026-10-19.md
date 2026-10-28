# Pedestrian skateboards and collision resets — October 19, 2026

Two living-world pedestrian differences from `sk82_na_m.xex` (Jan-17-2009),
checked in the IDB and the shipped `skater.vlt`.

## 1. `Hide_Skateboard`: challenge characters without a board

Every marquee recipe has a `SkateBoard` component (deck, trucks and wheels,
skinned to `Skateboard_Root`, `Truck_Front`, `Truck_Back` and the four wheel
bones). `MarqueeSkeletonPres.abin` has those bones at 25..31; `skateboard_root`
is a root bone. `UpdateFullHandFacePedestrians` (0x82809738) starts from the
`rig_Tpose` bind pose, then replaces the face, the body and the right hand prop.
`cLivingWorldPresEntityManager::Init` (0x828112b0) sizes the body range on
`PedestrianSkeletonPres.abin`: 1 + `LeftHandProp` − `Trajectory` = 26 bones,
copied to marquee bones 0..25. So `skateboard_root` takes the pedestrian's
left-hand-prop transform (relative to the hand) as a root transform. The trucks
and wheels keep the bind pose relative to it, and a drawn board lies at the
feet, turned, beside the character.

`cPresPedestrian::cPresPedestrian` (0x828122b0, also the base of
`cFullHandFacePresPedestrian` 0x828126f0) reads the presentation entity's
`livingworld_models` bool `Hide_Skateboard` (0x0626fa10792bba58, read at
0x828124f0). When it is set, `PresentationEntity::RemoveComponentInstances`
(0x827fa6e8) removes the component named by
`CACPartDB::ConvertToComponentString(SKATE_DECK)` (slot 11, `SkateBoard`,
`mPartSlotNames` 0x830073f0) from all three LODs
(`RemoveLODModelInstances` 0x827fa4d8). This happens before `InitSkinning`
(0x828063b8) and `FillInvBPBuffer` (0x827f8338). `InitFromRecipe` (0x827faab8)
already computed the bounds, and the removal does not recompute them.
`SwitchLOD` remaps only the remaining instances.

Shipped `livingworld_models` with `Hide_Skateboard` set: `attiba_jefferson`,
`big_black`, `freeway`, `giovanni_reda`, `michael_burnette`, `mike`, `sammy`,
`seb`. All other models, the pros among them, keep the board.

Port: `CharacterPalette::pedestrian` builds the skinning without the component
(`CharacterScene::build_skinning_state_without`). The palette keeps the removed
index for `select_pedestrian_lod`, and `from_replay_skinning` restores it for a
recorded skinning that left the board out. `PedestrianPresentationPose::new`
reads the flag from the entity's model attributes.

## 2. `LWPedestrianMan::ResetCollisionFlags`: repeated knockdowns

`Manager::Update1` resets collision flags for pedestrians, vehicles and DMOs in
that order (0x827281e8, 0x827281f0, 0x827281f8). `LWPedestrianMan::
ResetCollisionFlags` (0x82e83580) calls the virtual
`Pedestrian::ResetCollisionFlags` (0x82e6e550) for every pedestrian:

- `LWCollideableObject::ResetCollisionFlags` (0x82e4c6c8) clears the world and
  skater part bits and `CollidingWithSkater`.
- If the state storage's collision byte (+0xC5C) has 0x20 (the last collision is
  over), the collider (+0xC50/+0xC54) becomes invalid and the highest collision
  type (+0xC58) becomes 0. Bits 0x80, 0x40 and 0x20 clear and 0x10 is set.

0x10 gates `PedestrianColliding::ProcessCollisionFlags` (0x82714b80), which
broadcasts the `collision` mood event (`BroadcastCollisionEvent` 0x82711df8,
type 10). `tPedestrianStateStorage::Reset` (0x82e92778) also sets it, but only
the constructor calls that.

The `default` mood of `regular` pedestrians answers the skater's `collision`
events by occurrences: `warn` from the second, with no earlier reaction, and
`angrychase` (reset) from the third, after one or more reactions. `skaterdamage`,
which `UpdateMoodables` sends when a grind finishes, gives `angrychase`
directly.

The port had no pedestrian reset. Only a pedestrian's first collision after its
spawn broadcast, so the occurrence count never passed one. `CollidingWithSkater`
and the part bits also stayed set after the first contact. Port:
`LivingWorldGrindServices::pedestrian_reset_collision_flags` runs before the
vehicle and DMO resets in `try_update1`, and
`PedestrianConstruction::reset_collision_flags` mirrors 0x82e6e550.
