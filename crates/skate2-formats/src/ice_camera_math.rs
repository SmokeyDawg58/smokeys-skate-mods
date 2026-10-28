//! ICE fixed-angle math, source82a33330/82a33628; ATan table830022f8.
pub(super) const ATAN: [u16; 257] = [
    0, 41, 81, 122, 163, 204, 244, 285, 326, 367, 407, 448, 489, 529, 570, 610, 651, 692, 732, 773,
    813, 854, 894, 935, 975, 1015, 1056, 1096, 1136, 1177, 1217, 1257, 1297, 1337, 1377, 1417,
    1457, 1497, 1537, 1577, 1617, 1656, 1696, 1736, 1775, 1815, 1854, 1894, 1933, 1973, 2012, 2051,
    2090, 2129, 2168, 2207, 2246, 2285, 2324, 2363, 2401, 2440, 2478, 2517, 2555, 2594, 2632, 2670,
    2708, 2746, 2784, 2822, 2860, 2897, 2935, 2973, 3010, 3047, 3085, 3122, 3159, 3196, 3233, 3270,
    3307, 3344, 3380, 3417, 3453, 3490, 3526, 3562, 3599, 3635, 3670, 3706, 3742, 3778, 3813, 3849,
    3884, 3920, 3955, 3990, 4025, 4060, 4095, 4129, 4164, 4199, 4233, 4267, 4302, 4336, 4370, 4404,
    4438, 4471, 4505, 4539, 4572, 4605, 4639, 4672, 4705, 4738, 4771, 4803, 4836, 4869, 4901, 4933,
    4966, 4998, 5030, 5062, 5094, 5125, 5157, 5188, 5220, 5251, 5282, 5313, 5344, 5375, 5406, 5437,
    5467, 5498, 5528, 5559, 5589, 5619, 5649, 5679, 5708, 5738, 5768, 5797, 5826, 5856, 5885, 5914,
    5943, 5972, 6000, 6029, 6058, 6086, 6114, 6142, 6171, 6199, 6227, 6254, 6282, 6310, 6337, 6365,
    6392, 6419, 6446, 6473, 6500, 6527, 6554, 6580, 6607, 6633, 6660, 6686, 6712, 6738, 6764, 6790,
    6815, 6841, 6867, 6892, 6917, 6943, 6968, 6993, 7018, 7043, 7068, 7092, 7117, 7141, 7166, 7190,
    7214, 7238, 7262, 7286, 7310, 7334, 7358, 7381, 7405, 7428, 7451, 7475, 7498, 7521, 7544, 7566,
    7589, 7612, 7635, 7657, 7679, 7702, 7724, 7746, 7768, 7790, 7812, 7834, 7856, 7877, 7899, 7920,
    7942, 7963, 7984, 8005, 8026, 8047, 8068, 8089, 8110, 8131, 8151, 8172, 8192,
];

pub(super) fn atan(x: f32, y: f32) -> u16 {
    let quadrant = if x < 0. {
        if y < 0. { 2 } else { 1 }
    } else if y < 0. {
        3
    } else {
        0
    };
    let x = x.abs();
    let y = y.abs();
    let interp = |ratio: f32| {
        let q = (ratio * 65536.) as usize;
        let i = q >> 8;
        ATAN[i] + (((u32::from(ATAN[i + 1] - ATAN[i])) * (q as u32 & 255)) >> 8) as u16
    };
    let a = if x > y {
        interp(y / x)
    } else if y > x {
        0x4000 - interp(x / y)
    } else if x == 0. {
        0
    } else {
        0x2000
    };
    match quadrant {
        1 => 0x8000 - a,
        2 => a.wrapping_add(0x8000),
        3 => a.wrapping_neg(),
        _ => a,
    }
}
/// `ICE::XYZ2HSV` (`0x82a34040`): the hue in turns from `ICE::ATan(y, x)`,
/// the saturation `sqrt(x * x + y * y)` held to 1, and `z` as the value.
pub(super) fn xyz_to_hsv(x: f32, y: f32, z: f32) -> [f32; 3] {
    let h = f32::from(atan(y, x)) * f32::from_bits(0x3780_0000);
    let s = x.mul_add(x, y * y).sqrt();
    [h, if s <= 1. { s } else { 1. }, z]
}

/// `ICE::HSV2RGB` (`0x82a54200`): the sextant `(int)(6h)` picks the order
/// of v, p = (1 - s)v, q = (1 - fs)v and t = (1 - (1 - f)s)v. A hue from
/// `xyz_to_hsv` is below one turn, so the sextant is 0..=5.
pub(super) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h6 = h * 6.;
    let sextant = h6 as i32;
    let f = h6 - sextant as f32;
    let p = (1. - s) * v;
    let q = (-f).mul_add(s, 1.) * v;
    let t = (-(1. - f)).mul_add(s, 1.) * v;
    match sextant {
        0 | 6 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        5 => [v, p, q],
        _ => unreachable!("a hue below one turn"),
    }
}

/// `ICE::Sin` and `ICE::SinCos` share this: the angle in radians folded
/// into [-pi/2, pi/2) with the half-turn's sign.
fn reduce(angle: u16) -> (f32, f32) {
    let x = f32::from(angle) * f32::from_bits(0x38c9_0fdb);
    if x >= f32::from_bits(0x4096_cbe4) {
        (x - f32::from_bits(0x40c9_0fdb), 1.0)
    } else if x >= f32::from_bits(0x3fc9_0fdb) {
        (x - f32::from_bits(0x4049_0fdb), -1.0)
    } else {
        (x, 1.0)
    }
}

const S3: u32 = 0x3e2a_aaab;
const S5: u32 = 0x3c08_8889;
const S7: u32 = 0x3950_0d01;
const S9: u32 = 0x3638_ef1d;

/// `ICE::Sin` (`0x82a33330`): the odd terms accumulated with fused
/// multiply-adds (`fnmsubs`, `fmadds`).
pub(super) fn sin(angle: u16) -> f32 {
    let (x, sign) = reduce(angle);
    let x2 = x * x;
    let x3 = x2 * x;
    let x5 = x3 * x2;
    let x7 = x5 * x2;
    let x9 = x7 * x2;
    let s = (-x3).mul_add(f32::from_bits(S3), x);
    let s = x5.mul_add(f32::from_bits(S5), s);
    let s = (-x7).mul_add(f32::from_bits(S7), s);
    x9.mul_add(f32::from_bits(S9), s) * sign
}

/// `ICE::SinCos` (`0x82a333e8`): the same sine and an even series of its own
/// for the cosine, both fused and both signed by the half-turn.
pub(super) fn sin_cos(angle: u16) -> (f32, f32) {
    let (x, sign) = reduce(angle);
    let x2 = x * x;
    let x3 = x2 * x;
    let c = (-x2).mul_add(0.5, 1.0);
    let x4 = x2 * x2;
    let s = (-x3).mul_add(f32::from_bits(S3), x);
    let x5 = x3 * x2;
    let x6 = x4 * x2;
    let c = x4.mul_add(f32::from_bits(0x3d2a_aaab), c);
    let x7 = x5 * x2;
    let s = x5.mul_add(f32::from_bits(S5), s);
    let x8 = x6 * x2;
    let c = (-x6).mul_add(f32::from_bits(0x3ab6_0b61), c);
    let x9 = x7 * x2;
    let s = (-x7).mul_add(f32::from_bits(S7), s);
    let x10 = x8 * x2;
    let c = x8.mul_add(f32::from_bits(0x37d0_0d01), c);
    let s = x9.mul_add(f32::from_bits(S9), s);
    let c = (-x10).mul_add(f32::from_bits(0x3493_f27e), c);
    (s * sign, c * sign)
}

/// `ICE::MulMatrix` (`0x82a33778`): `b * a` (row vectors). Each element is a
/// fused chain from one rounded product; the scheduled term order differs
/// for three elements.
pub(super) fn mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    std::array::from_fn(|r| {
        std::array::from_fn(|c| {
            let [first, rest @ ..]: [usize; 4] = match (r, c) {
                (1, 0) => [2, 1, 3, 0],
                (3, 2) => [3, 2, 0, 1],
                (3, 3) => [1, 3, 2, 0],
                _ => [1, 0, 2, 3],
            };
            rest.iter().fold(b[r][first] * a[first][c], |sum, &k| b[r][k].mul_add(a[k][c], sum))
        })
    })
}

/// `ICE::RotateY` (`0x82a33c90`): `MulMatrix` by the turn `SinCos` gives.
pub(super) fn rotate_y(m: [[f32; 4]; 4], angle: u16) -> [[f32; 4]; 4] {
    let (s, c) = sin_cos(angle);
    mul(m, [[c, 0., -s, 0.], [0., 1., 0., 0.], [s, 0., c, 0.], [0., 0., 0., 1.]])
}

/// `ICE::RotateZ` (`0x82a33d40`).
pub(super) fn rotate_z(m: [[f32; 4]; 4], angle: u16) -> [[f32; 4]; 4] {
    let (s, c) = sin_cos(angle);
    mul(m, [[c, s, 0., 0.], [-s, c, 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]])
}

/// `ASinTable` (`0x83001c70`): each segment's start angle and slope (f32
/// bits). Level `l` holds 16 segments over [1 - 2^-l, 1 - 2^-(l+1)).
const ASIN_TABLE: [(u16, u32); 192] = [
    (0x0000, 0x3e230553), (0x0146, 0x3e231f8a), (0x028c, 0x3e236e2f), (0x03d3, 0x3e23f141), (0x051b, 0x3e24a8c1), (0x0664, 0x3e257a78),
    (0x07af, 0x3e266666), (0x08fc, 0x3e27a0f9), (0x0a4b, 0x3e290ff9), (0x0b9d, 0x3e2ab368), (0x0cf3, 0x3e2c8b44), (0x0e4c, 0x3e2e978d),
    (0x0fa9, 0x3e310cb3), (0x110b, 0x3e33d07d), (0x1273, 0x3e36c8b4), (0x13e0, 0x3e3a43fe), (0x1555, 0x3e3d3c36), (0x1612, 0x3e3f4880),
    (0x16d1, 0x3e418937), (0x1793, 0x3e43e426), (0x1857, 0x3e467382), (0x191d, 0x3e491d15), (0x19e6, 0x3e4bfb16), (0x1ab2, 0x3e4f27bb),
    (0x1b81, 0x3e5288ce), (0x1c54, 0x3e561e4f), (0x1d2a, 0x3e5a0275), (0x1e04, 0x3e5e353f), (0x1ee2, 0x3e62d0e5), (0x1fc5, 0x3e67bb30),
    (0x20ad, 0x3e6d288d), (0x219a, 0x3e733333), (0x228d, 0x3e780347), (0x2309, 0x3e7b98c8), (0x2387, 0x3e7f2e49), (0x2407, 0x3e818937),
    (0x2488, 0x3e839581), (0x250c, 0x3e85bc02), (0x2591, 0x3e8809d5), (0x2619, 0x3e8a7efa), (0x26a4, 0x3e8d1b71), (0x2731, 0x3e8fec57),
    (0x27c1, 0x3e92e48f), (0x2854, 0x3e961e4f), (0x28ea, 0x3e99999a), (0x2984, 0x3e9d6388), (0x2a21, 0x3ea17c1c), (0x2ac2, 0x3ea5f06f),
    (0x2b68, 0x3ea9930c), (0x2bbd, 0x3eac2268), (0x2c13, 0x3eaed917), (0x2c6b, 0x3eb1b717), (0x2cc4, 0x3eb4bc6a), (0x2d1e, 0x3eb7e910),
    (0x2d7a, 0x3ebb4a23), (0x2dd8, 0x3ebedfa4), (0x2e37, 0x3ec2b6ae), (0x2e98, 0x3ec6c227), (0x2efc, 0x3ecb295f), (0x2f61, 0x3ecfd220),
    (0x2fc9, 0x3ed4e3bd), (0x3034, 0x3eda511a), (0x30a1, 0x3ee04189), (0x3111, 0x3ee6c227), (0x3184, 0x3eebfb16), (0x31bf, 0x3eefaace),
    (0x31fb, 0x3ef39c0f), (0x3238, 0x3ef7b4a2), (0x3276, 0x3efc0ebf), (0x32b5, 0x3f004ea5), (0x32f5, 0x3f02bd3c), (0x3337, 0x3f055326),
    (0x3379, 0x3f0809d5), (0x33bd, 0x3f0afb7f), (0x3403, 0x3f0e1b09), (0x344a, 0x3f11758e), (0x3493, 0x3f15119d), (0x34dd, 0x3f18f5c3),
    (0x352a, 0x3f1d2f1b), (0x3578, 0x3f21d14e), (0x35c9, 0x3f258e22), (0x35f2, 0x3f283127), (0x361d, 0x3f2afb7f), (0x3647, 0x3f2de69b),
    (0x3673, 0x3f30ff97), (0x369f, 0x3f344674), (0x36cc, 0x3f37bb30), (0x36fa, 0x3f3b645a), (0x3729, 0x3f3f4880), (0x3759, 0x3f4374bc),
    (0x378a, 0x3f47e282), (0x37bb, 0x3f4cac08), (0x37ef, 0x3f51cac1), (0x3823, 0x3f5758e2), (0x3859, 0x3f5d5cfb), (0x3890, 0x3f63eab3),
    (0x38c9, 0x3f69374c), (0x38e6, 0x3f6cfaad), (0x3904, 0x3f70ebee), (0x3922, 0x3f75182b), (0x3941, 0x3f7978d5), (0x3960, 0x3f7e1b09),
    (0x3980, 0x3f817f63), (0x39a0, 0x3f841bda), (0x39c1, 0x3f86dfa4), (0x39e3, 0x3f89d14e), (0x3a05, 0x3f8cf766), (0x3a29, 0x3f905879),
    (0x3a4d, 0x3f93fe5d), (0x3a72, 0x3f97ec57), (0x3a98, 0x3f9c2f83), (0x3abf, 0x3fa0d4fe), (0x3ae7, 0x3fa49518), (0x3afb, 0x3fa73eab),
    (0x3b10, 0x3faa0c4a), (0x3b26, 0x3facfdf4), (0x3b3b, 0x3fb01a37), (0x3b51, 0x3fb3645a), (0x3b68, 0x3fb6dc5d), (0x3b7f, 0x3fba8c15),
    (0x3b96, 0x3fbe76c9), (0x3bae, 0x3fc2a305), (0x3bc6, 0x3fc71aa0), (0x3bdf, 0x3fcbe426), (0x3bf8, 0x3fd1096c), (0x3c12, 0x3fd69ad4),
    (0x3c2d, 0x3fdca234), (0x3c49, 0x3fe3367a), (0x3c65, 0x3fe889a0), (0x3c74, 0x3fec4d01), (0x3c83, 0x3ff044d0), (0x3c92, 0x3ff4710d),
    (0x3ca1, 0x3ff8d845), (0x3cb0, 0x3ffd7dbf), (0x3cc0, 0x40013405), (0x3cd0, 0x4003ced9), (0x3ce1, 0x40069446), (0x3cf2, 0x40098937),
    (0x3d03, 0x400cb0f2), (0x3d14, 0x4010154d), (0x3d27, 0x4013b98c), (0x3d39, 0x4017a92a), (0x3d4c, 0x401bedfa), (0x3d5f, 0x40209518),
    (0x3d73, 0x40245879), (0x3d7e, 0x402703b0), (0x3d88, 0x4029d2f2), (0x3d93, 0x402cc63f), (0x3d9e, 0x402fe282), (0x3da9, 0x40332b02),
    (0x3db4, 0x4036a64c), (0x3dbf, 0x403a5604), (0x3dcb, 0x403e425b), (0x3dd7, 0x4042703b), (0x3de3, 0x4046e7d5), (0x3def, 0x404bb2ff),
    (0x3dfc, 0x4050d9e8), (0x3e09, 0x40566cf4), (0x3e17, 0x405c75f7), (0x3e24, 0x40630be1), (0x3e33, 0x40685d64), (0x3e3a, 0x406c240b),
    (0x3e41, 0x40701bda), (0x3e49, 0x407449ba), (0x3e50, 0x4078af4f), (0x3e58, 0x407d54ca), (0x3e60, 0x4081205c), (0x3e68, 0x4083bc02),
    (0x3e70, 0x40868241), (0x3e79, 0x40897732), (0x3e81, 0x408c9fbe), (0x3e8a, 0x40900347), (0x3e93, 0x4093a92a), (0x3e9c, 0x409798c8),
    (0x3ea6, 0x409bde6a), (0x3eb0, 0x40a08659), (0x3eba, 0x40a449ba), (0x3ebf, 0x40a6f5c3), (0x3ec4, 0x40a9c433), (0x3ec9, 0x40acb780),
    (0x3ecf, 0x40afd495), (0x3ed4, 0x40b31de7), (0x3eda, 0x40b6985f), (0x3edf, 0x40ba48e9), (0x3ee5, 0x40be346e), (0x3eeb, 0x40c26320),
    (0x3ef1, 0x40c6db8c), (0x3ef7, 0x40cba6b5), (0x3efe, 0x40d0ce70), (0x3f04, 0x40d660aa), (0x3f0b, 0x40dc6b51), (0x3f12, 0x40e30069),
];

/// `ICE::ASin` (`0x82a33500`): the binary angle of x, from the table
/// segment holding it, less its slope times the remainder.
pub(super) fn asin(x: f32) -> u16 {
    let (x, negative) = if x < 0. { (-x, true) } else { (x, false) };
    if x >= 1. {
        return if negative { 0xc000 } else { 0x4000 };
    }
    let v = (x * 65536.) as i32;
    let (mut level, mut step, mut top) = (0i32, 0x8000i32, 0x8000i32);
    if v >= top {
        while level < 11 {
            step >>= 1;
            level += 1;
            top += step;
            if v < top {
                break;
            }
        }
    }
    let base = top - step;
    let index = (v - base) >> (11 - level);
    let start = (step >> 4) * index + base;
    let (angle, slope) = ASIN_TABLE[(level * 16 + index) as usize];
    let rest = (-(start as f32)).mul_add(f32::from_bits(0x3780_0000), x);
    let turn = (rest * f32::from_bits(slope) * -65536.) as i32;
    let result = (i32::from(angle) - turn) as u16;
    if negative { result.wrapping_neg() } else { result }
}

/// `ICELITE::NoiseInterpolated` (`0x82a34f68`): integer value noise at the
/// floor of x and the next integer, mixed linearly.
pub(super) fn noise_interpolated(x: f32) -> f32 {
    let mut floor = (x as i32) as f32;
    if floor > x {
        floor -= 1.;
    }
    let n = floor as i32;
    let value = |n: i32| {
        let v = (n << 13) ^ n;
        let h = (v.wrapping_mul(v).wrapping_mul(15731).wrapping_add(789_221).wrapping_mul(v).wrapping_sub(0x2df7_22f3)) & 0x7fff_ffff;
        (-(h as f32)).mul_add(f32::from_bits(0x3080_0000), 1.)
    };
    let fraction = x - n as f32;
    value(n).mul_add(1. - fraction, value(n.wrapping_add(1)) * fraction)
}

/// `ICE::Noise` (`0x82a35058`): six octaves of `NoiseInterpolated`, the
/// frequency doubling and the weight halving.
pub(super) fn noise(x: f32) -> f32 {
    let (mut frequency, mut weight, mut sum) = (1f32, 1f32, 0f32);
    for _ in 0..6 {
        sum = noise_interpolated(frequency * x).mul_add(weight, sum);
        frequency *= 2.;
        weight *= 0.5;
    }
    sum
}
