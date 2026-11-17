$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
 $oldTest=$env:KEYSOR_TEST_BUILD
 $oldUI=$env:KEYSOR_UIACCESS_BUILD
 try {
  Remove-Item Env:KEYSOR_TEST_BUILD -ErrorAction SilentlyContinue
  Remove-Item Env:KEYSOR_UIACCESS_BUILD -ErrorAction SilentlyContinue
  & cargo build --release --offline --bin keysor --bin keysor-guardian --bin keysor_recover
  if($LASTEXITCODE -ne 0){throw 'Portable build failed'}
 } finally {$env:KEYSOR_TEST_BUILD=$oldTest; $env:KEYSOR_UIACCESS_BUILD=$oldUI}
 $folder=Join-Path $root 'dist\Keysor-Portable'
 New-Item -ItemType Directory -Path $folder -Force | Out-Null
 Copy-Item -LiteralPath 'target\release\keysor.exe' -Destination (Join-Path $folder 'Keysor.exe') -Force
 Copy-Item -LiteralPath 'target\release\keysor-guardian.exe' -Destination (Join-Path $folder 'keysor-guardian.exe') -Force
 Copy-Item -LiteralPath 'target\release\keysor_recover.exe' -Destination (Join-Path $folder 'Keysor-Recover.exe') -Force
 Copy-Item -LiteralPath 'PORTABLE_WINDOWS.md' -Destination (Join-Path $folder 'README.md') -Force
 $hashes=@(Get-ChildItem -LiteralPath $folder -Filter '*.exe' | Get-FileHash -Algorithm SHA256)
 if(@($hashes.Hash | Select-Object -Unique).Count -ne 3){throw 'Main, Guardian and Recovery must be different executable images'}
 $hashes | ForEach-Object {$_.Hash.ToLowerInvariant()+'  '+(Split-Path $_.Path -Leaf)} | Set-Content -LiteralPath (Join-Path $folder 'SHA256SUMS.txt') -Encoding ASCII
 Compress-Archive -Path (Join-Path $folder '*') -DestinationPath (Join-Path $root 'dist\Keysor-Portable.zip') -Force
 $hashes | ConvertTo-Json | Set-Content -LiteralPath 'target\portable-package-hashes.json'
} finally {Pop-Location}
