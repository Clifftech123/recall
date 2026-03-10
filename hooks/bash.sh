# >>> recall hook start >>>
__recall_precmd() {
    local exit_code=$?
    local last_cmd=$(history 1 | sed 's/^[ ]*[0-9]*[ ]*//')
    if [ -n "$last_cmd" ]; then
        recall log "$last_cmd" --exit-code $exit_code --cwd "$(pwd)" --session $$ --shell bash 2>/dev/null &
    fi
}
if [[ ! "$PROMPT_COMMAND" == *"__recall_precmd"* ]]; then
    PROMPT_COMMAND="__recall_precmd;${PROMPT_COMMAND:-}"
fi
# <<< recall hook end <<<
