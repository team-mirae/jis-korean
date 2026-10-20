# Installs or removes the JIS Korean TIP on this PC (development install; needs admin, so it
# re-launches itself elevated). Other input methods and the rest of the language list are left
# as they are.
#   tools/install.ps1              copy build/<arch>/jiskorean.dll, register x64 + x86, add JIS
#                                  Korean to the Korean language's keyboards
#   tools/install.ps1 -Update      copy a new build over the installed one, no elevation (the
#                                  install folder is writable by the user who installed it)
#   tools/install.ps1 -Uninstall   undo all of it
#   tools/install.ps1 -Log         turn the diagnostic log on and tail it
# Apps that were running keep the old DLL (or none) until they restart.
param(
    [switch] $Uninstall,
    [switch] $Update,
    [switch] $Log,
    [string] $User  # set by the unelevated call: the SID that may update the install folder
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\.."
$dest = "$env:ProgramFiles\JIS Korean"
$data = "$env:ProgramData\JIS Korean"
$clsid = '{D10651AD-51EA-4DBA-8BFF-1905261B26A1}'
$tip = "0412:$clsid{669E0E87-FA58-46F6-96E1-0890A0224404}"

function Copy-Dlls {
    foreach ($arch in 'x64', 'x86') {
        New-Item "$dest\$arch" -ItemType Directory -Force | Out-Null
        $target = "$dest\$arch\jiskorean.dll"
        Get-ChildItem "$dest\$arch" -Filter 'jiskorean.dll.old-*' | Remove-Item -Force -ErrorAction SilentlyContinue
        if (Test-Path $target) {
            # A process with the TIP loaded keeps the file open, but it can be renamed: move it
            # aside so new processes load the new build.
            try { Remove-Item $target -Force } catch {
                Rename-Item $target "jiskorean.dll.old-$(Get-Date -f yyyyMMddHHmmssfff)"
            }
        }
        Copy-Item "$root\build\$arch\jiskorean.dll" "$dest\$arch\"
    }
}

if ($Update) {
    Copy-Dlls
    Write-Host "Updated $dest (apps load it when they next start or activate JIS Korean)"
    return
}

if ($Log) {
    New-Item 'HKCU:\Software\JIS Korean' -Force -ErrorAction SilentlyContinue | Out-Null
    Set-ItemProperty 'HKCU:\Software\JIS Korean' DiagnosticLog 1 -Type DWord
    Get-Content "$data\jiskorean.log" -Tail 40 -Wait -Encoding utf8
    return
}

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).
    IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    $argv = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"",
              '-User', [Security.Principal.WindowsIdentity]::GetCurrent().User.Value)
    if ($Uninstall) { $argv += '-Uninstall' }
    $p = Start-Process powershell -Verb RunAs -ArgumentList $argv -Wait -PassThru
    # The language list is per user: the elevated process may run as another account, so the
    # list is changed here, unelevated.
    if ($p.ExitCode) { exit $p.ExitCode }
    $list = Get-WinUserLanguageList
    $korean = $null
    foreach ($l in $list.ToArray()) { if ($l.LanguageTag -eq 'ko') { $korean = $l } }
    if ($Uninstall) {
        if ($korean -and $korean.InputMethodTips -contains $tip) {
            [void]$korean.InputMethodTips.Remove($tip)
            Set-WinUserLanguageList $list -Force
        }
    } else {
        if (-not $korean) {
            $korean = (New-WinUserLanguageList 'ko')[0]
            $korean.InputMethodTips.Clear()
            $list.Add($korean)
        }
        if ($korean.InputMethodTips -notcontains $tip) {
            $korean.InputMethodTips.Add($tip)
            Set-WinUserLanguageList $list -Force
        }
    }
    Write-Host 'Korean keyboards now:'
    foreach ($l in (Get-WinUserLanguageList).ToArray()) {
        if ($l.LanguageTag -eq 'ko') { $l.InputMethodTips | ForEach-Object { "  $_" } }
    }
    exit 0
}

# The elevated window closes on exit: keep a transcript.
New-Item $data -ItemType Directory -Force | Out-Null
Start-Transcript "$data\install.log" -Append | Out-Null
trap { Write-Host $_ -ForegroundColor Red; Stop-Transcript | Out-Null; exit 1 }

if ($Uninstall) {
    foreach ($arch in 'x64', 'x86') {
        $regsvr = if ($arch -eq 'x64') { 'System32' } else { 'SysWOW64' }
        if (Test-Path "$dest\$arch\jiskorean.dll") {
            & "$env:SystemRoot\$regsvr\regsvr32.exe" /s /u "$dest\$arch\jiskorean.dll"
        }
    }
    foreach ($key in "HKLM:\SOFTWARE\Microsoft\CTF\TIP\$clsid", "HKLM:\SOFTWARE\Classes\CLSID\$clsid",
                     "HKLM:\SOFTWARE\WOW6432Node\Classes\CLSID\$clsid") {
        Remove-Item $key -Recurse -ErrorAction SilentlyContinue
    }
    # Processes that loaded the DLL keep it open until they exit; delete what we can.
    Remove-Item $dest -Recurse -Force -ErrorAction SilentlyContinue
    if (Test-Path $dest) { Write-Warning "$dest is still in use; delete it after signing out." }
    Stop-Transcript | Out-Null
    exit 0
}

# The log folder, writable by every process, AppContainer (Store) apps included.
icacls $data /grant '*S-1-15-2-1:(OI)(CI)M' '*S-1-5-32-545:(OI)(CI)M' | Out-Null
# Program Files is readable by ALL APPLICATION PACKAGES; the user's Desktop isn't.
New-Item $dest -ItemType Directory -Force | Out-Null
# Development install: the installing user may update the DLLs (-Update) without elevation.
if ($User) { icacls $dest /grant "*${User}:(OI)(CI)M" | Out-Null }
Copy-Dlls
# The licenses of the data inside the DLL.
foreach ($f in "$root\..\engine\dict\LICENSE-libhangul-hanja.txt", "$root\..\engine\dict\LICENSE-unicode.txt",
               "$root\..\engine\predict\LICENSE-corpora.txt", "$root\..\engine\predict\LICENSE-wordnet.txt",
               "$root\..\LICENSE") {
    Copy-Item $f $dest -Force
}
& "$env:SystemRoot\System32\regsvr32.exe" /s "$dest\x64\jiskorean.dll"
if ($LASTEXITCODE) { throw "regsvr32 x64 failed ($LASTEXITCODE)" }
& "$env:SystemRoot\SysWOW64\regsvr32.exe" /s "$dest\x86\jiskorean.dll"
if ($LASTEXITCODE) { throw "regsvr32 x86 failed ($LASTEXITCODE)" }
Write-Host "Registered $dest"
Stop-Transcript | Out-Null
exit 0
