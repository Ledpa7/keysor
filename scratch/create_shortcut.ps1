if (Test-Path "C:\Users\wjdwl\Desktop\keysor.exe") {
    Remove-Item "C:\Users\wjdwl\Desktop\keysor.exe" -Force
    Write-Host "Removed keysor.exe from Desktop"
}
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("C:\Users\wjdwl\Desktop\keysor.lnk")
$Shortcut.TargetPath = "c:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor\target\release\keysor.exe"
$Shortcut.WorkingDirectory = "c:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor"
$Shortcut.Save()
Write-Host "Successfully created keysor.lnk on Desktop"
