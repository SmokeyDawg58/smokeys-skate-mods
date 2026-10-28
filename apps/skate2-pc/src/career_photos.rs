//! PC storage for career photo slots, beside SKATER.P. Full photos deliberately
//! use quality 95 / 4:4:4 without the console's 150 KiB cap; UI thumbnails retain
//! their original 160x120 size and budget.
use crate::gpu_renderer::RenderResult;
use skate2_core::texture::NativeTexture;
use skate2_formats::material::{LinearTextureLevel, TextureEndian, TextureType, PIXEL_FORMAT_A8R8G8B8};
use std::path::PathBuf;

fn path(slot: u32, thumbnail: bool) -> RenderResult<PathBuf> {
    if slot == 0 { return Err("career photo slot zero".into()); }
    let profile = skate2_core::edit_skater_profile::EditSkaterProfileStore::default_path()?;
    Ok(profile.parent().ok_or("profile has no directory")?.join("career_photos")
        .join(format!("{slot}{}.jpg", if thumbnail { "_thumb" } else { "" })))
}

pub(crate) fn texture_key(slot: u32, thumbnail: bool) -> String {
    format!("career_photo/{slot}/{}", if thumbnail { "thumb" } else { "full" })
}

/// StateMemcardPhotoSaveFlow::SetScreenShotAndThumbData 0x8274ad18:
/// quality starts at 75, decreases by 10, and the buffers are 150/15 KiB.
pub(crate) fn compress(width: u32, height: u32, rgba: &[u8], capacity: usize) -> RenderResult<Vec<u8>> {
    let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|pixel| pixel[..3].iter().copied()).collect();
    for quality in [75, 65, 55, 45, 35, 25, 15, 5] {
        let mut encoded = Vec::new();
        let mut encoder = jpeg_encoder::Encoder::new(&mut encoded, quality);
        // JPEG::WriteJPEG 0x8282aa18 uses jpeg_set_defaults; its YCbCr defaults
        // at 0x82b9a940 use 2x2 luma and 1x1 chroma (4:2:0).
        encoder.set_sampling_factor(jpeg_encoder::SamplingFactor::F_2_2);
        encoder.encode(&rgb, width.try_into()?, height.try_into()?, jpeg_encoder::ColorType::Rgb)?;
        if encoded.len() < capacity { return Ok(encoded); }
    }
    Err(format!("career photo exceeds the native {capacity}-byte JPEG buffer").into())
}

/// PC export quality. No downscaling or quality reduction to
/// fit a console save buffer; chroma detail is retained around bright signs.
pub(crate) fn full_quality(width:u32,height:u32,rgba:&[u8])->RenderResult<Vec<u8>> {
    if width==0||height==0||rgba.len()!=width as usize*height as usize*4 {
        return Err("invalid photo capture".into());
    }
    let rgb:Vec<u8>=rgba.chunks_exact(4).flat_map(|p|p[..3].iter().copied()).collect();
    let mut encoded=Vec::new();
    let mut encoder=jpeg_encoder::Encoder::new(&mut encoded,95);
    encoder.set_sampling_factor(jpeg_encoder::SamplingFactor::F_1_1);
    encoder.encode(&rgb,width.try_into()?,height.try_into()?,jpeg_encoder::ColorType::Rgb)?;
    Ok(encoded)
}

/// ApplySampleModeBilinearFour8BitUInt 0x82b1f640: the first coordinates are
/// (-0.5,-0.5), advanced by the f32 source/destination ratios. Four samples
/// clamp at the edges (RandomIterator::GetPixel 0x82afe008); channels truncate.
pub(crate) fn thumbnail(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(160 * 120 * 4);
    let dx = width as f32 / 160.0;
    let dy = height as f32 / 120.0;
    let mut row = 0.0_f32;
    for _ in 0..120 {
        let y = row - 0.5;
        let y0 = y.floor() as i32;
        let fy = y - y.floor();
        let mut x = -0.5_f32;
        for _ in 0..160 {
            let x0 = x.floor() as i32;
            let fx = x - x.floor();
            let sample = |sx: i32, sy: i32, channel: usize| {
                let sx = sx.clamp(0, width as i32 - 1) as usize;
                let sy = sy.clamp(0, height as i32 - 1) as usize;
                f32::from(rgba[(sy * width as usize + sx) * 4 + channel])
            };
            for channel in 0..4 {
                let value = ((sample(x0, y0, channel) * ((1.0 - fy) * (1.0 - fx))
                    + sample(x0 + 1, y0, channel) * ((1.0 - fy) * fx))
                    + sample(x0, y0 + 1, channel) * (fy * (1.0 - fx)))
                    + sample(x0 + 1, y0 + 1, channel) * (fy * fx);
                out.push(value.clamp(0.0, 255.0) as u8);
            }
            x += dx;
        }
        row += dy;
    }
    out
}

pub(crate) fn save(slot: u32, width: u32, height: u32, rgba: &[u8]) -> RenderResult<()> {
    if width == 0 || height == 0 || rgba.len() != width as usize * height as usize * 4 {
        return Err("invalid career photo capture".into());
    }
    let full = full_quality(width, height, rgba)?;
    let thumb = compress(160, 120, &thumbnail(width, height, rgba), 15360)?;
    let full_path = path(slot, false)?;
    let thumb_path = path(slot, true)?;
    std::fs::create_dir_all(full_path.parent().ok_or("photo has no directory")?)?;
    // Each replacement is completed from a temporary file, so readers cannot
    // consume an incomplete JPEG after an interrupted write.
    for (target, bytes) in [(full_path, full), (thumb_path, thumb)] {
        let temporary = target.with_extension("jpg.tmp");
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(temporary, target)?;
    }
    Ok(())
}

pub(crate) fn load(slot: u32, thumbnail: bool) -> RenderResult<Option<NativeTexture>> {
    let bytes = match std::fs::read(path(slot, thumbnail)?) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(bytes));
    let decoded = decoder.decode()?;
    let info = decoder.info().ok_or("career JPEG has no image info")?;
    if info.pixel_format != jpeg_decoder::PixelFormat::RGB24 { return Err("career JPEG is not RGB".into()); }
    let (width, height) = (u32::from(info.width), u32::from(info.height));
    let bgra = decoded.chunks_exact(3).flat_map(|pixel| [pixel[2], pixel[1], pixel[0], 255]).collect();
    Ok(Some(NativeTexture {
        dictionary_index: 0,
        guid: 0,
        name: texture_key(slot, thumbnail),
        texture_type: TextureType::TwoDimensional,
        endian: TextureEndian::None,
        width, height, depth: 1, mip_levels: 1,
        pixel_format: PIXEL_FORMAT_A8R8G8B8,
        packed_mips: false,
        mip_data_offset: 0,
        encoded_data: Vec::new(),
        linear_levels: vec![LinearTextureLevel {
            level: 0, face: 0, width, height,
            width_in_blocks: width, height_in_blocks: height,
            bytes_per_block: 4, row_pitch: width * 4, data: bgra,
        }],
    }))
}
