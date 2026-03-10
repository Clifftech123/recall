# Captures the last command using PSReadLine's AddToHistoryHandler.
# PSReadLine is the readline library used by PowerShell 5.1+ and PowerShell 7+.
# AddToHistoryHandler fires every time a command is added to history —
# which happens right after the user presses Enter. We store it in a global
# variable so the prompt function can pick it up on the next render.
#
# Returning $true from the handler tells PSReadLine to still add the command
# to its own history as normal — we are only observing, not replacing.
$Global:__RecallLastCmd = ""
Set-PSReadLineOption -AddToHistoryHandler {
    param($command)
    $Global:__RecallLastCmd = $command
    return $true
}

# The prompt function runs before every prompt is displayed — after every
# command. This is the PowerShell equivalent of bash's PROMPT_COMMAND.
# We read $LASTEXITCODE at the very top before anything else runs, because
# any PowerShell expression that executes will overwrite $LASTEXITCODE with
# its own result (even a successful [string] comparison returns 0).
#
# Start-Process is used instead of a direct call so recall runs in a
# separate process and never blocks the prompt. -NoNewWindow keeps it
# invisible. -RedirectStandardError "NUL" suppresses any error output
# so a recall failure is completely invisible to the user.
#
# After logging, __RecallLastCmd is reset to "" so the same command is
# never logged twice if prompt is called more than once in a session.
function prompt {
    $exitCode = $LASTEXITCODE
    if ($Global:__RecallLastCmd -ne "") {
        Start-Process `
            -NoNewWindow `
            -FilePath "recall" `
            -ArgumentList "log", "`"$Global:__RecallLastCmd`"", "--exit-code", "$exitCode", "--cwd", "$(Get-Location)", "--session", "$PID", "--shell", "pwsh" `
            -RedirectStandardError "NUL"
        $Global:__RecallLastCmd = ""
    }

    # Return the standard PowerShell prompt string.
    # This preserves the default PS prompt appearance — if the user has
    # a custom prompt function already, recall unhook restores it.
    "PS $($executionContext.SessionState.Path.CurrentLocation)$('>' * ($nestedPromptLevel + 1)) "
}
