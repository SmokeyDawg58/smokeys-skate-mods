//! FrontEndState_PhoneList82681980/82689d78, screen13. The authored phone
//! options select service scripts or real challenge activities and competitors.
use super::{activity, attrib_db::AttribDb, competitors::CompetitorInfo, film,
    services, signpost_progress, signup, sponsors,
    system::{Output, World}};
use crate::ui::pause_menu::PauseMenuInput;

const COMPETITORS: u64 = 0x9c86_9217_c5ae_e9b4;
const LOCATIONS: u64 = 0x1162_b4a4_9518_a203;
const TITLES: [&str;12] = ["ID_PHONELIST_USE_PHONE", "ID_PHONELIST_SERVICES_TITLE",
    "ID_PHONELIST_CHOOSE_LOCATION", "ID_PHONELIST_CHOOSE_CHALLENGE",
    "ID_PHONELIST_SELECT_PRO", "ID_PHONELIST_SELECT_PRO_1", "ID_PHONELIST_SELECT_PRO_2",
    "ID_PHONELIST_SELECT_PRO_3", "ID_PHONELIST_FILM_ANYWHERE_TITLE",
    "ID_PHONELIST_PICK_A_PRO_TITLE", "ID_PHONELIST_NOT_ENOUGH_CASH_TITLE", "ID_PHONE_COMPLETED_FILMS"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    pub key: String,
    pub number: Option<i32>,
    /// GetOptionText8265d828: the original '#\xbb %s' unbeaten prefix.
    pub unbeaten: bool,
}
impl Text {
    fn key(key: impl Into<String>) -> Self { Self {key:key.into(),number:None,unbeaten:false} }
    fn number(key: &str, number: i32) -> Self { Self {number:Some(number),..Self::key(key)} }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub page: usize,
    pub title: &'static str,
    pub options: Vec<Text>,
    pub selected: usize,
    pub description: Text,
    pub banner: &'static str,
    pub chosen: Vec<String>,
    pub cash: i32,
    pub select_enabled: bool,
    pub not_enough_cash: bool,
}
#[derive(Clone, Copy, Debug)]
enum Choice { None, Menu(usize), Service(i32), Location(u64), Pro(u64), PickPro(u64), Done, Film(usize) }
#[derive(Debug)]
pub struct Contacts {
    stack: Vec<usize>,
    selections: [usize;12],
    locations: Vec<u64>,
    all_locations: usize,
    all_pros: Vec<u64>,
    pros: Vec<u64>,
    pick_pros: Vec<u64>,
    chosen: Vec<u64>,
    location: u64,
    choices: Vec<Choice>,
    pub intro_complete: bool,
    pub view: View,
}

fn short(db:&AttribDb, class:u64, key:u64, offset:u32)->i32 {
    let word=db.layout_word(class,key,offset/4);
    let half=if offset%4==0 {word>>16} else {word};
    i32::from(half as u16 as i16)
}
fn quad(db:&AttribDb, class:u64,key:u64,offset:u32)->u64 {
    (u64::from(db.layout_word(class,key,offset/4))<<32)|u64::from(db.layout_word(class,key,offset/4+1))
}
fn message(world:&World, id:i32, default:bool)->bool {
    world.signpost.message_setting_value(&format!("messagesettingtype_{id}"))
        .map_or(default,|v|v!=0)
}
fn label(world:&World,class:u64,key:u64,offset:u32)->String {
    world.db.layout_text(class,key,offset/4).unwrap_or_default().to_owned()
}

/// PhonelistHUD::CanShowPhoneListOptions826b5d90, distinct from whether
/// any Contacts entry can be selected once its screen is opened.
pub fn can_show_options(world:&World)->bool {
    world.game_mode_flags==4 && services::message_setting(world,"Uncapping Available")
        && world.activities.main.is_none_or(|a|signup::challenge_type(world,a)==9)
}

/// PhonelistHUD::CanShowPhoneListAlerts826b5ea0: a single player game (IsSinglePlayerMode, the
/// career or a free skate) whose uncapping service progression unlocked, with no main challenge.
pub fn can_show_alerts(world:&World)->bool {
    matches!(world.game_mode_flags,2|4) && services::message_setting(world,"Uncapping Available")
        && world.activities.main.is_none()
}

/// The phone list HUD's alert facts: CanShowPhoneListAlerts and the settings GetSponsorPP826b6328
/// and GetAnywhereFilmPP826b6428 read.
pub fn alert_facts(world:&World)->crate::ui::phonelist::AlertFacts {
    crate::ui::phonelist::AlertFacts {can_show_alerts:can_show_alerts(world),
        sponsor_pinch_point:services::message_setting(world,"Sponsor PinchPoint"),
        anywhere_film_pinch_point:services::message_setting(world,"Anywhere Film PinchPoint")}
}

impl Contacts {
    /// OnEnter826876b0: leaf collections, original unlock settings and order.
    pub fn open(world:&World, saving:bool)->Self {
        let all=world.db.leaf_collections(LOCATIONS);
        let locations=all.iter().copied().filter(|&k|message(world,short(&world.db,LOCATIONS,k,52),true)).collect();
        let all_pros=world.db.leaf_collections(COMPETITORS);
        let pros:Vec<_>=all_pros.iter().copied().filter(|&k|message(world,short(&world.db,COMPETITORS,k,88),true)).collect();
        let sponsor=sponsors::board_sponsor_key(world).unwrap_or(u64::MAX) as u32;
        let pick_pros=pros.iter().copied().filter(|&k|world.db.layout_byte(COMPETITORS,k,76)!=0
            && world.db.layout_word(COMPETITORS,k,13)!=sponsor).collect();
        let mut this=Self {stack:vec![0],selections:[0;12],locations,all_locations:all.len(),
            all_pros,pros,pick_pros,chosen:Vec::new(),location:u64::MAX,choices:Vec::new(),intro_complete:false,
            view:View{page:0,title:TITLES[0],options:Vec::new(),selected:0,
                description:Text::key("ID_STRING_MISSING"),banner:"bb",chosen:Vec::new(),
                cash:0,select_enabled:false,not_enough_cash:false}};
        this.refresh(world,saving);
        this
    }
    fn refresh_pros(&mut self,world:&World) {
        self.pros=self.all_pros.iter().copied().filter(|k|!self.chosen.contains(k)
            && message(world,short(&world.db,COMPETITORS,*k,88),true)).collect();
    }
    fn push(&mut self,world:&World,page:usize,saving:bool) {
        self.stack.push(page); self.selections[page]=0;
        if matches!(page,4|5) {self.chosen.clear();}
        self.refresh(world,saving);
    }
    fn refresh(&mut self,world:&World,saving:bool) {
        let page=*self.stack.last().expect("open Contacts menu");
        let mut choices=Vec::new();
        match page {
            0=>{
                let service=world.services.big_black_active || (0..3).any(|s|services::available(world,s));
                let main=activity::can_start_main_challenge(world);
                let throw=main && !self.locations.is_empty() && !self.pros.is_empty()
                    && world.signpost.key_from_title("t_1b_01_skate_alphabet_shingo")
                        .is_some_and(|k|world.signpost.is_challenge_complete(k));
                let pick=main && !self.pick_pros.is_empty() && (signpost_progress::check_ranking(world,"SBM2")
                    ||signpost_progress::check_ranking(world,"Thrasher2"));
                if main && !world.services.introduction_in_progress && !saving
                    && (service||throw||pick||film::available(world,false)||film::available(world,true)) {
                    if service {choices.push(Choice::Menu(1));}
                    if throw {choices.push(Choice::Menu(2));}
                    if film::available(world,false) {choices.push(Choice::Menu(8));}
                    if film::available(world,true) {choices.push(Choice::Menu(11));}
                    if pick {choices.push(Choice::Menu(9));}
                } else {choices.push(Choice::None);}
            },
            1=>{
                for service in [0,2,1] {
                    if services::available(world,service)||(service==0&&world.services.big_black_active) {
                        choices.push(Choice::Service(service));
                    }
                }
            },
            2=>choices.extend(self.locations.iter().copied().map(Choice::Location)),
            4..=7=>{
                if page>=6 {choices.push(Choice::Done);}
                choices.extend(self.pros.iter().copied().map(Choice::Pro));
            },
            8|11=>choices.extend(film::list(world,page==11).iter().copied().map(Choice::Film)),
            9=>choices.extend(self.pick_pros.iter().copied().map(Choice::PickPro)),
            _=>{},
        }
        self.choices=choices;
        let selected=self.selections[page].min(self.choices.len().saturating_sub(1));
        self.selections[page]=selected;
        let options=self.choices.iter().map(|c|match *c {
            Choice::None=>Text::key("ID_PHONELIST_NO_OPTIONS_AVAILABLE"),
            Choice::Menu(p)=>Text::key(match p {2=>"ID_PHONELIST_THROWDOWN_TITLE",_=>TITLES[p]}),
            Choice::Service(0) if world.services.big_black_active=>Text::key("ID_PHONELIST_SERVICES_BIG_BLACK_SECURITY_DEACTIVATE_TITLE"),
            Choice::Service(s)=>Text::number(match s {
                0=>"ID_PHONELIST_SERVICES_BIG_BLACK_SECURITY_ACTIVATE_TITLE",
                1=>"ID_PHONELIST_SERVICES_MIKES_UNCAPPING_TITLE",
                _=>"ID_PHONELIST_SERVICES_SAMMIS_POOL_DRAINAGE_TITLE"},services::cost(s)),
            Choice::Location(k)=>Text::key(label(world,LOCATIONS,k,48)),
            Choice::Pro(k)=>Text{unbeaten:message(world,short(&world.db,COMPETITORS,k,92),true),
                ..Text::key(label(world,COMPETITORS,k,72))},
            Choice::PickPro(k)=>Text::key(label(world,COMPETITORS,k,68)),
            Choice::Done=>Text::key("ID_PHONELIST_THROWDOWN_COMPLETE"),
            Choice::Film(a)=>Text::key(film::info(world,a).0),
        }).collect();
        let selected_choice=self.choices.get(selected).copied().unwrap_or(Choice::None);
        let description=if page==2 {
            if self.locations.len()<self.all_locations {
                Text::number("ID_PHONELIST_LOCATION_NEXT_UNLOCK",world.signpost.message_setting_value("Next Unlock").unwrap_or(0) as i32)
            } else {
                let remaining=self.all_pros.iter().filter(|&&k|message(world,short(&world.db,COMPETITORS,k,92),false)).count() as i32;
                if remaining>0 {Text::number("ID_PHONELIST_LOCATION_PROS_UNBEATEN",remaining)}
                else {Text::number("\tID_PHONELIST_LOCATION_WINNING_STREAK",world.signpost.message_setting_value("Current Win Streak").unwrap_or(0) as i32)}
            }
        } else if let Choice::Film(a)=selected_choice {Text::key(film::info(world,a).1)}
        else {Text::key("ID_STRING_MISSING")};
        self.view=View {page,title:TITLES[page],options,selected,description,
            banner:match selected_choice {Choice::Service(1)=>"mikes",Choice::Service(2)=>"sammis",_=>"bb"},
            chosen:self.chosen.iter().map(|&k|label(world,COMPETITORS,k,72)).collect(),
            cash:signpost_progress::cash(world),select_enabled:!matches!(selected_choice,Choice::None),
            not_enough_cash:self.view.not_enough_cash};
    }

    /// Returns true when PopState closes Contacts. Menu picker clamps and the
    /// native B key pops one menu (including undoing the last pro selection).
    pub fn input(&mut self,world:&mut World,input:PauseMenuInput,saving:bool,sounds:&mut Vec<&'static str>)->bool {
        if !self.intro_complete {return false;}
        if self.view.not_enough_cash {
            if input.select {self.view.not_enough_cash=false;sounds.push("FE_SOUND_CORE_A_BUTTON");}
            return false;
        }
        if input.cancel {
            sounds.push("FE_SOUND_CELLPHONE_B_BUTTON");
            if self.stack.len()==1 {return true;}
            let previous=self.stack.pop().expect("Contacts stack");
            if matches!(previous,6|7) {self.chosen.pop();self.refresh_pros(world);}
            if matches!(self.stack.last(),Some(4|5)) {self.chosen.clear();}
            self.refresh(world,saving);
            return false;
        }
        let page=self.view.page;
        let count=self.choices.len();
        if count>1 {
            if input.up && self.selections[page]>0 {self.selections[page]-=1;sounds.push("FE_SOUND_CELLPHONE_NAVIGATE");}
            if input.down && self.selections[page]+1<count {self.selections[page]+=1;sounds.push("FE_SOUND_CELLPHONE_NAVIGATE");}
            if input.up||input.down {self.refresh(world,saving);}
        }
        if !input.select {return false;}
        sounds.push("FE_SOUND_CELLPHONE_A_BUTTON");
        match self.choices.get(self.selections[page]).copied().unwrap_or(Choice::None) {
            Choice::None=>{},
            Choice::Menu(p)=>self.push(world,p,saving),
            Choice::Service(0) if world.services.big_black_active=>{services::deactivate_security(world);return true;},
            Choice::Service(s)=>{
                // OnMenuSelect82689d78 rechecks availability for uncapping and
                // draining; security checks the active service and cash only.
                if s==0 || services::available(world,s) {
                    if signpost_progress::cash(world)>=services::cost(s) {services::activate(world,s);return true;}
                    self.view.not_enough_cash=true;sounds.push("FE_SOUND_CORE_POPUP");
                }
            },
            Choice::Location(k)=>{
                self.location=k;
                self.push(world,if world.signpost.message_setting_value("Multiple Pros")==Some(1) {5}else{4},saving);
            },
            Choice::Pro(k)=>{
                self.chosen.push(k);
                answer(world,k,false);
                self.refresh_pros(world);
                if matches!(page,5|6) {self.push(world,page+1,saving);}
                else {self.start_throwdown(world);return true;}
            },
            Choice::Done=>{self.start_throwdown(world);return true;},
            Choice::PickPro(k)=>{answer(world,k,true);start_pick_pro(world,k);return true;},
            Choice::Film(a)=>{film::start(world,a);return true;},
        }
        false
    }
    /// StartThrowdownChallenge8265e168: stored completion message and the
    /// SKATE profile at +24; location's SKATE activity at ClassRefSpec+8.
    fn start_throwdown(&self,world:&mut World) {
        for (i,&k) in self.chosen.iter().enumerate() {
            world.competitors.completed_messages[i]=quad(&world.db,COMPETITORS,k,8);
            world.competitors.infos[i]=Some(competitor_info(world,k,24));
        }
        let key=quad(&world.db,LOCATIONS,self.location,8);
        if activity::can_start_main_challenge(world) && let Some(a)=world.activities.find(key) {
            signup::player_trigger_sign_up(world,a,true);
        }
    }
}
fn competitor_info(world:&World,key:u64,offset:u32)->CompetitorInfo {
    CompetitorInfo{profile_name:world.db.layout_text(COMPETITORS,key,offset/4).map(str::to_owned),
        root_category:world.db.layout_word(COMPETITORS,key,offset/4+1) as i32}
}
fn answer(world:&mut World,key:u64,pick:bool) {
    let character=world.db.layout_word(COMPETITORS,key,20) as i32;
    let id=if pick {world.db.attribute(COMPETITORS,key,0x7945_1a98_71f5_a60e)
        .and_then(|a|a.value_bytes(0)).filter(|b|b.len()>=2).map_or(0,|b|i32::from(i16::from_be_bytes([b[0],b[1]])))}
        else {short(&world.db,COMPETITORS,key,84)};
    world.output(Output::PlayNpcSpeech{character,id,param:0});
}
/// StartPickAProChallenge8265de20, verified PPC offsets +32/+80.
fn start_pick_pro(world:&mut World,key:u64) {
    world.competitors.pro_film_pro=Some(key);
    world.competitors.infos[0]=Some(competitor_info(world,key,32));
    world.competitors.pro_beaten_message=world.db.attribute(COMPETITORS,key,0x2013_5bc5_613e_9280)
        .and_then(|a|a.ref_spec(0)).map_or(0,|r|r.1);
    world.competitors.audio_character=world.db.layout_word(COMPETITORS,key,20) as i32;
    let challenge=world.db.attribute(COMPETITORS,key,0x9be0_dd2b_3917_2d26).and_then(|a|a.ref_spec(0)).map(|r|r.1);
    if activity::can_start_main_challenge(world) && let Some(a)=challenge.and_then(|k|world.activities.find(k)) {
        world.teleport.is_pick_a_pro=true;
        signup::player_trigger_sign_up(world,a,true);
    }
}
