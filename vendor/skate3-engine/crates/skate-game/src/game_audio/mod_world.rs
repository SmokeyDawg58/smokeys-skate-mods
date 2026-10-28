//! Extra MixMap instances for published world objects (doc 16 "L3" / M3; world audio extension 3,
//! the default for a mod's cars and peds since extension 4: `sdk.world_audio.spawn(key, 'traffic'
//! | 'ped', …)` without `slots = 'shared'`; engine side the `world_audio::OwnAudioInstance`
//! component, which engine systems publishing the living world do not add).
//!
//! Retail gives the living world 4 traffic and 15 pedestrian MixMap instances (the more-audible
//! setting 8 / 24); every published car and ped competes for them, the nearest win. An object with
//! its own instance does not: it is played by a second world host ([`ModWorld`], the same code as
//! `world_sources`: retail's traffic and ped objects, posts, Splice steps, speech requests) on a
//! **private MixMap** built from the install's MixMap file with the Global slot and
//! [`TRAFFIC`] / [`PEDS`] instances of the Traffic and Pedestrian slots. Its Global inputs are
//! copied from the game's MixMap before each evaluation, so an object's words are what a retail
//! instance gives for the same position (the instances are the MixMap's own controllers).
//!
//! - The retail pools never see these objects (the bridge publishes them to [`OwnWorldOwners`]),
//!   so the map's cars and peds keep retail's instances; own objects within the list radii (40 m /
//!   50 m) all play, the nearest [`TRAFFIC`] / [`PEDS`].
//! - **A full private pool** (more than [`TRAFFIC`] own cars or [`PEDS`] own peds of all mods in
//!   reach): the nearest hold the instances (retail's own rule, `owners::Pool`), the others wait,
//!   silent, and take an instance as soon as one is among the nearest again (a nearer one moves
//!   away or goes). Never refused (only the ones in reach compete, a mod may publish 48) and never
//!   spilled into retail's pools (the map's objects keep all of theirs). Read back:
//!   `WorldAudioStats::own_waiting`, `read(key).waiting`, `info().own.waiting`; a warning is
//!   logged when waiting starts.
//! - Read back: `WorldAudioInstance { own: true, … }` (`read(key).own`).
//! - Lifecycle as the retail host: a map change (the epoch) resets it, a runtime restart builds the
//!   private MixMap again (the host's handles are forgotten with the old runtime); a hot swap
//!   keeps it. Its posts draw from the evaluator's random generator like every post (mod-only).
//! - Nothing runs while no object asks for its own instance: the game sounds exactly as before.
//! - NPC skaters stay in retail's Player slot (their host holds a whole skater's components and a
//!   grain bed per instance; open question in doc 16).
use std::sync::Arc;

use bevy::prelude::*;
use skate_audio::mixmap::{MixMap, MixMapFile, keys};

use super::world_sources::{WorldHeld, WorldHost, WorldOwners};
use super::{Library, Native};

/// Own Traffic / Pedestrian instances (the MixMap's group field allows 32 per slot).
pub(crate) const TRAFFIC: usize = 16;
pub(crate) const PEDS: usize = 16;

/// The published objects with their own instance (`world_audio::OwnAudioInstance`); the bridge
/// fills it instead of [`WorldOwners`].
#[derive(Resource, Default)]
pub(crate) struct OwnWorldOwners(pub(crate) WorldOwners);

/// The second world host and its private MixMap.
#[derive(Resource, Default)]
pub(crate) struct ModWorld {
    host: WorldHost,
    mix: Option<MixMap>,
    globals: Vec<u32>,
    /// The runtime it was built for (address, `AudioContent::runtime_generation`).
    built: Option<(usize, u64)>,
}

fn instances() -> [usize; 14] {
    let mut n = [0; 14];
    n[keys::slot::GLOBAL as usize] = 1;
    n[keys::slot::TRAFFIC as usize] = TRAFFIC;
    n[keys::slot::PEDESTRIAN as usize] = PEDS;
    n
}

impl ModWorld {
    /// Build for this runtime when needed; false without a MixMap.
    fn ensure(&mut self, library: &Library, native: &Native, runtime: u64) -> bool {
        let id = (Arc::as_ptr(&native.shared) as usize, runtime);
        if self.built == Some(id) {
            return self.mix.is_some();
        }
        // A new runtime: the old host's handles belong to the old one (forgotten, never released).
        self.built = Some(id);
        self.host = WorldHost::default();
        self.mix = None;
        self.globals.clear();
        let (Some(retail), Some(file)) = (&native.mixmap, &library.aems().mixmap) else {
            return false;
        };
        let Some(parsed) = library
            .read(file)
            .ok()
            .and_then(|b| MixMapFile::parse(&b).ok())
        else {
            return false;
        };
        self.globals = retail
            .controller_keys()
            .filter(|k| (k >> 16) & 0xFF == keys::slot::GLOBAL)
            .collect();
        self.mix = Some(MixMap::new(&parsed, &instances()));
        info!(
            "Game audio: private MixMap for own-instance world objects ({TRAFFIC} traffic, {PEDS} pedestrian instances)"
        );
        true
    }
}

/// The pass, after the game's MixMap ticked (`native::mixmap_tick`): the own objects' process on
/// the private MixMap, its evaluations (the Global inputs copied from the game's first), their
/// update. Returns at once while nothing asks for an own instance.
#[allow(clippy::too_many_arguments)]
pub(super) fn frame(
    native: Option<ResMut<Native>>,
    library: Option<Res<Library>>,
    owners: Res<OwnWorldOwners>,
    mut world: ResMut<ModWorld>,
    mut retail: ResMut<WorldHost>,
    mut held: ResMut<WorldHeld>,
    mut speech: ResMut<super::world_speech::WorldSpeech>,
    content: Res<super::AudioContent>,
    cues: Res<super::skate_events::Cues>,
    listener: Query<&GlobalTransform, With<super::GameAudioListener>>,
) {
    let owners = &owners.0;
    let idle = owners.vehicles.is_empty() && owners.peds.is_empty() && world.built.is_none();
    if idle {
        return;
    }
    let (Some(mut native), Some(library)) = (native, library) else {
        return;
    };
    let world = &mut *world;
    if !world.ensure(&library, &native, content.runtime_generation) {
        return;
    }
    let calls = native.pending.map_or(0, |p| p.calls);
    let camera = listener
        .single()
        .ok()
        .map(|t| (t.translation().to_array(), t.forward().as_vec3().to_array()));
    // The rules and the event rows follow the game's host.
    world.host.rules = retail.rules.clone();
    if retail.events.is_some() && world.host.events.is_none() {
        world.host.events = Some(Vec::new());
    } else if retail.events.is_none() {
        world.host.events = None;
    }
    let mut mix = world.mix.take();
    super::world_sources::pre_in(
        &mut world.host,
        owners,
        &mut native,
        mix.as_mut(),
        (TRAFFIC, PEDS),
        &library,
        camera,
        &cues.riding.audio,
        calls,
    );
    if let (Some(m), Some(game)) = (mix.as_mut(), native.mixmap.as_ref()) {
        if world.host.has_pass() {
            for &k in &world.globals {
                for id in 0..16 {
                    m.set_input(k, id, game.input(k, id));
                }
            }
            for _ in 0..calls {
                m.tick(skate_audio::mixmap::cadence::CONSOLE_DT);
            }
        }
    }
    super::world_sources::post_in(&mut world.host, owners, &mut native, mix.as_mut());
    world.mix = mix;
    speech.peds.append(&mut world.host.speech_requests);
    for (id, _) in world.host.held().1 {
        if let Some(p) = owners.peds.get(&id) {
            speech.ped_positions.insert(id, p.position);
        }
    }
    if let (Some(rows), Some(into)) = (world.host.events.as_mut(), retail.events.as_mut()) {
        into.append(rows);
    }
    let (traffic, peds) = world.host.held();
    if held.own_traffic != traffic || held.own_peds != peds {
        held.own_traffic = traffic;
        held.own_peds = peds;
    }
    // A full private pool: the nearest own objects play, the others in reach wait (read back as
    // `waiting`; logged when waiting starts, not every frame).
    let (cars, people) = &world.host.waiting;
    if held.own_waiting_traffic != *cars || held.own_waiting_peds != *people {
        let was = (held.own_waiting_traffic.len(), held.own_waiting_peds.len());
        if (cars.len() > was.0 && was.0 == 0) || (people.len() > was.1 && was.1 == 0) {
            warn!(
                "AUDIO_WORLD own instances full: {} car(s) / {} ped(s) in reach wait ({TRAFFIC} own traffic / {PEDS} own pedestrian instances; the nearest play, the next nearest takes a freed one)",
                cars.len(),
                people.len()
            );
        }
        held.own_waiting_traffic.clone_from(cars);
        held.own_waiting_peds.clone_from(people);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// L3 (data-gated): 6 cars near the listener with their own instance all hold one (retail's 4
    /// stay free for the map's cars), post the engine class and play; the game's host holds
    /// nothing; when they go the instances come back.
    #[test]
    #[ignore = "needs the private install data"]
    fn own_instance_cars_all_play_beside_retails_pool() {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"));
        let Ok(library) = Library::load(root) else {
            panic!("missing private data: no audio install")
        };
        let Ok(mut native) = Native::start(&library) else {
            panic!("missing private data: no AEMS install")
        };
        native.test_pass(1);
        let m = native.mixmap.as_mut().expect("a MixMap");
        for id in 1..=4 {
            m.set_input(keys::MASTER, id, 32767);
        }
        for id in [1, 2, 5] {
            m.set_input(keys::MUSIC, id, 32767);
        }
        m.set_input(keys::REVERB, 5, 32767);
        let mut w = World::new();
        w.insert_resource(native);
        w.insert_resource(library);
        w.init_resource::<OwnWorldOwners>();
        w.init_resource::<ModWorld>();
        w.init_resource::<WorldHost>();
        w.init_resource::<WorldHeld>();
        w.init_resource::<super::super::world_speech::WorldSpeech>();
        w.init_resource::<super::super::AudioContent>();
        w.init_resource::<super::super::skate_events::Cues>();
        w.spawn((super::super::GameAudioListener, GlobalTransform::default()));
        let record = w
            .resource::<Library>()
            .world_tuning()
            .engine("c04_taxi01")
            .unwrap_or_else(|| panic!("missing private data: no traffic engine records"));
        for i in 0..6u64 {
            let v = skate_audio::world::traffic::VehicleState {
                position: [3.0 * i as f32 - 8.0, 0.0, -6.0],
                direction: [1.0, 0.0, 0.0],
                speed: 8.0,
                velocity: [8.0, 0.0, 0.0],
                engine: record,
                ..Default::default()
            };
            w.resource_mut::<OwnWorldOwners>()
                .0
                .vehicles
                .insert(100 + i, v);
        }
        let posts = w.resource::<Native>().next_node();
        for _ in 0..3 {
            w.resource_mut::<Native>().test_pass(1);
            w.run_system_once(frame).unwrap();
        }
        let held = w.resource::<WorldHeld>();
        assert_eq!(
            held.own_traffic.len(),
            6,
            "all six hold an own instance: {:?}",
            held.own_traffic
        );
        assert!(held.traffic.is_empty(), "the game's pool is untouched");
        assert!(w.resource::<Native>().next_node() > posts, "they posted");
        w.resource_mut::<OwnWorldOwners>().0.vehicles.clear();
        w.resource_mut::<Native>().test_pass(1);
        w.run_system_once(frame).unwrap();
        assert!(w.resource::<WorldHeld>().own_traffic.is_empty(), "released");
    }

    /// Doc 16 M3 (data-gated): a full private pool. 18 own cars in reach: the 16 nearest hold the
    /// own instances, the 2 farthest wait (read back), retail's pool holds none (no spill), one
    /// beyond the list radius is not waiting; when a near one goes, the nearest waiting one takes
    /// its instance at the next pass, and with fewer than 16 in reach nobody waits.
    #[test]
    #[ignore = "needs the private install data"]
    fn a_full_private_pool_lets_the_nearest_play_and_the_rest_wait() {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"));
        let Ok(library) = Library::load(root) else {
            panic!("missing private data: no audio install")
        };
        let Ok(mut native) = Native::start(&library) else {
            panic!("missing private data: no AEMS install")
        };
        native.test_pass(1);
        let mut w = World::new();
        w.insert_resource(native);
        w.insert_resource(library);
        w.init_resource::<OwnWorldOwners>();
        w.init_resource::<ModWorld>();
        w.init_resource::<WorldHost>();
        w.init_resource::<WorldHeld>();
        w.init_resource::<super::super::world_speech::WorldSpeech>();
        w.init_resource::<super::super::AudioContent>();
        w.init_resource::<super::super::skate_events::Cues>();
        w.spawn((super::super::GameAudioListener, GlobalTransform::default()));
        let record = w
            .resource::<Library>()
            .world_tuning()
            .engine("c04_taxi01")
            .unwrap_or_else(|| panic!("missing private data: no traffic engine records"));
        // Car i at 2 + i m (horizontal): 0..15 nearest, 16 and 17 farther, 99 beyond 40 m.
        let car = |d: f32| skate_audio::world::traffic::VehicleState {
            position: [d, 0.0, 0.0],
            direction: [0.0, 0.0, 1.0],
            speed: 5.0,
            velocity: [0.0, 0.0, 5.0],
            engine: record,
            ..Default::default()
        };
        for i in 0..18u64 {
            w.resource_mut::<OwnWorldOwners>()
                .0
                .vehicles
                .insert(200 + i, car(2.0 + i as f32));
        }
        w.resource_mut::<OwnWorldOwners>()
            .0
            .vehicles
            .insert(299, car(45.0));
        let pass = |w: &mut World| {
            w.resource_mut::<Native>().test_pass(1);
            w.run_system_once(frame).unwrap();
        };
        for _ in 0..2 {
            pass(&mut w);
        }
        let held = w.resource::<WorldHeld>();
        let mut holders: Vec<u64> = held.own_traffic.iter().map(|h| h.0).collect();
        holders.sort_unstable();
        assert_eq!(
            holders,
            (200..216).collect::<Vec<_>>(),
            "the 16 nearest hold the own instances"
        );
        assert_eq!(
            held.own_waiting_traffic,
            vec![216, 217],
            "the 2 farther in reach wait; 299 is out of reach"
        );
        assert!(
            held.traffic.is_empty() && held.waiting_traffic.is_empty(),
            "no spill into retail's pool"
        );
        // A near one goes: the nearest waiting car takes its instance.
        w.resource_mut::<OwnWorldOwners>().0.vehicles.remove(&203);
        pass(&mut w);
        let held = w.resource::<WorldHeld>();
        assert!(
            held.own_traffic.iter().any(|h| h.0 == 216),
            "216 took the freed instance: {:?}",
            held.own_traffic
        );
        assert_eq!(held.own_waiting_traffic, vec![217]);
        w.resource_mut::<OwnWorldOwners>().0.vehicles.remove(&204);
        pass(&mut w);
        let held = w.resource::<WorldHeld>();
        assert_eq!(held.own_traffic.len(), 16);
        assert!(
            held.own_waiting_traffic.is_empty(),
            "16 in reach: nobody waits"
        );
    }
}
