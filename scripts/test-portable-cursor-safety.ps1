# Runs isolated desktop cursor tests. Always restores the pointer and restarts Keysor.
param([switch]$ExternalAppControl)
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $root
$runDir=Join-Path $root ('target\portable-native-'+(Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $runDir | Out-Null
$testExe=(Get-ChildItem 'target\debug\deps\keysor-*.exe' | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$recover=Join-Path $root 'dist\Keysor-Portable\Keysor-Recover.exe'
$guardian=Join-Path $root 'dist\Keysor-Portable\keysor-guardian.exe'
$original=@(Get-Process keysor -ErrorAction SilentlyContinue)
$shortcut=(New-Object -ComObject WScript.Shell).CreateShortcut('C:\Users\wjdwl\Desktop\Keysor.lnk')
$config='C:\Users\wjdwl\.keysor\keysor.yaml'
$configHash=(Get-FileHash -LiteralPath $config).Hash
$stopped=$false
$children=@()
$guardians=@()
$results=[ordered]@{Success=$false;RunDirectory=$runDir;Checks=@();Error=$null}
function Recover([string]$label) {
 $report=Join-Path $runDir ($label+'-restore.json')
 $proc=Start-Process -FilePath $recover -ArgumentList @('--report',('"'+$report+'"')) -WindowStyle Hidden -PassThru
 if(!$proc.WaitForExit(10000) -or $proc.ExitCode -ne 0){throw "Recovery process failed: $label"}
 $value=Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
 if(!$value.restore_ok){throw "Recovery API failed: $label"}
}
function NativeTest([string]$name,[string]$label,[bool]$wait=$true) {
 $env:KEYSOR_SAFETY_REPORT=Join-Path $runDir ($label+'.json')
 $env:KEYSOR_SAFETY_GUARDIAN=$guardian
 if($label -eq 'both-killed'){$env:KEYSOR_SAFETY_NO_MONITOR='1'}
 else {Remove-Item Env:KEYSOR_SAFETY_NO_MONITOR -ErrorAction SilentlyContinue}
 $stdout=Join-Path $runDir ($label+'.out')
 $stderr=Join-Path $runDir ($label+'.err')
 $proc=Start-Process -FilePath $testExe -ArgumentList @(('cursor_scheme::native_tests::'+$name),'--ignored','--exact','--test-threads=1') -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
 if($wait){
  if(!$proc.WaitForExit(15000)){Stop-Process -Id $proc.Id -ErrorAction SilentlyContinue;throw "Native test timed out: $label"}
  # Windows PowerShell can lose Start-Process's ExitCode; verify Rust's result
  # and the structured report as well, rather than treating null as failure.
  $proc.Refresh()
  $summary=Get-Content -LiteralPath $stdout -Raw
  if(($null -ne $proc.ExitCode -and $proc.ExitCode -ne 0) -or $summary -notmatch 'test result: ok\. 1 passed; 0 failed;'){throw "Native test failed: $label"}
 }
 else {
  for($i=0;$i -lt 150 -and !(Test-Path -LiteralPath $env:KEYSOR_SAFETY_REPORT);$i++){
   if($proc.HasExited){throw "Probe exited early: $label"}
   Start-Sleep -Milliseconds 50
  }
 }
 if(!(Test-Path -LiteralPath $env:KEYSOR_SAFETY_REPORT)){throw "Missing report: $label"}
 [pscustomobject]@{Process=$proc;Report=$env:KEYSOR_SAFETY_REPORT;Data=(Get-Content -LiteralPath $env:KEYSOR_SAFETY_REPORT -Raw | ConvertFrom-Json)}
}
function SameCursors($a,$b){ (@($a.cursors.hash) -join ',') -eq (@($b.cursors.hash) -join ',') }
try {
 if($original.Count -gt 0 -and !$ExternalAppControl){
  foreach($proc in $original){
   $knownPaths=@($shortcut.TargetPath,'C:\Program Files\Keysor\standard\keysor.exe')
   if(!$proc.Path -or $knownPaths -notcontains $proc.Path){throw 'Cannot verify the running Keysor path; administrator permission is required'}
   Stop-Process -Id $proc.Id
  }
  foreach($proc in $original){if(!$proc.WaitForExit(8000)){throw 'Existing Keysor did not close'}}
  $stopped=$true
  Start-Sleep -Milliseconds 500
 }
 Recover 'baseline'
 $baseline=(NativeTest 'native_snapshot' 'baseline').Data
 if(@($baseline.cursors | Where-Object {$_.visible_pixels -le 10}).Count){throw 'Baseline has invisible cursor roles'}
 $missing=(NativeTest 'native_missing_guardian' 'missing-guardian').Data
 if($missing.custom_applied -or !(SameCursors $baseline $missing)){throw 'Missing Guardian fail-safe changed cursors'}
 $results.Checks+=@{Case='Guardian unavailable';Passed=$true;CustomCursorBlocked=$true;ExactCursorMatch=$true}

 # Parent only dies: independent Guardian must restore the exact baseline artwork.
 $probe=NativeTest 'native_probe_child' 'main-killed' $false
 $children+=$probe.Process
 $guardians+=$probe.Data.guardian_pid
 Stop-Process -Id $probe.Process.Id
 $probe.Process.WaitForExit()
 $auto=$null
 for($i=0;$i -lt 100;$i++){
  $auto=Get-Content -LiteralPath $probe.Data.watchdog_report -Raw | ConvertFrom-Json
  if($auto.restore_ok){break}; Start-Sleep -Milliseconds 50
 }
 $after=(NativeTest 'native_snapshot' 'after-main-killed').Data
 if(!$auto.restore_ok -or !(SameCursors $baseline $after)){throw 'Automatic recovery did not restore baseline'}
 $results.Checks+=@{Case='Main force-terminated';Passed=$true;RestoreAPI=$auto.restore_ok;ExactCursorMatch=$true}

 # Guardian alone dies: main's kernel-handle watcher must restore immediately.
 $probe=NativeTest 'native_probe_child' 'guardian-killed' $false
 $children+=$probe.Process
 $guardians+=$probe.Data.guardian_pid
 Stop-Process -Id $probe.Data.guardian_pid
 $lost=$null
 for($i=0;$i -lt 100;$i++){
  $lost=Get-Content -LiteralPath $probe.Report -Raw | ConvertFrom-Json
  if($lost.guardian_lost){break}; Start-Sleep -Milliseconds 50
 }
 if(!$lost.guardian_lost -or !(SameCursors $baseline $lost)){throw 'Main did not restore after Guardian loss'}
 Stop-Process -Id $probe.Process.Id; $probe.Process.WaitForExit()
 $results.Checks+=@{Case='Guardian force-terminated';Passed=$true;ExactCursorMatch=$true}

 # Both die: no cleanup can run, but Windows must still have all visible artwork.
 $probe=NativeTest 'native_probe_child' 'both-killed' $false
 $children+=$probe.Process
 $guardians+=$probe.Data.guardian_pid
 # Disable only the test parent's cleanup watcher to model simultaneous loss
 # deterministically, then terminate Guardian before the test parent.
 $guardianProcess=Get-Process -Id $probe.Data.guardian_pid
 Stop-Process -Id $guardianProcess.Id
 $guardianProcess.WaitForExit()
 Stop-Process -Id $probe.Process.Id
 $probe.Process.WaitForExit()
 $both=(NativeTest 'native_snapshot' 'after-both-killed').Data
 if(SameCursors $baseline $both){throw 'Both-loss fixture returned to defaults; cannot verify retained custom artwork'}
 if(@($both.cursors | Where-Object {$_.visible_pixels -le 10}).Count){throw 'A cursor became invisible after both processes died'}
 Recover 'manual-after-both'
 $after=(NativeTest 'native_snapshot' 'after-manual').Data
 if(!(SameCursors $baseline $after)){throw 'Manual recovery did not restore baseline'}
 $results.Checks+=@{Case='Both terminated; separate manual recovery';Passed=$true;AllCursorRolesVisible=$true;ExactCursorMatch=$true}

 # Deliberately blank system pointer: independent recovery must also fix that state.
 $blank=(NativeTest 'native_blank_fixture' 'blank-fixture').Data
 if($blank.cursors[0].visible_pixels -ne 0){throw 'Blank fixture was not invisible'}
 Recover 'manual-after-blank'
 $after=(NativeTest 'native_snapshot' 'after-blank-recovery').Data
 if(!(SameCursors $baseline $after)){throw 'Recovery from invisible arrow failed'}
 $results.Checks+=@{Case='Invisible arrow; independent recovery';Passed=$true;ExactCursorMatch=$true}
 $results.Success=$true
} catch {$results.Error=$_.Exception.Message}
finally {
 foreach($proc in $children){if(!$proc.HasExited){Stop-Process -Id $proc.Id -ErrorAction SilentlyContinue}}
 foreach($pidToStop in $guardians){Stop-Process -Id $pidToStop -ErrorAction SilentlyContinue}
 try {Recover 'final'} catch {$results.Success=$false;$results.Error='Final recovery failed: '+$_.Exception.Message}
 $results.SettingsPreserved=((Get-FileHash -LiteralPath $config).Hash -eq $configHash)
 if($results.Success -and !$ExternalAppControl){
  $desktopLink='C:\Users\wjdwl\Desktop\Keysor.lnk'
  Copy-Item -LiteralPath $desktopLink -Destination (Join-Path $runDir 'Keysor-before-portable.lnk')
  $shortcut.TargetPath=Join-Path $root 'dist\Keysor-Portable\Keysor.exe'
  $shortcut.WorkingDirectory=Split-Path $shortcut.TargetPath -Parent
  $shortcut.IconLocation=$shortcut.TargetPath+',0'
  $shortcut.Save()
  $results.ShortcutTarget=$shortcut.TargetPath
 }
 if(!$ExternalAppControl -and ($stopped -or $results.Success)){
  Start-Process -FilePath $shortcut.TargetPath -ArgumentList $shortcut.Arguments -WorkingDirectory $shortcut.WorkingDirectory -WindowStyle Hidden
  Start-Sleep -Seconds 3
  $results.Runtime=@(Get-CimInstance Win32_Process -Filter "name='keysor.exe' OR name='keysor-guardian.exe'" | Select-Object Name,ProcessId,ExecutablePath,CommandLine)
  $results.GuardianStartup=Get-Content 'C:\Users\wjdwl\.keysor\watchdog-startup.json' -Raw | ConvertFrom-Json
 }
 $results | ConvertTo-Json -Depth 6 | Tee-Object -FilePath (Join-Path $root 'target\portable-native-results.json')
}
if(!$results.Success){exit 1}
