use super::*;
use crate::play_animation::EMPTY_FAST_STRING_36;

fn name36(text: &str) -> FastString36 {
    FastString36::encode(text).unwrap()
}
fn name30(text: &str) -> FastString30 {
    FastString30::encode(text).unwrap()
}
fn names() -> ToggleBoardNames {
    ToggleBoardNames {
        retrieve: name36("OB_RetrieveBoard"),
        retrieving: name36("OB_RetrievingBoard"),
        dropping: name36("OB_DroppingBoard"),
        throw: name36("OFB_THROW_0"),
        drop: name36("OFB_THROW_90"),
        retrieve_front: [
            name36("OFB_RETRIEVE_HIGH_FRONT_INTO"),
            name36("OFB_RETRIEVE_HIGH_FRONT_CYC"),
            name36("OFB_RETRIEVE_HIGH_FRONT_OUT"),
        ],
        retrieve_back: [
            name36("OFB_RETRIEVE_HIGH_BACK_INTO"),
            name36("OFB_RETRIEVE_HIGH_BACK_CYC"),
            name36("OFB_RETRIEVE_HIGH_BACK_OUT"),
        ],
        yaw: name30("yaw"),
        pitch: name30("pitch"),
    }
}

#[derive(Debug, PartialEq)]
enum Call {
    Motion(FastString30),
    Settable(FastString30, f32),
    Transition(FastString36, f32, ChannelParams, ChannelStartFlags),
    Sequence(FastString36, ChannelParams, ChannelStartFlags),
    End(f32),
}

struct Mock {
    holding: bool,
    yaw: f32,
    pitch: f32,
    mirrored: bool,
    channel: bool,
    left: f32,
    elapsed: f32,
    calls: Vec<Call>,
}
impl Mock {
    fn new() -> Self {
        Self {
            holding: false,
            yaw: 0.0,
            pitch: 0.0,
            mirrored: false,
            channel: true,
            left: 1.0,
            elapsed: 0.0,
            calls: Vec::new(),
        }
    }
}
impl ToggleBoardSource for Mock {
    type Error = &'static str;
    fn holding(&mut self) -> Result<bool, Self::Error> {
        Ok(self.holding)
    }
    fn yaw_to_free_board(&mut self) -> Result<f32, Self::Error> {
        Ok(self.yaw)
    }
    fn pitch_to_free_board(&mut self) -> Result<f32, Self::Error> {
        Ok(self.pitch)
    }
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        Ok(self.mirrored)
    }
    fn narrow(&mut self, name: FastString36) -> Result<FastString30, Self::Error> {
        Ok(FastString30::encode(&name.decode().unwrap()).unwrap())
    }
    fn motion_attribute(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error> {
        assert_eq!(value.to_bits(), 1.0_f32.to_bits());
        self.calls.push(Call::Motion(name));
        Ok(())
    }
    fn settable(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error> {
        self.calls.push(Call::Settable(name, value));
        Ok(())
    }
    fn has_channel(&mut self) -> Result<bool, Self::Error> {
        Ok(self.channel)
    }
    fn time_left(&mut self) -> Result<f32, Self::Error> {
        Ok(self.left)
    }
    fn time_elapsed(&mut self) -> Result<f32, Self::Error> {
        Ok(self.elapsed)
    }
    fn transition_to(
        &mut self,
        animation: FastString36,
        length: f32,
        params: ChannelParams,
        flags: ChannelStartFlags,
    ) -> Result<(), Self::Error> {
        self.calls
            .push(Call::Transition(animation, length, params, flags));
        Ok(())
    }
    fn sequence_to(
        &mut self,
        animation: FastString36,
        params: ChannelParams,
        flags: ChannelStartFlags,
    ) -> Result<(), Self::Error> {
        self.calls.push(Call::Sequence(animation, params, flags));
        Ok(())
    }
    fn end_channel(&mut self, length: f32) -> Result<(), Self::Error> {
        self.calls.push(Call::End(length));
        Ok(())
    }
    fn cvar(&mut self, cvar: ToggleBoardCvar) -> Result<f32, Self::Error> {
        Ok(match cvar {
            ToggleBoardCvar::YawChange => 5.0,
            ToggleBoardCvar::BlendFrames => 15.0,
            ToggleBoardCvar::BlendOutFrames | ToggleBoardCvar::RetrievePlayOutFrames => 10.0,
            ToggleBoardCvar::DropPlayOutFrames => 30.0,
        })
    }
    fn unwritten(&mut self, field: &'static str) -> Self::Error {
        field
    }
    fn assertion(&mut self, site: &'static str) -> Self::Error {
        site
    }
}

fn begun(names: &ToggleBoardNames) -> ToggleBoardInstance {
    let mut data = ToggleBoardInstance::after_allocation();
    data.begin(names);
    data
}

#[test]
fn command_gates_check_presence_in_native_lazy_order() {
    let mut seen = Vec::new();
    assert_eq!(
        select_command::<()>(
            false,
            |name| {
                seen.push(name.to_vec());
                Ok(false)
            },
            || panic!("absent command must not read state"),
            || panic!("absent command must not read holding"),
        )
        .unwrap(),
        None
    );
    assert_eq!(seen, vec![DROP.to_vec(), THROW.to_vec(), RETRIEVE.to_vec()]);
    let mut holding_reads = 0;
    assert_eq!(
        select_command::<()>(
            false,
            |name| Ok(name == DROP || name == THROW),
            || Ok(true),
            || {
                holding_reads += 1;
                Ok(false)
            },
        )
        .unwrap(),
        None
    );
    assert_eq!(holding_reads, 2);
    assert_eq!(
        select_command::<()>(false, |name| Ok(name == THROW), || Ok(true), || Ok(true)).unwrap(),
        Some(BoardCommand::Throw)
    );
    assert_eq!(
        select_command::<()>(false, |_| Ok(true), || Ok(true), || Ok(false)).unwrap(),
        Some(BoardCommand::Retrieve)
    );
    assert_eq!(
        select_command::<()>(
            true,
            |_| panic!("grabbing bypasses intents"),
            || panic!("grabbing bypasses state"),
            || panic!("grabbing bypasses holding"),
        )
        .unwrap(),
        None
    );
}

#[test]
fn intent_names_are_the_native_fast_string_values() {
    assert_eq!(DROP, b"OB_DropBoard");
    assert_eq!(THROW, b"OB_ThrowBoard");
    assert_eq!(RETRIEVE, b"OB_RetrieveBoard");
    assert_eq!(CHANNEL, "RetrieveBoard");
}

#[test]
fn throw_starts_the_selected_clip_and_expires_by_play_out_frames() {
    let names = names();
    let mut data = begun(&names);
    let mut mock = Mock::new();
    apply_command(&mut data, BoardCommand::Throw);
    dispatch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.state, Some(6));
    let blend = 15.0 * UPDATE_TIME;
    let expected = params(blend, false);
    assert_eq!(
        mock.calls,
        vec![
            Call::Motion(name30("OB_DroppingBoard")),
            Call::Settable(name30("OB_DroppingBoard"), 1.0),
            Call::Transition(
                names.throw,
                blend,
                expected,
                ChannelStartFlags {
                    resurrect_if_blending_out: true,
                    create_if_missing: true,
                    use_attributes: true,
                },
            ),
        ]
    );
    // Drop selects OFB_THROW_90 through the same state.
    let mut drop = begun(&names);
    let mut mock = Mock::new();
    apply_command(&mut drop, BoardCommand::Drop);
    dispatch(&mut mock, &mut drop, &names).unwrap();
    assert!(matches!(&mock.calls[2], Call::Transition(name, ..) if *name == names.drop));
    // Still inside the play-out window: nothing ends.
    let mut mock = Mock::new();
    mock.elapsed = 30.0 * UPDATE_TIME;
    dispatch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.state, Some(6));
    assert_eq!(mock.calls.len(), 2);
    // Strictly past it ends with the blend-out frames and returns to idle.
    let mut mock = Mock::new();
    mock.elapsed = 1.0;
    dispatch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.state, Some(0));
    assert_eq!(mock.calls[2], Call::End(10.0 * UPDATE_TIME));
}

#[test]
fn unordered_time_left_ends_the_drop_clip() {
    let names = names();
    let mut data = begun(&names);
    data.state = Some(6);
    let mut mock = Mock::new();
    mock.left = f32::NAN;
    dispatch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.state, Some(0));
    assert_eq!(mock.calls[2], Call::End(10.0 * UPDATE_TIME));
}

#[test]
fn retrieval_runs_into_cycle_and_out_with_native_flags() {
    let names = names();
    let mut data = begun(&names);
    apply_command(&mut data, BoardCommand::Retrieve);
    let mut mock = Mock::new();
    dispatch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.state, Some(2));
    let blend = 15.0 * UPDATE_TIME;
    assert_eq!(
        mock.calls[2],
        Call::Transition(
            names.retrieve_front[0],
            blend,
            params(blend, false),
            ChannelStartFlags {
                resurrect_if_blending_out: true,
                create_if_missing: true,
                use_attributes: true,
            },
        )
    );
    assert_eq!(mock.calls[3], Call::Settable(name30("yaw"), 0.0));
    assert_eq!(mock.calls[4], Call::Settable(name30("pitch"), 0.0));
    // More than the blend left: the into clip keeps playing.
    let mut mock = Mock::new();
    dispatch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.state, Some(2));
    assert_eq!(mock.calls.len(), 4);
    // At the blend boundary the cycle is sequenced without creation.
    let mut mock = Mock::new();
    mock.left = blend;
    dispatch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.state, Some(3));
    let cycle_flags = ChannelStartFlags {
        resurrect_if_blending_out: true,
        create_if_missing: false,
        use_attributes: true,
    };
    assert_eq!(
        mock.calls[2..5],
        [
            Call::Sequence(names.retrieve_front[1], params(blend, true), cycle_flags),
            Call::Motion(name30("OB_RetrieveBoard")),
            Call::Settable(name30("OB_RetrieveBoard"), 1.0),
        ]
    );
    // Holding the board in the cycle transitions to the out clip.
    let mut mock = Mock::new();
    mock.holding = true;
    dispatch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.state, Some(4));
    assert_eq!(
        mock.calls[2],
        Call::Transition(
            names.retrieve_front[2],
            blend,
            params(blend, true),
            cycle_flags
        )
    );
    // The out clip ends after its play-out frames; state4 skips yaw/pitch.
    let mut mock = Mock::new();
    mock.elapsed = 1.0;
    dispatch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.state, Some(0));
    assert_eq!(mock.calls[2], Call::End(10.0 * UPDATE_TIME));
    assert_eq!(mock.calls.len(), 5);
}

#[test]
fn missing_channel_resets_retrieval_to_idle() {
    let names = names();
    let mut data = begun(&names);
    data.state = Some(3);
    let mut mock = Mock::new();
    mock.channel = false;
    mock.holding = true;
    dispatch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.state, Some(0));
    assert!(
        !mock
            .calls
            .iter()
            .any(|call| matches!(call, Call::Transition(..) | Call::Sequence(..)))
    );
}

#[test]
fn yaw_folds_behind_then_smooths_and_selects_back_clips() {
    let names = names();
    let mut data = begun(&names);
    let mut mock = Mock::new();
    // -(2.5 rad) * -57.29578 = +143.2: no fold, first pass is unsmoothed.
    mock.yaw = -2.5;
    mock.pitch = 0.5;
    update_yaw_pitch(&mut mock, &mut data, &names).unwrap();
    let first = -2.5 * NEGATIVE_DEGREES;
    assert_eq!(data.yaw, Some(first));
    assert_eq!(data.signed_yaw, Some(first));
    assert_eq!(data.pitch, Some(0.5 * DEGREES));
    assert_eq!(data.yaw_cached, Some(true));
    assert_eq!(data.retrieve, names.retrieve_back);
    // Smoothing clamps the change to kMaxRetrieveBoardYawChange degrees.
    mock.yaw = 0.0;
    update_yaw_pitch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.yaw, Some(-5.0 + first));
    // Behind the skater with smoothing, the previous sign picks +180.
    mock.yaw = 3.0;
    update_yaw_pitch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.yaw, Some(5.0 + (-5.0 + first)));
    // Mirroring negates both the input and the corrected output.
    let mut mirrored = begun(&names);
    let mut mock = Mock::new();
    mock.mirrored = true;
    mock.yaw = 1.0;
    update_yaw_pitch(&mut mock, &mut mirrored, &names).unwrap();
    assert_eq!(mirrored.yaw, Some(NEGATIVE_DEGREES));
    assert_eq!(mirrored.signed_yaw, Some(-NEGATIVE_DEGREES));
    assert_eq!(mirrored.retrieve, names.retrieve_front);
}

#[test]
fn unsmoothed_behind_fold_uses_minus_135_split() {
    let names = names();
    let mut data = begun(&names);
    let mut mock = Mock::new();
    // +2.0 rad => -114.6 degrees: folded to -90.
    mock.yaw = 2.0;
    update_yaw_pitch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.yaw, Some(MINUS_QUARTER));
    let mut data = begun(&names);
    // +3.0 rad => -171.9 degrees: folded to +180.
    mock.yaw = 3.0;
    update_yaw_pitch(&mut mock, &mut data, &names).unwrap();
    assert_eq!(data.yaw, Some(HALF_TURN));
    assert_eq!(data.retrieve, names.retrieve_back);
    assert_ne!(names.retrieve_back[0], EMPTY_FAST_STRING_36);
}
