//! Ordered swing/twist correction from `TwistThatBone827d9540`.
use super::corrections::CorrectionChannels;
use super::move_root::matrix_inverse::PresentationMatrixNumeric;
use crate::{
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::{ReckoningSinCosNumeric, sine::try_sine, try_cosine},
    skeleton_query_rotation::try_atan,
};
#[cfg(test)]
use skate2_formats::andale::AndaleSqt;

#[derive(Debug)]
pub(crate) enum TwistError<N, R> {
    BindBone(usize),
    ReplayBone(usize),
    Numeric(N),
    Owner(super::ActorFaceError),
    Rounding(R),
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display for TwistError<N, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BindBone(index) => write!(f, "twist missing bind bone {index}"),
            Self::ReplayBone(index) => write!(f, "twist missing replay bone {index}"),
            Self::Owner(error) => write!(f, "twist pose access: {error:?}"),
            Self::Numeric(error) => write!(f, "twist numeric service: {error:?}"),
            Self::Rounding(error) => write!(f, "twist rounding service: {error:?}"),
        }
    }
}
fn flip(value: f32) -> f32 {
    f32::from_bits(value.to_bits() ^ 0x8000_0000)
}
fn refine_inverse(value: f32, seed: f32) -> f32 {
    seed.mul_add(-seed.mul_add(value, -1.0), seed)
}
fn refine_root(value: f32, seed: f32) -> f32 {
    let square = seed * seed;
    let half = seed * 0.5;
    half.mul_add(-value.mul_add(square, -1.0), seed)
}
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 3] {
    std::array::from_fn(|lane| {
        let next = (lane + 1) % 3;
        let last = (lane + 2) % 3;
        -a[last].mul_add(b[next], -(a[next] * b[last]))
    })
}
fn compose<N: ReckoningNumeric>(
    a: [f32; 4],
    b: [f32; 4],
    numeric: &mut N,
) -> Result<[f32; 4], N::Error> {
    let dot = numeric.dot3(a, b)?;
    let scalar = a[3] * b[3] - dot;
    let cross = cross(a, b);
    let xyz: [f32; 3] = std::array::from_fn(|lane| {
        let first = b[lane].mul_add(a[3], cross[lane]);
        a[lane].mul_add(b[3], first)
    });
    Ok([xyz[0], xyz[1], xyz[2], scalar])
}
fn rotate_x(q: [f32; 4]) -> [f32; 3] {
    let axis = [1.0, 0.0, 0.0, 0.0];
    let first = cross(q, axis);
    let middle: [f32; 4] = std::array::from_fn(|lane| {
        if lane < 3 {
            q[3].mul_add(axis[lane], first[lane])
        } else {
            0.0
        }
    });
    let second = cross(q, middle);
    std::array::from_fn(|lane| 2.0f32.mul_add(second[lane], axis[lane]))
}
fn quadrant(numerator: f32, denominator: f32, atan: f32) -> f32 {
    let sign = numerator.to_bits() & 0x8000_0000;
    let pi = f32::from_bits(0x4049_0fdb | sign);
    let half_pi = f32::from_bits(0x3fc9_0fdb | sign);
    let adjusted = if 0.0 > denominator { pi + atan } else { atan };
    if denominator == 0.0 {
        half_pi
    } else {
        adjusted
    }
}

/// How the swing axis is scaled by the inverse swing length.
#[derive(Clone, Copy)]
enum Swing {
    /// `SkaterPresentation::TwistThatBone`: a twice refined reciprocal
    /// estimate times the half angle's sine.
    Refined,
    /// `SceneCharacterAnim::TwistThatBone` (`0x82866258`): each axis
    /// component times an `fdivs` reciprocal, then times the sine
    /// (`0x82866500..48`); the cosine is taken before the sine.
    Divided,
}

/// Caller borrows the actual bind and synchronized replay allocation. Reads of
/// the bind destination occur after the source's intermediate destination store.
pub(crate) fn try_twist_channels<N: PresentationMatrixNumeric, R: ReckoningSinCosNumeric>(
    channels: &mut impl CorrectionChannels,
    source: usize,
    destination: usize,
    ratio: f32,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<(), TwistError<N::Error, R::Error>> {
    twist_channels(channels, source, destination, ratio, Swing::Refined, numeric, rounding)
}

/// `SceneCharacterAnim::TwistThatBone` (`0x82866258`): the skater twist with
/// the scene swing, and one `XMVectorATan` behind the twist's sign branch
/// where the skater takes both.
pub(crate) fn try_scene_twist_channels<N: PresentationMatrixNumeric, R: ReckoningSinCosNumeric>(
    channels: &mut impl CorrectionChannels,
    source: usize,
    destination: usize,
    ratio: f32,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<(), TwistError<N::Error, R::Error>> {
    twist_channels(channels, source, destination, ratio, Swing::Divided, numeric, rounding)
}

fn twist_channels<N: PresentationMatrixNumeric, R: ReckoningSinCosNumeric>(
    channels: &mut impl CorrectionChannels,
    source: usize,
    destination: usize,
    ratio: f32,
    swing: Swing,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<(), TwistError<N::Error, R::Error>> {
    use TwistError as E;
    let original = channels.bind(source).ok_or(E::BindBone(source))?.rotation; //955c
    let norm = numeric.dot4(original, original).map_err(E::Numeric)?; //9564
    let current = channels
        .replay(source)
        .ok_or(E::ReplayBone(source))?
        .rotation; //9570
    let seed = numeric.reciprocal_estimate(norm).map_err(E::Numeric)?;
    let inverse_norm = refine_inverse(norm, refine_inverse(norm, seed));
    let inverse = [
        flip(original[0]) * inverse_norm,
        flip(original[1]) * inverse_norm,
        flip(original[2]) * inverse_norm,
        original[3] * inverse_norm,
    ];
    let delta = compose(inverse, current, numeric).map_err(E::Numeric)?; //95f4..9620
    let direction = rotate_x(delta); //9624..9658
    let y_squared = direction[1] * direction[1];
    let x_seed = numeric
        .reciprocal_estimate(direction[0])
        .map_err(E::Numeric)?; //966c
    let transverse_squared = direction[2].mul_add(direction[2], y_squared); //9670
    let root_seed = numeric
        .reciprocal_sqrt_estimate(transverse_squared)
        .map_err(E::Numeric)?;
    let inverse_root = refine_root(
        transverse_squared,
        refine_root(transverse_squared, root_seed),
    );
    let length = if transverse_squared == 0.0 {
        0.0
    } else {
        transverse_squared * inverse_root
    }; //96f8
    let x_inverse = refine_inverse(direction[0], x_seed); //one refinement
    let atan = try_atan(length.mul_add(x_inverse, 0.0), numeric).map_err(E::Numeric)?;
    numeric.observe_twist_length(source, destination, length);
    let inverse_swing = match swing {
        Swing::Refined => {
            let length_seed = numeric.reciprocal_estimate(length).map_err(E::Numeric)?; //970c, even at zero
            let length_once = refine_inverse(length, length_seed);
            let half_angle = quadrant(length, direction[0], atan) * 0.5;
            let sine = try_sine(half_angle, rounding).map_err(E::Rounding)?; //9760
            let length_inverse = refine_inverse(length, length_once); //9770..74
            let factor = length_inverse * sine;
            let cosine = try_cosine(half_angle, rounding).map_err(E::Rounding)?; //9780
            [
                0.0,
                direction[2] * factor,
                flip(direction[1]) * factor,
                cosine,
            ]
        }
        Swing::Divided => {
            let half_angle = quadrant(length, direction[0], atan) * 0.5;
            let cosine = try_cosine(half_angle, rounding).map_err(E::Rounding)?;
            let sine = try_sine(half_angle, rounding).map_err(E::Rounding)?;
            let inverse = 1.0 / length;
            [
                0.0,
                (direction[2] * inverse) * sine,
                flip((direction[1] * inverse) * sine),
                cosine,
            ]
        }
    };
    let twist = compose(inverse_swing, delta, numeric).map_err(E::Numeric)?; //97f0..9818
    let positive_seed = numeric.reciprocal_estimate(twist[3]).map_err(E::Numeric)?;
    let positive_inverse = refine_inverse(twist[3], positive_seed);
    let positive_atan =
        try_atan(twist[0].mul_add(positive_inverse, 0.0), numeric).map_err(E::Numeric)?;
    let negative_w = flip(twist[3]);
    let negative_x = flip(twist[0]);
    let negative_seed = numeric
        .reciprocal_estimate(negative_w)
        .map_err(E::Numeric)?;
    let negative_inverse = refine_inverse(negative_w, negative_seed);
    let negative_atan =
        try_atan(negative_x.mul_add(negative_inverse, 0.0), numeric).map_err(E::Numeric)?;
    let positive = quadrant(twist[0], twist[3], positive_atan);
    let negative = quadrant(negative_x, negative_w, negative_atan);
    let angle = if 0.0 > twist[3] { negative } else { positive } * ratio;
    let cosine = try_cosine(angle, rounding).map_err(E::Rounding)?; //9930 before sine
    let sine = try_sine(angle, rounding).map_err(E::Rounding)?; //993c
    let adjustment = [sine, 0.0, 0.0, cosine];
    channels
        .rotation(destination, adjustment)
        .map_err(E::Owner)?; //9950
    let bind_destination = channels
        .bind(destination)
        .ok_or(E::BindBone(destination))?
        .rotation; //9954
    let result = compose(bind_destination, adjustment, numeric).map_err(E::Numeric)?;
    channels.rotation(destination, result).map_err(E::Owner)?; //9984
    Ok(())
}

#[cfg(test)]
pub(crate) fn try_twist<N: PresentationMatrixNumeric, R: ReckoningSinCosNumeric>(
    bind: &[AndaleSqt],
    replay: &mut [AndaleSqt],
    source: usize,
    destination: usize,
    ratio: f32,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<(), TwistError<N::Error, R::Error>> {
    try_twist_channels(
        &mut super::corrections::SliceChannels { bind, replay },
        source,
        destination,
        ratio,
        numeric,
        rounding,
    )
}
#[cfg(test)]
#[path = "skater_presentation_twist_tests.rs"]
pub(super) mod tests;
