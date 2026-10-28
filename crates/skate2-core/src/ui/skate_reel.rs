//! MainMenuOnlineOptions23 and the offline path from it to SkateReelGeneral55 and
//! MyFootage56. Native options82643650/826436b0, selection82674b58,
//! general826ab510, local media826a5ec0/826a0c80/826a0e38. Signed in to the dedicated server,
//! Freeskate and Unranked lead to the online matchmaking screens. The port adds Edit Nickname
//! above the native rows, for the nickname `online-config.json` signs in with, and the Lobby
//! Browser below them, and leaves out Ranked, the ranked leaderboard and the Skate profile, which
//! need ranking and profile services the port's server does not run.
use super::{pause_menu::PauseMenuInput,replay_editor::{Footage,FootageItem,Popup}};
pub const ONLINE_LABELS:[&str;6]=["#Edit Nickname","ID_ONLINEOPTIONS_FREEKSKATE","ID_ONLINEOPTIONS_SKATEREELBROWSER",
    "ID_ONLINE_HUB_CREATEASPOT_BROWSER","ID_ONLINEOPTIONS_UNRANKEDONLINE","#Lobby Browser"];
pub const DESCRIPTIONS:[&str;5]=["ID_SKATE_REEL_MY_FOOTAGE_DESC","ID_SKATE_REEL_COMMUNITY_DESC",
    "ID_SKATE_REEL_FRIEND_SHARED_DESC","ID_SKATE_REEL_RECENTLY_SAVED_DESC","ID_SKATE_REEL_RECENTLY_UPLOADED_DESC"];
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Page {Online,General,Footage,Photo(u32),Replay}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Action {None,Back,Browse(bool),Delete(bool,u32),Load(u32),Photo(u32),CreateASpot,EditNickname}
/// What the Online options start online: `GoToCustomFreeSkate` (FE state 68) and
/// `OnlineOptionPopupCallback` 8266a6c0's quick match (`StartUnrankedCustomMatch(Challenge_None,
/// quick)`) and custom match (`GoToUnrankedCustomMatch`, FE state 69), and the Lobby Browser.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum OnlineAction {Freeskate,QuickMatch,CustomMatch,LobbyBrowser}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Snapshot {pub page:Page,pub selected:usize,pub footage:Option<Footage>,pub popup:Option<Popup>,pub rating:bool,pub stars:u8,
    /// OnMenuGetItemEnabled826436b0 of each Online options row.
    pub enabled:[bool;6]}
#[derive(Clone,Copy)]
enum Dialog {Options(bool,u32),Delete(bool,u32),Notice,Match,Matching}
pub struct Controller {
    pub page:Page,pub ready:bool,pub pending:Action,
    selected:usize,online_selected:usize,footage:Option<Footage>,popup:Option<(Dialog,Popup)>,
    sounds:Vec<&'static str>,busy:bool,
    rating:bool,stars:u8,
    /// `Fesl::HubSingle` is `CONNECTION_READY`: signed in to the dedicated server.
    pub signed_in:bool,
    online:Option<OnlineAction>,
}
impl Controller {
    pub fn new()->Self {Self {page:Page::Online,ready:false,pending:Action::None,selected:2,online_selected:2,
        footage:None,popup:None,sounds:Vec::new(),busy:false,rating:false,stars:5,signed_in:false,online:None}}
    pub fn snapshot(&self)->Snapshot {Snapshot {page:self.page,selected:self.selected,rating:self.rating,stars:self.stars,
        footage:if self.page==Page::Footage {self.footage.clone()} else {None},popup:self.popup.as_ref().map(|(_,p)|p.clone()),
        enabled:std::array::from_fn(|row|self.enabled(row))}}
    /// OnMenuGetItemEnabled826436b0 on the rows the port keeps, a row down for Edit Nickname
    /// (always on): Skate.Reel and Create-a-Spot always; Freeskate and Unranked only when
    /// connected, as the Lobby Browser.
    fn enabled(&self,row:usize)->bool {matches!(row,0|2|3)||(self.signed_in&&matches!(row,1|4|5))}
    /// The online screen the Online options asked for.
    pub fn take_online_action(&mut self)->Option<OnlineAction> {self.online.take()}
    /// A failed sign-in or matchmaking: its notice over the Online options.
    pub fn online_notice(&mut self,description:&str) {self.busy=false;
        self.popup(Dialog::Notice,"ID_EANATION",description,&["ID_COMMON_OK"],false);}
    /// `OnlineBackEnd::OpenLoadingPopup` (0x825665e8) while a quick match searches: popup 38,
    /// `ID_ONLINE_MATCHING_SKATERS` with no options, not cancellable.
    pub fn matching(&mut self) {self.popup(Dialog::Matching,"ID_EANATION","ID_ONLINE_MATCHING_SKATERS",&[],false);}
    pub fn take_sounds(&mut self)->Vec<&'static str> {std::mem::take(&mut self.sounds)}
    /// The sounds of a screen over the Online options.
    pub fn extend_sounds(&mut self,sounds:Vec<&'static str>) {self.sounds.extend(sounds);}
    pub fn take_action(&mut self)->Action {std::mem::replace(&mut self.pending,Action::None)}
    fn page(&mut self,page:Page) {self.page=page;self.ready=false;}
    fn popup(&mut self,kind:Dialog,title:&str,description:&str,options:&[&str],cancellable:bool) {
        self.sounds.push("FE_SOUND_CORE_POPUP");self.popup=Some((kind,Popup {title:title.into(),description:description.into(),
            options:options.iter().map(|s|(*s).into()).collect(),selected:0,cancellable}));
    }
    pub fn failed(&mut self,message:&str) {self.busy=false;
        self.popup(Dialog::Notice,"ID_COMMON_WARNING_TITLE",&format!("#{message}"),&["ID_COMMON_OK"],false);}
    pub fn localized_notice(&mut self,description:&str) {self.busy=false;
        self.popup(Dialog::Notice,"ID_COMMON_WARNING_TITLE",description,&["ID_COMMON_OK"],false);}
    pub fn loaded(&mut self,replay:bool,items:Vec<FootageItem>) {
        if let Some(b)=self.footage.as_mut().filter(|b|b.replay==replay) {
            b.items=items;b.selected=b.selected.min(b.items.len().saturating_sub(1));
            b.first=(b.first.min(b.selected)/3)*3;b.slider_start=b.selected.saturating_sub(5);
        } else {self.footage=Some(Footage {replay,items,selected:0,first:0,slider_start:0,ready:false});}
        self.busy=false;
    }
    pub fn replay_opened(&mut self) {self.busy=false;self.page(Page::Replay);}
    pub fn replay_closed(&mut self) {self.page(Page::Footage);}
    pub fn photo_opened(&mut self,slot:u32) {self.busy=false;self.rating=false;self.stars=5;self.page(Page::Photo(slot));}
    pub fn update(&mut self,input:PauseMenuInput,focused:bool)->Action {
        if !focused||!self.ready||self.busy {return Action::None;}
        if let Some((kind,popup))=self.popup.as_mut() {
            if input.cancel&&popup.cancellable {self.popup=None;self.sounds.push("FE_SOUND_CORE_B_BUTTON");return Action::None;}
            // A popup without options has no picker items to move between or select.
            if popup.options.is_empty() {return Action::None;}
            for (pressed,up) in [(input.up,true),(input.down,false)] {if pressed {
                popup.selected=(popup.selected+if up {popup.options.len()-1} else {1})%popup.options.len();
                self.sounds.push(if up {"FE_SOUND_CORE_NAV_UP"} else {"FE_SOUND_CORE_NAV_DOWN"});
            }}
            if !input.select {return Action::None;}
            let (kind,index)=(*kind,popup.selected);self.popup=None;self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            match kind {
                // OnlineOptionPopupCallback8266a6c0: Quick or Custom.
                Dialog::Match=>{self.online=Some(if index==0 {OnlineAction::QuickMatch} else {OnlineAction::CustomMatch});}
                Dialog::Options(replay,slot) if index==0=>{self.busy=true;return if replay {Action::Load(slot)} else {Action::Photo(slot)};}
                Dialog::Options(replay,slot)=>self.popup(Dialog::Delete(replay,slot),"ID_COMMON_WARNING_TITLE",
                    if replay {"ID_MEMCARD_PROMPT_DELETE_REPLAY"} else {"ID_MEMCARD_PROMPT_DELETE_PHOTO"},&["ID_COMMON_NO","ID_COMMON_YES"],false),
                Dialog::Delete(replay,slot) if index==1=>{self.busy=true;return Action::Delete(replay,slot);}
                _=>()
            }
            return Action::None;
        }
        if input.cancel {
            // ReceiveInput826a6400 ignores B after entering eRatePhoto.
            if matches!(self.page,Page::Photo(_))&&self.rating {return Action::None;}
            self.sounds.push("FE_SOUND_CORE_B_BUTTON");
            match self.page {
                Page::Online=>return Action::Back,
                Page::General=>{self.selected=self.online_selected;self.page(Page::Online);}
                Page::Footage=>{self.selected=0;self.page(Page::General);}
                // IsCurrentMyPhoto8268f470 returns false with no FESL user;
                // the original offline B path still enters the rating panel.
                Page::Photo(_)=>{self.rating=true;self.stars=5;},Page::Replay=>()
            }return Action::None;
        }
        match self.page {
            Page::Online=>{
                for (pressed,up) in [(input.up,true),(input.down,false)] {if pressed {
                    // The picker steps over the rows OnMenuGetItemEnabled826436b0 disables.
                    let count=ONLINE_LABELS.len();
                    for step in 1..count {
                        let row=if up {(self.selected+count*step-step)%count} else {(self.selected+step)%count};
                        if self.enabled(row) {self.selected=row;break;}
                    }
                    self.sounds.push(if up {"FE_SOUND_CORE_NAV_UP"} else {"FE_SOUND_CORE_NAV_DOWN"});
                }}
                if input.select {self.sounds.push("FE_SOUND_CORE_A_BUTTON");self.online_selected=self.selected;
                    // OnMenuSelect82674b58.
                    match self.selected {
                        0=>return Action::EditNickname,
                        1=>self.online=Some(OnlineAction::Freeskate),
                        2=>{self.selected=0;self.page(Page::General);}
                        3=>return Action::CreateASpot,
                        // InitOnlineMenuPopup826713e8.
                        4=>self.popup(Dialog::Match,"ID_ONLINEOPTIONS_UNRANKEDONLINE","ID_ONLINEOPTIONS_CHOOSE_MATCH",
                            &["ID_ONLINEOPTIONS_QUICK_MATCH","ID_ONLINEOPTIONS_CUSTOM_MATCH"],true),
                        5=>self.online=Some(OnlineAction::LobbyBrowser),
                        _=>{}
                    }}
            }
            Page::General=>{
                // Original five-tile grid: top row2, bottom row3; vertical
                // navigation maps the rightmost columns together.
                for (pressed,vertical,left,sound) in [(input.up,true,true,"FE_SOUND_SKATE_REEL_NAV_UP"),
                    (input.down,true,false,"FE_SOUND_SKATE_REEL_NAV_DOWN"),(input.left,false,true,"FE_SOUND_SKATE_REEL_NAV_LEFT"),
                    (input.right,false,false,"FE_SOUND_SKATE_REEL_NAV_RIGHT")] {if pressed {
                    self.selected=if vertical {match self.selected {0=>2,1=>4,2=>0,_=>1}}
                        else if self.selected<2 {1-self.selected} else {2+(self.selected-2+if left {2} else {1})%3};
                    self.sounds.push(sound);
                }}
                if input.select {if self.selected==0 {self.page(Page::Footage);self.busy=true;return Action::Browse(true);}
                    self.popup(Dialog::Notice,"ID_EANATION","ID_ONLINE_PLEASE_SIGN_IN",&["ID_COMMON_OK"],false);}
            }
            Page::Footage=>{
                let Some(b)=&mut self.footage else {return Action::None};
                if input.misc {let replay=!b.replay;self.busy=true;self.sounds.push("FE_SOUND_CORE_X_BUTTON");return Action::Browse(replay);}
                for (pressed,key) in [(input.up,12),(input.down,13),(input.left,14),(input.right,15)] {if pressed {
                    b.navigate(key);self.sounds.push(match key {12=>"FE_SOUND_CORE_NAV_UP",13=>"FE_SOUND_CORE_NAV_DOWN",14=>"FE_SOUND_CORE_NAV_LEFT",_=>"FE_SOUND_CORE_NAV_RIGHT"});
                }}
                if input.select&&let Some(item)=b.items.get(b.selected) {
                    let (replay,slot)=(b.replay,item.slot);
                    self.popup(Dialog::Options(replay,slot),if replay {"ID_SKATE_REEL_REPLAY_MOVIE"} else {"ID_SKATEREEL_PHOTO"},"#",
                        &[if replay {"ID_SKATEREEL_LOAD_REPLAY"} else {"ID_SKATEREEL_VIEW_PHOTO"},"ID_SKATEREEL_DELETE_PHOTO"],true);
                }
            }
            Page::Photo(_)=>{
                if self.rating {if input.left {self.stars=self.stars.saturating_sub(1);self.sounds.push("FE_SOUND_CORE_NAV_LEFT");}
                    if input.right {self.stars=(self.stars+1).min(10);self.sounds.push("FE_SOUND_CORE_NAV_RIGHT");}}
                if input.select {self.sounds.push("FE_SOUND_CORE_A_BUTTON");self.page(Page::Footage);}
            }
            Page::Replay=>()
        }
        Action::None
    }
}
