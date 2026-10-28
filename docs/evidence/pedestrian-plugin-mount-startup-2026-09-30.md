# Plugin mount startup ordering

Runtime failure: BootPrefix World Props, `graph decoder failed: plugin mounted
assets absent`.

SelectedSession.try_boot_prefix constructed SelectedSessionWorldStartup before
assigning graph_owner.plugin_mounts. Inside that constructor, InitPropData's
actor-tracker initialization already invokes the behavior cache decoder, which
requires those mounts. Therefore the assignment could never be reached on a
successful first plugin load.

IDA recheck of Manager.PreLoadStateGraphs82724280 confirms BehaviourMan.Load
at82725480 precedes InitPropData8272550c. The fix passes the same retained
SelectedOriginalAssets.mounts into world startup and installs its Arc on the
existing graph owner before controller preload and actor-tracker initialization.
Removed the too-late assignment; no second mount, cache, graph owner or decoder
is created, and plugin loading remains mandatory.

Updated the production caller and both existing corpus/test callers. All four
edited files passed syntax parsing; caller search confirms all three calls
provide mounts. No compilation or runtime test was performed.
