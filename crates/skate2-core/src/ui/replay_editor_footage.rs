//! SaveLoadManager::InsufficientSlotsPopup8256ff60 opens state56 in delete
//! mode. This is the local memcard branch of SkateReelMyFootage, not an online
//! connection or a replacement replay menu. ReceiveInput826b1720, options
//! 826a5ec0/826a0c80, media_browser.apt NextValue/PrevValue/UpdateInput.
use super::{Controller,Action,Dialog,Popup,FeSoundRequest,Input,Result,Spaces};

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct FootageItem {pub slot:u32,pub seconds:u32,pub location:String}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Footage {
    pub replay:bool,
    pub items:Vec<FootageItem>,
    pub selected:usize,
    pub first:usize,
    pub slider_start:usize,
    pub ready:bool,
}
impl Footage {
    pub fn navigate(&mut self,key:u8) {
        if self.items.is_empty() {return;}
        let (forward,steps)=match key {12=>(false,3),13=>(true,3),14=>(false,1),15=>(true,1),_=>return};
        for _ in 0..steps {
            if forward {self.selected=(self.selected+1).min(self.items.len()-1);} else {self.selected=self.selected.saturating_sub(1);}
            if self.selected<self.first {self.first-=3;} else if self.selected>=self.first+6 {self.first+=3;}
            self.slider_start=self.slider_start.clamp(self.selected.saturating_sub(5),self.selected);
        }
    }
}
impl Controller {
    pub fn media_full(&mut self,replay:bool) {
        self.saving=false;
        self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
        self.popup=Some((Dialog::MediaFull(replay),Popup {
            title:"ID_COMMON_WARNING_TITLE".into(),description:if replay {"ID_MEMCARD_MAX_SLOTS_REACHED_REPLAY"} else {"ID_MEMCARD_MAX_SLOTS_REACHED_PHOTO"}.into(),
            options:vec!["ID_SKATE_REEL_BROWSER".into(),"ID_COMMON_CONTINUE_NO_SAVE".into()],selected:0,cancellable:false}));
    }
    pub fn footage_loaded(&mut self,replay:bool,items:Vec<FootageItem>) {
        if let Some(browser)=self.footage.as_mut().filter(|b|b.replay==replay) {
            browser.items=items;
            browser.selected=browser.selected.min(browser.items.len().saturating_sub(1));
            browser.first=(browser.first.min(browser.selected)/3)*3;
            browser.slider_start=browser.selected.saturating_sub(5);
        } else {self.footage=Some(Footage {replay,items,selected:0,first:0,slider_start:0,ready:false});}
        self.saving=false;
    }
    pub fn footage_ready(&mut self) {if let Some(browser)=&mut self.footage {browser.ready=true;}}
    pub(super) fn footage_input(&mut self,input:&Input,spaces:&Spaces)->Result<Action> {
        if !self.footage.as_ref().is_some_and(|b|b.ready) {return Ok(Action::None);}
        for &key in &input.keys {
            if self.popup.is_some() {
                if key==3&&self.popup.as_ref().is_some_and(|(d,_)|matches!(d,Dialog::MediaDelete(_,_))) {
                    self.popup=None;self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_B_BUTTON"));
                } else {self.popup_input(key,spaces)?;}
                if self.saving||self.pending!=Action::None {break;}
                continue;
            }
            let b=self.footage.as_mut().expect("open footage");
            match key {
                3=>{
                    self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_B_BUTTON"));
                    self.footage=None;self.ready=false;self.instant=!self.view_from_save;self.render_timeline=false;
                    self.mode=if self.instant {super::Mode::Instant} else {super::Mode::Timeline};
                    self.reset_deltas();break;
                }
                5=>{b.replay=!b.replay;b.items.clear();b.selected=0;b.first=0;b.slider_start=0;
                    self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_X_BUTTON"));
                    self.pending=Action::Browse(b.replay);break;}
                2 if !b.items.is_empty()=>{
                    let (replay,slot)=(b.replay,b.items[b.selected].slot);
                    self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
                    self.popup=Some((Dialog::MediaDelete(replay,slot),Popup {
                        title:if replay {"ID_SKATE_REEL_REPLAY_MOVIE"} else {"ID_SKATEREEL_PHOTO"}.into(),
                        description:"#".into(),options:vec!["ID_SKATEREEL_DELETE_PHOTO".into()],selected:0,cancellable:true}));
                }
                12..=15 if !b.items.is_empty()=>{
                    let (forward,steps,sound)=match key {12=>(false,3,"FE_SOUND_CORE_NAV_UP"),13=>(true,3,"FE_SOUND_CORE_NAV_DOWN"),
                        14=>(false,1,"FE_SOUND_CORE_NAV_LEFT"),_=>(true,1,"FE_SOUND_CORE_NAV_RIGHT")};
                    // Three sequential NextValue/PrevValue calls preserve the
                    // native partial-last-row clamp and rowwise scroll window.
                    for _ in 0..steps {
                        if forward {b.selected=(b.selected+1).min(b.items.len()-1);} else {b.selected=b.selected.saturating_sub(1);}
                        if b.selected<b.first {b.first-=3;}
                        else if b.selected>=b.first+6 {b.first+=3;}
                        // basic_slider advances one item independently of the
                        // browser's three-item row window.
                        b.slider_start=b.slider_start.clamp(b.selected.saturating_sub(5),b.selected);
                    }
                    self.sounds.push(FeSoundRequest::Play(sound));
                }
                _=>()
            }
        }
        Ok(std::mem::replace(&mut self.pending,Action::None))
    }
}
