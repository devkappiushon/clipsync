//! XWayland bridge: XFixes SelectionNotify on CLIPBOARD (x11rb) + reads via TARGETS/UTF8_STRING,
//! writes via arboard (X11 backend, no wayland-data-control). Validated on GNOME 50 / Mutter 50.1.
use super::{ChangeEvent, ClipError, ClipSnapshot, ClipboardAdapter, WatchMode};
use clipsync_proto::{MIME_TEXT_UTF8, mono_us};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xfixes::{self, ConnectionExt as _, SelectionEventMask};
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, WindowClass,
};
use x11rb::rust_connection::RustConnection;

const READ_TIMEOUT: Duration = Duration::from_secs(2);
const ZEROSIZE: &str = "application/x-zerosize";

pub struct X11Clipboard {
    conn: RustConnection,
    win: u32,
    clipboard: u32,
    targets: u32,
    utf8: u32,
    incr: u32,
    prop: u32,
    /// XFixes events that arrived while a read was waiting for its SelectionNotify.
    pending: VecDeque<ChangeEvent>,
    seq: u64,
    writer: arboard::Clipboard,
}

fn unavailable(e: impl std::fmt::Display) -> ClipError {
    ClipError::Unavailable(e.to_string())
}

impl X11Clipboard {
    pub fn new() -> Result<Self, ClipError> {
        if std::env::var_os("DISPLAY").is_none_or(|v| v.is_empty()) {
            return Err(ClipError::NoDisplay);
        }
        let (conn, screen) = RustConnection::connect(None).map_err(unavailable)?;
        let root = conn.setup().roots[screen].root;
        let win = conn.generate_id().map_err(unavailable)?;
        conn.create_window(
            0,
            win,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .map_err(unavailable)?;
        xfixes::query_version(&conn, 5, 0)
            .map_err(unavailable)?
            .reply()
            .map_err(unavailable)?;
        let atom = |name: &str| -> Result<u32, ClipError> {
            Ok(conn
                .intern_atom(false, name.as_bytes())
                .map_err(unavailable)?
                .reply()
                .map_err(unavailable)?
                .atom)
        };
        let clipboard = atom("CLIPBOARD")?;
        let (targets, utf8, incr, prop) = (
            atom("TARGETS")?,
            atom("UTF8_STRING")?,
            atom("INCR")?,
            atom("CLIPSYNC_READ")?,
        );
        conn.xfixes_select_selection_input(
            win,
            clipboard,
            SelectionEventMask::SET_SELECTION_OWNER
                | SelectionEventMask::SELECTION_WINDOW_DESTROY
                | SelectionEventMask::SELECTION_CLIENT_CLOSE,
        )
        .map_err(unavailable)?;
        conn.flush().map_err(unavailable)?;
        let writer =
            arboard::Clipboard::new().map_err(|e| ClipError::Unavailable(e.to_string()))?;
        Ok(Self {
            conn,
            win,
            clipboard,
            targets,
            utf8,
            incr,
            prop,
            pending: VecDeque::new(),
            seq: 0,
            writer,
        })
    }

    fn next_event(&mut self) -> ChangeEvent {
        self.seq += 1;
        ChangeEvent {
            seq: self.seq,
            t_mono_us: mono_us(),
        }
    }

    /// ConvertSelection to `target`; returns (type, bytes) or None when the owner refuses.
    fn convert(&mut self, target: u32) -> Result<Option<(u32, Vec<u8>)>, ClipError> {
        self.conn
            .delete_property(self.win, self.prop)
            .map_err(unavailable)?;
        self.conn
            .convert_selection(self.win, self.clipboard, target, self.prop, 0u32)
            .map_err(unavailable)?;
        self.conn.flush().map_err(unavailable)?;
        let end = Instant::now() + READ_TIMEOUT;
        while Instant::now() < end {
            match self.conn.poll_for_event().map_err(unavailable)? {
                Some(Event::SelectionNotify(e)) => {
                    if e.property == 0u32 {
                        return Ok(None);
                    }
                    let r = self
                        .conn
                        .get_property(true, self.win, self.prop, AtomEnum::ANY, 0, u32::MAX / 4)
                        .map_err(unavailable)?
                        .reply()
                        .map_err(unavailable)?;
                    return Ok(Some((r.type_, r.value)));
                }
                Some(Event::XfixesSelectionNotify(_)) => {
                    let ev = self.next_event();
                    self.pending.push_back(ev);
                }
                Some(_) => {}
                None => std::thread::sleep(Duration::from_millis(1)),
            }
        }
        Err(ClipError::Busy)
    }

    fn atom_name(&self, a: u32) -> String {
        self.conn
            .get_atom_name(a)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| String::from_utf8_lossy(&r.name).into_owned())
            .unwrap_or_default()
    }
}

impl ClipboardAdapter for X11Clipboard {
    fn name(&self) -> &'static str {
        "x11-xfixes"
    }
    fn watch_mode(&self) -> WatchMode {
        WatchMode::Events
    }
    fn wait_change(&mut self, timeout: Duration) -> Result<Option<ChangeEvent>, ClipError> {
        if let Some(ev) = self.pending.pop_front() {
            return Ok(Some(ev));
        }
        let end = Instant::now() + timeout;
        loop {
            match self.conn.poll_for_event().map_err(unavailable)? {
                Some(Event::XfixesSelectionNotify(_)) => return Ok(Some(self.next_event())),
                Some(_) => continue,
                None if Instant::now() >= end => return Ok(None),
                None => std::thread::sleep(Duration::from_millis(2)),
            }
        }
    }
    fn read_text(&mut self) -> Result<Option<ClipSnapshot>, ClipError> {
        let Some((_, raw)) = self.convert(self.targets)? else {
            return Ok(None);
        };
        let names: Vec<String> = raw
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| self.atom_name(u32::from_ne_bytes(*c)))
            .collect();
        let has_text = names.iter().any(|n| n == "UTF8_STRING");
        if names.is_empty()
            || !has_text
            || names
                .iter()
                .all(|n| n == ZEROSIZE || n == "TARGETS" || n == "TIMESTAMP")
        {
            return Ok(None);
        }
        let Some((ty, bytes)) = self.convert(self.utf8)? else {
            return Ok(None);
        };
        if ty == self.incr {
            return Err(ClipError::Unsupported("INCR transfer"));
        }
        if bytes.is_empty() {
            return Ok(None);
        }
        // Not valid UTF-8 means not text we can carry: treat as "no text".
        Ok(String::from_utf8(bytes).ok().map(|text| ClipSnapshot {
            mime: MIME_TEXT_UTF8.into(),
            text,
        }))
    }
    fn write_text(&mut self, text: &str) -> Result<(), ClipError> {
        self.writer
            .set_text(text)
            .map_err(|e| ClipError::Other(e.to_string()))
    }
}
