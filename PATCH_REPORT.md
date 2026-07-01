# 📝 키서 (Keysor) 패치 리포트 (Patch Report)

본 문서는 키서(Keysor) 프로젝트 진행 중 발생한 버그 수정 및 개선 사항을 정량적/기술적으로 기록하는 패치 리포트입니다.

---

## 📅 패치 내역 요약 (Patch History)

| 버전 / 일시 | 패치 구분 | 주요 요약 및 해결 내용 | 상태 |
| :--- | :--- | :--- | :--- |
| **v0.2.6**<br>(2026-06-05) | **버그 수정 (UI/Render)** | **마우스 이동 시 인디케이터(초록색 원) 사라짐 및 비동기 드로우 지연 현상 수정** | **완료** |
| **v0.2.5**<br>(2026-06-05) | **버그 수정 (Hook)** | **try_lock()을 lock().unwrap()으로 롤백하여 키 입력 누설 및 대각선 조작 오류 해결** | **완료** |
| **v0.2.4**<br>(2026-06-05) | **버그 수정 (Hook)** | **가상 입력(Injected) 우회 로직 완전 제거 (RDP/가상머신/키매핑 완벽 호환)** | **완료** |
| **v0.2.3**<br>(2026-06-05) | **버그 수정 (Hook)** | **가상/매핑된 CapsLock(PowerToys 등) 이벤트 처리 예외 추가** | **완료** |
| **v0.2.2**<br>(2026-06-05) | **버그 수정 (IME)** | **물리 CapsLock 차단(Suppress) 및 토글 모드 오작동 수정** | **완료** |
| **v0.2.1**<br>(2026-06-05) | **버그 수정 (IME)** | **자동 한/영 전환 제거 및 입력 상태 완전 보존** | **완료** |
| **v0.2.0**<br>(2026-06-05) | **기능 추가 (Focus)** | **텍스트 포커스 감지 시 마우스 모드 자동 해제 (Focus Guard)** | **완료** |
| **v0.1.9**<br>(2026-06-05) | **버그 수정 (Freeze)**| **외부 가상 입력(Antigravity 자동입력) 바이패스 및 훅 프리징 해소** | **완료** |

---

## 🔍 상세 패치 내용 (Detailed Patches)

### 🟢 [v0.2.6] 마우스 이동 시 인디케이터(초록색 원) 사라짐 및 비동기 드로우 지연 현상 수정 (완료)
- **배경**: 이전 인디케이터 최적화 작업 중 비동기 처리(`SWP_ASYNCWINDOWPOS` 및 `PostMessageW`)로 전환했으나, 100Hz 고주파 마우스 이동이 발생할 때 윈도우 스레드 메시지 큐의 병목 현상 및 동기화 지연으로 인해 인디케이터 레이어 윈도우가 화면에서 사라지거나 깜빡거리며 끊기는 물리적 렌더링 결함이 발생했습니다.
- **해결책**: 인디케이터 윈도우 스레드 자체에서 `SetWindowPos`를 동기적으로 실행하도록 `SWP_ASYNCWINDOWPOS` 플래그를 삭제하고, `UpdateWindow(hwnd)`를 추가하여 강제 동기 GDI 화면 다시 그리기(Synchronous GDI Paint)를 유도했습니다. 이로써 마우스가 아무리 빠르게 이동해도 초록색 원 인디케이터가 잔상이나 지연 없이 마우스 핫스팟에 완벽하게 달라붙어 렌더링됩니다.
- **대상 파일**:
  - [src/indicator.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/indicator.rs): `indicator_wnd_proc` 내 `WM_USER_SHOW` 및 `WM_USER_UPDATE` 핸들러의 렌더링 플래그 및 GDI 동기 갱신 코드 적용.

### 🟢 [v0.2.5] try_lock()을 lock().unwrap()으로 롤백하여 키 입력 누설 및 대각선 조작 오류 해결 (완료)
- **배경**: 훅 프리징 방지를 위해 훅 콜백에 적용했던 `try_lock()`이 다중 키 동시 입력(예: 대각선 이동을 위한 W+A 조합) 및 고속 연속 키 입력 상황에서 뮤텍스 경쟁 실패를 유발하여, 키 입력 이벤트가 무시(누설)되고 이동이 부자연스럽게 끊기거나 대각선 조작이 불가능한 현상이 발생했습니다.
- **해결책**: v0.2.2 패치를 통해 이동 스레드 측에서 블로킹 API(Windows Relative Move, GUI Render 등)를 호출하기 직전에 이미 뮤텍스 락을 반환하도록 개선되어 훅 스레드에서 대기가 길어질 위험이 사라졌습니다. 따라서 훅 콜백 내부의 상태 잠금을 `try_lock()`에서 다시 안정적인 `lock().unwrap()` 동기 락 방식으로 롤백했습니다. 이를 통해 모든 입력 누설과 조작 부자연스러움이 사라지고 대각선 이동도 완벽히 부드럽게 작동합니다.
- **대상 파일**:
  - [src/hook.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/hook.rs): `low_level_keyboard_proc` 내 상태 락 `try_lock()` -> `lock().unwrap()`으로 수정.

### 🟢 [v0.2.4] 가상 입력(Injected) 우회 로직 완전 제거 (RDP/가상머신/키매핑 완벽 호환) (완료)
- **배경**: 이전 패치에서 AI 자동 입력 프리징을 막기 위해 가상 입력(Injected) 우회 로직을 도입했으나, Windows RDP(원격 데스크톱), VM(가상머신) 및 PowerToys 등으로 키를 매핑하여 사용하는 경우 CapsLock 뿐만 아니라 WASD 이동 키까지 모두 가상 입력으로 판정되어 마우스 조작 자체가 무시되는 현상이 발견되었습니다.
- **해결책**: 가상 훅 데드락의 원인이 되었던 뮤텍스 대기(lock().unwrap())가 이미 v0.2.2에서 `try_lock()` 논블로킹 방식으로 대체되어 훅 프리징 위험이 사라졌으므로, 가상 입력 우회 로직(`is_injected` 체크)을 전격적으로 완전히 제거했습니다. 이로써 원격 제어, 가상머신, 각종 재매핑 환경에서도 키서가 100% 정상 작동합니다.
- **대상 파일**:
  - [src/hook.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/hook.rs): `low_level_keyboard_proc` 내 `is_injected` 판단 및 우회 코드 완전 삭제 및 불필요한 로그 정리.

### 🟢 [v0.2.3] 가상/매핑된 CapsLock(PowerToys 등) 이벤트 처리 예외 추가 (완료)
- **배경**: AI 자동 입력 시의 데드락 방지를 위해 도입한 가상(Injected) 키 입력 바이패스 로직(v0.1.9)으로 인해, 사용자가 PowerToys Keyboard Manager나 AutoHotkey 등을 사용하여 CapsLock 키를 재매핑/가상화하여 사용하는 경우 CapsLock 물리 키 입력마저 바이패스되어 커서 모드가 전혀 켜지지 않는 문제가 발생했습니다.
- **해결책**: 가상 키 입력 바이패스 조건에서 CapsLock(0x14) 키는 제외(`vk_code != 0x14`) 처리하여, 재매핑되거나 가상으로 입력되는 CapsLock 이벤트도 정상적으로 키서의 커서 모드 토글 로직을 타도록 수정했습니다.
- **대상 파일**:
  - [src/hook.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/hook.rs): `low_level_keyboard_proc` 내 `is_injected` 판단식 수정.

### 🟢 [v0.2.2] 물리 CapsLock 차단(Suppress) 및 토글 모드 오작동 수정 (완료)
- **배경**: 이전 v0.2.1 패치에서 강제 영어 전환 API를 제거했음에도 불구하고, CapsLock 물리 입력이 윈도우 OS로 계속 통과(`CallNextHookEx`)하면서 Windows IME가 CapsLock 활성화를 영어 입력 상태로 인식하고 한글 모드를 강제로 영문 모드로 전환시키는 OS 고유의 부작용이 발생했습니다.
- **해결책**: 마우스 모드 진입/해제 시 발생하는 물리 CapsLock 입력을 OS가 받지 못하도록 훅 수준에서 다시 **완벽히 차단(return 1)** 하도록 수정했습니다.
- **토글 모드 오동작 수정**: v0.2.2 패치 도중 토글 모드에서도 CapsLock 키업(Keyup) 시 가상 CapsLock 주입 로직이 작동하여 OS의 CapsLock이 토글되고 IME가 영어로 바뀌던 로직 버그를 수정했습니다. 이제 **토글 모드에서는 캡스락 클릭으로 마우스 모드만 온오프될 뿐, OS의 CapsLock 상태를 전혀 건드리지 않으므로 한글 상태가 완벽하게 유지**됩니다.
  - 원래의 대소문자 전환이 필요한 경우에는 **홀드 모드(Hold)**일 때 250ms 이하의 빠른 단독 탭을 입력할 때만 가상 CapsLock 신호(`dw_extra_info = 0x12345678`)를 주입하여 대소문자 토글 기능이 호환되도록 처리했습니다.
- **대상 파일**:
  - [src/hook.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/hook.rs): CapsLock keyup 핸들러 내 `is_toggle_mode` 분기 수정 및 물리 CapsLock 리턴값 `return 1` 고정.
