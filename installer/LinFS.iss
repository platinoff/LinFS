; LinFS 1.0.0 Inno Setup — bundles winfsp.msi silent install (band 212)
[Setup]
AppName=LinFS
AppVersion=1.0.0
AppPublisher=platinoff
DefaultDirName={pf}\LinFS
OutputBaseFilename=LinFS-1.0.0-x64
Compression=lzma
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64

[Files]
Source: "..\target\release\linfs.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "winfsp.msi"; DestDir: "{tmp}"; Flags: deleteafterinstall

[Run]
Filename: "msiexec.exe"; Parameters: "/i ""{tmp}\winfsp.msi"" /quiet /norestart"; StatusMsg: "Installing WinFSP driver..."; Flags: waituntilterminated
Filename: "{app}\linfs.exe"; Description: "Run LinFS"; Flags: nowait postinstall skipifsilent

[Icons]
Name: "{group}\LinFS"; Filename: "{app}\linfs.exe"
Name: "{group}\LinFS GUI"; Filename: "{app}\linfs-gui.exe"
