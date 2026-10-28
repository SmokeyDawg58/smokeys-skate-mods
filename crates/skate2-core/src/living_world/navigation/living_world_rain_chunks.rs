//! Native iteration order for RAIN's two dynamic asset-ID hash maps.
//! Constructor8271e210, insert82d62af8, rehash82d623e8, growth82bd2438.
use super::{asset::RainRecord, network::RainNetworkError};

// gPrimeNumberArray822852f0, all256 entries from the reference executable.
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
pub(crate) struct RainChunks {
    buckets: Vec<Vec<(u64, RainRecord)>>,
    count: usize,
    next_resize: usize,
}
fn hash(id: u64) -> u32 {
    (id as u32).wrapping_add((id >> 32) as u32)
}
impl RainChunks {
    pub(crate) fn new() -> Self {
        Self {
            buckets: vec![Vec::new()],
            count: 0,
            next_resize: 0,
        }
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub(crate) fn get(&self, id: u64) -> Option<&RainRecord> {
        self.buckets[hash(id) as usize % self.buckets.len()]
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, record)| record)
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &(u64, RainRecord)> {
        self.buckets.iter().flatten()
    }
    pub(crate) fn values(&self) -> impl Iterator<Item = &RainRecord> {
        self.iter().map(|(_, record)| record)
    }
    pub(crate) fn first_id(&self) -> Option<u64> {
        self.iter().next().map(|(id, _)| *id)
    }
    fn reserve_for(&mut self, additional: usize) -> Result<(), RainNetworkError> {
        let required = self.count + additional;
        if required <= self.next_resize {
            return Ok(());
        }
        let old = if self.buckets.len() == 1 {
            0
        } else {
            self.buckets.len()
        };
        if required <= old {
            self.next_resize = old;
            return Ok(());
        }
        // Source mfMaxLoadFactor=1, mfGrowthFactor=2. Current road pools stay
        // far below2^24, where native float conversion remains exact.
        let target = required.max(old * 2);
        let size = *PRIMES
            .iter()
            .find(|&&n| n as usize >= target)
            .ok_or(RainNetworkError::Allocation)? as usize;
        let mut buckets = Vec::new();
        buckets
            .try_reserve_exact(size)
            .map_err(|_| RainNetworkError::Allocation)?;
        buckets.resize_with(size, Vec::new);
        // Source walks old buckets ascending, head to tail, and prepends each
        // node to its new bucket. Rehash therefore changes collision order.
        for (id, value) in std::mem::take(&mut self.buckets).into_iter().flatten() {
            buckets[hash(id) as usize % size].insert(0, (id, value));
        }
        self.buckets = buckets;
        self.next_resize = size;
        Ok(())
    }
    pub(crate) fn insert(
        &mut self,
        id: u64,
        value: RainRecord,
        replace: bool,
    ) -> Result<(), RainNetworkError> {
        let bucket = hash(id) as usize % self.buckets.len();
        if let Some((_, old)) = self.buckets[bucket].iter_mut().find(|(key, _)| *key == id) {
            if replace {
                *old = value;
            }
            return Ok(());
        }
        self.reserve_for(1)?;
        let bucket = hash(id) as usize % self.buckets.len();
        self.buckets[bucket].insert(0, (id, value));
        self.count += 1;
        Ok(())
    }
    pub(crate) fn remove(&mut self, id: u64) -> Option<RainRecord> {
        let bucket_index = hash(id) as usize % self.buckets.len();
        let bucket = &mut self.buckets[bucket_index];
        let index = bucket.iter().position(|(key, _)| *key == id)?;
        self.count -= 1;
        Some(bucket.remove(index).1)
    }
    /// RemoveRAINData82727958: clear nodes while retaining capacity, then
    /// range insert82715dd8 reserves for the entire surviving map first.
    pub(crate) fn assign(&mut self, source: &Self) -> Result<(), RainNetworkError> {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        self.count = 0;
        self.reserve_for(source.count)?;
        for (id, record) in source.iter() {
            self.insert(*id, record.clone(), false)?;
        }
        Ok(())
    }
}
