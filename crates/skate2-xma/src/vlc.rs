// Copyright (c) 2000-2002 Fabrice Bellard, 2002-2004 Michael Niedermayer
// SPDX-License-Identifier: LGPL-2.1-or-later
//
// Variable length codes built from code lengths in tree order, as FFmpeg's
// `ff_vlc_init_from_lengths` (libavcodec/vlc.c) assigns them: the first code is all zeros
// and every later code is the previous one plus one unit of the previous length.

use crate::bits::BitReader;

const PRIMARY_BITS: u32 = 10;

#[derive(Clone, Copy)]
struct Code {
    code: u32,
    len: u8,
    symbol: i32,
}

pub(crate) struct Vlc {
    codes: Vec<Code>,
    /// Indexed by the next `PRIMARY_BITS` bits: the code index + 1, or 0 for a longer code.
    primary: Vec<u16>,
}

impl Vlc {
    pub(crate) fn from_lengths(entries: impl Iterator<Item = (u8, i32)>, offset: i32) -> Self {
        let mut codes = Vec::new();
        let mut code = 0u64;
        for (len, symbol) in entries {
            if len == 0 {
                continue;
            }
            codes.push(Code {
                code: code as u32,
                len,
                symbol: symbol + offset,
            });
            code += 1u64 << (32 - u32::from(len));
        }
        let mut primary = vec![0u16; 1 << PRIMARY_BITS];
        for (index, entry) in codes.iter().enumerate() {
            if u32::from(entry.len) <= PRIMARY_BITS {
                let first = (entry.code >> (32 - PRIMARY_BITS)) as usize;
                let span = 1usize << (PRIMARY_BITS - u32::from(entry.len));
                for slot in &mut primary[first..first + span] {
                    *slot = (index + 1) as u16;
                }
            }
        }
        Self { codes, primary }
    }

    /// `get_vlc2`: the symbol, or -1 without consuming bits for a code outside the tree.
    pub(crate) fn decode(&self, reader: &mut BitReader<'_>) -> i32 {
        let peek = reader.show_bits32();
        let slot = self.primary[(peek >> (32 - PRIMARY_BITS)) as usize];
        let index = if slot != 0 {
            usize::from(slot - 1)
        } else {
            match self.codes.partition_point(|entry| entry.code <= peek) {
                0 => return -1,
                after => after - 1,
            }
        };
        let entry = self.codes[index];
        let span = 1u64 << (32 - u32::from(entry.len));
        if u64::from(peek) >= u64::from(entry.code) + span {
            return -1;
        }
        reader.skip_bits(usize::from(entry.len));
        entry.symbol
    }
}
