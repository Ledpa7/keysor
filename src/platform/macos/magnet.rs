use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static IS_SNAPPED: AtomicBool = AtomicBool::new(false);
static SNAPPED_CENTER: Mutex<Option<(f64, f64)>> = Mutex::new(None);
static ESCAPED_CENTER: Mutex<Option<((f64, f64), Instant)>> = Mutex::new(None);

pub fn is_currently_snapped() -> bool {
    IS_SNAPPED.load(Ordering::SeqCst)
}

pub fn clear_magnetic_snapping() {
    IS_SNAPPED.store(false, Ordering::SeqCst);
    if let Ok(mut sc) = SNAPPED_CENTER.lock() {
        *sc = None;
    }
    if let Ok(mut ec) = ESCAPED_CENTER.lock() {
        *ec = None;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

// macOS ApplicationServices & CoreGraphics 프레임워크 링킹 선언
#[link(name = "ApplicationServices", kind = "framework")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreate(source: *mut c_void) -> *mut c_void;
    fn CGEventGetLocation(event: *mut c_void) -> CGPoint;
    fn CGWarpMouseCursorPosition(newCursorPosition: CGPoint) -> i32;
    fn CFRelease(obj: *mut c_void);

    fn AXUIElementCreateSystemWide() -> *mut c_void;
    fn AXUIElementCopyElementAtPosition(
        systemWideElement: *mut c_void,
        x: f32,
        y: f32,
        element: *mut *mut c_void,
    ) -> i32;
    fn AXUIElementCopyAttributeValue(
        element: *mut c_void,
        attribute: *mut c_void,
        value: *mut *mut c_void,
    ) -> i32;
    fn AXUIElementGetPid(
        element: *mut c_void,
        pid: *mut i32,
    ) -> i32;
    fn AXValueGetValue(
        value: *mut c_void,
        valueType: u32,
        outValue: *mut c_void,
    ) -> bool;
    fn CFStringCreateWithCString(
        alloc: *mut c_void,
        cStr: *const i8,
        encoding: u32,
    ) -> *mut c_void;
    fn CFStringGetCString(
        theString: *mut c_void,
        buffer: *mut i8,
        bufferSize: isize,
        encoding: u32,
    ) -> bool;
}

fn create_cf_string(s: &str) -> *mut c_void {
    let c_str = match std::ffi::CString::new(s) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };
    unsafe { CFStringCreateWithCString(std::ptr::null_mut(), c_str.as_ptr(), 0x08000100) }
}

fn get_cf_string_val(cf_str: *mut c_void) -> String {
    if cf_str.is_null() { return String::new(); }
    let mut buf = [0i8; 128];
    unsafe {
        if CFStringGetCString(cf_str, buf.as_mut_ptr(), 128, 0x08000100) {
            std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
        } else {
            String::new()
        }
    }
}

pub fn check_macos_magnetic_snapping(is_moving: bool) {
    check_macos_global_magnetic_snapping(is_moving);
}

pub fn check_macos_global_magnetic_snapping(is_moving: bool) {
    let is_enabled = {
        let state_arc = match crate::hook::APP_STATE.get() {
            Some(arc) => arc,
            None => return,
        };
        let state = match state_arc.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        state.config.settings.magnetic_mode.unwrap_or(false)
            || state.config.settings.global_magnetic_mode.unwrap_or(false)
    };

    if !is_enabled {
        IS_SNAPPED.store(false, Ordering::SeqCst);
        if let Ok(mut sc) = SNAPPED_CENTER.lock() {
            *sc = None;
        }
        if let Ok(mut ec) = ESCAPED_CENTER.lock() {
            *ec = None;
        }
        return;
    }

    let my_pid = std::process::id() as i32;

    unsafe {
        let event = CGEventCreate(std::ptr::null_mut());
        if event.is_null() { return; }
        let loc = CGEventGetLocation(event);
        CFRelease(event);

        let cx = loc.x;
        let cy = loc.y;

        // 1. 이미 흡착된 상태인 경우
        if let Ok(sc_opt) = SNAPPED_CENTER.lock() {
            if let Some((sx, sy)) = *sc_opt {
                if is_moving {
                    IS_SNAPPED.store(false, Ordering::SeqCst);
                    drop(sc_opt);
                    if let Ok(mut sc) = SNAPPED_CENTER.lock() {
                        *sc = None;
                    }
                    if let Ok(mut ec) = ESCAPED_CENTER.lock() {
                        *ec = Some(((sx, sy), Instant::now()));
                    }
                    return;
                } else {
                    let dist = ((cx - sx).powi(2) + (cy - sy).powi(2)).sqrt();
                    if dist <= 25.0 {
                        IS_SNAPPED.store(true, Ordering::SeqCst);
                        return;
                    } else {
                        IS_SNAPPED.store(false, Ordering::SeqCst);
                        drop(sc_opt);
                        if let Ok(mut sc) = SNAPPED_CENTER.lock() {
                            *sc = None;
                        }
                        if let Ok(mut ec) = ESCAPED_CENTER.lock() {
                            *ec = Some(((sx, sy), Instant::now()));
                        }
                    }
                }
            }
        }

        if is_moving {
            IS_SNAPPED.store(false, Ordering::SeqCst);
            return;
        }

        // 2. 근처 소형 UI 버튼 초고속 단일 패스 탐색 (Mach IPC 호출을 60회에서 3회로 95% 감축하여 0ms 버벅임 무지연 단번에 스냅)
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() { return; }

        let snap_threshold = 25.0; // 윈도우 고정 25.0px
        let attr_role = create_cf_string("AXRole");
        let attr_pos = create_cf_string("AXPosition");
        let attr_size = create_cf_string("AXSize");
        let attr_parent = create_cf_string("AXParent");

        let mut target_center: Option<(f64, f64)> = None;

        let mut elem: *mut c_void = std::ptr::null_mut();
        let err = AXUIElementCopyElementAtPosition(system_wide, cx as f32, cy as f32, &mut elem);
        if err == 0 && !elem.is_null() {
            let mut elem_pid: i32 = 0;
            if AXUIElementGetPid(elem, &mut elem_pid) == 0 && elem_pid != my_pid {
                let mut to_release: Vec<*mut c_void> = Vec::with_capacity(3);
                to_release.push(elem);

                let mut current_eval = elem;
                let mut depth = 0;

                while !current_eval.is_null() && depth < 2 {
                    let mut role_val: *mut c_void = std::ptr::null_mut();
                    if AXUIElementCopyAttributeValue(current_eval, attr_role, &mut role_val) == 0 && !role_val.is_null() {
                        let role = get_cf_string_val(role_val);
                        CFRelease(role_val);

                        let is_target_role = matches!(
                            role.as_str(),
                            "AXButton" | "AXLink" | "AXPopUpButton" | "AXCheckBox"
                            | "AXRadioButton" | "AXMenuItem" | "AXTabButton" | "AXMenuButton"
                        );

                        if is_target_role {
                            let mut pos_val: *mut c_void = std::ptr::null_mut();
                            let mut size_val: *mut c_void = std::ptr::null_mut();

                            let pos_res = AXUIElementCopyAttributeValue(current_eval, attr_pos, &mut pos_val);
                            let size_res = AXUIElementCopyAttributeValue(current_eval, attr_size, &mut size_val);

                            if pos_res == 0 && !pos_val.is_null() && size_res == 0 && !size_val.is_null() {
                                let mut pt = CGPoint { x: 0.0, y: 0.0 };
                                let mut sz = CGSize { width: 0.0, height: 0.0 };

                                if AXValueGetValue(pos_val, 1, &mut pt as *mut _ as *mut c_void)
                                    && AXValueGetValue(size_val, 2, &mut sz as *mut _ as *mut c_void)
                                {
                                    if sz.width >= 6.0 && sz.width <= 320.0 && sz.height >= 6.0 && sz.height <= 120.0 {
                                        let center_x = if sz.width <= 60.0 {
                                            pt.x + sz.width / 2.0
                                        } else {
                                            cx.clamp(pt.x + 12.0, pt.x + sz.width - 12.0)
                                        };
                                        let center_y = if sz.height <= 40.0 {
                                            pt.y + sz.height / 2.0
                                        } else {
                                            cy.clamp(pt.y + 6.0, pt.y + sz.height - 6.0)
                                        };

                                        let is_recently_escaped = if let Ok(ec_opt) = ESCAPED_CENTER.lock() {
                                            if let Some(((ex, ey), t_esc)) = *ec_opt {
                                                if t_esc.elapsed() < Duration::from_millis(200) {
                                                    ((center_x - ex).powi(2) + (center_y - cy).powi(2)).sqrt() <= 25.0
                                                } else {
                                                    false
                                                }
                                            } else {
                                                false
                                            }
                                        } else {
                                            false
                                        };

                                        if !is_recently_escaped {
                                            let dist = ((center_x - cx).powi(2) + (center_y - cy).powi(2)).sqrt();
                                            if dist <= snap_threshold {
                                                target_center = Some((center_x, center_y));
                                            }
                                        }
                                    }
                                }
                            }

                            if !pos_val.is_null() { CFRelease(pos_val); }
                            if !size_val.is_null() { CFRelease(size_val); }
                            break;
                        }
                    }

                    // 부모 Parent 엘리먼트 추적
                    let mut parent_val: *mut c_void = std::ptr::null_mut();
                    let parent_res = AXUIElementCopyAttributeValue(current_eval, attr_parent, &mut parent_val);
                    if parent_res == 0 && !parent_val.is_null() {
                        to_release.push(parent_val);
                        current_eval = parent_val;
                        depth += 1;
                    } else {
                        break;
                    }
                }

                for ptr in to_release {
                    if !ptr.is_null() {
                        CFRelease(ptr);
                    }
                }
            } else {
                CFRelease(elem);
            }
        }

        if !attr_role.is_null() { CFRelease(attr_role); }
        if !attr_pos.is_null() { CFRelease(attr_pos); }
        if !attr_size.is_null() { CFRelease(attr_size); }
        if !attr_parent.is_null() { CFRelease(attr_parent); }
        CFRelease(system_wide);

        if let Some((tc_x, tc_y)) = target_center {
            IS_SNAPPED.store(true, Ordering::SeqCst);
            if let Ok(mut sc) = SNAPPED_CENTER.lock() {
                *sc = Some((tc_x, tc_y));
            }
            CGWarpMouseCursorPosition(CGPoint { x: tc_x, y: tc_y });
        } else {
            IS_SNAPPED.store(false, Ordering::SeqCst);
        }
    }
}
