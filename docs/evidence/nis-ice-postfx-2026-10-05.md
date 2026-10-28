> Agent research (2026-10-05), read-only IDA (session 07f41359, sk82_na_m.xex). Nothing in the IDB was renamed,
> retyped, commented or saved. Bloom threshold/intensity and the fade colour (PostFX::SetParameters ICE path
> 0x8284c104..0x8284c32c, see nis-postfx-letterbox-2026-10-04.md and live_skating_view.rs `nis_postfx`) are
> already ported and were not re-derived. Items marked UNVERIFIED were not fully proven.

# NIS ICE post-fx overrides: ICE channels -> mICEPostfxData -> GPU

All addresses are guest (PPC) addresses. "P" is `Sk8::Presentation::msInstance` (global 0x83054640).

## 0. Method and conventions

- Channel handlers, PostFX CPU code and the reader inventory come from IDA decompile and raw disassembly.
- VMX: IDA prints the VA operand of VX128 ops (vmulfp128, vaddfp128, vor128, ...) 32 too high. The register fields
  were decoded from raw words with a scratch decoder (Xenia VX128 field layout). VA-form `vmaddfp` is printed in
  encoding order `vD, vA, vB, vC` and computes `vD = vA*vC + vB` (checked on 0x8284c064 = 0x11606B2E).
- ApplyFX_NEW 0x82823f08 was replayed symbolically from the raw PPC (every stack store, VMX load, splat and
  multiply-add). The resulting matrix equals the port's existing `render_postfx_math::colour_matrix` for all inputs.
- The post-fx pixel shaders are Xenos microcode embedded in the XEX. render.big has no post-fx shaders: its 900
  entries are only material effects (`data\render\shaders\{debug,final}\*.xml/.fpo/.vpo/.updb`). The blobs were
  dumped from IDA (exact sizes, 23 spot-checked words all match IDA) and disassembled with tools/xenos_disasm.py.
  CTAB names give the constant registers. The pseudo-HLSL below is reconstructed from microcode. It is not
  original source, because none ships.
- Census: a scratch Python re-implementation of crates/skate2-formats/src/ice_camera.rs (`Take::parse`,
  `Element::decode`, `key_index`) over the shipped `data\scene\ice_000.bin` (from scene.big). Its override counts
  reproduce the caller's census exactly.

## 1. Data and defaults

`Presentation.mPostfxPresentation` = P+0x7940. `mDefaultICEPostfxData` = P+0x7940, **`mICEPostfxData` = P+0x7A70**,
`mICEPostfxActive` = P+0x7C30. `ICEPostFX_t` (0x130 bytes), offsets inside the struct, defaults from
`PostfxPresentation::SetDefaults` 0x827eff00:

| off | field | default (store) | off | field | default (store) |
|---|---|---|---|---|---|
| 0x00 | nisNear.brightness | 0 (0x827effb0) | 0x40 | nisFar.brightness | 0 (0x827effe4) |
| 0x04 | nisNear.contrast | 1 (0x827effb8) | 0x44 | nisFar.contrast | 1 (0x827effe8) |
| 0x08 | nisNear.contrastMidpoint | 0.5 (0x827effc0) | 0x48 | nisFar.contrastMidpoint | 0.5 (0x827effec) |
| 0x0C | nisNear.greyscale | 0 (0x827effc8) | 0x4C | nisFar.greyscale | 0 (0x827efff0) |
| 0x10 | nisNear.saturateIntensity | 0 (0x827effd0) | 0x50 | nisFar.saturateIntensity | 0 (0x827efff4) |
| 0x20 | nisNear.saturateColour | (1,1,1,0) (0x827f00f0) | 0x60 | nisFar.saturateColour | (1,1,1,0) (0x827f00f4) |
| 0x30 | nisNear.negative | 0 (0x827effd4) | 0x70 | nisFar.negative | 0 (0x827efff8) |
| 0x34 | nisNear.postModulate | 1 (0x827effd8) | 0x74 | nisFar.postModulate | 1 (0x827efffc) |
| 0x38 | nisNear.postBrightness | 0.05 (0x827effdc) | 0x78 | nisFar.postBrightness | 0.34 (0x827f0000) |
| 0x3C | nisNear.postQuantise | 8 (0x827effe0) | 0x7C | nisFar.postQuantise | 255 (0x827f0004) |
| 0x80 | nisFarStart | 150 (0x827f0008) | 0x84 | nisFarEnd | 40 (0x827f000c) |
| 0x88 | tonemapValue | 0 (0x827f0010) | 0x8C | motionBlurValue | 1 (0x827f0014) |
| 0x90 | motionBlurObjectApply | 0 (0x827effb4) | 0x94/0x98 | bloomThreshold/Intensity | 1.4/0.5 (ported) |
| 0xB0 | noiseIntensity | 0 (0x827f0034) | 0xC0 | noiseColour | (1,1,1,0) (0x827f00f8) |
| 0xD0..0xE0 | noiseScaleX/Y, ScrollRate/X/Y | 1 each (0x827f0038..0x827f005c) | 0xE4 | fxMode | 0 (0x827effbc) |
| 0xE8 | fxModeValue | 0 (0x827f0064) | 0xEC | screenWipeValue | 0 (0x827f00b0) |
| 0xF0/0xF4 | fisheyeAdjustH/VRatio | 1/1 (0x827f00b4/b8) | 0xF8/0xFC | fisheyeH/VUpscaler | 0.9/0.9 (0x827f00bc/c0) |
| 0x100/0x104 | fisheyeH/VWarpMultiplier | 0.3/0.3 (0x827f00c4/c8) | 0x108 | fisheyeVignetteMultiplier | 0.26 (0x827f00cc) |
| 0x10C | fisheyeVignettePower | 4 (0x827f00d0) | 0x110 | fisheyeWarpMultiplier | 1 (0x827f00d4) |
| 0x114 | fisheyeWarpPower | 3 (0x827f00d8) | 0x118 | scanlineOffset | 0 (0x827f00dc) |
| 0x11C | radialBlur | 0 (0x827f00e0) | 0x120 | characterEffectMaskValue | 0 (0x827f00e4) |
| 0x124 | tintEnable | 0 (0x827f009c) | 0x125 | cheatEnable | 0 (0x827f00a4) |

`PostfxPresentation::PostfxPresentation` 0x827f6610 copies the defaults into `mICEPostfxData` once
(`ICEPostFX_t::operator=` 0x827f2ef0, call 0x827f667c). `operator=` has only one other caller (SetParameters' stack
copy, 0x8284bec4). No per-scene reset of `mICEPostfxData` was found (UNVERIFIED: an inlined copy is not excluded).
The channels rewrite most fields every frame anyway.

DOF lives in `CameraPresentation` (object at P+0x11F50): `mICEDOFDepth` +0x10DC (P+0x1302C), `mICEDOFFocalDistance`
+0x10E0 (P+0x13030), `mICEDOFBlurFalloff` +0x10E4 (P+0x13034), `mICEDOFBlurIntensity` +0x10E8 (P+0x13038). The
getters `GetDOFDepth/FocalDistance/BlurFalloff/BlurIntensity` 0x827ef918/0x827ef940/0x827ef968/0x827ef990 return
these when `mICECameraActive` (+0x1042) or `mNISSequenceCameraActive` (+0x1170) is set, else `camDOF*` (+0x2C..+0x38).

## 2. Update path and order

- `SceneCamera::Init` 0x82874528 -> `Sk8::ICE::Init` 0x8286a6d8 -> `ICE::RegisterChannels` 0x8286a1d0 registers the
  full-ICE handlers in this order: FXMode, Lens, Focus, RawFocus, Fade, LightRig, LetterBox, Tonemap, MotionBlur, Bloom,
  Noise, FishEye, NearFar, Brightness, Contrast, Saturate, Tint, ScreenWipe, Shadows, ShadowBias, ShadowPrecision,
  Scanline, RadialBlur, SunDirection, CharacterEffectMask, AudioEffect, AudioMusic, AudioPost, AudioSpeech, Overlays,
  Freeze. `ICEChannelManager::RegisterChannel` 0x82a56f08 appends to the tail.
- `ICECameraMover::UpdateFrameEnd` 0x82a3b2e0 (only with a take, and not while the active scene is queueing) calls
  `ICEChannelManager::UpdateFrameEnd` 0x82a37c20 at 0x82a3b40c. That function calls every handler's UpdateFrameEnd
  from the head, in registration order, every frame. So Focus always runs right before RawFocus.
- Handlers read the playing take's current values: `ICETake::GetValueFloat` 0x82a365e8 and `GetValueInt` 0x82a36650
  (the port's `Playback::float/int`). An element whose channel has no intervals reads its element default.
- Twin path: `SceneLiteCamera::Init` 0x82879940 -> `ICELITE::RegisterChannels` 0x82878ca8 registers `ICELITEUserChannel*`
  handlers, which write the same fields. That set also has Greyscale/PostModulate/PostQuantise/PostBrightness (0x826d5628,
  0x826d5e58, 0x826d5f90, 0x826d60c8) and lacks Scanline, RadialBlur and FXMode. The shipped NIS path (ScenePlayer ->
  Scene, see nis-playback-flow-2026-10-04.md) and the port's 146-element table are the full-ICE set.
- Frame order: `GameModule::Render` runs Presentation::Update, then SceneManager::UpdateFrameEnd (these channels), then
  RenderMan::Render (consumers, section 5). Values are consumed in the same frame (nis-postfx-letterbox 1.7).

## 3. Channel handlers (question 1)

Element names and ids follow crates/skate2-formats/src/ice_camera.rs `ELEMENTS`. "Off" means the override element's
current interval value is 0. "D.x" means the `mDefaultICEPostfxData` field.

**Ch 2, Focus, `ICEUserChannelFocus::UpdateFrameEnd` 0x826c7878.** This is context for DOF and has no override
element. It reads APERTURE (16), FOCAL_DISTANCE (15) and LENS_LENGTH (9). If aperture is in [0,37) it derives near and
far limits through `ICE::ConvertApertureNumberToFStop`. Otherwise it writes 0 to all four DOF fields every frame
(calls at 0x826c79f8, 0x826c7a10, 0x826c7a28 and 0x826c7a40 = vtable +0x30/+0x2C/+0x34/+0x38). No shipped take has
channel-2 data, so APERTURE is always its default 37 and this handler always zeroes the DOF fields.

**Ch 3, RawFocus, 0x826c7bd0.** The gate is RAWFOCUS_OVERRIDE (104), read at 0x826c7c10. When it is on:

| value | User_ICECamera call | field |
|---|---|---|
| `max((NEAR_FOCUS 18 + FAR_FOCUS 19) * 0.5, 0)` | `SetFocalDistance` 0x826d8298 (+0x30, 0x826c7cb0) | `mICEDOFFocalDistance` |
| `max(FAR - NEAR, 0)` | `SetDepthOfField` 0x826cd1b0 (+0x2C, 0x826c7cc8) | `mICEDOFDepth` |
| `BLUR_FALLOFF 20 * 2` | `SetDOFFalloff` 0x826cd1c8 (+0x34, 0x826c7ce0) | `mICEDOFBlurFalloff` |
| `BLUR_INTENSITY 21 * 3` | `SetDOFMaxIntensity` 0x826d82b0 (+0x38, 0x826c7cf8) | `mICEDOFBlurIntensity` |

The camera vtable is `User_ICECamera` 0x820b32f8. When the override is off, the handler writes nothing, so the
Focus channel's zeros stand. `ICEUserChannelRawFocus::Init` 0x826c7b20 also writes zeros.

**Ch 10, LightRig, 0x826cdaa0.** No override; it writes every frame into `mLightRigPresentation.mICELightRigData`:

| element | field |
|---|---|
| LIGHT_RIG_DIR_U 28 | P+0x7C7C |
| DIR_V 29 | P+0x7C80 |
| KEY_INTENSITY 30 | P+0x7C84, multiplied by 1.7 (`__real@3ffb333333333333`, 0x826cdb4c) when cvar `kNISLightRigHacks` is set (default true, initializer 0x82f6d940 `li r7,1`) |
| SEC 31 | P+0x7C88 |
| FILL 32 | P+0x7C8C |
| RIM 33 | P+0x7C90 |

With no channel data it writes the defaults (0, 0, 1.7, 1, 1, 1). This is lighting, not post-fx. The reader is
`RenderMan::UpdateLighting` (P+0x7C60 taken at 0x82833d8c). It was not decoded (UNVERIFIED).

**Ch 11, Tonemap, 0x826c85a0.** The gate is TONEMAP_OVERRIDE (109). When on, `tonemapValue` (P+0x7AF8) =
TONEMAP_VALUE (34) (0x826c85e0). When off, it gets D.tonemapValue = 0 (0x826c85fc/0x826c8600).

**Ch 12, MotionBlur, 0x826c8710.** The gate is MOTION_BLUR_OVERRIDE (110). When on, `motionBlurValue` (P+0x7AFC) =
MOTION_BLUR_VALUE (35) (0x826c8754) and `motionBlurObjectApply` (P+0x7B00) = (MOTION_BLUR_OBJECTAPPLY 111 != 0)
(0x826c8770). When off, the value gets D.motionBlurValue = 1 and objectApply = 0 (0x826c8784..0x826c878c).

**Ch 13, Bloom, 0x826c88e0.** Already ported (BLOOM_OVERRIDE 112 -> 37 threshold, 36 intensity). When off, the
threshold is reset only while no bloom pulse runs (0x826c8948).

**Ch 14, Noise, 0x826c8aa0.** No override element; the gate is cvar `kNISNoiseEnabled`, read at 0x826c8ac4. Its
default is **false** (initializer 0x82f5bfe0 `li r7,0`). When enabled, it writes:

| element | field |
|---|---|
| NOISE_INTENSITY 40 | P+0x7B20 |
| SCALEX 41 | P+0x7B40 |
| SCALEY 42 | P+0x7B44 |
| SCROLLRATE 43 | P+0x7B48 |
| SCROLLX 44 | P+0x7B4C |
| SCROLLY 45 | P+0x7B50 |

When disabled it writes nothing, which is the retail behaviour.

**Ch 15, FishEye, 0x826c8e70.** The gate is FISHEYE_OVERRIDE (115). Let `m = FISHEYE_MULTIPLIER(47) * 4.7619047`
(`__real@40986186` = 1/0.21, 0x826c8ef8).

| field | on | off |
|---|---|---|
| AdjustHRatio P+0x7B60 | FISHEYE_HORIZONTAL 46 (0x826c8eb4) | D (1) |
| AdjustVRatio P+0x7B64 | FISHEYE_VERTICAL 48 (0x826c8ec4) | D (1) |
| WarpMultiplier P+0x7B80 | m | D.WarpMultiplier (1) |
| HWarpMultiplier P+0x7B70 | m | **D.WarpMultiplier** (1, not D.HWarp 0.3; 0x826c8f90) |
| VWarpMultiplier P+0x7B74 | D.VWarp * m / D.Warp | D.VWarp |
| VignetteMultiplier P+0x7B78 | D.Vignette * m / D.Warp | D.Vignette |
| H/VUpscaler, VignettePower, WarpPower | D values | D values |

The "on" stores are at 0x826c8f08..0x826c8f34 and the "off" stores at 0x826c8f74..0x826c8fa8. Only WarpMultiplier
has a consumer (section 4).

**Ch 16, NearFar, 0x826c9108.** The gate is NEAR_FAR_OVERRIDE (116). When on, **`nisFarStart` (P+0x7AF0) =
INTERPOLATION_NEAR (50)** (0x826c914c) and **`nisFarEnd` (P+0x7AF4) = INTERPOLATION_FAR (49)** (0x826c915c). Ids
49 and 50 were checked in `ICE::RegisterElements`: tag "INTERPOLATION_FAR" -> id 0x31 at 0x82a5103c, "_NEAR" -> id
0x32 at 0x82a510c4. When off, it writes the defaults 150 and 40 (0x826c916c..0x826c9178).

**Ch 17, Brightness, 0x826c8cc0.** The gate is BRIGHTNESS_OVERRIDE (117). When on, nisNear.brightness (P+0x7A70) =
BRIGHTNESS_NEAR (52) and nisFar.brightness (P+0x7AB0) = BRIGHTNESS_FAR (51). When off, both get the defaults (0).

**Ch 18, Contrast, 0x826c92c8.** The gate is CONTRAST_OVERRIDE (118). When on:

| element | field |
|---|---|
| CONTRAST_NEAR 55 | nisNear.contrast P+0x7A74 |
| CONTRAST_NEAR_MIDPOINT 56 | nisNear.contrastMidpoint P+0x7A78 |
| CONTRAST_FAR 53 | nisFar.contrast P+0x7AB4 |
| CONTRAST_FAR_MIDPOINT 54 | nisFar.contrastMidpoint P+0x7AB8 |

When off, they get the defaults 1, 0.5, 1 and 0.5.

**Ch 19, Saturate, 0x826c94f0.** The gate is SATURATE_OVERRIDE (119). When on, nisNear.saturateIntensity
(P+0x7A80) = SATURATE_NEAR (58) and nisFar.saturateIntensity (P+0x7AC0) = SATURATE_FAR (57). When off, both get 0.

**Ch 20, Tint, 0x826c96a0.** The gate is TINT_OVERRIDE (122). When on:
- nisNear.saturateColour (PostfxPresentation+0x150) = `(HSV2RGB(XYZ2HSV(X 59, Y 60, Z 61)), 0)` (0x826c9774).
- nisNear.negative (P+0x7AA0) = (TINT_NEAR_NEGATIVE 120 != 0) ? 1.0 : 0.0 (0x826c97a4).
- The far side does the same from 62/63/64: saturateColour at +0x190 (0x826c9838) and negative at P+0x7AE0 from
  TINT_FAR_NEGATIVE 121 (0x826c9860).

When off, the defaults are copied: colours (1,1,1,0) and negative 0 (0x826c9874..0x826c98a8). `tintEnable` is never
written (no store to +0x124 anywhere). The colour conversion is:
- `ICE::XYZ2HSV` 0x82a34040: `h = ICE::ATan(Y, X)/65536` (u16 angle from the `FastATanTable` lookup 0x82a33628;
  this is atan2(X, Y) in turns), `s = min(sqrt(X^2+Y^2), 1)`, `v = Z`.
- `ICE::HSV2RGB` 0x82a54200: standard sextant HSV with `i = (int)(6h)`. Cases 0 and 6: (v,t,p); 1: (q,v,p);
  2: (p,v,t); 3: (p,q,v); 4: (t,p,v); 5: (v,p,q).

**Ch 21, CharacterEffectMask, 0x826c9b58.** The gate is element 123. When on, `characterEffectMaskValue`
(P+0x7B90) = element 65. When off, it gets the default 0.

**Ch 22, FXMode, 0x826c9cc8.** Ungated: `fxMode` (P+0x7B54) = int FXMODE (124) and `fxModeValue` (P+0x7B58) =
FXMODE_VALUE (66).

**Ch 23, ScreenWipe, 0x826c9ec0.** Ungated: `screenWipeValue` (P+0x7B5C) = SCREEN_WIPE_VALUE (67).

**Ch 24, Shadows, 0x826c9f60.** Not post-fx. When SHADOW_OVERRIDE (125) is on, it writes
`RenderMan.m_PostFX.m_NISooScale` (RenderMan+0x6E0+0xB64) = SHADOW_SCALE 68 and `m_NISooDirOfset` (+0xB70) =
(X 71, Y 69, Z 70, 0). When off it writes nothing.

**Ch 29, SunDirection, 0x826ca610.** Not post-fx. It writes `m_SunLightMan.mManualModeNIS` = (131 != 0) and a
direction at +0xA0 from Euler angles 78/79/80 in degrees; the direction is 0 when the override is off.

**Ch 27, Scanline, 0x826ca388.** The gate is SCANLINE_OVERRIDE (127). When on, `scanlineOffset` (P+0x7B88) =
`v - (float)(int)v` (truncation, `fctiwz` 0x826ca3bc), where `v` = SCANLINE_Y_OFFSET (74). When off, it gets the
default 0 (0x826ca408).

**Ch 28, RadialBlur, 0x826ca4a0.** The gate is RADIALBLUR_OVERRIDE (129). When on, `radialBlur` (P+0x7B8C) =
RADIALBLUR (76). When off, it gets 0 (0x826ca4fc).

## 4. Consumers (question 2)

### 4.1 Complete reader inventory

Searches covered the whole .text 0x82430000..0x82FC2000, in chunks:
- every absolute `0x7Axx/0x7Bxx(rN)` displacement,
- every `0x7C30(` and `lbz 0x2F0(` selector,
- every `addi rX, rY, 0x7940` base.

The only readers of `mICEPostfxData` are:

1. `PostFX::SetParameters` 0x8284bbf0. It sets r29 = ICE or default block by `mICEPostfxActive` (0x8284beac/0x8284beb8).
   Direct reads: 0x0, 0x4, 0x8, 0x10 (twice), 0x20..0x28, 0x40, 0x44, 0x48, 0x60..0x68, 0x80, 0x84, 0x94, 0x98, 0x110,
   0x118, 0x11C. Through the stack copy (`var_1D0`): 0x8C, 0xB0, 0xC0, 0xD0, 0xD4, 0xD8, 0xDC, 0xE0, 0x120.
2. `PostFX::Tonemap` 0x82828f68: tonemapValue (0x828292ac, 0x828292bc).
3. `PostFX::ApplyUber` 0x828334d8: cheatEnable (0x828337d0). The ICE block's copy is never set, so the cheat watermark
   is never drawn during an ICE scene. `PresThreadMsgMan::ReceiveCheatStateMsg` writes only the default block
   (0x8277bd94).
4. `PostfxPresentation::Update` 0x827efe58: the bloom pulse only (ported).

No reader exists for the following fields. Their writes are dead in this build:
- greyscale (0x0C/0x4C), negative (0x30/0x70), postModulate, postBrightness, postQuantise (0x34..0x3C, 0x74..0x7C);
- **nisFar.saturateIntensity (0x50)** (see 4.3);
- motionBlurObjectApply, fxMode, fxModeValue, screenWipeValue;
- every fisheye field except WarpMultiplier (0xF0..0x10C, 0x114);
- tintEnable.

Noise and motionBlurValue reach PostFX members, but no shipped shader reads them (4.9).

### 4.2 SetParameters ICE branch (0x8284bff8..0x8284c690) and common tail

Every frame it first sets: `m_RadialBlur` = 0 (0x8284bc58), `m_stencillerpmod` = 1 (0x8284bc60), `m_ScalineOffset`
= 0 (0x8284bc68), `m_FXFinalMult` = (1,1,1,0) (0x8284bc70), `m_Fisheye_Warp` = vault (x,y,z,0) (0x8284bd24).
Then it calls VisualFX_Animation (0x8284bd28), which writes the gameplay `m_FXMatrix*_A/_B` (0x82847ac8, 0x82848338).
It also writes `m_DOFCameraInfoX` (+0x550) = splat(-(f*n)/(f-n)) and `m_DOFCameraInfoY` (+0x560) = splat(f/(f-n)),
from CameraPresentation near/far planes (0x8284bd40..0x8284bde4). If `mICEPostfxActive`:

| step | PostFX member (offset) | value | address |
|---|---|---|---|
| fisheye | `m_Fisheye_Warp` (0x4F0) | `warp*w + (0, 1-w, 0, 0)`, w = fisheyeWarpMultiplier | 0x8284bffc..0x8284c068 |
| scanline | `m_ScalineOffset` (0x5D0) | (scanlineOffset, 0, 0, 0) | 0x8284c06c..0x8284c080 |
| radial blur | `m_RadialBlur` (0x9E4) | radialBlur | 0x8284c088 |
| DOF | far start / end | `start = 0.5*GetDOFFocalDistance + GetDOFDepth`; `end = start + GetDOFBlurFalloff`; if FocalDistance < 0.01: start=100, end=200 | 0x8284c08c..0x8284c0dc |
| DOF | `m_CoC` (0x1EC) | 5 * GetDOFBlurIntensity | 0x8284c0e4..0x8284c0f4 |
| NIS lerp | `m_NISNearDist`/`m_NISFarDist` (0x440/0x444) | nisFarEnd / nisFarStart | 0x8284c12c, 0x8284c13c |
| near grade | `m_FXMatrixR/G/B_A` (0x470/0x480/0x490) | `ApplyFX_NEW(near.contrast, near.midpoint, near.saturateIntensity+1, near.brightness x3, near.saturateColour.rgb)` | 0x8284c180 |
| far grade | `m_FXMatrixR/G/B_B` (0x4B0/0x4C0/0x4D0) | `ApplyFX_NEW(far.contrast, far.midpoint, **near**.saturateIntensity+1, far.brightness x3, far.saturateColour.rgb)` | 0x8284c1c8 (`lfs f3,0x10(r29)` at 0x8284c1a0) |
| fade | `m_FXFinalMult` (0x4E0) | ported | 0x8284c1f8..0x8284c260 |
| bloom | SetBloomCoeefs / m_bloomLuminance | ported | 0x8284c264..0x8284c284 |
| bloom | `m_bloomMultiply` (0x180) | (4*bi, **4*(1-m_RadialBlur)*bi**, 0, 0) | 0x8284c324, 0x8284c32c, 0x8284c368, 0x8284c3e4 |
| noise | `m_NoiseBrightness1` 0x410 / `m_NoiseModulate1` 0x400 / `m_NoiseScale1` 0x3F0 / `m_NoiseOffset1` 0x3E0 / `m_NoiseDotP1` 0x420 | (1-ni)x3,0 / (colour*ni, 1) / (sx*20, sy*11.25) / += scroll, wrapped per lane at 1 / 4-phase weights from 4*offset.z | 0x8284c268..0x8284c664 |
| motion blur | `m_MotionBlurIntensity` (0x590) | (motionBlurValue, 0, 0, 0) | 0x8284c668..0x8284c690 |
| character mask | `m_stencillerpmod` (0x4A0) | characterEffectMaskValue | 0x8284c688 |

The common tail (0x8284c6e8..0x8284c78c) runs in both modes:
- `m_focalConstantsNear` (0x530) = (0.002, -500, 0, 0)
- `m_focalConstantsFar` (0x540) = (-1/(end-start), start, 0, 0)
- **`m_NISLerp_Mult` (0x450) = (-1/(F-N), 1/(F-N), 0, 0)**
- **`m_NISLerp_Add` (0x460) = (F/(F-N), -N/(F-N), 0, 0)**

Here N = m_NISNearDist = nisFarEnd and F = m_NISFarDist = nisFarStart. The defaults are N=40 and F=150. Finally,
0x440/0x444 are reloaded from the selected block.

The non-ICE branch (0x8284bee0..0x8284bff4) writes vault DOF/motion values instead.

### 4.3 Colour matrix (ApplyFX_NEW 0x82823f08)

The symbolic replay of the raw VMX code (stack layout 0x82823f1c..0x82824050, products 0x8282402c..0x8282415c, stores
0x82824160..0x82824168) gives, for each output row `i` (R/G/B), `row_i = mult_i * S_i * C`:
- `C = diag(c,c,c)` with offset `o_k = m - c*m + brightness_k`;
- `S` is the saturation matrix
  `[[0.7s+0.3, 0.6(1-s), 0.1(1-s)], [0.3(1-s), 0.4s+0.6, 0.1(1-s)], [0.3(1-s), 0.6(1-s), 0.9s+0.1]]`;
- `mult` = saturateColour.rgb.

So `out_i = mult_i * sum_j S_ij*(c*x_j + o_j)`. The 4th lane is `mult_i * sum_j S_ij*o_j`. This is exactly the
port's `render_postfx_math::colour_matrix(c, m, s, [b;3], colour)`. Native quirk to keep: the far matrix uses
**nisNear.saturateIntensity**, and SATURATE_FAR is ignored. `PostFX::Apply` then lerps both A and B toward identity by
clamp(m_bloomCoefficient[0], 0, 1) (0x8284c92c..0x8284cafc, the bloom-to-white path, already documented).

### 4.4 NIS near/far grade: `visualFX_PS`

`visualFX_PS` is at 0x8212e570 (0x544 bytes, CRC32 5ea4e1b5). It is drawn by `ApplyVisualFX` 0x82846728 into m_main2Rt.
- Textures: tf0 screen (m_mainRt colour, 0x828467e0), tf1 depth_texture (m_mainRt depth/stencil, 0x82846808),
  tf3 bloom (m_bloom2Rt, 0x82846830).
- Constants: c0 g_focalConstantsNear, c1 g_focalConstantsFar, c2 g_DOFCameraInfoX, c3 g_DOFCameraInfoY,
  c4 g_PsConstant_BlurMatrixZZZ, c5 g_NISLerp_Mult, c6 g_NISLerp_Add, c7/c8/c9 g_FxMatrixR/G/B_A,
  c10/c11/c12 g_FxMatrixR/G/B_B, c13 g_StencilLerpMod, c14 g_maxCoC.
- `m_VecCoC` = (m_CoC,0,0,0) (0x82846770).
- Literals: c253 = (2, 1/255, 1.41412354, 0), c254 = (-0.2, 1, 0.5, 1e-4), c255 = (1/65793, 255/256, 1/257, 1).

Reconstructed from the microcode (ALU slot numbers from the disassembly):

```
float4 d  = tex2D(depth_texture, uv);                               // 4
float  dz = dot(d.<3 depth bytes>, float3(1/65793, 255/256, 1/257)); // 9: 24-bit depth
float  s  = saturate(d.<stencil byte>);                              // 14
float3 c  = tex2D(screen, uv).rgb * 1.41412354 + tex2D(bloom, uv).rgb; // 8
float  z  = c2.x / (dz - c3.x);                                      // 15-19 linear view depth (sign rule UNVERIFIED, sec. 8)
float2 w  = saturate(z * c5.xy + c6.xy);    // 20: w.x = (F-z)/(F-N) near, w.y = (z-N)/(F-N) far
float3 g  = w.x * (M_A * float4(c,1)) + w.y * (M_B * float4(c,1));  // 12-18, 23-26
float  k  = c13.x * saturate(2*s);                                   // 19, 27
oC0.rgb   = g + (c - g) * k;                                         // 27-28
float  dof = saturate((z - c1.y) * -c1.x);  // 21-26: far-only (z >= c0.y=-500 always selects c1)
float2 mv  = camera-motion reprojection (c4, interpolants) * 1e-4;   // 10-11, 29
float  mb  = saturate(dot(mv,mv) * (1 - saturate((2 - 0.2*z) * saturate(2*s - 1))));  // 25-31
oC0.a     = saturate(max(max(mb, 1/255), c14.x * 0.5 * dof));       // 31-33: blur amount for ApplyUber
```

So the grade is per pixel, from linear depth. It is fully near below N (40) and fully far beyond F (150), with a
linear blend between. In ICE mode `g_StencilLerpMod` = characterEffectMaskValue (0 in all shipped takes, so the
grade also covers characters). Outside ICE it is 1.

### 4.5 DOF from RAWFOCUS (30 Hz path only)

1. `visualFX` alpha (4.4) = blur amount, with `c14 = m_CoC = 5*mICEDOFBlurIntensity` and the far factor
   `saturate((z-start)/(end-start))`.
2. `GenerateDOFMotionBlurBuffers` 0x82831fd0 runs when `activeEffects & 1`, which is `kbEnableDof`, default true
   (initializer 0x82f70578, `li r7,1`). It has three passes:
   - `motionblurdofdownsample_PS` 0x8212f0e0 (0x200 bytes, CRC32 98d5a2cc, CTAB: tf0 only) draws into
     m_newDepthOfFieldRt (0x8283201c..0x828321a8). It is a 4-tap alpha-weighted average: taps at uv,
     uv+(1/1280,0), uv+(0,1/720) and uv+(1/1280,1/720); weights are the tap alphas (+0.001 on the centre);
     out.a = sum(a)/4.
   - `Blur9DOFMotionBlur` 0x82831018 with angle 0 (call 0x82832210) uses `tap9dofmotionblur_PS` 0x8212c3c8
     (0x51C bytes, CRC32 c49d1ced; c0 BlurMatrixZZZ, c10 maxCoC, tf0 scene, tf1 depth).
   - `Blur9DOF` 0x82830d68 with angle pi/2 (call 0x8283226c) uses `tap9dof_PS` 0x8212bb28 (0x424 bytes,
     CRC32 0a8842d3).
   - Both blur passes clamp the radius to at least 1.5 (0x82830db0, 0x8283106c). They pass it as `maxCoC`
     (g_COC.x, 0x82830f80/0x82831278), with unit-radius quadratic 9-tap weights from
     `InitWeights_DirBlur9Quadratic` 0x82822998.
   - `tap9dof_PS`: per pixel `r = max(maxCoC * centre.a, 0.1)` (ALU 6-7). Taps at `uv + r*offset_i`, tap weights
     `w_i * tap.a`; `out.rgb = sum(w_i a_i c_i) / sum(w_i a_i)`, `out.a = centre.a`.
   - `tap9dofmotionblur_PS` adds a per-pixel direction from depth reprojection. Its details are UNVERIFIED (sec. 8).
3. `uberPostFX_PS` lerps `screen -> dof_texture` by screen alpha (4.6).

With RAWFOCUS off, the Focus channel zeroes the DOF fields: `m_CoC` = 0 and start/end = 100/200, so only the camera
motion term remains.

### 4.6 `uberPostFX_PS`: scanline roll and the final ICE grade stage

`uberPostFX_PS` is at 0x8212dc28 (0x310 bytes, CRC32 0303bb42). It is drawn by `ApplyUber` into m_mainRt
(quad 0x82833770).
- Constants: **c0 g_ScanlineOffset**, c1 g_OverlayAdd, c2 g_FinalMult.
- Textures: tf0 screen (m_main2Rt), tf1 dof (m_newDepthOfFieldRt), tf4 colour cube (3D), tf6 overlay (x-ray or
  black), tf7 depth.
- The other uber params registered in `PostFX::Init` (g_Noise*, g_Time, g_bTint, g_VignetteUVOffset, ...) are not in
  the CTAB and are not read.

```
float2 uv2 = float2(uv.x, frac(uv.y + c0.x));        // 3-4: vertical roll with wrap
float4 sc  = tex2D(screen, uv2), df = tex2D(dof, uv2);
float4 col = lerp(sc, df, sc.a);                      // 7-8
float4 g   = tex3D(cube, float3(col.g*0.96875+0.015625, -col.r*0.96875+0.984375, col.b*0.96875+0.015625)); // 9-10
float4 ov  = tex2D(overlay, uv2); float dz = tex2D(depth, uv2).<one channel>;
g = g * (1 + c1 * (2*ov - 1));  g = lerp(g, col, dz * c1);   // 11-18 (OverlayAdd is 0 in NIS: reset at 0x8284768c)
oC0 = g * c2;                                         // 19
```

ICE `scanlineOffset` is therefore a **vertical scroll of the whole graded image with wrap-around**, not drawn
scanlines.

### 4.7 Fisheye

`m_Fisheye_Warp` = `vault*w + (0, 1-w, 0, 0)` with w = fisheyeWarpMultiplier (4.2). The vault value is
(0.72, 0.9, 1, 0) per fisheye-final-pass-2026-09-30.md. `SetUberParams` 0x82824178 replaces it with (0,1,0,0) only
when `activeEffects` lacks eFishEye (0x8282422c). `Blit_ToDeviceRT` 0x82831c18 then feeds it to
`basictex_fisheye_PS` c0 (g_Fisheye_Warp; the shader is decoded in that doc). Effects of w:
- warp.x = 0.72w
- warp.y = 0.9w + 1 - w
- warp.z (vignette radius scale) = w

The ICE vignette/upscaler/power fields do not reach the GPU. The vignette offset stays (0, 0.25, 0, bloom c[0]).

### 4.8 Radial blur (no separate pass; it reshapes the bloom)

- `SetBloomCoeefs` 0x828233b0 multiplies each lens-ghost colour `m_Bloom_Ghost1..6_ARGB` by (1 - m_RadialBlur).
  The reads are at 0x82823bc8, 0x82823c80, 0x82823cb0, 0x82823cd8, 0x82823d6c and 0x82823da4 (f31 = 1).
- `m_bloomMultiply.y` = 4*(1-rb)*bloomIntensity (4.2). `GenerateBloomBuffers` 0x82832c68 passes `m_RadialBlur` to
  all five `PfxHelper::Blur9` calls (0x8283310c, 0x82833130, 0x82833160, 0x82833188, 0x828331b8). The y lane is
  only the multiply of Blur9 #2 (0x82833130). Before the combine, .y is forced to 1 and .z to (z>1 ? 1/z : 1)
  (0x828331bc..0x8283325c).
- `Blur9` 0x82830a58 writes `sampleDirection` = (dir.x, dir.y, 0, **rb**) (0x82830c60..0x82830c78) for `tap9_PS`
  0x8212b748 (0x3E0 bytes, CRC32 29f77907). The shader computes
  `dir' = lerp(sampleDirection.xy, 5*(uv-0.5), rb)` (ALU 4-6); the centre tap shifts by `-0.75*c4.x*dir'*rb`
  (12-14); `out.rgb = blur * bloomMultiply.x * (1+rb)` (28-29, 39); out.a is unscaled. With rb = 1, every bloom blur
  becomes a zoom blur centred on the screen, at double strength.
- `Blur9_combined3` 0x82829520 sets `m_SampleDirection` = (0,0,0,rb) (0x8282955c). In `tap9_combined3_PS`
  0x82130478 (0x418 bytes, CRC32 04afc622) the 9-tap on tf0 (m_bloom2Rt) also gets an rb-dependent, position-based
  direction (ALU 5-17). Its exact form is UNVERIFIED; it is not identical to tap9_PS.
  `oC0.rgb = saturate((1+rb)*g_BloomMultiply.x*blur9(tf0) + g_BloomMultiply.y*tf1(m_bloomRtLow))` (ALU 33-47) and
  `oC0.a = 1 - saturate(5*length(max(|2uv-1| - 0.6, 0)) - 1)` (34-42).

### 4.9 Tonemap, motion blur, noise

- `Tonemap` 0x82828f68 is called from GenerateBloomBuffers at 0x828332e0, after the scene is drawn. If
  `mICEPostfxActive == 1` and `tonemapValue > 0` (0x828292a0..0x828292c0), it overwrites the published environment
  exposure (`*r28`, m_MaterialEnvAttributes data+0x30, 0x82828fe0) and the character exposure (`r30+8`,
  m_MaterialCharAttributes data+0x30, 0x82829008). This is the same slot the auto-exposure writes (shared-postfx doc).
  The material shaders use it on the next frame. `m_tonemap_CurrExposure` (0xA50) keeps the auto value, but the next
  adaptation starts from the published (ICE) value (`lfs f0,0(r28)` 0x828291cc).
- motionBlurValue reaches `m_MotionBlurIntensity`, which `Apply` zeroes when `!(effects & 1)` (0x8284cb40). It is
  registered as `g_MotionBlur_Intensity` for visualFX, but neither visualFX_PS nor tap9dof*_PS has it in its CTAB.
  `Blur9DOFMotionBlur` gets the pointer in r10 and never reads it (r10 is overwritten at 0x8283107c). There is **no
  consumer**.
- The noise constants are not in uberPostFX_PS's CTAB. `m_NoiseDotP1` is shared with `basictex_fisheye_noise_PS`, but
  `SetNoiseParameters` 0x82828ac0 rewrites 0x420/0x430 before every use of that program (Blit_ToDeviceRT 0x82831f2c).
  There is **no consumer**, and the channel is cvar-off anyway.

## 5. Pass order of an ICE (30 Hz) frame (question 3, order)

1. Channels write mICEPostfxData and the CameraPresentation DOF fields (section 2), before RenderMan::Render.
2. `RenderMan::RenderModeScene` 0x82857a78 draws the scene (shadows 0x82857b40, world, FE Render3D 0x82857cf8,
   particles), then `PostFX::End` 0x82857d60. activeEffects = eFishEye | DOF(1) | Tint(4), plus 0x10 in the FE
   (0x82857d74..0x82857ddc).
3. `PostFX::Apply` 0x82857dec -> 0x8284c7a0. 30 Hz mode = mICEPostfxActive (or FE, replay, ...) with CAS state 0
   (0x8284c848..0x8284c860). It runs:
   1. SetParameters 0x8284c920 (section 4.2).
   2. The matrix lerp by bloom c[0] (0x8284c92c).
   3. GenerateBloomBuffers 0x8284cb7c: bright pass, Blur9 x5 (radial when rb), combine, ghosts x(1-rb), Tonemap
      (ICE exposure).
   4. **ApplyVisualFX** 0x8284cbfc: bloom add plus the near/far grade by depth, then the blur alpha.
   5. **GenerateDOFMotionBlurBuffers** 0x8284cc0c.
   6. **ApplyUber** 0x8284cc18: scanline roll, DOF lerp, colour cube, FinalMult fade.
   7. **Blit_ToDeviceRT** 0x8284cc54: ICE-scaled fisheye and vignette.
4. FE Render2D 0x82857ea4 (HUD, subtitles, NIS overlays), then FinalQuadFade 0x82857fc4.

Gameplay uses ApplyFastUber 0x8284cbbc instead of steps 4-6 and never reads the ICE block. In the stereo path
(`Stereoscopy::IsActive` 0x82827988 needs it enabled and not FE) ApplyVisualFX runs without uber or blit.

## 6. Census of shipped takes (question 3, usage)

`ice_000.bin` has 598 takes: 584 NIS takes and 14 recorded shakes. Counts are takes with the override on in at least
one interval.

| override | takes | contexts | shipped values (keys of the on-intervals) |
|---|---|---|---|
| RAWFOCUS (104) | 445 | Photo 93, BestTrick 88, OwnTheSpot 87, Film 65, RaceStart 28, Flythrough 21, Story 15, GoTo 12, 1Up 10, Skate 9, ProChallenge 8, FrontEnd 7, Tutorial 1, Extras 1 | NEAR_FOCUS mostly 0 (2310/2428), max 85.8; FAR_FOCUS 0..471; FALLOFF 1.0 (2081) or 0; INTENSITY 0..0.386 (typ. 0.047/0.039/0.031, so m_CoC ~0.5..5.8) |
| FISHEYE (115) | 422 | Photo 94, OwnTheSpot 92, BestTrick 86, Film 65, RaceStart 28, Flythrough 20, GoTo 12, 1Up 9, Skate 8, Tutorial 4, ProChallenge 2, Story 1, Extras 1 | HORIZONTAL 1.0 always, VERTICAL ~1.0 (both dead); MULTIPLIER 0..1 (w = 4.7619*m: 0.2118 -> w 1.01, 0.1882 -> 0.90, 0.1373 -> 0.65, 0 -> no warp); w > 1.05 (MULTIPLIER 1.0 -> w 4.76, warp (3.43, 0.52, 4.76)) in 79 takes: Photo 33, OwnTheSpot 27, Film 19 |
| BLOOM (112) | 192 | (ported) | |
| TINT (122) | 116 | OwnTheSpot 63, Photo 32, Film 19, RaceStart 1, Story 1 | Z = 1 always; near (X,Y) e.g. (0.078,0.5) -> rgb ~(1,0.57,0.49); far (0.43,0.68) -> ~(1,0.63,0.20) (warm/sepia); NEGATIVE = 1 in 161/272 intervals (dead) |
| CONTRAST (118) | 115 | OwnTheSpot 61, Photo 32, Film 19, ProIntro 2, Story 1 | FAR 2.5679 at midpoint 0.6602 (228 keys), also 1.0/0.8/0.6; NEAR mostly 1.0 at midpoint 0.4843 |
| SATURATE (119) | 113 | OwnTheSpot 61, Photo 32, Film 19, RaceStart 1 | NEAR 0, -0.6957, -1.0, +0.129; FAR 0, -0.652, -1, +0.172 (FAR ignored by native) |
| SCANLINE (127) | 77 | Photo 32, OwnTheSpot 26, Film 19 | 0.9964 (76 keys), 0.9995 (1), else 0. In the sampled Photo takes the keys go 0.9964 -> 0 (cubic) over the first ~0.26 s of a 5 s take: one full-screen vertical roll-in |
| RADIALBLUR (129) | 77 | same 77 takes | 1.0 (231 keys) or 0 |
| SHADOW (125) | 72 | RaceStart 29, Story 28, ProChallenge 15 | not post-fx (section 3) |
| TONEMAP (109) | 44 | Story 21, ProChallenge 15, RaceStart 3, OwnTheSpot 2, Tutorial 2, GoTo 1 | 0..2.61 (RaceStart ~1.17, Story/ProChallenge mean ~1.55); a value <= 0 keeps auto-exposure |
| SUN_DIRECTION (131) | 35 | Story 21, ProChallenge 14 | not post-fx |
| MOTION_BLUR (110) | 6 | OwnTheSpot 2, ProIntro 2, Photo 1, Tutorial 1 | 0.28..3.15 (no consumer, so no visible effect) |
| BRIGHTNESS (117) | 4 | Skate 3, OwnTheSpot 1 | 0 or 1.0 (a +1 offset is a white flash) |
| NEAR_FAR (116) | 0 | | nisFarStart/End are always 150/40 in shipped data |
| CHARACTER_EFFECT_MASK (123) | 0 | | StencilLerpMod is always 0 in NIS |

FXMODE, SCREEN_WIPE and NOISE are never non-default. Channel 2 (aperture focus) has no data in any take. Channel 10
(light rig) has data in 3 takes; one non-default KEY value (0.937) appears in one ProChallenge take.

What is actually visible:
- RAWFOCUS (far-only DOF) and FISHEYE (warp strength) are almost everywhere.
- The Photo, OwnTheSpot and Film treatment is: contrast, desaturation via the near value, a warm tint, the vertical
  scanline roll-in, radial (zoom) bloom without lens ghosts, and sometimes a 4.76x fisheye.
- TONEMAP forces exposure in Story and ProChallenge.
- BRIGHTNESS flashes appear in 4 takes.
- MOTION_BLUR, negative, greyscale, quantise and FX mode do nothing in this build.

## 7. Notes for the port (observations from reading the repo, not IDA facts)

- The cinematic `visual` stage (gpu_renderer/postfx.wgsl) uses `assets.matrix`, the vault `m_NEWcolour_matrix`. Natively,
  in ICE mode, both matrices are replaced every frame by ApplyFX_NEW(near) and ApplyFX_NEW(far) (identity with default
  values). The far one is blended by linear depth with N=40 and F=150. Today the port applies the gameplay matrix in
  NIS.
- `IcePostfx` (core nis/manager.rs) carries only bloom, fade and bloom_anim. 1:1 also needs:
  - near/far brightness, contrast, midpoint, saturate (near only) and tint rgb;
  - nisFarStart/End;
  - fisheyeWarpMultiplier, scanlineOffset, radialBlur, tonemapValue, characterEffectMaskValue;
  - the four CameraPresentation DOF values.
  The tint hue needs `ICE::ATan`'s FastATanTable (0x830022f8), not atan2.
- `nis_postfx` sets both bloom multiplies to 4*bi. Natively the y lane is 4*(1-rb)*bi, the ghosts are x(1-rb), and rb
  must also reach every Blur9 pass and the combine.
- The final pass already has the vault warp. ICE needs `warp*w + (0,1-w,0,0)`.
- The cinematic grade needs the scanline `frac(v + offset)` resample before the cube. DOF (alpha plus a downsample
  and two 9-tap passes) is not ported.

## 8. Unverified / open

1. Sign rule of `addsc`/`subsc` with a negated constant. visualFX ALU 15 prints `subsc0 r0.x, -c3.x, r0.y`. Only the
   "negate applies to both operands" reading gives the standard `z = X/(d - Y)`. The literal "const - temp" reading
   gives `-(Y+d)` and nonsense depth. The same rule decides the stencil split `saturate(2s-1)` (ALU 17/23) versus 0.
   Treat both formulas as UNVERIFIED until checked against Xenia's ucode handling or a capture.
2. `tap9dofmotionblur_PS` and `quad_motionblur_VS` (0x8212af10) were not fully decoded. The motion vector, its
   1e-4 scale and the exact DOF/motion mix in the first DOF pass are partial.
3. The meaning of the stencil values tested in visualFX (which objects write them) was not traced.
4. Render-target formats and precision of m_main2Rt, the DOF targets and the bloom targets were not re-checked.
5. The readers of the light rig (RenderMan::UpdateLighting 0x82833d8c), shadows (m_NISoo*) and the sun
   (m_SunLightMan) were located but not decoded.
6. Reader searches cover direct displacements, selectors and the one struct copy. An access through a computed pointer
   or an inlined block copy could be missed. The reset-on-scene-start question (section 1) is open.
7. The census uses a Python port of the repo parser, not the native loader. Counts match the caller's numbers, and
   values are quantised keys, not per-frame samples.
8. The ICELITE path (SceneLite) was not traced for shipped use.

Scratch artefacts (not in the repo): shader blobs (*.bin) and their
disassembly (*.disasm.txt), `applyfx_sim.py` (ApplyFX_NEW replay), `vmx128.py` (VX128 field decoder), and
`ice_census.py`/`census2.py` with `census.txt`.

## Port (2026-10-05)

Re-verified in IDA before porting: the Tonemap, FishEye, NearFar, Brightness, Contrast, Saturate, Tint,
Scanline and RadialBlur handlers (section 3, every store); `ICE::XYZ2HSV` (ATan(Y, X) / 65536,
`fmadds` X*X + Y*Y then `fsqrts`, held to 1 with NaN taking 1; `ICE::ATan` never touches f3, so v is Z)
and `ICE::HSV2RGB` (`fnmsubs` for q and t; sextants 0 and 6 alike, an unsigned compare against 6
skips the stores otherwise); `SetDefaults`; `SetParameters`' ICE branch (fisheye `vmaddfp`, scanline,
radial blur, near/far distances, both `ApplyFX_NEW` calls with the near saturation), its tail
(f26 = -1, f30 = 1: g_NISLerp_Mult (-1/(F-N), 1/(F-N)), g_NISLerp_Add (F/(F-N), (-1/(F-N)) * N)) and
`m_DOFCameraInfoX/Y` (-(f*n/(f-n)), f/(f-n)); `GetNearPlane` (the ICE near plane) and `GetFarPlane`
(rendering layout +0x64); `SetBloomCoeefs`' six ghost colours times splat(1 - m_RadialBlur);
`PfxHelper::Blur9`'s (direction, 0, rb); `Tonemap`'s tail (f26 = 0: a positive ICE value replaces both
exposures). `GetValueFloat` returns the stored value unclamped, so cubic overshoot reaches the
shaders (photo_killed_m_1a_05_photo_rail: scanline -0.0141 and radial blur 1.01 at 0.27 s).
The `visualFX_PS` depth formula takes the only reading of `subsc0` that gives a view depth.

Ported: formats `Playback::{tonemap, fisheye, near_far, brightness, contrast, saturate, tint, scanline,
radial_blur}` (the census over every take's own channels matches section 6: 44, 422, 0, 4, 115, 113,
116, 77, 77); core `IceFrame::postfx` into `IcePostfx` (grade, far, tonemap, fisheye, scanline,
radial_blur; defaults when an override is off); PC `render_postfx::IceInputs`: the bloom's y lane and
ghosts by (1 - rb), rb into every Blur9 and the combine, the near and far matrices (with the bloom
to white lerp) blended per pixel by the scene depth (sample 0 of the depth aspect) in the `visual`
stage, the scanline roll in `cinematic_grade`, the warp scale in the final blit, the tonemap
exposure. Not ported: the Focus/RawFocus DOF (fields, `GenerateDOFMotionBlurBuffers` and the uber
lerp; visual's alpha stays 0), the stencil character mask (no shipped take turns it on), the light
rig, shadow and sun readers; motion blur and noise have no consumer.
