#![cfg_attr(windows, windows_subsystem = "windows")]
#![cfg_attr(windows, allow(unsafe_code))]
#![cfg_attr(not(windows), deny(unsafe_code))]

#[cfg(windows)]
mod windows_app {
    use safearc::backend::BackendBundle;
    use safearc::config::{Config, FilenameEncoding, default_config_path};
    use safearc::settings::{SettingsField, SettingsForm};
    use safearc::util;
    use std::cell::RefCell;
    use std::ffi::{OsStr, OsString, c_void};
    use std::fs;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use windows::Win32::Foundation::{ERROR_CANCELLED, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        COLOR_3DFACE, DEFAULT_GUI_FONT, GetStockObject, GetSysColorBrush, UpdateWindow,
    };
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoTaskMemFree, CoUninitialize,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, SetFocus};
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
        GWLP_USERDATA, GetMessageW, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, HMENU,
        IDC_ARROW, IDC_WAIT, IDYES, IsDialogMessageW, LoadCursorW, MB_ICONERROR,
        MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MB_YESNO, MESSAGEBOX_STYLE, MSG, MessageBoxW,
        PostQuitMessage, RegisterClassW, SW_SHOWNORMAL, SendMessageW, SetCursor, SetWindowLongPtrW,
        SetWindowTextW, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE,
        WM_COMMAND, WM_CREATE, WM_DESTROY, WM_NCCREATE, WM_SETFONT, WNDCLASSW, WS_CAPTION,
        WS_CHILD, WS_EX_CLIENTEDGE, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP,
        WS_VISIBLE, WS_VSCROLL,
    };
    use windows::core::{Error as WindowsError, HRESULT, PCWSTR};

    const WINDOW_CLASS: &str = "SafeArc.Settings.Window";
    const WINDOW_TITLE: &str = "SafeArc 設定";
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const BUTTON_CHECKED: usize = 1;

    const ID_BACKEND_BROWSE: usize = 1001;
    const ID_BACKEND_DOCTOR: usize = 1002;
    const ID_BACKEND_IMPORT: usize = 1003;
    const ID_BACKEND_MSYS2: usize = 1004;
    const ID_REGISTER: usize = 1101;
    const ID_UNREGISTER: usize = 1102;
    const ID_DEFAULT_APPS: usize = 1103;
    const ID_CONFIG_FOLDER: usize = 1104;
    const ID_DEFAULTS: usize = 1201;
    // IDOK and IDCANCEL let IsDialogMessageW map Enter and Escape naturally.
    const ID_SAVE: usize = 1;
    const ID_CANCEL: usize = 2;

    #[derive(Default)]
    struct Controls {
        backend: HWND,
        timeout_seconds: HWND,
        memory_limit_mib: HWND,
        max_archive_bytes: HWND,
        max_files: HWND,
        max_directories: HWND,
        max_total_bytes: HWND,
        max_single_file_bytes: HWND,
        max_depth: HWND,
        max_path_bytes: HWND,
        preserve_motw: HWND,
        open_after_double_click: HWND,
        encoding: HWND,
        status: HWND,
    }

    struct App {
        config_path: PathBuf,
        controls: Controls,
        saved_config: RefCell<Config>,
    }

    impl App {
        fn new(config_path: PathBuf) -> Self {
            Self {
                config_path,
                controls: Controls::default(),
                saved_config: RefCell::new(Config::default()),
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
                    "安全性に関わる全設定とWindows統合を、ここから検証・管理できます。",
                    18,
                    12,
                    888,
                    22,
                )?;
                add_static(
                    parent,
                    instance,
                    &format!("設定ファイル: {}", self.config_path.display()),
                    18,
                    36,
                    888,
                    20,
                )?;

                add_group(parent, instance, "バックエンド", 14, 60, 904, 108)?;
                add_static(parent, instance, "保存先", 28, 86, 92, 22)?;
                self.controls.backend = add_edit(parent, instance, 122, 82, 558, 25, false)?;
                add_button(
                    parent,
                    instance,
                    "選択(&B)...",
                    690,
                    81,
                    96,
                    27,
                    ID_BACKEND_BROWSE,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    "診断(&D)",
                    796,
                    81,
                    104,
                    27,
                    ID_BACKEND_DOCTOR,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    "bundleを取り込む(&I)...",
                    122,
                    119,
                    190,
                    28,
                    ID_BACKEND_IMPORT,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    "MSYS2から取り込む(&M)...",
                    322,
                    119,
                    190,
                    28,
                    ID_BACKEND_MSYS2,
                    false,
                )?;
                add_static(
                    parent,
                    instance,
                    "取り込み時にSHA-256 manifestを生成し、既存bundleを安全に置換します。",
                    526,
                    123,
                    372,
                    22,
                )?;

                add_group(parent, instance, "AppContainer", 14, 174, 904, 67)?;
                add_static(
                    parent,
                    instance,
                    "タイムアウト（1–86400秒）",
                    28,
                    201,
                    190,
                    22,
                )?;
                self.controls.timeout_seconds =
                    add_edit(parent, instance, 220, 197, 180, 25, true)?;
                add_static(
                    parent,
                    instance,
                    "メモリ上限（64 MiB以上）",
                    480,
                    201,
                    190,
                    22,
                )?;
                self.controls.memory_limit_mib =
                    add_edit(parent, instance, 676, 197, 180, 25, true)?;

                add_group(parent, instance, "展開・作成の上限", 14, 247, 904, 194)?;
                add_static(parent, instance, "入力書庫の上限", 28, 276, 164, 22)?;
                self.controls.max_archive_bytes =
                    add_edit(parent, instance, 194, 272, 222, 25, false)?;
                add_static(parent, instance, "ファイル数", 478, 276, 150, 22)?;
                self.controls.max_files = add_edit(parent, instance, 636, 272, 222, 25, true)?;

                add_static(parent, instance, "ディレクトリ数", 28, 316, 164, 22)?;
                self.controls.max_directories =
                    add_edit(parent, instance, 194, 312, 222, 25, true)?;
                add_static(parent, instance, "合計容量の上限", 478, 316, 150, 22)?;
                self.controls.max_total_bytes =
                    add_edit(parent, instance, 636, 312, 222, 25, false)?;

                add_static(parent, instance, "単一ファイル上限", 28, 356, 164, 22)?;
                self.controls.max_single_file_bytes =
                    add_edit(parent, instance, 194, 352, 222, 25, false)?;
                add_static(parent, instance, "パスの深さ", 478, 356, 150, 22)?;
                self.controls.max_depth = add_edit(parent, instance, 636, 352, 222, 25, true)?;

                add_static(parent, instance, "パス長（UTF-8 bytes）", 28, 396, 164, 22)?;
                self.controls.max_path_bytes = add_edit(parent, instance, 194, 392, 222, 25, true)?;
                add_static(
                    parent,
                    instance,
                    "容量は 16 GiB / 512 MiB のように入力できます。",
                    478,
                    396,
                    380,
                    22,
                )?;

                add_group(parent, instance, "展開時の動作", 14, 447, 904, 78)?;
                self.controls.preserve_motw = add_checkbox(
                    parent,
                    instance,
                    "Mark-of-the-Webを展開後のファイルへ引き継ぐ",
                    30,
                    474,
                    350,
                    24,
                )?;
                self.controls.open_after_double_click = add_checkbox(
                    parent,
                    instance,
                    "ダブルクリック展開後にフォルダを開く",
                    30,
                    498,
                    350,
                    24,
                )?;
                add_static(parent, instance, "既定の文字コード", 486, 481, 142, 22)?;
                self.controls.encoding = add_combo(parent, instance, 636, 475, 222, 120)?;
                for label in ["自動判定", "UTF-8", "CP932（日本語）", "CP437"] {
                    combo_add(self.controls.encoding, label);
                }

                add_group(parent, instance, "Windows 統合", 14, 531, 904, 72)?;
                add_button(
                    parent,
                    instance,
                    "関連付けを登録(&A)",
                    30,
                    557,
                    160,
                    29,
                    ID_REGISTER,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    "関連付けを解除(&U)",
                    200,
                    557,
                    160,
                    29,
                    ID_UNREGISTER,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    "既定のアプリを開く(&P)",
                    370,
                    557,
                    180,
                    29,
                    ID_DEFAULT_APPS,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    "設定フォルダを開く(&F)",
                    560,
                    557,
                    180,
                    29,
                    ID_CONFIG_FOLDER,
                    false,
                )?;

                add_button(
                    parent,
                    instance,
                    "既定値に戻す(&R)",
                    18,
                    616,
                    150,
                    32,
                    ID_DEFAULTS,
                    false,
                )?;
                add_button(
                    parent,
                    instance,
                    "保存(&S)",
                    670,
                    616,
                    110,
                    32,
                    ID_SAVE,
                    true,
                )?;
                add_button(
                    parent,
                    instance,
                    "閉じる(&C)",
                    790,
                    616,
                    110,
                    32,
                    ID_CANCEL,
                    false,
                )?;
                self.controls.status =
                    add_static(parent, instance, "設定を読み込みました。", 18, 656, 882, 22)?;
            }

            self.apply_config(&config);
            self.saved_config.replace(config);
            self.update_dirty_title(parent);
            Ok(())
        }

        fn apply_config(&self, config: &Config) {
            let form = SettingsForm::from_config(config);
            set_control_text(self.controls.backend, &form.backend_directory);
            set_control_text(self.controls.timeout_seconds, &form.timeout_seconds);
            set_control_text(self.controls.memory_limit_mib, &form.memory_limit_mib);
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
                error.to_string()
            })
        }

        fn read_form(&self) -> Result<SettingsForm, String> {
            let mut form = SettingsForm {
                backend_directory: control_text(self.controls.backend)?,
                timeout_seconds: control_text(self.controls.timeout_seconds)?,
                memory_limit_mib: control_text(self.controls.memory_limit_mib)?,
                max_archive_bytes: control_text(self.controls.max_archive_bytes)?,
                max_files: control_text(self.controls.max_files)?,
                max_directories: control_text(self.controls.max_directories)?,
                max_total_bytes: control_text(self.controls.max_total_bytes)?,
                max_single_file_bytes: control_text(self.controls.max_single_file_bytes)?,
                max_depth: control_text(self.controls.max_depth)?,
                max_path_bytes: control_text(self.controls.max_path_bytes)?,
                preserve_mark_of_the_web: is_checked(self.controls.preserve_motw),
                open_after_double_click: is_checked(self.controls.open_after_double_click),
                default_filename_encoding: FilenameEncoding::Auto,
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
                _ => return Err("既定の文字コードを選択してください。".to_owned()),
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
            set_control_text(self.controls.status, "設定を保存しました。");
            show_message(
                Some(parent),
                "設定を保存しました。次回の処理から反映されます。",
                MB_OK | MB_ICONINFORMATION,
            );
            Ok(())
        }

        fn browse_backend(&self, parent: HWND) -> Result<(), String> {
            let initial = control_text(self.controls.backend)?;
            if let Some(path) =
                choose_folder(parent, "backend-manifest.tsvを含むフォルダ", &initial)?
            {
                set_control_os_text(self.controls.backend, path.as_os_str());
                set_control_text(
                    self.controls.status,
                    "バックエンドの保存先を変更しました。保存前に診断できます。",
                );
            }
            Ok(())
        }

        fn doctor(&self, parent: HWND) -> Result<(), String> {
            let config = self.collect_config()?;
            let details = self.run_busy(
                parent,
                "バックエンドとAppContainerを診断しています...",
                || {
                    let backend_dir = config
                        .backend_directory()
                        .map_err(|error| error.to_string())?;
                    BackendBundle::verify(&backend_dir).map_err(|error| error.to_string())?;

                    let executable = sibling("safearc.exe")?;
                    let temporary_config = std::env::temp_dir()
                        .join(format!("safearc-doctor-{}.toml", util::unique_token()));
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
                    let output =
                        output_result.map_err(|error| format!("診断を開始できません: {error}"))?;
                    if !output.status.success() {
                        return Err(command_failure("診断", &output));
                    }
                    Ok(decoded_output(&output))
                },
            )?;
            set_control_text(self.controls.status, "診断に成功しました。");
            show_message(
                Some(parent),
                &format!("バックエンドとAppContainerの診断に成功しました。\n\n{details}"),
                MB_OK | MB_ICONINFORMATION,
            );
            Ok(())
        }

        fn import_backend(&self, parent: HWND, from_msys2: bool) -> Result<(), String> {
            let title = if from_msys2 {
                "MSYS2のルート（例: C:\\msys64）"
            } else {
                "bsdtar.exeと依存DLLを含むbundle"
            };
            let Some(source) = choose_folder(parent, title, "")? else {
                return Ok(());
            };
            let config = self.collect_config()?;
            let destination = config
                .backend_directory()
                .map_err(|error| error.to_string())?;
            if destination.exists()
                && !confirm_action(
                    parent,
                    "現在のバックエンドを検証済みの新しいbundleで置き換えます。続行しますか？",
                )
            {
                set_control_text(
                    self.controls.status,
                    "バックエンドの取り込みを中止しました。",
                );
                return Ok(());
            }

            let (script, source_argument) = if from_msys2 {
                ("export-msys2-backend.ps1", "-Msys2Root")
            } else {
                ("install-backend.ps1", "-SourceDirectory")
            };
            let bundle = self.run_busy(
                parent,
                "バックエンドを取り込み、検証しています...",
                || {
                    let output = run_script(
                        script,
                        &[
                            OsString::from(source_argument),
                            source.into_os_string(),
                            OsString::from("-DestinationDirectory"),
                            destination.as_os_str().to_owned(),
                        ],
                    )?;
                    if !output.status.success() {
                        return Err(command_failure("バックエンド取り込み", &output));
                    }
                    let bundle =
                        BackendBundle::verify(&destination).map_err(|error| error.to_string())?;
                    config
                        .save(&self.config_path)
                        .map_err(|error| error.to_string())?;
                    Ok(bundle)
                },
            )?;
            self.saved_config.replace(config);
            self.update_dirty_title(parent);
            set_control_text(
                self.controls.status,
                "バックエンドを取り込み、検証しました。",
            );
            show_message(
                Some(parent),
                &format!(
                    "バックエンドを取り込みました。\n\n保存先: {}\n実行ファイル: {}",
                    bundle.root().display(),
                    bundle.executable().display()
                ),
                MB_OK | MB_ICONINFORMATION,
            );
            Ok(())
        }

        fn association(&self, parent: HWND, register: bool) -> Result<(), String> {
            if !register
                && !confirm_action(
                    parent,
                    "現在のユーザーからSafeArcの関連付け候補を解除します。続行しますか？",
                )
            {
                set_control_text(self.controls.status, "関連付けの解除を中止しました。");
                return Ok(());
            }
            let progress = if register {
                "関連付け候補を登録しています..."
            } else {
                "関連付け候補を解除しています..."
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
                return Err(command_failure("関連付け", &output));
            }
            let message = if register {
                "SafeArcをアーカイブアプリ候補として登録しました。既定のアプリ画面で拡張子ごとの選択を確定してください。"
            } else {
                "SafeArcの関連付け候補を解除しました。Windowsの既定選択は必要に応じて変更してください。"
            };
            set_control_text(self.controls.status, message);
            show_message(Some(parent), message, MB_OK | MB_ICONINFORMATION);
            Ok(())
        }

        fn open_default_apps(&self) -> Result<(), String> {
            spawn_explorer(OsStr::new("ms-settings:defaultapps"))?;
            set_control_text(
                self.controls.status,
                "Windowsの既定のアプリ画面を開きました。",
            );
            Ok(())
        }

        fn open_config_folder(&self) -> Result<(), String> {
            let parent = self
                .config_path
                .parent()
                .ok_or_else(|| "設定ファイルに親フォルダがありません。".to_owned())?;
            fs::create_dir_all(parent)
                .map_err(|error| format!("設定フォルダを作成できません: {error}"))?;
            spawn_explorer(parent.as_os_str())?;
            set_control_text(self.controls.status, "設定フォルダを開きました。");
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
                &format!("{WINDOW_TITLE} — v{}{marker}", env!("CARGO_PKG_VERSION")),
            );
        }

        fn request_close(&self, parent: HWND) -> Result<(), String> {
            if self.has_unsaved_changes()
                && !confirm_action(parent, "保存していない変更を破棄して閉じますか？")
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
            match id {
                ID_BACKEND_BROWSE => self.browse_backend(parent),
                ID_BACKEND_DOCTOR => self.doctor(parent),
                ID_BACKEND_IMPORT => self.import_backend(parent, false),
                ID_BACKEND_MSYS2 => self.import_backend(parent, true),
                ID_REGISTER => self.association(parent, true),
                ID_UNREGISTER => self.association(parent, false),
                ID_DEFAULT_APPS => self.open_default_apps(),
                ID_CONFIG_FOLDER => self.open_config_folder(),
                ID_DEFAULTS => {
                    if !confirm_action(
                        parent,
                        "画面上のすべての設定を安全な既定値へ戻します。保存するまで反映されません。続行しますか？",
                    ) {
                        return Ok(());
                    }
                    self.apply_config(&Config::default());
                    set_control_text(
                        self.controls.status,
                        "既定値を表示しています。保存すると反映されます。",
                    );
                    Ok(())
                }
                ID_SAVE => self.save(parent),
                ID_CANCEL => self.request_close(parent),
                _ => Ok(()),
            }
        }
    }

    pub fn run() -> Result<(), String> {
        let config_path = parse_config_path()?;
        Config::load(&config_path).map_err(|error| error.to_string())?;

        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .ok()
                .map_err(|error| format!("COMを初期化できません: {error}"))?;
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
                "設定画面のwindow classを登録できません: {}",
                WindowsError::from_thread()
            ));
        }

        let app = Box::new(App::new(config_path));
        let app_pointer = Box::into_raw(app);
        let title = wide(&format!("{WINDOW_TITLE} — v{}", env!("CARGO_PKG_VERSION")));
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                950,
                720,
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
                return Err(format!("設定画面を作成できません: {error}"));
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
                    "window messageを取得できません: {}",
                    WindowsError::from_thread()
                ));
            }
            if result.0 == 0 {
                break;
            }
            if unsafe { IsDialogMessageW(window, &raw const message) }.as_bool() {
                continue;
            }
            unsafe {
                let _ = TranslateMessage(&raw const message);
                DispatchMessageW(&raw const message);
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
                            "処理に失敗しました。詳細はエラーダイアログを確認してください。",
                        );
                    }
                }
                if !app_pointer.is_null()
                    && (notification == EN_CHANGE as usize
                        || notification == CBN_SELCHANGE as usize
                        || (notification == BN_CLICKED as usize && id == 0))
                {
                    unsafe { (*app_pointer).update_dirty_title(window) };
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

    unsafe fn add_edit(
        parent: HWND,
        instance: HINSTANCE,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        numeric: bool,
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
                None,
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

    unsafe fn add_checkbox(
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
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                x,
                y,
                width,
                height,
                None,
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
                None,
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
        .map_err(|error| format!("controlを作成できません: {error}"))?;
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
            return Err("入力値の長さを取得できません。".to_owned());
        }
        let capacity = usize::try_from(length)
            .map_err(|_| "入力値が長すぎます。".to_owned())?
            .saturating_add(1);
        let mut buffer = vec![0u16; capacity];
        let copied = unsafe { GetWindowTextW(control, &mut buffer) };
        if copied < 0 {
            return Err("入力値を取得できません。".to_owned());
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
            unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }
                .map_err(|error| format!("フォルダ選択画面を作成できません: {error}"))?;
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
            Err(error) => return Err(format!("フォルダを選択できません: {error}")),
        }
        let item = unsafe { dialog.GetResult() }.map_err(|error| error.to_string())?;
        let display_name =
            unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }.map_err(|error| error.to_string())?;
        if display_name.is_null() {
            return Err("選択したフォルダのパスを取得できません。".to_owned());
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
            return Err("許可されていない管理scriptです。".to_owned());
        }
        let script = executable_directory()?.join("scripts").join(name);
        if !script.is_file() {
            return Err(format!("管理scriptが見つかりません: {}", script.display()));
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
            .map_err(|error| format!("PowerShellを開始できません: {error}"))
    }

    fn command_failure(operation: &str, output: &std::process::Output) -> String {
        let details = decoded_output(output);
        format!(
            "{operation}に失敗しました（終了コード: {}）。\n\n{details}",
            output.status.code().unwrap_or(-1)
        )
    }

    fn decoded_output(output: &std::process::Output) -> String {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}\n{stderr}");
        combined.trim().chars().take(8_000).collect()
    }

    fn executable_directory() -> Result<PathBuf, String> {
        let executable = std::env::current_exe()
            .map_err(|error| format!("実行ファイルの場所を取得できません: {error}"))?;
        executable
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| "実行ファイルに親フォルダがありません。".to_owned())
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
            .map_err(|error| format!("Explorerを開けません: {error}"))?;
        Ok(())
    }

    fn parse_config_path() -> Result<PathBuf, String> {
        let mut arguments = std::env::args_os().skip(1);
        match arguments.next() {
            None => default_config_path().map_err(|error| error.to_string()),
            Some(flag) if flag == "--config" => {
                let path = arguments
                    .next()
                    .map(PathBuf::from)
                    .ok_or_else(|| "--configには設定ファイルのパスが必要です。".to_owned())?;
                if arguments.next().is_some() {
                    return Err("設定アプリに不要な引数が指定されました。".to_owned());
                }
                Ok(path)
            }
            Some(_) => Err("使用方法: safearc-settings.exe [--config PATH]".to_owned()),
        }
    }

    fn show_error(owner: Option<HWND>, message: &str) {
        show_message(owner, message, MB_OK | MB_ICONERROR);
    }

    fn confirm_action(owner: HWND, message: &str) -> bool {
        let message = wide(message);
        let title = wide(WINDOW_TITLE);
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
        let title = wide(WINDOW_TITLE);
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
            let title: Vec<u16> = OsStr::new("SafeArc 設定")
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
    eprintln!("safearc-settings: the graphical settings screen is available on Windows");
    std::process::ExitCode::from(2)
}
