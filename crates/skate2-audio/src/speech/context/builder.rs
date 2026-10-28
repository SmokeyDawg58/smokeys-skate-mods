//! The context XML: `rw::core::xml::StateParser` (0x82bb61b0-0x82bb6cd0), and the
//! `ContextFileTyperXMLCallback` / `ContextBuilderXMLCallback` (`contextbuilder.cpp`) that type a
//! file and build its context.

use super::{Condition, Context, Getter, Interface, Sender, Source, Trigger};

/// `StateParser::mStack` and `mAttributes` capacities.
const MAX_DEPTH: usize = 64;
const MAX_ATTRIBUTES: usize = 64;

/// `rw::core::xml::Attribute`.
#[derive(Debug, Clone, Copy)]
struct Attribute<'a> {
    name: &'a [u8],
    value: &'a [u8],
}

/// `rw::core::xml::ICallback` as the context readers use it.
trait Callback {
    fn start_element(&mut self, name: &[u8], attributes: &[Attribute<'_>]);
    fn end_element(&mut self, name: &[u8]);
}

/// The `StateParser` constructor's `" \t\n\r"` (and NUL) white space.
fn is_white(byte: u8) -> bool {
    matches!(byte, 0 | b' ' | b'\t' | b'\n' | b'\r')
}

/// The constructor's alphanumerics (and NUL).
fn is_alphanumeric(byte: u8) -> bool {
    byte == 0 || byte.is_ascii_alphanumeric() || b"_.,%[]:\\-'&+;".contains(&byte)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Begin,
    Element,
    Done,
}

/// `rw::core::xml::StateParser` over NUL-terminated data (the end of the slice reads as NUL).
struct StateParser<'a> {
    data: &'a [u8],
    at: usize,
    state: State,
    stack: Vec<&'a [u8]>,
}

impl<'a> StateParser<'a> {
    fn byte(&self, offset: usize) -> u8 {
        self.data.get(self.at + offset).copied().unwrap_or(0)
    }

    fn starts_with(&self, text: &[u8]) -> bool {
        self.data.get(self.at..).is_some_and(|rest| rest.starts_with(text))
    }

    /// `SkipWhite` 0x82bb61b0.
    fn skip_white(&mut self) {
        while is_white(self.byte(0)) && self.byte(0) != 0 {
            self.at += 1;
        }
    }

    /// Past the next `>`; false at the end of the data.
    fn skip_past_close(&mut self) -> bool {
        while self.byte(0) != b'>' && self.byte(0) != 0 {
            self.at += 1;
        }
        if self.byte(0) == 0 {
            return false;
        }
        self.at += 1;
        true
    }

    /// `SkipWhiteAndComments` 0x82bb6200 (an unterminated comment ends at the end of the data).
    fn skip_white_and_comments(&mut self) {
        loop {
            self.skip_white();
            if !self.starts_with(b"<!--") {
                return;
            }
            loop {
                if !self.skip_past_close() {
                    return;
                }
                if self.data[self.at - 2] == b'-' && self.data[self.at - 3] == b'-' {
                    break;
                }
            }
        }
    }

    /// `ParseBegin` 0x82bb63c0: an `<?xml ... ?>` declaration is skipped.
    fn parse_begin(&mut self) -> bool {
        if self.starts_with(b"<?xml") {
            loop {
                if !self.skip_past_close() {
                    return false;
                }
                if self.data[self.at - 2] == b'?' {
                    break;
                }
            }
        }
        self.state = State::Element;
        true
    }

    /// `CheckCdata` 0x82bb64b8: a `<![CDATA[` section is skipped to its `]]>`.
    fn check_cdata(&mut self) -> bool {
        if !self.starts_with(b"![CDATA[") {
            return false;
        }
        self.at += 8;
        while self.data.len() >= self.at + 3 {
            if self.starts_with(b"]]>") {
                self.at += 3;
                return true;
            }
            self.at += 1;
        }
        true
    }

    /// `HandlePCData` 0x82bb6900: text up to the next tag.
    fn handle_pc_data(&mut self) -> bool {
        while !matches!(self.byte(0), 0 | b'<' | b'>') {
            self.at += 1;
        }
        self.byte(0) != b'>'
    }

    /// `ParseAttributes` 0x82bb65a0: `name = "value"` (or `'value'`) pairs until `>` or `/`.
    fn parse_attributes(&mut self, attributes: &mut Vec<Attribute<'a>>) -> bool {
        let mut result = false;
        loop {
            let name_start = self.at;
            while self.byte(0) != 0 && is_alphanumeric(self.byte(0)) {
                self.at += 1;
            }
            let name = &self.data[name_start..self.at];
            self.skip_white();
            if self.byte(0) != b'=' {
                return result;
            }
            self.at += 1;
            self.skip_white();
            let quote = self.byte(0);
            if quote != b'"' && quote != b'\'' {
                return result;
            }
            self.at += 1;
            let value_start = self.at;
            while self.byte(0) != quote && self.byte(0) != 0 {
                self.at += 1;
            }
            if self.byte(0) != quote {
                return result;
            }
            let value = &self.data[value_start..self.at];
            self.at += 1;
            if attributes.len() == MAX_ATTRIBUTES {
                return result;
            }
            attributes.push(Attribute { name, value });
            self.skip_white();
            result = true;
            if matches!(self.byte(0), b'>' | b'/') {
                return result;
            }
        }
    }

    /// `ParseElement` 0x82bb69b8 and `HandleCloseTag` 0x82bb67f0: a self-closing element starts
    /// and ends at once; closing the outermost element ends the document.
    fn parse_element(&mut self, callback: &mut dyn Callback) -> bool {
        self.skip_white_and_comments();
        if self.byte(0) != b'<' {
            return self.handle_pc_data();
        }
        self.at += 1;
        if self.check_cdata() {
            return true;
        }
        let open = self.byte(0) != b'/';
        if !open {
            self.at += 1;
        }
        let name_start = self.at;
        while self.byte(0) != 0 && !is_white(self.byte(0)) && is_alphanumeric(self.byte(0)) {
            self.at += 1;
        }
        let name = &self.data[name_start..self.at];
        self.skip_white();
        let mut attributes = Vec::new();
        let mut result = true;
        if is_alphanumeric(self.byte(0)) {
            result = open && self.parse_attributes(&mut attributes);
        }
        if self.byte(0) == b'/' && self.byte(1) == b'>' {
            self.at += 2;
            if open {
                callback.start_element(name, &attributes);
                callback.end_element(name);
                return result;
            }
            return false;
        }
        if self.byte(0) != b'>' {
            return false;
        }
        self.at += 1;
        if open {
            if self.stack.len() == MAX_DEPTH {
                return false;
            }
            self.stack.push(name);
            callback.start_element(name, &attributes);
            return true;
        }
        let Some(top) = self.stack.pop() else { return false };
        if !top.starts_with(name) {
            return false;
        }
        callback.end_element(name);
        if self.stack.is_empty() {
            self.state = State::Done;
        }
        true
    }
}

/// `rw::core::xml::Parser::Parse` 0x82bb6cd0 / `StateParser::Parse` 0x82bb6b88.
fn parse(data: &[u8], callback: &mut dyn Callback) {
    let mut parser = StateParser { data, at: 0, state: State::Begin, stack: Vec::new() };
    loop {
        if parser.state == State::Done || parser.byte(0) == 0 {
            return;
        }
        let parsed = match parser.state {
            State::Begin => parser.parse_begin(),
            _ => parser.parse_element(callback),
        };
        if !parsed {
            return;
        }
    }
}

/// `rw::core::stdc::ConvertAToI` 0x82bc3ce0 over a value of at most 127 bytes: past anything
/// before the first `-` or digit, the digits' low word.
fn to_int(value: &[u8]) -> i32 {
    let value = &value[..value.len().min(127)];
    let start = value.iter().position(|&b| b == b'-' || b.is_ascii_digit()).unwrap_or(value.len());
    let mut digits = &value[start..];
    let negative = digits.first() == Some(&b'-');
    if negative {
        digits = &digits[1..];
    }
    let mut number = 0i64;
    for &b in digits.iter().take_while(|b| b.is_ascii_digit()) {
        number = number.wrapping_mul(10).wrapping_add(i64::from(b - b'0'));
    }
    if negative { number.wrapping_neg() as i32 } else { number as i32 }
}

/// `ContextFileTyper::ContextFileType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileType {
    Unknown,
    Context,
    ContextGroup,
}

struct FileTyper(FileType);

impl Callback for FileTyper {
    /// `ContextFileTyperXMLCallback::StartElement` 0x82ab1398: an `AudioFrameworkData` element's
    /// `type` starting `ContextGroup` or `Context`.
    fn start_element(&mut self, name: &[u8], attributes: &[Attribute<'_>]) {
        if name != b"AudioFrameworkData" {
            return;
        }
        for attribute in attributes {
            if attribute.name != b"type" {
                continue;
            }
            if attribute.value.starts_with(b"ContextGroup") {
                self.0 = FileType::ContextGroup;
            } else if attribute.value.starts_with(b"Context") {
                self.0 = FileType::Context;
            }
        }
    }

    fn end_element(&mut self, _name: &[u8]) {}
}

/// `ContextFileTyper::ParseDescription` 0x82ab1500.
pub(super) fn file_type(xml: &[u8]) -> FileType {
    let mut typer = FileTyper(FileType::Unknown);
    parse(xml, &mut typer);
    typer.0
}

/// `ContextBuilderXMLCallback::Elements`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Nothing,
    Trigger,
    Condition,
    Output,
    Follow,
    ParamGetter,
}

/// Which event a `ParamGetter` belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SenderRef {
    Output,
    Follow,
}

/// `ContextBuilderXMLCallback`. A `FollowEvent` (none ships, and nothing sends `kDoFollow`) is
/// read into a sender that is dropped.
struct Builder {
    context: Context,
    mode: Mode,
    param_getter_entry_mode: Mode,
    /// `mCondition`, `mParamGetter` (the getter and its event) and `mFollow`.
    condition: Option<usize>,
    param_getter: Option<(SenderRef, usize)>,
    follow: Option<Sender>,
}

impl Builder {
    /// The constructor 0x82ab50a0's context.
    fn new() -> Self {
        let context = Context {
            name: String::new(),
            priority: 0,
            expiry: 0,
            times_used: 0,
            last_time: 0,
            id: 0,
            interrupt: 0,
            trigger: None,
            conditions: Vec::new(),
            output: None,
            getters: Vec::new(),
        };
        Self { context, mode: Mode::Nothing, param_getter_entry_mode: Mode::Nothing, condition: None, param_getter: None, follow: None }
    }

    fn sender(&mut self, sender: SenderRef) -> Option<&mut Sender> {
        match sender {
            SenderRef::Output => self.context.output.as_mut(),
            SenderRef::Follow => self.follow.as_mut(),
        }
    }

    /// `ParseCsis` 0x82ab1228.
    fn parse_csis(interface: &mut Interface, attributes: &[Attribute<'_>]) {
        for attribute in attributes {
            match attribute.name {
                b"SystemCrc" => interface.system_crc = to_int(attribute.value) as i16,
                b"InterfaceCrc" => interface.interface_crc = to_int(attribute.value) as i16,
                _ => {}
            }
        }
    }

    /// `ParseParamRef` 0x82ab6f88: the trigger parameter `sourceIndex` names, else a local 0
    /// (`CreateLocalParam` 0x82ab6cf0).
    fn parse_param_ref(&self, attributes: &[Attribute<'_>]) -> Source {
        let mut index = -1;
        for attribute in attributes {
            if attribute.name == b"sourceIndex" {
                index = to_int(attribute.value);
            }
        }
        let params = self.context.trigger.as_ref().map_or(0, |trigger| trigger.params.len());
        match usize::try_from(index) {
            Ok(index) if index < params => Source::Trigger(index),
            _ => Source::Local(0),
        }
    }

    /// `ParseParamGetter` 0x82ab70f0: a getter of the event's, kept with the context's local
    /// parameters.
    fn parse_param_getter(&mut self, sender: SenderRef, attributes: &[Attribute<'_>]) {
        let mut getter = Getter { script_id: 0, sources: Vec::new() };
        for attribute in attributes {
            if attribute.name == b"ScriptID" {
                getter.script_id = to_int(attribute.value);
            }
        }
        let index = self.context.getters.len();
        self.context.getters.push(getter);
        self.param_getter = Some((sender, index));
        if let Some(event) = self.sender(sender) {
            event.sources.push(Source::Getter(index));
        }
    }

    /// A source for the element being read (`ParamRef` / `ParamValue`).
    fn add_source(&mut self, source: Source) {
        match self.mode {
            Mode::Condition => {
                if let Some(condition) = self.condition {
                    self.context.conditions[condition].sources.push(source);
                }
            }
            Mode::Output => {
                if let Some(output) = self.context.output.as_mut() {
                    output.sources.push(source);
                }
            }
            Mode::Follow => {
                if let Some(follow) = self.follow.as_mut() {
                    follow.sources.push(source);
                }
            }
            Mode::ParamGetter => {
                if let Some((_, getter)) = self.param_getter {
                    self.context.getters[getter].sources.push(source);
                }
            }
            Mode::Nothing | Mode::Trigger => {}
        }
    }
}

impl Callback for Builder {
    /// `ContextBuilderXMLCallback::StartElement` 0x82ab72d0.
    fn start_element(&mut self, name: &[u8], attributes: &[Attribute<'_>]) {
        match name {
            b"Context" => {
                for attribute in attributes {
                    match attribute.name {
                        b"name" => self.context.name = String::from_utf8_lossy(attribute.value).into_owned(),
                        b"priority" => self.context.priority = to_int(attribute.value),
                        b"interrupt" => self.context.interrupt = to_int(attribute.value),
                        b"id" => self.context.id = to_int(attribute.value),
                        _ => {}
                    }
                }
            }
            b"TriggerEvent" => {
                self.mode = Mode::Trigger;
                let mut trigger = Trigger { interface: Interface::default(), params: Vec::new() };
                for attribute in attributes {
                    match attribute.name {
                        b"name" => trigger.interface.name = Some(String::from_utf8_lossy(attribute.value).into_owned()),
                        b"expiryTime" => self.context.expiry = to_int(attribute.value),
                        _ => {}
                    }
                }
                self.context.trigger = Some(trigger);
            }
            b"Condition" => {
                let mut condition = Condition { script_id: 0, sources: Vec::new() };
                for attribute in attributes {
                    if attribute.name == b"scriptID" {
                        condition.script_id = to_int(attribute.value);
                    }
                }
                self.condition = Some(self.context.conditions.len());
                self.context.conditions.push(condition);
                self.mode = Mode::Condition;
            }
            b"OutputEvent" | b"FollowEvent" => {
                let mut sender = Sender { interface: Interface::default(), sources: Vec::new() };
                for attribute in attributes {
                    if attribute.name == b"name" {
                        sender.interface.name = Some(String::from_utf8_lossy(attribute.value).into_owned());
                    }
                }
                if name == b"OutputEvent" {
                    self.context.output = Some(sender);
                    self.mode = Mode::Output;
                } else {
                    self.follow = Some(sender);
                    self.mode = Mode::Follow;
                }
            }
            b"Csis" => {
                let interface = match self.mode {
                    Mode::Trigger => self.context.trigger.as_mut().map(|trigger| &mut trigger.interface),
                    Mode::Output => self.context.output.as_mut().map(|output| &mut output.interface),
                    Mode::Follow => self.follow.as_mut().map(|follow| &mut follow.interface),
                    _ => None,
                };
                if let Some(interface) = interface {
                    Self::parse_csis(interface, attributes);
                }
            }
            b"ParamGetter" => {
                self.param_getter_entry_mode = self.mode;
                match self.mode {
                    Mode::Output => self.parse_param_getter(SenderRef::Output, attributes),
                    Mode::Follow => self.parse_param_getter(SenderRef::Follow, attributes),
                    _ => {}
                }
                self.mode = Mode::ParamGetter;
            }
            b"Param" => {
                if self.mode == Mode::Trigger
                    && let Some(trigger) = self.context.trigger.as_mut()
                {
                    trigger.params.push(0);
                }
            }
            b"ParamRef" => {
                if matches!(self.mode, Mode::Condition | Mode::Output | Mode::Follow | Mode::ParamGetter) {
                    let source = self.parse_param_ref(attributes);
                    self.add_source(source);
                }
            }
            b"ParamValue" => {
                let mut value = 0;
                for attribute in attributes {
                    if b"value".starts_with(attribute.name) {
                        value = to_int(attribute.value);
                    }
                }
                let wanted = match self.mode {
                    Mode::Condition | Mode::Output | Mode::Follow => true,
                    Mode::ParamGetter => self.context.output.is_some(),
                    Mode::Nothing | Mode::Trigger => false,
                };
                if wanted {
                    self.add_source(Source::Local(value));
                }
            }
            _ => {}
        }
    }

    /// `ContextBuilderXMLCallback::EndElement` 0x82ab1068.
    fn end_element(&mut self, name: &[u8]) {
        match name {
            b"TriggerEvent" | b"OutputEvent" => self.mode = Mode::Nothing,
            b"Condition" => {
                self.mode = Mode::Nothing;
                self.condition = None;
            }
            b"ParamGetter" => self.mode = self.param_getter_entry_mode,
            _ => {}
        }
    }
}

/// `AddContextXml` 0x82ab63c0's build: the context the file describes.
pub(super) fn build_context(xml: &[u8]) -> Context {
    let mut builder = Builder::new();
    parse(xml, &mut builder);
    builder.context
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &[u8] = b"<AudioFrameworkData type=\"Context\" />\r\n\
        <Context name=\"Context702\" priority=\"20\" interrupt=\"0\" id=\"2121541125\">\r\n\
        <TriggerEvent name=\"T_702_40_retry\" expiryTime=\"1000\">\r\n\
        <Csis SystemCrc=\"10889\" InterfaceCrc=\"21221\" />\r\n\
        <Param name=\"filmer\" />\r\n\
        </TriggerEvent>\r\n\
        <ConditionList>\r\n\
        <Condition name=\"NotInReplay\" scriptID=\"4\" />\r\n\
        <Condition name=\"IsSlappy\" scriptID=\"20\">\r\n\
        <ParamRef name=\"filmer\" sourceIndex=\"0\" />\r\n\
        </Condition>\r\n\
        </ConditionList>\r\n\
        <OutputEvent name=\"O_702_40_retry\">\r\n\
        <Csis SystemCrc=\"20444\" InterfaceCrc=\"14866\" />\r\n\
        <ParamGetter ScriptID=\"7\" />\r\n\
        <ParamRef sourceIndex=\"0\" />\r\n\
        <ParamRef sourceIndex=\"3\" />\r\n\
        </OutputEvent>\r\n\
        </Context>\r\n\
        \0\0\0";

    #[test]
    fn a_context_file_builds_its_trigger_conditions_and_output() {
        assert_eq!(file_type(XML), FileType::Context);
        let context = build_context(XML);
        assert_eq!((context.name.as_str(), context.priority, context.interrupt, context.id, context.expiry), ("Context702", 20, 0, 2_121_541_125, 1000));
        let trigger = context.trigger.as_ref().expect("trigger");
        assert_eq!(
            (trigger.interface.name.as_deref(), trigger.interface.system_crc, trigger.interface.interface_crc, trigger.params.len()),
            (Some("T_702_40_retry"), 10889, 21221, 1)
        );
        assert_eq!(context.conditions.iter().map(|c| (c.script_id, c.sources.clone())).collect::<Vec<_>>(), vec![(4, vec![]), (20, vec![Source::Trigger(0)])]);
        let output = context.output.as_ref().expect("output");
        assert_eq!((output.interface.name.as_deref(), output.interface.system_crc, output.interface.interface_crc), (Some("O_702_40_retry"), 20444, 14866));
        assert_eq!(output.sources, vec![Source::Getter(0), Source::Trigger(0), Source::Local(0)]);
        assert_eq!(context.getters.iter().map(|g| (g.script_id, g.sources.len())).collect::<Vec<_>>(), vec![(7, 0)]);
    }

    #[test]
    fn values_convert_like_convert_a_to_i() {
        assert_eq!(to_int(b"-279009080"), -279_009_080);
        assert_eq!(to_int(b"x12y"), 12);
        assert_eq!(to_int(b""), 0);
        assert_eq!(to_int(b"4294967297"), 1);
    }
}
