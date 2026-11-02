$ErrorActionPreference = 'Continue'
$previous = $env:KEYSOR_TEST_BUILD
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $env:KEYSOR_TEST_BUILD = '1'
    cargo test --bin keysor --offline
    $result = $LASTEXITCODE
} finally {
    if ($null -eq $previous) { Remove-Item Env:KEYSOR_TEST_BUILD -ErrorAction SilentlyContinue }
    else { $env:KEYSOR_TEST_BUILD = $previous }
    Pop-Location
}
exit $result
