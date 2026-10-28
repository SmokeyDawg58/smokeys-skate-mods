//! FrontEndState_TrickGuide: PopulateMenuLevel826834c8, OnMenuHilight8266c238,
//! SetNewTrickName82660838 and ReceiveInput8266c300. Catalog and previews are
//! read from the original Vault, including the leaf Actions/Terminology lists.
use crate::challenge::{attrib_db::AttribDb, tutorial_hud::{TrickSlot, collection_slots}};
use super::pause_menu::PauseMenuInput;
use std::sync::Arc;
use skate2_formats::attrib::string_hash64;

const CATEGORIES: [u64; 7] = [
    0x967345ffa6169017, 0x9e6279c16d46f6b3, 0x0ae5c05d32117e6c,
    0x046ed0ccbda1091f, 0xffe1bdbf3b1b7b7c, 0xf5c5f450b0c8703e, 0xd6ca2a68ffe6a743,
];
const SUBCATEGORIES: u64 = 0x57035d975f71d506;
const HAS_SUBMENU: u64 = 0xa990472b974734c3;
const SUBCATEGORY: u64 = 0x06513cc28a532524;
const ANIMATION: u64 = 0x3b71ea27d53d7c4f;
const CAMERA: u64 = 0x3d3a79f16dcb54bb;
pub const DEFAULT_ANIMATION: &str = "trickguide_fs_noseblunt_01";

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub level: usize,
    pub selected: usize,
    pub labels: Vec<String>,
    pub description: String,
    pub slots: Vec<TrickSlot>,
    pub has_submenu: bool,
    pub goofy: bool,
    pub stance_regular:bool,
    pub ready: bool,
}
pub struct Controller {
    db: Arc<AttribDb>,
    menus: [Vec<u64>; 3],
    history: [usize; 3],
    level: usize,
    selected: usize,
    leaf: bool,
    slots: Vec<TrickSlot>,
    pub animation: String,
    pub camera: u32,
    pub ready: bool,
    pub exit: bool,
    pub goofy: bool,
    sounds: Vec<&'static str>,
}
impl Controller {
    pub fn new(db: Arc<AttribDb>, goofy: bool) -> Self {
        Self { db, menus: [CATEGORIES.to_vec(), vec![], vec![]], history: [0;3],
            level:0, selected:0, leaf:false, slots:vec![], animation:DEFAULT_ANIMATION.into(),
            camera:1, ready:false, exit:false, goofy, sounds:vec![] }
    }
    fn class(&self) -> u64 {
        string_hash64(["scoring_trickcategory", "scoring_tricksubcategory", "scoring_trick"][self.level])
    }
    fn collection(&self) -> u64 { self.menus[self.level][self.selected] }
    fn has_submenu(&self) -> bool {
        self.level == 0 || (self.level == 1 && self.db.attribute(self.class(),self.collection(),HAS_SUBMENU)
            .and_then(|a| a.data(0)).and_then(|p| self.db.bytes(p,1)).is_some_and(|b| b[0]!=0))
    }
    pub fn snapshot(&self) -> Snapshot {
        let class=self.class();
        Snapshot {level:self.level,selected:self.selected,
            labels:self.menus[self.level].iter().map(|&id| self.db.layout_text(class,id,u32::from(self.level==1)).unwrap_or("").into()).collect(),
            description:self.db.layout_text(class,self.collection(),if self.level==1 {2}else{1}).unwrap_or("").into(),
            // sMethod_IsGoofy82623fe8 returns false in this build, independently
            // of the actual stance used by PlayerStance_IsRegular for buttons.
            slots:self.slots.clone(),has_submenu:self.has_submenu(),goofy:false,stance_regular:!self.goofy,ready:self.ready}
    }
    fn highlight(&mut self) {
        if !(self.leaf || self.level==2) { self.slots.clear(); return; }
        let (class,id)=(self.class(),self.collection());
        self.slots=collection_slots(&self.db,class,id);
        let animation=if self.level==1 {self.db.layout_text(class,id,0)} else {
            self.db.attribute(class,id,ANIMATION).and_then(|a|a.text(0))
        };
        if let Some(animation)=animation { self.animation=animation.into(); }
        // This is ClassRefSpec_trickbook_camera_tracks, not Attrib::RefSpec:
        // collection key first, followed by the cached collection pointer.
        // Constructor82653e50 supplies the class and a two-byte default layout;
        // SetNewTrickName82660994/82660a80 reads its signed short with lha.
        let key=self.db.attribute(class,id,CAMERA).and_then(|a|a.u64(0)).unwrap_or(0);
        let camera_class=string_hash64("trickbook_camera_tracks");
        self.camera=i16::from_be_bytes([self.db.layout_byte(camera_class,key,0),
            self.db.layout_byte(camera_class,key,1)]) as i32 as u32;
    }
    fn select(&mut self) {
        self.history[self.level]=self.selected;
        if self.level==0 {
            self.menus[1]=self.db.attribute(self.class(),self.collection(),SUBCATEGORIES)
                .map(|a|(0..a.len()).filter_map(|i|a.ref_spec(i).map(|(_,id)|id)).collect()).unwrap_or_default();
            if self.menus[1].is_empty() {return;}
            self.level=1;self.selected=0;
            self.leaf=!self.has_submenu();
            if self.menus[1].len()<2 { self.populate_tricks(); }
        } else if self.level==1 { self.populate_tricks(); }
        self.highlight();
    }
    fn populate_tricks(&mut self) {
        let sub=self.collection(); let class=string_hash64("scoring_trick");
        let mut tricks:Vec<_>=self.db.table_order(class).into_iter().filter(|&id| {
            self.db.attribute(class,id,SUBCATEGORY).and_then(|a|a.ref_spec(0)).is_some_and(|(_,k)|k==sub)
        }).collect();
        sort_tricks(&mut tricks,|id|self.db.layout_word(class,id,32) as i32);
        if !tricks.is_empty() {self.menus[2]=tricks;self.level=2;self.selected=0;}
    }
    pub fn update(&mut self,input:PauseMenuInput,focused:bool) {
        if !focused || !self.ready || self.exit {return;}
        if input.cancel {
            self.sounds.push("FE_SOUND_CORE_B_BUTTON");
            if self.level==0 {self.exit=true;return;}
            self.menus[self.level].clear();self.level-=1;
            if self.menus[self.level].len()<2 {self.menus[self.level].clear();self.level-=1;}
            self.selected=0;self.leaf=false;self.highlight();
        } else if input.up || input.down {
            let count=self.menus[self.level].len();
            // APT SetState enables wrap only for lists fitting its seven rows.
            self.selected=if count<=7 {(self.selected+if input.down {1}else{count-1})%count}
                else if input.down {(self.selected+1).min(count-1)} else {self.selected.saturating_sub(1)};
            self.sounds.push(if input.down {"FE_SOUND_CORE_NAV_DOWN"}else{"FE_SOUND_CORE_NAV_UP"});self.highlight();
        } else if input.select && self.has_submenu() {
            self.sounds.push("FE_SOUND_CORE_A_BUTTON"); self.select();
        }
    }
    pub fn take_sounds(&mut self)->Vec<&'static str> {std::mem::take(&mut self.sounds)}
}

/// EASTL quick_sort82676c68 / partition8266fa00 / insertion82652080. Both
/// the Vault table walk and partition swaps matter for equal sort indices.
fn sort_tricks(items:&mut [u64],key:impl Fn(u64)->i32+Copy) {
    fn adjust(a:&mut [u64],top:usize,n:usize,mut pos:usize,value:u64,key:impl Fn(u64)->i32+Copy) {
        let mut child=2*(pos+1);
        while child<n {
            if key(a[child])<key(a[child-1]) {child-=1;}
            a[pos]=a[child];pos=child;child=2*(child+1);
        }
        if child==n {a[pos]=a[child-1];pos=child-1;}
        while pos>top {
            let parent=(pos-1)/2;
            if key(value)<=key(a[parent]) {break;}
            a[pos]=a[parent];pos=parent;
        }
        a[pos]=value;
    }
    fn partition_sort(mut a:&mut [u64],mut depth:u32,key:impl Fn(u64)->i32+Copy) {
        while a.len()>28 && depth>0 {
            let (x,y,z)=(key(a[0]),key(a[a.len()/2]),key(a[a.len()-1]));
            let pivot=if x<y {if y<z {y}else if x<z {z}else{x}}
                else if x<z {x}else if y<z {z}else{y};
            let (mut lo,mut hi)=(0,a.len());
            loop {
                while key(a[lo])<pivot {lo+=1;}
                hi-=1;while pivot<key(a[hi]) {hi-=1;}
                if lo>=hi {break;}
                a.swap(lo,hi);lo+=1;
            }
            depth-=1;let (left,right)=a.split_at_mut(lo);
            partition_sort(right,depth,key);a=left;
        }
        if depth==0 && a.len()>1 {
            let n=a.len();
            for parent in (0..=(n-2)/2).rev(){let v=a[parent];adjust(a,parent,n,parent,v,key);}
            for n in (2..=a.len()).rev(){let v=a[n-1];a[n-1]=a[0];adjust(a,0,n-1,0,v,key);}
        }
    }
    if items.is_empty(){return;}
    let depth=2*(usize::BITS-items.len().leading_zeros())-2;
    partition_sort(items,depth,key);
    for i in 1..items.len() {
        let v=items[i];let mut j=i;
        while j>0 && key(v)<key(items[j-1]){items[j]=items[j-1];j-=1;}
        items[j]=v;
    }
}
