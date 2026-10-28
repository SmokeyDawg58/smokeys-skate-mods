//! CensusMan.EvaluatePropCensus82720d60: range evaluation and culling only.
use super::SelectedRecipeFrame;
use crate::{
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::dmo_objects::{
        CENSUS_RANGES_CLASS, CensusCircle, DmoCensusInputs, evaluate_census_circle,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, try_length, try_normalize_safe},
};

impl SelectedRecipeFrame<'_> {
    pub(crate) fn evaluate_props(
        &self,
        world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences,
        inputs: &DmoCensusInputs,
    ) -> Result<(), String> {
        // CensusMan ctor827209b8..9c0 retains StringToKey("props") at+0x78.
        let ranges = self.vault.instance(
            CENSUS_RANGES_CLASS,
            skate2_formats::attrib::string_hash64("props"),
        );
        if ranges.is_null() { return Err("Prop census range collection absent".into()); }
        let data = ranges.try_cached_layout_field(0, 0x28)
            .map_err(|e| format!("Prop census ranges: {e:?}"))?
            .ok_or("Prop census range layout absent")?;
        let scalar = |offset: usize| f32::from_be_bytes(
            data.bytes()[offset..offset+4].try_into().expect("checked range layout"));
        let circle = |base| CensusCircle {
            no_spawn: scalar(base), spawn: scalar(base+4), cull: scalar(base+8),
            shift: scalar(base+12), speed: scalar(base+16),
        };
        let mut numeric = HostReckoningNumeric;
        let speed = try_length(inputs.velocity, &mut numeric)
            .map_err(|e| format!("Prop census speed: {e:?}"))? * f32::from_bits(0x40666666);
        let circle = evaluate_census_circle(speed, circle(0), circle(0x14));
        let direction = try_normalize_safe(inputs.velocity, [0.;4], &mut numeric)
            .map_err(|e| format!("Prop census direction: {e:?}"))?;
        let center: [f32;4] = std::array::from_fn(|i| direction[i].mul_add(circle.shift, inputs.center[i]));
        let owner = std::sync::Arc::clone(world.grind_services.living_world());
        let mut living = owner.lock().map_err(|_| "Prop census LivingWorld lock")?;
        let props = living.props.as_deref().ok_or("Prop census PropMan absent")?;
        let mut culls = Vec::new();
        // EvaluativeCullProps8271f410 snapshots native map order, includes Y,
        // and culls equality too. Caller82720fb0 sets outside flag in r6 to1.
        for id in props.entities.ids() {
            let core = props.entities.get(id).and_then(|p| p.core.as_ref())
                .ok_or("Prop census entity core absent")?;
            if !core.allow_culling { continue; }
            let delta: [f32;4] = std::array::from_fn(|i| center[i]-core.transform[3][i]);
            // fcmpu + bge8271f57c (BO4, BI24) tests CR6.LT==0.
            // Preserve unordered as well as boundary/equality behavior.
            let distance = numeric.dot3(delta, delta)
                .map_err(|e| format!("Prop census distance: {e:?}"))? - circle.cull*circle.cull;
            if !(distance < 0.) { culls.push(id); }
        }
        if culls.is_empty() { return Ok(()); }
        let actors = world.entity_roster.as_ref().ok_or("Prop census Actor roster absent")?
            .snapshot().map_err(|e| format!("Prop census Actors: {e:?}"))?;
        let triggers = self.triggers.lock().map_err(|_| "Prop census trigger lock")?;
        let avoid = self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
        drop(triggers);
        let painter = self.painter.lock().map_err(|_| "Prop census WorldPainter lock")?;
        let mut random = living.random.take().ok_or("Prop census Manager random absent")?;
        let result = (|| {
            for id in culls {
                // A previous parent's recursive cull may already have detached
                // this snapshot entry. Native null entries are skipped too.
                if living.props.as_deref().and_then(|p| p.entities.get(id)).is_none() { continue; }
                self.prop_recipe_frame().request_contained_prop_cull(
                    &mut living, world, id, cvars, &painter, &mut random, &actors, &avoid)?;
            }
            Ok(())
        })();
        living.random = Some(random);
        result
    }
}
