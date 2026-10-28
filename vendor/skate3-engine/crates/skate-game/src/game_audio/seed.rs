//! A seedable audio random state for reproducible mod tests (doc 16 "L5"; `sdk.audio.seed(n)`,
//! capability `audio` = 4; `SKATE_AUDIO_SEED=<n>` for a whole run).
//!
//! The audio draws from several generators, each started at a fixed state as retail's is: the
//! evaluator's (every program's ops 7–9), the Splice player's picks, the rolling bed's grain picks,
//! the eEQChain buses' rolls, the Jitter walk, the world host's (`Lcg(0x5EED)`: traffic and peds)
//! and the speech host's (`Lcg(0x5EEC)`). They are deterministic from the start, but a mod test
//! cannot start them from a known point mid-session. A seed sets **all of them** from one number
//! (splitmix64 per generator), at the start of the next audio pass, so the same seed at the same
//! point gives the same draws.
//!
//! - Owned: the first mod to seed owns it (another mod's seed is an error); the same mod may seed
//!   again (a new starting point).
//! - `nil` (or the owner stopping) puts back the states the generators had at the first seed (the
//!   retail sequence continues from where it was then); the draws made while seeded are not undone.
//! - A runtime restart starts every generator at its retail state again: a seed in force is applied
//!   again after it.
//! - Unseeded nothing here runs ([`apply`] returns at once): the draws are retail's, unchanged.
use bevy::prelude::*;

use super::Native;

/// Every audio generator's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Generators {
    pub eval: [u32; 6],
    pub splice: u32,
    pub grains: [u32; 6],
    pub eq: [u32; 6],
    pub jitter: Option<[u32; 6]>,
    pub world: u32,
    pub speech: u32,
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Generators {
    /// The states a seed gives (each generator its own words from one splitmix64 stream).
    pub(crate) fn from_seed(seed: u64) -> Self {
        let mut s = seed;
        let mut word = || splitmix(&mut s) as u32;
        let mut six = || std::array::from_fn(|_| word());
        let (eval, grains, eq, jitter) = (six(), six(), six(), six());
        let mut s2 = seed ^ 0x5EED_5EED_5EED_5EED;
        let mut more = || splitmix(&mut s2) as u32;
        Self {
            eval,
            splice: more(),
            grains,
            eq,
            jitter: Some(jitter),
            world: more(),
            speech: more(),
        }
    }

    /// The running audio's generators (None without the runtime).
    pub(crate) fn read(world: &World) -> Option<Self> {
        let native = world.get_resource::<Native>()?;
        let jitter = native.player.as_ref().map(|p| p.jitter_rng().w);
        let rt = native.shared.lock().ok()?;
        Some(Self {
            eval: rt.eval.rng.w,
            splice: rt.splice.rng.0,
            grains: rt.grains.rng.w,
            eq: rt.mixer.buses.eq.rng.w,
            jitter,
            world: world
                .get_resource::<super::world_sources::WorldHost>()
                .map_or(0, |h| h.rng_state()),
            speech: world
                .get_resource::<super::world_speech::WorldSpeech>()
                .map_or(0, |s| s.rng_state()),
        })
    }

    pub(crate) fn write(&self, world: &mut World) {
        if let Some(mut native) = world.get_resource_mut::<Native>() {
            if let (Some(p), Some(j)) = (native.player.as_mut(), self.jitter) {
                p.jitter_rng_mut().w = j;
            }
            if let Ok(mut rt) = native.shared.lock() {
                rt.eval.rng.w = self.eval;
                rt.splice.rng.0 = self.splice;
                rt.grains.rng.w = self.grains;
                rt.mixer.buses.eq.rng.w = self.eq;
            }
        }
        if let Some(mut h) = world.get_resource_mut::<super::world_sources::WorldHost>() {
            h.set_rng_state(self.world);
        }
        if let Some(mut s) = world.get_resource_mut::<super::world_speech::WorldSpeech>() {
            s.set_rng_state(self.speech);
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct AudioSeed {
    owner: Option<String>,
    seed: Option<u64>,
    /// A change waits for the next pass.
    pending: bool,
    /// The states at the first seed (restored by `nil`).
    saved: Option<Generators>,
    /// `AudioContent::runtime_generation` the seed was applied in.
    applied: Option<u64>,
    /// Seeds applied (the info readout; tests).
    pub(crate) count: u64,
}

impl AudioSeed {
    /// Seed (`Some`) or release (`None`) for `owner`; the first owner wins.
    pub(crate) fn set(&mut self, owner: &str, seed: Option<u64>) -> Result<(), String> {
        if let Some(o) = self.owner.as_deref().filter(|o| *o != owner) {
            return Err(format!("audio seed is owned by another mod ({o})"));
        }
        match seed {
            Some(s) => {
                self.owner = Some(owner.to_owned());
                self.seed = Some(s);
            }
            None => {
                if self.owner.is_none() {
                    return Ok(());
                }
                self.owner = None;
                self.seed = None;
            }
        }
        self.pending = true;
        Ok(())
    }

    /// The owner stopped, failed or reloaded.
    pub(crate) fn clear_owner(&mut self, owner: &str) {
        if self.owner.as_deref() == Some(owner) {
            let _ = self.set(owner, None);
        }
    }

    /// (owner, seed) in force.
    pub(crate) fn current(&self) -> Option<(&str, u64)> {
        Some((self.owner.as_deref()?, self.seed?))
    }
}

/// The start of the audio pass (`content::frame`, after a restart): apply a new seed, a release,
/// or a seed in force again in a restarted runtime. Returns at once while unseeded.
pub(super) fn apply(world: &mut World) {
    let runtime = world
        .get_resource::<super::AudioContent>()
        .map_or(0, |c| c.runtime_generation);
    let Some(s) = world.get_resource::<AudioSeed>() else {
        return;
    };
    let again = s.seed.is_some() && s.applied != Some(runtime);
    if !s.pending && !again {
        return;
    }
    if world.get_resource::<Native>().is_none() {
        return;
    }
    let (seed, restarted) = (s.seed, s.applied != Some(runtime));
    match seed {
        Some(seed) => {
            let now = Generators::read(world);
            let mut s = world.resource_mut::<AudioSeed>();
            if s.saved.is_none() || restarted {
                s.saved = now;
            }
            s.applied = Some(runtime);
            s.pending = false;
            s.count += 1;
            Generators::from_seed(seed).write(world);
            info!("Game audio: audio random state seeded ({seed})");
        }
        None => {
            let mut s = world.resource_mut::<AudioSeed>();
            let saved = s.saved.take().filter(|_| s.applied == Some(runtime));
            s.applied = None;
            s.pending = false;
            if let Some(g) = saved {
                g.write(world);
                info!("Game audio: audio random state restored (unseeded)");
            }
        }
    }
}

/// `SKATE_AUDIO_SEED=<n>` (a whole run's tests): seeds at the first start, owned by "env".
pub(super) fn from_env(world: &mut World) {
    let Some(seed) = std::env::var("SKATE_AUDIO_SEED")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
    else {
        return;
    };
    if let Some(mut s) = world.get_resource_mut::<AudioSeed>() {
        let _ = s.set("env", Some(seed));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_gives_its_own_states_and_ownership_holds() {
        let a = Generators::from_seed(7);
        assert_eq!(a, Generators::from_seed(7));
        assert_ne!(a, Generators::from_seed(8));
        assert_ne!(a.eval, a.grains, "each generator its own words");
        let mut s = AudioSeed::default();
        s.set("dev.a", Some(1)).unwrap();
        assert!(s.set("dev.b", Some(2)).is_err(), "first owner wins");
        s.set("dev.a", Some(3)).unwrap();
        assert_eq!(s.current(), Some(("dev.a", 3)));
        s.clear_owner("dev.b");
        assert_eq!(s.current(), Some(("dev.a", 3)));
        s.clear_owner("dev.a");
        assert_eq!(s.current(), None);
        s.set("dev.b", Some(2)).unwrap();
        assert_eq!(s.current(), Some(("dev.b", 2)));
    }

    fn world(library: &super::super::Library) -> World {
        let mut w = World::new();
        w.insert_resource(
            Native::start(library).unwrap_or_else(|e| panic!("missing private data: {e}")),
        );
        w.init_resource::<super::super::AudioContent>();
        w.init_resource::<super::super::world_sources::WorldHost>();
        w.init_resource::<super::super::world_speech::WorldSpeech>();
        w.init_resource::<AudioSeed>();
        w
    }

    /// Posts of the `k`-th DownTown emitter bank (released and posted again), rendered.
    fn scenario(w: &mut World, library: &super::super::Library, k: usize) -> Vec<f32> {
        let r = library
            .emitters("sfx_downtown")
            .iter()
            .filter(|r| {
                r.kind == 1
                    && r.bank
                        .as_ref()
                        .is_some_and(|b| library.aems().banks.contains_key(b))
            })
            .nth(k)
            .expect("an emitter");
        let mut native = w.resource_mut::<Native>();
        native
            .ensure_bank(library, r.bank.as_ref().unwrap())
            .unwrap();
        let payload = native.emitter_payload(None, 0.8, 9000, r.patch);
        let mut node = native.post_emitter(&payload).unwrap();
        let mut out = vec![0.0f32; 2 * skate_audio::BLOCK];
        let mut all = Vec::new();
        for b in 0..900 {
            if b % 300 == 299 {
                native.release(node);
                node = native.post_emitter(&payload).unwrap();
            }
            native.shared.lock().unwrap().fill_stereo(&mut out);
            all.extend_from_slice(&out);
        }
        native.release(node);
        all
    }

    /// L5 (data-gated): unseeded, `apply` changes nothing (every generator keeps retail's state);
    /// the same seed at the same point gives the same output from equal worlds whose random
    /// states were scrambled just before it, another seed other draws (on an emitter bank whose program draws);
    /// `nil` puts back the states from the first seed; a restarted runtime is seeded again.
    #[test]
    #[ignore = "needs the private install data"]
    fn a_seed_makes_runs_reproducible_and_unseeded_is_retail() {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"));
        let Ok(library) = super::super::Library::load(root) else {
            panic!("missing private data: no audio install")
        };
        let same = |p: &[f32], q: &[f32]| {
            p.len() == q.len() && p.iter().zip(q).all(|(i, j)| i.to_bits() == j.to_bits())
        };
        let eligible = library
            .emitters("sfx_downtown")
            .iter()
            .filter(|r| {
                r.kind == 1
                    && r.bank
                        .as_ref()
                        .is_some_and(|b| library.aems().banks.contains_key(b))
            })
            .count();
        let mut drew = None;
        for k in 0..eligible.min(40) {
            let (mut a, mut b, mut c) = (world(&library), world(&library), world(&library));
            let retail = Generators::read(&a).unwrap();
            apply(&mut a);
            assert_eq!(
                Generators::read(&a).unwrap(),
                retail,
                "unseeded: nothing changes"
            );
            for w in [&mut a, &mut b, &mut c] {
                scenario(w, &library, k);
            }
            // a's generators scrambled just before the seed point: the seed sets every one.
            Generators::from_seed(99).write(&mut a);
            let before = Generators::read(&b).unwrap();
            for (w, seed) in [(&mut a, 5), (&mut b, 5), (&mut c, 6)] {
                w.resource_mut::<AudioSeed>()
                    .set("dev.t", Some(seed))
                    .unwrap();
                apply(w);
            }
            assert_eq!(Generators::read(&a), Generators::read(&b));
            let (x, y, z) = (
                scenario(&mut a, &library, k),
                scenario(&mut b, &library, k),
                scenario(&mut c, &library, k),
            );
            assert!(same(&x, &y), "emitter {k}: the same seed, the same output");
            if !same(&x, &z) {
                drew = Some((k, a, b, before));
                break;
            }
        }
        let Some((k, mut a, mut b, before)) = drew else {
            panic!("no DownTown emitter program draws")
        };
        eprintln!("emitter {k} draws: another seed, other output");
        // Release: b's generators get back their states from the first seed.
        b.resource_mut::<AudioSeed>().set("dev.t", None).unwrap();
        apply(&mut b);
        assert_eq!(Generators::read(&b).unwrap(), before);
        // A restart (new runtime generation) is seeded again.
        a.insert_resource(Native::start(&library).unwrap());
        a.resource_mut::<super::super::AudioContent>()
            .runtime_generation += 1;
        apply(&mut a);
        assert_eq!(
            Generators::read(&a).unwrap().eval,
            Generators::from_seed(5).eval
        );
        assert_eq!(a.resource::<AudioSeed>().count, 2);
    }
}
