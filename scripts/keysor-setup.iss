#ifndef AppVersion
  #define AppVersion "1.0.3"
#endif
#ifndef PackageOutputDir
  #define PackageOutputDir "..\dist"
#endif
[Setup]
AppId=Keysor-Standard
AppName=Keysor
AppVersion={#AppVersion}
DefaultDirName={autopf}\Keysor\standard
DisableDirPage=yes
UsePreviousAppDir=no
DefaultGroupName=Keysor
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#PackageOutputDir}
OutputBaseFilename=Keysor-Setup-{#AppVersion}
SetupIconFile=..\keysor.ico
UninstallDisplayIcon={app}\keysor.exe
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
CloseApplicationsFilter=keysor.exe,keysor-guardian.exe
RestartApplications=no
InfoBeforeFile=release-notice.md

[Files]
Source: "..\target\release-package\keysor.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release-package\keysor-guardian.exe"; DestDir: "{app}"; Flags: ignoreversion

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"

[Icons]
Name: "{group}\Keysor"; Filename: "{app}\keysor.exe"
Name: "{userdesktop}\Keysor"; Filename: "{app}\keysor.exe"; Tasks: desktopicon

[Code]
var
  SavedArguments: String;

function CompatibilityArguments(const Value: String): String;
begin
  Result := '';
  if Pos('--chrome-remote', Value) > 0 then Result := '--chrome-remote';
  if Pos('--diagnose-input', Value) > 0 then Result := Trim(Result + ' --diagnose-input');
end;

function InitializeSetup(): Boolean;
var
  Shell, Shortcut: Variant;
  ShortcutPath: String;
begin
  SavedArguments := '';
  ShortcutPath := ExpandConstant('{userdesktop}\Keysor.lnk');
  if FileExists(ShortcutPath) then begin
    try
      Shell := CreateOleObject('WScript.Shell');
      Shortcut := Shell.CreateShortcut(ShortcutPath);
      SavedArguments := CompatibilityArguments(Shortcut.Arguments);
    except
      Log('Could not read previous desktop shortcut arguments');
    end;
  end;
  Result := True;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Window: HWND;
  Attempt: Integer;
begin
  Result := '';
  Window := FindWindowByClassName('KeysorMainClass');
  if Window <> 0 then begin
    PostMessage(Window, $0010, 0, 0);
    for Attempt := 1 to 100 do begin
      if FindWindowByClassName('KeysorMainClass') = 0 then Break;
      Sleep(100);
    end;
    if FindWindowByClassName('KeysorMainClass') <> 0 then
      Result := 'Please exit Keysor and retry installation.'
    else
      Sleep(500); { Allow the independent Guardian to finish restoring the cursor. }
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Shell, Shortcut: Variant;
  Path, Existing: String;
begin
  if CurStep = ssPostInstall then begin
    Shell := CreateOleObject('WScript.Shell');
    Path := ExpandConstant('{userdesktop}\Keysor.lnk');
    if WizardIsTaskSelected('desktopicon') then begin
      Shortcut := Shell.CreateShortcut(Path);
      Shortcut.TargetPath := ExpandConstant('{app}\keysor.exe');
      Shortcut.WorkingDirectory := ExpandConstant('{app}');
      Shortcut.Arguments := SavedArguments;
      Shortcut.Save;
    end;
    { Preserve an enabled startup preference while removing the old build path. }
    Path := 'Software\Microsoft\Windows\CurrentVersion\Run';
    if RegQueryStringValue(HKCU, Path, 'Keysor', Existing) then
      RegWriteStringValue(HKCU, Path, 'Keysor', '"' + ExpandConstant('{app}\keysor.exe') + '" ' + CompatibilityArguments(Existing));
    Path := ExpandConstant('{userstartup}\Keysor.lnk');
    if FileExists(Path) then begin
      Shortcut := Shell.CreateShortcut(Path);
      Existing := Shortcut.Arguments;
      Shortcut.TargetPath := ExpandConstant('{app}\keysor.exe');
      Shortcut.WorkingDirectory := ExpandConstant('{app}');
      Shortcut.Arguments := CompatibilityArguments(Existing);
      Shortcut.Save;
    end;
  end;
end;

function InitializeUninstall(): Boolean;
var
  NeedsRestart: Boolean;
begin
  Result := PrepareToInstall(NeedsRestart) = '';
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Existing, Path: String;
  Shell, Shortcut: Variant;
begin
  if CurUninstallStep = usUninstall then begin
    Path := 'Software\Microsoft\Windows\CurrentVersion\Run';
    if RegQueryStringValue(HKCU, Path, 'Keysor', Existing) then
      if Pos('"' + ExpandConstant('{app}\keysor.exe') + '"', Existing) = 1 then
        RegDeleteValue(HKCU, Path, 'Keysor');
    Path := ExpandConstant('{userstartup}\Keysor.lnk');
    if FileExists(Path) then begin
      try
        Shell := CreateOleObject('WScript.Shell');
        Shortcut := Shell.CreateShortcut(Path);
        if CompareText(Shortcut.TargetPath, ExpandConstant('{app}\keysor.exe')) = 0 then
          DeleteFile(Path);
      except
        Log('Could not inspect the Keysor startup shortcut');
      end;
    end;
  end;
end;
