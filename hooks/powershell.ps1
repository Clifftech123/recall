# >>> recall hook start >>>
# >>> recall hook start >>>
$Global:__RecallLastCmd = ""
Set-PSReadLineOption -AddToHistoryHandler {
    param($command)
    $Global:__RecallLastCmd = $command
    return $true
}
function prompt {
    $exitCode = $LASTEXITCODE
    if ($Global:__RecallLastCmd -ne "") {
        Start-Process -NoNewWindow -FilePath "recall" -ArgumentList "log", "`"$Global:__RecallLastCmd`"", "--exit-code", "$exitCode", "--cwd", "$(Get-Location)", "--session", "$PID", "--shell", "pwsh" -RedirectStandardError "NUL"
        $Global:__RecallLastCmd = ""
    }
    "PS $($executionContext.SessionState.Path.CurrentLocation)$('>' * ($nestedPromptLevel + 1)) "
}
# <<< recall hook end <<<
# <<< recall hook end <<<
