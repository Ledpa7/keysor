$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    $version = [regex]::Match((Get-Content Cargo.toml -Raw), '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
    if (-not $version) { throw 'Missing package version' }
    $previousUIAccess = $env:KEYSOR_UIACCESS_BUILD
    $previousTest = $env:KEYSOR_TEST_BUILD
    try {
        Remove-Item Env:KEYSOR_UIACCESS_BUILD -ErrorAction SilentlyContinue
        Remove-Item Env:KEYSOR_TEST_BUILD -ErrorAction SilentlyContinue
        $ErrorActionPreference = 'Continue'
        & cargo build --release --bin keysor --offline
        $buildExit = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = 'Stop'
        $env:KEYSOR_UIACCESS_BUILD = $previousUIAccess
        $env:KEYSOR_TEST_BUILD = $previousTest
    }
    if ($buildExit -ne 0) { throw 'Standard release build failed' }
    New-Item -ItemType Directory -Force target\release-package | Out-Null
    Copy-Item target\release\keysor.exe target\release-package\keysor.exe -Force
    Copy-Item target\release\keysor.exe target\release-package\keysor-guardian.exe -Force
    if ((Get-AuthenticodeSignature target\release-package\keysor.exe).Status -ne 'NotSigned') { throw 'Expected an unsigned standard build' }
    & target\packaging-tools\inno\ISCC.exe "/DAppVersion=$version" scripts\keysor-setup.iss
    if ($LASTEXITCODE) { throw 'Installer compilation failed' }
    $setup = "dist\Keysor-Setup-$version.exe"
    Copy-Item target\release\keysor.exe dist\Keysor.exe -Force
    Copy-Item target\release\keysor.exe dist\Keysor-Guardian.exe -Force
    $hash = (Get-FileHash $setup -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  Keysor-Setup-$version.exe" | Set-Content dist\SHA256SUMS.txt -Encoding ASCII
    ((Get-FileHash dist\Keysor.exe -Algorithm SHA256).Hash.ToLowerInvariant() + '  Keysor.exe') | Add-Content dist\SHA256SUMS.txt -Encoding ASCII
    ((Get-FileHash dist\Keysor-Guardian.exe -Algorithm SHA256).Hash.ToLowerInvariant() + '  Keysor-Guardian.exe') | Add-Content dist\SHA256SUMS.txt -Encoding ASCII
    Copy-Item scripts\release-notice.md dist\WINDOWS-RELEASE-NOTES.txt -Force
    Copy-Item scripts\verify-test-install.ps1 dist\Verify-Keysor.ps1 -Force
    @{ success=$true; setup=$setup; sha256=$hash; build_mode='unsigned-standard'; clean_machine_tested=$false } | ConvertTo-Json | Set-Content target\release-package\result.json
} finally {
    Pop-Location
}
