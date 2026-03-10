# >>> recall hook start >>>
# Captures the last command via PSReadLine's AddToHistoryHandler, then logs
# it via recall from the prompt function.
#
# $LASTEXITCODE must be read at the very top of prompt{} — any other code
# run before it will overwrite the value.
# $PID is the PowerShell process ID, used as the session ID.
# Start-Job runs recall in a background thread so it never delays the prompt.
# Output is discarded so a recall failure stays invisible.
$Global:__RecallLastCmd = ""

Set-PSReadLineOption -AddToHistoryHandler {
    param($command)
    $Global:__RecallLastCmd = $command
    return $true
}

function prompt {
    $exitCode = $LASTEXITCODE
    $cmd = $Global:__RecallLastCmd
    if ($cmd -ne "") {
        $cwd = (Get-Location).Path
        $session = $PID
        Start-Job -ScriptBlock {
            param($c, $e, $d, $s)
            recall log $c --exit-code $e --cwd $d --session $s --shell pwsh 2>$null
        } -ArgumentList $cmd, $exitCode, $cwd, $session | Out-Null
        $Global:__RecallLastCmd = ""
    }
    "PS $($executionContext.SessionState.Path.CurrentLocation)$('>' * ($nestedPromptLevel + 1)) "
}
# <<< recall hook end <<<
