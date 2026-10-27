# Types key sequences through the installed TIP into jk_testpad and checks the text: a classic
# Edit control and a RichEdit through the IMM compatibility layer, and a RichEdit on TSF itself. The testpad is the foreground window while it runs: don't touch
# the keyboard. Settings that change what's typed (Mode, Predictions) are set for the run and
# restored after, and Learn from My Choices is off meanwhile: the test's picks aren't the user's.
#   tools/type-test.ps1                 all cases, both controls
#   tools/type-test.ps1 -Target rich    one control
param(
    [ValidateSet('edit', 'rich', 'tsf')] [string[]] $Target = @('edit', 'rich', 'tsf'),
    [string] $Filter = ''
)
$ErrorActionPreference = 'Stop'
$pad = Resolve-Path "$PSScriptRoot\..\build\x64\jk_testpad.exe"
$key = 'HKCU:\Software\JIS Korean'
if (-not (Test-Path $key)) { New-Item $key | Out-Null }

# name, Mode (0 A, 1 B, 2 C), Predictions, keys, expected text in the control[, DedicatedSymbols
# [, EnglishMode: ABC's layout id, 'unset' for the keyboard's default, 'windows' for Windows' own]]
$cases = @(
    @('한글 안녕 (Space commits)', 0, 0, 'KeyG KeyK KeyS KeyR KeyM KeyF Space KeyD KeyK KeyS KeyS KeyU KeyD Space', '한글 안녕 '),
    @('Enter commits, new line', 0, 0, 'KeyG KeyK KeyS Enter KeyG KeyK Space', "한`r`n하 "),
    @('arrow commits, then moves', 0, 0, 'KeyG KeyK KeyS ArrowLeft KeyG KeyK Space', '하 한'),
    @('Tab commits, then passes', 0, 0, 'KeyG KeyK KeyS Tab', "한`t"),
    @('Tab picks a prediction', 1, 1, 'KeyH KeyE KeyL KeyL KeyO Tab Tab Space', 'Hello '),
    @('F7 list: ↓ then 1-9 page pick', 0, 0, 'KeyG KeyK KeyR KeyR KeyY F7 ArrowDown Digit1 Space', '學校 '),
    @('implied ㅇ: ㅏ → 아', 0, 0, 'KeyK Space', '아 '),
    @('number row 1 → ㅃ, 0 → ㅘ', 0, 0, 'Digit1 KeyK Digit0 Space', '빠와 '),
    @('Shift keeps the syllable: 있', 0, 0, 'KeyD KeyL S-KeyT Space', '있 '),
    @('Backspace by jamo', 0, 0, 'KeyG KeyK KeyS Backspace Space', '하 '),
    @('F10 → gks, F10 → GKS', 0, 0, 'KeyG KeyK KeyS F10 F10 Space', 'GKS '),
    @('Shift+Z is F10', 0, 0, 'KeyG KeyK KeyS S-KeyZ Space', 'gks '),
    @('F9 full width', 0, 0, 'KeyG KeyK KeyS F9 Space', 'ｇｋｓ '),
    @('F6 drops the implied ㅇ', 0, 0, 'KeyK F6', 'ㅏ'),
    @('F7 hanja, Enter picks', 0, 0, 'KeyG KeyK KeyR KeyR KeyY F7 Enter Space', '學校 '),
    @('Mode A symbols: @ key → ''', 0, 0, 'BracketLeft Space', "' "),
    @('Mode B: Shift+[ → 「, Shift+2 → 2', 1, 0, 'S-BracketLeft S-Digit2 Space', '「2 '),
    @('Mode B: 한', 1, 0, 'KeyG KeyK KeyS Space', '한 '),
    @('한/영: ABC types English, back to 한글', 1, 0, 'KeyG KeyK Hangul KeyG KeyK Space Hangul KeyG KeyK Space', '하gk 하 '),
    @('predictions: hello', 1, 1, 'KeyH KeyE KeyL KeyL KeyO Space', 'hello '),
    @('predictions: 한글 stays Korean', 1, 1, 'KeyG KeyK KeyS KeyR KeyM KeyF Space', '한글 '),
    @('predictions: Shift+Enter commits hello, no new line', 1, 1, 'KeyH KeyE KeyL KeyL KeyO S-Enter', 'hello'),
    @('Mode C: ISO key · —, Shift+[ 「', 2, 0, 'IntlBackslash S-IntlBackslash S-BracketLeft KeyG KeyK KeyS Space', '·—「한 '),
    @('Mode C, dedicated symbol keys off', 2, 0, 'IntlBackslash S-IntlBackslash S-BracketLeft Space', '\|{ ', 0),
    @('ABC in Mode A: JIS English (default)', 0, 0, 'Hangul KeyG S-Digit2 BracketLeft Hangul', 'g"@', 1, 'unset'),
    @('ABC in Mode B: US English (default)', 1, 0, 'Hangul KeyG S-Digit2 BracketLeft Hangul', 'g@[', 1, 'unset'),
    @('ABC: JIS Dvorak', 0, 0, 'Hangul KeyS KeyD KeyF S-KeyJ Hangul', 'oeuH', 1, 7),
    @('ABC: Windows layout passes keys', 0, 0, 'Hangul KeyG S-Digit2 Hangul', 'g@', 1, 'windows'),
    @('ABC in Mode A, then 한글 again', 0, 0, 'Hangul KeyG KeyK Hangul KeyG KeyK Space', 'gk하 ', 1, 'unset')
)
# Password fields: every key passes through untouched (ABC), with predictions on.
# (The leading comma keeps a one-case list a list of cases: PowerShell would unroll it.)
$passwordCases = @(
    ,@('password field passes keys through', 1, 1, 'KeyG KeyK KeyS S-KeyR Digit1 KeyH KeyE KeyL KeyL KeyO', 'gksR1hello')
)
# Hosts that misbehave (jk_testpad's options and targets): name, Mode, Predictions, keys, expected,
# targets, extra jk_testpad arguments.
$hostCases = @(
    @('#5 auto-repeat: one jamo per repeat', 0, 0, 'D-KeyG D-KeyG D-KeyG D-KeyG R-KeyG KeyE KeyE Space', 'ㅎㅎㅎㅎㄷㄷ ', @('edit', 'rich', 'tsf'), @()),
    @('#5 a key tested twice runs once', 0, 0, 'KeyE KeyK KeyE KeyK Space', '다다 ', @('edit', 'rich', 'tsf'), @('--peek', '1', '--probe', '1')),
    @('#5 auto-repeat tested twice', 0, 0, 'D-KeyG D-KeyG D-KeyG R-KeyG Space', 'ㅎㅎㅎ ', @('tsf'), @('--peek', '1', '--probe', '1')),
    @('#4 Unicode-injected characters pass as typed', 0, 0, 'U+0061 U+0020 U+0062 U+002C U+002F U+0030 U+0035 U+0056 U+0073 U+007D U+AC10', 'a b,/05Vs}감', @('edit', 'rich', 'tsf'), @()),
    @('#4 Unicode-injected character commits first', 1, 1, 'KeyG KeyK U+0020 KeyG KeyK U+0078', '하 하x', @('edit', 'rich', 'tsf'), @()),
    @('#4 Unicode-injected characters in ABC', 0, 0, 'Hangul U+0061 U+0020 U+0030 Hangul', 'a 0', @('edit', 'tsf'), @())
)

$saved = @{}
foreach ($name in 'Mode', 'Predictions', 'DedicatedSymbols', 'EnglishMode', 'LearnFromChoices') {
    $saved[$name] = (Get-ItemProperty $key -Name $name -ErrorAction SilentlyContinue).$name
}
Set-ItemProperty $key LearnFromChoices 0 -Type DWord
$failed = 0
$out = Join-Path $env:TEMP 'jk-type-test.txt'
try {
    foreach ($c in @($cases) + @($passwordCases) + @($hostCases)) {
        if ($Filter -and $c[0] -notlike "*$Filter*") { continue }
        $hosted = $hostCases -contains $c
        Set-ItemProperty $key Mode $c[1] -Type DWord
        Set-ItemProperty $key Predictions $c[2] -Type DWord
        Set-ItemProperty $key DedicatedSymbols $(if ($c.Count -gt 5 -and -not $hosted) { $c[5] } else { 1 }) -Type DWord
        $english = if ($c.Count -gt 6 -and -not $hosted) { $c[6] } else { 'unset' }
        if ($english -eq 'unset') { Remove-ItemProperty $key -Name EnglishMode -ErrorAction SilentlyContinue }
        elseif ($english -eq 'windows') { Set-ItemProperty $key EnglishMode ([int32]-1) -Type DWord }
        else { Set-ItemProperty $key EnglishMode $english -Type DWord }
        $targets = if ([object]::ReferenceEquals($c, $passwordCases[0])) { @('password', 'pwscope') }
            elseif ($hosted) { $c[5] } else { $Target }
        $extra = if ($hosted) { $c[6] } else { @() }
        foreach ($t in $targets) {
            Remove-Item $out -ErrorAction SilentlyContinue
            $p = Start-Process $pad -ArgumentList (@('--profile', 'on', '--target', $t, '--keys', "`"$($c[3])`"", '--out', "`"$out`"") + $extra) -PassThru
            if (-not $p.WaitForExit(30000)) { $p.Kill(); Write-Host "TIMEOUT $($c[0]) [$t]" -ForegroundColor Red; $failed++; continue }
            $got = if (Test-Path $out) { [IO.File]::ReadAllText($out, [Text.Encoding]::UTF8) } else { '<no output>' }
            if ($got -ceq $c[4]) {
                Write-Host "ok   $($c[0]) [$t]"
            } else {
                Write-Host "FAIL $($c[0]) [$t]: got [$got], expected [$($c[4])]" -ForegroundColor Red
                $failed++
            }
        }
    }
} finally {
    foreach ($name in $saved.Keys) {
        if ($null -eq $saved[$name]) { Remove-ItemProperty $key -Name $name -ErrorAction SilentlyContinue }
        else { Set-ItemProperty $key $name $saved[$name] -Type DWord }
    }
}
if ($failed) { Write-Host "$failed failed" -ForegroundColor Red; exit 1 }
Write-Host 'all passed'
