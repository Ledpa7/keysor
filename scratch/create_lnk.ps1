$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("C:\Users\wjdwl\Desktop\keysor.lnk")
$Shortcut.TargetPath = "c:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor\target\release\keysor.exe"
$Shortcut.WorkingDirectory = "c:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor\target\release"
$Shortcut.Save()
Write-Host "Shortcut created successfully pointing to target\release\keysor.exe"
