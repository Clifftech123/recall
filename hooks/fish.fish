# Captures the last command from fish using the fish_postexec event.
# fish_postexec fires automatically after every command completes and
# passes the command string as $argv[1] — no history parsing needed.
# This is cleaner than bash/zsh because fish gives us the command directly.
#
# $status must be read at the very top of the function — it holds the exit
# code of the command that just ran. Any other code running first would
# overwrite it with its own exit code.
# %self is the fish equivalent of $$ in bash/zsh — the PID of the current
# shell session, used as the session ID.
# The call is backgrounded with & so it never delays the next prompt.
# Errors are suppressed with 2>/dev/null so a recall failure is invisible.
function __recall_post_exec --on-event fish_postexec
    set -l exit_code $status
    set -l last_cmd $argv[1]
    if test -n "$last_cmd"
        recall log "$last_cmd" --exit-code $exit_code --cwd (pwd) --session %self --shell fish 2>/dev/null &
    end
end
