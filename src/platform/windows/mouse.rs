use std::mem::size_of;
use std::path::PathBuf;
use windows_sys::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, ReleaseDC, LOGPIXELSX};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_HWHEEL,
    MOUSEINPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
};
use crate::platform::{SystemController, MouseButton};

const KEYSOR_SIGNATURE: usize = 0xFA17CAFE;

pub struct WindowsSystemController;

impl WindowsSystemController {
    pub fn new() -> Self {
        WindowsSystemController
    }
}

/// 하드웨어 수준에서 마우스 가상 이벤트를 전송하는 헬퍼 함수
fn send_mouse_input(dx: i32, dy: i32, mouse_data: i32, flags: u32) {
    unsafe {
        let mut input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: std::mem::zeroed(),
        };

        let mi = MOUSEINPUT {
            dx,
            dy,
            mouseData: mouse_data as u32,
            dwFlags: flags,
            time: 0,
            dwExtraInfo: KEYSOR_SIGNATURE,
        };

        input.Anonymous.mi = mi;

        SendInput(1, &input, size_of::<INPUT>() as i32);
    }
}

#[allow(dead_code)]
fn get_startup_shortcut_path() -> PathBuf {
    let mut path = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push("Microsoft");
    path.push("Windows");
    path.push("Start Menu");
    path.push("Programs");
    path.push("Startup");
    path.push("keysor.lnk");
    path
}

impl SystemController for WindowsSystemController {
    fn get_cursor_pos(&self) -> (i32, i32) {
        unsafe {
            let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
            windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
            (pt.x, pt.y)
        }
    }

    fn set_cursor_pos(&self, x: i32, y: i32) -> bool {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(x, y) != 0
        }
    }

    fn move_relative(&self, dx: i32, dy: i32) {
        if dx != 0 || dy != 0 {
            let (cx, cy) = self.get_cursor_pos();
            self.set_cursor_pos(cx + dx, cy + dy);
        }
    }

    fn left_down(&self) {
        println!("[Debug] WindowsSystemController::left_down()");
        send_mouse_input(0, 0, 0, MOUSEEVENTF_LEFTDOWN);
    }

    fn left_up(&self) {
        println!("[Debug] WindowsSystemController::left_up()");
        send_mouse_input(0, 0, 0, MOUSEEVENTF_LEFTUP);
    }

    fn left_click(&self) {
        println!("[Debug] WindowsSystemController::left_click()");
        self.left_down();
        std::thread::sleep(std::time::Duration::from_millis(10));
        self.left_up();
    }

    fn left_double_click(&self) {
        println!("[Debug] WindowsSystemController::left_double_click()");
        self.left_click();
        std::thread::sleep(std::time::Duration::from_millis(80));
        self.left_click();
    }

    fn right_down(&self) {
        println!("[Debug] WindowsSystemController::right_down()");
        send_mouse_input(0, 0, 0, MOUSEEVENTF_RIGHTDOWN);
    }

    fn right_up(&self) {
        println!("[Debug] WindowsSystemController::right_up()");
        send_mouse_input(0, 0, 0, MOUSEEVENTF_RIGHTUP);
    }

    fn right_click(&self) {
        println!("[Debug] WindowsSystemController::right_click()");
        self.right_down();
        std::thread::sleep(std::time::Duration::from_millis(10));
        self.right_up();
    }

    fn send_click(&self, button: MouseButton, press: bool) {
        match (button, press) {
            (MouseButton::Left, true) => self.left_down(),
            (MouseButton::Left, false) => self.left_up(),
            (MouseButton::Right, true) => self.right_down(),
            (MouseButton::Right, false) => self.right_up(),
        }
    }

    fn scroll(&self, amount: i32) {
        send_mouse_input(0, 0, amount, MOUSEEVENTF_WHEEL);
    }

    fn scroll_horizontal(&self, amount: i32) {
        send_mouse_input(0, 0, amount, MOUSEEVENTF_HWHEEL);
    }

    fn get_dpi_scale(&self) -> f64 {
        unsafe {
            let hdc = GetDC(0);
            if hdc != 0 {
                let dpi = GetDeviceCaps(hdc, LOGPIXELSX as i32);
                ReleaseDC(0, hdc);
                if dpi > 0 {
                    return dpi as f64 / 96.0;
                }
            }
            1.0
        }
    }

    fn register_startup(&self, active: bool) -> Result<(), String> {
        let shortcut_path = get_startup_shortcut_path();
        if active {
            if let Ok(exe_path) = std::env::current_exe() {
                let exe_path_str = exe_path.to_string_lossy();
                let exe_dir = exe_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let shortcut_path_str = shortcut_path.to_string_lossy();
                
                let script = format!(
                    "$WshShell = New-Object -ComObject WScript.Shell; \
                     $Shortcut = $WshShell.CreateShortcut('{}'); \
                     $Shortcut.TargetPath = '{}'; \
                     $Shortcut.WorkingDirectory = '{}'; \
                     $Shortcut.Save()",
                    shortcut_path_str, exe_path_str, exe_dir
                );
                
                let output = std::process::Command::new("powershell")
                    .args(&["-NoProfile", "-Command", &script])
                    .output();
                    
                match output {
                    Ok(out) if out.status.success() => Ok(()),
                    Ok(out) => Err(String::from_utf8_lossy(&out.stderr).into_owned()),
                    Err(e) => Err(e.to_string()),
                }
            } else {
                Err("Cannot retrieve current executable path".to_string())
            }
        } else {
            if shortcut_path.exists() {
                std::fs::remove_file(&shortcut_path).map_err(|e| e.to_string())
            } else {
                Ok(())
            }
        }
    }

    fn simulate_browser_navigation(&self, forward: bool) {
        unsafe {
            let vk = if forward { 0xA7 } else { 0xA6 }; // VK_BROWSER_FORWARD / VK_BROWSER_BACK
            let mut inputs = [
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: std::mem::zeroed(),
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: std::mem::zeroed(),
                },
            ];

            inputs[0].Anonymous.ki = KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: KEYSOR_SIGNATURE,
            };
            inputs[1].Anonymous.ki = KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: KEYSOR_SIGNATURE,
            };

            SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn simulate_virtual_desktop_navigation(&self, forward: bool) {
        let lshift_down = unsafe { (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0xA0) as u32 & 0x8000) != 0 };
        let rshift_down = unsafe { (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0xA1) as u32 & 0x8000) != 0 };

        std::thread::spawn(move || {
            unsafe {
                let arrow_vk = if forward { 0x27 } else { 0x25 }; // VK_RIGHT / VK_LEFT
                let arrow_scan = if forward { 0x4D } else { 0x4B };
                println!(
                    "[Debug] simulate_virtual_desktop_navigation thread: forward={}, lshift={}, rshift={}",
                    forward, lshift_down, rshift_down
                );

                const KEYEVENTF_EXTENDEDKEY: u32 = 0x0001;

                // 1. Release Shift keys that are down
                let mut release_inputs = Vec::new();
                if lshift_down {
                    let mut input = INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() };
                    input.Anonymous.ki = KEYBDINPUT {
                        wVk: 0xA0,
                        wScan: 0x2A,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: KEYSOR_SIGNATURE,
                    };
                    release_inputs.push(input);
                }
                if rshift_down {
                    let mut input = INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() };
                    input.Anonymous.ki = KEYBDINPUT {
                        wVk: 0xA1,
                        wScan: 0x36,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: KEYSOR_SIGNATURE,
                    };
                    release_inputs.push(input);
                }

                if !release_inputs.is_empty() {
                    SendInput(release_inputs.len() as u32, release_inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
                    std::thread::sleep(std::time::Duration::from_millis(15));
                }

                // 2. Send Ctrl + Win + Arrow
                let mut navigate = [
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Ctrl down
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Win down
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Arrow down
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Arrow up
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Win up
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Ctrl up
                ];

                // Ctrl down (0x11, scan 0x1D)
                navigate[0].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x11,
                    wScan: 0x1D,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                // Win down (0x5B, scan 0x5B)
                navigate[1].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x5B,
                    wScan: 0x5B,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                // Arrow down
                navigate[2].Anonymous.ki = KEYBDINPUT {
                    wVk: arrow_vk,
                    wScan: arrow_scan,
                    dwFlags: KEYEVENTF_EXTENDEDKEY,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                // Arrow up
                navigate[3].Anonymous.ki = KEYBDINPUT {
                    wVk: arrow_vk,
                    wScan: arrow_scan,
                    dwFlags: KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                // Win up
                navigate[4].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x5B,
                    wScan: 0x5B,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                // Ctrl up
                navigate[5].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x11,
                    wScan: 0x1D,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                SendInput(6, navigate.as_ptr(), std::mem::size_of::<INPUT>() as i32);

                // Wait 150ms for Windows to complete the desktop transition
                std::thread::sleep(std::time::Duration::from_millis(150));

                // 3. Clean up and force release Shift keys if they are NOT physically pressed anymore
                let lshift_still_down = (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0xA0) as u32 & 0x8000) != 0;
                let rshift_still_down = (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0xA1) as u32 & 0x8000) != 0;

                let mut cleanup_inputs = Vec::new();
                if !lshift_still_down {
                    let mut input = INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() };
                    input.Anonymous.ki = KEYBDINPUT {
                        wVk: 0xA0,
                        wScan: 0x2A,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: KEYSOR_SIGNATURE,
                    };
                    cleanup_inputs.push(input);
                }
                if !rshift_still_down {
                    let mut input = INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() };
                    input.Anonymous.ki = KEYBDINPUT {
                        wVk: 0xA1,
                        wScan: 0x36,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: KEYSOR_SIGNATURE,
                    };
                    cleanup_inputs.push(input);
                }

                if !cleanup_inputs.is_empty() {
                    SendInput(cleanup_inputs.len() as u32, cleanup_inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
                }
            }
        });
    }

    fn simulate_page_jump(&self, top: bool) {
        let amount = if top { 10000 } else { -10000 };
        self.scroll(amount);
    }

    fn simulate_tab_navigation(&self, forward: bool) {
        unsafe {
            if forward {
                // Ctrl + Tab
                let mut inputs = [
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Ctrl down
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Tab down
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Tab up
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Ctrl up
                ];

                inputs[0].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x11,
                    wScan: 0x1D,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                inputs[1].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x09,
                    wScan: 0x0F,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                inputs[2].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x09,
                    wScan: 0x0F,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                inputs[3].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x11,
                    wScan: 0x1D,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };

                SendInput(4, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
            } else {
                // Ctrl + Shift + Tab
                let mut inputs = [
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Ctrl down
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Shift down
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Tab down
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Tab up
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Shift up
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() }, // Ctrl up
                ];

                inputs[0].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x11,
                    wScan: 0x1D,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                inputs[1].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x10,
                    wScan: 0x2A,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                inputs[2].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x09,
                    wScan: 0x0F,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                inputs[3].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x09,
                    wScan: 0x0F,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                inputs[4].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x10,
                    wScan: 0x2A,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                inputs[5].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x11,
                    wScan: 0x1D,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };

                SendInput(6, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
            }
        }
    }

    fn run_app(&self, app_path: &str) -> Result<(), String> {
        std::process::Command::new("cmd")
            .args(&["/C", "start", "", app_path])
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn ensure_caps_lock_off(&self) {
        unsafe {
            let state = windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x14); // VK_CAPITAL = 0x14
            if (state & 0x0001) != 0 {
                let mut inputs = [
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() },
                    INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() },
                ];

                inputs[0].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x14,
                    wScan: 0x3A,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };
                inputs[1].Anonymous.ki = KEYBDINPUT {
                    wVk: 0x14,
                    wScan: 0x3A,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: KEYSOR_SIGNATURE,
                };

                SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
            }
        }
    }

    fn inject_caps_lock_toggle(&self) {
        unsafe {
            let mut inputs = [
                INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() },
                INPUT { r#type: INPUT_KEYBOARD, Anonymous: std::mem::zeroed() },
            ];

            inputs[0].Anonymous.ki = KEYBDINPUT {
                wVk: 0x14, // VK_CAPITAL
                wScan: 0x3A, // Caps Lock scan code
                dwFlags: 0,
                time: 0,
                dwExtraInfo: KEYSOR_SIGNATURE,
            };
            inputs[1].Anonymous.ki = KEYBDINPUT {
                wVk: 0x14, // VK_CAPITAL
                wScan: 0x3A,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: KEYSOR_SIGNATURE,
            };

            SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
        }
    }



    fn beep(&self) {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MessageBeep(utype: u32) -> i32;
        }
        unsafe {
            MessageBeep(0x00000030);
        }
    }
}
