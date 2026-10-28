# First bounded environment shader target

2026-09-07 read-only recovery, source confidence high. No renderer,
shader translation or visual-parity claim is made by this evidence packet.

`environmentsimple.default` maps through existing VLT evidence to effect
`defaultenvironment`, draw order 3. Original `render.big` contains all three
assets under `data\render\shaders\final\`:

| File | Bytes | SHA-256 |
|---|---:|---|
| defaultenvironment.xml | 1671 | FA026DA3B09D480A7242E4C2DF7F37F3C33C9B4ABCCDE91467206273F5070BC8 |
| defaultenvironment_defaultVS.vpo | 896 | ECF885470D91BBD75BFE784765BC5926C7EA91DE8095844D8FF89319D3400433 |
| defaultenvironment_defaultPS.fpo | 1220 | 4B2855C319B918B547E2E289BF919A5A6C4DE47DCC3B64E50981731A958FF90D |

The manifest has one default technique/pass zero, vertex program zero and
pixel program one. Explicit state is CULLMODE=FRONT. `cFxParser` construction
at `0x82e1a4e8` supplies depth enable/write and less-or-equal, stencil disabled,
solid fill, zero depth bias. Blend-state construction at `0x82e13db8` disables
blend/alpha test; the parser enables RGBA writes on target zero.

Lightmap/chromaticity samplers clamp U/V; diffuse retains default wrap. All
three explicitly use anisotropic min/mag, anisotropy four, linear mip and
THREEEIGHTHS trilinear threshold. A fourth reflected shadow sampler has no
manifest block: its actual global binding/state remains to be recovered.

The program files contain Xbox `_UCODE_HEADER`, embedded constant tables,
debugger UPDB path strings and compiled instruction payloads. Embedded targets
are vs_3_0/ps_3_0, compiler 2.0.7645.0, not portable D3D9 shader source.
Reflection was decoded against live PDB and `XGMicrocodeGetConstantTable`
`0x82c40bc0`, which reads header +0x10 and skips the table-length word.

| Stage/type | Parameter | Register/count |
|---|---|---|
| VS float | g_matVP | 0/4 |
| VS float | g_vViewPos | 4/1 |
| VS float | g_FogK1 | 5/1 |
| VS float | g_FogColour | 6/1 |
| VS float | i_monoLightmap_Dot | 7/1 |
| PS float | CSM_Mat_Row0 | 0/2 |
| PS float | CSM_Mat_Row1 | 2/1 |
| PS float | CSM_Mat_Row2 | 3/1 |
| PS float | g_envattributes | 4/3 |
| PS float | m_params | 7/1 |
| PS sampler | shadowTexture_CSM0 | 0/1 |
| PS sampler | i_lightmap | 1/1 |
| PS sampler | i_chromaticity | 2/1 |
| PS sampler | i_diffuse | 3/1 |

`ProgramBuffer::Xbox2CreateConstantTable` (`0x82bca210`) copies register
set/index/count and names. `GetVariableHandleByName` (`0x82bca310`) and parser
`StartElement` (`0x82e1e050`) bind samplers by reflected register, not XML order.
`cEffectList::AddPegasusMaterialToScope` (`0x82e17bb0`) begins at material
channel two, distinguishes numeric union pointers from texture stream pointers,
and selects monoLightmap_Dot from image-channel low two bits.
`cInstanceScope::SetParamPointerByHash` is `0x82e13190`;
`Program::ReadFromFile` (`0x82e149c8`) proves the final-shader virtual path.

Next source work: recover these two small programs' readable formulas and their
VP/fog/environment/material/shadow producers. Render a complete supported
material subset; do not replace this program with a diffuse-only shader and
call it original behavior. Backend winding conversion remains unproven.
Existing scene/texture/queue snapshots can supply actual geometry, transforms,
draw ranges and mip/face data. BC1/BC3 fallback, unresolved global texture
bindings, camera execution and device/resource residency remain separate work.
