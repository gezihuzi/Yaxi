use crate::display::request::*;
use crate::display::error::Error;
use crate::display::Atom;
use crate::keyboard::Keysym;

use std::sync::atomic::{Ordering, AtomicU16};
use std::sync::{Arc, Mutex};

macro_rules! lock {
    ($mutex:expr) => {
        $mutex.lock().map_err(|_| Into::<Box<dyn std::error::Error>>::into("failed to lock mutex"))
    }
}

#[non_exhaustive]
pub struct Response;

impl Response {
    pub const ERROR: u8 = 0;
    pub const REPLY: u8 = 1;

    pub const KEY_PRESS: u8 = 2;
    pub const KEY_RELEASE: u8 = 3;
    pub const BUTTON_PRESS: u8 = 4;
    pub const BUTTON_RELEASE: u8 = 5;
    pub const MOTION_NOTIFY: u8 = 6;
    pub const ENTER_NOTIFY: u8 = 7;
    pub const FOCUS_IN: u8 = 9;
    pub const FOCUS_OUT: u8 = 10;
    pub const CREATE_NOTIFY: u8 = 16;
    pub const DESTROY_NOTIFY: u8 = 17;
    pub const UNMAP_NOTIFY: u8 = 18;
    pub const MAP_NOTIFY: u8 = 19;
    pub const MAP_REQUEST: u8 = 20;
    pub const REPARENT_NOTIFY: u8 = 21;
    pub const CONFIGURE_NOTIFY: u8 = 22;
    pub const CONFIGURE_REQUEST: u8 = 23;
    pub const GRAVITY_NOTIFY: u8 = 24;
    pub const CIRCULATE_NOTIFY: u8 = 26;
    pub const CIRCULATE_REQUEST: u8 = 27;
    pub const SELECTION_CLEAR: u8 = 29;
    pub const SELECTION_REQUEST: u8 = 30;
    pub const SELECTION_NOTIFY: u8 = 31;
    pub const CLIENT_MESSAGE: u8 = 33;
    pub const MAPPING_NOTIFY: u8 = 34;
}

#[non_exhaustive]
pub struct Opcode;

impl Opcode {
    pub const CREATE_WINDOW: u8 = 1;
    pub const CHANGE_WINDOW_ATTRIBUTES: u8 = 2;
    pub const GET_WINDOW_ATTRIBUTES: u8 = 3;
    pub const DESTROY_WINDOW: u8 = 4;
    pub const DESTROY_SUBWINDOWS: u8 = 5;
    pub const REPARENT_WINDOW: u8 = 7;
    pub const MAP_WINDOW: u8 = 8;
    pub const MAP_SUBWINDOWS: u8 = 9;
    pub const UNMAP_WINDOW: u8 = 10;
    pub const UNMAP_SUBWINDOWS: u8 = 11;
    pub const CONFIGURE_WINDOW: u8 = 12;
    pub const GET_GEOMETRY: u8 = 14;
    pub const INTERN_ATOM: u8 = 16;
    pub const CHANGE_PROPERTY: u8 = 18;
    pub const DELETE_PROPERTY: u8 = 19;
    pub const GET_PROPERTY: u8 = 20;
    pub const GRAB_POINTER: u8 = 26;
    pub const UNGRAB_POINTER: u8 = 27;
    pub const GRAB_BUTTON: u8 = 28;
    pub const UNGRAB_BUTTON: u8 = 29;
    pub const GRAB_KEY: u8 = 33;
    pub const QUERY_POINTER: u8 = 38;
    pub const SET_INPUT_FOCUS: u8 = 42;
    pub const GET_INPUT_FOCUS: u8 = 43;
    pub const GET_KEYBOARD_MAPPING: u8 = 101;
    pub const KILL_CLIENT: u8 = 113;
}

#[non_exhaustive]
pub struct ErrorCode;

impl ErrorCode {
    pub const REQUEST: u8 = 1;
    pub const VALUE: u8 = 2;
    pub const WINDOW: u8 = 3;
    pub const PIXMAP: u8 = 4;
    pub const ATOM: u8 = 5;
    pub const CURSOR: u8 = 6;
    pub const FONT: u8 = 7;
    pub const MATCH: u8 = 8;
    pub const DRAWABLE: u8 = 9;
    pub const ACCESS: u8 = 10;
    pub const ALLOC: u8 = 11;
    pub const COLORMAP: u8 = 12;
    pub const G_CONTEXT: u8 = 13;
    pub const ID_CHOICE: u8 = 14;
    pub const NAME: u8 = 15;
    pub const LENGTH: u8 = 16;
    pub const IMPLEMENTATION: u8 = 17;
}

#[derive(Clone)]
pub struct Queue<T> {
    queue: Arc<Mutex<Vec<T>>>,
}

impl<T> Queue<T> {
    pub fn new() -> Queue<T> {
        Queue {
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn clone(&self) -> Queue<T> {
        Queue {
            queue: self.queue.clone(),
        }
    }

    pub fn poll(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(!lock!(self.queue)?.is_empty())
    }

    pub fn wait(&mut self) -> Result<T, Box<dyn std::error::Error>> {
        while !self.poll()? {}

        lock!(self.queue)?.pop().ok_or(Box::new(Error::NoReply))
    }

    pub fn push(&mut self, element: T) -> Result<(), Box<dyn std::error::Error>> {
        lock!(self.queue).map(|mut lock| lock.push(element))
    }
}

#[derive(Debug, Clone)]
pub enum Reply {
    InternAtom(InternAtomResponse),
    GetWindowAttributes(GetWindowAttributesResponse),
    GetGeometry(GetGeometryResponse),
    QueryPointer(QueryPointerResponse),
    GetInputFocus(GetInputFocusResponse),
    GrabPointer(GrabPointerResponse),
    GetProperty {
        value: Vec<u8>,
    },
    GetKeyboardMapping {
        keysyms: Vec<Keysym>,
        keysyms_per_keycode: u8,
    },
}

#[derive(Debug)]
pub enum ReplyKind {
    InternAtom,
    GetProperty,
    GetWindowAttributes,
    QueryPointer,
    GetKeyboardMapping,
    GetInputFocus,
    GetGeometry,
    GrabPointer,
}

#[derive(Debug)]
pub struct Sequence {
    pub id: u16,
    pub kind: ReplyKind,
}

impl Sequence {
    pub fn new(id: u16, kind: ReplyKind) -> Sequence {
        Sequence {
            id,
            kind,
        }
    }
}

#[derive(Clone)]
pub struct SequenceManager {
    id: Arc<AtomicU16>,
    sequences: Arc<Mutex<Vec<Sequence>>>,
}

impl SequenceManager {
    pub fn new() -> SequenceManager {
        SequenceManager {
            id: Arc::new(AtomicU16::default()),
            sequences: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get(&mut self, id: u16) -> Result<Sequence, Box<dyn std::error::Error>> {
        let mut lock = lock!(self.sequences)?;

        match lock.iter().position(|sequence| sequence.id == id) {
            Some(index) => Ok(lock.remove(index)),
            None => Err(Box::new(Error::InvalidId)),
        }
    }

    pub fn skip(&mut self) {
        self.id.fetch_add(1, Ordering::Relaxed);
    }

    pub fn append(&mut self, kind: ReplyKind) -> Result<(), Box<dyn std::error::Error>> {
        self.skip();

        lock!(self.sequences)?.push(Sequence::new(self.id.load(Ordering::Relaxed), kind));

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Coordinates {
    x: u16,
    y: u16,
    root_x: u16,
    root_y: u16,
}

impl Coordinates {
    pub fn new(x: u16, y: u16, root_x: u16, root_y: u16) -> Coordinates {
        Coordinates {
            x,
            y,
            root_x,
            root_y,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StackMode {
    Above = 0,
    Below = 1,
    TopIf = 2,
    BottomIf = 3,
    Opposite = 4,
}

impl From<u8> for StackMode {
    fn from(value: u8) -> StackMode {
        match value {
            0 => StackMode::Above,
            1 => StackMode::Below,
            2 => StackMode::TopIf,
            3 => StackMode::BottomIf,
            4 => StackMode::Opposite,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum Place {
    Top,
    Bottom,
}

impl From<u8> for Place {
    fn from(value: u8) -> Place {
        match value {
            0 => Place::Top,
            1 => Place::Bottom,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum RevertTo {
    No = 0,
    PointerRoot = 1,
    Parent = 2,
}

#[derive(Debug, Clone, Copy)]
pub enum WindowClass {
    CopyFromParent = 0,
    InputOutput = 1,
    InputOnly = 2,
}

impl From<u16> for WindowClass {
    fn from(value: u16) -> WindowClass {
        match value {
            0 => WindowClass::CopyFromParent,
            1 => WindowClass::InputOutput,
            2 => WindowClass::InputOnly,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VisualClass {
    StaticGray = 0,
    GrayScale = 1,
    StaticColor = 2,
    PsuedoColor = 3,
    TrueColor = 4,
    DirectColor = 5,
}

impl From<u8> for VisualClass {
    fn from(value: u8) -> VisualClass {
        match value {
            0 => VisualClass::StaticGray,
            1 => VisualClass::GrayScale,
            2 => VisualClass::StaticColor,
            3 => VisualClass::PsuedoColor,
            4 => VisualClass::TrueColor,
            5 => VisualClass::DirectColor,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum BackingStore {
    NotUseful = 0,
    WhenMapped = 1,
    Always = 2,
}

#[derive(Clone, Copy)]
pub enum Gravity {
    Forget = 0,
    NorthWest = 1,
    North = 2,
    NorthEast = 3,
    West = 4,
    Center = 5,
    East = 6,
    SouthWest = 7,
    South = 8,
    SouthEast = 9,
    Static = 10,
}

#[derive(Clone, Copy)]
pub enum KeyMask {
    Shift = 0x0001,
    Lock = 0x0002,
    Control = 0x0004,
    Mod1 = 0x0008,
    Mod2 = 0x0010,
    Mod3 = 0x0020,
    Mod4 = 0x0040,
    Mod5 = 0x0080,
    Button1 = 0x0100,
    Button2 = 0x0200,
    Button3 = 0x0400,
    Button4 = 0x0800,
    Button5 = 0x1000,
}

#[derive(Clone, Copy)]
pub enum Mode {
    Synchronous = 0,
    Asynchronous = 1,
}

pub type PointerMode = Mode;
pub type KeyboardMode = Mode;

#[derive(Debug, Clone, Copy)]
pub enum EnterMode {
    Normal,
    Grab,
    Ungrab,
}

impl From<u8> for EnterMode {
    fn from(value: u8) -> EnterMode {
        match value {
            0 => EnterMode::Normal,
            1 => EnterMode::Grab,
            2 => EnterMode::Ungrab,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum EventMask {
    NoEvent = 0,
    KeyPress = 1,
    KeyRelease = 2,
    ButtonPress = 4,
    ButtonRelease = 8,
    EnterWindow = 16,
    LeaveWindow = 32,
    PointerMotion = 64,
    PointerMotionHint = 128,
    Button1Motion = 256,
    Button2Motion = 512,
    Button3Motion = 1024,
    Button4Motion = 2048,
    Button5Motion = 4096,
    ButtonMotion = 8192,
    KeymapState = 16384,
    Exposure = 32768,
    VisibilityChange = 65536,
    StructureNotify = 131072,
    ResizeRedirect = 262144,
    SubstructureNotify = 524288,
    SubstructureRedirect = 1048576,
    FocusChange = 2097152,
    PropertyChange = 4194304,
    ColorMapChange = 8388608,
    OwnerGrabButton = 16777216,
}

// TODO: add cursors
#[derive(Clone, Copy)]
pub enum Cursor {
    Nop = 0x0,
}

#[derive(Debug, Clone, Copy)]
pub enum Button {
    Button1 = 1,
    Button2 = 2,
    Button3 = 3,
    Button4 = 4,
    Button5 = 5,
}

impl From<u8> for Button {
    fn from(value: u8) -> Button {
        match value {
            1 => Button::Button1,
            2 => Button::Button2,
            3 => Button::Button3,
            4 => Button::Button4,
            _ => Button::Button5,
        }
    }
}

#[derive(Debug)]
pub enum FocusDetail {
    Ancestor = 0,
    Virtual = 1,
    Inferior = 2,
    Nonlinear = 3,
    NonlinearVirtual = 4,
    Pointer = 5,
    PointerRoot = 6,
    Nop = 7,
}


impl From<u8> for FocusDetail {
    fn from(value: u8) -> FocusDetail {
        match value {
            0 => FocusDetail::Ancestor,
            1 => FocusDetail::Virtual,
            2 => FocusDetail::Inferior,
            3 => FocusDetail::Nonlinear,
            4 => FocusDetail::NonlinearVirtual,
            5 => FocusDetail::Pointer,
            6 => FocusDetail::PointerRoot,
            _ => FocusDetail::Nop,
        }
    }
}

#[derive(Debug)]
pub enum FocusMode {
    Normal = 0,
    Grab = 1,
    Ungrab = 2,
    WhileGrab = 3,
}

impl From<u8> for FocusMode {
    fn from(value: u8) -> FocusMode {
        match value {
            0 => FocusMode::Normal,
            1 => FocusMode::Grab,
            2 => FocusMode::Ungrab,
            3 => FocusMode::WhileGrab,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    Press,
    Release,
}

#[derive(Debug)]
pub enum Event {
    KeyEvent {
        kind: EventKind,
        coordinates: Coordinates,
        window: u32,
        root: u32,
        subwindow: u32,
        state: u16,
        keycode: u8,
        send_event: bool,
    },
    ButtonEvent {
        kind: EventKind,
        coordinates: Coordinates,
        window: u32,
        root: u32,
        subwindow: u32,
        state: u16,
        button: Button,
        send_event: bool,
    },
    MotionNotify {
        coordinates: Coordinates,
        window: u32,
        root: u32,
        subwindow: u32,
        state: u16,
        send_event: bool,
    },
    EnterNotify {
        root: u32,
        window: u32,
        child: u32,
        coordinates: Coordinates,
        state: u16,
        mode: EnterMode,
        focus: bool,
        same_screen: bool,
    },
    FocusIn {
        detail: FocusDetail,
        mode: FocusMode,
        window: u32,
    },
    FocusOut {
        detail: FocusDetail,
        mode: FocusMode,
        window: u32,
    },
    CreateNotify {
        parent: u32,
        window: u32,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    },
    DestroyNotify {
        event: u32,
        window: u32,
    },
    UnmapNotify {
        event: u32,
        window: u32,
        configure: bool,
    },
    MapNotify {
        event: u32,
        window: u32,
        override_redirect: bool,
    },
    MapRequest {
        parent: u32,
        window: u32,
    },
    ReparentNotify {
        event: u32,
        window: u32,
        parent: u32,
        x: u16,
        y: u16,
        override_redirect: bool,
    },
    ConfigureNotify {
        event: u32,
        window: u32,
        above_sibling: u32,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        border_width: u16,
        override_redirect: bool,
    },
    ConfigureRequest {
        stack_mode: StackMode,
        parent: u32,
        window: u32,
        sibling: u32,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        border_width: u16,
        mask: u16,
    },
    GravityNotify {
        event: u32,
        window: u32,
        x: u16,
        y: u16,
    },
    CirculateNotify {
        event: u32,
        window: u32,
        place: Place,
    },
    CirculateRequest {
        parent: u32,
        window: u32,
        place: Place,
    },
    SelectionClear {
        time: u32,
        owner: u32,
        selection: Atom,
    },
    SelectionRequest {
        time: u32,
        owner: u32,
        selection: Atom,
        target: Atom,
        property: Atom,
    },
    SelectionNotify {
        time: u32,
        requestor: u32,
        selection: Atom,
        target: Atom,
        property: Atom,
    },
    ClientMessage {
        format: u8,
        window: u32,
        type_: Atom,
    },
    MappingNotify {
        request: u8,
        keycode: u8,
        count: u8,
    },
}


