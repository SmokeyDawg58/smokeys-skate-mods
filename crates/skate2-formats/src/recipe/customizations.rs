//! Character::Customizations v2: Serialize82d00998/Deserialize82d08090.
//! Float bits are retained exactly, including values that are not canonical NaNs.
use super::{CharacterRecipeError as Error, Reader};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterCustomizations {
    pub is_male: bool,
    pub colour_capacity: u32,
    pub colours: Vec<Colour>,
    pub stamp_capacity: u32,
    pub stamps: Vec<TextureStamp>,
    pub embedded_capacity: u32,
    pub embedded: Vec<[u8; 2]>,
    pub morph_bits: Vec<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colour {
    pub key: i32,
    pub base_rgb_bits: [u32; 3],
    pub highlight_rgb_bits: [u32; 3],
    pub model_id: u64,
    pub material_id: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureStamp {
    pub path: String,
    pub model_id: u64,
    pub material_id: u64,
    pub stamp_material_id: u64,
    pub user_generated: bool,
    pub key: u8,
}
impl CharacterCustomizations {
    pub fn parse_prefix(bytes: &[u8]) -> Result<(Self, usize), Error> {
        let mut r = Reader::new(bytes);
        let value = Self::read(&mut r)?;
        Ok((value, r.position))
    }
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        if r.u32()? != 2 { return Err(Error::UnsupportedCustomizations); }
        let is_male = r.u8()? != 0;
        let colour_capacity = r.u32()?;
        let count = r.count(44, "customization colours")?;
        let mut colours = Vec::with_capacity(count);
        for _ in 0..count {
            colours.push(Colour { key: r.u32()? as i32,
                base_rgb_bits: [r.u32()?, r.u32()?, r.u32()?],
                highlight_rgb_bits: [r.u32()?, r.u32()?, r.u32()?],
                model_id: r.u64()?, material_id: r.u64()? });
        }
        let stamp_capacity = r.u32()?;
        let count = r.count(30, "customization stamps")?;
        let mut stamps = Vec::with_capacity(count);
        for _ in 0..count {
            stamps.push(TextureStamp { path: r.string()?, model_id: r.u64()?,
                material_id: r.u64()?, stamp_material_id: r.u64()?,
                user_generated: r.u8()? != 0, key: r.u8()? });
        }
        let embedded_capacity = r.u32()?;
        let count = r.count(2, "embedded stamps")?;
        let mut embedded = Vec::with_capacity(count);
        for _ in 0..count { embedded.push([r.u8()?, r.u8()?]); }
        let count = r.count(4, "customization morphs")?;
        let mut morph_bits = Vec::with_capacity(count);
        for _ in 0..count { morph_bits.push(r.u32()?); }
        Ok(Self { is_male, colour_capacity, colours, stamp_capacity, stamps,
            embedded_capacity, embedded, morph_bits })
    }
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        use super::encode::{count, string, u32_value, u64_value};
        let mut out = Vec::new();
        u32_value(&mut out, 2);
        out.push(u8::from(self.is_male));
        u32_value(&mut out, self.colour_capacity);
        count(&mut out, self.colours.len())?;
        for colour in &self.colours {
            u32_value(&mut out, colour.key as u32);
            for bits in colour.base_rgb_bits.into_iter().chain(colour.highlight_rgb_bits) {
                u32_value(&mut out, bits);
            }
            u64_value(&mut out, colour.model_id);
            u64_value(&mut out, colour.material_id);
        }
        u32_value(&mut out, self.stamp_capacity);
        count(&mut out, self.stamps.len())?;
        for stamp in &self.stamps {
            string(&mut out, &stamp.path)?;
            u64_value(&mut out, stamp.model_id);
            u64_value(&mut out, stamp.material_id);
            u64_value(&mut out, stamp.stamp_material_id);
            out.push(u8::from(stamp.user_generated)); out.push(stamp.key);
        }
        u32_value(&mut out, self.embedded_capacity);
        count(&mut out, self.embedded.len())?;
        for pair in &self.embedded { out.extend_from_slice(pair); }
        count(&mut out, self.morph_bits.len())?;
        for bits in &self.morph_bits { u32_value(&mut out, *bits); }
        Ok(out)
    }
}
