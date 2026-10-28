# Main menu: Exit to Desktop and the Downloadable Content notice (2026-10-17)

Two PC additions to the main menu (`crates/skate2-core/src/ui/main_menu.rs`). Neither exists in
the original game. Addresses are `sk82_na_m.xex` VAs (IDA, read-only).

## Exit to Desktop

- An eighth row, `#Exit to Desktop` (literal text), below Extras. `mainmenu.apt` (4860d06f)
  places `Menu_Picker_Component` from `source/controls/menu_picker` at (230.15, 16), scale 1.25.
  The picker (character 54) authors eight row slots, `dummy0` to `dummy7`, 38.75 apart; the
  original menu fills seven. The new row takes `dummy7`, about y 355 on the 1280x720 stage.
- Up and Down wrap through all eight rows.
- A plays the menu's A sound and returns `Selection::ExitToDesktop`. The session marks the menu
  as leaving and covers the frontend fade. Once the fade has covered the screen and no career
  save is being written (`exit_to_desktop_ready`), the window client's `exit_requested` ends
  the event loop, as the window's close button does.

## Downloadable Content

- Original: `FrontEndState_MainMenu::OnMenuSelectGame` 0x82681338, row 4, calls
  `SetState(25, -1)`, the Xbox LIVE Marketplace, only when
  `XenonSystemHudMonitor::IsCurrentUserSignedIn(1)`. Otherwise the row does nothing.
- Port: A opens sk8popup (asset 25) with the title `ID_MAIN_MENU_DOWNLOADABLE_CONTENT`, the
  message "DLC linking coming soon." and the one option `ID_COMMON_OK` (`DLC_NOTICE`).
- It behaves as `FrontEndState_Popup` does. `OnEnter` 0x826583d0 plays the core popup sound and
  enables no dimmer. The popup is not cancellable, and its one option neither moves nor sounds.
  A closes it (`OnMenuSelect` 0x82658598).

## Validation

- Unit tests in `main_menu.rs`: the eight-row wrap and the Exit selection; the notice's open and
  close input and sounds.
- rustfmt parse of the changed files. Not compiled or run when written.
