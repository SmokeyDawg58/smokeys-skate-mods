# ChallengeMap FELocation source implementation

Confidence: high for recovered browser/native arithmetic; no compilation or
runtime tests performed. Full career map,
interactive pan/zoom, DLC and challenge progress are outside this freeroam slice.

## Controller and data

- StartHub IStartHubState::OnMenuSelect82674d08 pushes state21 for ChallengeMap.
- OnEnter826539e8 opens screen14, initializes Stuff menu and region0. OnLoaded
  826879b8 emits states0/10/90 then activates the Stuff region.
- FEStuffMenu::PopulateMenus8263e170 populates Locations10 and child categories
  Landmarks19, Pools20, Plazas21, Skateparks22, SpawnPoints23. Pools require real
  challenge owners. Empty categories are filtered (FilterCategory82629430).
- gSubCategory82079860 confirms native root/category order. This implementation
  admits only the four actual FELocation groups, preserving source array order.
- PopulateMenuItemsFromLocationGroup82632cc0 reads attribute
  f1c6d5fa4a23951e of fe_location_group d897e7db651e58dc; location groups are
  map_general0229d7022a15e01e, map_plazas10b16dc9c4fe004f,
  map_skatepark17577ab7dec354e2, map_spawnpointsb632afae7d120e40.
  Actual unlock filtering is owned by the catalog/runtime. FreeSkate passes
  includeDLCUnlocks=true to IsLocationUnlocked; this is not an all-unlocked flag.
- GetCategoryHAL8261cb48 localizes gStuffCategoryHAL82fdb2f0 and uses the
  localized string as snprintf format with available item count. Controller
  exposes that count separately; no labels or counts are invented.
  Independent read-only decode of miscboot.big English Global/FrontEnd and
  HISTOGRAM (original RefPack, language table offsets and histogram expansion)
  confirms: Stuff, Locations (%d), Landmarks (%d), Plazas (%d), Skateparks (%d),
  Spawn Points (%d). GetNumSubMenuItems8261c8c0 returns categoryList10 length
  for Locations, not the number of child categories; root recursively sums.
- SelectMenuItem826357f0 descends categories, leaf FELocation selection calls
  SelectFELocationItem82629780 -> TriggerInfoComponent_FELocation8265f180:
  store key, info mode/FELocation substate, states72 then20.
- ReceiveInput8266b420: B closes info, backs a category, or pops map at root.
  Start has no explicit map action. A/up/down pass to authored picker.
  Independent movie review found challengemap constructor@5a73 explicitly
  disables picker wrap. Controller clamps both ends (StartHub still wraps).
- OnMenuSelect826828f0 info row0 requests fade then DoTeleport8265f280 invokes
  backend TeleportToFELocation and pops through StartHub7. Controller publishes
  Teleport(key); runtime owns completion, pause and stream synchronization.
- Mission IsDescriptionEnabled82625b40 explicitly returns false for FELocation.
  IsLocationImageEnabled826256d8 requires actual GetChallenge, so false for
  plain FELocation. The location image string must not fabricate a visible image.

## Viewport and icons

- SetMapInfo82606f08 raw PPC verified bounds offsets110/114/118/11c,
  ranges120/124, texture dimensions44/48; aspect is width/height or height/width
  depending on comparison after source scale0x3b09f874.
- ResetMap82617b88 UV order is (0,0),(1,0),(1,1),(0,1).
  kMapDefaultZoom initializer82f587e8 source default1.0.
- GetMapCenter82617648 and GetMapTopLeft82617778 use STANDARD_UV, not selected
  cursor coordinates. Raw scalar instructions verify center=(range*.5)+min
  using fused multiply-add; top-left=(xmin,ymin). World pair is X/Z.
- GetScreenCoordFromWorldCoord826178c0: texture-dimension*.5 divided by
  center-minus-top-left, multiplied by (world-minus-center)*icon scale. Scale
  cvar defaults X=.99,Y=.95 at82f588a8/82f58908. map_viewport.rs takes those
  cvar values explicitly so future overrides need not rewrite arithmetic.
- HighlightFELocationItem8261ceb8 obtains location orientation by layout+4
  startpoint, truncates X/Z to integer for FEMapCursor::SetCursorPosition826183b8,
  and calls EnlargeIcon82565478. It does not autozoom or recenter the map.
- FEMapCursor::Init82617c60 loads
  data/fe/source/screens/map/cursorchallengemap.rx2. FEMapManager::RenderMap
  82631f70 draws map plus MapIconManager::RenderMapIcons82631138; it does not
  directly draw the cursor. Cursor is not invented as an extra sprite here.
- AddLocationIcon82577850 reads group Icon attribute786255b8ff426a3d
  (getter8255c4a8 raw confirmed), then fe_map_icons classf788504a8d922ced
  (constructor82561f90 raw confirmed). AddIcon82634400 reads layout+0 basename
  and appends .Texture. World X/Z position comes from actual location transform.
- MapIcon ctor82601a50 / AddIcon set scale1, map scale1, non-pinchpoint.
  EnlargeIcon82565478 sets selected visible location map scale2, others1.
- RenderIconOnScreenMap8262e350 uses white RGBA and full texture UV. Size is
  kFEMapEventIconSize * mapScale * scale; default32 verified initializer
  82f58b40 and target reference83049b30. Thus location sprites are32 square,
  selected64 square before host viewport projection. Only local skater uses
  heading rotation. UpdateMap82618728 pulses pinchpoints only.
- HUD MiniMap::Load826c0330 loads the shared icon arena at the original literal
  820a8000: data/fe/source/images/icons/minimap.rx2, then initializes the same
  MapIconManager used by ChallengeMap. Do not guess an icons.rx2 filename.
- Local skater icon collection8f7f7c8ecf36fd3b is verified in raw Initialize
  82635c2c..4c. RenderIconOnScreenMap rotates heading+PI with
  x'=cos*x+sin*y, y'=cos*y-sin*x. Local offset(0,-.01*25*size) is rotated too;
  kFEMapLocalSkaterIconYOffsetPct default25.0 comes from82f58bf0.
- OnMenuHilight82644870 calls FilterIcons82607388 on the selected child
  category when current category has children; StuffMenuFilter825654d8 admits
  category-member keys only and resets their map scale1. Others become hidden.
  Root Locations means all four groups; Locations submenu means selected child
  group only; leaf/info preserve that group and selected enlargement. Local
  skater is a separate icon outside backend mMapIcons and remains visible.
- marker_ui.rs MapRenderComponent custom handler uses actual shape's second
  position for half extents (CustomControlRender caller82636730), full native
  UVs/white texture and category filtered icons. Native icon renderer uses
  identity matrix and adds only map matrix translation to texture-pixel offsets;
  no map-quad resize factor is applied to icon size/offset. Local skater follows
  native heading+PI rotation and25% back offset. MapRenderFacts carries retained
  controller/catalog/player values without exposing renderer to simulation.
  Independent movie-agent audit found no _xscale/_yscale operations anywhere in
  challengemap bytecode. Authored map shape stays approximately480square;
  original329x512 texture metadata does not authorize stretching the custom
  component by an invented aspect correction. Native aspect multipliers feed
  cursor math separately.

No guessed coordinate list, map bounds, autozoom, career progression or
unlock booleans have been added to the controller. Source syntax validation
is distinct from compilation and live behavior, which remain unverified here.
Rustfmt parsed/formatted map_menu.rs, map_viewport.rs and pause_menu.rs
successfully. No cargo, compiler, tests or game process was launched.
