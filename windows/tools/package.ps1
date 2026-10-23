# Builds the installer a user downloads: build/dist/JISKorean-<version>-setup.exe, one wizard with
# the full / lite choice inside (installer/jiskorean.iss, Inno Setup 6).
#   tools/package.ps1              build x64 + x86, full and lite (tools/build.ps1), then package
#   tools/package.ps1 -SkipBuild   package what's in build/ already
# Signing (off by default): set JK_SIGN_CERT to a .pfx file, and JK_SIGN_PASSWORD if it has one,
# and the DLLs, the installer and its uninstaller are signed with signtool (Windows SDK).
# JK_SIGN_TIMESTAMP overrides the timestamp server.
param([switch] $SkipBuild)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\.."
$build = "$root\build"

if (-not $SkipBuild) {
    & "$PSScriptRoot\build.ps1"
    & "$PSScriptRoot\build.ps1" -Lite
}
$dlls = foreach ($dir in 'x64', 'x86', 'x64-lite', 'x86-lite') { "$build\$dir\jiskorean.dll" }
foreach ($dll in $dlls) { if (-not (Test-Path $dll)) { throw "$dll is missing: run tools/build.ps1 (and -Lite)." } }

# The version, from the engine's Cargo.toml (as CMakeLists.txt reads it).
$line = Select-String -Path "$root\..\engine\Cargo.toml" -Pattern '^version = "(.+)"' | Select-Object -First 1
$version = $line.Matches[0].Groups[1].Value
$fullMB = [math]::Round(((Get-Item $dlls[0]).Length - (Get-Item $dlls[2]).Length) / 1MB, 1)

$iscc = (Get-Command iscc -ErrorAction SilentlyContinue).Source
foreach ($dir in "$env:LOCALAPPDATA\Programs\Inno Setup 6", "${env:ProgramFiles(x86)}\Inno Setup 6", "$env:ProgramFiles\Inno Setup 6") {
    if (-not $iscc -and (Test-Path "$dir\ISCC.exe")) { $iscc = "$dir\ISCC.exe" }
}
if (-not $iscc) { throw 'ISCC.exe not found: install Inno Setup 6 (winget install JRSoftware.InnoSetup).' }

$argv = @("/DVersion=$version", "/DBuild=$build", "/DFullMB=$fullMB")
if ($env:JK_SIGN_CERT) {
    $signtool = (Get-Command signtool -ErrorAction SilentlyContinue).Source
    if (-not $signtool) {
        $signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
            Sort-Object FullName | Select-Object -Last 1 -ExpandProperty FullName
    }
    if (-not $signtool) { throw 'signtool.exe not found: install the Windows SDK.' }
    $timestamp = if ($env:JK_SIGN_TIMESTAMP) { $env:JK_SIGN_TIMESTAMP } else { 'http://timestamp.digicert.com' }
    $sign = @('sign', '/fd', 'sha256', '/tr', $timestamp, '/td', 'sha256', '/f', $env:JK_SIGN_CERT)
    if ($env:JK_SIGN_PASSWORD) { $sign += @('/p', $env:JK_SIGN_PASSWORD) }
    & $signtool @sign @dlls
    if ($LASTEXITCODE) { throw 'signing the DLLs failed' }
    # Inno Setup runs this for the installer and the uninstaller; $f is the file.
    $quoted = ($sign | ForEach-Object { if ($_ -match '\s') { "`$q$_`$q" } else { $_ } }) -join ' '
    $argv += @('/DSign', "/Sjk=`$q$signtool`$q $quoted `$f")
}

& $iscc @argv "$root\installer\jiskorean.iss"
if ($LASTEXITCODE) { throw "ISCC failed ($LASTEXITCODE)" }
Write-Host "Built $build\dist\JISKorean-$version-setup.exe"
