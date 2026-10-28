// Copyright (c) 2002 Fabrice Bellard
// SPDX-License-Identifier: LGPL-2.1-or-later
//
// The half-length inverse MDCT of FFmpeg's `ff_imdct_half_c` (libavcodec/mdct_template.c):
// pre-rotation, an n/4-point inverse complex FFT, post-rotation. For a full transform of
// size n with coefficients X[0..n/2) the output is the middle half of
//     y[i] = -scale * sum_k X[k] * cos(pi * (2i + 1 + n/2) * (2k + 1) / (2n)),  i in 0..n,
// that is y[n/4..3n/4).

#[derive(Clone, Copy, Default)]
pub(crate) struct Complex {
    re: f32,
    im: f32,
}

struct InverseFft {
    reverse: Vec<usize>,
    twiddle: Vec<Complex>,
}

impl InverseFft {
    fn new(n: usize) -> Self {
        let bits = n.trailing_zeros();
        let reverse = (0..n)
            .map(|i| if bits == 0 { 0 } else { i.reverse_bits() >> (usize::BITS - bits) })
            .collect();
        let twiddle = (0..n / 2)
            .map(|k| {
                let angle = 2.0 * std::f64::consts::PI * k as f64 / n as f64;
                Complex {
                    re: angle.cos() as f32,
                    im: angle.sin() as f32,
                }
            })
            .collect();
        Self { reverse, twiddle }
    }

    /// `z[m] = sum_k x[k] * exp(+2 pi i k m / n)`, unnormalised.
    fn run(&self, input: &[Complex], output: &mut [Complex]) {
        let n = input.len();
        for (index, value) in input.iter().enumerate() {
            output[self.reverse[index]] = *value;
        }
        let mut len = 2;
        while len <= n {
            let half = len / 2;
            let step = n / len;
            for base in (0..n).step_by(len) {
                for j in 0..half {
                    let w = self.twiddle[j * step];
                    let u = output[base + j];
                    let x = output[base + j + half];
                    let v = Complex {
                        re: x.re * w.re - x.im * w.im,
                        im: x.re * w.im + x.im * w.re,
                    };
                    output[base + j] = Complex {
                        re: u.re + v.re,
                        im: u.im + v.im,
                    };
                    output[base + j + half] = Complex {
                        re: u.re - v.re,
                        im: u.im - v.im,
                    };
                }
            }
            len *= 2;
        }
    }
}

pub(crate) struct Imdct {
    n: usize,
    tcos: Vec<f32>,
    tsin: Vec<f32>,
    fft: InverseFft,
}

impl Imdct {
    /// `ff_mdct_init(s, nbits, 1, scale)`.
    pub(crate) fn new(nbits: u32, scale: f64) -> Self {
        let n = 1usize << nbits;
        let n4 = n >> 2;
        let theta = 1.0 / 8.0 + if scale < 0.0 { n4 as f64 } else { 0.0 };
        let scale = scale.abs().sqrt();
        let mut tcos = Vec::with_capacity(n4);
        let mut tsin = Vec::with_capacity(n4);
        for i in 0..n4 {
            let alpha = 2.0 * std::f64::consts::PI * (i as f64 + theta) / n as f64;
            tcos.push((-alpha.cos() * scale) as f32);
            tsin.push((-alpha.sin() * scale) as f32);
        }
        Self {
            n,
            tcos,
            tsin,
            fft: InverseFft::new(n4),
        }
    }

    /// `imdct_half`: `n/2` coefficients in, `n/2` samples out.
    pub(crate) fn half(&self, output: &mut [f32], input: &[f32]) {
        let n2 = self.n >> 1;
        let n4 = self.n >> 2;
        let n8 = self.n >> 3;
        let mut rotated = vec![Complex::default(); n4];
        for k in 0..n4 {
            let in1 = input[2 * k];
            let in2 = input[n2 - 1 - 2 * k];
            rotated[k] = Complex {
                re: in2 * self.tcos[k] - in1 * self.tsin[k],
                im: in2 * self.tsin[k] + in1 * self.tcos[k],
            };
        }
        let mut z = vec![Complex::default(); n4];
        self.fft.run(&rotated, &mut z);
        for k in 0..n8 {
            let a = z[n8 - k - 1];
            let b = z[n8 + k];
            let (sa, ca) = (self.tsin[n8 - k - 1], self.tcos[n8 - k - 1]);
            let (sb, cb) = (self.tsin[n8 + k], self.tcos[n8 + k]);
            let r0 = a.im * sa - a.re * ca;
            let i1 = a.im * ca + a.re * sa;
            let r1 = b.im * sb - b.re * cb;
            let i0 = b.im * cb + b.re * sb;
            z[n8 - k - 1] = Complex { re: r0, im: i0 };
            z[n8 + k] = Complex { re: r1, im: i1 };
        }
        for (index, value) in z.iter().enumerate() {
            output[2 * index] = value.re;
            output[2 * index + 1] = value.im;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Imdct;

    #[test]
    fn half_imdct_matches_the_reference_sum() {
        for nbits in [7u32, 8, 9, 10] {
            let n = 1usize << nbits;
            let scale = 1.0 / f64::from(1u32 << (nbits - 2)) / 32768.0;
            let imdct = Imdct::new(nbits, scale);
            let input: Vec<f32> = (0..n / 2)
                .map(|k| ((k * 7919 + 13) % 2001) as f32 - 1000.0)
                .collect();
            let mut output = vec![0.0f32; n / 2];
            imdct.half(&mut output, &input);
            for (j, value) in output.iter().enumerate() {
                let i = j + n / 4;
                let sum: f64 = input
                    .iter()
                    .enumerate()
                    .map(|(k, x)| {
                        let a = ((2 * i + 1 + n / 2) * (2 * k + 1)) as f64;
                        f64::from(*x) * (std::f64::consts::PI * a / (2 * n) as f64).cos()
                    })
                    .sum();
                let expected = -sum * scale;
                assert!(
                    (f64::from(*value) - expected).abs() < 1e-4,
                    "n={n} j={j}: {value} vs {expected}"
                );
            }
        }
    }
}
