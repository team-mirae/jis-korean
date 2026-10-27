# Builds jiskorean.dll for x64 and x86 (32-bit apps load the x86 one) and runs the shell tests.
#   tools/build.ps1            full build, with predictions (spec §10)
#   tools/build.ps1 -Lite      the lean deterministic engine only
#   tools/build.ps1 -Legacy    predictions ranked by the rules alone, no learned ranker (spec §10.13)
#   tools/build.ps1 -Arch x64  one architecture
# Outputs: build/<arch>/jiskorean.dll, build/<arch>/jiskorean_tests.exe, build/<arch>/jk_testpad.exe
# (build/<arch>-lite/ with -Lite, build/<arch>-legacy/ with -Legacy)
param(
    [ValidateSet('x64', 'x86')] [string[]] $Arch = @('x64', 'x86'),
    [switch] $Lite,
    [switch] $Legacy
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\.."
$engine = Resolve-Path "$root\..\engine"
$cargo = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $cargo) { $cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe" }
if (-not (Test-Path $cargo)) { throw 'cargo not found: install Rust (rustup) first.' }

if ($Lite -and $Legacy) { throw '-Lite has no predictions to rank: use one of -Lite and -Legacy.' }
$variant = if ($Lite) { 'lite' } elseif ($Legacy) { 'legacy' } else { 'full' }
# The English modes (spec §3.6.2) are optional in the engine: Windows uses them as ABC's layouts.
[string[]] $features = if ($Lite) { '--no-default-features', '--features', 'english' }
    elseif ($Legacy) { '--features', 'english,legacy-predict' } else { '--features', 'english' }
$predict = if ($Lite) { 'OFF' } else { 'ON' }
$learned = if ($Legacy) { 'OFF' } else { 'ON' }
$triples = @{ x64 = 'x86_64-pc-windows-msvc'; x86 = 'i686-pc-windows-msvc' }

foreach ($a in $Arch) {
    $triple = $triples[$a]
    $target = "$engine\target\windows-$variant"
    # The static CRT, like the DLL: it loads into every app and can't count on a VC++ runtime.
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    $ErrorActionPreference = 'Continue'
    & $cargo build --manifest-path "$engine\Cargo.toml" --release --lib @features --target $triple --target-dir $target
    $ErrorActionPreference = 'Stop'
    if ($LASTEXITCODE) { throw "cargo build $triple failed" }
    Remove-Item Env:RUSTFLAGS
    $lib = "$target\$triple\release\jis_korean.lib"
    $build = if ($Lite) { "build/$a-lite" } elseif ($Legacy) { "build/$a-legacy" } else { "build/$a" }
    & "$PSScriptRoot\dev.ps1" -Arch $a -Command "cmake --preset $a -B $build -DJK_PREDICT=$predict -DJK_LEARNED=$learned -DJK_ENGINE_LIB='$lib' | Out-Null; if (`$LASTEXITCODE) { exit 1 }; cmake --build $build; if (`$LASTEXITCODE) { exit 1 }; & '$build\jiskorean_tests.exe'"
    if ($LASTEXITCODE) { throw "build or tests failed for $a" }
}
