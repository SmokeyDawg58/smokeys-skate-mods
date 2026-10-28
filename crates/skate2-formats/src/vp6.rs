//! On2 VP6 video decoding, ported from the decoder linked into
//! `sk82_na_m.xex` (`VP6_DecodeFrameToYUV_internal` `0x82f10318`), which
//! `rw::movie::VideoDecoder_Vp6::Decode` (`0x82b986a8`) runs for every movie
//! frame. That decoder reads no filter header: prediction is bilinear
//! (`FilterBlock_Xenon` `0x82f16828` is called with bicubic off) and there is
//! no loop filter. Frames are reconstructed in coded row order, the first
//! coded row on top; the movies store the picture bottom row first, which
//! [`Vp6Decoder::copy_display_frame`] undoes.

mod idct;
mod tables;

use std::fmt;
use tables::{
    AC_UPDATE_PROBS, BASELINE_XMITTED_PROBS, COEFF_TO_BAND, COEFF_TO_HUFF_BAND, DC_NODE_EQS,
    DC_QUANT, DC_UPDATE_PROBS, DEFAULT_INTERLACED_SCAN_BANDS, DEFAULT_IS_SHORT_PROBS,
    DEFAULT_MV_LONG_PROBS, DEFAULT_MV_SHORT_PROBS, DEFAULT_SCAN_BANDS, DEFAULT_SIGN_PROBS,
    HUFF_TOKEN_MIN_VAL, MODE_TO_FRAME, MODE_USES_MC, MODE_VQ, MV_UPDATE_PROBS, NEAR_MACRO_BLOCKS,
    Q_THRESH, SCAN_BAND_UPDATE_PROBS, TOKEN_EXTRA_BITS, ZERO_RUN_PROB_DEFAULTS, ZRL_UPDATE_PROBS,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Vp6Error {
    /// A frame shorter than its header.
    Truncated,
    /// An inter frame before the first key frame.
    MissingKeyFrame,
    /// A key frame with zero macroblock rows or columns.
    InvalidDimensions { rows: u8, columns: u8 },
}

impl fmt::Display for Vp6Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid VP6 frame: {self:?}")
    }
}

impl std::error::Error for Vp6Error {}

/// `MVBorder` (`UpdateUMVBorder` `0x82f12cd0`): the replicated luma border;
/// the chroma border is half of it.
const BORDER: usize = 48;

/// `CODING_MODE`.
mod mode {
    pub const INTER_NO_MV: u8 = 0;
    pub const INTRA: u8 = 1;
    pub const INTER_PLUS_MV: u8 = 2;
    pub const INTER_NEAREST_MV: u8 = 3;
    pub const INTER_NEAR_MV: u8 = 4;
    pub const USING_GOLDEN: u8 = 5;
    pub const GOLDEN_MV: u8 = 6;
    pub const INTER_FOURMV: u8 = 7;
    pub const GOLD_NEAREST_MV: u8 = 8;
    pub const GOLD_NEAR_MV: u8 = 9;
}

/// The token after the DC and AC values (`DCT_EOB_TOKEN`).
const EOB_TOKEN: u8 = 11;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MotionVector {
    x: i16,
    y: i16,
}

/// `BOOL_CODER` with `VP6_StartDecode` (`0x82f14478`), `VP6_DecodeBool`
/// (`0x82f14828`) and `VP6_DecodeBool128` (`0x82f148b8`). The equiprobable
/// read always shifts once, so its range can reach 256. Bytes past the frame
/// read as zero.
struct BoolCoder<'a> {
    buffer: &'a [u8],
    pos: usize,
    value: u32,
    range: u32,
    count: u32,
}

impl<'a> BoolCoder<'a> {
    fn new(buffer: &'a [u8], start: usize) -> Self {
        let byte = |i: usize| u32::from(buffer.get(start + i).copied().unwrap_or(0));
        Self {
            buffer,
            pos: start + 4,
            value: (byte(0) << 24) | (byte(1) << 16) | (byte(2) << 8) | byte(3),
            range: 255,
            count: 8,
        }
    }

    fn next_byte(&mut self) -> u32 {
        let byte = self.buffer.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        u32::from(byte)
    }

    fn bool(&mut self, probability: u8) -> bool {
        let split = (((self.range - 1) * u32::from(probability)) >> 8) + 1;
        let mut range = split;
        let bit = self.value >= split << 24;
        if bit {
            range = self.range - split;
            self.value -= split << 24;
        }
        while range < 0x80 {
            self.count -= 1;
            range <<= 1;
            self.value <<= 1;
            if self.count == 0 {
                self.count = 8;
                self.value |= self.next_byte();
            }
        }
        self.range = range;
        bit
    }

    fn bool128(&mut self) -> bool {
        let mut split = (self.range + 1) >> 1;
        let bit = self.value >= split << 24;
        if bit {
            self.value -= split << 24;
            split = self.range - split;
        }
        self.range = split << 1;
        self.value <<= 1;
        self.count -= 1;
        if self.count == 0 {
            self.count = 8;
            self.value |= self.next_byte();
        }
        bit
    }

    /// `n` equiprobable bits, most significant first.
    fn bits(&mut self, n: u32) -> u32 {
        (0..n).fold(0, |value, _| (value << 1) | u32::from(self.bool128()))
    }

    /// A seven-bit probability update: `2 * value`, or 1 for zero.
    fn probability7(&mut self) -> u8 {
        let value = (self.bits(7) as u8) << 1;
        if value == 0 { 1 } else { value }
    }
}

/// `_BITREADER` (`br3`): the Huffman partition, read most significant bit
/// first in big-endian words. Bits past the frame read as zero.
struct BitReader<'a> {
    buffer: &'a [u8],
    bit: usize,
}

impl BitReader<'_> {
    fn read(&mut self, n: u32) -> u32 {
        let mut value = 0;
        for _ in 0..n {
            let byte = self.buffer.get(self.bit >> 3).copied().unwrap_or(0);
            value = (value << 1) | u32::from((byte >> (7 - (self.bit & 7))) & 1);
            self.bit += 1;
        }
        value
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HuffRef {
    Leaf(u8),
    Node(u8),
}

/// `_huffnode` trees from `VP6_BuildHuffTree` (`0x82f19a48`); node 0 is the
/// root, a 0 bit takes the left branch.
#[derive(Clone, Debug)]
struct HuffTree {
    nodes: Vec<[HuffRef; 2]>,
}

impl Default for HuffTree {
    fn default() -> Self {
        Self {
            nodes: vec![[HuffRef::Leaf(0), HuffRef::Leaf(0)]],
        }
    }
}

struct SortNode {
    next: Option<usize>,
    freq: u32,
    value: HuffRef,
}

/// `InsertSorted` (`0x82f199b0`): before the first node whose frequency is
/// not lower.
fn insert_sorted(nodes: &mut [SortNode], node: usize, start: &mut Option<usize>) {
    let mut next = *start;
    let mut previous = *start;
    if let Some(mut current) = *start {
        loop {
            if nodes[node].freq <= nodes[current].freq {
                next = Some(current);
                break;
            }
            previous = Some(current);
            next = nodes[current].next;
            match next {
                Some(following) => current = following,
                None => break,
            }
        }
    }
    if next == *start {
        *start = Some(node);
    } else if let Some(previous) = previous {
        nodes[previous].next = Some(node);
    }
    nodes[node].next = next;
}

impl HuffTree {
    /// `VP6_BuildHuffTree`: zero counts become 1, then the two lowest
    /// frequencies merge until one node is left; internal nodes are numbered
    /// down from `values - 2`.
    #[expect(clippy::cast_possible_truncation, reason = "at most twelve symbols")]
    fn build(counts: &[u32]) -> Self {
        let values = counts.len();
        let mut sort: Vec<SortNode> = counts
            .iter()
            .enumerate()
            .map(|(i, &count)| SortNode {
                next: None,
                freq: count.max(1),
                value: HuffRef::Leaf(i as u8),
            })
            .collect();
        let mut start = Some(0);
        for node in 1..values {
            insert_sorted(&mut sort, node, &mut start);
        }
        let mut nodes = vec![[HuffRef::Leaf(0), HuffRef::Leaf(0)]; values - 1];
        let mut internal = values - 1;
        while let Some(low) = start {
            let Some(next) = sort[low].next else { break };
            internal -= 1;
            let freq = sort[low].freq + sort[next].freq;
            nodes[internal] = [sort[low].value, sort[next].value];
            start = sort[next].next;
            sort.push(SortNode {
                next: None,
                freq,
                value: HuffRef::Node(internal as u8),
            });
            let pushed = sort.len() - 1;
            insert_sorted(&mut sort, pushed, &mut start);
        }
        Self { nodes }
    }

    fn decode(&self, reader: &mut BitReader<'_>) -> u8 {
        let mut node = 0usize;
        loop {
            match self.nodes[node][reader.read(1) as usize] {
                HuffRef::Leaf(token) => return token,
                HuffRef::Node(next) => node = usize::from(next),
            }
        }
    }
}

/// `BoolTreeToHuffCodes` (`0x82f130a0`): the twelve token frequencies of an
/// eleven-node token tree.
fn token_frequencies(p: &[u8; 11]) -> [u32; 12] {
    let p = p.map(u32::from);
    let mut f = [0u32; 12];
    f[11] = (p[0] * p[1]) >> 8;
    f[0] = ((255 - p[1]) * p[0]) >> 8;
    let v2 = 255 - p[0];
    f[1] = (p[2] * v2) >> 8;
    let v3 = ((255 - p[2]) * v2) >> 8;
    let v4 = (p[3] * v3) >> 8;
    f[2] = (p[4] * v4) >> 8;
    let v5 = ((255 - p[4]) * v4) >> 8;
    f[3] = (p[5] * v5) >> 8;
    f[4] = ((255 - p[5]) * v5) >> 8;
    let v6 = ((255 - p[3]) * v3) >> 8;
    let v7 = (p[6] * v6) >> 8;
    f[5] = (p[7] * v7) >> 8;
    f[6] = ((255 - p[7]) * v7) >> 8;
    let v8 = ((255 - p[6]) * v6) >> 8;
    let v9 = (p[8] * v8) >> 8;
    f[7] = (p[9] * v9) >> 8;
    f[8] = ((255 - p[9]) * v9) >> 8;
    let v10 = ((255 - p[8]) * v8) >> 8;
    f[9] = (p[10] * v10) >> 8;
    f[10] = ((255 - p[10]) * v10) >> 8;
    f
}

/// `ZerosBoolTreeToHuffCodes` (`0x82f13200`): the nine zero-run
/// frequencies (runs 1 to 8 and the escape).
fn run_frequencies(p: &[u8; 14]) -> [u32; 9] {
    let p = p.map(u32::from);
    let mut f = [0u32; 9];
    let v2 = (p[0] * p[1]) >> 8;
    f[0] = (p[2] * v2) >> 8;
    f[1] = ((255 - p[2]) * v2) >> 8;
    let v3 = ((255 - p[1]) * p[0]) >> 8;
    f[2] = (p[3] * v3) >> 8;
    f[3] = ((255 - p[3]) * v3) >> 8;
    let v4 = ((((255 - p[0]) * p[4]) >> 8) * p[5]) >> 8;
    f[4] = (p[6] * v4) >> 8;
    f[5] = ((255 - p[6]) * v4) >> 8;
    let v5 = ((((255 - p[0]) * p[4]) >> 8) * (255 - p[5])) >> 8;
    f[6] = (p[7] * v5) >> 8;
    f[7] = ((255 - p[7]) * v5) >> 8;
    f[8] = ((255 - p[0]) * (255 - p[4])) >> 8;
    f
}

/// `BLOCK_CONTEXT`: the token, reference frame and DC of the last block in
/// a row (left) or column (above). The mode field is never read by the
/// decoder and is not kept.
#[derive(Clone, Copy, Debug)]
struct BlockContext {
    token: u8,
    frame: u16,
    dc: i16,
}

impl Default for BlockContext {
    /// `VP6_ResetLeftContext` (`0x82f138d0`) / `VP6_ResetAboveContext`.
    fn default() -> Self {
        Self {
            token: 0,
            frame: 4,
            dc: 0,
        }
    }
}

/// Which context entry a block uses.
#[derive(Clone, Copy)]
enum ContextRef {
    LeftY(usize),
    LeftU,
    LeftV,
    AboveY(usize),
    AboveU(usize),
    AboveV(usize),
}

/// The geometry `VP6_InitFrameDetails` (`0x82f10eb0`) derives from a key
/// frame's macroblock rows and columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    width: usize,
    height: usize,
    y_stride: usize,
    uv_stride: usize,
    u_offset: usize,
    v_offset: usize,
    frame_size: usize,
    h_fragments: usize,
    mb_rows: usize,
    mb_cols: usize,
}

impl Layout {
    fn new(rows: usize, columns: usize) -> Self {
        let width = columns * 16;
        let height = rows * 16;
        let y_stride = width + 2 * BORDER;
        let y_size = (height + 2 * BORDER) * y_stride;
        Self {
            width,
            height,
            y_stride,
            uv_stride: y_stride / 2,
            u_offset: y_size,
            v_offset: y_size + y_size / 4,
            frame_size: y_size + y_size / 2,
            h_fragments: width / 8,
            mb_rows: rows + 6,
            mb_cols: columns + 6,
        }
    }
}

/// `QUANTIZER` after `VP6_UpdateQ` (`0x82f113e0`): DC quantizer at 0, AC
/// quantizer elsewhere, both four times the table value.
#[derive(Clone, Debug)]
struct Quantizer {
    index: Option<u8>,
    coefficients: [i16; 64],
}

impl Quantizer {
    #[expect(clippy::cast_possible_wrap, reason = "table values are at most 94")]
    fn update(&mut self, index: u8) {
        if self.index == Some(index) {
            return;
        }
        self.index = Some(index);
        let i = usize::from(index);
        self.coefficients = [(Q_THRESH[i] * 4) as i16; 64];
        self.coefficients[0] = (DC_QUANT[i] * 4) as i16;
    }
}

/// `_MACROBLOCK_INFO`: the macroblock being decoded.
#[derive(Clone, Debug)]
struct MacroBlock {
    mode: u8,
    block_mode: [u8; 6],
    mv: [MotionVector; 6],
    nearest_inter: MotionVector,
    near_inter: MotionVector,
    nearest_mv_index: usize,
    nearest_gold: MotionVector,
    near_gold: MotionVector,
    nearest_gold_index: usize,
    interlaced: bool,
    coefficients: [[i16; 64]; 6],
}

impl Default for MacroBlock {
    fn default() -> Self {
        Self {
            mode: mode::INTRA,
            block_mode: [mode::INTRA; 6],
            mv: [MotionVector::default(); 6],
            nearest_inter: MotionVector::default(),
            near_inter: MotionVector::default(),
            nearest_mv_index: 12,
            nearest_gold: MotionVector::default(),
            near_gold: MotionVector::default(),
            nearest_gold_index: 12,
            interlaced: false,
            coefficients: [[0; 64]; 6],
        }
    }
}

/// The entropy and prediction state that persists between frames.
#[derive(Clone, Debug)]
struct Model {
    interlaced: bool,
    prob_interlaced: u8,
    dc_probs: [[u8; 11]; 2],
    /// `AcProbs` by plane type, token type, band.
    ac_probs: [[[[u8; 11]; 6]; 3]; 2],
    zero_run_probs: [[u8; 14]; 2],
    dc_node_contexts: [[[u8; 5]; 3]; 2],
    scan_bands: [u8; 64],
    /// `ModifiedScanOrder` followed by `EobOffsetTable`.
    scan_order: [u8; 128],
    prob_xmitted: [[[u8; 10]; 2]; 3],
    prob_mode_same: [[u8; 10]; 3],
    prob_mode: [[[u8; 9]; 10]; 3],
    is_mv_short_prob: [u8; 2],
    mv_sign_probs: [u8; 2],
    mv_short_probs: [[u8; 7]; 2],
    mv_size_probs: [[u8; 8]; 2],
    dc_trees: [HuffTree; 2],
    /// `AcHuffTree` by token type, plane type, band.
    ac_trees: [[[HuffTree; 6]; 2]; 3],
    zero_trees: [HuffTree; 2],
}

impl Default for Model {
    fn default() -> Self {
        Self {
            interlaced: false,
            prob_interlaced: 0,
            dc_probs: [[0; 11]; 2],
            ac_probs: [[[[0; 11]; 6]; 3]; 2],
            zero_run_probs: ZERO_RUN_PROB_DEFAULTS,
            dc_node_contexts: [[[0; 5]; 3]; 2],
            scan_bands: DEFAULT_SCAN_BANDS,
            scan_order: [0; 128],
            prob_xmitted: BASELINE_XMITTED_PROBS,
            prob_mode_same: [[0; 10]; 3],
            prob_mode: [[[0; 9]; 10]; 3],
            is_mv_short_prob: DEFAULT_IS_SHORT_PROBS,
            mv_sign_probs: DEFAULT_SIGN_PROBS,
            mv_short_probs: DEFAULT_MV_SHORT_PROBS,
            mv_size_probs: DEFAULT_MV_LONG_PROBS,
            dc_trees: Default::default(),
            ac_trees: Default::default(),
            zero_trees: Default::default(),
        }
    }
}

/// The coefficient partition of a frame (`br2` or `br3`).
enum Coefficients<'a> {
    Bool(BoolCoder<'a>),
    Huffman(BitReader<'a>),
}

/// The per-frame and per-macroblock decoding state.
#[derive(Clone, Debug, Default)]
struct State {
    model: Model,
    quantizer: Quantizer,
    key_frame: bool,
    refresh_golden: bool,
    last_mode: u8,
    prediction_mode: Vec<u8>,
    mb_motion_vector: Vec<MotionVector>,
    above_y: Vec<BlockContext>,
    above_u: Vec<BlockContext>,
    above_v: Vec<BlockContext>,
    left_y: [BlockContext; 2],
    left_u: BlockContext,
    left_v: BlockContext,
    last_dc: [[i16; 4]; 3],
    dc_run: [u32; 2],
    ac1_run: [u32; 2],
    mbi: MacroBlock,
    near_offsets: [isize; 12],
}

impl Default for Quantizer {
    fn default() -> Self {
        Self {
            index: None,
            coefficients: [0; 64],
        }
    }
}

/// A decoded VP6 picture as planar 4:2:0 bytes in display order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct YuvFrame {
    pub width: usize,
    pub height: usize,
    pub y: Vec<u8>,
    pub u: Vec<u8>,
    pub v: Vec<u8>,
}

/// The VP6 decoder instance (`PB_INSTANCE`).
#[derive(Clone, Debug, Default)]
pub struct Vp6Decoder {
    layout: Option<Layout>,
    frames: [Vec<u8>; 3],
    this_frame: usize,
    last_frame: usize,
    golden_frame: usize,
    other_frame: Option<usize>,
    state: State,
}

impl Vp6Decoder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            this_frame: 0,
            last_frame: 1,
            golden_frame: 2,
            other_frame: None,
            ..Self::default()
        }
    }

    /// The coded picture size, known after the first key frame.
    #[must_use]
    pub fn dimensions(&self) -> Option<(usize, usize)> {
        self.layout.map(|layout| (layout.width, layout.height))
    }

    /// `VP6_DecodeFrameToYUV_internal` (`0x82f10318`): one frame, then the
    /// buffer swap, `UpdateUMVBorder` and the golden frame refresh.
    ///
    /// # Errors
    ///
    /// Returns [`Vp6Error`] for a truncated header, an inter frame before
    /// any key frame or a key frame without macroblocks.
    pub fn decode(&mut self, frame: &[u8]) -> Result<(), Vp6Error> {
        let mut header = HeaderBits::new(frame)?;
        let inter = header.read(1) == 1;
        let quantizer = header.read(6) as u8;
        header.read(1);
        let (mut br, coefficient_offset) = if inter {
            if self.layout.is_none() {
                return Err(Vp6Error::MissingKeyFrame);
            }
            let br = BoolCoder::new(frame, 3);
            let offset = header.read(16) as usize;
            (br, offset)
        } else {
            header.read(5);
            header.read(2);
            self.state.model.interlaced = header.read(1) == 1;
            let mut br = BoolCoder::new(frame, 4);
            let offset = header.read(16) as usize;
            let rows = br.bits(8) as u8;
            let columns = br.bits(8) as u8;
            br.bits(8);
            br.bits(8);
            br.bits(2);
            if rows == 0 || columns == 0 {
                return Err(Vp6Error::InvalidDimensions { rows, columns });
            }
            let layout = Layout::new(usize::from(rows), usize::from(columns));
            if self.layout != Some(layout) {
                self.init_frame_details(layout);
            }
            (br, offset)
        };
        self.state.key_frame = !inter;
        self.state.refresh_golden = inter && br.bool(128);
        let use_huffman = br.bool(128);
        self.state.quantizer.update(quantizer);
        let partition = frame.get(coefficient_offset..).unwrap_or(&[]);
        let mut coefficients = if use_huffman {
            Coefficients::Huffman(BitReader {
                buffer: partition,
                bit: 0,
            })
        } else {
            Coefficients::Bool(BoolCoder::new(partition, 0))
        };
        let layout = self.layout.ok_or(Vp6Error::MissingKeyFrame)?;
        let mut this = std::mem::take(&mut self.frames[self.this_frame]);
        let last = &self.frames[self.last_frame];
        let golden = &self.frames[self.golden_frame];
        self.state.decode_frame_mbs(
            &layout,
            &mut br,
            &mut coefficients,
            use_huffman,
            &mut this,
            last,
            golden,
        );
        self.frames[self.this_frame] = this;
        let old_last = self.last_frame;
        self.last_frame = self.this_frame;
        self.this_frame = self.other_frame.take().unwrap_or(old_last);
        update_umv_border(&layout, &mut self.frames[self.last_frame]);
        if !inter || self.state.refresh_golden {
            self.other_frame = Some(self.golden_frame);
            self.golden_frame = self.last_frame;
        }
        Ok(())
    }

    /// `VP6_InitFrameDetails`: new buffers and per-macroblock arrays.
    fn init_frame_details(&mut self, layout: Layout) {
        self.layout = Some(layout);
        for frame in &mut self.frames {
            *frame = vec![0; layout.frame_size];
        }
        self.this_frame = 0;
        self.last_frame = 1;
        self.golden_frame = 2;
        self.other_frame = None;
        let blocks = layout.mb_rows * layout.mb_cols;
        self.state.prediction_mode = vec![mode::INTRA; blocks];
        self.state.mb_motion_vector = vec![MotionVector::default(); blocks];
        self.state.above_y = vec![BlockContext::default(); layout.h_fragments + 8];
        self.state.above_u = vec![BlockContext::default(); (layout.h_fragments >> 1) + 8];
        self.state.above_v = vec![BlockContext::default(); (layout.h_fragments >> 1) + 8];
        #[expect(
            clippy::cast_possible_wrap,
            reason = "macroblock column counts are small"
        )]
        let cols = layout.mb_cols as isize;
        self.state.near_offsets = NEAR_MACRO_BLOCKS.map(|(row, col)| row * cols + col);
    }

    /// `VP6_GetYUVConfig` (`0x82f10248`) and `TransferYUVBuffer`
    /// (`0x82b97d58`): the last decoded picture without its border, with the
    /// rows put back in display order.
    #[must_use]
    pub fn copy_display_frame(&self) -> Option<YuvFrame> {
        let layout = self.layout?;
        let frame = &self.frames[self.last_frame];
        let plane = |offset: usize, stride: usize, border: usize, width: usize, height: usize| {
            let mut out = Vec::with_capacity(width * height);
            for row in (0..height).rev() {
                let start = offset + (border + row) * stride + border;
                out.extend_from_slice(&frame[start..start + width]);
            }
            out
        };
        Some(YuvFrame {
            width: layout.width,
            height: layout.height,
            y: plane(0, layout.y_stride, BORDER, layout.width, layout.height),
            u: plane(
                layout.u_offset,
                layout.uv_stride,
                BORDER / 2,
                layout.width / 2,
                layout.height / 2,
            ),
            v: plane(
                layout.v_offset,
                layout.uv_stride,
                BORDER / 2,
                layout.width / 2,
                layout.height / 2,
            ),
        })
    }
}

/// `FRAME_HEADER` with `InitHeaderBuffer` (`0x82f144d0`) and
/// `ReadHeaderBits` (`0x82f14518`): the raw bits at the start of a frame.
struct HeaderBits<'a> {
    buffer: &'a [u8],
    bit: usize,
}

impl<'a> HeaderBits<'a> {
    fn new(buffer: &'a [u8]) -> Result<Self, Vp6Error> {
        if buffer.len() < 8 {
            return Err(Vp6Error::Truncated);
        }
        Ok(Self { buffer, bit: 0 })
    }

    fn read(&mut self, n: u32) -> u32 {
        let mut reader = BitReader {
            buffer: self.buffer,
            bit: self.bit,
        };
        let value = reader.read(n);
        self.bit = reader.bit;
        value
    }
}

/// `UpdateUMVBorder` (`0x82f12cd0`): edge pixels copied 48 luma and 24
/// chroma pixels outwards, sideways first, then the outer rows.
fn update_umv_border(layout: &Layout, frame: &mut [u8]) {
    let planes = [
        (0, layout.y_stride, BORDER, layout.width, layout.height),
        (
            layout.u_offset,
            layout.uv_stride,
            BORDER / 2,
            layout.width / 2,
            layout.height / 2,
        ),
        (
            layout.v_offset,
            layout.uv_stride,
            BORDER / 2,
            layout.width / 2,
            layout.height / 2,
        ),
    ];
    for (offset, stride, border, width, height) in planes {
        for row in 0..height {
            let start = offset + (border + row) * stride;
            let left = frame[start + border];
            let right = frame[start + border + width - 1];
            frame[start..start + border].fill(left);
            frame[start + border + width..start + 2 * border + width].fill(right);
        }
        let top = offset + border * stride;
        let bottom = offset + (border + height - 1) * stride;
        for row in 0..border {
            frame.copy_within(top..top + stride, offset + row * stride);
            frame.copy_within(bottom..bottom + stride, bottom + (row + 1) * stride);
        }
    }
}

/// The plane a block belongs to and where its context and pixels are.
#[derive(Clone, Copy)]
struct BlockPosition {
    plane_type: usize,
    plane: usize,
    above: ContextRef,
    left: ContextRef,
    recon: usize,
    frame_stride: usize,
    current_stride: usize,
    mv_shift: u32,
    mv_mask: i32,
}

impl State {
    fn context(&self, which: ContextRef) -> BlockContext {
        match which {
            ContextRef::LeftY(i) => self.left_y[i],
            ContextRef::LeftU => self.left_u,
            ContextRef::LeftV => self.left_v,
            ContextRef::AboveY(i) => self.above_y[i],
            ContextRef::AboveU(i) => self.above_u[i],
            ContextRef::AboveV(i) => self.above_v[i],
        }
    }

    fn context_mut(&mut self, which: ContextRef) -> &mut BlockContext {
        match which {
            ContextRef::LeftY(i) => &mut self.left_y[i],
            ContextRef::LeftU => &mut self.left_u,
            ContextRef::LeftV => &mut self.left_v,
            ContextRef::AboveY(i) => &mut self.above_y[i],
            ContextRef::AboveU(i) => &mut self.above_u[i],
            ContextRef::AboveV(i) => &mut self.above_v[i],
        }
    }

    /// `VP6_DecodeFrameMbs` (`0x82f14040`).
    #[expect(clippy::too_many_arguments, reason = "the native frame state")]
    fn decode_frame_mbs(
        &mut self,
        layout: &Layout,
        br: &mut BoolCoder<'_>,
        coefficients: &mut Coefficients<'_>,
        use_huffman: bool,
        this: &mut [u8],
        last: &[u8],
        golden: &[u8],
    ) {
        if self.key_frame {
            self.model.prob_xmitted = BASELINE_XMITTED_PROBS;
            self.model.is_mv_short_prob = DEFAULT_IS_SHORT_PROBS;
            self.model.mv_short_probs = DEFAULT_MV_SHORT_PROBS;
            self.model.mv_sign_probs = DEFAULT_SIGN_PROBS;
            self.model.mv_size_probs = DEFAULT_MV_LONG_PROBS;
            self.prediction_mode.fill(mode::INTRA);
            self.model.scan_bands = if self.model.interlaced {
                DEFAULT_INTERLACED_SCAN_BANDS
            } else {
                DEFAULT_SCAN_BANDS
            };
            self.build_scan_order();
        } else {
            self.decode_mode_probs(br);
            self.configure_mv_entropy_decoder(br);
            self.last_mode = mode::INTER_NO_MV;
        }
        self.configure_entropy_decoder(br);
        if use_huffman {
            self.convert_bool_trees();
        }
        if self.model.interlaced {
            self.model.prob_interlaced = br.bits(8) as u8;
        }
        self.reset_above_context();
        self.mbi.coefficients = [[0; 64]; 6];
        self.dc_run = [0; 2];
        self.ac1_run = [0; 2];
        for row in 3..layout.mb_rows - 3 {
            self.left_y = [BlockContext::default(); 2];
            self.left_u = BlockContext::default();
            self.left_v = BlockContext::default();
            for col in 3..layout.mb_cols - 3 {
                self.decode_macro_block(layout, br, coefficients, row, col, this, last, golden);
            }
        }
    }

    /// `BuildScanOrder` (`0x82f12fe0`): positions by band, then each entry's
    /// running maximum plus one (`EobOffsetTable`).
    fn build_scan_order(&mut self) {
        let mut order = [0u8; 128];
        let mut next = 1;
        for band in 0..16 {
            for position in 1..64u8 {
                if self.model.scan_bands[usize::from(position)] == band {
                    order[next] = position;
                    next += 1;
                }
            }
        }
        let mut max = 0;
        for i in 0..64 {
            max = max.max(order[i]);
            order[64 + i] = max + 1;
        }
        self.model.scan_order = order;
    }

    /// The byte at `ModifiedScanOrder + index`. Indices past the 128 bytes
    /// of the scan order read the fields behind it in `PB_INSTANCE`:
    /// `ScanBands`, then the big-endian `mvNearOffset` words.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the bytes of a native word"
    )]
    fn scan_tail(&self, index: usize) -> u8 {
        match index {
            0..128 => self.model.scan_order[index],
            128..192 => self.model.scan_bands[index - 128],
            _ => {
                let word = index - 192;
                let offset = self.near_offsets.get(word / 4).copied().unwrap_or(0) as i32;
                offset.to_be_bytes()[word % 4]
            }
        }
    }

    /// `VP6_DecodeModeProbs` (`0x82f1a708`) and `VP6_BuildModeTree`
    /// (`0x82f1a350`).
    fn decode_mode_probs(&mut self, br: &mut BoolCoder<'_>) {
        for ctx in 0..3 {
            if br.bool(174) {
                let entry = &MODE_VQ[ctx][br.bits(4) as usize];
                for mode in 0..10 {
                    self.model.prob_xmitted[ctx][1][mode] = entry[2 * mode];
                    self.model.prob_xmitted[ctx][0][mode] = entry[2 * mode + 1];
                }
            }
            if br.bool(254) {
                for mode in 0..10 {
                    for half in [1, 0] {
                        let value = i32::from(self.model.prob_xmitted[ctx][half][mode])
                            + decode_mode_diff(br);
                        self.model.prob_xmitted[ctx][half][mode] = value.clamp(0, 255) as u8;
                    }
                }
            }
        }
        for ctx in 0..3 {
            for last in 0..10 {
                let same = u32::from(self.model.prob_xmitted[ctx][1][last]);
                let other = u32::from(self.model.prob_xmitted[ctx][0][last]);
                self.model.prob_mode_same[ctx][last] =
                    (255 - (255 * same) / (1 + same + other)) as u8;
                let p: [u32; 10] = std::array::from_fn(|m| {
                    if m == last {
                        0
                    } else {
                        100 * u32::from(self.model.prob_xmitted[ctx][0][m])
                    }
                });
                let p02 = p[0] + p[2];
                let p34 = p[3] + p[4];
                let p0234 = p02 + p34;
                let p17 = p[1] + p[7];
                let p56 = p[5] + p[6];
                let p89 = p[8] + p[9];
                let p5689 = p56 + p89;
                let p156789 = p17 + p5689;
                let probability = |num: u32, den: u32| (255 * num / (1 + den) + 1) as u8;
                self.model.prob_mode[ctx][last] = [
                    probability(p0234, p0234 + p156789),
                    probability(p02, p0234),
                    probability(p17, p156789),
                    probability(p[0], p02),
                    probability(p[3], p34),
                    probability(p[1], p17),
                    probability(p56, p5689),
                    probability(p[5], p56),
                    probability(p[8], p89),
                ];
            }
        }
    }

    /// `VP6_ConfigureMvEntropyDecoder` (`0x82f1a178`).
    fn configure_mv_entropy_decoder(&mut self, br: &mut BoolCoder<'_>) {
        for comp in 0..2 {
            if br.bool(MV_UPDATE_PROBS[comp][0]) {
                self.model.is_mv_short_prob[comp] = br.probability7();
            }
            if br.bool(MV_UPDATE_PROBS[comp][1]) {
                self.model.mv_sign_probs[comp] = br.probability7();
            }
        }
        for comp in 0..2 {
            for node in 0..7 {
                if br.bool(MV_UPDATE_PROBS[comp][2 + node]) {
                    self.model.mv_short_probs[comp][node] = br.probability7();
                }
            }
        }
        for comp in 0..2 {
            for node in 0..8 {
                if br.bool(MV_UPDATE_PROBS[comp][9 + node]) {
                    self.model.mv_size_probs[comp][node] = br.probability7();
                }
            }
        }
    }

    /// `VP6_ConfigureEntropyDecoder` (`0x82f135e8`) and
    /// `VP6_ConfigureContexts` (`0x82f19fd8`).
    fn configure_entropy_decoder(&mut self, br: &mut BoolCoder<'_>) {
        let mut last = [128u8; 11];
        for plane in 0..2 {
            for node in 0..11 {
                if br.bool(DC_UPDATE_PROBS[plane][node]) {
                    last[node] = br.probability7();
                    self.model.dc_probs[plane][node] = last[node];
                } else if self.key_frame {
                    self.model.dc_probs[plane][node] = last[node];
                }
            }
        }
        if self.key_frame {
            self.model.zero_run_probs = ZERO_RUN_PROB_DEFAULTS;
        }
        if br.bool(128) {
            for position in 1..64 {
                if br.bool(SCAN_BAND_UPDATE_PROBS[position]) {
                    self.model.scan_bands[position] = br.bits(4) as u8;
                }
            }
            self.build_scan_order();
        }
        for table in 0..2 {
            for node in 0..14 {
                if br.bool(ZRL_UPDATE_PROBS[table][node]) {
                    self.model.zero_run_probs[table][node] = br.probability7();
                }
            }
        }
        for token in 0..3 {
            for plane in 0..2 {
                for band in 0..6 {
                    for node in 0..11 {
                        if br.bool(AC_UPDATE_PROBS[token][plane][band][node]) {
                            last[node] = br.probability7();
                            self.model.ac_probs[plane][token][band][node] = last[node];
                        } else if self.key_frame {
                            self.model.ac_probs[plane][token][band][node] = last[node];
                        }
                    }
                }
            }
        }
        for plane in 0..2 {
            for ctx in 0..3 {
                for node in 0..5 {
                    let [scale, offset] = DC_NODE_EQS[node][ctx];
                    let value =
                        ((scale * i32::from(self.model.dc_probs[plane][node]) + 128) >> 8) + offset;
                    self.model.dc_node_contexts[plane][ctx][node] = value.clamp(1, 255) as u8;
                }
            }
        }
    }

    /// `ConvertBoolTrees` (`0x82f13318`).
    fn convert_bool_trees(&mut self) {
        for plane in 0..2 {
            self.model.dc_trees[plane] =
                HuffTree::build(&token_frequencies(&self.model.dc_probs[plane]));
            self.model.zero_trees[plane] =
                HuffTree::build(&run_frequencies(&self.model.zero_run_probs[plane]));
        }
        for token in 0..3 {
            for plane in 0..2 {
                for band in 0..6 {
                    self.model.ac_trees[token][plane][band] = HuffTree::build(&token_frequencies(
                        &self.model.ac_probs[plane][token][band],
                    ));
                }
            }
        }
    }

    /// `VP6_ResetAboveContext` (`0x82f13960`).
    fn reset_above_context(&mut self) {
        self.above_y.fill(BlockContext::default());
        self.above_u.fill(BlockContext::default());
        self.above_v.fill(BlockContext::default());
        self.last_dc = [[0; 4]; 3];
        self.last_dc[1][0] = 128;
        self.last_dc[2][0] = 128;
    }

    /// `VP6_FindNearestandNextNearest` (`0x82f15b38`): the first non-zero
    /// vector of a neighbour on the same reference frame, then the next
    /// different one; the context is 1 with none, 2 with one, 0 with two.
    fn find_nearest(&mut self, layout: &Layout, row: usize, col: usize, frame: u16) -> usize {
        let index = row * layout.mb_cols + col;
        let neighbour = |offset: isize| index.checked_add_signed(offset);
        let mut context = 1;
        let mut nearest = MotionVector::default();
        let mut near = MotionVector::default();
        let mut nearest_index = 12;
        for (i, &offset) in self.near_offsets.iter().enumerate() {
            let Some(n) = neighbour(offset) else { continue };
            if MODE_TO_FRAME[usize::from(self.prediction_mode[n])] == frame
                && self.mb_motion_vector[n] != MotionVector::default()
            {
                nearest = self.mb_motion_vector[n];
                nearest_index = i;
                context = 2;
                break;
            }
        }
        for &offset in self.near_offsets.iter().skip(nearest_index + 1) {
            let Some(n) = neighbour(offset) else { continue };
            let vector = self.mb_motion_vector[n];
            if MODE_TO_FRAME[usize::from(self.prediction_mode[n])] == frame
                && vector != nearest
                && vector != MotionVector::default()
            {
                near = vector;
                context = 0;
                break;
            }
        }
        if frame == 1 {
            self.mbi.nearest_mv_index = nearest_index;
            self.mbi.nearest_inter = nearest;
            self.mbi.near_inter = near;
            context
        } else {
            self.mbi.nearest_gold_index = nearest_index;
            self.mbi.nearest_gold = nearest;
            self.mbi.near_gold = near;
            context
        }
    }

    /// `VP6_DecodeMode` (`0x82f14bb8`).
    fn decode_mode(&self, br: &mut BoolCoder<'_>, context: usize) -> u8 {
        let last = usize::from(self.last_mode);
        if br.bool(self.model.prob_mode_same[context][last]) {
            return self.last_mode;
        }
        let p = &self.model.prob_mode[context][last];
        if br.bool(p[0]) {
            if br.bool(p[2]) {
                if br.bool(p[6]) {
                    if br.bool(p[8]) {
                        mode::GOLD_NEAR_MV
                    } else {
                        mode::GOLD_NEAREST_MV
                    }
                } else if br.bool(p[7]) {
                    mode::GOLDEN_MV
                } else {
                    mode::USING_GOLDEN
                }
            } else if br.bool(p[5]) {
                mode::INTER_FOURMV
            } else {
                mode::INTRA
            }
        } else if br.bool(p[1]) {
            if br.bool(p[4]) {
                mode::INTER_NEAR_MV
            } else {
                mode::INTER_NEAREST_MV
            }
        } else if br.bool(p[3]) {
            mode::INTER_PLUS_MV
        } else {
            mode::INTER_NO_MV
        }
    }

    /// `VP6_decodeMotionVector` (`0x82f15160`): the nearest vector (when it
    /// is one of the first two neighbours) plus a coded difference.
    fn decode_motion_vector(&self, br: &mut BoolCoder<'_>, golden: bool) -> MotionVector {
        let base = if golden {
            if self.mbi.nearest_gold_index < 2 {
                self.mbi.nearest_gold
            } else {
                MotionVector::default()
            }
        } else if self.mbi.nearest_mv_index < 2 {
            self.mbi.nearest_inter
        } else {
            MotionVector::default()
        };
        let mut components = [base.x, base.y];
        for (comp, value) in components.iter_mut().enumerate() {
            let mut delta: i32 = if br.bool(self.model.is_mv_short_prob[comp]) {
                let size = &self.model.mv_size_probs[comp];
                let mut long = 0;
                for bit in [0, 1, 2, 7, 6, 5, 4] {
                    long |= i32::from(br.bool(size[bit])) << bit;
                }
                if long & 0xF0 != 0 {
                    long | i32::from(br.bool(size[3])) << 3
                } else {
                    long | 8
                }
            } else {
                let short = &self.model.mv_short_probs[comp];
                if br.bool(short[0]) {
                    if br.bool(short[4]) {
                        6 + i32::from(br.bool(short[6]))
                    } else {
                        4 + i32::from(br.bool(short[5]))
                    }
                } else if br.bool(short[1]) {
                    2 + i32::from(br.bool(short[3]))
                } else {
                    i32::from(br.bool(short[2]))
                }
            };
            if delta != 0 && br.bool(self.model.mv_sign_probs[comp]) {
                delta = -delta;
            }
            *value = (i32::from(*value) + delta) as i16;
        }
        MotionVector {
            x: components[0],
            y: components[1],
        }
    }

    /// `VP6_DecodeMacroBlock` (`0x82f19368`).
    #[expect(clippy::too_many_arguments, reason = "the native frame state")]
    fn decode_macro_block(
        &mut self,
        layout: &Layout,
        br: &mut BoolCoder<'_>,
        coefficients: &mut Coefficients<'_>,
        row: usize,
        col: usize,
        this: &mut [u8],
        last: &[u8],
        golden: &[u8],
    ) {
        if self.model.interlaced {
            let p = self.model.prob_interlaced;
            let probability = if col > 3 {
                if self.mbi.interlaced {
                    p - (p >> 1)
                } else {
                    (((256 - u32::from(p)) >> 1) + u32::from(p)) as u8
                }
            } else {
                p
            };
            self.mbi.interlaced = br.bool(probability);
        } else {
            self.mbi.interlaced = false;
        }
        if self.key_frame {
            self.mbi.mode = mode::INTRA;
        } else {
            let context = self.find_nearest(layout, row, col, 1);
            let mode = self.decode_mode(br, context);
            let index = row * layout.mb_cols + col;
            self.last_mode = mode;
            self.prediction_mode[index] = mode;
            self.mbi.mode = mode;
            if mode == mode::INTER_FOURMV {
                for block in 0..4 {
                    self.mbi.block_mode[block] = match br.bits(2) {
                        1 => mode::INTER_PLUS_MV,
                        2 => mode::INTER_NEAREST_MV,
                        3 => mode::INTER_NEAR_MV,
                        _ => mode::INTER_NO_MV,
                    };
                }
                self.mbi.block_mode[4] = mode::INTER_FOURMV;
                self.mbi.block_mode[5] = mode::INTER_FOURMV;
                let (mut sum_x, mut sum_y) = (0i32, 0i32);
                for block in 0..4 {
                    let vector = match self.mbi.block_mode[block] {
                        mode::INTER_NEAREST_MV => self.mbi.nearest_inter,
                        mode::INTER_NEAR_MV => self.mbi.near_inter,
                        mode::INTER_PLUS_MV => self.decode_motion_vector(br, false),
                        _ => MotionVector::default(),
                    };
                    sum_x += i32::from(vector.x);
                    sum_y += i32::from(vector.y);
                    self.mbi.mv[block] = vector;
                }
                self.mb_motion_vector[index] = self.mbi.mv[3];
                let chroma = |sum: i32| ((sum + 1 + i32::from(sum >= 0)) >> 2) as i16;
                let vector = MotionVector {
                    x: chroma(sum_x),
                    y: chroma(sum_y),
                };
                self.mbi.mv[4] = vector;
                self.mbi.mv[5] = vector;
            } else {
                let vector = match mode {
                    mode::INTER_PLUS_MV => self.decode_motion_vector(br, false),
                    mode::INTER_NEAREST_MV => self.mbi.nearest_inter,
                    mode::INTER_NEAR_MV => self.mbi.near_inter,
                    mode::GOLDEN_MV => {
                        self.find_nearest(layout, row, col, 2);
                        self.decode_motion_vector(br, true)
                    }
                    mode::GOLD_NEAREST_MV => {
                        self.find_nearest(layout, row, col, 2);
                        self.mbi.nearest_gold
                    }
                    mode::GOLD_NEAR_MV => {
                        self.find_nearest(layout, row, col, 2);
                        self.mbi.near_gold
                    }
                    _ => MotionVector::default(),
                };
                self.mb_motion_vector[index] = vector;
                self.mbi.mv = [vector; 6];
                self.mbi.block_mode = [mode; 6];
            }
        }
        let y_stride = layout.y_stride;
        let (current_stride, lower) = if self.mbi.interlaced {
            (2 * y_stride, 1)
        } else {
            (y_stride, 8)
        };
        let y_recon = 16 * row * y_stride + 16 * col;
        let luma = |above: usize, left: usize, recon: usize| BlockPosition {
            plane_type: 0,
            plane: 0,
            above: ContextRef::AboveY(above),
            left: ContextRef::LeftY(left),
            recon,
            frame_stride: y_stride,
            current_stride,
            mv_shift: 2,
            mv_mask: 3,
        };
        let blocks = [
            luma(2 * col, 0, y_recon),
            luma(2 * col + 1, 0, y_recon + 8),
            luma(2 * col, 1, y_recon + lower * y_stride),
            luma(2 * col + 1, 1, y_recon + lower * y_stride + 8),
        ];
        for (bp, position) in blocks.into_iter().enumerate() {
            self.decode_block(coefficients, bp, position, this, last, golden);
        }
        let uv_stride = layout.uv_stride;
        let chroma =
            |plane: usize, above: ContextRef, left: ContextRef, offset: usize| BlockPosition {
                plane_type: 1,
                plane,
                above,
                left,
                recon: 8 * row * uv_stride + offset + 8 * col,
                frame_stride: uv_stride,
                current_stride: uv_stride,
                mv_shift: 3,
                mv_mask: 7,
            };
        let u = chroma(
            1,
            ContextRef::AboveU(col),
            ContextRef::LeftU,
            layout.u_offset,
        );
        self.decode_block(coefficients, 4, u, this, last, golden);
        let v = chroma(
            2,
            ContextRef::AboveV(col),
            ContextRef::LeftV,
            layout.v_offset,
        );
        self.decode_block(coefficients, 5, v, this, last, golden);
    }

    /// `VP6_DecodeBlock` (`0x82f16be0`): tokens, DC prediction, the
    /// transform `VP6_ReadTokensPredictA` or the Huffman partition selects,
    /// then reconstruction.
    fn decode_block(
        &mut self,
        coefficients: &mut Coefficients<'_>,
        bp: usize,
        position: BlockPosition,
        this: &mut [u8],
        last: &[u8],
        golden: &[u8],
    ) {
        let selector = match coefficients {
            Coefficients::Bool(br2) => self.read_tokens_bool(br2, bp, position),
            Coefficients::Huffman(reader) => {
                self.read_tokens_huffman(reader, bp, position.plane_type)
            }
        };
        self.predict_dc(bp, position);
        let frame = MODE_TO_FRAME[usize::from(self.mbi.mode)];
        let dc = self.mbi.coefficients[bp][0];
        for which in [position.left, position.above] {
            let context = self.context_mut(which);
            context.dc = dc;
            context.frame = frame;
        }
        let quantizers = &self.quantizer.coefficients;
        let block = &mut self.mbi.coefficients[bp];
        let residual = if selector > 10 {
            let residual = idct::full(block, quantizers);
            *block = [0; 64];
            residual
        } else if selector > 1 {
            let residual = idct::ten(block, quantizers);
            for i in (0..12).chain(16..20).chain(24..28).chain(32..33) {
                block[i] = 0;
            }
            residual
        } else {
            let residual = idct::dc_only(block, quantizers);
            block[0] = 0;
            residual
        };
        self.reconstruct(bp, position, &residual, this, last, golden);
    }

    /// `VP6_ReadTokensPredictA` (`0x82f13ac8`): the bool-coded tokens.
    /// Returns the transform selector, `EobOffsetTable[index]` at an end of
    /// block and `EobOffsetTable[index - 1]` past the last position.
    fn read_tokens_bool(
        &mut self,
        br: &mut BoolCoder<'_>,
        bp: usize,
        position: BlockPosition,
    ) -> u8 {
        let plane = position.plane_type;
        let ctx = usize::from(self.context(position.above).token)
            + usize::from(self.context(position.left).token);
        let nodes = self.model.dc_node_contexts[plane][ctx];
        let dc = self.model.dc_probs[plane];
        let mut token_type = 0;
        if br.bool(nodes[0]) {
            self.context_mut(position.left).token = 1;
            self.context_mut(position.above).token = 1;
            let value = if br.bool(nodes[2]) {
                token_type = 2;
                if br.bool(nodes[3]) {
                    let category = category(br, &dc);
                    extra_bits(br, category)
                } else if br.bool(nodes[4]) {
                    3 + i32::from(br.bool(dc[5]))
                } else {
                    2
                }
            } else {
                token_type = 1;
                1
            };
            self.mbi.coefficients[bp][0] = signed(br.bool128(), value);
        } else {
            self.context_mut(position.left).token = 0;
            self.context_mut(position.above).token = 0;
        }
        let mut index = 1usize;
        loop {
            let p = self.model.ac_probs[plane][token_type][usize::from(COEFF_TO_BAND[index])];
            if (index > 1 && token_type == 0) || br.bool(p[0]) {
                let value = if br.bool(p[2]) {
                    token_type = 2;
                    if br.bool(p[3]) {
                        let category = category(br, &p);
                        extra_bits(br, category)
                    } else if br.bool(p[4]) {
                        3 + i32::from(br.bool(p[5]))
                    } else {
                        2
                    }
                } else {
                    token_type = 1;
                    1
                };
                let scan = usize::from(self.model.scan_order[index]);
                self.mbi.coefficients[bp][scan] = signed(br.bool128(), value);
                index += 1;
            } else {
                if !br.bool(p[1]) {
                    return self.scan_tail(64 + index);
                }
                token_type = 0;
                let z = &self.model.zero_run_probs[usize::from(index >= 6)];
                let run = if br.bool(z[0]) {
                    if br.bool(z[4]) {
                        9 + (0..6).fold(0, |run, bit| {
                            run + (usize::from(br.bool(z[8 + bit])) << bit)
                        })
                    } else if br.bool(z[5]) {
                        7 + usize::from(br.bool(z[7]))
                    } else {
                        5 + usize::from(br.bool(z[6]))
                    }
                } else if br.bool(z[1]) {
                    3 + usize::from(br.bool(z[3]))
                } else {
                    1 + usize::from(br.bool(z[2]))
                };
                index += run;
            }
            if index >= 64 {
                return self.scan_tail(64 + index - 1);
            }
        }
    }

    /// The Huffman partition of `VP6_DecodeBlock`, with the zero-DC and
    /// end-of-block-at-the-first-AC runs (`CurrentDcRunLen`,
    /// `CurrentAc1RunLen`) shared by U and V.
    fn read_tokens_huffman(&mut self, reader: &mut BitReader<'_>, bp: usize, plane: usize) -> u8 {
        let mut token_type;
        if self.dc_run[plane] > 0 {
            self.dc_run[plane] -= 1;
            token_type = 0;
        } else {
            let token = self.model.dc_trees[plane].decode(reader);
            if token == EOB_TOKEN {
                return self.model.scan_order[64];
            }
            if token == 0 {
                self.dc_run[plane] = zero_block_run(reader);
                token_type = 0;
            } else {
                let value = huffman_value(reader, token);
                self.mbi.coefficients[bp][0] = signed(reader.read(1) == 1, value);
                token_type = if value > 1 { 2 } else { 1 };
            }
        }
        if self.ac1_run[plane] > 0 {
            self.ac1_run[plane] -= 1;
            return self.model.scan_order[64];
        }
        let mut index = 1usize;
        loop {
            let band = usize::from(COEFF_TO_HUFF_BAND[index]);
            let token = self.model.ac_trees[token_type][plane][band].decode(reader);
            if token == 0 {
                let run = self.model.zero_trees[usize::from(index >= 6)].decode(reader);
                index += if run >= 8 {
                    reader.read(6) as usize + 9
                } else {
                    usize::from(run) + 1
                };
                token_type = 0;
            } else if token == EOB_TOKEN {
                if index == 1 {
                    self.ac1_run[plane] = zero_block_run(reader);
                }
                return self.scan_tail(index + 63);
            } else {
                let value = huffman_value(reader, token);
                let scan = usize::from(self.model.scan_order[index]);
                self.mbi.coefficients[bp][scan] = signed(reader.read(1) == 1, value);
                token_type = if value > 1 { 2 } else { 1 };
                index += 1;
            }
            if index >= 64 {
                return self.scan_tail(index + 63);
            }
        }
    }

    /// `VP6_PredictDC` (`0x82f149c0`): the mean of the left and above DCs on
    /// the same reference frame, else the last DC of that frame.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the native stores a halfword"
    )]
    fn predict_dc(&mut self, bp: usize, position: BlockPosition) {
        let frame = MODE_TO_FRAME[usize::from(self.mbi.mode)];
        let left = self.context(position.left);
        let above = self.context(position.above);
        let mut count = 0;
        let mut dc = 0i32;
        if left.frame == frame {
            count = 1;
            dc = i32::from(left.dc);
        }
        if above.frame == frame {
            dc += i32::from(above.dc);
            count += 1;
        }
        let last = &mut self.last_dc[position.plane];
        if count == 0 {
            dc = i32::from(last[usize::from(frame)]);
        }
        if count == 2 {
            dc = (((dc >> 15) & 1) + dc) >> 1;
        }
        let value = (i32::from(self.mbi.coefficients[bp][0]) + dc) as i16;
        self.mbi.coefficients[bp][0] = value;
        last[usize::from(frame)] = value;
    }

    /// The reconstruction of `VP6_DecodeBlock` (`VP6_ReconstructBlock`
    /// `0x82f15cd0` is the same code): intra blocks add 128, inter blocks
    /// add the residual to the last or golden frame, moved by the block's
    /// vector with bilinear filtering of fractional positions.
    fn reconstruct(
        &self,
        bp: usize,
        position: BlockPosition,
        residual: &[i16; 64],
        this: &mut [u8],
        last: &[u8],
        golden: &[u8],
    ) {
        let mode = self.mbi.mode;
        let dst = position.recon;
        let stride = position.current_stride;
        if mode == mode::INTRA {
            for row in 0..8 {
                for col in 0..8 {
                    let value = i32::from(residual[row * 8 + col]) + 128;
                    this[dst + row * stride + col] = value.clamp(0, 255) as u8;
                }
            }
            return;
        }
        let reference = if MODE_TO_FRAME[usize::from(mode)] == 2 {
            golden
        } else {
            last
        };
        if !MODE_USES_MC[usize::from(mode)] {
            for row in 0..8 {
                for col in 0..8 {
                    let at = dst + row * stride + col;
                    let value = i32::from(reference[at]) + i32::from(residual[row * 8 + col]);
                    this[at] = value.clamp(0, 255) as u8;
                }
            }
            return;
        }
        let vector = self.mbi.mv[bp];
        let (x, y) = (i32::from(vector.x), i32::from(vector.y));
        let mask = position.mv_mask;
        let frame_stride = position.frame_stride;
        let whole = |v: i32| (((v >> 31) & mask) + v) >> position.mv_shift;
        let base = (position.recon as isize)
            + whole(y) as isize * frame_stride as isize
            + whole(x) as isize;
        let mut mod_x = x & mask;
        let mut mod_y = y & mask;
        let mut step = 0isize;
        if mod_x != 0 {
            step = if x > 0 { 1 } else { -1 };
        }
        if mod_y != 0 {
            step += if y > 0 {
                frame_stride as isize
            } else {
                -(frame_stride as isize)
            };
        }
        let mut prediction = [0u16; 64];
        let pixel = |at: isize| {
            u16::from(
                reference
                    .get(usize::try_from(at).unwrap_or(0))
                    .copied()
                    .unwrap_or(0),
            )
        };
        if step == 0 {
            for row in 0..8 {
                for col in 0..8 {
                    prediction[row * 8 + col] = pixel(base + (row * frame_stride + col) as isize);
                }
            }
        } else {
            if bp < 4 {
                mod_x *= 2;
                mod_y *= 2;
            }
            let first = base.min(base + step);
            let difference = step.abs();
            let tap = |a: u16, b: u16, m: i32| {
                ((i32::from(a) * (8 - m) + i32::from(b) * m + 4) >> 3) as u16
            };
            let fs = frame_stride as isize;
            if difference == 1 {
                for row in 0..8isize {
                    for col in 0..8isize {
                        let at = first + row * fs + col;
                        prediction[(row * 8 + col) as usize] = tap(pixel(at), pixel(at + 1), mod_x);
                    }
                }
            } else if difference == fs {
                for row in 0..8isize {
                    for col in 0..8isize {
                        let at = first + row * fs + col;
                        prediction[(row * 8 + col) as usize] =
                            tap(pixel(at), pixel(at + fs), mod_y);
                    }
                }
            } else {
                let source = if difference == fs - 1 {
                    first - 1
                } else {
                    first
                };
                let mut pass = [0u16; 72];
                for row in 0..9isize {
                    for col in 0..8isize {
                        let at = source + row * fs + col;
                        pass[(row * 8 + col) as usize] = tap(pixel(at), pixel(at + 1), mod_x);
                    }
                }
                for row in 0..8 {
                    for col in 0..8 {
                        prediction[row * 8 + col] =
                            tap(pass[row * 8 + col], pass[(row + 1) * 8 + col], mod_y);
                    }
                }
            }
        }
        for row in 0..8 {
            for col in 0..8 {
                let sum = (i32::from(prediction[row * 8 + col])
                    + i32::from(residual[row * 8 + col]))
                .clamp(i32::from(i16::MIN), i32::from(i16::MAX));
                this[dst + row * stride + col] = sum.clamp(0, 255) as u8;
            }
        }
    }
}

/// `VP6_decodeModeDiff` (`0x82f1a5d8`).
fn decode_mode_diff(br: &mut BoolCoder<'_>) -> i32 {
    if !br.bool(205) {
        return 0;
    }
    let sign = if br.bool128() { -1 } else { 1 };
    if br.bool(171) {
        if br.bool(199) {
            4 * br.bits(7) as i32 * sign
        } else if br.bool(140) {
            12 * sign
        } else if br.bool(125) {
            16 * sign
        } else if br.bool(104) {
            20 * sign
        } else {
            24 * sign
        }
    } else {
        sign << (3 - u32::from(br.bool(83)))
    }
}

/// The category tree of a coefficient above four (nodes 6 to 10).
fn category(br: &mut BoolCoder<'_>, p: &[u8; 11]) -> usize {
    if br.bool(p[6]) {
        if br.bool(p[8]) {
            9 + usize::from(br.bool(p[10]))
        } else {
            7 + usize::from(br.bool(p[9]))
        }
    } else {
        5 + usize::from(br.bool(p[7]))
    }
}

/// `VP6_TokenExtraBits2` (`0x823b3b08`): the category's smallest value plus
/// its extra bits, most significant first.
fn extra_bits(br: &mut BoolCoder<'_>, category: usize) -> i32 {
    let (min, top, probs) = &TOKEN_EXTRA_BITS[category - 5];
    (0..=*top).rev().fold(i32::from(*min), |value, bit| {
        value + (i32::from(br.bool(probs[bit])) << bit)
    })
}

/// A Huffman token's value: tokens above four read `token - 4` extra bits,
/// token 10 eleven.
fn huffman_value(reader: &mut BitReader<'_>, token: u8) -> i32 {
    let min = i32::from(HUFF_TOKEN_MIN_VAL[usize::from(token)]);
    match token {
        0..=4 => min,
        5..=9 => min + reader.read(u32::from(token) - 4) as i32,
        _ => min + reader.read(11) as i32,
    }
}

/// The run of following blocks whose DC (or first AC) holds no token.
fn zero_block_run(reader: &mut BitReader<'_>) -> u32 {
    match reader.read(2) {
        2 => 2 + reader.read(2),
        3 => {
            if reader.read(1) == 1 {
                10 + reader.read(6)
            } else {
                6 + reader.read(2)
            }
        }
        value => value,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the native stores a halfword"
)]
fn signed(negative: bool, value: i32) -> i16 {
    (if negative { -value } else { value }) as i16
}

#[cfg(test)]
mod tests;
