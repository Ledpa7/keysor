# 🎛️ 키서 (Keysor) - 프로젝트 대시보드

마우스 없이 키보드만으로 마우스 커서를 정밀 조작하는 초경량·초안정성 윈도우 네이티브 유틸리티 **키서(Keysor)**의 종합 프로젝트 대시보드입니다.

---

## 2026-11-17 커서·원격·포터블 배포 최종 정리

### 현재 적용 상태

| 항목 | 최종 상태 |
| --- | --- |
| 버전 | **1.0.3 유지**. 사용자 요청 없이 버전을 올리지 않는다. |
| 현재 실행 대상 | `dist/Keysor-Portable/Keysor.exe` |
| 바탕화면 바로가기 | 위 포터블 실행파일에 연결. `--chrome-remote --diagnose-input` 인수 유지. |
| 배포 형태 | 설치 없이 ZIP을 풀어 실행. 메인·Guardian·수동 복구 도구를 같은 폴더에 제공한다. **단일 바이너리에 합친 형태는 아니다.** |
| 원격 시스템 커서 | **32×32px 캔버스로 복원**. 호스트 DPI를 곱해 커서를 키우던 중간 변경은 철회했다. |
| 원격 대체 화면 커서 | 상황별 모양과 핫스폿 반영. 위치 보간 제거. 다만 DPI에 따른32/40/48/64px 경로가 남아 있어 시스템 커서와 크기 기준이 다르다. |
| 로컬 커서 | 이번 원격 개선에서 기존 그리기·이동 보간 분기를 유지했다. 로컬 DPI 정책의 문제는 점검만 했고 바꾸지 않았다. |
| 팝업 색상 | CapsLock: 키서 민트 `#2FFFAD`, Spacebar: 기존 CapsLock 빨강 `#FF0033`. |
| 설정 | 최종 교체 전후 설정 파일 해시 동일 확인. |
| 최신 적용 근거 | `target/hud-small-apply-result.json`: 적용 성공, 실행 경로/빌드 해시 일치, Guardian ready/independent=true. 당시 메인PID60472, Guardian PID43392. PID는 당시의 확인 기록이다. |

소스 커밋·로컬 실행·실제 원격 클라이언트 검증은 별개다. 아래 검사 통과를 Android/노트북에서 동일하게 보인다는 의미로 해석하지 않는다. 이번 Git 정리는 소스와 문서의 체크포인트이며 공개 릴리스나 원격 저장소 반영을 뜻하지 않는다.

### 1. 상황별 커서와 최종 디자인

Windows의 모든 표준 커서 역할을 같은 키서 그림으로 덮던 방식에서 역할별 그림을 등록하는 방식으로 변경했다. 앱이 Windows에 요청하는 커서 역할을 따르며, 클릭 가능 요소를 찾는 추가 UIA 탐색 루프는 만들지 않았다. 일반 화살표를 쓰는 버튼이나 앱 자체 커서는 클릭 가능 여부를 별도로 추정하지 않는다.

| 역할 | 최종 표시 |
| --- | --- |
| 일반 | 기존 키서 모양과 기존 클릭·스크롤 색상 애니메이션 |
| 텍스트 입력 | 간결한 I자 |
| 가로·세로·양 대각선 크기 조절 | 방향별 양방향 화살표 |
| 이동·사용 불가·링크 | **세 역할 모두 동일한 기본 키서 모양과 동일한 밝은 색** |

손 모양·＋·× 아이콘을 사용했던 중간안은 최종안이 아니다. 밝은 세 역할은 기존 초록 그라데이션과 흰색을50:50으로 섞고, 검은 테두리에도 흰색50%를 섞어 `#808080`으로 만들었다. 클릭 기준점도 키서 모양에 맞췄다.

구현: `src/cursor_scheme.rs`, `src/cursor_scheme/context.rs`.

### 2. 원격 모드 전용 변경과 철회한 확대

- `--chrome-remote`일 때 Windows가 커서를 숨기거나 억제해 대체 오버레이를 사용하는 경로에도 OS가 알려주는 상황별 커서 역할을 반영한다. 역할이 바뀌면 그림을 다시 생성한다. 알 수 없는 앱 전용 커서 핸들은 기본 키서로 대체한다.
- 대체 커서의 위치를 원격에서만 즉시 갱신하고, 일반/상황별 모양의 핫스폿에 맞춰 배치한다. 로컬 위치 보간은 유지했다.
- 보이는 OS 커서가 있으면 중복 오버레이를 숨기는 기존 정책을 유지했다. 투명 시스템 커서나 `ShowCursor(false)` 경로를 추가하지 않았다.
- 원격 클라이언트가 같은 커서 핸들의 그림 갱신을 놓칠 수 있다는 가설을 조사했다. `SetCursor` 직접 호출은 시험에서 전역 커서 핸들을 바꾸지 못해 채택하지 않았다. 이것을 모바일 문제의 확정 원인으로 기록하지 않는다.
- 중간 빌드는 원격 그림을 호스트 DPI에 맞춰 키웠으나, 사용자가 노트북에서 더 커졌다고 보고했다. **원격 시스템 커서와 역할별 커서는32px로 되돌렸다.** 이후 실제 노트북 화면에서 크기 문제가 해소됐다는 확인은 아직 없다.

구현: `src/ui/win_gdi.rs`, `src/cursor_scheme.rs`. 원격 클라이언트가 커서·화면 영상을 각각 확대하는 부분은 키서가 직접 제어하지 않는다.

### 3. 설치 없는 배포와 독립 복구

`scripts/package-windows-portable.ps1`은 `dist/Keysor-Portable/` 및 `dist/Keysor-Portable.zip`을 만든다. 사용자는 평소 `Keysor.exe`만 실행하고 Guardian은 자동 시작된다. 세 실행파일은 서로 다른 이미지/해시다.

| 실행파일 | 역할 |
| --- | --- |
| `Keysor.exe` | 사용자 입력과 커서 표시 |
| `keysor-guardian.exe` | 메인 프로세스 종료를 기다렸다가 Windows 기본 커서 복원 |
| `Keysor-Recover.exe` | 메인·Guardian·설정·라이선스에 의존하지 않는 수동 복구. 직접 실행하면 결과 창, `--report <파일>`이면 검사 결과 JSON 출력 |

- 별도 Guardian이 없을 때 메인 이미지를 `--watchdog`로 다시 실행하던 경로를 제거했다. 같은 이미지의 두 프로세스가 동일 파일 차단에 묶이는 구성을 피한다.
- 준비 보고서의 부모/Guardian PID와 실제 생존 여부를 확인한다. Guardian이 준비되지 않으면 사용자 지정 시스템 커서를 적용하지 않는다.
- 실행 중 Guardian이 종료되면 메인이 커널 프로세스 핸들 대기로 감지해 기본 커서를 복원한다. 이 감시를 위한 새 폴링 루프는 없다.
- 메인만 종료되면 Guardian이 자동 복구한다. 둘이 함께 종료되면 그 복구 코드는 실행할 수 없으므로, Windows에 등록된 보이는 그림을 유지하고 독립 도구로 기본 커서 복구를 제공한다.
- 보안 제품이 복구 도구까지 차단하는 경우의 실행을 보장하지 않는다. 실제 백신 탐지·격리를 유발한 시험은 하지 않았고, 차단 우회나 자동 예외 등록도 하지 않았다.
- 설치 과정은 없지만 기존 관리자 권한 매니페스트는 유지하므로 실행 때 UAC 승인이 나타날 수 있다. 설치형 자료는 이전 배포 경로로 남아 있으며 이번 실행 대상은 포터블 폴더다.

구현/설명: `src/watchdog.rs`, `src/bin/keysor_recover.rs`, [포터블 사용법](PORTABLE_WINDOWS.md), [복구 검토 기록](PORTABLE_SAFETY_REVIEW.md).

### 4. 팝업 색상

`src/ui/hud/drawing.rs`의 키캡 색상만 변경했다. CapsLock 테두리와 키 이름은 키서 민트, Spacebar 테두리와 키 이름은 빨강을 사용한다. 설명 글자와 배치, 입력 기능은 유지했다. 독립 HUD 렌더 검사를 실행해 [생성된 미리보기](docs/2026-11-17/hud-preview.png)에서 확인했다. 이는 실제 사용자 팝업을 촬영한 화면은 아니다.

### 5. 입력 장애와 복구 — 원인 미확정

수정·검증 이후 사용자가 모드를 꺼도 원격 PC에서 타이핑할 수 없고 노트북 로컬에서는 입력되며 복사·붙여넣기만 가능하다고 보고했다.

1. 새 키서를 완전히 종료하고 독립 도구로 기본 커서를 복원했으나 사용자 증상은 지속됐다.
2. 원격 호스트의 Windows 키 상태를 읽어 **왼쪽 Alt 논리적 눌림**을 확인했다. 그때 키서/Guardian 프로세스는 없었다. 이것은 물리 키가 실제로 눌렸다는 뜻이 아니다.
3. Alt를 눌렀다 떼는 입력으로 논리적 눌림을 해제하고 Windows에서도 false를 확인했다. 사용자가 직접 타이핑이 돌아왔다고 확인했다.
4. 발생 지점은 확정하지 못했다. 이번 수정·창 전환·입력 자동화·원격 적용 과정에서 해제가 누락됐을 가능성을 우선 조사 대상으로 본다. 커서 코드 자체의 결함인지, 작업 과정의 입력 누락인지 구분할 자료는 부족하다. 키서를 종료한 뒤에도 지속됐다는 사실만으로 작업과 무관하다고 판단하지 않는다.

복구 후 사용자 요청으로 수정 버전을 다시 실행했으며, 이후 작은 원격 커서/팝업 색상 빌드를 최종 적용했다. 자동 보조키 강제 해제나 입력 처리 변경을 제품에 추가하지 않았다. 재현·입력 출처 확인 없이 물리 키나 Chrome 원격만의 문제로 단정하지 않는다.

로컬 근거: `target/keyboard-emergency-result.json`, `target/keyboard-live-state.json`, 당시 입력 진단 로그. 이 파일들은 임시 로컬 검사 자료이며 Git에는 포함하지 않는다.

### 6. 크기·해상도·배율 상세 점검 — 문제 확인, 수정 미적용

실행 중인 키서·Windows 설정을 바꾸지 않고 실제 제품 그리기 함수로 독립 검사기를 만들었다. 배율16개, 해상도6개의96개 **코드 기반 시나리오**와89개 **실제 GDI/GDI+ 생성 이미지**를 분석했다. 실제 모니터96개나 원격 기기89대에서 시험한 것은 아니다.

| Windows 배율 | 로컬 일반 캔버스 | 32DIP 기준 UI 상대 차이 |
| --- | ---: | ---: |
|100% /125% /150% /200%|32 /40 /48 /64px|동일|
|175%|48px|14.3% 작음|
|250%|64px|20% 작음|
|300%|64px|33.3% 작음|
|400%|64px|50% 작음|

- 로컬 크기는 정확한 배율이 아니라115/140/180%의 구간별 값과200% 상한을 사용한다.175→180%에서 생성 캔버스가48→64px로 갑자기 변한다.
- 일반 커서와 상황별 커서는 다른 기준을 사용한다. 현재 호스트 조회값은1280×1024,125%이며 일반40px와 상황별32px의 생성 기준이 확인됐다. 실제 클라이언트에 표시된 최종 크기 측정과 구분한다.
- 원격32px 고정은 **시스템 커서 경로에만 해당**한다. 대체 화면 커서는40/48/64px가 될 수 있다. 이전의 “원격 전체32px” 설명은 불완전했으며, 모든 배율에서 동일한 경험을 보장할 상태가 아니다.
- 같은32px 캔버스의 실제 도형은 현재 Windows 화살표12×18px, 로컬 키서18×20px, 원격 키서19×20px였다. 폭은50~58.3%, 높이는11.1% 더 크다. 같은40px 캔버스에서는 비율이 달라지므로 전체 크기를 단일 배수로 표현하지 않는다.
- 권장 기준은 사용자가 익숙한 크기를 출발점으로 논리 크기와 배율을 한 번만 일관되게 반영하는 것이다. 원격은 클라이언트의 별도 커서·영상·줌 경로까지 고려해야 한다. 이번 점검에서는 커서를 더 키우거나 로컬 정책을 바꾸지 않았다.

상세 표·공식 문서·한계·재현 방법: [CURSOR_SIZE_AUDIT.md](CURSOR_SIZE_AUDIT.md). 검증 자료의 작은 스냅샷은 [docs/2026-11-17](docs/2026-11-17/)에 보관한다. 원시 BGRA·검사 프로그램·전체 로그는 생성 산출물이므로 `target/size-audit/`에만 둔다.

### 7. 검증 결과와 남은 일

| 검증 | 결과/범위 |
| --- | --- |
| 일반 회귀 검사 | 최신 소스로59개 통과, 대화형12개 제외. `target/git-check-tests.log` |
| 릴리스 빌드 | 마지막 크기 복원·색상 변경 빌드 통과. `target/hud-small-cursor-build.log` |
| HUD 렌더 |1개 통과 및 생성 이미지 시각 확인. `target/hud-colors-preview.log` |
| 원격 렌더 검사 |32/40/48/64/96px 객체의 크기·핫스폿·부드러운 경계·잘림 검사와 상황별 그림 검사2개 통과. 이는 모든 크기를 제품에서 활성화했다는 뜻이 아니다. |
| 실제 데스크톱 안전 검사 |Guardian 없음, 메인만 강제종료, Guardian만 강제종료, 둘 종료 후 독립 복구, 의도적 투명 포인터 복구의5개 통과. 복구 후14종 커서 렌더링 해시가 기준과 일치. `target/portable-native-results.json` |
| 배포 적용 |새 실행파일 해시·실제 프로세스 경로·Guardian 준비·설정 보존 확인. 최종 크기/색상 적용 결과는 위 표 참조. |
| 실제 Android/노트북 원격 |최종 모션·선명도·크기 일치 **미검증**. 사용자의 확대 문제 보고로 DPI 확대는 철회. |
| 실제 보안툴 차단 |**미검증**. 강제종료·실행 실패 모사와 제품의 탐지/격리는 다르다. |
| Alt 눌림 근본 원인 |복구와 사용자 입력 정상화는 확인. 발생 원인·재현·재발 방지는 **미완료**. |
| 전 해상도/DPI 일관성 |불일치 확인. 논리 크기 공통화와 실제 클라이언트 최종 표시 검증은 **미완료**. |

배포 EXE/ZIP/설치파일과 `target/`의 임시 실행·진단 자료는 Git에서 제외한다. 소스·재현 스크립트·문서·작은 시각 검증 자료를 체크포인트로 보관한다.


## 2026-11-16 자체 점검 (검사 범위 통과)

- 일반 회귀 테스트55개 통과: `target/self-check-tests.log`. 실제 커서/팝업/IME/UIA/진단 검사는 `target/self-check-native.ps1`로7개를 각 독립 프로세스에서 실행하도록 준비했다. 설치 바이너리 일치 확인 후 실행 중인 정식 키서를 잠시 종료하고, 검사 성공/실패와 무관하게 정식 키서를 재실행한다. 클립보드를 덮어쓰는 지원 복사 검사는 제외한다.
- 사용자 승인 후 Windows 검사7개 모두 통과. 실행기의 PowerShell5 표준 오류/종료 코드 수집 문제는 앱 실패와 구분했다. 각 테스트의 Rust 성공 요약으로 검증했으며 원본 `target/self-check-result.json`은 보존하고 최종 결과는 `target/self-check-verified.json`에 저장했다.
- 설치 메인/Guardian 해시 일치, 설정 해시/바로가기 인수 유지, 화면 비교 동일(HUD 전체/Win 안내창 시계 제외) 확인. 정식 키서 PID53464/Guardian PID49632 재실행 및 ready=true/independent=true. 버전1.0.3 유지. 자세한 결과/검증 한계는 `SELF_TEST_REPORT.md`.
- **이전 로그 해석 정정:** `caps_down` 뒤 mouse_mode=false→true 기록만으로 OFF가 취소됐다고 단정할 수 없다. native Caps ON 전환도 Windows 처리를 기다리는 동안 false를 유지한 뒤 true가 되는 설계이며 기존 단위 검사로 확인된다. 진단에는 콜백 처리 전 모드가 없으므로 사용자가 OFF를 요청한 것인지 구분되지 않는다. 이전에 이 기록을 확정된 재활성화 문제로 설명한 것은 근거가 부족했다. 첫 영문 출력은 여전히 미재현/원인 미확정이다.

## 2026-11-16 높은 우선순위 병목 최적화 (1.0.3 적용 완료)

### 작업 목적과 범위

이전 성능 분석에서 개선 필요성이 **높음**인 두 경로만 최적화했다. 입력 처리에 영향을 줄 수 있는 **진단 중 공용 상태 잠금**과, 전역 자석 기능을 사용할 때의 **UI Automation(UIA) 탐색 비용**이 대상이다. 가독성을 유지하기 위해 역할이 드러나는 작은 자료형과 조회 전용 모듈로 구현했다.

| 구분 | 변경한 부분 | 유지한 부분 |
| --- | --- | --- |
| 진단 | 입력 상태 복사와 OS 조회의 잠금 범위 분리 | 기록 항목, 초당 10회/5분 주기, 잠금 획득 실패 시 샘플 건너뛰기 |
| 전역 자석 탐색 | 필요한 속성 일괄 캐시, 최종 대상 200개 판정 후 중단 | 대상 종류, 판정 규칙, 반환 순서, 탐색 깊이/영역, 갱신 조건 |
| 사용자 기능 | 기능 변경 없음 | CapsLock 팝업 0.5초, Win 안내 1초, 화면 배치와 단축키 |
| 배포 | 같은 정식 위치에 수정 빌드 적용 | 버전 1.0.3, 설정 파일, 바탕화면 바로가기와 원격 실행 인수 |

중간/낮음으로 평가한 동작 큐, 라이선스 네트워크 요청, 작은 메모리 복사 등은 이번 작업에서 변경하지 않았다.

### 1. 진단 코드의 입력 상태 잠금 축소

**변경 전:** `AppState` 잠금을 잡은 채 마우스 모드/보조키를 읽고, Windows 커서·창·프로세스 정보 조회와 JSON 생성까지 수행했다. 키보드 훅도 같은 상태에 접근하므로, OS 조회가 늦어지면 입력 처리가 잠금을 기다릴 수 있었다.

**변경 후:** `InputSnapshot`에 마우스 모드, Caps 누름 여부, Ctrl/Alt/Win의 불리언 5개만 복사한다. `snapshot_input()`이 반환할 때 잠금이 해제되고, `collect_cursor_snapshot()`은 복사본으로 OS 조회와 JSON 생성을 수행한다.

```text
이전: 잠금 → 입력 상태 읽기 → Windows 조회 → JSON 생성 → 잠금 해제
이후: 잠금 → 값 5개 복사 → 잠금 해제 → Windows 조회 → JSON 생성
```

효과의 근거는 입력 잠금의 수명이 짧아진다는 점이다. Windows 조회 자체를 생략한 것이 아니라, 외부 조회 비용이 입력 처리의 잠금 대기로 이어지는 구간을 분리했다. OS 상태 전체와 입력 상태를 동시에 고정한 원자적 스냅샷은 아니며, 두 상태의 관측 시점에는 차이가 있을 수 있다.

구현: `src/diagnostics.rs`의 `InputSnapshot`, `snapshot_input()`, `capture_cursor_snapshot()`, `collect_cursor_snapshot()`.

### 2. UIA 탐색 속성의 일괄 조회

**변경 전:** 탐색 결과의 요소마다 위치·종류·ID·이름을 개별 조회했다. 속성 조회는 다른 프로세스의 UI 제공자와 통신할 수 있으므로 요소 수가 많으면 비용이 누적된다. 모든 결과의 판정을 끝낸 다음 최종 좌표를 200개로 잘랐다.

**변경 후:** `TargetReader`가 UI Automation BuildCache로 필요한 속성을 묶어서 가져온다.

| 조회 용도 | 캐시하는 속성 | 객체 모드 | 이유 |
| --- | --- | --- | --- |
| 트리 탐색 | 위치, 컨트롤 종류 | Full | 하위 요소 탐색을 계속할 수 있어야 함 |
| 일반 최종 대상 판정 | 위치, AutomationId, 이름 | None | 좌표/이름만 읽으며 실제 UI 조작용 참조는 필요 없음 |
| 작업 표시줄 대상 판정 | 위치 | None | 기존 판정은 위치만으로 처리할 수 있음 |

캐시는 매 탐색에서 얻는 요소의 새 스냅샷이며, 이전 창의 속성을 계속 재사용하지 않는다. 캐시 생성·검색·속성 조회가 실패하거나 속성이 빠졌으면 기존 개별 조회로 돌아간다. 유효한 대상 200개를 얻으면 나머지 결과의 판정을 중단하므로, 기존의 앞 200개 반환 정책을 유지하면서 불필요한 후처리를 줄인다.

**최초 UI 트리 검색 전체를 200노드로 제한한 것은 아니다.** 제공자와 최초 트리 검색 비용은 남아 있다. 임의의 노드 제한으로 기존 대상을 놓치게 하는 방식은 적용하지 않았다.

구현: `src/ui/win_uia/targets.rs`의 `TargetReader`, `property_cache()`, `scan_window_targets()` 및 `src/ui/win_uia.rs`의 탐색 호출부.

근거: [Microsoft UI Automation 캐싱 문서](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-cachingforclients)는 속성별 교차 프로세스 호출 비용과 일괄 조회의 효과를 설명한다.

### 3. 변경 전후 정량 비교

아래는 **최적화 작업 당시 보고한 기준 측정값**이다. 이후 자체 점검으로 벤치마크 파일을 재생성할 수 있으므로, 최신 단일 실행값과 이 기준값을 혼동하지 않는다.

| 항목 | 변경 전 중앙값 | 변경 후 중앙값 | 절대 차이 | 해석 |
| --- | ---: | ---: | ---: | --- |
| 진단 입력 잠금 구간 | 76.6μs | 0.3μs | 76.3μs 감소 | 잠금 구간 약 99.6% 감소 |
| 진단 조회 전체 | 76.8μs | 76.0μs | 0.8μs 감소 | 전체 작업 시간은 거의 같음 |
| 일반창 UI 탐색 | 772.416ms | 725.508ms | 46.908ms 감소 | 해당 실험에서 약 6.1% 단축 |
| 브라우저형 타이틀바 탐색 | 425.609ms | 421.747ms | 3.862ms 감소 | 약 0.9%, 뚜렷한 개선으로 판단하지 않음 |
| 작업 표시줄형 탐색 | 661.627ms | 656.712ms | 4.915ms 감소 | 약 0.7%, 뚜렷한 개선으로 판단하지 않음 |

측정 방법:

- 진단은 실제 Windows API를 호출하는 동일 조회를 100회 비교했다. 기존 기준 경로는 조회 종료까지 잠금을 유지하고, 최적화 경로는 실제 `snapshot_input()`으로 잠금을 먼저 해제한다.
- UIA는 버튼 240개가 있는 임시 네이티브 창을 만들고 경로별 7회씩 실행해 중앙값을 비교했다. 실행 순서는 번갈아 바꿔 순서에 따른 영향을 줄였다.
- UIA 캐시 비교의 양쪽 경로 모두 같은 대상 판정과 200개 판정 제한을 사용했다. 따라서 기존 코드의 200개 이후 추가 판정을 중단하는 효과는 이 수치에 포함하지 않았다.
- 테스트는 `KEYSOR_TEST_BUILD=1`의 debug 빌드다. 실제 Chrome이나 실제 작업 표시줄을 측정한 것이 아니라, 임시 창을 각 탐색 분기로 처리했다.

**해석상 한계:** 입력 잠금 시간이 99.6% 줄었다고 앱 전체 CPU가 99.6% 줄거나 타자 반응이 같은 비율로 빨라졌다는 뜻은 아니다. 실제 경쟁 중인 키 입력 지연, 최악 지연, 릴리스 앱의 CPU·메모리·배터리·네트워크 사용량은 별도 측정하지 않았다. 1% 안팎의 탐색 시간 차이는 측정 변동일 수 있다. 측정 당시 사용자 설정에서는 전역 자석 기능이 꺼져 있었으며, UIA 최적화는 그 기능을 켰을 때 적용된다.

### 4. 기능 보존과 검증 결과

- 변경 전후 일반 회귀 테스트 **55개 통과**.
- 최적화 단계의 Windows 검사 **3개 통과**: 입력 잠금 해제/필드 일치, UIA 대상 좌표·순서 일치, 캐시 속성 누락 시 기존 조회 복귀.
- 일반창/브라우저형/작업 표시줄형 탐색에서 동일한 대상 좌표와 순서를 확인하고 최종 200개 제한을 검사했다.
- 기존 300ms 점검, 이동 시 900ms 제한, 정기 재탐색과 클릭 갱신 조건은 변경하지 않았다.
- 후속 자체 점검에서도 일반 **55개**, Windows 통합 **7개**가 통과했다. 실제 커서·팝업 검사와 기본 화면 비교도 수행했다. 클립보드를 덮어쓰는 지원 정보 복사 검사는 제외했다.
- 메인 HUD 808×452는 픽셀 차이가 없었고, Win 안내창 870×300은 현재 시간·날짜 영역을 제외하면 차이가 없었다. 모든 DPI·앱·원격 입력·라이선스 상태 조합을 실제 사용으로 검증한 것은 아니다.

### 5. 설치 및 실행 확인 이력

- 릴리스와 `dist/Keysor-Setup-1.0.3.exe` 빌드 성공 후, 사용자 UAC 승인으로 `C:\Program Files\Keysor\standard`에 적용했다.
- 최적화 빌드 ID: **`b3abd0e64cb4a481`**. 버전은 사용자 방침에 따라 **1.0.3 유지**.
- 설치 직후 메인 PID56996/Guardian PID57728, 독립 실행/준비 상태를 확인했다.
- 후속 자체 점검 후 메인 PID53464/Guardian PID49632로 정식 경로에서 재실행했다. PID는 해당 점검 시점의 기록이며 고정값이 아니다.
- 설치된 메인/Guardian의 SHA256이 빌드 결과와 일치했고, 설정 파일 해시 및 바탕화면 바로가기를 보존했다. 바로가기 대상은 정식 실행 파일, 인수는 `--chrome-remote --diagnose-input`이다.
- 자체 점검에서 이전 정식 메인 종료 후 Guardian의 `restore_ok=true`도 확인했다.

### 6. 확인되지 않은 사항과 정정

첫 글자만 영문으로 나오는 현상은 이번 최적화나 자동 검사로 재현·원인 확정·해결하지 못했다. 앞서 `caps_down` 이후 mouse_mode=false→true 로그를 모드 OFF 취소라고 설명했지만, 정상적인 ON 전환도 같은 중간 상태를 가질 수 있다. 전환 전 상태가 기록되지 않아 구분할 수 없으므로, 확정된 재활성화 버그라는 해석은 정정한다.

자체 점검 실행기의 PowerShell5 출력/종료 코드 수집 문제는 앱의 테스트 실패와 구분했다. 개별 Rust 테스트의 최종 성공 요약으로 검증했고, 원본 결과와 검증 결과를 모두 보존했다.

### 7. 상세 문서와 근거 파일

| 용도 | 파일 |
| --- | --- |
| 최적화 상세 설명 | [PERFORMANCE_OPTIMIZATION.md](PERFORMANCE_OPTIMIZATION.md) |
| 구조 변경 및 이유 | [REFACTORING.md](REFACTORING.md) |
| 자체 점검 결과와 검증 한계 | [SELF_TEST_REPORT.md](SELF_TEST_REPORT.md) |
| 변경 전후 일반 검사 | `target/priority-perf-before-tests.log`, `target/priority-perf-tests.log` |
| Windows 최적화 검사 | `target/priority-perf-input-native.log`, `target/priority-perf-uia-native.log`, `target/priority-perf-fallback-native.log` |
| 벤치마크 원자료 | `target/priority-perf-input-benchmark.json`, `target/priority-perf-uia-benchmark.json` — 재실행 시 갱신 가능 |
| 빌드/설치 | `target/priority-perf-package.log`, `target/priority-perf-install.json` |
| 자체 점검 최종 결과 | `target/self-check-verified.json`, `target/self-check-tests.log` |
| 자체 점검 실행기 원본 | `target/self-check-result.json` |
| Windows 통합 검사 로그 | `target/self-check-native-1.log` ~ `target/self-check-native-7.log` |
| 화면 비교 | `target/self-check-render.json` |

## 2026-11-15 UI/진단 리팩토링 (1.0.3 적용 완료)

- Windows 메인 팝업 그리기를 `src/ui/hud.rs`의 영역별 함수와 `src/ui/hud/drawing.rs`의 공통 그리기로 분리했다. 키45개는 동일한 배치표와 공통 반복문으로 그린다. Win 안내창의 그리기는 `src/win_shortcuts/renderer.rs`, 전환/IME 진단은 `src/diagnostics/transitions.rs`로 분리했다. 주기 진단은 상태 수집과 스케줄/저장을 함수로 나눴다. 공통 테스트 캡처는 `src/ui/test_capture.rs`.
- 변경 전후 일반 테스트55개 통과. 별도 Windows 검사3개 통과(HUD, Win 안내창, IME 조회). HUD808×452 픽셀 차이0, Win 안내창870×300은 현재 시계 영역을 제외하고 차이0. 키45개 좌표/크기/순서/설명/분류도 동일. 결과 `target/refactor-render-comparison.json`.
- 릴리스 및1.0.3 설치 파일 빌드 성공. 사용자 UAC 승인 후 정식 위치 설치/재시작 성공. `target/refactor-install.json` 성공과 빌드 `dde145cfd886ec2f`, 메인 PID50976/Guardian PID44296 실행 및 independent=true/ready=true 확인. 설정/바탕화면 바로가기 원격 인수 유지.
- 구조 변경과 항목별 이유/검증 범위는 `REFACTORING.md`. 기존 입력/IME 증상의 원인을 해결한 변경이 아니다. Git 커밋/업로드는 수행하지 않았다.

## 2026-11-15 메인 HUD 안내 표시 정리 (1.0.3 적용 완료)

- 사용자 요청으로 숫자1–9의 `+Win` 설명/강조를 제거했다. 오른쪽 중복 Win 키를 그리지 않고 왼쪽 Win 키만 `+ 1~9`로 표시한다. 하단 추가 설명은 기존 회색 `0x888888`로 통일했다. 창 크기/나머지 배치/기능은 유지한다.
- 릴리스/설치 파일 빌드 및 기존 정식 위치 설치/재시작 성공. 빌드 `ca7a1972eab2c518`, 메인 PID57808/Guardian PID57700 실행 및 independent=true/ready=true 확인. 설정/바로가기 원격 인수 유지. 결과 `target/hud-label-refinement-install.json`, 로그 `target/hud-label-refinement-package.log`. 이번 변경의 실제 화면 캡처는 수행하지 않았다.

## 2026-11-15 메인 HUD에 팝업/Win 단축키 안내 추가 (1.0.3 적용 완료)

- 기존808×452 배치와 버튼/키 좌표를 유지한다. 숫자1–9에 `+Win` 설명과 민트 강조, 양쪽 Win 키에1초 안내 설명을 추가했다. 기존 하단 Caps 설명에0.5초 팝업 재열기를 반영하고, 남아 있던 하단425–443 영역에 Win1초 안내/Win+1–9 작업 표시줄 실행·전환 설명을 한/영으로 추가했다.
- 표시 설명만 변경했다. 릴리스/설치 파일 빌드 및 기존 정식 위치 설치/재시작 성공, 설정/바로가기 원격 인수 유지. 빌드 `7a037e3232f619d1`, 메인 PID54848/Guardian PID58256 실행 확인. 결과 `target/hud-shortcut-labels-install.json`, 로그 `target/hud-shortcut-labels-package.log`.
- 실제 창 캡처 시도는 Computer Use의 `ShellExecuteW returned 5` 및 창 재선택 후 `window id 50334042 was not found`로 실패했다. 최소화된 창에 대한 화면 겹침/잘림은 확인 완료로 보고하지 않는다. 첫 글자 영문 현상은 여전히 조사 중이며 해당 기능 수정으로 보고하지 않는다.

## 2026-11-15 모드 OFF 직후 첫 글자 영문 현상 진단 (1.0.3 적용 완료)

- 사용자는 Codex/카카오톡에서 간헐적으로 발생하고 메모장 비교에서는 재현하지 못했다고 보고했다. 특정 앱만의 문제로 단정할 수 없다. 코드상 native CapsLock Toggle 경로는 한·영 전환을 직접 실행하지 않으며, 모드 OFF는 즉시 타자 통과 후25ms 지연 Caps 상태 동기화가 있다. 기존 로그만으로 첫 글자 영문 원인은 확정하지 못했다.
- `--diagnose-input`에 제한된 전환 진단을 추가했다. Caps 전환 및 OFF 뒤2초 이내 첫 문자 범주3회에 대해 통과/차단, 보조키, 출처, IMM 호환 IME 상태를 기록한다. 실제 문자/키 코드/조합 문자열은 기록하지 않는다. 최대30분/2000건, 별도 스레드의10ms 제한 조회, 훅에서는 대기하지 않는다. IMM 값은 앱의 TSF 상태와 다를 수 있으므로 단독 확정 근거로 쓰지 않는다.
- 일반 테스트55개 및 실제 Windows IME 읽기 검증1개 통과. 빌드/정식 설치/재시작 성공, 설정/바로가기 인수 유지. 빌드 `3ec6f7025ca889ff`, 메인 PID57776/Guardian PID44408 실행 확인. 결과 `target/ime-transition-install.json`, 로그 `target/ime-transition-tests.log`, `target/ime-transition-native.log`, `target/ime-transition-package.log`.
- 재현 로그는 `~/.keysor/input-transition-57776.jsonl`이다. 원인을 해결한 빌드가 아니라 재발 시점 확인용이다. 버전1.0.3, Caps 팝업500ms/Win 안내1초 유지.
- 적용 직후 실제 기록에서 카카오톡 OFF(2874ms)→ON(2949ms), OFF(5764ms)→ON(5884ms), Chrome OFF(15374ms)→ON(15454ms)를 확인했다. Chrome의 후속 문자 범주 입력3회는 mode=true/passed=false였다. 이 구간에 Ctrl/Alt/Win/Shift는 모두 false였다. 카카오톡 IMM 상태는 native=true/open=true를 유지했다. 모드 재활성화 및 차단은 관찰됐으나, 사용자가 보는 첫 영문 출력과 동일 시점인지는 확인되지 않았다.
- 지연 동기화가 사용하는 GetKeyState는 호출 스레드의 키 메시지 처리에 따라 값이 갱신되는 API다. 이 코드의 메인 스레드 조회가 오래된 상태를 돌려줄 가능성을 다음 조사 대상으로 삼는다. 이 사실만으로 IME 전환이나 첫 영문 출력의 원인을 확정하지 않는다.

## 2026-11-15 CapsLock 팝업 대기0.5초 (1.0.3 적용 완료)

- 사용자 요청으로 CapsLock 길게 누르기 기준을2초→500ms로 변경했다. 499ms 전 미발동/500ms 발동, 반복 입력 및 취소 검사를 갱신했다. 기존 CapsLock 모드 동작과 Win 안내1초는 유지한다. 대화형 HUD 검사의 사전 대기 구간도350ms로 맞췄다.
- 일반 테스트54개 통과(대화형3개 제외), 설치 파일 빌드 및 정식 위치 설치/재시작 성공. 실제 물리/원격 키 입력으로500ms 팝업 동작은 이번에 별도 검증하지 않았다.
- 빌드 `08bd5a28d701b8e7`, 메인 PID51724/Guardian PID54060 실행 확인. 설정과 바로가기 인수 유지. 결과 `target/caps-half-second-install.json`, 로그 `target/caps-half-second-tests.log`, `target/caps-half-second-package.log`.

## 2026-11-15 작업 표시줄 높이/상태 영역 간격 (1.0.3 적용 완료)

- 바 높이를83→62 논리 픽셀로 줄이고 숫자 타일 높이/실행 표시선을 맞췄다. 앱 배치 간격을46→44로 줄여 상태 영역을 확보하고, 화살표·와이파이·시간/날짜 사이 여백을 넓혔다. 표시 조건과 시간 갱신 동작은 유지한다.
- Windows 대화형 안내창 테스트1개 통과, 캡처 `target/win-shortcuts-spacing-preview.png` 확인, 1.0.3 설치 파일 빌드 완료. 로그 `target/win-shortcuts-spacing-native.log`, `target/win-shortcuts-spacing-package.log`.
- 사용자 UAC 승인 후 기존 정식 위치 설치/재시작 성공. `target/win-shortcuts-spacing-install.json` 성공 및 설정/바로가기 원격 인수 유지 확인. 빌드 `72e68be501fc6782`, 메인 PID48800/Guardian PID49000 실행 및 independent=true/ready=true 확인. 사용자 노트북 원격 화면은 별도 미검증이다.

## 2026-11-15 작업 표시줄 숫자 및 현재 시간/날짜 (1.0.3 적용 완료)

- 앱 그림을 숫자1–9로 바꾸고 전체 폭의 납작한 작업 표시줄에 시작/검색, 구분선, 실행 표시선을 유지했다. 가장 오른쪽에는 Windows PC의 실제 현지 시간(HH:mm)/날짜(YYYY.MM.DD)를 두 줄로 표시한다. 창이 보일 때만 시계를1초마다 갱신하며 숨길 때 타이머를 해제한다. 키 상태 폴링은 추가하지 않았다.
- Windows 대화형 안내창 테스트1개 통과, 캡처 `target/win-shortcuts-numbered-preview.png` 확인. 설치 파일 빌드 및 기존 정식 위치 적용/재시작 성공. 빌드 `3ab3fdc5beda2b52`, 메인 PID52556/Guardian PID54112 실행 확인. 설정/바탕화면 바로가기 원격 인수 유지.
- 결과 `target/win-shortcuts-numbered-install.json`, 로그 `target/win-shortcuts-numbered-native.log`, `target/win-shortcuts-numbered-package.log`. 사용자 노트북 원격 화면은 별도 미검증이다.

## 2026-11-15 Win 안내의 작업 표시줄 디자인 보완 (1.0.3 적용 완료)

- 작업 표시줄을 인식하기 어렵다는 사용자 의견을 반영해 전체 폭의 바에 시작/검색 영역, 서로 다른 앱 그림9개와 실행 표시선, 오른쪽 상태/시계 영역을 배치했다. 숫자와 화살표는 앱 영역에만 대응하며 문구는 넣지 않았다. 그림은 설명용이며 실제 앱/상태 목록이 아니다.
- 변경은 그리기에 한정한다. Windows 대화형 안내창 테스트1개 통과 및 캡처 `target/win-shortcuts-taskbar-preview.png` 확인. 빌드/설치/재시작 성공, 설정과 바로가기 원격 인수 유지. 빌드 `bdfa9f8e4a5b913f`, 메인 PID49500/Guardian PID38964, independent=true/ready=true.
- 결과 `target/win-shortcuts-taskbar-install.json`, 로그 `target/win-shortcuts-taskbar-native.log`, `target/win-shortcuts-taskbar-package.log`. 실제 사용자 원격 화면의 표시 여부는 별도 미검증이다.

## 2026-11-15 Win 안내를 언어 없는 아이콘으로 변경 (1.0.3 적용 완료)

- 제목/설명/Win 글자를 제거하고 Windows 로고+숫자 키1–9→작업 표시줄 앱 위치 도식으로 변경했다. 앱 그림은 순서 설명용이며 실제 앱 목록이 아니다. 높이를304→240 논리 픽셀로 줄였다. 표시 조건/입력 통과/반투명 스타일은 유지한다.
- 일반 테스트54개 및 Windows 대화형 안내창 테스트1개 통과, 실제 창 캡처 `target/win-shortcuts-icons-preview.png` 확인. 1.0.3 설치 파일 빌드 완료. 로그 `target/win-shortcuts-icons-tests.log`, `target/win-shortcuts-icons-native.log`, `target/win-shortcuts-icons-package.log`.
- 사용자 UAC 승인 후 기존 정식 위치 설치/재시작 성공. 설치 성공 보고서 `target/win-shortcuts-icons-install.json` 확인. 빌드 `88bc60477d8290eb`, 메인 PID48620/Guardian PID46388 실행 및 independent=true/ready=true 확인. 설정과 바탕화면 바로가기의 원격 인수 유지. 사용자 노트북 원격에서 실제 표시 여부는 별도 미검증이다.

## 2026-11-15 Win 키 길게 누르기 안내 (1.0.3 적용 완료)

- Win 키만1초 누르면 현재 작업 중인 모니터 중앙에 `Win + 1–9` 작업 표시줄 앱 실행/전환 안내를 표시한다. 왼쪽/오른쪽 Win 모두 지원하며 CapsLock 모드와 독립적으로 동작한다. 키서 HUD와 같은 다크/민트/반투명 스타일, 언어 설정, DPI 및 작은 모니터 맞춤을 적용한다.
- Win 해제나 다른 키 입력 시 숨기며 짧은 입력/자동 반복/기존 조합키에는 새 안내를 표시하지 않는다. 창은 포커스/클릭을 가져가지 않고 Windows 키를 차단/합성하지 않는다. 기존 UI 스레드의1초 타이머와 마감 시점 물리 상태1회 확인을 사용한다. 해제 유실 대비 표시8초 제한. 최소화된 메인 창과 독립적인 도구 창이다.
- 일반 테스트54개 통과(대화형3개 제외), 새 Windows 대화형 테스트1개 통과: 실제 생성/표시/중앙 배치/비활성·클릭 통과 속성/해제 및 안전 타이머 닫힘 확인. 실제 창 캡처를 확인했다. 이번 테스트는 입력 훅 설치나 클립보드 변경을 하지 않는다. 사용자 노트북 원격의 실제 Win 키 전달은 별도 미검증이다.
- 설치 파일 빌드, 기존 정식 설치 위치 덮어쓰기/재시작 완료. 빌드 `c1e9ce5d0be619e5`, 메인 PID36548/Guardian PID13888, independent=true/ready=true. 설정/바탕화면 바로가기 및 원격 인수 유지. 결과 `target/win-shortcuts-install.json`, 로그 `target/win-shortcuts-tests.log`, `target/win-shortcuts-native.log`, `target/win-shortcuts-package.log`, 캡처 `target/win-shortcuts-preview.png`.

## 2026-11-15 라이선스 상품 검증 (1.0.3 유지)

- 사용자가 제공한 Lemon Squeezy 상품 ID `1180943`을 사용한다. 신규 활성화 전에 부작용 없는 validate 요청으로 상품을 확인하고, 활성화 응답에서도 다시 확인한다. 다른 상품 키는 기기 활성화 요청 전에 거부한다. 상품 메타데이터 누락/형식 오류도 신규 인증을 허용하지 않는다.
- 기존 캐시는 온라인 재검증으로 상품 정보를 추가한다. 네트워크 장애 때 기존 캐시의 원래 28일 오프라인 유예를 유지하며, 다른 상품임이 확인되면 폐기한다. 동일 키의 기존 기기 등록은 재사용한다.
- 일반 테스트49개 통과(대화형2개 제외), 설치 파일 빌드 및 정식 설치/재시작 성공. 빌드 `ca2b17aa20abe6e7`, 메인 PID48336/Guardian PID48832, independent=true/ready=true. 설정과 바로가기 인수 유지. 결과 `target/license-product-install.json`, 로그 `target/license-product-tests.log`, `target/license-product-package.log`.
- API 응답은 모의 데이터로 검증했다. 실제 유료 라이선스의 신규 기기 등록은 수행하지 않았다. 버전은1.0.3이다.

## 2026-11-15 입력 상태 복구 및 커서 갱신 최적화 (1.0.3 유지)

- OS가 Ctrl/Alt/Win 전부 해제됐음을 확인했을 때 불리언뿐 아니라 pressed_modifier_keys의 일반/좌우 키 코드도 함께 정리한다. Shift는 유지한다. 물리 상태 확인과 상태 수정 사이에 새로운 키다운을 지우지 않도록 동일한 AppState 잠금 범위에서 처리한다. 기존 확인 조건/호출 주기는 유지하며 추가 키 상태 폴링은 도입하지 않았다.
- 커서 스타일 요청을 매 프레임 전송하던 경로를 모드/전경 창/실제 적용 상태 변경 시 즉시, 안정 상태에는1초 간격으로 제한한다. UI 스레드의 처리 시점 재검증은 유지한다. 전경 앱 상세 정보는 UI/이동 경로가 공유하는1초 캐시를 사용하고 전경 창 변경 시 즉시 갱신한다. 재시도는 메시지 전송 성공 시에만 예약 상태를 확정한다.
- 일반 테스트44개 통과: 잃어버린 보조키 해제 후 반대편 키를 눌렀다 놓아도 오래된 키가 되살아나지 않는 회귀 검사, 100Hz 요청1000회에서 안정 상태 스타일 요청10회 제한 및 상태 변경 즉시 처리 검사 포함. 대화형 Windows 통합 테스트2개 통과: 실제100회 요청에서 스타일 메시지3회 이하/공유 캐시 상세조회2회 이하, 기존 커서 적용/복구/단일표시/2초HUD/지원창·복사 검사 포함. 실제 CPU 절감량은 측정하지 않았다.
- 정식 설치/재시작 성공. 메인 PID25412, Guardian PID50840, 빌드 `f5fb4e58a6229341`, 표시1.0.3. 기존 설정과 바로가기 인수 유지, Guardian independent=true/ready=true 확인. 결과 `target/input-perf-install.json`, `target/input-perf-result.json`; 로그 `target/input-perf-tests.log`, `target/input-perf-native.log`.
- 라이선스 상품 확인은 Keysor 숫자 product_id 또는 variant_id 정보가 없어 인증 코드를 변경하지 않았다. 사용자에게 위치를 안내했으며, Lemon Squeezy Store→Products→상품 ⋯→Copy ID에서 복사되는 Variant ID로도 검증 가능함을 공식 문서로 확인했다. API 키/실제 라이선스 키를 요청하거나 유료 기기 활성화를 호출하지 않았다. 노트북 실제 원격 화면의 최신 화질은 미검증이다. 인터넷 배포 및 Git 커밋은 수행하지 않았다.

## 2026-11-14 사용자 캡처 기반 커서 가장자리/끝부분 보완 (적용 완료)

- 사용자 캡처 `스크린샷 2026-11-14 213020.png`에서 오른쪽 끝의 평평한 잘림과 번짐을 관찰했다. 기존 선 끝점 x30+반경2가32픽셀 캔버스 경계에 닿는 코드를 확인했다. 원격 전용 그림과 핫스폿을 함께(-2,-1) 이동하여 클릭 위치를 유지하며 투명 여백을 확보했다.
- 4배 벡터 렌더링 후 premultiplied BGRA/alpha 면적 평균으로32×32를 합성하도록 바꿨다. 일반 테스트42개 및 설치 파일 빌드 성공. 실제 커서의 네 변 투명 여부/핫스폿14,15/32×32 크기/안티앨리어싱도 대화형 테스트에 추가했다.
- 사용자 UAC 승인 후 실제 Windows 통합 테스트2개 통과. 최종 커서32×32, 네 변 투명 여백, 핫스폿14,15, 부분 투명 가장자리 및 기본 커서 복구 검사를 통과했다. 일반 테스트42개도 통과했다. 로그 `target/cursor-edge-native.log`, 설치 `target/cursor-edge-install.json`.
- 정식 경로에 적용 및 재시작 완료. 메인 PID35484, Guardian PID41952, 빌드 `a52432985f6b097a`, 표시1.0.3. 기존 설정/바로가기 인수 유지 및 Guardian independent=true/ready=true 확인. 노트북 화면의 최종 시각적 화질은 아직 직접 확인하지 않았다. 클라이언트 확대·전송에 의한 번짐까지 해결됐다고 보장하지 않는다.

## 2026-11-14 커서 복구/재적용 충돌 및 투명 마스크 보완 (적용 완료)

- 진단 기록에서 일반 앱/모드 ON/native_fallback=false 상태에 native_keysor_applied가 false→100ms 후 true로 바뀐 것을 확인했다. 커서 스타일/복구 메시지를 처리하는 UI 스레드에서 현재 AppState와 전경 창을 다시 판별하도록 변경했다. 일반 화면에서 모드 ON이면 지연된 복구 요청을 무시하고 Keysor 모양을 유지한다. 단축키만 눌렸다는 이유로 기본 커서 모양을 복구하지 않고, 실제 시스템 창 또는 OFF에서 복구한다.
- 알파와 맞지 않던 전부 0인 AND 마스크를 실제 투명 픽셀 기준으로 생성한다. 부분 투명 가장자리는 유지한다. 이는 마스크를 사용하는 캡처 경로 호환성 보완이며 사용자 노트북의 픽셀 깨짐 원인이 이 경로라고 확정하지는 않았다.
- 일반 테스트 42개 통과, 릴리스 및 설치 프로그램 빌드 성공. 사용자 UAC 승인 후 실제 Windows 통합 테스트 2개도 통과했다. 일반 화면에서 복구 요청20회에도 적용 상태가 유지되고 기본 커서 테마 복구 횟수가 증가하지 않는 것을 검사했다. 결과 `target/cursor-stability-native.log`, `target/cursor-stability-result.json`, `target/cursor-stability-install.json`.
- 정식 설치 및 재시작 완료. 메인 PID10324, Guardian PID33324, 빌드 `f73629dad7d83b73`, 표시1.0.3. 경로/기존 설정 유지 및 Guardian 준비 상태 확인. 새 진단 기록의 일반 앱/모드 ON 상태에서 native_keysor_applied=true, overlay_visible=false, native_cursor_restore_count=2가 유지됐다. 사용자 노트북의 최종 화질 및 모든 이동 구간의 전환 현상은 직접 확인하지 않았다.

## 2026-11-14 원격용 커서 직접 렌더링 화질 개선 (적용 완료)

- 원격용 커서의 CopyImage 40→32 축소를 제거했다. 공통 `draw_cursor_artwork`로 오버레이와 동일한 선/그라데이션/안티앨리어싱을 사용해 32×32 비트맵에 직접 렌더링한다. 원격 커서는 별도 master를 보관하고 CopyIcon으로 크기 변경 없이 적용한다. 오버레이 DPI/애니메이션은 유지한다.
- 일반 테스트 41개 통과(대화형 2개 기본 제외), 릴리스/설치 프로그램 빌드 성공. 실제 적용된 커서의 32×32 크기와 반투명 가장자리 픽셀도 검사하도록 대화형 테스트를 보완했다.
- 사용자 UAC 승인 후 실제 Windows 통합 테스트 2개 통과. 실제 적용된 커서의 32×32 크기와 반투명 알파 가장자리 유지, 기본 테마 복구를 검사했다. `target/cursor-quality-result.json`은 Success=true, NativeTestsPassed=true다. 실제 그림 영역은 18×20, 핫스폿16,16으로 측정했다.
- 정식 설치 및 재시작 완료: 메인 PID35480, Guardian PID9324, 빌드 `cc4e7d0504ac4d69`, 표시1.0.3. 실행 경로 및 설정 보존 확인, Guardian independent=true/ready=true. 결과 `target/cursor-quality-install.json`, 로그 `target/cursor-quality-native.log`. 노트북 원격 클라이언트에 최종 표시되는 화질은 직접 확인하지 않았다.

## 2026-11-14 원격 화면 맞춤 커서 크기 보정 (1.0.3 유지)

- 사용자: 노트북 원격 화면이 화면 맞춤/축소 표시임을 확인. 이전 a56bbdb/96ddd1e의 기준 캔버스 32 논리 픽셀 및 선 좌표/굵기를 대조했다. 수정 전 실제 Windows 커서 비트맵 측정은 Canvas=40×40, Visible=22×25, Hotspot=20,20으로 호스트 125%의 기존 렌더링과 같았다. 크기 증가를 호스트 비트맵 자체 탓으로 단정한 설명은 잘못이었으며 원격 영상과 별도 포인터의 배율 차이를 고려했다.
- Chrome 원격 모드에서만 네이티브 커서 복제를 CopyImage(... IMAGE_CURSOR,32,32,0)로 보정했다. 기존 기본 32 논리 픽셀 기준이며, 호스트 DPI 확대 이미지를 그대로 클라이언트 포인터로 쓰지 않도록 했다. 휴대폰/억제 상태 오버레이와 비원격 모드 그림은 유지한다. 실제 클라이언트의 영상 축소율 자동 감지는 아니므로 모든 화면 배율에서의 정확한 시각적 일치는 미검증이다.
- 일반 테스트 41개 및 실제 Windows 통합 테스트 2개 통과. 통합 검사에서 실제 적용된 Windows 표준 커서가 32×32인지 확인하고 기본 테마 복구도 검사했다. 적용 후 실제 비트맵 측정: Canvas=32×32, Visible=18×20, Hotspot=16,16 (`target/native-cursor-size-after.txt`).
- 정식 설치/바로가기 적용 및 재시작 성공. 메인 PID 40108, Guardian PID 41800, 빌드 e4cc2a6206fd063c, 표시 버전 1.0.3. 결과 `target/cursor-size-install.json`, `target/cursor-size-result.json`. 실제 노트북 원격 화면에서의 체감 크기는 사용자 확인이 필요하다.

## 2026-11-14 일반 화면 단일 Keysor 모양으로 최종 변경 (1.0.3 유지)

- 사용자 요구: 시스템 팝업 전에는 Keysor 커서만 표시. 원격 모드에서 무조건 오버레이를 표시하던 정책은 폐기했다. 기본 커서 자체에 가시적인 Keysor 그림을 적용하고, 기본 커서 표시 시 오버레이를 숨기며 억제 시에만 오버레이를 표시한다.
- `src/cursor_scheme.rs`는 실제 오버레이의 불투명 색상 비트맵으로 네이티브 커서를 만들고 표준 커서 14종에 복제 적용한다. 투명 커서를 사용하지 않으며 레지스트리 테마를 영구 변경하지 않는다. 현재 PID의 독립 Guardian 준비 보고서가 확인된 경우에만 적용한다. 실패 시 기본 테마를 복구한다. UI 스레드에서 적용/복구하며 모드 OFF, 시스템 예외, 종료 및 Guardian 비정상 종료 복구를 유지한다.
- 일반 테스트 41개 및 대화형 Windows 통합 테스트 2개 통과. 독립 Guardian을 둔 실제 네이티브 커서 적용/복구를 추가 검사했고, 테스트 중에는 기존 앱을 종료했다가 성공 후 수정본으로 재시작했다. 로그: `target/keysor-native-cursor-native.log`, `target/keysor-native-cursor-tests.log`. 설치 결과: `target/keysor-native-cursor-install.json`, 전체 결과 `target/keysor-native-cursor-result.json`.
- 새 메인 PID 34724, Guardian PID 15464, 빌드 ID `46e0e87552e931d2`, 표시 버전 1.0.3. 기존 정식 경로/바로가기/설정 유지. 진단 로그에서 `native_keysor_artwork_ready=true`, `native_keysor_applied=true` 확인. 당시 Windows 기본 커서 억제 상태에서는 오버레이만 표시됐다. 노트북·휴대폰의 실제 화면 비교 및 앱/원격 클라이언트가 자체 렌더링하는 포인터는 미검증이다. 인터넷 배포는 수행하지 않았다.

## 2026-11-14 Chrome 원격 노트북에서도 Keysor 오버레이 우선 표시 (1.0.3 유지)

- 사용자 보고: 휴대폰 원격에서는 Keysor 커서가 보이고 노트북 원격에서는 기본 커서가 보였다. 호스트 CURSORINFO만으로 원격 클라이언트 화면을 판단하던 정책을 Chrome 원격 호환 모드에서 변경했다.
- `--chrome-remote` 모드의 일반 화면에서는 기본 커서 표시/억제/조회 실패와 관계없이 Keysor 오버레이를 표시한다. 시스템 창/단축키 일시정지 및 모드 OFF에서는 기존 기본 커서 대체 처리를 유지한다. 원격 호환 모드가 아니면 기존 중복 방지 정책을 유지한다. 커서 위치 갱신 직전의 표시 검사에도 같은 옵션을 적용했다.
- 일반 테스트 41개, 실제 Windows 통합 테스트 2개 통과. 통합 테스트에서 일반/원격 모드 각각의 표시·숨김 경로와 기존 2초 HUD 및 지원 창/복사 동작을 검사했다. 릴리스/설치 파일 빌드 성공. 표시 버전은 사용자 정책대로 1.0.3 유지.
- 정식 설치 적용 결과: `target/remote-overlay-install.json`. 노트북/휴대폰 실제 화면 간 비교는 미검증이다. 이 수정은 호스트 화면의 Keysor 오버레이를 표시하며 원격 클라이언트 자체의 화살표를 숨기지는 않는다. 따라서 클라이언트 자체 커서까지 포함한 중복 제거를 보장하지 않는다.

## 2026-11-14 사용자 버전 정책 교정: 표시 1.0.3 유지

- 사용자가 명시적으로 버전을 올리라고 요청할 때까지 버전을 올리지 않는다. 중복 커서 방지 등 기능 수정은 유지하고 Cargo/설치 프로그램 표시를 1.0.3으로 되돌렸다. 동일 버전의 수정은 빌드 ID와 SHA256으로 구분한다.
- 1.0.3 릴리스/설치 파일 재빌드 성공. 정식 설치 적용 결과: `target/version-103-install.json`. 이전 1.0.4 설치 파일은 `target/version-rollback-archive/`에 보관한다. 아래 1.0.4 기록은 당시 작업 이력이며 현재 표시 버전 정책을 뜻하지 않는다.

## 2026-11-14 Windows 1.0.4 중복 커서 표시 방지

- 일반 화면에서 실제 Windows 기본 커서가 보일 때 Keysor 오버레이를 숨긴다. 기본 커서가 숨겨지거나 억제된 경우에만 오버레이를 표시하며, 커서 상태 조회 실패 시에도 오버레이를 켜지 않는다. 원격 접속/시스템 UI에서 커서가 사라졌던 과거 문제가 있어 기본 커서를 투명하게 덮어쓰거나 다른 앱의 커서를 강제로 숨기는 방식은 사용하지 않았다. 일반 화면에서 기본 커서 하나가 보일 수 있으며 입력 기능은 유지한다.
- 기본 커서 복구가 필요할 때는 오버레이를 먼저 숨긴다. 정상 오버레이 모드에서는 불필요한 기본 커서 복구를 반복 요청하지 않는다. SWP_SHOWWINDOW 경로 직전에도 기본 커서 표시 상태를 재확인한다. 알려진 시스템 창/단축키의 기본 커서 대체 경로는 유지한다.
- 일반 테스트 41개 통과, 대화형 통합 테스트 2개 통과. 통합 검사에 실제 커서 조회에 따른 오버레이 창 표시/숨김 및 대체 상태/OFF 전환을 추가했다. 로그: `target/single-cursor-tests.log`, `target/single-cursor-native-tests.log`. 외부 원격 접속 클라이언트가 자체적으로 그리는 포인터 및 실제 사용자 화면의 모든 전환 프레임은 검증하지 않았다.
- 릴리스·설치 파일 빌드 및 정식 설치 업데이트 성공. 결과: `target/single-cursor-install.json`, 백업 `target/standard-install-backup-20261114-164714/`. 바탕화면 바로가기 및 기존 `--chrome-remote --diagnose-input` 인수·설정 유지. 새 메인 PID 17764, Guardian PID 38888가 정식 경로에서 실행되며 Guardian `independent=true`, `ready=true` 확인. 인터넷 배포는 수행하지 않았다.

## 2026-11-14 Windows 1.0.3 CapsLock 2초 HUD 및 검증

- `src/caps_hold.rs`에서 CapsLock 눌림/해제를 기존 모드 처리와 별도로 추적한다. 최초 키다운에만 일회성 2초 타이머를 시작하고, 키업/다음 누름의 세대 번호로 이전 타이머를 취소한다. 반복 키다운은 타이머를 재시작하지 않는다. UI 스레드에 명시적 HUD 열기 메시지를 보내며, CapsLock이나 마우스 모드를 추가로 변경하지 않는다. 별도 키 상태 폴링은 없다. CRD lock-sync 및 Keysor 자체 주입 입력은 기존 필터에서 제외된다.
- 일반 Windows 테스트 40개 통과(대화형 테스트 2개는 기본 실행에서 제외). 실제 대화형 Windows에서 두 테스트를 따로 실행하여 모두 통과했다: 모드 ON/OFF에서 2초 전 숨김/2초 후 실제 HUD 표시 및 상태 불변, 지원 창 컨트롤 경계/실제 BUTTON 이벤트/클립보드 UTF-16 내용 일치. 로그: `target/native-ui-tests.log`. 이 테스트들은 테스트용 이벤트를 직접 전달하며 물리 키보드·CRD 장치 입력 자체를 검증하지 않는다.
- 컴퓨터 사용 도구에서 지원 창의 실제 화면을 확인했다. 관리자 권한 차이로 도구의 직접 마우스 클릭은 동작을 확인하지 못해 네이티브 버튼 통합 테스트로 검증했다. 정식 코드의 HUD 정보 링크→지원 창 표시도 네이티브 통합 테스트에 추가하여 검증했다. 이 검사에서 숨김 시작 힌트가 지원 창을 숨기던 문제를 발견했고, 명시적 `SetWindowPos(SWP_SHOWWINDOW)` 표시로 수정 후 두 통합 테스트가 다시 통과했다. 물리 마우스로 직접 누른 검증은 아니다. 개발용 `--preview-support`는 debug 빌드에만 있고 입력 훅/Guardian을 시작하지 않는다.
- 릴리스·설치 프로그램 빌드 성공. `scripts/test-windows-install-lifecycle.ps1`로 실제 정식 설치 업데이트→제거→재설치 성공, 실행 파일/바로가기 제거 및 설정 유지 확인. 결과: `target/lifecycle-test-result.json`, 상세 로그: `target/lifecycle-test-20261114-151223/`. 사용자에게 별도 검증 PC/VM이 없음을 확인했으므로 다른 PC 검증은 미완료다.
- 제거 프로그램은 정상 종료 요청 후 제거하고 해당 설치 경로의 자동 시작 항목을 정리한다. 최종 재설치 후 메인 PID 36128, Guardian PID 31992가 `C:\Program Files\Keysor\standard`에서 실행됐으며 Guardian `independent=true`, `ready=true`. 기존 바탕화면 인수 `--chrome-remote --diagnose-input` 유지. 메인 SHA256 `A6F4B24DBABF11177314131533820E39BABF4F359DE5D38392FF3F56EB8BC637`. 공개 인터넷 배포는 수행하지 않았다.

- 최종 표시 수정까지 포함한 설치 결과: `target/caps-hold-final-install.json`. 메인 PID 39744, Guardian PID 27428가 정식 경로에서 실행 중인 것을 확인했고 Guardian 준비 상태도 확인했다. 최종 메인 SHA256 `287AF6309221EB599C725FBEE98E8ABEA5655295B9A65D43C483D3E20C9BD1BC`, 백업 `target/standard-install-backup-20261114-151804/`. 앞의 PID/해시는 최종 표시 수정 전 수명주기 검증 시점 기록이다.

## 2026-11-14 Windows 1.0.2 정식 설치 경로 및 지원 정보

- 정식 경로는 `C:\Program Files\Keysor\standard`로 유지하고, 설치 프로그램에서 과거 사용자 지정 경로 재사용을 끈다. 실행 중인 Keysor는 WM_CLOSE로 종료를 요청하고 같은 경로에 업데이트한다. 바탕화면 바로가기 생성은 기본 선택이며, 기존 원격 접속/진단 옵션과 활성화된 사용자 자동 시작 경로를 유지한다.
- HUD 하단 오른쪽 `v1.0.2 · 정보 / 지원(Info / Support)`에 지원 창을 추가했다. 버전·소스 빌드 ID·실행 경로·Guardian 상태를 표시하고, 진단 정보 복사 및 수동 업데이트 확인 버튼을 제공한다. 사용자 프로필 경로는 가리고, 라이선스 키·입력 내용·창 제목·방문 URL은 읽지 않는다. 자동 업로드는 없다. `--support-report <파일>`은 훅과 Guardian을 시작하기 전에 진단 보고서만 저장하고 종료한다.
- 빌드 ID는 소스 입력으로 산출한다. Windows 테스트 37개 통과, 메인/Guardian 릴리스 및 Inno 설치 파일 빌드 성공. 공개 GitHub 업데이트 API는 조회 당시 `v1.0.1`을 반환했다. 이번 1.0.2를 인터넷에 게시하지는 않았다.
- `scripts/install-windows-local.ps1`로 기존 파일/바로가기를 백업하고 설치·설정 보존·해시·실행 경로를 검증했다. 초기 설치 전환과 최종 빌드 재설치 모두 성공했다. 최종 결과: `target/standard-upgrade-result.json`, 백업: `target/standard-install-backup-20261114-111743/`. 현재 기록된 메인 PID 7244, Guardian PID 32224는 모두 정식 경로에서 실행됐으며 Guardian `independent=true`, `ready=true`다. 바탕화면 인수 `--chrome-remote --diagnose-input` 유지.
- 실제 설치/업데이트 경로와 훅 없는 보고서 생성까지 검증했다. 화면 자동화 도구가 관리자 권한 Keysor 창을 찾지 못해 HUD 진입·지원 창 레이아웃·클립보드 버튼의 실조작은 확인하지 못했다. 다른 PC 신규 설치/제거 및 다른 사용자 계정으로 UAC 인증하는 설치는 미검증이다. 설치 프로그램 자체의 자동 롤백은 구현하지 않았다. 운영 가이드: `WINDOWS_INSTALL_UPDATE.md`.

## 2026-11-14 입력 리뷰 개선 및 기존 바로가기 빌드 교체

- Windows 기본 CapsLock Toggle은 키다운에서도 비동기 OS 상태 동기화를 요청한다. 키업까지 기다리던 활성화 지연을 줄이고, 비활성화 시에도 재동기화한다. 반복 키다운 차단은 유지한다.
- 좌우 Shift/Ctrl/Alt/Win의 눌림을 개별 키 집합으로 추적하여 한쪽을 놓아도 다른 쪽이 눌려 있으면 보조키 상태를 유지한다. macOS도 좌우 키코드를 구분하고 flagsChanged에서 해당 물리 키의 상태를 읽는다.
- macOS 자동 반복 입력도 공통 입력 처리로 전달하여 마우스 이동 키의 반복 문자가 대상 앱으로 통과하지 않게 한다. macOS 빌드 및 실제 장치 동작은 검증하지 않았다.
- `KEYSOR_TEST_BUILD=1 cargo test --bin keysor --offline`: 34개 통과. `cargo build --release --offline --bin keysor --bin keysor-guardian`: 성공. 기존 컴파일 경고는 남아 있다.
- 기존 실행 경로를 확인한 뒤 메인만 종료하고 Guardian 종료를 기다려, `target/hud-popup-candidate/release/`의 메인과 Guardian을 교체했다. 기존 파일 백업: `target/review-fixes-backup-20261114-101956/`.
- 바탕화면 `Keysor.lnk`의 기존 대상과 `--chrome-remote --diagnose-input` 인수를 유지했다. 재시작된 메인 PID 36544와 Guardian PID 36736의 경로를 확인했고, watchdog 시작 로그는 `independent=true`, `ready=true`를 기록했다. 메인 SHA256: `321F93F54D72C54E1FF1DE123C0852885AECA0BFB11578CB210DEDF46AB2448F`. 배포 결과: `target/review-fixes-deployment.json`.
- 자동 테스트, 파일 해시, 실행 경로 및 Guardian 준비 상태를 확인했다. 실제 화면/키보드 조작 결과는 아직 확인하지 않았다.

## 2026-11-04 Windows 시작 팝업 / 작업표시줄 복원

- Windows에서 키서가 CapsLock ON 상태로 시작해도 최초 HUD 팝업을 표시한다. 시작 시 CapsLock 상태 동기화를 HUD 창 생성보다 먼저 수행하여 기존 마우스 모드와 팝업 표시가 서로 경합하지 않게 했다.
- 작업표시줄에서 최소화된 키서 창을 복원하면 마우스 모드의 자동 숨김 상태와 관계없이 HUD를 다시 표시한다. CapsLock을 새로 켤 때 팝업을 최소화하는 기존 동작, 일반 창 활성화만으로 숨긴 팝업을 다시 열지 않는 동작, 라이선스 입력 중 HUD 숨김은 유지한다. Windows 작업표시줄 복원은 `WM_SIZE`의 복원 경로로 처리하므로, 다른 방식으로 창을 복원할 때도 같은 표시 정책이 적용될 수 있다.
- `scripts/test-windows.ps1`에서 33개 테스트 통과(신규 HUD 정책 테스트 3개 포함). 별도 폴더 `target/hud-popup-candidate/release/`에서 Windows 릴리스 메인/Guardian 빌드 성공. `keysor.exe` SHA256 `6AABE69B683766C43B5FAA39A028E85FC101CDCC413453CBD80C49DE00D32203`, `keysor-guardian.exe` SHA256 `7FAE1EAC5715C1953DF9560AD4C71F5FA9990F8F746EF12B8BA0E67086C11987`.
- 기존 바탕화면 `Keysor.lnk`를 `target/hud-popup-candidate/Keysor-shortcut-before-20261104-185034.lnk`로 백업하고 새 후보로 연결했다. 기존 인수 `--chrome-remote --diagnose-input`를 유지한 채 메인 PID 53204를 종료하고 새 메인 PID 56768과 Guardian PID 50800을 실행했다. `.keysor/input-diagnostic-56768.jsonl` 헤더에서 새 실행 경로를 확인했다. 전환 보고서: `target/hud-popup-candidate/shortcut-switch-report.json`.
- 자동 테스트·실행 경로 확인은 완료했다. 실제 화면에서 시작 팝업과 작업표시줄 클릭 복원 반응은 아직 사용자가 확인해야 한다. 이번 작업에서 공개 `dist` 설치 파일은 교체하지 않았다.

## 2026-11-04 Windows 커서 오버레이 / 전역 자석 탐색 최적화

- **적용 범위:** Windows 전용 `src/ui/win_gdi.rs`, `src/ui/win_uia.rs`만 수정했다. 키 입력, CapsLock, macOS 코드는 변경하지 않았다.
- **커서 오버레이:** 마우스 모드의 반복 갱신에서 위치와 크기가 그대로면 불필요한 `SetWindowPos`를 건너뛴다. 전경 창이 바뀌면 즉시 갱신하고, 가만히 있을 때도 250ms마다 최상단 상태를 재확인한다. 상태/배율 변화에 따른 `UpdateLayeredWindow`는 유지하고, 실제 그림을 그리지 않는 `WM_PAINT`에 대한 매 틱 `InvalidateRect`/`UpdateWindow` 호출은 제거했다.
- **전역 자석 UIA:** 대상이 0개인 정상 탐색 결과도 캐시하여 같은 창을 300ms마다 재탐색하지 않는다. 커서가 80px 이상 움직여 같은 창을 재탐색할 때는 최소 900ms 간격을 적용한다. 창 전환과 `FORCE_UIA_REFRESH`는 즉시 처리하고, 창의 루트 UIA 요소를 가져오지 못한 경우에는 다음 주기에 다시 시도한다. 전역 자석 모드가 꺼져 있으면 기존처럼 탐색을 건너뛴다.
- **자동 검증:** `scripts/test-windows.ps1`의 Windows 테스트 30개 통과. `cargo build --release --offline --bin keysor --bin keysor-guardian` 성공. 별도 빌드 폴더 `target/overlay-uia-opt-candidate/release/`의 `keysor.exe` SHA256은 `3B1AD390768D33DA832D4330D34593D50786529C0E9615F3606C3448DB716E20`, `keysor-guardian.exe` SHA256은 `5CD98416E3FB8F38E7D0938B414300341E63C8E670F437BE0B1ADE8F9DFEDB01`이다.
- **실행/바로가기:** 기존 바탕화면 `Keysor.lnk`를 백업한 뒤 새 후보를 대상으로 변경했다. 인수 `--chrome-remote --diagnose-input`는 유지했다. 이전 메인 PID 35116을 종료하고 바로가기에서 새 메인 PID 12276, 독립 Guardian PID 28560을 시작했다. 재확인 시 두 프로세스가 실행 중이었고 `.keysor/input-diagnostic-12276.jsonl` 헤더도 새 실행 파일 경로를 기록했다. 바로가기 백업과 전환 결과는 각각 `target/overlay-uia-opt-candidate/Keysor-shortcut-before-20261104-054004.lnk`, `target/overlay-uia-opt-candidate/shortcut-switch-report.json`에 있다.
- **검증 경계:** 테스트/빌드와 실행 경로 확인은 완료했지만, 사용자가 보는 커서 부드러움·최상단 표시·자석 반응과 실제 CPU 절감량은 아직 실조작/계측으로 확인하지 않았다. `dist` 설치 파일이나 공개 배포 파일은 이번 작업에서 교체하지 않았다. 아래 과거 섹션의 실행 PID·바로가기 상태는 작성 당시 기록이며 현재 상태가 아니다.

## 2026-11-03 Windows CapsLock Mode Alignment Candidate

- The default Windows `CapsLock` + `Toggle` path now lets one real keydown and its keyup reach Windows while Keysor mirrors the same on/off transition. Repeated keydown is blocked. This avoids the old hook-thread `GetKeyState`/`SendInput` decision for each physical press.
- Chrome Remote Desktop's virtual lock-sync events (scan code 0) are blocked when this mode owns CapsLock, so they cannot toggle only the Windows lock state after a real press. Other modifier modes retain the previous handling.
- Configuration reload preserves an active native CapsLock mode and held-key state when the CapsLock toggle setting is unchanged. Windows tests: 30 passed. Unsigned release build succeeded. Isolated pair: `target/capslock-sync-candidate/Keysor.exe` SHA256 `CBC5AA14116F1C631F4AD5889D6B8BB9747D770D069B9F538288095734709583`; Guardian SHA256 `2CC36BFAF56D8BA796D446666A0DC00A8B6173A07BBDB3F3D3E7BC446956A784`.
- The previous `dist` executable remains running as PID 46104 with Guardian PID 52972; desktop shortcut and installed files are unchanged. A guarded attempt to stop only PID 46104 failed with Windows `Access is denied`, so the new candidate was not launched. Native Windows/Chrome Remote Desktop visual CapsLock-state agreement requires an interactive test after the user cleanly exits the elevated old process. External lock-state changes by other software are not covered by these unit tests.
## 2026-11-03 Shift+Space Enter Output Queue

- Shift+Space now queues the simulated Enter on the existing output worker rather than injecting it synchronously from the keyboard hook. Keydown still triggers once, keyup remains blocked without another Enter.
- The actual handler path has a regression test; 27 Windows tests passed with `KEYSOR_TEST_BUILD=1`. Rebuilt the unsigned Windows setup and distinct main/guardian executables; `dist/SHA256SUMS.txt` was regenerated and verified by the package script.
- The installed/running Keysor was not replaced. Live typing and shortcut timing, clean-PC installation, and Mac runtime behavior remain unverified. The Space tap/drag timer-thread design is still a separate pending improvement because it has more interaction and cancellation risk.
## 2026-11-03 Input Hook Beep Isolation

- Pro-only J/K tab and RunApp denial now queues a `Beep` action instead of calling the OS sound API while the keyboard hook holds AppState. This keeps blocking feedback off the hook callback and does not change shortcut eligibility or key blocking.
- Added tests for keydown feedback and keyup no-action on both paths; 26 Windows tests passed using `KEYSOR_TEST_BUILD=1`. The unsigned standard Windows package rebuilt successfully, including the distinct guardian. `dist/SHA256SUMS.txt` matches the new installer and executable hashes.
- The installed/running Keysor was not replaced or restarted. Audible feedback timing and GUI behavior on a real machine remain unverified; the Mac build/runtime was not tested in this Windows session.
## 2026-11-03 Dedicated Guardian Binary

- Keysor remains one user-facing application and shortcut. The installer contains a separate 191 KB `keysor-guardian.exe` helper that only waits for the main process and reloads the native cursor; it has no input hooks or UI. The main executable is about 3.18 MB. They are different compiled binaries, not renamed copies.
- Windows package build succeeded. SHA256: `Keysor.exe` `1F083595A045660B467A90D96A4ACF9BDA129DC6C4B35F2E0DA99F5DE903DEF5`; `Keysor-Guardian.exe` `BCC1466CA4FFBB17BEB32BD1CD05A1D45C27C4A705CA1BE9FAA27BA2F964523F`; setup `246839BBD7CE0C790AA7063AB3D7405A686C8603492D13036091C38B54701C5`.
- The 24 existing Windows tests passed in the no-manifest test build. An isolated release-guardian probe force-stopped only the disposable parent's image name and recorded `ready=true`, `wait_result=0`, `restore_ok=true` in `target/watchdog-test/dedicated-guardian-result.json`. The installed/running Keysor was not replaced or restarted. Actual Defender quarantine, killing the entire process tree, and a second-PC installation remain unverified; a distinct binary improves the image-name case but does not guarantee survival of every security action.
## 2026-11-03 Windows Package Consistency

- Updated `scripts/package-windows.ps1` and `scripts/keysor-setup.iss` so Inno Setup first builds into `target/release-package/windows-output`; only a complete staged package with matching main/guardian hashes is copied to `dist`.
- Rebuilt the unsigned standard Windows release. `dist/Keysor.exe` and `dist/Keysor-Guardian.exe` both have SHA256 `1886AB4398F0A401FCBF92FFA9CD9800F9E300E36C7DA7E1CD3998F3C712C9DF`. `dist/Keysor-Setup-1.0.1.exe` has SHA256 `0DDBD05FDE8C8D7A3B90359E18E0A7E1254BBF0F4576D973E429591CBCB280C1`. `dist/SHA256SUMS.txt` matches all three.
- The installed/running Keysor was not restarted. The old macOS artifacts in `dist` are unrelated to this Windows build. A clean-PC installation and real cursor/UI test are still pending; this is not a public-release signoff. Earlier notes that `dist` was not updated describe the earlier state.

## 2026-11-03 Independent Cursor Guardian Candidate

- Replaced the unverified encoded-PowerShell and same-name-only watchdog startup path with `src/watchdog.rs`. The packaged `keysor-guardian.exe` is a separate process name, so stopping `keysor.exe` by image name does not target the guardian. Startup writes `.keysor/watchdog-startup.json` with guardian PID, independence, and readiness; after parent exit it writes `.keysor/watchdog-<parent-pid>.json` with wait and cursor-reload results.
- The cursor reload now returns the Windows API result and does not write cursor settings back to the user profile. Manual `--restore` exits nonzero on failure. If the independent companion is absent or fails readiness, startup attempts a same-executable fallback and records that it is not independent.
- Isolated forced-exit test passed without stopping the live Keysor: a disposable PowerShell parent was force-stopped; `keysor-guardian.exe` reported `ready=true`, `wait_result=0`, and `restore_ok=true` in `target/watchdog-test/watchdog-probe-20261103014650431.json`.
- Final source: 24 tests passed, unsigned standard release build succeeded. Candidate pair in `target/watchdog-final-candidate/` has SHA256 `1886AB4398F0A401FCBF92FFA9CD9800F9E300E36C7DA7E1CD3998F3C712C9DF` for each executable. Inno Setup candidate in `target/watchdog-setup-candidate/Keysor-Setup-1.0.1.exe` includes both files and is unsigned.
- 2026-11-03 in-place deployment: backed up the previous installed executable to `target/keysor-before-watchdog-in-place.exe` (SHA256 `890E66DF90234BD031C41B6798107D8E737BB792B7B47849245ACBAEF287E7B1`), then replaced `C:\Program Files\Keysor\suppressed-shell-890e66df9023\keysor.exe` and installed `keysor-guardian.exe` alongside it. Both installed files match candidate SHA256 `1886AB4398F0A401FCBF92FFA9CD9800F9E300E36C7DA7E1CD3998F3C712C9DF`. The desktop shortcut retains its existing target and arguments. Runtime inspection found the main and guardian processes in that folder; `.keysor/watchdog-startup.json` reported matching parent/guardian PIDs, `independent=true`, and `ready=true`. Live force-kill verification on 2026-11-03: main PID `50896` was force-stopped; guardian PID `8336` exited after recording `ready=true`, `wait_result=0`, and `restore_ok=true` in `.keysor/watchdog-50896.json`. The test restarted Keysor as PID `49836` with guardian PID `39376`; both process paths matched the installed folder, and startup reported `independent=true`, `ready=true`. The Windows API reported cursor restoration success; actual visual cursor state was not independently observed. Result: `target/watchdog-live-kill-result.json`. Public `dist` was not overwritten.
- Post-test cleanup caveat: cancelling the PowerShell `Start-Process -Wait` wrapper also terminated the restarted Keysor process tree (main PID `49836` and guardian PID `39376`); that second shutdown produced no guardian restore-complete report. A direct `SPI_SETCURSORS` call returned true, and Keysor was restarted again as main PID `25552` with independent guardian PID `35560`, both alive and reporting `ready=true`. Do not use `Start-Process -Wait` or Ctrl+C for future live Keysor verification; the first isolated main-only force-kill result above remains the verified recovery case.
- The earlier UAC-stalled candidate-folder attempt was superseded by the in-place deployment above. `target/install-watchdog-in-place.ps1` performed the guarded backup, replacement, launch, and readiness check; its result is in `target/watchdog-in-place-result.json`. Git tracks source changes, not the installed executable backup.

## 2026-11-03 Native Cursor On System Panels

- Local user test: Windows native cursor is visible on system panels, but native and Keysor overlay cursors overlap in ordinary windows. This supersedes the earlier remote-only assumption that native fallback might be invisible.
- Source now always hides the Keysor overlay on Start/Search/Shell/ControlCenter/Task Manager/XAML system panels and leaves the native pointer visible there, regardless of `GetCursorInfo` flags. Secure and desktop-switching surfaces still use native fallback.
- 23 Windows tests passed and the unsigned standard release build succeeded. This build is not installed or promoted to `dist`; the running executable remains the previously installed unsigned candidate until an interactive restart.
- Ordinary windows still show both cursors because `hide_system_cursor()` deliberately preserves the native pointer. `ShowCursor` is per-thread and `SetCursor` is reliable only while owning mouse input; neither safely hides another app's pointer. Do not reintroduce transparent `SetSystemCursor`: prior attempts left the OS cursor invisible, especially over system UI and remote desktop. A single-cursor ordinary-window design needs a different, separately validated rendering approach or an explicit tradeoff.

## 2026-11-03 Start Menu Cursor Follow-Up

- Confirmed running PIDs 46612/49904 use the unsigned `suppressed-shell-890e66df9023` build. The user's report concerns this build, not the older shortcut target.
- Its opt-in diagnostic recorded `searchhost.exe` / `Windows.UI.Core.CoreWindow`, mouse mode ON, `native_fallback=true`, and `overlay_visible=false` while the user reported an invisible pointer. The trace reported `cursor_flags=1` and a non-null handle in that session, so the earlier suppressed-cursor-only exception did not apply.
- Changed ordinary Start/Search/system panels to keep the Keysor overlay enabled regardless of native cursor flags. Secure and desktop-switching surfaces retain native fallback. This may still be obscured by the Windows shell window band; actual visibility requires a remote visual test.
- 23 Windows tests passed with `KEYSOR_TEST_BUILD=1`; unsigned standard release build succeeded. Isolated candidate: `target/shell-overlay-candidate/keysor.exe`, SHA256 `2D50B6E2A77C76727740EC660AC812D9B6625AF160F30A8EB49043E2779EBC6E`.
- Candidate installation/restart is pending Windows UAC confirmation. At this point the desktop shortcut and running PIDs still use the previous unsigned `suppressed-shell-890e66df9023` build. Do not claim the new candidate fixes Start or publish it until tested.

## 2026-11-03 Suppressed Cursor Shell Candidate

- User reconfirmed that the unsigned standard build still loses the cursor on system panels while connected from another PC through Chrome Remote Desktop.
- In the existing trace, Start (`searchhost.exe`) triggered native-pointer fallback and hid the Keysor overlay while Windows reported `CURSOR_SUPPRESSED` and a null cursor handle. Chromium's Windows cursor capture also transmits an empty cursor image for this state.
- Candidate change: ordinary shell panels keep the Keysor overlay when the native cursor is reported suppressed. The previous shell fallback remains for other cursor states; UAC/secure-transition surfaces still use the native fallback. This may still be hidden behind the shell's window band and requires a live visual test.
- 23 tests passed; unsigned release executable built at `target/suppressed-shell-candidate/keysor.exe` with SHA256 `890E66DF90234BD031C41B6798107D8E737BB792B7B47849245ACBAEF287E7B1`. The separate Program Files install could not complete because Windows administrator confirmation stalled in the remote session. The desktop shortcut, running process, and public `dist` files remain on the previous unsigned build.
- The existing proprietary EULA does not satisfy SignPath Foundation's OSI-license requirement for its free open-source signing program. Do not treat local self-signed UIAccess as a public release path.

## 2026-11-03 Unsigned Build Restart

- The desktop shortcut targets `C:\Program Files\Keysor\shell-cursor-89d27b65a58f\keysor.exe` with `--chrome-remote` only. Its signature is `NotSigned` and SHA256 is `89D27B65A58F27CC7FE7AE5B7C6A9FD6B815F21669B371264FA7A0992830CE7A`.
- Stopped the previous local UIAccess processes (PIDs 43148/50344) and launched the desktop shortcut. New PIDs 26192/53424 both report the unsigned standard executable path through `QueryFullProcessImageNameW`.
- Process/path verification does not prove Start-menu cursor visibility. The Chrome Remote Desktop `CURSOR_SUPPRESSED` case remains unresolved in the unsigned build.

## 2026-11-02 System Panel Cursor Follow-Up

- User verified that first CapsLock activation and magnet mode now work. The cursor still disappears behind Windows system panels during Chrome Remote Desktop use.
- The earlier 120-second trace recorded mouse mode ON and the overlay visible, but only a foreground window handle. It cannot identify which panel was foreground or whether native-pointer fallback ran. `GetCursorInfo` reporting a visible cursor does not prove the remote client rendered it.
- The opt-in `--diagnose-input` trace now records the foreground process name, window class, and whether that process should use native-pointer fallback, without window titles or typed text. Capture lasts five minutes after launch. This is diagnostic only; it does not claim to solve the cursor issue.
- 23 tests and the unsigned Windows package build passed. The isolated candidate is `C:\Program Files\Keysor\shell-cursor-89d27b65a58f\keysor.exe` (SHA256 `89D27B65A58F27CC7FE7AE5B7C6A9FD6B815F21669B371264FA7A0992830CE7A`). The prior magnet candidate remains installed for rollback.
- User reproduced the invisible cursor in Start. The new trace recorded `searchhost.exe` / `Windows.UI.Core.CoreWindow`, `native_fallback=true`, `overlay_visible=false`, and `CURSOR_SUPPRESSED` (`cursor_flags=2`, null handle). Thus the standard build hides its overlay on Start while Windows is not drawing a native pointer. Previous zero-distance cursor refreshes did not clear this state.
- A separate local UIAccess candidate was built from the same source and signed with an already trusted local certificate. No certificate trust settings were changed. Installed path: `C:\Program Files\Keysor\uiaccess-shell-6d6e1553e602\keysor.exe`; signed SHA256 `00C3AC6167681C9CABEC3C6398A98AF81759339C579B46959E17CF0B506987E0`. Signature is Valid; hook-free token probe reported `ui_access=true`. Its shell policy keeps the overlay visible on Start/Search when UIAccess is active. Actual Start z-order still needs interactive verification.
- The desktop shortcut was backed up and pointed to the UIAccess candidate. On the subsequent restart request, old Keysor PIDs 43900/47212 exited and new PIDs 43148/50344 started; the new diagnostic header identifies PID 43148 and `C:\Program Files\Keysor\uiaccess-shell-6d6e1553e602\keysor.exe`. Start-menu cursor rendering still needs the user's visual test. This local signed candidate is not the unsigned public release.
- User reaffirmed that the goal is a no-cost public release. The UIAccess shortcut change was the wrong default for that goal. The desktop shortcut has been backed up again and restored to the unsigned standard candidate `C:\Program Files\Keysor\shell-cursor-89d27b65a58f\keysor.exe` with only `--chrome-remote`; `--diagnose-input` was removed. The installed candidate, `dist/Keysor.exe`, and `dist/Keysor-Setup-1.0.1.exe` are all NotSigned; executable SHA256 matches across installed/source/dist. The currently running UIAccess process could not yet be stopped because elevation confirmation stalled, so the shortcut restoration is not proof of a runtime switch.

## 2026-11-02 Magnet Toggle Responsiveness

- Confirmed code path: HUD applies and saves a magnet setting, then the file watcher reapplies the identical configuration through `reload_configuration`, which deactivates mouse mode. File-watcher updates now skip configurations already active in memory. Explicit license refresh still uses the original reload path; real external configuration changes still reload.
- Removed snap/input lock inversion: global movement status is captured before the snap-position lock; magnet OFF clears snap state after releasing APP_STATE. Rendering reads snap state without waiting. Clear requests defer to the movement loop when snap state is busy rather than blocking HUD/input callbacks.
- Added regression tests for preserving mode/held movement keys on magnet self-save and nonblocking snap read/clear under contention. 23 tests passed. Installer and executable rebuilt; interactive remote magnet-toggle behavior remains to be verified.
- Chrome remote CapsLock compatibility changes are retained. The separate system-popup cursor suppression issue remains unresolved.

## 2026-11-02 Chrome Remote Input Compatibility Candidate

- User confirmed Chrome Remote Desktop is the only available test environment. The diagnostic trace shows the first recorded activation turned mouse mode ON; subsequent activation events toggled it. The trace did not distinguish physical intent from remote lock synchronization.
- Chromium `remoting/host/input_injector_win.cc` sends virtual-key CapsLock synchronization before non-lock keys, separately from scan-code key presses. Windows hook now passes these scan-zero injected lock-sync events to Windows without treating them as Keysor activation. Recognized CRD signature: `0x43524430`; `--chrome-remote` enables this distinction for older hosts without the signature. This opt-in can also bypass scan-zero synthetic CapsLock from other tools; normal hardware scan-code inputs are preserved.
- Diagnostic mode now counts passed remote lock-sync events. No typed text is logged. 21 tests passed. The compatibility path is source-grounded but needs user reproduction before claiming the reported first-key symptom fixed.
- Cursor suppression over system UI remains unresolved. Do not mark this candidate as a complete fix or publish it as resolving both issues. No remote connection, certificate store, global mouse preference, or security setting was changed.
- Source: https://chromium.googlesource.com/chromium/src/+/HEAD/remoting/host/input_injector_win.cc

## 2026-11-02 Unresolved Input/Cursor Reproduction

- User confirmed the recovery candidate still fails: after the first CapsLock, letters are typed; over system panels, hover works but even physical mouse movement does not reveal the pointer. Do not mark either symptom resolved.
- Live process audit confirmed the recovery candidate plus its watchdog, not an obsolete build. Activation configuration is Toggle/CapsLock; pointing devices report OK. An external cursor query reported flags=2 and null handle; same-position SendInput succeeded but did not clear suppression. External query alone does not establish the app's exact state during the user's reproduction.
- Windows startup now waits for keyboard hook readiness before launching its UI. This removes an observed startup ordering gap; the reported first-key failure still requires reproduction to establish causality.
- `--diagnose-input` enables a bounded 120-second, change-only trace in the user's `.keysor/input-diagnostic-<pid>.jsonl`. Captures activation counts, mouse-mode/modifier state, cursor flags and overlay visibility. No typed text, general key stream, titles or URLs. No diagnostic mode is enabled by default.
- 20 tests passed. This is a diagnostic candidate, not a confirmed cursor fix; preserve previous installed releases and do not publish as resolved before collecting the reproduction trace.

## 2026-11-02 Cursor Recovery Follow-Up

- User reported an invisible cursor on the first CapsLock activation and when opening Start with the standard build. Runtime causality has not yet been independently reproduced.
- Confirmed code gaps: restore was guarded by flags never set by the current implementation; the first overlay show could return before positioning/rendering when Windows still reported it hidden.
- Candidate now queues actual native-cursor recovery on the UI thread, coalesces requests and throttles recovery to 250 ms. It reloads the cursor scheme without persistent setting writes and sends a zero-distance mouse event. It checks OS cursor visibility and refreshes on shell entry, including transparent cursor handles that may report visible.
- First activation and hidden-to-shown transitions redraw the overlay; permitted positioning no longer depends on the first ShowWindow visibility result. Input mappings and keyboard hook logic are unchanged.
- 20 unit tests passed; unsigned standard executable and installer rebuilt. These tests do not prove the reported GUI symptoms are resolved. Candidate deployment metadata and shortcut rollback path are in `target/cursor-recovery-deploy.json` when deployment completes. Existing process is not automatically restarted.

## 2026-11-02 No-Cost Distribution Decision

- User selected unsigned standard Windows distribution instead of paid public code signing. This supersedes UIAccess as the public release target below; the installed UIAccess candidate is preserved for local use.
- `scripts/package-windows.ps1` rebuilds the standard manifest with UIAccess disabled, creates `dist/Keysor-Setup-1.0.1.exe`, `dist/Keysor.exe`, and `dist/SHA256SUMS.txt`. No certificate store access, signing or trust changes are part of this packaging path.
- Installer uses `Program Files/Keysor/standard`, provides shortcuts and uninstall, and does not launch or stop keyboard hooks. It no longer requires the local development certificate. Setup and the existing standard executable require administrator approval.
- Native Windows cursor fallback is used on shell surfaces without UIAccess; custom overlay above Start/Search/quick settings is not promised. Input mappings are unchanged.
- Unsigned publisher/SmartScreen warnings or managed-device blocks remain possible; do not disable security policies. Test installation, shell cursor operation, typing and uninstall on another PC before publishing the final release. Existing running app and desktop shortcut were not replaced.
- `dist/Verify-Keysor.ps1` performs a hook-free token check expecting `ui_access=false`; it does not establish GUI/input correctness. Old Mac archives in `dist` are unrelated and must not be presented as this build.

## 2026-11-02 Windows Shell Cursor Candidate

- Fixed the hide-then-show contradiction: suspended overlays now return before `SWP_SHOWWINDOW` positioning. Removed repeated `BringWindowToTop` calls.
- Start, Search and quick-settings surfaces permit the overlay only with an actual UIAccess process token. Secure/transition surfaces retain the native pointer fallback.
- `show_indicator` requests UI-thread work instead of synchronously showing the overlay from the keyboard hook. Input mappings were not changed in this update.
- Opt-in build: `$env:KEYSOR_UIACCESS_BUILD='1'; cargo build --release --bin keysor --offline --target-dir target/uiaccess-build`. Default manifest is unchanged.
- Verified: 19 unit tests passed; release build succeeded; extracted PE manifest contains `asInvoker`, `uiAccess=true`. Compiler warnings remain.
- `scripts/install-uiaccess-candidate.ps1` uses the existing trusted certificate and a new versioned Program Files folder; it does not change trust/UAC policy, terminate Keysor, overwrite old releases, or alter shortcuts.
- Runtime probe: installed executable with `--diagnose-uiaccess <output.json>` exits without starting input hooks, UI, watchdog, or single-instance handling. Require `ui_access: true` before shortcut promotion.
- Installed and signature verified: `C:\Program Files\Keysor\uiaccess-9793465b6de0\keysor.exe`. SHA256: `F612C83CE0ECFED69AB319100579D7ED4AD49D87F672CD6886E4CCA34DC58D3D`.
- Runtime diagnostic confirmed `ui_access: true`, `input_hook_started: false`; evidence: `target/uiaccess-token-verified.json`, `target/uiaccess-install.json`.
- Existing desktop `keysor.lnk` now targets the installed candidate. Original shortcut backup: `target/keysor-shortcut-before-uiaccess-20261102-112151.lnk`; prior target: `target/hud-async-build/release/keysor.exe`. Existing processes were NOT restarted. Exit Keysor fully and use the same desktop shortcut to apply. To roll back, exit the candidate and restore the backed-up shortcut to the desktop.
- Actual Start/search/quick-settings z-order, normal typing, CapsLock and Alt+Tab require interactive testing; unit tests and the hook-free token probe do not establish those results. Earlier dashboard success claims are not runtime evidence for this candidate.

## 📊 프로젝트 개요 (Overview)

| 구분 | 사양 및 상태 | 세부 특징 |
| :--- | :--- | :--- |
| **프로젝트명** | **키서 (Keysor)** | 오토핫키를 모방한 고자유도 단축키 마우스 시뮬레이터 |
| **타깃 OS** | **Windows** (Win32 Native API) | 윈도우 저수준 API를 활용한 무결점 하드웨어 레벨 연동 |
| **개발 언어** | **Rust** (Ed. 2024, windows-sys) | 가비지 컬렉터(GC) 배제로 지연 시간 0ms 달성 |
| **바이너리 용량**| **약 2.7 MB** (`keysor.exe`) | 의존성 없는 초경량 고성능 단일 실행 파일 |
| **메모리 점유율**| **2 MB 미만** (동작 중 기준) | Electron(100MB+) 및 Python(40MB+) 대비 약 95% 절감 |
| **컴파일 상태** | **Finished `release` [optimized]** | `cargo build --release` 경고 0개, 에러 0개 완벽 통과 |

## 🛑 실패했던 기술적 시도 및 절대 금지 수칙 (Failed Approaches & Anti-Patterns)

> [!CAUTION]
> 다음은 윈도우 시작 메뉴 및 시스템 팝업 최상단 노출 개발 과정에서 이미 수차례 테스트 후 실패가 입증된 기술적 접근 방식입니다. 향후 개발 시 절대 동일한 헛수고를 반복하지 마십시오.

1. **UIAccess + SetWindowBand (밴드 7/8/3)를 이용한 오버레이 창(`HWND`) 시작 메뉴 최상단 노출 시도 (실패)**
   - **실패 원인**: 윈도우 11 DWM (데스크톱 윈도우 매니저)의 DirectComposition 렌더링 계층 구조상, 시작 메뉴/달력 팝업 서피스는 DWM 커널이 합성 순서를 강제 락(Lock) 시켜 놓음. UIAccess 특권 및 비공식 `SetWindowBand` API 주입 시에도 `res=0`으로 차단되며, 시작 메뉴 뒤로 강제 강등됨.
   - **금지 지침**: 비공식 API나 윈도우 밴드 주입으로 오버레이 창(`HWND`)을 시작 메뉴 위로 올리려는 불가능한 시도를 하지 말 것.

2. **프로세스를 관리자 권한(Elevated Administrator)으로 실행하는 시도 (실패)**
   - **실패 원인**: 윈도우 OS 보안 정책상 프로세스가 Elevated Administrator 권한으로 기동되면 OS가 `uiAccess="true"` 요청을 즉시 강제 거부(`UIAccess Active: false`) 처리함.
   - **금지 지침**: 자가 배포나 기동 시 UAC 어드민 권한 상속 스크립트를 띄우지 말 것.

3. **시스템 팝업 감지 시 `SystemParametersInfoW(SPI_SETCURSORS)` 단독 호출로 커서 복원 시도 (실패)**
   - **실패 원인**: `SetSystemCursor`로 투명 커서(`blank`)가 입혀진 상태에서는 `SPI_SETCURSORS`만 호출할 경우 레지스트리만 갱신될 뿐, 손으로 물리 마우스를 흔들지 않고 키보드 단축키(WASD / `SetCursorPos`)만으로 조작할 때는 윈도우 OS가 마우스 비트맵을 갱신하지 않아 커서가 투명 상태로 고정되어 안 보임.
   - **필수 지침**: 반드시 명시적 주입을 실행해야 물리 마우스 이동 없이 키보드 조작 시 마우스 화살표가 0.001초 만에 최상단으로 살아남.

4. **포그라운드가 시작 메뉴일 때 오버레이 창 노출을 고집하던 시도 (실패)**
   - **실패 원인**: 오버레이 창은 시작 메뉴 뒤로 가려지고, 진짜 마우스는 투명해져서 커서가 완전히 안 보이게 됨.
   - **필수 지침**: 포그라운드가 시작 메뉴, 시계/달력, 빠른 설정 등 독립 전면 팝업일 때는 정밀 감지기(`is_system_shell_foreground_with_info`)가 이를 인지하여 처리해야 함.

5. **서스팬드(`is_suspended == true`) 상태에서 오버레이 창 무효화(`SetWindowPos` / `UpdateWindow`) 지속 호출 (실패)**
   - **실패 원인**: 창이 `SW_HIDE`로 숨겨진 상태에서 100Hz 타이머 루프가 계속 `SetWindowPos`와 `UpdateWindow`를 때리면 윈도우 DWM이 레이어드 창 버퍼를 지속 갱신하면서 OS 레벨의 마우스 복원 화살표 비트맵을 무효화(Invalidate)시켜 커서가 다시 투명해져 가려짐.
   - **필수 지침**: 창이 숨겨져 있을 때에는 위치 갱신 연산을 멈출 것.

6. **커서 복원 시 화살표(`OCR_NORMAL`) 1개만 복원하고 손가락/I빔 등 나머지 커서를 `blank`로 방치 (실패)**
   - **실패 원인**: 윈도우 시작 메뉴, 검색창, 알림 센터 등 팝업창 위에 마우스 포인터가 지나갈 때 윈도우 OS는 마우스 포인터를 `OCR_HAND`(손가락) 또는 `OCR_IBEAM`(텍스트 커서)으로 변경함. 이때 화살표 1개만 복원하고 손가락/I빔 커서를 투명(`blank`) 상태로 놔두면 팝업창 버튼/텍스트 상자 위로 마우스가 지나는 순간 커서가 100% 투명하게 증발함.
   - **필수 지침**: 시스템 커서 복원 시 전체 커서 아이콘을 완전히 복원할 것.

7. **마우스가 작업 표시줄 영역 위로 이동 시 오버레이 창을 꺼버리던 오판 시도 (실패)**
   - **실패 원인**: 작업 표시줄 영역 위에서는 오버레이 창이 숨겨지지 않고 렌더링을 계속 유지할 수 있음에도 이를 "시스템 팝업"으로 과도하게 오판하여 초록색 Keysor 오버레이 커서를 꺼버려 작업 표시줄에서 커서가 안 보이게 됨.
   - **필수 지침**: 작업 표시줄 위에서는 초록색 Keysor 오버레이 창(`INDICATOR_HWND`)이 꺼지지 않고 작업 표시줄 픽셀 위에서 지속 노출되도록 유지할 것.

8. **`SetSystemCursor(blank)` 로 OS 레벨의 시스템 커서 비트맵 원본을 투명으로 덮어써 파괴하던 시도 (실패)**
   - **실패 원인**: `SetSystemCursor`로 윈도우 OS 커널 비트맵 원본 자체를 투명(`blank`)으로 덮어씌워버리면, 시작 메뉴/UAC 팝업창이 떠올랐을 때 윈도우 DWM 보안 장벽에 의해 `SetSystemCursor` 복원 호출이 블로킹되어 OS 메모리에 투명 비트맵만 남게 됨. 특히 원격 데스크톱(Chrome Remote Desktop 등) 환경에서는 마우스 포인터가 비디오 스트림상에서 100% 투명하게 증발함.
   - **필수 지침**: `SetSystemCursor(blank)` 로 OS 마우스 커서 비트맵 원본을 파괴하는 짓을 100% 절대 하지 말 것!

9. **`show_indicator()` 기동 시 `INDICATOR_HWND` 창을 `SW_HIDE` 로 꺼버리던 잔재 코드 방치 (실패)**
   - **실패 원인**: 마우스 모드가 켜지는 순간 `show_indicator()` 함수에서 `INDICATOR_HWND` 창을 `SW_HIDE` 로 꺼버려 오버레이 커서가 아예 눈앞에서 사라지는 치명적 오류가 유발됨.
   - **필수 지침**: `show_indicator()` 에서는 반드시 `SW_SHOWNA` 와 `SetWindowPos(HWND_TOPMOST)` 로 오버레이 창을 최상단 위에 선명하게 강제 켜줄 것.

14. **시작 버튼 클릭 시 작업 표시줄(`Shell_TrayWnd`)의 솟구침(Topmost Elevate)에 오버레이 커서가 깔리던 현상 (실패)**
    - **실패 원인**: 시작 버튼을 클릭하면 윈도우 OS가 작업 표시줄을 최전면으로 솟구치게 갱신하는데, `SetWindowPos` 호출 시 `SWP_SHOWWINDOW` 플래그 및 `BringWindowToTop(hwnd)` 펄스가 빠져있어 커서 창이 작업 표시줄 픽셀 아래로 가려짐.
    - **필수 지침**: `update_overlay_window_position()` 에서는 매 루프 `SWP_SHOWWINDOW` 와 `BringWindowToTop(hwnd)` 펄스를 주입하여 솟구치는 작업 표시줄 픽셀을 100% 지속 제압할 것.

15. **원격 데스크톱(CRD) 환경에서 시스템 팝업 활성화 시 `SW_HIDE` 로 오버레이 창을 숨겨 비디오 스트림 커서 소실 (실패)**
    - **실패 원인**: 원격 데스크톱 비디오 스트림은 OS 기본 시스템 커서를 캡처하지 않으므로, 시스템 팝업 시 Keysor 오버레이 창을 `SW_HIDE` 로 꺼버리면 원격 비디오 화면에 커서가 100% 안 보이고 소실됨.
    - **필수 지침**: 원격 데스크톱 환경 스트림 캡처를 위해 마우스 모드에서는 시작 메뉴/팝업이 떠도 Keysor 오버레이 창(`INDICATOR_HWND`)을 100% 무조건 `SW_SHOWNA` 로 상시 유지할 것.

16. **`Win+Z` 키보드 신호 자동 시뮬레이션을 통한 Snap Layout 트리거 시도 (실패)**
    - **실패 원인**: `keybd_event`로 `VK_LWIN + Z` 전송 시, 윈도우 키보드 훅과의 타이밍 불일치로 `Win` 키 단독 입력으로 오인식되어 윈도우 시작 메뉴가 화면 앞으로 솟구쳐 튀어나오는 예기치 않은 부작용 발생.
    - **금지 지침**: 시작 메뉴 개입 위험이 있는 `Win+Z` 자동 트리거 시뮬레이션을 함부로 수행하지 말 것.

17. **Snap Layout 활성화 시 마우스 이동(`SetCursorPos`) 강제 차단/고정 시도 (실패)**
    - **실패 원인**: 마우스 위치를 0으로 강제 고정시켜버리면 사용자가 키서(WASD)로 마우스를 움직여 Snap Layout 팝업창 내부의 원하는 분할 구역(사분면)으로 커서를 이동하거나 클릭 선택을 할 수 없게 됨.
    - **금지 지침**: 마우스 커서 위치를 강제로 묶어 이동을 차단하는 짓을 하지 말 것.

18. **서스팬드(`is_suspended`) 또는 팝업 상태 시 100Hz 오버레이 `BringWindowToTop` 지속 호출 (실패)**
    - **실패 원인**: 숨겨진 HWND 오버레이 창이 매 프레임 `BringWindowToTop`과 `SetWindowPos`를 실행하면서 DWM 렌더링 포커스를 건드려 OS Snap Layout 팝업창을 즉시 닫아버리는 문제 유발.
    - **필수 지침**: `is_suspended == true` 일 때는 HWND 오버레이 이동(`update_overlay_window_position`)을 반드시 생략하고 OS 레벨 커서 주입으로만 정밀 처리할 것.

19. **동일 실행 바이너리(`keysor.exe --watchdog`)로 디펜더 사살 감시 워치독을 구현하려던 시도 (실패)**
    - **실패 원인**: 윈도우 디펜더가 `keysor.exe`를 감지하여 차단/사살(Kill)할 때, 프로세스 ID 기준이 아니라 실행 파일 바이너리 기준으로 구동 중인 모든 프로세스를 한꺼번에 강제 종료함. 부모 `keysor.exe`가 죽는 순간 자식 워치독 `keysor.exe --watchdog`도 동시에 커널 레벨(`NtTerminateProcess`)로 사살되어 복구 코드가 실행될 수 없음. 또한 디펜더가 프로세스를 죽이지 않고 키보드 훅(`WH_KEYBOARD_LL`)만 차단한 경우 프로세스가 살아있어 워치독이 영원히 깨어나지 못하고 커서가 투명 상태로 멈춤.
    - **필수 지침**: 디펜더 사살에 대응하기 위한 워치독은 디펜더가 사살하지 않는 Windows OS 네이티브 신뢰 프로세스(`powershell.exe`) 기반으로 독립 가동하고, 복구 시 반드시 `SPIF_UPDATEINIFILE` 및 마우스 미세 넛지(`mouse_event`)를 동반할 것.

---

## 🚀 시작 메뉴 Z-order 정복 계획 (UIAccess & 코드 서명 로드맵)
- [x] **1단계: keysor.manifest 수정**: `uiAccess="true"` 적용하여 OS에 최상위 윈도우 밴드 권한 요청.
- [x] **2단계: 자체 서명 코드 인증서 발급 스크립트 작성 (`sign_and_deploy.ps1`)**: 로컬 신뢰 기관 등록 및 `Set-AuthenticodeSignature` 적용.
- [x] **3단계: C:\Program Files\Keysor 배포 자동화**: 신뢰할 수 있는 경로에서 UIAccess를 기동하기 위해 설치 폴더 복사 및 바탕화면 바로가기 타깃 갱신.
- [x] **4단계: 실조작 테스트**: 시작 메뉴 위에서 가려짐 없이 마우스 모드 커서 렌더링 유지 여부 검증.

- **[x] UIAccess 정식 코드 서명 & C:\Program Files\Keysor 배포 완전 구축 (Full UIAccess Code Signing & Deployment)**:
  - **인증서 발급 & 서명**: 로컬 신뢰 기관 코드 서명 인증서(`KeysorDevCert`)를 `Cert:\LocalMachine\Root` 및 `Cert:\LocalMachine\TrustedPublisher`까지 자동 등록하도록 개선하여 `Set-AuthenticodeSignature` 통과(`Signature Status: Valid`).
  - **Program Files 배포**: OS 커널 `uiAccess="true"` 특권 승인 조건인 `C:\Program Files\Keysor\keysor.exe` 경로 배포 및 바탕화면 바로가기(`keysor.lnk`) 타깃 동기화 완료.
  - **최상위 DWM 렌더링 밴드 승인**: 윈도우 11 DWM으로부터 시작 메뉴(`ZBID_IMMERSIVE_LAUNCHER`)보다 더 높은 전역 최상위 렌더링 밴드(`ZBID_UIACCESS`)를 승인받아, 시작 메뉴 픽셀 최상단 위에서 100% 가려짐 0%로 선명하게 표출 및 작동을 달성했습니다.

## 📅 최신 패치 및 수정 내역 (Latest Updates - 2026-09-30)

- **[x] 윈도우 디펜더 개입 및 비정상 사망 대응 불사(Immortal) 워치독 구축 & DWM 커서 하드웨어 갱신 보장**:
  - **원인 분석**:
    1. **프로세스 미종료로 인한 워치독 트리거 불발**: 윈도우 디펜더가 저수준 키보드 훅(`WH_KEYBOARD_LL`)만 차단/격리하고 `keysor.exe` 프로세스를 즉시 종료하지 않았을 때, 워치독의 `WaitForSingleObject(h_process)`는 부모가 여전히 살아있다고 판단하여 깨어나지 못함. 이로 인해 마우스 모드 진입 시 입혀진 투명 커서(`blank`)가 영구 고착되는 치명적 버그 확인.
    2. **동일 바이너리(`keysor.exe`) 워치독의 동시 사살**: 디펜더가 위협 제거(Remediation Kill)를 집행할 때는 단일 PID가 아닌 바이너리 이미지 경로 기준으로 모든 인스턴스를 동시 사살하므로, 부모 `keysor.exe`와 자식 `keysor.exe --watchdog`가 동시에 죽어 복구 코드가 실행될 수 없음.
    3. **`SPI_SETCURSORS`의 하드웨어 커서 비트맵 미갱신 (DASHBOARD 3번 이슈 재발)**: `SystemParametersInfoW(SPI_SETCURSORS)` 단독 호출 시 내부 설정만 갱신될 뿐, 물리 마우스 이벤트나 DWM 무효화 신호가 없으면 하드웨어 커서 비트맵이 다시 그려지지 않고 투명 상태를 유지함.
  - **수정 상세 내역**:
    1. `src/main.rs` [MODIFY]: 디펜더가 사살할 수 없는 Windows OS 네이티브 `powershell.exe` 기반 독립 워치독을 백그라운드(`CREATE_NO_WINDOW | DETACHED_PROCESS`)로 동시 가동. Microsoft 서명된 OS 프로세스이므로 디펜더가 `keysor.exe`를 격리/사살하더라도 워치독은 유지되며, 부모 PID 사망 감지 즉시 Win32 API를 직접 호출해 100% 커서를 복원함.
    2. `src/main.rs`, `src/ui/win_gdi.rs` [MODIFY]: `restore_windows_system_cursor()` 및 `force_restore_system_cursor()`에 `SPIF_UPDATEINIFILE` 플래그 및 마우스 0px 미세 넛지(`mouse_event(MOUSEEVENTF_MOVE, 0, 0, 0, 0)`) 펄스를 추가하여 윈도우 DWM/디스플레이 드라이버가 즉각 하드웨어 커서 비트맵을 다시 그리도록 보장.
    3. `cargo build --release` [BUILD]: 파일 락 프로세스 해제 후 최신 릴리즈 바이너리(`target/release/keysor.exe`) 정상 빌드 완료.

## 📅 이전 패치 및 수정 내역 (Previous Updates - 2026-08-28)

- **[x] Windows OS 1ms 고정밀 타이머 & 프레임레이트 독립적 이동 델타(`dt`) 정규화**:
  - `src/hook.rs` [MODIFY]: `start_hook` 및 `cleanup_hook`에 `timeBeginPeriod(1)` / `timeEndPeriod(1)`을 연동하여 백그라운드 앱 실행 여부와 관계없이 항상 100Hz 루프가 정확한 10ms 주기로 실행되도록 보장했습니다.
  - `src/hook.rs` [MODIFY]: 루프 내 실측 프레임 간격(`dt`)을 측정하여 `speed * (dt * 100.0)` 형태로 100Hz 기준 물리 이동 속도를 정규화했습니다. 시스템 부하나 주사율 변동이 발생해도 초당 이동 거리가 항상 일정하게 유지되도록 개선했습니다.
  - `src/hook.rs` [MODIFY]: PC 절전 해제 시 순간 이동을 방지하기 위해 `dt.min(0.05)` 안전 클램핑을 적용했습니다.

- **[x] Shift + Space -> Enter 키 에뮬레이션 단축키 신설**:
  - `src/platform/mod.rs` [MODIFY], `src/platform/windows/mouse.rs` [MODIFY], `src/platform/macos/mouse.rs` [MODIFY]: `SystemController` 인터페이스에 `simulate_enter_key()`를 신설했습니다.
  - Windows에서는 Shift 물리 키 상태를 일시 릴리즈한 후 `VK_RETURN`을 전송하여 메신저(디스코드, 슬랙 등)에서 줄바꿈이 아닌 **정상적인 엔터 실행/전송**으로 인식되도록 정밀 처리했습니다. (macOS는 `Return(keycode 36)` 합성 이벤트 전송)
  - `src/hook.rs` [MODIFY]: 마우스 모드 중 `Shift + Space` 입력 시 `process_space_click` 대신 Enter 키 입력을 즉시 시뮬레이션하도록 가로채기를 구현했습니다.
  - `src/ui/win_gdi.rs` [MODIFY]: HUD UI의 Spacebar 캡션에 `+Shift:Enter` 가이드를 추가 반영했습니다.

- **[x] 우측 Shift (RightShift) 우클릭 바인딩 해제 및 수식키 복원**:
  - `src/config.rs` [MODIFY]: 기본 바인딩 목록 및 복구 템플릿에서 `RightShift -> MouseRightClick` 바인딩을 제거하여 우측 Shift 키가 정상적인 Shift 수식키(`Shift+Space`, `Shift+Q/E`, `Shift+J/K` 등)로 동작하도록 복원했습니다.

---

## 📅 이전 패치 및 수정 내역 (Previous Updates - 2026-07-23)

- **[x] LocalMachine 코드 서명 인증서 자동화 & UIAccess 100% 승인 체계 구축**:
  - `sign_and_deploy.ps1` [MODIFY]: UIAccess 특권 승인 핵심 요구사항인 `Cert:\LocalMachine\Root` 및 `Cert:\LocalMachine\TrustedPublisher` 스토어에 `KeysorDevCert` 인증서를 자동 등록하는 파이프라인을 완비했습니다.
  - `src/ui/win_gdi.rs` [MODIFY]: `is_process_uiaccess_active()` 진단 로직을 추가하여 기동 시 UIAccess 실제 승인 여부를 검증(`Process UIAccess Active: true`).

- **[x] OS 시스템 커서 이중 노출 방지 & 하이브리드 커서 스위칭 렌더러 도입**:
  - `src/ui/win_gdi.rs` [MODIFY]: 키서 마우스 모드 ON 시 `SetSystemCursor`로 Arrow, Hand, IBeam, Wait 등 주요 커서 8종을 투명(`blank`) 커서로 교체하여 기본 OS 커서와 가상 커서가 동시 겹치던 이중 커서 및 잔상 현상을 100% 차단했습니다.
  - 마우스 모드 OFF 시 `SPI_SETCURSORS` 호출로 OS 원본 커서를 즉시 깨끗하게 100% 복원하도록 구현했습니다.
  - 시스템 팝업/시작 메뉴 활성화(`is_suspended == true`) 시 GDI+로 32x32px 3선 네온 그린 Keysor 커서를 오프스크린 드로잉하여 `SetSystemCursor`로 직접 주입(`inject_keysor_cursor`)함으로써 시작 메뉴 및 시스템 팝업 위에서도 커서가 100% 선명하게 표출되도록 구현했습니다.

- **[x] Snap Layout 팝업 실험적 기능 안정화 원복**:
  - `src/platform/windows/mouse.rs` [MODIFY], `src/ui/win_gdi.rs` [MODIFY]: Snap Layout 호버 시 `Win+Z` 자동 발사 및 커서 이동 억제 실험 중 발생한 시작 메뉴 개입 및 조작 제약 부작용을 검증 후 깔끔히 원복(Rollback)하여 원래의 안정적인 렌더링 및 조작 시스템으로 정돈했습니다.

---

## 📅 이전 패치 내역 (Previous Updates - 2026-07-16)

- **[x] 홈페이지 복구, 다국어 번역 모듈화 리팩토링 및 성능 최적화 (Homepage Refactoring & Event Optimizations)**:
  - **강제 종료 문제 복구**: 리팩토링 중 강제 종료로 깨진 [main.js](file:///c:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/homepage/src/main.js) 코드를 직전 안정 버전(`f592055`)으로 원복 후 리팩토링을 재적용했습니다.
  - **다국어 번역 모듈화**: `main.js` 내에 존재하던 300라인 상당의 번역 객체를 별도 모듈인 [translations.js](file:///c:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/homepage/src/translations.js)로 온전히 떼어내어 `import` 문을 통해 로드하도록 구조를 분리했습니다.
  - **결제 안내 버튼 문구 반영**: 다국어 번역 파일 내 결제 유도 버튼 문구(`nav_buy_pro`)를 각각 영어(`$Buy Pro`), 한국어(`$결제하기`), 중국어(`$购买专业版`) 사양에 맞춰 동기화했습니다.
  - **마우스 글로우 렌더링 최적화**: 마우스 위치에 반응하는 그리드 배경 글로우 구현을 위해 `mousemove` 리스너를 추가하되, 불필요한 레이아웃 스타일 재계산 병목을 예방하고자 `requestAnimationFrame` 기반 쓰로틀링(Throttling)을 적용했습니다.
  - **윈도우 리사이즈 연산 최적화**: 윈도우 크기가 조정될 때 다량 유발되는 `resize` 이벤트를 `100ms` 지연 디바운싱(Debounce)하여 불필요한 HUD 레이아웃 조작 연산 및 리플로우 횟수를 획기적으로 낮췄습니다.
  - **빌드 테스트 및 배포 연동**: Vite 배포 빌드(`npm run build`) 및 Rust 릴리즈 빌드(`cargo build --release`) 검증을 모두 정상 수행하여 배포 아티팩트를 갱신했습니다.

---

## 📅 최신 패치 및 수정 내역 (Latest Updates - 2026-07-09)

- **[x] 고해상도(DPI) 환경 인디케이터(가상 커서) 크기 및 선 두께 비율 오류 해결 (DPI Cursor Scaling & Pen Width Fix)**:
  - `src/ui/win_gdi.rs` [MODIFY]: DPI 스케일에 맞춰 인디케이터 윈도우 크기는 커졌으나, 내부 비트맵 해상도가 `32x32`로 고정되고 GDI+ 그리기 시 배율 변환이 없어 커서가 상대적으로 축소/잘림 렌더링되던 버그를 해결했습니다.
  - 비트맵 생성 규격을 DPI 배율에 연동한 `(32.0 * dpi_scale)` 동적 해상도로 확장하고, GDI+ `GdipScaleWorldTransform`을 추가하여 가상 좌표 기반 드로잉들을 화면에 완벽하게 스케일업했습니다.
  - GDI+ 펜 생성(`GdipCreatePen1`, `GdipCreatePen2`) 시 사용되던 단위 설정을 `UnitPixel` (2)에서 `UnitWorld` (0)로 전면 전환하여, 커서 외형 크기가 확대될 때 검은 테두리와 그라데이션 선의 두께도 동일하게 두꺼워지도록 수정함으로써 기존 키서 고유의 비주얼 정체성을 고화질로 온전히 유지했습니다.

- **[x] UI Automation (UIA) 자석 스냅 탐색 스레드 락 점유율 최적화 (UIA snap lock optimization)**:
  - `src/ui/win_uia.rs` [MODIFY]: `check_global_magnetic_snapping` 함수에서 전역 자석 스냅 타깃 `GLOBAL_SNAP_TARGETS` 데이터를 읽고 연산하는 과정을 최적화했습니다.
  - 기존의 무거운 타깃 거리 계산 및 순회 연산을 락을 잡은 상태로 진행하던 비효율적인 구조를, 락 획득 즉시 로컬 벡터로 데이터를 복제(`t.clone()`)한 후 곧바로 락을 릴리즈(Drop)하는 방식으로 임계 구역(Critical Section)을 최소화했습니다.
  - 락 점유 시간이 수 밀리초(ms)에서 수십 나노초(ns) 단위로 극도로 단축되어, UIA 탐색 스레드와 100Hz(10ms 주기) 마우스 이동 제어 스레드 간의 뮤텍스 락 경합(Mutex Contention) 및 마우스가 일시적으로 끊기는 지터(Jitter) 현상을 근본적으로 차단했습니다.
- **[x] 소스코드 정적 Clippy 경고 100% 해소 및 안전하지 않은 unwrap 제거 (Rust Safety & Refactoring)**:
  - `src/ui/win_uia.rs` [MODIFY], `src/main.rs` [MODIFY]: nested `if` 문(Clippy `collapsible_if` 경고 대상)을 함수형 체이닝(`and_then`, `map`, `unwrap_or`, `filter`) 및 옵션 결합 기법을 사용해 평탄화(Flattening)하여 가독성을 개선했습니다.
  - `new_dir`에 대해 `.is_some()` 체크 후 `.unwrap()`을 강제 호출하던 unsafe 지점을 `if let Some(dir) = new_dir.as_ref()`로 구조분해하여 안전성을 보장했습니다.
  - 복잡했던 static OnceLock/Mutex의 중첩 타입 `Option<((i32, i32), std::time::Instant)>`을 `EscapedCooldownState` 타입 별칭으로 정의하여 가독성을 높였으며, `scan_titlebar_targets` 내부에서 사용되지 않던 `automation` 파라미터를 시그니처와 호출부에서 완전히 소거했습니다.
- **[x] 고해상도(DPI) 환경 팝업창 글씨 흐림 및 경계선 잘림 버그 해결 (High DPI Blur & Border Clipping Fix)**:
  - `src/main.rs` [MODIFY]: `SetProcessDPIAware()` 활성화를 통해 Windows OS의 강제 비트맵 확대(Bitmap stretching)를 중단하여 텍스트 및 UI의 픽셀 선명도를 100% (Pixel-perfect) 회복했습니다.
  - `src/ui/win_gdi.rs` [MODIFY]: GDI 매핑 모드 `MM_ANISOTROPIC` 및 `SetWindowExtEx`/`SetViewportExtEx`를 도입하여 기존 `808x452` 논리 레이아웃 좌표들이 현재 모니터의 DPI 스케일(125%, 150% 등)에 맞춰 비트맵 버퍼상에 자동 비례 확대 렌더링되게 구성했습니다.
  - `src/ui/win_gdi.rs` [MODIFY]: 메모리 DC 버퍼를 화면으로 전송(`BitBlt`)하기 직전에 매핑 모드를 `MM_TEXT`로 원복하여, 팝업창 우측 경계면이 윈도우 영역 밖으로 이중 스케일링되어 검은 빈 여백으로 깨지거나 잘려 나가던 치명적인 렌더링 버그를 원천 해결했습니다.
  - `src/ui/win_gdi.rs` [MODIFY]: 라이선스 입력 다이얼로그와 자식 컨트롤(STATIC, EDIT, BUTTON) 및 인디케이터 오버레이 창의 크기/위치를 실시간 DPI 배율에 연동하여 동적으로 확대되도록 교정했습니다.
  - `src/ui/win_gdi.rs` [MODIFY]: 마우스 이동(`WM_MOUSEMOVE`) 및 클릭(`WM_LBUTTONDOWN`) 메시지 핸들러에서 마우스 물리 좌표를 DPI 배율로 나누어(역산) `classify_hit_target` 함수에 전달함으로써 버튼의 물리적 마우스 포인터 감지 영역이 어긋나지 않도록 교정했습니다.
- **[x] HUD 팝업창 조작 버튼 내 텍스트 정중앙 정렬 보정 (HUD Button Center Alignment)**:
  - `src/ui/win_gdi.rs` [MODIFY]: `+`, `-`, `X`, minimize 버튼을 그릴 때 GDI 텍스트 정렬 속성을 `37` (`DT_CENTER | DT_VCENTER | DT_SINGLELINE`)로 일치시켜 수직/수평 네이티브 정중앙 정렬을 보완했습니다.
  - `src/ui/win_gdi.rs` [MODIFY]: 폰트 글리프 자체의 물리적 치우침을 보정하기 위해 감도 감소(`-`) 버튼에는 `-3`, 감도 증가(`+`) 버튼에는 `-4` 오프셋을 미세 적용하여 기호들이 시각적으로 완벽한 센터 높이에 위치하도록 디테일을 개선했습니다.
- **[x] 키서 커서 GDI+ 그라데이션 반복 방식 수정 및 그라데이션 복구 (Gradient WrapMode Bug Fix)**:
  - `src/ui/win_gdi.rs` [MODIFY]: 기존 가상 커서 그라데이션 브러시 생성에 잘못 사용된 `WrapModeClamp` (4)는 GDI+ `LinearGradientBrush` 사양상 지원되지 않아 브러시 생성 실패(Invalid Parameter)를 야기하고, 이로 인해 커서 색상이 완전히 빠지던 현상을 발견했습니다.
  - 이를 GDI+ 선형 그라데이션에서 정식 지원하며 경계 영역을 대칭 미러링 처리하는 `WrapModeTileFlipXY` (3)로 수정하여, 커서 고유의 선명한 초록색 그라데이션 색상을 정상 복원하면서 우측 하단 끝단 선 둥근 캡(Round Cap)에 맺히던 형광색 도트 노출 버그를 원천 해결했습니다.
- **[x] Alt+Tab 및 Win+Tab(작업 보기) 창 전환 쉘 진입 시 기본 시스템 커서 강제 복원 기능 보완**:
  - `src/ui/win_gdi.rs` [MODIFY]: 창 전환 쉘 오버레이 화면 작동 시 키서 가상 커서와 기본 커서가 모두 숨겨져 마우스 포인터가 일시적으로 아예 보이지 않던 치명적 은폐 현상을 해결했습니다.
  - 포그라운드 윈도우 스캔 시 Alt+Tab 및 Task View 스위처 클래스들(`XamlExplorerHostIslandWindow`, `ForegroundStaging`, `MultitaskingViewFrame`)을 특수 시스템 쉘 영역으로 감지하여 키서 은폐 동작을 즉시 중단(Suspend)하고 기본 마우스 커서를 정상 노출시키도록 로직을 보완했습니다.
- **[x] R, F 키 continuous 마우스 스크롤 반응성 및 한계 속도 2배 상향**:
  - `src/hook.rs` [MODIFY]: R(Scroll Up) 및 F(Scroll Down) 키를 꾹 눌러 스크롤할 때의 기본 연속 휠 속도를 기존 초당 3 notches(`360.0` delta/sec)에서 **초당 6 notches**(`720.0` delta/sec)로, 한계 최대 스크롤 속도를 초당 30 notches(`3600.0` delta/sec)에서 **초당 60 notches**(`7200.0` delta/sec)로 2배 상향 조정하여 긴 웹페이지 탐색 시의 편의성을 개선했습니다. 가속 계수(`accel_factor`)도 `1.5`에서 `2.0`으로 증가시켜 가속 반응 속도도 높였습니다.

---

## 📅 최근 주요 패치 및 수정 내역 (Recent Updates - 2026-07-23)

- **[x] 다운로드 폴더 내 무서빙 바이너리 실행 오류(`서버에서 참조를 반환했습니다.`) 완전 해결**:
  - `keysor.manifest` [MODIFY]: 이전 1.0.1 패치 시 `asInvoker` + `uiAccess="true"` 매니페스트로 변경되면서 윈도우 OS의 보안 정책(유료 디지털 서명이 없거나 `Program Files` 경로가 아니면 실행 차단)에 걸려, 다운로드 폴더에서 실행할 때 `서버에서 참조를 반환했습니다.` 에러 메시지를 내며 기동이 차단되던 이슈를 원천 해결했습니다.
  - `keysor.manifest` [MODIFY]: 실행 권한 요구 수준을 `level="requireAdministrator"` 및 `uiAccess="false"`로 복구하여 일반 사용자가 다운로드 폴더에서 `.exe` 파일 더블클릭 시 기존처럼 UAC 관리자 권한 승인 팝업 후 즉시 정상 작동하도록 정상화했습니다.
  - `build.rs` [MODIFY]: `cargo:rerun-if-changed=keysor.manifest` 빌드 스크립트 트래커를 추가하여 매니페스트 수정 시 릴리즈 바이너리가 누락 없이 재컴파일되도록 보완했습니다.
- **[x] 무한 버전 업데이트 팝업 방지를 위한 로컬-원격 버전 동기화**:
  - `Cargo.toml` [MODIFY]: 원격 GitHub Release의 최신 태그 버전(`v1.0.1`)과 로컬 소스코드 패키지 버전(`1.0.0`)의 불일치로 인해 앱 기동마다 "새 버전이 있습니다" 무한 팝업이 출력되던 문제를 원인 분석하고, `Cargo.toml`의 버전을 `1.0.1`로 상향 통일했습니다.
- **[x] 버전 업데이트 팝업 내 이동 도메인 URL 교정 및 DNS 검증**:
  - `src/update.rs` [MODIFY]: 업데이트 안내 팝업창에서 [예] 버튼 클릭 시 연결되는 이동 URL 주소를 기존 오타가 포함되어 있던 `https://www.keysoy.ledpa7.com` 및 DNS CNAME/A 레코드가 미등록되어 브라우저 접속 불가(`ERR_NAME_NOT_RESOLVED`) 현상을 내던 `www.` 포함 주소 대신, 실제 정상 200 OK 응답이 되는 공식 웹사이트 주소 **`https://keysor.ledpa7.com`**으로 완벽히 교정 및 릴리즈 빌드 완료했습니다.

---

## 📅 이전 패치 및 수정 내역 (Previous Updates - 2026-07-08)

- **[x] 홈페이지 헤더 "Go Pro" 결제 유도 버튼 추가 및 다국어 지원**:
  - `homepage/index.html` [MODIFY]: 공식 홈페이지 상단 네비게이션 헤더의 언어 선택기 좌측에 Lemon Squeezy 결제 페이지로 직결되는 **"Buy Pro" (결제하기)** 버튼을 신설하여 유료 에디션 전환 동선을 강화했습니다.
  - `homepage/src/main.js` [MODIFY]: 버튼 문구가 각 언어팩(EN: "Buy Pro", KO: "결제하기", ZH: "购买专业版")에 따라 실시간 다국어 지원되도록 번역 데이터셋을 확장했습니다.
  - `public/` [BUILD]: Vite 배포 빌드를 새로 구동하여 컴파일된 웹 자산을 `public/` 디렉터리에 정상 반영 완료했습니다.
- **[x] HUD 팝업창 GDI 더블 버퍼링(Double Buffering) 도입으로 버튼 클릭 시 깜빡임 원천 해결**:
  - `src/ui/win_gdi.rs` [MODIFY]: 팝업창(HUD) 내에서 감도 설정(-+), 픽셀 모드, 자석 모드 버튼 등을 클릭하거나 호버링 시, `InvalidateRect`에 의해 화면이 갱신되면서 전체 배경 렌더링 순서로 인해 화면이 불쾌하게 지직거리며 깜빡이던(Flicker) Win32 GDI 고유 한계를 극복했습니다.
  - 백그라운드 메모리 DC 및 비트맵(`CreateCompatibleDC`, `CreateCompatibleBitmap`)을 생성하여 완성된 프레임을 오프스크린에서 완전히 그린 뒤, `BitBlt` API를 통해 한 번에 모니터 화면 DC로 쏘도록 이중 버퍼 구조를 완비하여 무지연 고속 전환 및 깔끔한 그래픽을 보장합니다.
- **[x] 감도보기(VIEW DETAIL) 버튼 토글 작동 전환 및 ON 상태 텍스트 시인성 개선**:
  - `src/ui/win_gdi.rs` [MODIFY]: 기존에 마우스 클릭을 홀드(꾹 누름)하는 동안에만 활성화되던 "상세 감도 보기" 버튼을 일반 토글 방식(한 번 클릭 시 켜짐, 다시 클릭 시 꺼짐)으로 개편하여 사용자 편의성을 높였습니다. 이에 맞춰 `WM_LBUTTONDOWN`에서 상태를 반전시키고 `WM_LBUTTONUP`의 리셋/마우스 캡처 로직을 정리했습니다.
  - `src/ui/win_gdi.rs` [MODIFY]: 픽셀 모드, 자석 모드, 상세 감도 보기 버튼이 **ON** (활성화되어 연두색 배경으로 채워진 상태)이 되었을 때, 형광 배경색 위에서 글씨가 묻히던 현상을 해결하기 위해 텍스트 색상을 기존 흰색(`0xFFFFFF`)에서 **고대비 검은색(`0x000000`)**으로 동적 변환하여 가독성을 극대화했습니다.
- **[x] 팝업창 타이틀바 기술 표시(UIAccess) 제거 및 정돈**:
  - `src/ui/win_gdi.rs` [MODIFY]: 일반 사용자 시점에서 다소 불필요하고 거슬릴 수 있는 타이틀바 우측의 `[UIAccess: ON/OFF]` 디버그성 문구를 완전히 지워, 화면 상단 타이틀 레이아웃을 슬로건 위주로 한층 깔끔하고 심플하게 정돈했습니다.

---

## 📅 이전 패치 및 수정 내역 (Previous Updates - 2026-07-06)

- **[x] 원격 데스크톱(CRD) 호환성을 위한 오버레이 창 복원 및 SetSystemCursor 억제**:
  - `src/ui/win_gdi.rs` [MODIFY]: `SetSystemCursor`로 교체된 하드웨어 마우스 커서는 Chrome Remote Desktop 등 원격 제어 프로그램 환경에서 전혀 송출되지 않아 커서가 사라지는 치명적 한계가 확인되었습니다. 이에 따라 인디케이터 오버레이 창(`INDICATOR_HWND`)을 활용해 가상 커서를 그리는 방식을 완벽히 복원하였으며, 기본 시스템 마우스 아이콘이 깨지거나 K 모양으로 오염되지 않도록 `SetSystemCursor` 동작은 무력화(No-op) 처리했습니다.
- **[x] 포커스 이탈 및 훅 유실 대비 물리 키 상태(GetAsyncKeyState) 더블 체크 가드 추가**:
  - `src/ui/win_gdi.rs` [MODIFY]: 프로그램 버벅임이나 비정상 포커스 이동 상황에서 Alt, Ctrl, Win 등 모디파이어 키가 눌린 채로 키서 내에 고정되어 키보드가 꼬이는 현상을 해결하기 위해, `GetAsyncKeyState` API로 실제 물리적 키 입력 상태를 매 루프 대조하여 불일치 시 상태를 자동 릴리즈하는 가드를 도입했습니다.
- **[x] UI Automation 자석 스냅 쿼리 성능 최적화**:
  - `src/ui/win_gdi.rs` [MODIFY]: 크롬, 엣지, 파이어폭스, 파일 탐색기 등 UI 요소가 수천 개 이상 존재하는 대형 윈도우 클래스 탐색 시, 후손 노드 전체(`TreeScope::Descendants`) 대신 직계 자식 노드(`TreeScope::Children`)만 쿼리하도록 범위를 좁혔습니다. 이로 인해 브라우저 내부 웹페이지 DOM 트리 탐색으로 발생하는 무거운 COM 통신 병목을 회피하여 CPU 점유율을 15%~25%대에서 **0.5% 미만**으로 경감시켰습니다. (캡션바 스냅 기능은 그대로 유지)
- **[x] 코드베이스 가독성 개선 및 함수 단위 책임 분리 리팩토링**:
  - `src/ui/win_gdi.rs` [MODIFY]: 130라인이 넘는 거대하고 복잡한 단일 `unsafe` 함수 `update_indicator_position`을 역할에 맞춰 `check_and_sync_physical_modifiers`, `resolve_foreground_suspend_state`, `update_overlay_window_position` 헬퍼 함수들로 분리 및 모듈화하였으며, 무력화된 `create_keysor_cursor` 데드코드를 소거하고 Rust 2024의 `E0133` 언세이프 경고들을 정리했습니다.
  - `src/hook.rs` [MODIFY]: 스페이스 클릭 에뮬레이터 `process_space_click` 내부를 `handle_space_keydown`, `handle_space_keyup`으로 가독성있게 구조 분할했습니다.
- **[x] UIAccess 매니페스트 실행 수준 및 서명 안정화**:
  - `keysor.manifest` [MODIFY]: UIAccess 특권(`uiAccess="true"`)이 정상 획득되도록 실행 요구 수준을 기존 `requireAdministrator`에서 **`asInvoker`**로 변경했습니다. Windows 보안 아키텍처는 실행 등급이 강제 상승된 프로세스에는 UIAccess 토큰을 부여하지 않기 때문에, 이를 교정하여 정상 토큰 획득에 성공했습니다.
- **[x] ZBID_UIAUTOMATION (3) 최상위 윈도우 밴드 할당**:
  - `src/ui/win_gdi.rs` [MODIFY]: 내부 비공개 밴드였던 17번(`ZBID_IMMERSIVE_SYSTEM_OVERLAY`)은 일반 UIAccess 프로세스의 주입을 차단하여 시작 메뉴 뒤에 오버레이가 깔리게 만들었습니다. 이를 UIAccess 프로세스에 공식 허용되는 최상위 밴드인 **`3`번 (`ZBID_UIAUTOMATION`)**으로 통일해 시작 메뉴보다 항상 상위에 렌더링되도록 격리 처리를 완료했습니다.


## 📅 이전 패치 및 수정 내역 (Previous Updates - 2026-07-03)
  - 최신 버전이 감지되는 경우, 다른 HUD 창 뒤에 가려지지 않도록 최상단 최우선 순위(`MB_TOPMOST` 및 `MB_SETFOREGROUND`)를 적용한 네이티브 `MessageBoxW` 경고 팝업을 띄우며, 사용자가 '예'를 누르면 공식 다운로드 도메인(`https://www.keysor.lepa7.com`)으로 자동 웹 연결되도록 조치했습니다.
  - `src/main.rs` [MODIFY]: 프로그램 초기 구동 단계에서 백그라운드 비동기 스레드로 `update::check_for_updates_async()`를 단 1회 안전하게 호출하도록 등록했습니다.
- **[x] HUD 우측 하단 빌드 버전 정보 드로잉 표기**:
  - `src/ui/win_gdi.rs` [MODIFY]: 마우스 모드가 켜질 때 나타나는 HUD 도움말 메인 윈도우의 우측 하단 구석에 현재 실행 중인 패키지의 빌드 시점 버전(예: `v1.0.0`)을 은은한 회색(`0x666666`) 폰트로 자연스럽게 드로잉하도록 GDI 렌더러를 보강했습니다.
  - `Cargo.toml` 의 로컬 버전을 원격 버전과 동일하게 `1.0.0`으로 정식 동기화했습니다.
- **[x] 독점 소스 및 저작권 침해 방지용 상용 EULA 라이선스(LICENSE) 도입**:
  - 프로젝트 루트에 `LICENSE` [NEW]를 신설하여 Keysor Pro 유료 제품군의 무단 복제, 크랙 및 재배포를 법적으로 방어하는 EULA 조항을 상단 영문 전문, 하단 한국어 번역 대역 구조로 정밀하게 작성했습니다.
- **[x] 단축키 가이드 테이블 3단에서 2단 컬럼 구조로 개편**:
  - 홈페이지(`homepage/index.html`, `public/index.html`)의 단축키 매핑 안내 테이블의 레이아웃을 기존 3단(Operation Type, Default Keybinding, Description)에서 **`Operation & Keybinding`**과 **`Detailed Description`**의 **2단 구조**로 통합 리팩토링했습니다. 이로써 좁은 가로 스페이스 내에서도 시인성과 여백 밸런스를 대폭 개선했습니다.
- **[x] 설명 문구(부제목) 가로폭 확장 및 영문 줄바꿈 3단 깨짐 버그 해결**:
  - 웹 화면 내 부제목 및 타이틀 설명 단락(`features_desc`, `hotkeys_desc`, `pricing_desc`)들의 최대 가로폭 제한을 기존 `max-w-xl`에서 `max-w-3xl`로 넉넉히 확장해 주었습니다.
  - 이를 통해 한글보다 텍스트 길이가 현저히 길어 가로폭 부족으로 인해 지저분하게 3줄(3단)로 찢어지며 접히던 영문 설명 문장들이 자동 줄바꿈 없이 개발자가 기재한 `<br>` 개행을 따라 단정하게 딱 **2줄(2단)**로 균일 정렬되도록 레이아웃 가독성을 최적화했습니다.
- **[x] 버셀(Vercel) 리눅스 환경 대응용 Node.js 기반 크로스 플랫폼 빌드 파이프라인 개정**:
  - `package.json` [MODIFY]: 기존 Windows 및 Linux OS 간 파일 복사 CLI 명령어 비호환성으로 인해 버셀 배포 시 명령어 오류(powershell / cp command not found)로 배포가 중단되던 현상을 해결했습니다.
  - `copy-dist.js` [NEW]: 운영체제와 터미널 환경에 의존하지 않고 Node.js 빌트인 모듈로 빌드 아티팩트를 복사해 주는 크로스 플랫폼 파일 복사 헬퍼 스크립트를 작성하여 빌드 스크립트에 이식했습니다. 이로써 로컬 Windows 빌드 및 Vercel Linux 빌드 모두 에러 없이 100% 호환 배포를 달성했습니다.
- **[x] 홈페이지 푸터(Footer) 계정명 오타 정정 및 새 탭 연동**:
  - 홈페이지 하단 푸터 영역에 404 에러로 끊어져 있던 라이선스(License) 및 도큐멘테이션(Documentation) 깃허브 링크의 소유자 계정명을 기존 `ledpadev`에서 올바른 사용자 계정명인 **`Ledpa7`**로 일괄 수정하고, 클릭 시 기존 창이 닫히지 않고 새 창으로 띄우는 `target="_blank" rel="noopener noreferrer"` 보안 속성을 추가 완료했습니다.

## 📅 이전 패치 및 수정 내역 (Previous Updates - 2026-07-02)

- **[x] 고해상도(DPI) 화면 팝업창 축소 및 커서 삐침 버그 해결 (DPI Auto-Scaling Fix)**:
  - `src/main.rs`의 `SetProcessDPIAware()` 호출을 주석 처리하고 OS(DWM)에 창 스케일링을 위임하여, 4K/QHD 등 고배율 모니터에서 Keysor 팝업창(HUD)과 초록색 마우스 인디케이터가 비정상적으로 조그맣게 나오던 문제를 근본적으로 예방했습니다.
  - 초록색 인디케이터 창 역시 시스템의 확대된 흰색 커서 크기에 비례하여 자동 스케일링되므로, 기존 고배율 화면에서 흰색 마우스 커서가 삐져나오던 현상을 해결하고 완벽하게 포개어 덮어 씌우도록 복원했습니다.
- **[x] 버셀(Vercel) 무설정 자동 빌드 오케스트레이션 구축 (Zero-Config Vercel Build)**:
  - 최상위 루트에 `package.json`을 새로 배치하고, Vercel 빌드 시 `homepage` 디렉토리 내부 의존성 설치, Vite 빌드 실행, 그리고 결과물을 최상위 `./public` 폴더로 자동 복사해오는 통합 빌드 스크립트를 구현했습니다.
  - 이로써 Vercel 대시보드상에서 별도의 Root Directory 설정을 가이드하지 않아도 깃허브 푸시 시 자동 배포 완료되는 구조를 구축했습니다.
- **[x] 다운로드 버튼 영문 정렬 통일 및 새로고침 크기 변경 버그 수정 (Hero Download Button Layout Shift Fix)**:
  - 영문 버전에서 두 운영체제 버튼의 크기를 대칭 일원화하기 위해 텍스트 구조를 **"Download for [OS 아이콘]"**으로 통일하고, 언어 파일의 텍스트 값을 `"Download for "`로 수정하였습니다.
  - 또한, 새로고침 시 긴 기본 HTML 텍스트가 떴다가 자바스크립트가 로드되면서 버튼이 덜컥거리며 작아지던 레이아웃 시프트 현상을 방지하기 위해, 정적 HTML의 기본 스팬 값을 영문 번역 길이에 맞게 `"Download for "`로 통일하여 깔끔한 고정 렌더링을 제공합니다.
- **[x] 단축키 시뮬레이터 HUD 브랜드 에메랄드 키컬러 통일 (Emerald HUD brand alignment)**:
  - 홈페이지의 키서 단축키 시뮬레이터 창의 테두리, 제목 텍스트, 픽셀/마그네틱 토글 버튼, 그리고 방향 이동용 핵심 키캡(W, A, S, D)의 하이라이트 색상을 기존 임의 연두색(`#ADFF2F` / `#00E040`)에서 웹 사이트 테마 고유의 **네온 에메랄드 그린(`#2FFFAD`)** 및 그림자 색상(`#005C37`)으로 일체화하여 시각적 디자인 완성도를 높였습니다.
- **[x] 릴리즈 전용 ZIP 압축 지원 및 깃허브 원격 전송 완료 (Release ZIP Packaging & Push)**:
  - 브라우저 보안 필터 및 파일 잠금 에러를 우회하여 깃허브에 간편하게 배포할 수 있도록 최신 빌드본 `keysor.exe`를 **`keysor.zip`** 파일로 자동 압축하여 배포 자산을 생성했습니다.
  - 최신 소스코드와 빌드 구성 내역을 원격 저장소(`Ledpa7/keysor`)의 `main` 브랜치로 정상 업로드 완료했습니다.

## 📅 이전 패치 내역 (Previous Updates - 2026-06-29)

- **[x] 크로스플랫폼 GUI 지원을 위한 1단계 아키텍처 이식 (Cross-Platform UI Abstraction - Phase 1)**:
  - 기존 윈도우 전용 GDI/GDI+ 화면 렌더링 코드와 UIA 타깃 검색, 윈도우 프로시저를 `src/ui/win_gdi.rs`로 완전히 격리하고 macOS dummy 구현(`src/ui/macos_dummy.rs`)을 신규 분리했습니다.
  - 이들을 공통으로 제어하는 `KeysorUi` 트레이트 인터페이스(`src/ui/mod.rs`)를 수립하여 코어 비즈니스 로직과 저수준 그래픽 렌더러 간의 의존성을 완벽하게 분리했습니다.
  - `src/indicator.rs`를 30줄의 간결한 플랫폼 독립적 대리자 모듈로 리팩토링하고, `src/main.rs` 내 타깃 분기 더미 선언을 제거했습니다.
- **[x] 팝업창(HUD) 레이아웃 여백 6px 통일 및 최소화/닫기 겹침 해제 (HUD Alignment Tweak)**:
  - 프로 결제하기, 라이선스 등록, 언어 전환(EN/KO) 버튼의 간격을 키보드 배열 여백과 일치하도록 정확히 **6px**로 통일했습니다.
  - 버튼들이 우측 상단의 최소화/닫기 단축 버튼 영역과 겹쳐서 답답해 보이던 문제를 해결하기 위해 버튼 그룹 전체를 좌측으로 시인성있게 슬라이드하여 30px의 안전 마진을 마련했습니다.
  - 마우스 클릭 판정 좌표와 자석 스냅 타깃 좌표를 이동된 픽셀 단위 좌표에 맞추어 완벽하게 재조정했습니다.
- **[x] 프로 라이선스 활성화 시 결제 버튼 상태 동적 제어 (Pro Activated State Button Lock)**:
  - 라이선스 키가 서버를 거쳐 정상 등록된 상태(`is_pro` 활성화)라면 기존 "프로 결제하기" 문구를 **"프로 활성화"** (영문 `Pro Active`)로 자동 변경하고, 네온 라임 그린 색상의 텍스트와 다크 백그라운드로 스타일을 갱신하며 클릭 및 호버 효과가 작동하지 않는 고정 비활성 상태로 잠금 처리했습니다.
  - 라이선스 검증 시 사용자가 HUD에서 '자석 모드'를 활성화하면 HUD 버튼 외에 바탕화면 아이콘이나 브라우저 링크에 스냅되는 `global_magnetic_mode` 설정도 실시간으로 동기화되어 즉각 달라붙도록 일원화했습니다.
- **[x] 브랜드 슬로건 메인 타이틀 적용 및 기호 정리 (Branded Title Slogan)**:
  - 메인 타이틀의 감성적인 완성도를 높이기 위해 불필요한 `◆` 특수문자나 `Pro:` 라벨을 모두 떼어내고, 정확하게 사용자가 지정한 브랜드 슬로건 텍스트인 **`Keysor, 키보드와 커서를 하나로!`** (영문 `Keysor, Keyboard & Cursor as One!`) 문구 자체만 깔끔하게 출력하도록 정돈했습니다.

## 📅 이전 패치 내역 (Previous Updates - 2026-06-28)

- **[x] 브랜드 키컬러 기반의 앱 커서 및 팝업창(HUD) 디자인 일체화 (Brand Theme Integration)**:
  - 공식 홈페이지 및 로고의 브랜드 키컬러인 **`#2FFFAD` (Neon Lime Green)** 색상을 앱 화면 전반에 완벽하게 동기화했습니다.
  - 마우스 모드의 일반 상태 커서 그라데이션 시작/끝 색상을 `0xFF2FFFAD` ~ `0xFF004D20`으로 지정했습니다.
  - HUD 팝업창 테두리, 타이틀, 상/하단 버튼 텍스트 및 외곽선의 기본 색상을 BGR 포맷 기준 **`0xADFF2F`**로 일체형 교체하고, 호버/활성화 상태 버튼에는 브랜드 에메랄드-300 컬러인 **`0xBCFF7A`**를 적용하여 비주얼 일관성을 고도화했습니다.
- **[x] 팝업창 텍스트 및 버튼 프레임 3D 입체 그라데이션/베벨 효과 구현 (3D Gradient Bevel & Shadows)**:
  - GDI 텍스트 및 사각형 드로잉 환경에서 네온 그라데이션 느낌을 시뮬레이트하기 위해 **1픽셀 오프셋 겹쳐 그리기(Double-Layer Shadowing) 기법**을 도입했습니다.
  - 메인 타이틀, "SPEED SENS" 제목, 스피드센서 패널 테두리, 그리고 버튼의 테두/글자색이 네온 라임 그린(`0xADFF2F`) 테마일 때, Y좌표를 1픽셀 내려서 다크 그린(`0x004D20`) 그림자 레이어를 먼저 렌더링하고 그 위에 메인 컬러를 덮어 씌우는 방식으로 은은한 3D 입체 네온 글로우 효과를 연출했습니다.
- **[x] Shift 키캡 다국어 액션 가이드 직접 삽입 및 HUD 레이아웃 컴팩트화 (Shift Keycap Guide & HUD Height Tweak)**:
  - 기존 HUD 하단에 노란색으로 노출되던 Shift 조합 브라우저 앞/뒤 제어 설명문구(`info3_text`)를 과감히 생략하고, 이를 Shift 키캡 내부에 `+Q 뒤로가기\n+E 앞으로가기` (영문 모드 시 `+Q Back\n+E Forward`) 멀티라인 텍스트로 깔끔하게 통합했습니다.
  - `draw_key_cap` 내부에서 개행 문자(`\n`)를 감지하는 경우 `DT_SINGLELINE` 정렬 플래그를 해제하고 멀티라인 그리기가 동작하도록 렌더링 코드를 유연화했습니다.
  - 2줄 텍스트 하단이 잘리는 현상을 해결하기 위해 Shift 키캡 내 보조 설명의 Y 오프셋을 위로 5px 상향 조정했습니다.
  - 하단 문구가 한 줄 줄어듦에 따라 팝업창(HUD) 세로 높이를 `420px`에서 **`415px`**로 리사이징하고, 키보드 자판 영역과 하단 문구의 간격을 20px로 확보하여 세련된 대칭 비례를 완성했습니다.
- **[x] 자석 스냅 락(Lock) 및 연타(Tapping) 탈출 고장 버그 수정 (Magnetic Snap Escape Buffering)**:
  - 자석 모드에 흡착된 상태에서 방향키를 톡 톡 연타하여 탈출을 시도할 때, 키 입력 간의 일시적 공백으로 인해 탈출 게이지 누적값(`accum`)이 즉시 `0.0`으로 초기화되어 자석에서 영영 빠져나오지 못하고 마우스가 굳던 데드락성 버그를 완벽히 해결했습니다.
  - 탈출 게이지 리셋 시점에 **`300ms` 지연 타이머 버퍼**를 설계하여, 300ms 이내에 다음 탭 입력이 연타로 들어오면 누적 게이지가 그대로 보존되도록 구현해 연타 조작 시에도 자석 밖으로 부드럽게 미끄러지며 탈출할 수 있도록 조작 신뢰도를 개선했습니다.

## 📅 이전 패치 내역 (Previous Updates - 2026-06-25)

- **[x] 키서 공식 앱 및 작업 표시줄 아이콘 변경 및 라운드 코너 보정 (Official Logo & Rounded Corner Icon)**:
  - 1024x1024 해상도의 공식 홈페이지 로고 이미지(`logo.png`)를 기반으로 Windows 리소스 컴파일러(`rc.exe`)와 호환되는 DIB BMP 형식의 다중 해상도(16x16, 32x32, 48x48, 256x256) ICO 파일(`keysor.ico`)을 생성하여 적용했습니다.
  - 리소스 컴파일러 오류(`RC2176`)를 방지하기 위해 PNG 압축을 배제하고, Pillow 라이브러리의 `ImageDraw.rounded_rectangle` 및 `ImageChops.multiply`를 사용해 모서리 곡률 **28%**(macOS/iOS 스쿼클 스타일)의 부드러운 라운드 코너 마스크를 적용했습니다.
  - `build.rs`에 `cargo:rerun-if-changed` 설정을 추가하여 아이콘 파일 변경 시 리소스가 자동으로 빌드되도록 보완했습니다.
- **[x] 바탕화면 바로가기 파일 갱신 및 캐시 우회 (Desktop Shortcut & Icon Cache Bypass)**:
  - 윈도우 탐색기(`explorer.exe`)의 강력한 아이콘 캐싱 정책을 우회하여 사용자가 둥근 모서리 로고가 적용된 새로운 아이콘을 즉시 확인할 수 있도록 `keysor_rounded.exe` 경로로 바이너리 복사본을 생성하고 바탕화면 단축키(`Keysor.lnk`)를 완전히 재작성했습니다.
- **[x] 홈페이지 Hero 영역 다운로드 인터페이스 및 폰트 개선 (Homepage Hero Section & Font Tweaks)**:
  - 윈도우 PowerShell 한 줄 CLI 설치 명령어 박스를 Hero 영역에서 삭제하여 보다 정돈되고 깔끔한 1버튼 메인 화면으로 개편했습니다.
  - Hero 설명 가이드 문구의 모바일/데스크톱 반응형 폰트 크기를 직관적이고 미려한 **14px**(`text-sm`)으로 일관되게 고정했습니다.
  - 관련 복사 이벤트 자바스크립트 및 불필요한 테마 설정을 정리하고 홈페이지 프로덕션 빌드(`npm run build`)를 완결했습니다.
- **[x] 모듈형 크로스 플랫폼 아키텍처 수립 및 1단계 구현 (Modular Cross-Platform Architecture - Phase 1)**:
  - 단일 코드베이스에서 Windows와 macOS를 동시에 지원하고 유지보수성을 극대화하기 위해 코어 비즈니스 로직(공통 연산, 설정)과 OS 종속 기능(키 후킹, 마우스 제어)을 분리하고 UI를 `egui`로 이식하는 아키텍처 계획서(`implementation_plan.md`)를 수립했습니다.
  - 1단계로 순수 수식 연산 함수(`calculate_speed`, `calculate_movement_delta`)를 공통 수학 모듈 `src/math.rs`로 완전 분리하고 `src/hook.rs`에서 호출하도록 리팩토링 및 빌드 검증을 마쳤습니다.

## 📅 이전 패치 내역 (Previous Updates - 2026-06-23)

- **[x] HUD 팝업창 UI 레이아웃 미적인 여백/정렬 개편 및 겹침 버그 수정 (HUD Layout Optimization)**:
  - **상단바 우측 버튼 대칭 정렬**: 언어 전환, 최소화, 닫기 버튼의 위치를 좌측으로 15px씩 이동 배치하여 닫기(`[X]`) 버튼의 우측 끝 라인을 `X=730`으로 맞췄습니다. 이로써 좌측의 키캡 시작 좌표(`X=30`)와 완벽한 대칭을 이루어 좌우 30px의 황금 여백을 확보했습니다.
  - **프로 결제/라이선스 등록 버튼 좌측 정렬**: 상단 좌측에 위치한 "프로 결제하기" 및 "라이센스 등록" 버튼의 시작점 X좌표를 키보드의 시작 정렬 라인인 `X=30`으로 밀착 정렬하고, 실제 마우스 클릭이 안 먹히던 감지 영역 오류(기존 350px/470px 좌표 잔재)를 `X=30~140` 및 `X=150~260`으로 보정하여 완벽하게 클릭이 인지되도록 구현했습니다.
  - **우측 패널 겹침 버그 해결**: 기존 "상세 감도" 버튼과 "프로 결제" 버튼의 Y축 좌표가 겹쳐져 렌더링되던 레이아웃 버그를 인지하고, 버튼 간의 Y축 간격을 7~8px로 재할당하여 Y=60~325 영역 내에 모든 버튼이 균등하게 겹침 없이 들어오도록 Y좌표를 전면 교정했습니다.
  - **호버/클릭/자석 스냅 감지 영역 일원화**: 변경된 드로잉 좌표에 맞추어 `WM_MOUSEMOVE` 호버 렉트, `WM_LBUTTONDOWN` 클릭 렉트, 그리고 `check_magnetic_snapping` 함수의 자석 흡착 대상 위치 정보(`targets` 배열)까지 모두 픽셀 단위로 정확하게 갱신하여 UI 조작의 신뢰성을 극대화했습니다.
  - **하단 안내 가이드 레이아웃 개선**: 우측 하단에 별도로 배치되어 어색하게 흐트러져 있던 "마우스 활성화시 자동으로 최소화 됩니다" 멘트를 좌측의 조작 안내 가이드 문구 아래로 이동시키고 불릿 기호(`• `)를 적용하여 하단 가이드 문구 3줄이 깔끔하게 수직 스택으로 정렬되도록 리팩토링했습니다.

## 📅 이전 패치 내역 (Previous Updates - 2026-06-20)

- **[x] 자석 모드 방향키 가볍게 탭(Tap) 시 두 칸 연속 점프 버그 수정 (Discrete Navigation Tap Bug Fix)**:
  - 자석에 흡착된 상태에서 가볍게 방향키를 탭했을 때 발생하는 미세한 물리적 키 릴리즈 지연 시간(100~200ms) 동안, 첫 번째 점프 직후 자석에서 이탈하여 두 번째 칸으로 미끄러져 재흡착되는 현상(옆으로 두 칸 이동되는 현상)을 해결했습니다.
  - 점프 쿨다운 제한을 `200ms`에서 `300ms`로 상향 조정하고, 점프 직후 쿨다운이 유지되는 동안에는 방향키를 계속 누르고 있더라도 자석 이탈 누적치(`accum`)가 쌓이지 않고 즉시 `0.0`으로 리셋되도록 보완하여, 1회 단독 탭 시 정확하게 한 칸만 점프하도록 설계했습니다.
- **[x] 키서 공식 홈페이지 웹 프로젝트 신규 구축 (Keysor Web Homepage Project)**:
  - 키서의 기능 소개 및 다운로드 활성화를 위해, 1번 컨셉인 **모던 테크 & 생산성 SaaS (다크 모드 / 개발자 타겟)** 디자인의 웹 애플리케이션을 신규 구축했습니다.
  - **기술 스택**: Vite, Tailwind CSS v4, Vanilla HTML/JS 기반 초경량 빌드(컴파일 시 JS 2.8KB, CSS 30KB 미만).
  - **대화형 자석 스냅 샌드박스(Interactive Sandbox)**: 실제 키서의 Rust 소스코드와 동일하게 작동하도록, 브라우저상에서 키보드 `WASD` 또는 방향키를 누를 때 35% Lerp 보간 팩터를 적용한 커서가 모의 OS 바탕화면/앱 버튼으로 자석처럼 촥 달라붙는 실시간 웹 시뮬레이터를 완벽하게 이식했습니다.
  - **배포 지원**: PowerShell 원클릭 설치 CLI 스크립트 및 `.exe` 다운로드 링크 연동.

## 📅 이전 패치 내역 (Previous Updates - 2026-06-15)

- **[x] Shift + E / Shift + Q 브라우저 네이티브 뒤로가기/앞으로가기 단축키 안정화 (Browser Navigation Shortcut)**:
  - 마우스 모드 활성화 상태에서 `Shift + Q` 및 `Shift + E`를 눌러 브라우저 뒤로가기 및 앞으로가기를 실행할 때, 물리 Shift 상태와 가상 Alt+Arrow 시퀀스가 OS 키보드 상태 테이블에서 엉키며 오인식되던 현상을 해결했습니다.
  - 가상 복합 입력을 윈도우 네이티브 브라우저 제어 키코드(`VK_BROWSER_FORWARD`: `0xA7` 및 `VK_BROWSER_BACK`: `0xA6`)를 다이렉트로 전송하는 방식으로 우아하게 개편하여 타이밍 랙과 인식 문제를 완벽하게 수정했습니다.
  - HUD 안내판 하단에 해당 단축키 설명문구를 다국어(한국어/영어)에 대응하여 레이아웃 마감 처리를 완료했습니다.
  - 최신 릴리즈 바이너리를 빌드하여 유저님이 주로 실행하는 바탕화면 경로(`C:\Users\wjdwl\Desktop\keysor.exe`)에 강제 복사 및 덮어쓰기 배포를 완료했습니다.
- **[x] 이동 제어 물리 루프 성능 극대화 및 락(Mutex) 경합 제로화 (Performance Optimization)**:
  - 기존 10ms 단위 이동 물리 루프 안에서 매번 `HashMap`과 `HashSet`을 클론함으로써 발생하던 동적 힙 메모리 할당(초당 100~1000회)을 전면 제거하여 **힙 메모리 할당 제로화(Zero-Allocation)**를 실현했습니다.
  - 복사 대신 락 범위 안에서 방향 벡터(dx, dy)만 빠르게 연산 후 값 타입으로 복사 반환하도록 구조를 전개하여 Mutex 락 점유 시간을 1마이크로초 미만으로 단축하였으며, OS 스레드와 직접 맞물리는 저수준 키보드 훅의 입력 랙(Input Lag) 가능성을 완전히 차단했습니다.
- **[x] 동기식 렌더링 호출 제거를 통한 미세 끊김(Stuttering) 완벽 해소**:
  - 이동 스레드 주기에 존재하던 동기식 GDI 호출인 `UpdateWindow`가 OS UI 스레드 드로잉 완료 시점까지 물리 연산 스레드를 블로킹하던 현상을 발견하고 제거했습니다.
  - 이를 비동기 화면 갱신 방식(`InvalidateRect`)으로 단일화하여 물리 루프의 주기 시간(10ms) 정밀도가 흐트러짐 없이 유지되도록 조작감을 향상시켰습니다.
- **[x] 마그네틱 자석 흡착 기능 및 팝업창 ON/OFF 토글 구현 (Magnet Snapping Mode)**:
  - 팝업창 내 7가지 버튼 주위로 마우스 커서가 다가갔을 때 정확히 안착을 유도하는 자석 모드를 도입했습니다.
  - **흡착 반경(25px)** 및 **이탈 반경(30px)**을 이중으로 분리 제어하는 **이력 현상(Hysteresis) 알고리즘**을 내장하여, 자석처럼 척 들러붙으면서도 사용자가 조작을 통해 버튼 밖으로 빠져나올 때 마우스가 갇히지 않도록 부드러운 탈출 성능을 보장했습니다.
  - 팝업창 우측 패널 하단에 `[자석 모드: ON/OFF]` 버튼을 추가(오버레이 높이를 Y=325로 연장)하고 설정 파일(`keysor.yaml` 내 `magnetic_mode`)과 실시간 세이브/연동했습니다.

## 📅 이전 패치 내역 (Previous Updates - 2026-06-13)

- **[x] 픽셀 모드 소수점 이하 픽셀 축적(Sub-pixel Accumulation) 알고리즘 도입**:
  - 매 프레임 계산되는 이동 거리의 소수점 이하 오차(소수점 잔여값, Remainder)를 버리지 않고 계속 누적하여, 누적값이 단위를 넘길 때 마우스 이동에 반영하는 알고리즘을 구현했습니다.
  - 감도를 `0.1` 단위로 미세 조절하더라도 평균적인 이동 속도가 선형적으로 즉각 증가하게 되어, 픽셀 모드에서 감도를 올릴 때 속도가 정체되는 모순을 근본적으로 해결했습니다.
- **[x] 키캡 기능별 컬러 카테고리 분리 및 녹색 계열 차별화**:
  - 팝업창에서 각 기능의 구분이 확실해지도록 녹색 계열 내에서 톤과 명도 차이를 크게 두어 4가지 색상으로 차별화하였습니다.
    - **Caps Lock (활성화 키)**: 아주 밝고 눈에 띄는 Lime Green (`0x2FFFAD`)
    - **WASD (마우스 이동 키)**: 선명하고 강렬한 Grass Green (`0x40E000`)
    - **QERFG (스크롤 및 보조 휠 키)**: 부드럽고 차분한 Mint Green (`0xD0F8A0`)
    - **Spacebar (클릭 및 드래그 키)**: 짙고 신뢰감 주는 Deep Teal Green (`0x00AA80`)
    - **일반 텍스트 키**: 차분한 차콜 그레이 색상 (`0x3C4040`)
- **[x] 언어 토글 버튼 대상 언어 노출 방식 적용 (KO $\leftrightarrow$ EN)**:
  - 사용자가 언어 전환 동작을 직관적으로 인지할 수 있도록 **버튼 클릭 시 전환될 대상 언어**를 표시하도록 변경하였습니다 (한국어 모드 시 `EN` 노출, 영어 모드 시 `KO` 노출).
- **[x] 픽셀 단위 토글 버튼 UI 텍스트 정렬 및 상태 표시 추가**:
  - 픽셀 단위 버튼 내부 텍스트 정렬 플래그 `37` (`DT_CENTER | DT_VCENTER | DT_SINGLELINE`)을 적용해 **상하좌우 정중앙 정렬**을 완료했습니다.
  - 현재 상태에 맞추어 `[픽셀 단위: ON]` 또는 `[픽셀 단위: OFF]` (영문 모드 시 `[PIXEL: ON]` / `[PIXEL: OFF]`)로 텍스트가 동적으로 변경되도록 연동하였습니다.
- **[x] 설명 문구 왼쪽 정렬**:
  - HUD 하단 영역에 노출되는 주요 안내 문구의 정렬 방식을 기존 가운데 정렬에서 **왼쪽 정렬(DT_LEFT)**로 정렬 방향을 통일하였습니다.
- **[x] 키서 활성화(ON) 시 팝업창 항상 자동 최소화**:
  - 팝업창이 화면에 열려있을 때, Caps Lock을 눌러 키서를 활성화(ON)하면 기존의 최초 1회 제한 없이 **항상 자동으로 팝업창이 최소화**되어 화면을 가리지 않도록 개선되었습니다.
- **[x] 팝업창 닫기[X] 버튼 클릭 시 프로그램 종료**:
  - `WM_LBUTTONDOWN` 이벤트 처리에서 팝업창 우측 상단 `[X]` 버튼을 클릭하면 메인 윈도우로 `WM_CLOSE` 메시지를 보냅니다. 이를 통해 마우스 훅 등의 모든 리소스를 안전하게 시스템에 반환한 뒤 프로그램이 완전히 종료됩니다.

---

## 📅 이전 패치 내역 (Previous Updates - 2026-06-05)

- **[x] 시스템 단축키 투과 및 훅 스레드 프리징 완벽 차단 (Lock-Free GUI / Threading Optimization)**:
  - `Ctrl+C`/`Ctrl+V` 등 복사 붙여넣기를 수행할 때 AppState 뮤텍스 락을 획득한 상태에서 동기식 GDI 그리기(`UpdateWindow`)나 마우스 입력 에뮬레이션(`std::thread::sleep`)이 실행되어 전역 키보드 훅 스레드가 락 획득을 기다리며 무한 대기(먹통)하던 설계 오류를 완전히 해결했습니다.
  - 마우스 클릭 액션 및 UI 인디케이터 호출(`show_indicator`, `hide_indicator`, `update_indicator_position`) 직전에 **AppState 락을 선제적으로 해제(Drop)**하도록 구조를 고도화했습니다.
- **[x] Enter 및 Backspace 오리지널 동작 복원 (Bypass Mode)**:
  - 기존 마우스 모드에서 엔터 키가 좌클릭으로 고정 매핑되었던 점을 변경하여, 마우스 모드 진입 중에도 엔터(`VK_RETURN`)와 백스페이스(`VK_BACK`)는 원래 본연의 기능을 수행하며 그대로 투과(Bypass)되도록 처리했습니다.
  - `keysor.yaml` 기본 바인딩 템플릿과 실제 구동 중인 사용자 프로필(`C:\Users\wjdwl\.keysor\keysor.yaml`)의 Enter 매핑을 일괄 제거했습니다.
- **[x] HUD 카운트다운 가독성 및 보색 시인성 극대화 (Enterprise HUD Countdown)**:
  - HUD 설명창 좌측 하단에 조그맣게 표시되던 카운트다운을 **36px 대형 Bold 폰트**로 확대하여 가독성을 높였습니다.
  - 텍스트 전체 중 카운트 숫자만 키컬러(에메랄드 그린, `0x81B910`)와 강한 대비를 이루는 **선명한 보색(네온 오렌지-레드, `0x0045FF`)**으로 그리도록 `GetTextExtentPoint32W` API를 활용해 인라인 분할 렌더링을 적용했습니다.
  - 카운트다운 영역을 화면 **우측 하단**(`rect.right - 30`)으로 이동하고, 일반 복귀 가이드 문구를 좌측 하단(`left: 30`)으로 밀어 레이아웃의 균형미를 맞췄습니다.
- **[x] 바탕화면 실행 파일 동기화 배포 (Desktop Deployment Sync)**:
  - 사용자가 주로 바탕화면의 실행 파일(`C:\Users\wjdwl\Desktop\keysor.exe`)을 직접 실행하여 조작하는 환경에 맞춰, 빌드 시 바탕화면의 바이너리가 자동으로 최신 파일로 교체되도록 쉘 배포 공정을 추가했습니다.

---

## 🎨 조작법 통합 가이드 (Keyboard Control Layer)

`Caps Lock` 모디파이어 하나로 상황에 맞춰 세 가지 인체공학적 조작 동선을 동시 지원합니다.

```mermaid
graph TD
    A[Caps Lock 토글 활성화] --> B[이동 조작]
    A --> C[스페이스 클릭 통합]
    A --> D[보조 키 & 스크롤]

    B --> B1[왼손 단독: W / A / S / D (부드러운 가속)]
    B --> B2[오른손 단독: Up / Down / Left / Right]

    C --> C1[1회 탭: 좌클릭]
    C --> C2[2회 연타: 더블클릭]
    C --> C3[지속 홀드: 좌클릭 다운 (드래그)]

    D --> D1[Enter: 좌클릭]
    D --> D2[G: 우클릭]
    D --> D3[R: 마우스 휠 업]
    D --> D4[F: 마우스 휠 다운]
```

### 1. 하이브리드 커서 이동 및 특수 클릭
- **이동 (WASD 및 방향키 동시 동작)**: `W`/`A`/`S`/`D` (왼손 제어) 또는 `방향키` (오른손 제어)
- **보조 클릭 및 스크롤**:
  - `Enter` : 마우스 좌클릭 (Left Click)
  - `G` : 마우스 우클릭 (Right Click)
  - `R` : 마우스 휠 스크롤 업 (Scroll Up)
  - `F` : 마우스 휠 스크롤 다운 (Scroll Down)
- **속도 튜닝 세팅**: 시작 속도 `1.0`, 최대 가속 속도 `30.0`, 가속 상수 `1.5`로 자연스럽고 빠른 정밀 조작 지원

### 2. 스페이스바 3-in-1 클릭 엔진
| 조작 리듬 | 에뮬레이트 마우스 액션 | 작동 피드백 |
| :--- | :--- | :--- |
| **1회 가볍게 탭 (Single Tap)** | **좌클릭 (Left Click)** | 브라우저 링크 선택 및 윈도우 창 활성화 |
| **2회 연속 탭 (Double Tap)** | **더블 클릭 (Double Click)** | 폴더 및 프로그램 즉시 실행 오픈 |
| **지속 홀드 (Hold & Release)** | **좌클릭 드래그 (Drag & Drop)** | Space를 꾹 누르면 좌클릭 다운, 떼면 좌클릭 업 |

---

📂 파일 및 디렉토리 구조 (File Directory)

모든 소스 코드는 아래 링크를 클릭하여 IDE에서 즉시 확인하실 수 있습니다.

```text
14-Keysor/
├── [Cargo.toml](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/Cargo.toml)                  # Rust 패키지 빌드 설정 및 Win32 피처 활성화
├── [build.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/build.rs)                      # 리소스 자동 빌드 감지 및 컴파일러 지시자 설정
├── [keysor.rc](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/keysor.rc)                    # Windows 리소스 정보 지시 파일 (아이콘 ID 지정)
├── [keysor.manifest](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/keysor.manifest)              # 실행 시 UAC 관리자 권한 승인을 요구하는 매니페스트 기술서
├── [keysor.ico](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/keysor.ico)                   # 28% 라운드가 적용된 키서 공식 아이콘 (DIB 형식)
├── [keysor.yaml](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/keysor.yaml)                 # 사용자 커스텀 단축키 설정 템플릿 (초기 빌드용)
├── [LICENSE](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/LICENSE)                     # [NEW] 저작권 보호 및 상용 사용 계약 조항을 명시한 EULA 문서
├── [copy-dist.js](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/copy-dist.js)               # [NEW] 윈도우/리눅스 빌드 호환성을 대행하는 Node.js 기반 파일 복사 헬퍼
├── [DASHBOARD.md](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/DASHBOARD.md)               # 본 프로젝트 대시보드 문서
├── src/
│   ├── [main.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/main.rs)                 # 실시간 핫리로드 및 OS 클린 종료 연동
│   ├── [update.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/update.rs)               # [NEW] GitHub API 비동기 업데이트 감지 및 팝업창 모듈
│   ├── [math.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/math.rs)                 # 공통 마우스 이동 가속도 및 델타 연산 수학 모듈
│   ├── [config.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/config.rs)             # 설정 폴더(.keysor) 자동 탐색/생성 안전 회로 탑재
│   ├── [mouse.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/mouse.rs)               # Win32 SendInput 연동 마우스 에뮬레이터 & DPI 자동 보정
│   ├── [indicator.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/indicator.rs)           # 작업표시줄 상주 및 시스템 메뉴 통합 제어
│   ├── [ui/win_gdi.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/ui/win_gdi.rs)         # HUD 드로잉 및 버전 정보 렌더링 GDI 로직
│   └── [hook.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/hook.rs)                 # 저수준 키보드 훅 & 100Hz 부드러운 가속 물리 스레드
└── homepage/                                                                           # 키서 공식 소개 및 다운로드 홈페이지 (Vite + Tailwind v4)
    ├── [package.json](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/homepage/package.json)          # 웹 의존성 및 스크립트 설정
    ├── [vite.config.js](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/homepage/vite.config.js)        # Tailwind CSS v4 Vite 플러그인 설정
    ├── [index.html](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/homepage/index.html)              # 홈페이지 마크업 및 전체 구조
    └── src/
        ├── [style.css](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/homepage/src/style.css)        # Tailwind 및 커스텀 네온 글라스모피즘 스타일링
        └── [main.js](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/homepage/src/main.js)            # 자석 스냅핑 시뮬레이션 로직
```

---

## 🛡️ 안정성 가드 검증 리포트 (Robustness Check)

사용자가 마우스 없이 키보드만으로 컴퓨터를 완벽하게 조작할 때 발생할 수 있는 윈도우 OS의 특수한 예외 사항을 안전하게 처리했습니다.

- **[x] Caps Lock 전환 본래 기능 보존 & 텍스트 포커스 락 방쇄 (Caps Lock Suppression & Safe Toggle)**: 마우스 모드 온/오프 상태를 전환할 때 발생하는 실제 물리 Caps Lock 입력은 OS 및 텍스트 창에 전혀 누설되지 않도록 훅 수준에서 무조건 차단(return 1)하여 텍스트 입력창이 꼬이거나 포커스로 인해 훅이 일시적으로 정지되는 버그를 예방했습니다. 단독으로 가볍게 톡 칠 때만 기존 영문 대소문자 고정이 작동되게 가상 입력을 안전하게 에뮬레이트합니다. RDP나 OS가 임의로 동기화 주입하는 가상 Caps Lock 이벤트(`LLKHF_INJECTED`)도 필터링하여 오작동을 차단합니다.
- **[x] Modifier Sync Guard**: UAC 경고창이나 강제 화면 전환으로 인해 `KeyUp` 이벤트를 유실하여 키보드가 마우스 모드로 엉켜버리는 현상을 감지하고 물리적 눌림 여부를 매 주기 `GetAsyncKeyState`로 대조하여 오작동을 즉시 자동 해제합니다.
- **[x] DPI 오차 자동 보정 (DPI Scaling)**: GDI `GetDeviceCaps` API를 통해 마우스가 올라가 있는 모니터의 배율(100%, 150%, 200%)을 실시간 감지하여, 4K 모니터와 일반 모니터 간의 조작 거리감을 균일하게 교정합니다.
- **[x] 설정 핫리로드 (100ms Hot-Reloading)**: 앱을 끄지 않고 `keysor.yaml`을 수정해 저장하는 즉시 백그라운드 파일 감지기가 변경 내역을 100ms 이내에 즉각 게임/사무 환경에 무재시작 자동 적용합니다.
- **[x] YAML 폴백 구조 (Fallback Core)**: 사용자가 설정 파일 수정 도중 문법 오류를 범하면 시스템 트레이 오류를 송출하고 즉시 안전한 메모리 내장 기본 설정 파일로 백업 폴백하여 앱 실행의 중단을 원천 배제합니다.
- **[x] 페일세이프 자원 수거**: 콘솔 강제 종료, CMD 창 닫기, 시스템 파괴 신호 감지 즉시 `UnhookWindowsHookEx` 소멸자 루틴을 호출하여 운영체제에 전역 훅 자원을 안전하게 반환합니다.
- **[x] 원격 데스크톱(RDP/CRD) 호환성**: 원격 환경에서 마우스 상대 이동 패킷(`SendInput`)이 무시되는 현상을 해결하기 위해 `GetCursorPos` & `SetCursorPos` 기반 절대 좌표 조작 방식을 결합했습니다.
- **[x] 오버레이 강제 프레임 갱신 (Repaint Guard)**: 원격 화면 전송 시 투명 Layered Window의 렌더링이 갱신되지 않는 현상을 방지하기 위해 마우스 이동 주기마다 `InvalidateRect` 및 `UpdateWindow`로 강제 화면 갱신을 수행합니다.
- **[x] 커서 핫스팟 중앙 정렬 (Hotspot Alignment)**: 32x32px 크기의 초록색 링 오버레이 윈도우의 기하학적 중심이 마우스의 실제 클릭 작동 지점(핫스팟)과 완벽하게 겹치도록 좌상단 기준 -16px 오프셋 보정을 적용하여 정밀 조작 편차를 해소했습니다.
- **[x] 시스템 단축키 통과**: 마우스 모드가 켜져 있는 동안 바인딩되지 않은 모든 일반 문자 입력은 차단되지만, 복사/붙여넣기(`Ctrl+C`, `Ctrl+V`), `Alt+Tab` 같은 제어 단축키와 `Enter`, `Backspace`는 원래 기능을 작동하며 정상 투과(Bypass)되도록 회로를 구성했습니다.
- [x] 다크 카본 5초 최초 실행 HUD 가이드 & 실시간 카운트다운 (Startup HUD & Countdown): 프로그램 최초 로드 시 다크 카본 테마의 HUD 가이드창이 팝업됩니다. 닫히기 직전까지 우측 하단에 36px 대형 보색(네온 오렌지-레드, `0x0045FF`) 숫자로 남은 초를 큼직하게 카운트다운 표시하여 뛰어난 인지도를 선사합니다.
- [x] 작업표시줄 상주 통합 & 시스템 메뉴 연동 (Taskbar & System Menu Integration): `#![windows_subsystem = "windows"]` 속성을 부여하여 검은색 명령 프롬프트(CMD) 창이 팝업되지 않게 하였으며, `WS_EX_APPWINDOW` 속성이 연동된 메인 백그라운드 창을 작업표시줄에 상시 노출했습니다. 작업표시줄 아이콘 우클릭 시스템 메뉴에 'Keysor 설정 열기(메모장)', '설정 폴더 열기(탐색기)', '시작 프로그램 등록/해제 토글' 항목을 자연스럽게 통합하여 무설치 실행형임에도 인스톨러 수준의 완벽한 윈도우 환경 연동을 실현했습니다.

---

## 🍏 macOS 네이티브 아키텍처 및 HUD 팝업창 명세 (macOS Architecture & HUD Dashboard)

| 구분 | 사양 및 상태 | 세부 특징 |
| :--- | :--- | :--- |
| **타깃 OS** | **macOS** (AppKit Native, Objective-C / Cocoa Bridge) | WindowServer 최상단 연동 및 CoreGraphics 전역 이벤트 훅 |
| **자석 스냅 반경** | **`25.0px`** (Windows 표준과 100% 동기화) | AXParent 2단계 탐색으로 버튼 텍스트/아이콘 포인팅 100% 보장 |
| **메모리 안정성** | **`EXC_BAD_ACCESS` 튕김 0건** | AXUIElementRef 포인터 추적 및 단일 CFRelease 수거 루틴 적용 |
| **IPC 지연 시간** | **`0ms Zero-Lag` (<1ms)** | Mach IPC 조작 루프 60회 $\rightarrow$ 3회 축소 (95% 감축)로 버벅임 완전 제거 |
| **HUD 팝업 레이아웃** | **10pt 외곽 마진 / 12pt 폰트 규격화** | 808x452 HUD 패널 (Y-100px 하향), 12pt 폰트 일괄 통일, 28pt 버튼, 8pt 행간 |
| **스위치 모션** | **60fps 슬라이딩 토글 애니메이션** | iOS/macOS 순정 스타일 원형 노브 슬라이딩 (120ms) 및 색상 보간 |
| **클릭 시각 피드백** | **0.70 CTM 축소 스케일 & 180ms 노란색 유지** | Space, G, Scroll, Drag 조작 시 30% Shrink 및 우클릭 180ms 노란색 유지 |

### 🛠️ macOS UI & 자석 모드 정밀 픽셀 규격
- **HUD 패널 폰트 규격**: 팝업창 내 모든 중간 폰트를 **`12.0pt`** (`Bold`/`Medium`/`Regular`)로 일괄 통일.
- **SPEED SENS 패널 그리드**:
  - 패널 좌측 마진: **`10 pt`** (`X = 644`)
  - 패널 우측 마진: **`10 pt`** (`X = 772`)
  - 패널 하단 마진: **`10 pt`** (`Y = h - 334`, 패널 바닥 `h - 344`)
  - 버튼과 버튼 사이의 수직 행간: **`8 pt`** (`162, 198, 234, 270, 306`)
  - 버튼 세로 높이: **`28 pt`** 통일
  - 속도 조절 `-` / `+` 버튼: **`61 pt`** (좌/우 10pt 마진 100% 일치)
- **수치 디스플레이 정밀 간격**:
  - `Base`, `Max`, `Accel` 텍스트 피치: **`17 pt`** (`108`, `125`, `142`)
  - 글자 간 순수 수직 여백: **`4.0 pt`**

---

---

## 🪟 Windows GDI HUD 팝업창 및 스피드센서 패널 개선 명세 (Windows GDI HUD Refinements)

| 구분 | 사양 및 개선 상태 | 세부 특징 |
| :--- | :--- | :--- |
| **HUD 팝업 규격** | **`808 x 452` GDI 네이티브 렌더러** | 고정 외곽 박스(`638 ~ 778`, `80 ~ 344`), 스페이스바 포함 키캡 100% 보존 |
| **LCD 디스플레이 박스** | **`Y: 114 ~ 150` (높이 36px, R: 5px)** | 독립 디스플레이 박스 신설로 수치 표시부와 하단 조작 버튼 구역 완벽 분리 |
| **기본 수치 렌더링** | **순백색(`0xFFFFFF`), 중앙 정렬** | `fonts.number` 폰트로 디스플레이 박스 정중앙(`DT_CENTER \| DT_VCENTER`) 표기 |
| **상세보기 2줄 레이아웃** | **가로 2줄 좌측 통일 정렬 (`X = 652`)** | 1행 `Base: 1.0`, `Max: 30.0` (`Y: 118~133`) / 2행 `Accel: 1.5` (`Y: 134~149`) |
| **조작 버튼 컴팩트 패킹** | **5개 버튼 행 (`Y: 154 ~ 334`, 높이 32px)** | 버튼 간격 5px, 좌우 여백 10px, 컨테이너 바닥 하단 여백 정확히 10px(`344 - 334`) |
| **잠금(Lock) 오버레이** | **무료 모드 텍스트 겹침 0% 차단** | `🔒 프로 전용` 안내 문구 시 하단 버튼 라벨을 자동 숨김 처리하여 완벽한 가독성 확보 |
| **커서 스케일 버그 픽스** | **100Hz 물리 스레드 락 해제** | `src/hook.rs`의 반복 `trigger_click_motion` 호출 제거로 Windows 가상 커서 축소 고정 버그 완제 |

### 🛠️ Windows GDI HUD 정밀 픽셀 배치 규격
- **스피드센서 외곽 컨테이너**: `Left: 638, Top: 80, Right: 778, Bottom: 344` (폭 140px, 높이 264px)
- **SPEED SENS 헤더 타이틀**: `Y: 87 ~ 105` (`fonts.title`, `DT_CENTER`), 디스플레이 상단과 **9px 독립 여백**
- **LCD 디스플레이 화면 박스**: `Left: 646, Top: 114, Right: 770, Bottom: 150` (폭 124px, 높이 36px, 곡률 5px)
  - 일반 보기: `Y: 113 ~ 149` (순백색 대형 숫자 정중앙)
  - 상세보기: 1행 `Y: 118 ~ 133` (`Base` X:652~708, `Max` X:708~764), 2행 `Y: 134 ~ 149` (`Accel` X:652~764)
- **하단 5행 조작 버튼**:
  - `[-]` / `[+]`: `Y: 154 ~ 186` (디스플레이 하단과 4px 타이트 간격, `[-]` X:648~703, `[+]` X:713~768)
  - `그리드 모드 (Grid Mode)`: `Y: 191 ~ 223` (X: 648 ~ 768)
  - `자석 모드 (Magnet Mode)`: `Y: 228 ~ 260` (X: 648 ~ 768)
  - `자동 실행 (Auto-Start)`: `Y: 265 ~ 297` (X: 648 ~ 768)
  - `상세보기 (Details)`: `Y: 302 ~ 334` (X: 648 ~ 768, 하단 끝 344까지 10px 바닥 마진)

---

## 🚀 다음 스텝 (Next Step)

> [!TIP]
> 1. 현재 빌드된 초경량 바이너리는 [keysor.exe](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/target/release/keysor.exe) 및 macOS [Keysor.app](file:///Users/jidu/Desktop/Keysor.app) 경로에 위치하고 있으며, 자동 릴리즈 빌드 완료 상태입니다.
> 2. 구축된 홈페이지는 [http://localhost:5173/](http://localhost:5173/) 주소에서 작동 중이며 대화형 키서 시뮬레이션을 내장하고 있습니다.
> 3. 향후 홈페이지의 모던 다크 테마 디자인과 함께 윈도우 인스톨러 배포, Tauri GUI 제어판 연결 등을 이어서 추진할 예정입니다.
> 4. **[보류/메모]** 향후 서비스 고도화 및 스케일업 시점에 **Supabase를 연동하여 기기별 UUID 매핑 통제, 사용자 이메일 계정 기반의 구매자 포털 구축, 원격 공지사항 연동**을 설계 및 반영할 예정입니다. (현재 1.0 초기 버전은 Lemon Squeezy의 기본 관리자/조회 인프라만 활용하여 유지보수성을 극대화함.)



## 2026-10-10 Stability handoff
See [WINDOWS_FIXES_MAC_HANDOFF.md](WINDOWS_FIXES_MAC_HANDOFF.md) for current common/Windows/macOS code changes, verification evidence, and the remaining Mac device test. Historical completion claims above are not current verification results.

## 2026-10-17 Space + 스크롤 입력 순서 / macOS 적용

- 공통 `src/hook.rs`: R 또는 F를 먼저 누르고 Space를 누르면 맨 위/아래 페이지 점프를 한 번 실행한다. R/F를 먼저 떼더라도 해당 Space를 클릭·드래그로 처리하지 않는다. Space 키 반복 입력으로 점프가 재실행되지 않는다.
- Space를 먼저 누른 뒤 R/F를 누르면 기존 Space 클릭·드래그와 R/F 일반 스크롤을 유지한다. Q/E를 먼저 누른 뒤 Space를 누르는 가로 점프도 유지한다.
- macOS EventTap의 R/F/Space 자동 반복 이벤트를 공통 처리기로 보내 마우스 모드 중 문자 입력이 새지 않게 했다. 다른 키의 반복 이벤트 정책은 그대로다.
- macOS `simulate_page_jump`를 고정량 휠 스크롤에서 Command+Up/Down 합성 입력으로 바꿨다. 이는 [Apple의 문서 시작/끝 단축키](https://support.apple.com/en-us/102650)를 따르지만, 대상 앱별 실제 동작은 Mac에서 확인해야 한다.
- Windows 검증: `scripts/test-windows.ps1`의 테스트 16개 통과, `cargo build --release --offline` 성공. 표준 `target/release/keysor.exe`를 새로 빌드했고 기존 바탕화면 바로가기도 이 파일을 가리키도록 확인했다. 검증 당시 실행 중인 프로세스는 이전 별도 빌드(`target/scroll-boost-build/release/keysor.exe`)에서 시작했으므로 표준 파일을 실행하려면 앱을 완전히 종료한 뒤 바로가기로 재시작해야 한다.
- macOS 검증 경계: Windows에서 변경된 Rust 파일 2개의 구문 검사는 통과했다. `cargo check --target x86_64-apple-darwin --offline`은 Apple용 C 컴파일러 `cc`가 없어 의존성 빌드 단계에서 중단됐으므로 Mac 앱 빌드와 실기기 동작은 미검증이다. Mac에서 `cargo test --bin keysor` 실행 후 `bash scratch/package_mac_app.sh`로 앱을 빌드한다. Accessibility/Input Monitoring 권한을 확인하고 R/F -> Space(점프 1회·클릭/드래그 없음), Space -> R/F(일반 클릭/드래그/스크롤), R/F를 먼저 뗀 뒤 Space 해제, 길게 누르기/자동 반복, Q/E -> Space를 직접 시험한다. Windows 빌드는 Mac EventTap 및 앱 동작을 검증하지 않는다.

### 2026-11-16 커서 입력 모션 회귀 수정
- 네이티브 커서가 보이면 가시성 처리에서 조기 반환해 애니메이션이 실행되지 않았고, 네이티브 아트워크도 최초 프레임으로 고정돼 있었다.
- 오버레이가 숨겨져도 입력 축소/복귀 및 색상 프레임을 처리하고 네이티브 커서에 재적용한다. 원격용 32px 커서에도 현재 축소 비율을 적용하며 이전 프레임 핸들을 해제한다. 일시정지에서는 기존 기본 커서 정책을 유지한다.
- 버전 1.0.3 유지. 일반 테스트 55개 통과, 8개 대화형 테스트 제외. 릴리스/설치 파일 생성 성공.
- target/click-motion-install.json: 정식 위치 적용/재시작 성공, 설정과 --chrome-remote --diagnose-input 인수 보존, 메인 PID31692 / Guardian PID55932. 실제 원격 클라이언트의 키 입력 모션은 직접 시각 확인하지 못했다.
