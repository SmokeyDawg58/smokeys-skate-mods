//! `PhysOutConditioner_Grinds::Update` 0x82e2f9d0 and `PhysOutConditioner_GameCam`
//! Update 0x82e2d380 (its tail branch to `UpdateGrindTarget` 0x82e2d000): the
//! second and last `UpdateAll` 0x82e29f88 children. Intents, MissionSystem,
//! Camera and Stats share the empty Update 0x825187c0.
use crate::physics_state_ids::{PHYSICS_CATEGORY_GRIND, PHYSICS_CATEGORY_GROUND};
use super::{
    AllocationSite, Conditioners, PhysicalPlayerPhysOut, SkeletonRootVector as V,
    grinds::{Approach, Grinds, TimedApproach},
    simple::GameCam,
};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences, grind_states::GrindCvar as C},
    physical_player_grind_manager::view::grind_type_raw,
    physical_player_phys_out::{
        PhysOutAccessError, PhysOutMotionVector,
        construct::{ConcretePhysOutError, prior_input::PriorInputReadError},
    },
    physics::GrindType,
    play_animation::FastString30,
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate},
    simulation_drive_jacobian::refine_drive_reciprocal,
};
use std::sync::Arc;

type Chromosome = super::super::grinds::GrindChromosome;

/// `InitGrindChromosomeFunction` 0x82e25c80 as copied by
/// `GrindNameTableInitializer` 0x82e2e890: (eSkatingGrindType, SkatingGrindNames
/// text, GrindNameTable text, EScorableID) at approach*80 + location*40 +
/// twist*20 + tilt*10 + orientation*5 + type. Entries 52, 62 and 72 pair
/// `FS_TAILSLIDE` with `eScorableID_bs_tailslide` in the shipped data.
const GRIND_FUNCTIONS: [(i32, &str, &str, i32); 160] = [
    (3, "FS_50_50", "FS 50-50", 23),
    (1, "FS_LIP", "FS Lipslide", 28),
    (13, "FS_NOSEBLUNT", "FS Noseblunt", 29),
    (16, "FS_OVERCROOK", "FS Overcrooks", 32),
    (13, "FS_NOSEBLUNT", "FS Noseblunt", 29),
    (3, "FS_50_50", "FS 50-50", 23),
    (2, "FS_BOARD", "FS Boardslide", 25),
    (4, "FS_NOSESLIDE", "FS Noseslide", 31),
    (15, "FS_CROOK", "FS Crooked", 26),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (3, "FS_50_50", "FS 50-50", 23),
    (1, "FS_LIP", "FS Lipslide", 28),
    (13, "FS_NOSEBLUNT", "FS Noseblunt", 29),
    (12, "FS_OVERWILLY", "FS Over Willy", 33),
    (13, "FS_NOSEBLUNT", "FS Noseblunt", 29),
    (3, "FS_50_50", "FS 50-50", 23),
    (2, "FS_BOARD", "FS Boardslide", 25),
    (4, "FS_NOSESLIDE", "FS Noseslide", 31),
    (11, "FS_WILLY", "FS Willy", 37),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (3, "FS_50_50", "FS 50-50", 23),
    (1, "FS_LIP", "FS Lipslide", 28),
    (13, "FS_NOSEBLUNT", "FS Noseblunt", 29),
    (6, "FS_NOSEGRIND", "FS Nosegrind", 30),
    (13, "FS_NOSEBLUNT", "FS Noseblunt", 29),
    (3, "FS_50_50", "FS 50-50", 23),
    (2, "FS_BOARD", "FS Boardslide", 25),
    (4, "FS_NOSESLIDE", "FS Noseslide", 31),
    (6, "FS_NOSEGRIND", "FS Nosegrind", 30),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (3, "FS_50_50", "FS 50-50", 23),
    (1, "FS_LIP", "FS Lipslide", 28),
    (13, "FS_NOSEBLUNT", "FS Noseblunt", 29),
    (12, "FS_OVERWILLY", "FS Over Willy", 33),
    (13, "FS_NOSEBLUNT", "FS Noseblunt", 29),
    (3, "FS_50_50", "FS 50-50", 23),
    (2, "FS_BOARD", "FS Boardslide", 25),
    (4, "FS_NOSESLIDE", "FS Noseslide", 31),
    (11, "FS_WILLY", "FS Willy", 37),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (3, "FS_50_50", "FS 50-50", 23),
    (1, "FS_LIP", "FS Lipslide", 28),
    (5, "FS_TAILSLIDE", "FS Tailslide", 36),
    (7, "FS_5_O", "FS 5-0", 22),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (3, "FS_50_50", "FS 50-50", 23),
    (2, "FS_BOARD", "FS Boardslide", 25),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
    (8, "FS_SALAD", "FS Salad", 34),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
    (3, "FS_50_50", "FS 50-50", 23),
    (1, "FS_LIP", "FS Lipslide", 28),
    (5, "FS_TAILSLIDE", "FS Tailslide", 20),
    (9, "FS_SMITH", "FS Smith", 35),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (3, "FS_50_50", "FS 50-50", 23),
    (2, "FS_BOARD", "FS Boardslide", 25),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
    (10, "FS_FEEBLE", "FS Feeble", 27),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
    (3, "FS_50_50", "FS 50-50", 23),
    (1, "FS_LIP", "FS Lipslide", 28),
    (5, "FS_TAILSLIDE", "FS Tailslide", 20),
    (7, "FS_5_O", "FS 5-0", 22),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (3, "FS_50_50", "FS 50-50", 23),
    (2, "FS_BOARD", "FS Boardslide", 25),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
    (7, "FS_5_O", "FS 5-0", 22),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
    (3, "FS_50_50", "FS 50-50", 23),
    (1, "FS_LIP", "FS Lipslide", 28),
    (5, "FS_TAILSLIDE", "FS Tailslide", 20),
    (9, "FS_SMITH", "FS Smith", 35),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (3, "FS_50_50", "FS 50-50", 23),
    (2, "FS_BOARD", "FS Boardslide", 25),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
    (10, "FS_FEEBLE", "FS Feeble", 27),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
    (19, "BS_50_50", "BS 50-50", 7),
    (18, "BS_BOARD", "BS Boardslide", 9),
    (20, "BS_NOSESLIDE", "BS Noseslide", 15),
    (31, "BS_CROOK", "BS Crooked", 10),
    (13, "FS_NOSEBLUNT", "FS Noseblunt", 29),
    (19, "BS_50_50", "BS 50-50", 7),
    (17, "BS_LIP", "BS Lipslide", 12),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (32, "BS_OVERCROOK", "BS Overcrooks", 16),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (19, "BS_50_50", "BS 50-50", 7),
    (18, "BS_BOARD", "BS Boardslide", 9),
    (20, "BS_NOSESLIDE", "BS Noseslide", 15),
    (27, "BS_WILLY", "BS Willy", 21),
    (13, "FS_NOSEBLUNT", "FS_Noseblunt", 29),
    (19, "BS_50_50", "BS 50-50", 7),
    (17, "BS_LIP", "BS Lipslide", 12),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (28, "BS_OVERWILLY", "BS Overwilly", 17),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (19, "BS_50_50", "BS 50-50", 7),
    (18, "BS_BOARD", "BS Boardslide", 9),
    (20, "BS_NOSESLIDE", "BS Noseslide", 15),
    (22, "BS_NOSEGRIND", "BS Nosegrind", 14),
    (13, "FS_NOSEBLUNT", "FS_Noseblunt", 29),
    (19, "BS_50_50", "BS 50-50", 7),
    (17, "BS_LIP", "BS Lipslide", 12),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (22, "BS_NOSEGRIND", "BS Nosegrind", 14),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (19, "BS_50_50", "BS 50-50", 7),
    (18, "BS_BOARD", "BS Boardslide", 9),
    (20, "BS_NOSESLIDE", "BS Noseslide", 15),
    (27, "BS_WILLY", "BS Willy", 21),
    (13, "FS_NOSEBLUNT", "FS_Noseblunt", 29),
    (19, "BS_50_50", "BS 50-50", 7),
    (17, "BS_LIP", "BS Lipslide", 12),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (28, "BS_OVERWILLY", "BS Over Willy", 17),
    (29, "BS_NOSEBLUNT", "BS Noseblunt", 13),
    (19, "BS_50_50", "BS 50-50", 7),
    (18, "BS_BOARD", "BS Boardslide", 9),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (24, "BS_SALAD", "BS Salad", 18),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (19, "BS_50_50", "BS 50-50", 7),
    (17, "BS_LIP", "BS Lipslide", 12),
    (21, "BS_TAILSLIDE", "BS Tailslide", 20),
    (23, "BS_5_O", "BS 5-0", 6),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
    (19, "BS_50_50", "BS 50-50", 7),
    (18, "BS_BOARD", "BS Boardslide", 9),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (26, "BS_FEEBLE", "BS Feeble", 11),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (19, "BS_50_50", "BS 50-50", 7),
    (17, "BS_LIP", "BS Lipslide", 12),
    (21, "BS_TAILSLIDE", "BS Tailslide", 20),
    (25, "BS_SMITH", "BS Smith", 19),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
    (19, "BS_50_50", "BS 50-50", 7),
    (18, "BS_BOARD", "BS Boardslide", 9),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (23, "BS_5_O", "BS 5-0", 6),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (19, "BS_50_50", "BS 50-50", 7),
    (17, "BS_LIP", "BS Lipslide", 12),
    (21, "BS_TAILSLIDE", "BS Tailslide", 20),
    (23, "BS_5_O", "BS 5-0", 6),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
    (19, "BS_50_50", "BS 50-50", 7),
    (18, "BS_BOARD", "BS Boardslide", 9),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (26, "BS_FEEBLE", "BS Feeble", 11),
    (30, "BS_BLUNT", "BS Bluntslide", 8),
    (19, "BS_50_50", "BS 50-50", 7),
    (17, "BS_LIP", "BS Lipslide", 12),
    (21, "BS_TAILSLIDE", "BS Tailslide", 20),
    (25, "BS_SMITH", "BS Smith", 19),
    (14, "FS_BLUNT", "FS Bluntslide", 24),
];

#[derive(Debug)]
pub(crate) enum GrindsConditionerError<N> {
    ForeignOutput,
    Missing(AllocationSite),
    Read(PriorInputReadError),
    Component(ConcretePhysOutError<std::convert::Infallible>),
    Access(PhysOutAccessError),
    Numeric(N),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    /// grindconditioner.cpp:151 (82e2ed28): approach, location, twist, tilt,
    /// orientation, type.
    ChromosomeAssert([i32; 6]),
    /// `FiniteHistory::RetrieveIMPL` 0x82e2a250 assertions.
    History {
        age: i32,
        count: i32,
    },
    /// A flattened chromosome index outside the 160 initialized entries.
    TableIndex([i32; 6]),
    Name(String),
}
impl<N: std::fmt::Debug> std::fmt::Display for GrindsConditionerError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignOutput => f.write_str("grinds conditioner belongs to another PhysOut"),
            Self::Missing(site) => write!(f, "grinds conditioner missing {site:?}"),
            Self::Read(e) => write!(f, "grinds conditioner component: {e:?}"),
            Self::Component(e) => write!(f, "grinds conditioner direct read: {e:?}"),
            Self::Access(e) => write!(f, "grinds conditioner output access: {e:?}"),
            Self::Numeric(e) => write!(f, "grinds conditioner numeric collaborator: {e:?}"),
            Self::Cvar(e) => write!(f, "grinds conditioner cvar: {e:?}"),
            Self::Overlay(e) => write!(f, "grinds conditioner overlay: {e}"),
            Self::ChromosomeAssert(c) => write!(f, "grind chromosome assertion 82e2ed28: {c:?}"),
            Self::History { age, count } => {
                write!(f, "approach history retrieve age {age} of {count}")
            }
            Self::TableIndex(c) => write!(f, "grind chromosome table index {c:?}"),
            Self::Name(e) => write!(f, "skating grind name: {e}"),
        }
    }
}
type E<N> = GrindsConditionerError<N>;

fn neg(value: V) -> V {
    value.map(|lane| -lane)
}
fn lanes(c: &Chromosome) -> [i32; 6] {
    [
        c.approach,
        c.location,
        c.twist,
        c.tilt,
        c.orientation,
        grind_type_raw(c.grind_type),
    ]
}
fn table_index<N>(c: &Chromosome) -> Result<usize, E<N>> {
    let l = lanes(c);
    let index = l[0]
        .wrapping_mul(80)
        .wrapping_add(l[1].wrapping_mul(40))
        .wrapping_add(l[2].wrapping_mul(20))
        .wrapping_add(l[3].wrapping_mul(10))
        .wrapping_add(l[4].wrapping_mul(5))
        .wrapping_add(l[5]);
    usize::try_from(index)
        .ok()
        .filter(|&i| i < GRIND_FUNCTIONS.len())
        .ok_or(E::TableIndex(l))
}
fn skating_name<N>(index: usize) -> Result<FastString30, E<N>> {
    FastString30::encode(GRIND_FUNCTIONS[index].1).map_err(|e| E::Name(format!("{e:?}")))
}

/// `sprintf(dest + at, "%s", text)`: the text and its terminator.
fn put(dest: &mut [u8; 256], at: usize, text: &str) -> usize {
    dest[at..at + text.len()].copy_from_slice(text.as_bytes());
    dest[at + text.len()] = 0;
    text.len()
}
/// `NameGrind` 0x82e2aff0: each part is skipped only when `255 - strlen`
/// equals the running length.
fn name_grind<N>(c: &Chromosome, dest: &mut [u8; 256]) -> Result<(), E<N>> {
    let mut len = put(dest, 0, "Grind Name : ( ");
    let kind = match c.grind_type {
        GrindType::FiftyFifty => "5050 ",
        GrindType::Boardslide => "BOARD ",
        GrindType::Tipslide => "TIP ",
        GrindType::FiveO => "5_O ",
        GrindType::Backslash => "BACKSLASH ",
        GrindType::None => "NA ",
    };
    for part in [
        if c.approach == 0 { "A-FS " } else { "A-BS " },
        if c.location == 0 { "NOSE " } else { "TAIL " },
        if c.twist == 0 { "TWST " } else { "STRT " },
        if c.tilt == 0 { "HI " } else { "LO " },
        if c.orientation == 0 { "FOR " } else { "BAK " },
        kind,
    ] {
        if 255 - part.len() != len {
            len += put(dest, len, part);
        }
    }
    if len != 253 {
        len += put(dest, len, ") ");
    }
    let name = GRIND_FUNCTIONS[table_index::<N>(c)?].2;
    if 255 - name.len() != len {
        put(dest, len, name);
    }
    Ok(())
}

/// `SetGrindApproachReckoning` 0x82e29490: seven latched `PhysOut` reads.
fn approach_reckoning<N>(output: &PhysicalPlayerPhysOut) -> Result<Approach, E<N>> {
    let deck_side = output
        .try_guarded_skateboard_reckoning()
        .map_err(E::Read)?
        .transform
        .columns()[0];
    let position = output
        .try_guarded_skateboard_reckoning()
        .map_err(E::Read)?
        .transform
        .columns()[3];
    let velocity = output
        .motion_vector(PhysOutMotionVector::DeckVelocity)
        .map_err(E::Access)?
        .value;
    let right_foot_position = output
        .try_guarded_skeleton_output()
        .map_err(E::Read)?
        .skeleton_foot_right_pos;
    let left_foot_position = output
        .try_guarded_skeleton_output()
        .map_err(E::Read)?
        .skeleton_foot_left_pos;
    let right_foot_direction = output
        .try_guarded_skeleton_output()
        .map_err(E::Read)?
        .skeleton_foot_right_look;
    let left_foot_direction = output
        .try_guarded_skeleton_output()
        .map_err(E::Read)?
        .skeleton_foot_left_look;
    Ok(Approach {
        deck_side,
        position,
        velocity,
        right_foot_position,
        left_foot_position,
        right_foot_direction,
        left_foot_direction,
    })
}

impl Grinds {
    /// `Update` 0x82e2f9d0.
    fn update<N: ReckoningNumeric>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        n: &mut N,
    ) -> Result<(), E<N::Error>> {
        self.updates = self.updates.wrapping_add(1);
        self.update_grind_chromosome(output, cvars, n)?;
        if !output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .is_grinding
        {
            return Ok(());
        }
        let chromosome = output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .grind_chromosome
            .clone();
        let scoring = output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .grind_chromosome_scoring
            .clone();
        name_grind(&chromosome, &mut self.grind_name)?;
        name_grind(&scoring, &mut self.scoring_grind_name)?;
        if cvars
            .try_grind_bool(C::kbGrindConditionerDebugDraw)
            .map_err(E::Cvar)?
        {
            // DebugDraw 0x82e2b2f8: the singleton assertion; both text calls
            // are the shared empty stub in this build.
            overlay.require().map_err(E::Overlay)?;
        }
        let index = table_index(&chromosome)?;
        output
            .try_guarded_grinds_mut()
            .map_err(E::Read)?
            .skating_grind_type = GRIND_FUNCTIONS[index].0;
        let name = skating_name(index)?;
        output
            .try_guarded_grinds_mut()
            .map_err(E::Read)?
            .skating_grind_name = name;
        let index = table_index(&scoring)?;
        output
            .try_guarded_grinds_mut()
            .map_err(E::Read)?
            .skating_grind_type_scoring = GRIND_FUNCTIONS[index].0;
        let name = skating_name(index)?;
        output
            .try_guarded_grinds_mut()
            .map_err(E::Read)?
            .skating_grind_name_scoring = name;
        output
            .try_guarded_grinds_mut()
            .map_err(E::Read)?
            .grind_scorable_id = GRIND_FUNCTIONS[index].3;
        Ok(())
    }

    /// `UpdateGrindChromosome` 0x82e2ebe8. The three getters and the type read
    /// are direct `PhysOut` loads without the write-latch check.
    fn update_grind_chromosome<N: ReckoningNumeric>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
        n: &mut N,
    ) -> Result<(), E<N::Error>> {
        let category = output
            .try_guarded_state_output()
            .map_err(E::Read)?
            .physics_state_category;
        self.watch_for_approach(output, category, cvars, n)?;
        if output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .is_grinding
        {
            let mut c = Chromosome::reset_values();
            c.approach = self.approach;
            c.location = self.location;
            c.orientation = grind_orientation(output, n)?;
            c.tilt = grind_tilt(output, cvars, n)?;
            c.twist = grind_twist(output, n)?;
            c.grind_type = output
                .try_camera_grinds_output()
                .map_err(E::Component)?
                .grind_type;
            let l = lanes(&c);
            if !l[..5].iter().all(|v| (0..2).contains(v)) || !(0..5).contains(&l[5]) {
                return Err(E::ChromosomeAssert(l));
            }
            if self.last_physics_category == PHYSICS_CATEGORY_GRIND {
                if c == self.candidate_chromosome {
                    self.frames_of_chromosome_equality =
                        self.frames_of_chromosome_equality.wrapping_add(1);
                } else {
                    self.candidate_chromosome = c;
                    self.frames_of_chromosome_equality = 0;
                }
            } else {
                let scoring = cvars
                    .try_grind_int(C::kMinChromosomeEqualityFrames_Scoring)
                    .map_err(E::Cvar)?;
                let normal = cvars
                    .try_grind_int(C::kMinChromosomeEqualityFrames)
                    .map_err(E::Cvar)?;
                let larger = if scoring > normal { scoring } else { normal };
                self.frames_of_chromosome_equality = larger.wrapping_add(1);
                self.candidate_chromosome = c;
            }
            let frames = self.frames_of_chromosome_equality;
            if frames
                > cvars
                    .try_grind_int(C::kMinChromosomeEqualityFrames)
                    .map_err(E::Cvar)?
            {
                self.stable_chromosome
                    .clone_from(&self.candidate_chromosome);
            }
            if frames
                > cvars
                    .try_grind_int(C::kMinChromosomeEqualityFrames_Scoring)
                    .map_err(E::Cvar)?
            {
                self.scoring_stable_chromosome
                    .clone_from(&self.candidate_chromosome);
            }
            output
                .try_guarded_grinds_mut()
                .map_err(E::Read)?
                .volatile_chromosome
                .clone_from(&self.candidate_chromosome);
            output
                .try_guarded_grinds_mut()
                .map_err(E::Read)?
                .grind_chromosome
                .clone_from(&self.stable_chromosome);
            output
                .try_guarded_grinds_mut()
                .map_err(E::Read)?
                .grind_chromosome_scoring
                .clone_from(&self.scoring_stable_chromosome);
        }
        self.last_physics_category = category;
        Ok(())
    }

    /// `WatchForApproach` 0x82e2e9e0.
    fn watch_for_approach<N: ReckoningNumeric>(
        &mut self,
        output: &PhysicalPlayerPhysOut,
        category: i32,
        cvars: &InputCvarReferences,
        n: &mut N,
    ) -> Result<(), E<N::Error>> {
        self.manage_approach_reckoning(output)?;
        let grind_type = output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .grind_type;
        let mut calc_approach = false;
        if category == PHYSICS_CATEGORY_GRIND {
            if self.last_physics_category != PHYSICS_CATEGORY_GRIND
                && self.consecutive_non_grind_frames
                    > cvars.try_min_approach_non_grind_frames().map_err(E::Cvar)?
            {
                calc_approach = true;
                self.recall_ground_approach()?;
            } else if grind_type != self.last_grind_type {
                self.recall_ground_approach()?;
            }
        }
        if calc_approach {
            self.calc_approach(output, n)?;
        }
        self.calc_location(output, n)?;
        self.last_grind_type = grind_type;
        self.consecutive_non_grind_frames = if category == PHYSICS_CATEGORY_GRIND {
            0
        } else {
            self.consecutive_non_grind_frames.wrapping_add(1)
        };
        Ok(())
    }

    /// `ManageApproachReckoning` 0x82e2cca8.
    fn manage_approach_reckoning<N>(&mut self, output: &PhysicalPlayerPhysOut) -> Result<(), E<N>> {
        if output
            .try_guarded_air_output()
            .map_err(E::Read)?
            .is_preparing_to_jump
        {
            self.last_ollie_approach = approach_reckoning(output)?;
            self.valid_history_entries = 0;
        }
        if output
            .try_guarded_state_output()
            .map_err(E::Read)?
            .physics_state_category
            == PHYSICS_CATEGORY_GROUND
        {
            let value = approach_reckoning(output)?;
            // FiniteHistory::Add 0x82e2c5d8 with the current mUpdates tick.
            let count = self.valid_history_entries;
            let slot = usize::try_from(count.wrapping_rem(30))
                .map_err(|_| E::History { age: -1, count })?;
            self.approach_ground_history[slot] = TimedApproach {
                value,
                tick: u32::from_ne_bytes(self.updates.to_ne_bytes()),
            };
            self.valid_history_entries = if count == 59 {
                30
            } else {
                count.wrapping_add(1)
            };
        }
        Ok(())
    }

    /// The oldest ground approach once thirty are held; otherwise the native
    /// copy is the ollie approach onto itself.
    fn recall_ground_approach<N>(&mut self) -> Result<(), E<N>> {
        let count = self.valid_history_entries;
        if count >= 30 {
            let age = count.min(30) - 1;
            // RetrieveIMPL 0x82e2a250.
            if !(age > -1 && age < count && count > 0) {
                return Err(E::History { age, count });
            }
            let index = count.wrapping_sub(age).wrapping_sub(1).wrapping_rem(30);
            let index = usize::try_from(index)
                .ok()
                .filter(|&i| i < 30)
                .ok_or(E::History { age, count })?;
            self.last_ollie_approach = self.approach_ground_history[index].value.clone();
        }
        Ok(())
    }

    /// `CalcApproach` 0x82e29190: the rail side against the feet's deck side.
    fn calc_approach<N: ReckoningNumeric>(
        &mut self,
        output: &PhysicalPlayerPhysOut,
        n: &mut N,
    ) -> Result<(), E<N::Error>> {
        let gap = &self.last_ollie_approach;
        let grind_position = output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .grind_position;
        let offset = gap.position.zip(grind_position, |a, b| a - b);
        let skew = output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .grind_skew;
        let side = if n.dot_vector(skew, offset).map_err(E::Numeric)? > 0.0 {
            skew
        } else {
            neg(skew)
        };
        let feet = gap
            .right_foot_direction
            .zip(gap.left_foot_direction, |a, b| a + b);
        let foot_side = if n.dot_vector(feet, gap.deck_side).map_err(E::Numeric)? > 0.0 {
            gap.deck_side
        } else {
            neg(gap.deck_side)
        };
        self.approach = i32::from(n.dot_vector(foot_side, side).map_err(E::Numeric)? > 0.0);
        Ok(())
    }

    /// `CalcLocation` 0x82e29340: nose or tail, only for 5-0, tipslide and
    /// backslash.
    fn calc_location<N: ReckoningNumeric>(
        &mut self,
        output: &PhysicalPlayerPhysOut,
        n: &mut N,
    ) -> Result<(), E<N::Error>> {
        let grind_type = output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .grind_type;
        if !matches!(
            grind_type,
            GrindType::FiveO | GrindType::Tipslide | GrindType::Backslash
        ) {
            self.location = 0;
            return Ok(());
        }
        output.try_guarded_skateboard_reckoning().map_err(E::Read)?;
        let deck = output
            .try_guarded_skateboard_reckoning()
            .map_err(E::Read)?
            .deck_position;
        let grind_position = output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .grind_position;
        let offset = grind_position.zip(deck, |a, b| a - b);
        let z = output
            .try_guarded_skateboard_reckoning()
            .map_err(E::Read)?
            .effective_transform
            .columns()[2];
        self.location = if n.dot_vector(offset, z).map_err(E::Numeric)? > 0.0 {
            0
        } else {
            1
        };
        Ok(())
    }
}

/// `GetGrindOrientation` 0x82e29838: the feet's deck side along the rail.
fn grind_orientation<N: ReckoningNumeric>(
    output: &PhysicalPlayerPhysOut,
    n: &mut N,
) -> Result<i32, E<N::Error>> {
    let skeleton = output.try_skeleton_output().map_err(E::Component)?;
    let board = output.try_camera_board_output().map_err(E::Component)?;
    let grinds = output.try_camera_grinds_output().map_err(E::Component)?;
    let feet = skeleton
        .skeleton_foot_right_look
        .zip(skeleton.skeleton_foot_left_look, |a, b| a + b);
    let x = board.transform.columns()[0];
    let side = if n.dot_vector(feet, x).map_err(E::Numeric)? > 0.0 {
        x
    } else {
        neg(x)
    };
    let forwards = n
        .dot_vector(side, grinds.grind_primitive_dir)
        .map_err(E::Numeric)?
        > 0.0;
    Ok(i32::from(!forwards))
}

/// `GetGrindTilt` 0x82e29760: the grind normal against the deck Z that points
/// away from the rail, compared with `kDeckTiltForUp`.
fn grind_tilt<N: ReckoningNumeric>(
    output: &PhysicalPlayerPhysOut,
    cvars: &InputCvarReferences,
    n: &mut N,
) -> Result<i32, E<N::Error>> {
    let grinds = output.try_camera_grinds_output().map_err(E::Component)?;
    let board = output.try_camera_board_output().map_err(E::Component)?;
    let offset = board.transform.columns()[3].zip(grinds.grind_position, |a, b| a - b);
    let z = board.transform.columns()[2];
    let z = if n.dot_vector(offset, z).map_err(E::Numeric)? > 0.0 {
        z
    } else {
        neg(z)
    };
    let tilt = n.dot_vector(grinds.grind_normal, z).map_err(E::Numeric)?;
    let limit = cvars.try_grind_float(C::kDeckTiltForUp).map_err(E::Cvar)?;
    Ok(if tilt > limit { 0 } else { 1 })
}

/// `GetGrindTwist` 0x82e29610: the nose projected off the grind normal, then
/// its alignment with the rail against 0x3f70902e.
fn grind_twist<N: ReckoningNumeric>(
    output: &PhysicalPlayerPhysOut,
    n: &mut N,
) -> Result<i32, E<N::Error>> {
    let grinds = output.try_camera_grinds_output().map_err(E::Component)?;
    let board = output.try_camera_board_output().map_err(E::Component)?;
    let normal = grinds.grind_normal;
    let nose = board.deck_nose_dir;
    let along = n.dot_vector(nose, normal).map_err(E::Numeric)?;
    let projected = nose.zip(normal, |a, b| a - b * along);
    let squared = n.dot_vector(projected, projected).map_err(E::Numeric)?;
    let seed = n.reciprocal_sqrt_estimate(squared).map_err(E::Numeric)?;
    let length = length_from_squared_and_estimate(squared, seed);
    if length <= f32::from_bits(0x3c23_d70a) {
        return Ok(1);
    }
    let seed = n.reciprocal_estimate(length).map_err(E::Numeric)?;
    let inverse = refine_drive_reciprocal(length, seed);
    let unit = projected.map(|lane| inverse * lane);
    let dir = grinds.grind_primitive_dir;
    let dot = n.dot_vector(unit, dir).map_err(E::Numeric)?;
    let alignment = if dot <= 0.0 {
        n.dot_vector(neg(unit), dir).map_err(E::Numeric)?
    } else {
        dot
    };
    Ok(if alignment <= f32::from_bits(0x3f70_902e) {
        0
    } else {
        1
    })
}

impl GameCam {
    /// `UpdateGrindTarget` 0x82e2d000 with `PopSmoother::Update` 0x82e2a1c8.
    fn update_grind_target(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
    ) -> Result<(), E<std::convert::Infallible>> {
        if !output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .is_grinding
        {
            self.was_grinding = false;
            return Ok(());
        }
        let position = output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .grind_position;
        if !self.was_grinding {
            self.error = self.zero;
            self.current = position;
            self.previous = position;
        }
        let grind_type = output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .grind_type;
        let mut pop = self.was_grinding && grind_type != self.last_grind_type;
        self.last_grind_type = grind_type;
        let end = output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .primitive_end_pos;
        let start = output
            .try_guarded_grinds_output()
            .map_err(E::Read)?
            .primitive_start_pos;
        let mid = start.zip(end, |a, b| a + b).map(|lane| lane * 0.5);
        let last = self.last_primitive_midpoint;
        if self.was_grinding
            && (mid.xyz[0] != last[0] || mid.xyz[1] != last[1] || mid.xyz[2] != last[2])
        {
            pop = true;
        }
        self.last_primitive_midpoint = mid.xyz;
        if pop {
            let extrapolated = self
                .current
                .zip(self.current.zip(self.previous, |a, b| a - b), |a, b| a + b);
            self.error = extrapolated.zip(position, |a, b| a - b);
        }
        let smoothness = self.smoothness;
        self.error = self.error.map(|lane| lane * smoothness);
        let result = position.zip(self.error, |a, b| a + b);
        self.previous = self.current;
        self.current = result;
        output
            .try_guarded_grinds_mut()
            .map_err(E::Read)?
            .cam_target_root = result;
        self.was_grinding = true;
        Ok(())
    }
}

impl Conditioners {
    /// `UpdateAll` slot 1, after the empty Intents slot.
    pub(crate) fn try_update_grinds<N: ReckoningNumeric>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), GrindsConditionerError<N::Error>> {
        if !self.matches_output(output) {
            return Err(E::ForeignOutput);
        }
        let grinds = self
            .grinds
            .published_value_mut()
            .ok_or(E::Missing(AllocationSite::Grinds))?;
        if !Arc::ptr_eq(&grinds.output, output.occurrence()) {
            return Err(E::ForeignOutput);
        }
        grinds.payload.update(output, cvars, overlay, numeric)
    }
    /// `UpdateAll` slot 10, the last child.
    pub(crate) fn try_update_game_cam(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
    ) -> Result<(), GrindsConditionerError<std::convert::Infallible>> {
        if !self.matches_output(output) {
            return Err(E::ForeignOutput);
        }
        let game_cam = self
            .game_cam
            .published_value_mut()
            .ok_or(E::Missing(AllocationSite::GameCam))?;
        if !Arc::ptr_eq(&game_cam.output, output.occurrence()) {
            return Err(E::ForeignOutput);
        }
        game_cam.payload.update_grind_target(output)
    }
}
