//! Replay2 frontend (82653b88/826799f8/82682b68) and ReplayEditorBackEnd.
//! The authored screen is replay2/replay_editor2, screen15, frontend state12.
use skate2_formats::ice_camera::{Spaces, editor::{Author, CameraKind, CAMERA, SHAKE, SPEED, element}};
use super::FeSoundRequest;
#[path="replay_editor_footage.rs"]
mod footage;
pub use footage::{Footage,FootageItem};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode { Play=1, Trim=2, Instant=5, Timeline=6, Follow=7, Tripod=8, Game=9, Free=10, Effects=11, FrameStep=13, Help=16 }
impl Mode {
    pub const fn editing_camera(self) -> bool { matches!(self, Self::Follow|Self::Tripod|Self::Game|Self::Free) }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Marker { None, Clip, Key }

/// The PRES input cache is drained once by the session. These are native FE
/// key codes and raw analog values, not a second controller sample.
#[derive(Clone, Debug, Default)]
pub struct Input {
    pub keys: Vec<u8>,
    pub left: [f32;2],
    pub right: [f32;2],
    pub triggers: [f32;2],
    pub dpad: [bool;4],
}

/// Live linked ReplayEditorTweaks, read at the original frontend update.
#[derive(Clone, Copy, Debug)]
pub struct Tuning {
    pub marker_threshold: f32,
    pub scrub_max: f32,
    pub scrub_smoothing: f32,
    pub zoom_delta: f32,
    pub free_ls: f32,
    pub free_rs: f32,
    pub free_height: f32,
    pub orbit_ls: f32,
    pub orbit_rs: f32,
    pub orbit_height: f32,
    pub orbit_multiplier: f32,
    pub frame_step: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Popup {
    pub title: String,
    pub description: String,
    pub options: Vec<String>,
    pub selected: usize,
    pub cancellable:bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dialog { Menu, Exit, SaveReplay, SavePhoto, Resolution, FrameRate, Upload, Delete(bool), Insert, Overwrite, Notice, MediaFull(bool), MediaDelete(bool,u32), MediaDeleteConfirm(bool,u32) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action { None, Exit, SaveReplay, SavePhoto, SignInRequired, Browse(bool), DeleteMedia(bool,u32) }

/// Host video export, advanced by encoded frames rather than wall time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportStatus { Waiting, Frame, Finished, Cancelled }

/// PC port addition: the saved video's height in lines (720 or 1080) and frame rate (30 or 60),
/// chosen after the save is confirmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VideoSettings { pub height: u32, pub fps: u32 }
impl Default for VideoSettings { fn default() -> Self { Self { height: 1080, fps: 60 } } }

#[derive(Clone, Debug)]
pub struct Interval {
    pub start: f32,
    pub end: f32,
    pub camera: CameraKind,
    pub hard: bool,
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub mode: Mode,
    pub render_timeline: bool,
    pub visible: bool,
    pub cursor_visible: bool,
    pub ready: bool,
    pub film_pack: bool,
    pub signed_in: bool,
    pub parameter: f32,
    pub start_head: f32,
    pub end_head: f32,
    pub cursor: i32,
    pub start: i32,
    pub end: i32,
    pub frames: i32,
    pub encoded_seconds: u32,
    pub video: VideoSettings,
    pub edit_selection: usize,
    pub speed: u32,
    pub shake: u8,
    pub watermark: u8,
    pub zoom: f32,
    pub marker: Marker,
    pub marker_index: Option<usize>,
    pub intervals: Vec<Interval>,
    pub selected_clip: [f32;2],
    pub popup: Option<Popup>,
    pub saving: bool,
    pub stalled: bool,
    pub footage:Option<Footage>,
}

pub struct Controller {
    pub author: Author,
    pub mode: Mode,
    pub ready: bool,
    pub saving: bool,
    pub stalled: bool,
    footage:Option<Footage>,
    film_pack: bool,
    signed_in: bool,
    view_from_save: bool,
    instant: bool,
    render_timeline: bool,
    visible: bool,
    cursor_visible: bool,
    previous_help: Mode,
    request_camera: bool,
    reset_collision: bool,
    selection: usize,
    speed: u32,
    shake: u8,
    watermark: u8,
    marker: Option<u8>,
    threshold: f32,
    popup: Option<(Dialog,Popup)>,
    menu_actions: Vec<Action>,
    trigger_velocity: f32,
    head_velocity: [f32;2],
    height_delta: f32,
    states: Vec<u32>,
    sounds: Vec<FeSoundRequest>,
    pending: Action,
    video: VideoSettings,
}

impl Controller {
    /// BE::Start82567450/FE::OnEnter82653b88. The caller freezes submissions
    /// and owns the native pause7 until exit restores the live presentation.
    pub fn new(length:f32, film_pack:bool, signed_in:bool, tuning:Tuning) -> Result<Self> {
        Ok(Self { author:Author::new(length)?,mode:Mode::Instant,ready:false,saving:false,stalled:false,
            film_pack,signed_in,view_from_save:false,footage:None,instant:true,render_timeline:false,visible:true,cursor_visible:true,previous_help:Mode::Instant,
            request_camera:false,reset_collision:false,selection:0,speed:100,shake:0,watermark:if film_pack {2} else {1},
            marker:None,threshold:tuning.marker_threshold,popup:None,menu_actions:Vec::new(),
            trigger_velocity:0.,head_velocity:[0.;2],height_delta:0.,states:Vec::new(),sounds:Vec::new(),pending:Action::None,video:VideoSettings::default() })
    }
    pub fn intro_complete(&mut self) -> Result<()> {
        if !self.ready {
            self.ready=true;self.set_mode(if self.instant {Mode::Instant} else {Mode::Timeline})?;
            // A loaded take receives state9 from APT InstantDone before its
            // UpdateReplayEditorReady callback allows frontend input.
            self.refresh_cursor(false);
        }
        Ok(())
    }
    /// PrepareLoadedData82567d98 / OnIntroDone82653d80: restored edits enter
    /// the timeline at the untrimmed end, rather than reset to instant replay.
    pub fn restore_take(&mut self,take:skate2_formats::ice_camera::editor::EditableTake) {
        self.author.take=take;self.author.take.set_parameter(1.);
        // SerializeReplayData82575728 does not store trim heads or watermark.
        [self.author.start,self.author.end]=[0.,1.];
        self.view_from_save=true;self.instant=false;self.mode=Mode::Timeline;
    }
    pub fn take_states(&mut self) -> Vec<u32> { std::mem::take(&mut self.states) }
    /// Start82567450 clears mbEnableRender. OnIntroDone82653d80 enables it
    /// for loaded footage; entering EditTimeline enables it for a live replay.
    pub fn enable_timeline_render(&mut self) {self.render_timeline=true;}
    pub fn browsing_footage(&self) -> bool {self.footage.is_some()}
    pub fn take_sounds(&mut self) -> Vec<FeSoundRequest> { std::mem::take(&mut self.sounds) }
    /// PrepareOrbit8255da40 / Tripod8255db50 / Free8255dd70 reset the
    /// world manager before the first constraint query of the new camera.
    pub fn take_collision_reset(&mut self)->bool {std::mem::take(&mut self.reset_collision)}
    fn screen(&mut self,state:u32) { self.states.push(state); }
    fn sound(&mut self, key:u64) { self.sounds.push(FeSoundRequest::PlayKey(key)); }
    fn reset_deltas(&mut self) { self.trigger_velocity=0.;self.head_velocity=[0.;2];self.height_delta=0.; }
    pub fn can_show_hide(&self) -> bool { matches!(self.mode,Mode::Instant|Mode::Timeline|Mode::Play) }
    pub fn marker_limit(&self) -> bool { self.author.take.key_count(CAMERA)>=100 || self.author.take.interval_count(CAMERA)>=100 }
    pub fn valid_cursor(&self) -> bool {
        let p=self.author.take.parameter(); p>self.threshold && p<1.-self.threshold
    }
    /// CanPlaceMarker8255d968 scans interval starts in order, with inclusive
    /// detection windows trimmed by the same live threshold at both outer ends.
    pub fn marker_index(&self) -> Option<usize> {
        let p=self.author.take.parameter();
        (0..self.author.take.interval_count(CAMERA)).find(|&i| {
            let s=self.author.take.interval_parameter(CAMERA,i);
            p>=(s-self.threshold).max(self.threshold) && p<=(s+self.threshold).min(1.-self.threshold)
        })
    }
    pub fn marker(&self) -> Marker {
        if self.author.take.interval_count(CAMERA)<=1 { return Marker::None; }
        self.marker_index().map_or(Marker::None,|i| if self.author.take.is_hard_cut(CAMERA,i,false) {Marker::Clip} else {Marker::Key})
    }
    fn refresh_cursor(&mut self,_force:bool) {
        self.screen(999);
        // 8264e2c0: instant replay never disables its A/Edit Timeline button.
        // OutOfBounds and None are distinct backend marker states.
        if self.instant || self.mode==Mode::Instant {return;}
        let marker=if !self.valid_cursor() {3} else if self.mode==Mode::Play {0}
            else {match self.marker() {Marker::None=>0,Marker::Clip=>1,Marker::Key=>2}};
        let changed=self.marker!=Some(marker);
        self.marker=Some(marker);
        if changed {
            self.screen(match marker {
                1=>1006, 2=>1009,
                0 if self.mode!=Mode::Play=>1007,
                _=>1008,
            });
        }
    }
    /// SetCurrentEditorMode8256e008 commits camera endpoints on leaving edit,
    /// except when opening help, which must retain the active edit.
    fn set_mode(&mut self,mode:Mode) -> Result<()> {
        self.marker=None;
        if self.mode!=mode {
            if self.mode==Mode::Play { self.sound(0x804e3ce1966c1795); }
            if self.mode.editing_camera() && mode!=Mode::Help { self.author.end_setup()?; }
        }
        self.mode=mode;
        match mode {
            Mode::Play=>{
                self.sound(0x804e3ce1966c1795);
                if self.author.cursor_frame()>=self.author.end_frame() {
                    // ResetPlayhead8255df30's sound precedes MoveCursorTo.
                    self.sound(0xbcf3916a2278df32);self.author.take.set_parameter(self.author.start);
                }
            }
            Mode::Instant=>self.author.reset_instant()?,
            Mode::Effects=>{
                self.speed=self.author.take.value(element::SPEED) as u32;
                self.shake=self.shake_at_cursor();
                // The native mode setter initializes row0; APT SetState4 then
                // calls GetCurrentCameraEditIndex82605110 ->82567910, which
                // writes the current camera's row back into the backend.
                self.selection=match (self.author.current_camera(),self.film_pack&&self.author.has_soft_keys()) {
                    (CameraKind::Follow,false)=>0,(CameraKind::Tripod,false)=>1,(CameraKind::Game,_)=>2,
                    (CameraKind::Free,false)=>3,(CameraKind::Follow,true)=>4,(CameraKind::Tripod,true)=>5,(CameraKind::Free,true)=>6,
                };
            }
            _=>()
        }
        Ok(())
    }
    fn return_previous(&mut self) -> Result<()> {
        self.set_mode(if self.instant {Mode::Instant} else {Mode::Timeline})?;
        self.screen(19);self.refresh_cursor(false);Ok(())
    }
    fn show(&mut self,show:bool) {
        if show {self.refresh_cursor(false);}
        self.screen(if show {1002} else {1003});self.visible=show;
    }
    fn help(&mut self) -> Result<()> {
        self.previous_help=self.mode;self.sound(0x648a6811d713694e);
        self.set_mode(Mode::Help)?;self.screen(15);Ok(())
    }
    fn notice(&mut self,title:&str,description:&str) {
        self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
        self.popup=Some((Dialog::Notice,Popup{title:title.into(),description:description.into(),options:vec!["ID_COMMON_OK".into()],selected:0,cancellable:false}));
    }
    fn confirm(&mut self,dialog:Dialog,title:&str,description:&str) {
        self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
        self.popup=Some((dialog,Popup{title:title.into(),description:description.into(),options:vec!["ID_COMMON_NO".into(),"ID_COMMON_YES".into()],selected:0,cancellable:false}));
    }
    /// PC port addition: one of two export settings, opened on the current one.
    fn choose(&mut self,dialog:Dialog,description:&str,options:[&str;2],selected:usize) {
        self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
        self.popup=Some((dialog,Popup{title:"ID_REPLAYEDITOR_BASIC_SAVE_REPLAY".into(),description:description.into(),
            options:options.map(String::from).to_vec(),selected,cancellable:false}));
    }
    fn limit_notice(&mut self) { self.sound(0x47f720222f2caed4);self.notice("ID_REPLAY_SAVEWARNING_TITLE","ID_REPLAY_EDITOR_NO_MORE_MARKERS"); }
    fn menu(&mut self) {
        self.sound(0x8eec5b3d73920fa7);
        self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
        let mut options=vec!["ID_REPLAYEDITOR_RETURNTOEDITOR".into(),"ID_REPLAYEDITOR_EXIT".into()];
        self.menu_actions=vec![Action::None,Action::Exit];
        if self.signed_in {
            if self.encoded_seconds()<=60 {options.push("ID_REPLAYEDITOR_UPLOAD".into());self.menu_actions.push(Action::SignInRequired);}
            options.push("ID_REPLAYEDITOR_BASIC_SAVE_REPLAY".into());self.menu_actions.push(Action::SaveReplay);
        }
        self.popup=Some((Dialog::Menu,Popup{title:"ID_REPLAYEDITOR_TITLE".into(),description:"#".into(),options,selected:0,cancellable:false}));
    }
    fn photo(&mut self,tuning:Tuning) -> Result<()> {
        if tuning.frame_step {return self.set_mode(Mode::FrameStep);}
        self.reset_deltas();
        if self.signed_in && !self.saving {
            self.sound(0xe28db5d65b9c9313);
            self.confirm(Dialog::SavePhoto,"ID_REPLAYEDITOR_TAKE_PHOTO","ID_REPLAY_EDITOR_POPUP_PHOTOSAVE");
        }
        Ok(())
    }
    fn basic_effects(&mut self) -> Result<()> { self.author.set_speed(self.speed)?;self.author.set_shake(self.shake) }
    fn shake_at_cursor(&self) -> u8 {
        let i=self.author.take.current_interval(SHAKE).unwrap_or(0);
        let kind=self.author.take.key_value(element::SHAKE_TYPE,i) as i32;
        let amplitude=self.author.take.key_value(element::SHAKE_AMPLITUDE,self.author.take.key_index(SHAKE,i));
        match kind {3 if amplitude<1.=>1,3=>2,4=>3,_=>0}
    }
    /// GetEditSelection8255e5e0: APT list positions have Game before Free.
    fn selected_camera(&self) -> Option<(CameraKind,bool)> {
        match self.selection {
            0=>Some((CameraKind::Follow,false)),1=>Some((CameraKind::Tripod,false)),2=>Some((CameraKind::Game,false)),
            3 if self.film_pack=>Some((CameraKind::Free,false)),
            4=>Some((CameraKind::Follow,true)),5=>Some((CameraKind::Tripod,true)),6=>Some((CameraKind::Free,true)),_=>None
        }
    }
    fn select_camera(&mut self,spaces:&Spaces) -> Result<()> {
        let Some((camera,advanced))=self.selected_camera() else {return Ok(())};
        if advanced && self.marker_limit() {self.limit_notice();return Ok(());}
        let different=self.author.current_camera()!=camera;
        let overwrite=different||(!advanced&&self.author.has_soft_keys());
        if !overwrite || (self.author.take.interval_count(CAMERA)==1&&self.author.current_camera()==CameraKind::Game) {
            self.enable_camera(camera,advanced,overwrite,false,spaces)?;
        } else {self.confirm(Dialog::Overwrite,"ID_REPLAY_EDITOR_REPLACE_CAMERA_TITLE","ID_REPLAY_EDITOR_REPLACE_CAMERA_DESC");}
        Ok(())
    }
    fn enable_camera(&mut self,camera:CameraKind,advanced:bool,overwrite:bool,edit:bool,spaces:&Spaces) -> Result<()> {
        if overwrite&&!edit {
            self.author.clear_soft_keys(true,false)?;
            if self.author.setup_camera(camera,false,false,spaces)? {
                self.author.reset_camera_settings()?;self.author.end_setup()?;
            }
            if camera!=CameraKind::Game {self.reset_collision=true;}
        }
        if self.author.setup_camera(camera,advanced,edit,spaces)? {
            let (mode,state)=match (camera,advanced) {
                (CameraKind::Follow,false)=>(Mode::Follow,1),(CameraKind::Follow,true)=>(Mode::Follow,12),
                (CameraKind::Tripod,false)=>(Mode::Tripod,2),(CameraKind::Tripod,true)=>(Mode::Tripod,13),
                (CameraKind::Free,false)=>(Mode::Free,7),(CameraKind::Free,true)=>(Mode::Free,14),
                (CameraKind::Game,_)=>(Mode::Game,10)
            };
            self.height_delta=0.;self.set_mode(mode)?;self.screen(state);
        }
        self.refresh_cursor(false);Ok(())
    }
    fn change_setting(&mut self,increase:bool) {
        match if self.film_pack {self.selection} else if self.selection==3 {7} else {self.selection} {
            7=>self.speed=if increase {(self.speed+10).min(100)} else {self.speed.saturating_sub(10).max(10)},
            8=>self.shake=if increase {(self.shake+1)%4} else {(self.shake+3)%4},
            9=>self.watermark=if increase {(self.watermark+1)%3} else {(self.watermark+2)%3},
            _=>()
        }
        self.screen(6);
    }
    fn popup_input(&mut self,key:u8,spaces:&Spaces) -> Result<()> {
        let Some((dialog,popup))=self.popup.as_mut() else {return Ok(())};
        if key==12||key==13 {
            let old=popup.selected;
            popup.selected=(popup.selected+if key==12 {popup.options.len()-1} else {1})%popup.options.len();
            if old!=popup.selected {self.sounds.push(FeSoundRequest::Play(if key==12 {"FE_SOUND_CORE_NAV_UP"} else {"FE_SOUND_CORE_NAV_DOWN"}));}
            return Ok(());
        }
        // sPopupInfo defaults do not enable cancellation for these dialogs.
        if key!=2 {return Ok(());}
        let (dialog,index)=(*dialog,popup.selected);self.popup=None;
        self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_A_BUTTON"));
        match dialog {
            Dialog::MediaFull(replay) if index==0=>{self.saving=true;self.pending=Action::Browse(replay);},
            Dialog::MediaDelete(replay,slot)=>self.confirm(Dialog::MediaDeleteConfirm(replay,slot),"ID_COMMON_WARNING_TITLE",
                if replay {"ID_MEMCARD_PROMPT_DELETE_REPLAY"} else {"ID_MEMCARD_PROMPT_DELETE_PHOTO"}),
            Dialog::MediaDeleteConfirm(replay,slot) if index==1=>{self.saving=true;self.pending=Action::DeleteMedia(replay,slot);},
            Dialog::Menu=>match self.menu_actions[index] {
                Action::None=>self.return_previous()?,
                Action::Exit=>self.confirm(Dialog::Exit,"ID_START_HUB_SKATE_REPLAY_EDITOR","ID_REPLAY_EDITOR_QUIT"),
                Action::SaveReplay=>self.confirm(Dialog::SaveReplay,"ID_REPLAYEDITOR_BASIC_SAVE_REPLAY","ID_REPLAY_EDITOR_POPUP_REPLAYSAVE"),
                Action::SignInRequired=>self.confirm(Dialog::Upload,"ID_REPLAYEDITOR_UPLOAD","ID_REPLAY_EDITOR_POPUP_UPLOAD"),_=>()
            },
            Dialog::Exit if index==1=>self.pending=Action::Exit,
            Dialog::Upload if index==1=>self.pending=Action::SignInRequired,
            Dialog::SaveReplay if index==1=>self.choose(Dialog::Resolution,"#Choose the video's resolution.",
                ["#720p","#1080p"],usize::from(self.video.height==1080)),
            Dialog::Resolution=>{
                self.video.height=if index==0 {720} else {1080};
                self.choose(Dialog::FrameRate,"#Choose the video's frame rate.",["#30 FPS","#60 FPS"],usize::from(self.video.fps==60));
            }
            Dialog::FrameRate=>{
                self.video.fps=if index==0 {30} else {60};
                self.saving=true;self.pending=Action::SaveReplay;self.return_previous()?;
            }
            Dialog::SavePhoto if index==1=>{self.saving=true;self.pending=Action::SavePhoto;}
            Dialog::Insert if index==1=>{self.author.insert_clip();self.refresh_cursor(false);}
            Dialog::Delete(reset) if index==1=>{
                if let Some(i)=self.marker_index() {self.author.delete_marker(i,reset,spaces)?;}
                self.refresh_cursor(false);
            }
            Dialog::Overwrite if index==1=>{
                if let Some((camera,advanced))=self.selected_camera() {self.enable_camera(camera,advanced,true,false,spaces)?;}
            }
            _=>()
        }
        Ok(())
    }

    /// FE::ReceiveInput82682b68. Native buttons use both pressed and released
    /// events: in particular timeline clips/keys act on the D-pad release.
    fn key(&mut self,key:u8,tuning:Tuning,spaces:&Spaces) -> Result<()> {
        if self.popup.is_some() {return self.popup_input(key,spaces);}
        match self.mode {
            Mode::Play=>match key {
                3=>{self.return_previous()?;self.show(true);self.key(3,tuning,spaces)?;}
                6|7|10=>self.return_previous()?,
                11=>self.show(!self.visible),_=>()
            }
            Mode::Trim=>if key==53 {self.set_mode(Mode::Timeline)?;self.cursor_visible=true;self.screen(1004);},
            Mode::Instant|Mode::Timeline=>{
                if self.request_camera {return Ok(());}
                if key==11 {self.show(!self.visible);return Ok(());}
                if key==6 {self.set_mode(Mode::Play)?;self.screen(18);return Ok(());}
                if key==3 {if !self.visible {self.screen(19);self.show(true);}self.menu();return Ok(());}
                if !self.visible {return Ok(());}
                match key {
                    0=>self.help()?,5=>self.photo(tuning)?,
                    2 if self.mode==Mode::Instant=>{
                        self.instant=false;self.enable_timeline_render();self.set_mode(Mode::Timeline)?;self.screen(9);self.sound(0xa1e6d757dffaa0fe);self.refresh_cursor(true);
                    }
                    2=>{
                        if self.marker()==Marker::Key {
                            let i=self.marker_index().ok_or("replay soft marker lost its interval")?;
                            self.author.take.set_parameter(self.author.take.interval_parameter(CAMERA,i)+0.0001);
                            self.request_camera=true;self.refresh_cursor(false);
                        } else if self.marker_index().is_none()&&self.valid_cursor() {self.set_mode(Mode::Effects)?;self.screen(4);}
                        else {self.sound(0x47f720222f2caed4);}
                    }
                    9 if self.mode==Mode::Timeline=>{self.cursor_visible=false;self.screen(1005);self.set_mode(Mode::Trim)?;}
                    32 if self.mode==Mode::Timeline=>match self.marker() {
                        Marker::Clip=>{self.sound(0x9def3bccef922b96);self.confirm(Dialog::Delete(true),"ID_REPLAY_EDITOR_DELETE_CLIP","ID_REPLAY_EDITOR_DELETE_CLIP_DESC");}
                        Marker::Key=>{self.sound(0x9def3bccef922b96);self.confirm(Dialog::Delete(false),"ID_REPLAY_EDITOR_DELETE_KEY","ID_REPLAY_EDITOR_DELETE_KEY_DESC");}
                        Marker::None=>self.sound(0x47f720222f2caed4)
                    }
                    33 if self.mode==Mode::Timeline=>{
                        if self.marker_index().is_some()||!self.valid_cursor() {self.sound(0x47f720222f2caed4);}
                        else if self.marker_limit() {self.limit_notice();}
                        else {
                            self.sound(0xee45aa440cebd1e8);
                            if self.author.has_soft_keys() {self.confirm(Dialog::Insert,"ID_REPLAY_SAVEWARNING_TITLE","ID_REPLAY_EDITOR_POPUP_CLIPMARKER");}
                            else {self.author.insert_clip();}self.refresh_cursor(false);
                        }
                    }
                    18 if self.mode==Mode::Instant=>self.previous_key(),
                    19 if self.mode==Mode::Instant=>self.next_key(),
                    34 if self.mode==Mode::Timeline=>self.previous_key(),
                    35 if self.mode==Mode::Timeline=>self.next_key(),_=>()
                }
            }
            Mode::Follow|Mode::Tripod|Mode::Game|Mode::Free=>match key {
                0=>self.help()?,2=>{self.screen(9);self.set_mode(Mode::Timeline)?;self.refresh_cursor(false);},_=>()
            }
            Mode::Effects=>match key {
                0=>self.help()?,2=>self.select_camera(spaces)?,
                3=>{self.basic_effects()?;self.set_mode(Mode::Timeline)?;self.screen(9);self.refresh_cursor(true);}
                12=>{self.selection=if self.selection==0 {if self.film_pack {9} else {3}} else {self.selection-1};self.screen(5);}
                13=>{self.selection=(self.selection+1)%if self.film_pack {10} else {4};self.screen(5);}
                14=>self.change_setting(false),15=>self.change_setting(true),_=>()
            }
            Mode::Help=>match key {2=>self.screen(16),3=>{self.set_mode(self.previous_help)?;self.screen(17);},_=>()},
            Mode::FrameStep=>match key {
                5=>self.return_previous()?,
                7|10=>{self.author.move_cursor(if key==7 {0.5} else {-0.5}/(self.author.take.length()*30.));self.refresh_cursor(false);},_=>()
            }
        }
        Ok(())
    }
    fn previous_key(&mut self) {self.sound(0x234fcd1ad7c5d294);self.author.jump_previous();self.refresh_cursor(false);}
    fn next_key(&mut self) {self.sound(0x939629816845cade);self.author.jump_next();self.refresh_cursor(false);}

    /// Native input updates are frontend ticks; playback alone uses elapsed
    /// time. Rendering at 120/240Hz must not multiply the scrub/camera deltas.
    pub fn update(&mut self,input:&Input,seconds:f32,focused:bool,tuning:Tuning,spaces:&Spaces) -> Result<Action> {
        self.threshold=tuning.marker_threshold;
        if !self.ready||self.saving {return Ok(Action::None);}
        if !focused {self.reset_deltas();return Ok(Action::None);}
        if self.footage.is_some() {return self.footage_input(input,spaces);}
        if self.request_camera&&self.popup.is_none() {
            self.request_camera=false;
            let camera=self.author.current_camera();
            if camera!=CameraKind::Game {self.enable_camera(camera,true,false,true,spaces)?;}
        }
        let mut allow_analog=true;
        for &key in &input.keys {
            // ReceiveInput/Process*Input only clear mbAllowInput on these
            // branches. D-pad repeats during camera movement must not cancel it.
            allow_analog &= !match self.mode {
                Mode::Play=>matches!(key,6|11),
                Mode::Instant=>matches!(key,11|18|19),
                Mode::Timeline=>matches!(key,32..=35)||(key==5&&!tuning.frame_step),
                Mode::Follow|Mode::Tripod|Mode::Game|Mode::Free=>matches!(key,0|2),
                Mode::Effects=>matches!(key,0|2|3|12..=15),
                Mode::Help=>matches!(key,2|3),
                _=>false,
            };
            self.key(key,tuning,spaces)?;
            if self.saving||self.pending!=Action::None {break;}
        }
        if self.popup.is_some()||self.saving {
            self.reset_deltas();if self.mode==Mode::Play {self.return_previous()?;}
        } else if allow_analog&&!self.request_camera {
            match self.mode {
                Mode::Instant|Mode::Timeline=>{
                    smooth(&mut self.trigger_velocity,input.triggers[1]-input.triggers[0],tuning);
                    self.author.move_cursor(self.trigger_velocity);
                    if self.trigger_velocity!=0. {self.refresh_cursor(false);}
                }
                Mode::Trim=>{
                    smooth(&mut self.head_velocity[0],input.left[0],tuning);
                    if self.head_velocity[0]!=0. {self.author.move_start(self.head_velocity[0]);self.screen(1000);}
                    else {smooth(&mut self.head_velocity[1],input.right[0],tuning);
                        if self.head_velocity[1]!=0. {self.author.move_end(self.head_velocity[1]);self.screen(1001);}}
                }
                Mode::Follow|Mode::Tripod=>{
                    self.author.orbit_position(limit(input.right[0]*tuning.orbit_multiplier,tuning.orbit_rs),
                        limit(input.right[1]*tuning.orbit_multiplier,tuning.orbit_rs),-limit(input.left[1],tuning.orbit_ls),spaces)?;
                    self.zoom(input,tuning)?;
                    // FE8267a844 uses literal .01, not the registered orbit height cvar.
                    self.author.height(if input.dpad[1] {-0.01} else if input.dpad[0] {0.01} else {0.},spaces)?;
                }
                Mode::Game=>self.zoom(input,tuning)?,
                Mode::Free=>{
                    if input.dpad[0] {self.height_delta+=tuning.free_height;}
                    if input.dpad[1] {self.height_delta-=tuning.free_height;}
                    self.height_delta=limit(self.height_delta,0.2);
                    self.author.free_position(limit(input.right[0]*tuning.orbit_multiplier,tuning.free_ls)*2.,
                        limit(input.right[1]*tuning.orbit_multiplier,tuning.free_ls)*2.,
                        limit(input.left[0],tuning.free_rs),limit(input.left[1],tuning.free_rs),spaces)?;
                    self.zoom(input,tuning)?;self.author.free_height(self.height_delta,spaces)?;self.height_delta*=0.6;
                }
                _=>()
            }
        } else {self.reset_deltas();}
        if self.mode==Mode::Play&&self.popup.is_none()&&!self.saving {
            self.author.advance(seconds);self.refresh_cursor(false);
            if self.author.cursor_frame()>=self.author.end_frame() {self.return_previous()?;}
        }
        Ok(std::mem::replace(&mut self.pending,Action::None))
    }
    fn zoom(&mut self,input:&Input,tuning:Tuning) -> Result<()> {
        let delta=if input.dpad[2] {-tuning.zoom_delta} else if input.dpad[3] {tuning.zoom_delta} else {0.};
        if delta!=0. {self.author.zoom(delta)?;self.screen(11);}Ok(())
    }
    /// 8255e6f0: duration includes each clip's speed and 30Hz frame rounding.
    pub fn encoded_seconds(&self) -> u32 {
        let mut frames=0u32;
        for i in 0..self.author.take.interval_count(CAMERA) {
            let frame=|n| (self.author.take.interval_parameter(CAMERA,n)*self.author.take.length()*30.+0.0005) as i32;
            let (start,end)=(frame(i),frame(i+1));
            if end>=self.author.start_frame()&&start<=self.author.end_frame() {
                let speed=self.author.take.key_value(element::SPEED,self.author.take.key_index(SPEED,i));
                frames=frames.wrapping_add(((end.min(self.author.end_frame())-start.max(self.author.start_frame())) as f32/(speed*0.01)) as u32);
            }
        }
        (frames as f32*(1./30.)+0.5) as u32
    }
    pub fn set_stalled(&mut self,stalled:bool) {
        if self.stalled!=stalled {self.stalled=stalled;self.screen(if stalled {1010} else {1011});}
    }
    pub fn save_complete(&mut self,replay:bool,_first_replay_save:bool,path:&str) {
        self.saving=false;
        // Intentional PC wording: these files are local media, not Xbox/EA uploads.
        let mut display=String::new();let mut line=0;
        for component in path.split_inclusive(['\\','/']) {
            let length=component.chars().count();
            if line>0&&line+length>42 {display.push('\n');line=0;}
            display.push_str(component);line+=length;
        }
        self.notice(if replay {"#Replay saved"} else {"#Photo saved"},&format!("#Find your {} in the saves folder:\n{display}",if replay {"MP4 video"} else {"photo"}));
    }
    pub fn export_progress(&mut self,progress:f32,finalizing:bool) {
        let description=if finalizing {"#Finishing your MP4 video...".to_owned()}
            else {format!("#Rendering your video: {}%\n{}p / {} FPS",(progress.clamp(0.,1.)*100.) as u32,self.video.height,self.video.fps)};
        self.popup=Some((Dialog::Notice,Popup {title:"#Saving replay".into(),description,options:vec!["ID_COMMON_CANCEL".into()],selected:0,cancellable:true}));
    }
    pub fn save_cancelled(&mut self) {self.saving=false;self.popup=None;}
    pub fn save_failed(&mut self,title:&str,description:&str) {self.saving=false;self.notice(title,&format!("#{description}"));}
    pub fn sign_in_notice(&mut self,title:&str,description:&str) {self.notice(title,description);}
    pub fn snapshot(&self) -> Snapshot {
        let i=self.author.take.current_interval(SHAKE).unwrap_or(0);
        Snapshot {mode:self.mode,render_timeline:self.render_timeline,visible:self.visible,cursor_visible:self.cursor_visible,ready:self.ready,
            film_pack:self.film_pack,signed_in:self.signed_in,parameter:self.author.take.parameter(),start_head:self.author.start,end_head:self.author.end,cursor:self.author.cursor_frame(),start:self.author.start_frame(),end:self.author.end_frame(),
            frames:self.author.frame_count(),encoded_seconds:self.encoded_seconds(),video:self.video,edit_selection:self.selection,
            speed:self.speed,shake:self.shake,watermark:self.watermark,zoom:self.author.take.value(element::LENS),
            marker:self.marker(),marker_index:self.marker_index(),
            intervals:(0..self.author.take.interval_count(CAMERA)).map(|i| Interval {
                start:self.author.take.interval_parameter(CAMERA,i),end:self.author.take.interval_parameter(CAMERA,i+1),
                camera:self.author.camera_kind(i),hard:self.author.take.is_hard_cut(CAMERA,i,false)
            }).collect(),selected_clip:[self.author.take.interval_parameter(SHAKE,i),self.author.take.interval_parameter(SHAKE,i+1)],
            popup:self.popup.as_ref().map(|(_,p)|p.clone()),saving:self.saving,stalled:self.stalled,footage:self.footage.clone()}
    }
}

fn limit(value:f32,max:f32) -> f32 { if value>=max {max} else if value<=-max {-max} else {value} }
fn smooth(velocity:&mut f32,input:f32,tuning:Tuning) {
    let max=tuning.scrub_max*0.005;let target=max*input;
    *velocity=limit((target-*velocity)*tuning.scrub_smoothing+*velocity,max);
    if target.abs()<0.00001&&velocity.abs()<=0.0001 {*velocity=0.;}
}
