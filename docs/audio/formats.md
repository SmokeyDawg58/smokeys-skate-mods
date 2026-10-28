# Audio file formats

All values are big-endian unless stated. "Verified" means checked against both the shipped data and
the parsing code in the debug xex. Paths are relative to the game root (`data/audio/...`).

## 1. Inventory (parsed 2026-10-02, Jan-17-2009 data)

| Location | Kind | Notes |
| --- | --- | --- |
| `audio/*.big` (19 archives) | BIG4 | AEMS banks, splicer banks, grains, ambience/post/fepost/loadscreen streams |
| `audio/*.bh` (ambience, emitters, splicercollision, splicerdmo) | Viv4 hashed header | Those four archives are mounted with `bUseBigHeader = true` |
| `audio/context/*.big` (6) | BIG4 | Speech context XML + `script.css`, `Trigger_Events.csi/.evt` |
| `audio/{english,french,german}/*.big` + `.bh` | BIG4 + Viv4 | Speech `.dat` clips, `*hdr.big`, `*sth.big`, `*Output_Events.evt/.csi`, NIS `.sns` |
| `audio/music/{game,ipod,world}.mpf` + `*_Stream.mus` | Pathfinder | Interactive music project + stream data |
| `audio/{sfx,music,reverb,crowds,speakers}.ems` | Emitter placements | World positions/radii per emitter (§9) |
| `audio/MixMapSK8.mxb` | Nicotine mix map | Loaded by `AudioSystem::InitializeMixer` (§10) |
| `audio/Whls_spins_{Jump,Man}_1.snr/.sek` | EAAC header + seek table | Loose wheel-spin loops |

Totals by entry type across all audio BIGs: `.dat` 9,373 (1,132.7 MB), `.sns` 1,119 (301.9 MB),
`.xml` 362, `.abk` 257 (71.2 MB), `.bnk` 37, `.snr` 31, nested `.big` 29, `.csi` 23, `.evt` 15,
`.grain` 14, `.css` 3, `.txt` 1. `emitters2.big:NPC_Cellphones.abk` is a 0-byte entry.

## 2. BIG4 / Viv4 archives — already implemented in the port

`crates/skate2-formats/src/big.rs` (`BigArchive::parse`, named `BIG*` and hashed `Viv4` tables,
`djb2` name hash) and `mount.rs` (`MountedBig`, `MountSet::read_path`) already read these, refpack
included. Audio must reuse them.

* BIG4: `'BIG4'`, u32 **little-endian** archive size, u32 count, u32 header size, then entries
  `{u32 offset, u32 size, cstring name}`. Verified: `skateboard.big` reports 0x004CF8F4 =
  5,044,468 bytes = file size.
* Runtime mount: `xcore::BigFile::BigFile(path, bUseBigHeader)` 0x82970fb0 →
  `BigFile::Mount` 0x8296e958 → `rw::core::BigHandler::AddArchiveFromFile(path, bUseBigHeader)`.
  Files are then opened **by bare name** (e.g. `"SK8_AEMS_skateboard.csi"`) through
  `xcore::FileLoader::SyncLoad`, which searches the mounted archives.

## 3. EAAC sample header ("SNR") — verified

Parsed by `rw::audio::core::SndPlayer1::UnpackHeader` 0x82bf2390 with a `BitGetter` (MSB first):

| Bits | Field | Notes |
| --- | --- | --- |
| (prefix) | — | If the first byte is `'H'` (0x48) skip a 4-byte chunk header |
| 4 | version | stored at request+76 |
| 4 | codec | stored at request+72 — **all shipped content = 3 (EA-XMA)** |
| 6 | channels − 1 | |
| 18 | sample rate | 22050 / 24000 / 36000 / 44100 / 48000 occur |
| 2 | type | 0 = RAM, 1 = stream, 2 = gigasample |
| 1 | loop flag | |
| 29 | sample count | |
| 32 | loop start | only if loop |
| 32 | prefetch samples | only if type 2 |
| 32 | loop stream offset | only if loop and (type 1 or (type 2 and loopStart ≥ prefetch)); else 0 |

Data follows immediately. With no header present: channels 0, codec −1, type 1, loop offset 0,
version 1. Example: `board_scrapes.abk` sample 0 = `03 00 BB 80 | 00 00 2E E0` → v0, EA-XMA, mono,
48000 Hz, RAM, no loop, 12000 samples. `ambienceresident.big` `.snr` files: 5 channels, 48 kHz,
stream, looped (e.g. 7,210,496 samples).

## 4. EAAC chunked data ("SNS" / RAM payload) — verified

`SndPlayer1::Format1ChunkParser` 0x82bf21c8 and `GetNextChunk` 0x82bf2658:

* Chunk = `[u8 type][u24 size]` (size includes the 4-byte chunk header).
* `type == 'E'` (0x45): last chunk (parser returns 2); other types return 1.
* `type == 'H'`: header chunk (consumed only when the request expects one); `'U'`: skipped.
* Data chunks (`0x00`): `[u8 0][u24 size][u32 samplesInChunk][codec payload]`.
* Alternative `Format0ChunkParser` 0x82bf2140: u32 with bit 31 = last and low 31 bits = size.

Seek tables: `rw::audio::core::SeekTableParser` (`ParseHeader0/1`, `ParseChunkSection0/1`).
Format details are UNVERIFIED (open-research R-F3).

## 5. EA-XMA codec payload — verified (decoder behaviour)

From `rw::audio::core::EaXmaDec::StartPair` 0x82c14200, `Service` 0x82c161b0, `Reset` 0x82c0e650,
`DecodeEvent` 0x82c169f8 and `ParseToNextFrame` 0x82c0e8a0:

* Channels are decoded in **stereo pairs** (one XMA context per pair; mono = one 1-channel pair).
* A data chunk payload is one sub-block per pair, concatenated:
  `[u32 w][XMA2 packets…]`, where `w >> 2` = sub-block size in bytes including `w`, and
  `w & 3` = the XMA context `DecoderTable` (passed to `Reset`).
* XMA packets are **2048 bytes**: a 32-bit packet header, then 16,352 bits of frame data.
  `firstFrameBitOffset = (header >> 11) & 0x7FFF` (≥ 0x3FE0 means no frame starts in this packet).
  Each frame begins with a **15-bit length in bits** read at `frameOffset + 32`. A frame crossing the
  end of the packet continues in the next one (ParseToNextFrame advances 2048 bytes and subtracts
  16352 bits).
* XMA context init: `NumSubframesToDecode 4`, `InputBufferReadOffset 32`, `NumChannels = pair − 1`,
  `OutputBufferSize 24` (× 256 = 6144-byte ring), two 2048-byte input buffers (ping-pong).
* **Frames decode to 512 samples.** At the start of a stream request (`feedType == 0`) the decoder
  skips **384 priming samples**: frames to skip = `(decoderSkip + 384) / 512` (without the +384 for
  continuation feeds), and `padding = 512 − ((decoderSkip(+384)) & 511)` samples are discarded at the
  end.
* **Hardware output is signed 16-bit PCM** (2 bytes × channels per sample), converted to float by
  `CopySamplesOut`. A bit-faithful PC decoder should produce s16 first, then convert.
* Seeking: optional per-pair seek data of varint (`Unpack0::UnpackUInt32`) frames-per-packet counts.
* Stall handling: `sXmaStallTimeoutThresholdMs` → `EnterRecoveryMode` → zero-filled output.

Codec census across all banks and `.snr` (4,936 samples): **100 % EA-XMA**, with 1/2/4/5/6 channels.
Per-archive breakdown: [open-research.md](open-research.md#census).

## 6. AEMS module bank (`.abk`, magic `ABKC`) — verified

`AemsDef::ModuleBank` (PDB, 92 bytes) is the file header:

| Off | Type | Field | `board_scrapes.abk` |
| --- | --- | --- | --- |
| 0x00 | char[4] | id | `ABKC` |
| 0x04 | u8 | ver | 1 |
| 0x05..07 | u8×3 | veraimex major/minor/patch | 1.2.2 |
| 0x08 | u8 | platform | 5 |
| 0x09 | u8 | targetType | 3 |
| 0x0A | u16 | nummodules | 1 |
| 0x0C | s32 | debugcrc | 0 |
| 0x10 | s32 | uniqueid | 0 |
| 0x14 | s32 | totalsize | 0x1EFBC (= file size) |
| 0x18 | s32 | residentsize | 0x1F00 |
| 0x1C | s32 | moduleoffset | 0x5C |
| 0x20 | s32 | sfxbankoffset | 0x1F00 |
| 0x24 | s32 | sfxbanksizepadded | 0x1D034 |
| 0x28 | s32 | midibankoffset | 0 |
| 0x2C | s32 | midibanksizepadded | 0 |
| 0x30 | s32 | funcfixupoffset | 0x1EF34 |
| 0x34 | s32 | staticdatafixupoffset | 0x1EF38 |
| 0x38 | s32 | interfaceOffset | 0x1EF98 |
| 0x3C.. | runtime | modulebankhandle, pSnd10SampleBankHeader, midibhandle, streamfilepath, streamfileoffset, list node, ptweakheader | runtime fields |

Loading (`SNDAEMS_addmodulebank` 0x82c1b7d0, `SNDAEMSI_resolvemodulebank` 0x82c1b478):

1. `modulebankhandle = ++counter` (wraps to 1 on overflow).
2. Allocate `sfxbanksizepadded + residentsize` (resident part + sample bank). The resident part
   ([0, residentsize)) is copied and the sample bank follows it (relocated if the allocation moved).
3. **Function fixups** (`FUNCFIXUPHEADER {s32 n; u32 off[n]}` at funcfixupoffset):
   `*(u32*)(bank+off) = sndaemsfuncs[*(u32*)(bank+off)] − (bank+off) − 4` — a relative function
   reference. In the port, keep the function **index** (0..0x27) instead of an address.
4. **Static data fixups** (`{s32 n; u32 off[n]}`): `*(u32*)(bank+off) += bank` — rebase pointers;
   the port should keep offsets.
5. **Interface references** (`{s32 n; {u32 handleOffset, u32 descOffset, u8 kind, pad[3]}[n]}`):
   `desc = {s16 systemCrc, s16 interfaceCrc, char name[]}`; `kind` 0 = GlobalVariableHandle,
   1 = ClassHandle, otherwise FunctionHandle; bound by name/CRC (`SetFast`).
6. **Modules** at moduleoffset; record stride = `60 + 4·numPlayers + 4·numClassControllers`.
   `AemsDef::Module`: `id; ClassHandle classHandle; ClassConstructorClient constructorClient;
   s16 curinstances, maxinstances; u16 numGlobals, numFunctions; u8 numPlayers,
   classDestructorPresent, classDataPresent, numClassControllers; pcode; pdata; datasize;
   destroydataoffset; moduleInstance list`, then `u32 playerStateOffsets[numPlayers]` (each player
   state's `pModuleBank` is pointed at the bank), then class-controller offsets.
   The constructor client is subscribed to the module's Csis class, so creating that class runs
   `SNDAEMSI_CreateModuleInstance`.
7. The stream file path is copied when given; the first bank registers the AEMS timer.

The full PDB definitions of every module SETTINGS/STATE struct (about 100 types) are reproduced in
[engine.md](engine.md#aems-structs).

## 7. Embedded sample bank (`S10A`) — verified

At `sfxbankoffset`: `Snd10SampleBankHeader {char id[4] = 'S10A'; u8 version; u8 pad; u16 serialNumber;
s32 numSamples}`, followed by `s32 offset[numSamples]` (relative to the `S10A` start; −1 = none)
and per-sample EAAC headers and data (§3–5). Players refer to samples via
`Snd10SampleEntry {s16 sampleNum (0xFFFF = none); u8 priority; u8 azimuth[5]; s32 streamFileOffset}`.
`SkateSamplePlayerFactory::CreateInstance` clamps a negative `streamFileOffset` to 0 and passes both
the RAM sample pointer and the stream path/offset to SndPlayer1. The RAM-versus-stream selection rule
is UNVERIFIED (R-A2).

## 8. Csis interface (`.csi`, magic `MOIR`) — partially verified

Loaded by `FileLoader::SyncLoad(name)` then `Csis::System::Subscribe(data)`. The header starts
`'MOIR' 00 03 01 00 …`. It holds the class, function and global-variable descriptors with name
strings (e.g. `r_GrabSpeed`, `Class_pre_lands_whsh`, `FX_Helicopter`, `Class_Squeaks`,
`SenseOfSpeed_wind`, `Rolling_Rattle_Class`, `Class_wheels_flip`). The exact descriptor layout is
UNVERIFIED (open-research R-F1). The CsisDef structs in the PDB give the in-memory form.

## 9. Emitter placement files (`*.ems`) — partially verified

`sfx.ems` begins `u32 count = 0x28E (654)`, `u32 5`, `u32 0`, then per-emitter float data such as
position (1722.53, 83.45, −547.70), radii (69.0, 42.0, 69.0), 0.6 and 1.0. Also `music.ems` (0x50),
`reverb.ems` (0x1A3), `crowds.ems` (0x24) and `speakers.ems` (0x80). The record layout and
consumers (`Sk8EmitterSystem::LoadGameFiles`, `EmitterSystem` octree) are UNVERIFIED (R-F2).

## 10. Nicotine mix map (`MixMapSK8.mxb`) — header and sections verified

`stMixMapHeader {MixMapID 0, NumStates 13, StateTableOffset 0x10, DynamicMapOffset −1}`, 13 state
offsets, then per-state `stMixMapStateHdr` and section headers. Full layout, record formats and
per-state counts are in [nicotine.md](nicotine.md). Consumed by `IDynamicMixer::InitMap` →
`NFSMixMaster::InitMixMap` → `NFSMixMap::InitMixMap` / `PreProcessMixMap`.

## 11. Splicer bank (`.bnk`, magic `SPLC`) — not yet decoded

e.g. `CellPhone_Rings.bnk`: `'SPLC' 00000003 00001164 00000005 00000001 00000000 00000005
"CellPhone_Rings"…`. Loaded as `eAssetType_SplicerData` into numbered slots (`Sk82_FE.bnk` slot 25,
`sk8_menu.bnk` slot 26). UNVERIFIED (R-S1).

## 12. Grain file (`.grain`) — not yet decoded

`grains.big` holds 14 surface grains (`asphalt_rough_hard.grain`, `asphalt_rough_soft.grain`, …),
played by `GrainPlayer::AudBoard`. UNVERIFIED (R-G1).

## 13. Music (`.mpf` magic `PFDx`, `.mus`) — not yet decoded

`game.mpf` begins `'PFDx' 05 03 B0 03 …` (EA Pathfinder project, v5). `*_Stream.mus` holds the
streams. Driven by `MusicCore` (Path::IPathTrack). UNVERIFIED (R-M1).

## 14. Speech — partially verified

* `<lang>/<group>speech.big` contains `<group>Output_Events.evt/.csi`, `<group>hdr.big` (one `.hdr`
  per clip, the VOX/event header format, R-SP1), `<group>sth.big` (one `.sth` per clip) and the `.dat`
  audio.
* **`.sth` (verified by census):** a sequence of records `{u32 offset into the .dat; 8-byte EAAC SNR
  header (§3); +4 bytes if the loop flag is set}`, one per variation or line. Every record parses as
  version 0, type 1 (stream), no loop.
* `.dat` = EAAC chunked data (§4) addressed by the `.sth` offsets.

| Language | announcer | cameraman | livingworld | maincast |
| --- | --- | --- | --- | --- |
| English | 2,605 × EA-XMA 1 ch 41000 | 2,218 × EA-XMA 2 ch 44100 | 7,827 × EA-XMA 1 ch 44100 | 11,198 × **EALayer3 v1** 1 ch 44100 |
| French | 897 × **EALayer3 v1** 1 ch 22050 | 1,307 × EA-XMA 2 ch 44100 | 6,323 × EA-XMA 1 ch 44100 | 4,977 × **EALayer3 v1** 1 ch 44100 |
| German | 1,343 × EA-XMA 1 ch 41000 | 1,302 × EA-XMA 2 ch 44100 | 6,252 × EA-XMA 1 ch 44100 | 5,002 × **EALayer3 v1** 1 ch 44100 |
* `context/<group>context.big` holds `script.css`, `Trigger_Events.csi`, `Trigger_Events.evt`;
  `context/<group>xml.big` holds the rule XML.
* The VOX bank format (`VOXBANKHDR`, `VoxEvent`, `VoxSentence`, `VoxPhrase`) is in the spch library.
  UNVERIFIED (R-SP1).
