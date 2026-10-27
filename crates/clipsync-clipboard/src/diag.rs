//! `diagnose()`: Linux checks what the XWayland clipboard bridge needs and whether the compositor
//! offers data-control; Windows reports the arboard polling adapter.
#[cfg(target_os = "linux")]
use crate::adapter::{ClipError, X11Clipboard};
#[cfg(target_os = "linux")]
use crate::msg;
use clipsync_proto::{DiagItem, DiagStatus};
#[cfg(target_os = "linux")]
use std::io::{Read, Write};
#[cfg(target_os = "linux")]
use std::os::unix::net::UnixStream;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

const AREA: &str = "clipboard";

fn item(name: &str, status: DiagStatus, detail: impl Into<String>) -> DiagItem {
    DiagItem {
        area: AREA.into(),
        name: name.into(),
        status,
        detail: detail.into(),
    }
}

pub async fn diagnose() -> Vec<DiagItem> {
    tokio::task::spawn_blocking(diagnose_blocking)
        .await
        .unwrap_or_else(|_| {
            vec![item(
                "diagnóstico",
                DiagStatus::Error,
                "a verificação do clipboard falhou",
            )]
        })
}

#[cfg(windows)]
fn diagnose_blocking() -> Vec<DiagItem> {
    use crate::adapter::WinClipboard;
    let adapter = match WinClipboard::new() {
        Ok(_) => item(
            "adaptador",
            DiagStatus::Ok,
            "arboard (Windows, polling 250 ms)",
        ),
        Err(e) => item("adaptador", DiagStatus::Error, e.to_string()),
    };
    vec![
        adapter,
        item(
            "classificação",
            DiagStatus::Warn,
            "experimental: só texto, mudanças detectadas por polling",
        ),
    ]
}

#[cfg(not(any(target_os = "linux", windows)))]
fn diagnose_blocking() -> Vec<DiagItem> {
    vec![item(
        "adaptador",
        DiagStatus::Missing,
        "sem adaptador de clipboard para este sistema",
    )]
}

#[cfg(target_os = "linux")]
fn diagnose_blocking() -> Vec<DiagItem> {
    let mut out = Vec::new();
    match X11Clipboard::new() {
        Ok(_) => {
            out.push(item(
                msg::BRIDGE_ITEM,
                DiagStatus::Ok,
                "DISPLAY definido; conexão X11 aberta",
            ));
            out.push(item(
                "XFixes",
                DiagStatus::Ok,
                "SelectionNotify do CLIPBOARD disponível",
            ));
        }
        Err(ClipError::NoDisplay) => {
            out.push(item(msg::BRIDGE_ITEM, DiagStatus::Missing, msg::NO_DISPLAY));
            out.push(item(
                "XFixes",
                DiagStatus::Missing,
                "sem DISPLAY não há como verificar",
            ));
        }
        Err(e) => {
            out.push(item(
                msg::BRIDGE_ITEM,
                DiagStatus::Warn,
                "DISPLAY definido, mas a conexão falhou",
            ));
            out.push(item("XFixes", DiagStatus::Error, e.to_string()));
        }
    }
    out.push(data_control_item());
    out
}

#[cfg(target_os = "linux")]
fn data_control_item() -> DiagItem {
    const NAME: &str = "data-control (Wayland)";
    match wayland_globals() {
        Err(why) => item(NAME, DiagStatus::Warn, format!("não verificado: {why}")),
        Ok(globals) => {
            let found: Vec<&str> = globals
                .iter()
                .map(String::as_str)
                .filter(|g| {
                    *g == "zwlr_data_control_manager_v1" || *g == "ext_data_control_manager_v1"
                })
                .collect();
            if found.is_empty() {
                item(
                    NAME,
                    DiagStatus::Warn,
                    "ausente (zwlr/ext_data_control): observação só pela ponte XWayland",
                )
            } else {
                item(
                    NAME,
                    DiagStatus::Ok,
                    format!("presente: {}", found.join(", ")),
                )
            }
        }
    }
}

#[cfg(target_os = "linux")]
/// Lists the interfaces of the Wayland registry (get_registry + sync over the raw socket; std only).
fn wayland_globals() -> Result<Vec<String>, String> {
    let name =
        std::env::var("WAYLAND_DISPLAY").map_err(|_| "WAYLAND_DISPLAY indefinido".to_owned())?;
    let path = if name.starts_with('/') {
        PathBuf::from(name)
    } else {
        let dir = std::env::var("XDG_RUNTIME_DIR")
            .map_err(|_| "XDG_RUNTIME_DIR indefinido".to_owned())?;
        PathBuf::from(dir).join(name)
    };
    let mut s = UnixStream::connect(&path).map_err(|e| format!("socket Wayland: {e}"))?;
    s.set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    let msg = |obj: u32, op: u16, arg: u32| {
        let mut b = Vec::new();
        b.extend_from_slice(&obj.to_ne_bytes());
        b.extend_from_slice(&((12u32 << 16) | u32::from(op)).to_ne_bytes());
        b.extend_from_slice(&arg.to_ne_bytes());
        b
    };
    let mut req = msg(1, 1, 2); // wl_display.get_registry(new_id = 2)
    req.extend(msg(1, 0, 3)); // wl_display.sync(new_id = 3)
    s.write_all(&req).map_err(|e| e.to_string())?;

    let (mut buf, mut globals) = (Vec::new(), Vec::new());
    let mut chunk = [0u8; 4096];
    loop {
        let n = s
            .read(&mut chunk)
            .map_err(|e| format!("leitura do registry: {e}"))?;
        if n == 0 {
            return Err("conexão encerrada antes do fim do registry".into());
        }
        buf.extend_from_slice(&chunk[..n]);
        let mut pos = 0;
        while buf.len() - pos >= 8 {
            let word = |o: usize| u32::from_ne_bytes([buf[o], buf[o + 1], buf[o + 2], buf[o + 3]]);
            let (obj, hdr) = (word(pos), word(pos + 4));
            let (size, op) = ((hdr >> 16) as usize, hdr & 0xffff);
            if size < 8 || buf.len() - pos < size {
                break;
            }
            if obj == 3 && op == 0 {
                return Ok(globals); // callback.done
            }
            if obj == 2 && op == 0 && size >= 20 {
                let len = word(pos + 12) as usize;
                if len >= 1 && pos + 16 + len <= buf.len() {
                    globals.push(
                        String::from_utf8_lossy(&buf[pos + 16..pos + 16 + len - 1]).into_owned(),
                    );
                }
            }
            pos += size;
        }
        buf.drain(..pos);
    }
}
