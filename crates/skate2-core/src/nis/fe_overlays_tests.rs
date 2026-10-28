use super::*;

fn frames(fe: &mut FeNisOverlays, n: usize) -> Vec<ScreenCommand> {
    for _ in 0..n {
        fe.apt_frame();
    }
    fe.take_commands()
}

#[test]
fn the_tutorial_gesture_tip_registers_a_frame_after_it_opens_and_completes_the_preload() {
    let mut fe = FeNisOverlays::default();
    // tutorial_m_1a_goto_slappys: Clear, TutorialGesture_OlliePt1 and Pt2.
    fe.preload_nis_states(&[0, 80, 81]);
    assert_eq!(fe.take_preloaded(), 0);
    // ProcessOpenScreens creates the movie; it registers on the next frame.
    assert_eq!(frames(&mut fe, 1), vec![]);
    assert_eq!(fe.take_preloaded(), 0);
    assert_eq!(frames(&mut fe, 1), vec![ScreenCommand::Create { id: 0, template: TUTORIAL_GESTURE_OLLIE }]);
    assert_eq!(fe.take_preloaded(), 1);
    fe.set_state(80);
    fe.set_state(81);
    // Leaving Pt2 and entering Clear send nothing.
    fe.set_state(0);
    assert_eq!(fe.take_commands(), vec![
        ScreenCommand::SetState { id: 0, state: 1 },
        ScreenCommand::SetState { id: 0, state: 2 },
    ]);
    // PopNIS: OnLeave destroys the gesture screen; RemoveScreen runs on the
    // next frame.
    fe.pop_nis();
    assert_eq!(fe.take_commands(), vec![]);
    assert_eq!(frames(&mut fe, 1), vec![ScreenCommand::Destroy { id: 0 }]);
    assert_eq!(fe.gesture_ollie_id, -1);
}

#[test]
fn a_second_screen_waits_out_the_open_block_and_sponsor_logos_hold_the_preload() {
    let mut fe = FeNisOverlays::default();
    // NIS_AChalmers_ProIntro_S01_0: Clear, ProBioInfoOverlay_Left and
    // SponsorLogo_ipath (sponsor 19).
    fe.preload_nis_states(&[0, 10, 39]);
    assert_eq!(frames(&mut fe, 2), vec![ScreenCommand::Create { id: 0, template: 38 }]);
    // The registering frame and seven more count the block down; the logo
    // screen opens on the eighth and registers a frame later, nine after the
    // first. Its logos load the frame after that.
    assert_eq!(frames(&mut fe, 8), vec![]);
    assert_eq!(frames(&mut fe, 1), vec![ScreenCommand::Create { id: 1, template: SPONSOR_LOGO_SETUP }]);
    assert_eq!(fe.take_preloaded(), 0);
    frames(&mut fe, 1);
    assert_eq!(fe.take_preloaded(), 1);
    fe.set_state(10);
    fe.set_state(39);
    fe.set_state(0);
    assert_eq!(fe.take_commands(), vec![
        ScreenCommand::SetState { id: 0, state: 0 },
        ScreenCommand::SetState { id: 0, state: 1 },
        ScreenCommand::SetState { id: 1, state: 19 },
        ScreenCommand::SetState { id: 1, state: 50 },
    ]);
    assert_eq!(fe.bios_shown, 1);
    // DeletePreloadedScreens destroys the loaded screens in order.
    fe.pop_nis();
    assert_eq!(frames(&mut fe, 1), vec![ScreenCommand::Destroy { id: 0 }]);
    assert_eq!(frames(&mut fe, 1), vec![ScreenCommand::Destroy { id: 1 }]);
}

#[test]
fn a_state_entered_while_its_screen_loads_sends_nothing_but_its_leave_does() {
    let mut fe = FeNisOverlays::default();
    fe.preload_nis_states(&[2]);
    // mScreenMap holds -1 until the screen registers.
    fe.set_state(2);
    assert_eq!(fe.take_commands(), vec![]);
    assert_eq!(frames(&mut fe, 2), vec![ScreenCommand::Create { id: 0, template: 48 }]);
    // SpotFound_BottomLeft's outro, then its intro and outro again.
    fe.set_state(0);
    fe.set_state(2);
    fe.set_state(0);
    assert_eq!(fe.take_commands(), vec![
        ScreenCommand::SetState { id: 0, state: 5 },
        ScreenCommand::SetState { id: 0, state: 4 },
        ScreenCommand::SetState { id: 0, state: 5 },
    ]);
}
