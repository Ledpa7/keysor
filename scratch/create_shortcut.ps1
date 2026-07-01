$desktop = [Environment]::GetFolderPath("Desktop")
$shortcutPath = Join-Path $desktop "Keysor.lnk"
$targetPath = "c:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor\target\release\keysor_rounded.exe"
$workingDir = "c:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor"

$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut($shortcutPath)
$Shortcut.TargetPath = $targetPath
$Shortcut.WorkingDirectory = $workingDir
$Shortcut.Save()
Write-Host "Keysor shortcut created at $shortcutPath"
