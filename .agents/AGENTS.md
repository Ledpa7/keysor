# 🟢 Keysor 개발 및 배포 행동 규칙 (Rules)

- **바탕화면 배포 규칙**: 바탕화면(`C:\Users\wjdwl\Desktop\`)에 직접 `keysor.exe` 실행 파일을 복사하거나 생성하지 마십시오. 대신 `target\release\keysor.exe` 빌드 결과물을 가리키는 바로가기(LNK) 파일(`C:\Users\wjdwl\Desktop\keysor.lnk`)만 생성해야 합니다.
- **항상 릴리즈 빌드 수행**: 코드 수정 또는 빌드 검증 작업을 마칠 때에는 항상 `cargo build --release` 명령을 수행하여 `target/release/keysor.exe` 결과물을 업데이트하십시오.
