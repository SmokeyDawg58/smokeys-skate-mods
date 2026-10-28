//! Original screens9/101/103, reached through MainMenuOnlineOptions23.
//! APT layouts and scripts: main_onlineoptions04e9e0b3,
//! skatereel_generalafcf3ebc and skatereel_photoaecfc0a2.
use super::{assets::Assets,clips::Clip,letterbox::LetterBox,map_movie,
    hud_leaderboard_movie::find_child,progression_popup::ProgressionPopup};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::skate_reel::{Page,Snapshot,ONLINE_LABELS,DESCRIPTIONS};
pub(super) const ONLINE:usize=97;
pub(super) const GENERAL:usize=98;
pub(super) const PHOTO:usize=99;
/// main_onlineoptions' board (shape 2, 512 high in its own units) in `shape_bands`, 78 units
/// shorter: two row pitches (`dummy0`'s 31.4 at `mMenu`'s 1.1875, over the board's 0.95), one for
/// the seventh native row the port's six leave out, one so that the space under the last row is
/// nearer the space over the first. The cut is where the board above and below it match.
const BOARD_BANDS:[[f32;4];2]=[[0.,208.,0.,208.],[286.,512.,208.,434.]];
const TITLES:[&str;5]=["ID_SKATE_REEL_MY_FOOTAGE","ID_SKATE_REEL_COMMUNITY",
    "ID_SKATE_REEL_FRIEND_SHARED","ID_SKATE_REEL_RECENTLY_SAVED","ID_SKATE_REEL_RECENTLY_UPLOADED"];
pub(super) const IMAGES:[&str;3]=["fe\\source\\images\\skatereel\\recentFriend",
    "fe\\source\\images\\skatereel\\recentSaved","fe\\source\\images\\skatereel\\recentUploads"];
pub(super) struct Movie {
    pub root:Clip,pub popup:ProgressionPopup,pub sounds:Vec<&'static str>,
    asset:usize,shown:Option<Snapshot>,pending:bool,intro:bool,outro:bool,destroy:bool,
}
impl Movie {
    pub fn new(assets:&Assets,asset:usize)->RenderResult<Self> {
        let mut root=Clip::new(assets,asset,0)?;root.visible=false;
        Ok(Self {root,popup:Default::default(),sounds:Vec::new(),asset,shown:None,
            pending:false,intro:false,outro:false,destroy:false})
    }
    pub fn pending(&self)->bool {self.pending}
    pub fn take_intro(&mut self)->bool {std::mem::take(&mut self.intro)}
    pub fn take_outro(&mut self)->bool {std::mem::take(&mut self.outro)}
    pub fn dimming(&self)->bool {self.asset==GENERAL&&self.shown.is_some()}
    pub fn update(&mut self,assets:&Assets,state:Option<&Snapshot>,letterbox:&mut LetterBox)->RenderResult<()> {
        let state=state.filter(|s|matches!((self.asset,s.page),(ONLINE,Page::Online)|(GENERAL,Page::General)|(PHOTO,Page::Photo(_))));
        if self.shown.as_ref()==state {return Ok(());}
        if self.shown.is_none()&&state.is_some() {self.pending=true;self.intro=false;}
        if self.shown.is_some()&&state.is_none()&&!std::mem::take(&mut self.pending) {self.root.goto(assets,"outro")?;}
        self.shown=state.cloned();
        if state.is_some()&&!self.pending {self.configure(assets)?;}
        self.popup.show(assets,state.and_then(|s|s.popup.as_ref()).map(|p|
            (p.title.clone(),p.description.clone(),p.options.clone(),p.selected,p.cancellable)))?;
        self.callbacks(assets,letterbox)
    }
    pub fn link(&mut self,assets:&Assets,letterbox:&mut LetterBox)->RenderResult<()> {
        if !std::mem::take(&mut self.pending) {return Ok(());}
        self.root=Clip::new(assets,self.asset,0)?;self.destroy=false;self.outro=false;
        self.configure(assets)?;
        if self.asset==ONLINE {
            letterbox.enable(assets,true)?;letterbox.set_title(super::main_movie::ONLINE_TITLE)?;
            map_movie::configure_button_items(letterbox.clip.named(&["mcLetterbox","mInfo","mcBottomMenu","mButtonMenu"])?,
                assets,&[("B","ID_COMMON_BACK"),("A","ID_COMMON_SELECT")],0xffff_ffec)?;
        } else {letterbox.enable(assets,false)?;}
        self.root.goto(assets,"intro")?;self.callbacks(assets,letterbox)
    }
    pub fn tick(&mut self,assets:&Assets,letterbox:&mut LetterBox)->RenderResult<()> {
        if std::mem::take(&mut self.destroy) {self.root.visible=false;}
        if self.root.visible {self.root.tick(assets)?;}
        self.popup.tick(assets)?;self.callbacks(assets,letterbox)
    }
    fn callbacks(&mut self,assets:&Assets,letterbox:&mut LetterBox)->RenderResult<()> {
        let mut calls=Vec::new();self.root.take_calls(&mut calls);
        for call in calls {match call {
            "ReelIntro"=>{self.intro=true;if self.asset==ONLINE {letterbox.enable_info(assets,true)?;}},
            "ReelOutroBegin"=>{letterbox.enable_info(assets,false)?;letterbox.cycle(assets)?;},
            "ReelOutro"=>{self.outro=true;self.destroy=true;},
            "FE_SOUND_SCRATCH_BOX_FX"=>self.sounds.push(call),
            _=>return Err(format!("untranslated Skate.Reel callback {call}").into()),
        }}Ok(())
    }
    fn configure(&mut self,assets:&Assets)->RenderResult<()> {
        let state=self.shown.as_ref().ok_or("Skate.Reel state absent")?;
        match self.asset {
            ONLINE=>{
                let labels=ONLINE_LABELS.map(|l|assets.localize(l)).into_iter().collect::<RenderResult<Vec<_>>>()?;
                let rows=labels.iter().enumerate().map(|(i,l)|(l.as_str(),state.enabled[i])).collect::<Vec<_>>();
                let picker=self.root.named(&["mMenu","mMenuPicker"])?;
                picker.properties.insert("picker_item_export".into(),"ITEMLABEL_HOM".into());
                map_movie::update_picker(picker,assets,&rows,state.selected,0,ONLINE_LABELS.len(),false)?;
                picker.named(&["mIndicator"])?.goto(assets,"dark")?;
                let board=[1,1,1].into_iter().try_fold(&mut self.root,|clip,depth|clip.children.get_mut(&depth).map(|c|&mut c.clip))
                    .ok_or("Online options board absent")?;
                board.properties.insert("shape_bands".into(),super::shape_bands(&BOARD_BANDS));
            }
            GENERAL=>{
                for field in ["txtTitle","txtGlow"] {label(self.root.named(&["mTitle",field])?,assets,"ID_SKATE_REEL_GENERAL_TITLE")?;}
                let categories=self.root.named(&["mCategories"])?;
                for (i,title) in TITLES[..2].iter().enumerate() {
                    let category=categories.named(&[&format!("mCategory{i}")])?;
                    selection(category,assets,if state.selected==i {"selected"} else {"unselected"})?;
                    category.named(&["mBacker"])?.goto(assets,"footage")?;
                    for field in ["txtCategory","txtGlow"] {if let Some(c)=find_child(category,field) {label(&mut c.clip,assets,title)?;}}
                }
                for (i,name) in ["mFriendsRecentName","mTopVideoName","mTopPhotoName"].into_iter().enumerate() {
                    label(categories.named(&[name])?,assets,TITLES[i+2])?;
                    let thumb=categories.named(&[&format!("mThumb{i}")])?;
                    selection(thumb,assets,if state.selected==i+2 {"on"} else {"off"})?;
                    let skin=thumb.named(&["mSkin"])?;skin.goto(assets,if i==2 {"photo"} else {"video"})?;
                    skin.named(&["mRating"])?.visible=false;
                    if let Some(c)=find_child(skin,"mRetired") {c.clip.visible=false;}
                    label(skin.named(&["txtTitle"])?,assets,"ID_COMMON_NA")?;
                    skin.named(&["mImageComponent"])?.properties.insert("cac_texture".into(),IMAGES[i].to_ascii_lowercase());
                }
                let content=self.root.named(&["mContent"])?;
                label(content.named(&["description_mc"])?,assets,DESCRIPTIONS[state.selected])?;
                map_movie::styled_button_items(content.named(&["mButtonHelp"])?,assets,
                    &[("B","ID_COMMON_BACK"),("A","ID_COMMON_SELECT")],0xffff_ffec,false,"Marker_streak",20.)?;
            }
            PHOTO=>{
                let Page::Photo(slot)=state.page else {return Err("photo slot absent".into());};
                self.root.named(&["mImageComponent"])?.properties.insert("cac_texture".into(),crate::replay_media::photo_key(slot));
                let clip=self.root.named(&["mClip"])?;
                if !clip.properties.contains_key("reel_wide") {clip.goto(assets,"wide")?;clip.properties.insert("reel_wide".into(),"1".into());}
                clip.named(&["mRateWide"])?.visible=state.rating;
                clip.named(&["mViewWide"])?.visible=!state.rating;
                let view=clip.named(&[if state.rating {"mRateWide"} else {"mViewWide"}])?;
                if !state.rating {view.named(&["txtGamertag"])?.text=Some(Vec::new());}
                map_movie::styled_button_items(view.named(&["mButtonHelp"])?,assets,
                    &[if state.rating {("A","ID_COMMON_CONTINUE")} else {("B","ID_COMMON_BACK")}],
                    0xffff_ffec,false,"Marker_streak",20.)?;
                if state.rating {
                    view.named(&["mRating"])?.goto(assets,&state.stars.to_string())?;
                    let width=super::hud_leaderboard_movie::width(super::pause_movie::child(view,"mButtonHelp")?,assets)?;
                    let rating=super::pause_movie::child(view,"mRating")?;
                    let x=rating.clip.properties.entry("rating_x".into()).or_insert_with(||rating.entry.matrix.tx.to_string()).parse::<f32>()?;
                    rating.entry.matrix.tx=x-width;rating.clip.script_transform=true;
                }
            }
            _=>return Err("unknown Skate.Reel movie".into()),
        }Ok(())
    }
}
fn label(clip:&mut Clip,assets:&Assets,key:&str)->RenderResult<()> {clip.text=Some(assets.localize(key)?.encode_utf16().collect());Ok(())}
fn selection(clip:&mut Clip,assets:&Assets,state:&str)->RenderResult<()> {
    if clip.properties.get("reel_selection").map(String::as_str)!=Some(state) {
        clip.goto(assets,state)?;clip.properties.insert("reel_selection".into(),state.into());
    }Ok(())
}
pub(super) fn action(clip:&mut Clip,offset:usize)->RenderResult<()> {
    match (clip.asset,clip.character,offset) {
        (ONLINE,0,0xc4c)|(GENERAL,0,0x5c20|0x657c)|(PHOTO,0,0x2d00)=>(),
        (ONLINE,0,0xe80)|(GENERAL,0,0x6558)|(PHOTO,0,0x3208)=>{clip.playback.stop();clip.calls.push("ReelIntro");},
        (ONLINE,0,0xea0)=>clip.calls.push("ReelOutroBegin"),
        (ONLINE,0,0xec4)|(GENERAL,0,0x6598)|(PHOTO,0,0x3234)=>{clip.playback.stop();clip.calls.push("ReelOutro");},
        (GENERAL,51,0x65d4)=>clip.calls.push("FE_SOUND_SCRATCH_BOX_FX"),
        _=>return Err(format!("untranslated Skate.Reel APT {}:{} {offset:#x}",clip.asset,clip.character).into()),
    }Ok(())
}
pub(super) fn init_action(asset:usize,character:u32,offset:usize)->bool {
    matches!((asset,character,offset),(ONLINE,6,0xc68)|(GENERAL,82,0x5c68)|(PHOTO,69,0x2d2c))
}
