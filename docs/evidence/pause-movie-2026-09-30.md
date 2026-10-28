# StartHub retained movie translation — 2026-09-30

Source only. No compilation, tests, or runtime launch were performed for this
change. Confidence: high for the audited
pause-mode ActionScript, not a claim of full frontend or pixel parity.

Original external archive: `<game folder>\data\fe\screens.big`.
Read-only extraction and `tools/inspect_apt.py --scripts` output are kept
outside the repository.

| Asset | APT FNV32 | Export used | Imported movie |
| --- | --- | --- | --- |
| starthub/starthubmenu | 4381187e | root0 | challenge_info |
| controls/challenge_info | 5273d80c | ChallengeInfo109 | button_menu, button_item2, menu_picker, image_renderer |
| controls/menu_picker | f52d4e28 | Menu_Picker_Component55; ITEMLABEL_Dark31 | none |
| controls/button_menu | 1ee55d34 | Button Menu7 | button_item2 |
| controls/image_renderer | 94517628 | Image_Renderer_Component3 | none |

All placed local characters in the four imported movies are shapes, text or
sprites. No directly placed bitmap characters or positive mask clip-depths
occur. Serialized `0xffffffff` clip-depth means -1, not a mask. button_menu has
no RX2 file; other listed movies have one. Original assets stay external.

StartHub is native screen11 (`gScreenTemplate`82fdb408), template type2
3D_Focus_Left. Root owns projection and state/native integration. Movie4 root
stage is1280x720 at16ms/frame. Root places mHub at(79,192), with challenge_info
at(27.15,-128.4). Its intro flickers alpha over frames1..9, then invokes
IntroComplete; outro frames10..18 plays FE_SOUND_PAUSE_MENU then OutroComplete.
The native unpause policy is independent of those retained fade frames.

`starthubmenu` action0xeb4 SetState0 calls SetMode0 then menu SetValue0/Update.
SetMode0 selects mHub label `pause` and challenge_info mode0. The constructor's
DARK_ON_LIGHT call targets `mHub.mMenu`, absent in the actual display tree;
it therefore does not change the real picker's default LIGHT_ON_DARK skin.
The root's constructor and class actions are0xe98/0xeb4. Timeline0x1108 is End,
0x1114 stops and reports intro completion,0x1138 plays pause audio,0x1158 stops
and reports outro completion. mHub stops at frame16 (`pause`) or34 (`tall`).

`challenge_info` class0x378c, SetMode0x482e and UpdatePositions0x3ec4 hide
nonpause sections, localize ID_PAUSEMENU_PAUSE, copy StartHub.GetNumOptions and
GetOptionText into the picker, then add A/ID_COMMON_SELECT and B/ID_COMMON_BACK
to mView. Cash is shown only outside PartyPlay and FreeSkate; this freeskate
shell keeps it hidden and does not fabricate a career balance. kPAUSE_OFFSET
is168 (0x55dc); header y increases by168, picker y is header bottom+10+168,
and mView y is577. Inaccessible map/signup/betting/challenge branches are not
implemented. Their authored clips remain retained but invisible.

`menu_picker` class0x28e0 initializes wrap=true, selection=-1 and
mItemOffset=dummy0._height. SetNumOptions0x2999 creates exactly the requested
number of attached rows; the eight authored dummy rows do not impose an
eight-row cap. Each dynamic row is ITEMLABEL_Dark31 placed at
index*mItemOffset. Update0x3211 checks native enablement, selects the first
enabled item if current=-1, plays selected/deselected/disabled labels, copies
localized text into itemText and mTextglow.itemText, and sets indicator y to
the selected row's y. NextValueCalc0x3034/PrevValueCalc0x3134 wrap and skip
disabled items. Root/native owns this input selection and feeds the movie.
There is no debug-option filtering in these scripts.

`button_menu` class0x4dc initializes SetInfo(Marker_streak,24,0xffffec), replacing
the authored16px black text. RefreshItems0x8a1 with left justification starts
at authored mText0.x, advances by button width-10, localized text width, then
spacing25. Font atlas and advances come from the existing original font Vault
and bitmap metrics. SetInfo text y is button0.y-textHeight/2-4.
SkateAptString82a70d60 computes textHeight=GetMaxCharHeight()*fontSize;
raw cFont::GetMaxCharHeight82849a80 multiplies Ascent by baseScale.Y (the
decompiler's bare Ascent return loses that vector multiply). TextExtent
82849ba0 accumulates glyph advances times caller baseScale.X*fontSize.
The image_renderer constructor0x284 starts unloaded;
pause mode never invokes image loading. Existing button_item2 custom renderer
is reused for A/B glyphs.

`pause_movie.rs` admits only these fingerprinted class/registration and
lifecycle actions. All remaining authored actions in reachable nested clips
are the shared exact Stop/End handler. Unknown actions/imports still fail.
No option destination is dispatched. Audio events are retained names only,
consistent with the frontend's presently absent sound backend.
