use super::*;
use std::sync::Arc;
#[test]
fn actual_row_alias_preserves_signed_selection_and_resets_only_selected_controller() {
    let owner = PlayersBackendSettings::construct_fields();
    let same = Arc::clone(&owner);
    for player in 0..9 {
        assert_eq!(owner.try_controller_index(player).unwrap(), 0);
    }
    owner.try_set_controller_index(8, -1).unwrap();
    owner.try_set_controller_index(3, 19).unwrap();
    assert_eq!(same.try_controller_index(8).unwrap(), -1);
    assert_eq!(same.try_controller_index(3).unwrap(), 19);
    let loader = PlayerLoaderSlot::new(3);
    let selected = BackendControllerSelection::try_capture(&same, &loader).unwrap();
    owner.try_set_controller_index(3, 2).unwrap();
    assert_eq!(selected.device_index(), 19);
    owner.try_set_controller_index(3, 19).unwrap();
    owner.try_reset_player_settings(8).unwrap();
    assert_eq!(same.try_controller_index(8).unwrap(), 0);
    assert_eq!(same.try_controller_index(3).unwrap(), 19);
    assert_eq!(
        owner.try_set_controller_index(9, 0),
        Err(PlayersSettingsError::InvalidPlayer(9))
    );
    assert_eq!(
        owner.try_controller_index(u32::MAX),
        Err(PlayersSettingsError::InvalidPlayer(u32::MAX))
    );
}
