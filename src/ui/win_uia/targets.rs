//! Snapshot UIA properties in bulk, preserving target order and fallback behavior.
use super::*;
use uiautomation::core::{UICacheRequest, UICondition, UIElement};
use uiautomation::errors::Result;
use uiautomation::types::{ControlType, ElementMode, Rect, TreeScope, UIProperty};

pub(super) struct TargetReader {
    cache: Option<UICacheRequest>,
    result_cache: Option<UICacheRequest>,
    taskbar_cache: Option<UICacheRequest>,
}
fn property_cache(
    automation: &UIAutomation,
    properties: &[UIProperty],
    mode: ElementMode,
) -> Option<UICacheRequest> {
    (|| -> Result<UICacheRequest> {
        let cache = automation.create_cache_request()?;
        cache.set_tree_scope(TreeScope::Element)?;
        cache.set_element_mode(mode)?;
        for &property in properties {
            cache.add_property(property)?;
        }
        Ok(cache)
    })()
    .ok()
}
impl TargetReader {
    pub(super) fn new(automation: &UIAutomation) -> Self {
        Self {
            // Traversal still needs live references. Final target snapshots do not.
            cache: property_cache(
                automation,
                &[UIProperty::BoundingRectangle, UIProperty::ControlType],
                ElementMode::Full,
            ),
            result_cache: property_cache(
                automation,
                &[
                    UIProperty::BoundingRectangle,
                    UIProperty::AutomationId,
                    UIProperty::Name,
                ],
                ElementMode::None,
            ),
            taskbar_cache: property_cache(
                automation,
                &[UIProperty::BoundingRectangle],
                ElementMode::None,
            ),
        }
    }
    fn has_cache(&self) -> bool {
        self.cache.is_some() || self.result_cache.is_some() || self.taskbar_cache.is_some()
    }
    fn element(&self, automation: &UIAutomation, handle: Handle) -> Result<UIElement> {
        if let Some(cache) = &self.cache {
            automation
                .element_from_handle_build_cache(handle, cache)
                .or_else(|_| automation.element_from_handle(handle))
        } else {
            automation.element_from_handle(handle)
        }
    }
    fn find_all(
        &self,
        element: &UIElement,
        scope: TreeScope,
        condition: &UICondition,
        taskbar: bool,
    ) -> Result<Vec<UIElement>> {
        let cache = if scope == TreeScope::Children {
            &self.cache
        } else if taskbar {
            &self.taskbar_cache
        } else {
            &self.result_cache
        };
        if let Some(cache) = cache {
            element
                .find_all_build_cache(scope, condition, cache)
                .or_else(|_| element.find_all(scope, condition))
        } else {
            element.find_all(scope, condition)
        }
    }
    fn bounds(&self, element: &UIElement) -> Result<Rect> {
        if self.has_cache() {
            element
                .get_cached_bounding_rectangle()
                .or_else(|_| element.get_bounding_rectangle())
        } else {
            element.get_bounding_rectangle()
        }
    }
    fn control_type(&self, element: &UIElement) -> Result<ControlType> {
        if self.has_cache() {
            element
                .get_cached_control_type()
                .or_else(|_| element.get_control_type())
        } else {
            element.get_control_type()
        }
    }
    fn id(&self, element: &UIElement) -> Result<String> {
        if self.has_cache() {
            element
                .get_cached_automation_id()
                .or_else(|_| element.get_automation_id())
        } else {
            element.get_automation_id()
        }
    }
    fn name(&self, element: &UIElement) -> Result<String> {
        if self.has_cache() {
            element.get_cached_name().or_else(|_| element.get_name())
        } else {
            element.get_name()
        }
    }
}

fn scan_titlebar_targets(
    element: &uiautomation::core::UIElement,
    win_rect: &RECT,
    true_cond: &uiautomation::core::UICondition,
    clickable_types: &[i32],
    depth: usize,
    results: &mut Vec<uiautomation::core::UIElement>,
    reader: &TargetReader,
) {
    if depth > 6 {
        return;
    }

    let mut should_traverse = true;

    if let Ok(rect) = reader.bounds(element) {
        let top = rect.get_top();

        // 만약 엘리먼트의 상단이 타이틀바 영역(win_rect.top + 100)보다 아래에 있다면 하위 탐색 중단 및 프루닝
        if top > win_rect.top + 100 {
            return;
        }

        if let Ok(ctrl_type) = reader.control_type(element) {
            let ctrl_type_val = ctrl_type as i32;

            // Document 타입(웹 페이지 본문 등)은 하위 탐색하지 않음
            if ctrl_type_val == 50030 {
                return;
            }

            // 클릭 가능한 대상 타입인 경우
            if clickable_types.contains(&ctrl_type_val) {
                should_traverse = false;
                results.push(element.clone());
            }
        }
    }

    let children_opt = if should_traverse {
        reader
            .find_all(
                element,
                uiautomation::types::TreeScope::Children,
                true_cond,
                false,
            )
            .ok()
    } else {
        None
    };

    if let Some(children) = children_opt {
        for child in &children {
            scan_titlebar_targets(
                child,
                win_rect,
                true_cond,
                clickable_types,
                depth + 1,
                results,
                reader,
            );
        }
    }
}

pub(super) fn scan_window_targets(
    automation: &UIAutomation,
    handle: Handle,
    class_str: &str,
    win_rect: &RECT,
    condition: &UICondition,
    clickable_types: &[i32],
    reader: &TargetReader,
) -> Result<Vec<(i32, i32)>> {
    let element = reader.element(automation, handle)?;
    let is_heavy = matches!(
        class_str,
        "Chrome_WidgetWin_1" | "MozillaWindowClass" | "ApplicationFrameWindow" | "CabinetWClass"
    );
    let mut elements = Vec::new();
    if is_heavy {
        if let Ok(true_cond) = automation.create_true_condition() {
            scan_titlebar_targets(
                &element,
                win_rect,
                &true_cond,
                clickable_types,
                0,
                &mut elements,
                reader,
            );
        }
    } else if let Ok(descendants) = reader.find_all(
        &element,
        TreeScope::Descendants,
        condition,
        matches!(class_str, "Shell_TrayWnd" | "SecondaryTrayWnd"),
    ) {
        elements = descendants;
    }
    let mut new_targets = Vec::new();
    for el in &elements {
        if let Ok(rect) = reader.bounds(el) {
            let left = rect.get_left();
            let top = rect.get_top();
            let right = rect.get_right();
            let bottom = rect.get_bottom();
            let w = right - left;
            let h = bottom - top;

            if w >= 5 && h >= 5 && w <= 800 && h <= 800 {
                let is_taskbar = class_str == "Shell_TrayWnd" || class_str == "SecondaryTrayWnd";
                let mut is_valid = is_taskbar;

                if !is_valid {
                    // Check if it's in the top-left or top-right title bar area of the application window
                    let in_titlebar_y = top >= win_rect.top - 20 && bottom <= win_rect.top + 80;
                    let in_top_right = in_titlebar_y
                        && right >= win_rect.right - 200
                        && left <= win_rect.right + 20;
                    let in_top_left =
                        in_titlebar_y && left >= win_rect.left - 20 && right <= win_rect.left + 220;

                    if in_top_right || in_top_left {
                        is_valid = true;
                    } else {
                        // Minimize/Maximize/Close ID/Name check as fallback
                        if let Ok(auto_id) = reader.id(el) {
                            let id_lower = auto_id.to_lowercase();
                            if id_lower.contains("minimize")
                                || id_lower.contains("maximize")
                                || id_lower.contains("close")
                                || id_lower.contains("restore")
                            {
                                is_valid = true;
                            }
                        }
                        if !is_valid
                            && reader
                                .name(el)
                                .map(|name| {
                                    let name_lower = name.to_lowercase();
                                    name_lower.contains("최소화")
                                        || name_lower.contains("최대화")
                                        || name_lower.contains("닫기")
                                        || name_lower.contains("복원")
                                        || name_lower.contains("minimize")
                                        || name_lower.contains("maximize")
                                        || name_lower.contains("close")
                                        || name_lower.contains("restore")
                                })
                                .unwrap_or(false)
                        {
                            is_valid = true;
                        }
                    }
                }

                if is_valid {
                    new_targets.push((left + w / 2, top + h / 2));
                    if new_targets.len() == 200 {
                        break;
                    }
                }
            }
        }
    }
    Ok(new_targets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    struct Fixture {
        hwnd: isize,
        worker: Option<std::thread::JoinHandle<()>>,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            unsafe {
                PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
            }
            if let Some(worker) = self.worker.take() {
                worker.join().unwrap();
            }
        }
    }
    unsafe extern "system" fn procedure(hwnd: isize, msg: u32, w: usize, l: isize) -> isize {
        unsafe {
            if msg == WM_DESTROY {
                PostQuitMessage(0);
                return 0;
            }
            DefWindowProcW(hwnd, msg, w, l)
        }
    }
    fn fixture(count: i32) -> Fixture {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || unsafe {
            let instance =
                windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
            let name = crate::ui::win_gdi::encode_wide("KeysorTargetBenchmark");
            let class = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: instance,
                lpszClassName: name.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&class);
            let hwnd = CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                name.as_ptr(),
                name.as_ptr(),
                WS_POPUP,
                20,
                20,
                900,
                700,
                0,
                0,
                instance,
                std::ptr::null(),
            );
            assert_ne!(hwnd, 0);
            for i in 0..count {
                let child = CreateWindowExW(
                    0,
                    crate::ui::win_gdi::encode_wide("BUTTON").as_ptr(),
                    crate::ui::win_gdi::encode_wide("Close").as_ptr(),
                    WS_CHILD | WS_VISIBLE,
                    8 + (i % 10) * 85,
                    8 + (i / 10) * 25,
                    80,
                    22,
                    hwnd,
                    (100 + i) as isize,
                    instance,
                    std::ptr::null(),
                );
                assert_ne!(child, 0);
            }
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            tx.send(hwnd).unwrap();
            let mut message = std::mem::zeroed();
            while GetMessageW(&mut message, 0, 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        });
        Fixture {
            hwnd: rx.recv_timeout(Duration::from_secs(10)).unwrap(),
            worker: Some(worker),
        }
    }
    #[test]
    #[ignore = "Checks UIA cache compatibility on temporary controls; no user input or settings changes"]
    fn native_target_cache_missing_properties_falls_back() {
        let window = fixture(16);
        let automation = UIAutomation::new().unwrap();
        let condition = automation
            .create_property_condition(
                UIProperty::ControlType,
                uiautomation::variants::Variant::from(50000),
                None,
            )
            .unwrap();
        let empty_cache = || Some(automation.create_cache_request().unwrap());
        let incomplete = TargetReader {
            cache: empty_cache(),
            result_cache: empty_cache(),
            taskbar_cache: empty_cache(),
        };
        let uncached = TargetReader {
            cache: None,
            result_cache: None,
            taskbar_cache: None,
        };
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetWindowRect(window.hwnd, &mut rect);
        }
        for class in ["BenchmarkWindow", "Chrome_WidgetWin_1", "Shell_TrayWnd"] {
            let expected = scan_window_targets(
                &automation,
                Handle::from(window.hwnd),
                class,
                &rect,
                &condition,
                &[50000],
                &uncached,
            )
            .unwrap();
            let actual = scan_window_targets(
                &automation,
                Handle::from(window.hwnd),
                class,
                &rect,
                &condition,
                &[50000],
                &incomplete,
            )
            .unwrap();
            assert!(!expected.is_empty());
            assert_eq!(actual, expected);
        }
    }

    #[test]
    #[ignore = "Measures native UIA scans on temporary controls; no input hook, user settings or clipboard changes"]
    fn native_target_cache_benchmark() {
        let window = fixture(240);
        let automation = UIAutomation::new().unwrap();
        let condition = automation
            .create_property_condition(
                UIProperty::ControlType,
                uiautomation::variants::Variant::from(50000),
                None,
            )
            .unwrap();
        let cached = TargetReader::new(&automation);
        assert!(cached.cache.is_some());
        let uncached = TargetReader {
            cache: None,
            result_cache: None,
            taskbar_cache: None,
        };
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetWindowRect(window.hwnd, &mut rect);
        }
        let handle = Handle::from(window.hwnd);
        let mut reports = Vec::new();
        // Compare identical target predicates, including the existing heavy-window pruning.
        for class in ["BenchmarkWindow", "Chrome_WidgetWin_1", "Shell_TrayWnd"] {
            let before = scan_window_targets(
                &automation,
                handle,
                class,
                &rect,
                &condition,
                &[50000],
                &uncached,
            )
            .unwrap();
            let after = scan_window_targets(
                &automation,
                handle,
                class,
                &rect,
                &condition,
                &[50000],
                &cached,
            )
            .unwrap();
            assert_eq!(
                before, after,
                "Target order/coordinates changed for {}",
                class
            );
            assert!(!after.is_empty());
            if class != "Chrome_WidgetWin_1" {
                assert_eq!(after.len(), 200);
            }
            let mut old_times = Vec::new();
            let mut new_times = Vec::new();
            for round in 0..7 {
                // Alternate execution order to reduce warm-cache bias.
                for optimized in if round % 2 == 0 {
                    [false, true]
                } else {
                    [true, false]
                } {
                    let start = Instant::now();
                    let result = scan_window_targets(
                        &automation,
                        handle,
                        class,
                        &rect,
                        &condition,
                        &[50000],
                        if optimized { &cached } else { &uncached },
                    )
                    .unwrap();
                    let elapsed = start.elapsed().as_micros() as u64;
                    assert_eq!(result, after);
                    if optimized {
                        new_times.push(elapsed);
                    } else {
                        old_times.push(elapsed);
                    }
                }
            }
            old_times.sort_unstable();
            new_times.sort_unstable();
            reports.push(serde_json::json!({"class":class,"controls":240,"targets":after.len(),
                "samples_per_path":7,"before_median_us":old_times[3],"after_median_us":new_times[3]}));
        }
        std::fs::write(
            "target/priority-perf-uia-benchmark.json",
            serde_json::to_string_pretty(&reports).unwrap(),
        )
        .unwrap();
    }
}
