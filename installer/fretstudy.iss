; Inno Setup script for the Windows installer (https://jrsoftware.org/isinfo.php).
; Built by scripts/package-windows.ps1. Installs per-user, so no admin password is needed.
#define AppVersion GetEnv("FRETSTUDY_VERSION")

[Setup]
AppId={{6E4A3C2B-5F1D-4C7A-9B8E-2D1F0A9C7E31}
AppName=FretStudy
AppVersion={#AppVersion}
AppPublisher=FretStudy
DefaultDirName={localappdata}\Programs\FretStudy
DefaultGroupName=FretStudy
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=FretStudy-{#AppVersion}-setup
SetupIconFile=..\assets\fretstudy.ico
UninstallDisplayIcon={app}\FretStudy.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Files]
Source: "..\target\release\fretstudy.exe"; DestDir: "{app}"; DestName: "FretStudy.exe"; Flags: ignoreversion

[Icons]
Name: "{group}\FretStudy"; Filename: "{app}\FretStudy.exe"
Name: "{autodesktop}\FretStudy"; Filename: "{app}\FretStudy.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Run]
Filename: "{app}\FretStudy.exe"; Description: "Start FretStudy"; Flags: nowait postinstall skipifsilent
