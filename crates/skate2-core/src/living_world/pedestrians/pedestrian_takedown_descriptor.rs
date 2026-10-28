//! Anim::tAnimTakedown PDB layout and live Vault descriptor reads.
use super::PedestrianConstruction;
pub(crate) const TAKE_DOWN_ARRAY:u64=0x76ac_331f_bd58_03ff;
const TAKE_DOWN_TYPE:u64=0x8ca6_a472_b877_60dc;
#[derive(Clone,Debug)]
pub(crate) struct TakeDownDescriptor {
    pub(crate) attack_name:String,
    pub(crate) mirrored_attack_name:String,
    pub(crate) time:f32,
    pub(crate) min_distance:f32,
    pub(crate) max_distance:f32,
    pub(crate) right_angle:f32,
    pub(crate) left_angle:f32,
    pub(crate) approach_direction:i32,
    pub(crate) approach_speed:f32,
    pub(crate) can_be_mirrored:bool,
    pub(crate) rotate_to_target:bool,
    pub(crate) fall_on_fail:bool,
}
impl PedestrianConstruction {
    pub(crate) fn take_down_descriptor(&self,index:u32)->Result<TakeDownDescriptor,String>{
        let data=&self.resources.as_ref().ok_or("takedown resources absent")?.take_down;
        //Only POD+8..39 is read as bytes; both pointers use the existing
        //typed relocation/text resolver. Source null pointer uses DefaultDataArea40.
        let bytes=data.try_array_field_data(TAKE_DOWN_ARRAY,index,TAKE_DOWN_TYPE,40,8..40)
            .map_err(|e|e.to_string())?.map_or(vec![0;32],|v|v.bytes().to_vec());
        let word=|offset:usize|u32::from_be_bytes(bytes[offset-8..offset-4].try_into().unwrap());
        let float=|offset|f32::from_bits(word(offset));
        let text=|offset|->Result<String,String>{
            data.try_array_text_field(TAKE_DOWN_ARRAY,index,TAKE_DOWN_TYPE,40,offset)
                .map_err(|e|e.to_string())?.map(|s|std::str::from_utf8(s.bytes()).map(str::to_owned).map_err(|e|e.to_string()))
                .transpose().map(|v|v.unwrap_or_default())
        };
        Ok(TakeDownDescriptor{attack_name:text(0)?,mirrored_attack_name:text(4)?,time:float(8),
            min_distance:float(12),max_distance:float(16),right_angle:float(20),left_angle:float(24),
            approach_direction:word(28) as i32,approach_speed:float(32),can_be_mirrored:bytes[28]!=0,
            rotate_to_target:bytes[29]!=0,fall_on_fail:bytes[30]!=0})
    }
    pub(crate) fn selected_take_down(&self)->Result<Option<TakeDownDescriptor>,String>{
        self.state.as_ref().ok_or("takedown state absent")?.take_down_type.map(|i|self.take_down_descriptor(i)).transpose()
    }
}
