# Keysor 공통·Windows 안정성 수정 / macOS 인계

작성일: 2026-10-10
대상: 14-Keysor, Cargo 버전 1.0.1

## 이번 작업의 범위

공통·Windows 안정성 수정에 이어 macOS 전용 코드와 Mac 앱 패키징 스크립트도 수정했다.
Mac 실기기에서 빌드·입력 권한·UI를 검증해야 한다.
이 문서는 이번 수정의 현재 동작을 설명한다. DASHBOARD.md의 과거 완료/금지 기록과
충돌하는 커서 정책은 아래의 최신 정책을 기준으로 확인한다.

작업 전부터 수정되어 있던 DASHBOARD.md, src/main.rs, src/ui/win_gdi.rs의 기존 변경을
유지하면서 추가 수정했다. 기존 dist/도 이번 작업에서 배포하거나 갱신하지 않았다.

## 수정 파일과 동작

### src/hook.rs (공통)

- Ctrl/Alt 단축키 통과 분기 전에 keyup의 이동/스크롤/우클릭 상태를 정리한다.
  W 누름 → Ctrl 누름 → W 해제 시 이동이 계속되는 문제를 방지한다.
- 이미 처리 중인 Space는 Ctrl/Shift가 나중에 눌려도 Space 해제를 먼저 처리한다.
- PendingMouseAction에 LeftDown/LeftUp을 추가하고 단일 출력 큐에서 OS 입력을 전송한다.
  드래그 상태 변경과 큐 삽입은 같은 AppState 잠금 안에서 수행한다.
  OS API는 출력 작업 스레드에서 호출하므로 AppState 잠금 안에서 실행하지 않는다.
- 모드 해제 시 지연 클릭 카운트/시간과 Space 상태를 취소하고 드래그 해제를 큐에 넣는다.
- 종료 시 새 출력/이동을 중단하고 큐의 LeftUp 완료를 최대 500ms 기다린다.
- modifier_key 설정을 실제 활성화 키 판정에 사용한다.
  기본값 CapsLock 및 Ctrl/Alt/Shift의 좌우 VK 매칭을 지원한다.
- Windows에서 활성화 키의 반복 keydown은 해제 전까지 무시한다.
- macOS CapsLock flagsChanged는 Mac 훅에서 눌림/해제로 정규화한다.
  공통 처리의 임시 150ms Mac 분기는 제거했다.
- 설정 재적용 시 진행 중 입력 상태를 초기화한다.
- start_hook은 Result를 반환한다. 초기 AppState 생성은 initial_state로 분리했다.

### src/config.rs, src/main.rs (공통)

- 파싱 실패 시 사용자 YAML을 기본 설정으로 덮어쓰던 복구 로직을 제거했다.
- 실행 중 try_load_config 실패 시 마지막 정상 설정을 유지하고 오류를 기록한다.
- 초기 실행에서 잘못된 파일은 그대로 두고 메모리에서 기본 설정으로 실행한다.
- 속도 유한성/범위, 모드, 활성화 키, 주사율, 클릭 지연, 바인딩 중복/액션을 검사한다.
- 앱이 설정을 저장할 때 임시 파일에 쓰고 sync_all 후 교체한다.
  Windows는 MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH), 비Windows는 rename이다.
- UI에서 설정 저장 실패를 별도 팝업으로 보여주는 것은 아직 구현하지 않았다.

### src/license.rs (공통)

- 로컬 캐시의 타임스탬프를 검사한다. 미래 시각과 마지막 검증 후 28일 이상은
  로컬 Pro 승인 대상에서 제외한다(14일 검증 주기 + 14일 오프라인 유예).
- AppState 준비 후 재검증 기한을 확인하고, 실행 중 권한 상태를 60초마다 갱신한다.
  네트워크 재시도는 최대 시간당 1회다.
- 재검증은 /v1/licenses/validate를 사용한다. 신규 활성화는 /activate를 사용한다.
- 신규 활성화의 instance.id를 캐시에 저장하고 이후 validate에 instance_id를 전송한다.
- 같은 키의 기존 캐시가 있으면 재활성화 대신 검증한다.
  UI/설정 감시/스케줄러의 라이선스 요청은 하나의 Mutex로 직렬화한다.
- HTTP 요청 타임아웃은 10초. 라이선스 키 원문 출력은 제거했다.
- 권한 변경 시 원본 설정을 다시 평가하여 Pro 바인딩까지 반영한다.
- 이전 4필드 캐시는 계속 읽을 수 있다. instance ID가 없는 구형 캐시는 key 단위로
  검증하므로 기기별 취소 여부까지 검증하지 못한다. 기기 인스턴스 마이그레이션은 남은 과제다.
- XOR 난독화는 여전히 서명이 아니다. 캐시 위변조 및 .trial 삭제에 의한 체험 초기화
  방지는 해결되지 않았다. 서버 서명/제품 식별자 검증은 별도 설계가 필요하다.
- 실제 유료 키로 활성화/검증 요청을 실행하지 않았다.

API 근거: https://docs.lemonsqueezy.com/api/license-api/validate-license-key

### src/platform/windows/hook.rs (Windows)

- SetWindowsHookExW 설치 결과를 채널로 호출자에게 전달한다.
- 실패하면 main에서 오류 창을 띄우고 종료한다. 성공처럼 실행을 계속하지 않는다.
- Hold 모드의 물리 키 확인도 설정된 modifier_key를 사용한다.
- 실행 중 OS에 의한 훅 제거의 자동 감지/재설치는 아직 구현하지 않았다.

### src/ui/win_gdi.rs (Windows)

- SetSystemCursor로 시스템 커서를 투명/초록 커서로 덮어쓰던 경로를 제거했다.
- 기본 OS 커서를 유지하고 오버레이를 추가 피드백으로 사용한다.
- 시작 메뉴/시스템 팝업에서는 기존 오버레이 숨김 정책을 유지하고 기본 커서를 사용한다.
- 이 정책으로 기본 화살표와 오버레이가 함께 보일 수 있다. 표시 크기·겹침은 수동 확인한다.
- 원격 데스크톱이 기본 커서를 전송하지 않는 환경은 별도 확인이 필요하다.
  로컬 커서 보호만으로 원격 화면 표시까지 보장하지 않는다.
- 기존 커서 복구/워치독 코드는 유지했다. 워치독의 '100% 복구'는 검증된 보장이 아니다.
- 라이선스 등록 성공 후 is_pro만 바꾸는 대신 공통 권한 재평가 함수를 호출한다.

## Windows 검증 결과와 실행

- 자동 테스트: 11개 통과, 실패 0개.
- 테스트는 입력 상태/큐 명령/설정 파일/시간 계산을 검사한다.
  실제 전역 키보드 훅이나 사용자 마우스 입력을 실행하는 테스트가 아니다.
- 테스트 빌드에서 앱의 requireAdministrator 매니페스트 때문에 실행 오류 740이 발생했다.
  build.rs에 KEYSOR_TEST_BUILD=1 + debug 프로필일 때만 리소스를 제외하는 분기를 추가했다.
  release에서는 이 변수와 무관하게 기존 리소스/권한 매니페스트가 포함된다.
- Windows 테스트 명령: powershell -ExecutionPolicy Bypass -File scripts/test-windows.ps1
  스크립트는 이전 환경변수를 복원하고 cargo의 종료 코드를 반환한다.
- 릴리즈 명령: cargo build --release --offline --target-dir target/verification
- 테스트 로그: target/mac-change-tests.log
- 릴리즈 로그: target/mac-change-release.log
- 릴리즈 결과: 2026-10-10 Windows 빌드 성공. 경고 63개가 남아 있다.
- 기존 코드의 unused/unsafe_op_in_unsafe_fn 경고가 남아 있다. 경고 0개라고 주장하지 않는다.

검증용 실행 파일은 target/verification/release/keysor.exe이며, 같은 파일을 기본 경로 target/release/keysor.exe에 설치했다.
2026-10-10 기본 실행 파일 교체 시 이전 실행 파일을 keysor.active-old.exe로 이름 변경한 뒤 새 빌드를 복사했다.
두 새 실행 파일의 SHA-256이 일치한다. 이전 실행 파일 백업은 keysor.previous.exe에도 보관했다.
바탕화면 바로가기는 target/release/keysor.exe를 가리킨다. 새 파일을 사용하려면 실행 중인 Keysor를 종료하고 바로가기로 재시작한다.
실제 Windows UI/전역 입력, 유료 라이선스 API, macOS 동작은 아직 수동 검증하지 않았다.
기존 실행을 종료한 뒤 새 파일을 실행하여 다음을 확인한다.

1. Toggle에서 CapsLock을 길게 눌러도 모드가 한 번만 전환되는지.
2. W → Ctrl → W 해제 → Ctrl 해제 후 커서가 멈추는지. R 스크롤도 같은 순서로 확인.
3. Space 드래그 중 Ctrl/Shift를 누른 뒤 Space를 놓으면 드래그가 해제되는지.
4. Space 클릭 직후 모드를 끄면 늦은 클릭이 취소되는지.
5. 드래그 중 모드를 끄거나 앱을 종료하면 버튼이 눌린 채 남지 않는지.
6. modifier_key를 충돌하지 않는 키로 바꾸고 Hold/Toggle을 각각 확인.
7. YAML 문법 오류 저장 시 파일이 덮어써지지 않고 마지막 정상 설정이 유지되는지.
8. 정상 YAML로 고친 뒤 새 설정이 적용되는지.
9. 시작 메뉴, Alt+Tab, 잠금/복귀, 다중 모니터에서 기본 커서가 보이는지.
10. 원격 데스크톱에서 기본 커서와 오버레이가 어떻게 캡처되는지.
11. 실제 테스트용 라이선스로 첫 활성화, 같은 키 재검증, 네트워크 끊김을 확인.
12. 체험/유료 권한 변경 후 바인딩과 HUD 상태가 함께 바뀌는지.

## Mac 구현 내용과 테스트 인계

- src/platform/macos/hook.rs: CapsLock flagsChanged의 플래그 변화는 눌림, 같은 플래그의 다음 이벤트는 해제로 처리한다.
  Toggle은 변화마다 한 번만 전환하고 즉시 해제 이벤트를 공통 처리에 전달한다.
  Hold는 해제 시 모드를 끈다. 실제 키보드의 이벤트 순서는 실기기에서 확인한다.
- 일반 수정 키의 좌우 키코드를 보완했다. Hold의 사용자 설정 키는 HID 물리 상태를 확인하여
  키 해제 이벤트를 놓쳤을 때 모드를 해제한다. CapsLock은 이벤트 기반으로 처리한다.
- EventTap 생성/권한 대기를 백그라운드에서 재시도한다. 준비/손쉬운 사용 권한 필요/
  입력 탭 생성 실패/정상 상태를 HUD 하단에 보여준다. 탭 비활성화 시 입력·드래그를 취소하고
  재활성화하거나 런루프를 정리한 뒤 다시 시도한다. 비활성화 알림은 0xFFFFFFFE/0xFFFFFFFF로 판정한다.
- 출력 큐의 CGEventPost는 작업 스레드에서 실행한다. 기존 Objective-C UI 함수는 메인 큐로
  디스패치한다. 이벤트 콜백 잠금은 호출 전 해제해 종료 단축키와 충돌하지 않게 했다.
- HUD 닫기, 앱 종료 경로는 Rust cleanup_hook을 먼저 호출한다. OS 커서는 숨기지 않으며
  화면 오버레이만 보여준다. 기본 커서와 오버레이가 겹칠 수 있으므로 시각적 확인이 필요하다.
- Mac 라이선스 IOPlatformUUID를 값만 추출하도록 고쳤다. 예전 잘못 파싱한 기기 ID는
  해당 UUID일 때 호환해 기존 캐시를 재검증할 수 있다. UUID를 얻지 못하면 저장된 .machine-id를
  사용하고, 없으면 uuidgen 결과를 저장한다. 최후에는 hostname과 HOME 조합을 쓴다.
- Mac HUD에 별도 수동 라이선스 버튼은 없다. 기존 자동 활성화 작업자가 공통
  refresh_entitlements()를 호출하므로 권한과 바인딩을 함께 갱신한다.
- scratch/package_mac_app.sh와 scratch/generate_mac_icon.swift에서 고정 사용자 경로를
  제거했다. 패키징 스크립트가 프로젝트 폴더를 찾아 릴리즈 빌드와 Keysor.app을 생성한다.
  기존 바탕화면 Keysor.app이 있으면 그 번들도 갱신한다. Finder 강제 재시작은 제거했다.

### Windows에서 확인한 범위

- Windows 테스트 11개 통과, Windows 릴리즈 빌드 성공.
- 설치된 x86_64-apple-darwin 타깃으로 Mac 이벤트 훅 Rust 파일의 독립 타입 검사 통과.
- 전체 Mac cargo check는 Windows에 Apple용 C 컴파일러가 없어 ring 의존성 빌드에서 중단.
- Objective-C, Swift, Mac 앱 번들, 권한 흐름과 실제 입력 동작은 Mac에서 아직 확인하지 못했다.

### Mac 실기기 테스트 순서

1. 이전 Keysor를 종료한 뒤 저장소에서 `cargo test --bin keysor`를 실행한다.
2. `bash scratch/package_mac_app.sh`를 실행하고 `target/release/Keysor.app/Contents/MacOS/keysor`가
   최신 `target/release/keysor`와 일치하는지 확인한다. 기존 바탕화면 앱도 갱신됐는지 확인한다.
3. 손쉬운 사용 권한을 끈 상태로 앱을 열어 HUD 안내를 확인한다. 권한을 부여한 뒤
   재시작 없이 키 입력이 활성화되는지 확인한다. 탭 생성 실패 시 입력 모니터링 권한도 확인한다.
4. CapsLock Toggle을 빠르게 여러 번 누르고 길게 눌러 한 번의 물리 누름마다 한 번만
   바뀌는지 확인한다. Hold에서 누르는 동안만 활성화되는지 확인한다.
5. modifier_key를 `W`, `Shift`, `Ctrl`, `Alt`로 바꿔 각각 Toggle/Hold를 확인한다.
   창 전환·잠금/복귀 후 키를 놓쳤을 때 모드가 남지 않는지도 확인한다.
6. Space 드래그 중 Ctrl/Shift를 누른 채 Space를 놓고, 모드를 끄거나 앱을 종료한다.
   버튼 눌림이 남지 않는지 확인한다. 탭 비활성화/권한 철회 후에도 같은 확인을 한다.
7. 다중 모니터와 Spaces에서 기본 커서·오버레이 위치, 중복 커서 표시를 확인한다.
8. 기존 Mac `.license`가 있다면 캐시 재검증과 기기 ID 이전을 테스트한다.
   실제 테스트용 키로 활성화/권한 갱신, 네트워크 끊김도 확인한다.
9. 앱 번들의 서명/권한 상태를 확인하고 새 실행 파일에서 손쉬운 사용 권한을 다시
   요구하는 경우 그 결과를 기록한다.

### 남은 Mac 확인 사항

- CapsLock flagsChanged에서 플래그가 눌림 시 바뀌고 해제 시 유지되는지는 실기기별 확인이 필요하다.
- 손쉬운 사용 권한 철회와 입력 모니터링 권한 변경 시 자동 재연결 동작을 실기기에서 확인한다.
- 작업 스레드의 CGEventPost와 UI 메인 큐 피드백이 실제 macOS에서 안정적인지 확인한다.
- 기존 바탕화면 앱이 실행 중일 때 패키징하지 말고 종료 후 갱신한다.

## 2026-11-02 Windows UIAccess Candidate

- Windows-only overlay visibility and signed UIAccess packaging changes; do not port Win32 manifests, certificates or token checks to macOS.
- Preserve the same design principle on Mac: a suspended/hidden overlay must not be re-shown by the position-update path; keep native pointer fallback and avoid focus-stealing updates.
- No Mac source was modified or tested in this update. No shared keyboard mapping changes were made.
- Windows verification/deployment status is recorded in `DASHBOARD.md`.

## 남은 공통 과제

- 서버 서명 인증서, store/product/variant ID 확인, 구형 라이선스 instance ID 마이그레이션.
- 체험 파일 삭제/시계 변경에 대한 제품 정책 및 서버 연계 여부 결정.
- 네트워크 실패/제품별 라이선스 시나리오의 HTTP 모의 테스트 추가.
- Windows 훅 강제 해제의 상태 감지/재설치 및 설치 실패 주입 테스트.
- HUD의 활성화 키 안내가 사용자 설정을 실제로 표시하도록 연결.
- 문서의 과거 '100%', '0ms' 표현을 측정 근거와 구분하고 큰 UI 파일 분리.
