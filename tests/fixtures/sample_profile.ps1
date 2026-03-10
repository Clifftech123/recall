# PowerShell Profile
# ~/.config/powershell/Microsoft.PowerShell_profile.ps1

# ── Environment ───────────────────────────────────────────────────────────────
$env:EDITOR = "nvim"
$env:PAGER  = "less"

# ── Aliases ───────────────────────────────────────────────────────────────────
Set-Alias -Name vim  -Value nvim
Set-Alias -Name grep -Value Select-String
Set-Alias -Name ll   -Value Get-ChildItem

# ── Prompt ────────────────────────────────────────────────────────────────────
function prompt {
    $location = Get-Location
    Write-Host "PS " -NoNewline -ForegroundColor DarkGray
    Write-Host "$location" -NoNewline -ForegroundColor Cyan
    Write-Host " >" -NoNewline -ForegroundColor DarkGray
    return " "
}

# ── PSReadLine ────────────────────────────────────────────────────────────────
if (Get-Module -ListAvailable -Name PSReadLine) {
    Set-PSReadLineOption -EditMode Emacs
    Set-PSReadLineOption -HistorySearchCursorMovesToEnd
    Set-PSReadLineKeyHandler -Key UpArrow   -Function HistorySearchBackward
    Set-PSReadLineKeyHandler -Key DownArrow -Function HistorySearchForward
}

# ── Functions ─────────────────────────────────────────────────────────────────
function which ($command) {
    Get-Command -Name $command -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty Source
}

function mkcd ($path) {
    New-Item -ItemType Directory -Path $path -Force | Out-Null
    Set-Location $path
}
