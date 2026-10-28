//! Selected fields of backend module14, shared by live chase consumers.
//! Source: StatsBackEnd.Init8255eaf0, StatsBackEndCore.Init82a72d48.
//! Includes the version13 statistics savable and native achievement request state.
use super::LocalBackendOwner;
use crate::input_cvar_runtime::InputCvarReferences;
#[path = "stats_hud.rs"]
pub(crate) mod hud;

#[derive(Clone, Copy)]
pub(crate) struct PersonalBest {
    pub(crate) value: f32,
    pub(crate) threshold: f32,
    pub(crate) broken: bool,
    pub(crate) recorded: bool,
    pub(crate) not_read_hud: bool,
}
pub(crate) struct StatsBackend {
    loading: bool,
    pub(crate) hom: std::sync::Arc<std::sync::Mutex<crate::challenge::hom::HomStats>>,
    pub(crate) personal_bests: [PersonalBest; 22],
    pub(crate) statistics: [f32; 65],
    achievements:[bool;45],
    dmo_statistics:[f32;13],movable_object_times:[f32;52],
    achievement_requests:Vec<(usize,u32)>,
}
impl StatsBackend {
    pub(crate) fn initialize() -> Self {
        // Exact threshold words from StaticPersonalBestsInfoTable82fda0c8.
        // Core.Init initializes each value to threshold, not to zero.
        let thresholds: [u32; 22] = [
            0x41200000,0,0x40000000,0x3fc00000,0x40a00000,0x40000000,
            0x3f800000,0x3f800000,0x41700000,0x41700000,0x40a00000,
            0x40a00000,0x40a00000,0x40000000,0x40a00000,0x469c4000,
            0x43fa0000,0x41100000,0x40c00000,0x44fa0000,0x41a00000,0x40800000,
        ];
        Self {
            hom: Default::default(),
            loading: false, //Core constructor82a72cc8+0xc.
            personal_bests: thresholds.map(|bits| PersonalBest {
                value: f32::from_bits(bits), threshold: f32::from_bits(bits),
                broken: false, recorded: false, not_read_hud: false,
            }),
            statistics: [0.; 65], //StaticStatsInfoTable82fd9db8.
            achievements:[false;45],achievement_requests:Vec::new(),
            dmo_statistics:[0.;13],movable_object_times:[0.;52],
        }
    }
    /// `GrantAchievement` (`0x8255f580`): the platform request is queued once.
    pub(crate) fn grant(&mut self, id: usize, delay: u32) -> Result<(), String> {
        let completed = self.achievements.get_mut(id).ok_or("achievement index out of range")?;
        if !*completed {
            *completed = true;
            self.achievement_requests.push((id, delay));
        }
        Ok(())
    }
    /// `UpdateStats` (`0x82568b90`) past its recording gate: 8, 25 and 64 store the value and the
    /// others add it; 14, 29 and 38 then grant Uninsurable, Skitched Up and Taste The Mongo past
    /// their thresholds.
    pub(crate) fn apply_stat(&mut self, item: usize, value: f32, thresholds: &StatThresholds) -> Result<(), String> {
        let statistic = self.statistics.get_mut(item).ok_or("statistics item out of bounds")?;
        if matches!(item, 8 | 25 | 64) {
            *statistic = value;
            return Ok(());
        }
        *statistic += value;
        let total = *statistic;
        let achievement = match item {
            14 if total > thresholds.bones - 0.1 => 43,
            29 if total > thresholds.skitch => 32,
            38 if total > 4999.5 => 36,
            _ => return Ok(()),
        };
        self.grant(achievement, 0)
    }
    /// `UpdatePersonalBests` (`0x8255f4c8`) past its recording gate: a value above the best and
    /// its threshold breaks it. Strict comparisons reject NaN.
    pub(crate) fn raise_best(&mut self, item: usize, value: f32) -> Result<(), String> {
        let best = self.personal_bests.get_mut(item).ok_or("personal best item out of bounds")?;
        if value > best.value && value > best.threshold {
            best.value = value;
            best.broken = true;
        }
        Ok(())
    }
    /// `RecordPersonalBests` (`0x82a72f40`) past its recording gate: a broken best is recorded.
    pub(crate) fn record_best(&mut self, item: usize) -> Result<(), String> {
        let best = self.personal_bests.get_mut(item).ok_or("personal best item assertion82a72f84")?;
        if best.broken {
            best.recorded = true;
        }
        Ok(())
    }
    /// `ClearPersonalBestFlags` (`0x82a72ed0`): a recorded best is no longer broken and waits for
    /// the HUD to read it.
    pub(crate) fn clear_best_flags(&mut self) {
        for best in &mut self.personal_bests {
            if best.recorded {
                best.broken = false;
                best.recorded = false;
                best.not_read_hud = true;
            }
        }
    }
    /// `UpdateDmoStats` (`0x82569090`): 0 adds the time (DIY past its threshold), 1 and 2 store
    /// the value, 3 to 12 keep the largest.
    pub(crate) fn update_dmo(&mut self, index: usize, value: f32, thresholds: &StatThresholds) -> Result<(), String> {
        let statistic = self.dmo_statistics.get_mut(index).ok_or("UpdateDmoStats index assertion")?;
        match index {
            0 => {
                *statistic += value;
                if *statistic > thresholds.moving_objects {
                    return self.grant(35, 0);
                }
            }
            1 | 2 => *statistic = value,
            _ => {
                if value > *statistic {
                    *statistic = value;
                }
            }
        }
        Ok(())
    }
    /// `UpdateMovableObjectStat` (`0x8255f158`).
    pub(crate) fn add_moving_time(&mut self, kind: usize, time: f32) -> Result<(), String> {
        *self.movable_object_times.get_mut(kind).ok_or("UpdateMovableObjectStat type assertion")? += time;
        Ok(())
    }
}

/// The `scoring2_general` achievement thresholds the statistics compare against; a missing
/// attribute reads `DefaultDataArea`'s zero.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct StatThresholds {
    /// `MinBonesBrokenForAchievement`, `MinSkitchDistanceForAchievement` and
    /// `MinTimeSpentMovingObjectsForAchievement`.
    pub(crate) bones: f32,
    pub(crate) skitch: f32,
    pub(crate) moving_objects: f32,
    /// `MinGrassGapLengthForAchievement` and `MinStairGapLengthForAchievement`.
    pub(crate) grass_gap: f32,
    pub(crate) stair_gap: f32,
    /// `MinTimeAtMaxSpeedForAchievement`, the top speed `UpdateDistanceTravelled` reads by key and
    /// the height `UpdateTrickCompletion` reads by key.
    pub(crate) time_at_max_speed: f32,
    pub(crate) max_speed: f32,
    pub(crate) top_of_the_world: f32,
}
impl StatThresholds {
    /// Each threshold through `read(key)`.
    pub(crate) fn try_read<E>(mut read: impl FnMut(u64) -> Result<f32, E>) -> Result<Self, E> {
        Ok(Self {
            bones: read(0x63fe_be7c_89f6_29d0)?,
            skitch: read(0x9739_773f_9505_9c11)?,
            moving_objects: read(0x4200_81c3_a296_242d)?,
            grass_gap: read(0x2939_3df3_ccc7_dd23)?,
            stair_gap: read(0xb19d_46d6_3a0e_40b6)?,
            time_at_max_speed: read(0xbfa1_ef35_275b_3327)?,
            max_speed: read(0xe4f1_a7b8_5e65_28e3)?,
            top_of_the_world: read(0x5b1d_5b4f_677c_ae0d)?,
        })
    }
}

/// GameBackEnd+84; native enum and inlined constructor8258106c.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameState { InGame = 0, NisPlaying = 1, MoviePlaying = 2, Loading = 3 }
impl LocalBackendOwner {
    pub(crate) fn hom_stats(&self)->Result<std::sync::Arc<std::sync::Mutex<crate::challenge::hom::HomStats>>,String> {
        Ok(std::sync::Arc::clone(&self.statistics.lock().map_err(|_|"StatsBackEnd lock")?.hom))
    }
    /// StateLoading.OnLeaveState827582e8. Called only after selected startup
    /// completes, never inferred from the existence of a local Actor.
    pub(crate) fn finish_loading(&self) -> Result<(), String> {
        *self.game_state.lock().map_err(|_| "GameBackEnd state lock")? = GameState::InGame;
        Ok(())
    }
    fn statistics_recording_allowed(&self, stats: &StatsBackend,
        cvars: &InputCvarReferences) -> Result<bool, String> {
        //8255f408: preserve lazy cvar/mode/loading/game-state evaluation.
        if cvars.try_allow_stats_anytime().map_err(|e|format!("stats cvar: {e:?}"))? { return Ok(true); }
        if self.try_game_mode().map_err(|e|e.to_string())? != 1 { return Ok(false); }
        if stats.loading { return Ok(false); }
        Ok(*self.game_state.lock().map_err(|_| "GameBackEnd state lock")? == GameState::InGame)
    }
    fn personal_bests_recording_allowed(&self, stats: &StatsBackend,
        cvars: &InputCvarReferences) -> Result<bool, String> {
        //8255f468 allows its own override before consulting the normal gate.
        if cvars.try_allow_personal_bests_anytime().map_err(|e|format!("personal best cvar: {e:?}"))? {return Ok(true);}
        self.statistics_recording_allowed(stats,cvars)
    }
    /// UpdatePersonalBests8255f4c8. Strict comparisons preserve unordered/NaN
    /// rejection and the original threshold; there is no host score rounding.
    pub(crate) fn update_personal_best(&self, item: usize, value: f32,
        cvars: &InputCvarReferences) -> Result<(), String> {
        let mut stats=self.statistics.lock().map_err(|_| "StatsBackEnd lock")?;
        if self.personal_bests_recording_allowed(&stats,cvars)? {stats.raise_best(item,value)?;}
        Ok(())
    }
    /// RecordPersonalBests82a72f40. A separate call deliberately samples the
    /// recording gate again, matching the two native calls in ChaseMan.
    pub(crate) fn record_personal_best(&self,item:usize,
        cvars:&InputCvarReferences)->Result<(),String>{
        let mut stats=self.statistics.lock().map_err(|_| "StatsBackEnd lock")?;
        if item>=stats.personal_bests.len(){return Err("personal best item assertion82a72f84".into());}
        if self.personal_bests_recording_allowed(&stats,cvars)? {stats.record_best(item)?;}
        Ok(())
    }
    /// UpdateStats82568b90 default accumulation branch used by ChaseMan.
    /// Excludes special achievement branches rather than silently omitting
    /// their required side effects. force is actual GPR6 per raw82568ba8.
    pub(crate) fn update_additive_statistic(&self,item:usize,value:f32,force:bool,
        cvars:&InputCvarReferences)->Result<(),String>{
        let mut stats=self.statistics.lock().map_err(|_| "StatsBackEnd lock")?;
        if !force && !self.statistics_recording_allowed(&stats,cvars)? {return Ok(());}
        if matches!(item,8|14|25|29|38|64){return Err(format!("non-additive/special statistics branch {item}"));}
        *stats.statistics.get_mut(item).ok_or("statistics item out of bounds")? += value;
        Ok(())
    }
}

impl LocalBackendOwner {
    /// ServiceManager.Activate*8289d5c8..8289d954 and the progression variants
    /// 8289c730/cc90. UpdateStats82568b90 assigns pool percentage (item25).
    pub(crate) fn record_service_statistics(&self, service:i32, paid:bool,
        pools:Option<(i32,i32)>, cvars:&InputCvarReferences)->Result<(),String> {
        let (count,spending,price,achievement)=match service {
            0=>(22,23,500.,33), 1=>(26,27,100.,14), 2=>(24,28,200.,16),
            _=>return Err("native service type assertion".into()),
        };
        self.update_additive_statistic(count,1.,false,cvars)?;
        if paid {self.update_additive_statistic(spending,price,false,cvars)?;}
        if let Some((drained,total))=pools {
            {
                let mut stats=self.statistics.lock().map_err(|_|"StatsBackEnd lock")?;
                if self.statistics_recording_allowed(&stats,cvars)? {
                    stats.statistics[25]=(drained as f32/total as f32)*100.;
                }
            }
            if drained==total {self.grant_achievement(achievement,0)?;}
        }else {
            let value=self.statistics.lock().map_err(|_|"StatsBackEnd lock")?.statistics[count];
            // Native GetStatsValue -> ConvertAToI, the integral service count.
            if value as i32==10 {self.grant_achievement(achievement,0)?;}
        }
        Ok(())
    }
    /// StatsBackEnd.GrantAchievement8255f580: queue the platform request once.
    pub(crate) fn grant_achievement(&self,id:usize,delay:u32)->Result<(),String> {
        self.statistics.lock().map_err(|_|"StatsBackEnd lock")?.grant(id,delay)
    }
    /// TrackBailStatsAndAchievements82569348. Its caller first samples
    /// IsRecordingAllowed8255f408; UpdateStats and PersonalBests sample it again.
    pub(crate) fn track_hom_statistics(&self,bail:&crate::challenge::mission::HomBail,
        enabled:bool,cvars:&InputCvarReferences,vault:&std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>)->Result<(),String> {
        {let stats=self.statistics.lock().map_err(|_|"StatsBackEnd lock")?;
            if !self.statistics_recording_allowed(&stats,cvars)? {return Ok(());}}
        let general=vault.instance(0x3492_15e2_e817_703c,0xd7ed_bd36_2d7d_2152);
        let threshold=|key|->Result<f32,String>{Ok(f32::from_bits(general.try_scalar_word(key,0)
            .map_err(|e|format!("HoM statistic attribute: {e:?}"))?.unwrap_or(0)))};
        if enabled {
            let broken=bail.bone_damage_levels.iter().filter(|&&level|level==4||level==5).count() as u32;
            if broken>=(threshold(0xdbcb_f5c1_8a92_f66c)?+0.1) as u32 {self.grant_achievement(39,0)?;}
            if bail.bail_speed>threshold(0xfbfc_d022_a138_fd8d)? {self.grant_achievement(31,0)?;}
            // UpdateStats14's native special branch82568d20.
            let uninsurable={
                let mut stats=self.statistics.lock().map_err(|_|"StatsBackEnd lock")?;
                if self.statistics_recording_allowed(&stats,cvars)? {
                    stats.statistics[14]+=broken as f32;
                    stats.statistics[14]>threshold(0x63fe_be7c_89f6_29d0)?-0.1
                }else{false}
            };
            if uninsurable {self.grant_achievement(43,0)?;}
            if bail.bone_damage_levels[23]!=-1||bail.bone_damage_levels[0]!=-1 {self.update_additive_statistic(15,1.,false,cvars)?;}
            if bail.bone_damage_levels[24]!=-1 {self.update_additive_statistic(16,1.,false,cvars)?;}
            for (item,value) in [(13,broken as f32),(14,bail.accumulated_air_drop_height),(15,bail.score as f32)] {
                self.update_personal_best(item,value,cvars)?;self.record_personal_best(item,cvars)?;
            }
        }
        for (item,count) in [(44,bail.entity_counts[0]),(45,bail.security_guards_hit),(48,bail.entity_counts[1])] {
            if count!=0 {self.update_additive_statistic(item,count as f32,false,cvars)?;}
        }
        Ok(())
    }
}

impl LocalBackendOwner {
    /// LoadData8257d640; unsupported leaf versions leave the current data intact.
    pub(crate) fn load_statistics(&self,leaf:Option<&skate2_formats::skater_profile::MemcardLeaf>,
        db:&crate::challenge::attrib_db::AttribDb)->Result<(),String> {
        let Some(leaf)=leaf.filter(|leaf|leaf.version==13) else{return Ok(());};
        let tuples=&leaf.tuples;
        if tuples.len()!=209 {return Err("StatsBackEnd version13 tuple count".into());}
        let mut at=0usize;
        let mut next=||->Result<&[u8],String>{let value=tuples.get(at).ok_or("StatsBackEnd tuple absent")?;at+=1;Ok(value)};
        let word=|b:&[u8]|->Result<u32,String>{Ok(u32::from_be_bytes(b.try_into().map_err(|_|"StatsBackEnd scalar size")?))};
        let mut stats=self.statistics.lock().map_err(|_|"StatsBackEnd lock")?;
        for value in &mut stats.achievements {let b=next()?;if b.len()!=1{return Err("StatsBackEnd bool size".into());}*value=b[0]!=0;}
        for value in &mut stats.statistics {*value=f32::from_bits(word(next()?)?);}
        for value in &mut stats.dmo_statistics {*value=f32::from_bits(word(next()?)?);}
        for best in &mut stats.personal_bests {best.value=f32::from_bits(word(next()?)?);best.broken=false;best.recorded=false;best.not_read_hud=false;}
        for value in &mut stats.movable_object_times {*value=f32::from_bits(word(next()?)?);}
        let mut hom=stats.hom.lock().map_err(|_|"HoM stats lock")?;
        crate::challenge::hom_save::load_totals(&mut hom,next()?)?;
        let count=word(next()?)? as usize;
        if count>10 {return Err("StatsBackEnd top bail capacity10".into());}
        hom.top.clear();hom.top_metadata.clear();
        for i in 0..10 {
            let bytes=next()?;
            if i<count {let (bail,metadata)=crate::challenge::hom_save::load_record(bytes,db)?;hom.top.push(bail);hom.top_metadata.push(metadata);}
        }
        Ok(())
    }
    /// SaveData8256e6b0's tuple order, including ten fixed HOMResultsData slots.
    pub(crate) fn save_statistics(&self,calendar:[u16;6])->Result<Vec<(&'static str,u32,Vec<Vec<u8>>)>,String> {
        use crate::challenge::hom_save;
        let stats=self.statistics.lock().map_err(|_|"StatsBackEnd lock")?;
        let mut tuples:Vec<Vec<u8>>=stats.achievements.iter().map(|&b|vec![u8::from(b)]).collect();
        for values in [&stats.statistics[..],&stats.dmo_statistics[..]] {tuples.extend(values.iter().map(|v|v.to_be_bytes().to_vec()));}
        tuples.extend(stats.personal_bests.iter().map(|v|v.value.to_be_bytes().to_vec()));
        tuples.extend(stats.movable_object_times.iter().map(|v|v.to_be_bytes().to_vec()));
        let hom=stats.hom.lock().map_err(|_|"HoM stats lock")?;
        tuples.push(hom_save::totals(&hom));tuples.push((hom.top.len() as u32).to_be_bytes().to_vec());
        for i in 0..10 {tuples.push(hom_save::record(hom.top.get(i),hom.top_metadata.get(i),calendar)?);}
        Ok(vec![(hom_save::DATA,13,tuples)])
    }
}

/// `StaticStatsInfoTable` (`0x82fd9db8`)'s visible statistics (`mVisible`), the 50
/// `StatsBackEndCore::GetStatsCount` (`0x82a72d88`) counts; `GetStatsName` (`0x82a72dc0`) indexes
/// the table itself, whose visible rows come first.
const STAT_NAMES: [&str; 50] = [
    "ID_RECORD_DMO_01", "ID_RECORD_DMO_03", "ID_RECORD_STATS_88", "ID_RECORD_STATS_86", "ID_RECORD_STATS_46",
    "ID_RECORD_STATS_50", "ID_RECORD_STATS_91", "ID_RECORD_STATS_36", "ID_RECORD_STATS_38", "ID_RECORD_STATS_92",
    "ID_RECORD_STATS_17", "ID_RECORD_STATS_16", "ID_RECORD_STATS_94", "ID_RECORD_STATS_95", "ID_RECORD_STATS_27",
    "ID_RECORD_STATS_32", "ID_RECORD_STATS_33", "ID_RECORD_STATS_85", "ID_RECORD_STATS_21", "ID_RECORD_STATS_30",
    "ID_RECORD_STATS_97", "ID_RECORD_STATS_98", "ID_RECORD_STATS_99", "ID_RECORD_STATS_100", "ID_RECORD_STATS_101",
    "ID_RECORD_STATS_102", "ID_RECORD_STATS_103", "ID_RECORD_STATS_104", "ID_RECORD_STATS_105", "ID_RECORD_STATS_106",
    "ID_RECORD_STATS_107", "ID_RECORD_STATS_03", "ID_RECORD_STATS_05", "ID_RECORD_STATS_08", "ID_RECORD_STATS_09",
    "ID_RECORD_STATS_10", "ID_RECORD_STATS_82", "ID_RECORD_STATS_20", "ID_RECORD_STATS_19", "ID_RECORD_STATS_110",
    "ID_RECORD_STATS_111", "ID_RECORD_STATS_112", "ID_RECORD_STATS_113", "ID_RECORD_STATS_114", "ID_RECORD_STATS_115",
    "ID_RECORD_STATS_116", "ID_RECORD_STATS_117", "ID_RECORD_STATS_118", "ID_RECORD_STATS_119", "ID_RECORD_STATS_120",
];

/// `StaticPersonalBestsInfoTable` (`0x82fda0c8`)'s names (`GetPersonalBestsName` `0x82a72e40`).
const PERSONAL_BEST_NAMES: [&str; 22] = [
    "ID_RECORD_BESTS_09", "ID_RECORD_BESTS_18", "ID_RECORD_BESTS_10", "ID_RECORD_BESTS_11", "ID_RECORD_BESTS_12",
    "ID_RECORD_BESTS_13", "ID_RECORD_BESTS_14", "ID_RECORD_BESTS_15", "ID_RECORD_BESTS_20", "ID_RECORD_BESTS_21",
    "ID_RECORD_BESTS_22", "ID_RECORD_BESTS_24", "ID_RECORD_BESTS_25", "ID_RECORD_BESTS_16", "ID_RECORD_BESTS_33",
    "ID_RECORD_BESTS_34", "ID_RECORD_BESTS_01", "ID_RECORD_BESTS_02", "ID_RECORD_BESTS_03", "ID_RECORD_BESTS_05",
    "ID_RECORD_BESTS_06", "ID_RECORD_BESTS_07",
];

/// The pedestrian types of statistics 51 to 63, which statistic 50 names by the most hit.
const PEDESTRIAN_LABELS: [&str; 13] = [
    "ID_LIVINGWORLD_BUM", "ID_LIVINGWORLD_JOCK", "ID_LIVINGWORLD_TEENGIRL", "ID_LIVINGWORLD_TEENGUY",
    "ID_LIVINGWORLD_ADULTMALE", "ID_LIVINGWORLD_ADULTFEMALE", "ID_LIVINGWORLD_BUSINESSMAN",
    "ID_LIVINGWORLD_BUSINESSWOMAN", "ID_LIVINGWORLD_MALESKATER", "ID_LIVINGWORLD_FEMALESKATER",
    "ID_LIVINGWORLD_MALETOURIST", "ID_LIVINGWORLD_FEMALETOURIST", "ID_LIVINGWORLD_SECURITYGUARD",
];

/// `PARTSLOT::SKATE_DECK`.
const SKATE_DECK: usize = 11;

impl StatsBackend {
    /// `StatsBackEnd::GetStatsValue(item)` (`0x82568270`).
    #[expect(clippy::cast_precision_loss, reason = "the native (float) of the count")]
    fn stat_value(&self, item: usize) -> crate::ui::skate_profile::StatValue {
        use crate::ui::skate_profile::StatValue;
        let s = &self.statistics;
        match item {
            0 => StatValue::time(self.dmo_statistics[0]),
            // GetNumObjectTypesMoved (0x8255f0d8): the object types moved for 1e-4 s or more.
            1 => StatValue::Number(self.movable_object_times.iter().filter(|&&time| time >= f32::from_bits(0x38d1_b717)).count() as f32),
            3 => StatValue::Percent(if s[6] <= 0.0 { 0.0 } else { (-(s[7] / s[6])).mul_add(100.0, 100.0) }),
            4 | 5 | 19 | 21 | 30 | 32 => StatValue::time(s[item]),
            9..=11 | 23 | 27 | 28 => StatValue::Currency(s[item]),
            18 | 20 => StatValue::DistanceInt(crate::ui::skate_profile::fctiwz(s[item])),
            25 => StatValue::Percent(s[25]),
            29 | 31 | 39 | 40 | 49 => StatValue::DistanceFloat(s[item]),
            35 => StatValue::Percent(if s[35] <= 0.0 { 0.0 } else { s[34] / s[35] * 100.0 }),
            50 => {
                let mut most = None;
                let mut hits = 0.0;
                for (label, &value) in PEDESTRIAN_LABELS.iter().zip(&s[51..64]) {
                    if value > hits {
                        most = Some(*label);
                        hits = value;
                    }
                }
                StatValue::Label(most.unwrap_or("ID_COMMON_HYPHEN"))
            }
            _ => StatValue::Number(s[item]),
        }
    }

    /// `StatsBackEnd::GetPersonalBestsValue(item)` (`0x8255f1d8`): `ID_COMMON_HYPHEN` unless the
    /// value reaches the threshold.
    fn personal_best_value(&self, item: usize) -> crate::ui::skate_profile::StatValue {
        use crate::ui::skate_profile::StatValue;
        let best = self.personal_bests[item];
        if best.value >= best.threshold {
            match item {
                0 | 2 | 17 | 20 => StatValue::time(best.value),
                3..=12 | 14 => StatValue::DistanceFloat(best.value),
                _ => StatValue::Number(best.value),
            }
        } else {
            StatValue::Hal("ID_COMMON_HYPHEN")
        }
    }
}

impl LocalBackendOwner {
    /// The skate.Profile rows: each visible statistic's `GetStatsValue` and each personal best's
    /// `GetPersonalBestsValue`, with `Stats.GetTotalCashEarned` (statistic 10). Statistic 8 takes
    /// the deck count first, as `GetStatsValue(8)` stores it.
    #[expect(clippy::cast_precision_loss, reason = "the native (float) of the part count")]
    pub(crate) fn skate_profile_rows(&self, decks: u32) -> Result<crate::ui::skate_profile::Lists, String> {
        use crate::ui::skate_profile::{Lists, Row};
        let mut stats = self.statistics.lock().map_err(|_| "StatsBackEnd lock")?;
        stats.statistics[8] = decks as f32;
        let rows = STAT_NAMES.iter().enumerate().map(|(item, &title)| Row { title, value: stats.stat_value(item) }).collect();
        let bests = PERSONAL_BEST_NAMES.iter().enumerate().map(|(item, &title)| Row { title, value: stats.personal_best_value(item) }).collect();
        Ok(Lists { stats: rows, personal_bests: bests, total_cash: stats.stat_value(10) })
    }
}

/// `CACRecord::PartCount(SKATE_DECK)` (`0x82b67290`) of the profile's created skater: the decks it
/// owns, and the one it rides when that is a part. A profile without a saved skater counts none.
pub(crate) fn profile_deck_count() -> Result<u32, String> {
    use crate::edit_skater_profile::EditSkaterProfileStore;
    let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let Some(record) = store.load_record().map_err(|e| e.to_string())? else { return Ok(0) };
    let installed = record.installed_parts[SKATE_DECK] != skate2_formats::skater_profile::CacPartId::invalid();
    Ok(u32::try_from(record.owned_parts[SKATE_DECK].len()).unwrap_or(u32::MAX).saturating_add(u32::from(installed)))
}
