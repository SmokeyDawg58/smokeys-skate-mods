//! The source FootPlantManager owned by the existing S2 physical manager.
//! This contains manager/query state only; the S2 body pool, solver, Actor and
//! simulation clock retain ownership of an independent Footplants replacement.
use super::FootPlantFields;
use crate::skate3_native::physics::footplant::Footplant;
use skate3_core::air::trajectory::Trajectory;

pub(crate) struct SourceFootplant {
    pub manager: Footplant,
    pub plant_wipeout: std::sync::Arc<skate3_core::player::wipeout::Settings>,
    pub pending: Option<Trajectory>,
    pub active: bool,
    diagnostic: Option<(bool, bool, bool, bool, Option<usize>)>,
}
struct Recipe {
    manager: Footplant,
    boneless: crate::skate3_native::physics::boneless::Boneless,
    plant_wipeout: std::sync::Arc<skate3_core::player::wipeout::Settings>,
}
fn recipe() -> Result<&'static Recipe, String> {
    static RECIPE: std::sync::OnceLock<Result<Recipe, String>> = std::sync::OnceLock::new();
    RECIPE
        .get_or_init(|| {
            let data = crate::skate3_native::physics::player::embedded_collections()?;
            Ok(Recipe {
                manager: Footplant::load(&data)?,
                boneless: crate::skate3_native::physics::boneless::Boneless::load(&data)?,
                plant_wipeout: std::sync::Arc::new(Footplant::load_plant_wipeout_settings(&data)?),
            })
        })
        .as_ref()
        .map_err(Clone::clone)
}
impl SourceFootplant {
    /// Called while the game is still loading, before any admitted physics
    /// tick. All mutable manager histories are private clones of this pristine
    /// recipe; only immutable wipeout settings are shared.
    pub(crate) fn new_boneless() -> Result<crate::skate3_native::physics::boneless::Boneless, String>
    {
        Ok(recipe()?.boneless.clone())
    }
    pub(crate) fn preload() -> Result<(), String> {
        recipe().map(|_| ())
    }
    pub(crate) fn load() -> Result<Self, String> {
        let recipe = recipe()?;
        Ok(Self {
            manager: recipe.manager.clone(),
            plant_wipeout: std::sync::Arc::clone(&recipe.plant_wipeout),
            pending: None,
            active: true,
            diagnostic: None,
        })
    }
    pub(crate) fn reset(&mut self, full: bool, enabled: bool) {
        if full {
            self.manager.full_reset();
            self.pending = None;
            self.active = false;
        } else {
            self.manager.reset();
            if !enabled {
                self.manager.enabled = false;
                self.pending = None;
                self.active = false;
            }
        }
    }
    pub(crate) fn record_changes(&mut self) {
        let f = &self.manager;
        let state = (f.candidate, f.hit, f.requested, f.perform, f.selected_toe);
        if self.diagnostic != Some(state) {
            self.diagnostic = Some(state);
            crate::diagnostics::test_log::event(
                "skate3_footplant_contact",
                format_args!(
                    "candidate={} hit={} requested={} perform={} toe={:?} time={} blend={} native_s2_solver=true",
                    f.candidate,
                    f.hit,
                    f.requested,
                    f.perform,
                    f.selected_toe,
                    f.contact_time,
                    f.target_blend
                ),
            );
        }
    }
    /// Read-only projection for S2's existing FootPlant Fill source. Its other
    /// manager fields retain constructor knowledge and are not used as drivers.
    pub(crate) fn fill_fields(&self) -> FootPlantFields {
        let source = &self.manager;
        let mut fields = FootPlantFields::constructed();
        fields.results_valid = source.hit;
        fields.time_to_land = source.contact_time;
        fields.total_time = source.scalar_596;
        fields.landing_next_frame = source.perform;
        fields.foot_planting = source.flag_627;
        fields.foot_part = source.selected_toe.map_or(-1, |toe| toe as i32);
        fields.surface_id = Some(source.surface);
        fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_recipe_never_shares_mutable_contact_or_query_history() {
        SourceFootplant::preload().unwrap();
        let mut first = SourceFootplant::load().unwrap();
        first.manager.selected_toe = Some(19);
        first.manager.requested = true;
        first.manager.perform = true;
        first.manager.contact = [1., 2., 3., 0.];
        first.pending = Some(Trajectory {
            position: [1., 2., 3., 0.],
            velocity: [2., 3., 4., 0.],
            acceleration: [0., -9.8, 0., 0.],
            duration: 1.,
        });
        let second = SourceFootplant::load().unwrap();
        assert!(std::sync::Arc::ptr_eq(
            &first.plant_wipeout,
            &second.plant_wipeout
        ));
        assert_eq!(second.manager.selected_toe, None);
        assert!(!second.manager.requested && !second.manager.perform);
        assert_eq!(second.manager.contact, [0.; 4]);
        assert!(second.pending.is_none());
        first.reset(true, false);
        assert!(!first.active);
        assert!(second.active);
        let mut first_boneless = SourceFootplant::new_boneless().unwrap();
        first_boneless.begin(true, [7., 8., 9., 0.]);
        let second_boneless = SourceFootplant::new_boneless().unwrap();
        assert_eq!(second_boneless.toe, 15);
        assert!(!second_boneless.right);
        assert_eq!(second_boneless.anchor, [0.; 4]);
    }
}

pub(crate) struct AirInput {
    pub footplant: skate3_core::air::known::KnownAirFootplantInput,
    pub requested: bool,
    pub time_in_state: f32,
}
impl SourceFootplant {
    /// Read the actual S2 KnownAir/selector and Actor packet. Source S3's
    /// trajectory sampling then shifts those completed observations to the
    /// current trajectory index; it advances no host timer or physical body.
    pub(crate) fn air_input(
        owner: &std::sync::Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
    ) -> Result<AirInput, String> {
        use crate::physical_player_assembly::remaining_states::{StateKind, StatePayload};
        use skate3_core::air::known::{KnownAirFootplantInput, KnownAirTrajectory};
        // These are new geometric input lanes. Unknown S2 W knowledge is not
        // written back or claimed as observed: source footplant geometry uses XYZ.
        let lanes = |v: crate::skeleton_root_transforms::SkeletonRootVector| {
            [v.xyz[0], v.xyz[1], v.xyz[2], v.w.unwrap_or(0.)]
        };
        let (time, remaining, collision, normal, index) = {
            let states = owner
                .remaining_states()
                .lock()
                .map_err(|_| "native footplant KnownAir lock")?;
            let Some(StatePayload::KnownAir(state)) = states
                .states
                .get(&StateKind::KnownAir)
                .filter(|state| state.published)
                .and_then(|state| state.value.as_ref())
            else {
                return Err("native footplant requires the constructed KnownAir owner".into());
            };
            if !state
                .base
                .physical
                .owner
                .upgrade()
                .is_some_and(|actual| std::sync::Arc::ptr_eq(&actual, owner))
            {
                return Err("foreign native footplant KnownAir owner".into());
            }
            let time = state
                .time_in_state
                .ok_or("native footplant KnownAir time")?;
            (
                time,
                state
                    .coll_time
                    .ok_or("native footplant KnownAir collision time")?
                    - time,
                lanes(
                    state
                        .coll_pos
                        .ok_or("native footplant KnownAir collision position")?,
                ),
                lanes(
                    state
                        .landing_normal
                        .ok_or("native footplant KnownAir landing normal")?,
                ),
                state
                    .traj_index
                    .ok_or("native footplant KnownAir trajectory index")?,
            )
        };
        let trajectory = {
            let trajectories = owner
                .trajectories()
                .lock()
                .map_err(|_| "native footplant selector lock")?;
            let selector = trajectories
                .onboard
                .as_ref()
                .ok_or("native footplant onboard selector")?;
            if selector.best_trajectory < 0 || selector.winning_result.is_none() {
                return Err("native footplant requires a completed selected trajectory".into());
            }
            selector.winning_com
        };
        let actual = Trajectory {
            position: lanes(trajectory.position),
            velocity: lanes(trajectory.velocity),
            acceleration: lanes(trajectory.gravity),
            duration: trajectory.time[0],
        };
        let tick_time = index as f32 * f32::from_bits(0x3c88_8889);
        let footplant = KnownAirFootplantInput {
            trajectory: KnownAirTrajectory {
                position: actual.position_at(tick_time),
                velocity: actual.velocity_at(tick_time),
                acceleration: actual.acceleration,
                scalar_48: actual.duration,
                word_52: trajectory.time[1].to_bits(),
                word_56: trajectory.time[2].to_bits(),
                word_60: trajectory.time[3].to_bits(),
            },
            remaining_collision_time: remaining,
            collision_position: collision,
            landing_normal: normal,
        };
        let packet = owner
            .animation_physics_input
            .lock()
            .map_err(|_| "native footplant Actor input lock")?
            .as_ref()
            .cloned()
            .ok_or("native footplant Actor input was not published")?;
        let packet = packet
            .lock()
            .map_err(|_| "native footplant animation packet lock")?;
        let attributes = packet
            .attributes
            .get(..packet.attribute_count)
            .ok_or("native footplant attribute capacity")?;
        let name =
            crate::play_animation::FastString30::encode("FootPlanting").expect("native literal");
        let requested = attributes
            .iter()
            .flatten()
            .any(|attribute| attribute.name == name)
            || owner
                .skate3_footplant_requested
                .load(std::sync::atomic::Ordering::Relaxed);
        Ok(AirInput {
            footplant,
            requested,
            time_in_state: time,
        })
    }
}
