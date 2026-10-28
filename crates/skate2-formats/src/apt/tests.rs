use super::*;

#[test]
fn glyph_bearings_adjust_atlas_origin_and_advance_is_ninth_field() {
    let font=BitmapFont::parse(b"Ascent: 30\nLineHeight: 40\nGlyphMetricsMap: 1 7 0 100 200 14 25 -2 23 16\nCharMapSet: 1 65 7\n").unwrap();
    assert_eq!(
        font.mapped_glyph(65),
        Some(&Glyph {
            width: 14.,
            height: 25.,
            u: 98.,
            v: 177.,
            advance: 16.,
            x_offset: -2.,
            y_offset: -23.
        })
    );
    assert_eq!(font.mapped_glyph(66), None);
    assert!(BitmapFont::parse(b"Ascent: 1\nLineHeight: 1\nGlyphMetricsMap: 2 7 0 100 200 14 25 -2 23 16\nCharMapSet: 1 65 7").is_err());
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_boot_bitmap_fonts() {
    let root = std::env::var_os("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let bytes = std::fs::read(std::path::Path::new(&root).join("data/big/miscboot.big")).unwrap();
    let archive = crate::big::BigArchive::parse(&bytes).unwrap();
    for name in [
        "debug",
        "helvetica_bold",
        "helvetica_monospace",
        "helvetica_presto",
        "helvetica_prestoGlow",
        "markerstreak",
    ] {
        let data = corpus_asset(&bytes, &archive, &format!(r"data\fe\fonts\{name}.bmpFont"));
        let font = BitmapFont::parse(&data).unwrap();
        assert!(!font.glyphs.is_empty());
        assert!(font.line_height > 0.);
        for id in font.character_map.values() {
            assert!(font.glyphs.contains_key(id), "{name}: missing glyph {id}");
        }
    }
}

#[test]
fn empty_geometry_and_invalid_pointer_are_distinct() {
    assert!(Geometry::parse(b"Sk8\0\0\0\0\0").unwrap().shapes.is_empty());
    let mut bytes = b"Sk8\0\0\0\0\x01\0\0\0\0".to_vec();
    assert_eq!(Geometry::parse(&bytes), Err(AptError::NullTable));
    bytes[8..12].copy_from_slice(&0xffff_fffc_u32.to_be_bytes());
    assert!(matches!(
        Geometry::parse(&bytes),
        Err(AptError::Bounds { .. })
    ));
}

#[test]
fn constants_use_their_own_file_base_and_keep_numeric_bits() {
    let mut bytes = b"Apt constant file\x1a\0\0".to_vec();
    for word in [0x80_u32, 3, 32, 1, 56, 6, 0x8000_0000, 7, 0xffff_fffd] {
        bytes.extend_from_slice(&word.to_be_bytes());
    }
    bytes.extend_from_slice(b"marker\0");
    let parsed = ConstantFile::parse(&bytes).unwrap();
    assert_eq!(parsed.root_offset, 0x80);
    assert_eq!(parsed.constants[0], Constant::String("marker".into()));
    match parsed.constants[1] {
        Constant::Float(v) => assert_eq!(v.to_bits(), 0x8000_0000),
        _ => panic!(),
    }
    assert_eq!(parsed.constants[2], Constant::Integer(-3));
    bytes.truncate(bytes.len() - 1);
    assert!(matches!(
        ConstantFile::parse(&bytes),
        Err(AptError::UnterminatedString(56))
    ));
}

fn corpus_asset(data: &[u8], archive: &crate::big::BigArchive, name: &str) -> Vec<u8> {
    let entry = archive
        .find_name(name)
        .unwrap_or_else(|| panic!("missing {name}"));
    let bytes = &data[entry.data_range().unwrap()];
    if crate::refpack::decompressed_size(bytes).is_ok() {
        crate::refpack::decompress(bytes).unwrap()
    } else {
        bytes.to_vec()
    }
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT, no original assets are stored in the repository"]
fn original_marker_hud_and_button_import() {
    let root = std::env::var_os("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let data = std::fs::read(std::path::Path::new(&root).join("data/fe/screens.big")).unwrap();
    let archive = crate::big::BigArchive::parse(&data).unwrap();
    for base in [
        r"data\fe\source\screens\hud2\hudphonelist",
        r"data\fe\source\controls\button_item2",
    ] {
        let apt = corpus_asset(&data, &archive, &format!("{base}.apt"));
        let constants = corpus_asset(&data, &archive, &format!("{base}.const"));
        let geo = corpus_asset(&data, &archive, &format!("{base}.geo"));
        let animation = Animation::parse(&apt, &constants).unwrap();
        let geometry = Geometry::parse(&geo).unwrap();
        for (i, character) in animation.characters.iter().enumerate() {
            if matches!(character, Some(Character::Shape { .. })) {
                assert!(geometry.shape(i as i32).is_some(), "shape {i} has no GEO");
            }
        }
        if base.ends_with("hudphonelist") {
            assert_eq!(
                (
                    animation.width,
                    animation.height,
                    animation.milliseconds_per_frame
                ),
                (1280, 720, 16)
            );
            assert_eq!(animation.characters.len(), 79);
            assert_eq!(
                animation.imports,
                [Import {
                    file: "source/controls/button_item2".into(),
                    name: "Button Item".into(),
                    character_id: 31
                }]
            );
            let quick = animation.characters[76].as_ref().unwrap().movie().unwrap();
            assert_eq!(quick.label_frame("hidden"), Some(0));
            assert_eq!(quick.label_frame("minimized"), Some(13));
            assert_eq!(quick.label_frame("maximized"), Some(27));
            assert_eq!(quick.label_frame("open"), Some(43));
            assert!(
                animation
                    .constants
                    .constants
                    .contains(&Constant::String("ID_PHONELIST_TITLE".into()))
            );
            // Every truncated header must return an error, not panic/index unchecked.
            for end in 0..32 {
                assert!(Animation::parse(&apt, &constants[..end]).is_err());
            }
        } else {
            assert!(animation.exports.iter().any(|e| e.name == "Button Item"));
        }
    }
}
