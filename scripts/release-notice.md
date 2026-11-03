Keysor standard Windows build (unsigned)

Close Keysor before installing or uninstalling. Setup does not stop running keyboard hooks or launch Keysor automatically.
Existing user settings are preserved. Use the new Start menu shortcut after installation.
The installer includes keysor-guardian.exe so forced termination of keysor.exe can still trigger cursor recovery. If using portable files, keep Keysor.exe and Keysor-Guardian.exe together in the same folder.

No paid signing service or certificate registration is required. This is an unsigned, non-UIAccess build. Windows may show an unknown-publisher or SmartScreen warning, and managed PCs may block it. Do not disable security protections to install it.

Setup and Keysor request administrator approval. No certificates, UAC policies or Defender settings are modified. Over Windows Start, Search and quick-settings surfaces, Keysor uses the native Windows pointer instead of its custom overlay. In Chrome Remote Desktop, Windows may suppress that pointer, making it invisible on those panels; this remains unresolved in the unsigned build. Other keyboard mappings are unchanged.

After installing, close any old Keysor instance and launch Keysor from the Start menu. Test normal typing while OFF, movement/clicks in Start/Search/Win+A while ON, CapsLock and Alt+Tab HUD behavior, R/F then Space page jumps, and exit/restart. A second-PC installation/uninstallation test is still required before publishing a final release.

Optional hook-free diagnostic (from the folder containing Verify-Keysor.ps1):
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\Verify-Keysor.ps1
This command applies execution policy only to this PowerShell process. Results are written to Desktop\Keysor-Test-Results.
