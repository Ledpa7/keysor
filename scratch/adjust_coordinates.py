import os

def main():
    fpath = 'src/indicator.rs'
    if not os.path.exists(fpath):
        print(f"Error: {fpath} not found")
        return

    content = open(fpath, 'r', encoding='utf-8').read()
    # Normalize to LF
    content = content.replace('\r\n', '\n')

    # 1. Update Lang button, Min button, Close button in WM_PAINT
    # RoundRect(hdc, 630, 15, 680, 35, 6, 6);
    # let mut r_lang = RECT { left: 630, top: 15, right: 680, bottom: 35 };
    content = content.replace('RoundRect(hdc, 630, 15, 680, 35, 6, 6);', 'RoundRect(hdc, 615, 15, 665, 35, 6, 6);')
    content = content.replace('let mut r_lang = RECT { left: 630, top: 15, right: 680, bottom: 35 };', 'let mut r_lang = RECT { left: 615, top: 15, right: 665, bottom: 35 };')

    # RoundRect(hdc, 690, 15, 715, 35, 6, 6);
    # let mut r_min = RECT { left: 690, top: 11, right: 715, bottom: 31 };
    content = content.replace('RoundRect(hdc, 690, 15, 715, 35, 6, 6);', 'RoundRect(hdc, 675, 15, 700, 35, 6, 6);')
    content = content.replace('let mut r_min = RECT { left: 690, top: 11, right: 715, bottom: 31 };', 'let mut r_min = RECT { left: 675, top: 11, right: 700, bottom: 31 };')

    # RoundRect(hdc, 720, 15, 745, 35, 6, 6);
    # let mut r_close = RECT { left: 720, top: 16, right: 745, bottom: 34 };
    content = content.replace('RoundRect(hdc, 720, 15, 745, 35, 6, 6);', 'RoundRect(hdc, 705, 15, 730, 35, 6, 6);')
    content = content.replace('let mut r_close = RECT { left: 720, top: 16, right: 745, bottom: 34 };', 'let mut r_close = RECT { left: 705, top: 16, right: 730, bottom: 34 };')

    # 2. Update dec, inc buttons in WM_PAINT
    # RoundRect(hdc, 610, 145, 650, 185, 8, 8);
    # let mut r_dec = RECT { left: 610, top: 135, right: 650, bottom: 180 };
    content = content.replace('RoundRect(hdc, 610, 145, 650, 185, 8, 8);', 'RoundRect(hdc, 610, 138, 650, 178, 8, 8);')
    content = content.replace('let mut r_dec = RECT { left: 610, top: 135, right: 650, bottom: 180 };', 'let mut r_dec = RECT { left: 610, top: 128, right: 650, bottom: 173 };')

    # RoundRect(hdc, 670, 145, 710, 185, 8, 8);
    # let mut r_inc = RECT { left: 670, top: 150, right: 710, bottom: 185 };
    content = content.replace('RoundRect(hdc, 670, 145, 710, 185, 8, 8);', 'RoundRect(hdc, 670, 138, 710, 178, 8, 8);')
    content = content.replace('let mut r_inc = RECT { left: 670, top: 150, right: 710, bottom: 185 };', 'let mut r_inc = RECT { left: 670, top: 143, right: 710, bottom: 178 };')

    # 3. Update Pixel, Magnet, Detail, Pro buttons in WM_PAINT
    # RoundRect(hdc, 610, 195, 710, 223, 6, 6);
    # let mut r_pm = RECT { left: 610, top: 195, right: 710, bottom: 223 };
    content = content.replace('RoundRect(hdc, 610, 195, 710, 223, 6, 6);', 'RoundRect(hdc, 610, 186, 710, 214, 6, 6);')
    content = content.replace('let mut r_pm = RECT { left: 610, top: 195, right: 710, bottom: 223 };', 'let mut r_pm = RECT { left: 610, top: 186, right: 710, bottom: 214 };')

    # RoundRect(hdc, 610, 230, 710, 258, 6, 6);
    # let mut r_mag = RECT { left: 610, top: 230, right: 710, bottom: 258 };
    content = content.replace('RoundRect(hdc, 610, 230, 710, 258, 6, 6);', 'RoundRect(hdc, 610, 221, 710, 249, 6, 6);')
    content = content.replace('let mut r_mag = RECT { left: 610, top: 230, right: 710, bottom: 258 };', 'let mut r_mag = RECT { left: 610, top: 221, right: 710, bottom: 249 };')

    # RoundRect(hdc, 610, 265, 710, 293, 6, 6);
    # let mut r_si = RECT { left: 610, top: 265, right: 710, bottom: 293 };
    content = content.replace('RoundRect(hdc, 610, 265, 710, 293, 6, 6);', 'RoundRect(hdc, 610, 256, 710, 284, 6, 6);')
    content = content.replace('let mut r_si = RECT { left: 610, top: 265, right: 710, bottom: 293 };', 'let mut r_si = RECT { left: 610, top: 256, right: 710, bottom: 284 };')

    # RoundRect(hdc, 610, 287, 710, 315, 6, 6);
    # let mut r_btn = RECT { left: 610, top: 287, right: 710, bottom: 315 };
    content = content.replace('RoundRect(hdc, 610, 287, 710, 315, 6, 6);', 'RoundRect(hdc, 610, 291, 710, 319, 6, 6);')
    content = content.replace('let mut r_btn = RECT { left: 610, top: 287, right: 710, bottom: 315 };', 'let mut r_btn = RECT { left: 610, top: 291, right: 710, bottom: 319 };')

    # 4. Update WM_MOUSEMOVE coordinates
    # Old:
    #                     if x >= 690 && x <= 715 && y >= 15 && y <= 35 {
    #                         new_hover = 1;
    #                     } else if x >= 720 && x <= 745 && y >= 15 && y <= 35 {
    #                         new_hover = 2;
    #                     } else if x >= 610 && x <= 650 && y >= 139 && y <= 177 {
    #                         new_hover = 3;
    #                     } else if x >= 670 && x <= 710 && y >= 139 && y <= 177 {
    #                         new_hover = 4;
    #                     } else if x >= 610 && x <= 710 && y >= 185 && y <= 211 {
    #                         new_hover = 5;
    #                     } else if x >= 630 && x <= 680 && y >= 15 && y <= 35 {
    #                         new_hover = 6;
    #                     } else if x >= 610 && x <= 710 && y >= 219 && y <= 245 {
    #                         new_hover = 7;
    #                     } else if x >= 610 && x <= 710 && y >= 253 && y <= 279 {
    #                         new_hover = 8;
    #                     } else if x >= 610 && x <= 710 && y >= 287 && y <= 315 {
    #                         new_hover = 9;
    
    old_mousemove = """                    if x >= 690 && x <= 715 && y >= 15 && y <= 35 {
                        new_hover = 1;
                    } else if x >= 720 && x <= 745 && y >= 15 && y <= 35 {
                        new_hover = 2;
                    } else if x >= 610 && x <= 650 && y >= 139 && y <= 177 {
                        new_hover = 3;
                    } else if x >= 670 && x <= 710 && y >= 139 && y <= 177 {
                        new_hover = 4;
                    } else if x >= 610 && x <= 710 && y >= 185 && y <= 211 {
                        new_hover = 5;
                    } else if x >= 630 && x <= 680 && y >= 15 && y <= 35 {
                        new_hover = 6;
                    } else if x >= 610 && x <= 710 && y >= 219 && y <= 245 {
                        new_hover = 7;
                    } else if x >= 610 && x <= 710 && y >= 253 && y <= 279 {
                        new_hover = 8;
                    } else if x >= 610 && x <= 710 && y >= 287 && y <= 315 {
                        new_hover = 9;"""

    new_mousemove = """                    if x >= 675 && x <= 700 && y >= 15 && y <= 35 {
                        new_hover = 1;
                    } else if x >= 705 && x <= 730 && y >= 15 && y <= 35 {
                        new_hover = 2;
                    } else if x >= 610 && x <= 650 && y >= 138 && y <= 178 {
                        new_hover = 3;
                    } else if x >= 670 && x <= 710 && y >= 138 && y <= 178 {
                        new_hover = 4;
                    } else if x >= 610 && x <= 710 && y >= 186 && y <= 214 {
                        new_hover = 5;
                    } else if x >= 615 && x <= 665 && y >= 15 && y <= 35 {
                        new_hover = 6;
                    } else if x >= 610 && x <= 710 && y >= 221 && y <= 249 {
                        new_hover = 7;
                    } else if x >= 610 && x <= 710 && y >= 256 && y <= 284 {
                        new_hover = 8;
                    } else if x >= 610 && x <= 710 && y >= 291 && y <= 319 {
                        new_hover = 9;"""
    
    content = content.replace(old_mousemove, new_mousemove)

    # 5. Update WM_LBUTTONDOWN coordinates
    # Old:
    #                 if x >= 690 && x <= 715 && y >= 15 && y <= 35 {
    #                     if let Some(&main_hwnd) = MAIN_HWND.get() {
    #                         ShowWindow(main_hwnd, SW_MINIMIZE);
    #                     }
    #                 } else if x >= 720 && x <= 745 && y >= 15 && y <= 35 {
    #                     if let Some(&main_hwnd) = MAIN_HWND.get() {
    #                         windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
    #                             main_hwnd,
    #                             windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE,
    #                             0,
    #                             0,
    #                         );
    #                     }
    #                 } else if x >= 610 && x <= 650 && y >= 139 && y <= 177 {
    #                     adjust_sensitivity(-0.1);
    #                     InvalidateRect(hwnd, std::ptr::null(), 1);
    #                 } else if x >= 670 && x <= 710 && y >= 139 && y <= 177 {
    #                     adjust_sensitivity(0.1);
    #                     InvalidateRect(hwnd, std::ptr::null(), 1);
    #                 } else if x >= 610 && x <= 710 && y >= 185 && y <= 211 {
    #                     toggle_pixel_mode();
    #                     InvalidateRect(hwnd, std::ptr::null(), 1);
    #                 } else if x >= 630 && x <= 680 && y >= 15 && y <= 35 {
    #                     toggle_language();
    #                     InvalidateRect(hwnd, std::ptr::null(), 1);
    #                 } else if x >= 610 && x <= 710 && y >= 219 && y <= 245 {
    #                     toggle_magnet();
    #                     InvalidateRect(hwnd, std::ptr::null(), 1);
    #                 } else if x >= 610 && x <= 710 && y >= 253 && y <= 279 {
    #                     SHOW_ALL_SENS.store(true, Ordering::SeqCst);
    #                     InvalidateRect(hwnd, std::ptr::null(), 1);
    #                     windows_sys::Win32::UI::Input::KeyboardAndMouse::SetCapture(hwnd);
    #                 } else if x >= 610 && x <= 710 && y >= 287 && y <= 315 {

    old_lbuttondown = """                if x >= 690 && x <= 715 && y >= 15 && y <= 35 {
                    if let Some(&main_hwnd) = MAIN_HWND.get() {
                        ShowWindow(main_hwnd, SW_MINIMIZE);
                    }
                } else if x >= 720 && x <= 745 && y >= 15 && y <= 35 {
                    if let Some(&main_hwnd) = MAIN_HWND.get() {
                        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
                            main_hwnd,
                            windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                            0,
                            0,
                        );
                    }
                } else if x >= 610 && x <= 650 && y >= 139 && y <= 177 {
                    adjust_sensitivity(-0.1);
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                } else if x >= 670 && x <= 710 && y >= 139 && y <= 177 {
                    adjust_sensitivity(0.1);
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                } else if x >= 610 && x <= 710 && y >= 185 && y <= 211 {
                    toggle_pixel_mode();
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                } else if x >= 630 && x <= 680 && y >= 15 && y <= 35 {
                    toggle_language();
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                } else if x >= 610 && x <= 710 && y >= 219 && y <= 245 {
                    toggle_magnet();
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                } else if x >= 610 && x <= 710 && y >= 253 && y <= 279 {
                    SHOW_ALL_SENS.store(true, Ordering::SeqCst);
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::SetCapture(hwnd);
                } else if x >= 610 && x <= 710 && y >= 287 && y <= 315 {"""

    new_lbuttondown = """                if x >= 675 && x <= 700 && y >= 15 && y <= 35 {
                    if let Some(&main_hwnd) = MAIN_HWND.get() {
                        ShowWindow(main_hwnd, SW_MINIMIZE);
                    }
                } else if x >= 705 && x <= 730 && y >= 15 && y <= 35 {
                    if let Some(&main_hwnd) = MAIN_HWND.get() {
                        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
                            main_hwnd,
                            windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                            0,
                            0,
                        );
                    }
                } else if x >= 610 && x <= 650 && y >= 138 && y <= 178 {
                    adjust_sensitivity(-0.1);
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                } else if x >= 670 && x <= 710 && y >= 138 && y <= 178 {
                    adjust_sensitivity(0.1);
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                } else if x >= 610 && x <= 710 && y >= 186 && y <= 214 {
                    toggle_pixel_mode();
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                } else if x >= 615 && x <= 665 && y >= 15 && y <= 35 {
                    toggle_language();
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                } else if x >= 610 && x <= 710 && y >= 221 && y <= 249 {
                    toggle_magnet();
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                } else if x >= 610 && x <= 710 && y >= 256 && y <= 284 {
                    SHOW_ALL_SENS.store(true, Ordering::SeqCst);
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::SetCapture(hwnd);
                } else if x >= 610 && x <= 710 && y >= 291 && y <= 319 {"""

    content = content.replace(old_lbuttondown, new_lbuttondown)

    # 6. Update magnetic snapping targets
    # Old:
    #             let targets = [
    #                 (1, 655, 25),
    #                 (2, 702, 25),
    #                 (3, 732, 25),
    #                 (4, 630, 158),
    #                 (5, 690, 158),
    #                 (6, 660, 198),
    #                 (7, 660, 232),
    #                 (8, 660, 266),
    #             ];
    
    old_targets = """            let targets = [
                (1, 655, 25),
                (2, 702, 25),
                (3, 732, 25),
                (4, 630, 158),
                (5, 690, 158),
                (6, 660, 198),
                (7, 660, 232),
                (8, 660, 266),
            ];"""

    new_targets = """            let targets = [
                (1, 640, 25),
                (2, 688, 25),
                (3, 718, 25),
                (4, 630, 158),
                (5, 690, 158),
                (6, 660, 200),
                (7, 660, 235),
                (8, 660, 270),
            ];"""
            
    content = content.replace(old_targets, new_targets)

    # Write back
    open(fpath, 'w', encoding='utf-8', newline='\r\n').write(content)
    print("Success")

if __name__ == '__main__':
    main()
