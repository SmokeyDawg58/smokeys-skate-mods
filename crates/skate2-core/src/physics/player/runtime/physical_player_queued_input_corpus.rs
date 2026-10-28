//! Original mounted input/tick connection. Hardware and remap replies are the
//! existing explicit test fixtures; these are not production boot collaborators.
use super::*;
use std::sync::Mutex;
#[path = "../../../input/recognition/juice_input_boot_corpus.rs"]
mod juice_boot;
use crate::{
    actor_construction::selected_startup::SelectedPlayerStartupOwners,
    actor_construction::selected_tick::SelectedQueuedTickInput,
    input_pad::{
        InputPad,
        device::tests::{Platform, Remap},
        device_manager::{
            InputDeviceManager,
            selected_runtime::{
                SelectedDeviceInputEvents, SelectedDeviceInputRuntime, SelectedInputGroup,
            },
        },
    },
    input_pad_cache::InputPadCache,
    native_skater_random::{NativeRandomGate, SimulationCommandRandom},
    players_backend_settings::controller::BackendControllerSelection,
    reckoning_numeric::HostReckoningNumeric,
    selected_sim_command_queue::{
        SelectedCommandSampleRequest, SelectedSimCommandQueue, SimCommandCounters,
    },
    simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo,
    skeleton_collision_numeric::HostSkeletonCollisionNumeric,
};
pub(super) use juice_boot::QueuedRemapPolicy;
use juice_boot::{CorpusRemap, CorpusRemapHandle};
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_device_queue_reaches_same_mounted_player_tick() {
    with_original_inputs(ground_startup::check_queued_ticks);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT; combined roster, camera and queued frame"]
fn original_selected_queue_reaches_same_native_roster_camera() {
    with_original_inputs(ground_startup::check_queued_actual_roster_ticks);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT; authored raw-pad Kickflip"]
fn original_scripted_raw_pad_kickflip_reaches_same_queued_physical_tick() {
    with_original_inputs(ground_startup::check_queued_scripted_ticks);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT; selected raw-pad Y dismount boundary"]
fn original_scripted_raw_pad_y_reaches_selected_dismount_boundary() {
    with_original_inputs(ground_startup::check_queued_scripted_y_ticks);
}

pub(super) struct QueuedInputCorpus<'a> {
    focus: Arc<Mutex<crate::selected_sim_command_queue::focus::SelectedPlayerFocus>>,
    pause: Arc<crate::selected_pause_manager::SelectedPauseManager>,
    input: SelectedDeviceInputRuntime<CorpusRemapHandle, HostSkeletonCollisionNumeric, CorpusRemap>,
    platform: Platform,
    selected: BackendControllerSelection<'a>,
    queue: SelectedSimCommandQueue,
    random: SimulationCommandRandom,
}
impl<'a> QueuedInputCorpus<'a> {
    pub(super) fn new(players: &'a SelectedPlayerStartupOwners, policy: QueuedRemapPolicy) -> Self {
        let mut devices = InputDeviceManager::new();
        devices.initialize().unwrap();
        let mut platform = Platform::new();
        for device in devices.devices_mut().unwrap() {
            device
                .reset_actuators(&mut platform, &players.cvars, &mut HostReckoningNumeric)
                .unwrap();
        }
        // Explicitly preserve the old fixture or select the actual disabled Juice branch.
        let (handle, remap) = juice_boot::supplied_remap(players, policy);
        devices.set_external_remap(Some(&handle));
        let mut sim = InputPadCache::new();
        sim.try_initialize().unwrap();
        let mut pres = InputPadCache::new();
        pres.try_initialize().unwrap();
        let map = Arc::clone(players.map.as_ref().expect("same original startup map"));
        let input = SelectedDeviceInputRuntime::from_owners(
            devices,
            sim,
            pres,
            Arc::clone(&map),
            HostSkeletonCollisionNumeric,
            remap,
        );
        assert!(Arc::ptr_eq(input.map(), &map));
        Self {
            focus: Arc::clone(&players.focus),
            pause: Arc::clone(&players.pause),
            input,
            platform,
            selected: BackendControllerSelection::try_capture(
                &players.services.settings,
                &players.loader,
            )
            .unwrap(),
            queue: SelectedSimCommandQueue::construct_offline_command_fields(
                SimCommandCounters::from_static_initializer(),
            ),
            random: SimulationCommandRandom::construct_field(
                NativeRandomGate::from_static_initializer(),
            ),
        }
    }
    /// Inverse of the source right-stick radial conversion, used only to choose
    /// a raw hardware candidate. The mounted map check below rejects a wrong
    /// sign or a point outside authored tolerance before the frame can pass.
    pub(super) fn set_scripted_point(&mut self, point: [f32; 2]) {
        assert_eq!(
            self.selected.device_index(),
            0,
            "script targets selected backend controller 0"
        );
        self.platform.scripted_controller_zero =
            Some(crate::input_pad::device::tests::scripted_right_point(point));
    }

    /// Original input.cfg maps FaceY (control 89) to pad button 15. The
    /// Xbox conversion copies hardware bit 15 into that same pad slot.
    pub(super) fn set_scripted_y(&mut self, held: bool) {
        assert_eq!(self.selected.device_index(), 0);
        self.platform.scripted_controller_zero = Some(crate::input_pad::XboxPadSample {
            buttons: if held { 1 << 15 } else { 0 },
            left_trigger: 0,
            right_trigger: 0,
            left_x: 0,
            left_y: 0,
            right_x: 0,
            right_y: 0,
        });
    }

    pub(super) fn assert_scripted_y_mapped(&self, held: bool) {
        let sample = self.input.raw_current_cache_line(0).unwrap();
        assert_eq!(sample.count, 24);
        let mut pad = InputPad::new();
        pad.try_update(&sample.values, sample.count).unwrap();
        assert_eq!(
            self.input.map().try_value(89, &mut pad).unwrap() != 0.0,
            held
        );
        assert_eq!(self.input.map().try_value(84, &mut pad).unwrap(), 0.0);
    }

    /// Actual hardware-converted current cache plus mounted input.cfg map.
    /// This does not advance or replace the player's recognizer history.
    pub(super) fn assert_scripted_mapped_point(&self, point: [f32; 2], tolerance: f32) {
        let sample = self.input.raw_current_cache_line(0).unwrap();
        assert_eq!(
            sample.count, 24,
            "scripted controller-0 pad must be connected"
        );
        let mut pad = InputPad::new();
        pad.try_update(&sample.values, 24).unwrap();
        let mapped = [
            self.input.map().try_value(77, &mut pad).unwrap(),
            -self.input.map().try_value(78, &mut pad).unwrap(),
        ];
        let error = (mapped[1] - point[1]).mul_add(
            mapped[1] - point[1],
            (mapped[0] - point[0]) * (mapped[0] - point[0]),
        );
        assert!(
            error <= tolerance,
            "raw-pad/map stage: authored {point:?}, mapped {mapped:?}, squared error {error}, tolerance {tolerance}"
        );
    }

    pub(super) fn poll_and_enqueue(&mut self, info: &PhysicalGameInfo) {
        type Input = SelectedDeviceInputRuntime<
            CorpusRemapHandle,
            HostSkeletonCollisionNumeric,
            CorpusRemap,
        >;
        let calls_before = self.platform.calls.len();
        let samples_before = self.platform.polled_samples.len();
        self.input.poll_hardware(&mut self.platform).unwrap();
        if let Some(expected) = self.platform.scripted_controller_zero {
            assert_eq!(
                self.platform.calls[calls_before..]
                    .iter()
                    .filter(|call| **call == (0, 0))
                    .count(),
                1,
                "raw-pad poll stage: selected controller 0 must be polled once"
            );
            let controller_zero = self.platform.polled_samples[samples_before..]
                .iter()
                .filter(|(index, _)| *index == 0)
                .collect::<Vec<_>>();
            assert_eq!(
                controller_zero.len(),
                1,
                "raw-pad poll stage: one controller-0 packet"
            );
            let actual = controller_zero[0].1;
            assert_eq!(
                (
                    actual.buttons,
                    actual.left_trigger,
                    actual.right_trigger,
                    actual.left_x,
                    actual.left_y,
                    actual.right_x,
                    actual.right_y,
                ),
                (
                    expected.buttons,
                    expected.left_trigger,
                    expected.right_trigger,
                    expected.left_x,
                    expected.left_y,
                    expected.right_x,
                    expected.right_y,
                ),
                "raw-pad poll stage: platform returned a different packet"
            );
        }
        for group in [
            SelectedInputGroup::Simulation,
            SelectedInputGroup::Presentation,
        ] {
            <Input as SelectedDeviceInputEvents<Platform>>::cache_input(&mut self.input, group)
                .unwrap();
        }
        assert!(
            <Input as SelectedDeviceInputEvents<Platform>>::last_input(
                &mut self.input,
                SelectedInputGroup::Simulation
            )
            .unwrap()
        );
        // Selected corpus admission supplies can-simulate/focus. This executes
        // only the native focus prefix; optional UI collaborators remain outside
        // this component fixture's admitted scope.
        assert!(
            self.focus
                .lock()
                .unwrap()
                .try_update_prefix(true, true, &mut self.queue)
                .unwrap()
        );
        self.input
            .try_sample_and_queue_player(
                SelectedCommandSampleRequest {
                    selection: &self.selected,
                    game_info: info,
                    has_focus: true,
                },
                &mut self.pause,
                &mut self.queue,
                &mut HostReckoningNumeric,
            )
            .unwrap();
        assert!(self.queue.try_generate_sync_command().unwrap());
    }
    pub(super) fn tick_input(&mut self) -> SelectedQueuedTickInput<'_> {
        SelectedQueuedTickInput {
            queue: &mut self.queue,
            random: &mut self.random,
        }
    }
    pub(super) fn assert_rng_index(&self, index: u32) {
        assert_eq!(self.random.random.index, index);
    }
}
