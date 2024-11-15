use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::display::{self};
use crate::proto::*;

use context::{ClipboardData, ClipboardEvent, Context};
use error::Error;

mod atoms;
mod context;
pub mod error;

pub struct Clipboard {
    context: context::Context,
    killed: Arc<AtomicBool>,
    server_handle: JoinHandle<Result<(), Error>>,
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        self.killed.store(true, Ordering::Relaxed);
        while !self.server_handle.is_finished() {}
    }
}

const EVENT_TIMEOUT: Duration = Duration::from_millis(1000);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

impl Clipboard {
    pub fn new(display: Option<&str>) -> Result<Clipboard, Error> {
        let display = display::open(display)?;
        let context = Context::new(display)?;
        let killed = Arc::new(AtomicBool::new(false));

        let context_clone = context.clone();
        let killed_clone = killed.clone();
        let handle = thread::spawn(move || serve_requests(context_clone, killed_clone));

        Ok(Clipboard {
            context,
            killed,
            server_handle: handle,
        })
    }
}

impl Clipboard {
    pub fn clear(&self) -> Result<(), Error> {
        self.context
            .set_selection_owner(self.context.atoms.selections.clipboard)?;
        self.context
            .state
            .window
            .delete_property(self.context.state.property)?;
        Ok(())
    }

    pub fn set_text(&self, text: &str) -> Result<(), Error> {
        let data = vec![ClipboardData::new(
            text.as_bytes().to_vec(),
            self.context.atoms.formats.utf8_string,
        )];

        self.context
            .write(data, self.context.atoms.selections.clipboard)?;
        Ok(())
    }

    pub fn get_text(&self) -> Result<Option<String>, Error> {
        let formats = [
            self.context.atoms.formats.utf8_string,
            self.context.atoms.formats.string,
            self.context.atoms.formats.plain,
        ];

        match self
            .context
            .read(&formats, self.context.atoms.selections.clipboard)?
        {
            Some(data) => {
                let bytes = data.bytes().to_owned();
                Ok(String::from_utf8(bytes).ok())
            }
            None => Ok(None),
        }
    }
}

fn serve_requests(context: Context, killed: Arc<AtomicBool>) -> Result<(), Error> {
    let mut display = context.display.clone();
    let mut last_event_time = Instant::now();

    while !killed.load(Ordering::Relaxed) {
        // 检查是否超时
        if last_event_time.elapsed() > EVENT_TIMEOUT {
            context.handle_event(ClipboardEvent::Timeout)?;
            last_event_time = Instant::now();
        }

        // 处理事件
        if let Ok(true) = display.poll_event() {
            last_event_time = Instant::now();

            let event = match display.next_event() {
                Ok(event) => event,
                Err(e) => {
                    context.handle_event(ClipboardEvent::Error(e))?;
                    continue;
                }
            };

            let clipboard_event = match event {
                Event::SelectionClear { selection, .. } => {
                    ClipboardEvent::SelectionClear(selection)
                }

                Event::SelectionRequest {
                    time,
                    owner,
                    selection,
                    target,
                    property,
                } => ClipboardEvent::SelectionRequest {
                    time,
                    owner,
                    selection,
                    target,
                    property,
                },

                Event::SelectionNotify {
                    property, target, ..
                } => {
                    let data = if !property.is_null() {
                        context
                            .state
                            .window
                            .get_property(property, target, true)?
                            .map(|(bytes, _)| bytes)
                    } else {
                        None
                    };

                    ClipboardEvent::SelectionNotify {
                        property,
                        target,
                        data,
                    }
                }

                _ => continue,
            };

            context.handle_event(clipboard_event)?;
        }

        // 避免CPU空转
        std::thread::sleep(POLL_INTERVAL);
    }

    Ok(())
}
