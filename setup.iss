; Inno Setup Script for the Liem Desktop Ecosystem
; Compile this script using Inno Setup Compiler (ISCC) to generate the installer.

[Setup]
AppName=Liem Desktop Ecosystem
AppVersion=0.2.5
AppPublisher=Liem Ecosystem Contributors
DefaultDirName=D:\Liem Desktop Ecosystem
DefaultGroupName=Liem Desktop Ecosystem
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\Liem Wallpaper\lw-service.exe
Compression=lzma2
SolidCompression=yes
OutputDir=target\installer
OutputBaseFilename=LiemEcosystemSetup
ChangesEnvironment=yes

[Types]
Name: "full"; Description: "Full installation"
Name: "custom"; Description: "Custom installation"; Flags: iscustom

[Components]
Name: "wallpaper"; Description: "Liem Wallpaper (GPU-Accelerated Background Manager)"; Types: full custom
Name: "bar"; Description: "Liem Bar (Desktop Status Panel)"; Types: full custom

[Files]
; Unified CLI
Source: "target\release\liem.exe"; DestDir: "{app}"; Flags: ignoreversion
; Liem Wallpaper files
Source: "target\release\lw-service.exe"; DestDir: "{app}\Liem Wallpaper"; Components: wallpaper; Flags: ignoreversion
Source: "target\release\lw.exe"; DestDir: "{app}\Liem Wallpaper"; Components: wallpaper; Flags: ignoreversion
Source: "apps\liem-wallpaper\shaders\*"; DestDir: "{app}\Liem Wallpaper\shaders"; Components: wallpaper; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "apps\liem-wallpaper\assets\icon.ico"; DestDir: "{app}\Liem Wallpaper"; Components: wallpaper; Flags: ignoreversion
; Liem Bar files
Source: "target\release\liem-bar.exe"; DestDir: "{app}\Liem Bar"; Components: bar; Flags: ignoreversion
Source: "target\release\lb.exe"; DestDir: "{app}\Liem Bar"; Components: bar; Flags: ignoreversion
Source: "apps\liem-bar\profiles\*"; DestDir: "{app}\Liem Bar\profiles"; Components: bar; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Liem Wallpaper Service"; Filename: "{app}\Liem Wallpaper\lw-service.exe"; Components: wallpaper
Name: "{group}\Liem Bar"; Filename: "{app}\Liem Bar\liem-bar.exe"; Components: bar
Name: "{group}\Liem CLI"; Filename: "{app}\liem.exe"

[Run]
; Spawn wallpaper service after successful installation
Filename: "{app}\Liem Wallpaper\lw-service.exe"; Description: "Start Liem Wallpaper Service"; Components: wallpaper; Flags: nowait postinstall runhidden
Filename: "{app}\Liem Bar\liem-bar.exe"; Description: "Start Liem Bar"; Components: bar; Flags: nowait postinstall runhidden

[Registry]
; Add Liem Wallpaper installation directory to User PATH
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}\Liem Wallpaper"; Components: wallpaper; Check: NeedsAddPath
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}\Liem Bar"; Components: bar; Check: NeedsAddPathBar
; Add Unified CLI installation directory to User PATH
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Check: NeedsAddPathCli

[UninstallDelete]
Type: filesandordirs; Name: "{app}\Liem Wallpaper\config.json"
Type: filesandordirs; Name: "{app}\Liem Bar\config.json"
Type: filesandordirs; Name: "{app}\config.json"
Type: filesandordirs; Name: "{app}\Liem Wallpaper\*.log"
Type: filesandordirs; Name: "{app}\Liem Bar\*.log"
Type: filesandordirs; Name: "{app}\*.log"
Type: filesandordirs; Name: "{app}"

[Code]
var
  WallpaperInstallDir: String;
  WallpaperAlreadyInstalled: Boolean;

// Helper to check if Liem Wallpaper is already installed and retrieve its path
function GetWallpaperInstallDir(var Path: String): Boolean;
begin
  Result := False;
  // Check the registry uninstall key of the standalone Liem Wallpaper installation
  if RegQueryStringValue(HKEY_CURRENT_USER, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\LiemWallpaper', 'InstallLocation', Path) then
  begin
    if Path <> '' then Result := True;
  end;
  
  if not Result then
  begin
    if RegQueryStringValue(HKEY_CURRENT_USER, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Liem Wallpaper_is1', 'InstallLocation', Path) then
    begin
      if Path <> '' then Result := True;
    end;
  end;
end;

procedure CleanOldPath(OldInstallDir: String);
var
  Path: String;
  PosOldDir: Integer;
begin
  if OldInstallDir <> '' then
  begin
    if RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path) then
    begin
      PosOldDir := Pos(';' + Uppercase(OldInstallDir), Uppercase(Path));
      if PosOldDir > 0 then
      begin
        Delete(Path, PosOldDir, Length(';' + OldInstallDir));
        RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path);
      end;
      
      PosOldDir := Pos(Uppercase(OldInstallDir) + ';', Uppercase(Path));
      if PosOldDir > 0 then
      begin
        Delete(Path, PosOldDir, Length(OldInstallDir + ';'));
        RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path);
      end;
      
      PosOldDir := Pos(Uppercase(OldInstallDir), Uppercase(Path));
      if PosOldDir > 0 then
      begin
        Delete(Path, PosOldDir, Length(OldInstallDir));
        RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path);
      end;
    end;
  end;
end;

function InitializeSetup(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;
  WallpaperAlreadyInstalled := GetWallpaperInstallDir(WallpaperInstallDir);
  
  if WallpaperAlreadyInstalled then
  begin
    CleanOldPath(WallpaperInstallDir);
  end;
  
  // Terminate any running instances of the daemon before installing
  Exec('taskkill.exe', '/F /IM lw-service.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM liem-bar.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM liem.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

function InitializeUninstall(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;
  // Terminate background services first so Inno Setup doesn't get file locks
  Exec('taskkill.exe', '/F /IM lw-service.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM lw.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM liem-bar.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM lb.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM liem.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

procedure InitializeWizard();
var
  InfoLabel: TNewStaticText;
begin
  // If Liem Wallpaper is already installed on the system, show an info message on the Welcome page
  if WallpaperAlreadyInstalled then
  begin
    InfoLabel := TNewStaticText.Create(WizardForm);
    InfoLabel.Parent := WizardForm.WelcomePage;
    InfoLabel.Left := ScaleX(176);
    InfoLabel.Top := ScaleY(180);
    InfoLabel.Width := ScaleX(300);
    InfoLabel.Height := ScaleY(60);
    InfoLabel.AutoSize := False;
    InfoLabel.WordWrap := True;
    InfoLabel.Font.Style := [fsBold];
    InfoLabel.Font.Color := clBlue;
    InfoLabel.Caption := 'Note: Liem Wallpaper was detected on this system at: ' + #13#10 + WallpaperInstallDir + #13#10 + 'The installer will automatically clean up old paths.';
  end;
end;

function NeedsAddPath(): Boolean;
var
  Path: String;
  AppDir: String;
  AppDirSemi: String;
begin
  AppDir := ExpandConstant('{app}\Liem Wallpaper');
  if RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path) then
  begin
    AppDirSemi := ';' + AppDir + ';';
    Result := (Pos(Uppercase(AppDirSemi), ';' + Uppercase(Path) + ';') = 0);
  end
  else
  begin
    Result := True;
  end;
end;

function NeedsAddPathBar(): Boolean;
var
  Path: String;
  AppDir: String;
  AppDirSemi: String;
begin
  AppDir := ExpandConstant('{app}\Liem Bar');
  if RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path) then
  begin
    AppDirSemi := ';' + AppDir + ';';
    Result := (Pos(Uppercase(AppDirSemi), ';' + Uppercase(Path) + ';') = 0);
  end
  else
  begin
    Result := True;
  end;
end;

function NeedsAddPathCli(): Boolean;
var
  Path: String;
  AppDir: String;
  AppDirSemi: String;
begin
  AppDir := ExpandConstant('{app}');
  if RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path) then
  begin
    AppDirSemi := ';' + AppDir + ';';
    Result := (Pos(Uppercase(AppDirSemi), ';' + Uppercase(Path) + ';') = 0);
  end
  else
  begin
    Result := True;
  end;
end;

procedure CleanPathSegment(AppDir: String);
var
  Path: String;
  PosAppDir: Integer;
begin
  if RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path) then
  begin
    PosAppDir := Pos(';' + Uppercase(AppDir), Uppercase(Path));
    if PosAppDir > 0 then
    begin
      Delete(Path, PosAppDir, Length(';' + AppDir));
      RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path);
    end;
    
    PosAppDir := Pos(Uppercase(AppDir) + ';', Uppercase(Path));
    if PosAppDir > 0 then
    begin
      Delete(Path, PosAppDir, Length(AppDir + ';'));
      RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path);
    end;
    
    PosAppDir := Pos(Uppercase(AppDir), Uppercase(Path));
    if PosAppDir > 0 then
    begin
      Delete(Path, PosAppDir, Length(AppDir));
      RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path);
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    CleanPathSegment(ExpandConstant('{app}\Liem Wallpaper'));
    CleanPathSegment(ExpandConstant('{app}\Liem Bar'));
    CleanPathSegment(ExpandConstant('{app}'));
  end;
end;
