# Start menu input and live pause scheduling

Source-ready, not compiled or runtime tested.

## Evidence

- `InputManager::LoadInputMap` 0x826dde58 compiles AptStart134, AptNext136,
  AptPrevious137, AptUp146 and AptDown147. The existing shared input.cfg
  parser now admits these five expressions in production.
- Original `miscboot.big/data/config/input.cfg`, read directly through the
  repository's existing read-only Python BIG/RefPack helpers: AptStart is
  Start.pressed; AptNext A.pressed; AptPrevious B.pressed; AptUp
  LStickU.repeat || DPadU.repeat; AptDown LStickD.repeat || DPadD.repeat.
  LStickU is Button18, LStickD Button19. No user remapping or thresholds added.
- `cInputExec::Execute` 0x8298ba00 opcode4 (0x8298bbd0..bc28) evaluates
  logical OR left first, reads right only if left is zero, emits 0/1.
  The bounded expression parser implements this ordering; no guest VM added.
- Existing `cInputPad::Update` 0x829863b0 already owns threshold >0.5,
  transition debounce, first pulse and subsequent 24/12 update repeats.
  Disconnect count0 produces no menu input and reconnect reinitializes exposed
  button history. Reused without a second hardware input owner.
- `TimerCallback` 0x82729770 polls once and caches SIM then PRES. Frontend now
  drains PRES through the existing retained cache; gameplay drains SIM once.
- `StartHub::OnEnter` 0x8265e548..e584 sends SetSimRate(1/60,1). The
  existing sim-rate owner queues and dispatches this on entry, restoring the
  normal timer before resume without advancing physics or camera.
- `CheckForPauseState` 0x826477b0 admits INIT0 or MANUALCAM6 pause owners.
  `PauseGame` 0x82647910 pushes FRONTEND3, not MENUSYS2. Existing shared
  `SelectedPauseManager` handles push/pull; stack owner admission is retained.

## Integration

The selected session updates its menu immediately after hardware polling and
both cache drains. An open menu, or the input that closes it, returns the new
Frontend outcome before terrain, Flow, PlayerUI, command enqueue, Actor,
physics, scoring, animation, camera, or world advancement. Nothing is queued
from paused input. The completed world draw buffers remain retained.

The application keeps polling at its ordinary 60Hz frontend cadence while
paused, bounded to the existing four polls per redraw. UI animation receives
wall time through MarkerUi.update_pause independently of simulation completion.
Open/close transitions reset timer debt; paused redraws keep only a fractional
input slot. Resume restores the retained simulation timer rate, without any
catch-up physics from paused wall time. Completed frame counters stay frozen.

Confidence: high for the recovered input expressions, repeats, pause owner and
simulation gate; host scheduling is an explicit adaptation of separate native
timer/frontend execution. Full FE state stacks/NIS gates are outside this
selected offline freeroam runtime. Destination actions remain intentionally
unconnected. No keyboard shortcuts or second XInput polling loop added.

Validation: pause_native independent static review found no concrete defect in
the input/gating/scheduling path. Rustfmt parse-only check of eight modified Rust
files completed successfully (no rewrites).
No Cargo, rustc, game launch or runtime replay performed.
