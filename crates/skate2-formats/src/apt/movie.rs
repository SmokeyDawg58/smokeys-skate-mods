use super::{AptError, Matrix, Reader, Rect};

#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
    String(String),
    Undefined,
    Register(u32),
    Boolean(bool),
    Float(f32),
    Integer(i32),
    Lookup(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstantFile {
    pub root_offset: usize,
    pub constants: Vec<Constant>,
}
impl ConstantFile {
    /// `AptLoader::CompleteLoad`82a1cbe0 and `_parseStream`829f6bc8.
    pub fn parse(bytes: &[u8]) -> Result<Self, AptError> {
        let r = Reader(bytes);
        if r.bytes(0, 20)? != b"Apt constant file\x1a\0\0" {
            return Err(AptError::Magic("CONST"));
        }
        let root_offset = r.offset(20)?;
        let count = r.offset(24)?;
        let table = r.offset(28)?;
        let mut constants = Vec::new();
        for i in r.array(table, count, 8)? {
            let o = table + i * 8;
            let value = r.u32(o + 4)?;
            constants.push(match r.u32(o)? {
                1 => Constant::String(r.string(value as usize)?.to_owned()),
                3 => Constant::Undefined,
                4 => Constant::Register(value),
                5 => Constant::Boolean(value != 0),
                6 => Constant::Float(f32::from_bits(value)),
                7 => Constant::Integer(value as i32),
                8 => Constant::Lookup(value),
                value => {
                    return Err(AptError::Unsupported {
                        offset: o,
                        kind: "constant",
                        value,
                    });
                }
            });
        }
        Ok(Self {
            root_offset,
            constants,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Import {
    pub file: String,
    pub name: String,
    pub character_id: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Export {
    pub name: String,
    pub character_id: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Movie {
    pub frames: Vec<Frame>,
}
impl Movie {
    pub fn label_frame(&self, label: &str) -> Option<usize> {
        self.frames.iter().position(|frame| {
            frame
                .controls
                .iter()
                .any(|control| matches!(control, Control::Label(value) if value.eq_ignore_ascii_case(label)))
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub controls: Vec<Control>,
}

/// Fields only replace existing display-list values when their flag is set.
/// Keeping the mask is essential: a move-only frame does not select character0.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub flags: u32,
    pub depth: i32,
    pub character_id: u32,
    pub matrix: Matrix,
    pub colour_scale: u32,
    pub colour_bias: u32,
    pub ratio: f32,
    pub name: Option<String>,
    pub clip_depth: i32,
    pub events: Vec<EventAction>,
    pub blend_mode: Option<i32>,
}
impl Placement {
    pub const MOVE: u32 = 1;
    pub const CHARACTER: u32 = 2;
    pub const MATRIX: u32 = 4;
    pub const COLOUR: u32 = 8;
    pub const RATIO: u32 = 16;
    pub const NAME: u32 = 32;
    pub const CLIP: u32 = 64;
    pub const EVENTS: u32 = 128;
}

/// Offset into original APT bytes. Executing a script is a separate service;
/// parsing it must never silently substitute a no-op for unknown behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionStream {
    pub offset: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct EventAction {
    pub triggers: u32,
    pub key_code: u32,
    pub actions: ActionStream,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Control {
    Action(ActionStream),
    InitAction {
        character_id: u32,
        actions: ActionStream,
    },
    Label(String),
    Place(Placement),
    Remove {
        depth: i32,
    },
    Background {
        colour: u32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub bounds: Rect,
    pub font_id: u32,
    pub alignment: u32,
    pub colour: u32,
    pub height: f32,
    pub read_only: bool,
    pub multiline: bool,
    pub word_wrap: bool,
    pub initial_text: String,
    pub variable: String,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Character {
    Shape { bounds: Rect },
    Text(Text),
    Font { name: String, glyph_ids: Vec<u32> },
    Sprite(Movie),
    Bitmap { texture_id: u32 },
    Animation(Movie),
}
impl Character {
    pub fn movie(&self) -> Option<&Movie> {
        match self {
            Self::Sprite(movie) | Self::Animation(movie) => Some(movie),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    pub width: u32,
    pub height: u32,
    pub milliseconds_per_frame: u32,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    /// Empty slots are imported characters, not locally drawable objects.
    pub characters: Vec<Option<Character>>,
    pub constants: ConstantFile,
}
impl Animation {
    /// The shipped APT1:6:4 / APT1:7:4 / APT1:8:4 representations,
    /// source82a1beb0/82a00a58.
    /// Unsupported character/control/filter kinds fail with their location;
    /// they are never omitted from the resulting movie.
    pub fn parse(bytes: &[u8], constant_bytes: &[u8]) -> Result<Self, AptError> {
        let r = Reader(bytes);
        // Phone assets use 1:7:4, trickdisplay2 1:8:4 and the challenge
        // banners (ownthespot, spotfound) 1:6:4. CompleteLoad 82a1cbe0 passes
        // every file to the same Resolve/Fixup layouts and nothing reads the
        // version (the XEX's only "Apt Data" bytes are an unreferenced
        // embedded movie at 82ff5500). Keep rejecting unknown formats.
        if !matches!(r.bytes(0, 16)?, b"Apt Data:1:6:4\x1a\0" | b"Apt Data:1:7:4\x1a\0" | b"Apt Data:1:8:4\x1a\0") {
            return Err(AptError::Magic("APT"));
        }
        let constants = ConstantFile::parse(constant_bytes)?;
        let root = constants.root_offset;
        if root == 0 {
            return Err(AptError::NullTable);
        }
        r.bytes(root, 68)?;
        if r.u32(root)? != 9 {
            return Err(AptError::Invalid {
                offset: root,
                field: "root type",
                value: r.u32(root)?,
            });
        }
        let count = r.offset(root + 28)?;
        let table = r.offset(root + 32)?;
        let mut characters = Vec::new();
        for i in r.array(table, count, 4)? {
            let o = r.offset(table + i * 4)?;
            characters.push(if o == 0 {
                None
            } else {
                Some(read_character(r, o)?)
            });
        }
        if count == 0 || r.offset(table)? != root {
            return Err(AptError::Invalid {
                offset: table,
                field: "root character slot",
                value: root as u32,
            });
        }
        let import_count = r.offset(root + 48)?;
        let import_table = r.offset(root + 52)?;
        let mut imports = Vec::new();
        for i in r.array(import_table, import_count, 16)? {
            let o = import_table + i * 16;
            let character_id = r.u32(o + 8)?;
            validate_id(character_id, count, o + 8)?;
            imports.push(Import {
                file: r.pointed_string(o)?.into(),
                name: r.pointed_string(o + 4)?.into(),
                character_id,
            });
        }
        let export_count = r.offset(root + 56)?;
        let export_table = r.offset(root + 60)?;
        let mut exports = Vec::new();
        for i in r.array(export_table, export_count, 8)? {
            let o = export_table + i * 8;
            let character_id = r.u32(o + 4)?;
            validate_id(character_id, count, o + 4)?;
            exports.push(Export {
                name: r.pointed_string(o)?.into(),
                character_id,
            });
        }
        Ok(Self {
            width: r.u32(root + 36)?,
            height: r.u32(root + 40)?,
            milliseconds_per_frame: r.u32(root + 44)?,
            imports,
            exports,
            characters,
            constants,
        })
    }
}

fn validate_id(id: u32, count: usize, offset: usize) -> Result<(), AptError> {
    if (id as usize) < count {
        Ok(())
    } else {
        Err(AptError::Invalid {
            offset,
            field: "character id",
            value: id,
        })
    }
}

fn read_character(r: Reader<'_>, o: usize) -> Result<Character, AptError> {
    r.bytes(o, 16)?;
    let p = o + 16;
    Ok(match r.u32(o)? {
        1 => Character::Shape { bounds: r.rect(p)? },
        2 => Character::Text(Text {
            bounds: r.rect(p)?,
            font_id: r.u32(p + 16)?,
            alignment: r.u32(p + 20)?,
            colour: r.u32(p + 24)?,
            height: r.f32(p + 28)?,
            read_only: r.u32(p + 32)? != 0,
            multiline: r.u32(p + 36)? != 0,
            word_wrap: r.u32(p + 40)? != 0,
            initial_text: r.pointed_string(p + 44)?.into(),
            variable: r.pointed_string(p + 48)?.into(),
        }),
        3 => {
            let count = r.offset(p + 4)?;
            let table = r.offset(p + 8)?;
            let mut glyph_ids = Vec::new();
            for i in r.array(table, count, 4)? {
                glyph_ids.push(r.u32(table + i * 4)?);
            }
            Character::Font {
                name: r.pointed_string(p)?.into(),
                glyph_ids,
            }
        }
        5 => Character::Sprite(read_movie(r, p)?),
        7 => Character::Bitmap {
            texture_id: r.u32(p)?,
        },
        9 => Character::Animation(read_movie(r, p)?),
        value => {
            return Err(AptError::Unsupported {
                offset: o,
                kind: "character",
                value,
            });
        }
    })
}

fn read_movie(r: Reader<'_>, o: usize) -> Result<Movie, AptError> {
    let count = r.offset(o)?;
    let table = r.offset(o + 4)?;
    let mut frames = Vec::new();
    for i in r.array(table, count, 8)? {
        let f = table + i * 8;
        let count = r.offset(f)?;
        let controls_table = r.offset(f + 4)?;
        let mut controls = Vec::new();
        for j in r.array(controls_table, count, 4)? {
            let c = r.offset(controls_table + j * 4)?;
            if c == 0 {
                return Err(AptError::NullTable);
            }
            controls.push(match r.u32(c)? {
                1 => Control::Action(read_action(r, c + 4)?),
                2 => Control::Label(r.pointed_string(c + 4)?.into()),
                3 => Control::Place(read_placement(r, c + 4, false)?),
                4 => Control::Remove {
                    depth: r.u32(c + 4)? as i32,
                },
                5 => Control::Background {
                    colour: r.u32(c + 4)?,
                },
                8 => Control::InitAction {
                    character_id: r.u32(c + 4)?,
                    actions: read_action(r, c + 8)?,
                },
                9 => Control::Place(read_placement(r, c + 4, true)?),
                value => {
                    return Err(AptError::Unsupported {
                        offset: c,
                        kind: "control",
                        value,
                    });
                }
            });
        }
        frames.push(Frame { controls });
    }
    Ok(Movie { frames })
}

fn read_action(r: Reader<'_>, o: usize) -> Result<ActionStream, AptError> {
    let offset = r.offset(o)?;
    if offset == 0 {
        return Err(AptError::NullTable);
    }
    r.bytes(offset, 1)?;
    Ok(ActionStream { offset })
}

fn read_placement(r: Reader<'_>, o: usize, version3: bool) -> Result<Placement, AptError> {
    r.bytes(o, if version3 { 72 } else { 60 })?;
    let flags = r.u32(o)?;
    let name = r.offset(o + 48)?;
    let event_set = r.offset(o + 56)?;
    let mut events = Vec::new();
    if event_set != 0 {
        let count = r.offset(event_set)?;
        let table = r.offset(event_set + 4)?;
        for i in r.array(table, count, 12)? {
            let e = table + i * 12;
            events.push(EventAction {
                triggers: r.u32(e)?,
                key_code: r.u32(e + 4)?,
                actions: read_action(r, e + 8)?,
            });
        }
    }
    let blend_mode = if version3 {
        let filters = r.u32(o + 64)?;
        if filters != 0 {
            return Err(AptError::Unsupported {
                offset: o + 64,
                kind: "placement filters",
                value: filters,
            });
        }
        Some(r.u32(o + 60)? as i32)
    } else {
        None
    };
    Ok(Placement {
        flags,
        depth: r.u32(o + 4)? as i32,
        character_id: r.u32(o + 8)?,
        matrix: r.matrix(o + 12)?,
        colour_scale: r.u32(o + 36)?,
        colour_bias: r.u32(o + 40)?,
        ratio: r.f32(o + 44)?,
        name: if name == 0 {
            None
        } else {
            Some(r.string(name)?.into())
        },
        clip_depth: r.u32(o + 52)? as i32,
        events,
        blend_mode,
    })
}
