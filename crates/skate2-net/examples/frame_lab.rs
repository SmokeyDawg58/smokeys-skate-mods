//! Compares frame coding options over a `SKATE2_DIAGNOSTIC_ONLINE_FRAME_DUMP` file (bits only;
//! nothing is decoded): pose precision, prediction from two acknowledged poses, the header's
//! parts, audio precision and the camera's rate.
//!
//! `cargo run --release -p skate2-net --example frame_lab -- <dump> [delay]`

use skate2_net::codec::BitWriter;
use skate2_net::frames::{NetFrame, read_dump_record};

fn frames(path: &str) -> Vec<NetFrame> {
    let data = std::fs::read(path).expect("dump readable");
    let mut frames = Vec::new();
    let mut at = 0;
    while at + 4 <= data.len() {
        let length = u32::from_le_bytes(data[at..at + 4].try_into().expect("length")) as usize;
        at += 4;
        frames.push(read_dump_record(&data[at..at + length]).expect("record"));
        at += length;
    }
    frames
}

#[expect(clippy::cast_possible_truncation, reason = "quantization")]
fn q(value: f32, units: f32) -> i64 {
    (f64::from(value) * f64::from(units)).round() as i64
}

#[derive(Clone, Copy)]
struct PoseOptions {
    rotation: f32,
    translation: f32,
    scale: f32,
    predict: bool,
    rice: bool,
    three: bool,
}

/// Adaptive Rice code of a residual with `k` from the lane's running mean.
fn rice(bits: &mut BitWriter, value: i64, mean: &mut u32) {
    let z = skate2_net::codec::zigzag(value);
    let k = 32 - (*mean).leading_zeros();
    let k = k.saturating_sub(1).min(20);
    let quotient = z >> k;
    if quotient < 16 {
        for _ in 0..quotient {
            bits.bit(true);
        }
        bits.bit(false);
        bits.write(z & ((1 << k) - 1), k);
    } else {
        for _ in 0..16 {
            bits.bit(true);
        }
        bits.gamma(z);
    }
    let z32 = u32::try_from(z.min(1 << 24)).unwrap_or(1 << 24);
    *mean = (*mean * 3 + z32 * 4 + 2) / 4 / 1;
    *mean = (*mean).min(1 << 24);
}

fn quant_pose(pose: &[[f32; 12]], o: PoseOptions) -> Vec<[i64; 12]> {
    pose.iter()
        .map(|bone| {
            std::array::from_fn(|lane| {
                let units = match lane {
                    0..4 => o.scale,
                    4..8 => o.rotation,
                    _ => o.translation,
                };
                q(bone[lane], units)
            })
        })
        .collect()
}

/// Bits of one pose against a baseline (and with prediction, the pose before it).
fn pose_bits(current: &[[i64; 12]], base: Option<(&[[i64; 12]], u32)>, before: Option<(&[[i64; 12]], u32)>, tick: u32, o: PoseOptions, means: &mut Vec<[u32; 12]>) -> usize {
    let predict = o.predict;
    let mut bits = BitWriter::new();
    if means.len() != current.len() {
        *means = vec![[4; 12]; current.len()];
    }
    let Some((base, base_tick)) = base else {
        for bone in current {
            for &v in bone {
                bits.signed(v);
            }
        }
        return bits.bit_len();
    };
    for (index, bone) in current.iter().enumerate() {
        let predicted: [i64; 12] = std::array::from_fn(|lane| {
            let b1 = base[index][lane];
            match before.filter(|_| predict) {
                Some((before, before_tick)) => {
                    let b0 = before[index][lane];
                    let span = i64::from(base_tick - before_tick).max(1);
                    let ahead = i64::from(tick - base_tick);
                    b1 + ((b1 - b0) * ahead + span / 2).div_euclid(span)
                }
                None => b1,
            }
        });
        let groups: [bool; 3] = std::array::from_fn(|g| (g * 4..g * 4 + 4).any(|lane| bone[lane] != predicted[lane]));
        let changed = groups.iter().any(|&g| g);
        bits.bit(changed);
        if !changed {
            continue;
        }
        for &g in &groups {
            bits.bit(g);
        }
        for (g, &c) in groups.iter().enumerate() {
            if c {
                for lane in g * 4..g * 4 + 4 {
                    if o.three && g == 1 && lane == 4 + largest(bone) {
                        continue;
                    }
                    if g == 0 && lane == 3 || g == 2 && lane == 11 {
                        continue;
                    }
                    if o.rice {
                        rice(&mut bits, bone[lane] - predicted[lane], &mut means[index][lane]);
                    } else {
                        bits.signed(bone[lane] - predicted[lane]);
                    }
                }
                if o.three && g == 1 {
                    bits.write(0, 2);
                }
            }
        }
    }
    bits.bit_len()
}

fn largest(bone: &[i64; 12]) -> usize {
    (0..4).max_by_key(|&i| bone[4 + i].abs()).unwrap_or(3)
}

fn pose_rate(frames: &[NetFrame], delay: u32, o: PoseOptions) -> f64 {
    let poses: Vec<(u32, Vec<[i64; 12]>)> = frames.iter().filter_map(|f| f.pose.as_ref().map(|p| (f.tick, quant_pose(p, o)))).collect();
    let mut total = 0usize;
    let mut means = Vec::new();
    for (i, (tick, pose)) in poses.iter().enumerate() {
        let acked: Vec<&(u32, Vec<[i64; 12]>)> = poses[..i].iter().rev().filter(|(t, _)| t + delay <= *tick).take(2).collect();
        let base = acked.first().map(|(t, p)| (p.as_slice(), *t));
        let before = acked.get(1).map(|(t, p)| (p.as_slice(), *t));
        total += pose_bits(pose, base, before, *tick, o, &mut means);
    }
    #[expect(clippy::cast_precision_loss, reason = "statistics")]
    let rate = total as f64 / 8.0 / frames.len() as f64 * 60.0 / 1024.0;
    rate
}

/// Header words 32..52 with the root quaternion and position quantized and predicted, the
/// rest XOR coded; words 0..32 (the hips and board local matrices) left out.
fn header_rate(frames: &[NetFrame], delay: usize, rotation: f32, position: f32, predict: bool) -> f64 {
    let values = |f: &NetFrame| -> [i64; 7] {
        std::array::from_fn(|i| {
            let value = f32::from_bits(f.header[32 + i]);
            q(value, if i < 4 { rotation } else { position })
        })
    };
    let mut total = 0usize;
    for (i, frame) in frames.iter().enumerate() {
        let mut bits = BitWriter::new();
        let current = values(frame);
        let base = i.checked_sub(delay);
        let predicted: [i64; 7] = match base {
            None => [0; 7],
            Some(b) => {
                let b1 = values(&frames[b]);
                match b.checked_sub(1).filter(|_| predict) {
                    Some(b0) => {
                        let b0 = values(&frames[b0]);
                        let ahead = i64::try_from(delay).unwrap_or(1);
                        std::array::from_fn(|k| b1[k] + (b1[k] - b0[k]) * ahead)
                    }
                    None => b1,
                }
            }
        };
        for k in 0..7 {
            bits.signed(current[k] - predicted[k]);
        }
        for word in 39..52 {
            let x = frame.header[word] ^ base.map_or(0, |b| frames[b].header[word]);
            bits.bit(x != 0);
            if x != 0 {
                let width = 32 - x.leading_zeros();
                bits.write(u64::from(width - 1), 5);
                bits.write(u64::from(x), width);
            }
        }
        total += bits.bit_len();
    }
    #[expect(clippy::cast_precision_loss, reason = "statistics")]
    let rate = total as f64 / 8.0 / frames.len() as f64 * 60.0 / 1024.0;
    rate
}

/// Audio floats quantized at `units`, every `every` ticks, against the delayed baseline.
fn audio_rate(frames: &[NetFrame], delay: usize, units: f32, every: usize) -> f64 {
    let mut total = 0usize;
    for (i, frame) in frames.iter().enumerate() {
        let mut bits = BitWriter::new();
        if i % every == 0 {
            let base = i.checked_sub(delay.max(every));
            for (k, &value) in frame.audio_floats.iter().enumerate() {
                let b = base.and_then(|b| frames[b].audio_floats.get(k)).map_or(0, |&v| q(v, units));
                bits.signed(q(value, units) - b);
            }
        }
        let base = i.checked_sub(delay);
        for (k, &word) in frame.audio_words.iter().enumerate() {
            let x = word ^ base.and_then(|b| frames[b].audio_words.get(k)).copied().unwrap_or(0);
            bits.bit(x != 0);
            if x != 0 {
                let width = 32 - x.leading_zeros();
                bits.write(u64::from(width - 1), 5);
                bits.write(u64::from(x), width);
            }
        }
        total += bits.bit_len();
    }
    #[expect(clippy::cast_precision_loss, reason = "statistics")]
    let rate = total as f64 / 8.0 / frames.len() as f64 * 60.0 / 1024.0;
    rate
}

fn bone_report(frames: &[NetFrame]) {
    let o = PoseOptions { scale: 1024.0, rotation: 2048.0, translation: 1024.0, predict: true, rice: false, three: false };
    let poses: Vec<(u32, Vec<[i64; 12]>)> = frames.iter().filter_map(|f| f.pose.as_ref().map(|p| (f.tick, quant_pose(p, o)))).collect();
    let bones = poses[0].1.len();
    let mut cost = vec![0usize; bones];
    let mut never = vec![true; bones];
    let mut group_changes = vec![[0usize; 3]; bones];
    for i in 4..poses.len() {
        let (tick, pose) = &poses[i];
        let (t1, b1) = &poses[i - 3];
        let (t0, b0) = &poses[i - 4];
        for bone in 0..bones {
            let one = [pose[bone]];
            let base = [b1[bone]];
            let before = [b0[bone]];
            let mut means = Vec::new();
            let bits = pose_bits(&one, Some((&base, *t1)), Some((&before, *t0)), *tick, o, &mut means);
            cost[bone] += bits;
            if pose[bone] != b1[bone] {
                never[bone] = false;
            }
            for g in 0..3 {
                if (g * 4..g * 4 + 4).any(|lane| pose[bone][lane] != b1[bone][lane]) {
                    group_changes[bone][g] += 1;
                }
            }
        }
    }
    let mut order: Vec<usize> = (0..bones).collect();
    order.sort_by_key(|&b| std::cmp::Reverse(cost[b]));
    let total: usize = cost.iter().sum();
    println!("bones never changing: {}", never.iter().filter(|&&n| n).count());
    let n = poses.len() - 3;
    for &b in order.iter().take(40) {
        println!("  bone {b:3}: {:5.1}% of pose bits, {:5.1} bits/sample, changes s/r/t {:.0}%/{:.0}%/{:.0}%", cost[b] as f64 * 100.0 / total as f64, cost[b] as f64 / n as f64,
            group_changes[b][0] as f64 * 100.0 / n as f64, group_changes[b][1] as f64 * 100.0 / n as f64, group_changes[b][2] as f64 * 100.0 / n as f64);
    }
    let translation_changing = (0..bones).filter(|&b| group_changes[b][2] > 0).count();
    let scale_changing = (0..bones).filter(|&b| group_changes[b][0] > 0).count();
    println!("bones with changing translation {translation_changing}, scale {scale_changing}");
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("dump path");
    let frames = frames(&path);
    if std::env::var_os("SKATE2_FRAME_BONES").is_some() {
        bone_report(&frames);
        return;
    }
    for delay in [2u32, 6, 12] {
        println!("delay {delay} ticks:");
        let base = PoseOptions { scale: 1024.0, rotation: 2048.0, translation: 1024.0, predict: true, rice: false, three: false };
        for (name, o) in [
            ("rot 2048 trans 1024 predicted", base),
            ("rot 2048 trans 1024 predicted rice", PoseOptions { rice: true, ..base }),
            ("rot 2048 trans 1024 predicted rice three", PoseOptions { rice: true, three: true, ..base }),
            ("rot 1024 trans 1024 predicted rice three", PoseOptions { rotation: 1024.0, rice: true, three: true, ..base }),
            ("rot 4096 trans 2048 predicted rice three", PoseOptions { rotation: 4096.0, translation: 2048.0, rice: true, three: true, ..base }),
        ] {
            println!("  pose {name:48}: {:6.2} KB/s", pose_rate(&frames, delay, o));
        }
        let d = delay as usize;
        println!("  header trimmed (rot 32768, pos 1024)            : {:6.2} KB/s", header_rate(&frames, d, 32768.0, 1024.0, false));
        println!("  header trimmed predicted                         : {:6.2} KB/s", header_rate(&frames, d, 32768.0, 1024.0, true));
        for (units, every) in [(4096.0, 1), (1024.0, 1), (256.0, 1), (1024.0, 2), (256.0, 2)] {
            println!("  audio floats 1/{units} every {every}                  : {:6.2} KB/s", audio_rate(&frames, d, units, every));
        }
    }
}
