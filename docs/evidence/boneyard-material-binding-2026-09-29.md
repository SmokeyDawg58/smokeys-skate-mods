# Boneyard startup material binding

Selecting Boneyard (`F_SH_Boneyard` in original `BAM_Sim.loc`) failed with
`invalid native render queue: UnboundEffect` at startup.

Read-only Jan17 asset audit used BIG header/range reads and existing RefPack
decoder. The original presentation focus query at
`[870.084717, 41.853645, -182.033508]` includes `cPres_1100_100_high`.
Its asset `8bea8102f95657c9` contains the legacy material name `model_default`.
Earlier full-corpus audits already classified seven such undotted materials.
The world binder skipped these because they have no parsed class/variant pair;
the render queue then required a binding and failed.

Live IDA evidence:

- `GetAttribulatorKeyFromAttrMaterialName` `82e13310`: only dotted names write
  class/collection keys; an undotted name leaves the output keys untouched.
- `CreateEffectListFromPegasusMaterialUnSkinned` `82e20948`: null metadata name
  selects `defaultenvironment.default`; after the collection lookup, a null
  collection selects `material_defaultenvironment` / `default`
  (`82e209d0..82e209e8`), then uses the normal effect-list/material-channel path.
- `CreateEffectListFromMaterialKey` `82e203f8`: obtains actual effect name and
  draw order from the Vault. No manually selected shader parameters are needed.

Change: the existing world-scene binder resolves an absent parsed key pair to
that original default collection before gathering Vault keys. Serialized names
remain intact for diagnostics. The parser itself and character/vehicle binding
paths remain unchanged. Initial loading and moving city loading already share
this binder. This models unresolved legacy keys explicitly; it does not emulate
the original unsplit-name path's uninitialized stack bytes. Dotted keys continue
through the existing strict collection lookup.

Future UnboundEffect errors now include the original material name, instead of
an anonymous marker. No continuous logging was enabled.

UNCOMPILED / UNRUN. Scoped render.rs
rustfmt succeeded, all UnboundEffect references were reviewed. No game launch,
Cargo, or tests were run. This fixes the identified binding gap; visual results
at Boneyard still need the next playtest.
