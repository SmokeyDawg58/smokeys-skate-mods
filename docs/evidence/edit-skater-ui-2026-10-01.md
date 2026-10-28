# Native Edit Skater UI and choices ? 2026-10-01

Implementation uses the original createskater APT, imported highlight controls, original RX2 thumbnails and sk8popup. No replacement artwork or generic menu was created. A temporary PC clothing unlock is included; the explicit policy bypasses lock/purchase checks while preserving sex, slot, category and on-disk availability filtering.

## Source chain

- FrontEnd constructor8267ccf8, OnLoaded8268ae20, SetScreenState82684b40, ReceiveInput8268d818 establish the 27 native menus, option table and stack. Ordinary editor entry is main9/CAS_Stand, not the Slappy couch. Slappy1 uses CAS_Sitting. Full64 camera/body keys recovered from PPC lis/ori/insrdi and checked against original Vault: main AD454A0ECD0E2EDF, Slappy B5C3CAC4B3DC0165, Pete E98620ED39C83B7A.
- GetParts8259d510, SortPartList82677d98, PruneWearStyle8259c2b0, DLCInit8259e080 and IsPackOnDisk8258fb20 govern catalogs. No additional DLC package is declared present by the host. Native sorting is shared with existing EASTL execution-order sort. Nullable categories start on Remove; nonnullable categories put current choice first.
- GetIntegerValue82674878, GetStringValue826775f0 and OnUpdate82670400 govern sliders/options. Morph input axes205/35 use authored threshold/step; UI morph enum is layout+8, settings' internal zone is+4. Normalize8258dcb0 and Interpolate8258dbb8 convert to/from authored male/female ranges. Morph undo restores only morphs.
- GetColour82670d28 uses preview RGB at16/20/24. Saved base/highlight use32/0. Colour helper82595918, zone82642d08 determine authored descendants and targets. Hair/eyebrow/facial/eye/skin/board selectors use native attributes; eyebrow filter82648af0 excludes male style7.
- Tattoos82599a70/82595530 concatenate skinned ARM/BODY/LEG zones. Available stamps8259c888/82595678 use typed16 array entries whose first8 bytes are collection keys (type761C03280F994647). MISC graphics8259a6b0 retain database order and remove logoEmpty825931c8. ApplyStamp82599c00 and825962b0 distinguish six embedded tattoos from three material stamps.
- Gender confirmation8268b408/82669460 uses native warning popup and Yes/No ordering. DoSwitchGender82590430 stores old outfit and restores the other sex's outfit or InitializePlayer8258fb90. Outfit Set82b6ab70/Serialize82b64b68 packs17060 bytes. SwitchToOutfit82b6b1a8 restores appearance, thumbnail, stance/style and keeps owned inventory/gestures/physics. First opposite-sex initialization changes default parts/colours/morphs/physics but retains unrelated data. Both native Closet outfits are persisted within SKATER.P.
- BodySet8259e8c8, gesture load8259bf98/82597070 and Process8259d280 use gender-specific authored gesture ABINs with five slots. Constructor82680054..82680470 supplies35 full64 body keys. Presentation8259f230 starts newly selected gesture at time0; returning to base resets time. Existing shared sampling/twist/palette machinery remains the pose owner.

## Assets and reproduction evidence

Original root: `<game folder>`. `data/fe/screens.big` supplies APT/components; `data/fe/cas.big` supplies `data/fe/cas/...rx2` thumbnails. `data/db/db.big` supplies skater.vlt/bin. Extraction for research is outside the repository.

APT fingerprints: createskater BB8D5810, popup E82ED6EC. Movie slots15 and25 plus imported highlight slots16?24 extend the existing APT owner. Popup original frame labels/callbacks, panel layout, picker, buttons and font resources are retained. MarkerUi handles original HAL localization and native texture drawing.

Raw evidence is in `artifacts/edit-skater-ui-native-20261001.txt`, `edit-skater-menu-data-20261001.json`, `edit-skater-apt-20261001.txt`, `edit-skater-popup-20261001.txt`, and the eight `edit-skater-highlight_*-20261001.txt` dumps. Closets use offsets colour15008, tattoos15584, sex15608, installed15609, morph16025, stamps16109, stance16988, style16992, skin16996 in each17060-byte record.

## Verification and limits

Rustfmt parsed the changed Rust sources. No build, tests or game run was performed; runtime and visual parity remain unverified. Original gesture libraries are loaded through shared Andale parsing; a non-direct clip or mismatched hierarchy reports an error rather than an invented animation. Available gesture IDs initially come from equipped saved gestures; a caller may supply the native unlock list. The temporary unlock override applies to clothing, not every gesture. Online/user-generated graphic acquisition is not added; shipped material graphics and tattoos are supported. Native operation scheduling is represented by the host facade's existing load/preview lifecycle rather than emulating original thread queues. The popup wraps text using original glyph advances and native font/size/panel constants; final rendered positioning needs a runtime check.

## Final static integration review

`EditSkaterSession` caches one CharacterHierarchy per sex, loaded lazily from the original editor ABIN. It no longer constructs a full EditSkaterAnimation solely to obtain names for every clothing preview. Read-only original anim.big inspection confirms cac_edit_male.abin hierarchy `cac_male` and cac_edit_female.abin hierarchy `cac_female`: each has148 bones/18 parts and the ordered bone-name lists are identical (trajectory, hips, spine, spine1 first). The room retains the full animation owner and reloads it when sex changes.

Replacement now loads and finalizes the actual return-to-game scene before removing the old registry entry, then transfers that same allocation to the existing loader (no duplicate geometry load). Runtime material admission precedes SKATER.P commit. Face loading, attribute application and presentation restoration still occur after registry release: a failure there preserves the disk profile but can leave the live session partially replaced. This is not an atomic rollback transaction. Source-native registration completion still waits for the published peer rather than manufacturing an acknowledgement.

Static integration review covered session lifecycle, selected mounted startup, player recipe replacement, presentation restoration and LiveSkatingClient scene switching. Rustfmt parsed the three edited integration files with skip_children enabled. No compiler, check, tests or game were run.

Pause opening now requires the actual player loader to be Loaded; pending registration keeps simulation/publication running. open_edit_skater also checks Loaded after the existing completion helper, so a peer mismatch cannot strand a Registering player in the paused editor. No acknowledgement is invented.
