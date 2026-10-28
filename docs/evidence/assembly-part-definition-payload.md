# Assembly PartDefinition inertia and material payload

Date: 2026-09-06 (Europe/Helsinki)

This checkpoint restores the collision-relevant `rw::physics::PartDefinition`
payload that the native arena parser previously discarded. It is a parsing and
ownership prerequisite for `EA::Physics::SerialisedAssembly`; it does not yet
construct live Parts, rigid bodies, `VolumeData`, or ground Contacts.

## Authority

- Xbox 360 executable SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`
- full PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`
- linker MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`
- supporting DMO/body-binding raw packet SHA-256:
  `8BA5D8692B87B688693B0E9E0E19A8354DA7C4012416CE654A5A5458118F4E8D`
- audited `collision.rs` source SHA-256:
  `288672B134ADE71C10528446F6D42887DAE92666BFE259E4ADACCA24C87562C4`
- audited DMO corpus-probe source SHA-256:
  `CA193E96000F8AD5E0165187FEDD6F8CAA2B75420FCE80CBADDD11D7D84C1747`

## Recovered layout

The PDB fixes `rw::physics::PartDefinition` at 0xa0 bytes. The parser now
retains these previously omitted members without normalizing their floats:

| Part offset | PDB member | Retained value |
| --- | --- | --- |
| `+0x10` | `m_inertia.mInvTens` | all four serialized vector lanes |
| `+0x20` | `m_inertia.mInvMass` | inverse mass |
| `+0x24` | `m_inertia.mSpherical` | spherical scalar |
| `+0x28` | `m_inertia.mMaxVelocity` | maximum velocity |
| `+0x2c` | `m_inertia.mMaxOmega` | maximum angular velocity |
| `+0x30` | `m_inertia.mLinearDrag` | linear drag |
| `+0x34` | `m_inertia.mAngularDrag` | angular drag |
| `+0x80` | `m_material.m_mus` | static friction |
| `+0x84` | `m_material.m_mud` | dynamic friction |
| `+0x88` | `m_material.m_res` | restitution |

`PartDefinition::Initialize` at `0x82ae9058..0x82ae90e8` independently
confirms every offset and the source defaults: four inverse-tensor lanes of
one, inverse mass and spherical one, both maxima `FLT_MAX`, both drags zero,
then material `0.5`, `0.3`, and `0.4`. Those defaults are evidence only; the
parser returns the authored bits instead of substituting them.

`AssemblyDefinition::Serialize` at `0x82ae1650..0x82ae1738` fixes the part-array
pointer relative to the arena base and passes the complete `m_numParts` static
array to the PartDefinition serializer. The existing parser's 0xa0 stride and
base-relative allocation resolution therefore remain unchanged.

## Validation

The synthetic parser discriminator stores distinct raw words in every inertia
slot, including negative zero and both signs of quiet NaN, plus independent
material words and both trailing flags. Assertions compare `to_bits()`, so a
field swap, float normalization, three-lane truncation, or offset drift fails.

The optimized real-data probe reparsed the complete dedicated DMO model bank:
125 arenas contained 93 AssemblyDefinitions and exactly 93 PartDefinitions.
An order-sensitive FNV-1a stream over arena/dictionary identity, volume ID,
all retained inertia/transform/material words, and both flags produced
`6e38d215339bf897`. That fingerprint is now a required corpus gate. All 93
Assembly targets remained type `0x00040006`, and the existing collision
ownership audit reported no errors.

## Confidence and remaining boundary

Field layout, endian parsing, and the observed DMO corpus fingerprint are
`exact`. The fourth inverse-tensor lane is intentionally retained but has no
claimed scalar meaning.

The next owner must keep the decoded arena alive while resolving
Assembly -> AssemblyDefinition -> ordered Part volume occurrences. It must not
use the existing clustered-mesh-only template as its authoritative source,
because that template deliberately omits primitive volumes and these payloads.
Live Part transforms, body occurrences, frozen state, group/user data,
`SerialisedAssembly::Create`/`Update`, and `GroundData` remain separate work.
