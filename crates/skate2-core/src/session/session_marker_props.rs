//! SessionMarker's locator map: constructor828adab8, insert828ad7c0,
//! rehash828abd00, add828aedf0, remove828add60 and clear828ad6a0.
use crate::living_world_grind_services::LivingWorldGrindServices;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MarkerProps {
    buckets: Vec<Vec<(u64, [f32; 16])>>,
    count: usize,
    pub(crate) cleared: Vec<u64>,
}
impl Default for MarkerProps {
    fn default() -> Self {
        Self {
            buckets: vec![Vec::new()],
            count: 0,
            cleared: Vec::new(),
        }
    }
}
fn hash(id: u64) -> usize {
    (id as u32).wrapping_add((id >> 32) as u32) as usize
}
impl MarkerProps {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &(u64, [f32; 16])> {
        self.buckets.iter().flatten()
    }
    pub(crate) fn contains(&self, id: u64) -> bool {
        self.buckets[hash(id) % self.buckets.len()]
            .iter()
            .any(|(key, _)| *key == id)
    }
    fn insert(&mut self, id: u64, transform: [f32; 16]) {
        let index = hash(id) % self.buckets.len();
        if let Some((_, value)) = self.buckets[index].iter_mut().find(|(key, _)| *key == id) {
            *value = transform;
            return;
        }
        if self.count + 1
            > if self.buckets.len() == 1 {
                0
            } else {
                self.buckets.len()
            }
        {
            let old = if self.buckets.len() == 1 {
                0
            } else {
                self.buckets.len()
            };
            let target = (self.count + 1).max(2 * old);
            let size = *PRIMES
                .iter()
                .find(|&&n| n as usize >= target)
                .expect("native prime table exhausted") as usize;
            let mut buckets = vec![Vec::new(); size];
            for (key, value) in std::mem::take(&mut self.buckets).into_iter().flatten() {
                buckets[hash(key) % size].insert(0, (key, value));
            }
            self.buckets = buckets;
        }
        let index = hash(id) % self.buckets.len();
        self.buckets[index].insert(0, (id, transform));
        self.count += 1;
    }
    pub(crate) fn remove(&mut self, id: u64) {
        let index = hash(id) % self.buckets.len();
        if let Some(at) = self.buckets[index].iter().position(|(key, _)| *key == id) {
            self.cleared.push(self.buckets[index].remove(at).0);
            self.count -= 1;
        }
    }
    pub(crate) fn clear(&mut self) {
        self.cleared.extend(
            self.buckets
                .iter_mut()
                .flat_map(|b| b.drain(..).map(|(id, _)| id)),
        );
        self.count = 0;
    }
    pub(crate) fn flush_cleared(
        &mut self,
        living: &mut LivingWorldGrindServices,
    ) -> Result<(), String> {
        let locators = living
            .dmo_locators
            .as_deref_mut()
            .ok_or("marker locator manager absent")?;
        for id in self.cleared.drain(..) {
            locators.set_in_session_marker(id, false);
        }
        Ok(())
    }
    pub(crate) fn add(
        &mut self,
        id: u64,
        living: &mut LivingWorldGrindServices,
    ) -> Result<(), String> {
        self.flush_cleared(living)?;
        let locators = living
            .dmo_locators
            .as_deref()
            .ok_or("marker locator manager absent")?;
        let Some(locator) = locators.locator(id) else {
            return Ok(());
        };
        let transform = locator.transform;
        let saved: Vec<_> = self.iter().copied().filter(|(key, _)| *key != id).collect();
        if saved.len() > 50 {
            return Err("native marker overlap list capacity50 exceeded".into());
        }
        let mut linked = Vec::new();
        update_list(living, id, &saved, &mut linked)?;
        for other in linked {
            if let Some(locator) = locators.locator(other) {
                self.insert(other, locator.transform);
            }
        }
        self.insert(id, transform);
        living
            .dmo_locators
            .as_deref_mut()
            .unwrap()
            .set_in_session_marker(id, true);
        Ok(())
    }
}

/// BuildRecursiveObjectLocatorUpdateList82d35850: append overlapping siblings
/// in saved-map order before descending, using each locator's current transform.
fn update_list(
    living: &LivingWorldGrindServices,
    id: u64,
    saved: &[(u64, [f32; 16])],
    list: &mut Vec<u64>,
) -> Result<(), String> {
    let locators = living
        .dmo_locators
        .as_deref()
        .ok_or("marker locator manager absent")?;
    let Some(locator) = locators.locator(id) else {
        return Ok(());
    };
    let Some(model) = living.dmo_models.model(locator.model_id) else {
        return Ok(());
    };
    let first = list.len();
    for &(other, transform) in saved {
        if list.contains(&other) {
            continue;
        }
        let Some(other_locator) = locators.locator(other) else {
            continue;
        };
        let Some(other_model) = living.dmo_models.model(other_locator.model_id) else {
            continue;
        };
        if crate::simulation_assembly_backend::constraint_session::dmo_living_world::reset_models_overlap(
            model,locator.transform,other_model,transform,0.05).map_err(|e|format!("marker linked props: {e:?}"))? {
            if list.len()==50 {return Err("native marker update list capacity50 exceeded".into());}
            list.push(other);
        }
    }
    let siblings = list[first..].to_vec();
    for other in siblings {
        update_list(living, other, saved, list)?;
    }
    Ok(())
}

// gPrimeNumberArray822852f0; copied below from the live reference.
const PRIMES: [u32; 256] = [
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
    103, 109, 113, 127, 137, 139, 149, 157, 167, 179, 193, 199, 211, 227, 241, 257, 277, 293, 313,
    337, 359, 383, 409, 439, 467, 503, 541, 577, 619, 661, 709, 761, 823, 887, 953, 1031, 1109,
    1193, 1289, 1381, 1493, 1613, 1741, 1879, 2029, 2179, 2357, 2549, 2753, 2971, 3209, 3469, 3739,
    4027, 4349, 4703, 5087, 5503, 5953, 6427, 6949, 7517, 8123, 8783, 9497, 10273, 11113, 12011,
    12983, 14033, 15173, 16411, 17749, 19183, 20753, 22447, 24281, 26267, 28411, 30727, 33223,
    35933, 38873, 42043, 45481, 49201, 53201, 57557, 62233, 67307, 72817, 78779, 85229, 92203,
    99733, 107897, 116731, 126271, 136607, 147793, 159871, 172933, 187091, 202409, 218971, 236897,
    256279, 277261, 299951, 324503, 351061, 379787, 410857, 444487, 480881, 520241, 562841, 608903,
    658753, 712697, 771049, 834181, 902483, 976369, 1056323, 1142821, 1236397, 1337629, 1447153,
    1565659, 1693859, 1832561, 1982627, 2144977, 2320627, 2510653, 2716249, 2938679, 3179303,
    3439651, 3721303, 4026031, 4355707, 4712381, 5098259, 5515729, 5967347, 6456007, 6984629,
    7556579, 8175383, 8844859, 9569143, 10352717, 11200489, 12117689, 13109983, 14183539, 15345007,
    16601593, 17961079, 19431899, 21023161, 22744717, 24607243, 26622317, 28802401, 31160981,
    33712729, 36473443, 39460231, 42691603, 46187573, 49969847, 54061849, 58488943, 63278561,
    68460391, 74066549, 80131819, 86693767, 93793069, 101473717, 109783337, 118773397, 128499677,
    139022417, 150406843, 162723577, 176048909, 190465427, 206062531, 222936881, 241193053,
    260944219, 282312799, 305431229, 330442829, 357502601, 386778277, 418451333, 452718089,
    489790921, 529899637, 573292817, 620239453, 671030513, 725980837, 785430967, 849749479,
    919334987, 994618837, 1076067617, 1164186217, 1259520799, 1362662261, 1474249943, 1594975441,
    1725587117, 1866894511, 2019773507, 2185171673, 2364114217, 2557710269, 2767159799, 2993761039,
    3238918481, 3504151727, 3791104843, 4101556399, 4294967291,
];
