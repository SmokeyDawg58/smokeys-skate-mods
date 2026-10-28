//! Skate bitmap font metrics, `cFont::ReadHeader`8284d300. The native reader
//! uses Ascent, LineHeight, GlyphMetricsMap and CharMapSet. Texture ownership
//! and base scale come from the font manager/Vault, not a guessed font size.
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontError {
    Utf8,
    Missing(&'static str),
    Number(&'static str),
    Count(&'static str),
}
impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid bitmap font: {self:?}")
    }
}
impl std::error::Error for FontError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub width: f32,
    pub height: f32,
    /// Atlas pixel origin after the original bearing adjustment.
    pub u: f32,
    pub v: f32,
    pub advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BitmapFont {
    pub ascent: f32,
    pub line_height: f32,
    pub glyphs: BTreeMap<i32, Glyph>,
    pub character_map: BTreeMap<u16, i32>,
}

fn tokens<'a>(
    text: &'a str,
    field: &'static str,
) -> Result<impl Iterator<Item = &'a str>, FontError> {
    let start = text.find(field).ok_or(FontError::Missing(field))? + field.len();
    let tail = &text[start..];
    let tail = tail.strip_prefix(':').ok_or(FontError::Missing(field))?;
    Ok(tail
        .split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|s| !s.is_empty()))
}
fn integer<'a>(
    it: &mut impl Iterator<Item = &'a str>,
    field: &'static str,
) -> Result<i32, FontError> {
    it.next()
        .ok_or(FontError::Count(field))?
        .parse()
        .map_err(|_| FontError::Number(field))
}
fn scalar(text: &str, field: &'static str) -> Result<f32, FontError> {
    tokens(text, field)?
        .next()
        .ok_or(FontError::Count(field))?
        .parse()
        .map_err(|_| FontError::Number(field))
}
impl BitmapFont {
    pub fn parse(bytes: &[u8]) -> Result<Self, FontError> {
        let text = std::str::from_utf8(bytes).map_err(|_| FontError::Utf8)?;
        let mut glyphs = BTreeMap::new();
        let mut input = tokens(text, "GlyphMetricsMap")?;
        let count = integer(&mut input, "GlyphMetricsMap")?;
        if count < 0 || count as usize > bytes.len() / 9 {
            return Err(FontError::Count("GlyphMetricsMap"));
        }
        for _ in 0..count {
            let mut v = [0i32; 9];
            for item in &mut v {
                *item = integer(&mut input, "GlyphMetricsMap")?;
            }
            // sscanf arguments confirmed at8284d46c..4a4: glyph,texture,x,y,
            // width,height,bearingX,bearingY,advance. The second field is not
            // used by cFont; this game binds one texture per font.
            glyphs.insert(
                v[0],
                Glyph {
                    width: v[4] as f32,
                    height: v[5] as f32,
                    u: v[2].wrapping_add(v[6]) as f32,
                    v: v[3].wrapping_sub(v[7]) as f32,
                    advance: v[8] as f32,
                    x_offset: v[6] as f32,
                    y_offset: v[7].wrapping_neg() as f32,
                },
            );
        }
        let mut character_map = BTreeMap::new();
        let mut input = tokens(text, "CharMapSet")?;
        let count = integer(&mut input, "CharMapSet")?;
        if count < 0 || count as usize > bytes.len() / 2 {
            return Err(FontError::Count("CharMapSet"));
        }
        for _ in 0..count {
            let code = integer(&mut input, "CharMapSet")? as u16;
            let glyph = integer(&mut input, "CharMapSet")?;
            character_map.insert(code, glyph);
        }
        Ok(Self {
            ascent: scalar(text, "Ascent")?,
            line_height: scalar(text, "LineHeight")?,
            glyphs,
            character_map,
        })
    }
    /// Missing-character fallback is a renderer policy; this only exposes the
    /// actual authored map, so callers cannot silently substitute another font.
    pub fn mapped_glyph(&self, code: u16) -> Option<&Glyph> {
        self.character_map
            .get(&code)
            .and_then(|id| self.glyphs.get(id))
    }
}
