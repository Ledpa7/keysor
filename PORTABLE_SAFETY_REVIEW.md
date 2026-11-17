# 포터블 배포와 독립 복구 점검 — 2026-11-17

아래 사전 점검 이후 사용자의 구현 요청으로 포터블 배포와 독립 복구 도구를 추가했다. 현재 검증 상태는 다음과 같다.

## 구현 후 상태

- 최종 검사 업데이트: `target/portable-native-results.json`에서 실제 데스크톱 검사5개 모두 통과했다. Guardian 누락 시 커서 변경 차단, 메인만 종료한 경우 자동 복구, Guardian만 종료한 경우 메인의 복구, 양쪽 종료 후 보이는 그림 유지 및 독립 수동 복구, 의도적 투명 포인터의 수동 복구를 확인했다.14종 커서의 복구 후 렌더링 해시가 기준과 일치했다. 이는 실제 보안툴의 탐지·격리를 유발한 시험은 아니다.
- 검사 실행기의 PowerShell 종료 코드가 null인 경우를 실제 Rust 성공 요약과 구조화 보고서로 판정하도록 수정했다. 양쪽 종료 시험에서는 생성자가 종료된 뒤 그리기 결과가 일부 달라질 수 있어 살아 있을 때의 해시와 동일해야 한다는 조건 대신, 기본 구성표와 다른 그림이 남아 있고 모든 역할이 보이는지 확인한다. 수동 복구 후에는 기준과 정확히 일치해야 한다.
- 포터블 배포 적용 및 바로가기 전환 완료. 최종 크기 복원·HUD 색상 빌드의 적용 결과는 `target/hud-small-apply-result.json`이며 당시 메인PID60472/Guardian PID43392, ready/independent=true였다. 설정 해시 보존 및 배포 메인/빌드 해시 일치 확인. 최신 일반 테스트59개 통과(대화형12개 제외).
- `scripts/package-windows-portable.ps1`: 설치 프로그램을 실행하지 않는 `dist/Keysor-Portable.zip`과 폴더를 생성한다. Main/Guardian/Recovery는 각각 다른 해시의 실행파일이다. 사용자는 Keysor.exe만 평소 실행하며 Guardian은 자동 시작한다. 단일 바이너리로 합치지 않았다.
- 메인 실행파일을 Guardian으로 재사용하던 대체 경로를 제거했다. Guardian 준비 보고서의 부모/Guardian PID 및 현재 생존 상태를 확인한다. 메인은 별도 스레드에서 커널 핸들 대기로 Guardian 종료를 감지해 기본 커서를 복원한다. 새 폴링 루프는 추가하지 않았다.
- `Keysor-Recover.exe`는 메인/Guardian/설정/라이선스에 의존하지 않고 Windows 기본 커서를 재로드한다. 평소 직접 실행하면 결과 창을 표시하며, 검사에서는 `--report`로 결과를 수집한다.
- 포터블 릴리스 빌드와 별도 실행파일 해시 구분 확인. 버전1.0.3 유지. 메인·Guardian·Recovery를 함께 제공하는 폴더 방식이며, 단일 실행파일 내장/추출 방식은 채택하지 않았다.
- 실제 데스크톱 검사는 `scripts/test-portable-cursor-safety.ps1`로 실행했다. 초기 권한 부족과 PowerShell 종료 코드 수집 오류를 수정한 뒤5개 검사가 모두 통과했다. 이 스크립트는 개발 PC의 바로가기/설정 경로를 사용하는 로컬 통합 검사 도구다. 상세 기록과 한계는 `DASHBOARD.md`를 참조한다.

이하 내용은 구현 전 사전 점검 기록이다.

## 실행한 시험

`target/portable-risk-probe.ps1`은 현재 `src/watchdog_core.rs`를 사용하는 디버그 Guardian과 별도의 PowerShell 시험 부모 프로세스를 실행한다. 실제 키서 프로세스는 종료하지 않았다. 사용자 커서를 시험용 그림으로 변경하지 않았으며, 실제 Windows 기본 커서 재로드 API의 반환값과 Guardian 보고서를 확인했다. 실제 그림의 원복까지 시각 검증한 시험은 아니다.

| 조건 | 결과 |
| --- | --- |
| 시험 부모만 강제 종료 | 종료 감지 성공(wait_result=0), 복원 API 성공(restore_ok=true) |
| Guardian을 강제 종료한 뒤 부모 종료 | 준비 보고서만 남고 복원 결과 없음 |
| Guardian 경로가 없는 실행 실패 모사 | 실행 실패, 복구 보고서 없음 |

최종 결과: `target/portable-risk-results.json`. 처음 샌드박스 실행에서는 종료 감지는 됐지만 복원 API가 false였다. 데스크톱 접근이 가능한 권한으로 재실행한 최종 결과와 구분한다. 런처 실패 시험은 파일 부재를 사용했으며 보안 제품의 격리를 재현한 것은 아니다. Windows Defender 등록 및 현재 배포 메인/Guardian의 NotSigned 상태를 조회했다. 실시간 보호 상태나 탐지 결과를 확인한 것은 아니다.

## 코드에서 확인한 한계

- `src/watchdog_core.rs`: 부모 프로세스 종료를 기다리는 Guardian 자신이 살아 있어야 복구 코드가 실행된다. 둘 모두 중단되면 이 경로로 복구할 수 없다.
- 점검 당시 `src/watchdog.rs`에는 별도 Guardian이 없으면 메인 실행파일을 `--watchdog`로 재사용하는 경로가 있었다. 현재는 제거했다.
- 점검 당시 `src/cursor_scheme.rs`는 준비 파일만 믿고 Guardian 생존을 확인하지 않았다. 현재는 생존을 확인하고, 메인이 별도 커널 핸들 대기로 Guardian 종료를 감지해 기본 커서를 복구한다.
- 설치형과 휴대형 모두 보안 제품이 두 프로세스를 중단하는 상황에 대한 생존 보장은 없다.

## 보안 차단 해석과 권장 방향

Microsoft Defender for Endpoint는 파일 해시에 따른 차단 및 실행 중 프로세스 종료/파일 격리를 지원한다. 메인 바이너리를 그대로 복사해 이름만 바꾸는 설계는 같은 해시를 공유하므로 같은 해시 차단의 대상이 된다. 별도 컴파일된 최소 Guardian은 다른 파일이지만, 별도의 판정이나 정책으로 함께 차단될 가능성을 배제할 수 없다. 이 판단은 문서와 구조에 근거한 추론이며 키서에 대한 실제 백신 탐지 결과가 아니다.

내장/추출 방식은 초기 검토안이었고 최종 구현은 서로 다른3개 실행파일을 함께 배포하는 포터블 폴더다. Guardian 준비 실패 시 커서를 교체하지 않고, 실행 중 Guardian이 종료돼도 살아 있는 메인이 기본 커서로 돌아가도록 구현했다. 이 보완도 둘을 동시에 종료하는 보안 조치에서 자동 복구를 보장하지는 않는다. 보안 제품의 차단을 우회하거나 자동 예외 등록하지 않는다. 실제 다운로드 환경에서 SmartScreen 및 제품별 검사와 격리 상황은 별도 검증이 필요하다.

공식 근거:

- https://learn.microsoft.com/en-us/defender-endpoint/respond-file-alerts
- https://learn.microsoft.com/en-us/defender-endpoint/indicator-file
- https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation

사전 점검 후 포터블 배포 구현과 바로가기 적용까지 진행했다. 현재 상태는 문서 상단과 `DASHBOARD.md`를 따른다.
