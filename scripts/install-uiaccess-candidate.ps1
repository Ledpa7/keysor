param(
    [string]$CertificateThumbprint = 'AE040C21E16E19CC0E86F346A8A8205FC546FD4A'
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$report = Join-Path $root 'target\uiaccess-install.json'
try {
    $source = Join-Path $root 'target\uiaccess-build\release\keysor.exe'
    $cert = Get-Item "Cert:\LocalMachine\My\$CertificateThumbprint"
    if (-not $cert.HasPrivateKey -or $cert.NotAfter -lt (Get-Date)) { throw 'Signing certificate unavailable or expired.' }
    if (-not (Test-Path "Cert:\LocalMachine\Root\$CertificateThumbprint")) { throw 'Certificate is not already trusted. No trust settings were changed.' }
    $hash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
    $folder = Join-Path $env:ProgramFiles ("Keysor\uiaccess-" + $hash.Substring(0, 12).ToLowerInvariant())
    # Never overwrite an existing release or stop the user's running hook.
    if (Test-Path -LiteralPath $folder) { throw "Candidate directory already exists: $folder" }
    New-Item -ItemType Directory -Path $folder | Out-Null
    $installed = Join-Path $folder 'keysor.exe'
    Copy-Item -LiteralPath $source -Destination $installed
    $signature = Set-AuthenticodeSignature -FilePath $installed -Certificate $cert -HashAlgorithm SHA256
    if ($signature.Status -ne 'Valid') { throw "Signing failed: $($signature.StatusMessage)" }
    @{
        success = $true
        executable = $installed
        source_sha256 = $hash
        installed_sha256 = (Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash
        signature = [string]$signature.Status
        certificate = $cert.Thumbprint
        shortcut_changed = $false
        running_processes_stopped = $false
    } | ConvertTo-Json | Set-Content -LiteralPath $report -Encoding UTF8
} catch {
    @{ success = $false; error = $_.Exception.Message } | ConvertTo-Json | Set-Content -LiteralPath $report -Encoding UTF8
    throw
}
