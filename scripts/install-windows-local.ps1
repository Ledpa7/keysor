# Installs the locally built package, preserves shortcuts/settings, and verifies the running files.
# Run from an elevated PowerShell. No public release is created.
param([string]$SetupPath, [string]$ReportPath)
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
if (!$SetupPath) {
 $version=[regex]::Match((Get-Content (Join-Path $root 'Cargo.toml') -Raw), '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
 if(!$version){throw 'Missing Cargo package version'}
 $SetupPath=Join-Path $root "dist\Keysor-Setup-$version.exe"
}
if (!$ReportPath) {$ReportPath=Join-Path $root 'target\standard-install-result.json'}
$dest=Join-Path $env:ProgramFiles 'Keysor\standard'
$backup=Join-Path $root ('target\standard-install-backup-'+(Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $backup | Out-Null
try {
 $SetupPath=(Resolve-Path -LiteralPath $SetupPath).Path
 $desktop=Join-Path ([Environment]::GetFolderPath('Desktop')) 'Keysor.lnk'
 $ws=New-Object -ComObject WScript.Shell
 if(Test-Path -LiteralPath $desktop) {Copy-Item -LiteralPath $desktop -Destination (Join-Path $backup 'Keysor.lnk')}
 foreach($name in @('keysor.exe','keysor-guardian.exe')) {
  $existing=Join-Path $dest $name
  if(Test-Path -LiteralPath $existing) {Copy-Item -LiteralPath $existing -Destination (Join-Path $backup $name)}
 }
 $config=Join-Path $env:USERPROFILE '.keysor\keysor.yaml'
 $configBefore=if(Test-Path -LiteralPath $config){(Get-FileHash -LiteralPath $config).Hash}else{$null}
 $log=Join-Path $backup 'setup.log'
 $installer=Start-Process -FilePath $SetupPath -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/TASKS=desktopicon',('/LOG="'+$log+'"')) -WindowStyle Hidden -PassThru
 if(!$installer.WaitForExit(60000)) {throw 'Installer did not finish within 60 seconds'}
 if($installer.ExitCode -ne 0) {throw "Installer failed: $($installer.ExitCode). See $log"}
 $configAfter=if(Test-Path -LiteralPath $config){(Get-FileHash -LiteralPath $config).Hash}else{$null}
 if($configBefore -ne $configAfter) {throw 'Settings unexpectedly changed during installation'}
 foreach($name in @('keysor.exe','keysor-guardian.exe')) {
  if((Get-FileHash -LiteralPath (Join-Path $root "target\release-package\$name")).Hash -ne (Get-FileHash -LiteralPath (Join-Path $dest $name)).Hash) {throw "Installed file mismatch: $name"}
 }
 $link=$ws.CreateShortcut($desktop)
 if($link.TargetPath -ne (Join-Path $dest 'keysor.exe')) {throw 'Desktop shortcut does not target the standard installation'}
 $probe=Join-Path $backup 'support-report.txt'
 $process=Start-Process -FilePath $link.TargetPath -ArgumentList ('--support-report "'+$probe+'"') -WindowStyle Hidden -PassThru
 if(!$process.WaitForExit(15000) -or $process.ExitCode -ne 0 -or !(Test-Path -LiteralPath $probe)) {throw 'Hook-free support report probe failed'}
 Start-Process -FilePath $link.TargetPath -ArgumentList $link.Arguments -WorkingDirectory $dest -WindowStyle Hidden
 Start-Sleep -Seconds 4
 $running=@(Get-CimInstance Win32_Process -Filter "name='keysor.exe' OR name='keysor-guardian.exe'" | Select-Object Name,ProcessId,ExecutablePath,CommandLine)
 if(@($running | Where-Object {$_.ExecutablePath -eq (Join-Path $dest 'keysor.exe')}).Count -ne 1) {throw 'Expected one main process from standard installation'}
 if(@($running | Where-Object {$_.ExecutablePath -ne (Join-Path $dest $_.Name)}).Count -gt 0) {throw 'An old Keysor process is still running'}
 [pscustomobject]@{Success=$true;Setup=$SetupPath;InstallDirectory=$dest;Backup=$backup;Shortcut=$desktop;Arguments=$link.Arguments;SettingsPreserved=($configBefore -eq $configAfter);SupportReport=$probe;Processes=$running;Hashes=@(Get-ChildItem $dest -Filter 'keysor*.exe'|Get-FileHash)} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $ReportPath
} catch {
 [pscustomobject]@{Success=$false;Error=$_.Exception.Message;Backup=$backup} | ConvertTo-Json | Set-Content -LiteralPath $ReportPath
 exit 1
}
