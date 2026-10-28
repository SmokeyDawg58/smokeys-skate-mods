// SPDX-License-Identifier: LGPL-2.1-or-later
//! Decodes raw 2048-byte XMA packets of one stream to interleaved little-endian f32.
//!
//!     xma_decode <packets> <output.f32> <channels> <decoder table 0..3>

use skate2_xma::{PACKET_BYTES, XmaStream};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let packets = std::fs::read(&args[1]).expect("packets");
    let channels: usize = args[3].parse().expect("channels");
    let table: u32 = args[4].parse().expect("table");
    let mut stream = XmaStream::new(channels, table).expect("stream");
    let mut out = Vec::new();
    let mut emit = |frame: &[&[f32]]| {
        for i in 0..frame[0].len() {
            for channel in frame {
                out.extend_from_slice(&channel[i].to_le_bytes());
            }
        }
    };
    for packet in packets.chunks(PACKET_BYTES) {
        let mut padded = [0u8; PACKET_BYTES];
        padded[..packet.len()].copy_from_slice(packet);
        stream.decode_packet(&padded, &mut emit);
    }
    stream.drain(&mut emit);
    std::fs::write(&args[2], out).expect("write");
}
