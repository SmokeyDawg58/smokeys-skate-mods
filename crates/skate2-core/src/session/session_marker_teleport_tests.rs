use super::*;

fn transform() -> SkeletonRootMatrix {
    SkeletonRootMatrix::known([
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ])
}

fn request(transform: SkeletonRootMatrix) -> TeleportToPosition {
    TeleportToPosition {
        transform,
        foot_forward: 1,
        on_board: true,
        session_marker: true,
    }
}

#[test]
fn pack_writes_the_header_then_only_xyz_of_each_row() {
    let bytes = PlayerTeleportData {
        where_: transform(),
        player_index: -1,
        foot_forward: 1,
        on_board: true,
        session_marker: false,
    }
    .pack();
    assert_eq!(bytes.len(), PACKED_LENGTH);
    assert_eq!(bytes[..10], [0xff, 0xff, 0xff, 0xff, 0, 0, 0, 1, 1, 0]);
    assert_eq!(bytes[10..14], 1.0_f32.to_bits().to_be_bytes());
    assert_eq!(bytes[22..26], 5.0_f32.to_bits().to_be_bytes());
    assert_eq!(bytes[54..58], 15.0_f32.to_bits().to_be_bytes());
}

#[test]
fn unpack_restores_rows_with_zero_w_and_rejects_other_lengths() {
    let data = PlayerTeleportData {
        where_: transform(),
        player_index: 0,
        foot_forward: 0,
        on_board: false,
        session_marker: true,
    };
    let unpacked = PlayerTeleportData::try_unpack(&data.pack()).unwrap();
    assert_eq!(
        unpacked.where_.known_lanes().unwrap(),
        [
            1.0, 2.0, 3.0, 0.0, 5.0, 6.0, 7.0, 0.0, 9.0, 10.0, 11.0, 0.0, 13.0, 14.0, 15.0, 0.0
        ]
    );
    assert_eq!(
        (
            unpacked.player_index,
            unpacked.foot_forward,
            unpacked.on_board,
            unpacked.session_marker
        ),
        (0, 0, false, true)
    );
    assert_eq!(
        PlayerTeleportData::try_unpack(&[0; 57]),
        Err(TeleportCommandError::PayloadLength(57))
    );
}

#[derive(Default)]
struct Services {
    actor: bool,
    calls: Vec<String>,
}

impl SkaterManagerTeleport for Services {
    type Error = String;
    fn has_actor(&mut self, player_index: i32) -> Result<bool, Self::Error> {
        self.calls.push(format!("find {player_index}"));
        Ok(self.actor)
    }
    fn adjust_spawn_position(
        &mut self,
        transform: &SkeletonRootMatrix,
    ) -> Result<SkeletonRootMatrix, Self::Error> {
        self.calls.push("adjust".to_owned());
        let mut adjusted = *transform;
        adjusted.set_column(3, SkeletonRootVector::known([13.0, 20.0, 15.0, 0.0]));
        Ok(adjusted)
    }
    fn spawn_foot_forward_stance(&mut self, foot_forward: FootForward) -> Result<(), Self::Error> {
        self.calls.push(format!("stance {foot_forward}"));
        Ok(())
    }
    fn teleport(
        &mut self,
        transform: &SkeletonRootMatrix,
        on_board: bool,
    ) -> Result<(), Self::Error> {
        self.calls.push(format!(
            "teleport {} {on_board}",
            transform.columns()[3].xyz[1]
        ));
        Ok(())
    }
    fn pin(&mut self, player_index: i32, pin: bool) -> Result<(), Self::Error> {
        self.calls.push(format!("pin {player_index} {pin}"));
        Ok(())
    }
    fn send_use_marker(&mut self, player_index: i32) -> Result<(), Self::Error> {
        self.calls.push(format!("use marker {player_index}"));
        Ok(())
    }
    fn command(&mut self, error: TeleportCommandError) -> Self::Error {
        format!("{error:?}")
    }
}

#[test]
fn on_command_adjusts_then_spawns_the_stance_then_teleports() {
    let payload = PlayerTeleportData {
        where_: transform(),
        player_index: 0,
        foot_forward: 1,
        on_board: false,
        session_marker: true,
    }
    .pack();
    let mut services = Services {
        actor: true,
        ..Services::default()
    };
    try_on_command(&mut services, TELEPORT_COMMAND, &payload).unwrap();
    assert_eq!(
        services.calls,
        ["find 0", "adjust", "stance 1", "teleport 20 false", "use marker 0"]
    );
}

#[test]
fn on_command_without_an_actor_stops_after_the_lookup() {
    let payload = PlayerTeleportData {
        where_: transform(),
        player_index: 3,
        foot_forward: 0,
        on_board: true,
        session_marker: true,
    }
    .pack();
    let mut services = Services::default();
    try_on_command(&mut services, TELEPORT_COMMAND, &payload).unwrap();
    assert_eq!(services.calls, ["find 3"]);
}

#[test]
fn on_command_other_types_are_the_native_assertion() {
    let mut services = Services::default();
    assert_eq!(
        try_on_command(&mut services, 8, &[]),
        Err("UnhandledCommand(8)".to_owned())
    );
    assert!(services.calls.is_empty());
}

#[derive(Default)]
struct Receiver {
    commands: Vec<(u32, u32, Vec<u8>)>,
    pulls: Vec<u32>,
}

impl PlayerUiTeleportReceiver for Receiver {
    type Error = ();
    fn do_command(&mut self, command: &EncodedGameInputCommand<'_>) -> Result<(), Self::Error> {
        self.commands
            .push((command.receiver_id, command.kind, command.payload.to_vec()));
        Ok(())
    }
    fn pull_pause(&mut self, state: u32) -> Result<(), Self::Error> {
        self.pulls.push(state);
        Ok(())
    }
    fn set_player(&mut self, player: i32) -> Result<FootForward, Self::Error> {
        self.commands.push((0, u32::MAX, player.to_be_bytes().to_vec()));
        Ok(1)
    }
}

#[test]
fn flow_then_game_sim_reach_the_proxy_one_stage_at_a_time() {
    let mut queues = SessionMarkerTeleportQueues::construct();
    queues.send_flow(request(transform()));
    let mut receiver = Receiver::default();
    queues.try_dispatch_game_sim(0, &mut receiver, None).unwrap();
    assert!(receiver.commands.is_empty());
    let mut focus = Vec::new();
    queues
        .try_pump_flow(|position| {
            focus.push(position.xyz);
            Ok::<(), ()>(())
        })
        .unwrap();
    assert_eq!(focus, [[13.0, 14.0, 15.0]]);
    queues.try_dispatch_game_sim(0, &mut receiver, None).unwrap();
    let expected = PlayerTeleportData {
        where_: transform(),
        player_index: 0,
        foot_forward: 1,
        on_board: true,
        session_marker: true,
    }
    .pack();
    assert_eq!(
        receiver.commands,
        [(SKATER_MANAGER_RECEIVER, TELEPORT_COMMAND, expected)]
    );
    assert_eq!(receiver.pulls, [PAUSE_STATE_MANUALCAM]);
}

#[test]
fn origin_translation_is_ignored_after_the_stance_is_stored() {
    let mut queues = SessionMarkerTeleportQueues::construct();
    let mut origin = transform();
    origin.set_column(3, SkeletonRootVector::known([0.0, 0.0, 0.0, 1.0]));
    queues.send_flow(request(origin));
    queues.try_pump_flow(|_| Ok::<(), ()>(())).unwrap();
    let mut receiver = Receiver::default();
    queues.try_dispatch_game_sim(0, &mut receiver, None).unwrap();
    assert!(receiver.commands.is_empty() && receiver.pulls.is_empty());
    assert_eq!(queues.player_ui_foot_forward, 1);
}

#[test]
fn set_session_marker_forces_the_ui_marker_only_when_there_is_one() {
    let mut queues = SessionMarkerTeleportQueues::construct();
    let mut receiver = Receiver::default();
    queues.send_set_session_marker(transform(), 0);
    queues.try_dispatch_game_sim(0, &mut receiver, None).unwrap();
    let mut marker = SessionMarker::construct();
    queues.send_set_session_marker(transform(), 0);
    queues
        .try_dispatch_game_sim(0, &mut receiver, Some(&mut marker))
        .unwrap();
    assert_eq!(marker.get(), Some((transform(), 0, true)));
    assert!(receiver.commands.is_empty() && receiver.pulls.is_empty());
}
