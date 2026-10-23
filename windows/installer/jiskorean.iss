; The JIS Korean installer (Inno Setup 6). Build it with tools/package.ps1, which passes:
;   Version   the engine's Cargo.toml version, e.g. 0.3.0
;   Build     windows/build, holding x64, x86, x64-lite and x86-lite
;   FullMB    how much larger the full DLL is than the lite one, in MB, for the choice page
;   Sign      defined when the installer and its uninstaller are signed (SignTool "jk")
; It does what tools/install.ps1 does, and nothing more; keep the two in step.

#ifndef Version
  #error Build with tools/package.ps1
#endif
#define Root "..\.."
#define Clsid "{D10651AD-51EA-4DBA-8BFF-1905261B26A1}"
; Shown in Apps & features and on the finish page.
#define Publisher "Mirae Research"
#define DocsUrl "https://jis-korean.sjun.me"

[Setup]
; Permanent: Windows finds an earlier install (to upgrade it) by this id. Never change it.
AppId={{790CFD3D-B00B-4F42-B96F-915F7DED7605}
AppName=JIS Korean
AppVersion={#Version}
AppVerName=JIS Korean {#Version}
AppPublisher={#Publisher}
AppPublisherURL={#DocsUrl}
AppSupportURL={#DocsUrl}
VersionInfoVersion={#Version}
AppCopyright=Copyright 2026 Mirae Research & Min Jun Sung
; tools/install.ps1's folder, so an upgrade over a development install replaces it in place.
DefaultDirName={autopf}\JIS Korean
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
DisableWelcomePage=no
LicenseFile={#Root}\LICENSE
PrivilegesRequired=admin
; Windows 11 is 64-bit only; the x86 DLL is for 32-bit apps on it.
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
; Every app has the DLL loaded: never offer to close them. The old DLL is moved aside instead
; (RenameAside below), and apps load the new one when they restart.
CloseApplications=no
RestartApplications=no
SetupIconFile={#Root}\windows\icons\taegeuk.ico
UninstallDisplayIcon={app}\taegeuk.ico
UninstallDisplayName=JIS Korean
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
OutputDir={#Build}\dist
OutputBaseFilename=JISKorean-{#Version}-setup
#ifdef Sign
SignTool=jk
SignedUninstaller=yes
#endif

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"

[Messages]
; Not the default "close all other applications": apps can stay open, and restart afterwards.
WelcomeLabel2=This will install [name/ver] on your computer.%n%nYou don't need to close other apps. Apps that are open keep the old version, or none, until they restart.
ConfirmUninstall=Remove JIS Korean from this PC and from your keyboards?
FinishedHeadingLabel=JIS Korean is installed
FinishedLabelNoIcons=Restart the apps that were open: they keep the old version, or none, until they restart.%n%nPress Win+Space and pick JIS Korean.%n%nRight-click the 한글 / ABC indicator in the taskbar to choose Mode A (JIS), Mode B (US ANSI) or Mode C (UK ISO).
FinishedLabel=Restart the apps that were open: they keep the old version, or none, until they restart.%n%nPress Win+Space and pick JIS Korean.%n%nRight-click the 한글 / ABC indicator in the taskbar to choose Mode A (JIS), Mode B (US ANSI) or Mode C (UK ISO).

[Files]
Source: "{#Build}\x64\jiskorean.dll"; DestDir: "{app}\x64"; Flags: ignoreversion; Check: IsFull
Source: "{#Build}\x86\jiskorean.dll"; DestDir: "{app}\x86"; Flags: ignoreversion; Check: IsFull
Source: "{#Build}\x64-lite\jiskorean.dll"; DestDir: "{app}\x64"; Flags: ignoreversion; Check: IsLite
Source: "{#Build}\x86-lite\jiskorean.dll"; DestDir: "{app}\x86"; Flags: ignoreversion; Check: IsLite
; The licenses of the data inside the DLL.
Source: "{#Root}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\engine\dict\LICENSE-libhangul-hanja.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\engine\dict\LICENSE-unicode.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\engine\predict\LICENSE-corpora.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\engine\predict\LICENSE-wordnet.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\windows\icons\taegeuk.ico"; DestDir: "{app}"; Flags: ignoreversion
; Run by Setup and the uninstaller as the user, to change their language list.
Source: "keyboards.ps1"; DestDir: "{app}"; Flags: ignoreversion

[Run]
Filename: "{#DocsUrl}"; Description: "Open the JIS Korean docs"; Flags: postinstall shellexec nowait skipifsilent unchecked runasoriginaluser

[Code]
const
  MOVEFILE_DELAY_UNTIL_REBOOT = 4;

var
  VariantPage: TInputOptionWizardPage;
  ModePage: TInputOptionWizardPage;
  RemoveUserData: Boolean;
  LeftForRestart: Boolean;

{ MoveFileExW with no new name: delete the file when Windows restarts. }
function DeleteAtRestart(Existing: String; NoNewName, Flags: Cardinal): Boolean;
  external 'MoveFileExW@kernel32.dll stdcall';

function IsLite: Boolean;
begin
  Result := VariantPage.SelectedValueIndex = 1;
end;

function IsFull: Boolean;
begin
  Result := not IsLite;
end;

{ A user who already has a mode (an upgrade, or a development install) keeps it. }
function HasMode: Boolean;
begin
  Result := RegValueExists(HKCU, 'Software\JIS Korean', 'Mode');
end;

function PowerShell(Params: String): String;
begin
  Result := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File "'
    + ExpandConstant('{app}\keyboards.ps1') + '" ' + Params;
end;

{ Runs keyboards.ps1 as the user who started Setup: the language list is per user, and the
  elevated process may be another account. The uninstaller can't do that (Inno Setup has no
  ExecAsOriginalUser there): it changes the list of the account it was elevated as, the same
  user unless they elevated with another account's password. }
function RunKeyboards(Params: String): Boolean;
var
  Exe: String;
  Code: Integer;
begin
  Exe := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
  if IsUninstaller then
    Result := Exec(Exe, PowerShell(Params), '', SW_HIDE, ewWaitUntilTerminated, Code)
  else
    Result := ExecAsOriginalUser(Exe, PowerShell(Params), '', SW_HIDE, ewWaitUntilTerminated, Code);
  Result := Result and (Code = 0);
end;

function RegSvr(Arch, Params: String): Boolean;
var
  Exe: String;
  Code: Integer;
begin
  if Arch = 'x64' then Exe := ExpandConstant('{sys}\regsvr32.exe')
  else Exe := ExpandConstant('{syswow64}\regsvr32.exe');
  Result := Exec(Exe, Params + ' "' + ExpandConstant('{app}\' + Arch + '\jiskorean.dll') + '"',
    '', SW_HIDE, ewWaitUntilTerminated, Code) and (Code = 0);
end;

{ Deletes the copies earlier installs moved aside, where no process holds them any more. The
  others stay, or with AtRestart are deleted when Windows restarts; returns whether any did. }
function DeleteOldCopies(Dir: String; AtRestart: Boolean): Boolean;
var
  Find: TFindRec;
begin
  Result := False;
  if FindFirst(Dir + '\jiskorean.dll.old-*', Find) then begin
    try
      repeat
        if not DeleteFile(Dir + '\' + Find.Name) and AtRestart then begin
          DeleteAtRestart(Dir + '\' + Find.Name, 0, MOVEFILE_DELAY_UNTIL_REBOOT);
          Result := True;
        end;
      until not FindNext(Find);
    finally
      FindClose(Find);
    end;
  end;
end;

{ A process with the TIP loaded keeps the file open, but it can be renamed: move it aside so new
  processes load the new build. Returns the new name, or '' if the file is gone. }
function RenameAside(Dir: String): String;
var
  Target: String;
begin
  Result := '';
  Target := Dir + '\jiskorean.dll';
  if FileExists(Target) and not DeleteFile(Target) then begin
    Result := Target + '.old-' + GetDateTimeString('yyyymmddhhnnsszzz', #0, #0);
    if not RenameFile(Target, Result) then
      RaiseException('Can''t replace ' + Target + '.');
  end;
end;

procedure InitializeWizard;
var
  Choice: String;
begin
  VariantPage := CreateInputOptionPage(wpLicense, 'Predictions',
    'Full or lite?',
    'The full version shows predictions as you type: Korean and English words, and Korean ' +
    'typed on the wrong layout. The lite version types exactly the same keys, without them.',
    True, False);
  VariantPage.Add('Full, with predictions (about {#FullMB} MB larger)');
  VariantPage.Add('Lite, without predictions');
  { /VARIANT=full or /VARIANT=lite on the command line (silent installs); else the last choice. }
  Choice := Lowercase(ExpandConstant('{param:variant|}'));
  if Choice = '' then Choice := GetPreviousData('Variant', 'full');
  if Choice = 'lite' then VariantPage.SelectedValueIndex := 1
  else VariantPage.SelectedValueIndex := 0;

  ModePage := CreateInputOptionPage(VariantPage.ID, 'Keyboard',
    'Which keyboard do you type on?',
    'JIS Korean starts in this mode. You can change it at any time: right-click the 한글 / ABC ' +
    'indicator in the taskbar.',
    True, False);
  ModePage.Add('Mode A: JIS (Japanese) keyboard');
  ModePage.Add('Mode B: US ANSI keyboard');
  ModePage.Add('Mode C: UK ISO keyboard');
  { /MODE=A, B or C on the command line; a mode the user already has still wins. }
  Choice := Uppercase(ExpandConstant('{param:mode|A}'));
  if Choice = 'B' then ModePage.SelectedValueIndex := 1
  else if Choice = 'C' then ModePage.SelectedValueIndex := 2
  else ModePage.SelectedValueIndex := 0;
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := (PageID = ModePage.ID) and HasMode;
end;

procedure RegisterPreviousData(PreviousDataKey: Integer);
begin
  if IsLite then SetPreviousData(PreviousDataKey, 'Variant', 'lite')
  else SetPreviousData(PreviousDataKey, 'Variant', 'full');
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Arch, Data, Mode: String;
  I, Code: Integer;
begin
  if CurStep = ssInstall then begin
    for I := 0 to 1 do begin
      if I = 0 then Arch := 'x64' else Arch := 'x86';
      DeleteOldCopies(ExpandConstant('{app}\' + Arch), False);
      RenameAside(ExpandConstant('{app}\' + Arch));
    end;
  end else if CurStep = ssPostInstall then begin
    { The log folder, writable by every process, AppContainer (Store) apps included. }
    Data := ExpandConstant('{commonappdata}\JIS Korean');
    ForceDirectories(Data);
    Exec(ExpandConstant('{sys}\icacls.exe'),
      '"' + Data + '" /grant *S-1-15-2-1:(OI)(CI)M *S-1-5-32-545:(OI)(CI)M',
      '', SW_HIDE, ewWaitUntilTerminated, Code);
    if not RegSvr('x64', '/s') or not RegSvr('x86', '/s') then
      MsgBox('JIS Korean couldn''t be registered with Windows. Run Setup again; if it keeps ' +
        'failing, see ' + '{#DocsUrl}.', mbError, MB_OK);
    Mode := '';
    if not HasMode then Mode := ' -Mode ' + IntToStr(ModePage.SelectedValueIndex);
    if not RunKeyboards(Mode) then
      MsgBox('JIS Korean is installed, but it couldn''t be added to your keyboards. Add it in ' +
        'Settings > Time & language > Language & region > Korean > Language options.',
        mbInformation, MB_OK);
  end;
end;

{ After the uninstaller's own "remove JIS Korean?" (ConfirmUninstall): whether to delete the user's own
  files too. Unticked by default. }
procedure AskAboutUserData;
var
  Form: TSetupForm;
  Text: TNewStaticText;
  Box: TNewCheckBox;
  Ok: TNewButton;
begin
  Form := CreateCustomForm(ScaleX(400), ScaleY(150), False, True);
  try
    Form.Caption := 'Uninstall JIS Korean';
    Text := TNewStaticText.Create(Form);
    Text.Parent := Form;
    Text.Left := ScaleX(16);
    Text.Top := ScaleY(16);
    Text.Width := Form.ClientWidth - ScaleX(32);
    Text.WordWrap := True;
    Text.Caption := 'Your training file and learned weights stay on this PC, for when you ' +
      'install JIS Korean again.';
    Box := TNewCheckBox.Create(Form);
    Box.Parent := Form;
    Box.Left := ScaleX(16);
    Box.Top := Text.Top + Text.Height + ScaleY(16);
    Box.Width := Form.ClientWidth - ScaleX(32);
    Box.Height := ScaleY(34);
    Box.Caption := 'Delete them too (%APPDATA%\JIS Korean)';
    Ok := TNewButton.Create(Form);
    Ok.Parent := Form;
    Ok.Caption := 'Continue';
    Ok.ModalResult := mrOk;
    Ok.Default := True;
    Ok.Cancel := True;
    Ok.Width := ScaleX(80);
    Ok.Height := ScaleY(24);
    Ok.Top := Form.ClientHeight - Ok.Height - ScaleY(12);
    Ok.Left := Form.ClientWidth - Ok.Width - ScaleX(16);
    Form.ShowModal;
    RemoveUserData := Box.Checked;
  finally
    Form.Free;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Arch, Dir, Aside: String;
  I: Integer;
begin
  if CurUninstallStep = usUninstall then begin
    if not UninstallSilent then AskAboutUserData;
    RunKeyboards('-Remove');
    for I := 0 to 1 do begin
      if I = 0 then Arch := 'x64' else Arch := 'x86';
      if FileExists(ExpandConstant('{app}\' + Arch + '\jiskorean.dll')) then RegSvr(Arch, '/s /u');
    end;
    RegDeleteKeyIncludingSubkeys(HKLM64, 'SOFTWARE\Microsoft\CTF\TIP\{#Clsid}');
    RegDeleteKeyIncludingSubkeys(HKLM64, 'SOFTWARE\Classes\CLSID\{#Clsid}');
    RegDeleteKeyIncludingSubkeys(HKLM32, 'SOFTWARE\Classes\CLSID\{#Clsid}');
    { Processes that loaded the DLL keep it open until they exit: move it aside and delete it
      when Windows restarts. }
    for I := 0 to 1 do begin
      if I = 0 then Arch := 'x64' else Arch := 'x86';
      Dir := ExpandConstant('{app}\' + Arch);
      if DeleteOldCopies(Dir, True) then LeftForRestart := True;
      Aside := RenameAside(Dir);
      if Aside <> '' then begin
        DeleteAtRestart(Aside, 0, MOVEFILE_DELAY_UNTIL_REBOOT);
        LeftForRestart := True;
      end;
      if LeftForRestart then DeleteAtRestart(Dir, 0, MOVEFILE_DELAY_UNTIL_REBOOT);
    end;
  end else if CurUninstallStep = usPostUninstall then begin
    if LeftForRestart then
      DeleteAtRestart(ExpandConstant('{app}'), 0, MOVEFILE_DELAY_UNTIL_REBOOT);
    if RemoveUserData then
      DelTree(ExpandConstant('{userappdata}\JIS Korean'), True, True, True);
    if LeftForRestart and not UninstallSilent then
      MsgBox('Some apps still have JIS Korean loaded. Its last files are deleted when Windows ' +
        'restarts.', mbInformation, MB_OK);
  end;
end;
