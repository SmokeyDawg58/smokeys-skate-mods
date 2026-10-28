//! `Sk8::Audio::SurfaceMapping` (the `aud_general` "mapping" record per audio material) and
//! `AudioSystem::mAudioSurfaceMapping` 0x82fd72d0 (each material's splice bank and `aud_material`
//! collection, a static table in the executable).

use crate::vault::{class, Collection, Vault};

/// `aud_general` "mapping" and its per-material record array.
const GENERAL_MAPPING: Collection = Collection::new(class::AUD_GENERAL, 0xC489_459A_0C07_D154);
const MAPPING_KEY: u64 = 0x4CA6_0755_8B1C_F440;

/// `SurfaceMapping::Get*`: word `field` of the 48-byte record for `material` (92 and above read
/// record 92).
#[must_use]
pub fn mapping(vault: &Vault, material: u32, field: usize) -> u32 {
    vault.struct_word(GENERAL_MAPPING, MAPPING_KEY, material.min(92), field)
}

/// `SurfaceMapping::GetRollingMaterial` 0x8251ba48.
#[must_use]
pub fn rolling_material(vault: &Vault, material: u32) -> u32 {
    mapping(vault, material, 1)
}

/// `SurfaceMapping::GetOllieAndLandMaterial` 0x8251bb20.
#[must_use]
pub fn ollie_and_land_material(vault: &Vault, material: u32) -> u32 {
    mapping(vault, material, 2)
}

/// `SurfaceMapping::GetPowerSlideMaterial` 0x8251bbf8.
#[must_use]
pub fn power_slide_material(vault: &Vault, material: u32) -> u32 {
    mapping(vault, material, 3)
}

/// `SurfaceMapping::GetGrindMaterial` 0x8251bcd0.
#[must_use]
pub fn grind_material(vault: &Vault, material: u32) -> u32 {
    mapping(vault, material, 4)
}

/// `SurfaceMapping::GetStoppingMaterial` 0x8251bda8.
#[must_use]
pub fn stopping_material(vault: &Vault, material: u32) -> u32 {
    mapping(vault, material, 5)
}

/// `SurfaceMapping::GetFootStepMaterial` 0x824ed078.
#[must_use]
pub fn foot_step_material(vault: &Vault, material: u32) -> u32 {
    mapping(vault, material, 6)
}

/// `SurfaceMapping::GetMaterialHardness` 0x824ed150.
#[must_use]
pub fn material_hardness(vault: &Vault, material: u32) -> u32 {
    mapping(vault, material, 7)
}

/// `SurfaceMapping::GetSeamType` 0x8251be80.
#[must_use]
pub fn seam_type(vault: &Vault, material: u32) -> u32 {
    mapping(vault, material, 8)
}

/// `SurfaceMapping::GetTransitionType` 0x8251bf58.
#[must_use]
pub fn transition_type(vault: &Vault, material: u32) -> u32 {
    mapping(vault, material, 9)
}

/// `aud_material`.
pub const AUD_MATERIAL: u64 = 0xD40C_B4C0_FFE4_5676;

/// `tAudioSurfaceMapping` for materials 0..132: (`SPLICE_TYPE`, `aud_material` collection).
pub const AUDIO_SURFACE_MAPPING: [(i32, u64); 132] = [
    (5, 0x84EF_7A70_636E_D529),
    (5, 0x9182_CCAD_6347_9434),
    (5, 0x49A5_B1F1_24E5_3FDE),
    (5, 0x49F1_67E3_FC3A_008B),
    (5, 0xD7EF_FA58_49BB_7B15),
    (5, 0x72B4_ECCD_C6CE_E6E8),
    (5, 0xAF8E_AB3B_0227_FD48),
    (5, 0x54D2_2F13_36F8_B796),
    (6, 0x42CD_1912_4AE6_13EB),
    (5, 0x0613_243A_490D_11EA),
    (6, 0xA4B6_334D_87D7_2CC7),
    (6, 0x3738_E970_8EB6_C776),
    (6, 0x1BFF_6E5C_A982_683A),
    (6, 0x5BBD_F449_2AC7_FB47),
    (6, 0x2213_2549_704E_8733),
    (6, 0x1F2A_CA29_DEBE_0729),
    (6, 0x5002_27DC_11D1_98AC),
    (6, 0xB7DB_FE81_3D02_1524),
    (6, 0x6EAA_CF19_DDD0_7B5A),
    (6, 0x60EC_16F4_8C0B_E620),
    (6, 0x791C_508A_6E12_05EA),
    (6, 0x4233_7BA2_D862_B622),
    (6, 0x10ED_4128_9D47_F052),
    (6, 0xBD55_6D5B_135B_BF8E),
    (6, 0xB74F_55A0_A965_16A0),
    (6, 0x3B21_4D29_5DB5_14F7),
    (6, 0xA7AB_65A3_C044_B3DA),
    (6, 0xE96D_23CB_6637_2699),
    (6, 0xB062_804C_25ED_D8A7),
    (6, 0xA8EF_0E67_A39D_62D9),
    (6, 0xF714_1297_FEB2_9B5D),
    (6, 0x0DEB_3D61_CAED_9567),
    (6, 0x7AC7_7169_773C_0CE4),
    (6, 0x0193_A08E_3EDA_269B),
    (6, 0xDBF3_EC4C_65A6_9F5D),
    (6, 0x3A43_FEE3_639B_A301),
    (6, 0x3090_33B5_E587_2675),
    (6, 0xEF0C_2293_B788_8864),
    (6, 0xD807_FA29_ADE6_5957),
    (6, 0x1001_7676_688A_A504),
    (5, 0xD8F1_32C0_2494_531F),
    (5, 0x5D4D_49C5_98C7_C2E3),
    (5, 0xD318_4F6B_D8E6_29CC),
    (5, 0xA227_D8D1_EB22_C467),
    (5, 0x7D5F_7C90_590C_D76C),
    (5, 0xE351_2303_72B8_AA43),
    (5, 0xC65B_AF14_769D_FCEB),
    (5, 0x13D1_9F33_ABDE_FDA4),
    (5, 0x6B17_0B79_6C32_5537),
    (5, 0x8640_A182_4538_2F66),
    (5, 0xC53F_A0D1_5249_C949),
    (5, 0x9988_214E_2D3F_25C2),
    (5, 0x6A0D_63DA_93EF_3459),
    (5, 0x9943_3514_E821_A363),
    (5, 0xF002_CCA1_F7FD_6978),
    (5, 0x947A_2EEC_879C_F0C4),
    (5, 0xC5C3_FCD9_9441_4A11),
    (5, 0x45EF_058B_C984_C7E8),
    (5, 0x0374_5EEB_6C08_A0B7),
    (5, 0xF3D7_8078_D942_C1F5),
    (5, 0x3E93_846A_0C09_04EF),
    (5, 0x71D3_FF58_545A_7240),
    (5, 0x36F1_FB39_9593_A42E),
    (5, 0xCA39_C5BF_26B5_A8CE),
    (5, 0x98CD_99EC_1E25_7D5A),
    (5, 0x94BF_C99C_9D77_FBE9),
    (6, 0x3F4E_33C9_A0B9_B05E),
    (6, 0xDC7B_0987_6FB1_9FFF),
    (6, 0x8BD8_7DE7_2E1A_3157),
    (5, 0x0613_243A_490D_11EA),
    (5, 0x0613_243A_490D_11EA),
    (5, 0x0613_243A_490D_11EA),
    (6, 0xBFBD_AC65_7253_E431),
    (5, 0x7323_5D7F_8A76_4EA3),
    (5, 0x070B_3A35_4E4B_91E8),
    (5, 0x6C32_2F84_8110_963C),
    (5, 0x07FB_54E5_A776_E0DD),
    (5, 0x4EE7_754C_4008_1FFB),
    (5, 0x59BA_34C2_182D_FAED),
    (5, 0x761C_67A1_D52C_27D8),
    (5, 0x5AAA_7F37_5D3F_AFD7),
    (6, 0x7CAF_97EA_114D_54A9),
    (5, 0xA029_E9DE_D863_C78E),
    (6, 0xB13F_D71E_A972_B2E7),
    (6, 0x061A_5C9D_1EB4_4C60),
    (5, 0x0526_D0B4_1AA7_E7F3),
    (5, 0xD7A4_B6C1_9268_884E),
    (5, 0x0023_C43A_FE54_25C2),
    (6, 0x0B56_CDEC_643B_3C5C),
    (5, 0xA5C4_BE35_A648_4D2A),
    (6, 0x44C4_5991_DF83_F528),
    (5, 0x954A_B262_2DDD_CDAE),
    (-1, 0x0000_0000_0000_0000),
    (5, 0xF512_E3A5_DFD6_C51C),
    (5, 0x27CD_E290_C328_67A9),
    (5, 0x379C_ADEC_E541_E342),
    (5, 0xFC13_9EC9_67F7_5F74),
    (5, 0xAB73_644D_AB95_C51A),
    (5, 0x65AF_5100_E77A_2982),
    (5, 0x0DB9_BC87_8EC6_B280),
    (7, 0xFB01_58EF_B888_9A2A),
    (7, 0x8768_4DAC_4583_AED5),
    (7, 0xDC73_596A_59C3_A8B9),
    (7, 0xF16B_BF49_DDEA_88A9),
    (7, 0xC83E_4BE3_8A95_3FAA),
    (5, 0xD2E3_F99A_952D_C96B),
    (5, 0x6615_3E29_A9E4_F5F0),
    (5, 0xB025_A0D5_D09B_AD53),
    (5, 0xC54D_B3B0_9121_5CA0),
    (5, 0xF2E9_CB8F_28FA_ED2D),
    (5, 0x7410_CAC4_852F_5168),
    (5, 0x7653_5DC1_C13D_C342),
    (6, 0x56AE_4185_FFB2_7F3B),
    (6, 0xBC46_374D_5FE9_45CC),
    (6, 0x4E3F_3F3D_686C_1376),
    (6, 0xFD06_F928_C97B_0C13),
    (6, 0x2724_30E4_6031_2991),
    (6, 0x8CDC_59AF_8D2F_E8C0),
    (6, 0x3C39_F714_BDF1_4002),
    (6, 0x582E_3694_A7D7_A81C),
    (6, 0xDAB7_1EE7_131F_B16F),
    (5, 0x5C7F_8A8C_D61A_DC6A),
    (5, 0xD715_DAAD_CCFE_3707),
    (5, 0xD110_4F04_0889_4184),
    (5, 0x0180_D422_774A_2E76),
    (5, 0x7EE4_E115_095A_35FE),
    (5, 0xE6AC_C17E_8C6E_545D),
    (5, 0x897D_82A7_4C3D_D55F),
    (5, 0x9322_B767_21A7_E961),
    (5, 0x88E7_0FFE_EB92_0FAC),
    (5, 0xE5E4_636B_15D9_21E1),
    (5, 0xB2B1_99D7_B0CC_6E56),
];

/// The `aud_material` collection of `material` (`None` past the table).
#[must_use]
pub fn material_collection(material: u32) -> Option<Collection> {
    AUDIO_SURFACE_MAPPING.get(material as usize).map(|&(_, key)| Collection::new(AUD_MATERIAL, key))
}

/// The splice bank of `material`.
#[must_use]
pub fn material_splice(material: u32) -> i32 {
    AUDIO_SURFACE_MAPPING.get(material as usize).map_or(-1, |&(splice, _)| splice)
}
