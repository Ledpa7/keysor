$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut('C:\Users\wjdwl\Desktop\keysor.lnk')
$Shortcut.TargetPath = 'C:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor\target\release\keysor.exe'
$Shortcut.WorkingDirectory = 'C:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor'
$Shortcut.IconLocation = 'C:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor\keysor.ico'
$Shortcut.Save()
Write-Output "Shortcut updated successfully!"
