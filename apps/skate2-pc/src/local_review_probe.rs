//! Opt-in bounded controller diagnostic. Normal play never constructs this owner.
use super::PcSession;
use std::time::Instant;
use skate2_core::{XboxPadSample, ui::{pause_menu::PauseMenuOption, replay_editor::Mode}};
use crate::gpu_renderer::RenderResult;

#[derive(Default)]
pub(super) struct Probe {
    ticks: u32,
    phase: u8,
    phase_ticks: u32,
    menu_samples: u32,
    heading: Option<f32>,
    playback: Option<(Instant, f32)>,
    pub done: bool,
}
impl Probe {
    fn next(&mut self, phase: u8) { self.phase = phase; self.phase_ticks = 0; }
    pub fn input(&mut self, session: &PcSession, completed: u64) -> RenderResult<XboxPadSample> {
        self.ticks += 1;
        self.phase_ticks += 1;
        if self.ticks > 1800 { return Err(format!("LOCAL_REVIEW timeout phase={} frames={completed}", self.phase).into()); }
        let mut pad = XboxPadSample {buttons:0,left_trigger:0,right_trigger:0,left_x:0,left_y:0,right_x:0,right_y:0};
        if std::env::var("SKATE2_DIAGNOSTIC_LOCAL_REVIEW").is_ok_and(|v| v == "mount") {
            if completed == 30 || completed == 150 { pad.buttons = 0x8000; }
            if (151..260).contains(&completed) { pad.left_x = 24000; pad.buttons |= 0x1000; }
            if completed % 5 == 0 {
                eprintln!("LOCAL_REVIEW mount frame={completed} heading={:?} {}",session.diagnostic_skate3_controls().map(|s|s.2),
                    session.diagnostic_skate3_transition().unwrap_or_default());
            }
            if completed == 151 {self.heading=session.diagnostic_skate3_controls().map(|s|s.2);}
            if completed >= 260 {self.done=true;}
            return Ok(pad);
        }
        if std::env::var("SKATE2_DIAGNOSTIC_LOCAL_REVIEW").is_ok_and(|v|v == "offboard-facing") {
            if completed == 60 {pad.buttons=0x8000;}
            if (160..200).contains(&completed) {pad.buttons=0x0100;}
            if completed==180 {pad.buttons|=0x0002;}
            if (220..300).contains(&completed) {pad.left_y=25000;}
            if (400..440).contains(&completed) {pad.buttons=0x0101;}
            if completed==175 {self.heading=session.diagnostic_skate3_controls().map(|s|s.2);}
            if matches!(completed,175|185|400|420|440) {eprintln!("LOCAL_REVIEW offboard-facing frame={completed} marker={:?} heading={:?} {}",session.diagnostic_marker_status(),session.diagnostic_skate3_controls().map(|s|s.2),session.diagnostic_skate3_transition().unwrap_or_default());}
            if completed==440 {
                let (sets,generation,on_board)=session.diagnostic_marker_status().ok_or("LOCAL_REVIEW offboard marker absent")?;
                let heading=session.diagnostic_skate3_controls().ok_or("LOCAL_REVIEW offboard source absent")?.2;
                let start=self.heading.ok_or("LOCAL_REVIEW initial offboard facing absent")?;
                let delta=(heading-start).sin().atan2((heading-start).cos());
                if sets==0||generation==0||on_board||delta.abs()>0.05 {return Err(format!("LOCAL_REVIEW offboard facing mismatch: sets={sets} generation={generation} onboard={on_board} delta={delta}").into());}
                eprintln!("LOCAL_REVIEW offboard-facing passed delta={delta}");
            }
            if completed>=470 {self.done=true;}
            return Ok(pad);
        }
        if std::env::var("SKATE2_DIAGNOSTIC_LOCAL_REVIEW").is_ok_and(|v|v == "marker-turn") {
            if (60..180).contains(&completed) || (340..380).contains(&completed) {pad.buttons=0x0100;}
            if completed==150 {pad.buttons|=0x0002;}
            if (200..300).contains(&completed) {pad.buttons=0x1000;}
            if (380..400).contains(&completed) {pad.buttons=0x0101;}
            if (400..450).contains(&completed) {pad.left_x=24000;}
            if (450..520).contains(&completed) {pad.left_x=-24000;}
            if completed%5==0 && completed>=390 {eprintln!("LOCAL_REVIEW marker-turn frame={completed} marker={:?} heading={:?} {}",session.diagnostic_marker_status(),session.diagnostic_skate3_controls().map(|s|s.2),session.diagnostic_skate3_transition().unwrap_or_default());}
            if completed==415 || completed==455 {self.heading=session.diagnostic_skate3_controls().map(|s|s.2);}
            if completed==440 || completed==485 {
                let (sets,generation,on_board)=session.diagnostic_marker_status().ok_or("LOCAL_REVIEW marker status absent")?;
                if sets==0||generation==0||!on_board {return Err("LOCAL_REVIEW did not place/return to an onboard marker".into());}
                let heading=session.diagnostic_skate3_controls().ok_or("LOCAL_REVIEW marker source absent")?.2;
                let start=self.heading.ok_or("LOCAL_REVIEW marker heading absent")?;
                let delta=(heading-start).sin().atan2((heading-start).cos());
                if delta.abs()<0.5 {return Err(format!("LOCAL_REVIEW marker steering too slow: frame={completed} delta={delta}").into());}
                eprintln!("LOCAL_REVIEW marker-turn direction passed frame={completed} delta={delta}");
            }
            if completed>=550 {self.done=true;}
            return Ok(pad);
        }
        if std::env::var("SKATE2_DIAGNOSTIC_LOCAL_REVIEW").is_ok_and(|v|v == "air" || v == "air-straight" || v == "tailwalk") {
            if completed < 145 {pad.buttons=0x1000;}
            if (150..166).contains(&completed) {pad.right_y=-30000;}
            if (167..169).contains(&completed) {pad.right_x=-30000;pad.right_y=30000;}
            if std::env::var("SKATE2_DIAGNOSTIC_LOCAL_REVIEW").is_ok_and(|v|v == "air")
                && session.diagnostic_state_ids().is_some_and(|(_,_,p)|p==200||p==201) {pad.left_x=24000;}
            if std::env::var("SKATE2_DIAGNOSTIC_LOCAL_REVIEW").is_ok_and(|v|v == "tailwalk") {
                if (167..169).contains(&completed) {pad.right_x=0;}
                if (175..225).contains(&completed) {pad.left_trigger=255;pad.right_y=30000;}
                if (185..220).contains(&completed) {pad.buttons=0x2000;}
            }
            if completed%5==0 {eprintln!("LOCAL_REVIEW air frame={completed} {}",session.diagnostic_skate3_transition().unwrap_or_default());}
            if completed>=350 {self.done=true;}
            return Ok(pad);
        }
        if std::env::var("SKATE2_DIAGNOSTIC_LOCAL_REVIEW").is_ok_and(|v|v == "startup-turn") {
            match self.phase {
                0 => if self.phase_ticks%15==0 {
                    if let Some(frame)=session.main_menu_frame()? {
                        if frame.menu_visible {
                            if frame.selected==2 {pad.buttons=0x1000;self.next(1);}
                            else {pad.buttons=0x0002;}
                        }
                    }
                },
                1 => if self.phase_ticks%15==0 {
                    if let Some(frame)=session.main_menu_frame()? {
                        if frame.freeskate.is_some_and(|s|s.image.is_some()) {pad.buttons=0x1000;self.next(2);}
                    }
                },
                2 => if session.main_menu_frame()?.is_none() {
                    if let Some((true, _, heading)) = session.diagnostic_skate3_controls() {
                        self.heading = Some(heading);
                        self.next(3);
                    }
                },
                3 => {
                    pad.left_x=24000;pad.buttons=0x1000;
                    if self.phase_ticks%5==0 {eprintln!("LOCAL_REVIEW startup-turn tick={} {:?} {}",self.phase_ticks,session.diagnostic_skate3_controls(),session.diagnostic_skate3_transition().unwrap_or_default());}
                    if self.phase_ticks>=45 {
                        let (focused,axes,heading)=session.diagnostic_skate3_controls().ok_or("LOCAL_REVIEW startup source absent")?;
                        let start=self.heading.ok_or("LOCAL_REVIEW startup heading absent")?;
                        let delta=(heading-start).sin().atan2((heading-start).cos()).abs();
                        if !focused || axes[0].abs()<0.1 || delta<0.01 {return Err(format!("LOCAL_REVIEW startup steering failed: focused={focused} axes={axes:?} delta={delta}").into());}
                        eprintln!("LOCAL_REVIEW startup-turn passed heading_delta={delta}");self.done=true;
                    }
                },
                _=>{}
            }
            return Ok(pad);
        }
        let online = std::env::var_os("SKATE2_DIAGNOSTIC_ONLINE").is_some();
        match self.phase {
            0 => {
                if (20..90).contains(&completed) {
                    pad.buttons=0x1000; pad.left_x=24000;
                }
                if completed==20 { self.heading=session.diagnostic_skate3_controls().map(|s|s.2); }
                if completed==90 {
                    let heading=session.diagnostic_skate3_controls().ok_or("LOCAL_REVIEW no source controller")?.2;
                    let delta=(heading-self.heading.ok_or("LOCAL_REVIEW no initial heading")?).sin().atan2((heading-self.heading.unwrap()).cos()).abs();
                    eprintln!("LOCAL_REVIEW steering heading_delta_radians={delta}");
                    if delta < 0.03 { return Err("LOCAL_REVIEW steering did not respond".into()); }
                }
                if completed>=195 {pad.left_x=24000;pad.buttons=0x1000;}
                if completed>=210 && (!online || session.online_simulates_under_frontend()) {
                    self.next(1); pad.buttons=0x10;
                }
            }
            1 => {
                if session.pause_menu_snapshot().visible {
                    self.next(2);
                    eprintln!("LOCAL_REVIEW menu opened online={online}");
                } else if self.phase_ticks%30==0 {pad.buttons=0x10;}
            }
            2 => {
                // Start with a deflected stick and keep it held while the FE owns input.
                pad.left_x=-30000;
                if self.phase_ticks>5 && session.online_simulates_under_frontend() {
                    let (focused,axes,_)=session.diagnostic_skate3_controls().ok_or("LOCAL_REVIEW source absent under menu")?;
                    if focused || axes.iter().any(|v|v.abs()>0.00001) {
                        return Err(format!("LOCAL_REVIEW menu input leaked focused={focused} axes={axes:?}").into());
                    }
                    self.menu_samples+=1;
                }
                if self.phase_ticks>=40 {
                    eprintln!("LOCAL_REVIEW menu held-input samples={} cleared",self.menu_samples);
                    self.next(if online {7} else {3});
                    pad.buttons=0;pad.left_x=0;
                }
            }
            3 => {
                let pause=session.pause_menu_snapshot();
                if session.replay_editor_open() {self.next(4);}
                else if self.phase_ticks%15==0 {
                    if pause.visible {
                        pad.buttons=if pause.options()[pause.selected]==PauseMenuOption::Replay {0x1000}else{0x0001};
                    } else {pad.buttons=0x10;}
                }
            }
            4 => {
                if session.replay_editor_snapshot().is_some_and(|s|s.ready) && session.frontend_fade_alpha()==0. {
                    self.next(5);
                }
            }
            5 => {
                let editor=session.replay_editor_snapshot().ok_or("LOCAL_REVIEW rewind editor absent")?;
                // Rewind far enough for the entire timed sample. A fixed number
                // of trigger ticks can leave less than a second before clip end.
                if editor.parameter>0.15 {pad.left_trigger=255;}
                else {self.next(9);}
            }
            9 => {
                if self.phase_ticks==2 {pad.buttons=0x0200;}
                else if let Some(editor)=session.replay_editor_snapshot() {
                    if editor.mode==Mode::Play {
                        eprintln!("LOCAL_REVIEW replay baseline parameter={} frames={} mode={:?}",editor.parameter,editor.frames,editor.mode);
                        self.playback=Some((Instant::now(),editor.parameter));self.next(6);
                    }
                }
            }
            6 => {
                let (start,parameter)=self.playback.ok_or("LOCAL_REVIEW playback baseline absent")?;
                if start.elapsed().as_secs_f32()>=1.0 {
                    let editor=session.replay_editor_snapshot().ok_or("LOCAL_REVIEW editor disappeared")?;
                    // Author frame labels are 30Hz; the recorded SIM is 60Hz.
                    let seconds=(editor.parameter-parameter)*editor.frames as f32/30.;
                    let wall=start.elapsed().as_secs_f32();let rate=seconds/wall;
                    eprintln!("LOCAL_REVIEW replay recorded_seconds={seconds} wall_seconds={wall} rate={rate} speed={} mode={:?} parameter={} frames={}",editor.speed,editor.mode,editor.parameter,editor.frames);
                    if !(0.75..=1.25).contains(&rate) {return Err("LOCAL_REVIEW replay did not play at normal speed".into());}
                    self.done=true;
                }
            }
            7 => {
                if self.phase_ticks%20==1 {pad.buttons=0x10;}
                if !session.pause_menu_snapshot().visible {
                    self.next(8);
                }
            }
            8 => {
                pad.left_x=26000;pad.buttons=0x1000;
                if self.phase_ticks>=10 {
                    let (focused,axes,_)=session.diagnostic_skate3_controls().ok_or("LOCAL_REVIEW resumed source absent")?;
                    eprintln!("LOCAL_REVIEW resumed focused={focused} axes={axes:?}");
                    if !focused || axes[0].abs()<0.1 {return Err("LOCAL_REVIEW input did not resume".into());}
                    self.done=true;
                }
            }
            _=>{}
        }
        Ok(pad)
    }
}
