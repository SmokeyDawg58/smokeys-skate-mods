//! TU3 Hall of Meat authored bones and curves. S2 caller offsets are translated
//! by stable field hashes; S3 added curves and moved the shared fields.
use crate::physics::PointGraph;
pub(super) const CLASS:u64=0x9e4c_471b_c6ea_b615;
const NAME:&str="Hash_9E4C471BC6EAB615";
// S2 host caller offsets -> shared field identities in the actual S3 schema.
const CURVES:[(usize,&str);6]=[
    (2100,"Hash_C2D28EFB70B37684"), // heavy landing damage (S3 offset2420)
    (2180,"Hash_96CDCC590988982E"), // airtime (S3 offset2500)
    (2260,"Hash_32B1A3C2961F4E6E"), // drop height (S3 offset2580)
    (2340,"Hash_B2A997F8579B1998"), // bail speed (S3 offset2820)
    (2420,"Hash_6CD9F0CD7675949C"), // rotation (S3 offset2900)
    (2500,"Hash_2451424AD330B08F"), // moving bail duration (S3 offset2980)
];
pub(super) struct Data { bones:[u32;525],curves:[PointGraph<8>;6] }
impl Data {
    pub(super) fn load(collections:&skate_data::collections::Collections)->Result<Self,String> {
        let bones=collections.words::<525>(NAME,"default","Hash_1F9B5EA147CBEEE9")?;
        let mut curves=std::array::from_fn(|_|PointGraph::<8>::default());
        for (i,(_,field)) in CURVES.iter().enumerate() {
            let words=collections.words::<20>(NAME,"default",field)?;
            curves[i]=PointGraph {x:std::array::from_fn(|i|f32::from_bits(words[i+4])),
                y:std::array::from_fn(|i|f32::from_bits(words[i+12]))};
        }
        Ok(Self{bones,curves})
    }
    pub(super) fn bone_word(&self,offset:usize)->Option<u32> {
        (offset%4==0).then(||self.bones.get(offset/4).copied()).flatten()
    }
    pub(super) fn curve(&self,offset:usize,x:f32)->Option<f32> {
        CURVES.iter().position(|&(o,_)|o==offset).map(|i|self.curves[i].evaluate(x))
    }
}
pub(super) fn data()->Result<&'static Data,String> {
    static DATA:std::sync::OnceLock<Result<Data,String>>=std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let collections=crate::skate3_native::physics::player::embedded_collections()?;
        Data::load(&collections)
    })
        .as_ref().map_err(Clone::clone)
}
