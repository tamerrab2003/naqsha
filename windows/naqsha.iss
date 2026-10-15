; Naqsha installer for Windows (Inno Setup 6). Built by .github/workflows/release.yml:
;   iscc /DAppVersion=0.1.0 windows\naqsha.iss
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{6C4DFF19-C3DD-4E0A-9A71-0F0F1A6E7173}
AppName=Naqsha
AppVersion={#AppVersion}
AppPublisher=Nixpx LLC
AppPublisherURL=https://nixpx.com/naqsha
AppSupportURL=https://github.com/tamerrab2003/naqsha/issues
DefaultDirName={autopf}\Naqsha
DefaultGroupName=Naqsha
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=..\dist
OutputBaseFilename=Naqsha-{#AppVersion}-windows-x64-setup
SetupIconFile=naqsha.ico
UninstallDisplayIcon={app}\Naqsha.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; per-user install by default (no admin prompt); the user can choose all users
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ChangesAssociations=yes

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "assoc"; Description: "Open .nqs diagrams with Naqsha, and list it under Open with for .vsdx and .drawio"; GroupDescription: "File types:"

[Files]
Source: "..\target\release\naqsha.exe"; DestDir: "{app}"; DestName: "Naqsha.exe"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Naqsha"; Filename: "{app}\Naqsha.exe"
Name: "{autodesktop}\Naqsha"; Filename: "{app}\Naqsha.exe"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Classes\Naqsha.Diagram"; ValueType: string; ValueData: "Naqsha diagram"; Flags: uninsdeletekey; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Naqsha.Diagram\DefaultIcon"; ValueType: string; ValueData: "{app}\Naqsha.exe,0"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Naqsha.Diagram\shell\open\command"; ValueType: string; ValueData: """{app}\Naqsha.exe"" ""%1"""; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.nqs"; ValueType: string; ValueData: "Naqsha.Diagram"; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.schx"; ValueType: string; ValueData: "Naqsha.Diagram"; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.drawio\OpenWithProgids"; ValueType: string; ValueName: "Naqsha.Diagram"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.vsdx\OpenWithProgids"; ValueType: string; ValueName: "Naqsha.Diagram"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc

[Run]
Filename: "{app}\Naqsha.exe"; Description: "{cm:LaunchProgram,Naqsha}"; Flags: nowait postinstall skipifsilent
