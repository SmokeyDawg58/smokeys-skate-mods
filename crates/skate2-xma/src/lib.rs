// SPDX-License-Identifier: LGPL-2.1-or-later
//
//! XMA decoding for the Skate 2 port.
//!
//! The Xbox 360 decoded XMA in hardware, so the game executable has no decoder to port. This
//! crate is a Rust port of the WMA Pro decoder in FFmpeg's libavcodec (wmaprodec.c and its
//! tables), restricted to XMA2 streams of one or two channels. It is licensed under the GNU
//! Lesser General Public License 2.1 or later, like the FFmpeg code it derives from; the rest
//! of the workspace is MIT.
//!
//! A stream is fed its 2048-byte packets in order with [`XmaStream::decode_packet`] and
//! returns 512-sample frames of floats in [-1, 1).

mod bits;
mod decoder;
mod mdct;
mod tables;
mod vlc;

pub use decoder::{Error, PACKET_BYTES, SAMPLES_PER_FRAME, XmaStream};
