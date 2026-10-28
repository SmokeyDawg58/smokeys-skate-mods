use super::*;

#[test]
fn histogram_handles_escape_fallback_and_native_capacity() {
    let mut entries = vec![0; 384];
    entries[128] = 0x20ac;
    entries[129] = 2;
    entries[256] = 0x4e00;
    let h = Histogram { entries };
    assert_eq!(
        h.unpack(&[128, 129, 128, 130, 0], 10, false).unwrap(),
        UnpackedString {
            units: vec![0x20ac, 0x4e00, 130],
            valid: true,
            terminated: true,
        }
    );
    assert_eq!(
        h.unpack(&[129, 1, 0], 10, false).unwrap(),
        UnpackedString {
            units: vec![95],
            valid: false,
            terminated: true,
        }
    );
    assert_eq!(h.unpack(&[128, 0], 10, true).unwrap().units, vec![128]);
    assert_eq!(
        h.unpack(b"AB\0", 1, false).unwrap(),
        UnpackedString {
            units: vec![65],
            valid: true,
            terminated: false,
        }
    );
    assert!(h.unpack(&[129], 2, false).is_err());
    assert_eq!(string_hash(b"A\0ignored"), 32);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_marker_strings_and_all_english_categories_decode() {
    let root = std::env::var_os("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let bytes = std::fs::read(std::path::Path::new(&root).join("data/big/miscboot.big")).unwrap();
    let archive = crate::big::BigArchive::parse(&bytes).unwrap();
    let read = |category: &str| {
        let name = format!(r"data\fe\languages\english\LANGUAGE_English_{category}_skate2ng.BIN");
        let e = archive.find_name(&name).unwrap();
        let data = &bytes[e.data_range().unwrap()];
        if crate::refpack::decompressed_size(data).is_ok() {
            crate::refpack::decompress(data).unwrap()
        } else {
            data.to_vec()
        }
    };
    let histogram = Histogram::parse(&read("HISTOGRAM")).unwrap();
    for category in ["Global", "FrontEnd"] {
        let bytes = read(category);
        let category = LanguageCategory::parse(&bytes).unwrap();
        for &(hash, packed) in &category.records {
            assert_eq!(category.packed_string(hash), Some(packed));
            let decoded = histogram.unpack(packed, packed.len(), false).unwrap();
            assert!(decoded.valid && decoded.terminated, "{hash:08x}");
        }
        if category.name == "Global" {
            for key in [
                "ID_PHONELIST_TITLE",
                "ID_PHONELIST_GOTO_MARKER",
                "ID_PHONELIST_PLACE_MARKER",
                "ID_PHONELIST_OPTIONS_TITLE",
            ] {
                let text = category
                    .packed_string(string_hash(key.as_bytes()))
                    .unwrap_or_else(|| panic!("missing {key}"));
                let decoded = histogram.unpack(text, text.len(), false).unwrap();
                assert!(!decoded.units.is_empty());
                println!("{key}: {}", String::from_utf16(&decoded.units).unwrap());
            }
        }
    }
}
