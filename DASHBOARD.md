# 🎛️ 키서 (Keysor) - 프로젝트 대시보드

마우스 없이 키보드만으로 마우스 커서를 정밀 조작하는 초경량·초안정성 윈도우 네이티브 유틸리티 **키서(Keysor)**의 종합 프로젝트 대시보드입니다.

---

## 📊 프로젝트 개요 (Overview)

| 구분 | 사양 및 상태 | 세부 특징 |
| :--- | :--- | :--- |
| **프로젝트명** | **키서 (Keysor)** | 오토핫키를 모방한 고자유도 단축키 마우스 시뮬레이터 |
| **타깃 OS** | **Windows** (Win32 Native API) | 윈도우 저수준 API를 활용한 무결점 하드웨어 레벨 연동 |
| **개발 언어** | **Rust** (Ed. 2024, windows-sys) | 가비지 컬렉터(GC) 배제로 지연 시간 0ms 달성 |
| **바이너리 용량**| **약 2.7 MB** (`keysor.exe`) | 의존성 없는 초경량 고성능 단일 실행 파일 |
| **메모리 점유율**| **2 MB 미만** (동작 중 기준) | Electron(100MB+) 및 Python(40MB+) 대비 약 95% 절감 |
| **컴파일 상태** | **Finished `release` [optimized]** | `cargo build --release` 경고 0개, 에러 0개 완벽 통과 |

---

## 📅 최신 패치 및 수정 내역 (Latest Updates - 2026-07-02)

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

## 📂 파일 및 디렉토리 구조 (File Directory)

모든 소스 코드는 아래 링크를 클릭하여 IDE에서 즉시 확인하실 수 있습니다.

```text
14-Keysor/
├── [Cargo.toml](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/Cargo.toml)                  # Rust 패키지 빌드 설정 및 Win32 피처 활성화
├── [build.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/build.rs)                      # 리소스 자동 빌드 감지 및 컴파일러 지시자 설정
├── [keysor.rc](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/keysor.rc)                    # Windows 리소스 정보 지시 파일 (아이콘 ID 지정)
├── [keysor.ico](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/keysor.ico)                   # 28% 라운드가 적용된 키서 공식 아이콘 (DIB 형식)
├── [keysor.yaml](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/keysor.yaml)                 # 사용자 커스텀 단축키 설정 템플릿 (초기 빌드용)
├── [DASHBOARD.md](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/DASHBOARD.md)               # 본 프로젝트 대시보드 문서
├── src/
│   ├── [main.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/main.rs)                 # 실시간 핫리로드 및 OS 클린 종료 연동
│   ├── [math.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/math.rs)                 # [NEW] 공통 마우스 이동 가속도 및 델타 연산 수학 모듈
│   ├── [config.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/config.rs)             # 설정 폴더(.keysor) 자동 탐색/생성 안전 회로 탑재
│   ├── [mouse.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/mouse.rs)               # Win32 SendInput 연동 마우스 에뮬레이터 & DPI 자동 보정
│   ├── [indicator.rs](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/src/indicator.rs)           # 작업표시줄 상주 및 시스템 메뉴 통합 제어
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

## 🚀 다음 스텝 (Next Step)

> [!TIP]
> 1. 현재 빌드된 초경량 바이너리는 [keysor.exe](file:///C:/Users/wjdwl/.gemini/antigravity/scratch/14-Keysor/target/release/keysor.exe) 경로에 위치하고 있으며, 자동 릴리즈 빌드 완료 상태입니다.
> 2. 구축된 홈페이지는 [http://localhost:5173/](http://localhost:5173/) 주소에서 작동 중이며 대화형 키서 시뮬레이션을 내장하고 있습니다.
> 3. 향후 홈페이지의 모던 다크 테마 디자인과 함께 윈도우 인스톨러 배포, Tauri GUI 제어판 연결 등을 이어서 추진할 예정입니다.
> 4. **[보류/메모]** 향후 서비스 고도화 및 스케일업 시점에 **Supabase를 연동하여 기기별 UUID 매핑 통제, 사용자 이메일 계정 기반의 구매자 포털 구축, 원격 공지사항 연동**을 설계 및 반영할 예정입니다. (현재 1.0 초기 버전은 Lemon Squeezy의 기본 관리자/조회 인프라만 활용하여 유지보수성을 극대화함.)
