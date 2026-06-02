//! Kitty graphics command parsing and response encoding.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParseError {
    InvalidData,
    InvalidFormat,
    OutOfMemory,
    Overflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Parser {
    kv: [Option<u32>; 256],
    kv_temp: [u8; 11],
    kv_temp_len: usize,
    kv_current: u8,
    data: Vec<u8>,
    max_bytes: usize,
    state: State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    ControlKey,
    ControlKeyIgnore,
    ControlValue,
    ControlValueIgnore,
    Data,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Command {
    pub(crate) control: CommandControl,
    pub(crate) quiet: Quiet,
    pub(crate) data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandControl {
    Query(Transmission),
    Transmit(Transmission),
    TransmitAndDisplay {
        transmission: Transmission,
        display: Display,
    },
    Display(Display),
    Delete(Delete),
    TransmitAnimationFrame(AnimationFrameLoading),
    ControlAnimation(AnimationControl),
    ComposeAnimation(AnimationFrameComposition),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Quiet {
    No,
    Ok,
    Failures,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Transmission {
    pub(crate) format: TransmissionFormat,
    pub(crate) medium: TransmissionMedium,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) size: u32,
    pub(crate) offset: u32,
    pub(crate) image_id: u32,
    pub(crate) image_number: u32,
    pub(crate) placement_id: u32,
    pub(crate) compression: TransmissionCompression,
    pub(crate) more_chunks: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransmissionFormat {
    Rgb,
    Rgba,
    Png,
    GrayAlpha,
    Gray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransmissionMedium {
    Direct,
    File,
    TemporaryFile,
    SharedMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransmissionCompression {
    None,
    ZlibDeflate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Display {
    pub(crate) image_id: u32,
    pub(crate) image_number: u32,
    pub(crate) placement_id: u32,
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) x_offset: u32,
    pub(crate) y_offset: u32,
    pub(crate) columns: u32,
    pub(crate) rows: u32,
    pub(crate) cursor_movement: CursorMovement,
    pub(crate) virtual_placement: bool,
    pub(crate) parent_id: u32,
    pub(crate) parent_placement_id: u32,
    pub(crate) horizontal_offset: i32,
    pub(crate) vertical_offset: i32,
    pub(crate) z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CursorMovement {
    After,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Delete {
    All {
        delete_images: bool,
    },
    Id {
        delete: bool,
        image_id: u32,
        placement_id: u32,
    },
    Newest {
        delete: bool,
        image_number: u32,
        placement_id: u32,
    },
    IntersectCursor {
        delete: bool,
    },
    AnimationFrames {
        delete: bool,
    },
    IntersectCell {
        delete: bool,
        x: u32,
        y: u32,
    },
    IntersectCellZ {
        delete: bool,
        x: u32,
        y: u32,
        z: i32,
    },
    Range {
        delete: bool,
        first: u32,
        last: u32,
    },
    Column {
        delete: bool,
        x: u32,
    },
    Row {
        delete: bool,
        y: u32,
    },
    Z {
        delete: bool,
        z: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnimationFrameLoading {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) create_frame: u32,
    pub(crate) edit_frame: u32,
    pub(crate) gap_ms: u32,
    pub(crate) composition_mode: CompositionMode,
    pub(crate) background: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnimationFrameComposition {
    pub(crate) frame: u32,
    pub(crate) edit_frame: u32,
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) left_edge: u32,
    pub(crate) top_edge: u32,
    pub(crate) composition_mode: CompositionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnimationControl {
    pub(crate) action: AnimationAction,
    pub(crate) frame: u32,
    pub(crate) gap_ms: u32,
    pub(crate) current_frame: u32,
    pub(crate) loops: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnimationAction {
    Invalid,
    Stop,
    RunWait,
    Run,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompositionMode {
    AlphaBlend,
    Overwrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Response<'a> {
    pub(crate) id: u32,
    pub(crate) image_number: u32,
    pub(crate) placement_id: u32,
    pub(crate) message: &'a [u8],
}

impl Default for Transmission {
    fn default() -> Self {
        Self {
            format: TransmissionFormat::Rgba,
            medium: TransmissionMedium::Direct,
            width: 0,
            height: 0,
            size: 0,
            offset: 0,
            image_id: 0,
            image_number: 0,
            placement_id: 0,
            compression: TransmissionCompression::None,
            more_chunks: false,
        }
    }
}

impl Default for Display {
    fn default() -> Self {
        Self {
            image_id: 0,
            image_number: 0,
            placement_id: 0,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            x_offset: 0,
            y_offset: 0,
            columns: 0,
            rows: 0,
            cursor_movement: CursorMovement::After,
            virtual_placement: false,
            parent_id: 0,
            parent_placement_id: 0,
            horizontal_offset: 0,
            vertical_offset: 0,
            z: 0,
        }
    }
}

impl Default for AnimationFrameLoading {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            create_frame: 0,
            edit_frame: 0,
            gap_ms: 0,
            composition_mode: CompositionMode::AlphaBlend,
            background: 0,
        }
    }
}

impl Default for AnimationFrameComposition {
    fn default() -> Self {
        Self {
            frame: 0,
            edit_frame: 0,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            left_edge: 0,
            top_edge: 0,
            composition_mode: CompositionMode::AlphaBlend,
        }
    }
}

impl Default for AnimationControl {
    fn default() -> Self {
        Self {
            action: AnimationAction::Invalid,
            frame: 0,
            gap_ms: 0,
            current_frame: 0,
            loops: 0,
        }
    }
}

impl<'a> Default for Response<'a> {
    fn default() -> Self {
        Self {
            id: 0,
            image_number: 0,
            placement_id: 0,
            message: b"OK",
        }
    }
}

impl Command {
    pub(crate) fn transmission(&self) -> Option<Transmission> {
        match &self.control {
            CommandControl::Query(transmission)
            | CommandControl::Transmit(transmission)
            | CommandControl::TransmitAndDisplay { transmission, .. } => Some(*transmission),
            CommandControl::Display(_)
            | CommandControl::Delete(_)
            | CommandControl::TransmitAnimationFrame(_)
            | CommandControl::ControlAnimation(_)
            | CommandControl::ComposeAnimation(_) => None,
        }
    }

    pub(crate) fn display(&self) -> Option<Display> {
        match &self.control {
            CommandControl::Display(display)
            | CommandControl::TransmitAndDisplay { display, .. } => Some(*display),
            CommandControl::Query(_)
            | CommandControl::Transmit(_)
            | CommandControl::Delete(_)
            | CommandControl::TransmitAnimationFrame(_)
            | CommandControl::ControlAnimation(_)
            | CommandControl::ComposeAnimation(_) => None,
        }
    }
}

impl TransmissionFormat {
    pub(crate) const fn bytes_per_pixel(self) -> Option<usize> {
        match self {
            Self::Gray => Some(1),
            Self::GrayAlpha => Some(2),
            Self::Rgb => Some(3),
            Self::Rgba => Some(4),
            Self::Png => None,
        }
    }
}

impl Parser {
    pub(crate) fn new(max_bytes: usize) -> Self {
        Self {
            kv: [None; 256],
            kv_temp: [0; 11],
            kv_temp_len: 0,
            kv_current: 0,
            data: Vec::new(),
            max_bytes,
            state: State::ControlKey,
        }
    }

    pub(crate) fn feed(&mut self, byte: u8) -> Result<(), ParseError> {
        match self.state {
            State::ControlKey => match byte {
                b'=' => {
                    if self.kv_temp_len != 1 {
                        self.state = State::ControlValueIgnore;
                        self.kv_temp_len = 0;
                    } else {
                        self.kv_current = self.kv_temp[0];
                        self.kv_temp_len = 0;
                        self.state = State::ControlValue;
                    }
                }
                b';' => self.state = State::Data,
                _ => self.accumulate_value(byte, State::ControlKeyIgnore)?,
            },
            State::ControlKeyIgnore => {
                if byte == b'=' {
                    self.state = State::ControlValueIgnore;
                }
            }
            State::ControlValue => match byte {
                b',' => self.finish_value(State::ControlKey)?,
                b';' => self.finish_value(State::Data)?,
                _ => self.accumulate_value(byte, State::ControlValueIgnore)?,
            },
            State::ControlValueIgnore => match byte {
                b',' => self.state = State::ControlKeyIgnore,
                b';' => self.state = State::Data,
                _ => {}
            },
            State::Data => {
                if self.data.len() >= self.max_bytes {
                    return Err(ParseError::OutOfMemory);
                }
                self.data.push(byte);
            }
        }
        Ok(())
    }

    pub(crate) fn complete(&mut self) -> Result<Command, ParseError> {
        match self.state {
            State::ControlKey | State::ControlKeyIgnore => return Err(ParseError::InvalidFormat),
            State::ControlValue => self.finish_value(State::Data)?,
            State::ControlValueIgnore | State::Data => {}
        }

        let action = self.get_char(b'a')?.unwrap_or(b't');
        let control = match action {
            b'q' => CommandControl::Query(self.parse_transmission()?),
            b't' => CommandControl::Transmit(self.parse_transmission()?),
            b'T' => CommandControl::TransmitAndDisplay {
                transmission: self.parse_transmission()?,
                display: self.parse_display()?,
            },
            b'p' => CommandControl::Display(self.parse_display()?),
            b'd' => CommandControl::Delete(self.parse_delete()?),
            b'f' => CommandControl::TransmitAnimationFrame(self.parse_animation_frame_loading()?),
            b'a' => CommandControl::ControlAnimation(self.parse_animation_control()?),
            b'c' => CommandControl::ComposeAnimation(self.parse_animation_frame_composition()?),
            _ => return Err(ParseError::InvalidFormat),
        };

        let quiet = match self.get(b'q') {
            None => Quiet::No,
            Some(0) => Quiet::No,
            Some(1) => Quiet::Ok,
            Some(2) => Quiet::Failures,
            Some(_) => return Err(ParseError::InvalidFormat),
        };

        Ok(Command {
            control,
            quiet,
            data: decode_base64(&self.data)?,
        })
    }

    fn accumulate_value(&mut self, byte: u8, overflow_state: State) -> Result<(), ParseError> {
        let idx = self.kv_temp_len;
        self.kv_temp_len += 1;
        if self.kv_temp_len > self.kv_temp.len() {
            self.state = overflow_state;
            self.kv_temp_len = 0;
            return Ok(());
        }
        self.kv_temp[idx] = byte;
        Ok(())
    }

    fn finish_value(&mut self, next_state: State) -> Result<(), ParseError> {
        self.state = next_state;
        if self.kv_temp_len == 1 {
            let c = self.kv_temp[0];
            if !c.is_ascii_digit() {
                self.kv[self.kv_current as usize] = Some(c as u32);
                self.kv_temp_len = 0;
                return Ok(());
            }
        }

        let bytes = &self.kv_temp[..self.kv_temp_len];
        let value = match self.kv_current {
            b'z' | b'H' | b'V' => {
                let text = std::str::from_utf8(bytes).map_err(|_| ParseError::InvalidFormat)?;
                let value = text.parse::<i32>().map_err(parse_int_error)?;
                value as u32
            }
            _ => {
                let text = std::str::from_utf8(bytes).map_err(|_| ParseError::InvalidFormat)?;
                text.parse::<u32>().map_err(parse_int_error)?
            }
        };
        self.kv[self.kv_current as usize] = Some(value);
        self.kv_temp_len = 0;
        Ok(())
    }

    fn get(&self, key: u8) -> Option<u32> {
        self.kv[key as usize]
    }

    fn get_char(&self, key: u8) -> Result<Option<u8>, ParseError> {
        self.get(key)
            .map(|value| u8::try_from(value).map_err(|_| ParseError::InvalidFormat))
            .transpose()
    }

    fn get_i32(&self, key: u8) -> Option<i32> {
        self.get(key).map(|value| value as i32)
    }

    fn parse_transmission(&self) -> Result<Transmission, ParseError> {
        let mut result = Transmission::default();
        if let Some(value) = self.get(b'f') {
            result.format = match value {
                24 => TransmissionFormat::Rgb,
                32 => TransmissionFormat::Rgba,
                100 => TransmissionFormat::Png,
                _ => return Err(ParseError::InvalidFormat),
            };
        }
        if let Some(value) = self.get_char(b't')? {
            result.medium = match value {
                b'd' => TransmissionMedium::Direct,
                b'f' => TransmissionMedium::File,
                b't' => TransmissionMedium::TemporaryFile,
                b's' => TransmissionMedium::SharedMemory,
                _ => return Err(ParseError::InvalidFormat),
            };
        }
        if let Some(value) = self.get(b's') {
            result.width = value;
        }
        if let Some(value) = self.get(b'v') {
            result.height = value;
        }
        if let Some(value) = self.get(b'S') {
            result.size = value;
        }
        if let Some(value) = self.get(b'O') {
            result.offset = value;
        }
        if let Some(value) = self.get(b'i') {
            result.image_id = value;
        }
        if let Some(value) = self.get(b'I') {
            result.image_number = value;
        }
        if let Some(value) = self.get(b'p') {
            result.placement_id = value;
        }
        if let Some(value) = self.get_char(b'o')? {
            result.compression = match value {
                b'z' => TransmissionCompression::ZlibDeflate,
                _ => return Err(ParseError::InvalidFormat),
            };
        }
        if result.medium == TransmissionMedium::Direct {
            if let Some(value) = self.get(b'm') {
                result.more_chunks = value > 0;
            }
        }
        Ok(result)
    }

    fn parse_display(&self) -> Result<Display, ParseError> {
        let mut result = Display::default();
        if let Some(value) = self.get(b'i') {
            result.image_id = value;
        }
        if let Some(value) = self.get(b'I') {
            result.image_number = value;
        }
        if let Some(value) = self.get(b'p') {
            result.placement_id = value;
        }
        if let Some(value) = self.get(b'x') {
            result.x = value;
        }
        if let Some(value) = self.get(b'y') {
            result.y = value;
        }
        if let Some(value) = self.get(b'w') {
            result.width = value;
        }
        if let Some(value) = self.get(b'h') {
            result.height = value;
        }
        if let Some(value) = self.get(b'X') {
            result.x_offset = value;
        }
        if let Some(value) = self.get(b'Y') {
            result.y_offset = value;
        }
        if let Some(value) = self.get(b'c') {
            result.columns = value;
        }
        if let Some(value) = self.get(b'r') {
            result.rows = value;
        }
        if let Some(value) = self.get(b'C') {
            result.cursor_movement = match value {
                0 => CursorMovement::After,
                1 => CursorMovement::None,
                _ => return Err(ParseError::InvalidFormat),
            };
        }
        if let Some(value) = self.get(b'U') {
            result.virtual_placement = match value {
                0 => false,
                1 => true,
                _ => return Err(ParseError::InvalidFormat),
            };
        }
        if let Some(value) = self.get_i32(b'z') {
            result.z = value;
        }
        if let Some(value) = self.get(b'P') {
            result.parent_id = value;
        }
        if let Some(value) = self.get(b'Q') {
            result.parent_placement_id = value;
        }
        if let Some(value) = self.get_i32(b'H') {
            result.horizontal_offset = value;
        }
        if let Some(value) = self.get_i32(b'V') {
            result.vertical_offset = value;
        }
        Ok(result)
    }

    fn parse_animation_frame_loading(&self) -> Result<AnimationFrameLoading, ParseError> {
        let mut result = AnimationFrameLoading::default();
        if let Some(value) = self.get(b'x') {
            result.x = value;
        }
        if let Some(value) = self.get(b'y') {
            result.y = value;
        }
        if let Some(value) = self.get(b'c') {
            result.create_frame = value;
        }
        if let Some(value) = self.get(b'r') {
            result.edit_frame = value;
        }
        if let Some(value) = self.get(b'z') {
            result.gap_ms = value;
        }
        if let Some(value) = self.get(b'X') {
            result.composition_mode = parse_composition_mode(value)?;
        }
        if let Some(value) = self.get(b'Y') {
            result.background = value;
        }
        Ok(result)
    }

    fn parse_animation_frame_composition(&self) -> Result<AnimationFrameComposition, ParseError> {
        let mut result = AnimationFrameComposition::default();
        if let Some(value) = self.get(b'c') {
            result.frame = value;
        }
        if let Some(value) = self.get(b'r') {
            result.edit_frame = value;
        }
        if let Some(value) = self.get(b'x') {
            result.x = value;
        }
        if let Some(value) = self.get(b'y') {
            result.y = value;
        }
        if let Some(value) = self.get(b'w') {
            result.width = value;
        }
        if let Some(value) = self.get(b'h') {
            result.height = value;
        }
        if let Some(value) = self.get(b'X') {
            result.left_edge = value;
        }
        if let Some(value) = self.get(b'Y') {
            result.top_edge = value;
        }
        if let Some(value) = self.get(b'C') {
            result.composition_mode = parse_composition_mode(value)?;
        }
        Ok(result)
    }

    fn parse_animation_control(&self) -> Result<AnimationControl, ParseError> {
        let mut result = AnimationControl::default();
        if let Some(value) = self.get(b's') {
            result.action = match value {
                0 => AnimationAction::Invalid,
                1 => AnimationAction::Stop,
                2 => AnimationAction::RunWait,
                3 => AnimationAction::Run,
                _ => return Err(ParseError::InvalidFormat),
            };
        }
        if let Some(value) = self.get(b'r') {
            result.frame = value;
        }
        if let Some(value) = self.get(b'z') {
            result.gap_ms = value;
        }
        if let Some(value) = self.get(b'c') {
            result.current_frame = value;
        }
        if let Some(value) = self.get(b'v') {
            result.loops = value;
        }
        Ok(result)
    }

    fn parse_delete(&self) -> Result<Delete, ParseError> {
        let what = self.get_char(b'd')?.unwrap_or(b'a');
        Ok(match what {
            b'a' | b'A' => Delete::All {
                delete_images: what == b'A',
            },
            b'i' | b'I' => Delete::Id {
                delete: what == b'I',
                image_id: self.get(b'i').unwrap_or(0),
                placement_id: self.get(b'p').unwrap_or(0),
            },
            b'n' | b'N' => Delete::Newest {
                delete: what == b'N',
                image_number: self.get(b'I').unwrap_or(0),
                placement_id: self.get(b'p').unwrap_or(0),
            },
            b'c' | b'C' => Delete::IntersectCursor {
                delete: what == b'C',
            },
            b'f' | b'F' => Delete::AnimationFrames {
                delete: what == b'F',
            },
            b'p' | b'P' => Delete::IntersectCell {
                delete: what == b'P',
                x: self.get(b'x').unwrap_or(0),
                y: self.get(b'y').unwrap_or(0),
            },
            b'q' | b'Q' => Delete::IntersectCellZ {
                delete: what == b'Q',
                x: self.get(b'x').unwrap_or(0),
                y: self.get(b'y').unwrap_or(0),
                z: self.get_i32(b'z').unwrap_or(0),
            },
            b'r' | b'R' => {
                let first = self.get(b'x').ok_or(ParseError::InvalidFormat)?;
                let last = self.get(b'y').ok_or(ParseError::InvalidFormat)?;
                if first > last {
                    return Err(ParseError::InvalidFormat);
                }
                Delete::Range {
                    delete: what == b'R',
                    first,
                    last,
                }
            }
            b'x' | b'X' => Delete::Column {
                delete: what == b'X',
                x: self.get(b'x').unwrap_or(0),
            },
            b'y' | b'Y' => Delete::Row {
                delete: what == b'Y',
                y: self.get(b'y').unwrap_or(0),
            },
            b'z' | b'Z' => Delete::Z {
                delete: what == b'Z',
                z: self.get_i32(b'z').unwrap_or(0),
            },
            _ => return Err(ParseError::InvalidFormat),
        })
    }
}

impl Response<'_> {
    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        if self.id == 0 && self.image_number == 0 {
            return;
        }
        out.extend_from_slice(b"\x1b_G");
        let mut prior = false;
        if self.id > 0 {
            prior = true;
            out.extend_from_slice(format!("i={}", self.id).as_bytes());
        }
        if self.image_number > 0 {
            if prior {
                out.push(b',');
            } else {
                prior = true;
            }
            out.extend_from_slice(format!("I={}", self.image_number).as_bytes());
        }
        if self.placement_id > 0 {
            if prior {
                out.push(b',');
            }
            out.extend_from_slice(format!("p={}", self.placement_id).as_bytes());
        }
        out.push(b';');
        out.extend_from_slice(self.message);
        out.extend_from_slice(b"\x1b\\");
    }

    pub(crate) fn ok(&self) -> bool {
        self.message == b"OK"
    }

    pub(crate) fn empty(&self) -> bool {
        self.id == 0 && self.image_number == 0
    }
}

fn parse_composition_mode(value: u32) -> Result<CompositionMode, ParseError> {
    match value {
        0 => Ok(CompositionMode::AlphaBlend),
        1 => Ok(CompositionMode::Overwrite),
        _ => Err(ParseError::InvalidFormat),
    }
}

fn parse_int_error(err: std::num::ParseIntError) -> ParseError {
    match err.kind() {
        std::num::IntErrorKind::PosOverflow | std::num::IntErrorKind::NegOverflow => {
            ParseError::Overflow
        }
        _ => ParseError::InvalidFormat,
    }
}

fn decode_base64(input: &[u8]) -> Result<Vec<u8>, ParseError> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::with_capacity((input.len() * 3) / 4);
    let mut chunk = [0u8; 4];
    let mut chunk_len = 0;
    let mut seen_padding = false;

    for &byte in input {
        if byte == b'=' {
            if chunk_len < 2 {
                return Err(ParseError::InvalidData);
            }
            seen_padding = true;
            chunk[chunk_len] = 64;
        } else {
            if seen_padding {
                return Err(ParseError::InvalidData);
            }
            chunk[chunk_len] = base64_value(byte).ok_or(ParseError::InvalidData)?;
        }
        chunk_len += 1;
        if chunk_len == 4 {
            push_base64_chunk(&chunk, &mut out)?;
            chunk = [0; 4];
            chunk_len = 0;
        }
    }

    if chunk_len > 0 {
        if chunk_len == 1 {
            return Err(ParseError::InvalidData);
        }
        for item in chunk.iter_mut().skip(chunk_len) {
            *item = 64;
        }
        push_base64_chunk(&chunk, &mut out)?;
    }

    Ok(out)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn push_base64_chunk(chunk: &[u8; 4], out: &mut Vec<u8>) -> Result<(), ParseError> {
    if chunk[0] == 64 || chunk[1] == 64 {
        return Err(ParseError::InvalidData);
    }
    let n = ((chunk[0] as u32) << 18)
        | ((chunk[1] as u32) << 12)
        | (if chunk[2] == 64 {
            0
        } else {
            (chunk[2] as u32) << 6
        })
        | (if chunk[3] == 64 { 0 } else { chunk[3] as u32 });
    out.push(((n >> 16) & 0xff) as u8);
    if chunk[2] != 64 {
        out.push(((n >> 8) & 0xff) as u8);
    }
    if chunk[3] != 64 {
        out.push((n & 0xff) as u8);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &[u8]) -> Result<Command, ParseError> {
        let mut parser = Parser::new(1024 * 1024);
        for &byte in input {
            parser.feed(byte)?;
        }
        parser.complete()
    }

    #[test]
    fn kitty_graphics_command_transmission_command() {
        let command = parse(b"f=24,s=10,v=20").unwrap();
        let CommandControl::Transmit(value) = command.control else {
            panic!("expected transmit");
        };
        assert_eq!(value.format, TransmissionFormat::Rgb);
        assert_eq!(value.width, 10);
        assert_eq!(value.height, 20);
    }

    #[test]
    fn kitty_graphics_command_transmission_ignores_m_if_medium_is_not_direct() {
        let command = parse(b"a=t,t=t,m=1").unwrap();
        let CommandControl::Transmit(value) = command.control else {
            panic!("expected transmit");
        };
        assert_eq!(value.medium, TransmissionMedium::TemporaryFile);
        assert!(!value.more_chunks);
    }

    #[test]
    fn kitty_graphics_command_transmission_respects_m_if_medium_is_direct() {
        let command = parse(b"a=t,t=d,m=1").unwrap();
        let CommandControl::Transmit(value) = command.control else {
            panic!("expected transmit");
        };
        assert_eq!(value.medium, TransmissionMedium::Direct);
        assert!(value.more_chunks);
    }

    #[test]
    fn kitty_graphics_command_query_command() {
        let command = parse(b"i=31,s=1,v=1,a=q,t=d,f=24;QUFBQQ").unwrap();
        let CommandControl::Query(value) = command.control else {
            panic!("expected query");
        };
        assert_eq!(value.medium, TransmissionMedium::Direct);
        assert_eq!(value.width, 1);
        assert_eq!(value.height, 1);
        assert_eq!(value.image_id, 31);
        assert_eq!(command.data, b"AAAA");
    }

    #[test]
    fn kitty_graphics_command_display_command() {
        let command = parse(b"a=p,U=1,i=31,c=80,r=120").unwrap();
        let CommandControl::Display(value) = command.control else {
            panic!("expected display");
        };
        assert_eq!(value.columns, 80);
        assert_eq!(value.rows, 120);
        assert_eq!(value.image_id, 31);
        assert!(value.virtual_placement);
    }

    #[test]
    fn kitty_graphics_command_delete_command() {
        let command = parse(b"a=d,d=p,x=3,y=4").unwrap();
        let CommandControl::Delete(Delete::IntersectCell { delete, x, y }) = command.control else {
            panic!("expected intersect cell delete");
        };
        assert!(!delete);
        assert_eq!(x, 3);
        assert_eq!(y, 4);
    }

    #[test]
    fn kitty_graphics_command_no_control_data() {
        let command = parse(b";QUFBQQ").unwrap();
        assert!(matches!(command.control, CommandControl::Transmit(_)));
        assert_eq!(command.data, b"AAAA");
    }

    #[test]
    fn kitty_graphics_command_ignore_unknown_keys_long() {
        let command = parse(b"f=24,s=10,v=20,hello=world").unwrap();
        let CommandControl::Transmit(value) = command.control else {
            panic!("expected transmit");
        };
        assert_eq!(value.format, TransmissionFormat::Rgb);
        assert_eq!(value.width, 10);
        assert_eq!(value.height, 20);
    }

    #[test]
    fn kitty_graphics_command_ignore_very_long_values() {
        let command = parse(b"f=24,s=10,v=2000000000000000000000000000000000000000").unwrap();
        let CommandControl::Transmit(value) = command.control else {
            panic!("expected transmit");
        };
        assert_eq!(value.format, TransmissionFormat::Rgb);
        assert_eq!(value.width, 10);
        assert_eq!(value.height, 0);
    }

    #[test]
    fn kitty_graphics_command_large_negative_values_do_not_get_skipped() {
        let command = parse(b"a=p,i=1,z=-2000000000").unwrap();
        let CommandControl::Display(value) = command.control else {
            panic!("expected display");
        };
        assert_eq!(value.image_id, 1);
        assert_eq!(value.z, -2_000_000_000);
    }

    #[test]
    fn kitty_graphics_command_u32_overflow_errors() {
        assert_eq!(parse(b"a=p,i=10000000000"), Err(ParseError::Overflow));
    }

    #[test]
    fn kitty_graphics_command_i32_overflow_errors() {
        assert_eq!(parse(b"a=p,i=1,z=-9999999999"), Err(ParseError::Overflow));
    }

    #[test]
    fn kitty_graphics_command_all_i32_values() {
        let command = parse(b"a=p,i=1,z=-1").unwrap();
        let CommandControl::Display(value) = command.control else {
            panic!("expected display");
        };
        assert_eq!(value.image_id, 1);
        assert_eq!(value.z, -1);

        let command = parse(b"a=p,i=1,H=-1").unwrap();
        let CommandControl::Display(value) = command.control else {
            panic!("expected display");
        };
        assert_eq!(value.image_id, 1);
        assert_eq!(value.horizontal_offset, -1);

        let command = parse(b"a=p,i=1,V=-1").unwrap();
        let CommandControl::Display(value) = command.control else {
            panic!("expected display");
        };
        assert_eq!(value.image_id, 1);
        assert_eq!(value.vertical_offset, -1);
    }

    #[test]
    fn kitty_graphics_command_invalid_base64_errors() {
        assert_eq!(parse(b";%%%%"), Err(ParseError::InvalidData));
        assert_eq!(parse(b";QU=F"), Err(ParseError::InvalidData));
        assert_eq!(parse(b";Q=UF"), Err(ParseError::InvalidData));
        assert_eq!(parse(b";QU==AA"), Err(ParseError::InvalidData));
    }

    #[test]
    fn kitty_graphics_command_max_bytes_enforced_before_decode() {
        let mut parser = Parser::new(2);
        parser.feed(b';').unwrap();
        parser.feed(b'Q').unwrap();
        parser.feed(b'U').unwrap();
        assert_eq!(parser.feed(b'F'), Err(ParseError::OutOfMemory));
    }

    #[test]
    fn kitty_graphics_command_response_encode_cases() {
        let mut out = Vec::new();
        Response::default().encode(&mut out);
        assert_eq!(out, b"");

        Response {
            id: 4,
            ..Response::default()
        }
        .encode(&mut out);
        assert_eq!(out, b"\x1b_Gi=4;OK\x1b\\");

        out.clear();
        Response {
            image_number: 4,
            ..Response::default()
        }
        .encode(&mut out);
        assert_eq!(out, b"\x1b_GI=4;OK\x1b\\");

        out.clear();
        Response {
            id: 12,
            image_number: 4,
            ..Response::default()
        }
        .encode(&mut out);
        assert_eq!(out, b"\x1b_Gi=12,I=4;OK\x1b\\");
    }

    #[test]
    fn kitty_graphics_command_response_helpers() {
        let response = Response::default();
        assert!(response.ok());
        assert!(response.empty());
        let response = Response {
            id: 1,
            message: b"ERR",
            ..Response::default()
        };
        assert!(!response.ok());
        assert!(!response.empty());
    }

    #[test]
    fn kitty_graphics_command_delete_range_cases() {
        let command = parse(b"a=d,d=r,x=3,y=4").unwrap();
        let CommandControl::Delete(Delete::Range {
            delete,
            first,
            last,
        }) = command.control
        else {
            panic!("expected range delete");
        };
        assert!(!delete);
        assert_eq!(first, 3);
        assert_eq!(last, 4);

        let command = parse(b"a=d,d=R,x=5,y=11").unwrap();
        let CommandControl::Delete(Delete::Range {
            delete,
            first,
            last,
        }) = command.control
        else {
            panic!("expected range delete");
        };
        assert!(delete);
        assert_eq!(first, 5);
        assert_eq!(last, 11);

        assert_eq!(parse(b"a=d,d=R,x=5,y=4"), Err(ParseError::InvalidFormat));
        assert_eq!(parse(b"a=d,d=R,x=5"), Err(ParseError::InvalidFormat));
        assert_eq!(parse(b"a=d,d=R,y=5"), Err(ParseError::InvalidFormat));
    }
}
