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
        & cargo build --release --bin keysor --bin keysor-guardian --offline
        $buildExit = $LASTEXITCODE
    } finally {
        $env:KEYSOR_UIACCESS_BUILD = $previousUIAccess
        $env:KEYSOR_TEST_BUILD = $previousTest
    }
    if ($buildExit -ne 0) { throw 'Standard release build failed' }

    $stage = 'target\release-package'
    $output = Join-Path $stage 'windows-output'
    New-Item -ItemType Directory -Path $stage, $output -Force | Out-Null
    $built = 'target\release\keysor.exe'
    $builtGuardian = 'target\release\keysor-guardian.exe'
    $main = Join-Path $stage 'keysor.exe'
    $guardian = Join-Path $stage 'keysor-guardian.exe'
    Copy-Item -LiteralPath $built -Destination $main -Force
    Copy-Item -LiteralPath $builtGuardian -Destination $guardian -Force
    $mainHash = (Get-FileHash -LiteralPath $main -Algorithm SHA256).Hash.ToLowerInvariant()
    $guardianHash = (Get-FileHash -LiteralPath $guardian -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($mainHash -eq $guardianHash) { throw 'Guardian must be a distinct executable, not a renamed copy' }
    foreach ($file in @($main, $guardian)) {
        if ((Get-AuthenticodeSignature -FilePath $file).Status -ne 'NotSigned') {
            throw "Expected unsigned standard build: $file"
        }
    }

    & target\packaging-tools\inno\ISCC.exe "/DAppVersion=$version" '/DPackageOutputDir=..\target\release-package\windows-output' scripts\keysor-setup.iss
    if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }
    $setupName = "Keysor-Setup-$version.exe"
    $setup = Join-Path $output $setupName
    if (-not (Test-Path -LiteralPath $setup)) { throw 'Installer output is missing' }
    $setupHash = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToLowerInvariant()
    Copy-Item -LiteralPath $main -Destination (Join-Path $output 'Keysor.exe') -Force
    Copy-Item -LiteralPath $guardian -Destination (Join-Path $output 'Keysor-Guardian.exe') -Force
    Copy-Item -LiteralPath 'scripts\release-notice.md' -Destination (Join-Path $output 'WINDOWS-RELEASE-NOTES.txt') -Force
    Copy-Item -LiteralPath 'scripts\verify-test-install.ps1' -Destination (Join-Path $output 'Verify-Keysor.ps1') -Force
    @(
        "$setupHash  $setupName"
        "$mainHash  Keysor.exe"
        "$guardianHash  Keysor-Guardian.exe"
    ) | Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS.txt') -Encoding ASCII
    foreach ($entry in @(@($setupName,$setupHash),@('Keysor.exe',$mainHash),@('Keysor-Guardian.exe',$guardianHash))) {
        if ((Get-FileHash -LiteralPath (Join-Path $output $entry[0]) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry[1]) {
            throw "Staged artifact hash mismatch: $($entry[0])"
        }
    }

    New-Item -ItemType Directory -Path 'dist' -Force | Out-Null
    foreach ($name in @($setupName,'Keysor.exe','Keysor-Guardian.exe','SHA256SUMS.txt','WINDOWS-RELEASE-NOTES.txt','Verify-Keysor.ps1')) {
        Copy-Item -LiteralPath (Join-Path $output $name) -Destination (Join-Path 'dist' $name) -Force
    }
    foreach ($entry in @(@($setupName,$setupHash),@('Keysor.exe',$mainHash),@('Keysor-Guardian.exe',$guardianHash))) {
        if ((Get-FileHash -LiteralPath (Join-Path 'dist' $entry[0]) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry[1]) {
            throw "Published artifact hash mismatch: $($entry[0])"
        }
    }
    @{ success=$true; setup="dist\$setupName"; setup_sha256=$setupHash;
       executable_sha256=$mainHash; guardian_sha256=$guardianHash;
       build_mode='unsigned-standard'; clean_machine_tested=$false } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stage 'result.json') -Encoding UTF8
} finally {
    Pop-Location
}