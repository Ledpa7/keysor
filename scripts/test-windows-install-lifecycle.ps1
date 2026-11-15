param([string]$Root=(Split-Path $PSScriptRoot -Parent))
$ErrorActionPreference='Stop'
$dest=Join-Path $env:ProgramFiles 'Keysor\standard'
$backup=Join-Path $Root ('target\lifecycle-test-'+(Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $backup | Out-Null
$report=[ordered]@{Success=$false;OtherPCVerified=$false;Backup=$backup;Upgrade=$false;Uninstall=$false;Reinstall=$false;Error=$null}
$desktop=Join-Path ([Environment]::GetFolderPath('Desktop')) 'Keysor.lnk'
$startup=Join-Path ([Environment]::GetFolderPath('Startup')) 'Keysor.lnk'
$run='HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$runValue=Get-ItemPropertyValue -LiteralPath $run -Name Keysor -ErrorAction SilentlyContinue
if(Test-Path -LiteralPath $desktop){Copy-Item -LiteralPath $desktop -Destination (Join-Path $backup 'Keysor.lnk')}
if(Test-Path -LiteralPath $startup){Copy-Item -LiteralPath $startup -Destination (Join-Path $backup 'Startup-Keysor.lnk')}
$config=Join-Path $env:USERPROFILE '.keysor\keysor.yaml'
$configHash=(Get-FileHash -LiteralPath $config).Hash
try {
 & (Join-Path $Root 'scripts\install-windows-local.ps1') -ReportPath (Join-Path $backup 'upgrade.json')
 if(!(Get-Content (Join-Path $backup 'upgrade.json') -Raw|ConvertFrom-Json).Success){throw 'Upgrade verification failed'}
 $report.Upgrade=$true
 $uninstallers=@(Get-ChildItem -LiteralPath $dest -Filter 'unins*.exe')
 if($uninstallers.Count -ne 1){throw 'Expected exactly one standard uninstaller'}
 $p=Start-Process -FilePath $uninstallers[0].FullName -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/LOG="'+(Join-Path $backup 'uninstall.log')+'"')) -WindowStyle Hidden -PassThru
 $processHandle=$p.Handle
 if(!$p.WaitForExit(60000)){throw 'Uninstaller timed out'}
 if($p.ExitCode -ne 0){throw "Uninstaller failed: $($p.ExitCode)"}
 foreach($name in @('keysor.exe','keysor-guardian.exe')){if(Test-Path -LiteralPath (Join-Path $dest $name)){throw "File remained after uninstall: $name"}}
 if(Test-Path -LiteralPath $desktop){throw 'Desktop shortcut remained after uninstall'}
 if(@(Get-CimInstance Win32_Process -Filter "name='keysor.exe' OR name='keysor-guardian.exe'" | Where-Object {$_.ExecutablePath -like "$dest\*"}).Count -ne 0){throw 'Installed Keysor process remained after uninstall'}
 if((Get-FileHash -LiteralPath $config).Hash -ne $configHash){throw 'Settings changed during uninstall'}
 $report.Uninstall=$true
} catch {$report.Error=$_.Exception.Message}
finally {
 try {
  # Restore the prior user preference so Setup can preserve the same launch options.
  if(Test-Path -LiteralPath (Join-Path $backup 'Keysor.lnk')){Copy-Item -LiteralPath (Join-Path $backup 'Keysor.lnk') -Destination $desktop -Force}
  if($null -ne $runValue){Set-ItemProperty -LiteralPath $run -Name Keysor -Value $runValue}
  if(Test-Path -LiteralPath (Join-Path $backup 'Startup-Keysor.lnk')){Copy-Item -LiteralPath (Join-Path $backup 'Startup-Keysor.lnk') -Destination $startup -Force}
  & (Join-Path $Root 'scripts\install-windows-local.ps1') -ReportPath (Join-Path $backup 'reinstall.json')
  $report.Reinstall=[bool](Get-Content (Join-Path $backup 'reinstall.json') -Raw|ConvertFrom-Json).Success
  $report.SettingsPreserved=(Get-FileHash -LiteralPath $config).Hash -eq $configHash
  $report.Success=$report.Upgrade -and $report.Uninstall -and $report.Reinstall -and $report.SettingsPreserved
 } catch {$report.Error="Reinstall failed: $($_.Exception.Message)"}
 $report|ConvertTo-Json -Depth 5|Set-Content -LiteralPath (Join-Path $Root 'target\lifecycle-test-result.json')
}
