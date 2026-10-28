//! Original phonelist.apt fbbb45c1, FE74 / screen13. Geometry, skin exports,
//! timelines, row spacing and button style come from its own SetState script.
use super::{assets::Assets,clips::Clip,map_movie,localizer::{self,AptArgument},progression_popup::ProgressionPopup};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::contacts::{View,Text};
pub(super) const ASSET:usize=101;
pub(super) const PICKER:usize=102;
pub(super) struct ContactsMovie {
    pub root:Clip,
    pub popup:ProgressionPopup,
    shown:Option<View>,pending:bool,
    pub intro:bool,pub destroyed:bool,start:usize,
}
impl ContactsMovie {
    pub fn new(assets:&Assets)->RenderResult<Self> {
        let mut root=Clip::new(assets,ASSET,0)?;root.visible=false;
        Ok(Self{root,popup:Default::default(),shown:None,pending:false,intro:false,destroyed:false,start:0})
    }
    pub fn pending(&self)->bool{self.pending}
    pub fn open(&self)->bool{self.shown.is_some()}
    pub fn update(&mut self,assets:&Assets,state:Option<&View>)->RenderResult<()> {
        if self.shown.as_ref()==state{return Ok(());}
        if state.is_some()&&self.shown.is_none(){self.pending=true;self.intro=false;self.start=0;}
        if state.is_none()&&self.shown.is_some()&&!std::mem::take(&mut self.pending){self.root.goto(assets,"outro")?;}
        if let Some(s)=state {
            let changed=self.shown.as_ref().is_none_or(|old|old.page!=s.page);
            if changed {self.start=0;}
            if !self.pending {self.populate(assets,s,changed)?;}
        }
        let dialog=state.filter(|s|s.not_enough_cash).map(|_|("ID_COMMON_WARNING_TITLE".to_owned(),"ID_PHONELIST_NOT_ENOUGH_CASH_DESC".to_owned(),0));
        if dialog.is_some() && self.shown.as_ref().is_none_or(|s|!s.not_enough_cash){self.popup=Default::default();}
        self.popup.update(assets,dialog.as_ref(),Some(&vec!["ID_COMMON_OK".to_owned()]))?;
        self.shown=state.cloned();Ok(())
    }
    pub fn link(&mut self,assets:&Assets)->RenderResult<()> {
        if !std::mem::take(&mut self.pending){return Ok(());}
        self.root=Clip::new(assets,ASSET,0)?;self.destroyed=false;
        if let Some(s)=self.shown.clone(){self.populate(assets,&s,true)?;}
        self.root.goto(assets,"intro")?;Ok(())
    }
    fn populate(&mut self,assets:&Assets,s:&View,changed:bool)->RenderResult<()> {
        let services=s.page==1;let description=s.page==3;
        let max=if services||description {3}else{6};
        let scrolling=s.options.len()>max;
        let quick=self.root.named(&["mcQuickMenu"])?;
        if changed {quick.goto(assets,if services{"services"}else if description {if scrolling{"desc_scrollbar"}else{"desc"}}else if scrolling{"menu_scrollbar"}else{"menu"})?;}
        let title=assets.localize(s.title)?.encode_utf16().collect::<Vec<_>>();
        quick.named(&["mcTextHeader","labelText"])?.text=Some(title.clone());
        quick.named(&["mcTextHeader","labelText_glow"])?.text=Some(title);
        self.start=self.start.min(s.selected);
        if s.selected>=self.start+max {self.start=s.selected+1-max;}
        let labels=s.options.iter().map(|t|text(assets,t)).collect::<RenderResult<Vec<_>>>()?;
        let rows=labels.iter().map(|l|(l.as_str(),true)).collect::<Vec<_>>();
        let picker=quick.named(&["mMenuPicker","mMenuPicker"])?;
        // Basic_Menu_Picker_Component18 wraps its named inner control17.
        if changed {picker.named(&["mItemGroup"])?.children.clear();}
        picker.properties.insert("picker_item_asset".into(),PICKER.to_string());
        picker.properties.insert("picker_item_export".into(),if scrolling{"ITEMLABEL_PhoneListShort"}else{"ITEMLABEL_PhoneListLong"}.into());
        picker.properties.insert("picker_item_offset".into(),"38".into());
        map_movie::update_picker(picker,assets,&rows,s.selected,self.start,max,false)?;
        if scrolling {map_movie::update_scrollbar(quick.named(&["mScrollbar"])?,assets,rows.len(),max,self.start,if description{115.}else{247.})?;}
        if services {
            quick.named(&["mTextCash","labelText"])?.text=Some(localizer::localized_format_string(assets,"ID_PHONELIST_CASH",&[AptArgument::Integer(s.cash)])?);
            quick.named(&["mBanner"])?.goto(assets,s.banner)?;
        }
        if description {quick.named(&["mTextInfo","labelTextInfo"])?.text=Some(text(assets,&s.description)?.encode_utf16().collect());}
        let mut buttons=vec![("B","ID_COMMON_BACK")];if s.select_enabled{buttons.push(("A","ID_COMMON_SELECT"));}
        map_movie::styled_button_items(quick.named(&["mBottomPanel","mButtonMenu"])?,assets,&buttons,0xff00_0000|13_034_748,true,"HelveticaBold",16.)?;
        Ok(())
    }
    pub fn tick(&mut self,assets:&Assets)->RenderResult<()> {
        if self.root.visible {self.root.tick(assets)?;}
        let mut calls=vec![];self.root.take_calls(&mut calls);
        for call in calls {match call {
            "ContactsIntro"=>self.intro=true,
            "ContactsOutro"=>{self.root.visible=false;self.destroyed=true;},
            _=>return Err(format!("unknown Contacts callback {call}").into()),
        }}self.popup.tick(assets)
    }
}
fn text(assets:&Assets,t:&Text)->RenderResult<String> {
    let value=if let Some(n)=t.number {String::from_utf16_lossy(&localizer::format_string_int(assets,&t.key,n)?)}else{assets.localize(&t.key)?};
    Ok(if t.unbeaten {format!("\u{bb} {value}")}else{value})
}
/// The original DoInitActions define `phonelist`, `menu_picker` and
/// `basic_menu_picker`, then register Basic_Menu_Picker_Component. Their
/// implementations are supplied by ContactsMovie and map_movie's picker.
/// ExecuteInitAction829f0e20 / ExecuteInitActions829f7f98 keep these separate
/// from ordinary timeline actions, including the imported component's class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!(
        (asset, character, offset),
        (ASSET, 74, 0x26a4)
            | (PICKER, 19, 0xd14)
            | (PICKER, 20, 0x1c28)
            | (PICKER, 18, 0x297c)
    )
}
pub(super) fn action(clip:&mut Clip,offset:usize)->RenderResult<()> {
    match offset {
        0x2684=>clip.playback.stop(), // constructor; Screen_EdgeOffset is zero
        0x2dd8=>{}, // undefined EnableDimmer script method
        0x2df4=>{clip.playback.stop();clip.calls.push("ContactsIntro");},
        0x2e20=>{clip.playback.stop();clip.calls.push("ContactsOutro");},
        _=>return Err(format!("untranslated Contacts action {offset:#x}").into()),
    }Ok(())
}
