# Central HUD InitAction dispatch

Reported `untranslated APT InitAction 17 0x3390`. Original hud2/hudcentral.apt
has exactly one InitAction: root frame0, character17, script3390. Independently
read the control table and verified FNV fingerprintd85e4167 matches Assets.
Existing decoded script is artifacts/central-source/hudcentral-audit.txt.

Script3390 guards global hudcentral, installs its constructor/prototype methods
and constants, and sets property flags. Constructor body33c8..33e0 stores
mScreen/mThis, stops the root and calls RegisterScreen; the initializer defines
that function rather than executing it. Root action3340 creates the instance.
IDA AptCharacterAnimation.ExecuteInitActions829f7f98 dispatches initialization
through ExecuteInitAction at829f8098/829f80c8, distinct from timeline actions.

CentralMovie already owns the translated chase-message flow, but clips.rs's
InitAction dispatch omitted it. Added exact (asset14, character17, offset3390)
recognition, like the existing score/pause/HoM translations. Kept root3340's
constructor stop; removed3390 from ordinary frame-stop dispatch. Unknown
initializers still fail, now including asset index for diagnostic identity.
No duplicate movie or generic unsupported-script bypass was added.

Both edited files passed syntax parsing. No
build or runtime test run for this fix.
