use crate::display::error::Error;
use crate::display::{self, *};
use crate::window::*;
use crate::proto::*;

use std::thread::{self, JoinHandle};
use std::io::{Read, Write};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};

macro_rules! write {
    ($mutex:expr) => {
        $mutex.write().map_err(|_| Error::FailedToLock)
    }
}

macro_rules! read {
    ($mutex:expr) => {
        $mutex.write().map_err(|_| Error::FailedToLock)
    }
}

#[derive(Clone)]
struct ClipboardData {
    bytes: Vec<u8>,
    format: Atom,
}

impl ClipboardData {
    pub fn new() -> ClipboardData {
        ClipboardData {
            bytes: Vec::new(),
            format: Atom::new(0),
        }
    }

    pub fn get(&self) -> Vec<u8> { self.bytes.clone() }

    pub fn set(&mut self, bytes: &[u8], format: Atom) {
        self.bytes = bytes.to_vec();

        self.format = format;
    }
}

struct Atoms {
    clipboard: Atom,
    utf8: Atom,
}

struct Target<T> where T: Send + Sync + Read + Write + TryClone {
    window: Window<T>,
    property: Atom,
}

pub struct Clipboard<T> where T: Send + Sync + Read + Write + TryClone {
    display: Display<T>,
    target: Target<T>,
    atoms: Atoms,
    data: Arc<RwLock<ClipboardData>>,
    listener: ListenerHandle,
}

impl<T> Drop for Clipboard<T> where T: Send + Sync + Read + Write + TryClone {
    fn drop(&mut self) {
        self.listener.kill();
    }
}

impl<T> Clipboard<T> where T: Send + Sync + Read + Write + TryClone + 'static {
    pub fn new() -> Result<Clipboard<T>, Error> {
        let mut display = display::open_unix(0)?;
        let mut root = display.default_root_window()?;

        let target = Target {
            window: root.create_window(WindowArguments {
                depth: root.depth(),
                x: 0,
                y: 0,
                width: 1,
                height: 1,
                class: WindowClass::InputOutput,
                border_width: 0,
                visual: root.visual(),
                values: ValuesBuilder::new(vec![]),
            })?,
            property: display.intern_atom("SKIBIDI_TOILET", false)?,
        };

        let atoms = Atoms {
            clipboard: display.intern_atom("CLIPBOARD", false)?,
            utf8: display.intern_atom("UTF8_STRING", false)?,
        };

        let data = Arc::new(RwLock::new(ClipboardData::new()));

        let listener = ListenerHandle::spawn(display.clone(), data.clone(), Arc::new(AtomicBool::new(false)));

        Ok(Clipboard {
            display,
            target,
            atoms,
            data,
            listener,
        })
    }

    /// set text into the clipboard
    pub fn set_text(&mut self, text: &str) -> Result<(), Error> {
        self.target.window.set_selection_owner(self.atoms.clipboard)?;

        write!(self.data).map(|mut lock| lock.set(text.as_bytes(), self.atoms.utf8))
    }

    fn to_string(&self, bytes: Vec<u8>) -> Result<String, Error> {
        String::from_utf8(bytes).map_err(|err| Error::Other { error: err.into() })
    }

    fn get_selection(&mut self) -> Result<Vec<u8>, Error> {
        self.target.window.convert_selection(self.atoms.clipboard, self.atoms.utf8, self.target.property)?;

        loop {
            println!("waiting for event");

            // TODO: its almost random, sometimes it works, other times it doesnt
            //
            // maybe its a deadlock?

            match self.display.next_event()? {
                Event::SelectionNotify { property, .. } => {
                    println!("huh?: {:?}", property);

                    let (bytes, _) = self.target.window.get_property(self.target.property, Atom::ANY_PROPERTY_TYPE, false)?;

                    return property.is_null()
                        .then(|| Ok(Vec::new()))
                        .unwrap_or_else(|| Ok(bytes));
                },
                _ => {},
            }
        }
    }

    /// get text from the clipboard
    pub fn get_text(&mut self) -> Result<String, Error> {
        let owner = self.display.get_selection_owner(self.atoms.clipboard)?;

        if owner == self.target.window.id() {
            self.to_string(read!(self.data)?.get())
        } else if self.display.window_from_id(owner).is_ok() {
            println!("ok?: {}", owner);

            let selection = self.get_selection()?;

            println!("selection: {:?}", selection);

            self.to_string(selection)
        } else {
            Ok(String::new())
        }
    }
}

struct ListenerHandle {
    thread: JoinHandle<Result<(), Error>>,
    kill: Arc<AtomicBool>,
}

impl ListenerHandle {
    pub fn spawn<T: Send + Sync + Read + Write + TryClone + 'static>(display: Display<T>, data: Arc<RwLock<ClipboardData>>, kill: Arc<AtomicBool>) -> ListenerHandle {
        let clone = kill.clone();

        ListenerHandle {
            thread: thread::spawn(move || -> Result<(), Error> {
                let mut listener = Listener::new(display, data, clone);

                listener.listen()
            }),
            kill,
        }
    }

    pub fn kill(&mut self) {
        self.kill.store(true, Ordering::Relaxed);

        while !self.thread.is_finished() {}
    }
}

struct Listener<T> where T: Send + Sync + Read + Write + TryClone {
    display: Display<T>,
    data: Arc<RwLock<ClipboardData>>,
    kill: Arc<AtomicBool>,
}

impl<T> Listener <T> where T: Send + Sync + Read + Write + TryClone + 'static {
    pub fn new(display: Display<T>, data: Arc<RwLock<ClipboardData>>, kill: Arc<AtomicBool>) -> Listener<T> {
        Listener {
            display,
            data,
            kill,
        }
    }

    pub fn is_valid(&mut self, target: Atom, property: Atom) -> Result<bool, Error> {
        Ok(target.id() == read!(self.data)?.format.id() && !property.is_null())
    }

    pub fn handle_request(&mut self, time: u32, mut owner: Window<T>, selection: Atom, target: Atom, property: Atom) -> Result<(), Error> {
        if self.is_valid(target, property)? {
            let data = read!(self.data)?.get();

            owner.change_property(property, target, PropFormat::Format8, PropMode::Replace, &data)?;
        }

        owner.send_event(Event::SelectionNotify {
            time,
            requestor: owner.id(),
            selection,
            target,
            property: self.is_valid(target, property)?.then(|| property).unwrap_or(Atom::new(0)),
        }, vec![], true)
    }

    pub fn listen(&mut self) -> Result<(), Error> {
        while !self.kill.load(Ordering::Relaxed) {
            if self.display.poll_event()? {
                match self.display.next_event()? {
                    Event::SelectionRequest { time, owner, selection, target, property } => {
                        let owner = self.display.window_from_id(owner)?;

                        self.handle_request(time, owner, selection, target, property)?;
                    },
                    _ => {},
                }
            }
        }

        Ok(())
    }
}


