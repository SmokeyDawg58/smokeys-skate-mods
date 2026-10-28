//! Original tricks/trickguide.apt f3067239, screen32. Layout, timelines and
//! SetState/UpdateInput use the authored picker, gestures and button controls.
use super::{assets::Assets,clips::Clip,letterbox::LetterBox,map_movie,
    objective_movie::{set_x,set_xscale},pause_movie::child,tutorial_movie};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::trick_guide::Snapshot;
pub(super) const ASSET:usize=100;
pub(super) struct TrickGuideMovie {
    pub root:Clip,
    shown:Option<Snapshot>,
    pending:bool,
    pub intro:bool,
    pub destroyed:bool,
    start:usize,
}
impl TrickGuideMovie {
    pub fn new(assets:&Assets)->RenderResult<Self> {
        let mut root=Clip::new(assets,ASSET,0)?;root.visible=false;
        Ok(Self {root,shown:None,pending:false,intro:false,destroyed:false,start:0})
    }
    pub fn pending(&self)->bool {self.pending}
    pub fn open(&self)->bool {self.shown.is_some() || self.root.visible}
    pub fn update(&mut self,assets:&Assets,state:Option<&Snapshot>,letterbox:&mut LetterBox)->RenderResult<()> {
        if self.shown.as_ref()==state {return Ok(());}
        if state.is_some() && self.shown.is_none() {self.pending=true;self.intro=false;self.start=0;}
        if state.is_none() && self.shown.is_some() && !std::mem::take(&mut self.pending) {self.root.goto(assets,"outro")?;}
        if let Some(s)=state {
            if self.shown.as_ref().is_none_or(|old|old.level!=s.level){self.start=0;}
            if !self.pending {self.populate(assets,s,letterbox)?;}
        }
        self.shown=state.cloned();Ok(())
    }
    pub fn link(&mut self,assets:&Assets,letterbox:&mut LetterBox)->RenderResult<()> {
        if !std::mem::take(&mut self.pending){return Ok(());}
        self.root=Clip::new(assets,ASSET,0)?;
        letterbox.enable(assets,true)?;letterbox.set_title(&assets.localize("ID_SCREEN_TRICK_GUIDE_TITLE")?)?;
        if let Some(s)=self.shown.clone(){self.populate(assets,&s,letterbox)?;}
        self.root.goto(assets,"intro")?;Ok(())
    }
    fn populate(&mut self,assets:&Assets,s:&Snapshot,letterbox:&mut LetterBox)->RenderResult<()> {
        self.start=self.start.min(s.selected);
        if s.selected>=self.start+7 {self.start=s.selected-6;}
        let labels=s.labels.iter().map(|l|assets.localize(l)).collect::<RenderResult<Vec<_>>>()?;
        let rows=labels.iter().map(|l|(l.as_str(),true)).collect::<Vec<_>>();
        let top=self.root.named(&["mTopPanel","mTopInner"])?;
        let picker=top.named(&["mMenuPicker","mMenuPicker"])?;
        map_movie::update_picker(picker,assets,&rows,s.selected,self.start,7,false)?;
        map_movie::update_scrollbar(top.named(&["mVerticalSlider"] )?,assets,rows.len(),7,self.start,253.)?;
        let mut buttons=vec![("B","ID_COMMON_BACK")];if s.has_submenu {buttons.push(("A","ID_COMMON_SELECT"));}
        map_movie::configure_button_items(letterbox.clip.named(&["mcLetterbox","mInfo","mcBottomMenu","mButtonMenu"] )?,assets,&buttons,0xffff_ffec)?;
        let inner=self.root.named(&["mBottomPanel","mBottomInner"])?;
        inner.named(&["mcTextDescription","labelText"] )?.text=Some(assets.localize(&s.description)?.encode_utf16().collect());
        let group=child(inner,"mcGestureButtonGroup")?;set_x(group,13.);
        for i in 0..4 {
            for name in ["mcGestureItem","mcButtonItem"] {
                // SetState10 APT004aee/004b20 replaces authored positions:
                // gestures start at48, buttons at47 (before kerning).
                let item=child(&mut group.clip,&format!("{name}{i}"))?;
                set_x(item,130.*i as f32+if name=="mcGestureItem" {48.} else {47.});item.clip.visible=false;
                if name=="mcGestureItem" {set_xscale(item,if s.goofy {-100.}else{100.});}
            }
            if i<3 {for name in ["mcConnectorArrow","mcConnectorPlus"] {
                let c=child(&mut group.clip,&format!("{name}{i}"))?;set_x(c,130.*i as f32+96.);c.clip.visible=false;
            }}
        }
        let mut kern=0.;let mut prior="";
        for (i,slot) in s.slots.iter().take(4).enumerate() {
            let mut button=slot.button.as_str();
            // Authored PlayerStance_IsRegular branch. Gestures use IsGoofy.
            if s.stance_regular {
                if !matches!(s.description.as_str(),"ID_TRICK_TYPE_JUMP_DESC"|"ID_TRICK_TYPE_SPRINT_DESC") {
                    button=match button {"A"=>"X","X"=>"A",b=>b};
                }
                button=match button {"L_Trigger"=>"R_Trigger","R_Trigger"=>"L_Trigger",b=>b};
            }
            let shown=if !slot.gesture.is_empty() {
                let item=child(&mut group.clip,&format!("mcGestureItem{i}"))?;set_x(item,item.entry.matrix.tx-kern);
                tutorial_movie::set_gesture_type(&mut item.clip,assets,&slot.gesture)?;item.clip.visible=true;true
            } else if !button.is_empty() {
                kern+=24.;let item=child(&mut group.clip,&format!("mcButtonItem{i}"))?;set_x(item,item.entry.matrix.tx-kern);
                item.clip.named(&["mButtonRender"] )?.properties.insert("_test".into(),button.into());item.clip.visible=true;kern+=24.;true
            } else {false};
            if shown && i>0 && matches!(prior,"Arrow"|"Plus") {
                group.clip.named(&[&format!("mcConnector{prior}{}",i-1)])?.visible=true;
            }
            if i<3 {for name in ["mcConnectorArrow","mcConnectorPlus"] {
                let c=child(&mut group.clip,&format!("{name}{i}"))?;set_x(c,c.entry.matrix.tx-kern);
            }}
            prior=&slot.connector;
        }
        if !s.slots.is_empty() {set_x(group,13.+(4-s.slots.len().min(4)) as f32*65.+kern*0.5);}
        Ok(())
    }
    pub fn tick(&mut self,assets:&Assets,letterbox:&mut LetterBox)->RenderResult<()> {
        if self.root.visible {self.root.tick(assets)?;}
        let mut calls=vec![];self.root.take_calls(&mut calls);
        for call in calls {match call {
            "TrickGuideIntro"=>{self.intro=true;letterbox.enable_info(assets,true)?;},
            "TrickGuideOutroBegin"=>{letterbox.enable_info(assets,true)?;letterbox.enable(assets,false)?;},
            "TrickGuideOutro"=>{self.root.visible=false;self.destroyed=true;},
            _=>return Err(format!("unknown Trick Guide callback {call}").into()),
        }}Ok(())
    }
}
pub(super) fn action(clip:&mut Clip,offset:usize)->RenderResult<()> {
    match offset {
        0x45c0=>clip.playback.stop(),
        0x45fc=>{}, // registered class; constructor and methods translated above
        0x50f4=>{clip.playback.stop();clip.calls.push("TrickGuideIntro");},
        0x5114=>clip.calls.push("TrickGuideOutroBegin"),
        0x5138=>{clip.playback.stop();clip.calls.push("TrickGuideOutro");},
        0x531c|0x5328|0x5334=>clip.playback.stop(),
        _=>return Err(format!("untranslated Trick Guide APT action {offset:#x}").into()),
    }Ok(())
}
