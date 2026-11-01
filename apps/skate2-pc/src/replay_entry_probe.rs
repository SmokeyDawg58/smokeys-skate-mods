//! Opt-in end-to-end Career -> StartHub -> Replay Editor controller route.
//! No fabricated recorder frames, state changes or readiness acknowledgements.
use super::PcSession;
use skate2_core::{XboxPadSample,ui::{pause_menu::PauseMenuOption,main_menu::Screen,replay_editor::Mode}};
use crate::gpu_renderer::RenderResult;
#[path="replay_media_probe.rs"]
mod replay_media_probe;
#[derive(Default)]
pub(super) struct Probe {
    media:replay_media_probe::Probe,
    ticks:u64,
    career_frames:Option<u64>,
    guide_entered:bool,
    guide_ticks:u64,
    guide_done:bool,
    entered:bool,
    ready_ticks:u64,
    rendered:u64,
    presented:u64,
    forward_parameter:Option<f32>,
    rewound:bool,
    camera_edited:bool,
    advanced_camera_edited:bool,
    key_inserted:bool,
    camera_before:Option<[[f32;3];2]>,
    camera_after:Option<[[f32;3];2]>,
    zoom_before:f32,
    trim_before:Option<[f32;2]>,
    exit_requested:bool,
    pause_ticks:u64,
    pause_presented:u64,
    resumed_at:Option<u64>,
    pub done:bool,
}
impl Probe {
    pub fn input(&mut self,session:&PcSession,completed:u64)->RenderResult<XboxPadSample> {
        self.ticks+=1;
        if self.ticks>14_400 {return Err(format!("Career replay test timed out: entered={} ready_ticks={} rendered={} presented={} frames={completed}",self.entered,self.ready_ticks,self.rendered,self.presented).into());}
        // No held buttons between scripted presses. The older skating demo's
        // neutral_demo_sample holds bit 0 (D-pad Up), which navigates FE lists.
        let mut sample=XboxPadSample {buttons:0,left_trigger:0,right_trigger:0,
            left_x:0,left_y:0,right_x:0,right_y:0};
        if let Some(guide)=session.trick_guide_snapshot() {
            self.guide_entered=true;
            if guide.ready && session.frontend_fade_alpha()==0. {
                if self.guide_ticks>=150 {
                    if self.ticks.is_multiple_of(30) {sample.buttons=0x2000;}
                } else if guide.has_submenu {
                    if self.ticks.is_multiple_of(30) {
                        let target=guide.labels.iter().position(|s|s.contains("GRAB")).unwrap_or(0);
                        sample.buttons=if guide.selected==target {0x1000} else {0x0002};
                    }
                } else {
                    self.guide_ticks+=1;
                    if self.guide_ticks==1 {eprintln!("REPLAY_TEST Trick Guide leaf: {:?}",guide.labels);}
                    if self.guide_ticks==60 {sample.buttons=0x0002;}
                    if self.guide_ticks==90 {sample.buttons=0x0001;}
                    if self.guide_ticks==120 && guide.selected!=0 {return Err("Trick Guide leaf navigation did not return to first item".into());}
                }
            }
        } else if let Some(editor)=session.replay_editor_snapshot() {
            if !self.entered {
                if session.career_local_player().is_none(){return Err("Replay test entered outside Career".into());}
                if editor.frames<150 {return Err("Replay test opened without 150 recorded frames".into());}
                self.entered=true;eprintln!("REPLAY_TEST entered Career editor: frames={} mode={:?}",editor.frames,editor.mode);
            }
            if editor.ready && session.frontend_fade_alpha()==0. {
                if self.media.enabled {
                    self.exit_requested|=self.media.input(session,&editor,&mut sample)?;
                    return Ok(sample);
                }
                self.ready_ticks+=1;
                if std::env::var_os("SKATE2_DIAGNOSTIC_WEATHER_REPLAY_SHORT").is_some() && self.ready_ticks==130 {
                    let weather=session.replay_weather().ok_or("replay omitted recorded weather")?;
                    eprintln!("REPLAY_WEATHER_TEST restored mode={:?} phase={:.3}",weather.mode,weather.phase);
                    self.done=true;
                    return Ok(sample);
                }
                match self.ready_ticks {
                    1..=60=>sample.right_trigger=255,
                    61=>{self.forward_parameter=Some(editor.parameter);eprintln!("REPLAY_TEST forward parameter={}",editor.parameter);},
                    62..=120=>sample.left_trigger=255,
                    121=>{
                        self.rewound=self.forward_parameter.is_some_and(|p|editor.parameter<p);
                        if !self.rewound{return Err(format!("Replay test LT failed to rewind: {:?} -> {}",self.forward_parameter,editor.parameter).into());}
                        eprintln!("REPLAY_TEST rewind parameter={}",editor.parameter);
                    }
                    135|195=>sample.buttons=0x0200, // RB: play/pause.
                    241=>sample.buttons=0x1000, // A: Edit timeline.
                    265=>{
                        if editor.mode!=Mode::Timeline || !editor.render_timeline {return Err("Replay test did not enter Edit timeline".into());}
                        eprintln!("REPLAY_TEST Edit timeline active");
                    }
                    270..=330=>sample.left_trigger=255,
                    345=>sample.buttons=0x1000, // A: camera/effects list at the cursor.
                    360=>{
                        if editor.mode!=Mode::Effects || editor.edit_selection!=2 {return Err(format!("Replay test expected the Game camera row: mode={:?} selection={}",editor.mode,editor.edit_selection).into());}
                    }
                    375=>sample.buttons=0x0002, // Game -> Free.
                    420=>{
                        if editor.edit_selection!=3 || !editor.film_pack {return Err("Replay test Freecam row unavailable".into());}
                    }
                    435=>sample.buttons=0x1000,
                    465=>{
                        if editor.mode!=Mode::Free {return Err(format!("Replay test failed to enter Freecam: {:?}",editor.mode).into());}
                        self.camera_edited=true;self.camera_before=Some(camera(session)?);self.zoom_before=editor.zoom;
                        eprintln!("REPLAY_TEST Freecam active: {:?}",self.camera_before);
                    }
                    466..=480=>sample.buttons=0x0001, // D-pad Up translates camera and target.
                    495=>{
                        let next=camera(session)?;let prior=self.camera_before.expect("baseline");
                        if next[0][1]-prior[0][1]<0.8 || distance(next[1],prior[1])>0.03 {
                            return Err(format!("Freecam height changed aim or failed to rise: {prior:?} -> {next:?}").into());
                        }
                        self.camera_before=Some(next);eprintln!("REPLAY_TEST Freecam height: {next:?}");
                    }
                    500..=530=>sample.right_x=12000,
                    540=>{
                        let next=camera(session)?;let prior=self.camera_before.expect("height");
                        if distance(next[0],prior[0])>0.03 || distance(next[1],prior[1])<0.1 {
                            return Err(format!("Freecam yaw failed or moved eye: {prior:?} -> {next:?}").into());
                        }
                        self.camera_before=Some(next);eprintln!("REPLAY_TEST Freecam yaw: {next:?}");
                    }
                    550..=565=>sample.left_x=8000,
                    575=>{
                        let next=camera(session)?;let prior=self.camera_before.expect("yaw");
                        if distance(next[0],prior[0])<0.05 || distance(next[1],prior[1])>0.03 {
                            return Err(format!("Freecam strafe failed or changed aim: {prior:?} -> {next:?}").into());
                        }
                        self.camera_after=Some(next);eprintln!("REPLAY_TEST Freecam strafe: {next:?}");
                    }
                    600..=610=>sample.buttons=0x0008, // D-pad Right: zoom.
                    620=>if editor.zoom<=self.zoom_before {return Err("Freecam zoom did not change".into());},
                    630=>sample.buttons=0x0010, // Start: help during camera editing.
                    660=>if editor.mode!=Mode::Help {return Err("Freecam help did not open".into());},
                    690=>sample.buttons=0x2000,
                    710=>if editor.mode!=Mode::Free {return Err("Help did not return to Freecam".into());},
                    720=>sample.buttons=0x1000, // A: finish basic Freecam.
                    750=>{
                        if editor.mode!=Mode::Timeline {return Err("Replay test did not return from Freecam".into());}
                        let next=camera(session)?;let prior=self.camera_after.expect("edited pose");
                        if distance(next[0],prior[0])>0.03 || distance(next[1],prior[1])>0.03 {
                            return Err(format!("Freecam commit lost the edited pose: {prior:?} -> {next:?}").into());
                        }
                        eprintln!("REPLAY_TEST Freecam pose committed: {next:?}");
                    }
                    765=>sample.buttons=0x1000,
                    795=>{
                        if editor.mode!=Mode::Effects || editor.edit_selection!=3 {return Err("Replay test did not reopen Freecam edit menu".into());}
                    }
                    810|840|870=>sample.buttons=0x0002, // Free -> Advanced Follow -> Tripod -> Free.
                    885=>{
                        if editor.edit_selection!=6 {return Err("Replay test Advanced Freecam row unavailable".into());}
                    }
                    900=>sample.buttons=0x1000,
                    930=>{
                        if editor.mode!=Mode::Free {return Err(format!("Replay test failed to enter Advanced Freecam: {:?}",editor.mode).into());}
                        self.advanced_camera_edited=true;eprintln!("REPLAY_TEST Advanced Freecam active");
                    }
                    955..=985=>{sample.left_x=6000;sample.right_x=12000;}
                    1005=>sample.buttons=0x1000,
                    1035=>{
                        if editor.mode!=Mode::Timeline || editor.intervals.len()<2 {return Err(format!("Replay test camera key missing: mode={:?} intervals={}",editor.mode,editor.intervals.len()).into());}
                        self.key_inserted=true;eprintln!("REPLAY_TEST camera key retained: {} intervals",editor.intervals.len());
                    }
                    1100..=1120=>sample.right_trigger=255,
                    1150=>sample.buttons=0x1000,
                    1165=>if editor.mode!=Mode::Effects || editor.edit_selection!=6 {return Err("Advanced Freecam effects menu unavailable".into());},
                    1180|1230|1290=>sample.buttons=0x0002,
                    1200=>sample.buttons=0x0004, // Speed 100 -> 90.
                    1260|1320=>sample.buttons=0x0008, // Shake Off -> Low; watermark Custom -> Off.
                    1350=>sample.buttons=0x2000,
                    1380=>sample.buttons=0x1000,
                    1410=>{
                        if editor.mode!=Mode::Effects || editor.speed!=90 || editor.shake!=1 || editor.watermark!=0 {
                            return Err(format!("Replay effects did not persist: speed={} shake={} watermark={} mode={:?}",editor.speed,editor.shake,editor.watermark,editor.mode).into());
                        }
                        eprintln!("REPLAY_TEST speed/shake/watermark committed and reopened");
                    }
                    1430=>sample.buttons=0x0010,
                    1460=>sample.buttons=0x1000,
                    1490=>sample.buttons=0x2000,
                    1520=>sample.buttons=0x2000,
                    1540=>self.trim_before=Some([editor.start_head,editor.end_head]),
                    1550..=1570=>{sample.buttons=0x0100;sample.left_x=12000;}
                    1571..=1620=>{sample.buttons=0x0100;sample.right_x=-12000;}
                    1630=>{
                        let prior=self.trim_before.expect("trim heads");
                        if editor.mode!=Mode::Timeline || editor.start_head<=prior[0] || editor.end_head>=prior[1] {
                            return Err(format!("LB trim controls failed: {prior:?} -> [{},{}] mode={:?}",editor.start_head,editor.end_head,editor.mode).into());
                        }
                        eprintln!("REPLAY_TEST both trim heads moved and LB release returned to Timeline");
                    }
                    1640|1690=>sample.buttons=0x0200,
                    1660=>if editor.mode!=Mode::Play {return Err("Trimmed playback did not start".into());},
                    1740=>sample.buttons=0x2000, // B: editor menu.
                    1770=>{
                        if !editor.popup.as_ref().is_some_and(|p|p.options.get(1).is_some_and(|s|s=="ID_REPLAYEDITOR_EXIT")) {
                            return Err("Replay test exit menu absent".into());
                        }
                        sample.buttons=0x0002;
                    }
                    1800=>sample.buttons=0x1000,
                    1830=>{
                        if !editor.popup.as_ref().is_some_and(|p|p.description=="ID_REPLAY_EDITOR_QUIT") {
                            return Err("Replay test exit confirmation absent".into());
                        }
                        sample.buttons=0x0002;
                    }
                    1860=>{self.exit_requested=true;sample.buttons=0x1000;}
                    _=>{},
                }
            }
        } else if self.entered {
            if !self.exit_requested {return Err("Replay test unexpectedly left the editor".into());}
            let pause=session.pause_menu_snapshot();
            if pause.visible {
                if !pause.screen_hidden && session.frontend_fade_alpha()==0. {
                    self.pause_ticks+=1;
                    if self.pause_ticks==1 {eprintln!("REPLAY_TEST visible pause menu restored");}
                    if self.pause_ticks>=120 && self.pause_presented>=120 && self.ticks.is_multiple_of(30) {
                        sample.buttons=0x2000; // B: resume actual simulation.
                    }
                }
            } else {
                if self.pause_ticks<120 || self.pause_presented<120 {return Err("Replay test left pause before a visible return".into());}
                let first=*self.resumed_at.get_or_insert(completed);
                if completed>=first+120 && (self.media.done || self.rewound && self.camera_edited && self.advanced_camera_edited && self.key_inserted) {
                    self.done=true;
                    if self.media.done {
                        eprintln!("REPLAY_TEST PASS: Career -> Replay Editor -> 1080p photo -> cancel MP4 -> save MP4/editable archive -> restored cursor/trim -> visible pause return -> 120 resumed simulation ticks");
                    } else {
                        eprintln!("REPLAY_TEST PASS: Career -> Trick Guide leaf/navigation -> working pause input -> Replay Editor; RT/LT, RB, Freecam height/yaw/strafe/zoom/commit/help, advanced key, effects persistence/help, both trim heads, visible pause return, 120 resumed simulation ticks; {} rendered, {} presented editor frames",self.rendered,self.presented);
                    }
                }
            }
        }
        else if let Some(menu)=session.main_menu_frame()? {
            if menu.name_prompt.is_some(){return Err("Replay test requires a copied existing Career profile".into());}
            if menu.screen==Screen::MainMenu && self.ticks.is_multiple_of(30) {
                sample.buttons=if menu.selected==0 {0x1000}else{0x0001};
            }
        } else if session.career_local_player().is_some() {
            let first=*self.career_frames.get_or_insert_with(|| {eprintln!("REPLAY_TEST Career active at {completed} completed frames");completed});
            let pause=session.pause_menu_snapshot();
            if self.guide_entered && !self.guide_done {
                if self.guide_ticks<150 {return Err("Trick Guide exited before leaf checks".into());}
                self.guide_done=true;eprintln!("REPLAY_TEST Trick Guide closed; checking actual pause input by opening Replay");
            }
            if pause.visible {
                if self.ticks.is_multiple_of(30) {
                    let destination=if self.guide_done||self.media.enabled {PauseMenuOption::Replay} else {PauseMenuOption::TrickGuide};
                    if pause.options()[pause.selected]==destination {
                        if !pause.item_enabled(destination){return Err("Career diagnostic destination disabled".into());}
                        sample.buttons=0x1000;
                    } else {sample.buttons=0x0002;}
                }
            } else if completed>=first+360 {
                if self.ticks.is_multiple_of(60){sample.buttons=0x0010;}
            } else if completed>first+60 && (completed-first)%90<20 {sample.buttons=0x1000;}
        }
        Ok(sample)
    }
    pub fn rendered(&mut self,ready:bool){if ready {self.rendered+=1;}}
    pub fn presented(&mut self,ready:bool,pause:bool,alpha:f32) {
        if self.done || alpha!=0. || self.rendered==0 {return;}
        if ready {self.presented+=1;}
        if self.exit_requested && pause {self.pause_presented+=1;}
    }
}

fn camera(session:&PcSession)->RenderResult<[[f32;3];2]> {
    let visual=session.try_replay_editor_visual_snapshot()?;
    Ok([visual.camera_world_position,visual.camera_forward])
}
fn distance(a:[f32;3],b:[f32;3])->f32 {a.into_iter().zip(b).map(|(a,b)|(a-b)*(a-b)).sum::<f32>().sqrt()}
