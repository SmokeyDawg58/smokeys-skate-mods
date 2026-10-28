# Four Motion expressions and unfinished Boneless602 investigation

## Implemented, source checked

Pre1188290ddd8 -> condition198 (sConditions+318); Pre1718290f2fc -> condition266 (+428); Pre1818290f704 -> condition274 (+448); Pre1988290fdc4 -> condition287 (+47c). All call existing HasIntent logic, including cached presence/comparison semantics. Sustain1818291fce8 independently reads274; Sustain118/171/198 go to native true default829221c8.

Added focused Pre/Sustain regression coverage (not executed). Rustfmt parse-only PASS on three touched Rust files.

## NOT FIXED: SystemLogic PendingTransition602

602 is PhysState_Boneless (GetType82d978c8), not an expression ID. Existing state allocation/constructor is in physical_player_remaining_states_fields.rs and physical_player_remaining_states_simple.rs, with Attribute class0xccb95a83c78b4ff9. No live CurrentPhysicalState, transition entry/update, fixed-part skeleton update or Boneless launch integration was implemented in this batch. Preserve the error; do not claim all five failures fixed.

Source traced:
- Vtable8234a578: Update82da0d20, Enter82d89638, Exit825187c0 empty, PredictFutureOfDeck82d83ae8 inherited, GetType82d978c8, FillPhysOut825187c0 empty, Post82d896b8, final slot825187c0 empty.
- Enter82d89638: EnableBoardAnimation; Processed+8c4 mask04000000 picks toe19 for right leg, otherwise15; capture GetPartTransform translation as landing_toe_pos; retain right_leg flag.
- Update82da0d20: Skeleton.UpdateFixedWorldPositionOfBone82b5d588(processed, captured toe, part), disable selected foot collisions3, set corresponding world IK target at blend1; if Processed+8d0 low bit and kDoJump, DoJump82d9ebe8.
- FixedWorldPosition82b5d588 calls UpdateRootTransformsFixedPart, composes animation record with anim-to-world into Processed root, sets Processed+8c4 bit80000, ApplyBoardAnimation(true), updates physics-board/skate-root matrices, GeneralUpdate, resets next trajectory matrix. Numerous decompiler VMX128 high-register operands are wrong: decode raw instruction fields before porting arithmetic.
- CalcBonelessJump82d978e0 uses Boneless attributes, processed ground normal/velocity and authored eight-point graphs, plus kMaxJumpErrorBoneless. Hidden struct-return arguments are mislabeled by decompiler. Do not copy pseudocode field names literally.
- DoJump82d9ebe8: CalcBonelessJump, Skeleton.GetLaunchInfo, replace velocity, player-jumped and explicit start/end position flags; positions use captured toe projected against ground normal and kTrajPosDelta; TrajectorySelector.Launch then Update.
- Post82d896b8 calls CheckForPlantWipeout82dde908: set WIPEOUT_TRICK; ordered squash18/collision1/contact-force0 tests, then deck acceleration check. Existing shared contact tests/IK world setters/trajectory launch machinery are candidates for reuse, not replacements for Boneless-specific sequencing.
- Existing selector physical_player_system_logic_session.rs selects602 from GroundAnimation103 when Processed flags8d0 bit20000. physical_player_ground_to_air.rs selected_transition currently rejects602. Full integration also requires current-type/category publication, update3 owner/update/controller path, postphysics, next-state selection and update5 empty-fill handling.

No compilation/runtime. Renderer unchanged. Boneless602 remains outstanding and must be completed before claiming this batch complete.
