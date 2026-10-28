//! Original input.cfg through registered gameplay receiver and native codec.
//! Raw channel samples below are explicit diagnostics, not `PlayerUI` or a host
//! polling/offline frame scheduler. No group-pad history is substituted.
use crate::{
    controller_input::{ControllerInputActivity, RawControllerInput},
    game_input_command::{GameInputWrite, try_pack_player_input},
    game_input_manager::{
        EncodedGameInputCommand, GameInputCommandManager, GameInputDispatch, GameInputSyncLevel,
    },
    game_input_player::{GameInputPlayer, PlayerInputData},
    input_map::SelectedInputMap,
    input_pad_cache::PublishedInputPadSample,
    reckoning_numeric::HostReckoningNumeric,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
struct DiagnosticSyncLevel;
impl GameInputSyncLevel for DiagnosticSyncLevel {
    type Error = Infallible;
    fn sync_level(&mut self) -> Result<i32, Self::Error> {
        Ok(2)
    }
}
fn send(manager: &mut GameInputCommandManager, receiver: u32, data: &PlayerInputData) -> Vec<u8> {
    let mut storage = [0xaa; 9];
    let count = {
        let mut write = GameInputWrite::new(&mut storage);
        try_pack_player_input(data, &mut write, &mut HostReckoningNumeric).unwrap();
        assert!(!write.has_error());
        write.position()
    };
    let payload = &storage[..count];
    assert_eq!(
        manager
            .try_do_command(
                &EncodedGameInputCommand {
                    receiver_id: receiver,
                    kind: 1,
                    payload
                },
                true,
                &mut DiagnosticSyncLevel
            )
            .unwrap(),
        GameInputDispatch::Delivered
    );
    payload.to_vec()
}
#[expect(
    clippy::too_many_lines,
    reason = "one retained original-map receiver through ordered command and focus transitions"
)]
pub(crate) fn check(input_cfg: &str) {
    let map = Arc::new(SelectedInputMap::parse(input_cfg).unwrap());
    let player = Arc::new(Mutex::new(GameInputPlayer::new(0, map)));
    let mut manager = GameInputCommandManager::new();
    let receiver = manager.try_construct_player(&player).unwrap();
    assert_eq!(receiver.id(), 10);
    let pad_pointer = std::ptr::from_ref(player.lock().unwrap().pad().unwrap());
    let mut sample = PlayerInputData {
        sample: PublishedInputPadSample {
            count: 24,
            values: [0.0; 24],
        },
        allow_sensor: true,
        has_focus: true,
    };
    // A source focusedzero packet unlocks all four independent face gates.
    assert_eq!(send(&mut manager, receiver.id(), &sample), [0]);
    sample.sample.values[..16].fill(1.0);
    sample.sample.values[10] = 0.5;
    sample.sample.values[11] = 0.25;
    sample.sample.values[16] = 0.5;
    sample.sample.values[19] = 0.5;
    sample.sample.values[20] = 1.0;
    sample.sample.values[23] = 1.0;
    // Frozen wire oracle: mask then LX/LY/RX/RY/LT/RT/B1/B2, truncation.
    assert_eq!(
        send(&mut manager, receiver.id(), &sample),
        [255, 63, 193, 127, 129, 127, 63, 255, 63]
    );
    let lx = 63.0 * f32::from_bits(0x3c01_0204);
    let lt = 127.0 * f32::from_bits(0x3b80_8081);
    let rt = 63.0 * f32::from_bits(0x3b80_8081);
    {
        let mut retained = player.lock().unwrap();
        assert_eq!(pad_pointer, std::ptr::from_ref(retained.pad().unwrap()));
        let expected = [
            lx, -lx, 1.0, 1.0, -1.0, 1.0, lt, rt, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            0.0, 0.0, 0.0, 0.0,
        ];
        for (id, value) in (74..=95).zip(expected) {
            assert_eq!(
                retained.try_value(id).unwrap().to_bits(),
                value.to_bits(),
                "original gameplay map ID{id}"
            );
        }
        // Host diagnostic previous record; not a fabricated native derived owner.
        let mut raw = RawControllerInput::default();
        raw.try_update(
            &mut *retained,
            &RawControllerInput::default(),
            false,
            ControllerInputActivity::default(),
        )
        .unwrap();
        assert_eq!(
            [
                raw.left_stick_x,
                raw.left_stick_y,
                raw.right_stick_x,
                raw.right_stick_y,
                raw.left_trigger,
                raw.right_trigger
            ]
            .map(f32::to_bits),
            [lx, -lx, 1.0, -1.0, lt, rt].map(f32::to_bits)
        );
        assert!(raw.left_stick_in && raw.right_stick_in);
        assert!(raw.left_bumper && !raw.right_bumper);
        assert!(raw.dpad_up && raw.dpad_down && raw.dpad_left && raw.dpad_right);
        assert!(raw.face_x && raw.face_y && raw.face_a && raw.face_b);
    }
    // Unfocused payload has zero bytes; native decode sets only Button13=1.
    sample.has_focus = false;
    assert!(send(&mut manager, receiver.id(), &sample).is_empty());
    {
        let mut retained = player.lock().unwrap();
        assert!(!retained.has_focus().unwrap());
        assert_eq!(retained.pad().unwrap().num_buttons(), 24);
        for id in 74..=95 {
            assert_eq!(
                retained.try_value(id).unwrap().to_bits(),
                if id == 91 { 1.0_f32.to_bits() } else { 0 },
                "unfocused native map ID{id}"
            );
        }
    }
    // GainFocus is routed through the SAME manager; it rearms bounce without
    // resetting pad history. Held face samples remain gated until a zero sample.
    manager
        .try_do_command(
            &EncodedGameInputCommand {
                receiver_id: receiver.id(),
                kind: 2,
                payload: &[],
            },
            false,
            &mut DiagnosticSyncLevel,
        )
        .unwrap();
    sample.has_focus = true;
    send(&mut manager, receiver.id(), &sample);
    {
        let mut retained = player.lock().unwrap();
        for id in 88..=91 {
            assert_eq!(retained.try_value(id).unwrap().to_bits(), 0);
        }
        assert_eq!(retained.try_value(74).unwrap().to_bits(), lx.to_bits());
    }
    assert_ne!(manager.crc(), 0);
    eprintln!(
        "original input.cfg: registered gameplay owner, sparse codec quantization, all22 selected mappings, raw controls, focus/bounce PASS (diagnostic samples; no PlayerUI/frame schedule)"
    );
}
