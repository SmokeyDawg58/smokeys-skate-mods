//! SHLightingMan8284e6f0/ea88: resident hull grids and the six-value probe cache.
//! SkaterPresEntity827e8568 owns the separate smoothed material coefficients.
use skate2_core::presentation::PresentationScene;
use skate2_formats::irradiance::{Coefficients, LightProbe};
use std::{collections::{BTreeMap, BTreeSet}, sync::Arc};

struct Grid {
    probes: Arc<[LightProbe]>,
    cells: Vec<Option<usize>>,
    minimum: [f32; 3],
    maximum: [f32; 3],
    scale: [f32; 2],
    offset: [f32; 2],
}
impl Grid {
    fn new(probes: Arc<[LightProbe]>) -> Self {
        let mut minimum = std::array::from_fn(|i| probes[0].position[i]);
        let mut maximum = minimum;
        for p in probes.iter().take(1681) {
            for i in 0..3 {
                minimum[i] = minimum[i].min(p.position[i]);
                maximum[i] = maximum[i].max(p.position[i]);
            }
        }
        // SetDimensions8282cef0: exact scalar enlargement before reciprocal.
        let scale = [0, 2].map(|i| 1. / ((maximum[i] - minimum[i]) * f32::from_bits(0x3f83_1f38)));
        let offset = [-minimum[0] * scale[0], -minimum[2] * scale[1]];
        let mut s = Self { probes, cells: vec![None; 1681], minimum, maximum, scale, offset };
        for (i, p) in s.probes.iter().take(1681).enumerate() {
            let [x, z] = s.cell([p.position[0], p.position[2]]);
            if (0..41).contains(&x) && (0..41).contains(&z) {
                s.cells[(z * 41 + x) as usize] = Some(i);
            }
        }
        s.minimum = s.minimum.map(|v|v-15.);
        s.maximum = s.maximum.map(|v|v+15.);
        s
    }
    fn cell(&self, p: [f32; 2]) -> [i32; 2] {
        std::array::from_fn(|i| {
            let v = (p[i] * self.scale[i] + self.offset[i]) * 41.;
            // PPC fctiwz returns integer-indefinite for NaN/out-of-range.
            if !v.is_finite() || v >= 2147483648. || v < -2147483648. { i32::MIN } else { v as i32 }
        })
    }
    fn closest(&self, p: [f32; 3], distance: &mut f32, selected: &mut Option<LightProbe>) {
        if (0..3).any(|i| p[i] < self.minimum[i] || p[i] > self.maximum[i]) { return; }
        let min = self.cell([p[0]-15., p[2]-15.]).map(|v|v.max(0));
        let max = self.cell([p[0]+15., p[2]+15.]).map(|v|v.min(40));
        let mut count = 0;
        for x in min[0]..=max[0] {
            for z in min[1]..=max[1] {
                // Native Intersect's limit counts empty descriptors too.
                if count == 512 { return; }
                count += 1;
                if let Some(i) = self.cells[(z * 41 + x) as usize] {
                    let probe = self.probes[i];
                    let d = distance_squared(p, probe.position);
                    if d < *distance { *distance = d; *selected = Some(probe); }
                }
            }
        }
    }
}
fn distance_squared(a: [f32; 3], b: [f32; 4]) -> f32 {
    let d: [f32;3] = std::array::from_fn(|i| a[i]-b[i]);
    d[0]*d[0]+d[1]*d[1]+d[2]*d[2]
}

#[derive(Default)]
pub(crate) struct WorldProbes {
    registered: BTreeSet<u64>,
    grids: BTreeMap<u64, Grid>,
    cache: Vec<LightProbe>,
}
impl WorldProbes {
    pub(crate) fn sync(&mut self, scene: &PresentationScene) {
        let present: BTreeSet<_> = scene.resident_collections()
            .flat_map(|c|c.light_probes.iter().map(|(id,_)|*id)).collect();
        self.grids.retain(|id,_|present.contains(id));
        self.registered.retain(|id|present.contains(id));
        for (id, probes) in scene.resident_collections().flat_map(|c|&c.light_probes) {
            if self.registered.insert(*id) && !probes.is_empty() && self.grids.len() < 12 {
                self.grids.insert(*id, Grid::new(Arc::clone(probes)));
            }
        }
        // RemHullLightProbes8284fcd8 removes only the grid, not cached values.
    }
    pub(crate) fn sample(&mut self, position: [f32; 3], enabled: bool, default: Coefficients) -> Coefficients {
        if !enabled { return default; }
        let mut distance = 6.25;
        let mut cached = None;
        for (i,p) in self.cache.iter().enumerate() {
            let d=distance_squared(position,p.position);
            if d < distance {distance=d; cached=Some(i);}
        }
        if let Some(i)=cached {
            let p=self.cache.remove(i);self.cache.insert(0,p);return p.coefficients;
        }
        let mut distance=f32::MAX;
        let mut selected=None;
        for grid in self.grids.values() {grid.closest(position,&mut distance,&mut selected);}
        if let Some(p)=selected {
            if self.cache.len() >= 6 {self.cache.pop();}
            self.cache.insert(0,p);
            p.coefficients
        } else {default}
    }
}

pub(crate) struct SkaterLighting {
    pub(crate) coefficients: Coefficients,
    last_tick: Option<u32>,
    frontend_remainder: f64,
}
impl SkaterLighting {
    pub(crate) fn new(default: Coefficients) -> Self {
        Self {coefficients:default, last_tick:None, frontend_remainder:0.}
    }
    pub(crate) fn update_tick(&mut self, target: Coefficients, tick: u32) {
        if self.last_tick == Some(tick) {return;}
        self.last_tick=Some(tick);
        self.step(target);
    }
    pub(crate) fn update_frontend(&mut self, target: Coefficients, dt: f32) {
        self.frontend_remainder += f64::from(dt) * 60.;
        while self.frontend_remainder >= 1. {
            self.frontend_remainder -= 1.;
            self.step(target);
        }
    }
    fn step(&mut self, mut target: Coefficients) {
        // Native topLightHack, added to Y coefficient before the .98/.02 blend.
        for (v,bits) in target[2].iter_mut().zip([0x3d4c_cccd,0x3d4c_cccd,0x3d54_fdf4,0]) {
            *v += f32::from_bits(bits);
        }
        for (out,goal) in self.coefficients.iter_mut().flatten().zip(target.into_iter().flatten()) {
            *out = out.mul_add(f32::from_bits(0x3f7a_e148), goal * f32::from_bits(0x3ca3_d70a));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires original SKATE2_GAME_ROOT assets"]
    fn original_resident_hull_probes() {
        use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root=std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("game root"));
        let mut mounts=MountSet::new();
        for spec in SKATE2_ARCHIVES {
            let path=spec.path_under(&root);
            if path.is_file(){mounts.push(MountedBig::open(path,spec.use_external_header).unwrap());}
        }
        let scene=crate::load_runtime_presentation_scene(&mounts,"F_WF_Waterfront").unwrap();
        let env=crate::render_environment::EnvironmentLighting::load(&mounts).unwrap();
        let mut world=WorldProbes::default();world.sync(&scene);
        assert!(!world.grids.is_empty(),"world light probes were lost during mounted collection decoding");
        let mut matched=0;
        for c in scene.resident_collections() {
            for (_,probes) in &c.light_probes {
                for probe in probes.iter().take(1681) {
                    assert!(probe.position.iter().chain(probe.coefficients.iter().flatten()).all(|v|v.is_finite()));
                    let p=std::array::from_fn(|i|probe.position[i]);
                    let result=world.sample(p,true,env.default_irradiance);
                    if result != env.default_irradiance {matched+=1;}
                }
            }
        }
        assert!(matched>0,"all probe lookups fell back to constant ambient lighting");
        eprintln!("resident SH grids={} sampled nondefault={} world_enabled={} default={:?}",world.grids.len(),matched,env.use_world_sh_probes,env.default_irradiance);
        let room=crate::render_environment::EnvironmentLighting::load_location(&mounts,crate::edit_skater_room::CAS_RENDER_LOCATION).unwrap();
        eprintln!("CAS world_enabled={} default={:?}",room.use_world_sh_probes,room.default_irradiance);
    }
    fn probe(x:f32,z:f32,value:f32)->LightProbe {
        LightProbe {position:[x,0.,z,1.],coefficients:[[value;4];9]}
    }
    #[test]
    fn grid_selection_cache_and_default() {
        let mut world=WorldProbes::default();
        world.grids.insert(1,Grid::new(Arc::from([probe(0.,0.,1.),probe(40.,40.,2.),probe(20.,20.,3.)])));
        assert_eq!(world.sample([20.,0.,20.],true,[[9.;4];9])[0][0],3.);
        world.grids.clear(); // Native cached copies survive hull removal.
        assert_eq!(world.sample([22.,0.,20.],true,[[9.;4];9])[0][0],3.);
        assert_eq!(world.sample([22.5,0.,20.],true,[[9.;4];9])[0][0],9.);
        assert_eq!(world.sample([20.,0.,20.],false,[[9.;4];9])[0][0],9.);
    }
    #[test]
    fn render_repeats_do_not_accelerate_skater_blend() {
        let mut light=SkaterLighting::new([[0.;4];9]);
        light.update_tick([[1.;4];9],7);
        let once=light.coefficients;
        for _ in 0..100 {light.update_tick([[1.;4];9],7);}
        assert_eq!(light.coefficients,once);
        assert_eq!(once[0][0],0.02);
        assert!(once[2][0]>once[0][0]);
    }
}
