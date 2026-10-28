# Original LB marker HUD / EA APT investigation

Status: **asset readers and native HUD controller implemented/tested;
no visible HUD delivered yet**. Work moved from
world materials to UI, starting with the LB session-marker popup. Work solo.
Existing marker placement/teleport logic is retained. This is not Scaleform:
the executable contains EA APT, its ActionScript interpreter, display lists,
Flash movie timelines and Skate-specific geometry/font integration.

## Existing code and duplication audit

`src/session/session_marker{,_player_ui,_teleport}.rs` and the selected-session
bindings already implement input states52/53, placement rules, hold-to-teleport,
actor stance and respawn. Their previous evidence explicitly excluded frontend
notifications and HUD. No APT parser, movie player or original HUD renderer
exists in formats/core/app at the start of this task. FPS text is unrelated.
Do not duplicate marker gameplay or replace authored UI with a generic panel.

## Native HUD and screen ownership (high confidence)

* `PhonelistHUD` ctor826b5b20 selects screen enum12. `gScreenTemplate`82fdb358
  has16-byte entries; entry12 at82fdb418 is `PhoneListHUD`, URL
  `hud2/hudphonelist.swf`, transition0, screen type5. Resolve screen type5's
  camera/parent before claiming the 3D presentation is recovered. PDB confirms
  type5=eFEScreenType_3D_HUD_Focus_Left. SetVertexMatrix826095e8 selects
  SetApt3DFocusLeftMatrix826082b0 for layers2/5.
* Show826c0580 polls SIM input state29 when becoming visible; if held and
  CanShowSessionMarker, Maximize; otherwise SetScreenState0/minimized.
  Hide sends screen state3. UpdateInput826c0848 consumes input4 to open,
  input8 to close, input256 for full phone options if allowed. HUD input
  bit translation82641648: FE key9=>LB4, key53=>LB release8,
  key48=>OptionReleased256. ReceiveInput82656b00..6e70 forwards these while
  FE_STATE_IN_GAME, behind NIS/transition/input-lock/popup checks.
  GetInputSigniture826b2e60=1020. Live frontend input dispatch is not wired.
* Maximize826be8c8 caches marker IsUseAllowed/IsSetAllowed, selects group
  `phonelist`, screen1 when options allowed, otherwise screen2.
  Minimize826be870 clears the group filter and sends screen0.
* Update826c06f0 consumes dirty PhonelistHUDData: TrySet=>screen51,
  TryUse=>screen52, Close=>screen50. While maximized, changes to either
  marker permission refresh screen50. Native OnTrySet826bea28,
  OnTryUse826bea88, UpdateMainMenu826be9c8 only call SetScreenState.
* FrontEndManager826063c0 delegates ScreenManagerBase82d65d50, which calls
  registered ActionScript SetScreenState(screenId,state), passing decimal
  strings. Actual authored script/timeline interpretation is still required.
* StateManagerFrontEnd::OnTrySetSessionMarker8266fbf0 marks TrySet/dirty only
  on success (failure is sound); OnTryUse8266fcc0 similarly only on success.
* CanShowSessionMarker826b5c00 rejects video playing/buffering/opening, some
  HallOfMeat actor flags; accepts online/free-skate, or single-player outside
  challenge signup. CanShowPhoneListOptions826b5d90 requires single player,
  unlocked uncapping service and no main challenge or OwnTheSpot challenge.
* GetSetSessionMarkerText826b6010 / GetGotoSessionMarkerText826b61d0 select
  localized IDs; ordinary free skating uses ID_PHONELIST_PLACE_MARKER and
  ID_PHONELIST_GOTO_MARKER. Do not hardcode invented labels.

## Original assets (external, never commit)

Original root `<game folder>`.
`data/fe/screens.big` holds compressed `.apt`, `.const`, `.geo`, `.rx2` files;
existing named-BIG/RefPack readers suffice. This archive is not in the existing
20 simulation archive mounts. `source/screens/phonelist/phonelist` is the
full phone menu, **not this HUD**. Correct path is
`data/fe/source/screens/hud2/hudphonelist`.

Decoded HUD sizes: apt23893, const2139, geo2908, rx2610552. Root offset0x1a8,
79 character slots, stage1280x720,16 milliseconds/frame,31 root frames.
One import: `source/controls/button_item2` / `Button Item` at character31.
Root places character76 as mcQuickMenu; labels hudintro1, hudoutro24.
mcQuickMenu labels hidden0,minimized13,maximized27,open43. Its mQuickOpen
character75 labels main0/main2 at10. Authored marker text children mTextHelp0,
mTextHelp1 and option mTextHelp2 use enabled/disabled and pulse timelines.
Original strings/icons include DPad_Up,DPad_Down,L_Bumper, HelveticaGlow,
HelveticaPresto, Helvetica LT Std. Button component imports more runtime
resources; follow it before assuming this movie is self-contained.

Temporary decoded audit files under `%TEMP%/skate2-ui-audit-20260930`.
Boot miscboot.big includes Helvetica bitmap-font metrics and RX2 atlases.

## Asset layout / source evidence

* AptLoader::CompleteLoad82a1cbe0 obtains root offset from .const+20 and
  resolves it against APT base. Const header32 bytes: magic20,root,count,
  table offset. Eight-byte constants are type/value; strings relative to
  CONST base. `_parseStream`829f6bc8 verifies this and runtime constant fixup.
* AptCharacterAnimation::Fixup82a1beb0: character/import/export tables and
  strings relative to APT base; shapes load GEO by character-table index.
  Link82a19a68 resolves imports by exported name and binds bitmap IDs.
* AptCharacter: type+0, parent+4,flags+8,file+12,payload+16. Shape payload
  bounds16 and runtime render pointer; sprite payload is AptMovie{framecount,
  frame-table,labels}. Animation extends sprite with character count/table,
  width,height,frame-ms,import count/table,export count/table,constant index.
* AptMovie::resolve82a00a58: frames8 bytes(count,pointer-table); each control
  begins type. Types1 action,2 label,3 PlaceObject2,4 remove,5 background,
  6/7 sound,8 init action,9 PlaceObject3. All serialized pointers base-relative.
* PlaceObject2 payload60 bytes: flags,depth,character,matrix6floats,
  packed scale/bias,ratio,name,clip-depth,event-set. Flags move1,char2,matrix4,
  colour8,ratio16,name32,clip64,events128. PlaceObject3 adds blend/filter fields.
* GEO Sk8 header, shape count, inline pointer table. Shape ID,count,inline
  element pointers. AptGeometryInfo82a6fc90/GetGEO82a6fec8/ShapeInitialize
  82a6faa8. Element: type,+4 RGBA,+20textureID,+24matrix6floats,+48triangle
  count,+52 inline XY triangle vertices. SOLID1,clamped2,wrapped3.
  AptTextureUnit826098a8 asserts2 triangles (6vertices); AptVectorUnit82609ee8
  uses3*NumPrims. Factory8260a110 only creates units1..3; type0 LINE returns
  null, and Draw82a704ec skips null units. Actual HUD contains a line element
  (character77); reader retains it as Line, not guessed drawable triangles.

## Drawing investigation

Render2D82631a98: APT screen camera level8; APT3D camera levels7,6,5,4,3,2;
screen camera levels0,9. DrawRenderingUnit8261da48 applies accumulated vertex
and colour transforms, alpha blend, depth-based masks. SetColourTransform
82609488,SetVertexMatrix826095e8,DrawString826096d8 still need detailed audit.
Render3D8262a550 optionally draws a rendered-screen texture onto transformed
1280x720 world quads. This is **not proof that the marker HUD uses this path**.
AptMovieIntegration is the VP6/video player wrapper, not the Flash player.

Do not trust VMX128 high register bits in decompiler text; decode raw operands
before porting camera/matrix operations. Do not claim complete APT/3D/parity
until runtime draw, masking, fonts, imports, animation and input are verified.

## Validation

Implemented `skate2-formats::apt`: checked base-relative movie/constant/GEO
readers, labels, placements with original masks, imported character slots,
text metadata, frame-action/event offsets. Script execution is deliberately a
separate missing service; offsets are not a claim that actions execute.
Unsupported character/control/filter kinds error explicitly.

`skate2-core::ui::phonelist` implements the native visibility/input/data/marker
permission state machine and emits frontend commands. Native constructor dirty
flag verified826b3ee8=true; reset preserves dirty. This component is registered
but **not connected to selected-session input or GPU**. Existing session-marker
physics/teleport code has not changed.

`tools/inspect_apt.py` is a read-only authored ActionScript/timeline inspector,
not a game runtime or PPC interpreter. Original opcode names from APT's
sGlobalTable82ff7800; operand/fn-body layouts checked against829f6bc8,
82a100a8,82a10280. Decoded audits in artifacts/marker-{hud,button}-apt-audit-
20260930.txt; no original assets committed.

Focused release formats tests: **5 pass**, including original HUD/import GEO
and all six boot fonts, plus malformed pointers/strings and glyph bearings.
Standalone rustc tests of the production HUD controller: **3 pass**, including
hold-on-show vs edge input, pulse-before-permission-refresh, and dirty-data
lifetime while hidden/minimized. Cargo check of the full app passed at the
initial parser/controller checkpoint; see artifacts/marker-ui-workspace-check-20260930.log.
No UI runtime draw test yet.

## Authored script behavior verified

HUD ActionScript SetState atAPT0x5250:50 UpdateMainMenu;51 plays `pulse` on
mcQuickMenu.mQuickOpen.mTextHelp1;52 only traces and returns. These three do
not change mState.0 calls UpdateMinimizedMenu;1 plays maximized+main;2 plays
maximized+main2;3 plays hidden. Both maximize forms refresh marker text.
UpdateMainMenu0x55fc uses native Game.CanUseSessionMarker/CanPlaceSessionMarker
to play enabled/disabled on mTextHelp0/1.mText, then sets original localization
IDs for title, goto/place marker and options. UpdateMinimizedMenu0x5374 hides
the quick menu if no service icons or pinchpoint alerts; do not invent an
always-visible LB prompt. Constructor applies global Screen_EdgeOffset and
stops the root while children animate. Frame actions outside initialization
are authored Stop actions; do not replace timeline timing with an arbitrary ease.

Imported button_item2 registers Button Item class. Instance load actions set
mButtonImage DPad_Up/DPad_Down/L_Bumper; the component sets child mButtonRender
`_test`. That child has `_type=CustomButton`; CustomControlRender826363d8 builds
`button_%s.Texture`, then RenderButton8262d800 uses original texture size,
48-pixel anchor adjustment, source-alpha blending and no depth test/write.
Do not render the component's placeholder shape as its real button image.

## Fonts / perspective findings and remaining work

Added BitmapFont reader alongside APT formats. ReadHeader8284d300 only uses
Ascent,LineHeight,GlyphMetricsMap,CharMapSet. Nine metrics fields are
glyph,texture,x,y,width,height,bearingX,bearingY,advance; verified sscanf raw
arguments8284d46c..4a4. Runtime u=x+bearingX,v=y-bearingY,xoffset=bearingX,
yoffset=-bearingY. Character keys narrow to u16. Font base scales and mappings
must be read from Vault: InitializeFontDetails8282b930..bbc4, per-language
fontlist and referenced font collections; no hardcoded substitute font sizes.

SkateAptString82a70b28: '#' denotes literal text; other strings go through
LocalizerManager StringHash/GetPackedString/PackedToUCS2. No localization
service existed before this work. The language-category/histogram readers
described below now decode the original pack; live font/string binding remains.
Draw82a71150 multiplies glyph base scale by authored font height, applies text
bounds/alignment and original color transform. UI textures can reuse existing
RW4/texture ownership and GPU upload rather than duplicating decoding.

Left focus matrix826082b0 centers stage at(640,360), incorporates UI jolt,
uses inverse APT camera view and original rotation/distance cvars. Static
initializer82f57ab0 angle default11.25 degrees;82f57b10 distance default624.
Need raw VMX audit + original cvar binding/projection before porting math.
Known render-to-texture world UI path is separate from this layer.

Remaining for actual LB popup: native timeline display list/playback and
authored script hookup; font Vault/localization/button textures; masks/color
transforms/GPU draws; 3D camera; selected input/marker notification integration;
then visible LB/place/release/teleport validation. No parity or playable UI claim.
Raw IDA captures: marker-hud-ida-2026-09-30.txt.

## Display-list and localization continuation

Source confidence high, NOT a completed APT interpreter or live UI. Native
DoTemporaryFrameControls829f9900 / jumpToFrame82a1b3d0 / mergeState82a1a688
ported as ui/apt_timeline. Preserves child creation identity across moves,
folds skipped placements without executing skipped frame actions, removes
later depths on rewind, and queues target actions after display changes.
ReplaceDisplyListItem82a1a4c8 handles character-less moves by updating existing
matrix/colour even when the folded frame differs from creation frame.
Retained merges do not reapply load events/ratios. The service emits script
and lifecycle effects for its consumer; it does not execute or discard them.
Script-created/protected instances and exported Animation characters still
need their owner in the future full display tree. This implementation covers
authored static timeline state, not every dynamic APT instance operation.

ui/apt_playback ports top-level tick82a21c10's millisecond accumulator,
multi-interval catch-up, saved-input single-step exception, stopped clips,
same-frame gotoAndPlay resume and one-frame movie behavior. Nested traversal
and script execution are not wired. Parent Stop MUST NOT stop its children.
Native tick order is timers -> display lists -> actions -> inputs -> linker.
Original multMatrix829dae70 raw829daf1c..3c uses fused fmadds; Rust mul_add
preserves this rather than the decompiler's misleading separate additions.

Independent corpus checks compare every frame of every HUD movie/sprite via
sequential playback, direct jumps and rewinds; also verify hidden/maximized
preserve mQuickOpen, and main2 hides Contacts/help/button while main restores.
Ten controller/timeline/clock tests PASS by compiling production ui/mod.rs
directly with rustc and the built formats library. Log:
artifacts/marker-ui-isolated-tests-20260930.log. Full cargo core unit tests are
blocked by unrelated existing test errors: grind_data_runtime_tests.rs111 uses
Arc::downgrade on GrindSplineOwner; selected_session_public_facade_corpus.rs137
omits SelectedBootProgress::DmoBankPending. No unrelated code changed to mask it.

Added formats/localization.rs. Source8297ede0/8297f128 category chunks39000;
8297e718/829825d0 histogram39001; payloads LITTLE endian, unlike APT/CONSTS/GEO.
Records are sorted unsigned hash+string offset relative to the string table;
record/string tables are relative to chunk payload (+8). ICELITE hash8297e018
seedFFFFFFFF, case-sensitive33*hash+byte, terminates at NUL (not BIG DJB2 seed).
UnpackString829824e8 handles ASCII, direct histogram mappings, escaped mappings,
zero-entry byte fallback, native underscore/error status and capacity boundary.
All10,592 Global and33 FrontEnd English records decode successfully using the
original miscboot.big histogram. Marker keys resolve to Cell Phone, Go To
Marker, Place Marker, Contacts. No English labels hardcoded into runtime.
Two tests PASS; artifacts/marker-ui-localization-tests-20260930.log.

Font source follow-up: FindChar82841250 returns tMissingCharGlyph on missing
code or glyph. cFont constructor8284d098..d2a0 initializes its seven floats to
zero; no replacement box/advance. Raw InitializeFontDetails8282b930 corrected
the decompiler's reversed full64 keys: classfecfbcaf356518c4, font group class
4e432e4cbff3aeca, reference array5a6eb4d57b7f4ea3. Group selected by manager+8:
default14df58a3f395690f, value1 f4c8e199e0319fbe, value2 914ccb8e7018b77b.
First two have six font refs; last debug only. Array header8bytes, stride24.
Font layout offsets and strings remain as above. Actual mappings/constants
captured in marker-hud-font-vault-2026-09-30.txt; DO NOT hardcode those asset
values into runtime. ScalePresto/Glow0.0298999994993, Bold0.0252999998629 are
validation observations only. Index0 overrides base scale0.8 in native code.
Original controller-button arena is screens.big entry
data/fe/source/images/buttons/xbox360/buttons.rx2. Import does not own an RX2.

Continuation native functions: marker-hud-ida-continuation-2026-09-30.txt.
Final full application cargo check (including timeline/playback/Localizer)
PASSED: artifacts/marker-ui-final-check-20260930.log, existing warnings only.
No runtime or new release executable built for UI. Raw left-focus VMX register
audit and top-level tick captured in marker-hud-transform-raw-2026-09-30.txt;
the 3D projection is not yet implemented or claimed verified end-to-end.
Remaining: imported instance/script lifecycle and authored screen class;
font mappings loaded through existing Vault resolver, font/shape/button GPU
draws/masks; raw native perspective/projection; live LB frontend input and
existing marker success notifications; compile and visible test.

## Live binding follow-up, September 30 (verification in progress)

The popup now has a live consumer in `apps/skate2-pc/src/marker_ui.rs`:
retained nested APT clips/imports, original initialize-event assignments,
translated `hudphonelist` class, original Localizer/font/texture assets and
GPU triangle submission. The two translated movie assets are fingerprinted
so a different asset cannot silently run an incompatible class translation.
Unknown frame scripts, imports and clip events fail explicitly. This does
not claim to implement a general ActionScript VM or unrelated screens.

The original marker owner publishes permissions, SIM state29 and a monotonic
successful-set notification in the completed visual snapshot. Set success is
recorded only when existing `SessionMarker::try_set` returns true. Gameplay
marker/teleport implementation is reused. Current free-skate has no career
phone services/other HUD components/audio backend; options and sounds are
not added by inventing replacements.

Additional native checks:
- `ViewMan::Init`82848f30, call8284911c: perspective60deg, near1/far10000;
  copied from camera4 to APT3D camera7 at8284912c.
- `SetPerspectiveMatrixRadians`82840770: vertical denominator is f32 radians
  times f32 2/pi; horizontal is f64 times16/9 narrowed before division.
  X projection is NEGATIVE. Do not replace this with tan(fov/2).
- Left-focus rotation uses the ALREADY PORTED XMVectorSinCos polynomial,
  not libc sine/cosine. `ui/apt_projection.rs` reuses that implementation.
  Default cvars angle11.25/distance624; no UI tuning exposed in this first HUD.
- `FrontEndManager::SetScreenSize`82614300: only non-widescreen calls
  SetScreenEdgeOffset(136); native16:9 layout remains unshifted.
- `AptIntegrationUtils::SetAptTextureMatrix`82a6f508 raw loads confirm direct
  a,b / c,d / tx,ty columns. `SetGameTextureInfo`82609aa0 normalizes U/V by
  texture width/height. Element colour transform is before texture sampling.
- `cFont::SetScale`82849918 multiplies base scales by requested font height.
  `AllocateString`82601fe0 supplies line offsets0,0 and line-height scale1.
  `DrawstringLocal<unsigned short>`82855550 uses (bearingY+ascent)*scaleY;
  missing glyph is zeroed. All six font atlases here are DXT5/32bit.
- `font.xml` final manifest: source alpha/inverse source alpha for BOTH
  colour and alpha; font_FontPS original source multiplies texture*colour.
- UI RX2 assets are STANDALONE RW4 files, not compressed Stream arenas.
  Use existing Arena::split_file_resources + own_texture_arena. The first
  live attempt rejected a mistaken compressed-arena loader; it was corrected.

Validation so far: full app check passed; first release built.17 focused
production core UI/numeric tests pass via artifacts/marker-ui-core-tests.rs
(avoids unrelated preexisting full-core test compilation errors). Texture
probe decodes original HUD/button/six font arenas successfully. Full app
asset test and final live captures remain in progress; do not call this
playable until those checks below are recorded.

Input admission correction: the first live completed-tick attempt exposed
Map(UnsupportedMap(29)). Original InputManager::LoadInputMap826dde58 explicitly
compiles index29 from TopLeftShoulderHeld; original miscboot input.cfg defines
TopLeftShoulderHeld = LBumper.held and LBumper = Button8. Added that definition
to the existing parsed session-marker input group, retaining52/53 unchanged.
No parallel XInput polling or guessed LB test. Fourteen production input-map
and controller tests pass through artifacts/marker-ui-input-tests.rs.

## Visible release verification completed

Release executable built successfully with `cargo build -p skate2-pc --release
-j 1` (marker-ui-input-build-20260930.log). No full-core test compile claim:
the unrelated existing test-only errors listed above remain outside this task.

- 17 production UI/numeric tests pass in the focused harness.
- 14 production input/controller tests pass, including held input29.
- 108 existing application tests pass (3 unrelated ignored tests), recorded in
  marker-ui-other-app-tests-20260930.log.
- Final actual Cargo application test PASSES:
  `cargo test -p skate2-pc --release -j 1 marker_ui -- --include-ignored`.
  Original external movie/assets open, draw button/text geometry, pulse and
  close in the production traversal. Log marker-ui-final-app-tests-20260930.log.
- Live210-tick raw-pad marker-ui sequence exits0; screenshot
  artifacts/marker-ui-open-20260930.png visibly shows original Cell Phone panel,
  Go To Marker / Place Marker, D-pad icons and left-focus perspective. Trace
  confirms successful_sets0->1 and use_allowed false->true after placement.
- Live600-tick sequence exits0: opens, places, releases, reopens, places again,
  holds LB+D-pad Up, releases. successful_sets reaches2. Final screenshot
  artifacts/marker-ui-closed-20260930.png visually confirms popup is gone.
  Trace marker-ui-closed-20260930.log records native command transitions.
  This stationary sequence does NOT verify a distant marker teleport.

Normal play-quiet.bat uses this built executable. Hold LB to open; while held,
D-pad Down places the marker and D-pad Up uses it. Release LB closes. The
scripted raw inputs and SKATE2_TRACE_MARKER_UI flag were diagnostic command-
local settings only, not launcher defaults. Normal quiet mode remains quiet.
No game/compiler process remains running at this checkpoint.

Scope limits: native controller and this original authored movie are ported;
this is not a complete general-purpose ActionScript VM, career phone menu,
all UI masks/screens, or UI sound playback. No reference-game pixel comparison
was performed, so the checked source/asset path and visible result must not be
reported as proof of perfect overall UI parity. Original game assets remain
external and are required at launch.
