; Inno Setup 6 script for the Windows installer.
; Build: ISCC /DAppVersion=X.Y.Z /DSourceDir=<staged app dir> /DOutputDir=<dir> pgs2srt.iss

#ifndef AppVersion
  #error AppVersion must be defined
#endif
#ifndef SourceDir
  #error SourceDir must be defined
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif

[Setup]
AppId={{6F0C7E64-2B1D-4C1B-9B1E-6C2E8B5A9D41}
AppName=pgs2srt
AppVersion={#AppVersion}
AppVerName=pgs2srt {#AppVersion}
AppPublisher=Milan Bolaric
AppPublisherURL=https://github.com/mbolaric/pgs2srt
AppSupportURL=https://github.com/mbolaric/pgs2srt/issues
DefaultDirName={autopf}\pgs2srt
DefaultGroupName=pgs2srt
DisableProgramGroupPage=yes
; Per-user install by default, no administrator rights needed; the dialog offers all users.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=pgs2srt-{#AppVersion}-windows-x64-setup
SetupIconFile={#SourceDir}\pgs2srt.ico
UninstallDisplayIcon={app}\pgs2srt-desktop.exe
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\pgs2srt"; Filename: "{app}\pgs2srt-desktop.exe"; IconFilename: "{app}\pgs2srt.ico"
Name: "{autodesktop}\pgs2srt"; Filename: "{app}\pgs2srt-desktop.exe"; IconFilename: "{app}\pgs2srt.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\pgs2srt-desktop.exe"; Description: "{cm:LaunchProgram,pgs2srt}"; Flags: nowait postinstall skipifsilent
