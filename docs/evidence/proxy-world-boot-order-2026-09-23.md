# Proxy world boot order

Read-only IDA/caller audit; no runtime implementation or Cargo invocation.

## Conclusion

An empty constructed proxy manager is **not** the default first-world-stream
producer. Native boot first loads and activates the proxy world when the live
`kEnableProxyWorldStream` setting is enabled. Its authored default is true.
An explicitly disabled path exists, but also selects different earlier loose
asset loads. This audit establishes source defaults and branches, not the value
of any externally overridden setting at runtime.

## Exact chronology

- `cGameAssetManager` constructor `824dfb68` calls the asset-stream-system
  constructor at `824dfc54`; `cAssetStreamSystem` constructor `824de440` constructs
  its retained proxy manager at `824de780` (`cProxyWorldManager` ctor `824ddef0`).
  The constructor-empty maps therefore exist before either boot branch.
- `GameModule::OneOffLoad` `8273add8` initializes Simulation (`8273b120`) and
  Presentation (`8273b15c`), then calls `OneOffLoadTexture` at `8273b194`.
- `OneOffLoadTexture` `82735730` reads the proxy cvar at `82735828..830`. If false,
  it adds `data\\content/world/models/DIST_ProxyWorld_Textures.rx2` at `8273584c`
  and `data\\content/world/models/DIST_ProxyWorld.rx2` at `82735860`, then calls
  the existing loose-asset loader at `82735868`.
- Back in `OneOffLoad`, Physics World Reset is at `8273b3f8`.
- `8273b41c` loads the live proxy cvar target; `8273b420` loads its byte. The false
  branch `8273b428` skips loading to `8273b494`. Otherwise `8273b474` calls
  `cAssetStreamSystem::SyncLoadProxyWorldStream` (`824e0710`), which forwards to
  the same manager's `SyncLoadProxyWorldStream` (`824df648`). Arguments are
  `data\\content\\world\\stream` and `PROXY`.
- After loading, `8273b490` reloads the cvar target. `8273b494` reads the byte
  again; if enabled `8273b4e0` calls `SyncActivateProxyWorld` through wrapper
  `824daef0` to manager method `824d9ef8`. The load and activation gates are
  distinct live reads, not one cached bool for all boot work.
- Only then does `8273b524` call `StartWorldStreamLoad` (`8272d0c0`) with the
  actual backend `worldFile`; `8273b528` stores its handle. The normal outer
  `UpdateGameStreamFileLoaded` loop follows at `8273b550`, then streamer location
  initialization `8273b58c`. `ResetGame` occurs much later at `8273bb9c`.

The direct caller search found `8273b474` as the wrapper's caller and
`824e0714` as the manager load's caller. High metadata effects originate in
`cStreamFile::Update` (`824e0ba8`), calling `AddHighCollection` at `824e11c8`.

## Completion is not merely map population

`SyncLoadProxyWorldStream` first loads/waits for the Presentation stream
(`824df6b8..714`), then reads its XSS (`824df72c..780`). Count comes from XSS+16;
rows start at +24 with stride80. It constructs/sorts the spatial order, then
for each sorted ID inserts a zero counter into `mProxyCollectionMap`
(`824dfa64`), calls the actual `LoadCollection` (`824dfa78`), and retains the
ID (`824dfa88`). It waits for all load callbacks (`824dfab8..fac0`). Actual
`cLoadProxyWorldRequestCallback::OnComplete` `824d68a8` increments the same
manager's loaded count at `824d68b4`.

`SyncActivateProxyWorld` asserts the loaded count first, sends two Presentation
messages per retained ID (`824d9fe4`, `824da080`, the second carrying a real
activation callback), then waits for the actual activated count
(`824da0cc..a100`). Neither reading XSS nor inserting map keys proves this wait
complete.

`AddHighCollection` `824dc678` formats signed coordinates as
`cPres_%d_%d_high_proxy`, hashes the text with `rw::RwHash64String`, tests the
proxy collection map, and only on a hit inserts highID-to-proxyID into the
information map (`824dc6c0..c704`). No hit is a legitimate no-op, but inventing an
empty input map would skip effects owed by an enabled, loaded boot.

## Reuse and smallest missing collaborator

Reuse `skate2_formats::hash::rw_hash64_string`, the existing `CollectionSpace`
XSS parser, catalog/mounted collection loader, and retained stream machinery.
Do not fork those parsers or instantiate the Simulation-only residency builder
for Presentation assets.

The missing collaborator is a retained **Presentation proxy collection owner**
with genuine load and activation completion, shared with `WorldStreamEffects`.
A map-only metadata adapter is insufficient to claim enabled boot completed.
For an explicitly selected disabled setting, the actual constructor-empty owner
can legitimately service high lookups as misses, provided the caller preserves
the disabled loose-asset path and does not claim it is the authored default.
No empty-map, no-op callback or replacement subsystem was implemented here.

Cvar provenance: initializer `82f65df0`, table slot `82fd1d94`, reference
`830504a8`; section `Stream`, label `kEnableProxyWorldStream`, title `Enable Proxy
World Stream`. Raw `82f65e14` sets callback0, `82f65e18` sets default1, and
`82f65e1c` invokes the bool constructor.
