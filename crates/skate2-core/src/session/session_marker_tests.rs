use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GROUND, PHYSICS_CATEGORY_OFFBOARD, PHYSICS_CATEGORY_WIPEOUT,
    PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_OFFBOARD_PUSHING, PHYSICS_STATE_PHYSICS_GROUND,
    PHYSICS_STATE_SKITCHING, PHYSICS_STATE_WIPEOUT_GROUND,
};
use super::*;

const GROUND: LocalSkaterOutput = LocalSkaterOutput {
    category: PHYSICS_CATEGORY_GROUND,
    state: PHYSICS_STATE_PHYSICS_GROUND,
    wiping_out: false,
    can_leave_wipeout: false,
    wheels_in_contact: 4,
    surface: 0,
};

/// Scripted game-layer answers; the network and security-setting reads fail
/// the test because the offline single-player paths never reach them.
struct Rules {
    fe_in_game: bool,
    game_mode: u32,
    local: Option<LocalSkaterOutput>,
    main_activity: Option<bool>,
    chase: bool,
    hom: bool,
    message: bool,
    signing_up: bool,
    area_key: u64,
    position_ok: bool,
    messages: Vec<&'static str>,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            fe_in_game: true,
            game_mode: GM_SINGLE_PLAYER,
            local: Some(GROUND),
            main_activity: None,
            chase: false,
            hom: true,
            message: true,
            signing_up: false,
            area_key: 0,
            position_ok: true,
            messages: Vec::new(),
        }
    }
}

impl SessionMarkerRules for Rules {
    type Error = &'static str;
    fn fe_in_game(&mut self) -> Result<bool, Self::Error> {
        Ok(self.fe_in_game)
    }
    fn network_game(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn in_create_a_spot(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn disable_preconditions(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn security_service_running(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn game_mode(&mut self) -> Result<u32, Self::Error> {
        Ok(self.game_mode)
    }
    fn security_setting(&mut self) -> Result<bool, Self::Error> {
        Err("securitySetting read")
    }
    fn local_present(&mut self) -> Result<bool, Self::Error> {
        Ok(self.local.is_some())
    }
    fn local_output(&mut self) -> Result<LocalSkaterOutput, Self::Error> {
        self.local.ok_or("PhysOut read without a local Actor")
    }
    fn local_respawn_position_ok(&mut self) -> Result<bool, Self::Error> {
        Err("network respawn check")
    }
    fn main_activity_allows(&mut self, _use_marker: bool) -> Result<Option<bool>, Self::Error> {
        Ok(self.main_activity)
    }
    fn no_overrides(&mut self, _use_marker: bool) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn chase_is_on(&mut self) -> Result<bool, Self::Error> {
        Ok(self.chase)
    }
    fn hom_allows_use(&mut self) -> Result<bool, Self::Error> {
        Ok(self.hom)
    }
    fn message_setting(&mut self, name: &'static str) -> Result<bool, Self::Error> {
        self.messages.push(name);
        Ok(self.message)
    }
    fn signing_up(&mut self) -> Result<bool, Self::Error> {
        Ok(self.signing_up)
    }
    fn security_area_key(&mut self, _position: [f32; 2]) -> Result<u64, Self::Error> {
        Ok(self.area_key)
    }
    fn position_ok(&mut self, _marker: &SkeletonRootMatrix) -> Result<bool, Self::Error> {
        Ok(self.position_ok)
    }
}

fn transform(x: f32, z: f32) -> SkeletonRootMatrix {
    SkeletonRootMatrix::known([
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, 2.0, z, 1.0,
    ])
}

/// A marker set at the origin with permission from one update.
fn valid_marker(rules: &mut Rules) -> SessionMarker {
    let mut marker = SessionMarker::construct();
    marker.try_update(rules).unwrap();
    assert!(
        marker
            .try_set(rules, &transform(0.0, 0.0), 0, false, true)
            .unwrap()
    );
    marker
}

#[test]
fn constructor_matches_native_fields() {
    let marker = SessionMarker::construct();
    assert!(!marker.is_set_allowed() && !marker.is_use_allowed());
    assert_eq!(marker.get(), None);
    assert!(marker.on_board && !marker.in_no_skate_zone);
    assert_eq!(marker.foot_forward, 0);
}

#[test]
fn grounded_single_player_sets_then_may_use_the_valid_marker() {
    let mut rules = Rules::default();
    let mut marker = SessionMarker::construct();
    marker.try_update(&mut rules).unwrap();
    assert!(marker.is_set_allowed());
    assert!(!marker.is_use_allowed());
    assert!(
        marker
            .try_set(&mut rules, &transform(3.0, 4.0), 1, false, false)
            .unwrap()
    );
    assert_eq!(marker.get(), Some((transform(3.0, 4.0), 1, false)));
    marker.try_update(&mut rules).unwrap();
    assert!(marker.is_set_allowed() && marker.is_use_allowed());
    assert_eq!(
        rules.messages,
        [
            "Set Session Marker Visible",
            "Set Session Marker Visible",
            "Use Session Marker Visible"
        ]
    );
}

#[test]
fn skating_rules_gate_setting() {
    for (local, allowed) in [
        (
            LocalSkaterOutput {
                wheels_in_contact: 1,
                ..GROUND
            },
            false,
        ),
        (
            LocalSkaterOutput {
                state: PHYSICS_STATE_SKITCHING,
                ..GROUND
            },
            false,
        ),
        (
            LocalSkaterOutput {
                category: PHYSICS_CATEGORY_OFFBOARD,
                state: PHYSICS_STATE_OFFBOARD_PUSHING,
                ..GROUND
            },
            false,
        ),
        (
            LocalSkaterOutput {
                category: PHYSICS_CATEGORY_OFFBOARD,
                state: PHYSICS_STATE_OFFBOARD,
                wheels_in_contact: 0,
                ..GROUND
            },
            true,
        ),
        (
            LocalSkaterOutput {
                category: PHYSICS_CATEGORY_WIPEOUT,
                state: PHYSICS_STATE_WIPEOUT_GROUND,
                ..GROUND
            },
            false,
        ),
    ] {
        let mut rules = Rules {
            local: Some(local),
            ..Rules::default()
        };
        let mut marker = SessionMarker::construct();
        marker.try_update(&mut rules).unwrap();
        assert_eq!(marker.is_set_allowed(), allowed, "{local:?}");
    }
}

#[test]
fn stairs_forbid_setting_but_not_use() {
    let mut rules = Rules::default();
    let mut marker = valid_marker(&mut rules);
    rules.local = Some(LocalSkaterOutput {
        surface: 8,
        ..GROUND
    });
    marker.try_update(&mut rules).unwrap();
    assert!(!marker.is_set_allowed());
    assert!(marker.is_use_allowed());
}

#[test]
fn use_rules_block_wipeouts_that_cannot_leave_skitching_and_offboard_pushing() {
    for (local, allowed) in [
        (
            LocalSkaterOutput {
                wiping_out: true,
                ..GROUND
            },
            false,
        ),
        (
            LocalSkaterOutput {
                wiping_out: true,
                can_leave_wipeout: true,
                ..GROUND
            },
            true,
        ),
        (
            LocalSkaterOutput {
                state: PHYSICS_STATE_SKITCHING,
                ..GROUND
            },
            false,
        ),
        (
            LocalSkaterOutput {
                category: PHYSICS_CATEGORY_OFFBOARD,
                state: PHYSICS_STATE_OFFBOARD_PUSHING,
                ..GROUND
            },
            false,
        ),
    ] {
        let mut rules = Rules::default();
        let mut marker = valid_marker(&mut rules);
        rules.local = Some(local);
        marker.try_update(&mut rules).unwrap();
        assert_eq!(marker.is_use_allowed(), allowed, "{local:?}");
    }
}

#[test]
fn leaving_the_game_clears_both_permissions_but_keeps_the_marker() {
    let mut rules = Rules::default();
    let mut marker = valid_marker(&mut rules);
    marker.try_update(&mut rules).unwrap();
    assert!(marker.is_use_allowed());
    rules.fe_in_game = false;
    marker.try_update(&mut rules).unwrap();
    assert!(!marker.is_set_allowed() && !marker.is_use_allowed());
    assert!(marker.get().is_some());
}

#[test]
fn restricted_areas_and_bad_positions_reject_unforced_sets() {
    for key in ["NoSkateZone_Area", "HighAlert_Area"] {
        let mut rules = Rules {
            area_key: string_hash64(key),
            ..Rules::default()
        };
        let mut marker = SessionMarker::construct();
        marker.try_update(&mut rules).unwrap();
        assert!(marker.is_set_allowed());
        assert!(
            !marker
                .try_set(&mut rules, &transform(1.0, 1.0), 0, false, true)
                .unwrap()
        );
        assert_eq!(marker.get(), None);
        assert!(
            marker
                .try_set(&mut rules, &transform(1.0, 1.0), 0, true, true)
                .unwrap()
        );
    }
    let mut rules = Rules {
        position_ok: false,
        ..Rules::default()
    };
    let mut marker = SessionMarker::construct();
    marker.try_update(&mut rules).unwrap();
    assert!(
        !marker
            .try_set(&mut rules, &transform(1.0, 1.0), 0, false, true)
            .unwrap()
    );
}

#[test]
fn unforced_set_needs_permission() {
    let mut rules = Rules::default();
    let mut marker = SessionMarker::construct();
    assert!(
        !marker
            .try_set(&mut rules, &transform(1.0, 1.0), 0, false, true)
            .unwrap()
    );
    assert_eq!(marker.get(), None);
}

#[test]
fn progression_and_sign_up_gate_single_player_only() {
    let mut rules = Rules {
        message: false,
        ..Rules::default()
    };
    let mut marker = SessionMarker::construct();
    marker.try_update(&mut rules).unwrap();
    assert!(!marker.is_set_allowed());
    let mut rules = Rules {
        signing_up: true,
        ..Rules::default()
    };
    marker.try_update(&mut rules).unwrap();
    assert!(!marker.is_set_allowed());
    let mut rules = Rules {
        game_mode: GM_PROFILE_FREE_SKATE,
        message: false,
        signing_up: true,
        ..Rules::default()
    };
    marker.try_update(&mut rules).unwrap();
    assert!(marker.is_set_allowed());
    assert!(rules.messages.is_empty());
}

#[test]
fn challenge_rules_gate_use_and_main_activity_gates_both() {
    let mut rules = Rules::default();
    let mut marker = valid_marker(&mut rules);
    rules.chase = true;
    marker.try_update(&mut rules).unwrap();
    assert!(marker.is_set_allowed() && !marker.is_use_allowed());
    rules.chase = false;
    rules.hom = false;
    marker.try_update(&mut rules).unwrap();
    assert!(marker.is_set_allowed() && !marker.is_use_allowed());
    rules.hom = true;
    rules.main_activity = Some(false);
    marker.try_update(&mut rules).unwrap();
    assert!(!marker.is_set_allowed() && !marker.is_use_allowed());
}
