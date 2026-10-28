//! Read-only original texture identity/type audit, using production parsers.
//! Link against the existing skate2-formats rlib; optional --city scans BAM_Tex.
use skate2_formats::{
    hash::rw_hash64_string,
    material::{TEXTURE_TYPE_ID, Texture},
    mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
    pegasus::{TABLE_OF_CONTENTS_TYPE_ID, TableOfContents},
    rw4::Arena,
    stream::{CollectionMap, StreamFile, decode_arena},
};

fn inspect(
    label: &str,
    resources: &[Vec<u8>; 5],
    all: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let arena = Arena::parse(resources)?;
    for entry in &arena.dictionary {
        if entry.type_id != TABLE_OF_CONTENTS_TYPE_ID {
            continue;
        }
        for item in
            TableOfContents::parse(arena.object_bytes(resources, entry).ok_or("TOC bytes")?)?
                .entries
        {
            let Some(entry) = arena
                .local_entry(item.object)
                .filter(|e| e.type_id == TEXTURE_TYPE_ID)
            else {
                continue;
            };
            let wanted = [
                0x000053d203e3870a,
                0x0000050003e3870a,
                0x000026ee03e3870a,
                0x000000c603e3870a,
                0x0000270103e3870a,
                0x0000325903e3870a,
            ];
            if !all && !wanted.contains(&(item.guid & !1)) {
                continue;
            }
            let texture = Texture::parse(
                arena
                    .object_bytes(resources, entry)
                    .ok_or("texture bytes")?,
            )?;
            println!(
                "{label}\t{:016x}\t{}\t{:?}\t{}x{}x{}\tmips={}\tformat={:08x}",
                item.guid,
                item.name,
                texture.texture_type,
                texture.width,
                texture.height,
                texture.depth,
                texture.mip_levels,
                texture.pixel_format
            );
            if [0x0000270103e3870a, 0x0000325903e3870a].contains(&(item.guid & !1)) {
                let data = arena
                    .local_entry(texture.data)
                    .and_then(|e| arena.object_bytes(resources, e))
                    .ok_or("texture payload")?;
                let levels = texture.decode_levels(data)?;
                // Conservative proof only: zero blue in BOTH RGB565 endpoints
                // implies zero in every decoded DXT1 texel, including all mips.
                if texture.pixel_format == skate2_formats::material::PIXEL_FORMAT_DXT1 {
                    let all_zero = levels.iter().all(|level| {
                        level.data.chunks_exact(8).all(|b| {
                            u16::from_le_bytes([b[0], b[1]]) & 31 == 0
                                && u16::from_le_bytes([b[2], b[3]]) & 31 == 0
                        })
                    });
                    println!("  specular_blue_zero_all_faces_mips={all_zero}");
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("game root required")?);
    let mut mounts = MountSet::new();
    for spec in &SKATE2_ARCHIVES {
        let path = spec.path_under(&root);
        if path.is_file() {
            mounts.push(MountedBig::open(path, spec.use_external_header)?);
        }
    }
    println!(
        "missingtexture.Texture GUID {:016x}",
        rw_hash64_string("missingtexture.Texture")
    );
    let path = r"data\content\textures\globaltextures.rx2";
    inspect(
        path,
        &Arena::split_file_resources(&mounts.read_path(path)?)?,
        true,
    )?;
    if std::env::args().any(|s| s == "--city") {
        let map =
            CollectionMap::parse(&mounts.read_path(r"data\content\world\stream\BAM\BAM_Tex.xsm")?)?;
        for row in &map.collections {
            let path = format!(r"data\content\world\stream\BAM\{}.xsf", row.filename);
            let bytes = mounts.read_path(&path)?;
            let file = StreamFile::parse(&bytes, row.asset_count)?;
            for block in &file.assets {
                inspect(
                    &row.filename,
                    &decode_arena(&bytes[block.data_range.clone()])?.resources,
                    false,
                )?;
            }
        }
    }
    Ok(())
}
