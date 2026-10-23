# The per-user half of the installer (installer/jiskorean.iss runs it as the user who started
# Setup, not elevated): adds JIS Korean to the Korean language's keyboards, or removes it, as
# tools/install.ps1 does. Other input methods and the rest of the language list are left alone.
#   keyboards.ps1             add JIS Korean (and Korean, if it isn't in the list)
#   keyboards.ps1 -Mode 1     also set the starting mode (0 A, 1 B, 2 C) if none is set yet
#   keyboards.ps1 -Remove     take it out again
param(
    [switch] $Remove,
    [int] $Mode = -1
)
$ErrorActionPreference = 'Stop'
$tip = '0412:{D10651AD-51EA-4DBA-8BFF-1905261B26A1}{669E0E87-FA58-46F6-96E1-0890A0224404}'

$list = Get-WinUserLanguageList
$korean = $null
foreach ($l in $list.ToArray()) { if ($l.LanguageTag -eq 'ko') { $korean = $l } }
if ($Remove) {
    if ($korean -and $korean.InputMethodTips -contains $tip) {
        [void]$korean.InputMethodTips.Remove($tip)
        Set-WinUserLanguageList $list -Force
    }
    exit 0
}
if (-not $korean) {
    $korean = (New-WinUserLanguageList 'ko')[0]
    $korean.InputMethodTips.Clear()
    $list.Add($korean)
}
if ($korean.InputMethodTips -notcontains $tip) {
    $korean.InputMethodTips.Add($tip)
    Set-WinUserLanguageList $list -Force
}

# The starting mode, only for a user who hasn't picked one: an upgrade keeps theirs.
if ($Mode -ge 0) {
    $key = 'HKCU:\Software\JIS Korean'
    if (-not (Test-Path $key)) { New-Item $key | Out-Null }
    if ($null -eq (Get-ItemProperty $key -Name Mode -ErrorAction SilentlyContinue)) {
        Set-ItemProperty $key Mode $Mode -Type DWord
    }
}
exit 0
