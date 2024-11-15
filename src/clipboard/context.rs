use std::sync::{Arc, Condvar, Mutex, RwLock};

use crate::display::{error::Error, Atom, Display};
use crate::proto::{Event, WindowClass};
use crate::window::{PropFormat, PropMode, ValuesBuilder, Window, WindowArguments};

use super::atoms::Atoms;

#[derive(Default)]
pub(super) struct Selection {
    data: RwLock<Option<Vec<ClipboardData>>>,
    mutex: Mutex<()>,
    has_changed: Condvar,
}

impl Selection {
    pub(super) fn write(&self, data: Option<Vec<ClipboardData>>) -> Result<(), Error> {
        let _guard = self.mutex.lock().map_err(|_| Error::FailedToLock)?;
        let mut guard = self.data.write().map_err(|_| Error::FailedToLock)?;
        *guard = data;
        self.has_changed.notify_all();
        Ok(())
    }

    pub(super) fn read(&self) -> Result<Option<Vec<ClipboardData>>, Error> {
        let _guard = self.mutex.lock().map_err(|_| Error::FailedToLock)?;
        let guard = self.data.read().map_err(|_| Error::FailedToLock)?;
        Ok(guard.clone())
    }
}

#[derive(Debug, Clone)]
pub(super) struct ClipboardData {
    pub(super) bytes: Arc<Vec<u8>>,
    pub(super) format: Atom,
}

impl ClipboardData {
    pub(super) fn new(bytes: Vec<u8>, format: Atom) -> ClipboardData {
        ClipboardData {
            bytes: Arc::new(bytes),
            format,
        }
    }

    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
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
    pub(super) data: Arc<Selection>,
}

impl Context {
    pub(super) fn new(display: Display) -> Result<Self, Error> {
        let state = State::from_display(&display)?;
        let atoms = Atoms::new(&display)?;
        let data = Arc::new(Selection::default());
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

    pub fn write(&self, data: Vec<ClipboardData>, selection: Atom) -> Result<(), Error> {
        self.set_selection_owner(selection)?;

        self.data.write(Some(data))?;

        Ok(())
    }

    pub fn read(&self, formats: &[Atom], selection: Atom) -> Result<Option<ClipboardData>, Error> {
        // If we own the selection, return our data
        if self.is_owner(selection)? {
            let data = self.data.read()?;
            if let Some(data_list) = data {
                for data in data_list {
                    for format in formats {
                        if format == &data.format {
                            return Ok(Some(data.clone()));
                        }
                    }
                }
            }
            return Ok(None);
        }

        // Request data from current owner
        for format in formats {
            match self.request_conversion(selection, *format)? {
                Some(bytes) => {
                    return Ok(Some(ClipboardData::new(bytes, *format)));
                }
                None => continue,
            }
        }

        Ok(None)
    }

    fn check_selection_notify(&self) -> Result<Option<Vec<u8>>, Error> {
        let mut display = self.display.clone();
        let event = display.next_event()?;

        if let Event::SelectionNotify {
            property, target, ..
        } = event
        {
            if property == self.state.property {
                if let Some((bytes, _)) = self.state.window.get_property(property, target, true)? {
                    self.state.window.delete_property(property)?;
                    return Ok(Some(bytes));
                }
            }
        }

        Ok(None)
    }

    fn request_conversion(&self, selection: Atom, target: Atom) -> Result<Option<Vec<u8>>, Error> {
        self.state.window.delete_property(self.state.property)?;

        self.state
            .window
            .convert_selection(selection, target, self.state.property)?;

        // Wait for SelectionNotify with timeout
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(1000);

        while start.elapsed() < timeout {
            if let Some(data) = self.check_selection_notify()? {
                return Ok(Some(data));
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        Ok(None)
    }

    fn is_owner(&self, selection: Atom) -> Result<bool, Error> {
        let owner = self.display.get_selection_owner(selection)?;
        Ok(owner == self.state.window.id())
    }
}

#[derive(Debug)]
pub(super) enum ClipboardEvent {
    SelectionClear(Atom),
    SelectionRequest {
        time: u32,
        owner: u32,
        selection: Atom,
        target: Atom,
        property: Atom,
    },
    SelectionNotify {
        property: Atom,
        target: Atom,
        data: Option<Vec<u8>>,
    },
    Timeout,
    Error(Error),
}

impl Context {
    pub(super) fn handle_event(&self, event: ClipboardEvent) -> Result<(), Error> {
        match event {
            ClipboardEvent::SelectionClear(selection) => {
                if selection == self.atoms.selections.clipboard {
                    self.data.write(None)?;
                }
            }

            ClipboardEvent::SelectionRequest {
                time,
                owner,
                selection,
                target,
                property,
            } => {
                let requestor = self.display.window_from_id(owner)?;

                // 处理 TARGETS 请求
                if target == self.atoms.protocol.targets {
                    if let Some(data) = self.data.read()? {
                        let targets: Vec<_> = data
                            .iter()
                            .map(|d| d.format)
                            .flat_map(|f| f.to_ne_bytes())
                            .collect();

                        requestor.change_property(
                            property,
                            self.atoms.protocol.atom,
                            PropFormat::Format32,
                            PropMode::Replace,
                            &targets,
                        )?;

                        self.send_selection_notify(time, owner, selection, target, property)?;
                    } else {
                        self.send_selection_notify(
                            time,
                            owner,
                            selection,
                            target,
                            Atom::default(),
                        )?;
                    }
                }
                // 处理常规数据请求
                else if let Some(data_list) = self.data.read()? {
                    if let Some(data) = data_list.iter().find(|d| d.format == target) {
                        requestor.change_property(
                            property,
                            target,
                            PropFormat::Format8,
                            PropMode::Replace,
                            &data.bytes,
                        )?;

                        self.send_selection_notify(time, owner, selection, target, property)?;
                    } else {
                        self.send_selection_notify(
                            time,
                            owner,
                            selection,
                            target,
                            Atom::default(),
                        )?;
                    }
                } else {
                    self.send_selection_notify(time, owner, selection, target, Atom::default())?;
                }
            }

            ClipboardEvent::SelectionNotify {
                property,
                target,
                data,
            } => {
                if !property.is_null() {
                    if let Some(bytes) = data {
                        self.data
                            .write(Some(vec![ClipboardData::new(bytes, target)]))?;
                    }
                }
            }

            ClipboardEvent::Timeout => {
                // 处理超时情况
                // log::warn!("Event handling timeout");
            }

            ClipboardEvent::Error(e) => {
                // log::error!("Event handling error: {:?}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    fn send_selection_notify(
        &self,
        time: u32,
        requestor: u32,
        selection: Atom,
        target: Atom,
        property: Atom,
    ) -> Result<(), Error> {
        let requestor_window = self.display.window_from_id(requestor)?;
        let event = Event::SelectionNotify {
            time,
            requestor,
            selection,
            target,
            property,
        };
        requestor_window.send_event(event, vec![], false)?;
        Ok(())
    }
}
