//! Original `ROPA2FILE_0.5` cloth effects (Effect::ParseLoadedFile 82860100).
//! Disk scalars are little endian, independently of the RX2 container.
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RopaError(pub &'static str);
impl fmt::Display for RopaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for RopaError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RopaParticle {
    pub position: [f32; 3],
    pub inverse_mass: f32,
    pub movable: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RopaDriverRange {
    pub min: f32,
    pub max: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RopaDistance {
    pub particle: u16,
    pub reference: u16,
    pub min: f32,
    pub max: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RopaEffect {
    pub particles: Vec<RopaParticle>,
    pub drivers: Vec<RopaDriverRange>,
    /// Native conflict-free octet order, including empty SIMD lanes.
    pub distances: Vec<[Option<RopaDistance>; 8]>,
}

impl RopaEffect {
    pub fn parse(bytes: &[u8]) -> Result<Self, RopaError> {
        let mut input = Input { bytes, offset: 0 };
        let magic = input.take(16)?;
        // ParseLoadedFile compares a C string in the 16-byte header; bytes
        // following its terminator are padding, not part of the signature.
        if !magic.starts_with(b"ROPA2FILE_0.5\0") {
            return Err(RopaError("unsupported ROPA magic"));
        }
        let header = input.take(40)?;
        let counts: [usize; 10] = std::array::from_fn(|i| uint(header, i * 4) as usize);
        // Native signed particle indices reserve 0x8000 and one dummy particle.
        if counts[0] >= 0x8000 || counts[9] > counts[0] || counts[4].div_ceil(8) >= 0x10000 {
            return Err(RopaError("ROPA native index capacity exceeded"));
        }
        // These authored records are consumed but intentionally unused by native loaders.
        input.take(52)?;
        input.records(counts[1], 116)?;
        input.records(counts[2], 88)?;
        let particle_bytes = input.records(counts[0], 88)?;
        let mut particles = Vec::with_capacity(counts[0]);
        for row in particle_bytes.chunks_exact(88) {
            let mass = real(row, 84);
            let position = std::array::from_fn(|i| real(row, i * 4));
            if !mass.is_finite() || position.iter().any(|v| !v.is_finite()) {
                return Err(RopaError("non-finite ROPA particle"));
            }
            particles.push(RopaParticle {
                position,
                inverse_mass: if mass >= 1.0e-5 { mass.recip() } else { 1.0 },
                movable: uint(row, 80) != 1,
            });
        }
        input.records(counts[9], 240)?;
        let mut drivers = vec![RopaDriverRange::default(); counts[9]];
        let mut packer = DistancePacker::new(counts[4].div_ceil(8));
        for row in input.records(counts[4], 48)?.chunks_exact(48) {
            let particle = uint(row, 4) as usize;
            let reference = uint(row, 12) as usize;
            let min = real(row, 16);
            let max = real(row, 20);
            if particle >= particles.len() || !min.is_finite() || !max.is_finite() {
                return Err(RopaError("invalid ROPA distance constraint"));
            }
            if uint(row, 8) != 0 {
                if reference != particle || reference >= drivers.len() {
                    return Err(RopaError("ROPA driver particle mismatch"));
                }
                drivers[reference] = RopaDriverRange { min, max };
                if min == 0.0 && max == 0.0 {
                    particles[particle].movable = false;
                }
            } else {
                if reference >= particles.len() {
                    return Err(RopaError("invalid ROPA referenced particle"));
                }
                packer.insert(RopaDistance {
                    particle: particle as u16,
                    reference: reference as u16,
                    min,
                    max,
                })?;
            }
        }
        input.records(counts[6], 24)?;
        input.records(counts[5], 24)?;
        input.records(counts[7], 76)?;
        input.records(counts[8], 120)?;
        Ok(Self {
            particles,
            drivers,
            distances: packer.finish(),
        })
    }
}

/// LoadDistanceConstraints 8285ef90..8285f5b4. Prior full octets can exchange
/// their rotating lane with the pending constraint; a simple greedy sort is
/// not equivalent and changes the solver's convergence order.
struct DistancePacker {
    octets: Vec<[Option<RopaDistance>; 8]>,
    first: usize,
    rotate: usize,
    last: usize,
}
impl DistancePacker {
    fn new(count: usize) -> Self {
        Self {
            octets: vec![[None; 8]; count],
            first: 0,
            rotate: 0,
            last: 0,
        }
    }
    /// One `LoadDistanceConstraints` placement (8285efb8..8285f554). The conflict test reads
    /// `current` (r29) while `first` (r26) may already have jumped to the last octet. An exchange
    /// with an empty lane carries its marker (indices -1) on as the pending constraint, which
    /// then conflicts exactly with octets that still have an empty lane.
    fn insert(&mut self, value: RopaDistance) -> Result<(), RopaError> {
        let count = self.octets.len();
        while self.first + 1 < count && self.octets[self.first].iter().all(Option::is_some) {
            self.first += 1;
        }
        let mut pending = Some(value);
        let mut first = self.first;
        let mut current = first;
        let mut attempts = first as isize;
        let selected = loop {
            let done = if !conflicts(&self.octets[current], pending)
                && self.octets[first].iter().any(Option::is_none)
            {
                Some(current)
            } else if let Some(previous) = (0..first).rev().find(|&i| !conflicts(&self.octets[i], pending)) {
                pending = std::mem::replace(&mut self.octets[previous][self.rotate], pending);
                self.rotate = (self.rotate + 1) & 7;
                current = first;
                None
            } else {
                Some(
                    ((first + 1)..count)
                        .find(|&i| !conflicts(&self.octets[i], pending) && self.octets[i].iter().any(Option::is_none))
                        .ok_or(RopaError("ROPA native constraint octet capacity exhausted"))?,
                )
            };
            attempts -= 1;
            if attempts < 0 {
                first = count - 1;
            }
            if let Some(selected) = done {
                break selected;
            }
        };
        let lane = self.octets[selected]
            .iter_mut()
            .find(|v| v.is_none())
            .ok_or(RopaError("full ROPA octet"))?;
        *lane = pending;
        self.last = self.last.max(selected);
        Ok(())
    }
    fn finish(mut self) -> Vec<[Option<RopaDistance>; 8]> {
        if !self.octets.is_empty() {
            self.octets.truncate(self.last + 1);
        }
        self.octets
    }
}
/// An octet lane shares a particle with the pending constraint (both indices compared, sign
/// extended); the empty marker's -1 indices match only empty lanes.
fn conflicts(octet: &[Option<RopaDistance>; 8], value: Option<RopaDistance>) -> bool {
    let Some(value) = value else { return octet.iter().any(Option::is_none) };
    octet.iter().flatten().any(|v| {
        v.particle == value.particle
            || v.reference == value.particle
            || v.particle == value.reference
            || v.reference == value.reference
    })
}
struct Input<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Input<'a> {
    fn take(&mut self, size: usize) -> Result<&'a [u8], RopaError> {
        let end = self
            .offset
            .checked_add(size)
            .ok_or(RopaError("ROPA byte count overflow"))?;
        let result = self
            .bytes
            .get(self.offset..end)
            .ok_or(RopaError("truncated ROPA effect"))?;
        self.offset = end;
        Ok(result)
    }
    fn records(&mut self, count: usize, stride: usize) -> Result<&'a [u8], RopaError> {
        self.take(
            count
                .checked_mul(stride)
                .ok_or(RopaError("ROPA record count overflow"))?,
        )
    }
}
fn uint(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("bounded ROPA record"),
    )
}
fn real(bytes: &[u8], offset: usize) -> f32 {
    f32::from_bits(uint(bytes, offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_signature_accepts_padding_after_terminator() {
        // Header plus empty counts and global parameters. Native strcmp
        // stops at byte 13; neither of the remaining two bytes is significant.
        let mut bytes = vec![0; 16 + 40 + 52];
        let signature = b"ROPA2FILE_0.5\0";
        bytes[..signature.len()].copy_from_slice(signature);
        bytes[14..16].copy_from_slice(&[0xa5, 0x5a]);
        let effect = RopaEffect::parse(&bytes).unwrap();
        assert!(effect.particles.is_empty());
        assert!(effect.drivers.is_empty());
        assert!(effect.distances.is_empty());

        bytes[13] = b'x';
        assert_eq!(
            RopaEffect::parse(&bytes),
            Err(RopaError("unsupported ROPA magic"))
        );
        bytes[13] = 0;
        bytes[12] = b'4';
        assert_eq!(
            RopaEffect::parse(&bytes),
            Err(RopaError("unsupported ROPA magic"))
        );
    }
}
