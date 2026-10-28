# Nicotine dynamic mixer (`MixMapSK8.mxb`)

Nicotine is the EA (NFS-era) dynamic mixer. Each frame it turns SFX-controller inputs (speed, impact,
distance, game state) into every SFX object's final mix outputs: volume, pitch (cents),
azimuth, filter values and dB roll-off. It runs in `AudioSystem::Update` phase B as
`IDynamicMixer::ProcessMixMap(dt2)` before every state manager's `ProcessUpdate`. Status: **data model
and file header verified; processing algorithms are open (R-N1).**

## 1. Objects — verified (PDB)

* `Nicotine::IDynamicMixer` (36 bytes, a base of AudioSystem at +0x04): `miCamState`,
  `NFSMixMaster* mMixMaster`, `NFSLiveLink* mLiveLink`, `MAP_CREATE_PARAMS {mPrintSelect, NumMixStates,
  MixerAllocator}`, `SnapshotMixer* mpSnapshot`, `mpRefreshedMapData`. Functions: `CreateMixer`
  0x829ca168, `InitMap` 0x829c9f48, `ProcessMixMap` 0x829c9fb0, `DestroyMap` 0x829ca010.
* `NFSMixMaster` (128 bytes): main `NFSMixMap*`, map data, `mNumStates`, `m_StateRefCount[25]`,
  `m_bMapReady`, `m_pSBActiveMasks`, `m_LoadMapID`. Functions: `InitMixMap`, `CreateMainMainMap`,
  `ProcessMixMap`, `AssignSFXCallbacks`, `DestroyMainMainMap`.
* `NFSMixMap` (564 bytes): per-state ref counts, `stMixMapHeader*`, map type, camera state
  (prev/cur), `m_fDeltaTime`, `m_msDeltaTime`, state procs, `DMixIO` object arrays, the counters and
  allocation blocks for mix controls, sub/master channels, 3D and event controls, scale ratios, curves
  (`stCurveDataProc`, `m_CurveProcsTotal[10][2]`), and `m_fDeltaTimeRatio[2]`. Key functions:
  `InitMixMap` 0x829d1330, `PreProcessMixMap` 0x829cf288, `ProcessMixMap` 0x829d0df8,
  `UpdateSubChannels` 0x829cc8c8, `UpdateMasterChannels` 0x829cc990, `MixMasterChannels` 0x829cca58,
  `Update3DMixCtls` 0x829cd9f0, `UpdateEvtMixCtls` 0x829ce770, `UpdateADSREvent` /
  `UpdateASREvent` / `UpdateAREvent`, `ConnectMixMap` 0x829cc518, `GetCurveDataPtr`, `AddScaleIDs`.
* `NFSMixMapState` (96 bytes): one per (state, instance). `Initialize(map, stateIndex,
  numStateCopies, objIndex)` 0x829d1390 (the first instance's ref count is shared across copies);
  `CreateMixCtls`, `Create3DMixCtls`, `CreateSubMixChannels`, `CreateMasterMixChannels`,
  `CreateEvtMixCtls`, `InitializeSub/MasterChannels`.
* `NFSMixShape`: `GetCurveOutput`, `GetFloatCurveOutput`, `GetAzimShapeOutput`,
  `GetCentsFromPitchMult` / `GetPitchMultFromCents` / `GetIntPitchMultFromCents`.
* `Nicotine::DMixIO` (16 bytes): `m_ID`, `int* m_pDMixInputBlock`, `int* m_pDMixOutputBlock`.
  `DMix3DIO` is the 3D variant. The ID encodes state, instance and SFX id; see
  `ConnectDMixIO` in game-layer.md §4.
* `Nicotine::SnapshotMixer` / `SnapshotChannel` / `SnapshotStatus` / `SnapshotHeader`: snapshot
  volume lerps per master channel (`mi16NicotineVolume`, `mi16SnapshotVolume`, attack time).

## 2. File layout — verified on MixMapSK8.mxb (19,248 bytes)

```
stMixMapHeader { MixMapID = 0, NumStates = 13, StateTableOffset = 0x10, DynamicMapOffset = -1 }
s32 stateOffset[13]                       ; absolute
per state, at stateOffset: stMixMapStateHdr {
    StateIndex (= 0x1F0000 | eMAINMAPSTATES), OffsetMixCtlData, Offset3DMixCtlData, OffsetSubMixData,
    OffsetMasterMixData, OffsetPresetData, OffsetEventCtlData, reserved }   ; offsets relative to the
                                                                            ; state header, -1 = none
section headers (16 bytes each):
    stMixCtlHdr   { NumMixCtls, NumNewMixDataProcs, NumMainMixDataProcs, NumMainMixCtlOut }
    st3DMixCtlHdr { Num3DMixCtls, NumMainMap3DMixCtls, reserved, reserved }
    stMixChHdr    { NumMixChannels, NumUniqueSFXOBJs, NumMainIn, NumSecIn }   (sub and master)
    stMixEventHdr { NumEvents, reserved ×3 }   (the reserved words hold tool garbage)
```

| State | MixCtls | 3D ctls | Sub ch | Master ch (unique SFX objs) | Events |
| --- | --- | --- | --- | --- | --- |
| MAIN | 118 | 1 | 10 | 57 (7) | 136 |
| PLAYER | 102 | 21 | 21 | 89 (12) | 36 |
| ENVIRONMENT | 3 | — | 1 | 4 (1) | — |
| COLLISION | 1 | 2 | 1 | 5 (1) | — |
| TRAFFICCAR | 12 | 14 | 4 | 19 (3) | 1 |
| PEDESTRIAN | 12 | 19 | 6 | 23 (2) | — |
| EMITTER | — | 1 | 2 | 8 (1) | — |
| CROWD | 2 | 2 | 2 | 6 (1) | — |
| DYNAMICOBJECT | 1 | 1 | 2 | 9 (2) | — |
| SPEAKER | 12 | 12 | — | 12 (1) | — |
| WHOOSH | — | 1 | 1 | 2 (1) | — |
| OBJECTINSTANCE | — | — | — | — | — |
| NISCHARACTER | — | 1 | 2 | 5 (1) | — |

### Record formats
* **Mix control** (`stMixCtlParams` + variable tail), parsed by `NFSMixMapState::CreateMixCtls`
  0x829d1548: `{s32 nINPUTID; s32 nUScaleCntSwing; s32 scaleIDs[(nUScaleCntSwing >> 16) & 0x1F]}`.
  * `nINPUTID`: bits 11..15 are replaced at load by the object (instance) index
    (`id & 0xFFFF07FF | objIndex << 11`) before `GetCurveDataPtr`.
  * `MIXCTLOBJID = (mapType << 8) | ((nINPUTID >> 16) & 0xE000) | (nINPUTID & 0x0FFF0000) | ctlIndex`.
  * `nUScaleCntSwing` bit 15 set: swing is negative, so `nRatio = 0x7FFF − dBToQ15(swing)`. Clear:
    `nOffset = swing & 0x7FFF` and `nRatio = 0x7FFF − dBToQ15(−nOffset)`.
  * dB values are in **hundredths of a dB**. `dBToQ15(x)`: `k = x / −602` (whole 6.02 dB
    halvings) and `r = remainder`, giving `g_dBToQ15XForm[601 − r] >> k` for k ≤ 15, else 0.
* 3D control (`st3DMixCtlParams`, 28 bytes): `nINPUTID`, `st3DStateParams {n3DSTATEINFOID,
  nCURVEID_DOPPLER, nQ0..Q3MinMax}`. Runtime state: azimuth, dB roll-off, Q15 roll-off, Doppler cents,
  previous distance and delta, `fMinMaxDist[4][2]`, camera-state transitions.
* Sub channel (`stSubMixChParams`): `{MIXCHID, UpperLowerSwing}` + inputs.
  Master channel (`stMasterMixChParams`): `{MIXCHID, MixData, SFXOBJID}` + inputs + presets.
* Event control (`stMixEvtParams`, 24 bytes): `{nEVTCTLID, nUScaleCntSwing, nTriggerID, nParam_00..02}`;
  runtime envelope stage, elapsed ms, start, trigger pointer, outputs, and attack/decay/sustain/release
  durations.

## 3. Constant tables in the executable — extracted

Section 0004 (initialised data), Nicotine `nfsmixshape.cpp` / `NFSMixMap.cpp`:

| Symbol | Address | Contents |
| --- | --- | --- |
| `g_dBToQ15XForm` | 0x82ff2b48 | 602 × s32, 16384 → 32730 (one 6.02 dB octave of Q15 gains) |
| `g_Q15todBXForm` | 0x82ff34b0 | 512 × s32, 0 → 601 |
| `g_nArrayCosTable` | 0x82ff3cb0 | 512 × s32 Q15 cosine, 32767 → 0 (+1 trailing 0) |
| `g_fPitchSemitoneTable` | 0x82ff44b4 | 12 × f32 equal-temperament semitone ratios (1.0 … 1.8877) |
| `g_fPitchCentTable` | 0x82ff44e8 | 100 × f32 cent ratios (1.0 … 1.05885) |
| `nUnityMixScale` | 0x82ff467c | 32767 |
| `F_DT_FRAME_LOCK` | 0x82ff4680 | 0.0333667 (1/29.97 s) |
| `DOPPLER_SMOOTHING_FACTOR` | 0x82ff4684 | 0.2 |
| `nDUMMY_3DOUT` | 0x82ff4688 | −10000 |

The values live in the executable, not in game data, so the port must embed them as constants with
this evidence. The full arrays were extracted to the research scratch (`nicotine_tables.json`); the
implementation phase will commit a generated table with a source note.

## 4. Open (R-N1)
`InitMixMap` / `PreProcessMixMap` (allocation and wiring, including `NumStateCopies` per state,
which presumably follows the state managers' state counts); the `ProcessMixMap` order; curve data
(`stCurveDataProc`, `GetCurveDataPtr`, the 10 curve kinds); scale IDs; sub and master channel maths
(`UpperLowerSwing`, presets); `Update3DMixCtls` (distance roll-off curves, Doppler with smoothing
0.2, azimuth relative to `MicrophoneMan`, camera states); event envelopes (ADSR/ASR/AR); snapshot
lerps; and how `SFXObjBase::Get*FromMixer` read the output blocks.
