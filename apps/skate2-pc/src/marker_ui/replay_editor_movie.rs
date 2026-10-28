//! Original replay_editor2.apt (9cb595f1), screen15. The movie supplies all
//! artwork, layout, timelines and help text. This translates its own scripts.
use super::{assets::Assets, clips::Clip, pause_movie::child, map_movie,
    edit_skater_movie, progression_popup::ProgressionPopup, localizer};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::{edit_skater::EditSkaterRow,replay_editor::{Snapshot,Mode}};
pub(super) const ASSET:usize=93;
const ROWS:[(&str,&str);10]=[
    ("mFollowCam","ID_REPLAYEDITOR_FOLLOW_CAMERA"),("mTripodCam","ID_REPLAYEDITOR_TRIPOD_CAMERA"),
    ("mDefaultCam","ID_REPLAYEDITOR_DEFAULT_CAMERA"),("mFreeCam","ID_REPLAYEDITOR_FREE_CAMERA"),
    ("mAdvFollowCam","ID_REPLAYEDITOR_ADVANCED_FOLLOW_CAMERA"),("mAdvTripodCam","ID_REPLAYEDITOR_ADVANCED_TRIPOD_CAMERA"),
    ("mAdvFreeCam","ID_REPLAYEDITOR_ADVANCED_FREE_CAMERA"),("mSpeed","ID_REPLAYEDITOR_EFFECTS_SPEED"),
    ("mShake","ID_REPLAYEDITOR_SHAKE"),("mWatermark","ID_REPLAYEDITOR_WATERMARK")];
pub(super) struct ReplayEditorMovie {
    pub root:Clip,
    pub popup:ProgressionPopup,
    shown:Option<Snapshot>,
    pending:bool,
    intro:bool,
    editor_ready:bool,
    outro:bool,
    destroy:bool,
    closing:bool,
    mode:u32,
    page:u8,
    pages:u8,
    source_y:f32,
    cursor_offset:f32,
    head_offset:f32,
    over_key:bool,
    playing:bool,
    queued:Vec<u32>,
}
impl ReplayEditorMovie {
    pub fn new(assets:&Assets)->RenderResult<Self> {
        let mut root=Clip::new(assets,ASSET,0)?;root.visible=false;root.playback.stop();
        Ok(Self {root,popup:Default::default(),shown:None,pending:false,intro:false,editor_ready:false,outro:false,destroy:false,closing:false,
            mode:0,page:0,pages:0,source_y:0.,cursor_offset:0.,head_offset:0.,over_key:false,playing:false,queued:Vec::new()})
    }
    pub fn pending(&self)->bool {self.pending}
    pub fn open(&self)->bool {self.shown.is_some()}
    pub fn take_intro_complete(&mut self)->bool {std::mem::take(&mut self.intro)}
    pub fn take_editor_ready(&mut self)->bool {std::mem::take(&mut self.editor_ready)}
    pub fn take_outro_complete(&mut self)->bool {std::mem::take(&mut self.outro)}
    pub fn update(&mut self,assets:&Assets,state:Option<&Snapshot>,states:&[u32])->RenderResult<()> {
        if state.is_some()&&self.shown.is_none() {self.pending=true;self.intro=false;self.editor_ready=false;self.queued.clear();}
        if state.is_none()&&self.shown.is_some() {
            if !std::mem::take(&mut self.pending) {self.closing=true;self.root.goto(assets,"outro")?;}
            self.queued.clear();
        }
        self.shown=state.cloned();
        if self.pending {self.queued.extend_from_slice(states);}
        else if state.is_some() {
            for &state in states {self.set_state(assets,state)?;}
            self.publish_timeline()?;
        }
        let dialog=state.and_then(|s|s.popup.as_ref());
        self.popup.show(assets,dialog.map(|p|(p.title.clone(),p.description.clone(),p.options.clone(),p.selected,p.cancellable)))?;
        self.callbacks(assets)
    }
    pub fn link(&mut self,assets:&Assets)->RenderResult<()> {
        if !std::mem::take(&mut self.pending) {return Ok(());}
        self.root=Clip::new(assets,ASSET,0)?;self.destroy=false;self.closing=false;self.outro=false;self.mode=0;self.over_key=false;self.playing=false;
        self.source_y=child(&mut self.root,"mReplay")?.entry.matrix.ty;
        let replay=self.root.named(&["mReplay"])?;
        self.cursor_offset=child(replay,"mPlayhead")?.entry.matrix.tx;
        self.head_offset=child(replay,"mReplayIn")?.entry.matrix.tx;
        for name in ["mPlayhead","mReplayIn","mReplayOut"] {replay.named(&[name])?.visible=false;}
        let film=self.shown.as_ref().ok_or("Replay screen linked without state")?.film_pack;
        let effects=replay.named(&["mEffects"])?;
        effects.goto(assets,if film {"advanced"} else {"basic"})?;
        let title=if film {"ID_REPLAYEDITOR_ADVANCED_EDIT_MENU"} else {"ID_REPLAYEDITOR_BASIC_EDIT_MENU"};
        label_pair(effects.named(&["mEffectsTitle"])?,assets,title)?;
        label(effects.named(&["mFilmerText","labelText"])?,assets,if film {"#"} else {"ID_REPLAYEDITOR_DLC"})?;
        for (index,(name,title)) in ROWS.iter().enumerate() {
            let row=EditSkaterRow {kind:if index<7 {"option"} else {"selector"}.into(),title:assets.localize(title)?,..Default::default()};
            edit_skater_movie::highlight(effects.named(&[name])?,assets,&row,false)?;
        }
        buttons(effects.named(&["mButton"])?,assets,&[("Start","ID_REPLAYEDITOR_HELP"),("B","ID_COMMON_BACK"),("A","ID_COMMON_SELECT")])?;
        for (name,key) in [
            ("mPlay","ID_REPLAY_EDITOR_HELP_PLAYBACK_PLAYPAUSE_DESC"),("mSkipStart","ID_REPLAY_EDITOR_HELP_PLAYBACK_SKIP_TO_START_DESC"),
            ("mScrubback","ID_REPLAY_EDITOR_HELP_PLAYBACK_REWIND_DESC"),("mSkipEnd","ID_REPLAY_EDITOR_HELP_PLAYBACK_SKIP_TO_END_DESC"),
            ("mScrubForward","ID_REPLAY_EDITOR_HELP_PLAYBACK_FORWARD_DESC"),("mScrubTool","ID_REPLAY_EDITOR_HELP_PLAYBACK_PLAYHEAD_DESC"),
            ("mMarker","ID_REPLAY_EDITOR_HELP_MARKERS_CLIP_DESC"),("mPlace","ID_REPLAY_EDITOR_HELP_MARKERS_INSERT_CLIP_DESC"),
            ("mInPoint","ID_REPLAY_EDITOR_HELP_MARKERS_START_MARKER_DESC"),("mDelete","ID_REPLAY_EDITOR_HELP_MARKERS_DELETE_CLIP_DESC"),
            ("mOutPoint","ID_REPLAY_EDITOR_HELP_MARKERS_END_MARKER_DESC"),("mSkipPrev","ID_REPLAY_EDITOR_HELP_MARKERS_PREVIOUS_CLIP_DESC"),
            ("mKey","ID_REPLAY_EDITOR_HELP_MARKERS_KEY_DESC"),("mSkipNext","ID_REPLAY_EDITOR_HELP_MARKERS_NEXT_CLIP_DESC"),
            ("mCamRotate","ID_REPLAY_EDITOR_HELP_CAMERAS__ROTATE_DESC"),("mCamInOut","ID_REPLAY_EDITOR_HELP_CAMERAS_LEFTRIGHT_DESC"),
            ("mCamUpDown","ID_REPLAY_EDITOR_HELP_CAMERAS_UPDOWN_DESC"),("mCamZoom","ID_REPLAY_EDITOR_HELP_CAMERAS_ZOOM_DESC")
        ] {label(replay.named(&["mHelp",name,"labelText"])?,assets,key)?;}
        self.set_state(assets,0)?;
        self.show_hide(assets,true)?;
        self.root.goto(assets,"intro")?;
        for state in std::mem::take(&mut self.queued) {self.set_state(assets,state)?;}
        self.publish_timeline()?;
        self.callbacks(assets)
    }
    pub fn tick(&mut self,assets:&Assets)->RenderResult<()> {
        if std::mem::take(&mut self.destroy) {self.root.visible=false;self.closing=false;}
        if self.root.visible||self.closing {self.root.tick(assets)?;}
        self.popup.tick(assets)?;
        self.callbacks(assets)
    }
    fn callbacks(&mut self,assets:&Assets)->RenderResult<()> {
        let mut calls=Vec::new();self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "ReplayConstruct"=>(),
                "ReplayIntroComplete"=>self.intro=true,
                "ReplayOutroComplete"=>{self.outro=true;self.destroy=true;},
                "ReplayInstantDone"=>{
                    if self.shown.as_ref().is_some_and(|s|s.mode!=Mode::Instant&&self.mode==0) {
                        // APT InstantDone calls UpdateReplayEditorReady only
                        // for a saved replay, after its instant-title animation.
                        self.editor_ready=true;self.set_state(assets,9)?;
                    }
                }
                _=>return Err(format!("untranslated replay callback {call}").into()),
            }
        }
        Ok(())
    }
    fn show_hide(&mut self,assets:&Assets,show:bool)->RenderResult<()> {
        self.root.named(&["mHide"])?.visible=show;
        self.root.named(&["mHideText"])?.visible=show;
        let visible=self.shown.as_ref().is_none_or(|s|s.visible);
        label_pair(self.root.named(&["mHideText"])?,assets,if visible {"ID_REPLAY_EDITOR_HIDE"} else {"ID_REPLAY_EDITOR_SHOW"})
    }
    fn timeline(&mut self,enable:bool)->RenderResult<()> {
        let replay=self.root.named(&["mReplay"])?;
        for name in ["mCustomRender","mTimeline"] {alpha(replay,name,if enable {255} else {0})?;}
        for name in ["mPlayhead","mReplayIn","mReplayOut"] {replay.named(&[name])?.visible=enable;}
        Ok(())
    }
    fn length(&mut self,assets:&Assets)->RenderResult<()> {
        let state=self.shown.as_ref().ok_or("replay state absent")?;
        let time=format!("{} {}",state.encoded_seconds,assets.localize("ID_COMMON_METRIC_TIME")?);
        let text=String::from_utf16_lossy(&localizer::format_string_text(assets,"ID_REPLAY_EDITOR_LENGTH",&[time])?);
        let view=self.root.named(&["mReplay","mTextTime"])?;
        raw_pair(view,&text)?;
        view.named(&["labelText"])?.properties.insert("text_colour".into(),(0xff00_0000u32|if state.encoded_seconds<=60 {0xff_ffff} else {7_935_233}).to_string());
        view.named(&["labelTextGlow"])?.properties.insert("text_colour".into(),0xff00_0000u32.to_string());
        Ok(())
    }
    pub fn set_state(&mut self,assets:&Assets,state:u32)->RenderResult<()> {
        let Some(snapshot)=self.shown.clone() else {return Ok(())};
        match state {
            999|1000|1001=>{
                // APT GetCursorOffset/GetStartHead/GetEndHead use the same
                // authored817px span, each with its own normalized position.
                let (name,base)=match state {999=>("mPlayhead",self.cursor_offset),1000=>("mReplayIn",self.head_offset),_=>("mReplayOut",self.head_offset)};
                let entry=child(self.root.named(&["mReplay"])?,name)?;
                let parameter=match state {1000=>snapshot.start_head,1001=>snapshot.end_head,_=>snapshot.parameter};
                entry.entry.matrix.tx=base+parameter*817.;entry.clip.script_transform=true;
                if state!=999 {self.length(assets)?;}
            }
            1002|1003=>{
                let replay=child(&mut self.root,"mReplay")?;
                replay.entry.matrix.ty=if state==1002 {self.source_y} else {1100.};
                replay.clip.script_transform=true;
                replay.clip.named(&["mPlayhead"])?.visible=state==1002;
                self.show_hide(assets,matches!(snapshot.mode,Mode::Instant|Mode::Timeline|Mode::Play))?;
            }
            1004|1005=>self.root.named(&["mReplay","mPlayhead"])?.visible=state==1004,
            1006..=1009=>{
                self.over_key=matches!(state,1006|1009);
                let replay=self.root.named(&["mReplay"])?;
                // The authored effects frame29 removes depths53/59. Camera
                // timelines inherit that absence, but RefreshCursor8264e2c0
                // still sends these states. APT SetMember82a0f160 does nothing
                // when GetMember82a0ef30 returned undefined for a removed clip.
                alpha_if_present(replay,"mcDropMarker",if state==1007 {255} else {102})?;
                alpha_if_present(replay,"mcRemoveMarker",if self.over_key {255} else {102})?;
                enable_button(replay.named(&["mButtonMenu"])?,if snapshot.signed_in {3} else {2},matches!(state,1007|1009))?;
            }
            1010|1011=>alpha(self.root.named(&["mReplay"])?,"mBlack",if state==1010 {255} else {0})?,
            0|9=>{
                self.mode=state;self.playing=false;self.show_hide(assets,true)?;
                if state==9 {self.timeline(true)?;self.set_state(assets,1002)?;}
                let replay=self.root.named(&["mReplay"])?;
                replay.goto(assets,if state==0 {"instant"} else {"main"})?;
                replay.named(&["mPlayhead"])?.visible=true;
                label(replay.named(&["mTextReplay","labelText"])?,assets,if state==0 {"ID_REPLAYEDITOR_INSTANT_REPLAY"} else {"ID_REPLAYEDITOR_EDIT_TIMELINE"})?;
                let mut items=vec![("Start","ID_REPLAYEDITOR_HELP"),("B","ID_REPLAY_EDITOR_FINISHED")];
                if snapshot.signed_in {items.push(("X","ID_REPLAYEDITOR_TAKE_PHOTO"));}
                items.push(("A",if state==0 {"ID_REPLAYEDITOR_EDIT_TIMELINE"} else {"ID_REPLAYEDITOR_EDIT_CLIP"}));
                buttons(replay.named(&["mButtonMenu"])?,assets,&items)?;
                self.length(assets)?;
            }
            1|2|7|10|12|13|14=>{
                self.mode=state;self.show_hide(assets,false)?;self.timeline(true)?;
                let (timeline,row)=match state {1=>("follow",0),2=>("tripod",1),7=>("freecam",3),10=>("game",2),12=>("follow",4),13=>("tripod",5),_=>("freecam",6)};
                let replay=self.root.named(&["mReplay"])?;replay.goto(assets,timeline)?;
                label(replay.named(&["mTextReplay","labelText"])?,assets,ROWS[row].1)?;
                buttons(replay.named(&["mButtonMenu"])?,assets,&[("Start","ID_REPLAYEDITOR_HELP"),("A","ID_REPLAYEDITOR_COMMIT")])?;
                self.set_state(assets,11)?;
            }
            4=>{
                self.mode=state;self.show_hide(assets,false)?;
                self.root.named(&["mReplay"])?.goto(assets,"effects")?;self.timeline(false)?;
                self.effects(assets)?;
            }
            5|6=>self.effects(assets)?,
            11=>{
                let text=String::from_utf16_lossy(&localizer::format_string_int(assets,"ID_REPLAYEDITOR_ZOOM",snapshot.zoom as i32)?);
                raw_pair(self.root.named(&["mReplay","mTextZoom"])?,&text)?;
            }
            15=>{self.page=0;self.show_hide(assets,false)?;self.root.named(&["mReplay"])?.goto(assets,"help")?;self.timeline(false)?;self.help(assets)?;}
            16=>if self.pages>1 {self.page=(self.page+1)%2;self.help(assets)?;},
            17=>self.set_state(assets,self.mode)?,
            18|19=>{
                self.playing=state==18;let enabled=!self.playing;let a=if enabled {255} else {102};
                let replay=self.root.named(&["mReplay"])?;
                let names:&[&str]=if self.mode==0 {&["mcStart","mcEnd"]} else {&["mcPrevMarker","mcNextMarker","mcLBMove","mcRBMove"]};
                for name in names {alpha(replay,name,a)?;}
                let menu=replay.named(&["mButtonMenu"])?;enable_button(menu,0,enabled)?;enable_button(menu,2,enabled)?;
                if self.mode==0&&!self.over_key {enable_button(menu,if snapshot.signed_in {3} else {2},enabled)?;}
            }
            3=>(),
            _=>return Err(format!("untranslated replay screen state {state}").into()),
        }
        Ok(())
    }
    fn effects(&mut self,assets:&Assets)->RenderResult<()> {
        let s=self.shown.as_ref().ok_or("replay state absent")?;
        let selected=if !s.film_pack&&s.edit_selection==3 {7} else {s.edit_selection};
        let values=[s.speed.to_string(),assets.localize(["ID_REPLAY_EDITOR_CAMERASHAKE_OFF","ID_REPLAY_EDITOR_CAMERASHAKE_LOW","ID_REPLAY_EDITOR_CAMERASHAKE_MEDIUM","ID_REPLAY_EDITOR_CAMERASHAKE_HIGH"][s.shake as usize])?,
            assets.localize(["ID_REPLAYEDITOR_WATERMARK_OFF","ID_REPLAYEDITOR_WATERMARK_DEFAULT","ID_REPLAYEDITOR_WATERMARK_CUSTOM"][s.watermark as usize])?];
        let effects=self.root.named(&["mReplay","mEffects"])?;
        for (i,(name,title)) in ROWS.iter().enumerate() {
            let row=EditSkaterRow {kind:if i<7 {"option"} else {"selector"}.into(),title:assets.localize(title)?,subtitle:if i>=7 {values[i-7].clone()} else {String::new()},..Default::default()};
            edit_skater_movie::highlight(effects.named(&[name])?,assets,&row,i==selected)?;
        }
        let mut items=vec![("Start","ID_REPLAYEDITOR_HELP"),("B","ID_COMMON_BACK")];
        if selected<7 {items.push(("A","ID_COMMON_SELECT"));}
        buttons(effects.named(&["mButton"])?,assets,&items)
    }
    fn help(&mut self,assets:&Assets)->RenderResult<()> {
        let (base,pages)=match self.mode {
            0=>("INSTANTREPLAY",2),9=>("MAINTIMELINE",2),4=>("EDITMENU",2),1=>("FOLLOWCAMERA",1),
            2=>("TRIPODCAMERA",1),10=>("GAMECAMERA",1),7=>("FREECAMERA",1),12=>("FOLLOWCAMERA_ADVANCED",2),
            13=>("TRIPODCAMERA_ADVANCED",2),14=>("FREECAMERA_ADVANCED",2),_=>return Err("replay help mode".into()),
        };self.pages=pages;
        let prefix=format!("ID_REPLAYEDITOR_HELP_TEXT_{base}");
        let help=self.root.named(&["mReplay","mHelp"])?;
        label_pair(help.named(&["mTextTitle"])?,assets,&format!("{prefix}_TITLE"))?;
        let text=if pages==2 {format!("{prefix}{}",self.page+1)} else {prefix};
        label(help.named(&["mTextHelp","labelText"])?,assets,&text)?;
        let mut items=vec![("B","ID_COMMON_BACK")];if pages>1 {items.push(("A","ID_COMMON_NEXT"));}
        // This help menu retains button_menu's default SetInfo (24px Marker_streak).
        map_movie::configure_button_items(help.named(&["mcHelpButtonMenu"])?,assets,&items,0xffff_ffff)
    }
    fn publish_timeline(&mut self)->RenderResult<()> {
        let Some(s)=&self.shown else {return Ok(())};
        let custom=self.root.named(&["mReplay","mCustomRender"])?;
        // Render82579988 gates both custom draws independently of APT alpha.
        // Instant playback changes mode to Play but leaves this gate disabled.
        if !s.render_timeline {custom.properties.remove("replay_range");return Ok(());}
        custom.properties.insert("replay_range".into(),format!("{} {} {} {} {} {} {}",s.start_head,s.end_head,s.selected_clip[0],s.selected_clip[1],s.marker_index.map_or(-1,|n|n as i32),u8::from(s.mode==Mode::Instant),u8::from(s.mode.editing_camera())));
        custom.properties.insert("replay_intervals".into(),s.intervals.iter().map(|i|format!("{} {} {} {}",i.start,i.end,i.camera as u8,u8::from(i.hard))).collect::<Vec<_>>().join(";"));
        Ok(())
    }
}
fn buttons(clip:&mut Clip,assets:&Assets,items:&[(&str,&str)])->RenderResult<()> {
    map_movie::styled_button_items(clip,assets,items,0xffff_ffff,false,"HelveticaPresto",20.)
}
fn label(clip:&mut Clip,assets:&Assets,key:&str)->RenderResult<()> {clip.text=Some(assets.localize(key)?.encode_utf16().collect());Ok(())}
fn label_pair(clip:&mut Clip,assets:&Assets,key:&str)->RenderResult<()> {raw_pair(clip,&assets.localize(key)?) }
fn raw_pair(clip:&mut Clip,text:&str)->RenderResult<()> {
    for name in ["labelText","labelTextGlow"] {clip.named(&[name])?.text=Some(text.encode_utf16().collect());}Ok(())
}
fn alpha(clip:&mut Clip,name:&str,value:u8)->RenderResult<()> {
    let item=child(clip,name)?;item.entry.colour_scale=(item.entry.colour_scale&0x00ff_ffff)|(u32::from(value)<<24);
    item.clip.script_transform=true;Ok(())
}
fn alpha_if_present(clip:&mut Clip,name:&str,value:u8)->RenderResult<()> {
    if clip.children.values().any(|c|c.entry.name.as_deref().is_some_and(|n|n.eq_ignore_ascii_case(name))) {
        alpha(clip,name,value)?;
    }
    Ok(())
}
fn enable_button(menu:&mut Clip,index:usize,enabled:bool)->RenderResult<()> {
    // button_menu.Enable: text and button alpha 100/40, preserving their positions.
    // Camera mode has only Help/Commit: the retained EditClip index can refer
    // to a cleared item. The original dynamic member assignment is a no-op.
    for name in [format!("mText{index}"),format!("mButton{index}")] {alpha_if_present(menu,&name,if enabled {255} else {102})?;}Ok(())
}
pub(super) fn action(clip:&mut Clip,offset:usize)->RenderResult<()> {
    match (clip.character,offset) {
        (0,0x82a0)=>clip.calls.push("ReplayConstruct"),
        (0,0x82d8)=>(), // fingerprinted class definition; methods translated above
        (0,0xa0f0)=>{clip.playback.stop();clip.calls.push("ReplayIntroComplete");},
        (0,0xa114)=>clip.visible=false,
        (0,0xa128)=>{clip.playback.stop();clip.calls.push("ReplayOutroComplete");},
        (196,0xa618)=>{clip.playback.stop();clip.calls.push("ReplayInstantDone");},
        _=>return Err(format!("untranslated Replay APT {} {offset:#x}",clip.character).into()),
    }
    Ok(())
}

/// BE DrawHardCuts82571018/DrawMarkers825714b8. Colours also checked against
/// the four PPC switch destinations (Hex-Rays only recovered the Follow case).
pub(super) fn draw_timeline(assets:&Assets,clip:&Clip,m:skate2_formats::apt::Matrix,
    colour:super::Colour,size:(u32,u32),out:&mut Vec<super::Batch>,flat:bool)->RenderResult<()> {
    let Some(range)=clip.properties.get("replay_range") else {return Ok(())};
    let values=range.split_whitespace().map(str::parse::<f32>).collect::<Result<Vec<_>,_>>()?;
    let [start,end,selected_start,selected_end,marker,instant,editing]=values.as_slice() else {return Err("replay timeline range".into())};
    let intervals=clip.properties.get("replay_intervals").ok_or("replay intervals absent")?;
    let intervals=intervals.split(';').map(|v|v.split_whitespace().map(str::parse::<f32>).collect::<Result<Vec<_>,_>>()).collect::<Result<Vec<_>,_>>()?;
    // CustomControlRender82636860..826368a4 supplies the first rendering
    // unit's strip vertex4.y to DrawHardCuts. In the original shape175 this
    // is30, not the51px height that DrawMarkers uses for its icon textures.
    let shape=clip.children.get(&1).ok_or("replay custom timeline shape absent")?;
    let strip=assets.movies[shape.clip.asset].geometry.shape(shape.clip.character as i32)
        .and_then(|s|s.elements.first()).ok_or("replay custom timeline strip absent")?;
    let height=strip.positions.get(4).ok_or("replay custom timeline vertex absent")?[1];
    let m=super::concatenate(m,shape.entry.matrix);
    for interval in &intervals {
        let [a,b,camera,_hard]=interval.as_slice() else {return Err("replay timeline interval".into())};
        if *b<*start||*a>*end {continue;}
        let highlighted=*marker<0.&&a>=selected_start&&b<=selected_end&&*instant==0.;
        let rgb=match (*camera as u8,highlighted) {
            (0,false)=>[0.41,0.41,0.19],(0,true)=>[0.74,0.74,0.19],
            (1,false)=>[0.12,0.28,0.36],(1,true)=>[0.29,0.54,0.65],
            (2,false)=>[0.27,0.40,0.15],(2,true)=>[0.52,0.72,0.36],
            (3,false)=>[0.30,0.30,0.35],(3,true)=>[0.53,0.53,0.53],
            _=>return Err("replay camera kind".into()),
        };
        let alpha=colour.apply([1.;4])[3]*if highlighted {0.85} else {0.65};
        out.push(super::Batch {texture:None,wrapped:false,vertices:super::quad(m,[a.max(*start)*817.,3.,b.min(*end)*817.,height],
            [0.,0.,1.,1.],[rgb[0],rgb[1],rgb[2],alpha],size,flat)});
    }
    for (i,interval) in intervals.iter().enumerate().skip(1) {
        if *marker==i as f32 && *editing!=0. {continue;}
        let glow=*marker==i as f32;
        let name=match (interval[3]!=0.,glow) {(true,false)=>"marker_default.Texture",(true,true)=>"marker_glow.Texture",(false,false)=>"key_default.Texture",(false,true)=>"key_glow.Texture"};
        let texture=assets.texture(name)?;let x=interval[0]*817.-0.5;
        out.push(super::Batch {texture:Some(texture.name.clone()),wrapped:false,vertices:super::quad(m,[x-6.5,0.,x+6.5,51.],[0.,0.,1.,1.],[1.;4],size,flat)});
    }
    Ok(())
}
