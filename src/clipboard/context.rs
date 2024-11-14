use std::sync::{Arc, RwLock};

use crate::display::{error::Error, Atom, Display};
use crate::proto::WindowClass;
use crate::window::{ValuesBuilder, Window, WindowArguments};

use super::atoms::Atoms;

macro_rules! try_write {
    ($mutex:expr) => {
        $mutex.write().map_err(|_| Error::FailedToLock)
    };
}

macro_rules! try_read {
    ($mutex:expr) => {
        $mutex.read().map_err(|_| Error::FailedToLock)
    };
}

#[derive(Debug, Clone)]
pub struct Target {
    pub atom: Atom,
    pub name: Option<String>,
}

impl From<Atom> for Target {
    fn from(atom: Atom) -> Target {
        Target { atom, name: None }
    }
}

impl Target {
    pub fn new(atom: Atom, name: Option<String>) -> Target {
        Target { atom, name }
    }

    pub fn atom(&self) -> Atom {
        self.atom
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

pub(super) struct ClipData {
    pub(super) bytes: Vec<u8>,
    pub(super) target: Atom,
}

#[derive(Clone)]
pub(super) struct SelectionData {
    pub(super) bytes: Option<Vec<u8>>,
    pub(super) target: Atom,
    pub(super) saved_targets: Vec<(Target, Vec<u8>)>,
}

impl SelectionData {
    pub fn new() -> SelectionData {
        SelectionData {
            bytes: None,
            target: Atom::new(0),
            saved_targets: Vec::new(),
        }
    }

    #[inline]
    pub fn poll(&self) -> bool {
        self.bytes.is_some()
    }

    #[inline]
    pub fn reset(&mut self) {
        self.bytes = None
    }

    #[inline]
    pub fn get(&self) -> Vec<u8> {
        self.bytes.clone().unwrap_or_default()
    }

    #[inline]
    pub fn set(&mut self, bytes: &[u8], format: Atom) {
        self.bytes.replace(bytes.to_vec());

        self.target = format;
    }
}

#[derive(Clone)]
pub(super) struct State {
    pub(super) window: Window,
    pub(super) property: Atom,
}

impl State {
    pub(super) fn new(window: Window, property: Atom) -> State {
        State { window, property }
    }

    pub(super) fn from_display(display: &Display) -> Result<State, Error> {
        let root = display.default_root_window()?;

        let window = root.create_window(WindowArguments {
            depth: root.depth(),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            class: WindowClass::InputOutput,
            border_width: 0,
            visual: root.visual(),
            values: ValuesBuilder::new(vec![]),
        })?;

        let property = display.intern_atom("SKIBIDI_TOILET", false)?;
        Ok(State::new(window, property))
    }
}

#[derive(Clone)]
pub(super) struct Context {
    pub(super) display: Display,
    pub(super) state: State,
    pub(super) atoms: super::atoms::Atoms,
    pub(super) data: Arc<RwLock<SelectionData>>,
}

impl Context {
    pub(super) fn new(display: Display) -> Result<Self, Error> {
        let state = State::from_display(&display)?;
        let atoms = Atoms::new(&display)?;
        let data = Arc::new(RwLock::new(SelectionData::new()));
        Ok(Context {
            display,
            state,
            atoms,
            data,
        })
    }

    pub(super) fn set_selection_as_clipboard(&self) -> Result<(), Error> {
        self.set_selection_owner(self.atoms.selections.clipboard)
    }

    pub(super) fn set_selection_owner(&self, selection: Atom) -> Result<(), Error> {
        self.state.window.set_selection_owner(selection)?;
        Ok(())
    }

    pub(super) fn convert_selection(
        &self,
        selection: Atom,
        target: Atom,
    ) -> Result<Vec<u8>, Error> {
        try_write!(self.data)?.reset();

        self.state
            .window
            .convert_selection(selection, target, self.state.property)?;

        while !try_read!(self.data)?.poll() {}

        try_read!(self.data).map(|data| data.get())
    }

    pub(super) fn get_bytes(&self, target: Atom) -> Result<Option<Vec<u8>>, Error> {
        let owner = self
            .display
            .get_selection_owner(self.atoms.selections.clipboard)?;

        let window = self.display.window_from_id(owner)?;
        let selection = if window.id() != self.state.window.id() {
            self.convert_selection(self.atoms.selections.clipboard, target)?
        } else {
            try_read!(self.data).map(|data| data.get())?
        };

        Ok(Some(selection))
    }

    pub(super) fn set_bytes(&self, bytes: &[u8], target: Atom) -> Result<(), Error> {
        try_write!(self.data)?.set(bytes, target);
        Ok(())
    }

    pub(super) fn get_string(&self, target: Atom) -> Result<Option<String>, Error> {
        let bytes = self.get_bytes(target)?;
        let string = bytes
            .map(|bytes| String::from_utf8(bytes).map_err(|e| Error::Other { error: e.into() }))
            .transpose()?;
        Ok(string)
    }

    pub(super) fn set_string(&self, string: &str, target: Atom) -> Result<(), Error> {
        let bytes = string.as_bytes();
        self.set_bytes(bytes, target)
    }

    pub(super) fn get_targets(&self, selection: Atom) -> Result<Vec<Target>, Error> {
        let targets = self.convert_selection(selection, self.atoms.protocol.targets)?;
        let mut atoms = vec![];

        for i in 0..targets.len() / 4 {
            let bytes = &targets[i * 4..(i + 1) * 4];
            if let Ok(atom) = Atom::try_from(bytes) {
                atoms.push(atom);
            }
        }

        let targets = atoms.into_iter().map(Target::from).collect();
        Ok(targets)
    }

    pub(super) fn read_saved_targets(&self) -> Result<Vec<Atom>, Error> {
        try_read!(self.data).map(|data| {
            data.saved_targets
                .iter()
                .map(|(target, _)| target.atom())
                .collect()
        })
    }

    pub fn read_clipboard_data(&self) -> Result<ClipData, Error> {
        let data = try_read!(self.data)?;
        Ok(ClipData {
            bytes: data.get(),
            target: data.target,
        })
    }

    // 写入数据的示例
    pub fn write_clipboard_data(&self, bytes: &[u8], target: Atom) -> Result<(), Error> {
        try_write!(self.data)?.set(bytes, target);
        Ok(())
    }

    pub(super) fn get_saved_data(&self, target: Atom) -> Result<Option<Vec<u8>>, Error> {
        try_read!(self.data).map(|data| {
            data.saved_targets
                .iter()
                .find(|(t, _)| t.atom() == target)
                .map(|(_, data)| data.clone())
        })
    }

    pub(super) fn save_clipboard(&self, selection: Atom) -> Result<(), Error> {
        let targets = self.get_targets(selection)?;
        let mut saved_data = Vec::new();

        for target in targets {
            if let Some(data) = self.get_bytes(target.atom())? {
                saved_data.push((target, data));
            }
        }

        try_write!(self.data).map(|mut lock| lock.saved_targets = saved_data)?;

        Ok(())
    }
}
