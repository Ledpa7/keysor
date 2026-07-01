# 키서 (Keysor)

마우스 없이 키보드만으로 마우스 커서를 정밀하고 부드럽게 조작할 수 있도록 돕는 초안정성 크로스 플랫폼 유틸리티 소프트웨어입니다.

---

## 📂 프로젝트 코드 구성 및 아키텍처 가이드

본 가이드는 키서의 프로젝트 구조를 처음 접하는 개발자가 목적에 따라 어떤 모듈을 수정해야 하는지 직관적으로 안내하기 위해 작성되었습니다.

### 1. 전체 디렉토리 및 모듈 구성

```text
src/
├── main.rs          # 앱 진입점, 초기화, 라이선스 체크 스케줄러, 설정 핫리로드 루프
├── config.rs        # keysor.yaml 설정 로드, 바인딩 매핑 및 감도 기본값 제어
├── hook.rs          # 비즈니스 로직 (키보드 이벤트 분기, 가속 이동 스레드, 드래그/클릭 에뮬레이션)
├── math.rs          # 마우스 가속도 물리 연산 및 정밀 이동(Pixel Mode) 델타 계산 공식
├── license.rs       # 트라이얼 상태 검증(14일), Lemon Squeezy API 라이선스 인증 및 머신 ID 캐싱
├── indicator.rs     # GDI/GDI+ 시각 인디케이터 창 드로잉, 자석 스냅(Magnetic Snapping) UIA 탐색
└── platform/        # 저수준 운영체제(OS) 추상화 인터페이스 레이어
    ├── mod.rs       # SystemController 및 KeyboardHook 공통 Trait 인터페이스 정의
    ├── windows/     # Windows용 전역 low-level 훅 및 SendInput 마우스 드라이버
    └── macos/       # macOS용 CGEventTap 훅 및 CoreGraphics 마우스 드라이버
```

---

## 🛠️ 목적별 수정 모듈 가이드

원하는 수정 사항이 있을 때 아래의 모듈을 수정하십시오.

### A. 새로운 단축키 동작이나 감도 옵션을 추가하고 싶을 때
- 🔍 **대상 파일**: [src/config.rs](file:///c:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/config.rs) 및 프로젝트 루트의 `keysor.yaml`
- **방법**: `Config` 및 `Settings` 구조체에 필드를 추가하고, `get_vk_bindings` 매핑 리스트에 가상 키코드 상수를 연동합니다.

### B. 키 입력 시의 마우스 조작 행동(Space 탭/드래그, 마우스 보조 버튼 등)을 수정하고 싶을 때
- 🔍 **대상 파일**: [src/hook.rs](file:///c:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/hook.rs)
- **방법**: 키 이벤트 훅 콜백인 `handle_keyboard_event`와 `execute_pending_action` 헬퍼 함수, 혹은 `AppState` 내부의 키 입력 처리 관련 메서드를 수정하십시오.

### C. 마우스 가속도 곡선이나 정밀 이동 물리 공식을 다듬고 싶을 때
- 🔍 **대상 파일**: [src/math.rs](file:///c:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/math.rs)
- **방법**: `calculate_speed` (시간 경과에 따른 가속 속도 산출) 및 `calculate_movement_delta` (DPI 스케일과 픽셀 연산을 조합한 가감속 연산) 함수 내부의 수학적 모델을 튜닝합니다.

### D. 시각 피드백 디자인, UI 레이아웃, 혹은 자석 스냅 대상을 수정하고 싶을 때
- 🔍 **대상 파일**: [src/indicator.rs](file:///c:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/indicator.rs)
- **방법**:
  - **시각 잔상/디자인**: `draw_indicator` 및 layered window 업데이트 부분을 수정합니다.
  - **자석 스냅**: `check_magnetic_snapping` 이나 `check_global_magnetic_snapping` 내부의 스냅 좌표 리스트 및 감도를 조정합니다.
  - **성능 튜닝**: UIA 쿼리 스킵 블랙리스트 필터는 `start_global_targets_thread` 내부에 위치합니다.

### E. 저수준 OS 드라이버 레벨 제어(마우스 클릭 신호 변경, macOS/Linux 지원 등)를 확장하고 싶을 때
- 🔍 **대상 디렉토리**: [src/platform/](file:///c:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/platform/) 하위의 플랫폼 폴더
- **방법**:
  - Windows 하드웨어 입력 제어는 `platform/windows/` 하위 모듈을 수정합니다.
  - macOS CoreGraphics 기반 입력 제어는 `platform/macos/` 하위 모듈을 수정합니다.

### F. 라이선스 인증 방식이나 암호화, 트라이얼 일수 규정을 수정하고 싶을 때
- 🔍 **대상 파일**: [src/license.rs](file:///c:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/license.rs)
- **방법**: `check_trial_status` (트라이얼 파일 복호화 및 날짜 비교) 및 `activate_license` API 연동 함수를 수정하십시오.

---

## ⚠️ 중요 개발 맥락 및 히스토리 (리셋 대비 인계 사항)

대화 컨텍스트가 초기화되었을 때 다음 AI 또는 개발자가 절대 훼손하지 않아야 하는 중요 설계 규정입니다.

1. **UIA CPU 점유율 스파이크 방지 (브라우저 블랙리스트)**:
   - [indicator.rs](file:///c:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/indicator.rs)의 `start_global_targets_thread`에서 `Chrome_WidgetWin_1` (크롬, 엣지, 일렉트론 등)과 `MozillaWindowClass` (파이어폭스) 브라우저는 자식 노드 탐색(`Descendants`) 오버헤드가 극도로 크므로 UIA 탐색을 즉시 스킵해야 합니다.
2. **동기식 레지스트리 프로세스(`reg.exe`) 실행 방지**:
   - [license.rs](file:///c:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/license.rs)의 `get_machine_id()`는 매번 프로세스를 띄우지 않도록 `OnceLock<String>` 메모리 캐시를 반드시 유지해야 핫리로드 시 화면 프리징이 예방됩니다.
3. **재진입 락 데드락 방지**:
   - Windows 훅 내부에서 가상 키보드 입력을 임베딩하여 에뮬레이션할 때 동기적으로 전역 훅 콜백이 재귀 호출됩니다. 이 때문에 훅 콜백 스레드 내부에 `Mutex` 가드가 중첩되면 무조건적인 데드락이 발생하여 마우스가 우측 상단으로 쏠리며 동결됩니다. 락 가드 범위와 실행부(`execute_pending_action`) 분리 구조를 절대 변경하지 마십시오.
4. **macOS용 CoreGraphics / EventTap 구현체 분리**:
   - [platform/macos/](file:///c:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/platform/macos/) 하위 파일들은 Windows 크로스 컴파일(빌드) 호환성을 해치지 않으면서도 실제 맥 환경에서 테스트 구동되도록 `CFRunLoop`와 `CGEventTap` C API 바인딩이 구현되어 있습니다. 수정 시 `cfg(target_os = "windows")` 조건부 컴파일 분기를 깨뜨리지 않도록 유의하십시오.


