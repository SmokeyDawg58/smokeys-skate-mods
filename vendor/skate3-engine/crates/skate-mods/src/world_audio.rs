//! World audio commands (API 2, world audio extension 1): mods publish traffic vehicles,
//! pedestrians and skaters to the game's retail world audio (the same components an engine
//! system adds, `skate-game` `world_audio.rs`), fire their one-shots (horn, alarm, speech) and read
//! back whether an object is audible. Keys belong to the calling mod; the host enforces 48
//! objects per mod and 128 in all, parks an object that has not been updated for 0.5 s, and removes
//! everything a mod published when it is disabled or reloaded.
use serde::Deserialize;

/// Objects per mod / in all.
/// Objects a mod may publish (and in all). Retail's instance pools pick the audible few (4 cars,
/// 15 peds, 1 NPC skater) from everything published, so the limits only bound the bridge's and
/// the hosts' per-frame work (a distance per object, one sort) and the update commands a mod
/// sends; 48 lets one mod publish a street's worth (the test mod: 16 cars, 20 peds, a ghost).
pub const MAX_OBJECTS_PER_MOD: usize = 48;
pub const MAX_OBJECTS_TOTAL: usize = 128;
/// An object not updated for this long is parked (speed 0, feet up, horn off).
pub const PARK_SECONDS: f64 = 0.5;

/// What kind of object a key publishes.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Traffic,
    Ped,
    Skater,
    /// World audio extension 2: a sound emitter (an `.ems` eVolumeType 1 record added to the map's
    /// live list: retail's reach test, falloff curve and `c_emitter` post).
    Emitter,
    /// World audio extension 2: a reverb zone (eVolumeType 5: joins the zone list the reverb preset
    /// selector walks).
    ReverbZone,
}

/// Which MixMap instances a mod's traffic vehicle or ped plays on (world audio extension 3, doc 16
/// L3; the default flipped to `own` in extension 4, user decision 2026-10-04: "yes I agree it
/// should match" the mod emitters, which have their own instances by default).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Slots {
    /// Its own instance of a private MixMap (the default; not retail): it plays whenever it is
    /// within retail's list radius (40 m cars, 50 m peds) and among the 16 nearest own cars / 16
    /// nearest own peds of all mods; a farther one waits (silent, `read(key).waiting`) until it is
    /// among them. It never takes one of retail's instances, so the map's objects keep retail's
    /// pools.
    #[default]
    Own,
    /// Retail's pools (4 traffic / 15 pedestrian instances, the more-audible setting 8 / 24),
    /// shared with the map's objects: the nearest win. `retail` is accepted as well (the name
    /// extension 3 used), so a mod passing `retail` gets the pools on 3 and 4 alike.
    #[serde(alias = "retail")]
    Shared,
}

impl Slots {
    /// Spawn `slots` → the instances the object takes (`None` = the default, [`Slots::Own`]).
    pub fn resolve(slots: Option<Self>) -> Self {
        slots.unwrap_or_default()
    }
}

/// Every field a spawn or an update may carry; each kind reads its own and ignores none silently
/// (a field of another kind is rejected by [`WorldAudioOptions::validate_for`]).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldAudioOptions {
    // ---- all kinds
    /// World position (m). Ignored while `body` is set.
    #[serde(default)]
    pub position: Option<[f32; 3]>,
    /// World velocity (m/s); without it the host derives it from the position change.
    #[serde(default)]
    pub velocity: Option<[f32; 3]>,
    /// Heading (rad about +Y; 0 = +Z).
    #[serde(default)]
    pub heading: Option<f32>,
    /// Follow one of this mod's physics bodies (position, rotation and velocity from it).
    #[serde(default)]
    pub body: Option<String>,
    /// Traffic and peds, spawn only: `own` (the default since extension 4: its own MixMap
    /// instance) or `shared` (alias `retail`: retail's pools with the map's objects).
    #[serde(default)]
    pub slots: Option<Slots>,
    // ---- traffic
    /// The `aud_traffic_engine` record (`c01_family01`, `c03_sports01`, `c04_taxi01`,
    /// `c05_truck01`, …) or a living-world model mapped to one (`taxi01`, `sedan02`, `suv`, …): a
    /// mod car opts in to the retail traffic engine sound (not retail).
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub speed: Option<f32>,
    /// The driver's signed acceleration (m/s²); without it, from the speed change.
    #[serde(default)]
    pub load: Option<f32>,
    /// 0 none, 1..=5 a horn kind, 6 the alarm (prefer the `horn` / `alarm` events).
    #[serde(default)]
    pub horn: Option<i32>,
    #[serde(default)]
    pub skidding: Option<bool>,
    /// The car is parked (retail's `StayingParked`): an `impact` can set its alarm off. Default:
    /// parked while the mod does not update it (0.5 s).
    #[serde(default)]
    pub parked: Option<bool>,
    // ---- peds
    /// Speech voice id 1..=96 (0 = none): a ped's model (its shoe class, kind and speech words
    /// follow from it unless given) or a skater's voice (the bail grunt, its reactions). The pros
    /// 1–29 and the special cast 30–38 speak on the main-cast channel.
    #[serde(default)]
    pub voice: Option<u32>,
    #[serde(default)]
    pub shoe_class: Option<u8>,
    #[serde(default)]
    pub weight: Option<u8>,
    #[serde(default)]
    pub close_range: Option<bool>,
    /// Foot plants (A, B).
    #[serde(default)]
    pub feet: Option<[bool; 2]>,
    /// Audio surface materials under the feet (A, B).
    #[serde(default)]
    pub materials: Option<[u32; 2]>,
    /// Footsteps on (default: retail's 3-nearest rule).
    #[serde(default)]
    pub footsteps: Option<bool>,
    /// The ped is tazing (the `c_tazer` burst plays while it holds; the `tazer` event holds it
    /// for a time instead).
    #[serde(default)]
    pub tazing: Option<bool>,
    /// Raise the game flag of the photographer's repeat (a ped holding speech value 29 repeats it
    /// every second while any published ped sets this).
    #[serde(default)]
    pub photo_flag: Option<bool>,
    // ---- skaters
    /// `lite` (the default: a rolling-only state from these fields) or `state_log:<name>` (a
    /// ghost replaying a recorded audio state log, `logs/<name>.tsv` in the mod or
    /// `SKATE_AUDIO_STATE_LOGS`).
    #[serde(default)]
    pub source: Option<String>,
    /// Ghost window: start (s) and length (s) of the log to loop.
    #[serde(default)]
    pub from: Option<f32>,
    #[serde(default)]
    pub seconds: Option<f32>,
    /// Lite skater: wheels down (FL, FR, RL, RR), the material under the board (default: the
    /// ground's), grinding and its material, in the air.
    #[serde(default)]
    pub wheels: Option<[bool; 4]>,
    #[serde(default)]
    pub material: Option<u32>,
    #[serde(default)]
    pub grinding: Option<bool>,
    #[serde(default)]
    pub grind_material: Option<u32>,
    #[serde(default)]
    pub air: Option<bool>,
    /// Lite skater: the loose-board state (0 none, 1 upside down, 2 on its side): the board slide
    /// holds while it is set (ghosts take it from their log).
    #[serde(default)]
    pub loose_board: Option<u8>,
    // ---- emitters (extension 2)
    /// The AEMS bank bound to `c_emitter` (a retail bank stem, or one a content overlay adds).
    #[serde(default)]
    pub bank: Option<String>,
    /// The attribute patch = the `c_emitter` selector (0..=500).
    #[serde(default)]
    pub patch: Option<i32>,
    /// Attribute volume 0..=1 (level = volume × falloff curve).
    #[serde(default)]
    pub volume: Option<f32>,
    /// `eVolumeFalloffType`: `squared` (the default), `linear`, `flat`.
    #[serde(default)]
    pub falloff: Option<crate::audio::FalloffCurve>,
    // ---- emitters and reverb zones (extension 2): the record's shape
    /// Extents (m): a sphere of radius `extent[0]` when all three are equal, else an ellipsoid with
    /// semi-axes along forward, up and side.
    #[serde(default)]
    pub extent: Option<[f32; 3]>,
    /// The ellipsoid's forward axis in the object's frame (default +X; turned by `heading`).
    #[serde(default)]
    pub forward: Option<[f32; 3]>,
    /// Inner core fraction 0..=1 at full level.
    #[serde(default)]
    pub core: Option<f32>,
    // ---- reverb zones (extension 2)
    /// The reverb preset (`aud_reverb` key, 16 hex digits).
    #[serde(default)]
    pub preset: Option<String>,
}

/// One-shot options (`horn`: kind, seconds; `speech`: value; `tazer`: seconds; `body_fall`:
/// kind; `reaction`: value, by).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldAudioEventOptions {
    /// `horn`: the horn kind 1..=5. `body_fall`: the animation's `BodyFallType` key (8, 9 or
    /// any other value 1..=255: three sounds).
    #[serde(default)]
    pub kind: Option<u8>,
    #[serde(default)]
    pub seconds: Option<f32>,
    /// A speech value: a number or a name (`warn`, `cheer`, `slam`, `flee`, `DoWarning`, …).
    /// `reaction`: `slam`, `slam_b`, `trick`, `crash` or `chase`.
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    /// `reaction`: the other skater's model (0 = the player; a pro 1..=29 picks the pro-on-pro lines).
    #[serde(default)]
    pub by: Option<u32>,
    /// `impact`: the contact's speed (m/s, the length retail's car alarm rule tests; 0..=200).
    #[serde(default)]
    pub speed: Option<f32>,
    /// `impact`: who touched the car (`player`, `character`, `vehicle`, `object`; default `player`).
    #[serde(default)]
    pub source: Option<String>,
}

/// The impact sources a mod may name.
pub const IMPACT_SOURCES: [&str; 4] = ["player", "character", "vehicle", "object"];

/// `alarm_rule` options (retail's car alarm trigger): fields given replace the rule's current
/// numbers (`enabled`, `min_impact` m/s, `seconds`); no options = back to retail's (the install's
/// setup data). Cleared when the mod stops.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AlarmRuleOptions {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub min_impact: Option<f32>,
    #[serde(default)]
    pub seconds: Option<f32>,
}

impl AlarmRuleOptions {
    pub fn validate(&self) -> bool {
        self.min_impact
            .is_none_or(|v| v.is_finite() && (0.0..=200.0).contains(&v))
            && self
                .seconds
                .is_none_or(|v| v.is_finite() && (0.0..=600.0).contains(&v))
    }
}

/// `announce` options (the announcer channel, retail's `PlayAnnouncerSpeech`): `pro` = a skater
/// model 1..=96 whose announcer pro id fills word 2 (as a pro's crash does for `480_slam_pro`);
/// `words` = the request block from word 0 (a list of at most 33 numbers; word 0 = 0 takes the
/// running announcer). Nothing plays without an announcer character (`sdk.world_audio.announcer`).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AnnounceOptions {
    #[serde(default)]
    pub pro: Option<u32>,
    /// A Lua list arrives as an array, an empty table as a map: both are read by [`Self::words`].
    #[serde(default)]
    pub words: Option<serde_json::Value>,
}

/// Words per announcer request block (`sub_824AAC40` reads up to word 32).
pub const ANNOUNCE_WORDS: usize = 33;

impl AnnounceOptions {
    /// The block words (None: not a list of at most 33 non-negative integers below 2^32).
    pub fn words(&self) -> Option<Vec<u32>> {
        let word = |v: &serde_json::Value| v.as_u64().and_then(|w| u32::try_from(w).ok());
        match &self.words {
            None => Some(Vec::new()),
            Some(serde_json::Value::Array(a)) if a.len() <= ANNOUNCE_WORDS => {
                a.iter().map(word).collect()
            }
            // A Lua table with keys 1..n (or empty).
            Some(serde_json::Value::Object(m)) if m.len() <= ANNOUNCE_WORDS => {
                let mut out = vec![0; m.len()];
                for (k, v) in m {
                    let i: usize = k.parse().ok().filter(|i| (1..=m.len()).contains(i))?;
                    out[i - 1] = word(v)?;
                }
                Some(out)
            }
            _ => None,
        }
    }

    pub fn validate(&self) -> bool {
        self.pro.is_none_or(|p| p <= 96) && self.words().is_some()
    }
}

/// An announcer event: its id (24576..=24751, `0x6000 | n`) or its name (`480_slam_pro`, `480`).
pub fn valid_announcer_event(event: &serde_json::Value) -> bool {
    match event {
        serde_json::Value::Number(n) => n.as_u64().is_some_and(|v| (24576..=24751).contains(&v)),
        serde_json::Value::String(s) => {
            !s.is_empty()
                && s.len() <= 64
                && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

/// An announcer character a mod may name (a model below 104; the export's `announcer_id` gives its
/// word: 35 → 1, 36 → 2, any other model none).
pub fn valid_announcer_character(character: Option<u32>) -> bool {
    character.is_none_or(|c| (1..104).contains(&c))
}

/// The NPC skater reactions a mod can raise (`reaction` event values).
pub const REACTIONS: [&str; 5] = ["slam", "slam_b", "trick", "crash", "chase"];

fn finite(v: &[f32]) -> bool {
    v.iter().all(|x| x.is_finite())
}
fn point(p: &[f32; 3]) -> bool {
    finite(p) && p.iter().all(|x| x.abs() <= 100_000.0)
}
fn velocity(v: &[f32; 3]) -> bool {
    finite(v) && v.iter().all(|x| x.abs() <= 200.0)
}
fn name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
        && !s.contains("..")
}

fn hex16(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `lite`, or `state_log:<name>` with a safe file name.
pub fn valid_source(s: &str) -> bool {
    s == "lite" || s.strip_prefix("state_log:").is_some_and(name)
}

impl WorldAudioOptions {
    /// Bounds shared by every kind.
    pub fn validate(&self) -> bool {
        self.position.as_ref().is_none_or(point)
            && self.velocity.as_ref().is_none_or(velocity)
            && self
                .heading
                .is_none_or(|h| h.is_finite() && h.abs() <= 1000.0)
            && self.body.as_deref().is_none_or(crate::schema::valid_id)
            && self.engine.as_deref().is_none_or(name)
            && self
                .speed
                .is_none_or(|v| v.is_finite() && (0.0..=200.0).contains(&v))
            && self.load.is_none_or(|v| v.is_finite() && v.abs() <= 100.0)
            && self.horn.is_none_or(|v| (0..=6).contains(&v))
            && self.voice.is_none_or(|v| v <= 96)
            && self.shoe_class.is_none_or(|v| (1..=5).contains(&v))
            && self.weight.is_none_or(|v| (1..=5).contains(&v))
            && self.materials.is_none_or(|m| m.iter().all(|x| *x <= 143))
            && self.source.as_deref().is_none_or(valid_source)
            && self
                .from
                .is_none_or(|v| v.is_finite() && (0.0..=36_000.0).contains(&v))
            && self
                .seconds
                .is_none_or(|v| v.is_finite() && (1.0..=600.0).contains(&v))
            && self.material.is_none_or(|v| v <= 143)
            && self.grind_material.is_none_or(|v| v <= 143)
            && self.loose_board.is_none_or(|v| v <= 2)
            && self.bank.as_deref().is_none_or(name)
            && self.patch.is_none_or(|v| (0..=500).contains(&v))
            && self
                .volume
                .is_none_or(|v| v.is_finite() && (0.0..=1.0).contains(&v))
            && self.extent.is_none_or(|e| {
                e.iter()
                    .all(|x| x.is_finite() && (0.1..=10_000.0).contains(x))
            })
            && self.forward.is_none_or(|f| {
                finite(&f)
                    && f.iter().map(|x| x * x).sum::<f32>() > 1e-6
                    && f.iter().all(|x| x.abs() <= 1000.0)
            })
            && self
                .core
                .is_none_or(|v| v.is_finite() && (0.0..=1.0).contains(&v))
            && self.preset.as_deref().is_none_or(hex16)
    }

    /// The fields of other kinds are rejected (so a typo'd kind is an error, not silence).
    pub fn validate_for(&self, kind: ObjectKind) -> bool {
        let traffic = self.engine.is_some()
            || self.speed.is_some()
            || self.load.is_some()
            || self.horn.is_some()
            || self.skidding.is_some()
            || self.parked.is_some();
        // `voice` is a ped's or a skater's (the AI skaters' 89–96, the pros 1–29: bail grunt, reactions).
        let ped = self.shoe_class.is_some()
            || self.weight.is_some()
            || self.close_range.is_some()
            || self.feet.is_some()
            || self.materials.is_some()
            || self.footsteps.is_some()
            || self.tazing.is_some()
            || self.photo_flag.is_some();
        let skater = self.source.is_some()
            || self.from.is_some()
            || self.seconds.is_some()
            || self.wheels.is_some()
            || self.material.is_some()
            || self.grinding.is_some()
            || self.grind_material.is_some()
            || self.air.is_some()
            || self.loose_board.is_some();
        let emitter = self.bank.is_some()
            || self.patch.is_some()
            || self.volume.is_some()
            || self.falloff.is_some();
        let shape = self.extent.is_some() || self.forward.is_some() || self.core.is_some();
        let zone = self.preset.is_some();
        let moving = self.velocity.is_some() || self.voice.is_some() || self.speed.is_some();
        self.validate()
            && (self.slots.is_none() || matches!(kind, ObjectKind::Traffic | ObjectKind::Ped))
            && match kind {
                ObjectKind::Traffic => !ped && !skater && !emitter && !shape && !zone,
                ObjectKind::Ped => !traffic && !skater && !emitter && !shape && !zone,
                // A lite skater takes its speed from `speed` too.
                ObjectKind::Skater => {
                    !ped && self.engine.is_none()
                        && self.load.is_none()
                        && self.horn.is_none()
                        && self.skidding.is_none()
                        && !emitter
                        && !shape
                        && !zone
                        && self.parked.is_none()
                }
                ObjectKind::Emitter => !traffic && !ped && !skater && !zone && !moving,
                ObjectKind::ReverbZone => !traffic && !ped && !skater && !emitter && !moving,
            }
    }

    /// What a spawn must give: an emitter its bank and extent, a reverb zone its preset and extent.
    pub fn complete_for(&self, kind: ObjectKind) -> bool {
        match kind {
            ObjectKind::Emitter => self.bank.is_some() && self.extent.is_some(),
            ObjectKind::ReverbZone => self.preset.is_some() && self.extent.is_some(),
            _ => true,
        }
    }
}

impl WorldAudioEventOptions {
    pub fn validate(&self, event: &str) -> bool {
        match event {
            _ if event != "reaction" && self.by.is_some() => false,
            _ if event != "impact" && (self.speed.is_some() || self.source.is_some()) => false,
            "impact" => {
                self.kind.is_none()
                    && self.seconds.is_none()
                    && self.value.is_none()
                    && self
                        .speed
                        .is_some_and(|s| s.is_finite() && (0.0..=200.0).contains(&s))
                    && self
                        .source
                        .as_deref()
                        .is_none_or(|s| IMPACT_SOURCES.contains(&s))
            }
            "reaction" => {
                self.kind.is_none()
                    && self.seconds.is_none()
                    && self.by.is_none_or(|b| b <= 96)
                    && matches!(&self.value, Some(serde_json::Value::String(s)) if REACTIONS.contains(&s.as_str()))
            }
            "horn" => {
                self.kind.is_none_or(|k| (1..=5).contains(&k))
                    && self
                        .seconds
                        .is_none_or(|s| s.is_finite() && (0.0..=30.0).contains(&s))
                    && self.value.is_none()
            }
            "alarm" => self.kind.is_none() && self.seconds.is_none() && self.value.is_none(),
            "tazer" => {
                self.kind.is_none()
                    && self
                        .seconds
                        .is_none_or(|s| s.is_finite() && (0.0..=30.0).contains(&s))
                    && self.value.is_none()
            }
            "body_fall" => {
                self.kind.is_some_and(|k| k >= 1) && self.seconds.is_none() && self.value.is_none()
            }
            "speech" => {
                self.kind.is_none()
                    && self.seconds.is_none()
                    && match &self.value {
                        Some(serde_json::Value::Number(n)) => {
                            n.as_i64().is_some_and(|v| (0..=127).contains(&v))
                        }
                        Some(serde_json::Value::String(s)) => {
                            !s.is_empty()
                                && s.len() <= 64
                                && s.chars().all(|c| {
                                    c.is_ascii_alphanumeric() || c == '_' || c == ' ' || c == '/'
                                })
                        }
                        _ => false,
                    }
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn options_validate_per_kind() {
        let car: WorldAudioOptions = serde_json::from_value(json!({"engine":"c04_taxi01","position":[1,2,3],"speed":12,"load":-3.5,"skidding":true})).unwrap();
        assert!(car.validate_for(ObjectKind::Traffic));
        assert!(!car.validate_for(ObjectKind::Ped));
        let ped: WorldAudioOptions = serde_json::from_value(
            json!({"voice":59,"shoe_class":3,"feet":[true,false],"materials":[0,0]}),
        )
        .unwrap();
        assert!(ped.validate_for(ObjectKind::Ped) && !ped.validate_for(ObjectKind::Traffic));
        let ghost: WorldAudioOptions = serde_json::from_value(json!({"source":"state_log:state_20261003_143434","from":30,"seconds":20,"position":[0,0,0]})).unwrap();
        assert!(ghost.validate_for(ObjectKind::Skater));
        let lite: WorldAudioOptions = serde_json::from_value(json!({"source":"lite","speed":5,"wheels":[true,true,true,true],"air":false,"loose_board":2})).unwrap();
        assert!(lite.validate_for(ObjectKind::Skater) && !lite.validate_for(ObjectKind::Ped));
        let zapper: WorldAudioOptions =
            serde_json::from_value(json!({"voice":72,"tazing":true,"photo_flag":false})).unwrap();
        assert!(zapper.validate_for(ObjectKind::Ped) && !zapper.validate_for(ObjectKind::Skater));
        for bad in [
            json!({"voice":97}),
            json!({"shoe_class":0}),
            json!({"horn":7}),
            json!({"source":"state_log:../x"}),
            json!({"source":"record"}),
            json!({"engine":"a b"}),
            json!({"position":[0,1e9,0]}),
            json!({"velocity":[1000,0,0]}),
            json!({"seconds":0}),
            json!({"loose_board":3}),
        ] {
            let o: WorldAudioOptions = serde_json::from_value(bad.clone()).unwrap();
            assert!(!o.validate(), "accepted {bad}");
        }
        assert!(serde_json::from_value::<WorldAudioOptions>(json!({"typo":1})).is_err());
    }

    /// World audio extension 2: emitters and reverb zones take their own fields only, need their
    /// bank / preset and extent at spawn, and keep within the record bounds.
    #[test]
    fn emitter_and_reverb_zone_options_validate() {
        let fountain: WorldAudioOptions = serde_json::from_value(json!({"bank":"water_fountain","patch":81,"position":[1,2,3],"extent":[6,6,6],"core":0.2,"volume":0.5,"falloff":"linear"})).unwrap();
        assert!(
            fountain.validate_for(ObjectKind::Emitter)
                && fountain.complete_for(ObjectKind::Emitter)
        );
        assert!(
            !fountain.validate_for(ObjectKind::Traffic)
                && !fountain.validate_for(ObjectKind::ReverbZone)
        );
        let cave: WorldAudioOptions = serde_json::from_value(json!({"preset":"BEEFC8E3DE04FBAE","position":[0,0,0],"extent":[20,8,12],"forward":[0,0,1],"heading":1.5})).unwrap();
        assert!(
            cave.validate_for(ObjectKind::ReverbZone) && cave.complete_for(ObjectKind::ReverbZone)
        );
        assert!(!cave.validate_for(ObjectKind::Emitter) && !cave.validate_for(ObjectKind::Ped));
        let bare: WorldAudioOptions = serde_json::from_value(json!({"position":[0,0,0]})).unwrap();
        assert!(
            bare.validate_for(ObjectKind::Emitter) && !bare.complete_for(ObjectKind::Emitter),
            "an update may omit them, a spawn not"
        );
        assert!(!bare.complete_for(ObjectKind::ReverbZone));
        for bad in [
            json!({"patch":501}),
            json!({"volume":1.5}),
            json!({"extent":[0,1,1]}),
            json!({"extent":[1,1,20000]}),
            json!({"forward":[0,0,0]}),
            json!({"core":2}),
            json!({"preset":"beef"}),
            json!({"preset":"GGGGGGGGGGGGGGGG"}),
            json!({"bank":"../x"}),
        ] {
            let o: WorldAudioOptions = serde_json::from_value(bad.clone()).unwrap();
            assert!(!o.validate(), "accepted {bad}");
        }
        let moving: WorldAudioOptions =
            serde_json::from_value(json!({"bank":"x","extent":[1,1,1],"velocity":[1,0,0]}))
                .unwrap();
        assert!(
            !moving.validate_for(ObjectKind::Emitter),
            "emitters are static records"
        );
        assert!(serde_json::from_value::<WorldAudioOptions>(json!({"falloff":"cubic"})).is_err());
    }

    /// World audio extension 4 (doc 16 M3): a mod's car / ped takes its own instance unless it
    /// asks for retail's pools (`shared`, alias `retail`); `own` stays valid; only traffic and
    /// peds take `slots`; anything else is a typo, not silence.
    #[test]
    fn slots_default_to_own_and_shared_opts_in() {
        let none: WorldAudioOptions =
            serde_json::from_value(json!({"engine":"c04_taxi01"})).unwrap();
        assert_eq!(
            Slots::resolve(none.slots),
            Slots::Own,
            "no slots = its own instance"
        );
        for (v, want) in [
            ("own", Slots::Own),
            ("shared", Slots::Shared),
            ("retail", Slots::Shared),
        ] {
            let o: WorldAudioOptions = serde_json::from_value(json!({"slots": v})).unwrap();
            assert_eq!(Slots::resolve(o.slots), want, "{v}");
            assert!(
                o.validate_for(ObjectKind::Traffic) && o.validate_for(ObjectKind::Ped),
                "{v}"
            );
            for kind in [
                ObjectKind::Skater,
                ObjectKind::Emitter,
                ObjectKind::ReverbZone,
            ] {
                assert!(!o.validate_for(kind), "{v} on {kind:?}");
            }
        }
        for bad in ["extra", "Own", "pool", ""] {
            assert!(
                serde_json::from_value::<WorldAudioOptions>(json!({"slots": bad})).is_err(),
                "accepted {bad:?}"
            );
        }
    }

    #[test]
    fn announce_options_read_lists_and_lua_tables() {
        let o: AnnounceOptions = serde_json::from_value(json!({"pro":4,"words":[0,0,8]})).unwrap();
        assert_eq!(o.words(), Some(vec![0, 0, 8]));
        assert!(o.validate());
        let o: AnnounceOptions = serde_json::from_value(json!({"words":{}})).unwrap();
        assert_eq!(o.words(), Some(vec![]), "an empty Lua table");
        let o: AnnounceOptions = serde_json::from_value(json!({"words":{"2":5,"1":1}})).unwrap();
        assert_eq!(o.words(), Some(vec![1, 5]));
        for bad in [
            json!({"pro":97}),
            json!({"words":[-1]}),
            json!({"words":{"x":1}}),
            json!({"words": vec![0u32; 34]}),
        ] {
            let o: AnnounceOptions = serde_json::from_value(bad.clone()).unwrap();
            assert!(!o.validate(), "accepted {bad}");
        }
        assert!(
            valid_announcer_event(&json!(24708))
                && valid_announcer_event(&json!("480_slam_pro"))
                && !valid_announcer_event(&json!(8210))
                && !valid_announcer_event(&json!("a b"))
        );
        assert!(
            valid_announcer_character(None)
                && valid_announcer_character(Some(36))
                && !valid_announcer_character(Some(104))
        );
    }

    #[test]
    fn events_validate() {
        let horn: WorldAudioEventOptions =
            serde_json::from_value(json!({"kind":3,"seconds":1.2})).unwrap();
        assert!(horn.validate("horn") && !horn.validate("alarm") && !horn.validate("bark"));
        assert!(WorldAudioEventOptions::default().validate("alarm"));
        let hit: WorldAudioEventOptions =
            serde_json::from_value(json!({"speed":3.3,"source":"player"})).unwrap();
        assert!(hit.validate("impact") && !hit.validate("alarm") && !hit.validate("horn"));
        assert!(
            !WorldAudioEventOptions::default().validate("impact"),
            "an impact needs its speed"
        );
        for bad in [
            json!({"speed":-1}),
            json!({"speed":500}),
            json!({"speed":1,"source":"meteor"}),
            json!({"speed":1,"kind":2}),
        ] {
            let o: WorldAudioEventOptions = serde_json::from_value(bad.clone()).unwrap();
            assert!(!o.validate("impact"), "accepted {bad}");
        }
        let parked: WorldAudioOptions = serde_json::from_value(json!({"parked":true})).unwrap();
        assert!(
            parked.validate_for(ObjectKind::Traffic)
                && !parked.validate_for(ObjectKind::Ped)
                && !parked.validate_for(ObjectKind::Skater)
        );
        let rule: AlarmRuleOptions =
            serde_json::from_value(json!({"enabled":true,"min_impact":2,"seconds":4})).unwrap();
        assert!(rule.validate() && AlarmRuleOptions::default().validate());
        assert!(
            !serde_json::from_value::<AlarmRuleOptions>(json!({"seconds":-1}))
                .unwrap()
                .validate()
        );
        assert!(serde_json::from_value::<AlarmRuleOptions>(json!({"typo":1})).is_err());
        let speech: WorldAudioEventOptions =
            serde_json::from_value(json!({"value":"warn"})).unwrap();
        assert!(speech.validate("speech"));
        let speech: WorldAudioEventOptions = serde_json::from_value(json!({"value":23})).unwrap();
        assert!(speech.validate("speech"));
        let bad: WorldAudioEventOptions = serde_json::from_value(json!({"value":500})).unwrap();
        assert!(!bad.validate("speech"));
        assert!(WorldAudioEventOptions::default().validate("tazer"));
        let zap: WorldAudioEventOptions = serde_json::from_value(json!({"seconds":2.0})).unwrap();
        assert!(zap.validate("tazer") && !zap.validate("body_fall"));
        let fall: WorldAudioEventOptions = serde_json::from_value(json!({"kind":9})).unwrap();
        assert!(fall.validate("body_fall") && !fall.validate("tazer"));
        assert!(
            !WorldAudioEventOptions::default().validate("body_fall"),
            "a fall needs its kind"
        );
        let react: WorldAudioEventOptions =
            serde_json::from_value(json!({"value":"trick","by":24})).unwrap();
        assert!(react.validate("reaction") && !react.validate("speech"));
        let bad: WorldAudioEventOptions = serde_json::from_value(json!({"value":"dance"})).unwrap();
        assert!(!bad.validate("reaction"));
        assert!(
            !WorldAudioEventOptions::default().validate("reaction"),
            "a reaction needs its value"
        );
    }
}
