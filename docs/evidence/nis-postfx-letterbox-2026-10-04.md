> Agent research (2026-10-04), read-only IDA. Bloom-to-white (1.2-1.6) and the letterbox
> conclusion (2.2) were re-verified and ported; re-verify the rest before porting it.

# NIS presentation -> screen: bloomToWhiteValue (general post-fx) and ICE letterbox

Source: sk82_na_m.xex IDB (session 8ee0dbb0), read-only. All addresses are guest (PPC) addresses.
Status: written incrementally; see "Unverified / open" at the end.

## 0. Layout facts used below (IDA types)

- `Sk8::Presentation::msInstance` = global 0x83054640. `Presentation.mPostfxPresentation` at +0x7940 (type_inspect
  Sk8::Presentation). So, absolute offsets from the Presentation pointer:
  - mDefaultICEPostfxData +0x7940, mICEPostfxData +0x7A70
  - mDefaultGeneralPostfxData +0x7BA0, **mICEGeneralPostfxData +0x7BC0**, **mGameGeneralPostfxData +0x7BE0**
    (bloomToWhiteValue at +0x7BD8 / +0x7BF8 -- exactly the two `stfs f31, 0x7BF8(r10)` / `0x7BD8(r10)` at
    0x827d5638 / 0x827d563c in BloomToWhiteTransitionEffect::Update)
  - mDefaultPostFEPostfxData +0x7C00, mICEPostFEPostfxData +0x7C10, mGamePostFEPostfxData +0x7C20
  - **mICEPostfxActive +0x7C30** (bool)
  - mCameraPresentation (PresentationObj) at +0x11F30; letterbox height at +0x13028.
- `Sk8::Render::PostFX` (render side, size 0xBA0) members used here: m_OverlayAdd +0x3D0, m_VignetteUVOffset +0x3C0,
  m_NISLerp_Mult +0x450, m_NISLerp_Add +0x460, m_FXMatrixR_A..B_B +0x470..0x4D0, **m_FXFinalMult +0x4E0**,
  m_NoiseFade +0x510, **m_bloomCoefficient float[4] +0x660**, m_bloomLuminance +0x670, m_bloomThreshold +0x680,
  m_bloomSaturation +0x684, m_RadialBlur +0x9E4.
- `PostfxPresentation::SetDefaults` 0x827eff00: mDefaultGeneralPostfxData = { screenFadeColour (1,1,1,1)
  (stvx128 v60 [=vmx 1,1,1,1] at this+0x260, 0x827f00f0..), wipeEffectType NONE (0x827f00ac), wipeEffectValue 0
  (0x827f00e8), bloomToWhiteValue 0 (0x827f00ec) }; mDefaultPostFEPostfxData = 0 (v59 zero at +0x2C0).
  Default ICE: bloomThreshold 1.4 (0x827f0018), bloomIntensity 0.5 (0x827f001c).

## 1. bloomToWhiteValue -> screen

### 1.1 Writers (sim/presentation thread side)

- `BloomToWhiteTransitionEffect::Update` 0x827d55c0: v = t>0 ? (fadeUp ? (dur-t)/dur : t/dur) : (fadeUp?1:0);
  writes BOTH mGameGeneralPostfxData.bloomToWhiteValue (0x827d5638, +0x7BF8) and mICEGeneral...(0x827d563c, +0x7BD8),
  plus AudioSystem::mNISFadeAmount (+0xDF8, 0x827d5660).
- (For comparison) `ColorFadeTransitionEffect::Update` 0x827d5500: same t/dur fraction f, writes vec4(colour.r,
  colour.g, colour.b, f) (colour = effect+0x10/0x14/0x18) to **mGamePostFEPostfxData (+0x7C20, 0x827d5590)** and
  **mICEPostFEPostfxData (+0x7C10, 0x827d55b8)** -- i.e. BLACK_FADE/WHITE_FADE transitions use the PostFE blocks,
  NOT the general blocks' screenFadeColour.

### 1.2 Readers: two, both in the render-side PostFX, both pick ICE-vs-Game by mICEPostfxActive

There are no IDA field xrefs (xrefs_to_field returns none); readers were found from the PostFX code itself.

(a) `PostFX::VisualFX_Animation` 0x82847610 (gameplay path; always runs, called from SetParameters at 0x8284bd28):
```
v10 = m_NEWbloom.layout[0]   (bloom threshold attrib, 0x82847698; blended toward HoM/success/failure attribs, or
                              minus VD sFadeInfo.mNormalizedFadeValue in the bFadeInfo branch 0x828485f0)
v11 = m_NEWbloom.layout[1]   (0x828476a0; "saturation" argument)
p = Presentation->mPostfxPresentation.mICEPostfxActive ? &mICEGeneralPostfxData   // 0x828486e8 / 0x828486f4
                                                       : &mGameGeneralPostfxData; // 0x828486fc
threshold = v10 - 2.0 * p->bloomToWhiteValue;          // 0x82848718 (fnmsubs form: -(b2w*2 - v10))
SetParameters_MotionBlur(this);                         // 0x8284879c
SetBloomCoeefs(this, threshold, v11);                   // 0x828487ac
```
(b) `PostFX::SetParameters` 0x8284bbf0, ICE path only:
```
p = &P->mPostfxPresentation; if (P->mPostfxPresentation.mICEPostfxActive /*lbz 0x7C30, 0x8284beac*/) p += 0x130;
ice = *p   (ICEPostFX_t copy, operator= 0x8284bec4)
if (!mICEPostfxActive) { ...attrib-driven motion blur/DOF only; general blocks NOT read... }   // 0x8284bedc -> bee0..bff4
else {                                                                                      // 0x8284bff8..
  base = ice.bloomThreshold(+0x94) * 0.5;                          // lfs f13,0x94(r29); fmuls f0,f13,f28(0.5) 0x8284c104/8
  g = mICEPostfxActive ? P+0x7940+0x280 (ICE general) : +0x2A0 (Game general);   // 0x8284c0fc..0x8284c118
  threshold = base - 2.0 * g->bloomToWhiteValue;                   // lfs f13,0x18(r28) 0x8284c11c;
                                                                   // fnmsubs f24,f13,f29(2.0),f0  0x8284c170
  ApplyFX_NEW(near...), ApplyFX_NEW(far...)                       // 0x8284c180, 0x8284c1c8
  m_FXFinalMult = from g->screenFadeColour (see 1.8)               // 0x8284c1f8..0x8284c260
  SetBloomCoeefs(this, threshold, /*saturation*/ 1.0);             // f1=f24, f2=f30(1.0) 0x8284c264 (overrides (a))
  m_bloomLuminance = {1.0, 0.0, 0.5, ?}                            // 0x8284c274..c284 (+0x670..0x678)
  bloomIntensity: v105 = (1 - m_RadialBlur) * ice.bloomIntensity(+0x98); v106 = ice.bloomIntensity*4  // 0x8284c324/c32c
  ...
}
```
f30 = 1.0 (lfs 0x8284bc48), f31 = 0.0 (0x8284bc40), f28 = 0.5 (0x8284c0a8), f29 = 2.0 (0x8284c15c).
So in BOTH paths: **effective bloom threshold = base - 2*bloomToWhiteValue**. The block chosen is ICE when
`mICEPostfxActive` else Game. Since BloomToWhiteTransitionEffect writes both blocks the same value, the chosen block
does not matter for that effect. In path (b) the re-check at 0x8284c100 is redundant (already inside the ICE branch).

### 1.3 SetBloomCoeefs 0x828233b0 (CPU math -> shader constant `coefficients`)
```
m_bloomThreshold  = t;          // 0x828233e4
m_bloomSaturation = s;          // 0x828233ec
c[3] = t; c[1] = 0; c[0] = 0;   // 0x828233f0..f8
r = 1 - t;  if (r > 1) r = 1; else if (r < 0.1) r = 10.0;   // 0x828233dc..0x82823418 (10.0, not 0.1, sic)
c[2] = 1 / r;                   // 0x82823420
if (t < 0) c[0] = -t;           // 0x82823428/30
(rest: lens-flare ghost constants from m_NEWflare attribs, unrelated)
```
`m_bloomCoefficient` (+0x660) is registered as shader constant **"coefficients"** of `tap4_minusthreshold_PS`
(blob 0x8212b250, 0x2F8 bytes; PostFX::Init AddParam 0x8284aa30) and of `tap4_minusthreshold_alphalum_PS`
(0x8212f2e0, 0x200 bytes; AddParam 0x8284aae8 -- registered there but NOT bound by that shader's CTAB, see 1.5). Other constants registered for the bright pass: "luminance" (m_bloomLuminance; not present in its CTAB, so unused),
"sampleOffsets" x4, g_MatrixR/G/B (= m_FXMatrix*_Bloom +0x5A0..0x5C0).

Second consumer of c[0]: `PostFX::Blit_ToDeviceRT` 0x82831c18: m_VignetteUVOffset.w = (c[0] > 0) ? c[0] : 0
(0x82831cb8 lfs; 0x82831d5c test; 0x82831d68 store) -> constant "g_VignetteUVOffset" of the final blit programs
basictex_fisheye_PS (0x8212cda8) / basictex_fisheye_noise_PS (0x8212cff0) (AddParam 0x8284b768 / 0x8284b818).

### 1.4 Apply-time CPU step: colour grading neutralised while c[0] > 0

`PostFX::Apply` 0x8284c7a0, right after `SetParameters` (call 0x8284c920):
```
c = m_bloomCoefficient[0];                 // lfs f0,0x660(r30) 0x8284c92c
if (c > 0) {                               // 0x8284c93c
  if (c > 1) c = 1;                        // 0x8284c944/48
  k = 1 - c;                               // 0x8284c950
  FXMatrixR_A = FXMatrixR_A*k + (c,0,0,0); // vmaddfp 0x8284ca30 (this+0x470)
  FXMatrixG_A = FXMatrixG_A*k + (0,c,0,0); // 0x8284ca7c (+0x480)
  FXMatrixB_A = FXMatrixB_A*k + (0,0,c,0); // 0x8284cad4 (+0x490)
  FXMatrixR_B/G_B/B_B likewise              // 0x8284cae0/caec/caf8 (+0x4B0/+0x4C0/+0x4D0)
}
```
i.e. the near (A) and far (B) colour matrices are lerped toward identity by clamp(-threshold, 0, 1).

Operand-order note (verified on raw bytes, e.g. 0x8284ca30 = 0x11606B2E: vD=11 vA=0 vB=13 vC=12, xo=46): IDA prints
`vmaddfp vD,vA,vB,vC` in ENCODING order; the operation is vD = vA*vC + vB. IDA also mis-prints the VA operand of
VX128 ops (vmulfp128/vaddfp128) 32 too high (0x8284c254 = 0x15B278B1 decodes VA=v50, IDA shows v82). Both facts
matter when reading the VMX code quoted below.

### 1.5 Shader side

Shaders are Xenos microcode blobs embedded in the xex. I dumped the bytes from IDA and disassembled them with a
scratch copy of the repo tool tools/xenos_disasm.py (no repo files touched).

Bright pass `tap4_minusthreshold_PS` (blob 0x8212b250). CTAB: c0..c3 = sampleOffsets[4], **c4 = coefficients**,
c5/c6/c7 = g_MatrixR/G/B, tf0 = scene_texture (m_mainRt, GenerateBloomBuffers 0x82832d2c), render target m_bloomRt.
Literal c255 = (0.3, 0.4, 0.25, 1.0):
```
s_i  = tex(tf0, uv + sampleOffsets[i]).rgb, i=0..3
avg  = 0.25 * (s0^2 + s1^2 + s2^2 + s3^2)                         // ALU 11..15
col  = sat((avg - coefficients.w) * coefficients.z) + coefficients.x   // ALU 15, 18, 19
oC0.rgb = sat( dot4(g_MatrixR/G/B, (col, 1)) )                       // ALU 20..22
oC0.a   = sat( dot(sat(avg), (0.3, 0.4, 0.3)) )                      // ALU 16, 17
```
`tap4_minusthreshold_alphalum_PS` (0x8212f2e0) does NOT bind "coefficients". It is a plain sat(0.25 * sum of 4 taps)
downsample (bloomRt -> bloomRtLow -> bloomRtUberLow; GenerateBloomBuffers 0x82832ed8..0x828330e0).

Bloom composite into the scene, NIS / 30 Hz path: `visualFX_PS` (0x8212e570). ApplyVisualFX 0x82846728 binds
tf0 = m_mainRt colour, tf1 = mainRt depth/stencil, **tf3 = m_bloom2Rt** (0x82846830), output m_main2Rt:
```
c    = scene.rgb * 1.41412354 + bloom.rgb                                 // ALU 8 (c253.z)
near = (g_FxMatrixR_A, G_A, B_A) . (c,1);  far = (R_B, G_B, B_B) . (c,1)  // ALU 12..18
out  = depth lerp(near, far) via g_NISLerp_Mult/Add, then g_StencilLerpMod lerp   // ALU 19..28
```
Gameplay path `fast_uberPostFX_PS` (0x8212df38): c = scene*1.41 + bloom; M_A . (c,1); 3D colour cube (tf4).
No g_FinalMult in that shader.

Final blit `basictex_fisheye_PS` (0x8212cda8): out = warped_scene * sat(tex(Vignette_texture) + g_VignetteUVOffset.w)
(ALU 12/13). With w = c[0] (Blit_ToDeviceRT) the vignette darkening is lifted toward 1 as the white-out completes.

### 1.6 Net effect of bloomToWhiteValue (b): what to reproduce
- Threshold t = base - 2b. ICE/NIS path: base = 0.5 * ICE.bloomThreshold (default 1.4 -> 0.7). Gameplay path:
  base = m_NEWbloom attrib [0] (possibly blended, see 1.2a).
- Bright pass: (avg - t) * c.z, where c.z = 1/clamp'(1-t) (= 1 once t <= 0). Once t < 0, a constant -t is ADDED to
  every bloom texel after the saturate, then the matrix and a final saturate. So the bloom buffer goes uniformly to 1
  (white) as b -> 1. With base 0.7, t < 0 for b > 0.35; at b = 1, t = -1.3.
- Composite: scene*1.414 + bloom (scaled by the m_bloomMultiply factors in the blur/combine chain), then the colour
  matrices, which are lerped to identity by clamp(-t,0,1) at the same time (1.4), then the colour cube, then the final
  blit with the vignette lifted by c[0]. Result: a bloom "flash" to white. It is NOT a plain lerp of the final colour
  to white, and there is no dedicated shader constant for bloomToWhiteValue.
- It is drawn before the FE/HUD: FrontEndManager::Render2D (0x82857ea4) comes after PostFX::Apply (0x82857dec) in
  RenderModeScene.

### 1.7 Hand-off / threading: no separate render-thread copy
`GameModule::Render` 0x827320f0 runs, in order: SceneManager::UpdateFrameBegin (0x827322e8), DataSys restore,
**Presentation::Update** (0x82732434: TransitionPresentation::Update 0x827f75dc -> BloomToWhite/ColorFade effects;
PostfxPresentation::Update 0x827f7608), **SceneManager::UpdateFrameEnd** (0x82732490: ICE user channels'
UpdateFrameEnd -> SetFadeColor/SetLetterBox etc.), AudioSystem update, then **RenderMan::Render** (0x827324ec) ->
RenderModeScene 0x82857a78 -> PostFX::Apply 0x82857dec. Apply reads `Presentation::msInstance->mPostfxPresentation`
directly (SetParameters / VisualFX_Animation). No VisualDirector/VDWriter copy of these fields was found. The VD state
(mpVDPresentationState->mOutData) only feeds the gameplay effects (fade info, HoM, challenge) in VisualFX_Animation.
The values are consumed in the same frame they are written.

### 1.8 screenFadeColour (general blocks) -> shader `g_FinalMult`
- Writer: `User_ICECamera::SetFadeColor(r,g,b,a)` 0x826d34f8 stores vec4 at Presentation+0x7BC0 (ICE general
  screenFadeColour). Caller `ICEUserChannelFade::UpdateFrameEnd` 0x826c80f8: a = clamp(element27 * 0.01, 0, 1)
  (0x826c8160/64). Colour comes from int element 108 (0x826c8168): 0 = black (0,0,0), 1 = white (1,1,1),
  2 = (1,0,0), 3 = (0,1,0), 4 = (0,0,1); any other value makes no call (0x826c817c). The call goes through
  vtable slot +0x24 (0x826c81f8).
- Reader: the SetParameters ICE path only (1.2b), with g = ICE general (re-selected at 0x8284c100):
  `m_FXFinalMult = (rgb * a + (1 - a), 1 - a)`, i.e. rgb' = lerp(1, fadeRGB, a) and w = 1 - a
  (0x8284c1f8 lvx g; 0x8284c210 1-a; vmaddfp 0x8284c25c = v0*v12 + v13 with v0 = (r,g,b,0), v12 = splat(a),
  v13 = splat(1-a)). Non-ICE: m_FXFinalMult = (1,1,1,0) (0x8284bc70). The default block (1,1,1,1) also gives (1,1,1,0).
- Shader: `uberPostFX_PS` (0x8212dc28), **c2 = g_FinalMult**; the last instruction is `oC0 = r0 * c2` (ALU 19), so the
  final graded colour is multiplied per channel (alpha by 1-a). uberPostFX also has c1 = g_OverlayAdd and
  c0 = g_ScanlineOffset. ApplyUber only runs in the m_b30HzMode branch of Apply (0x8284cbfc..0x8284cc18).
  m_b30HzMode is set when mICEPostfxActive (or replay FX / editor / screenshot / kbForceNISMode / lock30 / stereo)
  and CAS state == 0 (0x8284c848..0x8284c860). Gameplay uses ApplyFastUber, which has no FinalMult.
- So an ICE FADE of 100 with colour 0 multiplies the 3D image to black BEFORE the HUD/FE (Render2D) is drawn.

### 1.9 PostFE blocks (ColorFade transitions) -> `RenderMan::FinalQuadFade` 0x8283fba0
p = mICEPostfxActive ? +0x2D0 (ICE PostFE) : +0x2E0 (Game PostFE) (0x8283fbb8..0x8283fbd0); v = (r,g,b,a).
If a > 0 (0x8283fc08/18), it draws a full-screen QuadListColoured in CAMERAIDX_ORTHO_SCREENSPACE, sized
frontBufferWidth x frontBufferHeight (GfxMan+0x107F8/0x107FC, 0x8283fc60/68), with colour (r,g,b,a) and blend
CSrcSrcAlpha_CDestInvSrcAlpha_Add (0x8283fd28) => out = mix(dst, rgb, a). It is called at the very end of
RenderModeScene (0x82857fc4), AFTER FE Render2D (0x82857ea4), so this fade covers the HUD too.
BLACK_FADE/WHITE_FADE presentation transitions use this; BLOOM_TO_WHITE uses 1.2-1.6.

### 1.10 wipeEffectType / wipeEffectValue
No reader found. I searched the whole .text listing for Presentation-relative displacements 0x7BA0..0x7BFF and
0x7C00..0x7C3F, and for every `lbz rX,0x2F0(rY)` selector. The selectors are only ApplyUber 0x828337b4 (ICE
cheatEnable), FinalQuadFade 0x8283fbbc, VisualFX_Animation 0x828486e8 and SetParameters 0x8284c100, and none of them
touches +0x10/+0x14 of a general block. ICEPostFX_t.screenWipeValue (+0xEC, written by ICEUserChannelScreenWipe
0x826c9ec0 from element 67) is also not read by SetParameters. Treat both as unused in this build (see the open list).

mICEPostfxActive itself: set to 1 by Scene::Init (stb r21 at 0x82878284, r21 = 1 at 0x82877cdc) when the scene's
property byte +0xA5 is set and the ICE editor is off; set to 0 by Scene::Purge (0x8286aaa8, r29 = 0 at 0x8286a9e8).
SceneLite::Init/Purge do the same (0x8287e384 / 0x8287a840).

## 2. NIS letterbox (ICE user channel 8)

### 2.1 Writer chain (confirmed)
- `ICEUserChannelLetterBox::ICEUserChannelLetterBox` 0x826cdb90: mLetterBoxHeight (this+0xC) = 0.125 (0x826cdbd8),
  mTypeID = 8, label "LBox", format "lBox  %d".
- `ICEUserChannelLetterBox::UpdateFrameEnd` 0x826c84b8:
  `h = this->mLetterBoxHeight (0.125) * clamp(GetValueFloat(take, 26 /*0x1A, 0x826c84ec*/) * 0.01, 0, 1)`
  (0x826c8500..0x826c8520), then `camera->vtbl[+0x28](h)` (0x826c8524..0x826c852c), only if
  TheICEManager.mCameraMover->mCamera != 0 (0x826c84e4).
- `ICEUserChannelLetterBox::Init` 0x826c8478 calls the same slot with 0.0.
- ICELITE twin: `ICELITEUserChannelLetterBox::UpdateFrameEnd` 0x826d44d0 (same math, element 26).
- vtable slot +0x28 = 0x826cd198 `User_ICECamera::SetLetterBox` in both User_ICECamera (vtable 0x820b32f8, entry at
  0x820b3320) and User_ICELITECamera (vtable 0x820b73e8, entry at 0x820b7410). Slot +0x24 = SetFadeColor 0x826d34f8.
- `SetLetterBox` 0x826cd198: `stfsx f1, Presentation, 0x13028` (lis 1 / ori 0x3028 at 0x826cd19c/a0) =
  `Presentation.mCameraPresentation.mObject.mICELetterBoxHeight` (PresentationObj header 0x20, object at
  Presentation+0x11F50, field at CameraPresentation+0x10D8; type_inspect Sk8::CameraPresentation).
- CameraPresentation ctor 0x827f2130 zeroes mICELetterBoxHeight (0x827f22b0) and camLetterBoxHeight (+0x28,
  0x827f22f4).

### 2.2 Readers: none found in this build
Exhaustive listing searches over the whole .text (0x82430000..0x82FC2000, in chunks) for every way the field can be
addressed:
- `0x10D8(`, bare `0x10D8` (CameraPresentation-relative): only the ctor store 0x827f22b0 plus unrelated structs
  (FrontEndState_CaC 0x826800a8, OnlineHubMenu 0x8268c21c, SkateReelGeneral 0x8269bc4c, xml parser 0x82960594,
  DynamicObject 0x82d2e9c0/c8, ChallengePresentationDirector 0x82e06d30).
- `0x13028` (Presentation-relative via lis/ori) and `0x3028(` / bare `0x3028` (via addis 1): only SetLetterBox
  0x826cd1a0.
- 16-byte vector copies of the 0x10D0 block (`0x10D0`, `0x13020`, `0x3020`): only GetFOVY's `lfs f1,0x10D0(r3)`
  (0x827f6510, = mICEFOV) and the ctor.
- camLetterBoxHeight (CameraPresentation+0x28, Presentation+0x11F78): no `0x11F78`/`0x1F78(` hit except the unrelated
  `addi r28,r31,0x1F78` in VIPresentation::UpdateLocationSplines 0x828035ac (r31 there is another object +0x10).
- There is no CameraPresentation getter for the letterbox (the class has GetDOFDepth/FocalDistance/BlurFalloff/
  BlurIntensity 0x827ef918..0x827ef990, GetFOVY 0x827f64b0, GetNearPlane/FarPlane, but no letterbox accessor).
  For comparison, the DOF getters select `mICECameraActive(+0x1042) || mNISSequenceCameraActive(+0x1170) ? mICE* :
  cam*` (GetDOFDepth 0x827ef918).
- Render side: RenderMan::UpdateCamera 0x82848978 (view/projection from GetMatrix/GetFOVY/GetNear/FarPlane only),
  UpdateRenderParams 0x82852780, PostFX (all functions above): no letterbox.
- FE: the only "letterbox" in the FE is the Apt movie `main/letterbox.swf` ("LetterBox", strings 0x8207b968 /
  0x8207b974), toggled by `FrontEndManager::EnableLetterBox(bool)` 0x82610990 ->
  AptIntegration::CallFunction(eASCall 7 "EnableLetterBox", "true"/"false") (0x826109ec / 0x82610a04). Its callers are
  menu/loading code only: StartGameModule 0x82633dcc, ChallengeMap (0x8264dd94, 0x8265f470), Credits::OnOutroDone
  0x8265a408, Extras::OnMenuSelect (5x), OnlineFeManager::Exit 0x8266c49c, MainMenuOnlineOptions (5x),
  MainMenu::StartSinglePlayer 0x82678de8, TOS::OnEnter 0x82694668, SetLoadingState 0x8274834c,
  StateMainGameFlow::LoadLocalSkaterState 0x8277b3d8. It takes a bool, not the ICE height, and none of these callers
  is NIS/ICE code. FrontEndState_NIS::OnEnterState 0x826675a0 has no letterbox screen state either.
- The ICE editor widget `ICEUserChannelLetterBox::RenderUI` 0x826c8548 only prints "lBox %d" in the editor timeline.

Conclusion (as far as IDA shows): the ICE letterbox height is computed (0..0.125) and stored every frame, but
nothing in this executable reads it. No code draws bars from it: no top/bottom quads, no colour, no viewport or
scissor change, and it is not handled by the FE/HUD or by the renderer before or after post-fx. For 1:1 parity the port should store
the value (if it mirrors CameraPresentation) and draw nothing from it. If bars are visible in a real NIS, they come
from another source (e.g. a cMsgNISFEEvent overlay screen, the content of the shot itself, or the FE letterbox.swf
during loading), not from channel 8. The 0.125 factor suggests the authors meant "per-bar fraction of the screen
height", but that use is not implemented in this build.

## 3. Extra verified details used above

- SetBloomCoeefs raw disasm (0x828233c4..0x82823430): f31 = 1.0, f30 = 0.0, f29 = 0.1 (`__real@3dcccccd`);
  stores c[3] at +0x66C, c[1] at +0x664, c[0] at +0x660, m_bloomThreshold at +0x680, m_bloomSaturation at +0x684.
  The narrow-range replacement is `lfs f0, __real@41200000` (10.0) at 0x82823418, so c[2] = 0.1 when 1-t < 0.1.
- VisualFX_Animation prologue (0x82847654..0x828476a0): every frame it resets m_bloomMultiply (+0x180) = (1,1,0,0)
  (0x8284767c), m_VignetteUVOffset (+0x3C0) = (0, 0.25, 0, 0) (0x82847684) and m_OverlayAdd (+0x3D0) = 0
  (0x8284768c). The base threshold is f29 = m_NEWbloom.layout[0] (r8 = this+0x8A4, 0x82847698) and f28 = layout[1]
  (0x828476a0). bFadeInfo branch (VD gameplay fade): threshold -= mNormalizedFadeValue (0x828485f0). If the result is
  < 0, m_bloomMultiply.x += min(-t*0.5, 0.5) (0x82848618..0x82848650). This is a separate gameplay white-out, not the
  NIS transition.
- ICE path bloom multiplies: m_bloomMultiply (+0x180) = (4*bloomIntensity, 4*(1-m_RadialBlur)*bloomIntensity, 0, 0)
  (f4/f5 = 0x8284c32c/0x8284c368, store `stvx128 v37,r31,r9(=0x180)` at 0x8284c3e4).
- Blit_ToDeviceRT 0x82831c18: m_VignetteUVOffset.w = c[0] if c[0] > 0 else 0 (0x82831d5c..0x82831d84).
- Frame order inside RenderModeScene 0x82857a78: UpdateRenderParams 0x82857b18 -> 3D scene -> PostFX::End
  0x82857d60 -> PostFX::Apply 0x82857dec (bloom, visualFX/uber or fast uber, Blit_ToDeviceRT) ->
  FrontEndManager::Render2D 0x82857ea4 (HUD/FE/subtitles) -> FinalQuadFade 0x82857fc4.

## 4. Port notes (observations from reading the repo; not IDA facts)

- apps/skate2-pc/src/live_skating_view.rs:151 builds `postfx.bloom = [frame.bloom[0]*0.5, 1., 0., frame.bloom[1]*4.,
  frame.bloom[1]*4.]`. That matches the ICE-path base (0.5*bloomThreshold, saturation 1, 4*bloomIntensity). The
  bloomToWhite term is missing: per 0x8284c170 it should be `frame.bloom[0]*0.5 - 2.0*bloom_to_white`
  (gameplay path: attrib threshold - 2*b2w, 0x82848718).
- gpu_renderer/postfx.rs:449 already uses `math::bloom_coefficients(threshold)` (= SetBloomCoeefs; the 10.0 quirk
  matches 0x82823418). Still missing for 1:1: the Apply-time lerp of the colour matrices toward identity by
  clamp(c[0],0,1) (1.4), and vignette.w = max(c[0],0) in the final pass (final_pass.rs already documents it, line 52).
- postfx.wgsl `extract` matches the bright-pass math in 1.5. `visual` matches visualFX_PS for a single matrix with
  scene*1.41412354 + bloom.
- screenFadeColour -> g_FinalMult (uber, NIS path) and PostFE -> FinalQuadFade (after HUD) are separate passes with
  different placement relative to the HUD (1.8 / 1.9).
- Letterbox: no drawing to reproduce (section 2).

## Unverified / open

1. Bloom chain internals in the ICE path (Blur9 / Blur9_combined3 weights, the m_bloomMultiply rewrite in
   GenerateBloomBuffers 0x828331bc..0x82833258, ghost pass) were not re-derived here. The claim that the bloom ends up
   ~white relies on the bright-pass offset c.x plus additive compositing. Exact final intensities also depend on
   render-target formats (8-bit vs float clamping of scene*1.414+bloom in m_main2Rt / bloom RTs), which I did not
   check.
2. Source of g_MatrixR/G/B in the bright pass (m_FXMatrixR/G/B_Bloom +0x5A0..0x5C0) was not traced. They multiply the
   bloom colour before the final saturate; I assume they are close to identity.
3. Shader disassembly relies on the repo's xenos_disasm.py conventions (Xenia ucode layout, relative swizzles). The
   results are self-consistent and agree with the port's existing WGSL, but that is not an independent proof.
4. VMX128 decoding: I relied on the Xenia VX128 field layout to correct IDA's VA operand (+32 error) and on the raw
   VA-form encoding for vmaddfp operand order. Both were checked against data flow at 0x8284c254/0x8284c25c,
   0x8284c438 and 0x8284ca30..0x8284caf8.
5. camLetterBoxHeight (CameraPresentation+0x28) reads through a generic `0x28(rX)` displacement cannot be excluded by
   text search. No accessor exists and no Presentation-relative access was found.
6. wipeEffectType / wipeEffectValue (general blocks) and ICEPostFX_t.screenWipeValue: no reader found with the
   displacement and selector searches. A reader using a computed or struct-copy pointer could be missed. In
   SetParameters the copied ICEPostFX_t (stack var_1D0) is not read at +0xEC per the decompiler.
7. Effect of FinalMult.w = 1 - a on the uber output alpha (m_mainRt alpha) downstream: not checked. The final blit
   multiplies the alpha too.
8. If ICE is active but CASPresManager state != 0, m_b30HzMode stays 0 (0x8284c854..0x8284c860). Apply then takes
   ApplyFastUber and screenFadeColour/FinalMult has no visible effect. This edge case was not exercised.
9. ICE element semantics beyond the code: element 26 = letterbox percent, 27 = fade percent, 108 = fade colour enum
   (0..4) come from the GetValue calls. Default values in takes were not checked.
10. The redundant ICE/Game re-select at 0x8284c100 inside the ICE-only branch is harmless as long as mICEPostfxActive
    does not change between 0x8284bed4 and 0x8284c100 (same thread, so it cannot).
