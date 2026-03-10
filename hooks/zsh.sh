# Captures the last command from zsh history using fc -ln -1, which prints
# the last history entry without a line number (the -n flag strips it).
# This is the zsh equivalent of bash's "history 1 | sed ..." approach.
#
# $? must be read at the very top of the function — it holds the exit code
# of the command that just ran. Any other code running first would overwrite it.
# $$ is the PID of the current shell, used as the session ID.
# The call is backgrounded with & so it never delays the next prompt.
# Errors are suppressed with 2>/dev/null so a recall failure is invisible.
__recall_precmd() {
    local exit_code=$?
    local last_cmd=$(fc -ln -1)
    if [ -n "$last_cmd" ]; then
        recall log "$last_cmd" --exit-code $exit_code --cwd "$(pwd)" --session $$ --shell zsh 2>/dev/null &
    fi
}

# add-zsh-hook is the idiomatic zsh way to register a precmd function.
# precmd hooks run before each prompt is displayed — after every command.
# autoload -Uz ensures the add-zsh-hook function is loaded before we call it.
# Using add-zsh-hook is safer than appending directly to the precmd array
# because it handles deduplication and does not break other precmd hooks.
autoload -Uz add-zsh-hook
add-zsh-hook precmd __recall_precmd
