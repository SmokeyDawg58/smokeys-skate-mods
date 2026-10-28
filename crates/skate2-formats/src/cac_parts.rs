//! CACPartDB::Initialize82b6ca18, string table82b68590, part table82b6c828.
//! Tables retain their authored order; slots index the original contiguous ranges.
use crate::recipe::{CharacterRecipe, CharacterRecipeError};
use std::ops::Range;

pub const DATABASE_PATH: &str = "data\\content\\recipe\\createacharacter\\cas_db_000.recipe";
pub const SLOT_NAMES: [&str; 26] = ["Arm","Leg","Hair","Rostral","Torso","Feet",
    "Accessory","Equipment","Skeleton","Organ","Misc","SkateBoard","SkateTruck",
    "SkateWheel","Beard","Pants","Underwear","Sock","Hat","Belt","Glasses",
    "WristItem","InnerTorso","OuterTorso","Piercings","Jewellery"];

#[derive(Clone, Debug)]
pub struct CacPartDb {
    pub master: CharacterRecipe,
    pub parts: Vec<CacPart>,
    pub strings: Vec<String>,
    ranges: [Range<usize>; 26],
}
#[derive(Clone, Debug)]
pub struct CacPart {
    pub model_id: u64,
    pub material_id: u64,
    pub cost: u32,
    pub slot: u8,
    pub colourizable: u8,
    pub default: u8,
    pub default_wear_style: u8,
    pub layered: u8,
    pub logoable: u8,
    pub skinned: u8,
    pub used_in_game: u8,
    pub attributes: [u16; 43],
}
#[derive(Debug)]
pub enum CacPartError { Recipe(CharacterRecipeError), Invalid(&'static str), StringIndex(u16), Attribute(u32) }
impl std::fmt::Display for CacPartError {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { write!(f,"CAC part database: {self:?}") }
}
impl std::error::Error for CacPartError {}
impl CacPartDb {
    pub fn parse(bytes: &[u8]) -> Result<Self, CacPartError> {
        use CacPartError as E;
        let (master, consumed) = CharacterRecipe::parse_prefix(bytes).map_err(E::Recipe)?;
        let word = |offset: usize| -> Result<u32,E> {
            let b = bytes.get(offset..offset+4).ok_or(E::Invalid("truncated table"))?;
            Ok(u32::from_be_bytes(b.try_into().unwrap()))
        };
        let start = (consumed+15)&!15;
        let count = word(start)? as usize;
        if count > bytes.len()/4 { return Err(E::Invalid("string count")); }
        let mut strings = Vec::with_capacity(count);
        let mut end = start;
        for i in 0..count {
            let mut offset = word(start+4+i*4)? as usize;
            if offset < start { offset = offset.checked_add(0x10000).ok_or(E::Invalid("string offset"))?; }
            let rest = bytes.get(offset..).ok_or(E::Invalid("string offset"))?;
            let size = rest.iter().position(|&b| b==0).ok_or(E::Invalid("string terminator"))?;
            strings.push(std::str::from_utf8(&rest[..size]).map_err(|_|E::Invalid("string encoding"))?.to_owned());
            end = end.max(offset+size+1);
        }
        let start = (end+15)&!15;
        let count = word(start)? as usize;
        let version = word(start+4)?;
        if !matches!(version,6|7) { return Err(E::Invalid("part table version")); }
        let header = start+8;
        let raw = (header+208+15)&!15;
        let raw_end = raw.checked_add(count.checked_mul(128).ok_or(E::Invalid("part count"))?).ok_or(E::Invalid("part count"))?;
        if raw_end.checked_add(4) != Some(bytes.len()) { return Err(E::Invalid("part table extent")); }
        let mut ranges: [Range<usize>;26] = std::array::from_fn(|_|0..0);
        for (slot, range) in ranges.iter_mut().enumerate() {
            let count = word(header+slot*4)? as usize;
            let first = word(header+104+slot*4)? as usize;
            let last = first.checked_add(count).ok_or(E::Invalid("slot range"))?;
            if last > (raw_end-raw)/128 { return Err(E::Invalid("slot range")); }
            *range=first..last;
        }
        let mut parts=Vec::with_capacity(count);
        for b in bytes[raw..raw_end].chunks_exact(128) {
            parts.push(CacPart { model_id:u64::from_be_bytes(b[..8].try_into().unwrap()),
                material_id:u64::from_be_bytes(b[8..16].try_into().unwrap()), cost:u32::from_be_bytes(b[16..20].try_into().unwrap()),
                slot:b[20], colourizable:b[21], default:b[22], default_wear_style:b[23],layered:b[24],
                logoable:b[25],skinned:b[26],used_in_game:b[27],
                attributes:std::array::from_fn(|i|u16::from_be_bytes([b[28+i*2],b[29+i*2]])) });
        }
        let mut result = Self {master, parts, strings, ranges};
        result.apply_native_fixes()?;
        Ok(result)
    }
    pub fn string(&self,index:u16)->Result<&str,CacPartError> {
        if index == u16::MAX { return Ok(""); }
        self.strings.get(usize::from(index)).map(String::as_str).ok_or(CacPartError::StringIndex(index))
    }
    pub fn attribute(&self,part:&CacPart,attribute:u32)->Result<&str,CacPartError> {
        let index = match attribute {0..=2=>attribute,5..=23=>attribute-2,31..=34=>attribute-9,
            36=>26,38..=53=>attribute-11,_=>return Err(CacPartError::Attribute(attribute))};
        self.string(part.attributes[index as usize])
    }
    pub fn parts_in_slot(&self,slot:u8)->impl Iterator<Item=&CacPart> {
        self.parts[self.slot_range(slot)].iter()
    }
    /// The indices of a slot's parts (`CACPartDB::GetParts`): its authored range, which a runtime
    /// correction of a part's ComponentType leaves it in.
    pub fn slot_range(&self,slot:u8)->Range<usize> {
        self.ranges.get(usize::from(slot)).cloned().unwrap_or(0..0)
    }
    pub fn find(&self,slot:u8,model:u64,material:u64)->Option<&CacPart> {
        self.parts_in_slot(slot).find(|p|p.model_id==model&&p.material_id==material)
    }

    /// Original runtime data corrections, CACDBPartContainer::HackFixData82b6c048.
    /// Slot ranges remain unchanged when a correction changes ComponentType.
    fn apply_native_fixes(&mut self)->Result<(),CacPartError> {
        let mut intern = |s:&str| -> Result<u16,CacPartError> {
            if let Some(i)=self.strings.iter().position(|v|v==s) { return Ok(i as u16); }
            let i=u16::try_from(self.strings.len()).map_err(|_|CacPartError::Invalid("extra strings capacity"))?;
            self.strings.push(s.to_owned()); Ok(i)
        };
        let diamond=intern("diamond")?;let anti_hero=intern("anti_hero")?;let style7=intern("style7")?;
        let index=|s:&str|self.strings.iter().position(|v|v==s).map_or(u16::MAX,|i|i as u16);
        let diamondsupply=index("diamondsupply");let antihero=index("antihero");let zero=index("0");
        let female=index("female");let long_loose=index("long loose");let normal=index("normal");
        let pirate=index("Street_Pirate_Hat2");let newera=index("hat_newera_hMat_77");
        let wind_closed=index("windbreaker_closed_male");let wind_open=index("windbreaker_open_male");
        let polo=index("shortsleeve_polo_shirt_male");let diffuse7=index("diffuse_1_tga7");
        let wind_mat=index("windbreaker_openMATbase");let blinn94=index("blinn94");
        let diffuse179=index("txture_diffuse_179");let diffuse34=index("diffuse_34_tga");
        let female_shirts=[index("shortsleeve_tshirt_medium_female"),index("shortsleeve_tshirt_baggy_female"),index("shortsleeve_tshirt_female")];
        let circa=index("circa");
        for slot in 0..26 {
            for part in &mut self.parts[self.ranges[slot].clone()] {
                let a=&mut part.attributes;
                if a[25]==diamondsupply {a[25]=diamond;} else if a[25]==antihero {a[25]=anti_hero;}
                match slot {
                    2=>{if a[17]==long_loose&&a[13]==female{part.slot=19;} if a[30]==u16::MAX{a[30]=zero;}},
                    3=>{if a[9]==u16::MAX{a[9]=style7;}if a[15]==u16::MAX{a[15]=zero;}},
                    11=>{
                        let name=self.strings.get(a[26] as usize).ok_or(CacPartError::StringIndex(a[26]))?;
                        if name.contains("_stampable_"){if !name.contains("dirty"){part.colourizable=1;}}
                        else if name.ends_with(['_', 'd']) {part.used_in_game=1;}
                    },
                    18=>{if a[26]==pirate||a[26]==newera{part.slot=19;part.default_wear_style=0;}
                        else if a[18]==normal{part.default_wear_style=1;}},
                    23=>{
                        if (a[27]==wind_closed&&a[26]==diffuse7)||(a[27]==wind_open&&a[26]==wind_mat)
                            ||(a[27]==polo&&a[26]==blinn94)||(female_shirts.contains(&a[27])&&a[26]==diffuse179){
                            part.slot=19;a[40]=u16::MAX;
                        }else if female_shirts.contains(&a[27])&&a[26]==diffuse34{a[25]=circa;}
                    },
                    _=>{},
                }
            }
        }
        Ok(())
    }
}
