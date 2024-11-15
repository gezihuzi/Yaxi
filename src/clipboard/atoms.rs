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
    pub manager: Atom,          // "CLIPBOARD_MANAGER"
    pub targets: Atom,          // "TARGETS"
    pub multiple: Atom,         // "MULTIPLE"
    pub timestamp: Atom,        // "TIMESTAMP"
    pub target_sizes: Atom,     // "TARGET_SIZES"
    pub save_targets: Atom,     // "SAVE_TARGETS"
    pub delete: Atom,           // "DELETE"
    pub insert_property: Atom,  // "INSERT_PROPERTY"
    pub insert_selection: Atom, // "INSERT_SELECTION"
    pub incr: Atom,             // "INCR"
    pub atom: Atom,             // "ATOM"
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
            targets: display.intern_atom("TARGETS", false)?,
            multiple: display.intern_atom("MULTIPLE", false)?,
            timestamp: display.intern_atom("TIMESTAMP", false)?,
            target_sizes: display.intern_atom("TARGET_SIZES", false)?,
            save_targets: display.intern_atom("SAVE_TARGETS", false)?,
            delete: display.intern_atom("DELETE", false)?,
            insert_property: display.intern_atom("INSERT_PROPERTY", false)?,
            insert_selection: display.intern_atom("INSERT_SELECTION", false)?,
            incr: display.intern_atom("INCR", false)?,
            atom: display.intern_atom("ATOM", false)?,
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
