Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public class Keyboard {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);

    public const byte VK_CAPITAL = 0x14;
    public const uint KEYEVENTF_KEYUP = 0x0002;

    public static void PressCapsLock() {
        keybd_event(VK_CAPITAL, 0, 0, UIntPtr.Zero);
        System.Threading.Thread.Sleep(100);
        keybd_event(VK_CAPITAL, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
    }
}
"@

[Keyboard]::PressCapsLock()
Write-Output "CapsLock pressed and released via user32!"
