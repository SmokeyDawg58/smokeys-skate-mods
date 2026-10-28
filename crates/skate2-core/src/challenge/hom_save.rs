//! StatsBackEnd.SaveData8256e6b0 / LoadData8257d640, version13.
//! The native raw records are big endian. Padding is zero on newly written PC
//! records; loaded bail output images retain their unused fields.
//! Part.GetTransform826c3480 copies RigidBody.mCom + mId (0x10/0x1c),
//! and its mapped branch also mixes adjacent mStasis/mInertia/mTag lanes.
//! These are not position components. Portable saves write zero for an unknown
//! fourth bone-position lane; its provenance remains marked in BailImage.
//! No canonical save byte is promoted into a known gameplay vector lane.
//! Result consumers826b2010/826b2588/8260ae08 read damage, never these lanes.
use super::{
    attrib_db::AttribDb,
    hom::{HomMetadata, HomStats},
    mission::HomBail,
};
pub const SAVABLE: &str = "STATISTICS";
pub const DATA: &str = "StatsBackEnd::Data";
pub const BEFORE: [&str; 4] = ["UNLOCKABLES", "CAC", "SKATER_PROFILE", "online"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BailImage {
    pub bytes: [u8; 0x4f0],
    /// Bone Vector3 W lanes whose original adjacent body metadata is unavailable.
    /// This mask is host provenance, not part of the native save record.
    pub unrecorded_bone_w: u32,
}
impl BailImage {
    pub fn word(&self, at: usize) -> u32 {
        u32::from_be_bytes(self.bytes[at..at + 4].try_into().unwrap())
    }
    pub fn set(&mut self, at: usize, value: u32) {
        self.bytes[at..at + 4].copy_from_slice(&value.to_be_bytes());
    }
    pub fn empty() -> Self {
        let mut image = Self {
            bytes: [0; 0x4f0],
            unrecorded_bone_w: 0,
        };
        for i in 0..25 {
            image.set(i * 32 + 16, u32::MAX);
            image.set(i * 32 + 20, u32::MAX);
        }
        for set in 0..3 {
            for i in 0..8 {
                image.set(0x320 + set * 0x6c + i * 12 + 4, u32::MAX);
            }
        }
        image.set(0x4e8, u32::MAX);
        image
    }
    pub fn bail(self) -> HomBail {
        let w = |i| self.word(i);
        let f = |i| f32::from_bits(w(i));
        let count = w(0x380) as usize;
        HomBail {
            save_image: Some(self),
            bone_damage_levels: std::array::from_fn(|i| w(i * 32 + 16).cast_signed()),
            score: w(0x494),
            bone_scores: std::array::from_fn(|i| w(0x4b8 + i * 4)),
            bonus_scores: [
                w(0x498),
                w(0x48c),
                w(0x488),
                w(0x4a8),
                w(0x4a4),
                w(0x4ac),
                w(0x384),
                w(0x3f0),
                w(0x45c),
            ],
            overall_duration: f(0x4d4),
            max_air_frames: w(0x474),
            accumulated_air_drop_height: f(0x490),
            rotation: f(0x4e4),
            moving_duration: f(0x4d0),
            skeleton_over_duration: f(0x4d8),
            severe_duration: f(0x4dc),
            entity_counts: std::array::from_fn(|i| w(0x380 + i * 0x6c)),
            security_guards_hit: (0..count).filter(|i| w(0x324 + i * 12) == 3).count() as u32,
            max_air_drop_height: f(0x480),
            accumulated_air_drop_frames: w(0x484),
            bone_score: w(0x49c),
            bonus_score: w(0x4a0),
            rotation_score: w(0x4a8),
            num_body_tweaks_performed: w(0x4b4),
            bail_speed: f(0x4e0),
            current_body_tweak: w(0x4e8).cast_signed(),
        }
    }
}

/// FELocationManager.GetLocationName82615b68; the top-bail binding handles -1.
pub fn location_name(db: &AttribDb, key: u64) -> String {
    if key == u64::MAX {
        return "ID_COMMON_NEW_SAN_VANELONA".into();
    }
    for (class, offset) in [
        (super::keys::class::FE_LOCATIONS, 2),
        (super::keys::hash64("worldpainter_district"), 0),
        (super::keys::hash64("worldpainter_fe_location_triggers"), 0),
    ] {
        if db.has_collection(class, key) {
            return db.layout_text(class, key, offset).unwrap_or("").into();
        }
    }
    "ID_TAG_NONE".into()
}

pub fn record(
    bail: Option<&HomBail>,
    metadata: Option<&HomMetadata>,
    calendar: [u16; 6],
) -> Result<Vec<u8>, String> {
    let image = match bail {
        Some(b) => b
            .save_image
            .ok_or("HoM save requires the complete bail output")?,
        None => BailImage::empty(),
    };
    let mut record = vec![0; 0x550];
    record[..0x4f0].copy_from_slice(&image.bytes);
    let date = metadata.and_then(|m| m.calendar).unwrap_or(calendar);
    for (i, value) in date.into_iter().enumerate() {
        record[0x4f0 + i * 4..0x4f4 + i * 4].copy_from_slice(&u32::from(value).to_be_bytes());
    }
    if let Some(bail) = bail {
        let mut counts = [0u32; 5];
        for (i, level) in bail.bone_damage_levels.into_iter().enumerate() {
            if (0..=5).contains(&level) {
                counts[4 - (level as usize).min(4)] += 1;
            }
            record[0x528 + i] = u8::from(matches!(level, 4 | 5));
        }
        for (i, value) in counts.into_iter().enumerate() {
            record[0x508 + i * 4..0x50c + i * 4].copy_from_slice(&value.to_be_bytes());
        }
        record[0x520..0x528].copy_from_slice(
            &metadata
                .ok_or("HoM record metadata absent")?
                .location_key
                .to_be_bytes(),
        );
    }
    Ok(record)
}

pub fn load_record(bytes: &[u8], db: &AttribDb) -> Result<(HomBail, HomMetadata), String> {
    if bytes.len() != 0x550 {
        return Err("StatsBackEnd.LoadData assertion1346: HOMResultsData size".into());
    }
    let image = BailImage {
        bytes: bytes[..0x4f0].try_into().unwrap(),
        unrecorded_bone_w: 0,
    };
    for i in 0..3 {
        if image.word(0x380 + i * 0x6c) > 8 {
            return Err("HoM entity count exceeds native capacity8".into());
        }
    }
    let mut calendar = [0u16; 6];
    for (i, value) in calendar.iter_mut().enumerate() {
        *value = u32::from_be_bytes(bytes[0x4f0 + i * 4..0x4f4 + i * 4].try_into().unwrap())
            .try_into()
            .map_err(|_| "HoM calendar field out of range")?;
    }
    let key = u64::from_be_bytes(bytes[0x520..0x528].try_into().unwrap());
    Ok((
        image.bail(),
        HomMetadata {
            calendar: Some(calendar),
            location_key: key,
            location: location_name(db, key),
        },
    ))
}

pub fn totals(stats: &HomStats) -> Vec<u8> {
    [
        stats.injuries[4],
        stats.injuries[3],
        stats.injuries[2],
        stats.injuries[1],
        stats.injuries[0],
        stats.tweaks,
        stats.entity_hits[1],
        stats.entity_hits[0],
        stats.time_bailing.to_bits(),
        stats.total_bails,
    ]
    .into_iter()
    .flat_map(u32::to_be_bytes)
    .collect()
}
pub fn load_totals(stats: &mut HomStats, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() != 40 {
        return Err("StatsBackEnd.LoadData assertion1330: sBailStatsData size".into());
    }
    let values: Vec<_> = bytes
        .chunks_exact(4)
        .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
        .collect();
    stats.injuries = [values[4], values[3], values[2], values[1], values[0]];
    stats.tweaks = values[5];
    stats.entity_hits = [values[7], values[6]];
    stats.time_bailing = f32::from_bits(values[8]);
    stats.total_bails = values[9];
    Ok(())
}
