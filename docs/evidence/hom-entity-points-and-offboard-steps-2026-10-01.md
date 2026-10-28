# Hall of Meat entity points and off-board step updates

Playtest reports:
- Score Attribute UnsupportedArray11412694815255377850 (9e620977a93d37ba).
- Motion PendingStateUpdate2190411432 (828f06a8).

No build/check/test or runtime launched. Both edited
Rust sources passed rustfmt syntax parsing, which does not establish type or
runtime correctness. Read-only authored-data and state-slot checks also passed.

## Scoring array

IDA PedestrianWipeoutScorer.GetPointValueFromEntityType82decde8 and the paired
VehicleWipeoutScorer method82decec8 iterate their original attribute arrays,
compare the entity-type word at offset0 and return the unsigned points at4.
An absent element uses DefaultDataArea(8); no matching type falls back to the
separate original default-points attribute, using DefaultDataArea(4) if absent.

The existing EntityScorer.entity_points already implemented this loop/default
policy, but called scalar try_get_data for each array element. It now uses
the existing checked try_plain_array_data with type d215ad904e7f2356 and size8.
Both pedestrian and vehicle consumers share this fix. No score values or new
attribute decoder were added; malformed type/stride/relocation checks remain.

Read-only inspection using tools/vault_dump.py of the original extracted
skater.vlt/bin under %TEMP%/skate2-score-assets-20260930/data/db:
- Class9e4c471bc6eab615, default collection.
- Pedestrian key9e620977a93d37ba, type d215ad904e7f2356, size8, max16, flags05.
  BIN800596 header capacity1/count1/stride8/padding0; record (type3, points500).
- Vehicle keyff1ced9b60b28cda has the same type/size/flags.
  BIN800612 header capacity2/count2/stride8/padding0;
  records (type12, points2000), (type13, points2000).
These are evidence values only; runtime continues to read the authored Vault.

## Step update dispatch

IDA828f06a8 is StateUpdate_StepBackward_310 and is shared by StepForward309.
It calls OffBoard_1 (828eddc8) once, then packed slots14/15/16:
SteppingBoardNoBoard, CreateAttribute, PlayAnimation. Generated state ancestry
is leaf -> Idle105 ->33 ->8 ->OffBoard1 ->Motion0. The intervening parents
have no additional behaviors. The existing generic runtime already has the
instances and all three callback implementations; only the explicit update
route was missing.

Added exact expected lists to the existing lifecycle dispatcher:
- 309: global behaviors0..13,756,757,758.
- 310: global behaviors0..13,759,760,761.

Read-only ancestry expansion verified both lists and their unique17-slot order.
Existing active-set and instance-identity validation is retained, and callbacks
run once per slot through the existing dispatcher. No animation/state clone.

SteppingBoardNoBoard.Update82ca68c8, reached through thunk82ca6ab0, reads
PhysOut.OffBoard byte63 (holding board), chooses its board/no-board attribute
name, and adds a zero-valued animation attribute. Existing
motion_stepping_board_attribute matches that path and required no replacement.
