#![allow(unsafe_code)]

use std::ffi::c_void;
use std::path::Path;

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{DEFAULT_GUI_FONT, GetStockObject};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::EM_SETLIMITTEXT;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    BS_DEFPUSHBUTTON, BS_PUSHBUTTON, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW,
    DestroyWindow, DispatchMessageW, ES_AUTOHSCROLL, ES_PASSWORD, GetMessageW, GetSystemMetrics,
    GetWindowTextLengthW, GetWindowTextW, HMENU, IDC_ARROW, IsDialogMessageW, LoadCursorW,
    MB_ICONWARNING, MB_OK, MSG, MessageBoxW, PostMessageW, RegisterClassW, SM_CXSCREEN,
    SM_CYSCREEN, SW_SHOWNORMAL, SendMessageW, SetForegroundWindow, SetWindowTextW, ShowWindow,
    TranslateMessage, UnregisterClassW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE,
    WM_COMMAND, WM_SETFONT, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME,
    WS_GROUP, WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};
use windows::core::{Error as WindowsError, PCWSTR};
use zeroize::Zeroizing;

use crate::error::{IrohaZipError, Result};
use crate::password::{ArchivePassword, MAX_PASSWORD_UTF16_UNITS};

const BASE_DPI: u32 = 96;
const ID_PASSWORD: usize = 100;
const ID_CONFIRM: usize = 1;
const ID_CANCEL: usize = 2;
const WM_PASSWORD_RESULT: u32 = WM_APP + 0x251;

pub fn prompt_archive_password(archive: &Path) -> Result<Option<ArchivePassword>> {
    let instance = module_instance()?;
    let class_name = wide_null(&format!("iroha-zip.PasswordPrompt.{}", std::process::id()));
    let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }
        .map_err(|error| windows_error("cannot load password-dialog cursor", error))?;
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(password_window_proc),
        hInstance: instance,
        hCursor: cursor,
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..Default::default()
    };
    let atom = unsafe { RegisterClassW(&raw const class) };
    if atom == 0 {
        return Err(windows_error(
            "cannot register password-dialog window class",
            WindowsError::from_thread(),
        ));
    }
    let _class_guard = ClassGuard {
        class_name: &class_name,
        instance,
    };

    let dpi = unsafe { GetDpiForSystem() }.max(BASE_DPI);
    let width = scale(560, dpi);
    let height = scale(220, dpi);
    let x = (unsafe { GetSystemMetrics(SM_CXSCREEN) } - width).max(0) / 2;
    let y = (unsafe { GetSystemMetrics(SM_CYSCREEN) } - height).max(0) / 2;
    let title = wide_null("Archive password / 書庫のパスワード");
    let window = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            PCWSTR(class_name.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            x,
            y,
            width,
            height,
            None,
            None,
            Some(instance),
            None,
        )
    }
    .map_err(|error| windows_error("cannot create password dialog", error))?;
    let window_guard = WindowGuard(window);

    let archive_name = safe_archive_label(archive);
    let explanation =
        format!("Password for {archive_name}\n{archive_name} のパスワード（保存されません）");
    add_control(
        window,
        instance,
        WINDOW_EX_STYLE(0),
        "STATIC",
        &explanation,
        WS_CHILD | WS_VISIBLE | WS_GROUP,
        24,
        18,
        510,
        42,
        None,
        dpi,
    )?;
    add_control(
        window,
        instance,
        WINDOW_EX_STYLE(0),
        "STATIC",
        "&Password / パスワード (&P):",
        WS_CHILD | WS_VISIBLE,
        24,
        72,
        210,
        24,
        None,
        dpi,
    )?;
    let edit = add_control(
        window,
        instance,
        WS_EX_CLIENTEDGE,
        "EDIT",
        "",
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE((ES_PASSWORD | ES_AUTOHSCROLL) as u32),
        226,
        68,
        306,
        30,
        Some(ID_PASSWORD),
        dpi,
    )?;
    unsafe {
        SendMessageW(
            edit,
            EM_SETLIMITTEXT,
            Some(WPARAM(MAX_PASSWORD_UTF16_UNITS)),
            Some(LPARAM(0)),
        );
    }
    add_control(
        window,
        instance,
        WINDOW_EX_STYLE(0),
        "BUTTON",
        "OK / 実行",
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
        310,
        124,
        104,
        32,
        Some(ID_CONFIRM),
        dpi,
    )?;
    add_control(
        window,
        instance,
        WINDOW_EX_STYLE(0),
        "BUTTON",
        "Cancel / キャンセル",
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        424,
        124,
        108,
        32,
        Some(ID_CANCEL),
        dpi,
    )?;

    unsafe {
        let _ = ShowWindow(window, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(window);
        let _ = SetFocus(Some(edit));
    }

    let result = message_loop(window, edit);
    clear_edit(edit);
    drop(window_guard);
    result
}

fn message_loop(window: HWND, edit: HWND) -> Result<Option<ArchivePassword>> {
    loop {
        let mut message = MSG::default();
        let status = unsafe { GetMessageW(&raw mut message, None, 0, 0) };
        if status.0 == -1 {
            return Err(windows_error(
                "password-dialog message loop failed",
                WindowsError::from_thread(),
            ));
        }
        if status.0 == 0 {
            return Err(IrohaZipError::Usage(
                "password dialog closed before it returned a result".to_owned(),
            ));
        }

        if message.hwnd == window && message.message == WM_PASSWORD_RESULT {
            match message.wParam.0 & 0xffff {
                ID_CONFIRM => match read_password(edit) {
                    Ok(password) => return Ok(Some(password)),
                    Err(error) => {
                        show_validation_error(window, &error.to_string());
                        unsafe {
                            let _ = SetFocus(Some(edit));
                        }
                    }
                },
                ID_CANCEL => return Ok(None),
                _ => {}
            }
        }

        if !unsafe { IsDialogMessageW(window, &raw const message) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&raw const message);
                DispatchMessageW(&raw const message);
            }
        }
    }
}

fn read_password(edit: HWND) -> Result<ArchivePassword> {
    let length = unsafe { GetWindowTextLengthW(edit) };
    if length < 0 {
        return Err(windows_error(
            "cannot measure archive password input",
            WindowsError::from_thread(),
        ));
    }
    let length = usize::try_from(length)
        .map_err(|_| IrohaZipError::Usage("archive password length overflow".to_owned()))?;
    if length > MAX_PASSWORD_UTF16_UNITS {
        return Err(IrohaZipError::Usage(format!(
            "archive password exceeds {MAX_PASSWORD_UTF16_UNITS} UTF-16 units"
        )));
    }
    let mut buffer = Zeroizing::new(vec![0u16; length.saturating_add(1)]);
    let copied = unsafe { GetWindowTextW(edit, &mut buffer) };
    if copied < 0 {
        return Err(windows_error(
            "cannot read archive password input",
            WindowsError::from_thread(),
        ));
    }
    buffer.truncate(usize::try_from(copied).unwrap_or(0));
    let units = std::mem::take(&mut *buffer);
    ArchivePassword::from_utf16(units)
}

#[allow(clippy::too_many_arguments)]
fn add_control(
    parent: HWND,
    instance: HINSTANCE,
    extended_style: WINDOW_EX_STYLE,
    class: &str,
    text: &str,
    style: WINDOW_STYLE,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    id: Option<usize>,
    dpi: u32,
) -> Result<HWND> {
    let class = wide_null(class);
    let text = wide_null(text);
    let menu = id.map(|value| HMENU(value as *mut c_void));
    let control = unsafe {
        CreateWindowExW(
            extended_style,
            PCWSTR(class.as_ptr()),
            PCWSTR(text.as_ptr()),
            style,
            scale(x, dpi),
            scale(y, dpi),
            scale(width, dpi),
            scale(height, dpi),
            Some(parent),
            menu,
            Some(instance),
            None,
        )
    }
    .map_err(|error| windows_error("cannot create password-dialog control", error))?;
    let font = unsafe { GetStockObject(DEFAULT_GUI_FONT) };
    unsafe {
        SendMessageW(
            control,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
    }
    Ok(control)
}

fn clear_edit(edit: HWND) {
    let empty = [0u16];
    let _ = unsafe { SetWindowTextW(edit, PCWSTR(empty.as_ptr())) };
}

fn show_validation_error(parent: HWND, message: &str) {
    let message = wide_null(message);
    let title = wide_null("Password input / パスワード入力");
    let _ = unsafe {
        MessageBoxW(
            Some(parent),
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONWARNING,
        )
    };
}

fn safe_archive_label(archive: &Path) -> String {
    let source = archive
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("archive"))
        .to_string_lossy();
    let mut label = String::new();
    for character in source.chars().filter(|character| !character.is_control()) {
        if label.chars().count() >= 80 {
            label.push('…');
            break;
        }
        label.push(character);
    }
    if label.is_empty() {
        "archive".to_owned()
    } else {
        label
    }
}

fn module_instance() -> Result<HINSTANCE> {
    let module = unsafe { GetModuleHandleW(None) }
        .map_err(|error| windows_error("cannot identify application module", error))?;
    Ok(HINSTANCE(module.0))
}

fn scale(value: i32, dpi: u32) -> i32 {
    value.saturating_mul(i32::try_from(dpi).unwrap_or(i32::MAX))
        / i32::try_from(BASE_DPI).unwrap_or(96)
}

fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn windows_error(context: &str, error: WindowsError) -> IrohaZipError {
    IrohaZipError::Usage(format!("{context}: {error}"))
}

unsafe extern "system" fn password_window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_CLOSE {
        let _ = unsafe {
            PostMessageW(
                Some(window),
                WM_PASSWORD_RESULT,
                WPARAM(ID_CANCEL),
                LPARAM(0),
            )
        };
        return LRESULT(0);
    }
    if message == WM_COMMAND {
        let identifier = wparam.0 & 0xffff;
        if matches!(identifier, ID_CONFIRM | ID_CANCEL) {
            let _ = unsafe {
                PostMessageW(
                    Some(window),
                    WM_PASSWORD_RESULT,
                    WPARAM(identifier),
                    LPARAM(0),
                )
            };
            return LRESULT(0);
        }
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

struct WindowGuard(HWND);

impl Drop for WindowGuard {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

struct ClassGuard<'a> {
    class_name: &'a [u16],
    instance: HINSTANCE,
}

impl Drop for ClassGuard<'_> {
    fn drop(&mut self) {
        let _ = unsafe { UnregisterClassW(PCWSTR(self.class_name.as_ptr()), Some(self.instance)) };
    }
}
