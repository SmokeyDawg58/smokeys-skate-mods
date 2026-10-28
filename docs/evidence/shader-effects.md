# Shader effects and material binding

Last updated: 2026-09-02

## Reconstruction boundary

`render.big` contains unusually strong source-level evidence: every effect has a
plain XML manifest naming its techniques, passes, compiled programs, render
states, samplers, and sampler states. The companion `.vpo` and `.fpo` files are
Xbox GPU programs. They may be studied to recover formulas and observable
behavior, but they are not translated into the PC runtime. Native shaders must
be maintained as readable PC shader source.

## Reference control flow

| Reference routine | Address | Recovered behavior |
|---|---:|---|
| `cFxLoader::GenerateEffectHash` | `0x82e12f80` | Produces the FX loader cache key |
| `cMaterialManager::GetAttribulatorKeyFromAttrMaterialName` | `0x82e13310` | Splits `class.variant`, hashes `material_class` and `variant` |
| `cEffectList::AddPegasusMaterialToScope` | `0x82e17bb0` | Copies Pegasus channels into shader scope parameters |
| `cFxLoader::LoadFx` | `0x82e1a898` | Loads a named manifest/effect into an FX handle |
| `cFxLoader::PreLoadAllEffects` | `0x82e1aef0` | Loads the configured global effect-name list |
| `cMaterialManager::CreateEffectListFromEffectName` | `0x82e201d0` | Creates an instance scope for every technique in an effect |
| `cMaterialManager::CreateEffectListFromMaterialKey` | `0x82e203f8` | Reads effect/draw metadata from the material attribute class |

For `environment.default`, the exact reference split is attribute class
`material_environment` and material variant `default`. Runtime
`PresentationMaterialEffect` now retains the serialized name, this split, and
the exact 64-bit class/variant keys used by the native attribute lookup.
The seven corpus records named `model_default` have no dot; they remain explicit
legacy names rather than being guessed into the Attribulator scheme.

PDB types independently establish the ownership hierarchy:

- `cFxHandle` owns named programs and a name-to-technique map;
- each `Technique` owns up to four passes and parameter groups at global,
  effect, material, and instance levels;
- each `Pass` selects vertex/pixel programs plus depth, blend, rasterizer, and
  sixteen sampler-state handles;
- `cEffectList` maps a technique name to its instance scope and vertex-program
  state.

## Native manifest model

`skate2-formats::shader` parses manifests into `ShaderManifest`,
`ShaderProgram`, `ShaderTechnique`, `ShaderPass`, `ShaderSampler`, and
`NamedShaderState`. It validates declared program/technique counts, nesting,
required program bindings, and integer fields. Program names remain external
asset references.

The `--inventory-render-assets` complete-corpus probe proves:

- 124 version `0.01.0` manifests: 62 debug and 62 final;
- 180 referenced vertex programs totaling 177,664 declared bytes;
- 204 referenced pixel programs totaling 272,708 declared bytes;
- zero missing referenced program assets;
- 246 named techniques and 292 passes;
- 2,542 render-state assignments;
- 552 sampler bindings and 3,000 sampler-state assignments;
- zero manifest parse or count failures.

The archive has six deliberate program assets outside manifest program lists:
debug/final `baseenvironment_lodPS`, `baseenvironment_lodVS`, and
`baseenvironment_optimizedVS`. Their names and the reference control flow imply
code-selected environment variants; that dispatch remains to be reconstructed.

## World-material effect census

Every one of the 45,678 BAM materials contains an
`AttribulatorMaterialName` channel. The corpus uses 31 serialized effect names.
The dominant bindings are:

| Serialized material effect | Materials |
|---|---:|
| `environment.default` | 24,057 |
| `environmentsimple.default` | 3,906 |
| `environment.decal` | 3,102 |
| `animated.tree` | 2,280 |
| `dynamicobject.default` | 2,347 |
| `environment.transparent` | 2,213 |
| `environmentsimple.alphatest` | 1,451 |
| `tree.default` | 1,196 |

The runtime-owned Rob's Park scene carries a typed effect key for all 2,377
resident materials. Moving the presentation focus east and back restores all
2,377 keys with the scene.

## Native attribute-vault boundary

The material lookup is no longer represented by a guessed 31-entry table.
`skate2-formats::attrib` now parses the real `skater.vlt`/`skater.bin` pair and
models the symbol-proven vault records as readable Rust.

The 851,568-byte VLT is exactly six big-endian chunks:

| Chunk | Bytes | Recovered role |
|---|---:|---|
| `Vers` | 16 | 64-bit vault version |
| `StrN` | 16 | string-node marker |
| `DepN` | 64 | dependency IDs, name offsets, and names |
| `DatN` | 520,256 | exported object data |
| `PtrN` | 213,552 | pointer-fixup stream and terminator |
| `ExpN` | 117,664 | 4,902 typed 24-byte export records |

The dependency getter routines prove entry zero represents the VLT buffer and
must be skipped by callers. The sole external dependency is the 812,520-byte
`skater.bin`. Its `StrE` header ends at byte 81,536, delimiting 3,966
NUL-separated strings; reflected object storage follows directly and is not a
second VLT-style chunk stream.

The executable's `Attrib::StringHash64` is reconstructed directly as the named
lookup8 mix with seed `0xabcdef0011223344`. It reproduces all three dynamically
registered export type IDs:

| Export type | Type ID | Records | Serialized bytes |
|---|---:|---:|---:|
| `Attrib::DatabaseLoadData` | `0x0b38846845e9c175` | 1 | 2,716 |
| `Attrib::ClassLoadData` | `0x2a7895ac4a876152` | 304 | 12,160 |
| `Attrib::CollectionLoadData` | `0xad303b8f42b3307e` | 4,597 | 505,360 |

Every class export has the exact 40-byte PDB layout and its embedded class ID
equals the export object ID. The decoded headers describe 4,706 attribute
definitions. Collection headers cover 4,597 collections across 252 loaded
classes, including 3,907 parented collections and 6,952 explicit entries.
The pointer stream contains 13,345 validated records: two kind-2 dependency
base selections and 13,343 kind-3 direct fixups, followed by the padded
terminator used by `Vault::Initialize`.

All 26 generated `ClassRefSpec_material_*` names found in the BIN hash to real
VLT class exports. More importantly, complete BAM validation hashes every
dotted material name using the recovered split and finds its exact
`(class_id, collection_id)` pair in the VLT: 45,671 of 45,671 attributed
materials match, with no missing bindings. The only remaining records are the
seven explicitly preserved legacy `model_default` names.

The native fixup pass represents all relocated references as logical
dependency/offset pairs. All 13,343 direct fixups resolve in bounds; no host
pointer is written into the VLT or BIN. Class definitions and collection inline
tables then provide a generic attribute lookup supporting static data, layout
data, explicit entries, and parent inheritance.

`CreateEffectListFromMaterialKey` initializes its effect name to the literal
`default` and replaces it only for a non-null `effect` attribute. Applying that
rule resolves every attributed corpus material. The 30 family mappings are:

| Serialized family | Requested effect |
|---|---|
| `advertisement.default` | `advertisement` |
| `animated.flag` | `vertexanimate` |
| `animated.tree` | `treeanimate` |
| `building.default` | `building_ceiling_worldspace` |
| `decal.default` | `default` |
| `dynamicobject.alphatest` | `alphatestdynamicobject` |
| `dynamicobject.default` | `dynamicobject` |
| `environment.decal` | `decalenvironment` |
| `environment.default` | `baseenvironment` |
| `environment.transparent` | `transparentenvironment` |
| `environmentsimple.alphatest` | `alphatestdefaultenvironment` |
| `environmentsimple.default` | `defaultenvironment` |
| `glare.default` | `default` |
| `godray.spotlight` | `godrayspotlight` |
| `incandescent.backlit` | `baseincandescent` |
| `incandescent.backlituvscroll` | `scrollincandescent` |
| `incandescent.backlitzbias` | `zbiasincandescentbacklit` |
| `incandescent.default` | `baseincandescent` |
| `incandescent.incandescentzbias` | `zbiasincandescent` |
| `incandescent.transparent` | `transparentincandescent` |
| `incandescent.videoscreen` | `videoscreen` |
| `ocean.default` | `default` |
| `proxyworld.default` | `default` |
| `terrain.decal` | `decalterrain` |
| `terrain.default` | `baseterrain` |
| `trafficlight.one` | `trafficlight_one` |
| `trafficlight.two` | `trafficlight_two` |
| `tree.default` | `default` |
| `water.default` | `water` |
| `water.flowing` | `flowingwater` |

For the five rows requesting `default`, this is the exact factory input rather
than proof that a corresponding manifest is present. Exact virtual-path probes
find no `data\render\shaders\final\default.xml` in the supplied mounts. The
runtime impact and typed deferred-submission policy are documented in
`render-queue.md`.

## Draw-order reference resolution

PDB layout records prove `DrawOrder` is an `Attrib::RefSpec` with a 64-bit class
key, 64-bit collection key, and runtime-only collection pointer. The generic
attribute-location resolver reads the two serialized keys, finds that target
class and collection, resolves its `GUID` definition with the same
static/layout/entry/inheritance rules, and reads the big-endian integer.

All 30 material families resolve with no missing reference or value. Their
draw-order values are:

| Draw order | Serialized families |
|---:|---|
| 0 | `water.flowing` |
| 1 | `dynamicobject.alphatest`, `trafficlight.one`, `trafficlight.two`, `tree.default` |
| 2 | `animated.flag`, `animated.tree` |
| 3 | `advertisement.default`, `building.default`, `environment.decal`, `environment.default`, `environmentsimple.default`, `incandescent.backlit`, `incandescent.backlituvscroll`, `incandescent.default`, `incandescent.videoscreen`, `ocean.default`, `terrain.decal`, `terrain.default`, `water.default` |
| 4 | `decal.default`, `incandescent.backlitzbias`, `incandescent.incandescentzbias` |
| 6 | `proxyworld.default` |
| 8 | `dynamicobject.default` |
| 11 | `environment.transparent`, `environmentsimple.alphatest`, `glare.default`, `godray.spotlight`, `incandescent.transparent` |

`CreateEffectListFromMaterialKey` later multiplies this integer by 1,000 when
constructing an uninitialized technique sort key. The recovered draw order now
feeds the native packed sort model; replacing Xbox program assets with readable
native shader source remains outstanding.

The runtime-owned presentation material now contains a separate typed binding
with the resolved manifest name and draw order. The Rob's Park scene binds all
2,377 materials; its adjacent streamed focus set binds all 2,034. Returning to
Rob's Park restores the exact serialized name, manifest name, and draw order for
every material according to the strengthened presentation-scene signature.
