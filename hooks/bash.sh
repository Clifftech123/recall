# Captures the last command from bash history, strips the history number
# prefix (e.g. "  42  git status" → "git status"), then logs it via recall.
#
# $? must be read at the very top of the function — by the time any other
# code runs the exit code of the previous command is still in $?.
# $$ is the PID of the current shell, used as the session ID.
# The call is backgrounded with & so it never delays the next prompt.
# Errors are suppressed with 2>/dev/null so a recall failure is invisible.
__recall_precmd() {
    local exit_code=$?
    local last_cmd=$(history 1 | sed 's/^[ ]*[0-9]*[ ]*//')
    if [ -n "$last_cmd" ]; then
        recall log "$last_cmd" --exit-code $exit_code --cwd "$(pwd)" --session $$ --shell bash 2>/dev/null &
    fi
}

# Only add __recall_precmd to PROMPT_COMMAND once.
# PROMPT_COMMAND is a bash variable that holds a list of functions to run
# before each prompt is displayed — this is how we hook into every command.
# ${PROMPT_COMMAND:-} expands to an empty string if PROMPT_COMMAND is unset,
# avoiding an "unbound variable" error when strict mode is on.
if [[ ! "$PROMPT_COMMAND" == *"__recall_precmd"* ]]; then
    PROMPT_COMMAND="__recall_precmd;${PROMPT_COMMAND:-}"
fi
