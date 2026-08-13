#![cfg_attr(windows, windows_subsystem = "windows")]
#![cfg_attr(windows, allow(unsafe_code))]
#![cfg_attr(not(windows), deny(unsafe_code))]

#[cfg(windows)]
mod windows_app {
    use iroha_zip::backend::BackendBundle;
    use iroha_zip::backend_evidence::BackendEvidence;
    use iroha_zip::config::{
        AttachmentHandoffPolicy, Config, FilenameEncoding, IsolationMode, default_config_path,
    };
    use iroha_zip::settings::{
        BASE_DPI, SettingsAction, SettingsField, SettingsForm, control_id, scale_logical,
    };
    use iroha_zip::util;
    use std::cell::{Cell, RefCell};
    use std::ffi::{OsStr, OsString, c_void};
    use std::fs;
    use std::mem::size_of;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::OnceLock;
    use windows::Win32::Foundation::{
        ERROR_CANCELLED, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
    };
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    use windows::Win32::Graphics::Gdi::{
        COLOR_3DFACE, DEFAULT_GUI_FONT, GetStockObject, GetSysColorBrush, ScreenToClient,
        UpdateWindow,
    };
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoTaskMemFree, CoUninitialize,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Controls::SetScrollInfo;
    use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
    use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, GetFocus, SetFocus};
    use windows::Win32::UI::Shell::{
        FOS_DONTADDTORECENT, FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS,
        FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName,
        SIGDN_FILESYSPATH,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        BM_GETCHECK, BM_SETCHECK, BN_CLICKED, BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, BS_GROUPBOX,
        BS_PUSHBUTTON, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CBN_SELCHANGE, CBS_DROPDOWNLIST,
        CBS_HASSTRINGS, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW,
        DefWindowProcW, DestroyWindow, DispatchMessageW, EN_CHANGE, ES_AUTOHSCROLL, ES_NUMBER,
        GWLP_USERDATA, GetClientRect, GetMessageW, GetScrollInfo, GetSystemMetrics,
        GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, HMENU, IDC_ARROW,
        IDC_WAIT, IDYES, IsChild, IsDialogMessageW, LoadCursorW, MB_ICONERROR, MB_ICONINFORMATION,
        MB_ICONWARNING, MB_OK, MB_YESNO, MESSAGEBOX_STYLE, MSG, MessageBoxW, PostQuitMessage,
        RegisterClassW, SB_BOTTOM, SB_HORZ, SB_LINEDOWN, SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP,
        SB_THUMBPOSITION, SB_THUMBTRACK, SB_TOP, SB_VERT, SCROLLINFO, SIF_ALL, SIF_PAGE, SIF_POS,
        SIF_RANGE, SM_CXSCREEN, SM_CYSCREEN, SW_ERASE, SW_INVALIDATE, SW_SCROLLCHILDREN,
        SW_SHOWNORMAL, ScrollWindowEx, SendMessageW, SetCursor, SetWindowLongPtrW, SetWindowTextW,
        ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_COMMAND,
        WM_CREATE, WM_DESTROY, WM_HSCROLL, WM_MOUSEWHEEL, WM_NCCREATE, WM_SETFONT, WM_SIZE,
        WM_VSCROLL, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE, WS_HSCROLL, WS_MAXIMIZEBOX,
        WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME, WS_VISIBLE,
        WS_VSCROLL,
    };
    use windows::core::{Error as WindowsError, HRESULT, PCWSTR};

    const WINDOW_CLASS: &str = "iroha-zip.Settings.Window";
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const BUTTON_CHECKED: usize = 1;
    const CONTENT_WIDTH: i32 = 932;
    const CONTENT_HEIGHT: i32 = 720;
    const PRIMARY_LANGUAGE_JAPANESE: u16 = 0x11;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Language {
        Japanese,
        English,
    }

    static LANGUAGE: OnceLock<Language> = OnceLock::new();

    fn language_from_tag(value: &OsStr) -> Option<Language> {
        let normalized = value.to_string_lossy().trim().to_ascii_lowercase();
        if normalized == "ja" || normalized.starts_with("ja-") || normalized.starts_with("ja_") {
            Some(Language::Japanese)
        } else if normalized == "en"
            || normalized.starts_with("en-")
            || normalized.starts_with("en_")
        {
            Some(Language::English)
        } else {
            None
        }
    }

    fn detect_language() -> Language {
        if let Some(value) = std::env::var_os("IROHA_ZIP_LANGUAGE")
            && let Some(language) = language_from_tag(&value)
        {
            return language;
        }
        let ui_language = unsafe { GetUserDefaultUILanguage() };
        if ui_language & 0x03ff == PRIMARY_LANGUAGE_JAPANESE {
            Language::Japanese
        } else {
            Language::English
        }
    }

    fn selected_language() -> Language {
        *LANGUAGE.get_or_init(detect_language)
    }

    fn tr(japanese: &'static str, english: &'static str) -> &'static str {
        match selected_language() {
            Language::Japanese => japanese,
            Language::English => english,
        }
    }

    pub fn window_title() -> &'static str {
        tr("iroha-zip 設定", "iroha-zip Settings")
    }

    #[derive(Default)]
    struct Controls {
        backend: HWND,
        timeout_seconds: HWND,
        memory_limit_mib: HWND,
        isolation: HWND,
        max_archive_bytes: HWND,
        max_files: HWND,
        max_directories: HWND,
        max_total_bytes: HWND,
        max_single_file_bytes: HWND,
        max_depth: HWND,
        max_path_bytes: HWND,
        preserve_motw: HWND,
        attachment_handoff: HWND,
        open_after_double_click: HWND,
        encoding: HWND,
        status: HWND,
    }

    struct App {
        config_path: PathBuf,
        controls: Controls,
        saved_config: RefCell<Config>,
        scroll_x: Cell<i32>,
        scroll_y: Cell<i32>,
    }

    impl App {
        fn new(config_path: PathBuf) -> Self {
            Self {
                config_path,
                controls: Controls::default(),
                saved_config: RefCell::new(Config::default()),
                scroll_x: Cell::new(0),
                scroll_y: Cell::new(0),
            }
        }

        unsafe fn create_controls(
            &mut self,
            parent: HWND,
            instance: HINSTANCE,
        ) -> Result<(), String> {
            let config = Config::load(&self.config_path).map_err(|error| error.to_string())?;

            unsafe {
                add_static(
                    parent,
                    instance,
                    tr(
                        "安全性に関わる全設定とWindows統合を、ここから検証・管理できます。",
                        "Validate and manage all security settings and Windows integration here.",
                    ),
                    18,
                    12,
                    888,
                    22,
                )?;
                add_static(
                    parent,
                    instance,
                    &format!(
                        "{} {}",
                        tr("設定ファイル:", "Configuration file:"),
                        self.config_path.display()
                    ),
                    18,
                    36,
                    888,
                    20,
                )?;

                add_group(
                    parent,
                    instance,
                    tr("バックエンド", "Backend"),
                    14,
                    60,
                    904,
                    108,
                )?;
                add_static(
                    parent,
                    instance,
                    tr("保存先(&L)", "&Location"),
                    28,
                    86,
                    92,
                    22,
                )?;
                self.controls.backend = add_edit(
                    parent,
                    instance,
                    122,
                    82,
                    558,
                    25,
                    false,
                    control_id::BACKEND_DIRECTORY,
                )?;
                add_button(
                    parent,
                    instance,
                    tr("選択(&B)...", "&Browse..."),
                    690,
                    81,
                    96,
                    27,
                    control_id::BACKEND_BROWSE,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    tr("診断(&D)", "&Diagnose"),
                    796,
                    81,
                    104,
                    27,
                    control_id::BACKEND_DOCTOR,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    tr("bundleを取り込む(&I)...", "&Import bundle..."),
                    122,
                    119,
                    190,
                    28,
                    control_id::BACKEND_IMPORT,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    tr("MSYS2から取り込む(&M)...", "Import from &MSYS2..."),
                    322,
                    119,
                    190,
                    28,
                    control_id::BACKEND_MSYS2,
                    false,
                )?;
                add_static(
                    parent,
                    instance,
                    tr(
                        "取り込み時にSHA-256 manifestを生成し、既存bundleを安全に置換します。",
                        "Import generates a SHA-256 manifest and safely replaces the existing bundle.",
                    ),
                    526,
                    123,
                    372,
                    22,
                )?;

                add_group(parent, instance, "AppContainer", 14, 174, 904, 67)?;
                add_static(
                    parent,
                    instance,
                    tr("分離モード(&Q)", "Isolation (&Q)"),
                    28,
                    201,
                    92,
                    22,
                )?;
                self.controls.isolation =
                    add_combo(parent, instance, 122, 197, 190, 120, control_id::ISOLATION)?;
                for label in [
                    tr("AppContainer（互換）", "AppContainer (compatible)"),
                    tr("LPAC（実験）", "LPAC (experimental)"),
                ] {
                    combo_add(self.controls.isolation, label);
                }
                add_static(
                    parent,
                    instance,
                    tr("時間(&T)（1–86400秒）", "&Timeout (1–86400 s)"),
                    340,
                    201,
                    142,
                    22,
                )?;
                self.controls.timeout_seconds = add_edit(
                    parent,
                    instance,
                    484,
                    197,
                    118,
                    25,
                    true,
                    control_id::TIMEOUT_SECONDS,
                )?;
                add_static(
                    parent,
                    instance,
                    tr("メモリ(&Y)（MiB）", "Memor&y (MiB)"),
                    630,
                    201,
                    112,
                    22,
                )?;
                self.controls.memory_limit_mib = add_edit(
                    parent,
                    instance,
                    744,
                    197,
                    112,
                    25,
                    true,
                    control_id::MEMORY_LIMIT_MIB,
                )?;

                add_group(
                    parent,
                    instance,
                    tr("展開・作成の上限", "Extraction and creation limits"),
                    14,
                    247,
                    904,
                    194,
                )?;
                add_static(
                    parent,
                    instance,
                    tr("入力書庫の上限(&Z)", "Archive si&ze limit"),
                    28,
                    276,
                    164,
                    22,
                )?;
                self.controls.max_archive_bytes = add_edit(
                    parent,
                    instance,
                    194,
                    272,
                    222,
                    25,
                    false,
                    control_id::MAX_ARCHIVE_BYTES,
                )?;
                add_static(
                    parent,
                    instance,
                    tr("ファイル数(&N)", "File cou&nt"),
                    478,
                    276,
                    150,
                    22,
                )?;
                self.controls.max_files = add_edit(
                    parent,
                    instance,
                    636,
                    272,
                    222,
                    25,
                    true,
                    control_id::MAX_FILES,
                )?;

                add_static(
                    parent,
                    instance,
                    tr("ディレクトリ数(&G)", "Directory count (&G)"),
                    28,
                    316,
                    164,
                    22,
                )?;
                self.controls.max_directories = add_edit(
                    parent,
                    instance,
                    194,
                    312,
                    222,
                    25,
                    true,
                    control_id::MAX_DIRECTORIES,
                )?;
                add_static(
                    parent,
                    instance,
                    tr("合計容量の上限(&H)", "Total size limit (&H)"),
                    478,
                    316,
                    150,
                    22,
                )?;
                self.controls.max_total_bytes = add_edit(
                    parent,
                    instance,
                    636,
                    312,
                    222,
                    25,
                    false,
                    control_id::MAX_TOTAL_BYTES,
                )?;

                add_static(
                    parent,
                    instance,
                    tr("単一ファイル上限(&J)", "Single-file limit (&J)"),
                    28,
                    356,
                    164,
                    22,
                )?;
                self.controls.max_single_file_bytes = add_edit(
                    parent,
                    instance,
                    194,
                    352,
                    222,
                    25,
                    false,
                    control_id::MAX_SINGLE_FILE_BYTES,
                )?;
                add_static(
                    parent,
                    instance,
                    tr("パスの深さ(&K)", "Path depth (&K)"),
                    478,
                    356,
                    150,
                    22,
                )?;
                self.controls.max_depth = add_edit(
                    parent,
                    instance,
                    636,
                    352,
                    222,
                    25,
                    true,
                    control_id::MAX_DEPTH,
                )?;

                add_static(
                    parent,
                    instance,
                    tr("パス長(&V)（UTF-8 bytes）", "Path length (&V), UTF-8 bytes"),
                    28,
                    396,
                    164,
                    22,
                )?;
                self.controls.max_path_bytes = add_edit(
                    parent,
                    instance,
                    194,
                    392,
                    222,
                    25,
                    true,
                    control_id::MAX_PATH_BYTES,
                )?;
                add_static(
                    parent,
                    instance,
                    tr(
                        "容量は 16 GiB / 512 MiB のように入力できます。",
                        "Sizes accept values such as 16 GiB or 512 MiB.",
                    ),
                    478,
                    396,
                    380,
                    22,
                )?;

                add_group(
                    parent,
                    instance,
                    tr("展開時の動作", "Extraction behavior"),
                    14,
                    447,
                    904,
                    104,
                )?;
                self.controls.preserve_motw = add_checkbox(
                    parent,
                    instance,
                    tr(
                        "Mark-of-the-Webを展開後のファイルへ引き継ぐ(&X)",
                        "Propagate Mark-of-the-Web to extracted files (&X)",
                    ),
                    30,
                    474,
                    350,
                    24,
                    control_id::PRESERVE_MOTW,
                )?;
                self.controls.open_after_double_click = add_checkbox(
                    parent,
                    instance,
                    tr(
                        "ダブルクリック展開後にフォルダを開く(&O)",
                        "&Open folder after double-click extraction",
                    ),
                    30,
                    498,
                    350,
                    24,
                    control_id::OPEN_AFTER_DOUBLE_CLICK,
                )?;
                add_static(
                    parent,
                    instance,
                    tr("既定の文字コード(&E)", "Default &encoding"),
                    486,
                    481,
                    142,
                    22,
                )?;
                self.controls.encoding =
                    add_combo(parent, instance, 636, 475, 222, 120, control_id::ENCODING)?;
                for label in [
                    tr("自動判定", "Automatic"),
                    "UTF-8",
                    tr("CP932（日本語）", "CP932 (Japanese)"),
                    "CP437",
                ] {
                    combo_add(self.controls.encoding, label);
                }
                add_static(
                    parent,
                    instance,
                    tr("Windows信頼連携(&W)", "Windows trust (&W)"),
                    486,
                    515,
                    142,
                    22,
                )?;
                self.controls.attachment_handoff = add_combo(
                    parent,
                    instance,
                    636,
                    509,
                    262,
                    120,
                    control_id::ATTACHMENT_HANDOFF,
                )?;
                for label in [
                    tr("無効（既定）", "Disabled (default)"),
                    tr("best-effort（失敗を表示）", "Best effort (report failure)"),
                    tr(
                        "必須（失敗時は公開しない）",
                        "Required (do not publish on failure)",
                    ),
                ] {
                    combo_add(self.controls.attachment_handoff, label);
                }

                add_group(
                    parent,
                    instance,
                    tr("Windows 統合", "Windows integration"),
                    14,
                    557,
                    904,
                    72,
                )?;
                add_button(
                    parent,
                    instance,
                    tr("関連付けを登録(&A)", "Register &associations"),
                    30,
                    583,
                    160,
                    29,
                    control_id::REGISTER,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    tr("関連付けを解除(&U)", "&Unregister associations"),
                    200,
                    583,
                    160,
                    29,
                    control_id::UNREGISTER,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    tr("既定のアプリを開く(&P)", "Open default a&pps"),
                    370,
                    583,
                    180,
                    29,
                    control_id::DEFAULT_APPS,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    tr("設定フォルダを開く(&F)", "Open config &folder"),
                    560,
                    583,
                    180,
                    29,
                    control_id::CONFIG_FOLDER,
                    false,
                )?;

                add_button(
                    parent,
                    instance,
                    tr("既定値に戻す(&R)", "&Restore defaults"),
                    18,
                    642,
                    150,
                    32,
                    control_id::DEFAULTS,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    tr("保存(&S)", "&Save"),
                    670,
                    642,
                    110,
                    32,
                    control_id::SAVE,
                    true,
                )?;
                add_button(
                    parent,
                    instance,
                    tr("閉じる(&C)", "&Close"),
                    790,
                    642,
                    110,
                    32,
                    control_id::CANCEL,
                    false,
                )?;
                self.controls.status = add_static(
                    parent,
                    instance,
                    tr("設定を読み込みました。", "Settings loaded."),
                    18,
                    682,
                    882,
                    22,
                )?;
            }

            self.apply_config(&config);
            self.saved_config.replace(config);
            self.update_dirty_title(parent);
            self.update_scrollbar(parent);
            Ok(())
        }

        fn apply_config(&self, config: &Config) {
            let form = SettingsForm::from_config(config);
            set_control_text(self.controls.backend, &form.backend_directory);
            set_control_text(self.controls.timeout_seconds, &form.timeout_seconds);
            set_control_text(self.controls.memory_limit_mib, &form.memory_limit_mib);
            let isolation_index = match form.isolation {
                IsolationMode::AppContainer => 0,
                IsolationMode::Lpac => 1,
            };
            unsafe {
                SendMessageW(
                    self.controls.isolation,
                    CB_SETCURSEL,
                    Some(WPARAM(isolation_index)),
                    Some(LPARAM(0)),
                );
            }
            set_control_text(self.controls.max_archive_bytes, &form.max_archive_bytes);
            set_control_text(self.controls.max_files, &form.max_files);
            set_control_text(self.controls.max_directories, &form.max_directories);
            set_control_text(self.controls.max_total_bytes, &form.max_total_bytes);
            set_control_text(
                self.controls.max_single_file_bytes,
                &form.max_single_file_bytes,
            );
            set_control_text(self.controls.max_depth, &form.max_depth);
            set_control_text(self.controls.max_path_bytes, &form.max_path_bytes);
            set_check(self.controls.preserve_motw, form.preserve_mark_of_the_web);
            let attachment_handoff_index = match form.attachment_handoff {
                AttachmentHandoffPolicy::Disabled => 0,
                AttachmentHandoffPolicy::BestEffort => 1,
                AttachmentHandoffPolicy::Required => 2,
            };
            unsafe {
                SendMessageW(
                    self.controls.attachment_handoff,
                    CB_SETCURSEL,
                    Some(WPARAM(attachment_handoff_index)),
                    Some(LPARAM(0)),
                );
            }
            set_check(
                self.controls.open_after_double_click,
                form.open_after_double_click,
            );
            let encoding_index = match form.default_filename_encoding {
                FilenameEncoding::Auto => 0,
                FilenameEncoding::Utf8 => 1,
                FilenameEncoding::Cp932 => 2,
                FilenameEncoding::Cp437 => 3,
            };
            unsafe {
                SendMessageW(
                    self.controls.encoding,
                    CB_SETCURSEL,
                    Some(WPARAM(encoding_index)),
                    Some(LPARAM(0)),
                );
            }
        }

        fn collect_config(&self) -> Result<Config, String> {
            self.read_form()?.into_config().map_err(|error| {
                self.focus_field(error.field);
                match selected_language() {
                    Language::Japanese => error.to_string(),
                    Language::English => error.english(),
                }
            })
        }

        fn read_form(&self) -> Result<SettingsForm, String> {
            let mut form = SettingsForm {
                backend_directory: control_text(self.controls.backend)?,
                timeout_seconds: control_text(self.controls.timeout_seconds)?,
                memory_limit_mib: control_text(self.controls.memory_limit_mib)?,
                isolation: IsolationMode::AppContainer,
                max_archive_bytes: control_text(self.controls.max_archive_bytes)?,
                max_files: control_text(self.controls.max_files)?,
                max_directories: control_text(self.controls.max_directories)?,
                max_total_bytes: control_text(self.controls.max_total_bytes)?,
                max_single_file_bytes: control_text(self.controls.max_single_file_bytes)?,
                max_depth: control_text(self.controls.max_depth)?,
                max_path_bytes: control_text(self.controls.max_path_bytes)?,
                preserve_mark_of_the_web: is_checked(self.controls.preserve_motw),
                attachment_handoff: AttachmentHandoffPolicy::Disabled,
                open_after_double_click: is_checked(self.controls.open_after_double_click),
                default_filename_encoding: FilenameEncoding::Auto,
            };
            let isolation = unsafe {
                SendMessageW(
                    self.controls.isolation,
                    CB_GETCURSEL,
                    Some(WPARAM(0)),
                    Some(LPARAM(0)),
                )
                .0
            };
            form.isolation = match isolation {
                0 => IsolationMode::AppContainer,
                1 => IsolationMode::Lpac,
                _ => {
                    return Err(tr(
                        "分離モードを選択してください。",
                        "Select an isolation mode.",
                    )
                    .to_owned());
                }
            };
            let attachment_handoff = unsafe {
                SendMessageW(
                    self.controls.attachment_handoff,
                    CB_GETCURSEL,
                    Some(WPARAM(0)),
                    Some(LPARAM(0)),
                )
                .0
            };
            form.attachment_handoff = match attachment_handoff {
                0 => AttachmentHandoffPolicy::Disabled,
                1 => AttachmentHandoffPolicy::BestEffort,
                2 => AttachmentHandoffPolicy::Required,
                _ => {
                    return Err(tr(
                        "Windows信頼連携の方針を選択してください。",
                        "Select a Windows trust handoff policy.",
                    )
                    .to_owned());
                }
            };
            let encoding = unsafe {
                SendMessageW(
                    self.controls.encoding,
                    CB_GETCURSEL,
                    Some(WPARAM(0)),
                    Some(LPARAM(0)),
                )
                .0
            };
            form.default_filename_encoding = match encoding {
                0 => FilenameEncoding::Auto,
                1 => FilenameEncoding::Utf8,
                2 => FilenameEncoding::Cp932,
                3 => FilenameEncoding::Cp437,
                _ => {
                    return Err(tr(
                        "既定の文字コードを選択してください。",
                        "Select a default filename encoding.",
                    )
                    .to_owned());
                }
            };
            Ok(form)
        }

        fn save(&self, parent: HWND) -> Result<(), String> {
            let config = self.collect_config()?;
            config
                .save(&self.config_path)
                .map_err(|error| error.to_string())?;
            self.saved_config.replace(config);
            self.update_dirty_title(parent);
            set_control_text(
                self.controls.status,
                tr("設定を保存しました。", "Settings saved."),
            );
            show_message(
                Some(parent),
                tr(
                    "設定を保存しました。次回の処理から反映されます。",
                    "Settings were saved and apply to the next operation.",
                ),
                MB_OK | MB_ICONINFORMATION,
            );
            Ok(())
        }

        fn browse_backend(&self, parent: HWND) -> Result<(), String> {
            let initial = control_text(self.controls.backend)?;
            if let Some(path) = choose_folder(
                parent,
                tr(
                    "backend-manifest.tsvを含むフォルダ",
                    "Folder containing backend-manifest.tsv",
                ),
                &initial,
            )? {
                set_control_os_text(self.controls.backend, path.as_os_str());
                set_control_text(
                    self.controls.status,
                    tr(
                        "バックエンドの保存先を変更しました。保存前に診断できます。",
                        "Backend location changed. You can diagnose it before saving.",
                    ),
                );
            }
            Ok(())
        }

        fn doctor(&self, parent: HWND) -> Result<(), String> {
            let config = self.collect_config()?;
            let details = self.run_busy(
                parent,
                tr(
                    "バックエンドとAppContainerを診断しています...",
                    "Diagnosing the backend and AppContainer...",
                ),
                || {
                    let backend_dir = config
                        .backend_directory()
                        .map_err(|error| error.to_string())?;
                    BackendBundle::verify(&backend_dir).map_err(|error| error.to_string())?;

                    let executable = sibling("iroha-zip.exe")?;
                    let temporary_config = std::env::temp_dir()
                        .join(format!("iroha-zip-doctor-{}.toml", util::unique_token()));
                    config
                        .save(&temporary_config)
                        .map_err(|error| error.to_string())?;
                    let output_result = Command::new(&executable)
                        .arg("--config")
                        .arg(&temporary_config)
                        .arg("doctor")
                        .stdin(Stdio::null())
                        .creation_flags(CREATE_NO_WINDOW)
                        .output();
                    let _ = fs::remove_file(&temporary_config);
                    let output = output_result.map_err(|error| {
                        format!(
                            "{}: {error}",
                            tr("診断を開始できません", "Cannot start diagnosis")
                        )
                    })?;
                    if !output.status.success() {
                        return Err(command_failure(tr("診断", "Diagnosis"), &output));
                    }
                    Ok(decoded_output(&output))
                },
            )?;
            set_control_text(
                self.controls.status,
                tr("診断に成功しました。", "Diagnosis succeeded."),
            );
            show_message(
                Some(parent),
                &format!(
                    "{}\n\n{details}",
                    tr(
                        "バックエンドとAppContainerの診断に成功しました。",
                        "Backend and AppContainer diagnosis succeeded."
                    )
                ),
                MB_OK | MB_ICONINFORMATION,
            );
            Ok(())
        }

        fn import_backend(&self, parent: HWND, from_msys2: bool) -> Result<(), String> {
            let title = if from_msys2 {
                tr(
                    "MSYS2のルート（例: C:\\msys64）",
                    "MSYS2 root (for example, C:\\msys64)",
                )
            } else {
                tr(
                    "bsdtar.exeと依存DLLを含むbundle",
                    "Bundle containing bsdtar.exe and dependent DLLs",
                )
            };
            let Some(source) = choose_folder(parent, title, "")? else {
                return Ok(());
            };
            if !from_msys2
                && !confirm_action(
                    parent,
                    tr(
                        "警告: 任意のbundleは未対応の取得元です。配布元の署名や由来を検証できません。\n\nこの警告を理解し、未検証の取得元として取り込みますか？",
                        "Warning: An arbitrary bundle is an unsupported source. Its distributor signature and origin cannot be verified.\n\nDo you understand this warning and want to import it as an unverified source?",
                    ),
                )
            {
                set_control_text(
                    self.controls.status,
                    tr(
                        "未検証bundleの取り込みを中止しました。",
                        "Unverified bundle import cancelled.",
                    ),
                );
                return Ok(());
            }
            let config = self.collect_config()?;
            let destination = config
                .backend_directory()
                .map_err(|error| error.to_string())?;
            if destination.exists()
                && !confirm_action(
                    parent,
                    tr(
                        "現在のバックエンドを検証済みの新しいbundleで置き換えます。続行しますか？",
                        "Replace the current backend with the newly verified bundle?",
                    ),
                )
            {
                set_control_text(
                    self.controls.status,
                    tr(
                        "バックエンドの取り込みを中止しました。",
                        "Backend import cancelled.",
                    ),
                );
                return Ok(());
            }

            let (script, source_argument) = if from_msys2 {
                ("export-msys2-backend.ps1", "-Msys2Root")
            } else {
                ("install-backend.ps1", "-SourceDirectory")
            };
            let (bundle, evidence) = self.run_busy(
                parent,
                tr(
                    "バックエンドと供給元証跡を取り込み、検証しています...",
                    "Importing and verifying the backend and source evidence...",
                ),
                || {
                    let mut arguments = vec![
                        OsString::from(source_argument),
                        source.into_os_string(),
                        OsString::from("-DestinationDirectory"),
                        destination.as_os_str().to_owned(),
                    ];
                    if !from_msys2 {
                        arguments.push(OsString::from("-AllowUnsupportedSource"));
                    }
                    let output = run_script(script, &arguments)?;
                    if !output.status.success() {
                        return Err(command_failure(
                            tr("バックエンド取り込み", "Backend import"),
                            &output,
                        ));
                    }
                    let bundle =
                        BackendBundle::verify(&destination).map_err(|error| error.to_string())?;
                    let evidence =
                        BackendEvidence::verify(&bundle).map_err(|error| error.to_string())?;
                    config
                        .save(&self.config_path)
                        .map_err(|error| error.to_string())?;
                    Ok((bundle, evidence))
                },
            )?;
            self.saved_config.replace(config);
            self.update_dirty_title(parent);
            set_control_text(
                self.controls.status,
                tr(
                    "バックエンドを取り込み、検証しました。",
                    "Backend imported and verified.",
                ),
            );
            let source_description = if evidence.is_supported() {
                tr(
                    "MSYS2 UCRT64（署名方針を強制して検証済み）",
                    "MSYS2 UCRT64 (verified with enforced signature policy)",
                )
            } else {
                tr(
                    "未対応・未検証（明示承認済み）",
                    "Unsupported and unverified (explicitly approved)",
                )
            };
            let message = format!(
                "{}\n\n{}: {}\n{}: {}\n{}: {}\n{}: {}",
                tr("バックエンドを取り込みました。", "Backend imported."),
                tr("保存先", "Location"),
                bundle.root().display(),
                tr("実行ファイル", "Executable"),
                bundle.executable().display(),
                tr("取得元", "Source"),
                source_description,
                tr("証跡", "Evidence"),
                evidence.root().display()
            );
            show_message(Some(parent), &message, MB_OK | MB_ICONINFORMATION);
            Ok(())
        }

        fn association(&self, parent: HWND, register: bool) -> Result<(), String> {
            if !register
                && !confirm_action(
                    parent,
                    tr(
                        "現在のユーザーからiroha-zipの関連付け候補を解除します。続行しますか？",
                        "Remove iroha-zip as an association candidate for the current user?",
                    ),
                )
            {
                set_control_text(
                    self.controls.status,
                    tr(
                        "関連付けの解除を中止しました。",
                        "Association removal cancelled.",
                    ),
                );
                return Ok(());
            }
            let progress = if register {
                tr(
                    "関連付け候補を登録しています...",
                    "Registering association candidates...",
                )
            } else {
                tr(
                    "関連付け候補を解除しています...",
                    "Removing association candidates...",
                )
            };
            let output = self.run_busy(parent, progress, || {
                if register {
                    let install_dir = executable_directory()?;
                    run_script(
                        "register-associations.ps1",
                        &[
                            OsString::from("-InstallDirectory"),
                            install_dir.into_os_string(),
                            OsString::from("-DoNotOpenSettings"),
                        ],
                    )
                } else {
                    run_script("unregister-associations.ps1", &[])
                }
            })?;
            if !output.status.success() {
                return Err(command_failure(tr("関連付け", "Association"), &output));
            }
            let message = if register {
                tr(
                    "iroha-zipをアーカイブアプリ候補として登録しました。既定のアプリ画面で拡張子ごとの選択を確定してください。",
                    "iroha-zip is registered as an archive-app candidate. Confirm each extension in Windows Default Apps.",
                )
            } else {
                tr(
                    "iroha-zipの関連付け候補を解除しました。Windowsの既定選択は必要に応じて変更してください。",
                    "iroha-zip association candidates were removed. Change existing Windows defaults if needed.",
                )
            };
            set_control_text(self.controls.status, message);
            show_message(Some(parent), message, MB_OK | MB_ICONINFORMATION);
            Ok(())
        }

        fn open_default_apps(&self) -> Result<(), String> {
            spawn_explorer(OsStr::new("ms-settings:defaultapps"))?;
            set_control_text(
                self.controls.status,
                tr(
                    "Windowsの既定のアプリ画面を開きました。",
                    "Opened Windows Default Apps.",
                ),
            );
            Ok(())
        }

        fn open_config_folder(&self) -> Result<(), String> {
            let parent = self.config_path.parent().ok_or_else(|| {
                tr(
                    "設定ファイルに親フォルダがありません。",
                    "The configuration file has no parent folder.",
                )
                .to_owned()
            })?;
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "{}: {error}",
                    tr(
                        "設定フォルダを作成できません",
                        "Cannot create the configuration folder"
                    )
                )
            })?;
            spawn_explorer(parent.as_os_str())?;
            set_control_text(
                self.controls.status,
                tr(
                    "設定フォルダを開きました。",
                    "Opened the configuration folder.",
                ),
            );
            Ok(())
        }

        fn focus_field(&self, field: SettingsField) {
            let control = match field {
                SettingsField::General | SettingsField::BackendDirectory => self.controls.backend,
                SettingsField::TimeoutSeconds => self.controls.timeout_seconds,
                SettingsField::MemoryLimitMib => self.controls.memory_limit_mib,
                SettingsField::MaxArchiveBytes => self.controls.max_archive_bytes,
                SettingsField::MaxFiles => self.controls.max_files,
                SettingsField::MaxDirectories => self.controls.max_directories,
                SettingsField::MaxTotalBytes => self.controls.max_total_bytes,
                SettingsField::MaxSingleFileBytes => self.controls.max_single_file_bytes,
                SettingsField::MaxDepth => self.controls.max_depth,
                SettingsField::MaxPathBytes => self.controls.max_path_bytes,
            };
            if !control.is_invalid() {
                let _ = unsafe { SetFocus(Some(control)) };
            }
        }

        fn update_scrollbar(&self, parent: HWND) {
            let mut client = RECT::default();
            if unsafe { GetClientRect(parent, &raw mut client) }.is_err() {
                return;
            }
            let page_width = (client.right - client.left).max(1);
            let page_height = (client.bottom - client.top).max(1);
            let content_width = scale_for_window(parent, CONTENT_WIDTH).max(1);
            let content_height = scale_for_window(parent, CONTENT_HEIGHT).max(1);
            let max_horizontal_scroll = (content_width - page_width).max(0);
            let max_scroll = (content_height - page_height).max(0);
            if self.scroll_x.get() > max_horizontal_scroll {
                self.scroll_horizontal_to(parent, max_horizontal_scroll);
            }
            if self.scroll_y.get() > max_scroll {
                self.scroll_vertical_to(parent, max_scroll);
            }
            let horizontal_info = SCROLLINFO {
                cbSize: u32::try_from(size_of::<SCROLLINFO>()).unwrap_or(u32::MAX),
                fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
                nMin: 0,
                nMax: content_width - 1,
                nPage: u32::try_from(page_width).unwrap_or(u32::MAX),
                nPos: self.scroll_x.get(),
                nTrackPos: 0,
            };
            unsafe {
                SetScrollInfo(parent, SB_HORZ, &raw const horizontal_info, true);
            }
            let vertical_info = SCROLLINFO {
                cbSize: u32::try_from(size_of::<SCROLLINFO>()).unwrap_or(u32::MAX),
                fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
                nMin: 0,
                nMax: content_height - 1,
                nPage: u32::try_from(page_height).unwrap_or(u32::MAX),
                nPos: self.scroll_y.get(),
                nTrackPos: 0,
            };
            unsafe {
                SetScrollInfo(parent, SB_VERT, &raw const vertical_info, true);
            }
        }

        fn scroll_horizontal_to(&self, parent: HWND, requested: i32) {
            let mut client = RECT::default();
            if unsafe { GetClientRect(parent, &raw mut client) }.is_err() {
                return;
            }
            let page_width = (client.right - client.left).max(1);
            let content_width = scale_for_window(parent, CONTENT_WIDTH).max(1);
            let target = requested.clamp(0, (content_width - page_width).max(0));
            let previous = self.scroll_x.replace(target);
            if previous != target {
                unsafe {
                    ScrollWindowEx(
                        parent,
                        previous - target,
                        0,
                        None,
                        None,
                        None,
                        None,
                        SW_SCROLLCHILDREN | SW_INVALIDATE | SW_ERASE,
                    );
                }
            }
            let info = SCROLLINFO {
                cbSize: u32::try_from(size_of::<SCROLLINFO>()).unwrap_or(u32::MAX),
                fMask: SIF_POS,
                nPos: target,
                ..Default::default()
            };
            unsafe {
                SetScrollInfo(parent, SB_HORZ, &raw const info, true);
            }
        }

        fn scroll_vertical_to(&self, parent: HWND, requested: i32) {
            let mut client = RECT::default();
            if unsafe { GetClientRect(parent, &raw mut client) }.is_err() {
                return;
            }
            let page_height = (client.bottom - client.top).max(1);
            let content_height = scale_for_window(parent, CONTENT_HEIGHT).max(1);
            let target = requested.clamp(0, (content_height - page_height).max(0));
            let previous = self.scroll_y.replace(target);
            if previous != target {
                unsafe {
                    ScrollWindowEx(
                        parent,
                        0,
                        previous - target,
                        None,
                        None,
                        None,
                        None,
                        SW_SCROLLCHILDREN | SW_INVALIDATE | SW_ERASE,
                    );
                }
            }
            let info = SCROLLINFO {
                cbSize: u32::try_from(size_of::<SCROLLINFO>()).unwrap_or(u32::MAX),
                fMask: SIF_POS,
                nPos: target,
                ..Default::default()
            };
            unsafe {
                SetScrollInfo(parent, SB_VERT, &raw const info, true);
            }
        }

        fn handle_scroll(&self, parent: HWND, wparam: WPARAM, horizontal: bool) {
            let bar = if horizontal { SB_HORZ } else { SB_VERT };
            let mut info = SCROLLINFO {
                cbSize: u32::try_from(size_of::<SCROLLINFO>()).unwrap_or(u32::MAX),
                fMask: SIF_ALL,
                ..Default::default()
            };
            if unsafe { GetScrollInfo(parent, bar, &raw mut info) }.is_err() {
                return;
            }
            let command = i32::from(u16::try_from(wparam.0 & 0xffff).unwrap_or(0));
            let line = scale_for_window(parent, 32).max(1);
            let page = i32::try_from(info.nPage).unwrap_or(i32::MAX).max(line);
            let requested = if command == SB_LINEUP.0 {
                info.nPos - line
            } else if command == SB_LINEDOWN.0 {
                info.nPos + line
            } else if command == SB_PAGEUP.0 {
                info.nPos - page
            } else if command == SB_PAGEDOWN.0 {
                info.nPos + page
            } else if command == SB_THUMBPOSITION.0 || command == SB_THUMBTRACK.0 {
                info.nTrackPos
            } else if command == SB_TOP.0 {
                0
            } else if command == SB_BOTTOM.0 {
                i32::MAX
            } else {
                info.nPos
            };
            if horizontal {
                self.scroll_horizontal_to(parent, requested);
            } else {
                self.scroll_vertical_to(parent, requested);
            }
        }

        fn handle_mouse_wheel(&self, parent: HWND, wparam: WPARAM) {
            let wheel_bits = u16::try_from((wparam.0 >> 16) & 0xffff).unwrap_or(0);
            let delta = i32::from(wheel_bits.cast_signed());
            if delta != 0 {
                let line = scale_for_window(parent, 32).max(1);
                self.scroll_vertical_to(parent, self.scroll_y.get() - delta.signum() * line * 3);
            }
        }

        fn ensure_focus_visible(&self, parent: HWND) {
            let focus = unsafe { GetFocus() };
            if focus.is_invalid() || !unsafe { IsChild(parent, focus) }.as_bool() {
                return;
            }
            let mut focus_rect = RECT::default();
            let mut client = RECT::default();
            if unsafe { GetWindowRect(focus, &raw mut focus_rect) }.is_err()
                || unsafe { GetClientRect(parent, &raw mut client) }.is_err()
            {
                return;
            }
            let mut top_left = POINT {
                x: focus_rect.left,
                y: focus_rect.top,
            };
            let mut bottom_right = POINT {
                x: focus_rect.right,
                y: focus_rect.bottom,
            };
            unsafe {
                let _ = ScreenToClient(parent, &raw mut top_left);
                let _ = ScreenToClient(parent, &raw mut bottom_right);
            }
            let margin = scale_for_window(parent, 12).max(1);
            if top_left.x < margin {
                self.scroll_horizontal_to(parent, self.scroll_x.get() + top_left.x - margin);
            } else if bottom_right.x > client.right - margin {
                self.scroll_horizontal_to(
                    parent,
                    self.scroll_x.get() + bottom_right.x - client.right + margin,
                );
            }
            if top_left.y < margin {
                self.scroll_vertical_to(parent, self.scroll_y.get() + top_left.y - margin);
            } else if bottom_right.y > client.bottom - margin {
                self.scroll_vertical_to(
                    parent,
                    self.scroll_y.get() + bottom_right.y - client.bottom + margin,
                );
            }
        }

        fn has_unsaved_changes(&self) -> bool {
            match self
                .read_form()
                .and_then(|form| form.into_config().map_err(|error| error.to_string()))
            {
                Ok(config) => config != *self.saved_config.borrow(),
                Err(_) => true,
            }
        }

        fn update_dirty_title(&self, parent: HWND) {
            let dirty = self.has_unsaved_changes();
            let marker = if dirty { " *" } else { "" };
            set_control_text(
                parent,
                &format!(
                    "{} — v{}{marker}",
                    window_title(),
                    env!("CARGO_PKG_VERSION")
                ),
            );
        }

        fn request_close(&self, parent: HWND) -> Result<(), String> {
            if self.has_unsaved_changes()
                && !confirm_action(
                    parent,
                    tr(
                        "保存していない変更を破棄して閉じますか？",
                        "Discard unsaved changes and close?",
                    ),
                )
            {
                return Ok(());
            }
            unsafe { DestroyWindow(parent).map_err(|error| error.to_string()) }
        }

        fn run_busy<T>(
            &self,
            parent: HWND,
            status: &str,
            operation: impl FnOnce() -> Result<T, String>,
        ) -> Result<T, String> {
            set_control_text(self.controls.status, status);
            unsafe {
                let _ = EnableWindow(parent, false);
                let _ = UpdateWindow(parent);
                if let Ok(cursor) = LoadCursorW(None, IDC_WAIT) {
                    SetCursor(Some(cursor));
                }
            }
            let result = operation();
            unsafe {
                let _ = EnableWindow(parent, true);
                if let Ok(cursor) = LoadCursorW(None, IDC_ARROW) {
                    SetCursor(Some(cursor));
                }
            }
            result
        }

        fn handle_command(&self, parent: HWND, id: usize) -> Result<(), String> {
            let Some(action) = SettingsAction::from_control_id(id) else {
                return Ok(());
            };
            match action {
                SettingsAction::BrowseBackend => self.browse_backend(parent),
                SettingsAction::DiagnoseBackend => self.doctor(parent),
                SettingsAction::ImportBackendBundle => self.import_backend(parent, false),
                SettingsAction::ImportMsys2Backend => self.import_backend(parent, true),
                SettingsAction::RegisterAssociations => self.association(parent, true),
                SettingsAction::UnregisterAssociations => self.association(parent, false),
                SettingsAction::OpenDefaultApps => self.open_default_apps(),
                SettingsAction::OpenConfigFolder => self.open_config_folder(),
                SettingsAction::RestoreDefaults => {
                    if !confirm_action(
                        parent,
                        tr(
                            "画面上のすべての設定を安全な既定値へ戻します。保存するまで反映されません。続行しますか？",
                            "Restore every displayed setting to its safe default? Changes do not apply until saved.",
                        ),
                    ) {
                        return Ok(());
                    }
                    self.apply_config(&Config::default());
                    self.update_dirty_title(parent);
                    set_control_text(
                        self.controls.status,
                        tr(
                            "既定値を表示しています。保存すると反映されます。",
                            "Safe defaults are displayed. Save to apply them.",
                        ),
                    );
                    Ok(())
                }
                SettingsAction::Save => self.save(parent),
                SettingsAction::Cancel => self.request_close(parent),
            }
        }
    }

    pub fn run() -> Result<(), String> {
        let _ = LANGUAGE.set(detect_language());
        let config_path = parse_config_path()?;
        Config::load(&config_path).map_err(|error| error.to_string())?;

        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .ok()
                .map_err(|error| {
                    format!(
                        "{}: {error}",
                        tr("COMを初期化できません", "Cannot initialize COM")
                    )
                })?;
        }
        let result = unsafe { run_message_loop(config_path) };
        unsafe { CoUninitialize() };
        result
    }

    unsafe fn run_message_loop(config_path: PathBuf) -> Result<(), String> {
        let module = unsafe { GetModuleHandleW(None) }.map_err(|error| error.to_string())?;
        let instance = HINSTANCE(module.0);
        let class_name = wide(WINDOW_CLASS);
        let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }.map_err(|error| error.to_string())?;
        let window_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: cursor,
            hbrBackground: unsafe { GetSysColorBrush(COLOR_3DFACE) },
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        if unsafe { RegisterClassW(&raw const window_class) } == 0 {
            return Err(format!(
                "{}: {}",
                tr(
                    "設定画面のwindow classを登録できません",
                    "Cannot register the settings window class"
                ),
                WindowsError::from_thread()
            ));
        }

        let app = Box::new(App::new(config_path));
        let app_pointer = Box::into_raw(app);
        let title = wide(&format!(
            "{} — v{}",
            window_title(),
            env!("CARGO_PKG_VERSION")
        ));
        let dpi = unsafe { GetDpiForSystem() }.max(BASE_DPI);
        let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        let available_width = (screen_width - scale_logical(32, dpi)).max(320);
        let available_height = (screen_height - scale_logical(32, dpi)).max(320);
        let window_width = scale_logical(950, dpi).min(available_width);
        let window_height = scale_logical(760, dpi).min(available_height);
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPED
                    | WS_CAPTION
                    | WS_SYSMENU
                    | WS_MINIMIZEBOX
                    | WS_MAXIMIZEBOX
                    | WS_THICKFRAME
                    | WS_HSCROLL
                    | WS_VSCROLL,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                window_width,
                window_height,
                None,
                None,
                Some(instance),
                Some(app_pointer.cast_const().cast::<c_void>()),
            )
        };
        let window = match window {
            Ok(window) => window,
            Err(error) => {
                unsafe { drop(Box::from_raw(app_pointer)) };
                return Err(format!(
                    "{}: {error}",
                    tr(
                        "設定画面を作成できません",
                        "Cannot create the settings window"
                    )
                ));
            }
        };

        unsafe {
            let _ = ShowWindow(window, SW_SHOWNORMAL);
            let _ = UpdateWindow(window);
        }

        let mut message = MSG::default();
        loop {
            let result = unsafe { GetMessageW(&raw mut message, None, 0, 0) };
            if result.0 == -1 {
                unsafe { drop(Box::from_raw(app_pointer)) };
                return Err(format!(
                    "{}: {}",
                    tr(
                        "window messageを取得できません",
                        "Cannot retrieve a window message"
                    ),
                    WindowsError::from_thread()
                ));
            }
            if result.0 == 0 {
                break;
            }
            if unsafe { IsDialogMessageW(window, &raw const message) }.as_bool() {
                unsafe { (*app_pointer).ensure_focus_visible(window) };
                continue;
            }
            unsafe {
                let _ = TranslateMessage(&raw const message);
                DispatchMessageW(&raw const message);
                (*app_pointer).ensure_focus_visible(window);
            }
        }
        unsafe { drop(Box::from_raw(app_pointer)) };
        Ok(())
    }

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if message == WM_NCCREATE {
            let create = lparam.0 as *const CREATESTRUCTW;
            if !create.is_null() {
                unsafe {
                    SetWindowLongPtrW(window, GWLP_USERDATA, (*create).lpCreateParams as isize);
                }
            }
        }

        let app_pointer = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) } as *mut App;
        match message {
            WM_CREATE => {
                if app_pointer.is_null() {
                    return LRESULT(-1);
                }
                let instance = unsafe {
                    let create = &*(lparam.0 as *const CREATESTRUCTW);
                    create.hInstance
                };
                if let Err(error) = unsafe { (*app_pointer).create_controls(window, instance) } {
                    show_error(Some(window), &error);
                    return LRESULT(-1);
                }
                LRESULT(0)
            }
            WM_COMMAND => {
                let id = wparam.0 & 0xffff;
                let notification = (wparam.0 >> 16) & 0xffff;
                if notification == BN_CLICKED as usize
                    && !app_pointer.is_null()
                    && let Err(error) = unsafe { (*app_pointer).handle_command(window, id) }
                {
                    show_error(Some(window), &error);
                    if !unsafe { (*app_pointer).controls.status }.is_invalid() {
                        set_control_text(
                            unsafe { (*app_pointer).controls.status },
                            tr(
                                "処理に失敗しました。詳細はエラーダイアログを確認してください。",
                                "Operation failed. See the error dialog for details.",
                            ),
                        );
                    }
                }
                if !app_pointer.is_null()
                    && control_id::is_setting(id)
                    && (notification == EN_CHANGE as usize
                        || notification == CBN_SELCHANGE as usize
                        || notification == BN_CLICKED as usize)
                {
                    unsafe { (*app_pointer).update_dirty_title(window) };
                }
                LRESULT(0)
            }
            WM_SIZE => {
                if !app_pointer.is_null() {
                    unsafe { (*app_pointer).update_scrollbar(window) };
                }
                LRESULT(0)
            }
            WM_VSCROLL => {
                if !app_pointer.is_null() && lparam.0 == 0 {
                    unsafe { (*app_pointer).handle_scroll(window, wparam, false) };
                }
                LRESULT(0)
            }
            WM_HSCROLL => {
                if !app_pointer.is_null() && lparam.0 == 0 {
                    unsafe { (*app_pointer).handle_scroll(window, wparam, true) };
                }
                LRESULT(0)
            }
            WM_MOUSEWHEEL => {
                if !app_pointer.is_null() {
                    unsafe { (*app_pointer).handle_mouse_wheel(window, wparam) };
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                if app_pointer.is_null() {
                    let _ = unsafe { DestroyWindow(window) };
                } else if let Err(error) = unsafe { (*app_pointer).request_close(window) } {
                    show_error(Some(window), &error);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
        }
    }

    unsafe fn add_static(
        parent: HWND,
        instance: HINSTANCE,
        text: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<HWND, String> {
        unsafe {
            add_control(
                parent,
                instance,
                WINDOW_EX_STYLE(0),
                "STATIC",
                text,
                WS_CHILD | WS_VISIBLE,
                x,
                y,
                width,
                height,
                None,
            )
        }
    }

    unsafe fn add_group(
        parent: HWND,
        instance: HINSTANCE,
        text: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<HWND, String> {
        unsafe {
            add_control(
                parent,
                instance,
                WINDOW_EX_STYLE(0),
                "BUTTON",
                text,
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_GROUPBOX as u32),
                x,
                y,
                width,
                height,
                None,
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn add_edit(
        parent: HWND,
        instance: HINSTANCE,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        numeric: bool,
        id: usize,
    ) -> Result<HWND, String> {
        let mut style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32);
        if numeric {
            style |= WINDOW_STYLE(ES_NUMBER as u32);
        }
        unsafe {
            add_control(
                parent,
                instance,
                WS_EX_CLIENTEDGE,
                "EDIT",
                "",
                style,
                x,
                y,
                width,
                height,
                Some(id),
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn add_button(
        parent: HWND,
        instance: HINSTANCE,
        text: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        id: usize,
        default: bool,
    ) -> Result<HWND, String> {
        let kind = if default {
            BS_DEFPUSHBUTTON
        } else {
            BS_PUSHBUTTON
        };
        unsafe {
            add_control(
                parent,
                instance,
                WINDOW_EX_STYLE(0),
                "BUTTON",
                text,
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(kind.cast_unsigned()),
                x,
                y,
                width,
                height,
                Some(id),
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn add_checkbox(
        parent: HWND,
        instance: HINSTANCE,
        text: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        id: usize,
    ) -> Result<HWND, String> {
        unsafe {
            add_control(
                parent,
                instance,
                WINDOW_EX_STYLE(0),
                "BUTTON",
                text,
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                x,
                y,
                width,
                height,
                Some(id),
            )
        }
    }

    unsafe fn add_combo(
        parent: HWND,
        instance: HINSTANCE,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        id: usize,
    ) -> Result<HWND, String> {
        unsafe {
            add_control(
                parent,
                instance,
                WINDOW_EX_STYLE(0),
                "COMBOBOX",
                "",
                WS_CHILD
                    | WS_VISIBLE
                    | WS_TABSTOP
                    | WS_VSCROLL
                    | WINDOW_STYLE((CBS_DROPDOWNLIST | CBS_HASSTRINGS) as u32),
                x,
                y,
                width,
                height,
                Some(id),
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn add_control(
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
    ) -> Result<HWND, String> {
        let class = wide(class);
        let text = wide(text);
        let menu = id.map(|value| HMENU(value as *mut c_void));
        let x = scale_for_window(parent, x);
        let y = scale_for_window(parent, y);
        let width = scale_for_window(parent, width);
        let height = scale_for_window(parent, height);
        let control = unsafe {
            CreateWindowExW(
                extended_style,
                PCWSTR(class.as_ptr()),
                PCWSTR(text.as_ptr()),
                style,
                x,
                y,
                width,
                height,
                Some(parent),
                menu,
                Some(instance),
                None,
            )
        }
        .map_err(|error| {
            format!(
                "{}: {error}",
                tr("controlを作成できません", "Cannot create a control")
            )
        })?;
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

    fn scale_for_window(window: HWND, value: i32) -> i32 {
        let dpi = unsafe { GetDpiForWindow(window) }.max(BASE_DPI);
        scale_logical(value, dpi)
    }

    fn set_control_text(control: HWND, text: &str) {
        set_control_os_text(control, OsStr::new(text));
    }

    fn set_control_os_text(control: HWND, text: &OsStr) {
        let text = wide_os(text);
        let _ = unsafe { SetWindowTextW(control, PCWSTR(text.as_ptr())) };
    }

    fn control_text(control: HWND) -> Result<String, String> {
        let length = unsafe { GetWindowTextLengthW(control) };
        if length < 0 {
            return Err(tr(
                "入力値の長さを取得できません。",
                "Cannot read the input length.",
            )
            .to_owned());
        }
        let capacity = usize::try_from(length)
            .map_err(|_| tr("入力値が長すぎます。", "The input is too long.").to_owned())?
            .saturating_add(1);
        let mut buffer = vec![0u16; capacity];
        let copied = unsafe { GetWindowTextW(control, &mut buffer) };
        if copied < 0 {
            return Err(tr("入力値を取得できません。", "Cannot read the input.").to_owned());
        }
        buffer.truncate(usize::try_from(copied).unwrap_or(0));
        Ok(String::from_utf16_lossy(&buffer))
    }

    fn set_check(control: HWND, checked: bool) {
        let state = if checked { BUTTON_CHECKED } else { 0 };
        unsafe {
            SendMessageW(control, BM_SETCHECK, Some(WPARAM(state)), Some(LPARAM(0)));
        }
    }

    fn is_checked(control: HWND) -> bool {
        unsafe {
            SendMessageW(control, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0))).0
                == BUTTON_CHECKED.cast_signed()
        }
    }

    fn combo_add(control: HWND, text: &str) {
        let text = wide(text);
        unsafe {
            SendMessageW(
                control,
                CB_ADDSTRING,
                Some(WPARAM(0)),
                Some(LPARAM(text.as_ptr() as isize)),
            );
        }
    }

    fn choose_folder(owner: HWND, title: &str, initial: &str) -> Result<Option<PathBuf>, String> {
        let dialog: IFileOpenDialog =
            unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }.map_err(
                |error| {
                    format!(
                        "{}: {error}",
                        tr(
                            "フォルダ選択画面を作成できません",
                            "Cannot create the folder picker"
                        )
                    )
                },
            )?;
        let options = unsafe { dialog.GetOptions() }.map_err(|error| error.to_string())?;
        unsafe {
            dialog
                .SetOptions(
                    options
                        | FOS_PICKFOLDERS
                        | FOS_FORCEFILESYSTEM
                        | FOS_PATHMUSTEXIST
                        | FOS_DONTADDTORECENT,
                )
                .map_err(|error| error.to_string())?;
            let title = wide(title);
            dialog
                .SetTitle(PCWSTR(title.as_ptr()))
                .map_err(|error| error.to_string())?;
        }

        if !initial.trim().is_empty() {
            let initial_path = PathBuf::from(initial.trim());
            if initial_path.exists() {
                let path = wide_os(initial_path.as_os_str());
                if let Ok(item) = unsafe {
                    SHCreateItemFromParsingName::<_, _, IShellItem>(PCWSTR(path.as_ptr()), None)
                } {
                    let _ = unsafe { dialog.SetFolder(&item) };
                }
            }
        }

        match unsafe { dialog.Show(Some(owner)) } {
            Ok(()) => {}
            Err(error) if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) => {
                return Ok(None);
            }
            Err(error) => {
                return Err(format!(
                    "{}: {error}",
                    tr("フォルダを選択できません", "Cannot select a folder")
                ));
            }
        }
        let item = unsafe { dialog.GetResult() }.map_err(|error| error.to_string())?;
        let display_name =
            unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }.map_err(|error| error.to_string())?;
        if display_name.is_null() {
            return Err(tr(
                "選択したフォルダのパスを取得できません。",
                "Cannot retrieve the selected folder path.",
            )
            .to_owned());
        }
        let path = unsafe { os_string_from_wide_ptr(display_name.0) };
        unsafe { CoTaskMemFree(Some(display_name.0.cast::<c_void>())) };
        Ok(Some(PathBuf::from(path)))
    }

    fn run_script(name: &str, arguments: &[OsString]) -> Result<std::process::Output, String> {
        let allowed = [
            "install-backend.ps1",
            "export-msys2-backend.ps1",
            "register-associations.ps1",
            "unregister-associations.ps1",
        ];
        if !allowed.contains(&name) {
            return Err(tr(
                "許可されていない管理scriptです。",
                "The management script is not allowed.",
            )
            .to_owned());
        }
        let script = executable_directory()?.join("scripts").join(name);
        if !script.is_file() {
            return Err(format!(
                "{}: {}",
                tr("管理scriptが見つかりません", "Management script not found"),
                script.display()
            ));
        }
        let windows_directory = std::env::var_os("SystemRoot")
            .or_else(|| std::env::var_os("WINDIR"))
            .unwrap_or_else(|| OsString::from(r"C:\Windows"));
        let powershell = PathBuf::from(windows_directory)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        Command::new(&powershell)
            .arg("-NoLogo")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&script)
            .args(arguments)
            .stdin(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|error| {
                format!(
                    "{}: {error}",
                    tr("PowerShellを開始できません", "Cannot start PowerShell")
                )
            })
    }

    fn command_failure(operation: &str, output: &std::process::Output) -> String {
        let details = decoded_output(output);
        let exit_code = output.status.code().unwrap_or(-1);
        match selected_language() {
            Language::Japanese => {
                format!("{operation}に失敗しました（終了コード: {exit_code}）。\n\n{details}")
            }
            Language::English => {
                format!("{operation} failed (exit code: {exit_code}).\n\n{details}")
            }
        }
    }

    fn decoded_output(output: &std::process::Output) -> String {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}\n{stderr}");
        combined.trim().chars().take(8_000).collect()
    }

    fn executable_directory() -> Result<PathBuf, String> {
        let executable = std::env::current_exe().map_err(|error| {
            format!(
                "{}: {error}",
                tr(
                    "実行ファイルの場所を取得できません",
                    "Cannot locate the executable"
                )
            )
        })?;
        executable.parent().map(Path::to_path_buf).ok_or_else(|| {
            tr(
                "実行ファイルに親フォルダがありません。",
                "The executable has no parent folder.",
            )
            .to_owned()
        })
    }

    fn sibling(name: &str) -> Result<PathBuf, String> {
        Ok(executable_directory()?.join(name))
    }

    fn spawn_explorer(argument: &OsStr) -> Result<(), String> {
        let windows_directory = std::env::var_os("WINDIR")
            .or_else(|| std::env::var_os("SystemRoot"))
            .unwrap_or_else(|| OsString::from(r"C:\Windows"));
        let explorer = PathBuf::from(windows_directory).join("explorer.exe");
        Command::new(&explorer)
            .arg(argument)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                format!(
                    "{}: {error}",
                    tr("Explorerを開けません", "Cannot open File Explorer")
                )
            })?;
        Ok(())
    }

    fn parse_config_path() -> Result<PathBuf, String> {
        let mut arguments = std::env::args_os().skip(1);
        match arguments.next() {
            None => default_config_path().map_err(|error| error.to_string()),
            Some(flag) if flag == "--config" => {
                let path = arguments.next().map(PathBuf::from).ok_or_else(|| {
                    tr(
                        "--configには設定ファイルのパスが必要です。",
                        "--config requires a configuration file path.",
                    )
                    .to_owned()
                })?;
                if arguments.next().is_some() {
                    return Err(tr(
                        "設定アプリに不要な引数が指定されました。",
                        "Unexpected arguments were supplied to the settings application.",
                    )
                    .to_owned());
                }
                Ok(path)
            }
            Some(_) => Err(tr(
                "使用方法: iroha-zip-settings.exe [--config PATH]",
                "Usage: iroha-zip-settings.exe [--config PATH]",
            )
            .to_owned()),
        }
    }

    fn show_error(owner: Option<HWND>, message: &str) {
        show_message(owner, message, MB_OK | MB_ICONERROR);
    }

    fn confirm_action(owner: HWND, message: &str) -> bool {
        let message = wide(message);
        let title = wide(window_title());
        unsafe {
            MessageBoxW(
                Some(owner),
                PCWSTR(message.as_ptr()),
                PCWSTR(title.as_ptr()),
                MB_YESNO | MB_ICONWARNING,
            ) == IDYES
        }
    }

    fn show_message(owner: Option<HWND>, message: &str, style: MESSAGEBOX_STYLE) {
        let message = wide(message);
        let title = wide(window_title());
        unsafe {
            MessageBoxW(
                owner,
                PCWSTR(message.as_ptr()),
                PCWSTR(title.as_ptr()),
                style,
            );
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        wide_os(OsStr::new(value))
    }

    fn wide_os(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    unsafe fn os_string_from_wide_ptr(value: *const u16) -> OsString {
        let mut length = 0usize;
        while unsafe { *value.add(length) } != 0 {
            length += 1;
        }
        OsString::from_wide(unsafe { std::slice::from_raw_parts(value, length) })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn explicit_language_tags_cover_japanese_and_english() {
            assert_eq!(
                language_from_tag(OsStr::new("ja-JP")),
                Some(Language::Japanese)
            );
            assert_eq!(
                language_from_tag(OsStr::new("en_US")),
                Some(Language::English)
            );
            assert_eq!(language_from_tag(OsStr::new("fr-FR")), None);
        }
    }
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    match windows_app::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
            use windows::core::PCWSTR;

            let body: Vec<u16> = OsStr::new(&error)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let title: Vec<u16> = OsStr::new(windows_app::window_title())
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                MessageBoxW(
                    None,
                    PCWSTR(body.as_ptr()),
                    PCWSTR(title.as_ptr()),
                    MB_OK | MB_ICONERROR,
                );
            }
            std::process::ExitCode::from(2)
        }
    }
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("iroha-zip-settings: the graphical settings screen is available on Windows");
    std::process::ExitCode::from(2)
}
