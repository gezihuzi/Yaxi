use std::hash::Hash;
use std::sync::{Arc, Condvar, Mutex, RwLock};

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

#[derive(Debug, Clone)]
pub(super) struct ClipData {
    pub(super) bytes: Vec<u8>,
    pub(super) target: Atom,
}

impl Hash for ClipData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.target.hash(state);
    }
}

impl PartialEq for ClipData {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target
    }
}

impl Eq for ClipData {}

#[derive(Clone, Default)]
pub(super) struct SelectionData {
    pub(super) data: Vec<ClipData>,
}

impl SelectionData {
    #[inline]
    pub fn poll(&self) -> bool {
        !self.data.is_empty()
    }

    #[inline]
    pub fn reset(&mut self) {
        self.data.clear()
    }

    #[inline]
    pub fn get(&self, target: Atom) -> Option<ClipData> {
        self.data.iter().find(|d| d.target == target).cloned()
    }

    #[inline]
    pub fn get_latest(&self) -> Option<ClipData> {
        self.data.last().cloned()
    }

    #[inline]
    pub fn targets(&self) -> Vec<Atom> {
        self.data.iter().map(|d| d.target).collect()
    }

    #[inline]
    pub fn set(&mut self, bytes: &[u8], target: Atom) {
        let clip_data = ClipData {
            bytes: bytes.to_vec(),
            target,
        };
        if let Some(pos) = self.data.iter().position(|d| d.target == target) {
            self.data.remove(pos);
        }
        self.data.push(clip_data);
    }

    pub fn remove(&mut self, target: Atom) -> Option<ClipData> {
        if let Some(pos) = self.data.iter().position(|d| d.target == target) {
            Some(self.data.remove(pos))
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ClipData> {
        self.data.iter()
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
        let data = Arc::new(RwLock::new(SelectionData::default()));
        Ok(Context {
            display,
            state,
            atoms,
            data,
        })
    }
}

impl Context {
    pub(super) fn set_selection_owner(&self, selection: Atom) -> Result<(), Error> {
        self.state.window.set_selection_owner(selection)?;
        Ok(())
    }

    pub(super) fn get_string(&self, target: Atom) -> Result<Option<String>, Error> {
        self.get_target_data(target)
            .map(|d| d.map(|d| String::from_utf8_lossy(&d.bytes).to_string()))
    }

    pub(super) fn set_string(&self, string: &str, target: Atom) -> Result<(), Error> {
        let bytes = string.as_bytes();
        self.write_data(bytes, target)
    }

    pub(super) fn get_targets(&self, selection: Atom) -> Result<Vec<Target>, Error> {
        let targets = self
            .wait_for_data(selection, self.atoms.protocol.targets)?
            .map(|d| d.bytes)
            .unwrap_or_default();

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
        try_read!(self.data).map(|d| d.targets())
    }

    pub(super) fn get_target_sizes(&self) -> Result<Option<Vec<(Atom, i32)>>, Error> {
        if let Some(data) = self.get_target_data(self.atoms.protocol.target_sizes)? {
            let mut sizes = Vec::new();
            for chunk in data.bytes.chunks(8) {
                if chunk.len() == 8 {
                    if let (Ok(atom), Ok(size_bytes)) = (
                        Atom::try_from(&chunk[0..4]),
                        <[u8; 4]>::try_from(&chunk[4..8]),
                    ) {
                        let size = i32::from_ne_bytes(size_bytes);
                        sizes.push((atom, size));
                    }
                }
            }
            Ok(Some(sizes))
        } else {
            Ok(None)
        }
    }

    pub(super) fn wait_for_data(
        &self,
        selection: Atom,
        target: Atom,
    ) -> Result<Option<ClipData>, Error> {
        try_write!(self.data)?.reset();

        self.state
            .window
            .convert_selection(selection, target, self.state.property)?;

        while !try_read!(self.data)?.poll() {}

        try_read!(self.data).map(|data| data.get(target))
    }

    pub(super) fn get_target_data(&self, target: Atom) -> Result<Option<ClipData>, Error> {
        let owner = self
            .display
            .get_selection_owner(self.atoms.selections.clipboard)?;

        let window = self.display.window_from_id(owner)?;
        if window.id() != self.state.window.id() {
            self.wait_for_data(self.atoms.selections.clipboard, target)
        } else {
            try_read!(self.data).map(|d| d.get(target))
        }
    }

    pub(super) fn write_data(&self, bytes: &[u8], target: Atom) -> Result<(), Error> {
        try_write!(self.data)?.set(bytes, target);
        Ok(())
    }

    pub(super) fn read_data(&self, target: Atom) -> Result<Option<Vec<u8>>, Error> {
        try_read!(self.data).map(|d| d.get(target).map(|d| d.bytes.clone()))
    }

    pub(super) fn clear_data(&self) -> Result<(), Error> {
        try_write!(self.data)?.reset();
        Ok(())
    }
}
