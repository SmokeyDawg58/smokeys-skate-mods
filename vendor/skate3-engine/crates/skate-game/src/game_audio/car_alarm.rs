//! Retail's car alarm trigger (spec `audio-specs/world-traffic-audio.md` "Car alarm trigger"): a
//! vehicle in `StayingParked` ([`VehicleParked`]) whose collision callback sees a contact longer
//! than `min_impact` sets its alarm off ([`VehicleAlarm`] → horn state 6 for the rule's time), and
//! every further such contact restarts the time (recomp `sub_82C3C150`: alarm flag `+3424` bit
//! 0x10 on, alarm timer `+3716` and parked timer `+3712` to 0; StopAlarming `sub_82C3A4D0` ends it
//! after 8 s). Any collider counts; a moving car never alarms. Inputs: [`VehicleImpact`] from
//! engine systems (none yet: the engine has no traffic) and mods (`sdk.world_audio.event(key,
//! 'impact', {speed=...})`). Tuning: [`CarAlarmRule`] (setup export, then a mod's or the engine's
//! override). Inert without impacts.
use bevy::prelude::*;

use crate::world_audio::*;

pub(crate) fn register(app: &mut App) {
    app.init_resource::<CarAlarmRule>()
        .add_message::<VehicleImpact>()
        .add_message::<VehicleAlarmStarted>()
        .add_systems(
            Update,
            (load_tuning, react)
                .chain()
                .before(super::world_bridge::WorldAudioPublish)
                .after(crate::app::FrameSet::Animation),
        );
}

/// The setup export's numbers, whenever the library (re)loads.
fn load_tuning(library: Option<Res<super::Library>>, mut rule: ResMut<CarAlarmRule>) {
    let Some(library) = library else { return };
    if !library.is_changed() {
        return;
    }
    let setup = library.world_tuning().vehicle_alarm();
    if rule.setup != setup {
        rule.setup = setup;
    }
}

/// The rule: impacts on parked cars → alarms.
fn react(
    rule: Res<CarAlarmRule>,
    bridge: Res<super::world_bridge::Bridge>,
    mut impacts: MessageReader<VehicleImpact>,
    parked: Query<Has<VehicleParked>, With<TrafficAudio>>,
    mut alarms: MessageWriter<VehicleAlarm>,
    mut started: MessageWriter<VehicleAlarmStarted>,
) {
    if impacts.is_empty() {
        return;
    }
    let tuning = rule.tuning();
    let mut done: Vec<Entity> = Vec::new();
    for hit in impacts.read() {
        let Ok(is_parked) = parked.get(hit.vehicle) else {
            continue;
        };
        if !tuning.sets_off(is_parked, hit.impact) {
            continue;
        }
        // Several contacts in one frame restart the alarm once.
        if done.contains(&hit.vehicle) {
            continue;
        }
        done.push(hit.vehicle);
        let restart = bridge.alarm_left(hit.vehicle).is_some();
        alarms.write(VehicleAlarm {
            vehicle: hit.vehicle,
        });
        started.write(VehicleAlarmStarted {
            vehicle: hit.vehicle,
            by: hit.by,
            restart,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_audio::world_bridge;
    use crate::game_audio::world_sources::{WorldHeld, WorldOwners};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<WorldOwners>()
            .init_resource::<crate::game_audio::npc_skaters::NpcSkaters>()
            .init_resource::<WorldHeld>();
        app.init_resource::<LivingWorldAudio>()
            .init_resource::<WorldAudioStats>()
            .init_resource::<world_bridge::Bridge>()
            .init_resource::<crate::game_audio::mod_world::OwnWorldOwners>()
            .init_resource::<CarAlarmRule>();
        app.add_message::<PedSpeechEvent>()
            .add_message::<VehicleHorn>()
            .add_message::<VehicleAlarm>()
            .add_message::<PedTazerEvent>()
            .add_message::<PedBodyFallEvent>()
            .add_message::<NpcSkaterReactionEvent>();
        app.add_message::<VehicleImpact>()
            .add_message::<VehicleAlarmStarted>();
        app.add_systems(Update, (react, world_bridge::publish).chain());
        app.world_mut().spawn((
            crate::game_audio::GameAudioListener,
            Transform::default(),
            GlobalTransform::default(),
        ));
        app
    }

    fn car(app: &mut App, parked: bool) -> Entity {
        let t = Transform::from_xyz(0.0, 0.0, 5.0);
        let mut e =
            app.world_mut()
                .spawn((TrafficAudio::new("c04_taxi01"), t, GlobalTransform::from(t)));
        if parked {
            e.insert(VehicleParked);
        }
        e.id()
    }

    fn horn(app: &App, car: Entity) -> i32 {
        app.world().resource::<WorldOwners>().vehicles[&car.to_bits()].horn
    }

    fn started(app: &App) -> Vec<VehicleAlarmStarted> {
        let messages = app.world().resource::<Messages<VehicleAlarmStarted>>();
        messages.iter_current_update_messages().copied().collect()
    }

    /// Step the app by `dt` of game time (manual time strategy; the first update after start has no delta).
    fn step(app: &mut App, dt: f32) {
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(dt),
        ));
        app.update();
    }

    #[test]
    fn retail_conditions() {
        let t = AlarmTuning::default();
        assert_eq!((t.min_impact, t.seconds), (0.1, 8.0));
        assert!(t.sets_off(true, Vec3::new(0.0, 0.0, 0.11)));
        assert!(
            !t.sets_off(true, Vec3::new(0.0, 0.0, 0.1)),
            "retail's test is a strict >"
        );
        assert!(!t.sets_off(true, Vec3::ZERO));
        assert!(
            !t.sets_off(false, Vec3::new(5.0, 0.0, 0.0)),
            "a car that is not parked never alarms"
        );
        assert!(!t.sets_off(true, Vec3::new(f32::NAN, 0.0, 0.0)));
        assert!(
            !AlarmTuning {
                enabled: false,
                ..t
            }
            .sets_off(true, Vec3::X * 5.0)
        );
        // The length counts, not one axis.
        assert!(t.sets_off(true, Vec3::new(0.06, 0.06, 0.06)));
    }

    #[test]
    fn hold_is_whole_console_frames_past_the_time() {
        let dt = skate_audio::mixmap::cadence::CONSOLE_DT;
        let t = AlarmTuning::default();
        assert!(
            (t.hold_seconds() - 241.0 * dt).abs() < 1e-5,
            "8 s = 241 console frames: {}",
            t.hold_seconds()
        );
        assert!((AlarmTuning { seconds: 0.5, ..t }.hold_seconds() - 16.0 * dt).abs() < 1e-5);
        assert!((AlarmTuning { seconds: 0.0, ..t }.hold_seconds() - dt).abs() < 1e-6);
    }

    #[test]
    fn an_impact_on_a_parked_car_sets_its_alarm_off_and_contacts_restart_it() {
        let mut app = app();
        let parked = car(&mut app, true);
        let moving = car(&mut app, false);
        step(&mut app, 0.0);
        assert_eq!(horn(&app, parked), 0);
        // A touch below the threshold, and a hit on the moving car: nothing.
        app.world_mut()
            .write_message(VehicleImpact::speed(parked, ImpactSource::Player, 0.05));
        app.world_mut()
            .write_message(VehicleImpact::speed(moving, ImpactSource::Player, 6.0));
        step(&mut app, 1.0 / 60.0);
        assert_eq!((horn(&app, parked), horn(&app, moving)), (0, 0));
        assert!(started(&app).is_empty());
        // The player walks into the parked taxi (session 161156: on foot, ~3 m/s): the alarm, the same frame.
        app.world_mut()
            .write_message(VehicleImpact::speed(parked, ImpactSource::Player, 3.3));
        app.world_mut()
            .write_message(VehicleImpact::speed(parked, ImpactSource::Player, 3.4));
        step(&mut app, 1.0 / 60.0);
        assert_eq!(horn(&app, parked), 6);
        assert_eq!(
            started(&app),
            vec![VehicleAlarmStarted {
                vehicle: parked,
                by: ImpactSource::Player,
                restart: false
            }],
            "two contacts in a frame start it once"
        );
        // 6 s later another contact restarts the time: still sounding 7.9 s after it, off by 8.1 s.
        for _ in 0..360 {
            step(&mut app, 1.0 / 60.0);
        }
        assert_eq!(horn(&app, parked), 6);
        app.world_mut()
            .write_message(VehicleImpact::speed(parked, ImpactSource::Object, 1.0));
        step(&mut app, 1.0 / 60.0);
        assert_eq!(
            started(&app),
            vec![VehicleAlarmStarted {
                vehicle: parked,
                by: ImpactSource::Object,
                restart: true
            }]
        );
        for _ in 0..474 {
            step(&mut app, 1.0 / 60.0);
        }
        assert_eq!(horn(&app, parked), 6, "7.9 s after the last contact");
        for _ in 0..12 {
            step(&mut app, 1.0 / 60.0);
        }
        assert_eq!(horn(&app, parked), 0, "8.1 s after the last contact");
    }

    /// The same alarm length at 30, 60 and 144 fps (console cadence, frame-independent).
    #[test]
    fn alarm_length_does_not_depend_on_the_frame_rate() {
        for fps in [30.0f32, 60.0, 144.0] {
            let mut app = app();
            let c = car(&mut app, true);
            step(&mut app, 0.0);
            app.world_mut()
                .write_message(VehicleImpact::speed(c, ImpactSource::Player, 2.0));
            step(&mut app, 1.0 / fps);
            let mut on = 0.0f32;
            for _ in 0..(fps as usize * 10) {
                if horn(&app, c) == 6 {
                    on += 1.0 / fps;
                }
                step(&mut app, 1.0 / fps);
            }
            assert!((on - 8.033).abs() <= 1.0 / fps + 1e-3, "{fps} fps: {on} s");
        }
    }

    #[test]
    fn tuning_overrides_and_disable() {
        let mut app = app();
        let c = car(&mut app, true);
        step(&mut app, 0.0);
        app.world_mut().resource_mut::<CarAlarmRule>().overrides = Some(AlarmTuning {
            enabled: false,
            ..Default::default()
        });
        app.world_mut()
            .write_message(VehicleImpact::speed(c, ImpactSource::Player, 9.0));
        step(&mut app, 1.0 / 60.0);
        assert_eq!(horn(&app, c), 0, "disabled");
        app.world_mut().resource_mut::<CarAlarmRule>().overrides = Some(AlarmTuning {
            enabled: true,
            min_impact: 2.0,
            seconds: 1.0,
        });
        app.world_mut()
            .write_message(VehicleImpact::speed(c, ImpactSource::Player, 1.5));
        step(&mut app, 1.0 / 60.0);
        assert_eq!(horn(&app, c), 0, "below the mod's threshold");
        app.world_mut()
            .write_message(VehicleImpact::speed(c, ImpactSource::Player, 2.5));
        step(&mut app, 1.0 / 60.0);
        assert_eq!(horn(&app, c), 6);
        for _ in 0..66 {
            step(&mut app, 1.0 / 60.0);
        }
        assert_eq!(horn(&app, c), 0, "the mod's 1 s");
        // The override cleared: the setup's (here the retail constants) again.
        app.world_mut().resource_mut::<CarAlarmRule>().overrides = None;
        assert_eq!(
            app.world().resource::<CarAlarmRule>().tuning(),
            AlarmTuning::default()
        );
    }
}
