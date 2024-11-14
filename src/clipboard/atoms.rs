use crate::display::{error::Error, Atom, Display};

#[derive(Clone)]
pub(super) struct Atoms {
    pub(super) selections: SelectionAtoms,
    pub(super) protocol: ProtocolAtoms,
    pub(super) formats: FormatAtoms,
}

#[derive(Clone)]
pub(super) struct SelectionAtoms {
    pub(super) clipboard: Atom,
    pub(super) primary: Atom,
    pub(super) secondary: Atom,
}

#[derive(Clone)]
pub(super) struct ProtocolAtoms {
    pub(super) manager: Atom,
    pub(super) save_targets: Atom,
    pub(super) targets: Atom,
    pub(super) atom: Atom,
    pub(super) incremental: Atom,
}

#[derive(Clone)]
pub(super) struct FormatAtoms {
    pub(super) utf8_string: Atom,
    pub(super) utf8_mime: Atom,
    pub(super) utf8_mime_alt: Atom,
    pub(super) string: Atom,
    pub(super) text: Atom,
    pub(super) plain: Atom,
    pub(super) html: Atom,
    pub(super) rtf: Atom,
    pub(super) png: Atom,
    pub(super) jpeg: Atom,
    pub(super) tiff: Atom,
    pub(super) pdf: Atom,
    pub(super) uri_list: Atom,
}

impl Atoms {
    pub(super) fn new(display: &Display) -> Result<Atoms, Error> {
        Ok(Atoms {
            selections: SelectionAtoms::new(display)?,
            protocol: ProtocolAtoms::new(display)?,
            formats: FormatAtoms::new(display)?,
        })
    }
}

impl SelectionAtoms {
    fn new(display: &Display) -> Result<Self, Error> {
        Ok(Self {
            clipboard: display.intern_atom("CLIPBOARD", false)?,
            primary: display.intern_atom("PRIMARY", false)?,
            secondary: display.intern_atom("SECONDARY", false)?,
        })
    }
}

impl ProtocolAtoms {
    fn new(display: &Display) -> Result<Self, Error> {
        Ok(Self {
            manager: display.intern_atom("CLIPBOARD_MANAGER", false)?,
            save_targets: display.intern_atom("SAVE_TARGETS", false)?,
            targets: display.intern_atom("TARGETS", false)?,
            atom: display.intern_atom("ATOM", false)?,
            incremental: display.intern_atom("INCR", false)?,
        })
    }
}

impl FormatAtoms {
    fn new(display: &Display) -> Result<Self, Error> {
        Ok(Self {
            utf8_string: display.intern_atom("UTF8_STRING", false)?,
            utf8_mime: display.intern_atom("text/plain;charset=utf-8", false)?,
            utf8_mime_alt: display.intern_atom("text/plain;charset=utf8", false)?,
            string: display.intern_atom("STRING", false)?,
            text: display.intern_atom("TEXT", false)?,
            plain: display.intern_atom("text/plain", false)?,
            html: display.intern_atom("text/html", false)?,
            rtf: display.intern_atom("text/rtf", false)?,
            png: display.intern_atom("image/png", false)?,
            jpeg: display.intern_atom("image/jpeg", false)?,
            tiff: display.intern_atom("image/tiff", false)?,
            pdf: display.intern_atom("application/pdf", false)?,
            uri_list: display.intern_atom("text/uri-list", false)?,
        })
    }
}
