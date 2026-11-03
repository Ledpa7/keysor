#ifndef AppVersion
  #define AppVersion "1.0.1"
#endif
[Setup]
AppId=Keysor-Standard
AppName=Keysor
AppVersion={#AppVersion}
DefaultDirName={autopf}\Keysor\standard
DisableDirPage=yes
DefaultGroupName=Keysor
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=Keysor-Setup-{#AppVersion}
SetupIconFile=..\keysor.ico
UninstallDisplayIcon={app}\keysor.exe
Compression=lzma2
SolidCompression=yes
CloseApplications=no
RestartApplications=no
InfoBeforeFile=release-notice.md

[Files]
Source: "..\target\release-package\keysor.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release-package\keysor-guardian.exe"; DestDir: "{app}"; Flags: ignoreversion

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; Flags: unchecked

[Icons]
Name: "{group}\Keysor"; Filename: "{app}\keysor.exe"
Name: "{commondesktop}\Keysor"; Filename: "{app}\keysor.exe"; Tasks: desktopicon
