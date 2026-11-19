//! Windows 自定义窗口边缘：圆角、边框与 Snap Layouts 命中区。
//!
//! `decorations: false` 后 WebView2 子窗口会先接收鼠标命中测试，因此不能只在
//! Tauri 主窗口的 `WM_NCHITTEST` 中返回 `HTMAXBUTTON`。这里在最大化按钮位置
//! 创建一个不可见的原生子窗口，由它返回 `HTMAXBUTTON`，让 Windows 11 恢复
//! Snap Layouts 悬停菜单；点击、悬停通过 Tauri 事件回传前端。
//!
//! 该子窗口只覆盖最大化按钮，不承担任何业务逻辑；窗口控制仍由前端调用
//! Tauri 的最小化 / 最大化 / 关闭命令完成。非 Windows 平台为空实现。

#[cfg(windows)]
mod platform {
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex, OnceLock};

    use tauri::{Emitter, WebviewWindow};
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE,
            DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        },
        Graphics::Gdi::{GetStockObject, HBRUSH, NULL_BRUSH},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::GetDpiForWindow,
            Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT},
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, RegisterClassExW,
                SetForegroundWindow, SetWindowPos, CS_HREDRAW, CS_VREDRAW, HTMAXBUTTON, HWND_TOP,
                SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_SHOWWINDOW, WM_CLOSE, WM_DPICHANGED,
                WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE,
                WM_SIZE, WNDCLASSEXW, WS_CHILD, WS_CLIPSIBLINGS, WS_OVERLAPPED, WS_VISIBLE,
            },
        },
    };

    use crate::window_chrome::{BUTTON_WIDTH, TITLEBAR_HEIGHT};

    pub const EVENT_ENTER: &str = "xushu://chrome/snap-enter";
    pub const EVENT_LEAVE: &str = "xushu://chrome/snap-leave";

    const SUBCLASS_ID: usize = 0x7875_7368_7563_6872;
    const RIGHT_INDEX: u32 = 1;

    static CLASS_NAME: OnceLock<Vec<u16>> = OnceLock::new();
    static OVERLAYS: LazyLock<Mutex<HashMap<isize, OverlayState>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    struct OverlayState {
        overlay: HWND,
        window: tauri::Window,
        hovering: bool,
    }

    // HWND 是主线程窗口句柄；状态只在主线程窗口过程与主线程安装路径访问。
    unsafe impl Send for OverlayState {}

    pub fn install(window: &WebviewWindow) -> tauri::Result<()> {
        let hwnd = window.hwnd()?.0 as isize;
        let tauri_window = window.as_ref().window();

        window.run_on_main_thread(move || unsafe {
            install_hwnd(hwnd, tauri_window);
        })?;

        Ok(())
    }

    fn class_name() -> &'static [u16] {
        CLASS_NAME
            .get_or_init(|| "XushuSnapOverlay".encode_utf16().chain([0]).collect())
            .as_slice()
    }

    unsafe fn register_class() {
        let name = class_name();
        let mut class: WNDCLASSEXW = std::mem::zeroed();
        class.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        class.style = CS_HREDRAW | CS_VREDRAW;
        class.lpfnWndProc = Some(overlay_proc);
        class.hInstance = GetModuleHandleW(std::ptr::null());
        class.hbrBackground = GetStockObject(NULL_BRUSH) as HBRUSH;
        class.lpszClassName = name.as_ptr();

        RegisterClassExW(&class);
    }

    unsafe fn install_hwnd(hwnd: isize, window: tauri::Window) {
        register_class();

        let name = class_name();
        let parent = hwnd as HWND;
        let overlay = CreateWindowExW(
            0,
            name.as_ptr(),
            name.as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            parent,
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        );

        if overlay.is_null() {
            return;
        }

        {
            let mut overlays = OVERLAYS.lock().expect("Snap overlay state poisoned");
            if let Some(previous) = overlays.remove(&hwnd) {
                RemoveWindowSubclass(parent, Some(parent_subclass_proc), SUBCLASS_ID);
                DestroyWindow(previous.overlay);
            }

            overlays.insert(
                hwnd,
                OverlayState {
                    overlay,
                    window,
                    hovering: false,
                },
            );
        }

        apply_window_frame(parent);
        SetWindowSubclass(parent, Some(parent_subclass_proc), SUBCLASS_ID, 0);
        update_overlay_position(parent);
    }

    unsafe fn apply_window_frame(hwnd: HWND) {
        let corner = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &corner as *const _ as *const _,
            std::mem::size_of_val(&corner) as u32,
        );

        let border = DWMWA_COLOR_NONE;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR as u32,
            &border as *const _ as *const _,
            std::mem::size_of_val(&border) as u32,
        );
    }

    unsafe fn update_overlay_position(parent: HWND) {
        let mut rect: RECT = std::mem::zeroed();
        if GetClientRect(parent, &mut rect) == 0 {
            return;
        }

        let dpi = GetDpiForWindow(parent);
        let button_width = scaled(BUTTON_WIDTH, dpi).max(1);
        let titlebar_height = scaled(TITLEBAR_HEIGHT, dpi).max(1);
        let x = rect.right - button_width * (RIGHT_INDEX as i32 + 1);

        let overlays = OVERLAYS.lock().expect("Snap overlay state poisoned");
        if let Some(state) = overlays.get(&(parent as isize)) {
            SetWindowPos(
                state.overlay,
                HWND_TOP,
                x,
                0,
                button_width,
                titlebar_height,
                SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    fn scaled(value: u32, dpi: u32) -> i32 {
        ((u64::from(value) * u64::from(dpi) + 48) / 96) as i32
    }

    unsafe fn remove(parent: HWND) {
        RemoveWindowSubclass(parent, Some(parent_subclass_proc), SUBCLASS_ID);
        if let Some(state) = OVERLAYS
            .lock()
            .expect("Snap overlay state poisoned")
            .remove(&(parent as isize))
        {
            DestroyWindow(state.overlay);
        }
    }

    unsafe extern "system" fn parent_subclass_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _subclass_id: usize,
        _ref_data: usize,
    ) -> LRESULT {
        match msg {
            WM_SIZE | WM_DPICHANGED => update_overlay_position(hwnd),
            WM_CLOSE => remove(hwnd),
            _ => {}
        }

        DefSubclassProc(hwnd, msg, wparam, lparam)
    }

    unsafe extern "system" fn overlay_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_NCHITTEST => return HTMAXBUTTON as LRESULT,
            WM_NCMOUSEMOVE => {
                if let Some(parent) = parent_for_overlay(hwnd) {
                    let window = {
                        let mut overlays = OVERLAYS.lock().expect("Snap overlay state poisoned");
                        overlays.get_mut(&(parent as isize)).and_then(|state| {
                            if state.hovering {
                                None
                            } else {
                                state.hovering = true;
                                Some(state.window.clone())
                            }
                        })
                    };

                    if let Some(window) = window {
                        let _ = window.emit(EVENT_ENTER, ());
                    }

                    let mut track = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE | TME_NONCLIENT,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    TrackMouseEvent(&mut track);
                }

                return 0;
            }
            WM_NCMOUSELEAVE => {
                if let Some(parent) = parent_for_overlay(hwnd) {
                    let window = {
                        let mut overlays = OVERLAYS.lock().expect("Snap overlay state poisoned");
                        overlays.get_mut(&(parent as isize)).map(|state| {
                            state.hovering = false;
                            state.window.clone()
                        })
                    };

                    if let Some(window) = window {
                        let _ = window.emit(EVENT_LEAVE, ());
                    }
                }

                return 0;
            }
            WM_NCLBUTTONDOWN => {
                if let Some(parent) = parent_for_overlay(hwnd) {
                    let window = OVERLAYS
                        .lock()
                        .expect("Snap overlay state poisoned")
                        .get(&(parent as isize))
                        .map(|state| state.window.clone());

                    SetForegroundWindow(parent);

                    if let Some(window) = window {
                        let maximized = window.is_maximized().unwrap_or(false);
                        let _ = if maximized {
                            window.unmaximize()
                        } else {
                            window.maximize()
                        };
                    }
                }

                return 0;
            }
            WM_NCLBUTTONUP => return 0,
            _ => {}
        }

        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    unsafe fn parent_for_overlay(overlay: HWND) -> Option<HWND> {
        OVERLAYS
            .lock()
            .expect("Snap overlay state poisoned")
            .iter()
            .find_map(|(parent, state)| (state.overlay == overlay).then_some(*parent as HWND))
    }

    #[cfg(test)]
    mod tests {
        use super::scaled;

        #[test]
        fn scales_logical_pixels_to_window_dpi() {
            assert_eq!(scaled(46, 96), 46);
            assert_eq!(scaled(46, 144), 69);
            assert_eq!(scaled(44, 192), 88);
        }
    }
}

#[cfg(windows)]
pub use platform::install;

#[cfg(windows)]
const TITLEBAR_HEIGHT: u32 = 44;
#[cfg(windows)]
const BUTTON_WIDTH: u32 = 46;

#[cfg(not(windows))]
pub fn install(_window: &tauri::WebviewWindow) -> tauri::Result<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn macos_titlebar_double_click(window: tauri::WebviewWindow) -> Result<(), String> {
    use objc2_app_kit::{NSWindow, NSWindowStyleMask};
    use objc2_foundation::{ns_string, NSRect, NSUserDefaults};
    use std::sync::Mutex;

    static FRAME_BEFORE_FILL: Mutex<Option<NSRect>> = Mutex::new(None);

    if window.label() != "main" {
        return Err("窗口操作仅适用于主窗口".to_owned());
    }
    let target = window.clone();
    window
        .run_on_main_thread(move || {
            let Ok(handle) = target.ns_window() else {
                return;
            };
            // Tauri owns this NSWindow and the cloned handle lives through the callback.
            let native = unsafe { handle.cast::<NSWindow>().as_ref() };
            let Some(native) = native else {
                return;
            };
            if native.styleMask().contains(NSWindowStyleMask::FullScreen) {
                return;
            }
            let settings = NSUserDefaults::standardUserDefaults();
            let preference = settings
                .stringForKey(ns_string!("AppleActionOnDoubleClick"))
                .map(|value| value.to_string());
            match preference.as_deref() {
                Some("Minimize") => native.performMiniaturize(None),
                Some("Fill") => {
                    if let (Some(screen), Ok(mut previous)) =
                        (native.screen(), FRAME_BEFORE_FILL.lock())
                    {
                        let available = screen.visibleFrame();
                        if native.frame() == available {
                            if let Some(frame) = previous.take() {
                                let frame =
                                    native.constrainFrameRect_toScreen(frame, Some(&screen));
                                native.setFrame_display_animate(frame, true, true);
                            } else {
                                native.performZoom(None);
                            }
                        } else {
                            *previous = Some(native.frame());
                            native.setFrame_display_animate(available, true, true);
                        }
                    }
                }
                Some("Maximize" | "Zoom") => native.performZoom(None),
                None if settings.boolForKey(ns_string!("AppleMiniaturizeOnDoubleClick")) => {
                    native.performMiniaturize(None);
                }
                None => native.performZoom(None),
                // Includes the system's explicit "None" preference.
                Some(_) => {}
            }
        })
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod config_tests {
    use serde_json::{json, Value};

    #[test]
    fn the_macos_override_preserves_the_shared_window_configuration() {
        let base: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let macos: Value = serde_json::from_str(include_str!("../tauri.macos.conf.json")).unwrap();
        let mut expected = base["app"]["windows"].clone();
        assert_eq!(expected[0]["decorations"], false);
        expected[0]["decorations"] = json!(true);
        expected[0]["titleBarStyle"] = json!("Overlay");
        expected[0]["hiddenTitle"] = json!(true);
        expected[0]["trafficLightPosition"] = json!({ "x": 14, "y": 15 });
        // Tauri replaces the array as a unit, so the override carries the complete window.
        assert_eq!(macos["app"]["windows"], expected);
    }
}
