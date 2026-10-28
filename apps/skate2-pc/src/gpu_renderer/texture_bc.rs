//! BC1/BC3 expansion for original textures whose logical base is not 4-aligned.
//! Consumes the existing endian-normalized, untiled mip blocks. Microsoft BC
//! layout: RGB565 endpoints, 2-bit colour indices; BC3 adds 3-bit alpha indices.
//! https://learn.microsoft.com/en-us/windows/win32/direct3d10/d3d10-graphics-programming-guide-resources-block-compression
use super::RenderResult;
use skate2_formats::material::LinearTextureLevel;

pub(super) fn expand(level: &LinearTextureLevel, bc3: bool) -> RenderResult<Vec<u8>> {
    let size = usize::try_from(level.width)?.checked_mul(usize::try_from(level.height)?)
        .and_then(|n| n.checked_mul(4)).ok_or("BC expansion size overflow")?;
    let mut rgba = vec![0; size];
    let stride = if bc3 { 16 } else { 8 };
    for by in 0..level.height_in_blocks {
        for bx in 0..level.width_in_blocks {
            let offset = usize::try_from(by * level.row_pitch + bx * stride)?;
            let block = level.data.get(offset..offset + stride as usize).ok_or("BC block truncated")?;
            let colours = if bc3 { &block[8..] } else { block };
            let c0 = u16::from_le_bytes([colours[0], colours[1]]);
            let c1 = u16::from_le_bytes([colours[2], colours[3]]);
            let rgb = |c: u16| -> [u8; 4] {
                let (r,g,b) = ((c >> 11) as u8, ((c >> 5) & 63) as u8, (c & 31) as u8);
                [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2), 255]
            };
            let mut palette = [rgb(c0), rgb(c1), [0,0,0,255], [0;4]];
            if bc3 || c0 > c1 {
                for lane in 0..3 {
                    let (a,b) = (u16::from(palette[0][lane]), u16::from(palette[1][lane]));
                    palette[2][lane] = ((2*a+b)/3) as u8;
                    palette[3][lane] = ((a+2*b)/3) as u8;
                }
                palette[3][3] = 255;
            } else {
                for lane in 0..3 {
                    palette[2][lane] = ((u16::from(palette[0][lane])+u16::from(palette[1][lane]))/2) as u8;
                }
            }
            let indices = u32::from_le_bytes(colours[4..8].try_into()?);
            let mut alpha = [255u8;8];
            let mut alpha_indices = 0u64;
            if bc3 {
                alpha[0] = block[0]; alpha[1] = block[1];
                let steps = if alpha[0] > alpha[1] { 7 } else { 5 };
                for i in 1..steps {
                    alpha[i+1] = (((steps-i) * usize::from(alpha[0]) + i * usize::from(alpha[1])) / steps) as u8;
                }
                if steps == 5 { alpha[6] = 0; alpha[7] = 255; }
                for (i,&byte) in block[2..8].iter().enumerate() { alpha_indices |= u64::from(byte) << (8*i); }
            }
            for y in 0..4 {
                for x in 0..4 {
                    let (px,py) = (bx*4+x, by*4+y);
                    if px >= level.width || py >= level.height { continue; }
                    let i = y*4+x;
                    let mut colour = palette[((indices >> (2*i)) & 3) as usize];
                    if bc3 { colour[3] = alpha[((alpha_indices >> (3*i)) & 7) as usize]; }
                    let dst = (py as usize * level.width as usize + px as usize)*4;
                    rgba[dst..dst+4].copy_from_slice(&colour);
                }
            }
        }
    }
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn level(width:u32,height:u32,data:Vec<u8>,bc3:bool)->LinearTextureLevel {
        let stride = if bc3 {16} else {8};
        LinearTextureLevel { level:0,face:0,width,height,width_in_blocks:width.div_ceil(4),
            height_in_blocks:height.div_ceil(4),bytes_per_block:stride,
            row_pitch:width.div_ceil(4)*stride,data }
    }
    #[test]
    fn bc3_short_base_crops_edge_rows_and_preserves_alpha() {
        // 16x2 original slider shape: four blocks, solid red, half alpha.
        let block = [128,128,0,0,0,0,0,0,0,248,0,0,0,0,0,0];
        let result = expand(&level(16,2,block.repeat(4),true),true).unwrap();
        assert_eq!(result, [255,0,0,128].repeat(32));
    }
    #[test]
    fn bc1_transparent_mode_and_bc3_forced_four_colour() {
        let colours = [0,0,255,255,255,255,255,255];
        assert_eq!(expand(&level(1,1,colours.to_vec(),false),false).unwrap(), [0,0,0,0]);
        let mut block = vec![0,255,255,255,255,255,255,255];
        block.extend(colours);
        assert_eq!(expand(&level(1,1,block,true),true).unwrap(), [170,170,170,255]);
    }
}
