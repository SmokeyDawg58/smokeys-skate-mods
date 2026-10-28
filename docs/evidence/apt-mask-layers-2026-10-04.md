# Apt mask layers (clip depth) - native semantics - evidence 2026-10-04

Source: debug XEX `sk82_na_m.xex` in IDA (read only; nothing renamed, typed,
commented or saved). Data: original `data\fe\screens.big` and
`data\big\render.big`, extracted read-only into a scratch folder (never
into the repo). Markers: **VERIFIED** = read in disassembly/decompilation at
the cited address; **DERIVED** = follows from verified pieces but was not
observed at runtime; **UNVERIFIED** = not fully checked. VMX128 operands quoted
below were decoded from the raw instruction words because IDA's text misreports
VMX128 register high bits (same caveat as `marker-hud-2026-09-30.md`).

## 0. Summary

1. The EA Apt core treats a placement as a mask layer when its 16-bit clip
   depth is non-negative. It records the mask geometry with
   `AptMaskRenderOperation` Add (+1), records the clipped content with None (0)
   and a mask-depth counter n+1, then records the same mask geometry again with
   Subtract (-1). That is a stencil-style design (VERIFIED, sections 1-4).
2. Skate 2's renderer callback `Sk8::FE::AptRenderingIntegration::DrawRenderingUnit`
   0x8261da48 uses **no stencil** and **ignores the mask depth**. It turns each
   mask element into ONE axis-aligned rectangle. The rectangle comes from
   vertices 0 and 3 of the mask shape's first geometry element. Only the
   accumulated matrix's translation is applied: scale, rotation and mirroring
   are dropped. Add writes depth 0.0 into that rectangle. Subtract writes depth
   1.0 into the last rectangle saved by any Add. No colour is drawn for mask
   geometry (VERIFIED, section 5).
3. All Apt shapes are drawn at z = 0.5 with LESSEQUAL and no depth write.
   Render2D clears the depth buffer to 1.0 once at its start (VERIFIED).
   **DERIVED:** content drawn while a mask is active is rejected *inside* the
   rectangle and drawn everywhere else. The native "mask" is therefore a
   rectangular occluder: the opposite of Flash clipping, and its size ignores
   scale. This was not observed on hardware or an emulator. Confirm it with a
   capture before shipping a port (section 10).
4. A stencil implementation of Flash masks would **not** be 1:1 with the Xbox.
   To match it, emulate section 8. A 1-bit "occluded" stencil reproduces the
   depth trick exactly.
5. Shipped data: 40 mask placements in 6 movies, all in `screens.big`.
   killedit holds 34 of them: shape masks driven by animated shape tweens. The
   other 6 are sprite masks that each contain one rectangle shape. There are no
   text, morph or button masks, no `setMask` scripts, and no nested or
   overlapping masks (section 9).

## 1. `AptMaskRenderOperation` values (Q1)

| Value | Name (IDB enum) | Meaning | Evidence |
|---|---|---|---|
| 0 | `AptMaskRenderOperation_None` | normal drawing | top-level call `li r6,0` 0x82a0318c (`_AptInternalRender` 0x82a03110); DrawRenderingUnit case 0 0x8261dadc -> 0x8261dc80 |
| 1 | `AptMaskRenderOperation_Add` | push mask geometry | `li r6,1` 0x82a004e0 (clip layer) and 0x82a005cc (setMask); case 1 0x8261dae4 -> 0x8261db08 |
| -1 | `AptMaskRenderOperation_Subtract` | pop mask geometry | `li r6,-1` 0x82a004a0, 0x82a00814 (clip pop), 0x82a007ac (setMask pop); case -1 0x8261dad4 -> 0x8261dcfc |

* Any other value reaches `FailAssert` (aptrenderingintegration.cpp line 383,
  0x8261daf8).
* The value is stored in `AptRenderInfo.meMaskOper` (+0x60) and the counter in
  `AptRenderInfo.mnMaskDepth` (+0x5C). See `AptCharacter::render` 0x829f8e68,
  stores at 0x829f8f30/0x829f8f40.
* Apt reuses the field for effect elements (`PushEffectRenderElement`
  0x829f8360 sets Add as "push"; `AptRenderElement::Render` 0x829f1e40 calls
  `pfnPushEffect` if Add, else `pfnPopEffect`). Sk8 never installs
  `pfnCreateEffect`, so no effect elements exist (section 5.1).

## 2. Mask detection and storage

* PlaceObject2 `AptDisplayList::placeObject` 0x82a17580 passes payload+0x34
  unmodified as `nClipDepth` (`lwz r10,0x34(r31)` at 0x82a176a4 and 0x82a17850).
  The move-an-existing-instance path passes -1 (0x82a178e0) but does not
  re-instantiate, so a move never changes the clip depth. PlaceObject3
  0x82a17928 reads the same +0x34 (0x82a1794c).
* The flag 0x40 is not consulted. `AptDisplayList::instantiateCharacter`
  0x82a119a0 stores the value as int16 in `AptRenderItem.mnClipDepth` (+0x16)
  at 0x82a11e38.
* A render item is a mask layer iff `(u16)mnClipDepth < 0x8000`
  (`lhz 0x16`, `cmplwi 0x8000`, 0x82a004c8/0x82a004cc). So clip 0 is a mask
  too; the port's `entry.clip_depth > 0` differs only there.
* Shipped data: every non-mask placement stores 0xFFFFFFFF (36172
  placements). Every mask stores a small positive value with flag 0x40
  (40 placements). `crates/skate2-formats/src/apt/movie.rs` reads the same word
  (`o + 52`, o = payload).
* Display lists are kept in ascending depth order (`AptDisplayListState::findInst`
  0x829da118 walks `while depth < nDepth`). Traversal order is back-to-front.
* Render item fields used below (all VERIFIED by use):

| Field | Meaning |
|---|---|
| `mnDepth` +0x14 | int16 |
| `mnClipDepth` +0x16 | int16 |
| flags +0x18 bit31 | `mbIsVisible` |
| flags +0x18 bit30 | `mbIsMask` (setMask target) |
| flags +0x18 bit29 | `mbHasMask` |
| flags +0x18 bits 18-23 | character type |
| `mpMask` +0x1C | setMask mask |

* CIH fields: `mpNext` +0x18, `mpParent` +0x1C.

## 3. Traversal (EA Apt core) (Q2)

### 3.1 `AptDecoupleTreeTraversal` 0x82a003d0

Signature: `(cur, tick, ctx, op, n, levels, useLevelFilter)`. The top-level
call passes op 0, n 0 (`li r7,0` 0x82a03188) and useLevelFilter 1
(`li r9,1` 0x82a03178).

```
clipper = clipEnd = scriptMask = null
for (; cur; cur = cur.next)                                    // 0x82a007f0
  if useLevelFilter:                                           // root level list only
    if !((1 << (i16)cur.item.mnDepth) & levels): continue      // 0x82a00434..0x82a00450
    suCurrentRenderAnimLevel = depth                           // 0x82a00454
  if clipper:
    if (i16)clipper.mnClipDepth >= (i16)cur.mnDepth: goto DRAW // 0x82a00484/0x82a00488
    n -= 1                                                     // 0x82a0048c
    Clipper(clipper ..= clipEnd, Subtract, n)                  // 0x82a004b0 (pop)
    clipper = clipEnd = null
  if (u16)cur.mnClipDepth < 0x8000:                            // mask layer
    clipEnd = ClippedTreeTraversal(cur, Add, n)                // 0x82a004f0
    clipper = cur; cur = clipEnd; n += 1                       // 0x82a004f8..0x82a00500
    continue
DRAW:
  skip if cur or any ancestor has mbIsVisible == 0             // 0x82a0050c..0x82a0054c
  if mbHasMask && mpMask:                                      // setMask, 0x82a00550..0x82a0060c
    scriptMask = GetMask(); PushRenderDataAbsolute;
    MaskedTreeTraversal(Add, n); PopRenderData(Add, n); n += 1
  if !mbIsMask:                                                // 0x82a0061c
    if sprite/movie (list at inst+0x1C) or button (inst+0x14) with children,
       and render-item type != 16:
      PushRenderData(op, n); Render(op, n);                    // 0x82a006b8/0x82a006ec
      AptDecoupleTreeTraversal(children, op, n, levels, false) // 0x82a0070c
      PopRenderData(op, n)
    else: Render(op, n)                                        // 0x82a00760
  if scriptMask:                                               // 0x82a0076c..0x82a007ec
    n -= 1; MaskedTreeTraversal(Subtract, n); scriptMask = null
end
if clipper: Clipper(clipper ..= clipEnd, Subtract, n - 1)      // 0x82a00810/0x82a00824
```

### 3.2 `AptDecoupleClippedTreeTraversal` 0x829f8bf8

```
child = GetFirstChild(mask)                                    // 0x829f8c1c
if child && child.type != 16:                                  // 0x829f8c38..0x829f8c40
  PushRenderData(Add, n); Clipper(child ..= end of list, Add, n); PopRenderData
else: mask.Render(Add, n)                                      // 0x829f8cf4
last = mask
for sib = mask.next; sib; sib = sib.next:                      // 0x829f8cf8..0x829f8e58
  if (i16)mask.mnClipDepth >= (i16)sib.mnDepth: break          // 0x829f8d2c/0x829f8d30
  last = sib; render sib (recursively) with Add
return last                                                    // 0x829f8e5c
```

`AptDecoupleTreeTraversalClipper` 0x829f8a88 renders begin, begin.next, ...
up to and including `end` (`cmplw r27,r26` at 0x829f8bdc), or the whole list
when `end` is NULL. It recurses into every container child (end is passed down
unchanged, so whole child lists are drawn). It does **no** visibility, level,
setMask or clip-depth checks.

### 3.3 Which depths are clipped

* The content is the siblings that follow the mask in the same display list
  with `(i16)depth <= (i16)clipDepth`. That is the range `(maskDepth, clipDepth]`,
  including clipDepth (`bge` at 0x82a00488). Their descendants are drawn inside
  it.
* The mask element carries n. The content carries n+1. The pop happens at the
  first sibling with depth > clipDepth, or at the end of the list. It re-renders
  `[mask ..= clipEnd]` with Subtract and n.
* Normally clipEnd is the mask itself, so the pop re-renders only the mask.
  It uses the same CIH in the same traversal, so the transform and colour are
  identical to the Add.

### 3.4 Quirks (VERIFIED in code)

* **(a) Mask visibility is ignored.** Mask detection comes before the
  visibility walk, and Clipper never checks `mbIsVisible`. A `_visible=false`
  mask layer still masks. A sprite mask also draws its invisible children.
* **(b) Overlapping masks in one list are not supported.** While a clip range
  is active, 0x82a00488 jumps straight to DRAW and skips the mask test. A
  second mask layer inside the range is drawn as ordinary visible content with
  op None, and its clip depth is ignored.
* **(c) An empty range swallows everything above it.** If the sibling right
  after the mask has depth > clipDepth, then that sibling and every later
  sibling are emitted as mask geometry (Add), followed by Subtract. None of
  them is emitted as content (loop at 0x829f8d2c). Linear timeline playback of
  every shipped mask never empties its range (scratch `empty_range.py`).
  Script-driven removals are UNVERIFIED.
* **(d) Sequential masks work.** Mask A (d1,c3), content d2, then mask B
  (d4,...) gives: pop A at d4, then B is detected (0x82a004bc).
* **(e) Nested masks across hierarchy levels work in Apt.** n keeps
  incrementing. See section 7 for what the renderer does with them.

### 3.5 Elements per render-item type (op recorded / honoured)

| Item type | Native behaviour | Evidence |
|---|---|---|
| Shape | `PushMatrices`, then `AptCharacter::render`. Only `eType == Shape` emits an element carrying op/n. | 0x829fa878; 0x829f8e8c, 0x829f8fa0 |
| StaticText | Each glyph shape goes through `AptCharacter::render` with op, so glyphs act like shapes. | 0x829fa260 |
| DynamicText | `AptRenderableString` element carrying op/n. The renderer ignores op (section 5.6). | 0x82a15648 |
| Morph | Start character drawn with alpha (1-ratio), end character with alpha ratio, both via `AptCharacter::render` with op. | 0x829fa730 |
| Button | `Render` emits nothing; children are traversed. | 0x829e0990 |
| CustomControl | Element carrying op/n. `CustomControlRender` 0x826363d8 never reads +0x60. | 0x829fa078 |
| Sprite | `Render` slot is an empty function: vtable 0x821faf78 +0x10 -> 0x825187c0. | 0x821faf78 |

## 4. Render buffer and callbacks

* `AptRenderBuffer::Add` 0x829f1f50 appends elements in traversal order.
* `AptRender` 0x829f6b20 -> `AptRenderListSet::Render` 0x829f2090 ->
  `AptRenderBuffer::Render` 0x829f2030 replays them in that order, once per
  `AptRender(level)` call. Elements are filtered by `(1 << mnLevel) & mask`
  (0x829f1e88).
* Before each renderable, `CallLegacyTransformCallbacks` 0x829eec98 calls
  `pfnSetColourTransform` and `pfnSetVertexMatrix` with the element's
  accumulated cxform and 2D matrix.
* Then:
  * geometry -> `gAptFuncs.pfnDrawRenderingUnit` (`AptRenderableGeometry::Render`
    0x829dad40);
  * strings -> `pfnDrawString` (0x829dadc8).
* The accumulated transform and colour come from the AptMath stack:
  * `gAptOptFlags |= 4` (`AptIntegration::Initialize` 0x8263e6e8) selects
    `_drawCharacterInstOpti` 0x829e58f0;
  * matrix is MatMul2d;
  * cxform multiplies byte scales and saturating-adds translations.

## 5. Sk8 GPU implementation (Q3)

### 5.1 Callbacks

`Sk8::FE::AptIntegration::SetupCustomCallbacks` 0x8263e5c0 installs:

| Callback | Function |
|---|---|
| `pfnDrawRenderingUnit` | `DrawRenderingUnit` 0x8261da48 |
| `pfnDrawString` | 0x826096d8 |
| `pfnSetColourTransform` | 0x82609488 |
| `pfnSetVertexMatrix` | 0x826095e8 |
| `pfnCustomControlRender` | 0x826363d8 |

No effect callbacks are installed, here or in `ApplyLoadingCallbacks`
0x82a72ba0 / `ApplyVariableCallbacks` 0x82a71870.

### 5.2 `DrawRenderingUnit(unit, info)` 0x8261da48

**Early out.** The draw returns if `ColourScale.a <= 0`. At 0x8261da98,
`vcmpgefp128.` compares splat(0.0) with splat(ColourScale.a); the all-true CR6
bit branches away at 0x8261daa8. This applies to Add, None and Subtract alike.
`ColourScale` = (r,g,b,a) of the element's accumulated cxform scale /255
(`SetColourTransform` 0x82609488). On levels other than 0/4/8/9 a VisualDirector
fade forces a = 0.

**Add** (0x8261db08..0x8261dc74):

1. Get the geometry, with no type check:
   * `unit0 = AptShape::GetRenderingUnit(0)` 0x826019a0;
   * `strip = AptVectorUnit::GetStrip(unit0, 0)` 0x82601e30.
2. Load `p0 = strip.pPositions[0]` and `p3 = strip.pPositions[3]`
   (`lwz 4(r3)` 0x8261db1c, `+0x30` 0x8261db24).
   * Positions are (x, y, 0.5, 1): the element's triangle-list vertices
     (`AptVectorUnit` ctor 0x82609ee8, one strip, NumVerts = 3*prims at
     0x82609fe0).
   * So p3 is the first vertex of the second triangle.
3. Transform: `q = p.xyz + VertexMatrix.row3 * p.w`, with rows 0-2 replaced by
   `gIVector`/`gJVector`/`gKVector`. Raw decode at 0x8261db58..0x8261dbb4:
   * `vmulfp128 v6,v63(I),v58(p.x)`;
   * `v3 = v0(J)*v11(p.y)+v6`;
   * `v1 = v13(K)*v9(p.z)+v3`;
   * `v30 = v12(VertexMatrix+0x30)*v7(p.w)+v1`.
4. Row 3 is (tx, ty, 0, 1) (`SetAptMatrix` 0x82601780; rows (a,b,0,0),
   (c,d,0,0), (0,0,1,0), (tx,ty,0,1)). So only translation is applied.
5. `rect = [min(qx),max(qx)] x [min(qy),max(qy)]` (0x8261dbc8..0x8261dc1c).
   It is saved in four statics (one rectangle, no stack):

| Static | Address |
|---|---|
| `m_maskX` | 0x8304874c |
| `m_maskY` | 0x83048750 |
| `m_maskX1` | 0x83048754 |
| `m_maskY1` | 0x83048758 |

   The statics are 0 at startup.
6. `renderToDepth(0.0, rect)` at 0x8261dc24.
7. Afterwards `StateShadow::SetState(ZTestDisable_ZWriteEnable)` 0x8261dc34
   and `(CSrcZero_CDestOne_ASrcZero_ADestOne)` 0x8261dc44. Then
   `cFxStateHandle::mCurrent` blend/depth are re-applied (0x8261dc5c,
   0x8261dc74). Because this happens after the quad, these calls do not affect
   it.

**None** (0x8261dc80..0x8261dcf0):

1. `SetState(ZTestEnable_ZWriteDisable)` 0x8261dc8c.
2. `SetState(CSrcSrcAlpha_CDestInvSrcAlpha_Add)` 0x8261dc9c.
3. `AptShape::Draw(VertexMatrix, ColourScale, ColourTranslate,
   overrideZDepth=false, 0.0)` 0x8261dcc0.
4. Restore `mCurrent`.

**Subtract** (0x8261dcfc..0x8261dd24): `renderToDepth(1.0, saved m_mask*)`.
It uses the saved rectangle, not this element's own geometry.

### 5.3 Mask quad

* `renderToDepth(depth,l,r,t,b)` 0x82609770 builds a quad list
  (l,t),(r,t),(r,b),(l,b) at z=depth with world = identity.
* It calls `SimpleDraw::Draw_QuadList_DepthOnly` 0x8283b2c0, which:
  * sets `i_ConstantDepth = (depth,0,0,0)` (0x8283b3b4..0x8283b418);
  * calls `cMaterialManager::Dispatch(DepthOnly, force 0)` 0x8283b434. This
    applies the DepthOnly pass states through `Technique::Dispatch` 0x82e162c0
    whenever the scope changed;
  * sets `D3DDevice_SetPixelShader(NULL)` 0x8283b444;
  * draws `D3DPT_QUADLIST` with 4 vertices (0x8283b4a0).
* Technique `DepthOnly` in `render.big` `data/render/shaders/final/simpledraw.xml`:
  * ZFUNC ALWAYS, ZWRITEENABLE true, COLORWRITE 0, ZENABLE true;
  * no stencil states (parser default StencilEnable 0, `cFxParser` ctor
    0x82e1a4e8).
* `DepthOnlyVS` (simpledraw UPDB source):
  `Out = mul(mul(Pos,i_matWorld), g_matVP); Out.z = i_ConstantDepth.x*Out.w`.
  NDC depth is therefore exactly 0.0 (Add) or 1.0 (Subtract).

### 5.4 Content state

* `ZTestEnable_ZWriteDisable` is defined in `AddDefaultDepthStates` 0x82e17de0:
  * ZEnable 1;
  * ZFunc 3 = LESSEQUAL (`li r25,3` 0x82e17e04, stored 0x82e18170;
    `DepthStencilFunction` 0x82e15680 maps "LESSEQUAL" to 3);
  * ZWrite 0, StencilEnable 0.
* The value reaches the GPU unchanged:
  * `functionTranslation` 0x83024ee4 is the identity;
  * `renderengine::Device::SetState` 0x82bcbdc8 calls
    `D3DDevice_SetRenderState_ZFunc` 0x82c34f08, which writes the
    RB_DEPTHCONTROL ZFUNC field (Xenos 3 = LEQUAL).
* Apt shape draws dispatch with force 4 (`DrawParameters::Draw` 0x8283a784):
  that is `DispatchEffectNoStates` 0x82e16708, so the state above stays in
  effect.
* Vertex z = `ZDepth` 0x82fdecf0 = 0.5f (`AptVectorUnit` 0x8260a064,
  `AptTextureUnit` 0x826099b8). It is never overridden (0x8261dcb8).

### 5.5 Frame setup

* `AptRenderingIntegration::Render2D` 0x82631a98:
  1. Camera 8.
  2. `XBox2GraphicsMan::Clear(depthOnly=1)` 0x82631ac4 -> `Device::Clear`
     0x82bcbfd0 -> `D3DDevice_Clear(flags 0x30, Z 1.0, stencil 0)`. Values from
     `mClearDepthStencilParams` {0x30, 1.0, 0}: GraphicsMan ctor
     0x8282d4cc..0x8282d4d4; 1.0 again at 0x8282c6d4.
  3. States RasterizerState_Solid, ZTestDisable_ZWriteDisable,
     AlphaBlendDisabled_AlphaTestDisabled (0x82631ad4..0x82631af4).
  4. Levels: 8 (camera 8); 7..2 (camera 7, perspective); 0 and 9 (camera 8)
     (0x82631b84..0x82631c40).
* There is **one depth clear per Render2D**. Occluders therefore persist
  across levels until the next frame.
* Camera 8 (`AptRenderingIntegration` ctor 0x826091c0, `SetOrthoMatrix` call
  0x8260933c):
  * view = identity;
  * `D3DXMatrixOrthoOffCenterLH` with l=0, r=1280 (4:3: 160..1120), b=720,
    t=0, zn=0, zf=1;
  * so NDC z = object z.
* A depth surface is bound:
  * `PostFX::Apply` 0x8284c7a0 ends with
    `Device::SetState(m_defaultRenderTargetState)`;
  * the depth slot of that state is `m_depthStencilSurface` (`Device::Start`
    0x82bcef80);
  * that surface is auto-created (`depthStencilAutoCreate=1`,
    `Parameters::Initialize` 0x82bcb520);
  * `D3DDevice_SetRenderState_ZEnable` 0x82c34e98 forces Z off only when no
    depth surface is bound.
* **DERIVED:** content at 0.5 with LESSEQUAL passes where depth = 1.0 and
  fails where an Add wrote 0.0.

### 5.6 Text, custom controls, effects

* `DrawString` 0x826096d8 ignores `meMaskOper` and has the same alpha early
  out.
* `SkateAptString::Draw` 0x82a71150 brackets each line with
  `IFont::SetTestDepth(true/false)` (vtable +0x3C) and draws it at z 0.5.
* `cFont::DrawstringLocal` 0x82855550 selects the `font.xml` technique
  `depthTested` (ZENABLE true, ZWRITE false, ZFUNC = parser default LESSEQUAL)
  through `GetFontEffectPair` 0x82854f18. It dispatches with states (force 3).
* Result: text content is occluded like shapes. Text that is part of a mask is
  drawn **visibly**, twice (Add and Subtract).
* Custom controls are UNVERIFIED (section 10).

### 5.7 GPU state changes (answer to Q3)

* **Stencil:** no `D3DRS_STENCIL*` changes anywhere in the Apt path. All
  states used keep StencilEnable 0. The only stencil write is the frame clear
  (0).
* **Mask Add/Subtract quad:**
  * ZENABLE 1, ZFUNC ALWAYS, ZWRITE 1;
  * colour writes off (COLORWRITE 0) and NULL pixel shader;
  * depth 0.0 (Add) or 1.0 (Subtract) over the rectangle only.
* **Content:** ZENABLE 1, ZFUNC LESSEQUAL, ZWRITE 0, src-alpha/inv-src-alpha
  blending, z = 0.5.
* **Clear:** once per Render2D, depth 1.0 and stencil 0, flags 0x30. The Xbox
  360 D3DCLEAR bit names are UNVERIFIED; the TARGET bits are 0x0F, so 0x30 is
  Z+stencil.

## 6. Mask colour transform and alpha; text/sprite masks (Q4)

* The mask element carries its accumulated cxform, but Sk8 draws no colour for
  it.
* The only effect: if the accumulated alpha-multiplier byte is 0
  (`ColourScale.a <= 0`), the Add is skipped entirely, and so is the matching
  Subtract (5.2).
* Morph masks: the start and end characters are skipped independently by
  their alpha (1-ratio)/ratio (3.5).
* The colour translate is irrelevant.
* Sprite masks: every child of the mask sprite is drawn with Add via Clipper,
  including invisible children (3.2). Each child's Add overwrites the single
  saved rectangle.
* Text masks:
  * Dynamic-text masks would draw visible text and never occlude (5.6).
  * Static-text masks would add one rectangle per glyph.
* In the shipped data, mask characters are shapes (34) and sprites (6). Each
  sprite mask holds exactly one 2-triangle rectangle shape. There are no text,
  morph or button masks (section 9).

## 7. Nesting and limits (Q5)

* **Apt:** no explicit limit. n is a plain int, incremented per active clip
  layer or setMask and passed down the recursion.
* **Sk8:** ignores n. A scan of the FE Apt rendering code 0x82601000..0x82640000
  and 0x82a6f000..0x82a74000 found no reads of +0x5C, and the only read of
  +0x60 is 0x8261dad0. Sk8 keeps one global rectangle.
* Nested masks therefore are not restored correctly. Sequence: Add A, Add B,
  content, Subtract B, Subtract A. The second Subtract restores B's rectangle
  again, so `A minus B` stays occluded until the next Render2D clear.
* Same-list overlap: the inner mask is drawn as visible content (3.4 b).
* Shipped data contains no nested masks (transitively through timelines) and
  no overlaps (scratch `nesting.py`).

## 8. Port recipe (native-exact, stencil form)

Per Render2D frame:

1. Clear `occluded = 0` (this mirrors the depth clear to 1.0). Keep
   `last_rect` across frames (statics), with initial value (0,0,0,0).
2. Walk the element list in order, per level.
3. For each element, skip it if its accumulated alpha byte is 0. Then:
   * **Shape Add:**
     1. Take vertices 0 and 3 of the first geometry element.
     2. Add the accumulated (tx, ty). Ignore a, b, c and d.
     3. Store the bbox in `last_rect`.
     4. Draw the bbox with stencil REPLACE 1, colour writes off.
   * **Shape Subtract:** draw `last_rect` with stencil REPLACE 0, colour
     writes off.
   * **Shape None, and text with any op:** draw with stencil test EQUAL 0.
4. If the mask's first geometry element is textured (not a solid vector
   unit), the native code misreads it (section 9). Log it; do not invent a
   rectangle.
5. Reproduce the traversal quirks of 3.4, because they decide which elements
   get Add/None/Subtract.

A Flash-correct stencil (shape coverage, EQUAL n) would be a deliberate
deviation from the Xbox.

## 9. Data census and killedit

Scratch tools (not in the repository): `scan_masks.py`, `nesting.py`,
`empty_range.py`, `setmask_scan.py`, `clip_values.py`. They scanned all 594
APT/const/geo files in `data\fe\*.big` and `data\big\*.big`.

* No file contains the string `setMask`.
* Mask placements by movie (all `screens.big`, `data/fe/source/screens/...`):

| Movie | Holder | Mask | Clip range -> content |
|---|---|---|---|
| challenge/killedit | sprites 32, 62 | shapes (below) | see below |
| hud2/hudthrowmes | sprite 43 | sprite 8 -> rect shape 7 (392x134) | d6 c13 -> sprite 14 (d8). Mask slides by translation f154-160. |
| hudcomponents/filmtreatment | root | sprite 7 -> rect shape 6 (6.8x37.9) | d5 c9 -> sprite 10 (d7). Mask x-scaled 1 -> 3.1+ from f2. |
| hudcomponents/unlock | sprite 22 | sprite 12 -> rect shape 11 (4x19) | d5 c9 -> sprite 21 (d7) |
| hudcomponents/unlock | sprite 112 | sprite 38 -> rect shape 1 (545x512, inner (1.007,0,0,1.467,18,394.95)) | d1 c8 -> `mc3LinePaper` sprite 41 (d3), textured shape 43 (d7). Window height set by mask y-scale per label 1line..6line. |
| main/main_options (legacy) | sprite 9 | `mcBarMask` sprite 3 -> rect shape 2 | d2 c5 -> bar fill shapes 4/8 (d4). Script sets `mcBarMask._xscale` = setting. |
| online/messenger_new | root | sprite 3 -> shape 2 whose first element is **textured** (kind 2, 762x470) | d1 c215 -> sprite 92 (d3) |

* Native consequence 1: scale-driven masks (filmtreatment, unlock,
  main_options) collapse to the unscaled rectangle plus translation.
* Native consequence 2 (messenger_new):
  * `AptTextureUnit` keeps `m_strip` at +8, so read as an `AptVectorUnit`,
    `m_numStrips` = 6 and `m_pStrips` = `pPositions`;
  * `strip.pPositions` is therefore the bit pattern of vertex 0's y,
    430.0f = 0x43D70000;
  * the rectangle comes from arbitrary memory at 0x43D70000/0x43D70030.
    UNVERIFIED runtime effect.

### killedit (`data\fe\source\screens\challenge\killedit.apt`)

Both masks are shape masks placed with flags 0x46 (character|matrix|clip),
identity matrix and cxform ffffffff/0.

**Sprite 32: mask at depth 1, clip_depth 8.**

* Mask shapes by frame: 17 (f0), 22 (f2), 23 (f4), 24 (f6), 25 (f8), 26 (f10),
  27 (f12), 28 (f14), 29 (f16), 30 (f18), 31 (f20). Every 2 frames the timeline
  removes depths 1, 2 and 5 and re-places them.
* Content in range: depth 2 is sprite 21 at (135,-96); depth 5 is sprite 21
  mirrored (-1,0,0,1,-135,-96). Sprite 21 = textured shape 20, 256x192.
* 21 frames; Stop at frame 20 (0x20014).
* Root placement: depth 1 on frames 28/143/258/373 (labels
  bl_flourish/brflourish/tlflourish/trflourish) at (640,312). Scaled 1.5 at
  f106 (scale ignored natively). Removed at f108.

**Sprite 62: mask at depth 1, clip_depth 4.**

* Mask shapes by frame: 37 (f0), 41..61 (f1..f21), 37 (f22), one per frame.
* Content in range: depth 3, textured shape 40, 288x128.
* 23 frames; Stop at frame 22 (0x20018).
* Placed twice inside sprite 63:
  * depth 1 at (82.75,-0.25);
  * depth 6 mirrored (-1,0,0,1,-82.75,0.25). The mirror is ignored natively.
* Sprite 63 is placed by the root at depth 14 on frames 44/159/274/389 at
  (639.75,406.75), with small jitter. Removed at f110.

Per-frame native rectangles (mask-local coordinates; add the accumulated
translation). Flash bbox is the authored mask's extent, for comparison.

| Sprite | Frame | Shape | Prims | Native rect | Area | Flash bbox |
|---|---|---|---|---|---|---|
| 32 | 0 | 17 | 8 | (-442.5,-61.5)-(-147.5,116.5) | 52510 | (-442.5,-183.5)-(474.5,152) |
| 32 | 2/4/6 | 22/23/24 | 8 | (-442.5,-61.5)-(-391.5,124.5) | 9486 | ~(-442.5,-183.5)-(474.5,136..148) |
| 32 | 8 | 25 | 8 | point (-442.5,-61.5) | 0 | (-459.5,-183.5)-(512.5,124) |
| 32 | 10 | 26 | 8 | point (-514.5,-139.5) | 0 | (-531.5,-183.5)-(534.5,58) |
| 32 | 12 | 27 | 7 | (-538.5,-207.5)-(-496,-40) | 7118.75 | (-555.5,-207.5)-(530.5,-8) |
| 32 | 14 | 28 | 7 | (-485,-318)-(-442.5,-150.5) | 7118.75 | (-502,-374)-(450,-8) |
| 32 | 16/18/20 | 29/30/31 | 7 | points | 0 | (-342..-306,-292)-(350..360,-150..-8) |
| 62 | 0, 18-22 | 37, 58-61 | 7-8 | (90.5,91.5)-(133.5,103.5) | 516 | (-71.5,-140.5)-(216.5..303.5,159.5) |
| 62 | 1-6 | 41-46 | 2 | (-58.5,34.5..-38.5)-(-7.5..264.5,129.5), growing | 4845..54264 | same, from x -85.5 |
| 62 | 7-17 | 47-57 | 2-8 | small to medium (scratch `killedit_table.py`) | 328..34020 | (-99.5,-145.5)-(181.5,147.5) |

Under the native model:

* The ornaments are drawn whole except inside these rectangles. Rectangles of
  area 0 hide nothing.
* After sprite 32 stops, the ornaments stay fully visible until removed at
  f108.
* With Flash clipping, shape 31's extent (y -292..-150) would hide them
  (DERIVED).

## Port (2026-10-04)

Re-verified in IDA before use: `DrawRenderingUnit` 0x8261da48 (Add/None/Subtract, vertices 0 and 3,
translation only, saved `m_mask*`), the mask test 0x82a004c8 and range test 0x82a00484,
`ZTestEnable_ZWriteDisable` ZFunc 3 (0x82e18170) and camera 8's `SetOrthoMatrix` 0x8260933c. The PC
`flatten_projected` walks each display list as `AptDecoupleTreeTraversal` does and cuts every content
triangle drawn in a mask's range out of the mask rectangle on the CPU (the four regions around it,
shared edges rasterising once), the pixels the depth test rejects. It refuses what no ported movie
needs: sprite masks, textured or line first units, masks on perspective levels and empty ranges.
killedit is the only ported movie with masks.

2026-10-07: getting a S.K.A.T.E. letter crashed with "Apt sprite mask not ported": hudthrowmes'
`mSkate` (character 43) masks depths 7-13 with sprite 8, whose only child is rectangle shape 7.
`flatten_projected` now keeps one `MaskDepth` per movie draw (the depth-0 rectangles and the saved
`m_mask*` rectangle), shared by every nesting level as the single global rectangle is (section 7):
* Mask geometry follows `AptDecoupleClippedTreeTraversal` and `AptDecoupleTreeTraversalClipper`:
  the mask and each following sibling past its clip depth (the 3.4 c swallow case), a clip with a
  display list recursing into all its children, visible or not (3.4 a). A mask inside an active
  range draws as content (3.4 b).
* A shape Add writes its rectangle and saves it, and a shape Subtract removes the saved rectangle
  from the hidden area. Both are skipped under an alpha byte of 0 (section 6).
* What a clip draws itself is cut out of the hidden area when it is drawn.
It still refuses textured or line first units (section 8.4), text or morph mask geometry, and masks
on perspective levels. The Dispatch-skip question in section 10 stays open: a Subtract here always
restores. `original_skate_letter_banner_masks_its_letters_with_a_sprite` checks the banner frame by
frame: only depths 7-13 are cut, and the outro carries the letter panel under the rectangle, which
hides it.

## 10. UNVERIFIED / open items

* No runtime or footage confirmation of the occluder result (sections 0.3 and
  5.5). Recommended checks, ideally Xenia or real hardware:
  * the career unlock popup (unlock sprite 112 paper);
  * the killedit corner flourishes.
* `CustomControlRender` 0x826363d8 depth behaviour; the scan only checked that
  +0x60 is not read.
* The text path when a text draw directly follows a mask Add. The device then
  holds the DepthOnly pass state, and the font technique re-dispatches with
  states.
* Dispatch is skipped when two DepthOnly draws are adjacent with no other
  material in between: `Dispatch` returns early when the scope equals the last
  scope and force is 0 (0x82e16858). `i_ConstantDepth` may then not be
  re-uploaded. That happens only in degenerate sequences, e.g. Add then
  Subtract with nothing drawn between. In that case the Subtract would write
  the previous constant again.
* Script-driven display-list changes inside clip ranges (the 3.4 c swallow
  case) and runtime-attached clips.
* Levels 2-7 (perspective camera 7, `SetApt3DMatrix` variants): the rectangle
  maths uses that matrix's row 3. Not analysed.
* Type 16 is assumed to be the custom-control render-item type (the name is
  UNVERIFIED).
