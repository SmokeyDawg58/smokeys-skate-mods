//! Selected original input.cfg gameplay expressions, IDs74..95, plus the
//! PlayerUI session marker states52/53.
//! Domain expressions are compiled to a typed tree, not guest instructions.
//! Device polling, retained pad production and relative-path mounting belong
//! to the caller; a missing pad is never synthesized as an empty device.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected input owners await platform polling and Actor scheduling"
    )
)]
use crate::controller_input::{ControllerControl, TryMappedControllerInput};
use std::fmt;

/// Evaluates each control against the same retained map and current pad.
/// Neither construction nor sampling polls hardware or copies input history.
pub(crate) struct MappedPadInput<'a, P> {
    pub(crate) map: &'a SelectedInputMap,
    pub(crate) pad: &'a mut P,
}
impl<P: InputMapPadValues> TryMappedControllerInput for MappedPadInput<'_, P> {
    type Error = InputMapReadError<P::Error>;
    fn try_mapped_value(&mut self, control: ControllerControl) -> Result<f32, Self::Error> {
        self.map.try_value(u32::from(control as u8), self.pad)
    }
}

#[path = "input_map_parser.rs"]
mod parser;
pub(crate) use parser::InputMapParseError;

const NAMES: [&str; 22] = [
    "GP_LStickX",
    "GP_LStickY",
    "GP_LStickIn",
    "GP_RStickX",
    "GP_RStickY",
    "GP_RStickIn",
    "GP_LTrigger",
    "GP_RTrigger",
    "GP_LBumper",
    "GP_RBumper",
    "GP_UDPad",
    "GP_DDPad",
    "GP_LDPad",
    "GP_RDPad",
    "GP_XFace",
    "GP_YFace",
    "GP_AFace",
    "GP_BFace",
    "GP_LStickRawX",
    "GP_LStickRawY",
    "GP_SensorZ",
    "GP_SensorX",
];
const EVENT_NAMES: [&str; 5] = [
    "DebugPrev",
    "DebugNext",
    "DebugPause",
    "DebugReplaceSkateboard",
    "DebugResetWorld",
];
/// `kIS_SessionMarkerSet`52 and `kIS_SessionMarkerUse`53.
const SESSION_MARKER_NAMES: [&str; 3] = [
    "SessionMarkerSet",
    "SessionMarkerUse",
    "TopLeftShoulderHeld",
];

/// Actual current cInputPad+4, then selected retained tButton.Value. Each
/// expression operand independently samples count and value in that order.
pub(crate) trait InputMapPadValues {
    type Error;
    fn num_buttons(&mut self) -> Result<i32, Self::Error>;
    fn value(&mut self, index: usize) -> Result<f32, Self::Error>;
    fn held(&mut self, index: usize) -> Result<bool, Self::Error>;
    fn pressed(&mut self, index: usize) -> Result<bool, Self::Error>;
    fn released(&mut self, index: usize) -> Result<bool, Self::Error>;
    fn repeat(&mut self, index: usize) -> Result<bool, Self::Error>;
}

#[derive(Debug)]
enum Expression {
    Literal(f32),
    Button(usize),
    Held(usize),
    Pressed(usize),
    Released(usize),
    Repeat(usize),
    Subtract(Box<Self>, Box<Self>),
    Negate(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
}
impl Expression {
    fn evaluate<P: InputMapPadValues>(
        &self,
        pad: &mut P,
    ) -> Result<f32, InputMapReadError<P::Error>> {
        use InputMapReadError as E;
        match self {
            Self::Literal(value) => Ok(*value),
            Self::Button(index) | Self::Held(index) | Self::Pressed(index) | Self::Released(index) | Self::Repeat(index) => {
                let count = pad.num_buttons().map_err(E::Pad)?;
                if count == 0 {
                    return Ok(0.0);
                }
                if usize::try_from(count).map_or(true, |count| *index >= count) {
                    return Err(E::ButtonRange {
                        index: *index,
                        count,
                    });
                }
                // GetButton82988fe8: held/pressed modifiers are 0/1 floats.
                match self {
                    Self::Pressed(_) => pad.pressed(*index),
                    Self::Released(_) => pad.released(*index),
                    Self::Repeat(_) => pad.repeat(*index),
                    Self::Held(_) => pad.held(*index),
                    _ => return pad.value(*index).map_err(E::Pad),
                }
                .map(|set| if set { 1.0 } else { 0.0 })
                .map_err(E::Pad)
            }
            Self::Subtract(left, right) => {
                let left = left.evaluate(pad)?;
                let right = right.evaluate(pad)?;
                Ok(left - right)
            }
            Self::Negate(value) => Ok(-value.evaluate(pad)?),
            // Execute8298ba00 opcode5: the right register is read only for a
            // nonzero left; the result is 1 or 0 (NaN counts as nonzero).
            Self::Or(left, right) => {
                if left.evaluate(pad)? != 0.0 { Ok(1.0) }
                else { Ok(if right.evaluate(pad)? != 0.0 { 1.0 } else { 0.0 }) }
            }
            Self::And(left, right) => {
                if left.evaluate(pad)? == 0.0 {
                    return Ok(0.0);
                }
                Ok(if right.evaluate(pad)? == 0.0 {
                    0.0
                } else {
                    1.0
                })
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct SelectedInputMap {
    expressions: [Expression; 22],
    input_events: Option<[Expression; 5]>,
    session_markers: Option<[Expression; 3]>,
    frontend: Option<[Expression; 34]>,
}
impl SelectedInputMap {
    /// Selected original named definitions only; unrelated expression bodies
    /// are retained by the scanner but not evaluated. No global arguments are
    /// consumed by this supported grammar; a selected config reference refuses.
    pub(crate) fn parse(text: &str) -> Result<Self, InputMapParseError> {
        parser::parse(text, false, false)
    }
    /// Adds source CompileMap59..63 (826de030..78), requiring all five
    /// authored debug definitions. Missing controls remain parser failures.
    pub(crate) fn parse_with_input_events(text: &str) -> Result<Self, InputMapParseError> {
        parser::parse(text, true, false)
    }
    /// The production map: debug events plus the PlayerUI session marker
    /// states52/53 read by `UpdateSessionMarker`828af3f0.
    pub(crate) fn parse_with_player_ui_inputs(text: &str) -> Result<Self, InputMapParseError> {
        parser::parse(text, true, true)
    }
    /// Original LoadInputMap826dde58 AptStart/Select/Next/Previous/Up/Down.
    pub(crate) fn parse_with_frontend_inputs(text: &str) -> Result<Self, InputMapParseError> {
        let mut map = Self::parse_with_player_ui_inputs(text)?;
        map.frontend = Some(parser::parse_frontend(text)?);
        Ok(map)
    }
    pub(crate) fn try_value<P: InputMapPadValues>(
        &self,
        id: u32,
        pad: &mut P,
    ) -> Result<f32, InputMapReadError<P::Error>> {
        if let Some(index) = [134, 136, 137, 146, 147, 148, 149, 205, 35, 139, 144, 141, 138, 164, 165, 150, 151, 152, 153, 166,
            204, 206, 207, 140, 143, 145, 187, 167, 168, 169, 182, 135, 185, 188].iter().position(|state| *state == id) {
            return self.frontend.as_ref().ok_or(InputMapReadError::UnsupportedMap(id))?[index].evaluate(pad);
        }
        // LoadInputMap826dde58 compiles TopLeftShoulderHeld as29.
        // PhonelistHUD::Show826c0580 reads this retained SIM-map state.
        if id == 29 {
            return self
                .session_markers
                .as_ref()
                .ok_or(InputMapReadError::UnsupportedMap(id))?[2]
                .evaluate(pad);
        }
        if (52..=53).contains(&id) {
            return self
                .session_markers
                .as_ref()
                .ok_or(InputMapReadError::UnsupportedMap(id))?
                [usize::try_from(id - 52).expect("two map identities")]
            .evaluate(pad);
        }
        if (59..=63).contains(&id) {
            return self
                .input_events
                .as_ref()
                .ok_or(InputMapReadError::UnsupportedMap(id))?
                [usize::try_from(id - 59).expect("five map identities")]
            .evaluate(pad);
        }
        let offset = id
            .checked_sub(74)
            .ok_or(InputMapReadError::UnsupportedMap(id))?;
        let expression = self
            .expressions
            .get(usize::try_from(offset).map_err(|_| InputMapReadError::UnsupportedMap(id))?)
            .ok_or(InputMapReadError::UnsupportedMap(id))?;
        expression.evaluate(pad)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InputMapReadError<E> {
    UnsupportedMap(u32),
    ButtonRange { index: usize, count: i32 },
    Pad(E),
}
impl<E: fmt::Display> fmt::Display for InputMapReadError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMap(id) => {
                write!(f, "input map {id} is outside selected gameplay mappings")
            }
            Self::ButtonRange { index, count } => {
                write!(f, "input pad button {index} outside retained count {count}")
            }
            Self::Pad(error) => write!(f, "input pad: {error}"),
        }
    }
}
#[cfg(test)]
#[path = "input_map_tests.rs"]
pub(crate) mod tests;

/// Const view of a retained group pad: `cInputMap::GetState` only reads it.
pub(crate) struct RetainedPadView<'a>(pub(crate) &'a crate::input_pad::InputPad);
impl InputMapPadValues for RetainedPadView<'_> {
    type Error = crate::input_pad::InputPadError;
    fn pressed(&mut self, index: usize) -> Result<bool, Self::Error> {
        self.0
            .pressed(i32::try_from(index).expect("parser bounds button index below154"))
    }
    fn released(&mut self, index: usize) -> Result<bool, Self::Error> {
        self.0.released(i32::try_from(index).expect("parser bounds button index below154"))
    }
    fn repeat(&mut self, index: usize) -> Result<bool, Self::Error> {
        self.0.repeat(i32::try_from(index).expect("parser bounds button index below154"))
    }
    fn held(&mut self, index: usize) -> Result<bool, Self::Error> {
        self.0
            .held(i32::try_from(index).expect("parser bounds button index below154"))
    }
    fn num_buttons(&mut self) -> Result<i32, Self::Error> {
        Ok(self.0.num_buttons())
    }
    fn value(&mut self, index: usize) -> Result<f32, Self::Error> {
        self.0
            .value(i32::try_from(index).expect("selected button identity"))
    }
}

impl InputMapPadValues for crate::input_pad::InputPad {
    type Error = crate::input_pad::InputPadError;
    fn pressed(&mut self, index: usize) -> Result<bool, Self::Error> {
        Self::pressed(
            self,
            i32::try_from(index).expect("parser bounds button index below154"),
        )
    }
    fn released(&mut self, index: usize) -> Result<bool, Self::Error> {
        Self::released(self, i32::try_from(index).expect("parser bounds button index below154"))
    }
    fn repeat(&mut self, index: usize) -> Result<bool, Self::Error> {
        Self::repeat(self, i32::try_from(index).expect("parser bounds button index below154"))
    }
    fn held(&mut self, index: usize) -> Result<bool, Self::Error> {
        Self::held(
            self,
            i32::try_from(index).expect("parser bounds button index below154"),
        )
    }
    fn num_buttons(&mut self) -> Result<i32, Self::Error> {
        Ok(crate::input_pad::InputPad::num_buttons(self))
    }
    fn value(&mut self, index: usize) -> Result<f32, Self::Error> {
        // Selected compiled Button identities are at most153; conversion cannot fail.
        crate::input_pad::InputPad::value(
            self,
            i32::try_from(index).expect("selected button identity"),
        )
    }
}
