//! The only file of the repository with `unsafe` (contract §13.3 item 3). Everything here is a safe
//! function that closes its invariant inside the block; nothing `unsafe` leaves this module
//! (Sentinel C2). The callbacks (`extern "system"`) stay private.
//!
//! Rules this file keeps:
//! - callback state lives in `thread_local!` cells on the thread that runs the message loop, never in
//!   a `static mut`, a `Box::into_raw` or `GWLP_USERDATA` (C3);
//! - every callback body runs inside `catch_unwind`: a panic passes the event and sends
//!   `Notice::HooksLost` once (C3);
//! - every handle is closed by a `Drop` (C4);
//! - no key, scan code or character reaches a log: nothing here logs at all (§13.2 rule 2e).
#![allow(unsafe_code)]

use crate::{Modifier, Monitor, Notice, RawEvent, Synth, Verdict, WinButton, WinError, WinKey};
use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows::Win32::Foundation::{
    COLORREF, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT,
    POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTS_CURRENT_SESSION, WTSFreeMemory, WTSINFOEX_LEVEL1_W, WTSINFOEXW,
    WTSQuerySessionInformationW, WTSRegisterSessionNotification, WTSSessionInfoEx,
    WTSUnRegisterSessionNotification,
};
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_READOBJECTS, GetUserObjectInformationW, HDESK,
    OpenInputDesktop, UOI_NAME,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThreadId, GetProcessHandleCount,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor,
    MDT_EFFECTIVE_DPI, SetThreadDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyboardLayoutNameW, HOT_KEY_MODIFIERS, INPUT, INPUT_0, INPUT_KEYBOARD,
    INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, MOD_ALT, MOD_NOREPEAT, MOD_WIN, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE,
    MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
    RegisterHotKey, SendInput, UnregisterHotKey, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateCursor, CreateWindowExW, DefWindowProcW, DestroyCursor, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics, HCURSOR, HHOOK, HOOKPROC,
    KBDLLHOOKSTRUCT, KillTimer, LLKHF_EXTENDED, LLKHF_INJECTED, LLKHF_LOWER_IL_INJECTED, LLKHF_UP,
    LLMHF_INJECTED, LLMHF_LOWER_IL_INJECTED, LWA_ALPHA, MSG, MSLLHOOKSTRUCT, PostThreadMessageW,
    RegisterClassW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    SW_SHOWNOACTIVATE, SetCursorPos, SetLayeredWindowAttributes, SetTimer, SetWindowsHookExW,
    ShowWindow, UnhookWindowsHookEx, UnregisterClassW, WH_KEYBOARD_LL, WH_MOUSE_LL,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_DISPLAYCHANGE, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER,
    WM_WTSSESSION_CHANGE, WM_XBUTTONDOWN, WM_XBUTTONUP, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPED, WS_POPUP,
};
use windows::core::{PCWSTR, PWSTR};

fn werr(call: &'static str, e: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: e.code().0 as u32,
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn utf16_until_nul(units: &[u16]) -> String {
    let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

// ---------------------------------------------------------------------------------------------
// DPI

/// Puts the calling thread in per-monitor-v2 awareness and puts the previous context back on drop
/// (contract §13.5: the process does not change).
struct DpiGuard(DPI_AWARENESS_CONTEXT);

impl DpiGuard {
    fn enter() -> DpiGuard {
        // SAFETY: the constant is one of the documented pseudo-handles; the call only changes the
        // awareness of the calling thread and returns its previous context.
        let old =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        DpiGuard(old)
    }
}

impl Drop for DpiGuard {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the context the system returned for this very thread in `enter`.
        let _ = unsafe { SetThreadDpiAwarenessContext(self.0) };
    }
}

// ---------------------------------------------------------------------------------------------
// Simple queries

pub(crate) fn virtual_screen() -> (i32, i32, i32, i32) {
    let _dpi = DpiGuard::enter();
    // SAFETY: GetSystemMetrics takes an index and no pointer.
    let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    // SAFETY: as above.
    let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    // SAFETY: as above.
    let w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    // SAFETY: as above.
    let h = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    (x, y, w, h)
}

pub(crate) fn cursor_pos() -> Result<(i32, i32), WinError> {
    let _dpi = DpiGuard::enter();
    let mut point = POINT::default();
    // SAFETY: `point` is a live, writable POINT for the whole call.
    unsafe { GetCursorPos(&mut point) }.map_err(|e| werr("GetCursorPos", &e))?;
    Ok((point.x, point.y))
}

pub(crate) fn keyboard_layout() -> Option<String> {
    let mut buffer = [0u16; 9];
    // SAFETY: the API takes exactly a `[u16; 9]` (KL_NAMELENGTH) and writes a NUL-terminated name.
    unsafe { GetKeyboardLayoutNameW(&mut buffer) }.ok()?;
    Some(utf16_until_nul(&buffer))
}

/// Whether a virtual key is physically down right now. Private on purpose: the public API only
/// reads modifiers and mouse buttons (Sentinel S7-01), never a letter.
fn async_down(vk: i32) -> bool {
    // SAFETY: GetAsyncKeyState takes a virtual-key code and no pointer.
    let state = unsafe { GetAsyncKeyState(vk) };
    state < 0
}

pub(crate) fn modifier_down(modifier: Modifier) -> bool {
    async_down(match modifier {
        Modifier::LeftShift => 0xA0,
        Modifier::RightShift => 0xA1,
        Modifier::LeftCtrl => 0xA2,
        Modifier::RightCtrl => 0xA3,
        Modifier::LeftAlt => 0xA4,
        Modifier::RightAlt => 0xA5,
        Modifier::LeftWin => 0x5B,
        Modifier::RightWin => 0x5C,
    })
}

pub(crate) fn button_down(button: WinButton) -> bool {
    async_down(match button {
        WinButton::Left => 0x01,
        WinButton::Right => 0x02,
        WinButton::Middle => 0x04,
        WinButton::X1 => 0x05,
        WinButton::X2 => 0x06,
    })
}

pub(crate) fn process_handle_count() -> Result<u32, WinError> {
    let mut count = 0u32;
    // SAFETY: GetCurrentProcess returns a pseudo-handle that needs no closing and takes no argument.
    let process = unsafe { GetCurrentProcess() };
    // SAFETY: `process` is the pseudo-handle above and `count` is a live, writable u32.
    unsafe { GetProcessHandleCount(process, &mut count) }
        .map_err(|e| werr("GetProcessHandleCount", &e))?;
    Ok(count)
}

/// Closes the input desktop when it goes out of scope.
struct DesktopGuard(HDESK);

impl Drop for DesktopGuard {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful OpenInputDesktop and is closed exactly once.
        let _ = unsafe { CloseDesktop(self.0) };
    }
}

pub(crate) fn input_desktop_is_default() -> bool {
    // SAFETY: no pointer arguments; the access mask asks only to read the object's name.
    let opened = unsafe { OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS) };
    let Ok(desktop) = opened else {
        return false;
    };
    let guard = DesktopGuard(desktop);
    let mut name = [0u16; 64];
    let mut needed = 0u32;
    // SAFETY: `guard.0` is a live desktop handle opened with DESKTOP_READOBJECTS; `name` is a
    // writable buffer of exactly `size_of_val(&name)` bytes and `needed` a writable u32.
    let got = unsafe {
        GetUserObjectInformationW(
            HANDLE(guard.0.0),
            UOI_NAME,
            Some(name.as_mut_ptr().cast()),
            std::mem::size_of_val(&name) as u32,
            Some(&mut needed),
        )
    };
    got.is_ok() && utf16_until_nul(&name).eq_ignore_ascii_case("Default")
}

/// Owns the allocation returned by WTS, including early-error paths after the query.
struct WtsBuffer(PWSTR);
impl Drop for WtsBuffer {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this non-null allocation is returned by WTSQuerySessionInformationW,
            // owned solely by this guard and freed exactly once with its matching API.
            unsafe { WTSFreeMemory(self.0.0.cast()) };
        }
    }
}

pub(crate) fn session_locked() -> Result<bool, WinError> {
    let mut buffer = PWSTR::null();
    let mut bytes = 0;
    // SAFETY: local/current session only; buffer and bytes are initialized writable outputs.
    // The returned allocation is immediately adopted by WtsBuffer and never escapes this call.
    let result = unsafe {
        WTSQuerySessionInformationW(
            None,
            WTS_CURRENT_SESSION,
            WTSSessionInfoEx,
            &mut buffer,
            &mut bytes,
        )
    };
    let guard = WtsBuffer(buffer);
    result.map_err(|e| werr("WTSQuerySessionInformationW", &e))?;
    if guard.0.is_null() {
        return Err(crate::invalid_wts_info());
    }
    crate::validate_wts_size(bytes, std::mem::size_of::<WTSINFOEXW>())?;
    let base = guard.0.0.cast::<u8>();
    let level_pointer = base
        .wrapping_add(std::mem::offset_of!(WTSINFOEXW, Level))
        .cast::<u32>();
    // SAFETY: the successful allocation is non-null and at least size_of<WTSINFOEXW>();
    // Level is an initialized u32 field at this offset. Unaligned read avoids alignment assumptions.
    let level = unsafe { level_pointer.read_unaligned() };
    if level != 1 {
        return Err(crate::invalid_wts_info());
    }
    let offset = std::mem::offset_of!(WTSINFOEXW, Data)
        + std::mem::offset_of!(WTSINFOEX_LEVEL1_W, SessionFlags);
    let flags_pointer = base.wrapping_add(offset).cast::<i32>();
    // SAFETY: Level==1 selects WTSINFOEX_LEVEL1_W. The validated allocation contains its
    // initialized SessionFlags field at this offset; no account/name/padding fields are read.
    let flags = unsafe { flags_pointer.read_unaligned() };
    crate::decode_wts_lock(level, flags)
}

// ---------------------------------------------------------------------------------------------
// Monitors

thread_local! {
    static ENUMERATED: RefCell<Vec<HMONITOR>> = const { RefCell::new(Vec::new()) };
}

unsafe extern "system" fn enum_monitor(
    monitor: HMONITOR,
    _dc: HDC,
    _rect: *mut RECT,
    _data: LPARAM,
) -> windows::core::BOOL {
    let _ = catch_unwind(|| {
        ENUMERATED.with(|list| {
            if let Ok(mut list) = list.try_borrow_mut() {
                list.push(monitor);
            }
        });
    });
    true.into()
}

/// GDI device name (`\\.\DISPLAY1`) to the friendly name and the monitor path of `QueryDisplayConfig`.
fn display_names() -> Vec<(String, String, String)> {
    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = Vec::new();
    for _ in 0..3 {
        let (mut n_paths, mut n_modes) = (0u32, 0u32);
        // SAFETY: both counters are live, writable u32.
        let sizes = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut n_paths, &mut n_modes)
        };
        if sizes != ERROR_SUCCESS {
            return Vec::new();
        }
        paths = vec![DISPLAYCONFIG_PATH_INFO::default(); n_paths as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); n_modes as usize];
        // SAFETY: the arrays have `n_paths` and `n_modes` elements, the counters say so, and the
        // topology pointer is optional (None).
        let query = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut n_paths,
                paths.as_mut_ptr(),
                &mut n_modes,
                modes.as_mut_ptr(),
                None,
            )
        };
        if query == ERROR_SUCCESS {
            paths.truncate(n_paths as usize);
            break;
        }
        paths.clear();
        if query != ERROR_INSUFFICIENT_BUFFER {
            return Vec::new();
        }
    }

    let mut names = Vec::new();
    for path in &paths {
        let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
        source.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
        source.header.size = std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
        source.header.adapterId = path.sourceInfo.adapterId;
        source.header.id = path.sourceInfo.id;
        // SAFETY: the API writes the whole request, not only the header: the pointer is to the whole live
        // struct, whose header (its first field, `repr(C)`) says its `size`.
        if unsafe { DisplayConfigGetDeviceInfo(std::ptr::from_mut(&mut source).cast()) } != 0 {
            continue;
        }
        let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
        target.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
        target.header.size = std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
        target.header.adapterId = path.targetInfo.adapterId;
        target.header.id = path.targetInfo.id;
        // SAFETY: as above, for the target request.
        if unsafe { DisplayConfigGetDeviceInfo(std::ptr::from_mut(&mut target).cast()) } != 0 {
            continue;
        }
        names.push((
            utf16_until_nul(&source.viewGdiDeviceName),
            utf16_until_nul(&target.monitorFriendlyDeviceName),
            utf16_until_nul(&target.monitorDevicePath),
        ));
    }
    names
}

pub(crate) fn monitors() -> Result<Vec<Monitor>, WinError> {
    let _dpi = DpiGuard::enter();
    ENUMERATED.with(|list| list.borrow_mut().clear());
    // SAFETY: a null DC and no clip rectangle enumerate every monitor; `enum_monitor` is a valid
    // callback that only touches a thread-local list and ignores `data`.
    let ok = unsafe { EnumDisplayMonitors(None, None, Some(enum_monitor), LPARAM(0)) };
    if !ok.as_bool() {
        return Err(WinError {
            call: "EnumDisplayMonitors",
            code: 0,
        });
    }
    let handles = ENUMERATED.with(|list| std::mem::take(&mut *list.borrow_mut()));
    let names = display_names();

    let mut out = Vec::with_capacity(handles.len());
    for handle in handles {
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        // SAFETY: `info` is a live MONITORINFOEXW whose `cbSize` says so; the API takes the
        // MONITORINFO header at its start, which is the first field.
        let got =
            unsafe { GetMonitorInfoW(handle, std::ptr::from_mut(&mut info).cast::<MONITORINFO>()) };
        if !got.as_bool() {
            continue;
        }
        let rect = info.monitorInfo.rcMonitor;
        let device = utf16_until_nul(&info.szDevice);
        let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
        // SAFETY: `handle` came from the enumeration above; both outputs are live, writable u32.
        let dpi = unsafe { GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) }
            .map_or(96, |()| dpi_x);
        let (friendly, path) = names
            .iter()
            .find(|(gdi, _, _)| *gdi == device)
            .map(|(_, friendly, path)| (friendly.clone(), path.clone()))
            .unwrap_or_default();
        out.push(Monitor {
            friendly,
            path: if path.is_empty() {
                device.clone()
            } else {
                path
            },
            device,
            x: rect.left,
            y: rect.top,
            width: (rect.right - rect.left).max(0) as u32,
            height: (rect.bottom - rect.top).max(0) as u32,
            dpi,
            primary: info.monitorInfo.dwFlags & 1 != 0,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// SendInput

fn mouse_input(dx: i32, dy: i32, data: u32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn key_input(key: WinKey, up: bool) -> INPUT {
    let mut flags = if up {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };
    let (vk, scan) = match key {
        WinKey::Scan { code, extended } => {
            flags |= KEYEVENTF_SCANCODE;
            if extended {
                flags |= KEYEVENTF_EXTENDEDKEY;
            }
            (0, u16::from(code))
        }
        WinKey::Vk(vk) => (vk, 0),
    };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn button_input(button: WinButton, down: bool) -> INPUT {
    let (flags, data) = match (button, down) {
        (WinButton::Left, true) => (MOUSEEVENTF_LEFTDOWN, 0),
        (WinButton::Left, false) => (MOUSEEVENTF_LEFTUP, 0),
        (WinButton::Right, true) => (MOUSEEVENTF_RIGHTDOWN, 0),
        (WinButton::Right, false) => (MOUSEEVENTF_RIGHTUP, 0),
        (WinButton::Middle, true) => (MOUSEEVENTF_MIDDLEDOWN, 0),
        (WinButton::Middle, false) => (MOUSEEVENTF_MIDDLEUP, 0),
        (WinButton::X1, true) => (MOUSEEVENTF_XDOWN, 1),
        (WinButton::X1, false) => (MOUSEEVENTF_XUP, 1),
        (WinButton::X2, true) => (MOUSEEVENTF_XDOWN, 2),
        (WinButton::X2, false) => (MOUSEEVENTF_XUP, 2),
    };
    mouse_input(0, 0, data, flags)
}

pub(crate) fn send(inputs: &[Synth]) -> Result<usize, WinError> {
    let built: Vec<INPUT> = inputs
        .iter()
        .map(|synth| match *synth {
            Synth::MoveAbs { nx, ny } => mouse_input(
                nx,
                ny,
                0,
                MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
            ),
            Synth::Button { button, down } => button_input(button, down),
            Synth::Wheel { delta, horizontal } => mouse_input(
                0,
                0,
                delta as u32,
                if horizontal {
                    MOUSEEVENTF_HWHEEL
                } else {
                    MOUSEEVENTF_WHEEL
                },
            ),
            Synth::Key { key, up } => key_input(key, up),
        })
        .collect();
    if built.is_empty() {
        return Ok(0);
    }
    // SAFETY: `built` is a live slice of fully initialized INPUT structs, and the size argument is
    // the size of one INPUT, as the API requires.
    let sent = unsafe { SendInput(&built, std::mem::size_of::<INPUT>() as i32) };
    // Fewer inputs than asked is a failure too (a blocked input, for instance): the caller must not
    // believe a half batch went through.
    if sent as usize != built.len() {
        return Err(WinError {
            call: "SendInput",
            code: last_error(),
        });
    }
    Ok(built.len())
}

fn last_error() -> u32 {
    windows::core::Error::from_thread().code().0 as u32
}

// ---------------------------------------------------------------------------------------------
// The thread with a message loop (hooks, watcher, cursor park, resume hotkey)

/// A thread that runs a message loop. Dropping it posts `WM_QUIT` and waits for the thread to clean
/// up, unless the drop happens on that very thread (a callback dropping its own `Hooks`): then it
/// only posts the quit.
pub(crate) struct ThreadHandle {
    id: u32,
    join: Option<JoinHandle<()>>,
}

impl ThreadHandle {
    fn start<F>(name: &'static str, body: F) -> Result<ThreadHandle, WinError>
    where
        F: FnOnce(&mpsc::Sender<Result<u32, WinError>>) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let join = thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || body(&tx))
            .map_err(|_| WinError {
                call: "thread spawn",
                code: 0,
            })?;
        match rx.recv() {
            Ok(Ok(id)) => Ok(ThreadHandle {
                id,
                join: Some(join),
            }),
            Ok(Err(e)) => {
                let _ = join.join();
                Err(e)
            }
            Err(_) => {
                let _ = join.join();
                Err(WinError {
                    call: "thread init",
                    code: 0,
                })
            }
        }
    }
}

impl Drop for ThreadHandle {
    fn drop(&mut self) {
        // SAFETY: posting WM_QUIT to a thread id has no pointer arguments; if the thread is gone the
        // call fails and nothing happens.
        let _ = unsafe { PostThreadMessageW(self.id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        if let Some(join) = self.join.take() {
            // SAFETY: GetCurrentThreadId takes no argument.
            let me = unsafe { GetCurrentThreadId() };
            if me != self.id {
                let _ = join.join();
            }
        }
    }
}

fn pump_messages(mut on_message: impl FnMut(&MSG)) {
    let mut msg = MSG::default();
    loop {
        // SAFETY: `msg` is a live, writable MSG; a null window means this thread's messages.
        let got = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if got.0 <= 0 {
            break;
        }
        on_message(&msg);
        // SAFETY: `msg` was filled by GetMessageW just above.
        let _ = unsafe { DispatchMessageW(&msg) };
    }
}

fn module_instance() -> Result<HINSTANCE, WinError> {
    // SAFETY: a null module name asks for the handle of the running executable.
    let module = unsafe { GetModuleHandleW(None) }.map_err(|e| werr("GetModuleHandleW", &e))?;
    Ok(HINSTANCE(module.0))
}

static CLASS_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A window class registered for one window and removed on drop.
struct WindowClass {
    name: Vec<u16>,
    instance: HINSTANCE,
}

impl WindowClass {
    fn register(
        prefix: &str,
        instance: HINSTANCE,
        proc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
        cursor: HCURSOR,
    ) -> Result<WindowClass, WinError> {
        let n = CLASS_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = wide(&format!("{prefix}.{n}"));
        let class = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance,
            hCursor: cursor,
            lpszClassName: PCWSTR(name.as_ptr()),
            ..Default::default()
        };
        // SAFETY: `class` is fully initialized; `name` outlives the call and is NUL-terminated.
        let atom = unsafe { RegisterClassW(&class) };
        if atom == 0 {
            return Err(WinError {
                call: "RegisterClassW",
                code: last_error(),
            });
        }
        Ok(WindowClass { name, instance })
    }

    fn name(&self) -> PCWSTR {
        PCWSTR(self.name.as_ptr())
    }
}

impl Drop for WindowClass {
    fn drop(&mut self) {
        // SAFETY: the class was registered by `register` with this name and instance, and its window
        // (declared after it, so dropped before it) is already destroyed.
        let _ = unsafe { UnregisterClassW(self.name(), Some(self.instance)) };
    }
}

struct Window(HWND);

impl Window {
    fn create(
        class: &WindowClass,
        ex_style: WINDOW_EX_STYLE,
        style: WINDOW_STYLE,
        rect: (i32, i32, i32, i32),
    ) -> Result<Window, WinError> {
        // SAFETY: the class is registered and alive; the title is a static empty string; no parent,
        // menu or creation data.
        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                class.name(),
                windows::core::w!(""),
                style,
                rect.0,
                rect.1,
                rect.2,
                rect.3,
                None,
                None,
                Some(class.instance),
                None,
            )
        }
        .map_err(|e| werr("CreateWindowExW", &e))?;
        Ok(Window(hwnd))
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // SAFETY: the window was created by this thread and is destroyed exactly once, here.
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

// ----- the callback state (C3): thread-local, on the thread that runs the loop

type InputFn = Box<dyn FnMut(&RawEvent) -> Verdict + Send>;
type NoticeFn = Box<dyn FnMut(Notice) + Send>;

struct Callbacks {
    on_input: Option<InputFn>,
    on_notice: NoticeFn,
    lost: bool,
}

thread_local! {
    static CALLBACKS: RefCell<Option<Callbacks>> = const { RefCell::new(None) };
    static DESKTOP_WAS_DEFAULT: Cell<Option<bool>> = const { Cell::new(None) };
    static TICKS: Cell<u32> = const { Cell::new(0) };
}

/// Where the cursor is parked while a capture is on: `(x << 32) | y` as `u32` pairs, or `NO_PARK`.
/// The hooks thread reads it every second (I-S7-3); `CursorPark` writes it.
static PARKED: AtomicU64 = AtomicU64::new(NO_PARK);
const NO_PARK: u64 = u64::MAX;

fn pack(x: i32, y: i32) -> u64 {
    (u64::from(x as u32) << 32) | u64::from(y as u32)
}

fn notice(n: Notice) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        CALLBACKS.with(|cell| {
            if let Ok(mut guard) = cell.try_borrow_mut()
                && let Some(callbacks) = guard.as_mut()
            {
                (callbacks.on_notice)(n);
            }
        });
    }));
}

/// A callback failed: the hook is dead (passes everything) and the session has to end. Once.
fn mark_lost() {
    let first = CALLBACKS
        .try_with(|cell| {
            cell.try_borrow_mut().is_ok_and(|mut guard| {
                guard.as_mut().is_some_and(|callbacks| {
                    let first = !callbacks.lost;
                    callbacks.lost = true;
                    first
                })
            })
        })
        .unwrap_or(false);
    if first {
        notice(Notice::HooksLost);
    }
}

fn dispatch_input(event: &RawEvent) -> Verdict {
    CALLBACKS
        .try_with(|cell| match cell.try_borrow_mut() {
            Ok(mut guard) => match guard.as_mut() {
                Some(Callbacks {
                    on_input: Some(on_input),
                    lost: false,
                    ..
                }) => on_input(event),
                _ => Verdict::Pass,
            },
            Err(_) => Verdict::Pass,
        })
        .unwrap_or(Verdict::Pass)
}

fn pass_on(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: forwards the exact arguments this hook procedure received; the hook handle may be
    // None, as the documentation allows.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn finish(
    outcome: std::thread::Result<Verdict>,
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match outcome {
        Ok(Verdict::Block) => LRESULT(1),
        Ok(Verdict::Pass) => pass_on(code, wparam, lparam),
        Err(_) => {
            mark_lost();
            pass_on(code, wparam, lparam)
        }
    }
}

fn keyboard_verdict(code: i32, wparam: WPARAM, lparam: LPARAM) -> Verdict {
    if code < 0 || lparam.0 == 0 {
        return Verdict::Pass;
    }
    // SAFETY: for WH_KEYBOARD_LL with a non-negative code, `lparam` points to a KBDLLHOOKSTRUCT that
    // the system keeps alive for the duration of this callback; it is copied out here.
    let info = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let flags = info.flags;
    if flags.contains(LLKHF_INJECTED) || flags.contains(LLKHF_LOWER_IL_INJECTED) {
        return Verdict::Pass;
    }
    let message = wparam.0 as u32;
    if !matches!(message, WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP) {
        return Verdict::Pass;
    }
    dispatch_input(&RawEvent::Key {
        vk: info.vkCode,
        scan: info.scanCode,
        extended: flags.contains(LLKHF_EXTENDED),
        up: flags.contains(LLKHF_UP),
    })
}

fn mouse_verdict(code: i32, wparam: WPARAM, lparam: LPARAM) -> Verdict {
    if code < 0 || lparam.0 == 0 {
        return Verdict::Pass;
    }
    // SAFETY: for WH_MOUSE_LL with a non-negative code, `lparam` points to an MSLLHOOKSTRUCT that the
    // system keeps alive for the duration of this callback; it is copied out here.
    let info = unsafe { *(lparam.0 as *const MSLLHOOKSTRUCT) };
    if info.flags & (LLMHF_INJECTED | LLMHF_LOWER_IL_INJECTED) != 0 {
        return Verdict::Pass;
    }
    let high_word = (info.mouseData >> 16) as u16;
    let button = |button, down| Some(RawEvent::Button { button, down });
    let x_button = |down| match high_word {
        1 => button(WinButton::X1, down),
        2 => button(WinButton::X2, down),
        _ => None,
    };
    let event = match wparam.0 as u32 {
        WM_MOUSEMOVE => Some(RawEvent::Move {
            x: info.pt.x,
            y: info.pt.y,
        }),
        WM_LBUTTONDOWN => button(WinButton::Left, true),
        WM_LBUTTONUP => button(WinButton::Left, false),
        WM_RBUTTONDOWN => button(WinButton::Right, true),
        WM_RBUTTONUP => button(WinButton::Right, false),
        WM_MBUTTONDOWN => button(WinButton::Middle, true),
        WM_MBUTTONUP => button(WinButton::Middle, false),
        WM_XBUTTONDOWN => x_button(true),
        WM_XBUTTONUP => x_button(false),
        WM_MOUSEWHEEL => Some(RawEvent::Wheel {
            delta: i32::from(high_word as i16),
            horizontal: false,
        }),
        WM_MOUSEHWHEEL => Some(RawEvent::Wheel {
            delta: i32::from(high_word as i16),
            horizontal: true,
        }),
        _ => None,
    };
    event.map_or(Verdict::Pass, |event| dispatch_input(&event))
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let outcome = catch_unwind(AssertUnwindSafe(|| keyboard_verdict(code, wparam, lparam)));
    finish(outcome, code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let outcome = catch_unwind(AssertUnwindSafe(|| mouse_verdict(code, wparam, lparam)));
    finish(outcome, code, wparam, lparam)
}

const DESKTOP_TIMER: usize = 1;
const DESKTOP_POLL_MS: u32 = 500;
const TICKS_PER_CURSOR_CHECK: u32 = 2;

/// The 500 ms clock: tells when the input desktop changes (the system does not say) and, with a
/// capture on, whether the hooks are still alive (I-S7-3: the system removes a slow hook silently,
/// and the cursor then leaves the parked point).
fn tick(check_cursor: bool) {
    let now = input_desktop_is_default();
    let before = DESKTOP_WAS_DEFAULT.with(|c| c.replace(Some(now)));
    if before.is_some_and(|was| was != now) {
        notice(Notice::DesktopSwitched);
    }
    if !check_cursor {
        return;
    }
    let count = TICKS.with(|c| {
        c.set(c.get().wrapping_add(1));
        c.get()
    });
    if !count.is_multiple_of(TICKS_PER_CURSOR_CHECK) {
        return;
    }
    let parked = PARKED.load(Ordering::Acquire);
    if parked == NO_PARK {
        return;
    }
    // Compared in the same per-monitor space the park was made in.
    let moved = cursor_pos().is_ok_and(|(x, y)| pack(x, y) != parked);
    if moved {
        mark_lost();
    }
}

fn window_message(msg: u32, wparam: WPARAM, check_cursor: bool) {
    match msg {
        WM_DISPLAYCHANGE => notice(Notice::DisplayChanged),
        WM_WTSSESSION_CHANGE => match wparam.0 as u32 {
            windows::Win32::UI::WindowsAndMessaging::WTS_SESSION_LOCK => {
                notice(Notice::SessionLocked)
            }
            windows::Win32::UI::WindowsAndMessaging::WTS_SESSION_UNLOCK => {
                notice(Notice::SessionUnlocked);
            }
            _ => {}
        },
        WM_TIMER if wparam.0 == DESKTOP_TIMER => tick(check_cursor),
        _ => {}
    }
}

unsafe extern "system" fn watcher_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let handled = catch_unwind(AssertUnwindSafe(|| {
        window_message(msg, wparam, CHECK_CURSOR.with(Cell::get));
    }));
    if handled.is_err() {
        mark_lost();
    }
    // SAFETY: forwards the exact arguments this window procedure received.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

thread_local! {
    static CHECK_CURSOR: Cell<bool> = const { Cell::new(false) };
}

/// Everything the hooks thread owns, in the order it has to be released.
struct WatcherResources {
    mouse: Option<HHOOK>,
    keyboard: Option<HHOOK>,
    timer: Option<HWND>,
    wts: Option<HWND>,
    // Declared in drop order after the fields above: window, then class.
    window: Option<Window>,
    class: Option<WindowClass>,
}

impl Drop for WatcherResources {
    fn drop(&mut self) {
        if let Some(hook) = self.keyboard.take() {
            // SAFETY: the hook was installed by this thread and is removed exactly once.
            let _ = unsafe { UnhookWindowsHookEx(hook) };
        }
        if let Some(hook) = self.mouse.take() {
            // SAFETY: as above.
            let _ = unsafe { UnhookWindowsHookEx(hook) };
        }
        if let Some(hwnd) = self.timer.take() {
            // SAFETY: the timer was set on this window with this id.
            let _ = unsafe { KillTimer(Some(hwnd), DESKTOP_TIMER) };
        }
        if let Some(hwnd) = self.wts.take() {
            // SAFETY: the window was registered for session notifications and is still alive.
            let _ = unsafe { WTSUnRegisterSessionNotification(hwnd) };
        }
        self.window.take();
        self.class.take();
        CALLBACKS.with(|cell| {
            if let Ok(mut guard) = cell.try_borrow_mut() {
                *guard = None;
            }
        });
    }
}

fn install_hook(
    id: windows::Win32::UI::WindowsAndMessaging::WINDOWS_HOOK_ID,
    proc: HOOKPROC,
    instance: HINSTANCE,
    call: &'static str,
) -> Result<HHOOK, WinError> {
    // SAFETY: `proc` is one of this module's hook procedures (valid for the life of the process); the
    // module handle is this executable; thread id 0 means every thread, as WH_*_LL requires.
    unsafe { SetWindowsHookExW(id, proc, Some(instance), 0) }.map_err(|e| werr(call, &e))
}

fn watcher_thread(
    on_input: Option<InputFn>,
    on_notice: NoticeFn,
    init: &mpsc::Sender<Result<u32, WinError>>,
) {
    let _dpi = DpiGuard::enter();
    let with_hooks = on_input.is_some();
    CHECK_CURSOR.with(|c| c.set(with_hooks));
    CALLBACKS.with(|cell| {
        *cell.borrow_mut() = Some(Callbacks {
            on_input,
            on_notice,
            lost: false,
        });
    });

    let mut resources = WatcherResources {
        mouse: None,
        keyboard: None,
        timer: None,
        wts: None,
        window: None,
        class: None,
    };
    let mut setup = || -> Result<(), WinError> {
        let instance = module_instance()?;
        let class = WindowClass::register(
            "ClipSyncWin.Watcher",
            instance,
            watcher_proc,
            HCURSOR::default(),
        )?;
        // A top-level window that is never shown: a message-only window does not get the
        // WM_DISPLAYCHANGE broadcast.
        let window = Window::create(&class, WINDOW_EX_STYLE(0), WS_OVERLAPPED, (0, 0, 0, 0))?;
        let hwnd = window.0;
        resources.class = Some(class);
        resources.window = Some(window);
        // SAFETY: `hwnd` is the live window just created, owned by this thread.
        unsafe { WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) }
            .map_err(|e| werr("WTSRegisterSessionNotification", &e))?;
        resources.wts = Some(hwnd);
        // SAFETY: `hwnd` is live; no timer callback (the window procedure gets WM_TIMER).
        let timer = unsafe { SetTimer(Some(hwnd), DESKTOP_TIMER, DESKTOP_POLL_MS, None) };
        if timer == 0 {
            return Err(WinError {
                call: "SetTimer",
                code: last_error(),
            });
        }
        resources.timer = Some(hwnd);
        DESKTOP_WAS_DEFAULT.with(|c| c.set(Some(input_desktop_is_default())));
        if with_hooks {
            resources.mouse = Some(install_hook(
                WH_MOUSE_LL,
                Some(mouse_proc),
                instance,
                "SetWindowsHookExW(mouse)",
            )?);
            resources.keyboard = Some(install_hook(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                instance,
                "SetWindowsHookExW(keyboard)",
            )?);
        }
        Ok(())
    };
    match setup() {
        Err(e) => {
            let _ = init.send(Err(e));
            return;
        }
        Ok(()) => {
            // SAFETY: GetCurrentThreadId takes no argument.
            let id = unsafe { GetCurrentThreadId() };
            if init.send(Ok(id)).is_err() {
                return;
            }
        }
    }
    pump_messages(|_| {});
    drop(resources);
}

pub(crate) fn start_watcher(
    on_input: Option<InputFn>,
    on_notice: NoticeFn,
) -> Result<ThreadHandle, WinError> {
    ThreadHandle::start("clipsync-win-hooks", move |init| {
        watcher_thread(on_input, on_notice, init)
    })
}

// ---------------------------------------------------------------------------------------------
// CursorPark

unsafe extern "system" fn park_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: forwards the exact arguments this window procedure received; it has no state of its own.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

struct CursorResource(HCURSOR);

impl Drop for CursorResource {
    fn drop(&mut self) {
        // SAFETY: the cursor was created by CreateCursor in this thread and is destroyed once.
        let _ = unsafe { DestroyCursor(self.0) };
    }
}

fn blank_cursor(instance: HINSTANCE) -> Result<CursorResource, WinError> {
    // A 32x32 monochrome cursor whose AND mask is all ones and XOR mask all zeros: fully transparent.
    let and_plane = [0xFFu8; 128];
    let xor_plane = [0u8; 128];
    // SAFETY: both planes are 32x32 1-bit bitmaps (128 bytes) that outlive the call, which copies them.
    let cursor = unsafe {
        CreateCursor(
            Some(instance),
            0,
            0,
            32,
            32,
            and_plane.as_ptr().cast(),
            xor_plane.as_ptr().cast(),
        )
    }
    .map_err(|e| werr("CreateCursor", &e))?;
    Ok(CursorResource(cursor))
}

fn park_thread(x: i32, y: i32, init: &mpsc::Sender<Result<u32, WinError>>) {
    let _dpi = DpiGuard::enter();
    let setup = || -> Result<(CursorResource, WindowClass, Window), WinError> {
        let instance = module_instance()?;
        let cursor = blank_cursor(instance)?;
        let class = WindowClass::register("ClipSyncWin.Park", instance, park_proc, cursor.0)?;
        let window = Window::create(
            &class,
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
            WS_POPUP,
            (x - 2, y - 2, 5, 5),
        )?;
        // Nearly invisible, but still a window that the pointer is "over" and that picks the cursor.
        // SAFETY: the window is live and layered (WS_EX_LAYERED), as the call requires.
        unsafe { SetLayeredWindowAttributes(window.0, COLORREF(0), 1, LWA_ALPHA) }
            .map_err(|e| werr("SetLayeredWindowAttributes", &e))?;
        // SAFETY: the window is live and owned by this thread.
        let _ = unsafe { ShowWindow(window.0, SW_SHOWNOACTIVATE) };
        // SAFETY: no pointer arguments.
        unsafe { SetCursorPos(x, y) }.map_err(|e| werr("SetCursorPos", &e))?;
        Ok((cursor, class, window))
    };
    // The window is dropped before its class, and the class before the cursor it uses.
    let resources = match setup() {
        Ok(resources) => resources,
        Err(e) => {
            let _ = init.send(Err(e));
            return;
        }
    };
    PARKED.store(pack(x, y), Ordering::Release);
    // SAFETY: GetCurrentThreadId takes no argument.
    let id = unsafe { GetCurrentThreadId() };
    if init.send(Ok(id)).is_ok() {
        pump_messages(|_| {});
    }
    PARKED.store(NO_PARK, Ordering::Release);
    let (cursor, class, window) = resources;
    drop(window);
    drop(class);
    drop(cursor);
}

pub(crate) fn start_park(x: i32, y: i32) -> Result<ThreadHandle, WinError> {
    ThreadHandle::start("clipsync-win-park", move |init| park_thread(x, y, init))
}

// ---------------------------------------------------------------------------------------------
// ResumeHotkey

const HOTKEY_ID: i32 = 1;
const VK_P: u32 = 0x50;

struct HotkeyRegistration;

impl Drop for HotkeyRegistration {
    fn drop(&mut self) {
        // SAFETY: registered on this thread with this id and removed exactly once.
        let _ = unsafe { UnregisterHotKey(None, HOTKEY_ID) };
    }
}

fn hotkey_thread(
    vk: u32,
    mut on_press: Option<Box<dyn FnMut() + Send>>,
    init: &mpsc::Sender<Result<u32, WinError>>,
) {
    // SAFETY: a null window registers the hotkey for this thread's message queue.
    let registered = unsafe {
        RegisterHotKey(
            None,
            HOTKEY_ID,
            HOT_KEY_MODIFIERS(MOD_WIN.0 | MOD_ALT.0 | MOD_NOREPEAT.0),
            vk,
        )
    };
    if let Err(e) = registered {
        let _ = init.send(Err(werr("RegisterHotKey", &e)));
        return;
    }
    let _registration = HotkeyRegistration;
    // SAFETY: GetCurrentThreadId takes no argument.
    let id = unsafe { GetCurrentThreadId() };
    if init.send(Ok(id)).is_err() {
        return;
    }
    // Only the combination is reported: the system never hands over a key.
    pump_messages(|msg| {
        if msg.message == WM_HOTKEY
            && let Some(on_press) = on_press.as_mut()
        {
            let _ = catch_unwind(AssertUnwindSafe(&mut *on_press));
        }
    });
}

/// Registers Win+Alt+`vk` and, when `on_press` is given, keeps it registered until the handle drops.
pub(crate) fn start_hotkey(
    vk: u32,
    on_press: Option<Box<dyn FnMut() + Send>>,
) -> Result<ThreadHandle, WinError> {
    ThreadHandle::start("clipsync-win-hotkey", move |init| {
        hotkey_thread(vk, on_press, init)
    })
}

pub(crate) const RESUME_VK: u32 = VK_P;

/// The callbacks are called here with structs built by hand, on the test's own thread (the state is
/// thread-local, so every test has a clean slate). Runs on Windows only, with the rest of the file.
#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;
    use windows::Win32::UI::WindowsAndMessaging::KBDLLHOOKSTRUCT_FLAGS;

    struct Probe {
        input_calls: Arc<AtomicUsize>,
        lost: Arc<AtomicUsize>,
    }

    fn install(on_input: impl FnMut(&RawEvent) -> Verdict + Send + 'static) -> Probe {
        let lost = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&lost);
        let input_calls = Arc::new(AtomicUsize::new(0));
        let calls = Arc::clone(&input_calls);
        let mut on_input = on_input;
        CALLBACKS.with(|cell| {
            *cell.borrow_mut() = Some(Callbacks {
                on_input: Some(Box::new(move |event| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    on_input(event)
                })),
                on_notice: Box::new(move |n| {
                    if n == Notice::HooksLost {
                        counter.fetch_add(1, Ordering::SeqCst);
                    }
                }),
                lost: false,
            });
        });
        Probe { input_calls, lost }
    }

    fn key(flags: u32) -> LRESULT {
        let info = KBDLLHOOKSTRUCT {
            vkCode: 0x41,
            scanCode: 0x1E,
            flags: KBDLLHOOKSTRUCT_FLAGS(flags),
            time: 0,
            dwExtraInfo: 0,
        };
        // SAFETY: `info` lives for the whole call, as the system's struct does for a hook callback; the
        // code 0 (HC_ACTION) says `lparam` points to it.
        unsafe {
            keyboard_proc(
                0,
                WPARAM(WM_KEYDOWN as usize),
                LPARAM(std::ptr::from_ref(&info) as isize),
            )
        }
    }

    fn mouse() -> LRESULT {
        let info = MSLLHOOKSTRUCT {
            pt: POINT { x: 5, y: 6 },
            mouseData: 0,
            flags: 0,
            time: 0,
            dwExtraInfo: 0,
        };
        // SAFETY: as in `key`, for the mouse struct.
        unsafe {
            mouse_proc(
                0,
                WPARAM(WM_MOUSEMOVE as usize),
                LPARAM(std::ptr::from_ref(&info) as isize),
            )
        }
    }

    #[test]
    fn a_panic_in_the_keyboard_callback_passes_the_event_and_reports_the_loss_once() {
        let probe = install(|_| panic!("boom"));
        assert_ne!(key(0).0, 1, "the event has to pass");
        assert_eq!(probe.lost.load(Ordering::SeqCst), 1);
        // the hook is dead: it passes everything and never calls the closure again
        assert_ne!(key(0).0, 1);
        assert_ne!(mouse().0, 1);
        assert_eq!(probe.input_calls.load(Ordering::SeqCst), 1);
        assert_eq!(probe.lost.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_panic_in_the_mouse_callback_is_the_same() {
        let probe = install(|_| panic!("boom"));
        assert_ne!(mouse().0, 1);
        assert_eq!(probe.lost.load(Ordering::SeqCst), 1);
        assert_ne!(key(0).0, 1);
        assert_eq!(probe.input_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn block_stops_the_event_and_pass_lets_it_through() {
        let probe = install(|_| Verdict::Block);
        assert_eq!(key(0).0, 1);
        assert_eq!(mouse().0, 1);
        assert_eq!(probe.input_calls.load(Ordering::SeqCst), 2);
        let probe = install(|_| Verdict::Pass);
        assert_ne!(key(0).0, 1);
        assert_eq!(probe.lost.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn injected_events_are_never_read() {
        let probe = install(|_| Verdict::Block);
        for flags in [
            LLKHF_INJECTED.0,
            LLKHF_LOWER_IL_INJECTED.0,
            LLKHF_INJECTED.0 | LLKHF_UP.0,
        ] {
            assert_ne!(key(flags).0, 1);
        }
        assert_eq!(probe.input_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_callback_that_runs_while_the_state_is_borrowed_passes() {
        let probe = install(|_| Verdict::Block);
        let held = CALLBACKS.with(|cell| {
            let _guard = cell.borrow_mut();
            key(0).0
        });
        assert_ne!(held, 1);
        assert_eq!(probe.input_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_negative_code_passes_without_reading_the_struct() {
        let probe = install(|_| Verdict::Block);
        // SAFETY: a negative code means the system sent no struct; the pointer is never read.
        let result = unsafe { keyboard_proc(-1, WPARAM(0), LPARAM(0)) };
        assert_ne!(result.0, 1);
        assert_eq!(probe.input_calls.load(Ordering::SeqCst), 0);
    }
}
