Keysor standard Windows build (unsigned)

Setup uses the fixed location Program Files\Keysor\standard for new installations and upgrades. Setup asks running Keysor processes to close before replacing files; it does not launch Keysor automatically. Save ongoing work before installing. The desktop shortcut is selected by default and points to this installation.
Existing user settings in %USERPROFILE%\.keysor are preserved. Use the desktop or Start menu shortcut after installation. Click Info / Support at the bottom right of the HUD to see the version and source build ID, copy a report, or check for updates. Reports are only copied on request and are not uploaded. Update checks query the public GitHub release endpoint; installation remains a separate step.
The installer includes a separate, small keysor-guardian.exe helper. Only Keysor has a user-facing shortcut; the helper starts automatically and has no input hook or UI. Keep both files together for portable use. The helper can recover the native cursor when only the Keysor process is terminated, but survival during actual Defender quarantine or whole-process-tree termination is not guaranteed or tested.

No paid signing service or certificate registration is required. This is an unsigned, non-UIAccess build. Windows may show an unknown-publisher or SmartScreen warning, and managed PCs may block it. Do not disable security protections to install it.

Setup and Keysor request administrator approval. No certificates, UAC policies or Defender settings are modified. Over Windows Start, Search and quick-settings surfaces, Keysor uses the native Windows pointer instead of its custom overlay. In Chrome Remote Desktop, Windows may suppress that pointer, making it invisible on those panels; this remains unresolved in the unsigned build. Other keyboard mappings are unchanged.

After installing, close any old Keysor instance and launch Keysor from the Start menu. Test normal typing while OFF, movement/clicks in Start/Search/Win+A while ON, CapsLock and Alt+Tab HUD behavior, R/F then Space page jumps, and exit/restart. A second-PC installation/uninstallation test is still required before publishing a final release.

Optional hook-free diagnostic (from the folder containing Verify-Keysor.ps1):
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\Verify-Keysor.ps1
This command applies execution policy only to this PowerShell process. Results are written to Desktop\Keysor-Test-Results.
