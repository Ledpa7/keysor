param(
    [string]$Executable = "$env:ProgramFiles\Keysor\standard\keysor.exe",
    [string]$ReportDirectory = (Join-Path ([Environment]::GetFolderPath('Desktop')) 'Keysor-Test-Results')
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Path $ReportDirectory -Force | Out-Null
$id = Get-Date -Format 'yyyyMMdd-HHmmss-fff'
$reportPath = Join-Path $ReportDirectory "verification-$id.json"
$report = [ordered]@{
    executable = $Executable
    signature = $null
    sha256 = $null
    ui_access = $false
    input_hook_started = $null
    passed = $false
    error = $null
    manual_checks = @('Normal typing while OFF', 'Start/Search/Win+A cursor movement and clicks', 'CapsLock and Alt+Tab HUD minimization', 'R/F then Space page jump without click', 'Exit restores normal pointer and typing')
}
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    $signature = Get-AuthenticodeSignature -LiteralPath $Executable
    $report.signature = [string]$signature.Status
    $report.sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
    if ($signature.Status -notin @('NotSigned', 'Valid')) { throw 'Unexpected signature status. No certificate or security settings were changed.' }
    $probePath = Join-Path $ReportDirectory "token-$id.json"
    # This entry point exits before hooks, UI, watchdogs and single-instance logic.
    $process = Start-Process -FilePath $Executable -ArgumentList ('--diagnose-uiaccess "' + $probePath + '"') -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit(15000)) { throw 'Token probe timed out; existing Keysor processes were not stopped.' }
    $probe = Get-Content -LiteralPath $probePath -Raw | ConvertFrom-Json
    $report.ui_access = $probe.ui_access
    $report.input_hook_started = $probe.input_hook_started
    if ($probe.executable -ne $Executable -or $probe.ui_access -ne $false -or $probe.input_hook_started -ne $false) {
        throw 'Expected standard non-UIAccess build with no hook started.'
    }
    $report.passed = $true
} catch {
    $report.error = $_.Exception.Message
} finally {
    $report | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $reportPath -Encoding UTF8
}
Write-Output "Report: $reportPath"
if (-not $report.passed) { Write-Output $report.error; exit 1 }
Write-Output 'PASS: standard non-UIAccess executable; no input hook started. Complete the manual checks in the report.'
