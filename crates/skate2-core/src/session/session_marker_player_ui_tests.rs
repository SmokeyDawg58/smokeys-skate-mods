use crate::physics_state_ids::{PHYSICS_CATEGORY_GROUND, PHYSICS_STATE_PHYSICS_GROUND};
use super::*;
use crate::session_marker::LocalSkaterOutput;

const MIN_TIME: f32 = 0.2;
const MAX_TIME: f32 = 1.0;
const MIN_DISTANCE: f32 = 100.0;
const MAX_DISTANCE: f32 = 1000.0;
const TELEPORT_DISTANCE: f32 = 2.0;

/// Scripted PlayerUI owners over an offline single-player local skater.
struct Source {
    actor: bool,
    set: bool,
    hold_use: bool,
    offboard: bool,
    foot: FootForward,
    respawn: SkeletonRootMatrix,
    distance: f32,
    safe: bool,
    debug: bool,
    security: bool,
    sent_set_markers: u32,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            actor: true,
            set: false,
            hold_use: false,
            offboard: false,
            foot: 0,
            respawn: SkeletonRootMatrix::known([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 7.0, 1.0, 9.0, 1.0,
            ]),
            distance: 550.0,
            safe: true,
            debug: false,
            security: false,
            sent_set_markers: 0,
        }
    }
}

impl SessionMarkerRules for Source {
    type Error = &'static str;
    fn fe_in_game(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
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
        Ok(self.security)
    }
    fn game_mode(&mut self) -> Result<u32, Self::Error> {
        Ok(1)
    }
    fn security_setting(&mut self) -> Result<bool, Self::Error> {
        Err("securitySetting read")
    }
    fn local_present(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn local_output(&mut self) -> Result<LocalSkaterOutput, Self::Error> {
        Ok(LocalSkaterOutput {
            category: PHYSICS_CATEGORY_GROUND,
            state: PHYSICS_STATE_PHYSICS_GROUND,
            wiping_out: false,
            can_leave_wipeout: false,
            wheels_in_contact: 4,
            surface: 0,
        })
    }
    fn local_respawn_position_ok(&mut self) -> Result<bool, Self::Error> {
        Err("network respawn check")
    }
    fn main_activity_allows(&mut self, _use_marker: bool) -> Result<Option<bool>, Self::Error> {
        Ok(None)
    }
    fn no_overrides(&mut self, _use_marker: bool) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn chase_is_on(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn hom_allows_use(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn message_setting(&mut self, _name: &'static str) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn signing_up(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn security_area_key(&mut self, _position: [f32; 2]) -> Result<u64, Self::Error> {
        Ok(0)
    }
    fn position_ok(&mut self, _marker: &SkeletonRootMatrix) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

impl PlayerUiSessionMarkerSource for Source {
    fn player_actor_present(&mut self) -> Result<bool, Self::Error> {
        Ok(self.actor)
    }
    fn input_state(&mut self, input: SessionMarkerInput) -> Result<bool, Self::Error> {
        Ok(match input {
            SessionMarkerInput::Set => self.set,
            SessionMarkerInput::Use => self.hold_use,
        })
    }
    fn is_offboard(&mut self) -> Result<bool, Self::Error> {
        Ok(self.offboard)
    }
    fn foot_forward_with_fakie(&mut self) -> Result<FootForward, Self::Error> {
        Ok(self.foot)
    }
    fn respawn_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        Ok(self.respawn)
    }
    fn distance_to(&mut self, _point: SkeletonRootVector) -> Result<f32, Self::Error> {
        Ok(self.distance)
    }
    fn teleport_requires_load_screen(
        &mut self,
        _transform: &SkeletonRootMatrix,
    ) -> Result<bool, Self::Error> {
        Err("load screen query")
    }
    fn safe_to_use(&mut self) -> Result<bool, Self::Error> {
        Ok(self.safe)
    }
    fn debug_session_teleport(&mut self) -> Result<bool, Self::Error> {
        Ok(self.debug)
    }
    fn cvar(&mut self, cvar: SessionMarkerCvar) -> Result<f32, Self::Error> {
        Ok(match cvar {
            SessionMarkerCvar::MinDistanceForTeleport => TELEPORT_DISTANCE,
            SessionMarkerCvar::MinTimeHeld => MIN_TIME,
            SessionMarkerCvar::MaxTimeHeld => MAX_TIME,
            SessionMarkerCvar::MinDistanceForMinTime => MIN_DISTANCE,
            SessionMarkerCvar::MaxDistanceForMaxTime => MAX_DISTANCE,
        })
    }
    fn send_set_marker(&mut self) -> Result<(), Self::Error> {
        self.sent_set_markers += 1;
        Ok(())
    }
    fn dismissal_popup(&mut self) -> Self::Error {
        "popup"
    }
}

fn ramp(distance: f32) -> f32 {
    hold_time(distance, MIN_TIME, MIN_DISTANCE, MAX_TIME, MAX_DISTANCE)
}

/// A marker set through the stage's own Set input, with use permitted.
fn usable_marker(source: &mut Source) -> SessionMarker {
    let mut marker = SessionMarker::construct();
    marker.can_set = true;
    source.set = true;
    PlayerUiSessionMarker::default()
        .try_update(&mut marker, source)
        .unwrap();
    source.set = false;
    marker.can_use = true;
    marker
}

#[test]
fn hold_time_is_a_clamped_fused_ramp() {
    assert_eq!(ramp(50.0), MIN_TIME);
    assert_eq!(ramp(MIN_DISTANCE), MIN_TIME);
    assert_eq!(ramp(MAX_DISTANCE), MAX_TIME);
    assert_eq!(ramp(5000.0), MAX_TIME);
    assert_eq!(ramp(f32::NAN), MIN_TIME);
    let slope = (MAX_TIME - MIN_TIME) / (MAX_DISTANCE - MIN_DISTANCE);
    assert_eq!(
        ramp(550.0).to_bits(),
        550.0_f32
            .mul_add(slope, (-MIN_DISTANCE).mul_add(slope, MIN_TIME))
            .to_bits()
    );
}

#[test]
fn set_input_stores_the_respawn_transform_stance_and_board() {
    let mut source = Source {
        set: true,
        offboard: true,
        foot: 1,
        ..Source::default()
    };
    let mut marker = SessionMarker::construct();
    marker.can_set = true;
    let mut ui = PlayerUiSessionMarker::default();
    assert_eq!(ui.try_update(&mut marker, &mut source).unwrap(), None);
    assert_eq!(marker.get(), Some((source.respawn, 1, false)));
    assert_eq!((source.sent_set_markers, ui.successful_sets), (1, 1));
}

#[test]
fn set_input_needs_permission_and_the_player_actor() {
    let mut source = Source {
        set: true,
        ..Source::default()
    };
    let mut marker = SessionMarker::construct();
    let mut ui = PlayerUiSessionMarker::default();
    ui.try_update(&mut marker, &mut source).unwrap();
    assert_eq!(marker.get(), None);
    marker.can_set = true;
    source.actor = false;
    ui.try_update(&mut marker, &mut source).unwrap();
    assert_eq!(marker.get(), None);
    assert_eq!(source.sent_set_markers, 0);
}

#[test]
fn held_use_teleports_once_the_clock_passes_the_ramp_then_latches() {
    let mut source = Source::default();
    let mut marker = usable_marker(&mut source);
    source.hold_use = true;
    let hold = ramp(source.distance);
    let mut ui = PlayerUiSessionMarker::default();
    let mut held = 0.0_f32;
    loop {
        held += UPDATE_TIME;
        let request = ui.try_update(&mut marker, &mut source).unwrap();
        let noise = ui.take_out_of_boundary_amount();
        assert_eq!(noise, Some((held / hold).min(1.0)));
        assert_eq!(ui.take_out_of_boundary_amount(), None);
        assert_eq!(ui.updates_held.to_bits(), held.to_bits());
        if held > hold {
            assert_eq!(
                request,
                Some(TeleportToPosition {
                    transform: source.respawn,
                    foot_forward: 0,
                    on_board: true,
                    session_marker: true,
                })
            );
            break;
        }
        assert_eq!(request, None);
    }
    assert!(ui.use_session_marker);
    assert_eq!(ui.extra_frames_of_snow, 2);
    assert_eq!(ui.try_update(&mut marker, &mut source).unwrap(), None);
    assert_eq!(ui.updates_held.to_bits(), held.to_bits());
    assert_eq!(ui.extra_frames_of_snow, 1);
    assert_eq!(ui.take_out_of_boundary_amount(), Some(1.0));
    assert_eq!(ui.try_update(&mut marker, &mut source).unwrap(), None);
    assert_eq!(ui.take_out_of_boundary_amount(), Some(1.0));
    assert_eq!(ui.extra_frames_of_snow, 0);
    assert_eq!(ui.try_update(&mut marker, &mut source).unwrap(), None);
    assert_eq!(ui.take_out_of_boundary_amount(), None);
}

#[test]
fn releasing_use_resets_the_latch_and_the_clock() {
    let mut source = Source {
        debug: true,
        ..Source::default()
    };
    let mut marker = usable_marker(&mut source);
    source.hold_use = true;
    let mut ui = PlayerUiSessionMarker::default();
    assert!(ui.try_update(&mut marker, &mut source).unwrap().is_some());
    source.hold_use = false;
    ui.try_update(&mut marker, &mut source).unwrap();
    assert!(!ui.use_session_marker && !ui.told_front_end_cant_use);
    assert_eq!(ui.updates_held.to_bits(), 0);
}

#[test]
fn nearby_marker_only_advances_the_clock() {
    let mut source = Source {
        distance: TELEPORT_DISTANCE,
        ..Source::default()
    };
    let mut marker = usable_marker(&mut source);
    source.hold_use = true;
    let mut ui = PlayerUiSessionMarker::default();
    for _ in 0..120 {
        assert_eq!(ui.try_update(&mut marker, &mut source).unwrap(), None);
    }
    assert!(ui.updates_held > MAX_TIME);
    assert!(!ui.use_session_marker);
}

#[test]
fn debug_teleport_skips_the_hold_but_not_the_safety_check() {
    let mut source = Source {
        debug: true,
        safe: false,
        ..Source::default()
    };
    let mut marker = usable_marker(&mut source);
    marker.can_use = false;
    source.hold_use = true;
    let mut ui = PlayerUiSessionMarker::default();
    assert_eq!(ui.try_update(&mut marker, &mut source).unwrap(), None);
    source.safe = true;
    assert!(ui.try_update(&mut marker, &mut source).unwrap().is_some());
}

#[test]
fn refused_use_latches_the_front_end_notice() {
    let mut source = Source::default();
    let mut marker = usable_marker(&mut source);
    marker.can_use = false;
    source.hold_use = true;
    let mut ui = PlayerUiSessionMarker::default();
    assert_eq!(ui.try_update(&mut marker, &mut source).unwrap(), None);
    assert!(ui.told_front_end_cant_use);
    assert_eq!(ui.updates_held.to_bits(), 0);
}

#[test]
fn security_popup_path_is_an_explicit_boundary() {
    let mut source = Source {
        security: true,
        ..Source::default()
    };
    let mut marker = usable_marker(&mut source);
    source.hold_use = true;
    let mut ui = PlayerUiSessionMarker::default();
    assert_eq!(
        ui.try_update(&mut marker, &mut source),
        Err("load screen query")
    );
}
