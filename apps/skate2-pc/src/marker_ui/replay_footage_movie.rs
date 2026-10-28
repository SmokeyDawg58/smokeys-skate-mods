//! Original screen102 skatereel_myfootage, media_browser and mediathumb.
//! The local offline library and Replay2's insufficient-slots delete browser
//! share the original six-thumbnail grid (8256ff60/826ab8e0).
use super::{assets::Assets,clips::Clip,map_movie::{styled_button_items,update_scrollbar},
    progression_popup::ProgressionPopup,hud_leaderboard_movie::{width,find_child},pause_movie::{bounds,child}};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::replay_editor::{Footage,Popup};
pub(super) const ASSET:usize=94;
pub(super) const BROWSER:usize=95;
pub(super) const THUMB:usize=96;

pub(super) struct FootageMovie {
    pub root:Clip,pub popup:ProgressionPopup,
    pub sounds:Vec<&'static str>,
    shown:Option<Footage>,pending:bool,closing:bool,intro:bool,outro:bool,destroy:bool,dimming:bool,
}
impl FootageMovie {
    pub fn new(assets:&Assets)->RenderResult<Self> {
        let mut root=Clip::new(assets,ASSET,0)?;root.visible=false;root.playback.stop();
        Ok(Self {root,popup:Default::default(),sounds:Vec::new(),shown:None,pending:false,closing:false,intro:false,outro:false,destroy:false,dimming:false})
    }
    pub fn open(&self)->bool {self.shown.is_some()}
    pub fn pending(&self)->bool {self.pending}
    pub fn dimming(&self)->bool {self.dimming}
    pub fn take_intro(&mut self)->bool {std::mem::take(&mut self.intro)}
    pub fn take_outro(&mut self)->bool {std::mem::take(&mut self.outro)}
    pub fn update(&mut self,assets:&Assets,state:Option<&Footage>,popup:Option<&Popup>)->RenderResult<()> {
        // gFEStateInfo82fded80: state56 dims; its popup state34 does not.
        self.dimming=state.is_some()&&popup.is_none();
        if state.is_some()&&self.shown.is_none() {self.pending=true;self.intro=false;}
        if state.is_none()&&self.shown.is_some() {
            if !std::mem::take(&mut self.pending) {self.closing=true;self.root.goto(assets,"outro")?;}
        }
        let changed=self.shown.as_ref()!=state;
        let reload=match (self.shown.as_ref(),state) {
            (Some(old),Some(new))=>old.replay!=new.replay||old.items!=new.items,
            _=>true,
        };
        self.shown=state.cloned();
        if changed&&!self.pending&&state.is_some() {self.configure(assets,reload)?;}
        self.popup.show(assets,popup.map(|p|(p.title.clone(),p.description.clone(),p.options.clone(),p.selected,p.cancellable)))?;
        self.callbacks()
    }
    pub fn link(&mut self,assets:&Assets)->RenderResult<()> {
        if !std::mem::take(&mut self.pending) {return Ok(());}
        self.root=Clip::new(assets,ASSET,0)?;self.closing=false;self.destroy=false;
        self.root.goto(assets,"intro")?;
        self.configure(assets,true)?;self.callbacks()
    }
    pub fn tick(&mut self,assets:&Assets)->RenderResult<()> {
        if std::mem::take(&mut self.destroy) {self.root.visible=false;self.closing=false;}
        if self.root.visible||self.closing {self.root.tick(assets)?;}
        self.popup.tick(assets)?;self.callbacks()
    }
    fn callbacks(&mut self)->RenderResult<()> {
        let mut calls=Vec::new();self.root.take_calls(&mut calls);
        for call in calls {match call {
            "FootageIntro"=>self.intro=true,
            "FootageOutro"=>{self.outro=true;self.destroy=true;},
            "FE_SOUND_SCRATCH_BOX_FX"=>self.sounds.push(call),
            _=>return Err(format!("untranslated footage callback {call}").into()),
        }}Ok(())
    }
    pub fn texture_requests(&self)->Vec<(bool,u32)> {
        self.shown.as_ref().map(|s|s.items.iter().skip(s.first).take(6).map(|i|(s.replay,i.slot)).collect()).unwrap_or_default()
    }
    fn configure(&mut self,assets:&Assets,reload:bool)->RenderResult<()> {
        let s=self.shown.as_ref().ok_or("footage without state")?;
        let window=self.root.named(&["mWindow"])?;
        if reload {
        let title=if s.replay {"ID_SKATE_REEL_MY_FOOTAGE_VIDEO"} else {"ID_SKATE_REEL_MY_FOOTAGE_PHOTO"};
        for name in ["txtTitle","txtGlow"] {label(window.named(&["mTitle",name])?,assets,title)?;}
        window.named(&["mAvailable"])?.visible=false;
        let tabs=window.named(&["mFilterTabs"])?;
        for name in ["lt_btn","rt_btn","mFilterTab1","mFilterTab2"] {tabs.named(&[name])?.visible=false;}
        let tab=child(tabs,"mFilterTab0")?;tab.entry.matrix.tx=37.;tab.clip.script_transform=true;
        tab.clip.visible=true;tab.clip.goto(assets,"selected")?;
        label(tab.clip.named(&["labelText"])?,assets,"ID_SKATE_REEL_FILTER_MY_SAVED_FOOTAGE")?;
        autosize(tab.clip.named(&["labelText"])?,"left");
        let label_width=width(child(&mut tab.clip,"labelText")?,assets)?;
        let fill_width=128.+label_width;
        let label_piece=child(&mut tab.clip,"labelText")?;label_piece.entry.matrix.tx=72.;label_piece.clip.script_transform=true;
        let fill=child(&mut tab.clip,"backer_fill")?;fill.entry.matrix.a=fill_width/32.;fill.clip.script_transform=true;
        let right=child(&mut tab.clip,"backer_corner_right")?;right.entry.matrix.tx=fill_width+16.;right.clip.script_transform=true;

        // SetFilter displays the *other* content type (the X-button action).
        let filter=window.named(&["mFilter"])?;
        label(filter.named(&["txtFilter"])?,assets,if s.replay {"ID_SKATE_REEL_PHOTOS"} else {"ID_SKATE_REEL_VIDEOS"})?;
        autosize(filter.named(&["txtFilter"])?,"left");
        let extent=(width(child(filter,"txtFilter")?,assets)?-4.).floor()+40.;
        for (name,x) in [("backFill",-extent-4.),("backLeft",-extent-36.),("btnIcon",-extent-2.),("txtFilter",-extent+21.)] {
            let part=child(filter,name)?;part.entry.matrix.tx=x;part.clip.script_transform=true;
            if name=="backFill" {part.entry.matrix.a=extent/8.;}
        }
        window.goto(assets,"full")?;
        }
        let display=window.named(&["mDisplay"])?;
        if reload {display.goto(assets,"on")?;}
        let empty=s.items.is_empty();
        display.named(&["mNoDisplay"])?.visible=empty;
        let message=display.named(&["mNoDisplay","mMessage"])?;
        label(message.named(&["txtStatus"])?,assets,"ID_SKATEREEL_BROWSER_NO_FOOTAGE")?;
        label(message.named(&["txtGlow"])?,assets,"ID_SKATEREEL_BROWSER_NO_FOOTAGE")?;
        autosize(message.named(&["txtStatus"])?,"left");
        let message_width=width(child(message,"txtStatus")?,assets)?;
        let message=child(display.named(&["mNoDisplay"])?,"mMessage")?;
        message.entry.matrix.tx=350.-(message_width+52.)*0.5;message.clip.script_transform=true;
        let browser=display.named(&["mBrowser"])?;browser.visible=!empty;
        for n in 0..6 {
            let thumb=browser.named(&[&format!("mMediaThumb{n}")])?;
            let item=s.items.get(s.first+n);thumb.visible=item.is_some();
            let Some(item)=item else {continue};
            let selected=if s.selected==s.first+n {"on"} else {"off"};
            if thumb.properties.get("footage_selected").is_none_or(|v|v!=selected) {
                thumb.goto(assets,selected)?;thumb.properties.insert("footage_selected".into(),selected.into());
            }
            let skin=thumb.named(&["mSkin"])?;skin.goto(assets,if s.replay {"video"} else {"photo"})?;
            skin.named(&["txtTitle"])?.text=Some(" ".encode_utf16().collect());
            skin.named(&["mRating"])?.visible=false;
            if let Some(retired)=find_child(skin,"mRetired") {retired.clip.visible=false;}
            let image=skin.named(&["mImageComponent"])?;
            image.properties.insert("cac_texture".into(),crate::replay_media::texture_key(s.replay,item.slot));
        }
        update_scrollbar(browser.named(&["mSlider"])?,assets,s.items.len(),6,s.slider_start,306.)?;
        let panel=window.named(&["mInfoPanel"])?;
        panel.goto(assets,if s.replay {"video"} else {"photo"})?;
        // The original APT deliberately shows dashes for local media's online
        // statistics and length, even though GetMediaThumbDetails has a length.
        for name in ["txtDate","txtViews","txtLength","txtRating","txtCreator"] {
            if let Some(field)=find_child(panel,name) {field.clip.text=Some(vec![b'-' as u16]);}
        }
        for name in ["txtStyle","txtTrick"] {if let Some(field)=find_child(panel,name) {label(&mut field.clip,assets,"ID_TAG_NONE")?;}}
        let location=if s.replay {s.items.get(s.selected).map(|i|i.location.as_str()).unwrap_or("ID_TAG_NONE")} else {"ID_TAG_NONE"};
        label(panel.named(&["txtLocation"])?,assets,location)?;
        let location=child(panel,"txtLocation")?;autosize(&mut location.clip,"left");
        let w=width(location,assets)?-4.;let scale=if w>176. {176./w} else {1.};
        let h=bounds(&location.clip,assets)?.ok_or("footage location text bounds")?;
        location.entry.matrix.a=scale;location.entry.matrix.d=scale;location.entry.matrix.tx=564.;
        location.entry.matrix.ty=8.+(h.bottom-h.top)*(1.-scale)*0.5;location.clip.script_transform=true;
        let buttons=if empty {vec![("B","ID_COMMON_CANCEL")]} else {vec![("B","ID_COMMON_BACK"),("A","ID_COMMON_SELECT")]};
        styled_button_items(window.named(&["mButtonHelp"])?,assets,&buttons,0xffff_ffec,false,"Marker_streak",20.)?;
        Ok(())
    }
}
fn label(clip:&mut Clip,assets:&Assets,key:&str)->RenderResult<()> {clip.text=Some(assets.localize(key)?.encode_utf16().collect());Ok(())}
fn autosize(clip:&mut Clip,alignment:&str) {clip.properties.insert("text_autosize".into(),alignment.into());}
pub(super) fn action(clip:&mut Clip,offset:usize)->RenderResult<()> {
    match (clip.asset,clip.character,offset) {
        (ASSET,0,0x6a8c|0x8240|0x8280)=>(),
        (ASSET,0,0x825c)=>{clip.playback.stop();clip.calls.push("FootageIntro");},
        (ASSET,0,0x829c)=>{clip.playback.stop();clip.calls.push("FootageOutro");},
        (THUMB,36,0x6438)=>clip.calls.push("FE_SOUND_SCRATCH_BOX_FX"),
        _=>return Err(format!("untranslated footage APT {}:{} {offset:#x}",clip.asset,clip.character).into()),
    }Ok(())
}
pub(super) fn init_action(asset:usize,character:u32,offset:usize)->bool {
    matches!((asset,character,offset),
        (ASSET,125,0x6aa8)|(BROWSER,4,0x480)|(BROWSER,5,0xe04)|(BROWSER,3,0x174c)
        |(THUMB,81,0x5d40)|(THUMB,80,0x640c))
}
