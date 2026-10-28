# PC Skate 101, Big Black, Hall of Meat progression and boot textures

2026-10-13. Solo, PC tree only. Source changes.

## Startup failure and replay/photo watermarks

The previous watermark-residency change added both watermark textures to the
loose sky/boot loader. The original **normal** watermark uses `LIN_DXT5`, which
`Texture::decode_levels` did not support. That error propagated to a wrapper
that logged it and discarded the entire collection. The later vignette lookup
therefore reported `original boot vignette_gradient.Texture is not resident`.
The vignette was already requested; adding it again would not fix this.

Original external `miscboot.big` / `data/content/textures/globaltextures.rx2`:

| Texture | GUID | Layout | Dimensions | Mips | Payload |
| --- | --- | --- | --- | --- | --- |
| `logo_watermark.Texture` | `9df5f895b0cdc6fc` | `0x1a200054`, linear DXT5 | 128 x 128 | 1 | 16,384 bytes |
| `logo_watermark_pro.Texture` | `12b77e6baa817dcc` | `0x1a200154`, tiled DXT5 | 128 x 64 | 8 | 49,152 bytes |

The normal watermark's fetch words are
`01000002 00002054 000fe07f 00000d10 00000000 00000200`.
Its GPU payload is dictionary entry 2, resource 2, offset 8192. The encoded
Pitch field is 4: 128 texels, 32 BC3 blocks, 512 bytes per block row, 32 rows.
The endian mode is 8-in-16; the existing normalization is retained.

Live IDA references:

- `renderengine::PixelFormat::PIXELFORMAT_LIN_DXT5 = 0x1a200054`.
- `GetSurfaceLayout` **82c2ef30**, especially **82c2f048**: base-level pitch
  is the fetch Pitch field times 32 texels.
- `GetTextureDesc` **82c2f188**, especially **82c2f550..82c2f5a8**: compressed
  block dimensions, row pitch multiplied by block height, bytes per block.
- `D3D::AlignTextureDimensions` **82c3a720** and
  `D3D::FindSurfaceWithinTexture` **82c3b2d0** corroborate surface allocation.
- `Texture::Serialize` **82bcf540**: fetch serialization / format extraction.
- `PostFX::SetTexturePointers` **828287cc / 82828840** and watermark setters
  **8255d7f0 / 82828610** resolve the original registered global inventory.

Implemented the original linear BC3 row layout, with bounds checks and endian
normalization, and admitted that format in the GPU's BC3 and software expansion
paths. Tiled formats keep their existing decoding. This adds the audited
single-level, unbordered linear 2D layout; it does **not** claim support for
unrecovered linear mip-tail/cube/volume layouts.

Mandatory boot textures now have their own resident collection. An optional
sky/material failure cannot remove them. A boot decoding failure reports its
actual cause at load time instead of a later missing-vignette error. Both
watermarks, vignette, xray and missingtexture remain original asset data.

## Skate 101 background

`FrontEndState_Skate101::OnEnter` **8269fd58** opens screen 135 and pins the
player. Original asset 35 has a 1280 x 720 stage. Its root depth 1/character 3
contains the fullscreen background shape **2** at identity; the panel and
content use other shapes. Original intro/outro changes the background alpha.

Extended the existing PC background-only adaptation to this shape. Horizontal
extension covers wide windows; vertical extension covers narrower aspect
ratios. UVs, authored transforms/fades and foreground layout remain intact.
This is an intentional PC window-size adaptation, not native 16:9 art.

## Big Black physics re-audit

No new mismatch found in the audited path; no physics code changed in this task.

- `Actor::IsSkaterBouncy` **825ed010**: enabled cvar, offline and ghost-system
  exclusions, full 64-bit character ID `0376a7b6e3aadc5c` at **825ed098..ac**.
- Actor/SkeletonBody construction **825ff278 / 82b62594** stores the flag.
  The retained-Actor spawn bridge refreshes it from the actual selected player
  in Free Skate, Career and Party Play, including Change Game.
- `SkeletonBody::SetUpRagdoll` **82b4ffd8** uses that byte OR the global bouncy
  cheat. It reads the live restitution cvar for head then body (default 0.99,
  bits `3f7d70a4`), selects body material for parts 1..23, head for 1/3/7.
  Ordinary characters return to the original Vault materials.
- Collision publication copies/refreshed materials feed the ground producer.
  `AssemblyTriangleIntersectionTester::RunIntersectionTests` **827cd1e0**
  selects minimum restitution and maximum friction, and writes restitution at
  **827cd708**. Original static ground material **82fef110** is `[0, 0, 1]`.
  The port follows that path, so Big Black's restitution is not lost there.
- Skater/DMO contacts retain their separate native zero-restitution cap
  (**827ce7b8..827ce7f4**). It is not a missing Big Black multiplier.

No evidence for extra Big Black push/ollie/gravity/mass multipliers was found.
This is source/reference parity evidence, not a per-tick numeric validation
of the whole host physics implementation or full retained-Actor reconstruction.

## Challenge unlocking and Hall of Meat

Checked activation **825e02c8**, completion **825e00f8**, group activation
**825e0680**, group-exit accounting **825ddcc8 / 825dd850**, prerequisite
filtering **82cd9e88**, and HoM completion **825bfec8** against the existing
producer/consumer paths. Save/reload retains active/completed challenge IDs,
group counts and message settings; replayed completed missions keep the existing
progression guard. No forced unlock or save migration was added.

Read-only original Vault/Lua audit resolves 62 entries (Slappy follow, tutorial,
60 path challenges), all four original 15-entry lists and all 56 successive
win-to-next-challenge links:

1. Winning `m_1a_07_follow_slappy` activates `hom_tutorial`, sends `Unlock Hom`
   and unlocks Slappy's. The ordinary progression handler executes those calls.
2. `hom_tutorial_0` requires at least one bone at `BreakPart` or above. Its win
   machine, `Trigger HOM_tutorial`, activates `f_1a_01_hom` through
   `f_4a_01_hom`.
3. Each path's challenges 1..14 trigger the original next challenge. Terminal
   rewards and repeat behavior remain in the original `HOM_Give_Reward` flow,
   with existing sponsor/character unlock bindings (including Dem Bones).
4. Main challenges, signups and security chases use the existing native HoM
   evaluation exclusions, including the bail-time re-enable restriction in
   **825b30a8**. The earlier PC no-slowdown override remains.

Two omissions fixed:

- The map's Hall of Meat selection returned `Unported`. It now follows
  `TriggerHomPanel` **8265f308** (states 70,40), `DisableHomPanel` **8265f368**
  (71,41 and restored root buttons), and `ReceiveInput` **8266b420**: B returns
  to the same category/cursor; the other inputs are consumed.
- `ObjectiveHOM.IsLastBailTop10` **82d48590 / 828070b8** was still hardcoded
  false. It now reads the implemented stats owner's `last_top_bail` flag.
  The original `global_hom_top10_0` objective uses this query. This flag is not
  replaced with a comparison against a still-accumulating bail score.

Map presentation uses the original `mHallofMeat` character 57 and four path
sprites. `FEChallengeMap.GetHomData` **8261b128** reads lists 70,69,68,67;
counts completed **or active** entries; and formats the first active objective's
HAL key/arguments. APT `SetHomPanelData` **66e4..67bc** writes `#count/total`
and shows the completed stamp at 50% alpha when count equals total. Preserve
that native counting rule, including its active final challenge. With no active
objective the host shows an empty description, not an invented objective.

## Validation and remaining work

- Live debug IDA and original external APT/GEO/Vault/texture metadata audited.
- All seven edited Rust source files pass non-mutating rustfmt syntax parsing.
- Scoped diff reviewed against this task's backups; existing unrelated edits,
  accepted Trick Guide layout and prior intentional overrides preserved.
- **No compilation, Rust tests, game launch, visual comparison or save round
  trip performed.** Runtime confirmation is
  still needed for boot, replay/photo watermark saves, resized Skate 101 and
  Hall of Meat panel entry/return and progression.
- Android and the real save were not edited.

Evidence/backups: `.scratch/skate101-bounce-progression-20261013/`:
`native-reference.json`, decoded APTs, `hom-objectives.json`,
`hom-progression-original.json`, `unlock-audit.json`, `backups.json` and
`scoped-final.diff`. Confidence: high for audited control flow and data; full
runtime visual/numeric parity unverified.

## Subsequent release build

Normal `cargo build --release -p skate2-pc -j 1 --timings` succeeded, exit 0,
790.044 seconds, finished 2026-10-13T16:18:13.2346228+03:00.
This supersedes the uncompiled status above. No source or optimization-setting
changes were needed during the build. The seven source hashes still match the
source-review manifest. Compiler warnings: core 437, PC 18.

`target/release/skate2-pc.exe`: 43,521,536 bytes, SHA256
`88498E6E6C9DD2196502804B9E53E7AF4CFF45D45541F73AF26CAF7B22865DD4`.

No tests or game launch were performed; visual/controller/save behavior remains
unverified in the rebuilt executable. Build log, prior executable/documentation
backups, timing result and artifact manifest are in
`.scratch/skate101-bounce-progression-20261013/pc-release-build/`.
