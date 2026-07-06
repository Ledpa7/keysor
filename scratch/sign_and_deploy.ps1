# 관리자 권한 여부 확인
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "이 스크립트는 반드시 관리자 권한으로 실행되어야 합니다."
    exit 1
}

$certSubject = "CN=KeysorUIAccessCert"
$certStoreRoot = "Cert:\LocalMachine\Root"
$certStoreMy = "Cert:\LocalMachine\My"

# 1. 인증서 검사 및 생성
Write-Host "1. 코드 서명 인증서 검사 중..."
$cert = Get-ChildItem -Path $certStoreMy | Where-Object { $_.Subject -eq $certSubject } | Select-Object -First 1

if ($null -eq $cert) {
    Write-Host "인증서가 존재하지 않습니다. 자체 서명 인증서를 생성합니다..."
    # LocalMachine의 My 저장소에 코드 서명용 인증서 생성
    $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject $certSubject -CertStoreLocation $certStoreMy -KeyLength 2048 -NotAfter (Get-Date).AddYears(10)
    
    # 신뢰할 수 있는 루트 인증 기관(Root)에 추가
    $rootStore = New-Object System.Security.Cryptography.X509Certificates.X509Store("Root", "LocalMachine")
    $rootStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
    $rootStore.Add($cert)
    $rootStore.Close()

    # 신뢰할 수 있는 게시자(TrustedPublisher)에 추가
    $pubStore = New-Object System.Security.Cryptography.X509Certificates.X509Store("TrustedPublisher", "LocalMachine")
    $pubStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
    $pubStore.Add($cert)
    $pubStore.Close()
    
    Write-Host "인증서 생성, 루트 기관 및 신뢰할 수 있는 게시자 등록 완료."
} else {
    # 루트 기관 등록 더블체크
    $rootCert = Get-ChildItem -Path $certStoreRoot | Where-Object { $_.Subject -eq $certSubject } | Select-Object -First 1
    if ($null -eq $rootCert) {
        Write-Host "루트 인증 기관에 인증서를 다시 등록합니다..."
        $rootStore = New-Object System.Security.Cryptography.X509Certificates.X509Store("Root", "LocalMachine")
        $rootStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
        $rootStore.Add($cert)
        $rootStore.Close()
    }
    
    # 신뢰할 수 있는 게시자 등록 더블체크
    $pubCert = Get-ChildItem -Path "Cert:\LocalMachine\TrustedPublisher" | Where-Object { $_.Subject -eq $certSubject } | Select-Object -First 1
    if ($null -eq $pubCert) {
        Write-Host "신뢰할 수 있는 게시자 저장소에 인증서를 다시 등록합니다..."
        $pubStore = New-Object System.Security.Cryptography.X509Certificates.X509Store("TrustedPublisher", "LocalMachine")
        $pubStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
        $pubStore.Add($cert)
        $pubStore.Close()
    }
    Write-Host "기존 인증서 확인 및 신뢰 저장소 등록 완료."
}

# 2. 기존 실행 중인 keysor.exe 종료
Write-Host "2. 실행 중인 Keysor 프로세스 종료 중..."
Stop-Process -Name keysor -Force -ErrorAction SilentlyContinue

# 3. keysor.exe에 디지털 서명 적용
$projectRoot = Split-Path -Path $PSScriptRoot -Parent
$exePath = Join-Path $projectRoot "target\release\keysor.exe"
if (Test-Path $exePath) {
    Write-Host "3. target\release\keysor.exe 디지털 서명 적용 중..."
    $signResult = Set-AuthenticodeSignature -Certificate $cert -FilePath $exePath
    if ($signResult.Status -eq "Valid") {
        Write-Host "디지털 서명 성공적으로 적용 완료!"
    } else {
        Write-Error "디지털 서명 적용 실패: $($signResult.StatusMessage)"
        exit 1
    }
} else {
    Write-Error "target\release\keysor.exe 파일을 찾을 수 없습니다. 빌드를 먼저 수행해 주세요."
    exit 1
}

# 4. Program Files\Keysor 복사
$targetDir = "C:\Program Files\Keysor"
Write-Host "4. $targetDir 경로로 바이너리 복사 중..."
if (-not (Test-Path $targetDir)) {
    New-Item -Path $targetDir -ItemType Directory | Out-Null
}
Copy-Item -Path $exePath -Destination $targetDir -Force
Write-Host "복사 완료."

# 5. 바탕화면 바로가기 생성/갱신
Write-Host "5. 바탕화면 바로가기 갱신 중..."
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("C:\Users\wjdwl\Desktop\keysor.lnk")
$Shortcut.TargetPath = Join-Path $targetDir "keysor.exe"
$Shortcut.WorkingDirectory = $targetDir
$Shortcut.Save()
Write-Host "바탕화면 바로가기(keysor.lnk) 갱신 완료."
Write-Host "배포 파이프라인이 성공적으로 완료되었습니다!"
