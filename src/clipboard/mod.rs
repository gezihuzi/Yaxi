use context::{Context, Target};

use crate::display::error::Error;
use crate::display::{self, *};
use crate::proto::*;
use crate::window::*;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

mod atoms;
mod context;

pub struct Clipboard {
    context: context::Context,
    listener: Listener,
    handle: JoinHandle<Result<(), Error>>,
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        self.listener.kill.store(true, Ordering::Relaxed);
        while !self.handle.is_finished() {}
    }
}

impl Clipboard {
    /// create a new clipboard helper instance
    pub fn new(display: Option<&str>) -> Result<Clipboard, Error> {
        let display = display::open(display)?;
        let context = Context::new(display)?;
        let listener = Listener::new(context.clone());

        let listener_clone = listener.clone();
        let handle = thread::spawn(move || listener_clone.listen());

        Ok(Clipboard {
            context,
            listener,
            handle,
        })
    }
}

impl Clipboard {
    pub fn clear(&self) -> Result<(), Error> {
        self.context.set_selection_as_clipboard()?;
        self.context
            .state
            .window
            .delete_property(self.context.state.property)
    }

    /// set text into the clipboard
    pub fn set_text(&self, text: &str) -> Result<(), Error> {
        self.context.set_selection_as_clipboard()?;
        self.context
            .set_string(text, self.context.atoms.formats.utf8_string)
    }

    // TODO: this deadlocks if the owner terminates during the call
    /// get text from the clipboard
    pub fn get_text(&self) -> Result<Option<String>, Error> {
        self.context
            .get_string(self.context.atoms.formats.utf8_string)
    }

    pub fn get_html(&self) -> Result<Option<String>, Error> {
        self.context.get_string(self.context.atoms.formats.html)
    }

    pub fn get_rtf(&self) -> Result<Option<String>, Error> {
        self.context.get_string(self.context.atoms.formats.rtf)
    }

    pub fn get_uri_list(&self) -> Result<Option<Vec<String>>, Error> {
        let uris = self
            .context
            .get_string(self.context.atoms.formats.uri_list)?
            .map(|string| string.lines().map(|line| line.to_string()).collect());
        Ok(uris)
    }

    pub fn get_plain_text(&self) -> Result<Option<String>, Error> {
        self.context
            .get_string(self.context.atoms.formats.utf8_string)
            .or_else(|_| self.context.get_string(self.context.atoms.formats.plain))
            .or_else(|_| self.context.get_string(self.context.atoms.formats.string))
    }

    pub fn get_targets(&self) -> Result<Vec<Target>, Error> {
        self.context
            .get_targets(self.context.atoms.selections.clipboard)
    }
}

#[derive(Clone)]
struct Listener {
    context: Context,
    kill: Arc<AtomicBool>,
}

impl Listener {
    pub fn new(context: Context) -> Listener {
        let kill = Arc::new(AtomicBool::new(false));
        Listener { context, kill }
    }

    fn get_saved_data(&self, target: Atom) -> Result<Option<Vec<u8>>, Error> {
        self.context.get_saved_data(target)
    }

    fn handle_manager_request(
        &self,
        requestor: &Window,
        target: Atom,
        property: Atom,
    ) -> Result<(), Error> {
        if target == self.context.atoms.protocol.save_targets {
            self.handle_save_targets(requestor, property)?;
        } else if let Some(data) = self.get_saved_data(target)? {
            requestor.change_property(
                property,
                target,
                PropFormat::Format8,
                PropMode::Replace,
                &data,
            )?;
        }
        Ok(())
    }

    fn handle_save_targets(&self, requestor: &Window, property: Atom) -> Result<(), Error> {
        let targets = self.context.read_saved_targets()?;

        let data: Vec<u8> = targets.iter().flat_map(|atom| atom.to_ne_bytes()).collect();
        requestor.change_property(
            property,
            self.context.atoms.protocol.atom,
            PropFormat::Format8,
            PropMode::Replace,
            &data,
        )?;

        Ok(())
    }

    fn handle_data_request(
        &self,
        requestor: &Window,
        target: Atom,
        property: Atom,
    ) -> Result<(), Error> {
        let data = self.context.read_clipboard_data()?;
        if target.id() == data.target.id() && !property.is_null() {
            requestor.change_property(
                property,
                target,
                PropFormat::Format8,
                PropMode::Replace,
                &data.bytes,
            )?;
        }
        Ok(())
    }

    fn send_selection_notify(
        &self,
        time: u32,
        owner: Window,
        selection: Atom,
        target: Atom,
        property: Atom,
    ) -> Result<(), Error> {
        let event = Event::SelectionNotify {
            time,
            requestor: owner.id(),
            selection,
            target,
            property,
        };
        owner.send_event(event, vec![], false)?;
        Ok(())
    }

    pub fn handle_request(
        &self,
        time: u32,
        owner: Window,
        selection: Atom,
        target: Atom,
        property: Atom,
    ) -> Result<(), Error> {
        if selection == self.context.atoms.protocol.manager {
            self.handle_manager_request(&owner, target, property)?;
        } else if selection == self.context.atoms.selections.clipboard {
            if target == self.context.atoms.protocol.targets {
                self.handle_save_targets(&owner, property)?;
            } else {
                self.handle_data_request(&owner, target, property)?;
            }
        }
        self.send_selection_notify(time, owner, selection, target, property)
    }

    fn listen(&self) -> Result<(), Error> {
        let mut display = self.context.display.clone();
        while !self.kill.load(Ordering::Relaxed) {
            if display.poll_event()? {
                match display.next_event()? {
                    Event::SelectionClear { selection, .. } => {
                        if selection == self.context.atoms.selections.clipboard {
                            self.context.save_clipboard(selection)?;
                        }
                    }
                    Event::SelectionRequest {
                        time,
                        owner,
                        selection,
                        target,
                        property,
                    } => {
                        let owner = self.context.display.window_from_id(owner)?;
                        self.handle_request(time, owner, selection, target, property)?;
                    }
                    Event::SelectionNotify { property, .. } => {
                        if let Some((bytes, _)) = self.context.state.window.get_property(
                            self.context.state.property,
                            Atom::ANY_PROPERTY_TYPE,
                            false,
                        )? {
                            let bytes = if property.is_null() {
                                Vec::new()
                            } else {
                                bytes
                            };
                            self.context.write_clipboard_data(
                                &bytes,
                                self.context.atoms.formats.utf8_string,
                            )?;
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
}
